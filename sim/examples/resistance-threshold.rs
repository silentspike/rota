//! Ab welchem Widerstand aendert sich die beste Rotation? (Messung zu #328, AC6)
//!
//! ```text
//! cargo remote -c -- run --release --example resistance-threshold
//! ```
//!
//! Der Boss wird hier im Code gebaut und nicht aus den Daten gelesen: Gesucht
//! ist die SCHWELLE, und dafuer braucht es viele Faktoren — Datendateien fuer
//! eine Suchfrage waeren elf Dateien fuer eine Antwort.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, Boss};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{all_candidates, greedy};

const RUNS: u32 = 20;
const SEED: u32 = 0;
const MAX_LEN: usize = 6;

fn main() {
    println!("== Schwelle des Widerstands (Messung zu #328, AC6) ==");
    println!("Parameter: RUNS={RUNS}, SEED={SEED}, MAX_LEN={MAX_LEN}, Basis=test-metronome");
    println!("           stop_on_kill=false, threads=1, Widerstand nur auf CALCINATION");
    println!();

    let basis = boss_by_id("test-metronome").expect("test-metronome fehlt");
    let pool = all_candidates();
    let mut vorige: Option<String> = None;

    for (label, resistances) in [
        ("1,00 (keiner)", &[][..]),
        ("0,75 (Korridor)", &[("CALCINATION", 0.75)][..]),
        ("0,60", &[("CALCINATION", 0.60)][..]),
        ("0,50", &[("CALCINATION", 0.50)][..]),
        ("0,40", &[("CALCINATION", 0.40)][..]),
        ("0,30", &[("CALCINATION", 0.30)][..]),
        ("0,20", &[("CALCINATION", 0.20)][..]),
    ] {
        let boss: &'static Boss = Box::leak(Box::new(Boss {
            resistances,
            ..*basis
        }));
        let config = BalanceConfig {
            iterations: RUNS,
            base_seed: SEED,
            threads: 1,
            duration_ms: None,
            stop_on_kill: false,
            factors: NEUTRAL,
            boss,
            ..BalanceConfig::default()
        };
        let gefunden = greedy(&config, &pool, MAX_LEN);
        let apl = gefunden.best.apl.to_string().replace('\n', " | ");
        let geaendert = vorige.as_ref().is_some_and(|v| *v != apl);
        println!(
            "  {label:16} {:9.1}   {}{}",
            gefunden.best.mean,
            apl,
            if geaendert { "   <== ANDERS" } else { "" }
        );
        vorige = Some(apl);
    }
}
