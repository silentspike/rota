//! Den **besten** Aufbau suchen, statt einen anzunehmen (Issue #118, AC1).
//!
//! # Warum das nötig ist, und woran es sich zeigt
//!
//! 13.5 verlangt wörtlich „bester Satz-Aufbau gegen **besten** Affix-Aufbau".
//! Ein beliebiger Aufbau als Gegenspieler misst nicht die Frage, sondern die
//! Wahl des Aufbaus.
//!
//! Gemessen am 2026-08-04, Budget 1130 Promille, 200 Durchgänge je Zeile:
//!
//! ```text
//! gemischt zu Dritteln                8840,7 +/- 478,7
//! purity 700 + yield 430              9810,4 +/- 414,2
//! ```
//!
//! Beide sind „gemischt". Der eine verliert gegen jeden reinen Aufbau, der
//! andere schlägt sie alle um fast tausend Punkte. Die erste Fassung des
//! Vergleichs hatte die Gleichverteilung genommen — und schien damit 13.1b zu
//! widersprechen („weil sie sich multiplizieren, will man alle drei mischen").
//! Der Widerspruch war die Bauart, nicht das Design.
//!
//! # Warum eine Suche und keine Formel
//!
//! Der beste Aufbau lässt sich nicht ausrechnen. PURITY hat einen Deckel, ab
//! dem jeder weitere Punkt wertlos ist; YIELD wirkt nur, soweit gekritet wird;
//! CATALYSIS wirkt auf Zeiten und damit auf die Zahl der Treffer. Die drei
//! greifen ineinander, und der Kampf entscheidet.
//!
//! Deshalb wird gerechnet, nicht hergeleitet — und deshalb steht das Ergebnis
//! im Bericht statt in einer Zusicherung.

use crate::apl::Apl;
use crate::balance::{run, BalanceConfig, Statistics};
use crate::gear::WornValues;

/// Wie fein das Budget verteilt wird.
///
/// **Ein Raster, keine vollständige Suche.** Bei 1130 Promille und einem
/// Schritt von 10 gibt es rund 6500 Aufteilungen auf drei Werte; jede über
/// mehrere hundert Durchgänge zu rechnen wäre Stunden. Das Raster von 50
/// Promille lässt 300 Aufteilungen übrig und trifft das Optimum auf 50
/// Promille genau — genug für eine Balance-Aussage, deren Zahlen ohnehin
/// vorläufig sind.
///
/// **Der Wert steht hier und nicht in der Suche**, damit ein feineres Raster
/// eine Zeile ist und keine Umstellung.
pub const SEARCH_STEP_PERMILLE: u32 = 50;

/// Ein durchgerechneter Aufbau samt seinem Ergebnis.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Die getragenen Werte.
    pub worn: WornValues,
    /// Was er über die Durchgänge geleistet hat.
    pub damage: Statistics,
}

impl Candidate {
    /// Ob dieser Aufbau **messbar** besser ist als ein anderer.
    ///
    /// # Nicht der Mittelwert allein
    ///
    /// Seit #177 trägt PURITY echten Zufall (6.5), und die Streuung ist groß:
    /// über 400 Punkte bei einem Abstand von unter hundert. Ein Vergleich der
    /// Mittelwerte allein erklärte dann einen Aufbau zum Sieger, dessen
    /// Vorsprung im Rauschen liegt.
    ///
    /// Verglichen wird deshalb gegen den **Standardfehler**, nicht gegen die
    /// Standardabweichung: Er sagt, wie genau der Mittelwert bekannt ist, und
    /// schrumpft mit der Zahl der Durchgänge. Genau das soll er — wer feiner
    /// messen will, rechnet länger.
    #[must_use]
    pub fn clearly_better_than(&self, other: &Self) -> bool {
        let unschaerfe = self.damage.stderr + other.damage.stderr;
        self.damage.mean > other.damage.mean + unschaerfe
    }
}

/// Alle Aufteilungen eines Budgets auf die drei Sonderwerte, im Raster.
///
/// Topf 4 bleibt leer: Er ist die **andere** Seite des Vergleichs (Satz-Aufbau)
/// und darf sich nicht in den Affix-Aufbau mischen.
fn splits(budget: u32, step: u32) -> Vec<WornValues> {
    let mut out = Vec::new();
    let mut catalysis = 0;
    while catalysis <= budget {
        let mut purity = 0;
        while catalysis + purity <= budget {
            out.push(WornValues {
                catalysis,
                purity,
                yield_bonus: budget - catalysis - purity,
                // Die Aufbau-Suche verteilt das Affix-Budget auf die drei
                // Sonderwerte. Grundwerte gehoeren nicht dazu — sie haengen
                // am Gegenstand, nicht am Budget (Issue #178).
                essence: 0,
                vessel: 0,
                ward: 0,
                gear_effects: 0,
            });
            purity += step;
        }
        catalysis += step;
    }
    out
}

/// Sucht den besten Affix-Aufbau für ein Budget.
///
/// Rechnet jede Aufteilung im Raster über `iterations` Durchgänge und gibt die
/// beste zurück — gemessen am Mittelwert, denn hier geht es um die Spitze und
/// nicht um einen Vergleich zweier Kandidaten.
///
/// # Warum ein Raster und nicht eine Halbierung (Issue #940)
///
/// **Die Rastersuche ist teuer, und sie bleibt es mit Absicht.** Sie geht den
/// Raum in Schritten von [`SEARCH_STEP_PERMILLE`] ab und überspringt nichts;
/// der Vergleich unten verfolgt nur das Maximum und verwirft keine Region.
/// Damit setzt sie **keine Monotonie voraus** — und kann deshalb an einem Boss
/// mit umgekehrter Bedingung nicht das falsche Optimum finden.
///
/// Eine Halbierung wäre schneller und hier **falsch**: Sie setzt voraus, dass
/// mehr Wert ein besseres Ergebnis liefert. Das gilt bezüglich erwartetem
/// Schaden bei stabilem Trace — und nicht bezüglich Sieg an einem Boss mit
/// umgekehrter Bedingung, Zeit bis zum Sieg über einen Phasenwechsel oder
/// eines Ziels mit Lebenspunkt-Prädikat.
///
/// > Wer hier halbiert oder Kandidaten per Dominanz verwirft, muss vorher
/// > [`crate::domaene::ParameterDomain::binaersuche_erlaubt`] fragen. Ohne
/// > vollständiges Zertifikat lehnt sie ab, und die Ablehnung nennt ihren
/// > Grund.
///
/// Der Hinweis steht hier und nicht nur in `domaene.rs`, weil dies die Stelle
/// ist, die jemand liest, der die Laufzeit senken will.
///
/// # Panics
///
/// Nie: `splits` liefert für jedes Budget mindestens eine Aufteilung (die mit
/// allem in YIELD), und der Vergleich läuft über eine nichtleere Liste.
#[must_use]
pub fn best_affix_build(budget: u32, iterations: u32, apl: &Apl) -> Candidate {
    let mut best: Option<Candidate> = None;
    for worn in splits(budget, SEARCH_STEP_PERMILLE) {
        let ergebnis = run(
            &BalanceConfig {
                iterations,
                stop_on_kill: false,
                worn,
                ..BalanceConfig::default()
            },
            apl,
        );
        let kandidat = Candidate {
            worn,
            damage: ergebnis.damage,
        };
        match &best {
            Some(bisher) if bisher.damage.mean >= kandidat.damage.mean => {}
            _ => best = Some(kandidat),
        }
    }
    best.expect("splits liefert immer mindestens eine Aufteilung")
}

/// Rechnet den **Satz-Aufbau**: dasselbe Budget, vollständig in Topf 4.
///
/// # Der Preis ist die Wahlfreiheit (13.5, entschieden am 2026-08-04)
///
/// Ein Satz-Teil trägt dieselben Affixe wie ein freies Stück derselben
/// Qualität — es wird nichts abgezogen. Der Preis ist ein anderer: Wer sechs
/// Satz-Teile trägt, kann auf diesen sechs Plätzen **nicht die besten
/// Einzelstücke** tragen. Er nimmt, was der Satz an Affixen mitbringt, statt
/// gezielt zu wählen.
///
/// **Deshalb darf der Satz-Aufbau seine Affixe nicht optimieren.** Ein
/// Vergleich, in dem beide Seiten frei verteilen, misst den Preis überhaupt
/// nicht — und fiele dann zwangsläufig zugunsten der Sätze aus, weil der Bonus
/// reiner Aufschlag wäre.
///
/// `affix_share` ist, was die Satz-Teile an Affixen zufällig mitbringen. Über
/// diesen Wert misst der Bericht eine **Spanne**: Wessen Teile passende Affixe
/// tragen, zahlt kaum etwas; wessen Teile danebenliegen, zahlt viel.
#[must_use]
pub fn set_build_candidate(
    set_bonus: u32,
    affix_share: WornValues,
    iterations: u32,
    apl: &Apl,
) -> Candidate {
    let worn = WornValues {
        gear_effects: set_bonus,
        ..affix_share
    };
    let ergebnis = run(
        &BalanceConfig {
            iterations,
            stop_on_kill: false,
            worn,
            ..BalanceConfig::default()
        },
        apl,
    );
    Candidate {
        worn,
        damage: ergebnis.damage,
    }
}

/// Der **Kipppunkt**: der Satz-Bonus, bei dem beide Aufbauten gleichauf liegen
/// (Issue #118, AC2).
///
/// # Warum das die richtige Frage ist
///
/// "Wer gewinnt bei diesen Zahlen" ist bei vorlaeufigen Zahlen wertlos — die
/// Antwort aendert sich mit der naechsten Balance-Runde. Der Kipppunkt bleibt:
/// Er sagt, WIE VIEL Satz-Bonus noetig ist, damit 13.5 erfuellt ist
/// ("Satz-Boni und Affixe muessen etwa gleichwertig sein").
///
/// Die Auflage im Issue sagt es woertlich: *"Eine Messung, die nur
/// 'ausgeglichen' sagt, ist bei der naechsten Zahlenaenderung wertlos.
/// Gebraucht wird der Kipppunkt, damit man weiss, wie viel Spielraum bleibt."*
///
/// # Wie gesucht wird
///
/// Der Satz-Bonus waechst im Raster, bis der Satz-Aufbau den besten
/// Affix-Aufbau **messbar** einholt. Kein Halbierungsverfahren: Der Schaden
/// waechst zwar mit dem Bonus, aber er streut auch — eine Halbierung koennte
/// an einer verrauschten Stelle in die falsche Haelfte laufen und nie
/// zurueckfinden.
///
/// Gibt `None`, wenn selbst `max_bonus` nicht reicht. Das ist ein Ergebnis,
/// kein Fehler: Dann sind Saetze in diesem Rahmen nicht konkurrenzfaehig.
#[must_use]
pub fn tipping_set_bonus(
    affix_reference: &Candidate,
    affix_share: WornValues,
    max_bonus: u32,
    iterations: u32,
    apl: &Apl,
) -> Option<(u32, Candidate)> {
    let mut bonus = 0;
    while bonus <= max_bonus {
        let kandidat = set_build_candidate(bonus, affix_share, iterations, apl);
        if kandidat.clearly_better_than(affix_reference) {
            return Some((bonus, kandidat));
        }
        bonus += SEARCH_STEP_PERMILLE;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn das_raster_deckt_das_budget_restlos_ab() {
        // Jede Aufteilung setzt genau das Budget ein — nichts verfaellt, nichts
        // kommt dazu. Ohne diese Pruefung koennte die Suche Aufbauten
        // vergleichen, die verschieden viel tragen.
        let budget = 1000;
        for worn in splits(budget, SEARCH_STEP_PERMILLE) {
            assert_eq!(
                worn.catalysis + worn.purity + worn.yield_bonus,
                budget,
                "{worn:?} setzt nicht das ganze Budget ein"
            );
            assert_eq!(worn.gear_effects, 0, "Topf 4 gehoert der anderen Seite");
        }
    }

    #[test]
    fn das_raster_enthaelt_die_reinen_aufbauten() {
        // Die Ecken des Suchraums. Faehlten sie, koennte die Suche einen reinen
        // Aufbau nie finden — und der Vergleich "gemischt gegen rein" waere
        // schon durch den Suchraum entschieden.
        let budget = 1000;
        let alle = splits(budget, SEARCH_STEP_PERMILLE);
        assert!(alle.iter().any(|w| w.catalysis == budget));
        assert!(alle.iter().any(|w| w.purity == budget));
        assert!(alle.iter().any(|w| w.yield_bonus == budget));
    }

    #[test]
    fn das_raster_waechst_nicht_ins_unermessliche() {
        // Die Suche rechnet jede Aufteilung ueber hunderte Durchgaenge. Bei
        // einem Raster von 50 Promille und 1130 Budget sind es rund 300 —
        // machbar. Ohne Raster waeren es 6500 und der Lauf liefe Stunden.
        let anzahl = splits(1130, SEARCH_STEP_PERMILLE).len();
        assert!(anzahl > 100, "zu grob: nur {anzahl} Aufteilungen");
        assert!(anzahl < 1000, "zu fein: {anzahl} Aufteilungen");
    }

    #[test]
    fn messbar_besser_verlangt_mehr_als_einen_hoeheren_mittelwert() {
        // **Der Kern des Vergleichs.** Ein Vorsprung, der kleiner ist als die
        // Unschaerfe beider Messungen, ist keiner.
        let mach = |mean: f64, stderr: f64| Candidate {
            worn: WornValues::none(),
            damage: Statistics {
                count: 200,
                mean,
                stddev: stderr * 14.14,
                stderr,
                ci95: stderr * 1.96,
                min: mean,
                max: mean,
            },
        };

        // Vorsprung 100, Unschaerfe zusammen 60 -> messbar.
        assert!(mach(9900.0, 30.0).clearly_better_than(&mach(9800.0, 30.0)));
        // Vorsprung 50, Unschaerfe zusammen 60 -> NICHT messbar.
        assert!(!mach(9850.0, 30.0).clearly_better_than(&mach(9800.0, 30.0)));
        // Und ohne Streuung genuegt der blosse Vorsprung.
        assert!(mach(9801.0, 0.0).clearly_better_than(&mach(9800.0, 0.0)));
    }
}
