//! Issue #1177: Ist jeder Boss auf jeder Stufe zu schaffen, und mit welcher
//! Reserve?
//!
//! ```text
//! cargo remote -c -- build --release --example stufen-machbarkeit
//! ./target/release/examples/stufen-machbarkeit [max-len]     # auf node3, unter flock
//! ```
//!
//! # Was gemessen wird
//!
//! Je Boss mit Stufen und je Stufe (Normal, Heroisch, Mythisch):
//!
//! 1. die beste Rotation, die die gierige Suche findet (`greedy`, dieselbe wie
//!    `rota-sim run`), gemessen ueber den vollen Kampf;
//! 2. dieselbe Rotation noch einmal mit `stop_on_kill` — daraus Sieg, Zeit bis
//!    zum Sieg und die RESERVE: Zeitlimit minus Siegzeit, absolut und als
//!    Anteil des Zeitlimits. Beim umgekehrten Ziel (`runaway`) ist der Sieg
//!    das Ueberstehen der Frist; dort steht die Reserve auf null und der
//!    Ring-Rest in der Spalte `schaden` gegen `hp`.
//!
//! Das ist die Boss-Machbarkeitspruefung aus dem Masterplan (A.5c, Ausgabe des
//! Balance-Laeufers) fuer die Stufen.
//!
//! # Was diese Messung NICHT ist
//!
//! - **Nicht der Spieler auf seiner Stufe.** Der Balance-Laeufer rechnet auf
//!   Spielerstufe 1, ohne Ausruestung und Talente, aber mit dem vollen
//!   Zauberbestand als Kandidaten. Er sagt, was der Bestand gegen die Stufe
//!   hergibt — nicht, was ein Spieler mit Stufe 12 und seinem Draft hat.
//! - **Nicht das Optimum.** Die Suche ist gierig; was sie findet, ist ein
//!   Zeuge fuer Machbarkeit, kein Beweis fuer Unmachbarkeit.
//! - **Ein Durchgang je Messung**, und das ist kein Abkuerzen: Ohne Ausruestung
//!   wuerfelt der Kampf nicht (`ein_durchgang_genuegt`) — das Programm bricht
//!   ab, wenn das nicht mehr gilt.

use rota_sim::balance::{run, BalanceConfig};
use rota_sim::data::{boss_at, difficulty_name, BOSS_STAGES, DIFFICULTY_COUNT, EFFECT_NODES};
use rota_sim::durchgaenge::ein_durchgang_genuegt;
use rota_sim::rotation_search::{all_candidates, greedy};

fn main() {
    let max_len: usize = std::env::args()
        .nth(1)
        .map_or(4, |s| s.parse().expect("max-len ist keine Zahl"));
    let pool = all_candidates();
    println!(
        "boss\tstufe\thp\tzeitlimit_ms\tsieg\tsieg_ms\treserve_ms\treserve_promille\ttod\tschaden\trotation"
    );
    for stufen in &BOSS_STAGES {
        let id = stufen[0].id;
        for stufe in 0..DIFFICULTY_COUNT {
            let boss = boss_at(id, stufe).expect("die Stufe gibt es");
            let config = BalanceConfig {
                iterations: 1,
                threads: 1,
                boss,
                ..BalanceConfig::default()
            };
            assert!(
                ein_durchgang_genuegt(EFFECT_NODES, &config.worn),
                "der Kampf wuerfelt — ein Durchgang genuegt nicht mehr"
            );
            let suche = greedy(&config, &pool, max_len);
            let lauf = run(
                &BalanceConfig {
                    stop_on_kill: true,
                    ..config
                },
                &suche.best.apl,
            );
            let sieg = lauf.kills > 0;
            let umgekehrt = boss.runaway.is_some();
            let sieg_ms = if sieg && !umgekehrt {
                lauf.kill_time_ms.mean
            } else {
                f64::NAN
            };
            let reserve = if sieg && !umgekehrt {
                f64::from(boss.overpressure_ms) - sieg_ms
            } else {
                0.0
            };
            let rotation: Vec<&str> = suche
                .chosen
                .iter()
                .map(|k| rota_sim::data::SPELLS[k.spell].id)
                .collect();
            println!(
                "{id}\t{}\t{}\t{}\t{}\t{sieg_ms:.0}\t{reserve:.0}\t{:.0}\t{}\t{:.0}\t{}",
                difficulty_name(stufe).unwrap_or("?"),
                boss.hp,
                boss.overpressure_ms,
                if sieg { "ja" } else { "nein" },
                reserve * 1000.0 / f64::from(boss.overpressure_ms),
                lauf.deaths,
                lauf.damage.mean,
                rotation.join(",")
            );
        }
    }
}
