//! Der 3. Boss der vierten Instanz im Paritaets-Weg.
//!
//! # Was er verlangt
//!
//! die SPAETE STEIGERUNG: `stacks` mit 8000/12/4/6 statt der 4000/15/2/4 von
//! boss-3-3. Der Aufbau dauert dreimal so lange, faellt dafuer doppelt so
//! steil aus und ist erst bei sechs Stapeln gedeckelt — statt einer
//! Anfangshuerde ist es eine Uhr, die den ganzen Kampf ueber laeuft.
//!
//! # Warum dieselben Zahlen wie bei boss-4-2
//!
//! Beide bekommen dieselbe Druckfolge, weil keiner eine Antwort auf der Leiste
//! verlangt. Die Begruendung samt Gegenprobe steht in
//! `parity_instance4_boss2`.
//!
//! **Was hier zusaetzlich gilt:** Der Stapel-Zaehler steht in der Pruefsumme,
//! nicht im Schaden am Boss. Ein Simulator, der die Stapel anders zaehlt oder
//! frueher deckelt, rechnete denselben Schaden — und faellt trotzdem auf, aber
//! nur beim Vergleich Takt fuer Takt.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance4-boss-3.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance4-boss-3.json ist kein JSON");
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
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-4-3"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
    // **Der Unterschied zu boss-4-2 steht in der Pruefsumme.**
    assert_ne!(
        json["expected"]["finalChecksum"].as_u64(),
        Some(3_977_015_145),
        "diese Aufzeichnung ist dieselbe wie die von boss-4-2 — dann prueft sie die Stapel nicht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-4-3").expect("boss-4-3 fehlt im Simulator");
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

/// Takt fuer Takt — hier haengt der Stapel-Zaehler drin.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-4-3").expect("boss-4-3 fehlt im Simulator");
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
