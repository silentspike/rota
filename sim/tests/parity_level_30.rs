//! Die STUFE, Takt fuer Takt (Issue #289, AC5).
//!
//! WARUM ES DIESE DATEI GIBT
//! -------------------------
//! Bis zum 2026-08-05 lag **keine einzige Vergleichsdatei ueber Stufe 1** — und
//! dort ist `stat_at_level` neutral. Die Bitgleichheit war damit fuer jede
//! hoehere Stufe unbelegt: Ein Auseinanderlaufen waere unsichtbar geblieben,
//! weil kein Fall es zeigen konnte.
//!
//! Das ist die Klasse „der Prueffall kann den Unterschied nicht zeigen", an der
//! teuersten Stelle, an der sie stehen kann. Die Bitgleichheit ist das
//! Fundament, auf dem der Balance-Laeufer ruht — sie fuer den einen Wert zu
//! belegen, bei dem beide Seiten ohnehin dasselbe tun, belegt nichts.
//!
//! **Stufe 30, nicht 2.** Der Faktor ist dort 4,116 und trennt die Zahlen
//! deutlich; bei 1,05 saehe ein Rundungsfehler aus wie Uebereinstimmung.
//!
//! Die Stufe steht IN der Datei (`playerLevel`), nicht in der Annahme des
//! Lesers.

use serde_json::Value;

use rota_sim::data::{index_of, stat_at_level, PLAYER};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const LEVEL_JSON: &str = include_str!("../../tests/parity/recordings/level-30.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(LEVEL_JSON).expect("level-30.json ist kein JSON");
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

/// Die Stufe kommt AUS DER DATEI, nicht aus einer Konstante hier.
fn level(json: &Value) -> u32 {
    u32::try_from(json["playerLevel"].as_u64().expect("playerLevel fehlt")).expect("zu gross")
}

fn replay(json: &Value) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        duration_ms: Some(recording.ticks * TICK_MS),
        player_level: level(json),
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
}

fn zahlen(json: &Value, feld: &str) -> Vec<u32> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect()
}

#[test]
fn die_stufe_trennt_die_zahlen_ueberhaupt() {
    // **Die Zusicherung, die diesem Pruefstand seinen Wert gibt.** Auf Stufe 1
    // ist `stat_at_level` neutral — ein Vergleich dort belegt nichts ueber die
    // Stufe. Diese Datei taugt nur, solange die Zahlen auseinandergehen.
    let json = data();
    let stufe = level(&json);
    assert!(stufe > 1, "auf Stufe 1 pruefte der Fall nichts");

    let grundwert = PLAYER.vessel;
    let auf_stufe = stat_at_level(grundwert, stufe);
    assert!(
        auf_stufe > grundwert * 2,
        "Stufe {stufe}: {auf_stufe} gegen {grundwert} — zu nah beieinander, \
         ein Rundungsfehler saehe aus wie Uebereinstimmung"
    );

    // Und die Datei sagt dasselbe.
    assert_eq!(
        json["player"]["vesselAtLevel"].as_u64(),
        Some(u64::from(auf_stufe)),
        "Spiel und Simulator rechnen den Grundwert der Stufe verschieden"
    );
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet = zahlen(&json, "checksums");

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: Spiel {}, Simulator {}",
        erwartet.len(),
        result.checksums.len()
    );

    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auf Stufe {} auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}",
            level(&json),
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
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
        "verschiedener Schaden auf Stufe {}",
        level(&json)
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        erwartet["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum fehlt"),
        "verschiedene Folge-Pruefsumme"
    );
}
