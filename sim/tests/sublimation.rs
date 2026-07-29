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
//! **AC5 des Issues bleibt damit ausdruecklich offen**: Dass Spiel und
//! Simulator dieselbe Verkuerzung rechnen, laesst sich erst zeigen, wenn ein
//! Zauber das Feld traegt. Beide Seiten pruefen heute dieselben Aussagen
//! ueber die Rechnung — das ist etwas anderes als ein gemeinsamer Prueffall.

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
fn kein_ausgelieferter_zauber_verkuerzt_etwas() {
    // Der Grund, warum die Referenz unberuehrt bleibt — und zugleich der
    // Grund, warum AC5 offen ist.
    for spell in data::SPELLS {
        assert_eq!(
            spell.cooldown_reduction_ms, 0,
            "Zauber {} traegt eine Verkuerzung; dann ist AC5 pruefbar \
             und dieser Test gehoert ersetzt",
            spell.id
        );
    }
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
