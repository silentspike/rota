//! Die Zähler — je Strang eine eigene Cache-Zeile (#933, AC9).
//!
//! # Warum 128 Bytes und nicht 64
//!
//! Eine Cache-Zeile ist auf allen drei Rechnern des Projekts 64 Bytes lang.
//! Der L2-Vorabholer der Intel-Kerne (proxmox, gonet) lädt aber **Paare**
//! benachbarter Zeilen, ausgerichtet auf 128 Bytes. Zwei Zähler in
//! benachbarten 64-Byte-Zeilen teilen sich damit doch eine Einheit, und jede
//! Erhöhung des einen zieht die Zeile des anderen mit — genau die Bandbreite,
//! die die Anzeige messen soll. 128 ist dieselbe Wahl, die `crossbeam` für
//! `CachePadded` auf x86-64 trifft.
//!
//! # Was der heiße Pfad tut — und was nicht
//!
//! **Eine Erhöhung je Kampf, sonst nichts.** Kein Zeitstempel, keine Sperre,
//! keine Speicheranforderung. Kandidaten, Durchgänge und der Laufzustand werden
//! je Auswertung gezählt, also einmal je `iterations` Kämpfe, und das vom
//! Strang, der die Auswertung angestoßen hat.
//!
//! # Warum über eine Strang-Anmeldung und nicht über ein Feld in `BalanceConfig`
//!
//! `BalanceConfig` beschreibt, WAS gerechnet wird, und ist `Copy`. Ein Zähler
//! darin gehörte zur Identität jeder Konfiguration, die irgendwo verglichen oder
//! kopiert wird — dieselbe Trennung, die [`crate::drosselung`] für die
//! Steuerung zieht. Die Anmeldung je Strang folgt dem Muster von
//! [`crate::diagnostics`] und erreicht damit jeden Weg, der heute über
//! [`crate::balance`] rechnet, ohne dass ein Aufrufer etwas durchreicht.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Wie weit zwei Strangzähler mindestens auseinanderliegen, in Bytes.
pub const CACHE_ZEILE: usize = 128;

/// Die Auswertungswege, die es heute gibt.
///
/// **Genau drei, und keiner ist erfunden.** Der Plan nennt in seiner
/// Beispielzeile Cache, analytisch, Ledger und Suffix — keiner davon steht
/// heute in einem Lauf (siehe [`WEGE_OHNE_LAUF`]). Sie hier mit 0 % zu führen
/// hieße, „der Cache trifft nie" zu behaupten, wo es gar keinen gibt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weg {
    /// Ein Kampf entscheidet: Nirgends wird gewürfelt, sein Ausgang ist exakt
    /// (#908 im festen Lauf, `EinDurchgang` im gestaffelten).
    EinKampf,
    /// Die feste Stichprobe: `iterations` Kämpfe, gemittelt.
    Stichprobe,
    /// Die gestaffelte Stichprobe (#915, AC3): Kämpfe, bis die Aussage sicher ist.
    Gestaffelt,
}

impl Weg {
    /// Wie viele Wege es gibt — die Länge der Zählerfelder.
    pub const ANZAHL: usize = 3;

    /// Alle Wege in fester Reihenfolge.
    pub const ALLE: [Self; Self::ANZAHL] = [Self::EinKampf, Self::Stichprobe, Self::Gestaffelt];

    /// Der Platz im Zählerfeld.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::EinKampf => 0,
            Self::Stichprobe => 1,
            Self::Gestaffelt => 2,
        }
    }

    /// Der Schlüssel im JSON-Strom — stabil, unabhängig von `Debug`.
    #[must_use]
    pub const fn schluessel(self) -> &'static str {
        match self {
            Self::EinKampf => "one_fight",
            Self::Stichprobe => "fixed_sample",
            Self::Gestaffelt => "sequential",
        }
    }

    /// Der Name in der Zeile für Menschen.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::EinKampf => "ein Kampf",
            Self::Stichprobe => "Stichprobe",
            Self::Gestaffelt => "gestaffelt",
        }
    }
}

/// Die Auswertungswege des Plans, die heute in keinem Lauf stehen (E11.4b).
///
/// Sie stehen in beiden Ausgaben **mit Namen**, nicht als 0 %: Wer die Zeile
/// liest, soll sehen, dass es sie nicht gibt — nicht, dass sie nie greifen.
/// `analytisch` ist dabei kein Versehen: [`crate::analytisch`] existiert als
/// Funktion, aber kein Lauf ruft sie; ohne Router gibt es keinen Weg dorthin.
pub const WEGE_OHNE_LAUF: [&str; 5] = ["cache", "analytic", "ledger", "suffix", "avx2"];

/// Die Zähler eines Strangs, auf einer eigenen Cache-Zeile.
///
/// Nur `kaempfe` liegt im heißen Pfad. Die übrigen Felder ändern sich einmal je
/// Auswertung, und auch das nur im Strang, der die Auswertung angestoßen hat.
#[repr(C, align(128))]
#[derive(Debug, Default)]
pub struct Strangzaehler {
    /// Vollständig gerechnete Kämpfe — vom Kern gerechnet, nicht angefordert.
    kaempfe: AtomicU64,
    /// Durchgänge, die eine Auswertung geliefert hat, einschließlich der
    /// vervielfachten aus der Reduktion (#908). `durchgaenge - kaempfe` sind die
    /// vermiedenen Vollkämpfe.
    durchgaenge: AtomicU64,
    /// Abgeschlossene Auswertungen je [`Weg`].
    kandidaten: [AtomicU64; Weg::ANZAHL],
}

// Die Zusicherung aus AC9 als Übersetzungsbedingung: Ein Umbau, der die
// Ausrichtung verliert, baut nicht mehr — er fällt nicht erst in einer Messung auf.
const _: () = assert!(std::mem::align_of::<Strangzaehler>() >= CACHE_ZEILE);
const _: () = assert!(std::mem::size_of::<Strangzaehler>() % CACHE_ZEILE == 0);

/// Wie viel Arbeit zu einem Lauf gehört — in abgeschlossenen Auswertungen.
///
/// Die drei Fälle aus AC3, und nur sie. Die Mengen sind **absolut**: gezählt
/// ab Laufbeginn, nicht ab der aktuellen Runde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arbeitsmenge {
    /// Noch nicht gesetzt — dann gibt es auch keine Restzeit.
    Unbekannt,
    /// Vorher bekannt: Durchlauf, fester Lauf, Durchsatzmessung.
    Bekannt {
        /// Auswertungen insgesamt.
        kandidaten: u64,
    },
    /// Nach oben beschränkt: Die gierige Suche endet spätestens an der
    /// Längengrenze, früher, wenn keine Zeile mehr trägt.
    Hoechstens {
        /// So viele Auswertungen gibt es mindestens.
        mindestens: u64,
        /// Mehr können es nicht werden.
        hoechstens: u64,
    },
    /// Offen bis zur Konvergenz — **keine Restzeit**, das ist eine Zusicherung.
    Offen,
}

/// Das beste bisher gefundene Ergebnis und seit wann es steht.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bestes {
    /// Der Wert, wie die Suche ihn vergleicht.
    pub wert: f64,
    /// Wann er gesetzt wurde, in Nanosekunden ab Laufbeginn.
    pub seit_ns: u64,
    /// Wie viele Auswertungen bis dahin abgeschlossen waren.
    pub bei_kandidaten: u64,
}

/// Der Zustand eines Laufs jenseits der Zähler — selten geschrieben.
///
/// Steht hinter einer Sperre, nicht in Atomen: Runde und Rundenplan müssen
/// **zusammen** gelesen werden. Zwei getrennte Atome könnten die neue Runde mit
/// dem alten Plan zeigen, und dann stünde für eine Viertelsekunde ein Balken
/// über hundert Prozent. Geschrieben wird einmal je Runde, gelesen viermal je
/// Sekunde — die Sperre liegt nicht im heißen Pfad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Laufstand {
    /// Wie viel Arbeit zum Lauf gehört.
    pub menge: Arbeitsmenge,
    /// Die laufende Runde der Suche, ab 1 — oder `None` ohne Runden.
    pub runde: Option<u64>,
    /// Abgeschlossene Auswertungen beim Beginn dieser Runde.
    pub runde_ab_kandidaten: u64,
    /// Auswertungen, die diese Runde sicher kostet.
    pub runde_geplant: u64,
    /// Das bisher beste Ergebnis — `None`, solange es keines gibt.
    pub bestes: Option<Bestes>,
}

impl Default for Laufstand {
    fn default() -> Self {
        Self {
            menge: Arbeitsmenge::Unbekannt,
            runde: None,
            runde_ab_kandidaten: 0,
            runde_geplant: 0,
            bestes: None,
        }
    }
}

/// Alle Zähler eines Laufs.
///
/// Platz 0 gehört dem **Leitstrang** — dem, der den Lauf angemeldet hat. Die
/// Plätze ab 1 gehören den Arbeitssträngen eines parallelen Laufs, in der
/// Reihenfolge ihrer Bereiche.
#[derive(Debug)]
pub struct Zaehlwerk {
    beginn: Instant,
    straenge: Box<[Strangzaehler]>,
    zustand: Mutex<Laufstand>,
}

/// Was eine Momentaufnahme festhält — alles, woraus eine [`Anzeige`] entsteht.
///
/// [`Anzeige`]: crate::fortschritt::Anzeige
#[derive(Debug, Clone, PartialEq)]
pub struct Momentaufnahme {
    /// Nanosekunden ab Laufbeginn, monoton.
    pub elapsed_ns: u64,
    /// Gerechnete Kämpfe, über alle Stränge.
    pub kaempfe: u64,
    /// Gelieferte Durchgänge, über alle Stränge.
    pub durchgaenge: u64,
    /// Abgeschlossene Auswertungen je [`Weg`], über alle Stränge.
    pub kandidaten: [u64; Weg::ANZAHL],
    /// Gerechnete Kämpfe je Strangplatz — für aktive Stränge und Strangrate.
    pub kaempfe_je_strang: Vec<u64>,
    /// Der Laufzustand zum selben Zeitpunkt.
    pub stand: Laufstand,
}

impl Momentaufnahme {
    /// Abgeschlossene Auswertungen über alle Wege.
    #[must_use]
    pub fn kandidaten_gesamt(&self) -> u64 {
        self.kandidaten.iter().sum()
    }
}

/// Ein angemeldeter Strang: sein Zählwerk und sein Platz darin.
#[derive(Debug, Clone)]
pub struct Strang {
    werk: Arc<Zaehlwerk>,
    platz: usize,
}

/// Hebt die Anmeldung beim Fallenlassen wieder auf.
///
/// Stellt den **vorherigen** Zustand wieder her, nicht „keiner": Ein Lauf, der
/// innerhalb eines angemeldeten Laufs einen eigenen anmeldet, gibt danach den
/// äußeren zurück.
#[derive(Debug)]
#[must_use = "die Anmeldung gilt nur, solange dieser Wert lebt"]
pub struct Anmeldung {
    vorher: Option<Strang>,
}

thread_local! {
    static ANGEMELDET: RefCell<Option<Strang>> = const { RefCell::new(None) };
}

impl Zaehlwerk {
    /// Ein Zählwerk für einen Leitstrang und `arbeitsstraenge` Arbeitsstränge.
    #[must_use]
    pub fn neu(arbeitsstraenge: usize) -> Arc<Self> {
        todo!("#933 {arbeitsstraenge}")
    }

    /// Meldet den laufenden Strang als Leitstrang an (Platz 0).
    pub fn anmelden(self: &Arc<Self>) -> Anmeldung {
        todo!("#933 {self:?}")
    }

    /// Wie viele Strangplätze es gibt, Leitstrang eingeschlossen.
    #[must_use]
    pub fn plaetze(&self) -> usize {
        self.straenge.len()
    }

    /// Die Adresse eines Strangzählers — für die Prüfung der Ausrichtung.
    #[must_use]
    pub fn adresse(&self, platz: usize) -> usize {
        std::ptr::from_ref(&self.straenge[platz]) as usize
    }

    /// Liest alle Zähler und den Laufzustand.
    #[must_use]
    pub fn momentaufnahme(&self) -> Momentaufnahme {
        todo!("#933 {self:?}")
    }

    /// Setzt die Arbeitsmenge des Laufs.
    pub fn setze_menge(&self, menge: Arbeitsmenge) {
        todo!("#933 {menge:?}")
    }

    /// Nanosekunden seit dem Anlegen.
    #[must_use]
    pub fn elapsed_ns(&self) -> u64 {
        u64::try_from(self.beginn.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

impl Strang {
    /// **Der heiße Pfad:** ein gerechneter Kampf.
    #[inline]
    pub fn kampf(&self) {
        todo!("#933 {self:?}")
    }

    /// Der Strang für den Arbeitsbereich `nummer` desselben Zählwerks.
    #[must_use]
    pub fn arbeiter(&self, nummer: usize) -> Self {
        todo!("#933 {nummer}")
    }

    /// Meldet diesen Strang für den laufenden Strang an.
    pub fn anmelden(self) -> Anmeldung {
        todo!("#933 {self:?}")
    }

    /// Der Platz im Zählwerk.
    #[must_use]
    pub const fn platz(&self) -> usize {
        self.platz
    }
}

impl Drop for Anmeldung {
    fn drop(&mut self) {
        let vorher = self.vorher.take();
        ANGEMELDET.with(|a| *a.borrow_mut() = vorher);
    }
}

/// Der angemeldete Strang des laufenden Strangs — einmal je Bereich holen,
/// nicht je Kampf.
#[must_use]
pub fn strang() -> Option<Strang> {
    ANGEMELDET.with(|a| a.borrow().clone())
}

/// Eine abgeschlossene Auswertung auf `weg`, die `durchgaenge` Durchgänge
/// geliefert hat.
pub fn kandidat(weg: Weg, durchgaenge: u64) {
    todo!("#933 {weg:?} {durchgaenge}")
}

/// Setzt die Arbeitsmenge des angemeldeten Laufs.
pub fn menge(menge: Arbeitsmenge) {
    todo!("#933 {menge:?}")
}

/// Eine Runde der Suche beginnt.
///
/// `geplant` sind die Auswertungen, die diese Runde sicher kostet;
/// `rest_hoechstens` die obere Schranke für alle übrigen bis zum Ende der
/// Suche, diese Runde eingeschlossen.
pub fn suchrunde(runde: u64, geplant: u64, rest_hoechstens: u64) {
    todo!("#933 {runde} {geplant} {rest_hoechstens}")
}

/// Die laufende Runde kostet `zusatz` Auswertungen mehr als geplant.
pub fn runde_ergaenzt(zusatz: u64) {
    todo!("#933 {zusatz}")
}

/// Die Suche hat ein neues bestes Ergebnis.
pub fn bestes(wert: f64) {
    todo!("#933 {wert}")
}

/// Dieser Strang trägt nur `AtomicU64`, und `Ordering` braucht er hier nicht
/// selbst — die Bindung hält den Import sichtbar, bis die Umsetzung steht.
#[allow(dead_code)]
const _ORDNUNG: Ordering = Ordering::Relaxed;
