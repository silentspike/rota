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
use crate::hostcaps::jetzt;
use crate::kalibrierung::{
    streuung, takt_drift, Abbruchgrund, RateModel, ABSCHNITTE_MIN, STREUUNG_MAX,
};

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
    ///
    /// Behalten wird die älteste Probe, die **noch im Fenster** liegt — auf der
    /// Grenze eingeschlossen. Bei vier Proben je Sekunde liegt sie genau 30
    /// Sekunden vor der jüngsten, und das Fenster mittelt über volle 30.
    pub fn aufnehmen(&mut self, probe: Momentaufnahme) {
        let grenze = probe.elapsed_ns.saturating_sub(nanos(self.dauer));
        self.proben.push_back(probe);
        while self.proben.front().is_some_and(|p| p.elapsed_ns < grenze) {
            self.proben.pop_front();
        }
    }

    /// Die jüngste Probe.
    #[must_use]
    pub fn letzte(&self) -> Option<&Momentaufnahme> {
        self.proben.back()
    }

    /// Die älteste Probe, die noch im Fenster liegt — der Bezugspunkt der
    /// Engpassdiagnose (#934), derselbe wie der von `gleitend`.
    #[must_use]
    pub fn aelteste(&self) -> Option<&Momentaufnahme> {
        self.proben.front()
    }

    /// Die Raten — `None`, solange keine zwei Proben vorliegen.
    #[must_use]
    pub fn raten(&self) -> Option<Raten> {
        let neu = self.proben.back()?;
        let alt = self.proben.front()?;
        let spanne_s = sekunden(neu.elapsed_ns.checked_sub(alt.elapsed_ns)?);
        if spanne_s <= 0.0 {
            return None;
        }
        let kurz = self.vor(neu, AKTUELL)?;
        let kurz_s = sekunden(neu.elapsed_ns - kurz.elapsed_ns);
        let rate = |neu: u64, alt: u64, s: f64| neu.saturating_sub(alt) as f64 / s;

        let mut wege_gleitend = [0.0; Weg::ANZAHL];
        for weg in Weg::ALLE {
            let i = weg.index();
            wege_gleitend[i] = rate(neu.kandidaten[i], alt.kandidaten[i], spanne_s);
        }
        let aktive_straenge = neu
            .kaempfe_je_strang
            .iter()
            .zip(&kurz.kaempfe_je_strang)
            .filter(|(n, a)| n > a)
            .count();
        Some(Raten {
            kaempfe_aktuell: rate(neu.kaempfe, kurz.kaempfe, kurz_s),
            kaempfe_gleitend: rate(neu.kaempfe, alt.kaempfe, spanne_s),
            kandidaten_aktuell: rate(neu.kandidaten_gesamt(), kurz.kandidaten_gesamt(), kurz_s),
            kandidaten_gleitend: rate(neu.kandidaten_gesamt(), alt.kandidaten_gesamt(), spanne_s),
            wege_gleitend,
            fenster_s: spanne_s,
            aktive_straenge,
        })
    }

    /// Die Kampfrate **eines** Strangs, als [`RateModel`] — `None`, solange
    /// weniger als [`ABSCHNITTE_MIN`] Sekundenabschnitte vorliegen.
    ///
    /// Ein Abschnitt ist mindestens eine Sekunde lang ([`AKTUELL`]); bei vier
    /// Proben je Sekunde sind es genau vier Takte.
    ///
    /// # Warum der fleißigste Strang und nicht die Summe
    ///
    /// Die Ablaufsimulation verteilt Aufgaben auf Stränge, sie braucht also die
    /// Rate eines Strangs. Die Summe geteilt durch die Strangzahl wäre falsch,
    /// sobald Stränge warten — und auf dem Weg „ein Kampf" rechnet von vier
    /// Strängen nur einer. Der fleißigste Strang eines Abschnitts ist der, der
    /// die ganze Zeit gerechnet hat; seine Rate enthält, was ein Strang zwischen
    /// zwei Kämpfen sonst noch tut, und keine fremde Wartezeit.
    ///
    /// # Warum `ABSCHNITTE_MIN` und die Streuung aus der Kalibrierung
    ///
    /// Dieselbe Frage, dieselbe Antwort wie in [`crate::kalibrierung`]: Unter
    /// vier Abschnitten ist eine Streuung Zufall, und Standardabweichung durch
    /// Mittelwert ist dort begründet. Ein zweites Verfahren hier wäre eine
    /// zweite Wahrheit über dieselbe Größe.
    ///
    /// `reason` ist [`Abbruchgrund::Stabil`] nur unter [`STREUUNG_MAX`], sonst
    /// [`Abbruchgrund::Hoechstdauer`]: Das Fenster ist mit 30 Sekunden genau die
    /// Höchstdauer der Kalibrierung, und eine Rate, die darin nicht zur Ruhe
    /// kommt, ist im Sinne von #912 nicht stabil.
    #[must_use]
    pub fn strangrate(&self) -> Option<RateModel> {
        let raten = self.sekundenraten()?;
        let mittel = raten.iter().sum::<f64>() / raten.len() as f64;
        let spread = streuung(&raten);
        let gesamt_ns = self.proben.back()?.elapsed_ns - self.proben.front()?.elapsed_ns;
        Some(RateModel {
            rate_per_s: mittel,
            spread,
            samples: raten.len(),
            duration_ms: gesamt_ns / 1_000_000,
            reason: if spread < STREUUNG_MAX {
                Abbruchgrund::Stabil
            } else {
                Abbruchgrund::Hoechstdauer
            },
            measured_at: jetzt(),
            psi_cpu: None,
            takt_drift: None,
        })
    }

    /// Wie stark sich die Strangrate innerhalb des Fensters verschoben hat:
    /// Mittel der zweiten Hälfte der Sekundenabschnitte gegen das der ersten,
    /// als Anteil — `None` unter [`ABSCHNITTE_MIN`] Abschnitten.
    ///
    /// **Das ist der gemessene Teil des Restkostenfehlers (AC7).** Die Streuung
    /// sagt, wie unruhig die Rate um ihr Mittel liegt; der Trend sagt, dass das
    /// Mittel selbst wandert. Gemessen am 2026-10-28 an einer Suche über vier
    /// Stränge: Die Strangrate fiel binnen fünf Sekunden von 16 981 auf 12 480
    /// Kämpfe je Sekunde — spätere Runden rechnen längere Rotationen, und die
    /// Maschine war belegt (Last 4,66 auf 16 Kernen). Die obere Grenze der
    /// Restzeit lag bei Sekunde vier mit 3,39 s unter den tatsächlichen 5,18 s.
    ///
    /// Dieselbe Hälftenrechnung wie [`crate::kalibrierung::takt_drift`], und
    /// deshalb dieselbe Funktion: Ein Mittel über Hälften glättet das Rauschen
    /// eines einzelnen Abschnitts und lässt den Trend stehen.
    #[must_use]
    pub fn strangtrend(&self) -> Option<f64> {
        takt_drift(&self.sekundenraten()?)
    }

    /// Die Kampfrate des fleißigsten Strangs je Sekundenabschnitt — `None`
    /// unter [`ABSCHNITTE_MIN`] Abschnitten.
    fn sekundenraten(&self) -> Option<Vec<f64>> {
        let mut raten = Vec::new();
        let mut start = self.proben.front()?;
        for probe in self.proben.iter().skip(1) {
            let dauer_ns = probe.elapsed_ns - start.elapsed_ns;
            if dauer_ns < nanos(AKTUELL) {
                continue;
            }
            let fleissigster = probe
                .kaempfe_je_strang
                .iter()
                .zip(&start.kaempfe_je_strang)
                .map(|(n, a)| n.saturating_sub(*a))
                .max()
                .unwrap_or(0);
            raten.push(fleissigster as f64 / sekunden(dauer_ns));
            start = probe;
        }
        (raten.len() >= ABSCHNITTE_MIN).then_some(raten)
    }

    /// Die jüngste Probe, die mindestens `abstand` vor `neu` liegt — oder die
    /// älteste, wenn das Fenster so weit nicht zurückreicht.
    fn vor(&self, neu: &Momentaufnahme, abstand: Duration) -> Option<&Momentaufnahme> {
        let grenze = neu.elapsed_ns.saturating_sub(nanos(abstand));
        self.proben
            .iter()
            .rev()
            .find(|p| p.elapsed_ns <= grenze)
            .or_else(|| self.proben.front())
            .filter(|p| p.elapsed_ns < neu.elapsed_ns)
    }
}

fn nanos(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

fn sekunden(ns: u64) -> f64 {
    ns as f64 / 1e9
}
