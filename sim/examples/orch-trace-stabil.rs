//! MESSWERKZEUG DES ORCHESTRATORS — nicht committen.
//!
//! **Die wichtigste offene Zahl des Architekturplans (E11.3).**
//!
//! Der Trace-Ledger, die Trace-Regionen (E11.3a), die inverse Treppenfunktion
//! (E12.2) und der zweiteilige Cache-Schluessel (E11.4b) stehen alle auf
//! derselben ungeprueften Annahme:
//!
//! > Wenn sich eine ZAHL aendert (Grundschaden, Talentwert, Bosswerte),
//! > bleibt die Cast-Folge gleich — jedenfalls in einem Bereich um den
//! > Ausgangswert.
//!
//! Fuer ROTATIONSaenderungen ist das gemessen widerlegt (99,1 % weichen vor
//! der Einfuegeposition ab). Fuer ZAHLEN hat es niemand gemessen.
//!
//! # Was hier gemessen wird
//!
//! Fuer jede Groesse, die ein Balance-Optimierer bewegen wuerde, wird der
//! Faktor schrittweise veraendert und geprueft, ab welcher Aenderung die
//! Cast-Folge kippt. Ausgegeben wird die BREITE des stabilen Bereichs.
//!
//! # Kontrollzeilen
//!
//! ```text
//!   Aenderung 0 %      -> Folge MUSS identisch sein
//!   Aenderung x1000    -> Folge MUSS abweichen (sonst misst der Weg nichts)
//! ```

use rota_sim::apl::{Apl, Rule};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::factors::{DamageFactors, NEUTRAL};
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::{Fight, FightConfig};

const DAUER_MS: u32 = 60_000;
const SEED: u32 = 5;
const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"];

fn folge(boss_id: &str, apl: &Apl, f: DamageFactors) -> Vec<(u32, u8)> {
    let boss = boss_by_id(boss_id).expect("Boss fehlt");
    let mut fight = Fight::new(FightConfig {
        seed: SEED,
        boss,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: true, // ENTSCHEIDEND: sonst aendert Schaden nichts
        factors: f,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(apl);
    fight
        .log()
        .entries()
        .into_iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| (e.time, e.spell))
        .collect()
}

/// Wie lange lief der Kampf? Ein frueherer Sieg ist eine Trace-Aenderung,
/// auch wenn die Cast-Folge bis dahin gleich war.
fn dauer(boss_id: &str, apl: &Apl, f: DamageFactors) -> u32 {
    let boss = boss_by_id(boss_id).expect("Boss fehlt");
    let mut fight = Fight::new(FightConfig {
        seed: SEED,
        boss,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: true,
        factors: f,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(apl);
    fight
        .log()
        .entries()
        .into_iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| e.time)
        .max()
        .unwrap_or(0)
}

/// Setzt EINE Groesse auf `wert`, alle anderen neutral.
fn mit(name: &str, wert: f64) -> DamageFactors {
    let mut f = NEUTRAL;
    match name {
        "essence" => f.essence = wert,
        "talents" => f.talents = wert,
        "gear_effects" => f.gear_effects = wert,
        "buffs" => f.buffs = wert,
        "synergy" => f.synergy = wert,
        "resistance" => f.resistance = wert,
        "vulnerability" => f.vulnerability = wert,
        "crit" => f.crit = wert,
        _ => panic!("unbekannte Groesse {name}"),
    }
    f
}

fn main() {
    let config = BalanceConfig {
        iterations: 20,
        base_seed: SEED,
        threads: 1,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: boss_by_id(BOSSE[0]).expect("Boss fehlt"),
        ..BalanceConfig::default()
    };
    let pool = all_candidates();
    let gefunden = greedy(&config, &pool, 6);
    let apl = Apl {
        name: "fest".to_owned(),
        rules: gefunden
            .chosen
            .iter()
            .enumerate()
            .map(|(i, k)| Rule {
                line: i + 1,
                spell: k.spell,
                condition: k.condition.clone(),
            })
            .collect(),
    };
    println!("Rotation aus greedy, {} Zeilen", apl.rules.len());
    println!();

    // ---- Kontrollzeilen
    let basis = folge(BOSSE[0], &apl, NEUTRAL);
    println!("== Kontrollzeilen ==");
    println!("  Basisfolge: {} Casts", basis.len());
    assert_eq!(
        basis,
        folge(BOSSE[0], &apl, mit("essence", 1.0)),
        "Kontrollzeile 1 gebrochen: 0 % Aenderung veraendert die Folge"
    );
    println!("  0 % Aenderung      -> Folge identisch          OK");
    let extrem = folge(BOSSE[0], &apl, mit("essence", 1000.0));
    println!(
        "  Faktor 1000        -> Folge {}   {}",
        if extrem == basis {
            "IDENTISCH"
        } else {
            "abweichend"
        },
        if extrem == basis {
            "*** der Messweg misst nichts ***"
        } else {
            "OK"
        }
    );
    println!();

    // ---- Die eigentliche Messung: ab welcher Aenderung kippt die Folge?
    println!("== Ab welcher Aenderung kippt die Cast-Folge? ==");
    println!(
        "{:<16} {:>10} {:>10} {:>10} {:>10}",
        "Groesse", "boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"
    );

    let groessen = [
        "essence",
        "talents",
        "gear_effects",
        "buffs",
        "synergy",
        "resistance",
        "vulnerability",
        "crit",
    ];
    // Schrittweise: +0,1 %, +0,5 %, +1 %, +2 %, +5 %, +10 %, +25 %, +50 %, +100 %
    let schritte: [f64; 9] = [0.001, 0.005, 0.01, 0.02, 0.05, 0.10, 0.25, 0.50, 1.00];

    for g in groessen {
        print!("{g:<16}");
        for boss in BOSSE {
            let b = folge(boss, &apl, NEUTRAL);
            let mut kipp = None;
            for s in schritte {
                if folge(boss, &apl, mit(g, 1.0 + s)) != b {
                    kipp = Some(s);
                    break;
                }
            }
            match kipp {
                Some(s) => print!("{:>9.1}%", s * 100.0),
                None => print!("{:>10}", ">100%"),
            }
        }
        println!();
    }
    println!();
    println!("  Ein Wert von >100 % heisst: die Folge blieb ueber den ganzen");
    println!("  geprueften Bereich STABIL — dort traegt der Ledger.");
    println!("  Ein Wert von 0,1 % heisst: schon die kleinste Aenderung kippt sie.");
    println!();
    println!("  ACHTUNG: gemessen mit EINER Rotation und EINEM Startwert.");
    println!("  Eine andere Rotation kann andere Schwellen haben.");
}
