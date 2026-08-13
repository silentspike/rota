//! Der Stapel-Aufbau sammelt den Rest — auf beiden Seiten gleich (#419).
//!
//! # Warum es diese Datei gibt
//!
//! Der Rest ist ein ZUSTAND, der ueber den ganzen Kampf mitlaeuft. Zwei
//! Umsetzungen, die ihn verschieden fuehren, laufen auseinander, ohne dass es
//! sofort auffaellt: Der Unterschied zeigt sich erst beim uebernaechsten Wurf.
//!
//! Deshalb rechnet er GANZZAHLIG in Promille, hier wie im Spiel:
//!
//! ```text
//!   gesamt = anzahl x (1000 + promille) + rest
//!   ganze  = gesamt / 1000
//!   rest   = gesamt - ganze x 1000
//! ```
//!
//! # Die Zahlen stehen im Spiel-Fall daneben
//!
//! `tests/unit/decay-rest-sammeln.test.ts` fuehrt dieselbe Folge (1, 3, 4, 5)
//! mit derselben Begruendung. Wer eine Seite aendert, muss beide anfassen.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Wirft CANKER `wuerfe` mal im Abstand von 300 Takten und gibt die Stapel.
fn nach_wuerfen(wuerfe: u32, permille: u32) -> usize {
    let boss = boss_by_id("first-crucible").expect("first-crucible fehlt");
    let canker = index_of("canker").expect("canker fehlt");
    let mut fight = Fight::new(FightConfig {
        boss,
        talent_decay_stack_permille: permille,
        ..FightConfig::default()
    });
    let presses: Vec<Press> = (0..wuerfe)
        .map(|i| Press {
            tick: i * 300,
            slot: 0,
        })
        .collect();
    fight.replay(
        &Recording {
            name: "decay-rest".to_owned(),
            seed: 0,
            ticks: wuerfe * 300 + 10,
            bindings: vec![Binding {
                slot: 0,
                spell: canker,
            }],
            presses,
        },
        false,
    );
    fight.decay_stacks()
}

/// **Die Zusicherung vor der Messung.** Ohne Talent legt jeder Wurf genau
/// einen Stapel — sonst haette schon das alte Abschneiden gewirkt.
#[test]
fn ohne_talent_ein_stapel_je_wurf() {
    assert_eq!(
        nach_wuerfen(1, 0),
        1,
        "der erste Wurf legt nicht einen Stapel"
    );
    assert_eq!(nach_wuerfen(3, 0), 3, "drei Wuerfe legen nicht drei Stapel");
}

#[test]
fn faktor_eins_komma_fuenf_legt_bei_jedem_zweiten_wurf_nach() {
    // 500 Promille. Wurf 1: 1500 -> 1 Stapel, Rest 500.
    //                Wurf 2: 1500 + 500 = 2000 -> 2 Stapel, Rest 0.
    assert_eq!(nach_wuerfen(1, 500), 1, "der erste Wurf legt schon zwei");
    assert_eq!(nach_wuerfen(2, 500), 3, "der zweite Wurf legt nicht nach");
    assert_eq!(nach_wuerfen(3, 500), 4, "der dritte legt wieder nur einen");
}

/// **Die Gegenprobe.** Ohne den gesammelten Rest bliebe es bei einem Stapel je
/// Wurf — genau der Zustand, der den Weg wirkungslos machte.
#[test]
fn ohne_rest_waere_der_weg_wirkungslos() {
    assert_eq!(
        nach_wuerfen(2, 0),
        2,
        "ohne Talent muessen zwei Wuerfe zwei Stapel legen"
    );
    assert!(
        nach_wuerfen(2, 500) > nach_wuerfen(2, 0),
        "mit Talent kommt nach zwei Wuerfen kein Stapel dazu — der Rest wird verworfen"
    );
}
