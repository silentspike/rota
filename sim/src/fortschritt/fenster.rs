//! Das gleitende Fenster: die letzten 30 Sekunden, nicht der ganze Lauf (AC4).
//!
//! # Warum nicht der Gesamtmittelwert
//!
//! Nach einer Drosselung wäre er stundenlang falsch. Ein Lauf, der eine Stunde
//! mit 1000 je Sekunde lief und seit zehn Minuten mit 100 läuft, hat einen
//! Gesamtmittelwert von 870 — und eine Restzeit daraus wäre um Faktor neun zu
//! kurz, genau in dem Moment, in dem jemand sie braucht.
//!
//! # Zwei Zeitskalen, beide aus denselben Proben
//!
//! `aktuell` mittelt über die letzte Sekunde, `gleitend` über 30. Beide sind
//! Differenzen zweier Momentaufnahmen geteilt durch deren Abstand — kein
//! eigener Zähler, keine zweite Wahrheit.

use std::collections::VecDeque;
use std::time::Duration;

use super::zaehlwerk::{Momentaufnahme, Weg};
use crate::kalibrierung::RateModel;

/// Über wie lange `gleitend` mittelt.
pub const FENSTER: Duration = Duration::from_secs(30);

/// Über wie lange `aktuell` mittelt.
pub const AKTUELL: Duration = Duration::from_secs(1);

/// Die Raten aus dem Fenster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Raten {
    /// Gerechnete Kämpfe je Sekunde über die letzte Sekunde — der Kernel, roh.
    pub kaempfe_aktuell: f64,
    /// Gerechnete Kämpfe je Sekunde über das Fenster.
    pub kaempfe_gleitend: f64,
    /// Abgeschlossene Auswertungen je Sekunde über die letzte Sekunde.
    pub kandidaten_aktuell: f64,
    /// Abgeschlossene Auswertungen je Sekunde über das Fenster.
    pub kandidaten_gleitend: f64,
    /// Abgeschlossene Auswertungen je Sekunde und [`Weg`], über das Fenster.
    pub wege_gleitend: [f64; Weg::ANZAHL],
    /// Über wie viele Sekunden `gleitend` tatsächlich mittelt — am Anfang
    /// eines Laufs weniger als 30.
    pub fenster_s: f64,
    /// Stränge, die in der letzten Sekunde mindestens einen Kampf gerechnet
    /// haben.
    pub aktive_straenge: usize,
}

/// Die Proben der letzten [`FENSTER`] Sekunden.
#[derive(Debug, Clone)]
pub struct Fenster {
    proben: VecDeque<Momentaufnahme>,
    dauer: Duration,
}

impl Fenster {
    /// Ein leeres Fenster über `dauer`.
    #[must_use]
    pub fn neu(dauer: Duration) -> Self {
        Self {
            proben: VecDeque::new(),
            dauer,
        }
    }

    /// Nimmt eine Probe auf und verwirft, was älter als das Fenster ist.
    pub fn aufnehmen(&mut self, probe: Momentaufnahme) {
        todo!("#933 {probe:?} {:?}", self.dauer)
    }

    /// Die jüngste Probe.
    #[must_use]
    pub fn letzte(&self) -> Option<&Momentaufnahme> {
        self.proben.back()
    }

    /// Die Raten — `None`, solange keine zwei Proben vorliegen.
    #[must_use]
    pub fn raten(&self) -> Option<Raten> {
        todo!("#933 {self:?}")
    }

    /// Die Kampfrate **eines** Strangs, als [`RateModel`] — `None`, solange
    /// weniger als [`crate::kalibrierung::ABSCHNITTE_MIN`] Sekundenabschnitte
    /// vorliegen.
    #[must_use]
    pub fn strangrate(&self) -> Option<RateModel> {
        todo!("#933 {self:?}")
    }
}
