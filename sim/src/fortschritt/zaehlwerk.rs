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
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use crate::engpass::{Kampfzaehlung, Rohstand, Strangzeit};

/// Wie weit zwei Strangzähler mindestens auseinanderliegen, in Bytes.
pub const CACHE_ZEILE: usize = 128;

/// Die Auswertungswege, die es heute gibt.
///
/// **Genau vier, und keiner ist erfunden.** Der Plan nennt in seiner
/// Beispielzeile Cache, analytisch, Ledger und Suffix — seit #920 steht
/// `analytisch` in einem Lauf, die übrigen nicht (siehe [`WEGE_OHNE_LAUF`]).
/// Sie hier mit 0 % zu führen hieße, „der Cache trifft nie" zu behaupten, wo
/// es gar keinen gibt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weg {
    /// Ein Kampf entscheidet: Nirgends wird gewürfelt, sein Ausgang ist exakt
    /// (#908 im festen Lauf, `EinDurchgang` im gestaffelten).
    EinKampf,
    /// Die feste Stichprobe: `iterations` Kämpfe, gemittelt.
    Stichprobe,
    /// Die gestaffelte Stichprobe (#915, AC3): Kämpfe, bis die Aussage sicher ist.
    Gestaffelt,
    /// Der Krit ausintegriert (#920): zwei Randläufe statt einer Stichprobe,
    /// wo die Abhängigkeitsmaske keinen verzweigenden Leser findet.
    Analytisch,
}

impl Weg {
    /// Wie viele Wege es gibt — die Länge der Zählerfelder.
    pub const ANZAHL: usize = 4;

    /// Alle Wege in fester Reihenfolge.
    pub const ALLE: [Self; Self::ANZAHL] = [
        Self::EinKampf,
        Self::Stichprobe,
        Self::Gestaffelt,
        Self::Analytisch,
    ];

    /// Der Platz im Zählerfeld.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::EinKampf => 0,
            Self::Stichprobe => 1,
            Self::Gestaffelt => 2,
            Self::Analytisch => 3,
        }
    }

    /// Der Schlüssel im JSON-Strom — stabil, unabhängig von `Debug`.
    #[must_use]
    pub const fn schluessel(self) -> &'static str {
        match self {
            Self::EinKampf => "one_fight",
            Self::Stichprobe => "fixed_sample",
            Self::Gestaffelt => "sequential",
            Self::Analytisch => "analytic",
        }
    }

    /// Der Name in der Zeile für Menschen.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::EinKampf => "ein Kampf",
            Self::Stichprobe => "Stichprobe",
            Self::Gestaffelt => "gestaffelt",
            Self::Analytisch => "analytisch",
        }
    }
}

/// Die Auswertungswege des Plans, die heute in keinem Lauf stehen (E11.4b).
///
/// Sie stehen in beiden Ausgaben **mit Namen**, nicht als 0 %: Wer die Zeile
/// liest, soll sehen, dass es sie nicht gibt — nicht, dass sie nie greifen.
/// `analytic` stand bis #920 hier: [`crate::analytisch`] existierte als
/// Funktion, aber kein Lauf rief sie. Seit die Durchgangswahl ihn wählt, ist
/// er [`Weg::Analytisch`].
pub const WEGE_OHNE_LAUF: [&str; 4] = ["cache", "ledger", "suffix", "avx2"];

/// Die Zähler eines Strangs, auf einer eigenen Cache-Zeile.
///
/// Nur `kaempfe` liegt im heißen Pfad. Die übrigen Felder ändern sich einmal je
/// Auswertung, und auch das nur im Strang, der die Auswertung angestoßen hat.
///
/// Die Ausrichtung prüft `tests/fortschritt.rs` zur Laufzeit und nicht als
/// Übersetzungsbedingung: Die Gegenprobe — `align(128)` durch `align(8)`
/// ersetzen — soll dort beim Namen rot werden, nicht als Baufehler ohne
/// Prüffall.
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
    /// Nanosekunden, die dieser Platz in parallelen Abschnitten gerechnet hat
    /// (#934). Geschrieben vom Leitstrang, einmal je Abschnitt.
    arbeit_ns: AtomicU64,
    /// Nanosekunden, die dieser Platz in parallelen Abschnitten auf die
    /// anderen gewartet hat (#934).
    warten_ns: AtomicU64,
    /// Wie oft eine Auswertung dieses Strangs geprüft hat, ob ein Kampf
    /// genügt (#908, #934).
    reduktion_versuche: AtomicU64,
}

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
    /// Die zuletzt veröffentlichten Kampfzähler aus [`crate::diagnostics`]
    /// (#934). Getrennt von [`Laufstand`]: Der ist `Copy` und wird von
    /// Prüffällen als Ganzes gebaut; dieser Wert gehört der Erhebung.
    kampf: Mutex<Option<Kampfzaehlung>>,
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
    /// Was die Engpasserkennung braucht (#934). Das Zählwerk füllt seinen
    /// Teil; Leistungszähler, CPU-Zeit und Druck trägt der Anzeigestrang im
    /// selben Takt nach ([`crate::engpass::Erhebung::ergaenzen`]).
    pub engpass: Rohstand,
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
    ///
    /// Mindestens ein Arbeitsplatz entsteht immer: Ein Lauf mit `threads: 0`
    /// rechnet wie einer mit einem Strang ([`crate::balance`] nimmt
    /// `threads.max(1)`), und dann darf ihm kein Platz fehlen.
    #[must_use]
    pub fn neu(arbeitsstraenge: usize) -> Arc<Self> {
        let plaetze = arbeitsstraenge.max(1) + 1;
        Arc::new(Self {
            beginn: Instant::now(),
            straenge: (0..plaetze).map(|_| Strangzaehler::default()).collect(),
            zustand: Mutex::new(Laufstand::default()),
            kampf: Mutex::new(None),
        })
    }

    /// Meldet den laufenden Strang als Leitstrang an (Platz 0).
    pub fn anmelden(self: &Arc<Self>) -> Anmeldung {
        Strang {
            werk: Arc::clone(self),
            platz: 0,
        }
        .anmelden()
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
    ///
    /// Die Zähler werden einzeln gelesen, nicht unter einer Sperre — das ist
    /// der Preis dafür, dass der heiße Pfad keine anfasst. Zwischen dem ersten
    /// und dem letzten Platz können ein paar Kämpfe fertig werden; jede Zahl
    /// für sich ist trotzdem ein Stand, den es gegeben hat.
    #[must_use]
    pub fn momentaufnahme(&self) -> Momentaufnahme {
        let elapsed_ns = self.elapsed_ns();
        let mut kaempfe = 0;
        let mut durchgaenge = 0;
        let mut kandidaten = [0; Weg::ANZAHL];
        let mut kaempfe_je_strang = Vec::with_capacity(self.straenge.len());
        let mut engpass = Rohstand {
            straenge: Vec::with_capacity(self.straenge.len() - 1),
            ..Rohstand::default()
        };
        for (platz, z) in self.straenge.iter().enumerate() {
            let k = z.kaempfe.load(Ordering::Relaxed);
            kaempfe_je_strang.push(k);
            kaempfe += k;
            durchgaenge += z.durchgaenge.load(Ordering::Relaxed);
            for (summe, zaehler) in kandidaten.iter_mut().zip(&z.kandidaten) {
                *summe += zaehler.load(Ordering::Relaxed);
            }
            engpass.reduktion_versuche += z.reduktion_versuche.load(Ordering::Relaxed);
            // Der Leitstrang rechnet in keinem parallelen Abschnitt mit; er
            // startet ihn und wartet auf sein Ende. Seine Zeit dort wäre
            // Warten, das kein Paket verkürzt.
            if platz > 0 {
                engpass.straenge.push(Strangzeit {
                    arbeit_ns: z.arbeit_ns.load(Ordering::Relaxed),
                    warten_ns: z.warten_ns.load(Ordering::Relaxed),
                });
            }
        }
        engpass.kampf = *self.kampf.lock().unwrap_or_else(PoisonError::into_inner);
        Momentaufnahme {
            elapsed_ns,
            kaempfe,
            durchgaenge,
            kandidaten,
            kaempfe_je_strang,
            stand: *self.stand(),
            engpass,
        }
    }

    /// Setzt die Arbeitsmenge des Laufs.
    pub fn setze_menge(&self, menge: Arbeitsmenge) {
        self.stand().menge = menge;
    }

    /// Nanosekunden seit dem Anlegen.
    #[must_use]
    pub fn elapsed_ns(&self) -> u64 {
        u64::try_from(self.beginn.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    /// Abgeschlossene Auswertungen über alle Plätze.
    fn kandidaten_gesamt(&self) -> u64 {
        self.straenge
            .iter()
            .flat_map(|z| z.kandidaten.iter())
            .map(|k| k.load(Ordering::Relaxed))
            .sum()
    }

    /// Der Laufzustand, auch wenn ein anderer Strang beim Schreiben abstürzte.
    ///
    /// Eine vergiftete Sperre heißt hier nur, dass ein Strang mitten in einer
    /// Zuweisung von `Copy`-Feldern starb. Der Wert ist dann der alte oder der
    /// neue, nie ein halber — und die Anzeige soll den Absturz melden können,
    /// statt selbst daran zu sterben.
    fn stand(&self) -> std::sync::MutexGuard<'_, Laufstand> {
        self.zustand.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Strang {
    /// **Der heiße Pfad:** ein gerechneter Kampf.
    ///
    /// `Relaxed`, weil nichts anderes an diesem Wert hängt: Der Anzeigestrang
    /// liest ihn als Zahl und folgert daraus nichts über andere Speicherstellen.
    #[inline]
    pub fn kampf(&self) {
        self.werk.straenge[self.platz]
            .kaempfe
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Der Strang für den Arbeitsbereich `nummer` desselben Zählwerks.
    ///
    /// Gibt es mehr Bereiche als Plätze, teilen sich die überzähligen den
    /// letzten. Das bleibt richtig — jede Erhöhung ist atomar —, kostet dann
    /// aber genau die geteilte Zeile, die AC9 verbietet. Wer ein Zählwerk
    /// anlegt, gibt ihm deshalb die Strangzahl des Laufs.
    #[must_use]
    pub fn arbeiter(&self, nummer: usize) -> Self {
        Self {
            werk: Arc::clone(&self.werk),
            platz: (nummer + 1).min(self.werk.straenge.len() - 1),
        }
    }

    /// Trägt für den Arbeitsbereich `nummer` ein, wie lange er in einem
    /// parallelen Abschnitt gerechnet und gewartet hat (#934).
    ///
    /// Gerufen vom Leitstrang, nachdem der Abschnitt fertig ist — einmal je
    /// Abschnitt und Bereich, nicht je Kampf. Der Platz ist derselbe, den
    /// [`Strang::arbeiter`] dem Bereich gibt.
    pub fn abschnitt(&self, nummer: usize, arbeit_ns: u64, warten_ns: u64) {
        let z = &self.werk.straenge[self.arbeiter(nummer).platz];
        z.arbeit_ns.fetch_add(arbeit_ns, Ordering::Relaxed);
        z.warten_ns.fetch_add(warten_ns, Ordering::Relaxed);
    }

    /// Meldet diesen Strang für den laufenden Strang an.
    pub fn anmelden(self) -> Anmeldung {
        let vorher = ANGEMELDET.with(|a| a.borrow_mut().replace(self));
        Anmeldung { vorher }
    }

    /// Der Platz im Zählwerk.
    #[must_use]
    pub const fn platz(&self) -> usize {
        self.platz
    }

    /// Das Zählwerk dieses Strangs.
    #[must_use]
    pub const fn zaehlwerk(&self) -> &Arc<Zaehlwerk> {
        &self.werk
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

/// Ruft `f` mit dem Strang des laufenden Strangs, falls er angemeldet ist.
///
/// Ohne Anmeldung geschieht nichts — dieselbe Haltung wie bei
/// [`crate::diagnostics`]: Wer nicht misst, zahlt einen Vergleich.
fn mit_strang(f: impl FnOnce(&Strang)) {
    ANGEMELDET.with(|a| {
        if let Some(s) = &*a.borrow() {
            f(s);
        }
    });
}

/// Eine abgeschlossene Auswertung auf `weg`, die `durchgaenge` Durchgänge
/// geliefert hat.
///
/// Ist im laufenden Strang die Erfassung aus [`crate::diagnostics`] an,
/// veröffentlicht derselbe Aufruf ihre Kennzahlen für die Anzeige (#934):
/// Die Zähler sind strang-lokal, der Anzeigestrang sähe sie sonst nie. Einmal
/// je Auswertung, ohne das Histogramm zu kopieren.
pub fn kandidat(weg: Weg, durchgaenge: u64) {
    mit_strang(|s| {
        let z = &s.werk.straenge[s.platz];
        z.kandidaten[weg.index()].fetch_add(1, Ordering::Relaxed);
        z.durchgaenge.fetch_add(durchgaenge, Ordering::Relaxed);
        if crate::diagnostics::is_enabled() {
            *s.werk.kampf.lock().unwrap_or_else(PoisonError::into_inner) =
                Some(crate::diagnostics::kampfzaehlung());
        }
    });
}

/// Eine Auswertung prüft, ob **ein** Kampf sie entscheidet (#908) — der
/// Nenner der Trefferquote dieses Wegs (#934). Ob sie es tut, zählt
/// [`kandidat`] mit dem Weg [`Weg::EinKampf`].
pub fn reduktionsversuch() {
    mit_strang(|s| {
        s.werk.straenge[s.platz]
            .reduktion_versuche
            .fetch_add(1, Ordering::Relaxed);
    });
}

/// Setzt die Arbeitsmenge des angemeldeten Laufs.
pub fn menge(menge: Arbeitsmenge) {
    mit_strang(|s| s.werk.setze_menge(menge));
}

/// Eine Runde der Suche beginnt.
///
/// `geplant` sind die Auswertungen, die diese Runde sicher kostet;
/// `rest_mindestens` und `rest_hoechstens` die Schranken für alle übrigen bis
/// zum Ende der Suche, diese Runde eingeschlossen. Daraus wird die absolute
/// [`Arbeitsmenge::Hoechstens`] — gegen die Zahl, die in diesem Augenblick
/// abgeschlossen ist.
pub fn suchrunde(runde: u64, geplant: u64, rest_mindestens: u64, rest_hoechstens: u64) {
    mit_strang(|s| {
        let fertig = s.werk.kandidaten_gesamt();
        let mut stand = s.werk.stand();
        stand.runde = Some(runde);
        stand.runde_ab_kandidaten = fertig;
        stand.runde_geplant = geplant;
        stand.menge = Arbeitsmenge::Hoechstens {
            mindestens: fertig + rest_mindestens,
            hoechstens: fertig + rest_hoechstens,
        };
    });
}

/// Die laufende Runde kostet `zusatz` Auswertungen mehr als geplant.
///
/// Die Untergrenze wächst mit: Was jetzt begonnen hat, wird auch gemessen. Die
/// Obergrenze bleibt — sie enthielt diese Auswertungen schon.
pub fn runde_ergaenzt(zusatz: u64) {
    mit_strang(|s| {
        let mut stand = s.werk.stand();
        stand.runde_geplant += zusatz;
        if let Arbeitsmenge::Hoechstens {
            mindestens,
            hoechstens,
        } = stand.menge
        {
            stand.menge = Arbeitsmenge::Hoechstens {
                mindestens: (mindestens + zusatz).min(hoechstens),
                hoechstens,
            };
        }
    });
}

/// Die Messung nach der letzten Runde beginnt — die der gefundenen Liste.
///
/// Sie gehört zur letzten Runde: Die Runde zählt bis zum Ende der Suche, und
/// diese Messung wird ihr zugerechnet. Bis zu diesem Aufruf stand sie nur in
/// den Grenzen der Menge (dort als `+ 1`, siehe `melde_runde`), nicht im
/// Rundenplan; die end-Meldung zeigte deshalb eine Auswertung mehr, als die
/// Runde geplant hatte (Befund am 2026-10-28: 361 von 360). Die Grenzen bleiben
/// unberührt — sie enthielten diese Messung schon.
pub fn schlussmessung() {
    mit_strang(|s| {
        let mut stand = s.werk.stand();
        if stand.runde.is_some() {
            stand.runde_geplant += 1;
        }
    });
}

/// Die Suche hat ein neues bestes Ergebnis.
///
/// Derselbe Wert ein zweites Mal setzt die Uhr **nicht** zurück: „seit wann
/// unverändert" soll die Zeit seit der letzten Verbesserung sein, nicht seit
/// dem letzten Aufruf.
pub fn bestes(wert: f64) {
    mit_strang(|s| {
        let fertig = s.werk.kandidaten_gesamt();
        let jetzt = s.werk.elapsed_ns();
        let mut stand = s.werk.stand();
        if stand
            .bestes
            .is_some_and(|b| b.wert.to_bits() == wert.to_bits())
        {
            return;
        }
        stand.bestes = Some(Bestes {
            wert,
            seit_ns: jetzt,
            bei_kandidaten: fertig,
        });
    });
}
