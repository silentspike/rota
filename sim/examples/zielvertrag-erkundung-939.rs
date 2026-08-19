//! Welche Rotation traegt den Nachweis zu Issue #939, AC8?
//!
//! AC8 verlangt einen Kandidaten, der **viel Schaden macht und die
//! Siegbedingung des umgekehrten Bosses verletzt**, plus eine Kontrollprobe an
//! einem gewoehnlichen Boss. Welche der vorhandenen Prioritaetenlisten das
//! leistet, ist eine Messung und keine Annahme — der Boss-Kommentar in
//! `data/bosses/boss-2-3.json` nennt Zahlen aus dem **Spiel**, nicht aus dem
//! Simulator.
//!
//! Dieses Programm misst die Ausgaenge und gibt die Tabelle aus, aus der der
//! Prueffall dann seine Rotationen nimmt.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig};
use rota_sim::data::boss_by_id;
use rota_sim::objective::{Outcome, WinCondition};

/// Die Listen, die geprueft werden — Name und Inhalt.
const LISTEN: [(&str, &str); 9] = [
    ("e939-schutz", include_str!("../apl/e939-schutz.apl")),
    ("step1-good", include_str!("../apl/step1-good.apl")),
    ("e607-basis", include_str!("../apl/e607-basis.apl")),
    ("e607-crust", include_str!("../apl/e607-crust.apl")),
    ("e607-decoction", include_str!("../apl/e607-decoction.apl")),
    ("e607-flashpoint", include_str!("../apl/e607-flashpoint.apl")),
    ("e607b-crust", include_str!("../apl/e607b-crust.apl")),
    ("e607b-poultice", include_str!("../apl/e607b-poultice.apl")),
    ("e607b-wisp", include_str!("../apl/e607b-wisp.apl")),
];

/// Die Bosse: der umgekehrte und zwei gewoehnliche als Kontrolle.
const BOSSE: [&str; 3] = ["boss-2-3", "boss-1-1", "boss-2-2"];

fn main() {
    for boss_id in BOSSE {
        let Some(boss) = boss_by_id(boss_id) else {
            println!("BOSS FEHLT: {boss_id}");
            continue;
        };
        let condition = WinCondition::of(boss);
        println!(
            "\n=== {boss_id} — {} (hp {}, Frist {} ms) ===",
            if condition.is_inverted() {
                "UMGEKEHRTES ZIEL"
            } else {
                "gewoehnlich"
            },
            boss.hp,
            boss.overpressure_ms
        );
        println!(
            "{:<18} {:>7} {:>7} {:>7} {:>10} {:>10}",
            "Liste", "Siege", "Ueberdr", "Tode", "Schaden", "kills-roh"
        );

        for (name, quelle) in LISTEN {
            let Ok(apl) = parse(AplName(name), quelle) else {
                println!("{name:<18} LISTE NICHT LESBAR");
                continue;
            };
            let config = BalanceConfig {
                iterations: 20,
                base_seed: 4242,
                threads: 1,
                boss,
                ..BalanceConfig::default()
            };
            let ergebnis = run(&config, &apl);

            let mut siege = 0;
            let mut ueberdruck = 0;
            let mut tode = 0;
            for sample in &ergebnis.samples {
                match Outcome::of(sample, condition) {
                    Outcome::Won => siege += 1,
                    Outcome::Overpressure => ueberdruck += 1,
                    Outcome::Died => tode += 1,
                }
            }

            // `kills` daneben, weil genau hier der Unterschied sichtbar wird:
            // Bei umgekehrtem Ziel zaehlt die rohe Zahl die Niederlagen.
            println!(
                "{name:<18} {siege:>7} {ueberdruck:>7} {tode:>7} {:>10.0} {:>10}",
                ergebnis.damage.mean, ergebnis.kills
            );
            let _: &Apl = &apl;
        }
    }
}
