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

/// Welche Zauber verkuerzen, um wie viel — und warum die Referenz das nicht merkt.
///
/// # Die Frage ist zweimal umgestellt worden, nie geloescht
///
/// Zuerst hiess dieser Fall `kein_ausgelieferter_zauber_verkuerzt_etwas` und
/// verlangte ausdruecklich seine eigene Abloesung:
///
///   "Zauber {} traegt eine Verkuerzung; dann ist AC5 pruefbar und dieser
///    Test gehoert ersetzt"
///
/// Mit VAPOUR (#201) wurde daraus `genau_ein_ausgelieferter_zauber_...`, und
/// der trug die Auflage weiter: "Wer hier einen Zauber ergaenzt, aendert die
/// Referenz-Rechnung — dann gehoert sie NACHGEMESSEN, nicht diese Liste
/// erweitert."
///
/// # Mit WISP (#480) ist genau das eingetreten, und hier ist die Nachmessung
///
/// Die Liste steht jetzt auf zwei Namen. Der Satz, den sie ersetzt, war aber
/// nicht "es ist genau einer", sondern "die Referenz-Rechnung ist unberuehrt"
/// — und der wird nicht laenger BEHAUPTET, sondern an den Daten geprueft:
/// Keiner der drei Zauber, die `rotation::good` ueberhaupt werfen kann, traegt
/// eine Verkuerzung. Deshalb kann die Mechanik im Referenzlauf gar nicht
/// ansetzen, egal wie viele Verkuerzer es sonst im Spiel gibt.
///
/// Das ist die schaerfere Zusicherung: Sie faellt auch dann, wenn jemand
/// KINDLE eine Verkuerzung gibt — der alte Listen-Vergleich haette das
/// gemeldet, ohne den Grund zu nennen, und ein blosses Erweitern der Liste
/// haette es verschluckt.
#[test]
fn welche_zauber_verkuerzen_und_warum_die_referenz_das_nicht_merkt() {
    let mit: Vec<&str> = data::SPELLS
        .iter()
        .filter(|spell| spell.cooldown_reduction_ms > 0)
        .map(|spell| spell.id)
        .collect();
    assert_eq!(
        mit,
        vec!["vapour", "wisp"],
        "die Verkuerzer haben gewechselt — dann gehoert die Nachmessung \
         darunter erneut gelesen, nicht nur diese Liste angepasst"
    );

    // Die Betraege stammen aus den Daten, nicht aus dem Code: Sie muessen dem
    // entsprechen, was `data/spells/sublimation.json` sagt.
    let betrag = |id: &str| {
        data::SPELLS
            .iter()
            .find(|spell| spell.id == id)
            .unwrap_or_else(|| panic!("{id} fehlt"))
            .cooldown_reduction_ms
    };
    assert_eq!(betrag("vapour"), 1000);
    assert_eq!(betrag("wisp"), 500, "WISP verkuerzt die Haelfte von VAPOUR");

    // **DIE NACHMESSUNG.** Der Referenzlauf bleibt unberuehrt, weil kein
    // Zauber, den er wirklich wirft, etwas verkuerzt. Das ist der Grund —
    // `der_referenzlauf_bleibt_unberuehrt` unten zeigt nur die Folge.
    //
    // # Abgelesen, nicht abgeschrieben
    //
    // Hier stand zuerst `[data::KINDLE, data::SMOULDER, data::PYRE]` — eine
    // ABSCHRIFT von `rotation::good`. Die haette einen vierten Zauber in der
    // Rotation nicht bemerkt, und damit genau den Fall verschluckt, den diese
    // Nachmessung abdecken soll. Dieselbe Klasse, vor der der alte Warnsatz
    // warnte, nur eine Ebene versetzt: eine Liste, die neben der Sache liegt,
    // statt aus ihr zu stammen.
    //
    // Jetzt wird der Referenzlauf GEFAHREN und abgelesen, welche Zauber
    // wirklich zum Zug kamen. Wer die Rotation aendert, aendert damit
    // automatisch, was hier geprueft wird.
    let mut referenz = Fight::new(FightConfig::default());
    let lauf = referenz.run(rotation::good);
    let gewirkt: Vec<usize> = (0..data::SPELL_COUNT)
        .filter(|&i| lauf.casts[i] > 0)
        .collect();

    // **Die Zusicherung in den Datensatz.** Ohne einen einzigen Wurf prueft
    // die Schleife nichts und waere trotzdem gruen.
    assert!(
        gewirkt.len() >= 3,
        "der Referenzlauf wirft nur {} Zauber — dann prueft diese Nachmessung \
         fast nichts. Sie gehoert nicht gelockert, sondern gelesen: Ist die \
         Referenz-Rotation kaputt?",
        gewirkt.len()
    );

    for index in gewirkt {
        let spell = &data::SPELLS[index];
        assert_eq!(
            spell.cooldown_reduction_ms, 0,
            "{} traegt eine Verkuerzung und wird im Referenzlauf GEWIRKT — \
             damit setzt die Mechanik dort an, und die Sollwerte in \
             der_referenzlauf_bleibt_unberuehrt gehoeren nachgerechnet",
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
        "die Mechanik hat angesetzt, obwohl kein Zauber der Referenz-Rotation \
         etwas verkuerzt — seit #480 gibt es ZWEI Verkuerzer im Spiel, nur \
         wirft rotation::good keinen davon (im Fall darueber nachgemessen)"
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
