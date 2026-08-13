//! COAGULATIONs Schluesseltalent rechnet auf beiden Seiten gleich (#419).
//!
//! Ein brechender Schild legt eine HALBE Schicht nach — einmal je gelegtem
//! Schild. Die Begrenzung ist der Kern: Ohne sie waere die Schicht nicht mehr
//! aufzubrauchen.
//!
//! # Was die Paritaet traegt
//!
//! Die abgerundete Haelfte der ZULETZT GELEGTEN Dicke (nicht des Restes), und
//! dass der Schlag trotzdem durchkommt, soweit er die Schicht uebersteigt.
//! Dieselbe Rechnung steht in `combat.ts` und im Spiel-Fall
//! `tests/unit/keystone-shield-refund.test.ts`.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Legt eine Schicht mit CRUST und schlaegt zweimal hart zu.
///
/// Gibt (Dicke der Schicht, Schild nach dem ersten Bruch, Schild nach dem
/// zweiten).
fn schild_nach(ticks: u32, shield_refund: bool) -> u32 {
    // **test-onslaught schlaegt hart genug**: Sein Zauber traegt 2000, die
    // Schicht 900. Ein eigener Treffer-Aufruf waere eine Methode, die es nur
    // fuer den Test gaebe — hier laeuft der Schlag den normalen Weg.
    //
    // **Der Zauber TRIFFT erst nach seiner Wirkzeit**, nicht wenn er beginnt
    // (castTimeMs 2000). Mein erster Entwurf mass bei 3500 ms und sah eine
    // unversehrte Schicht — der Schlag lag noch in der Luft.
    //
    //     3000 ms + 2000 Wirkzeit = 5000   erster Treffer  -> Nachlage
    //    10000 ms + 2000 Wirkzeit = 12000  zweiter Treffer -> nichts mehr
    let boss = boss_by_id("test-onslaught").expect("test-onslaught fehlt");
    let crust = index_of("crust").expect("crust fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        shield_refund,
        ..FightConfig::default()
    });
    let recording = Recording {
        name: "shield-refund".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![Binding {
            slot: 0,
            spell: crust,
        }],
        presses: vec![Press { tick: 0, slot: 0 }],
    };
    fight.replay(&recording, false);
    fight.shield()
}

/// Wie dick CRUST legt — vor jedem Treffer.
fn gelegte_dicke() -> u32 {
    schild_nach(10, false)
}

/// **Die Zusicherung vor der Messung.** Ohne Talent muss die Schicht nach dem
/// ersten Bruch weg sein — sonst misst der Vergleich unten nichts.
#[test]
fn ohne_talent_ist_die_schicht_weg() {
    assert!(
        gelegte_dicke() > 0,
        "CRUST legt keine Schicht — der Fall misst nichts"
    );
    assert_eq!(
        schild_nach(550, false),
        0,
        "ohne Talent darf nach dem Bruch nichts nachkommen"
    );
}

#[test]
fn mit_talent_kommt_die_haelfte_nach() {
    assert_eq!(
        schild_nach(550, true),
        gelegte_dicke() / 2,
        "mit Talent muss die halbe Schicht nachkommen"
    );
}

#[test]
fn und_nur_einmal() {
    assert!(schild_nach(550, true) > 0, "die erste Nachlage fehlt");
    assert_eq!(
        schild_nach(1250, true),
        0,
        "die zweite Nachlage darf es nicht geben"
    );
}
