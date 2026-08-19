//! Wertungsmenge und Instanzgruppen (Issue #981).
//!
//! # Zwei Fragen, maschinell beantwortet
//!
//! Geht dieser Boss in eine Wertung ein? Zu welcher Gruppe gehoert er? Ohne
//! Antworten darauf ist keine Aussage ueber Eliten, Viabilitaet oder Abdeckung
//! definiert.
//!
//! # Warum der Ausschluss hart ist und nicht warnt
//!
//! Im selben Verzeichnis wie die Spielbosse liegen Pruef- und Lastprofile, die
//! absichtlich extreme Werte tragen: `test-runaway` traegt eine umgekehrte
//! Siegbedingung, und ein Grossteil von ihnen steht auf 999999 Lebenspunkten,
//! damit der Kampf nicht von selbst endet. **Eine Elite, die versehentlich
//! gegen so einen Boss optimiert wurde, sieht im Ergebnis vollkommen plausibel
//! aus** — sie hat eine Zahl, einen Aufbau, einen Trace, und sie ist wertlos.
//!
//! Eine Warnung genuegt dafuer nicht: Man kann sie uebersehen, und das
//! Ergebnis, das danebensteht, sieht richtig aus. Deshalb bricht
//! [`ScoringSet::of`] ab und nennt den Boss.
//!
//! # Es gibt keine Gruppe „alle Bosse"
//!
//! Ein Aufbau wird **je Instanz** bewertet, und innerhalb einer Instanz zaehlt
//! sein schlechtestes Ergebnis ueber deren Bosse. Eine Gruppe ueber alle Bosse
//! waere der erste Schritt zu einer globalen Elite — die es nicht geben soll.
//! Dieses Modul bietet sie deshalb nicht an, auch nicht als Bequemlichkeit fuer
//! einen Prueffall.
//!
//! # Woher die Klassifikation kommt
//!
//! Aus dem Feld `instance` jedes Bosses, gegen die in `build.rs` gefuehrten
//! Mengen [`data::SCORED_INSTANCES`] und [`data::PROBE_INSTANCES`]. Beide
//! werden dort **erzeugt**, nicht hier ein zweites Mal aufgeschrieben: Zwei
//! Listen an zwei Stellen laufen auseinander, und der Fehler waere still.
//!
//! Ein Boss mit einem Wert, der in keiner der beiden Mengen steht, laesst
//! bereits den **Bau** scheitern. Dass [`BossClass::of`] trotzdem einen
//! `Option`-Rueckgabewert hat, ist kein Widerspruch: Die Funktion nimmt eine
//! beliebige Zeichenkette, nicht nur die eines eingebauten Bosses.

use crate::data::{self, Boss};

/// Wozu ein Boss gehoert (Issue #981, AC1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossClass {
    /// Gehoert zu einer Spielinstanz und geht in Wertungen ein.
    Scored,
    /// Pruef- oder Lastprofil. **Kein Spieler sieht diesen Boss.**
    Probe,
}

impl BossClass {
    /// Die Klasse zu einem `instance`-Wert.
    ///
    /// `None` heisst: Der Wert steht in keiner der beiden Mengen. Bei einem
    /// eingebauten Boss kann das nicht vorkommen — `build.rs` bricht vorher.
    #[must_use]
    pub fn of(instance: &str) -> Option<Self> {
        if data::SCORED_INSTANCES.contains(&instance) {
            Some(Self::Scored)
        } else if data::PROBE_INSTANCES.contains(&instance) {
            Some(Self::Probe)
        } else {
            None
        }
    }

    /// Ob diese Klasse in eine Wertung eingeht.
    #[must_use]
    pub const fn is_scored(self) -> bool {
        matches!(self, Self::Scored)
    }
}

/// Die Klasse dieses Bosses.
///
/// # Warum das hier steht und nicht als Feld am Boss
///
/// Ein zweites erzeugtes Feld waere eine zweite Wahrheit ueber dieselbe Sache.
/// Die Klasse folgt aus `instance` — und dass sie folgt, ist gerade der Punkt
/// von AC5: Aendert jemand `instance`, aendert sich die Gruppierung mit.
#[must_use]
pub fn class_of(boss: &Boss) -> BossClass {
    BossClass::of(boss.instance).unwrap_or_else(|| {
        panic!(
            "Boss \"{}\" traegt instance \"{}\", das keiner Klasse zugeordnet ist — \
             build.rs haette das abfangen muessen",
            boss.id, boss.instance
        )
    })
}

/// Eine Instanzgruppe: die wertungsfaehigen Bosse **einer** Instanz (AC3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceGroup {
    /// Der `instance`-Wert, der diese Gruppe benennt.
    pub instance: &'static str,
}

impl InstanceGroup {
    /// Die Bosse dieser Gruppe, in der Reihenfolge des Bestands.
    ///
    /// Ohne Container mit Streuung: Die Reihenfolge fliesst in Ergebnisse ein,
    /// und GAME-DESIGN 18.4 verbietet Hash-Abbildungen an solchen Stellen.
    #[must_use]
    pub fn members(&self) -> Vec<&'static Boss> {
        data::BOSSES
            .iter()
            .filter(|boss| boss.instance == self.instance)
            .collect()
    }

    /// Wie viele Bosse in dieser Gruppe stehen.
    #[must_use]
    pub fn len(&self) -> usize {
        self.members().len()
    }

    /// Ob die Gruppe leer ist.
    ///
    /// Kann bei einer aus dem Bestand gebildeten Gruppe nicht vorkommen — sie
    /// entsteht ja aus einem Boss. Steht hier, weil `len` ohne `is_empty` eine
    /// Clippy-Warnung ist, und weil ein Aufrufer beides erwartet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members().is_empty()
    }
}

/// Warum eine Wertungsmenge zurueckgewiesen wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringError {
    /// Ein Pruefboss stand in der Menge (AC2).
    ProbeBossInScoringSet {
        /// Die Kennung des Bosses.
        boss: &'static str,
        /// Sein `instance`-Wert.
        instance: &'static str,
        /// Die wievielte Stelle der uebergebenen Menge — damit der Aufrufer
        /// weiss, WO er nachsehen muss, nicht nur DASS etwas falsch war.
        position: usize,
    },
}

impl core::fmt::Display for ScoringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ProbeBossInScoringSet {
                boss,
                instance,
                position,
            } => write!(
                f,
                "Boss \"{boss}\" (instance \"{instance}\") ist ein Pruefprofil und gehoert \
                 in keine Wertung — Stelle {position} der uebergebenen Menge. \
                 Kein Spieler sieht diesen Boss; ein Ergebnis gegen ihn sieht plausibel \
                 aus und ist wertlos."
            ),
        }
    }
}

/// Eine geprueft wertungsfaehige Menge von Bossen.
///
/// Der Typ ist der Nachweis: Wer eine `ScoringSet` in der Hand hat, weiss, dass
/// kein Pruefboss darin steht. Das ist der Unterschied zu einer Pruefung, die
/// man aufrufen **kann** — diese kann man nicht vergessen.
// **Kein `Eq`.** `Boss` traegt Widerstandsfaktoren als `f64` und ist damit nur
// `PartialEq` — gemessen beim Bau, nicht vermutet.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoringSet {
    bosses: Vec<&'static Boss>,
}

impl ScoringSet {
    /// Prueft eine Menge und weist sie zurueck, wenn ein Pruefboss darin steht
    /// (AC2).
    ///
    /// # Errors
    ///
    /// [`ScoringError::ProbeBossInScoringSet`], sobald ein Boss keiner
    /// Spielinstanz angehoert. Der **erste** solche Boss beendet die Pruefung:
    /// Eine Liste aller Verstoesse verleitet dazu, sie zu ueberfliegen.
    pub fn of(bosses: &[&'static Boss]) -> Result<Self, ScoringError> {
        for (position, boss) in bosses.iter().enumerate() {
            if !class_of(boss).is_scored() {
                return Err(ScoringError::ProbeBossInScoringSet {
                    boss: boss.id,
                    instance: boss.instance,
                    position,
                });
            }
        }
        Ok(Self {
            bosses: bosses.to_vec(),
        })
    }

    /// Wie [`Self::of`], bricht aber ab statt zurueckzugeben.
    ///
    /// # Panics
    ///
    /// Wenn ein Pruefboss in der Menge steht. Fuer Aufrufer, bei denen das ein
    /// Programmierfehler ist und kein Fall, den man behandelt.
    #[must_use]
    pub fn require(bosses: &[&'static Boss]) -> Self {
        Self::of(bosses).unwrap_or_else(|fehler| panic!("{fehler}"))
    }

    /// Alle wertungsfaehigen Bosse des Bestands.
    #[must_use]
    pub fn all_scored() -> Self {
        Self {
            bosses: data::BOSSES
                .iter()
                .filter(|boss| class_of(boss).is_scored())
                .collect(),
        }
    }

    /// Die Bosse dieser Menge.
    #[must_use]
    pub fn bosses(&self) -> &[&'static Boss] {
        &self.bosses
    }

    /// Wie viele Bosse in der Menge stehen.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bosses.len()
    }

    /// Ob die Menge leer ist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bosses.is_empty()
    }

    /// Die Instanzgruppen dieser Menge (AC3).
    ///
    /// **Die Gruppe ist die Einheit jeder Elite-Aussage.** Eine Gruppe „alle
    /// Bosse" gibt es hier nicht und soll es nicht geben.
    ///
    /// Die Reihenfolge folgt dem ersten Auftreten im Bestand, nicht einer
    /// Sortierung nach Namen: Sie ist damit dieselbe wie die Boss-Reihenfolge,
    /// und die ist Teil des Determinismus.
    #[must_use]
    pub fn groups(&self) -> Vec<InstanceGroup> {
        let mut gruppen: Vec<InstanceGroup> = Vec::new();
        for boss in &self.bosses {
            if !gruppen.iter().any(|g| g.instance == boss.instance) {
                gruppen.push(InstanceGroup {
                    instance: boss.instance,
                });
            }
        }
        gruppen
    }
}

/// Alle Bosse des Bestands, die **keiner** Wertung angehoeren.
#[must_use]
pub fn probe_bosses() -> Vec<&'static Boss> {
    data::BOSSES
        .iter()
        .filter(|boss| !class_of(boss).is_scored())
        .collect()
}
