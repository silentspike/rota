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

/// **Der Waechter ist gefallen — und das ist sein Erfolg** (Issue #223).
///
/// Bis heute stand hier ein `should_panic`-Fall: Eine Aufzeichnung mit REFLUX
/// brach den Lauf ab, weil `Fight::replay` den Auswahlmodus nicht kannte. Der
/// Druck-Pfad kennt ihn jetzt, also darf sie durchlaufen.
///
/// **Ersetzt, nicht geloescht.** Was er festhielt, ist jetzt die Zusicherung
/// in der anderen Richtung: Eine Aufzeichnung mit REFLUX laeuft durch UND der
/// Ruecksetzer wirkt. Ihn ersatzlos zu streichen hiesse, die Stelle
/// unbewacht zu lassen, an der beide Seiten heute frisch zusammengefuegt sind.
///
/// Der eigentliche Nachweis, dass sie gleich rechnen, steht in
/// `parity_cooldown_reset_run.rs` — Pruefsumme Takt fuer Takt.
#[test]
fn eine_aufzeichnung_mit_reflux_laeuft_durch_und_der_ruecksetzer_wirkt() {
    let reflux = index_of("reflux").expect("REFLUX gehoert seit Issue #201 zum Zauberkasten");
    let pyre = index_of("pyre").expect("pyre");
    let recording = Recording {
        name: "reflux-probe".to_owned(),
        seed: 0,
        ticks: 800,
        bindings: vec![
            Binding {
                slot: 0,
                spell: pyre,
            },
            Binding {
                slot: 1,
                spell: reflux,
            },
        ],
        // PYRE wirken, REFLUX wirken, PYRE waehlen, PYRE erneut wirken.
        presses: vec![
            Press { tick: 0, slot: 0 },
            Press { tick: 150, slot: 1 },
            Press { tick: 300, slot: 0 },
            Press { tick: 450, slot: 0 },
        ],
    };
    let mut fight = Fight::new(FightConfig::default());
    let result = fight.replay(&recording, false);

    assert_eq!(result.casts[reflux], 1, "REFLUX selbst wurde gewirkt");
    assert_eq!(
        result.casts[pyre], 2,
        "der zweite PYRE-Druck waere ohne den Ruecksetzer an der Abklingzeit \
         gescheitert — er belegt, dass die Wahl gegriffen hat"
    );
}

/// Und die Gegenrichtung: OHNE den Ruecksetzer scheitert derselbe Druck.
///
/// Ohne diesen Fall bliebe der obige auch dann gruen, wenn PYRE aus einem
/// ganz anderen Grund wieder bereit waere.
#[test]
fn ohne_den_ruecksetzer_scheitert_derselbe_zweite_druck() {
    let pyre = index_of("pyre").expect("pyre");
    let recording = Recording {
        name: "ohne-reflux".to_owned(),
        seed: 0,
        ticks: 800,
        bindings: vec![Binding {
            slot: 0,
            spell: pyre,
        }],
        presses: vec![Press { tick: 0, slot: 0 }, Press { tick: 450, slot: 0 }],
    };
    let mut fight = Fight::new(FightConfig::default());
    let result = fight.replay(&recording, false);

    assert_eq!(
        result.casts[pyre], 1,
        "PYRE hat 5 s Abklingzeit — der zweite Druck bei 4,5 s muss scheitern"
    );
}
