//! Vergleicht den Simulator mit einer im Spiel aufgezeichneten Runde.
//!
//! ```text
//! cargo run --example parity
//! ```
//!
//! Beendet sich mit 0, wenn beide Seiten uebereinstimmen, sonst mit 1. Bei einer
//! Abweichung nennt die Ausgabe **den Takt**, in dem sie zuerst auftritt — nicht
//! bloss die Tatsache, dass etwas nicht stimmt (Issue #54, AC4).
//!
//! Der Test `tests/parity_recording.rs` prueft dasselbe. Dieses Werkzeug gibt es
//! zusaetzlich, weil eine Fehlermeldung aus einem Testlauf schwer zu lesen ist,
//! wenn man wissen will, **was** sich zwischen zwei Takten unterscheidet — und
//! weil der CI-Job `parity` damit beide Seiten in einem Schritt ausfuehrt.

use std::process::ExitCode;

use serde_json::Value;

use rota_sim::data::{index_of, SPELLS};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Die aufgezeichnete Runde, wie das Spiel sie geschrieben hat.
const RECORDING_JSON: &str = include_str!("../../tests/parity/recordings/cooldown-precision.json");

fn main() -> ExitCode {
    let json: Value = match serde_json::from_str(RECORDING_JSON) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Die Aufzeichnung ist nicht lesbar: {error}");
            return ExitCode::FAILURE;
        }
    };

    let recording = build_recording(&json);
    let expected = &json["expected"];

    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, true);

    println!("{}", "=".repeat(66));
    println!("ROTA — Abgleich Simulator gegen Spiel");
    println!("Aufzeichnung \"{}\"", recording.name);
    println!(
        "{} Tastendruecke, Startwert {}, {} Takte",
        recording.presses.len(),
        recording.seed,
        recording.ticks
    );
    println!("{}", "=".repeat(66));
    println!();

    let mut ok = true;
    let mut check = |name: &str, spiel: i64, simulator: i64| {
        let gleich = spiel == simulator;
        if !gleich {
            ok = false;
        }
        println!(
            "  {name:<24} Spiel {spiel:>12}   Simulator {simulator:>12}   {}",
            if gleich { "gleich" } else { "ABWEICHUNG" }
        );
    };

    check(
        "Schaden",
        expected["damage"].as_i64().unwrap_or(-1),
        i64::from(result.damage),
    );
    check(
        "Sieg bei (ms)",
        expected["killedAtMs"].as_i64().unwrap_or(-1),
        result.killed_at_ms.map_or(-1, i64::from),
    );
    check(
        "angenommene Druecke",
        expected["accepted"].as_i64().unwrap_or(-1),
        i64::from(result.accepted),
    );
    check(
        "abgelehnte Druecke",
        expected["rejected"].as_i64().unwrap_or(-1),
        i64::from(result.rejected),
    );
    check(
        "Ziehungen",
        expected["randomDraws"].as_i64().unwrap_or(-1),
        i64::from(result.random_draws),
    );
    check(
        "Folge-Pruefsumme",
        expected["sequenceChecksum"].as_i64().unwrap_or(-1),
        i64::from(result.sequence_checksum),
    );
    check(
        "End-Pruefsumme",
        expected["finalChecksum"].as_i64().unwrap_or(-1),
        i64::from(result.final_checksum),
    );

    for spell in SPELLS.iter() {
        let index = index_of(spell.id).expect("Zauber aus SPELLS");
        check(
            &format!("Wirkungen {}", spell.id),
            expected["casts"][spell.id].as_i64().unwrap_or(-1),
            i64::from(result.casts[index]),
        );
    }

    println!();
    let expected_checksums: Vec<u32> = json["checksums"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_u64())
                .filter_map(|v| u32::try_from(v).ok())
                .collect()
        })
        .unwrap_or_default();

    match first_divergence(&expected_checksums, &result.checksums, TICK_MS) {
        None => {
            println!(
                "  Zustands-Pruefsumme      {} Takte, alle gleich",
                expected_checksums.len()
            );
        }
        Some(d) => {
            ok = false;
            println!("  Zustands-Pruefsumme      ABWEICHUNG");
            println!(
                "      erster abweichender Takt   {} ({} ms Kampfzeit)",
                d.tick, d.time_ms
            );
            println!("      Spiel                      {}", d.expected);
            println!("      Simulator                  {}", d.actual);
            println!("      gleiche Takte davor        {}", d.matched_ticks);
            println!(
                "      Laenge                     Spiel {}, Simulator {}",
                expected_checksums.len(),
                result.checksums.len()
            );
        }
    }

    println!();
    println!("{}", "-".repeat(66));
    if ok {
        println!("BESTANDEN: Simulator und Spiel rechnen dieselbe Runde gleich.");
        ExitCode::SUCCESS
    } else {
        println!("FEHLGESCHLAGEN: siehe ABWEICHUNG oben.");
        ExitCode::FAILURE
    }
}

fn build_recording(json: &Value) -> Recording {
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| Binding {
            slot: entry["slot"].as_u64().expect("slot fehlt") as u8,
            spell: index_of(entry["spellId"].as_str().expect("spellId fehlt"))
                .expect("unbekannter Zauber"),
        })
        .collect();

    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: entry["tick"].as_u64().expect("tick fehlt") as u32,
            slot: entry["slot"].as_u64().expect("slot fehlt") as u8,
        })
        .collect();

    Recording {
        name: json["name"].as_str().unwrap_or("?").to_owned(),
        seed: json["seed"].as_u64().unwrap_or(0) as u32,
        ticks: json["ticks"].as_u64().unwrap_or(0) as u32,
        bindings,
        presses,
    }
}
