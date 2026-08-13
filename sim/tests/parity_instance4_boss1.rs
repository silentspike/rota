//! Der 1. Boss der vierten Instanz im Paritaets-Weg.
//!
//! # Was er verlangt
//!
//! die VERWUNDBARKEIT: `resistances` hebt CALCINATION auf 1.25. Er ist der
//! erste ausgelieferte Boss, bei dem derselbe Zauber MEHR anrichtet als
//! anderswo — bis dahin gab es nur die Gegenrichtung (boss-3-1 mit 0.8).
//!
//! Er traegt die meisten Lebenspunkte der Instanz (4700) und faellt trotzdem
//! als Erster (37510 ms gegen 43510 und 45010). Genau daran ist der Baustein
//! zu erkennen: nicht an der Zahl in den Daten, sondern am Verlauf.
//!
//! # Warum diese Datei existiert
//!
//! Der Waechter `jeder-boss-hat-eine-aufzeichnung` hat sie eingefordert, als
//! instance-4 in `instances.json` eingetragen wurde. Ohne Aufzeichnung waeren
//! Spiel und Simulator bei diesen vier Bossen nie verglichen worden.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance4-boss-1.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance4-boss-1.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

fn u32_at(node: &Value, key: &str) -> u32 {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    u32::try_from(raw).expect("passt nicht in 32 Bit")
}

fn recording(json: &Value) -> Recording {
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt"))
                    .expect("slot > 255"),
                spell: index_of(spell_id)
                    .unwrap_or_else(|| panic!("Zauber {spell_id} kennt der Simulator nicht")),
            }
        })
        .collect();

    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick fehlt"))
                .expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name fehlt").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed fehlt")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks fehlen"))
            .expect("ticks zu gross"),
        bindings,
        presses,
    }
}

/// **Die Zusicherung vor jeder Messung** — mit Zahlen, nicht mit Anwesenheit.
#[test]
fn die_aufzeichnung_ist_noch_dieselbe() {
    let json = data();
    assert_eq!(
        json["expected"]["casts"]["kindle"].as_u64(),
        Some(20),
        "kindle wird nicht 20-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["smoulder"].as_u64(),
        Some(3),
        "smoulder wird nicht 3-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["pyre"].as_u64(),
        Some(3),
        "pyre wird nicht 3-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4790),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    // **Der Traeger der Signatur.** Weniger Druecke als bei den anderen dreien
    // und trotzdem der hoechste Schaden: Das ist die Verwundbarkeit.
    assert_eq!(
        json["expected"]["accepted"].as_u64(),
        Some(26),
        "andere Zahl angenommener Druecke — der Kampf ist nicht mehr derselbe"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-4-1"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    // **Der Boss gehoert in die Konfiguration.** Ohne diese Zeile faehrt der
    // Vergleich gegen den Standard-Boss — und dessen Widerstaende sind leer,
    // womit genau die Signatur dieses Bosses aus der Messung fiele.
    let boss = boss_by_id("boss-4-1").expect("boss-4-1 fehlt im Simulator");
    let mut fight = Fight::new(FightConfig {
        boss,
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);
    let erwartet = &json["expected"];

    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
    );
    assert_eq!(
        ergebnis.accepted,
        u32_at(erwartet, "accepted"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert_eq!(
        ergebnis.final_checksum,
        u32_at(erwartet, "finalChecksum"),
        "die Schluss-Pruefsumme weicht ab"
    );
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-4-1").expect("boss-4-1 fehlt im Simulator");
    let mut fight = Fight::new(FightConfig {
        boss,
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);

    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();

    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
