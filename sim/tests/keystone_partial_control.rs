//! FIXATIONs Schluesseltalent rechnet auf beiden Seiten gleich (#419).
//!
//! Ohne das Talent SCHEITERT ein Kontroll-Zauber, wenn der Vorrat nicht
//! reicht. Mit ihm wirkt er mit dem, was da ist — und die Wirkung sinkt im
//! selben Verhaeltnis.
//!
//! # Was hier die Paritaet traegt
//!
//! Nicht "es wirkt", sondern die RUNDUNG: `amount * faktor * einsatz / preis`
//! in EINER Rundung, in genau dieser Reihenfolge. Wer erst rundet und dann
//! teilt, bekommt bei 4 von 6 Punkten eine andere Zahl — und die
//! Bitgleichheit reisst an einer Stelle, die niemand mit dem Talent in
//! Verbindung braechte. Dieselbe Rechnung steht in `combat.ts` (`staerke`)
//! und im Spiel-Fall `tests/unit/keystone-partial-control.test.ts`.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Sammelt mit BIND Kontrolle und wirft dann ANCHOR (Kosten 6, Senkung 10 %).
///
/// `binds` steuert den Vorrat: BIND gibt zwei Punkte je Wurf, zwei Wuerfe sind
/// also vier von sechs Punkten.
fn mit_binds(binds: usize, partial_control: bool) -> u32 {
    let boss = boss_by_id("test-phase").expect("test-phase fehlt");
    let bind = index_of("bind").expect("bind fehlt");
    let anchor = index_of("anchor").expect("anchor fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        partial_control,
        ..FightConfig::default()
    });

    let bindings = vec![
        Binding {
            slot: 0,
            spell: bind,
        },
        Binding {
            slot: 1,
            spell: anchor,
        },
    ];
    // **150 Takte je Wartezeit** (TICK_MS 10, GCD_MS 1500). Zwei Druecke in
    // dieselbe Wartezeit zu legen hiesse, den zweiten zu verlieren.
    let mut presses: Vec<Press> = (0..binds)
        .map(|n| Press {
            tick: u32::try_from(n).expect("zu viele") * 160,
            slot: 0,
        })
        .collect();
    presses.push(Press {
        tick: u32::try_from(binds).expect("zu viele") * 160 + 160,
        slot: 1,
    });

    let recording = Recording {
        name: "partial-control".to_owned(),
        seed: 0,
        ticks: 800,
        bindings,
        presses,
    };
    fight.replay(&recording, false);
    fight.phase_hold_percent()
}

/// **Die Zusicherung vor der Messung.** Bei vollem Vorrat muss ANCHOR die
/// vollen zehn Prozentpunkte senken — sonst misst der Vergleich unten nicht
/// das Talent, sondern einen kaputten Aufbau.
#[test]
fn bei_vollem_vorrat_senkt_anchor_um_zehn() {
    // Der Zaehler `control_failed` ist auf dieser Seite nicht oeffentlich —
    // gemessen wird deshalb die WIRKUNG, und die ist die haertere Aussage:
    // 0 heisst, der Zauber hat nichts bewirkt.
    assert_eq!(
        mit_binds(3, false),
        10,
        "ANCHOR senkt bei vollem Vorrat nicht um 10"
    );
}

#[test]
fn ohne_talent_scheitert_er_bei_zu_wenig() {
    assert_eq!(
        mit_binds(2, false),
        0,
        "ohne Talent darf bei zu wenig Vorrat nichts gesenkt werden"
    );
}

#[test]
fn mit_talent_wirkt_er_anteilig() {
    // Vier von sechs Punkten: round(10 * 4 / 6) = 7. Dieselbe Zahl steht im
    // Spiel-Fall — sie ist der eigentliche Paritaets-Nachweis.
    assert_eq!(
        mit_binds(2, true),
        7,
        "mit Talent muss anteilig gesenkt werden"
    );
}

/// Und es ist KEIN Rabatt: Bei vollem Einsatz bleibt die volle Wirkung.
#[test]
fn bei_vollem_vorrat_aendert_das_talent_nichts() {
    assert_eq!(
        mit_binds(3, true),
        mit_binds(3, false),
        "bei vollem Vorrat darf sich nichts aendern"
    );
}
