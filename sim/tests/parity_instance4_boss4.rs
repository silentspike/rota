//! Der 4. Boss der vierten Instanz im Paritaets-Weg.
//!
//! # Was er verlangt
//!
//! die LANGE VERKUERZUNG: `escalation` mit 200 ms Schritten bis hinunter auf
//! 600 ms. boss-1-1 ist nach drei Runden am Boden (400 ms bis 1200 ms), dieser
//! bleibt zehn Runden in Bewegung — das Fenster steht nie fest.
//!
//! **QUENCH kommt hier dreimal vor** und nirgends sonst in dieser Instanz. Das
//! ist der Traeger der Signatur: Ohne Unterbrechung betraefe die Aufzeichnung
//! den Boss nur von aussen, und die schrumpfende Wirkzeit waere eine Zahl in
//! den Daten statt einer Anforderung im Kampf.
//!
//! # escalation wirkt nur mit Wiederholung
//!
//! Sie greift an der Zeitleiste, und die muss sich wiederholen
//! (`timelineLoopMs`). Ohne sie gaebe es keine zweite Runde, in der etwas
//! kuerzer sein koennte. Derselbe Zusammenhang steht bei boss-1-1.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance4-boss-4.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance4-boss-4.json ist kein JSON");
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
        Some(23),
        "kindle wird nicht 23-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["smoulder"].as_u64(),
        Some(4),
        "smoulder wird nicht 4-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    // **Der Traeger der Signatur.** Drei Unterbrechungen — bei boss-1-1 mit
    // seiner schnell festen Frist sieht der Verlauf anders aus.
    assert_eq!(
        json["expected"]["casts"]["quench"].as_u64(),
        Some(3),
        "quench wird nicht 3-mal gewirkt — die Unterbrechung kommt nicht mehr vor"
    );
    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4684),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    // **Der Spieler ueberlebt.** Stuerbe er, endete die Aufzeichnung vorzeitig
    // und pruefte den Rest des Kampfes nicht mehr.
    assert_eq!(
        json["expected"]["diedAtMs"].as_i64(),
        Some(-1),
        "der Spieler stirbt — die Unterbrechung greift nicht mehr"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-4-4"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-4-4").expect("boss-4-4 fehlt im Simulator");
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

/// Takt fuer Takt — hier haengt die schrumpfende Wirkzeit drin.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-4-4").expect("boss-4-4 fehlt im Simulator");
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
