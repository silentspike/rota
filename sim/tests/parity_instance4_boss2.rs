//! Der 2. Boss der vierten Instanz im Paritaets-Weg.
//!
//! # Was er verlangt
//!
//! die TEILENTLADUNG: `energy.resetTo` steht auf 45 statt auf 0. Die Leiste
//! faellt nach dem Ausloesen nicht auf null zurueck, sondern auf 45 — der
//! Rhythmus verdichtet sich, waehrend der Kampf laeuft.
//!
//! Er ist der erste Boss im Bestand, bei dem `resetTo` ueberhaupt einen
//! anderen Wert traegt als 0. Vorher war das Feld vom Standardwert nicht
//! unterscheidbar; gemessen am 2026-08-12 loeste eine Aenderung daran keinen
//! einzigen Prueffall aus.
//!
//! # Warum dieselben Zahlen wie bei boss-4-3, und warum das RICHTIG ist
//!
//! Schaden und Druckfolge stimmen mit `parity_instance4_boss3` ueberein: Beide
//! Bosse verlangen keine Antwort auf der Leiste und bekommen deshalb dieselbe
//! Folge. **Die Pruefsumme unterscheidet sie** — sie traegt den Boss-Zustand,
//! hier die Energieleiste und dort die Stapel. Ein eigener Fall unten haelt
//! das fest.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance4-boss-2.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance4-boss-2.json ist kein JSON");
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
        Some(22),
        "kindle wird nicht 22-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["smoulder"].as_u64(),
        Some(4),
        "smoulder wird nicht 4-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4544),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    // **Die Gegenprobe zur Leiste:** Dieser Boss verlangt keine Unterbrechung.
    assert_eq!(
        json["expected"]["casts"]["quench"].as_u64(),
        Some(0),
        "quench wird gewirkt — die Rotation ist nicht mehr dieselbe"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-4-2"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
    // **Der Unterschied zu boss-4-3 steht in der Pruefsumme.** Waeren beide
    // Dateien versehentlich derselbe Kampf, waere auch sie gleich — und die
    // Teilentladung haette keinen Nachweis.
    assert_ne!(
        json["expected"]["finalChecksum"].as_u64(),
        Some(1_864_356_516),
        "diese Aufzeichnung ist dieselbe wie die von boss-4-3 — dann prueft sie die Leiste nicht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-4-2").expect("boss-4-2 fehlt im Simulator");
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

/// Takt fuer Takt — hier haengt der Stand der Energieleiste drin, und genau
/// der unterscheidet diesen Kampf von dem gegen boss-4-3.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-4-2").expect("boss-4-2 fehlt im Simulator");
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
