//! Zwei Ausgabewege aus EINER Anzeige (AC1, AC2, AC8).
//!
//! Die Zeile für Menschen und der JSON-Strom entstehen aus demselben Wert vom
//! Typ [`Anzeige`], und der hält genau eine [`Momentaufnahme`]. Keine der
//! beiden Ausgaben liest selbst einen Zähler — sie können also nicht
//! auseinanderlaufen, und ein Prüffall hält das fest.

use std::time::Duration;

use super::fenster::Raten;
use super::restzeit::{Kostenbild, Restzeit};
use super::zaehlwerk::Momentaufnahme;
use crate::drosselung::ControlSnapshot;

/// Welche Art Meldung eine Zeile des Stroms ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ereignis {
    /// Der Lauf beginnt.
    Start,
    /// Ein Takt des Anzeigestrangs.
    Fortschritt,
    /// Der Lauf ist zu Ende.
    Ende,
}

impl Ereignis {
    /// Der Name im Feld `event`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Fortschritt => "progress",
            Self::Ende => "end",
        }
    }
}

/// Ein Rechner mit seinen Strängen, wie die Zeile ihn zeigt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RechnerZeile {
    /// Kennung, etwa der Hostname.
    pub id: String,
    /// Arbeitsstränge, die der Lauf bekommen hat.
    pub straenge: usize,
    /// Davon in der letzten Sekunde tätig.
    pub aktiv: usize,
    /// Kerne, die der Prozess benutzen darf — `None`, wenn nicht lesbar.
    pub erlaubt: Option<usize>,
}

/// Alles, was beide Ausgaben brauchen — aus einer Momentaufnahme.
///
/// Ohne `PartialEq`: [`ControlSnapshot`] trägt keines, und zwei Anzeigen
/// vergleicht ohnehin niemand — verglichen werden ihre beiden Ausgaben.
#[derive(Debug, Clone)]
pub struct Anzeige {
    /// Welche Meldung.
    pub ereignis: Ereignis,
    /// Laufende Nummer dieses Rechners, ab 1 — zeigt Lücken und Verspätungen.
    pub sequenz: u64,
    /// Weltzeit der Momentaufnahme, ISO 8601 in UTC mit Mikrosekunden.
    pub utc: String,
    /// Was gerade läuft: `greedy`, `fest`, `gestaffelt`, `bench` …
    pub phase: String,
    /// Gegen welchen Boss.
    pub boss: Option<String>,
    /// Die eine Momentaufnahme.
    pub probe: Momentaufnahme,
    /// Die Raten aus dem Fenster — `None` in der ersten Viertelsekunde.
    pub raten: Option<Raten>,
    /// Was eine Auswertung kostet.
    pub kosten: Kostenbild,
    /// Die Restzeit oder das Plateau.
    pub restzeit: Restzeit,
    /// Die Rechner mit ihren Strängen.
    pub rechner: Vec<RechnerZeile>,
    /// Die aktive Drosselung — `None` ohne Steuerdatei.
    pub drosselung: Option<ControlSnapshot>,
}

impl Anzeige {
    /// Die Zeilen für Menschen.
    #[must_use]
    pub fn zeilen(&self) -> Vec<String> {
        todo!("#933 {self:?}")
    }

    /// Eine Zeile JSON Lines, ohne Zeilenende.
    #[must_use]
    pub fn jsonl(&self, run_id: &str, host_id: &str) -> String {
        todo!("#933 {run_id} {host_id}")
    }
}

/// Eine Zeit seit der Unix-Epoche als ISO 8601 in UTC, mit Mikrosekunden.
#[must_use]
pub fn utc_iso8601(seit_epoche: Duration) -> String {
    todo!("#933 {seit_epoche:?}")
}

/// Eine ganze Zahl mit Leerzeichen als Tausendertrenner: `1 084 000`.
#[must_use]
pub fn format_ganzzahl(n: u64) -> String {
    todo!("#933 {n}")
}
