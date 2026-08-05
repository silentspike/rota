//! Der Widerstand auf einer GEMISCHTEN Leiste (Issue #328, AC7).
//!
//! WARUM DIESE DATEI NEBEN `parity_resistances.rs` STEHT
//! ------------------------------------------------------
//! Seit dem 2026-08-05 haengt der Widerstand am ZAUBER und nicht mehr am Kampf.
//! Damit belegt eine Aufzeichnung, auf der nur Kalzinations-Zauber liegen, nur
//! noch den Fall, in dem ALLE Zauber dieselbe Schule haben — also genau den, in
//! dem die Umstellung nichts aendert.
//!
//! Hier liegen drei Schulen auf der Leiste:
//!
//! | Zauber | Schule | am Boss `test-resistant` |
//! | --- | --- | --- |
//! | `bind` | FIXATION | unberuehrt |
//! | `canker` | PUTREFACTION | unberuehrt |
//! | `kindle` | CALCINATION | 0,75 |
//!
//! Beide Seiten muessen dieselbe **Zuordnung** treffen, nicht nur dieselbe
//! Summe. Eine Umsetzung, die den Widerstand auf alles anwendet, kaeme auf eine
//! andere Zahl; eine, die ihn auf nichts anwendet, ebenfalls.
//!
//! Der Schaden JE ZAUBER steht mit in der Datei: Weichen die Seiten ab, sagt er
//! **welcher** Zauber falsch gerechnet wurde — statt nur, dass die Summe nicht
//! stimmt.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const MIXED_JSON: &str = include_str!("../../tests/parity/recordings/resistances-mixed.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(MIXED_JSON).expect("resistances-mixed.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn recording(json: &Value) -> Recording {
    // **Die Belegung kommt aus der DATEI**, nicht aus einer Abschrift hier.
    // Der Pruef-Zauber steht in keiner Leiste des Spiels; wer seinen Platz hier
    // hinschriebe, haette die Zuordnung an zwei Orten — und einer laeuft weg.
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(spell_id).unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt")),
            }
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick")).expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("ticks zu gross"),
        bindings,
        presses,
    }
}

fn replay(json: &Value) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
}

fn zahlen(json: &Value, feld: &str) -> Vec<i64> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| v.as_i64().expect("Zahl"))
        .collect()
}

#[test]
fn der_fall_kann_den_unterschied_ueberhaupt_zeigen() {
    // **Die Zusicherung, die diesem Pruefstand seinen Wert gibt**, und sie hat
    // beim Bauen zweimal angeschlagen: Alle drei Zauber haben 1500 bis 2000 ms
    // Abklingzeit, die Wartezeit hat 1500. Eine Vorrangliste laesst deshalb
    // immer denselben zum Zug kommen — gemessen erst {"kindle":1050,...} und
    // dann {"bind":840,...}, beide Male mit zwei Nullen.
    //
    // Eine "gemischte" Leiste, auf der nur eine Schule vorkommt, prueft genau
    // das nicht, wofuer es sie gibt.
    let json = data();
    let je_zauber = json["damageBySpell"]
        .as_object()
        .expect("damageBySpell fehlt");

    assert_eq!(je_zauber.len(), 3, "drei Zauber, drei Schulen");
    for (id, wert) in je_zauber {
        assert!(
            wert.as_u64().expect("Zahl") > 0,
            "{id} hat keinen Schaden gemacht — dann prueft der Fall seine Schule nicht"
        );
    }
}

#[test]
fn die_drei_zauber_gehoeren_drei_verschiedenen_schulen() {
    // Sonst waere es eine gemischte Leiste dem Namen nach. Ueber die
    // einkompilierten Daten geprueft, nicht ueber die Datei: Hier soll stehen,
    // dass der SIMULATOR die Schulen kennt.
    let schulen: Vec<Option<&str>> = ["bind", "canker", "kindle"]
        .iter()
        .map(|id| {
            let i = index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt"));
            rota_sim::data::SPELLS[i].school
        })
        .collect();
    assert_eq!(
        schulen,
        vec![Some("FIXATION"), Some("PUTREFACTION"), Some("CALCINATION")]
    );
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet: Vec<u32> = zahlen(&json, "checksums")
        .into_iter()
        .map(|v| u32::try_from(v).expect("32 Bit"))
        .collect();

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: Spiel {}, Simulator {}",
        erwartet.len(),
        result.checksums.len()
    );

    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             Schaden je Zauber im Spiel: {}",
            d.tick, d.time_ms, d.expected, d.actual, json["damageBySpell"],
        );
    }
}

#[test]
fn der_schaden_je_zauber_stimmt_auf_beiden_seiten() {
    // **Die eigentliche Aussage dieser Datei.** Die Pruefsumme traegt den
    // Gesamtschaden; sie kann nicht sagen, ob er richtig ZUGEORDNET ist. Zwei
    // Umsetzungen, die den Widerstand verschieden verteilen, koennen auf
    // dieselbe Summe kommen — hier nicht.
    let json = data();
    let result = replay(&json);
    let je_zauber = json["damageBySpell"]
        .as_object()
        .expect("damageBySpell fehlt");

    for (id, wert) in je_zauber {
        let i = index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt"));
        assert_eq!(
            u64::from(result.damage_by_spell[i]),
            wert.as_u64().expect("Zahl"),
            "Spiel und Simulator rechnen {id} verschieden — der Widerstand ist \
             der falschen Schule zugeordnet"
        );
    }
}

#[test]
fn der_schluss_stand_stimmt() {
    let json = data();
    let result = replay(&json);
    let erwartet = &json["expected"];

    assert_eq!(
        u64::from(result.damage),
        erwartet["damage"].as_u64().expect("damage fehlt"),
        "verschiedener Schaden"
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        erwartet["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum fehlt"),
        "verschiedene Folge-Pruefsumme"
    );
}
