//! Ergebnis und Bewertung getrennt speichern (Issue #913).
//!
//! # Warum drei Speicher und nicht einer
//!
//! Die Zielfunktion — wonach ein Ergebnis bewertet wird — ist ein
//! **versioniertes Feld des Laufvertrags**, kein fuer immer festgeschriebener
//! Wert. Ein Endgame-Boss darf eine andere Sicherheit verlangen als einer der
//! ersten Instanz, und eine Rangfolge, die sich im Spiel als falsch erweist,
//! wird geaendert.
//!
//! Speichert der Ergebnisspeicher eine Bewertung mit, kostet jeder Zielwechsel
//! **jede gerechnete Kampfstunde erneut**. Getrennt kostet er Mikrosekunden:
//!
//! ```text
//! Zielfunktion geaendert
//!    -> rohes Ergebnis bleibt vollstaendig gueltig
//!    -> nur die Bewertung neu rechnen
//!    -> Suche warm fortsetzen
//! ```
//!
//! # Was hier NICHT zugesichert wird
//!
//! **Nicht**, dass drei Speicher weniger Platz brauchen — sie brauchen mehr.
//! **Nicht**, dass jede Aenderung billig ist: Aendert sich die
//! **Kampfsemantik**, sind auch die rohen Ergebnisse ungueltig, und dann hilft
//! die Trennung nicht. Genau dafuer traegt der Ergebnisschluessel einen
//! Semantik-Fingerabdruck (siehe [`ErgebnisSchluessel`]).
//!
//! # Die Sperre gegen die stille Bewertung
//!
//! Ein Feld, das jemand spaeter hinzufuegt, weil es 'praktisch' ist, hebelt die
//! ganze Trennung aus — und niemand merkt es. Dagegen stehen **zwei**
//! Mechanismen, nicht einer:
//!
//! 1. [`RoherErgebnisvektor::felder_aufgezaehlt`] destrukturiert ohne `..` —
//!    ein neues Feld bricht den Bau.
//! 2. [`Zielfunktionsfrei`] traegt **kein** Typ, der die Zielfunktion kennt.
//!    Ein Feld, das sie braucht, bricht auch dann, wenn jemand die
//!    Destrukturierung brav mitpflegt.
//!
//! # Woher die Zahlen kommen
//!
//! Nicht aus einem Nachbau: [`crate::balance::BalanceResult::ergebnisentwurf`]
//! fuellt den Entwurf aus dem **vorhandenen** Ergebnissatz und destrukturiert
//! ihn dabei ebenfalls ohne `..`. Ein Modul ohne diese Speisung waere gebaut
//! und tot.

use core::fmt;

use crate::balance::Statistics;

// ---------------------------------------------------------------- Fingerabdruecke

/// Fingerabdruck der **Kampfsemantik**.
///
/// Er steht im Ergebnisschluessel, nicht im Wert. Aendert sich, wie ein Kampf
/// gerechnet wird, aendert sich der Schluessel — und alte Ergebnisse werden
/// nicht mehr gefunden, statt still weiterverwendet zu werden.
///
/// **Das ist der Unterschied zwischen 'die Trennung hilft' und 'die Trennung
/// luegt'.** Ohne ihn liefe die dritte Gegenprobe aus Issue #913 ins Leere:
/// Ein Lauf mit geaenderter Semantik muss ALLE Kaempfe neu rechnen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SemantikHash(pub u64);

/// Fingerabdruck der **Zielfunktion**.
///
/// # Warum ein Hash und keine Rangfolge
///
/// Die Wahl der Zielfunktion gehoert nicht in dieses Modul — Issue #913 macht
/// sie **billig aenderbar**, es trifft sie nicht. Hier steht deshalb nur ihre
/// Identitaet.
///
/// Dieser Typ traegt **absichtlich kein** [`Zielfunktionsfrei`]. Er ist der
/// Marker, an dem der Bau bricht, wenn jemand eine Bewertung in den
/// Ergebnisspeicher schmuggelt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZielfunktionsHash(pub u64);

impl fmt::Display for ZielfunktionsHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "zf:{:016x}", self.0)
    }
}

// ------------------------------------------------------------ Der Marker

/// Ein Typ, der **nichts** ueber die Zielfunktion weiss.
///
/// # Wofuer der Marker da ist
///
/// Er ist die zweite Haelfte der Sperre aus AC2. Die Destrukturierung allein
/// faengt nur **neue** Felder; sie sagt nichts darueber, ob ein Feld eine
/// Bewertung TRAEGT. Dieser Marker tut genau das: Wer ein Feld vom Typ
/// [`ZielfunktionsHash`] oder [`Bewertung`] in den rohen Vektor legt, bekommt
/// keinen uebersetzbaren Simulator.
///
/// **Bewusst nicht fuer `bool` implementiert.** Ein `bool` kann alles
/// bedeuten, auch 'war gut' — und genau das waere eine Bewertung im
/// Ergebnisspeicher. Wer einen Wahrheitswert braucht, nennt ihn beim Namen
/// (etwa [`Siegzahl`]).
pub trait Zielfunktionsfrei {}

impl Zielfunktionsfrei for u32 {}
impl Zielfunktionsfrei for u64 {}
// **Kein eigener Eintrag fuer `Millis`**: `time::Millis` ist ein Alias auf
// `u32` (`pub type Millis = u32`), kein eigener Typ. Eine zweite Umsetzung
// waere dieselbe und wird vom Uebersetzer als Konflikt gemeldet (E0119) —
// gemessen beim Bau, nicht vermutet.

/// Mittelwert und Streuung sagen, **was** passiert ist, nicht ob es gut war.
impl Zielfunktionsfrei for Statistics {}

// ---------------------------------------------------------- Der rohe Vektor

/// Wie oft der Boss gefallen ist.
///
/// Ein eigener Typ statt `u32`, damit an der Verwendungsstelle steht, was
/// gezaehlt wird — und damit niemand versucht ist, hier ein `bool` 'gewonnen'
/// abzulegen. Ob viele Siege gut sind, entscheidet die Zielfunktion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Siegzahl(pub u32);

impl Zielfunktionsfrei for Siegzahl {}

/// Wie oft jeder Zauber gewirkt wurde — die **Ereignisse** aus AC1.
///
/// Ein eigener Typ statt `Vec<u32>`, damit der Marker daran haengen kann und
/// damit der Name sagt, was die Zahlen bedeuten. Der Index ist der
/// Zauber-Index aus [`crate::data`].
///
/// **Zielfunktionsfrei**, obwohl es verlockend waere, hier eine 'Effizienz'
/// abzulegen: Wie oft etwas gewirkt wurde, ist eine Beobachtung. Ob das gut
/// war, ist eine Bewertung.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Wirkungszahlen(pub Vec<u32>);

impl Zielfunktionsfrei for Wirkungszahlen {}

impl Wirkungszahlen {
    /// Summe ueber alle Zauber.
    #[must_use]
    pub fn gesamt(&self) -> u64 {
        self.0.iter().map(|n| u64::from(*n)).sum()
    }
}

/// Das rohe Ergebnis eines Laufs — **ohne jede Bewertung**.
///
/// # Was hier NICHT hineingehoert
///
/// Auch ein 'Gesamtwert' oder ein 'Rang' ist bereits eine Bewertung. Der
/// Speicher enthaelt, **was im Kampf passiert ist** — und nichts darueber, ob
/// das gut war. Wer eine Zahl braucht, die zwei Ergebnisse vergleicht, baut
/// sie aus diesen Feldern **beim Bewerten**, nicht hier.
///
/// Ebenfalls nicht hier: Beweismittel abgebrochener Kaempfe. Sie sind ein
/// vierter Speicher und gehoeren in ein eigenes Issue — festgehalten ist nur,
/// dass sie **nicht** hierher gehoeren.
///
/// # Warum genau diese zwoelf Felder
///
/// Sie sind die Abbildung des **vorhandenen** Ergebnissatzes
/// ([`crate::balance::BalanceResult`]) ohne dessen Laufmetadaten. Erfunden ist
/// hier nichts; was der Simulator heute misst, steht hier und sonst nirgends
/// noch einmal.
#[derive(Debug, Clone, PartialEq)]
pub struct RoherErgebnisvektor {
    /// Ausgeteilter Schaden.
    pub schaden: Statistics,
    /// **Durchgekommener** Schaden am Spieler — ohne das Abgefangene.
    pub erlittener_schaden: Statistics,
    /// Was der Boss insgesamt geschickt hat: durchgekommen PLUS abgefangen.
    pub eingehender_schaden: Statistics,
    /// Von der Schutzschicht abgefangener Schaden.
    pub absorption: Statistics,
    /// Zurueckgeholtes Leben des Spielers.
    pub heilung: Statistics,
    /// Vom BOSS zurueckgedrehter Fortschritt — **nicht** die Spielerheilung.
    pub boss_heilung: Statistics,
    /// Zeit bis zum Sieg, nur ueber die gewonnenen Durchgaenge.
    pub siegzeit_ms: Statistics,
    /// Zeit bis zum Tod des Spielers, nur ueber die verlorenen Durchgaenge.
    pub todeszeit_ms: Statistics,
    /// Wie viele Durchgaenge gerechnet wurden.
    pub durchgaenge: u32,
    /// Wie oft der Boss gefallen ist.
    pub siege: Siegzahl,
    /// Wie oft der Spieler gestorben ist.
    pub tode: u32,
    /// Wirkungen je Zauber — die Ereignisse.
    pub wirkungen: Wirkungszahlen,
}

/// Wie viele Felder das Ergebnisschema hat.
///
/// Steht als Zahl da, damit ein Pruefall sie festnageln kann. Weicht sie von
/// der Laenge aus [`RoherErgebnisvektor::felder_aufgezaehlt`] ab, ist die
/// Aufzaehlung unvollstaendig — und dann haette der Bau brechen muessen.
pub const FELDER_IM_SCHEMA: usize = 12;

impl RoherErgebnisvektor {
    /// Jedes Feld einzeln — **und hier bricht der Bau** (AC2).
    ///
    /// Die Destrukturierung nennt jedes Feld und benutzt **kein** `..`; ein
    /// neues Feld meldet `E0027`. Zusaetzlich laeuft jedes Feld durch
    /// `nur_zielfunktionsfrei`, das [`Zielfunktionsfrei`] verlangt — ein Feld,
    /// das die Zielfunktion kennt, bricht dort.
    ///
    /// **Beide Mechanismen zusammen**, weil jeder allein eine Luecke laesst:
    /// Die Destrukturierung faengt nur neue Felder, der Marker nur falsche
    /// Typen. Wer ein neues Feld hinzufuegt UND die Destrukturierung pflegt,
    /// laeuft in den zweiten; wer ein vorhandenes Feld auf einen bewertenden
    /// Typ umstellt, ebenfalls.
    ///
    /// Die zurueckgegebene Zahl je Feld ist der Mittelwert beziehungsweise die
    /// Zahl selbst; fuer die Wirkungen ihre Summe. Sie dient dem Pruefstand,
    /// nicht der Rechnung — der Speicher gibt den vollen Vektor heraus.
    #[must_use]
    pub fn felder_aufgezaehlt(&self) -> Vec<(&'static str, f64)> {
        fn nur_zielfunktionsfrei<T: Zielfunktionsfrei>(_wert: &T) {}

        let RoherErgebnisvektor {
            schaden,
            erlittener_schaden,
            eingehender_schaden,
            absorption,
            heilung,
            boss_heilung,
            siegzeit_ms,
            todeszeit_ms,
            durchgaenge,
            siege,
            tode,
            wirkungen,
        } = self;

        nur_zielfunktionsfrei(schaden);
        nur_zielfunktionsfrei(erlittener_schaden);
        nur_zielfunktionsfrei(eingehender_schaden);
        nur_zielfunktionsfrei(absorption);
        nur_zielfunktionsfrei(heilung);
        nur_zielfunktionsfrei(boss_heilung);
        nur_zielfunktionsfrei(siegzeit_ms);
        nur_zielfunktionsfrei(todeszeit_ms);
        nur_zielfunktionsfrei(durchgaenge);
        nur_zielfunktionsfrei(siege);
        nur_zielfunktionsfrei(tode);
        nur_zielfunktionsfrei(wirkungen);

        vec![
            ("schaden", schaden.mean),
            ("erlittener_schaden", erlittener_schaden.mean),
            ("eingehender_schaden", eingehender_schaden.mean),
            ("absorption", absorption.mean),
            ("heilung", heilung.mean),
            ("boss_heilung", boss_heilung.mean),
            ("siegzeit_ms", siegzeit_ms.mean),
            ("todeszeit_ms", todeszeit_ms.mean),
            ("durchgaenge", f64::from(*durchgaenge)),
            ("siege", f64::from(siege.0)),
            ("tode", f64::from(*tode)),
            ("wirkungen_gesamt", wirkungen.gesamt() as f64),
        ]
    }
}

// ------------------------------------------------------------- Der Entwurf

/// Welche Felder einem Entwurf fehlen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unvollstaendig {
    /// Die Namen der fehlenden Felder, in der Reihenfolge des Schemas.
    pub fehlend: Vec<&'static str>,
}

impl fmt::Display for Unvollstaendig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unvollstaendiger Ergebnisvektor, es fehlen: {:?}",
            self.fehlend
        )
    }
}

impl std::error::Error for Unvollstaendig {}

/// Ein Ergebnis im Aufbau — noch nicht speicherbar.
///
/// # Warum es diesen Zwischenschritt gibt (AC6)
///
/// Ein Kampf ohne vollstaendigen Vektor ist ein **Torso**. Ein spaeterer Lauf
/// mit anderer Zielfunktion faende ihn im Speicher und hielte ihn fuer ein
/// Ergebnis — der Fehler waere still und nicht mehr auffindbar.
///
/// Die Sperre steht **bevor** der erste Abbruch sie treffen kann: Einen
/// Kampfabbruch gibt es heute nicht, er entsteht erst mit den frueher
/// greifenden Schranken. Wer sie erst dann baut, baut sie einmal zu spaet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ergebnisentwurf {
    /// Ausgeteilter Schaden.
    pub schaden: Option<Statistics>,
    /// Durchgekommener Schaden.
    pub erlittener_schaden: Option<Statistics>,
    /// Durchgekommener plus abgefangener Schaden.
    pub eingehender_schaden: Option<Statistics>,
    /// Abgefangener Schaden.
    pub absorption: Option<Statistics>,
    /// Zurueckgeholtes Leben.
    pub heilung: Option<Statistics>,
    /// Vom Boss zurueckgedrehter Fortschritt.
    pub boss_heilung: Option<Statistics>,
    /// Zeit bis zum Sieg.
    pub siegzeit_ms: Option<Statistics>,
    /// Zeit bis zum Tod.
    pub todeszeit_ms: Option<Statistics>,
    /// Zahl der Durchgaenge.
    pub durchgaenge: Option<u32>,
    /// Zahl der Siege.
    pub siege: Option<Siegzahl>,
    /// Zahl der Tode.
    pub tode: Option<u32>,
    /// Wirkungen je Zauber.
    pub wirkungen: Option<Wirkungszahlen>,
}

impl Ergebnisentwurf {
    /// Schliesst den Entwurf ab — oder nennt, was fehlt.
    ///
    /// Die Reihenfolge der Pruefung ist die des Schemas, damit die Meldung
    /// stabil ist und sich vergleichen laesst. Es wird **jedes** Feld geprueft
    /// und nicht beim ersten fehlenden abgebrochen: Wer einen Torso repariert,
    /// will alle Luecken auf einmal sehen.
    ///
    /// # Errors
    ///
    /// [`Unvollstaendig`] mit den Namen der fehlenden Felder.
    pub fn abschliessen(self) -> Result<RoherErgebnisvektor, Unvollstaendig> {
        let mut fehlend: Vec<&'static str> = Vec::new();
        if self.schaden.is_none() {
            fehlend.push("schaden");
        }
        if self.erlittener_schaden.is_none() {
            fehlend.push("erlittener_schaden");
        }
        if self.eingehender_schaden.is_none() {
            fehlend.push("eingehender_schaden");
        }
        if self.absorption.is_none() {
            fehlend.push("absorption");
        }
        if self.heilung.is_none() {
            fehlend.push("heilung");
        }
        if self.boss_heilung.is_none() {
            fehlend.push("boss_heilung");
        }
        if self.siegzeit_ms.is_none() {
            fehlend.push("siegzeit_ms");
        }
        if self.todeszeit_ms.is_none() {
            fehlend.push("todeszeit_ms");
        }
        if self.durchgaenge.is_none() {
            fehlend.push("durchgaenge");
        }
        if self.siege.is_none() {
            fehlend.push("siege");
        }
        if self.tode.is_none() {
            fehlend.push("tode");
        }
        if self.wirkungen.is_none() {
            fehlend.push("wirkungen");
        }
        if !fehlend.is_empty() {
            return Err(Unvollstaendig { fehlend });
        }
        Ok(RoherErgebnisvektor {
            schaden: self.schaden.expect("oben geprueft"),
            erlittener_schaden: self.erlittener_schaden.expect("oben geprueft"),
            eingehender_schaden: self.eingehender_schaden.expect("oben geprueft"),
            absorption: self.absorption.expect("oben geprueft"),
            heilung: self.heilung.expect("oben geprueft"),
            boss_heilung: self.boss_heilung.expect("oben geprueft"),
            siegzeit_ms: self.siegzeit_ms.expect("oben geprueft"),
            todeszeit_ms: self.todeszeit_ms.expect("oben geprueft"),
            durchgaenge: self.durchgaenge.expect("oben geprueft"),
            siege: self.siege.expect("oben geprueft"),
            tode: self.tode.expect("oben geprueft"),
            wirkungen: self.wirkungen.expect("oben geprueft"),
        })
    }
}

// ---------------------------------------------------------- Ergebnisspeicher

/// Woruber ein Ergebnis eindeutig bestimmt ist.
///
/// **Der Semantik-Hash gehoert in den SCHLUESSEL**, nicht in den Wert: Aendert
/// sich, wie gerechnet wird, findet eine Abfrage das alte Ergebnis gar nicht
/// erst — statt es zurueckzugeben und dabei zu luegen.
///
/// Die Zielfunktion steht hier **nicht**. Sie hat mit dem Ergebnis nichts zu
/// tun, nur mit seiner Bewertung. Stuende sie hier, entstuende je Zielfunktion
/// ein eigener Ergebnissatz — und ein Zielwechsel koestete wieder jeden Kampf.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ErgebnisSchluessel {
    /// Welcher Aufbau gerechnet wurde (Rotation, Ausruestung, Talente).
    pub aufbau: String,
    /// Gegen welchen Boss.
    pub boss: String,
    /// Mit welcher Kampfsemantik.
    pub semantik: SemantikHash,
}

/// Die rohen Ergebnisse, ohne jede Bewertung.
///
/// # Der Kampfzaehler
///
/// [`Ergebnisspeicher::gerechnete_kaempfe`] zaehlt, wie oft tatsaechlich
/// gerechnet wurde. Er ist die Messgroesse fuer AC4: Ein Zielwechsel darf ihn
/// **nicht** bewegen — ein Lauf mit geaenderter Kampfsemantik dagegen schon.
#[derive(Debug, Default)]
pub struct Ergebnisspeicher {
    eintraege: Vec<(ErgebnisSchluessel, RoherErgebnisvektor)>,
    gerechnete_kaempfe: u64,
    abgelehnte: u64,
}

impl Ergebnisspeicher {
    /// Ein leerer Speicher.
    #[must_use]
    pub fn neu() -> Self {
        Self::default()
    }

    /// Wie viele Kaempfe seit dem Anlegen wirklich gerechnet wurden.
    #[must_use]
    pub const fn gerechnete_kaempfe(&self) -> u64 {
        self.gerechnete_kaempfe
    }

    /// Wie viele Entwuerfe als unvollstaendig abgelehnt wurden.
    ///
    /// Steht getrennt, damit 'nichts gespeichert' und 'alles abgelehnt'
    /// unterscheidbar bleiben.
    #[must_use]
    pub const fn abgelehnte(&self) -> u64 {
        self.abgelehnte
    }

    /// Wie viele Ergebnisse liegen.
    #[must_use]
    pub fn anzahl(&self) -> usize {
        self.eintraege.len()
    }

    /// Legt ein abgeschlossenes Ergebnis ab.
    ///
    /// Nimmt einen **Entwurf** und lehnt ihn ab, wenn er unvollstaendig ist
    /// (AC6). Ein Torso im Speicher waere spaeter nicht mehr als solcher
    /// erkennbar.
    ///
    /// # Errors
    ///
    /// [`Unvollstaendig`], wenn dem Entwurf ein Feld fehlt. Dann wird
    /// **nichts** abgelegt.
    pub fn speichere(
        &mut self,
        schluessel: ErgebnisSchluessel,
        entwurf: Ergebnisentwurf,
    ) -> Result<(), Unvollstaendig> {
        match entwurf.abschliessen() {
            Err(fehler) => {
                self.abgelehnte += 1;
                Err(fehler)
            }
            Ok(vektor) => {
                match self.eintraege.iter_mut().find(|(k, _)| *k == schluessel) {
                    Some(vorhanden) => vorhanden.1 = vektor,
                    None => self.eintraege.push((schluessel, vektor)),
                }
                Ok(())
            }
        }
    }

    /// Holt ein Ergebnis, falls es zu genau diesem Schluessel vorliegt.
    #[must_use]
    pub fn hole(&self, schluessel: &ErgebnisSchluessel) -> Option<&RoherErgebnisvektor> {
        self.eintraege
            .iter()
            .find(|(k, _)| k == schluessel)
            .map(|(_, v)| v)
    }

    /// Holt ein Ergebnis oder rechnet es — und zaehlt nur im zweiten Fall.
    ///
    /// **Das ist die Stelle, an der AC4 gemessen wird.** Liegt das Ergebnis
    /// vor, wird `rechne` nicht aufgerufen und der Zaehler bewegt sich nicht.
    ///
    /// # Errors
    ///
    /// [`Unvollstaendig`], wenn `rechne` einen Torso liefert.
    pub fn hole_oder_rechne(
        &mut self,
        schluessel: ErgebnisSchluessel,
        rechne: impl FnOnce() -> Ergebnisentwurf,
    ) -> Result<RoherErgebnisvektor, Unvollstaendig> {
        if let Some(vorhanden) = self.hole(&schluessel) {
            return Ok(vorhanden.clone());
        }
        self.gerechnete_kaempfe += 1;
        let entwurf = rechne();
        let vektor = entwurf.abschliessen().inspect_err(|_| {
            self.abgelehnte += 1;
        })?;
        self.eintraege.push((schluessel, vektor.clone()));
        Ok(vektor)
    }

    /// Alle Schluessel, in Einfuegereihenfolge.
    ///
    /// Keine Hash-Abbildung: Die Reihenfolge ist Teil des Determinismus
    /// (GAME-DESIGN 18.3).
    pub fn schluessel(&self) -> impl Iterator<Item = &ErgebnisSchluessel> {
        self.eintraege.iter().map(|(k, _)| k)
    }
}

// --------------------------------------------------------- Bewertungsspeicher

/// Was eine Zielfunktion aus einem rohen Ergebnis macht.
///
/// Traegt **absichtlich kein** [`Zielfunktionsfrei`]: Dieser Typ darf nie in
/// den Ergebnisspeicher geraten.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bewertung(pub f64);

/// Woruber eine Bewertung eindeutig bestimmt ist.
///
/// **Der Zielfunktions-Hash steht im SCHLUESSEL, nicht im Wert** (AC3). Stuende
/// er im Wert, lieferte eine Abfrage die Bewertung der VORIGEN Zielfunktion
/// zurueck — und der Fehler waere still, weil die Zahl plausibel aussieht.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BewertungsSchluessel {
    /// Welches Ergebnis bewertet wurde.
    pub ergebnis: ErgebnisSchluessel,
    /// Nach welcher Zielfunktion.
    pub zielfunktion: ZielfunktionsHash,
}

/// Die Bewertungen, je Zielfunktion getrennt.
#[derive(Debug, Default)]
pub struct Bewertungsspeicher {
    eintraege: Vec<(BewertungsSchluessel, Bewertung)>,
}

impl Bewertungsspeicher {
    /// Ein leerer Speicher.
    #[must_use]
    pub fn neu() -> Self {
        Self::default()
    }

    /// Legt eine Bewertung ab.
    pub fn setze(&mut self, schluessel: BewertungsSchluessel, wert: Bewertung) {
        match self.eintraege.iter_mut().find(|(k, _)| *k == schluessel) {
            Some(vorhanden) => vorhanden.1 = wert,
            None => self.eintraege.push((schluessel, wert)),
        }
    }

    /// Holt eine Bewertung — nur fuer **genau diese** Zielfunktion.
    #[must_use]
    pub fn hole(&self, schluessel: &BewertungsSchluessel) -> Option<Bewertung> {
        self.eintraege
            .iter()
            .find(|(k, _)| k == schluessel)
            .map(|(_, v)| *v)
    }

    /// Wie viele Bewertungen zu einer Zielfunktion vorliegen.
    #[must_use]
    pub fn anzahl_fuer(&self, zielfunktion: ZielfunktionsHash) -> usize {
        self.eintraege
            .iter()
            .filter(|(k, _)| k.zielfunktion == zielfunktion)
            .count()
    }

    /// Wie viele Bewertungen insgesamt vorliegen.
    #[must_use]
    pub fn anzahl(&self) -> usize {
        self.eintraege.len()
    }
}

// -------------------------------------------------------------- Sucharchiv

/// Was ein Zielwechsel mit einem Archiv-Teil macht.
///
/// # Die Entscheidung aus AC5, und warum sie so faellt
///
/// 'Unentschieden ist falsch' — deshalb steht sie im **Typ** und nicht in
/// einem Kommentar. Ein Pruefall kann eine Konstante festnageln; einen
/// Kommentar kann er nicht lesen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zielwechsel {
    /// Bleibt gueltig.
    Ueberlebt,
    /// Wird verworfen.
    Verworfen,
}

/// Welche Nischen des Suchraums schon erreicht wurden.
///
/// **Ueberlebt einen Zielwechsel.** Eine Nische ist durch das ERGEBNIS
/// definiert — welcher Bereich des Raums erreicht wurde —, nicht dadurch, ob
/// das Ergebnis gut war. Sie zu verwerfen hiesse, gerechnete Kaempfe
/// wegzuwerfen; genau das verhindert dieses Modul.
#[derive(Debug, Default)]
pub struct Nischenbelegung {
    belegt: Vec<String>,
}

impl Nischenbelegung {
    /// Wie ein Zielwechsel sie behandelt.
    pub const VERHALTEN: Zielwechsel = Zielwechsel::Ueberlebt;

    /// Merkt eine Nische als belegt.
    pub fn belege(&mut self, nische: impl Into<String>) {
        let n = nische.into();
        if !self.belegt.contains(&n) {
            self.belegt.push(n);
        }
    }

    /// Ob diese Nische belegt ist.
    #[must_use]
    pub fn ist_belegt(&self, nische: &str) -> bool {
        self.belegt.iter().any(|n| n == nische)
    }

    /// Wie viele Nischen belegt sind.
    #[must_use]
    pub fn anzahl(&self) -> usize {
        self.belegt.len()
    }
}

/// Welcher Aufbau nach der aktuellen Zielfunktion der beste war.
///
/// **Wird bei einem Zielwechsel verworfen.** 'Das beste Ergebnis' ist ohne
/// Zielfunktion nicht definiert; ein alter Bestwert unter neuem Ziel ist keine
/// Behauptung, sondern eine falsche.
#[derive(Debug, Default)]
pub struct Optimalitaet {
    beste: Option<(String, Bewertung, ZielfunktionsHash)>,
}

impl Optimalitaet {
    /// Wie ein Zielwechsel sie behandelt.
    pub const VERHALTEN: Zielwechsel = Zielwechsel::Verworfen;

    /// Meldet einen Kandidaten.
    ///
    /// Gehoert der gespeicherte Bestwert zu einer **anderen** Zielfunktion,
    /// wird er ersetzt statt verglichen: Zwei Bewertungen verschiedener
    /// Zielfunktionen sind nicht vergleichbar.
    pub fn melde(&mut self, aufbau: impl Into<String>, wert: Bewertung, zf: ZielfunktionsHash) {
        let ersetzen = match &self.beste {
            Some((_, alt, alte_zf)) => *alte_zf != zf || wert.0 > alt.0,
            None => true,
        };
        if ersetzen {
            self.beste = Some((aufbau.into(), wert, zf));
        }
    }

    /// Der beste Aufbau **fuer genau diese Zielfunktion**.
    ///
    /// Gibt `None` zurueck, wenn der gespeicherte Bestwert zu einer anderen
    /// Zielfunktion gehoert — lieber keine Antwort als eine falsche.
    #[must_use]
    pub fn bester(&self, zf: ZielfunktionsHash) -> Option<(&str, Bewertung)> {
        match &self.beste {
            Some((aufbau, wert, gespeicherte_zf)) if *gespeicherte_zf == zf => {
                Some((aufbau.as_str(), *wert))
            }
            _ => None,
        }
    }
}

/// Das Sucharchiv: Nischenbelegung und Optimalitaet **getrennt** (AC5).
#[derive(Debug, Default)]
pub struct Sucharchiv {
    /// Welche Nischen erreicht wurden — ueberlebt einen Zielwechsel.
    pub nischen: Nischenbelegung,
    /// Welcher Aufbau der beste war — wird bei einem Zielwechsel verworfen.
    pub optimalitaet: Optimalitaet,
}

impl Sucharchiv {
    /// Ein leeres Archiv.
    #[must_use]
    pub fn neu() -> Self {
        Self::default()
    }

    /// Vollzieht einen Zielwechsel.
    ///
    /// Die Nischenbelegung bleibt, die Optimalitaet wird verworfen — die
    /// Entscheidung aus AC5, hier ausgefuehrt statt beschrieben.
    pub fn zielwechsel(&mut self) {
        // Nischen bleiben unberuehrt: Nischenbelegung::VERHALTEN
        self.optimalitaet = Optimalitaet::default();
    }
}
