//! Der Satz-Effekt `sealed-vessel` wirkt auch im Simulator (13.5, Issue #480).
//!
//! Ueberschuessiger Schutz ueber der Stapel-Grenze wird zu Schaden.
//!
//! **SCAB und nicht CRUST**: CRUST hat 6 s Abklingzeit, und Wuerfe in kuerzerem
//! Abstand werden abgelehnt — die Schicht erreicht ihre Grenze nie, und beide
//! Laeufe zeigen dasselbe. Auf der Spiel-Seite ist mir genau das passiert.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Schaden nach `n` SCAB im Abstand ihrer Abklingzeit.
fn schaden_nach(sealed_vessel: bool, n: u32) -> u32 {
    let boss = boss_by_id("test-warden").expect("Boss fehlt");
    let scab = index_of("scab").expect("scab fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        sealed_vessel,
        ..FightConfig::default()
    });

    let presses: Vec<Press> = (0..n)
        .map(|i| Press {
            tick: i * 150,
            slot: 0,
        })
        .collect();
    let recording = Recording {
        name: "sealed".to_owned(),
        seed: 0,
        ticks: n * 150 + 10,
        bindings: vec![Binding {
            slot: 0,
            spell: scab,
        }],
        presses,
    };
    let result = fight.replay(&recording, false);
    result.damage
}

#[test]
fn bis_zur_grenze_aendert_sich_nichts() {
    // Die ersten drei Wuerfe stapeln; da gibt es keinen Ueberschuss.
    for n in 1..=3 {
        assert_eq!(
            schaden_nach(true, n),
            schaden_nach(false, n),
            "bei {n} Wuerfen darf der Effekt noch nichts auszahlen"
        );
    }
    // ZUSICHERUNG: Es wurde ueberhaupt Schaden gemacht.
    assert!(
        schaden_nach(false, 3) > 0,
        "kein Schaden, der Lauf misst nichts"
    );
}

#[test]
fn der_vierte_wurf_zahlt_aus() {
    let ohne = schaden_nach(false, 4);
    let mit = schaden_nach(true, 4);
    assert!(
        mit > ohne,
        "mit dem Effekt {mit}, ohne {ohne} — der Ueberschuss kommt nicht an"
    );
}

#[test]
fn und_jeder_weitere_wurf_zahlt_erneut() {
    let vier = schaden_nach(true, 4);
    let fuenf = schaden_nach(true, 5);
    let ohne_vier = schaden_nach(false, 4);
    let ohne_fuenf = schaden_nach(false, 5);
    // Der Zuwachs je Wurf ist mit dem Effekt groesser als ohne — und zwar um
    // denselben Betrag, den die volle Schicht nicht mehr aufnimmt.
    assert!(
        (fuenf - vier) > (ohne_fuenf - ohne_vier),
        "der zweite Ueberschuss zahlt nicht"
    );
}
