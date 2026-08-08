//! Das VERLORENE Wettrennen (Issue #651).
//!
//! # Was diese Datei absichert, das `parity_race_heal.rs` nicht kann
//!
//! Seit #639 wird `race_won` zwischen beiden Seiten verglichen — aber
//! `race-heal.json` fuehrte den Wert als einzige Aufzeichnung, und dort steht
//! `true`. **Der verlorene Pfad wurde von keiner Aufzeichnung beruehrt:**
//! Liefen die Seiten dort auseinander, meldete es nichts.
//!
//! # Der Unterschied, auf den es ankommt
//!
//! Ein Lauf, in dem niemand drueckt, ergaebe ebenfalls `race_won == false` —
//! aber aus dem falschen Grund. Dort faende das Rennen gar nicht statt, das
//! Fenster liefe leer, und der Vergleich belegte nichts ueber das VERLIEREN.
//!
//! Gemessen ist der Unterschied `damage_lost_to_race`:
//!
//! ```text
//!   race_won=false, damage_lost_to_race=0    das Rennen fand NICHT statt
//!   race_won=false, damage_lost_to_race>0    es fand statt und ging verloren
//! ```
//!
//! `boss-1-4` verlangt 900 im Fenster von 6 s. Dieselbe gute Rotation, die bei
//! `test-racer` (Schwelle 300) gewinnt, schafft hier 780 — sie verliert also,
//! **ohne dass das Fenster leer bliebe.**

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const LOST_JSON: &str = include_str!("../../tests/parity/recordings/race-lost.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(LOST_JSON).expect("race-lost.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn recording(json: &Value) -> Recording {
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

#[test]
fn das_verlorene_rennen_stimmt_auf_beiden_seiten_ueberein() {
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    // **Die Zusicherung vor der Messung, und sie ist hier die eigentliche
    // Arbeit.** Ohne sie koennte diese Datei einen Lauf pruefen, in dem das
    // Rennen nie begann — `race_won` staende dann auf seinem Anfangswert, und
    // der Vergleich darunter waere `false` gegen `false` ohne Aussage.
    assert!(
        result.damage_lost_to_race > 0,
        "in diesem Lauf ist kein Schaden ins Fenster gefallen — dann hat das \
         Rennen gar nicht stattgefunden und der Vergleich belegt nichts ueber \
         das Verlieren"
    );
    assert_eq!(
        u64::from(result.damage_lost_to_race),
        expected["damageLostToRace"]
            .as_u64()
            .expect("damageLostToRace"),
        "verschieden viel Schaden ins Fenster gefallen"
    );

    // Die Aufzeichnung MUSS ein verlorenes Rennen fuehren — sonst deckt diese
    // Datei dieselbe Richtung ab wie parity_race_heal.rs und ist ueberfluessig.
    assert!(
        !expected["raceWon"].as_bool().expect("raceWon"),
        "die Aufzeichnung fuehrt ein GEWONNENES Rennen — dafuer gibt es \
         parity_race_heal.rs; diese Datei deckt die andere Richtung ab"
    );
    assert_eq!(
        result.race_won,
        expected["raceWon"].as_bool().expect("raceWon"),
        "die Seiten sind sich ueber den Ausgang des verlorenen Wettrennens nicht einig"
    );
}

#[test]
fn die_pruefsummen_stimmen() {
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"].as_u64().expect("finalChecksum"),
        "verschiedene Endsumme"
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        expected["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum"),
        "verschiedene Folgesumme"
    );
}
