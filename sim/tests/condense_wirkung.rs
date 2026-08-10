//! Wirkt CONDENSE? (Issue #480)
//!
//! **Nicht am Feld gemessen, sondern am Schaden.** Ein Zauber, dessen
//! Weglassen nichts bewegt, ist nicht angeschlossen — AC3 des Issues.

use rota_sim::data::index_of;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig};

fn lauf(mit_condense: bool) -> u32 {
    let condense = index_of("condense").expect("CONDENSE steht im Zauberkasten");
    let kindle = index_of("kindle").expect("KINDLE steht im Zauberkasten");

    // KINDLE einmal, davor wahlweise CONDENSE. Die Gegenprobe setzt an
    // dieselbe Stelle einen Zauber, der nichts verdoppelt — sonst
    // unterschieden sich die Läufe um CONDENSE **und** um eine Wartezeit.
    let vorher = if mit_condense {
        condense
    } else {
        index_of("vapour").unwrap()
    };

    let recording = Recording {
        name: "condense-probe".to_owned(),
        seed: 1,
        ticks: 400,
        bindings: vec![
            Binding {
                slot: 0,
                spell: vorher,
            },
            Binding {
                slot: 1,
                spell: kindle,
            },
        ],
        presses: vec![Press { tick: 0, slot: 0 }, Press { tick: 160, slot: 1 }],
    };
    let mut fight = Fight::new(FightConfig {
        seed: 1,
        duration_ms: Some(400 * rota_sim::TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, false);
    fight.damage()
}

#[test]
fn ac2_der_naechste_zauber_wirkt_zweimal() {
    let mit = lauf(true);
    let ohne = lauf(false);
    assert!(
        mit > ohne,
        "CONDENSE bewegt den Schaden nicht: mit {mit}, ohne {ohne} — \
         verdoppelt der Merker die Wirkung des naechsten Wurfs?"
    );
    println!("mit CONDENSE {mit}, ohne {ohne}");
}
