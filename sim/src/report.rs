//! Die drei Auswertungen, für die der Simulator gebaut wird (GAME-DESIGN 18.5).
//!
//! | Auswertung | Frage | Warum sie gebraucht wird |
//! | --- | --- | --- |
//! | [`spell_table`] | Was trägt jeder Zauber bei? | Deckt **tote Zauber** auf, die niemand nutzt |
//! | [`feasibility`] | Fällt der Boss im Zeitlimit? | Ein Boss, der nicht fällt, ist keiner |
//! | [`achievement_run`] | Gibt es für jeden Erfolg eine Lösung? | **Ohne Lösung gilt der Boss als nicht auslieferbar** (12.9) |
//!
//! Alle drei geben Text und JSON aus, keine Diagramme — sie liest Claude, nicht
//! der Auftraggeber.
//!
//! # Was diese Auswertungen heute noch nicht können
//!
//! Der Erfolgs-Prüflauf soll laut 12.9 über **Boss × Schwierigkeit × Erfolg**
//! laufen. Von den dreien gibt es heute genau eines:
//!
//! * **Ein Boss** — THE FIRST CRUCIBLE, ohne Mechaniken und ohne Phasen
//!   (`STEP1-SPEC` § 3).
//! * **Keine Schwierigkeitsstufen** — 11.7 nennt Normal/Heroisch/Mythisch,
//!   aber welche Mechanik auf welcher Stufe dazukommt, ist noch nicht
//!   ausgeplant (GAME-DESIGN 22, vor E4).
//! * **Keine Erfolge** — die konkreten Ziele je Boss stehen ebenfalls in
//!   Kapitel 22 (vor E10).
//!
//! Ableitbar ist trotzdem etwas, und zwar genau das, was 21.6 dem Balance-Läufer
//! zuschreibt: die **Zeitmarken**. Aus der errechneten Bestzeit folgen Gold,
//! Silber und Bronze als feste Vielfache — siehe [`time_tiers`]. Damit prüft der
//! Prüflauf heute drei echte Erfolge statt gar keiner, und die Maschinerie steht,
//! sobald die übrigen dazukommen.

use crate::apl::Apl;
use crate::data::{BOSS, SPELLS, SPELL_COUNT};
use crate::fight::{Fight, FightConfig, RunResult};
use crate::time::Millis;

/// Was ein einzelner Zauber beigetragen hat.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellContribution {
    /// Index in [`crate::data::SPELLS`].
    pub spell: usize,
    /// Bezeichner des Zaubers.
    pub id: &'static str,
    /// Wie oft er gewirkt wurde.
    pub casts: u32,
    /// Wie viel Schaden auf ihn entfällt.
    pub damage: u32,
    /// Anteil am Gesamtschaden, 0 bis 1.
    pub share: f64,
    /// Schaden je Wirkung. 0, wenn nie gewirkt.
    pub damage_per_cast: f64,
}

impl SpellContribution {
    /// Ob dieser Zauber in der geprüften Rotation **gar nicht vorkommt**.
    ///
    /// Das ist der eigentliche Zweck der Tabelle: Ein Zauber, den die beste
    /// Rotation nie wirkt, ist entweder falsch bemessen oder überflüssig —
    /// beides muss auffallen, bevor Inhalte darauf gebaut werden.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.casts == 0
    }
}

/// Beitrag und Häufigkeit jedes Zaubers.
///
/// Nach Schaden absteigend sortiert; bei Gleichstand entscheidet die Reihenfolge
/// aus `data/`, damit zwei Läufe dieselbe Tabelle ergeben.
#[must_use]
pub fn spell_table(result: &RunResult) -> Vec<SpellContribution> {
    let total = f64::from(result.damage);
    let mut rows: Vec<SpellContribution> = (0..SPELL_COUNT)
        .map(|spell| {
            let damage = result.damage_by_spell[spell];
            let casts = result.casts[spell];
            SpellContribution {
                spell,
                id: SPELLS[spell].id,
                casts,
                damage,
                share: if total > 0.0 {
                    f64::from(damage) / total
                } else {
                    0.0
                },
                damage_per_cast: if casts > 0 {
                    f64::from(damage) / f64::from(casts)
                } else {
                    0.0
                },
            }
        })
        .collect();

    rows.sort_by(|a, b| b.damage.cmp(&a.damage).then_with(|| a.spell.cmp(&b.spell)));
    rows
}

/// Ob ein Boss im Zeitlimit zu schaffen ist.
#[derive(Debug, Clone, PartialEq)]
pub struct Feasibility {
    /// Name des Bosses.
    pub boss: &'static str,
    /// Zeitlimit, gegen das gerechnet wurde.
    pub limit_ms: Millis,
    /// Ob er gefallen ist.
    pub defeated: bool,
    /// Wann er gefallen ist.
    pub kill_time_ms: Option<Millis>,
    /// Wie viel Zeit übrig blieb.
    pub reserve_ms: Millis,
    /// Reserve als Anteil des Zeitlimits, 0 bis 1.
    pub reserve_share: f64,
    /// Verursachter Schaden.
    pub damage: u32,
    /// Anteil der Lebenspunkte, der abgetragen wurde.
    pub hp_share: f64,
}

/// Prüft, ob der Boss mit dieser Rotation im Zeitlimit fällt.
///
/// `limit_ms` überschreibt den Überdruck-Timer des Bosses. Genau darüber laufen
/// die Gegenproben: Ein Zeitlimit, das zu kurz ist, macht die Lage
/// nachweislich unschaffbar — und eine Prüfung, die **nur ja sagen kann**,
/// prüft nichts.
#[must_use]
pub fn feasibility(apl: &Apl, limit_ms: Option<Millis>) -> Feasibility {
    let mut fight = Fight::new(FightConfig {
        duration_ms: limit_ms,
        stop_on_kill: true,
        log_capacity: 0,
        ..FightConfig::default()
    });
    let result = fight.run_apl(apl);
    let limit = limit_ms.unwrap_or(BOSS.overpressure_ms);

    Feasibility {
        boss: BOSS.name,
        limit_ms: limit,
        defeated: result.killed_at_ms.is_some(),
        kill_time_ms: result.killed_at_ms,
        reserve_ms: result.remaining_ms,
        reserve_share: if limit > 0 {
            f64::from(result.remaining_ms) / f64::from(limit)
        } else {
            0.0
        },
        damage: result.damage,
        hp_share: if BOSS.hp > 0 {
            f64::from(result.damage.min(BOSS.hp)) / f64::from(BOSS.hp)
        } else {
            0.0
        },
    }
}

/// Die drei Tempo-Stufen eines Bosses.
///
/// Aus GAME-DESIGN 21.6: *„Balance-Läufer liefert Vorschlagswerte (Gold ≈ 110 %,
/// Silber ≈ 130 %, Bronze ≈ 160 % der errechneten Bestzeit)."* Das entlastet die
/// 288 Zeitmarken, die sonst von Hand gesetzt werden müssten — und die dort als
/// *„viel Arbeit und fehleranfällig"* geführt sind.
///
/// Die Werte sind **Vorschläge**, keine Festlegung: Was am Ende im Spiel steht,
/// entscheidet der Auftraggeber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTiers {
    /// Bestzeit, aus der die Stufen abgeleitet sind.
    pub best_ms: Millis,
    /// 110 % der Bestzeit.
    pub gold_ms: Millis,
    /// 130 % der Bestzeit.
    pub silver_ms: Millis,
    /// 160 % der Bestzeit.
    pub bronze_ms: Millis,
}

/// Leitet die Zeitmarken aus einer Bestzeit ab.
///
/// Ganzzahlig gerechnet: `best × 110 / 100`. Die Reihenfolge der beiden
/// Rechenschritte ist wichtig — erst mal, dann geteilt, sonst geht bei kurzen
/// Zeiten die Genauigkeit verloren.
#[must_use]
pub fn time_tiers(best_ms: Millis) -> TimeTiers {
    let scale = |percent: u64| -> Millis {
        Millis::try_from(u64::from(best_ms) * percent / 100).unwrap_or(Millis::MAX)
    };
    TimeTiers {
        best_ms,
        gold_ms: scale(110),
        silver_ms: scale(130),
        bronze_ms: scale(160),
    }
}

/// Ein geprüfter Erfolg.
#[derive(Debug, Clone, PartialEq)]
pub struct AchievementCheck {
    /// Name des Erfolgs.
    pub name: String,
    /// Ob eine Lösung gefunden wurde.
    pub solved: bool,
    /// Ob der Erfolg überhaupt **etwas unterscheidet**.
    ///
    /// Eine Zeitmarke ist eine Obergrenze: „schaffe es unter X". Liegt X über
    /// dem Zeitlimit des Bosses, erfüllt sie **jeder** Sieg — sie ist erfüllbar,
    /// aber sie trennt niemanden von niemandem.
    ///
    /// Das ist kein Fehler des Prüflaufs, sondern ein Befund über die Marke.
    /// Deshalb steht es als eigenes Feld neben `solved` und nicht darin: Ein
    /// Erfolg, den alle bekommen, ist keine gescheiterte Prüfung, sondern ein
    /// Erfolg, der nichts wert ist.
    pub meaningful: bool,
    /// Womit belegt — die Zahl, die zum Urteil geführt hat.
    pub evidence: String,
}

/// Der Erfolgs-Prüflauf (GAME-DESIGN 12.9).
///
/// > Der Simulator läuft über jede Kombination aus Boss × Schwierigkeit ×
/// > Erfolg und bestätigt, dass eine Lösung existiert. **Ohne gefundene Lösung
/// > gilt der Boss als nicht auslieferbar.**
///
/// Heute prüft er die drei Zeitmarken und den Sieg selbst — mehr gibt es nicht,
/// solange Erfolge und Schwierigkeitsstufen nicht ausgeplant sind (Kapitel 22).
/// Die Kriterien sind **absolut formuliert**, wie 12.9 es verlangt: „unter
/// dieser Zeit", nicht „besser als der Durchschnitt".
#[must_use]
pub fn achievement_run(apl: &Apl, limit_ms: Option<Millis>) -> Vec<AchievementCheck> {
    let result = feasibility(apl, limit_ms);
    let mut checks = Vec::with_capacity(4);

    checks.push(AchievementCheck {
        name: "Sieg".to_owned(),
        solved: result.defeated,
        meaningful: true,
        evidence: match result.kill_time_ms {
            Some(t) => format!("besiegt bei {t} ms von {} ms", result.limit_ms),
            None => format!(
                "nicht besiegt, {} von {} Lebenspunkten abgetragen ({:.1} %)",
                result.damage,
                BOSS.hp,
                result.hp_share * 100.0
            ),
        },
    });

    let Some(best) = result.kill_time_ms else {
        // Ohne Sieg gibt es keine Bestzeit, aus der sich Zeitmarken ableiten
        // liessen. Die drei Stufen als "ungeloest" zu melden waere richtig, aber
        // nichtssagend — der Sieg-Eintrag oben sagt bereits alles.
        return checks;
    };

    let tiers = time_tiers(best);
    for (name, ziel) in [
        ("Gold", tiers.gold_ms),
        ("Silber", tiers.silver_ms),
        ("Bronze", tiers.bronze_ms),
    ] {
        // Eine Zeitmarke ist eine OBERGRENZE: "schaffe es unter X". Sie ist
        // erfuellt, wenn die Bestzeit darunter liegt — mehr braucht es nicht.
        let solved = best <= ziel;

        // Liegt die Marke ueber dem Zeitlimit, erfuellt sie jeder Sieg. Dann
        // ist sie zwar geloest, unterscheidet aber niemanden. Das ist ein
        // Befund ueber die Marke, nicht ueber den Lauf — siehe
        // `AchievementCheck::meaningful`.
        let meaningful = ziel < result.limit_ms;

        checks.push(AchievementCheck {
            name: format!("Zeitmarke {name}"),
            solved,
            meaningful,
            evidence: if meaningful {
                format!("Bestzeit {best} ms, Marke {ziel} ms")
            } else {
                format!(
                    "Bestzeit {best} ms, Marke {ziel} ms — die Marke liegt ueber dem \
                     Zeitlimit von {} ms und wird damit von jedem Sieg erfuellt",
                    result.limit_ms
                )
            },
        });
    }

    checks
}

/// Ob der Prüflauf den Boss freigibt.
///
/// **Ohne gefundene Lösung gilt der Boss als nicht auslieferbar** (12.9). Diese
/// Funktion ist die Stelle, an der aus einer Liste von Prüfungen ein Urteil wird.
#[must_use]
pub fn is_shippable(checks: &[AchievementCheck]) -> bool {
    !checks.is_empty() && checks.iter().all(|check| check.solved)
}

/// Erfolge, die zwar loesbar sind, aber niemanden von niemandem trennen.
///
/// Sie sperren die Auslieferung **nicht** — 12.9 verlangt nur, dass eine Loesung
/// existiert. Sie gehoeren aber in jeden Bericht: Ein Erfolg, den jeder Sieg
/// mitbringt, vergibt Punkte fuer nichts.
#[must_use]
pub fn meaningless(checks: &[AchievementCheck]) -> Vec<&AchievementCheck> {
    checks
        .iter()
        .filter(|check| check.solved && !check.meaningful)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeitmarken_folgen_der_regel_aus_216() {
        let t = time_tiers(40_510);
        assert_eq!(t.best_ms, 40_510);
        assert_eq!(t.gold_ms, 44_561, "110 %");
        assert_eq!(t.silver_ms, 52_663, "130 %");
        assert_eq!(t.bronze_ms, 64_816, "160 %");
    }

    #[test]
    fn zeitmarken_steigen_streng() {
        let t = time_tiers(1000);
        assert!(t.best_ms < t.gold_ms);
        assert!(t.gold_ms < t.silver_ms);
        assert!(t.silver_ms < t.bronze_ms);
    }

    #[test]
    fn zeitmarken_bei_null_stuerzen_nicht_ab() {
        let t = time_tiers(0);
        assert_eq!(t.gold_ms, 0);
        assert_eq!(t.bronze_ms, 0);
    }

    #[test]
    fn ohne_pruefungen_ist_nichts_freigegeben() {
        // Eine leere Liste bedeutet "nichts geprueft", nicht "alles in Ordnung".
        // `all()` liefert auf leeren Listen true — genau die Falle, die hier
        // ausdruecklich zugemacht wird.
        assert!(!is_shippable(&[]));
    }

    #[test]
    fn eine_einzige_ungeloeste_pruefung_sperrt() {
        let checks = vec![
            AchievementCheck {
                name: "a".to_owned(),
                solved: true,
                meaningful: true,
                evidence: String::new(),
            },
            AchievementCheck {
                name: "b".to_owned(),
                solved: false,
                meaningful: true,
                evidence: String::new(),
            },
        ];
        assert!(!is_shippable(&checks));
    }
}
