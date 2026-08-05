//! Aendern Widerstaende die beste Rotation? (Issue #328, AC6)
//!
//! ```text
//! cargo remote -c -- run --release --example resistance-rotation
//! ```
//!
//! Zwei Boesse, ein Feld Unterschied: `test-resistant` haelt Kalzination mit
//! 0,75 ab, `test-vulnerable` ist ihr gegenueber mit 1,25 schwach. Alles
//! andere ist Zeichen fuer Zeichen gleich.
//!
//! Der Bericht druckt seine Parameter mit (Regel aus Issue #242): Eine Zahl
//! ohne ihren Aufruf ist nicht nachrechenbar.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{all_candidates, greedy};

const RUNS: u32 = 50;
const SEED: u32 = 0;
const MAX_LEN: usize = 6;
const BOSSES: &[&str] = &["test-resistant", "test-vulnerable"];

fn config(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

fn main() {
    println!("== Widerstand und beste Rotation (Issue #328, AC6) ==");
    println!("Parameter: RUNS={RUNS}, SEED={SEED}, MAX_LEN={MAX_LEN}, Bosse={BOSSES:?}");
    println!("           stop_on_kill=false, Faktoren=NEUTRAL, threads=1");
    println!();

    let pool = all_candidates();
    let mut rotationen: Vec<(String, String, f64)> = Vec::new();

    for boss_id in BOSSES {
        let config = config(boss_id);
        let boss = boss_by_id(boss_id).expect("Boss");
        let gefunden = greedy(&config, &pool, MAX_LEN);
        let apl = gefunden.best.apl.to_string();

        println!("Boss {boss_id}");
        println!("  Widerstand   {:.2}", boss.resistance_for("CALCINATION"));
        println!(
            "  gefunden     {:9.1} +/- {:.1}   ({} Listen gemessen)",
            gefunden.best.mean, gefunden.best.stderr, gefunden.measured
        );
        println!("  Rotation:");
        for zeile in apl.lines() {
            println!("    {zeile}");
        }
        println!();
        rotationen.push(((*boss_id).to_owned(), apl, gefunden.best.mean));
    }

    let (a_id, a_apl, a_mean) = &rotationen[0];
    let (b_id, b_apl, b_mean) = &rotationen[1];
    println!("== Vergleich ==");
    println!("  {a_id}: {a_mean:.1}");
    println!("  {b_id}: {b_mean:.1}");
    if a_apl == b_apl {
        println!("  ROTATION GLEICH — der Widerstand aendert die Wahl NICHT.");
    } else {
        println!("  ROTATION VERSCHIEDEN — der Widerstand aendert die Wahl.");
    }
}
