//! Der 2. Boss der ersten Instanz im Paritaets-Weg (Issue #544, AC4).
//!
//! # Was diese Datei zeigt
//!
//! Dieselbe Tastenfolge gegen einen anderen Boss. Die vier Aufzeichnungen der
//! Instanz unterscheiden sich NUR im Gegner — was sie trennt, ist also
//! ausschliesslich dessen Verhalten.
//!
//! # Warum hier HARTE ZAHLEN stehen
//!
//! Eine Zusicherung, die nur prueft, DASS ein Zauber gewirkt wird, faengt den
//! Fall aus #340 (ein Zauber, den niemand drueckt) — aber nicht, dass sich
//! die Aufzeichnung still verschoben hat. Die Zahlen unten sagen, WIE OFT.
//! Aendert jemand den Boss oder die Rotation, meldet sich das hier, statt
//! stillschweigend eine andere Runde zu vergleichen.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance1-boss-2.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance1-boss-2.json ist kein JSON");
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
        Some(30),
        "kindle wird nicht 30-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["pyre"].as_u64(),
        Some(5),
        "pyre wird nicht 5-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["quench"].as_u64(),
        Some(1),
        "quench wird nicht 1-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["smoulder"].as_u64(),
        Some(5),
        "smoulder wird nicht 5-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );

    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(5952),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-1-2"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    // **Der Boss gehoert in die Konfiguration.** Ohne diese Zeile faehrt der
    // Vergleich gegen den Standard-Boss, und die Aufzeichnung waere zwar
    // gruen zu bekommen, verglichen wuerde aber ein anderer Kampf.
    let boss = boss_by_id("boss-1-2").expect("boss-1-2 fehlt im Simulator");
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
    // **Der Boss gehoert in die Konfiguration.** Ohne diese Zeile faehrt der
    // Vergleich gegen den Standard-Boss, und die Aufzeichnung waere zwar
    // gruen zu bekommen, verglichen wuerde aber ein anderer Kampf.
    let boss = boss_by_id("boss-1-2").expect("boss-1-2 fehlt im Simulator");
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

    // `first_divergence` nennt den Takt statt nur „ungleich" — bei dreitausend
    // Eintraegen ist das der Unterschied zwischen einem Hinweis und einer
    // Suche.
    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
