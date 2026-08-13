//! Der Satz-Effekt `long-grip` wirkt auch im Simulator (13.5, Issue #480).
//!
//! Der erste Kontroll-Ausgeber nach jedem Phasenwechsel kostet nichts —
//! der ABZUG entfaellt, die Voraussetzung bleibt.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Punkte aufbauen, Phase wechseln lassen, dann ausgeben.
///
/// Gibt (Phase, Punkte danach, Zahl der kostenlosen Ausgaben).
fn nach_phasenwechsel(long_grip: bool) -> (usize, u32, u32) {
    let boss = boss_by_id("test-phase").expect("Boss fehlt");
    let bind = index_of("bind").expect("bind fehlt");
    let stall = index_of("stall").expect("stall fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        long_grip,
        ..FightConfig::default()
    });

    let mut presses: Vec<Press> = (0..12)
        .map(|n| Press {
            tick: n * 150,
            slot: 0,
        })
        .collect();
    presses.push(Press {
        tick: 12 * 150,
        slot: 1,
    });

    let recording = Recording {
        name: "long-grip".to_owned(),
        seed: 0,
        ticks: 12 * 150 + 10,
        bindings: vec![
            Binding {
                slot: 0,
                spell: bind,
            },
            Binding {
                slot: 1,
                spell: stall,
            },
        ],
        presses,
    };
    fight.replay(&recording, false);
    (
        fight.phase(),
        fight.control_points(),
        fight.free_control_spends(),
    )
}

#[test]
fn ohne_den_effekt_kostet_der_ausgeber() {
    let (phase, _, umsonst) = nach_phasenwechsel(false);
    // ZUSICHERUNG: Es gab ueberhaupt einen Phasenwechsel.
    assert!(phase > 0, "kein Phasenwechsel, der Lauf misst nichts");
    assert_eq!(umsonst, 0, "ohne den Effekt darf nichts umsonst sein");
}

#[test]
fn mit_dem_effekt_ist_der_erste_umsonst() {
    let (phase, _, umsonst) = nach_phasenwechsel(true);
    assert!(phase > 0, "kein Phasenwechsel, der Lauf misst nichts");
    assert_eq!(umsonst, 1, "genau EIN Ausgeber muss umsonst sein");
}

#[test]
fn und_die_punkte_bleiben_stehen() {
    let (_, ohne, _) = nach_phasenwechsel(false);
    let (_, mit, _) = nach_phasenwechsel(true);
    assert!(
        mit > ohne,
        "mit dem Effekt {mit} Punkte, ohne {ohne} — der Abzug entfaellt nicht"
    );
}
