//! Der Satz-Effekt `slow-rot` wirkt auch im Simulator (13.5, Issue #480).
//!
//! Zersetzungs-Stapel verfallen nicht, solange ein DoT laeuft.
//!
//! # Der Aufbau, und zwei die ich verworfen habe
//!
//! Auf der Spiel-Seite sind mir beim Bauen zwei Messwege durchgefallen, und
//! beide sahen aus wie „der Effekt greift nicht":
//!
//! - **BLIGHT** legt Stapel, aber KEINEN DoT. Die Bedingung war nie erfuellt.
//! - **ROT** legt beides — und fuellt damit laufend nach, was verfaellt. Beide
//!   Laeufe zeigten dieselbe Zahl, weil Nachschub und Verfall vermengt waren.
//!
//! Hier baut ROT die Stapel auf und hoert dann auf; SMOULDER haelt den DoT am
//! Laufen, ohne selbst Stapel zu legen.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Wie viele Stapel nach `ticks` Takten noch stehen.
fn stapel_nach(slow_rot: bool, ticks: u32) -> usize {
    let boss = boss_by_id("test-warden").expect("Boss fehlt");
    let rot = index_of("rot").expect("rot fehlt");
    let smoulder = index_of("smoulder").expect("smoulder fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        slow_rot,
        ..FightConfig::default()
    });

    // ROT einmal am Anfang (baut ueber vier Sekunden Stapel auf), dann alle
    // zehn Sekunden ein SMOULDER, damit der DoT durchgehend laeuft.
    let mut presses = vec![Press { tick: 0, slot: 0 }];
    let mut t = 400;
    while t < ticks {
        presses.push(Press { tick: t, slot: 1 });
        t += 1000;
    }

    let recording = Recording {
        name: "slow-rot".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![
            Binding {
                slot: 0,
                spell: rot,
            },
            Binding {
                slot: 1,
                spell: smoulder,
            },
        ],
        presses,
    };
    fight.replay(&recording, false);
    fight.decay_stacks()
}

#[test]
fn ohne_den_effekt_verfallen_die_stapel() {
    // Nach zwanzig Sekunden ist die Haltbarkeit der Stapel abgelaufen.
    let stehen = stapel_nach(false, 2000);
    assert_eq!(stehen, 0, "ohne den Effekt duerfen keine Stapel stehen");
}

#[test]
fn mit_dem_effekt_stehen_sie_noch() {
    let stehen = stapel_nach(true, 2000);
    assert!(
        stehen > 0,
        "mit dem Effekt muessen Stapel stehen bleiben, gemessen {stehen}"
    );
}

#[test]
fn der_aufbau_ist_in_beiden_laeufen_gleich() {
    // Kurz nach dem Aufbau, bevor irgendetwas verfallen kann: Beide Laeufe
    // muessen dieselbe Zahl zeigen. Waere sie verschieden, liesse sich der
    // Unterschied oben nicht dem Verfall zuschreiben.
    assert_eq!(stapel_nach(false, 500), stapel_nach(true, 500));
    // ZUSICHERUNG: Es wurde ueberhaupt etwas aufgebaut.
    assert!(stapel_nach(false, 500) > 0, "kein Stapel aufgebaut");
}
