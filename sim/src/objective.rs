//! Der Zielvertrag: harte Bedingungen vor Rangfolge (Issue #939).
//!
//! # Warum keine gewichtete Summe
//!
//! Eine Zahl aus „Schaden plus a mal Verhindert plus b mal Ueberlebt" versteckt
//! die Balance-Entscheidung in ihren Gewichten. Niemand kann spaeter sagen,
//! warum eine Zusammenstellung gewonnen hat — die Begruendung ist in einem
//! Zahlenpaar aufgeloest, das niemand mehr auseinandernehmen kann.
//!
//! Hier entscheidet stattdessen eine **lexikographische Rangfolge**: Stufe 2
//! kommt nur zum Zug, wenn Stufe 1 gleich steht. Jeder Vergleich nennt die
//! Stufe, die ihn entschieden hat (siehe [`RankingComparison::decided_by`]).
//!
//! # Und warum die Zulaessigkeit VOR der Rangfolge steht
//!
//! An einer Bossart liefert „mehr Schaden ist besser" die falsche Antwort. Bei
//! einem Boss mit umgekehrtem Ziel ([`WinCondition::SurviveTimer`]) ist ein
//! aufgebrauchter Ring die **Niederlage**: Der Sieg besteht darin, die Frist zu
//! ueberstehen. Eine Zielfunktion der Form „maximiere Schaden" findet dort
//! genau die falsche Antwort.
//!
//! Deshalb taucht „Sieg" **genau einmal** auf, als Zulaessigkeitsbedingung, und
//! die Rangfolge beginnt **unterhalb** davon. Ein unzulaessiger Kandidat kann
//! ueber keine noch so gute Zahl gewinnen.
//!
//! # Nicht gemessen ist nicht erfuellt
//!
//! Eine harte Bedingung, deren Messwert fehlt, macht den Kandidaten
//! **unzulaessig** — nicht zulaessig. Sonst besteht jeder Kandidat genau die
//! Bedingung, die niemand prueft.
//!
//! Davon zu unterscheiden ist die Bedingung, die ein Lauf **gar nicht
//! verlangt**: Sie steht im Vertrag als `None` (siehe
//! [`FeasibilityThresholds`]) und wird nicht geprueft. Der Unterschied ist
//! nicht kosmetisch — er trennt „wir wissen es nicht" von „wir fordern es
//! hier nicht".
//!
//! # Was aus einem Balance-Lauf heute WIRKLICH speisbar ist
//!
//! Gemessen am Bestand vom 2026-08-19, nicht angenommen:
//!
//! ```text
//! Stufe                          speisbar aus einem BalanceResult?
//! 1 Erfolgswahrscheinlichkeit    JA    aus den Einzelergebnissen
//! 2 Ueberlebensreserve           JA    wenn der Vertrag die Lebensgrenze traegt
//! 3 Mechanikqualitaet            NEIN  kein Mechanikwert verlaesst den Kampf
//! 4 Abstand zum Zielband         JA    wenn der Vertrag ein Band traegt
//! 5 Zeit bis Sieg/Eindaemmung    TEILS nur bei gewoehnlicher Siegbedingung
//! 6 Robustheit                   NEIN  braucht mehrere Begegnungen
//! 7 Aenderung an den Daten       JA    kommt vom Kandidaten, nicht vom Kampf
//! 8 Ressourceneffizienz          NEIN  `RunResult` traegt sie, das Aggregat nicht
//! ```
//!
//! Was nicht speisbar ist, steht als [`StageValue::Unmeasured`] da — mit
//! geschriebener Messvorschrift an der jeweiligen Stufe. Eine Stufe ohne
//! Messvorschrift entscheidet still gar nichts und taeuscht dabei Genauigkeit
//! vor; **beides waere zulaessig, das Schweigen nicht** (Issue #939, AC6b).
//!
//! # Was hier NICHT zugesichert wird
//!
//! **Nicht**, dass die Werte stimmen — dieses Modul rechnet keinen Kampf. Es
//! nimmt Messungen entgegen und ordnet sie.
//!
//! **Nicht**, dass die Rangfolge richtig ist. Sie ist Vorgabe des
//! Auftraggebers und ein versioniertes Feld des Laufvertrags
//! ([`RunContract::version`]). Wer sie aendert, aendert die Version — sonst
//! sind zwei Laeufe nicht mehr vergleichbar und niemand merkt es.
//!
//! **Nicht**, dass ein zulaessiger Kandidat gut ist. Zulaessig heisst: er
//! gewinnt den Kampf oft genug. Was „gut" heisst, entscheidet die Rangfolge
//! darunter.

use core::cmp::Ordering;

use crate::balance::{BalanceResult, Sample, Statistics};
use crate::data::Boss;
use crate::speicher::RoherErgebnisvektor;

// ------------------------------------------------------- Wahrscheinlichkeit

/// Der z-Wert des zweiseitigen 95-Prozent-Intervalls.
///
/// Derselbe Wert wie in [`crate::balance::Statistics::ci95`] — dort auf 1,96
/// gerundet, hier voll, weil er in eine Wurzel eingeht.
const Z_95: f64 = 1.959_963_984_540_054;

/// Eine harte Bedingung ist eine **Wahrscheinlichkeit, kein Wahrheitswert**
/// (Issue #939, AC4).
///
/// Ein einzelner Durchgang beantwortet die Frage „gewinnt dieser Kandidat?"
/// nicht. Bei einem stochastischen Kampf ist die Antwort ein Anteil, und der
/// Anteil aus einer Stichprobe ist eine Schaetzung mit Streuung.
///
/// # Warum die untere Grenze entscheidet, nie der Schaetzwert (AC4b)
///
/// Ein Schaetzwert von 99,2 Prozent mit einer unteren Grenze von 97 Prozent
/// erfuellt eine Schwelle von 99 Prozent **nicht**. Ohne diese Regel besteht
/// jeder Kandidat, dessen Stichprobe zufaellig guenstig lag — und je kleiner
/// die Stichprobe, desto leichter faellt ihm das.
///
/// Der Fall, an dem es am deutlichsten wird: 100 von 100 Siegen. Der
/// Schaetzwert ist 1,0 und besteht jede Schwelle. Die untere Wilson-Grenze
/// liegt bei rund 0,963 — aus hundert Durchgaengen laesst sich eine
/// Sicherheit von 99 Prozent schlicht nicht belegen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Probability {
    /// Exakt gerechnet, ohne Stichprobe.
    ///
    /// Der Regelfall in diesem Projekt: Der Kampf ist deterministisch, solange
    /// die Reinheit auf null steht — dann gibt es keine Zufallsstroeme, keine
    /// Streuung und folglich keine Konfidenzgrenze. Der Wert entscheidet
    /// unmittelbar.
    Exact(f64),
    /// Aus einer Stichprobe gewonnen.
    Sampled {
        /// Der Anteil in der Stichprobe — **entscheidet nicht**.
        estimate: f64,
        /// Die untere Grenze des 95-Prozent-Intervalls — **sie entscheidet**.
        lower_bound: f64,
        /// Wie viele Durchgaenge die Stichprobe hatte.
        trials: u32,
    },
    /// Nicht gemessen.
    ///
    /// **Gilt als nicht erfuellt**, wo die Bedingung gefordert ist. Siehe den
    /// Modulkopf.
    Unmeasured,
}

impl Probability {
    /// Aus Erfolgen und Versuchen, mit der unteren **Wilson**-Grenze.
    ///
    /// # Warum Wilson und nicht die Normal-Naeherung
    ///
    /// Die uebliche Naeherung `p ± z·sqrt(p(1-p)/n)` liefert bei `p = 1` die
    /// Breite null und damit die Untergrenze 1,0 — genau im wichtigsten Fall
    /// („alle Durchgaenge gewonnen") behauptet sie Sicherheit, die die
    /// Stichprobe nicht hergibt. Wilson tut das nicht.
    ///
    /// Null Versuche ergeben [`Probability::Unmeasured`], nicht null Prozent:
    /// Ein Lauf, der nicht stattgefunden hat, ist keine Messung mit dem
    /// Ergebnis „schlecht".
    #[must_use]
    pub fn from_trials(successes: u32, trials: u32) -> Self {
        if trials == 0 {
            return Self::Unmeasured;
        }
        let n = f64::from(trials);
        let estimate = f64::from(successes.min(trials)) / n;
        let z2 = Z_95 * Z_95;
        let denominator = 1.0 + z2 / n;
        let center = (estimate + z2 / (2.0 * n)) / denominator;
        let margin =
            Z_95 * (estimate * (1.0 - estimate) / n + z2 / (4.0 * n * n)).sqrt() / denominator;
        Self::Sampled {
            estimate,
            lower_bound: (center - margin).clamp(0.0, 1.0),
            trials,
        }
    }

    /// Der Wert, der **entscheidet** (AC4b).
    ///
    /// Bei einer Stichprobe die untere Grenze, bei einer exakten Rechnung der
    /// Wert selbst, bei fehlender Messung `None`.
    #[must_use]
    pub const fn decisive(self) -> Option<f64> {
        match self {
            Self::Exact(p) => Some(p),
            Self::Sampled { lower_bound, .. } => Some(lower_bound),
            Self::Unmeasured => None,
        }
    }

    /// Der Schaetzwert — **fuer die Anzeige, nicht fuer die Entscheidung**.
    #[must_use]
    pub const fn estimate(self) -> Option<f64> {
        match self {
            Self::Exact(p) => Some(p),
            Self::Sampled { estimate, .. } => Some(estimate),
            Self::Unmeasured => None,
        }
    }

    /// Ob die Schwelle erfuellt ist.
    ///
    /// **Eine fehlende Messung erfuellt nichts.**
    #[must_use]
    pub fn meets(self, threshold: f64) -> bool {
        self.decisive().is_some_and(|value| value >= threshold)
    }
}

// ------------------------------------------------------------ Siegbedingung

/// Woran sich der Sieg bemisst (AC2).
///
/// Der Simulator fuehrt diese Unterscheidung **nicht**: `fight.rs` setzt
/// `killed_at`, sobald der ausgeteilte Schaden die Lebenspunkte des Bosses
/// erreicht, ohne Fallunterscheidung. Bei einem Boss mit umgekehrtem Ziel ist
/// genau das die Niederlage. Die Umkehrung steht im Spiel
/// (`combat.ts::checkDefeat`) und ab hier auch hier — wer `BalanceResult::kills`
/// roh nimmt, zaehlt gegen `boss-2-3` die verlorenen Durchgaenge als Siege.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinCondition {
    /// Gewoehnlich: Ring auf null ist der Sieg, die abgelaufene Frist die
    /// Niederlage.
    RingDepleted,
    /// Umgekehrt (`runaway`): Die ueberstandene Frist ist der Sieg, der
    /// aufgebrauchte Ring die Niederlage.
    SurviveTimer,
}

impl WinCondition {
    /// Die Siegbedingung dieses Bosses.
    ///
    /// Sie haengt am Feld `runaway` — dasselbe Merkmal, an dem `combat.ts` sie
    /// festmacht.
    #[must_use]
    pub const fn of(boss: &Boss) -> Self {
        if boss.runaway.is_some() {
            Self::SurviveTimer
        } else {
            Self::RingDepleted
        }
    }

    /// Ob dieser Boss das Ziel umkehrt.
    #[must_use]
    pub const fn is_inverted(self) -> bool {
        matches!(self, Self::SurviveTimer)
    }
}

/// Wie ein einzelner Durchgang ausging.
///
/// Drei Ausgaenge, wie im Spiel: gewonnen, am Ueberdruck gescheitert, gestorben.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Gewonnen.
    Won,
    /// Am Ueberdruck gescheitert.
    Overpressure,
    /// Der Spieler ist gestorben.
    Died,
}

impl Outcome {
    /// Der Ausgang eines Durchgangs unter dieser Siegbedingung.
    ///
    /// # Die Reihenfolge ist nicht frei
    ///
    /// Erst der Boss, dann der Spieler, dann die Frist — dieselbe Reihenfolge
    /// wie in `combat.ts::checkDefeat` und in `Fight::replay`. Fallen Boss und
    /// Spieler im selben Takt, gewinnt der Spieler. Wer hier anders prueft,
    /// laesst beide Seiten genau in dem Takt auseinanderlaufen, den niemand
    /// vorhersieht.
    ///
    /// # Der eigene Tod kehrt sich NICHT um
    ///
    /// Die Umkehrung dreht das **Ziel** (Ring gegen Frist). Der Tod des
    /// Spielers ist in beiden Faellen dasselbe Ende. Ein `if inverted` an
    /// dieser Stelle hiesse, dass ein toter Spieler gewinnen kann, wenn der
    /// Boss davonlaeuft.
    #[must_use]
    pub fn of(sample: &Sample, condition: WinCondition) -> Self {
        if sample.killed_at_ms.is_some() {
            return if condition.is_inverted() {
                Self::Overpressure
            } else {
                Self::Won
            };
        }
        if sample.died {
            return Self::Died;
        }
        if condition.is_inverted() {
            Self::Won
        } else {
            Self::Overpressure
        }
    }

    /// Ob dieser Ausgang ein Sieg ist.
    #[must_use]
    pub const fn is_win(self) -> bool {
        matches!(self, Self::Won)
    }
}

// ------------------------------------------------------------- Der Vertrag

/// Die Schwellen der harten Bedingungen.
///
/// **Felder des Laufvertrags, keine globalen Konstanten** (AC5). Ein
/// Endgame-Boss darf eine andere Sicherheit verlangen als einer der ersten
/// Instanz.
///
/// # `None` heisst „hier nicht gefordert", nicht „egal"
///
/// Eine Schwelle auf `None` schaltet die Bedingung fuer diesen Lauf ab. Das
/// ist etwas anderes als eine geforderte Bedingung ohne Messwert: Die eine
/// wird nicht geprueft, die andere macht den Kandidaten unzulaessig. Wer beides
/// vermengt, laesst ungemessene Kandidaten bestehen und nennt es Absicht.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeasibilityThresholds {
    /// Geforderte Siegwahrscheinlichkeit.
    pub min_win_probability: Option<f64>,
    /// Geforderte Wahrscheinlichkeit, dass die Pflicht-Mechaniken geloest sind.
    pub min_mechanic_probability: Option<f64>,
}

impl FeasibilityThresholds {
    /// Die Schwellen aus Issue #939 — **Ausgangswert, nicht Zwang**.
    ///
    /// Sie stehen hier, damit ein Lauf sie nicht jedes Mal neu erfinden muss.
    /// Wer andere braucht, setzt andere und zaehlt [`RunContract::version`]
    /// hoch.
    ///
    /// # Warum der Mechanikerfolg trotzdem `None` ist
    ///
    /// Das Issue nennt 99,9 Prozent. Gemessen am Bestand verlaesst **kein**
    /// Mechanikwert den Kampf: `RunResult` hat zehn Felder, keines davon eine
    /// Mechanik-Kennzahl. Stuende hier 0,999, waere jeder Kandidat unzulaessig
    /// — nicht weil er scheitert, sondern weil niemand hinsieht. Das waere
    /// eine Sperre, die aussieht wie eine Pruefung.
    ///
    /// Die Schwelle gehoert eingeschaltet, sobald der Kampf die Groesse
    /// herausgibt. Bis dahin ist die Luecke hier sichtbar statt versteckt.
    pub const DECIDED: Self = Self {
        min_win_probability: Some(0.99),
        min_mechanic_probability: None,
    };
}

/// Ob der Kampf dieses Laufs eine Zufallsquelle hat (AC4b).
///
/// # Warum das im Vertrag steht und nicht geraten wird
///
/// „Wird sie exakt gerechnet — der Regelfall —, gibt es keine Konfidenzgrenze
/// und der exakte Wert entscheidet unmittelbar" (Issue #939, AC4b). In diesem
/// Projekt ist das die uebliche Lage: Solange die Reinheit auf null steht, hat
/// der Kampf keinen Zufallsstrom, und zwanzig Durchgaenge sind zwanzigmal
/// derselbe Durchgang.
///
/// Wer sie trotzdem als Stichprobe behandelt, bekommt aus zwanzig gewonnenen
/// Durchgaengen eine untere Grenze von rund 0,83 — und damit waere ein
/// Kandidat unzulaessig, der **jeden** Durchgang gewinnt. Das waere keine
/// Vorsicht, sondern ein Rechenfehler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Determinism {
    /// Ohne Zufallsquelle — der Anteil **ist** die Wahrscheinlichkeit.
    Exact,
    /// Mit Zufallsquelle — der Anteil ist eine Stichprobe und bekommt eine
    /// untere Grenze.
    Stochastic,
}

/// Das Band, in dem ein Wert liegen soll (Stufe 4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetBand {
    /// Untere Kante, eingeschlossen.
    pub low: f64,
    /// Obere Kante, eingeschlossen.
    pub high: f64,
}

impl TargetBand {
    /// Der Abstand zum Band — **null innerhalb**, sonst der Weg bis zur Kante.
    ///
    /// Nicht der Abstand zur Mitte: Ein Band ist ein Bereich, in dem es keinen
    /// besseren und keinen schlechteren Platz gibt. Wer zur Mitte misst, macht
    /// aus dem Band heimlich einen Zielwert.
    #[must_use]
    pub fn distance(&self, value: f64) -> f64 {
        if value < self.low {
            self.low - value
        } else if value > self.high {
            value - self.high
        } else {
            0.0
        }
    }
}

/// Was an Abdeckung nachgewiesen sein muss (AC1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageObligations {
    /// Wie viele **verschiedene** Loesungsfamilien je Pflicht-Mechanik
    /// gefordert sind.
    pub solution_families_per_mechanic: u32,
    /// Ob jeder Zauber eine tragfaehige Nische haben muss.
    pub every_spell_needs_a_niche: bool,
    /// Ob jedes Schluesseltalent vertreten sein muss.
    pub every_key_talent_represented: bool,
    /// Ob jeder vorgesehene Ausruestungs-Archetyp vertreten sein muss.
    pub every_gear_archetype_represented: bool,
}

/// Was Robustheit verlangt (AC1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustnessRequirements {
    /// Ueber wie viele Begegnungen die Erfolgsquote halten muss.
    ///
    /// **Eins ist keine Robustheit.** Eine Kurve an einem einzigen Boss sagt
    /// nichts darueber, ob sie verallgemeinerbar ist.
    pub min_encounters: u32,
    /// Welche Erfolgswahrscheinlichkeit in **jeder** dieser Begegnungen
    /// mindestens gelten muss.
    pub min_probability_per_encounter: f64,
}

/// Was eine Aenderung an den Daten kostet (AC1, AC7).
///
/// # Die vorletzte Stufe ist keine Feinheit
///
/// Zwischen zwei gleichwertigen Loesungen gewinnt die, die weniger Zahlen im
/// Spiel veraendert. Jede geaenderte Zahl entwertet ein Stueck der bereits
/// geleisteten Balance-Arbeit — und die Arbeit ist nicht im Ergebnis sichtbar,
/// weshalb sie ohne diese Stufe gratis waere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChangeCostRule {
    /// Was eine geaenderte Zahl kostet.
    pub cost_per_changed_value: f64,
}

/// Der Laufvertrag: **versioniert**, weil er die Vergleichbarkeit traegt (AC5).
///
/// Zwei Ergebnisse aus verschiedenen Vertragsversionen sind nicht vergleichbar.
/// Die Versionsnummer steht deshalb im Vertrag und nicht im Kopf desjenigen,
/// der ihn geaendert hat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunContract {
    /// Version dieses Vertrags.
    pub version: u32,
    /// Die Schwellen der harten Bedingungen.
    pub thresholds: FeasibilityThresholds,
    /// Ob der Kampf dieses Laufs eine Zufallsquelle hat.
    pub determinism: Determinism,
    /// Das Zielband fuer Stufe 4, falls dieser Lauf eines hat.
    pub target_band: Option<TargetBand>,
    /// Die Lebensgrenze des Spielers, falls bekannt — Nenner der
    /// Ueberlebensreserve (Stufe 2).
    ///
    /// Sie steht im Vertrag und nicht im Kampfergebnis, weil das Ergebnis sie
    /// nicht herausgibt. Fehlt sie, bleibt Stufe 2 ungemessen statt geraten.
    pub vessel_max: Option<u32>,
}

/// Der Zielvertrag (AC1).
///
/// Fuenf Bestandteile — und die Reihenfolge im Typ ist die Reihenfolge der
/// Wirkung: Erst muessen die harten Bedingungen gelten, dann ordnet die
/// Rangfolge, und die drei uebrigen Felder sagen, was dabei nachgewiesen sein
/// muss.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BalanceObjective {
    /// Was gelten MUSS — sonst ist der Kandidat ungueltig.
    pub hard_constraints: FeasibilityThresholds,
    /// Die Rangfolge, **keine Gewichte**.
    pub lexicographic_dimensions: [RankingStage; STAGE_COUNT],
    /// Was nachgewiesen sein muss.
    pub coverage_obligations: CoverageObligations,
    /// Was Robustheit verlangt.
    pub robustness_requirements: RobustnessRequirements,
    /// Wie teuer eine Aenderung an den Daten ist.
    pub designer_change_cost: ChangeCostRule,
}

impl BalanceObjective {
    /// Der entschiedene Vertrag aus Issue #939.
    ///
    /// Die Rangfolge ist Vorgabe des Auftraggebers und wird hier umgesetzt,
    /// nicht gewaehlt.
    #[must_use]
    pub const fn decided() -> Self {
        Self {
            hard_constraints: FeasibilityThresholds::DECIDED,
            lexicographic_dimensions: RANKING_ORDER,
            coverage_obligations: CoverageObligations {
                solution_families_per_mechanic: 2,
                every_spell_needs_a_niche: true,
                every_key_talent_represented: true,
                every_gear_archetype_represented: true,
            },
            robustness_requirements: RobustnessRequirements {
                min_encounters: 2,
                min_probability_per_encounter: 0.99,
            },
            designer_change_cost: ChangeCostRule {
                cost_per_changed_value: 1.0,
            },
        }
    }
}

// ------------------------------------------------------------- Drei Ebenen

/// Ob eine Grenze der Parameterdomaene eingehalten ist.
///
/// Die Domaenen selbst sind nicht Sache dieses Moduls (Issue #939, out of
/// scope) — hier steht nur, ob sie eingehalten wurden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundsVerdict {
    /// Innerhalb aller Grenzen.
    Within,
    /// Ausserhalb — mit dem Parameter, an dem es lag.
    Violated {
        /// Welcher Parameter.
        parameter: &'static str,
    },
    /// Nicht geprueft.
    Unmeasured,
}

/// Was fuer **einen** Kandidaten gelten muss (AC2, Ebene 1).
///
/// Ein einzelner Kandidat kann gar nicht erfuellen, dass *jeder Zauber* eine
/// Nische hat — das ist eine Eigenschaft der Loesungsmenge. Deshalb drei
/// Ebenen und nicht eine Liste.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateFeasibility {
    /// Die Siegbedingung dieser Begegnung — **bei umgekehrtem Ziel die
    /// umgekehrte**.
    pub encounter_win_condition: Probability,
    /// Ob die Pflicht-Mechaniken geloest wurden.
    pub mandatory_mechanics_solved: Probability,
    /// Ob die Parametergrenzen eingehalten sind.
    pub parameter_bounds: BoundsVerdict,
}

/// Ein Nachweis in Form „so viele von so vielen".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// Nachgewiesen.
    Measured {
        /// Wie viele abgedeckt sind.
        covered: u32,
        /// Wie viele gefordert sind.
        required: u32,
    },
    /// Nicht nachgewiesen — **gilt als nicht erfuellt**.
    Unmeasured,
}

impl Coverage {
    /// Ob der Nachweis erbracht ist.
    #[must_use]
    pub const fn is_met(self) -> bool {
        match self {
            Self::Measured { covered, required } => covered >= required,
            Self::Unmeasured => false,
        }
    }
}

/// Was fuer die **Menge** der Loesungen gelten muss (AC2, Ebene 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioConstraints {
    /// Jede Mechanik hat mehrere Loesungsfamilien.
    pub every_mechanic_has_multiple_solution_families: Coverage,
    /// Keine Schule ist ueberall Pflicht.
    pub no_school_is_globally_mandatory: Coverage,
    /// Die Loesungen halten ueber die Begegnungen.
    pub robustness_across_encounters: Coverage,
}

impl PortfolioConstraints {
    /// Ob alle drei Bedingungen erfuellt sind.
    #[must_use]
    pub const fn all_met(self) -> bool {
        self.every_mechanic_has_multiple_solution_families.is_met()
            && self.no_school_is_globally_mandatory.is_met()
            && self.robustness_across_encounters.is_met()
    }
}

/// Was fuer den **Lauf als Ganzes** gelten muss (AC2, Ebene 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignCoverage {
    /// Jeder Zauber hat eine tragfaehige Nische.
    pub every_spell_has_viable_niche: Coverage,
    /// Jedes Schluesseltalent ist vertreten.
    pub every_key_talent_is_represented: Coverage,
    /// Jeder vorgesehene Ausruestungs-Archetyp ist vertreten.
    pub every_intended_gear_archetype_is_represented: Coverage,
}

impl CampaignCoverage {
    /// Ob alle drei Nachweise erbracht sind.
    #[must_use]
    pub const fn all_met(self) -> bool {
        self.every_spell_has_viable_niche.is_met()
            && self.every_key_talent_is_represented.is_met()
            && self.every_intended_gear_archetype_is_represented.is_met()
    }
}

// --------------------------------------------------------------- Rangfolge

/// Wie viele Stufen die Rangfolge hat.
pub const STAGE_COUNT: usize = 8;

/// Die Stufen der Rangfolge (AC6).
///
/// **Die Reihenfolge ist Vorgabe.** Grob gelesen: Stufen 1 und 2 sind
/// Ueberleben, Stufe 5 ist Schaden, Stufe 8 sind Ressourcen — Ueberleben
/// fuehrt, Schaden folgt, Ressourcen entscheiden zuletzt.
///
/// „Sieg" steht **nicht** darunter. Er ist Zulaessigkeitsbedingung und taucht
/// genau einmal auf (AC3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RankingStage {
    /// 1 — Erfolgswahrscheinlichkeit ueber der Mindestgrenze.
    ///
    /// Messvorschrift: entscheidender Wert der Siegwahrscheinlichkeit minus
    /// geforderte Schwelle. Groesser ist besser. Ohne geforderte Schwelle die
    /// Wahrscheinlichkeit selbst.
    SuccessMargin,
    /// 2 — Ueberlebensreserve.
    ///
    /// Messvorschrift: `1 - durchgekommener Schaden / Lebensgrenze`, gemittelt
    /// ueber die Durchgaenge. Groesser ist besser.
    ///
    /// **Was diese Groesse NICHT ist:** der kleinste Lebensstand waehrend des
    /// Kampfes. Sie rechnet mit der Summe des durchgekommenen Schadens; ein
    /// Kampf, der zwischendurch fast toedlich war und wieder hochgeheilt wurde,
    /// sieht hier harmloser aus, als er war. Die genauere Groesse braucht den
    /// Kampfverlauf, den ein Balance-Ergebnis nicht traegt.
    SurvivalReserve,
    /// 3 — Mechanikqualitaet.
    ///
    /// Messvorschrift: der **kleinste** Sicherheitsabstand zu einer
    /// Mechanik-Schwelle ueber den Kampf, auf die Schwelle normiert. Ein Boss
    /// mit einem toedlichen Zaehler, der hoechstens 60 Prozent seiner Schwelle
    /// erreicht, hat 0,4. Groesser ist besser. Ueber mehrere Mechaniken das
    /// Minimum, nicht der Mittelwert — die knappste entscheidet.
    ///
    /// **Heute ungemessen.** Kein Mechanikwert verlaesst den Kampf: `doom`
    /// kommt 58-mal in `fight.rs` vor und steht in keinem Ergebnisfeld. Der
    /// Kampfkern ist in Issue #939 ausdruecklich ausgenommen; die Speisung
    /// gehoert in ein Folge-Issue.
    ///
    /// Nicht gestrichen, weil die Groesse nicht undefinierbar ist, sondern nur
    /// ungespeist — und weil eine gestrichene Stufe die Vergleichbarkeit mit
    /// jedem spaeteren Lauf kostet.
    MechanicQuality,
    /// 4 — Abstand zum Zielband.
    ///
    /// Messvorschrift: **negativer** Abstand des Durchsatzes zum Band aus dem
    /// Laufvertrag, damit auch hier groesser besser ist. Innerhalb des Bandes
    /// null.
    TargetBandDistance,
    /// 5 — Zeit bis zum Sieg beziehungsweise zur Eindaemmung.
    ///
    /// Messvorschrift bei gewoehnlicher Siegbedingung: **negative** mittlere
    /// Siegzeit — schneller ist besser.
    ///
    /// Bei umgekehrtem Ziel ist die Siegzeit die Frist und damit fuer jeden
    /// Sieger gleich; sie entscheidet dort nichts. Die Entsprechung ist die
    /// Zeit bis zur **Eindaemmung**: der frueheste Zeitpunkt, ab dem der Ring
    /// nicht mehr steigt. Sie braucht den Kampfverlauf und bleibt deshalb aus
    /// einem Balance-Ergebnis ungemessen.
    ///
    /// **Die Rotationssuche braucht trotzdem eine Zahl** und nimmt dort seit
    /// #891 N1 den Ring am Fristende (Festlegung des Orchestrators vom
    /// 2026-10-29: Reserve = Lebenspunkte minus Fortschritt, eine Niederlage
    /// hoechstens 0; [`crate::rotation_search::bewerte`]). Diese Stufe bleibt
    /// davon unberuehrt: Sie misst die Eindaemmung, und die ist weiter
    /// ungemessen.
    TimeToWin,
    /// 6 — Robustheit.
    ///
    /// Messvorschrift: die **kleinste** Siegwahrscheinlichkeit ueber alle
    /// geforderten Begegnungen. Groesser ist besser. Aus einem einzelnen Lauf
    /// nicht bestimmbar — dafuer gibt es
    /// [`CandidateMeasurement::with_robustness`].
    Robustness,
    /// 7 — moeglichst kleine Aenderung an den Daten.
    ///
    /// Messvorschrift: **negative** Summe der Aenderungskosten. Weniger
    /// geaenderte Zahlen sind besser.
    DesignerChangeCost,
    /// 8 — Ressourceneffizienz.
    ///
    /// Messvorschrift: **negativer** Anteil der am Deckel verpufften Ressource,
    /// hilfsweise die Leerlaufzeit. Beides fuehrt `RunResult` mit
    /// (`resource_overflow`, `idle_ms`); beim Zusammenfassen zu einem
    /// `BalanceResult` wird es heute verworfen. Bis das anders ist: ungemessen.
    ResourceEfficiency,
}

/// Die Rangfolge in der entschiedenen Reihenfolge.
pub const RANKING_ORDER: [RankingStage; STAGE_COUNT] = [
    RankingStage::SuccessMargin,
    RankingStage::SurvivalReserve,
    RankingStage::MechanicQuality,
    RankingStage::TargetBandDistance,
    RankingStage::TimeToWin,
    RankingStage::Robustness,
    RankingStage::DesignerChangeCost,
    RankingStage::ResourceEfficiency,
];

impl RankingStage {
    /// Der Platz in der Rangfolge, beginnend bei 1.
    #[must_use]
    pub const fn rank(self) -> usize {
        match self {
            Self::SuccessMargin => 1,
            Self::SurvivalReserve => 2,
            Self::MechanicQuality => 3,
            Self::TargetBandDistance => 4,
            Self::TimeToWin => 5,
            Self::Robustness => 6,
            Self::DesignerChangeCost => 7,
            Self::ResourceEfficiency => 8,
        }
    }

    /// Der Kurzname fuer Ausgaben.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SuccessMargin => "Erfolgswahrscheinlichkeit",
            Self::SurvivalReserve => "Ueberlebensreserve",
            Self::MechanicQuality => "Mechanikqualitaet",
            Self::TargetBandDistance => "Abstand zum Zielband",
            Self::TimeToWin => "Zeit bis Sieg oder Eindaemmung",
            Self::Robustness => "Robustheit",
            Self::DesignerChangeCost => "Aenderung an den Daten",
            Self::ResourceEfficiency => "Ressourceneffizienz",
        }
    }
}

/// Der Wert einer Stufe.
///
/// **Groesser ist immer besser.** Groessen, bei denen klein besser ist (Zeit,
/// Abstand, Kosten), werden beim Fuellen negiert — einmal an der Quelle statt
/// achtmal im Vergleich.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StageValue {
    /// Gemessen.
    Measured(f64),
    /// Nicht gemessen.
    ///
    /// Die Stufe **entscheidet dann nichts** und wird uebersprungen. Sie
    /// bekommt weder den besten noch den schlechtesten Platz: Beides waere
    /// eine Aussage ueber einen Wert, den niemand kennt.
    Unmeasured,
}

impl StageValue {
    /// Der gemessene Wert, falls es einen gibt.
    #[must_use]
    pub const fn value(self) -> Option<f64> {
        match self {
            Self::Measured(v) => Some(v),
            Self::Unmeasured => None,
        }
    }
}

/// Die acht Stufenwerte eines Kandidaten.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LexicographicScore {
    stages: [StageValue; STAGE_COUNT],
}

impl Default for LexicographicScore {
    fn default() -> Self {
        Self::unmeasured()
    }
}

impl LexicographicScore {
    /// Alle Stufen ungemessen — der ehrliche Ausgangszustand.
    #[must_use]
    pub const fn unmeasured() -> Self {
        Self {
            stages: [StageValue::Unmeasured; STAGE_COUNT],
        }
    }

    /// Der Wert einer Stufe.
    #[must_use]
    pub const fn get(&self, stage: RankingStage) -> StageValue {
        self.stages[stage.rank() - 1]
    }

    /// Setzt den Wert einer Stufe.
    pub const fn set(&mut self, stage: RankingStage, value: StageValue) {
        self.stages[stage.rank() - 1] = value;
    }

    /// Wie viele Stufen einen Messwert tragen.
    #[must_use]
    pub fn measured_count(&self) -> usize {
        self.stages
            .iter()
            .filter(|v| matches!(v, StageValue::Measured(_)))
            .count()
    }
}

/// Eine Menge von Stufen, als Bitmaske.
///
/// Steht neben dem Vergleichsergebnis, damit sichtbar ist, **welche** Stufen
/// nichts entscheiden konnten. Eine Zahl „drei uebersprungen" waere zu wenig:
/// Ob die Ueberlebensreserve oder die Ressourceneffizienz geschwiegen hat,
/// sind zwei sehr verschiedene Lagen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StageSet(u8);

impl StageSet {
    /// Die leere Menge.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Nimmt eine Stufe auf.
    pub const fn insert(&mut self, stage: RankingStage) {
        self.0 |= 1 << (stage.rank() - 1);
    }

    /// Ob die Stufe enthalten ist.
    #[must_use]
    pub const fn contains(self, stage: RankingStage) -> bool {
        self.0 & (1 << (stage.rank() - 1)) != 0
    }

    /// Ob die Menge leer ist.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Wie viele Stufen enthalten sind.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }
}

/// Was einen Vergleich entschieden hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Die **Zulaessigkeit** entschied — keine Stufe kam zum Zug (AC3, AC8).
    Feasibility,
    /// Diese Stufe entschied.
    Stage(RankingStage),
    /// Keine Stufe konnte entscheiden.
    Tie,
}

/// Das Ergebnis eines Vergleichs, **mit Begruendung**.
///
/// # Warum kein `Ord`
///
/// Ungemessene Stufen werden uebersprungen. Damit ist der Vergleich nicht
/// transitiv: Steht A auf Stufe 1 bei 5, B ungemessen und C bei 3, dann gilt
/// A > C, waehrend A und B sowie B und C auf dieser Stufe gleichstehen. Ein
/// `Ord` waere an dieser Stelle eine Zusicherung, die nicht gilt — und
/// `sort_by` wuerde still eine beliebige Reihenfolge liefern.
///
/// Wer sortieren will, muss zuerst dafuer sorgen, dass die entscheidenden
/// Stufen gemessen sind. [`RankingComparison::silent_stages`] sagt, welche
/// fehlen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankingComparison {
    /// Die Ordnung: [`Ordering::Greater`] heisst, der erste Kandidat ist
    /// besser.
    pub ordering: Ordering,
    /// Was entschieden hat.
    pub decided_by: Decision,
    /// Welche Stufen uebersprungen wurden, weil ein Wert fehlte.
    pub silent_stages: StageSet,
}

// ------------------------------------------------------- Messung und Urteil

/// Warum ein Kandidat unzulaessig ist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Violation {
    /// Die Siegwahrscheinlichkeit liegt unter der Schwelle.
    WinProbabilityTooLow {
        /// Der entscheidende Wert, oder `None`, wenn gar nicht gemessen wurde.
        decisive: Option<f64>,
        /// Die geforderte Schwelle.
        threshold: f64,
    },
    /// Der Mechanikerfolg liegt unter der Schwelle.
    MechanicProbabilityTooLow {
        /// Der entscheidende Wert, oder `None`, wenn gar nicht gemessen wurde.
        decisive: Option<f64>,
        /// Die geforderte Schwelle.
        threshold: f64,
    },
    /// Eine Parametergrenze ist verletzt.
    ParameterOutOfBounds {
        /// Welcher Parameter.
        parameter: &'static str,
    },
    /// Die Parametergrenzen wurden nicht geprueft.
    ParameterBoundsUnmeasured,
}

/// Alles, was ueber einen Kandidaten gemessen wurde.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateMeasurement {
    /// Die harten Groessen.
    pub feasibility: CandidateFeasibility,
    /// Die Stufenwerte.
    pub stages: LexicographicScore,
}

/// Das Urteil ueber einen Kandidaten.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateVerdict {
    /// Ob der Kandidat zulaessig ist.
    pub feasible: bool,
    /// Woran es liegt, wenn nicht.
    pub violations: Vec<Violation>,
    /// Die Stufenwerte — **auch bei einem unzulaessigen Kandidaten**.
    ///
    /// Sie werden nicht geloescht: Ein unzulaessiger Kandidat, der auf Stufe 5
    /// glaenzt, ist eine nuetzliche Beobachtung. Er darf nur nicht gewinnen.
    pub score: LexicographicScore,
}

impl BalanceObjective {
    /// Prueft die harten Bedingungen (AC3, AC8).
    ///
    /// Die Rangfolge kommt hier **nicht** vor. Sie beginnt unterhalb der
    /// Zulaessigkeit, und diese Trennung ist der Zweck des ganzen Moduls.
    #[must_use]
    pub fn judge(&self, measurement: &CandidateMeasurement) -> CandidateVerdict {
        let mut violations = Vec::new();
        let feasibility = &measurement.feasibility;

        if let Some(threshold) = self.hard_constraints.min_win_probability {
            if !feasibility.encounter_win_condition.meets(threshold) {
                violations.push(Violation::WinProbabilityTooLow {
                    decisive: feasibility.encounter_win_condition.decisive(),
                    threshold,
                });
            }
        }
        if let Some(threshold) = self.hard_constraints.min_mechanic_probability {
            if !feasibility.mandatory_mechanics_solved.meets(threshold) {
                violations.push(Violation::MechanicProbabilityTooLow {
                    decisive: feasibility.mandatory_mechanics_solved.decisive(),
                    threshold,
                });
            }
        }
        match feasibility.parameter_bounds {
            BoundsVerdict::Within => {}
            BoundsVerdict::Violated { parameter } => {
                violations.push(Violation::ParameterOutOfBounds { parameter });
            }
            BoundsVerdict::Unmeasured => {
                violations.push(Violation::ParameterBoundsUnmeasured);
            }
        }

        CandidateVerdict {
            feasible: violations.is_empty(),
            violations,
            score: measurement.stages,
        }
    }

    /// Vergleicht zwei Kandidaten (AC3, AC6, AC8).
    ///
    /// # Die Zulaessigkeit entscheidet zuerst, und sie ist keine Stufe
    ///
    /// Ein unzulaessiger Kandidat verliert gegen jeden zulaessigen — unabhaengig
    /// von jeder Zahl in jeder Dimension. Das ist AC8, und es ist der Grund,
    /// warum „Sieg" nicht zusaetzlich in der Rangfolge steht: Stuende er dort,
    /// koennte ein unzulaessiger Kandidat ueber eine gute Zahl in einer unteren
    /// Stufe gewinnen.
    ///
    /// Zwei unzulaessige Kandidaten werden **nicht** gerangt. Sie stehen
    /// gleich, und das Ergebnis sagt es: [`Decision::Feasibility`] mit
    /// [`Ordering::Equal`]. Wer sie doch sortiert, sortiert Kandidaten, die
    /// alle ausgeschieden sind.
    #[must_use]
    pub fn compare(&self, a: &CandidateVerdict, b: &CandidateVerdict) -> RankingComparison {
        if a.feasible != b.feasible {
            return RankingComparison {
                ordering: if a.feasible {
                    Ordering::Greater
                } else {
                    Ordering::Less
                },
                decided_by: Decision::Feasibility,
                silent_stages: StageSet::empty(),
            };
        }
        if !a.feasible {
            return RankingComparison {
                ordering: Ordering::Equal,
                decided_by: Decision::Feasibility,
                silent_stages: StageSet::empty(),
            };
        }

        let mut silent = StageSet::empty();
        for stage in self.lexicographic_dimensions {
            let (Some(left), Some(right)) =
                (a.score.get(stage).value(), b.score.get(stage).value())
            else {
                silent.insert(stage);
                continue;
            };
            // `partial_cmp` statt `total_cmp`: Ein NaN in einem Stufenwert ist
            // ein Fehler in der Speisung, kein Rangplatz. Er darf nicht
            // stillschweigend als „kleiner als alles" durchgehen — die Stufe
            // schweigt dann, und das steht im Ergebnis.
            let Some(ordering) = left.partial_cmp(&right) else {
                silent.insert(stage);
                continue;
            };
            if ordering != Ordering::Equal {
                return RankingComparison {
                    ordering,
                    decided_by: Decision::Stage(stage),
                    silent_stages: silent,
                };
            }
        }
        RankingComparison {
            ordering: Ordering::Equal,
            decided_by: Decision::Tie,
            silent_stages: silent,
        }
    }
}

// -------------------------------------------------------------- Die Speisung

/// Was die Rechnung aus einem Lauf liest — gleich, woher er kommt.
///
/// Fuenf Groessen und nicht der ganze Ergebnissatz: Genau diese fuenf haelt
/// auch der Ergebnisspeicher. Was hier steht, laesst sich deshalb ohne einen
/// einzigen neuen Kampf neu bewerten.
struct Kennzahlen<'a> {
    /// Gewonnene Durchgaenge, mit der Umkehrung des Ziels.
    wins: usize,
    /// Gerechnete Durchgaenge.
    trials: u32,
    /// Mittlerer ausgeteilter Schaden (Stufe 4).
    damage_mean: f64,
    /// Mittlerer durchgekommener Schaden (Stufe 2).
    damage_taken_mean: f64,
    /// Zeit bis zum Sieg ueber die gewonnenen Durchgaenge (Stufe 5).
    kill_time: &'a Statistics,
}

impl CandidateMeasurement {
    /// Aus einem echten Balance-Ergebnis (AC2, AC4).
    ///
    /// # Die Siegzahl wird hier NICHT uebernommen, sondern neu bestimmt
    ///
    /// `BalanceResult::kills` zaehlt, wie oft der ausgeteilte Schaden die
    /// Lebenspunkte erreicht hat. Bei einem Boss mit umgekehrtem Ziel ist das
    /// die Niederlage. Diese Funktion geht deshalb ueber die Einzelergebnisse
    /// und bestimmt jeden Ausgang mit [`Outcome::of`].
    ///
    /// # Was ungemessen bleibt, und warum
    ///
    /// Mechanikqualitaet, Robustheit und Ressourceneffizienz. Die Gruende
    /// stehen an der jeweiligen Stufe in [`RankingStage`]; keiner davon ist
    /// „vergessen", alle drei liegen hinter der Grenze, die Issue #939 um den
    /// Kampfkern zieht.
    #[must_use]
    pub fn from_balance_result(
        result: &BalanceResult,
        boss: &Boss,
        contract: &RunContract,
        change_cost: Option<f64>,
    ) -> Self {
        let condition = WinCondition::of(boss);
        let trials = u32::try_from(result.samples.len()).unwrap_or(u32::MAX);
        let wins = result
            .samples
            .iter()
            .filter(|sample| Outcome::of(sample, condition).is_win())
            .count();
        Self::aus_kennzahlen(
            &Kennzahlen {
                wins,
                trials,
                damage_mean: result.damage.mean,
                damage_taken_mean: result.damage_taken.mean,
                kill_time: &result.kill_time_ms,
            },
            condition,
            contract,
            change_cost,
        )
    }

    /// Aus einem **rohen Ergebnis des Ergebnisspeichers** (Epic #903).
    ///
    /// # Wofuer es das gibt
    ///
    /// Ein Zielwechsel soll KEINEN Kampf kosten. Dann muss das Urteil aus dem
    /// entstehen, was der Speicher haelt — und der haelt keine
    /// Einzelergebnisse, nur ihre Kennzahlen. Ohne diese Funktion liesse sich ein gespeichertes
    /// Ergebnis nicht neu bewerten, und jeder Zielwechsel liefe doch wieder
    /// ueber [`crate::balance::run`].
    ///
    /// # Die Siegzahl kommt hier aus dem Speicher, nicht aus den Ausgaengen
    ///
    /// [`Self::from_balance_result`] bestimmt jeden Ausgang mit
    /// [`Outcome::of`]; hier liegen die Einzelergebnisse nicht mehr vor. Die
    /// Siegzahl im Vektor ist [`crate::balance::siege`] — dieselbe Lesart mit
    /// derselben Umkehrung bei `runaway`. Dass beide Wege dasselbe Urteil
    /// liefern, haelt `sim/tests/speicher_zielwechsel.rs` an echten Laeufen
    /// gegen einen gewoehnlichen UND einen umgekehrten Boss fest — als
    /// Gleichheit der ganzen Messung, nicht einer einzelnen Zahl.
    ///
    /// Gerechnet wird in **derselben** Funktion wie dort
    /// (`aus_kennzahlen`), nicht in einer zweiten Fassung daneben.
    #[must_use]
    pub fn from_raw_outcome(
        vektor: &RoherErgebnisvektor,
        boss: &Boss,
        contract: &RunContract,
        change_cost: Option<f64>,
    ) -> Self {
        Self::aus_kennzahlen(
            &Kennzahlen {
                wins: vektor.siege.0 as usize,
                trials: vektor.durchgaenge,
                damage_mean: vektor.schaden.mean,
                damage_taken_mean: vektor.erlittener_schaden.mean,
                kill_time: &vektor.siegzeit_ms,
            },
            WinCondition::of(boss),
            contract,
            change_cost,
        )
    }

    /// Die eine Rechnung hinter beiden Speisungen.
    fn aus_kennzahlen(
        werte: &Kennzahlen<'_>,
        condition: WinCondition,
        contract: &RunContract,
        change_cost: Option<f64>,
    ) -> Self {
        let win_probability = Self::win_probability(werte.wins, werte.trials, contract.determinism);

        let mut stages = LexicographicScore::unmeasured();

        // Stufe 1 — Abstand zur geforderten Mindestgrenze.
        if let Some(decisive) = win_probability.decisive() {
            let margin = contract
                .thresholds
                .min_win_probability
                .map_or(decisive, |threshold| decisive - threshold);
            stages.set(RankingStage::SuccessMargin, StageValue::Measured(margin));
        }

        // Stufe 2 — Ueberlebensreserve, nur mit bekannter Lebensgrenze.
        if let Some(vessel_max) = contract.vessel_max {
            if vessel_max > 0 {
                let reserve = 1.0 - werte.damage_taken_mean / f64::from(vessel_max);
                stages.set(RankingStage::SurvivalReserve, StageValue::Measured(reserve));
            }
        }

        // Stufe 4 — negativer Abstand zum Zielband.
        if let Some(band) = contract.target_band {
            stages.set(
                RankingStage::TargetBandDistance,
                StageValue::Measured(-band.distance(werte.damage_mean)),
            );
        }

        // Stufe 5 — nur bei gewoehnlicher Siegbedingung; sonst waere es die
        // Frist, die fuer jeden Sieger gleich ist.
        if !condition.is_inverted() && werte.kill_time.count > 0 {
            stages.set(
                RankingStage::TimeToWin,
                StageValue::Measured(-werte.kill_time.mean),
            );
        }

        // Stufe 7 — kommt vom Kandidaten, nicht aus dem Kampf.
        if let Some(cost) = change_cost {
            stages.set(
                RankingStage::DesignerChangeCost,
                StageValue::Measured(-cost),
            );
        }

        Self {
            feasibility: CandidateFeasibility {
                encounter_win_condition: win_probability,
                mandatory_mechanics_solved: Probability::Unmeasured,
                parameter_bounds: BoundsVerdict::Unmeasured,
            },
            stages,
        }
    }

    /// Die Siegwahrscheinlichkeit — exakt oder als Stichprobe (AC4b).
    ///
    /// # Die Behauptung „exakt" wird geprueft, nicht geglaubt
    ///
    /// Ein deterministischer Lauf hat in **jedem** Durchgang denselben Ausgang.
    /// Streuen die Ausgaenge, war er nicht deterministisch — dann ist
    /// [`Determinism::Exact`] im Vertrag falsch, und diese Funktion faellt auf
    /// die Stichprobe mit unterer Grenze zurueck.
    ///
    /// Der Rueckfall geht in die vorsichtige Richtung: Er kann eine Sicherheit
    /// kleiner machen, nie groesser. Die umgekehrte Bequemlichkeit — der
    /// Behauptung folgen — wuerde aus einer streuenden Messung eine exakte Zahl
    /// machen, und niemand saehe es.
    fn win_probability(wins: usize, trials: u32, determinism: Determinism) -> Probability {
        if trials == 0 {
            return Probability::Unmeasured;
        }
        let einheitlich = wins == 0 || wins == trials as usize;
        if determinism == Determinism::Exact && einheitlich {
            return Probability::Exact(if wins == 0 { 0.0 } else { 1.0 });
        }
        Probability::from_trials(u32::try_from(wins).unwrap_or(u32::MAX), trials)
    }

    /// Setzt die Parametergrenzen.
    ///
    /// Getrennt, weil sie nicht aus dem Kampf kommen: Ob ein Kandidat innerhalb
    /// seiner Domaene liegt, weiss der, der ihn erzeugt hat.
    #[must_use]
    pub const fn with_bounds(mut self, bounds: BoundsVerdict) -> Self {
        self.feasibility.parameter_bounds = bounds;
        self
    }

    /// Setzt den Mechanikerfolg.
    ///
    /// Getrennt aus demselben Grund wie [`Self::with_bounds`] — und heute ohne
    /// Quelle im Simulator.
    #[must_use]
    pub const fn with_mechanics(mut self, probability: Probability) -> Self {
        self.feasibility.mandatory_mechanics_solved = probability;
        self
    }

    /// Setzt die Robustheit aus mehreren Begegnungen (Stufe 6).
    ///
    /// # Warum das Minimum und nicht der Mittelwert
    ///
    /// Robustheit heisst, dass es **nirgends** einbricht. Ein Mittelwert laesst
    /// eine verlorene Begegnung von zwei gewonnenen ausgleichen — und genau
    /// diese Begegnung ist die, um die es geht.
    ///
    /// Weniger Begegnungen als gefordert lassen die Stufe ungemessen: Eine
    /// Robustheit aus einer einzigen Begegnung ist keine.
    #[must_use]
    pub fn with_robustness(
        mut self,
        per_encounter: &[Probability],
        requirements: RobustnessRequirements,
    ) -> Self {
        let needed = requirements.min_encounters as usize;
        if per_encounter.len() < needed.max(1) {
            return self;
        }
        let mut worst = f64::INFINITY;
        for probability in per_encounter {
            let Some(decisive) = probability.decisive() else {
                return self;
            };
            worst = worst.min(decisive);
        }
        self.stages
            .set(RankingStage::Robustness, StageValue::Measured(worst));
        self
    }
}
