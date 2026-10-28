//! Die Restzeit: gewichtet, simuliert und mit hergeleiteter Spanne (AC3, AC5–AC7).
//!
//! # Drei Fehler, gegen die dieses Modul gebaut ist
//!
//! 1. **Restkandidaten durch Kandidaten je Sekunde** (AC5). Eine Auswertung auf
//!    dem Weg „ein Kampf" kostet einen Kampf, eine feste Stichprobe kostet
//!    `iterations`. Die Division ohne Gewicht wird genau dann falsch, wenn der
//!    Verteiler gut arbeitet.
//! 2. **Restzyklen durch Zyklen je Sekunde** (AC6). Die gewichtete Division
//!    setzt voraus, dass jede Aufgabe auf jedem Strang gleich läuft und alles
//!    parallel geht. Hier gilt das ausdrücklich nicht: Eine Auswertung auf dem
//!    Weg „ein Kampf" ist EIN Kampf auf EINEM Strang, und die nächste beginnt
//!    erst nach dem Sammelpunkt. Vier Stränge machen diese Kette nicht viermal
//!    schneller.
//! 3. **Eine Spanne ohne Herleitung** (AC7). Sie wäre eine erfundene Zahl.
//!
//! # Die Kosten kommen aus einem `RateModel`
//!
//! Gewicht einer Aufgabe = ihre Einheiten geteilt durch die Rate der
//! zugehörigen [`Arbeitsart`] auf dem Strang, der sie rechnet. Die Rate ist ein
//! [`RateModel`] — derselbe Vertrag wie in der Fähigkeitserhebung (#912), mit
//! Streuung und Abschnittszahl. Heute rechnet jeder Weg in Kämpfen, also in
//! `ScalarFullFight`; ein späterer Weg in einer anderen Einheit bekommt seine
//! eigene Art, und ein Rechner ohne diese Art ist für ihn nicht zulässig.

use super::fenster::Raten;
use super::zaehlwerk::{Momentaufnahme, Weg};
use crate::kalibrierung::{Arbeitsart, RateModel};

/// Wahrscheinlichkeit, dass eine Aufgabe im lokalen Lauf wiederholt werden muss.
///
/// **Null, und das ist gemessen am Bau, nicht angenommen:** Ein lokaler Kampf
/// scheitert nicht. Ein Strang, der abstürzt, beendet den ganzen Lauf
/// (`expect("ein Strang ist abgestuerzt")` in [`crate::balance`]) — es gibt
/// keinen Weg, auf dem eine Aufgabe ein zweites Mal gerechnet würde. Der Term
/// steht trotzdem in der Herleitung, mit diesem Wert: Sobald Zusatzrechner
/// Pakete verlieren können (E15.5), ist er der erste, der wächst.
pub const WIEDERHOLUNG_LOKAL: f64 = 0.0;

/// Ein Rechner, wie die Ablaufsimulation ihn sieht.
#[derive(Debug, Clone)]
pub struct Rechner {
    /// Kennung, etwa der Hostname.
    pub id: String,
    /// Wie viele Stränge er für diesen Lauf rechnet.
    pub straenge: usize,
    /// Seine Rate je Strang und [`Arbeitsart`]. Fehlt eine Art, darf er
    /// Aufgaben dieser Art nicht rechnen — etwa gonet ohne AVX2.
    pub raten: Vec<(Arbeitsart, RateModel)>,
}

/// Eine Stufe: Aufgaben, die gleichzeitig laufen dürfen. Zwischen zwei Stufen
/// liegt ein Sammelpunkt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stufe {
    /// Was gerechnet wird.
    pub art: Arbeitsart,
    /// Wie viele gleichartige Aufgaben die Stufe hat.
    pub aufgaben: u64,
    /// Einheiten der [`Arbeitsart`] je Aufgabe — bei Kämpfen: Kämpfe.
    pub einheiten_je_aufgabe: f64,
    /// Wie oft diese Stufe nacheinander kommt.
    pub wiederholungen: u64,
}

/// Was die Ablaufsimulation ergibt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ablauf {
    /// Simulierte Dauer bis zum letzten Sammelpunkt, in Sekunden.
    pub dauer_s: f64,
    /// Die Kette, die auch unendlich viele Stränge nicht verkürzen: je Stufe die
    /// längste einzelne Aufgabe auf dem schnellsten zulässigen Strang.
    pub kritischer_pfad_s: f64,
    /// Die Division, die AC6 verwirft: alle Einheiten durch alle Raten.
    pub naiv_s: f64,
    /// Wie viele Rechner Aufgaben bekommen konnten.
    pub rechner: usize,
}

/// Warum keine Ablaufsimulation möglich ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ablauffehler {
    /// Kein Rechner kann diese Arbeitsart rechnen.
    KeinRechner(Arbeitsart),
    /// Eine Rate ist null, negativ oder keine Zahl.
    UngueltigeRate(Arbeitsart),
}

/// Wie viele Kämpfe eine Auswertung kostet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kampfzahl {
    /// Genau so viele — der feste Weg.
    Fest(u64),
    /// Höchstens so viele — die gestaffelte Stichprobe endet früher, sobald die
    /// Aussage sicher ist.
    Hoechstens(u64),
}

/// Was eine Auswertung dieses Laufs kostet und wie sie sich verteilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kostenbild {
    /// Der Weg, den jede Auswertung dieses Laufs nimmt.
    pub weg: Weg,
    /// Kämpfe je Auswertung.
    pub kaempfe_je_kandidat: Kampfzahl,
}

/// Wie genau die verbleibende Arbeit feststeht — der Restkostenfehler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restkosten {
    /// Auf den Kampf genau.
    Exakt {
        /// Verbleibende Kämpfe.
        kaempfe: u64,
    },
    /// Zwischen zwei Grenzen.
    Spanne {
        /// So viele verbleiben mindestens.
        mindestens: u64,
        /// Mehr können es nicht werden.
        hoechstens: u64,
    },
}

/// Woraus eine Restzeit entstanden ist (AC7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Herleitung {
    /// Wie genau die Restmenge feststeht.
    pub restkosten: Restkosten,
    /// Kämpfe je Sekunde und Strang, aus dem Fenster.
    pub rate_je_strang: f64,
    /// Relative Streuung dieser Rate über die Sekundenabschnitte.
    pub rate_streuung: f64,
    /// Wie viele Sekundenabschnitte in die Streuung eingingen.
    pub rate_abschnitte: usize,
    /// Simulierte Dauer der kleinsten Restmenge.
    pub ablauf_min_s: f64,
    /// Simulierte Dauer der größten Restmenge.
    pub ablauf_max_s: f64,
    /// Kritischer Pfad der größten Restmenge.
    pub kritischer_pfad_s: f64,
    /// Rechner, über die gemittelt werden darf.
    pub rechner: usize,
    /// Der Anteil der Spanne aus der Ratenstreuung, in Sekunden.
    pub sigma_rate_s: f64,
    /// Wahrscheinlichkeit einer Wiederholung je Aufgabe.
    pub wiederholung_p: f64,
    /// Der Anteil der Spanne aus Wiederholungen, in Sekunden.
    pub sigma_wiederholung_s: f64,
    /// Zum Vergleich, AC5: Restkandidaten durch Kandidaten je Sekunde.
    pub naiv_kandidaten_s: Option<f64>,
    /// Zum Vergleich, AC6: Restkämpfe durch alle Stränge mal Rate.
    pub naiv_division_s: f64,
}

/// Die Restzeit eines Laufs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Restzeit {
    /// Eine Restzeit mit Spanne.
    Spanne {
        /// Der Mittelwert — nur bei exakt bekannter Restmenge.
        mitte_s: Option<f64>,
        /// Untere Grenze, in Sekunden.
        unten_s: f64,
        /// Obere Grenze, in Sekunden.
        oben_s: f64,
        /// Woraus sie entstanden ist.
        herleitung: Herleitung,
    },
    /// Offene Suche: **keine Restzeit**, sondern das Plateau.
    Plateau,
    /// Noch nicht bestimmbar — mit Grund, nicht mit einer Null.
    Unbestimmt(&'static str),
    /// Der Lauf ist zu Ende.
    Fertig,
}

/// Die Ablaufsimulation (AC6): Listenzuteilung je Stufe, Sammelpunkt zwischen
/// den Stufen.
///
/// # Errors
///
/// [`Ablauffehler`], wenn eine Stufe keinen zulässigen Rechner hat oder eine
/// Rate unbrauchbar ist.
pub fn ablauf(stufen: &[Stufe], rechner: &[Rechner]) -> Result<Ablauf, Ablauffehler> {
    todo!("#933 {stufen:?} {rechner:?}")
}

/// Was auf einem Weg noch zu tun ist.
///
/// Ein Lauf von heute hat genau einen Anteil — alle seine Auswertungen nehmen
/// denselben Weg. Die Liste ist trotzdem eine Liste: Sobald ein Router
/// (E11.4b) Auswertungen auf verschiedene Wege schickt, steht jeder mit seinen
/// eigenen Kosten darin, und AC5 verlangt genau diese Gewichtung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Restanteil {
    /// Der Weg.
    pub weg: Weg,
    /// Kämpfe je Auswertung auf diesem Weg.
    pub kaempfe_je_kandidat: Kampfzahl,
    /// So viele Auswertungen stehen mindestens noch aus.
    pub mindestens: u64,
    /// Mehr können es nicht werden.
    pub hoechstens: u64,
}

/// Die Restzeit aus Restanteilen (AC5–AC7).
///
/// `begonnen` sind die Kämpfe, die die laufende Auswertung des ersten Anteils
/// schon gerechnet hat. `raten` dienen nur dem Vergleich mit der naiven
/// Division in der [`Herleitung`].
#[must_use]
pub fn restzeit(
    anteile: &[Restanteil],
    begonnen: u64,
    rechner: &[Rechner],
    raten: Option<&Raten>,
    wiederholung_p: f64,
) -> Restzeit {
    todo!("#933 {anteile:?} {begonnen} {rechner:?} {raten:?} {wiederholung_p}")
}

/// Die Restzeit eines Laufs aus seiner Momentaufnahme (AC3).
///
/// Hier fällt die Entscheidung aus AC3: [`Arbeitsmenge::Offen`] ergibt
/// [`Restzeit::Plateau`] — vor jeder Rechnung, nicht als deren Ergebnis.
///
/// [`Arbeitsmenge::Offen`]: super::zaehlwerk::Arbeitsmenge::Offen
#[must_use]
pub fn restzeit_aus(
    probe: &Momentaufnahme,
    kosten: &Kostenbild,
    rechner: &[Rechner],
    raten: Option<&Raten>,
    wiederholung_p: f64,
) -> Restzeit {
    todo!("#933 {probe:?} {kosten:?} {rechner:?} {raten:?} {wiederholung_p}")
}
