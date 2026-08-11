//! Der Satz-Effekt `whole-work` wirkt auch im Simulator (13.5, Issue #480).
//!
//! Der globale Cooldown sinkt um 100 ms. Gemessen wird nicht die Konstante,
//! sondern was sie BEWIRKT: wie viele Wuerfe in dieselbe Zeit passen.
//!
//! **Warum das der schaerfere Nachweis ist:** Eine Pruefung auf `gcd_length()`
//! vergliche eine Zahl mit sich selbst. Erst die Wurfzahl zeigt, dass die
//! Laenge an beiden Ausloesestellen ankommt — es sind zwei, und ein Abzug an
//! nur einer waere still.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Wie viele KINDLE in zehn Sekunden durchgehen.
///
/// Gedrueckt wird auf JEDEM Takt — so entscheidet allein die Wartezeit, wie
/// viele Wuerfe ankommen, nicht der Abstand der Druecke.
fn wuerfe_in_zehn_sekunden(whole_work: bool) -> u32 {
    let boss = boss_by_id("first-crucible").expect("Boss fehlt");
    let kindle = index_of("kindle").expect("kindle fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        whole_work,
        ..FightConfig::default()
    });

    let smoulder = index_of("smoulder").expect("smoulder fehlt");
    let ticks = 1000;
    // **ZWEI Zauber abwechselnd, nicht einer.** Die kleinste Abklingzeit im
    // ganzen Bestand ist 1500 ms — genau die Wartezeit. Bei EINEM Zauber wird
    // sie ab 1400 zur Bremse, und die kuerzere Wartezeit bewegt nichts.
    // Gemessen: 7 Wuerfe mit und ohne Effekt.
    //
    // Erst im Wechsel zweier Zauber ist die Wartezeit die Bremse — und genau
    // dort wirkt der Effekt.
    let presses: Vec<Press> = (0..ticks)
        .map(|tick| Press {
            tick,
            slot: u8::from(tick % 2 == 1),
        })
        .collect();
    let recording = Recording {
        name: "whole-work".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![
            Binding {
                slot: 0,
                spell: kindle,
            },
            Binding {
                slot: 1,
                spell: smoulder,
            },
        ],
        presses,
    };
    let result = fight.replay(&recording, false);
    result.casts.iter().sum()
}

#[test]
fn mit_dem_effekt_gehen_mehr_wuerfe_durch() {
    let ohne = wuerfe_in_zehn_sekunden(false);
    let mit = wuerfe_in_zehn_sekunden(true);

    // ZUSICHERUNG: Es wurde ueberhaupt geworfen — sonst waere 0 > 0 falsch
    // und der Fall aus dem richtigen Grund rot, aber blind.
    assert!(ohne > 0, "kein einziger Wurf, der Lauf misst nichts");
    assert!(
        mit > ohne,
        "mit dem Effekt {mit} Wuerfe, ohne {ohne} — die kuerzere Wartezeit kommt nicht an"
    );
}

#[test]
fn die_zahl_stimmt_und_nicht_nur_die_richtung() {
    // Zehn Sekunden im Wechsel zweier Zauber, Wartezeit 1500 gegen 1400.
    // Gemessen statt gerechnet: Der Fall haelt fest, was herauskommt, und
    // faellt, wenn sich eine der beteiligten Groessen bewegt.
    //
    // Ein Wurf mehr in zehn Sekunden ist genau, was 100 ms weniger Wartezeit
    // hergeben: 10000/1400 = 7,1 gegen 10000/1500 = 6,7.
    assert_eq!(wuerfe_in_zehn_sekunden(false), 7);
    assert_eq!(wuerfe_in_zehn_sekunden(true), 8);
}

#[test]
fn ohne_den_effekt_bleibt_es_beim_grundwert() {
    // Der Gegenfall zur Wurfzahl: Die Wartezeit selbst.
    let boss = boss_by_id("first-crucible").expect("Boss fehlt");
    let ohne = Fight::new(FightConfig {
        boss,
        whole_work: false,
        ..FightConfig::default()
    });
    let mit = Fight::new(FightConfig {
        boss,
        whole_work: true,
        ..FightConfig::default()
    });
    assert_eq!(ohne.gcd_length_for_test(), 1500);
    assert_eq!(mit.gcd_length_for_test(), 1400);
}
