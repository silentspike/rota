//! Der Entlader-Weg des globalen Baums rechnet auf beiden Seiten gleich (#419).
//!
//! # Was diese Datei absichert
//!
//! `global-release` gibt Zaubern mit `spender` oder `consumes_decay_stacks`
//! einen Schadens-Zuschlag, jedem anderen nicht. Das ist der erste
//! Talent-Effekt, der JE ZAUBER verschieden ausfaellt — bis hierher galt jede
//! Talent-Groesse fuer den ganzen Kampf.
//!
//! # Warum KINDLE in derselben Aufzeichnung steht
//!
//! Truege der Zuschlag versehentlich auf ALLES, bliebe ein Lauf mit nur PYRE
//! trotzdem bitgleich — beide Seiten waeren dann eben gleich falsch. Erst der
//! Fueller daneben trennt die beiden Faelle. Die Aufzeichnung fuehrt deshalb
//! beide Zahlen in `expected`, und der erste Fall unten liest sie.
//!
//! # Die Verteilung kommt als PROMILLE herein
//!
//! Wie bei `decay-talent`: Der Simulator kennt die Talent-Daten nicht, er
//! beginnt bei den Promille. Was diese Datei deshalb NICHT prueft, ist die
//! Umrechnung Raenge -> Promille; sie prueft, ob beide Seiten mit DENSELBEN
//! Promille dasselbe rechnen.

use std::fs;

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

fn daten() -> Value {
    // **Der Pfad steht woertlich da, nicht zusammengesetzt** — der Waechter in
    // `gegenstueck.test.ts` sucht `/<name>.json` im Rust-Quelltext.
    let pfad = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/parity/recordings/spender-talent.json"
    );
    serde_json::from_str(&fs::read_to_string(pfad).unwrap_or_else(|_| panic!("{pfad} fehlt")))
        .unwrap_or_else(|_| panic!("{pfad} ist kein JSON"))
}

/// Baut aus den `presses` der Aufzeichnung eine abspielbare Runde.
///
/// Die Leiste entsteht dabei aus den vorkommenden Zaubern — die Aufzeichnung
/// fuehrt `spellId` je Druck, nicht eine feste Belegung.
fn runde(json: &Value) -> Recording {
    let mut bindings: Vec<Binding> = Vec::new();
    let mut presses: Vec<Press> = Vec::new();

    for eintrag in json["presses"].as_array().expect("presses fehlen") {
        let id = eintrag["spellId"].as_str().expect("spellId fehlt");
        let index = index_of(id).unwrap_or_else(|| panic!("Zauber {id} kennt der Simulator nicht"));
        let slot = match bindings.iter().position(|b| b.spell == index) {
            Some(s) => u8::try_from(s).expect("mehr Plaetze als eine Leiste fasst"),
            None => {
                let neu = u8::try_from(bindings.len()).expect("mehr Plaetze als eine Leiste fasst");
                bindings.push(Binding {
                    slot: neu,
                    spell: index,
                });
                neu
            }
        };
        presses.push(Press {
            tick: u32::try_from(eintrag["tick"].as_u64().expect("tick fehlt")).expect("zu gross"),
            slot,
        });
    }

    Recording {
        name: String::from("spender-talent"),
        seed: 0,
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks fehlen")).expect("zu gross"),
        bindings,
        presses,
    }
}

fn spielt(json: &Value, spender_permille: u32) -> rota_sim::replay::ReplayResult {
    let runde = runde(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let mut fight = Fight::new(FightConfig {
        boss: boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt")),
        duration_ms: Some(runde.ticks * TICK_MS),
        talent_spender_damage_permille: spender_permille,
        ..FightConfig::default()
    });
    fight.replay(&runde, true)
}

fn permille(json: &Value) -> u32 {
    u32::try_from(
        json["talentPermille"]["spenderDamage"]
            .as_u64()
            .expect("talentPermille.spenderDamage fehlt"),
    )
    .expect("zu gross")
}

/// **Die Zusicherung vor jedem Vergleich.**
///
/// Eine Aufzeichnung ohne Entlader belegt fuer den Zuschlag nichts, und eine
/// ohne Fueller trennt ihn nicht von einem Zuschlag auf alles.
#[test]
fn die_aufzeichnung_traegt_beide_faelle() {
    let json = daten();
    let pyre = json["expected"]["pyreCasts"]
        .as_u64()
        .expect("pyreCasts fehlt");
    let kindle = json["expected"]["kindleCasts"]
        .as_u64()
        .expect("kindleCasts fehlt");
    assert!(
        pyre > 0,
        "kein Entlader in der Aufzeichnung — sie belegt nichts"
    );
    assert!(
        kindle > 0,
        "kein Fueller in der Aufzeichnung — ein Zuschlag auf ALLES saehe genauso aus"
    );
    assert!(
        permille(&json) > 0,
        "ohne Promille misst der Vergleich nichts"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = daten();
    let ergebnis = spielt(&json, permille(&json));
    assert_eq!(
        u64::from(ergebnis.damage),
        json["expected"]["damage"].as_u64().expect("damage fehlt"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
    );
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = daten();
    let ergebnis = spielt(&json, permille(&json));
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();

    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "die Pruefsummen laufen auseinander"
    );
}

/// **Die Gegenprobe: ohne den Zuschlag ist es weniger.**
///
/// Ohne sie waere gruen hier auch dann gruen, wenn beide Seiten den Zuschlag
/// gleichermassen ignorierten.
#[test]
fn ohne_den_zuschlag_faellt_der_schaden() {
    let json = daten();
    let mit = spielt(&json, permille(&json)).damage;
    let ohne = spielt(&json, 0).damage;
    assert!(
        mit > ohne,
        "der Entlader-Zuschlag aendert gar nichts: {ohne} gegen {mit}"
    );
}
