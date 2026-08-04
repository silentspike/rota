//! SUBLIMATION im Kampf (Issue E3-3).
//!
//! Die Rechnung selbst prueft `src/sublimation.rs` in seinen eigenen Tests —
//! ohne Kampf, weil sie ohne Kampf auskommt. Hier steht, was nur der Kampf
//! zeigen kann: dass die Mechanik **angemeldet** ist, an der richtigen Stelle
//! der Reihenfolge steht, und dass sie heute nichts tut.
//!
//! # Warum "tut heute nichts" ein Ergebnis ist und kein Mangel
//!
//! Kein ausgelieferter Zauber traegt `cooldownReductionMs` — SUBLIMATION hat
//! im Design noch keine Zauber (GAME-DESIGN 22 setzt die ersten zwei auf
//! Schritt 3 an). Der Empfaenger laeuft also mit und verkuerzt nichts, und
//! genau deshalb bleibt die Referenz-Rechnung unberuehrt.
//!
//! **Seit Issue #201 traegt VAPOUR das Feld.** Damit ist die Verkuerzung auf
//! beiden Seiten an einem echten Zauber pruefbar; der Platzhalter-Fall ist
//! ersetzt, nicht geloescht.
//!
//! REFLUX dagegen kann der Simulator noch nicht: Der Druck-Pfad in
//! `Fight::replay` kennt nur "wirken" und "ablehnen", nicht "waehlen". Das ist
//! Issue #223, und der Waechter unten haelt es fest.

use rota_sim::data::index_of;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::rules_event::RuleEvent;
use rota_sim::{data, rotation, Fight, FightConfig};

#[test]
fn die_mechanik_ist_angemeldet_und_steht_hinter_der_stapel_mechanik() {
    let fight = Fight::new(FightConfig::default());
    let ids = fight.rule_events().receiver_ids(RuleEvent::SpellCast);

    let stapel = ids
        .iter()
        .position(|id| id == "stacks:heat")
        .expect("die Stapel-Mechanik muss angemeldet sein");
    let subl = ids
        .iter()
        .position(|id| id == "sublimation")
        .expect("die Sublimation muss angemeldet sein");
    assert!(
        stapel < subl,
        "Rang 10 muss hinter Rang 0 stehen, Reihenfolge war {ids:?}"
    );
}

#[test]
fn genau_ein_ausgelieferter_zauber_verkuerzt_etwas() {
    // **ERSETZT, nicht geloescht** (Issue #201). Vorher hiess dieser Fall
    // `kein_ausgelieferter_zauber_verkuerzt_etwas` und verlangte ausdruecklich
    // seine eigene Abloesung:
    //
    //   "Zauber {} traegt eine Verkuerzung; dann ist AC5 pruefbar und dieser
    //    Test gehoert ersetzt"
    //
    // Seit VAPOUR traegt einer sie. Ihn zu loeschen haette die Absicherung
    // herausgenommen, die #104 verlangt — geblieben ist deshalb die Frage,
    // nur andersherum gestellt: WELCHE Zauber verkuerzen, und verkuerzen sie
    // dasselbe wie im Spiel?
    let mit: Vec<&str> = data::SPELLS
        .iter()
        .filter(|spell| spell.cooldown_reduction_ms > 0)
        .map(|spell| spell.id)
        .collect();
    assert_eq!(
        mit,
        vec!["vapour"],
        "Wer hier einen Zauber ergaenzt, aendert die Referenz-Rechnung — \
         dann gehoert sie nachgemessen, nicht diese Liste erweitert"
    );

    // Und der Betrag stammt aus den Daten, nicht aus dem Code: Er muss dem
    // entsprechen, was `data/spells/sublimation.json` sagt.
    let vapour = data::SPELLS
        .iter()
        .find(|spell| spell.id == "vapour")
        .expect("vapour");
    assert_eq!(vapour.cooldown_reduction_ms, 1000);
}

#[test]
fn der_referenzlauf_bleibt_unberuehrt() {
    let mut fight = Fight::new(FightConfig::default());
    let result = fight.run(rotation::good);

    assert_eq!(result.damage, 4240, "die Referenz-Rechnung hat sich bewegt");
    assert_eq!(result.killed_at_ms, Some(40_510));
    assert_eq!(
        fight.sublimation_use_count(),
        0,
        "die Mechanik hat angesetzt, obwohl kein Zauber etwas verkuerzt"
    );
}

/// Der Waechter greift, sobald eine Aufzeichnung REFLUX enthaelt (Issue #223).
///
/// **Der Gegenbeweis zum Waechter selbst.** Ein Waechter, der nie anschlaegt,
/// ist von einem fehlenden nicht zu unterscheiden — deshalb wird hier eine
/// Aufzeichnung gebaut, die ihn ausloest, und belegt, dass sie es tut.
#[test]
#[should_panic(expected = "Auswahlmodus")]
fn eine_aufzeichnung_mit_reflux_bricht_den_lauf_ab() {
    let reflux = index_of("reflux").expect("REFLUX gehoert seit Issue #201 zum Zauberkasten");
    let recording = Recording {
        name: "reflux-probe".to_owned(),
        seed: 0,
        ticks: 10,
        bindings: vec![Binding {
            slot: 0,
            spell: reflux,
        }],
        presses: vec![Press { tick: 1, slot: 0 }],
    };
    let mut fight = Fight::new(FightConfig::default());
    fight.replay(&recording, false);
}

/// Und die Gegenrichtung: Ohne REFLUX laeuft dieselbe Aufzeichnung durch.
///
/// Ohne diesen Fall koennte der Waechter bei JEDEM Druck anschlagen, und der
/// Fall darueber wuerde es nicht merken.
#[test]
fn dieselbe_aufzeichnung_mit_kindle_laeuft_durch() {
    let kindle = index_of("kindle").expect("kindle");
    let recording = Recording {
        name: "kindle-probe".to_owned(),
        seed: 0,
        ticks: 10,
        bindings: vec![Binding {
            slot: 0,
            spell: kindle,
        }],
        presses: vec![Press { tick: 1, slot: 0 }],
    };
    let mut fight = Fight::new(FightConfig::default());
    let result = fight.replay(&recording, false);
    assert_eq!(result.casts[kindle], 1, "KINDLE muss gewirkt worden sein");
}
