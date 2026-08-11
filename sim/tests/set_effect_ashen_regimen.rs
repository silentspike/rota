//! Der Satz-Effekt `ashen-regimen` wirkt auch im Simulator (13.5, Issue #480).
//!
//! Die sechste Satz-Stufe traegt keinen Werte-Bonus, sondern eine Kennung, auf
//! die der Kampf reagiert. Welche Effekte ein Aufbau ausloest, rechnet das
//! SPIEL (`rotationEffectsFrom`) — der Simulator bekommt das Ergebnis als
//! Wahrheitswert in der `FightConfig`, genau wie beim Schluesseltalent
//! `split_spender`.
//!
//! Geprueft wird deshalb hier nicht die Ableitung, sondern die WIRKUNG: Laesst
//! PYROCLASM ein Drittel der Hitze stehen, wenn das Feld gesetzt ist?

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Ein Lauf, der Hitze auflaedt und dann mit PYROCLASM entlaedt.
///
/// Gibt den Vorrat VOR dem Entladen und den Rest DANACH.
fn laden_und_entladen(ashen_regimen: bool) -> (u16, u16) {
    let boss = boss_by_id("first-crucible").expect("Boss fehlt");
    let kindle = index_of("kindle").expect("kindle fehlt");
    let pyroclasm = index_of("pyroclasm").expect("pyroclasm fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        ashen_regimen,
        ..FightConfig::default()
    });

    let mut presses: Vec<Press> = (0..12)
        .map(|n| Press {
            tick: n * 150,
            slot: 0,
        })
        .collect();
    let vor_dem_entladen_tick = 12 * 150;
    presses.push(Press {
        tick: vor_dem_entladen_tick,
        slot: 1,
    });

    let recording = Recording {
        name: "ashen".to_owned(),
        seed: 0,
        ticks: vor_dem_entladen_tick + 10,
        bindings: vec![
            Binding {
                slot: 0,
                spell: kindle,
            },
            Binding {
                slot: 1,
                spell: pyroclasm,
            },
        ],
        presses,
    };

    // **Der Messlauf muss GENAUSO LANG sein wie der echte** — die Lehre steht
    // ausfuehrlich in `keystone_split_spender.rs`: In den Takten zwischen
    // Ablesen und Entladen verfaellt Hitze, und ein kuerzerer Messlauf liest
    // einen Vorrat ab, mit dem das Entladen nie gerechnet hat.
    let mut messung = Fight::new(FightConfig {
        boss,
        ashen_regimen,
        ..FightConfig::default()
    });
    let nur_laden = Recording {
        presses: recording.presses[..12].to_vec(),
        ..recording.clone()
    };
    messung.replay(&nur_laden, false);
    let vorher = messung.resource();

    fight.replay(&recording, false);
    (vorher, fight.resource())
}

#[test]
fn ohne_den_effekt_verbraucht_pyroclasm_alles() {
    let (vorher, nachher) = laden_und_entladen(false);
    // ZUSICHERUNG: Es war ueberhaupt Hitze da — sonst waere die 0 unten aus
    // dem falschen Grund richtig.
    assert!(vorher > 0, "kein Vorrat aufgebaut, der Lauf misst nichts");
    assert_eq!(nachher, 0, "ohne den Effekt darf nichts stehen bleiben");
}

#[test]
fn mit_dem_effekt_bleibt_ein_drittel_stehen() {
    let (vorher, nachher) = laden_und_entladen(true);
    assert!(vorher > 0, "kein Vorrat aufgebaut, der Lauf misst nichts");
    assert_eq!(
        nachher,
        vorher / 3,
        "erwartet ein Drittel von {vorher}, gemessen {nachher}"
    );
    // Und es ist wirklich etwas uebrig — bei einem Drittel von 2 waere 0
    // richtig und der Fall trotzdem blind.
    assert!(nachher > 0, "ein Drittel ist 0 — der Fall belegt nichts");
}

#[test]
fn beide_laeufe_starten_gleich() {
    // Der Unterschied oben gehoert dem Effekt und nicht dem Aufbau. Waere der
    // Vorrat verschieden, liesse sich der Rest nicht zuschreiben.
    let (ohne, _) = laden_und_entladen(false);
    let (mit, _) = laden_und_entladen(true);
    assert_eq!(
        ohne, mit,
        "die zwei Laeufe bauen verschieden viel Hitze auf"
    );
}
