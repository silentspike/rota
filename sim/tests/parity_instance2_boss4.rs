//! Der 4. Boss der zweiten Instanz im Paritaets-Weg (Issue #574, AC4).
//!
//! # Was er verlangt
//!
//! die Kollision aus Rennen und Wirkzauber.
//!
//! # Warum HARTE ZAHLEN
//!
//! Eine Zusicherung, die nur prueft DASS ein Zauber gewirkt wird, faengt den
//! Fall aus #340 — aber nicht, dass sich die Aufzeichnung still verschoben
//! hat. Bei der ersten Instanz hat genau das eine veraltete Runde gefangen,
//! nachdem die Lebenspunkte eines Bosses geaendert wurden.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance2-boss-4.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance2-boss-4.json ist kein JSON");
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
        json["expected"]["casts"]["crust"].as_u64(),
        Some(5),
        "crust wird nicht 5-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["kindle"].as_u64(),
        Some(26),
        "kindle wird nicht 26-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["pyre"].as_u64(),
        Some(4),
        "pyre wird nicht 4-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["quench"].as_u64(),
        Some(5),
        "quench wird nicht 5-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["casts"]["smoulder"].as_u64(),
        Some(5),
        "smoulder wird nicht 5-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );

    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4920),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-2-4"),
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
    let boss = boss_by_id("boss-2-4").expect("boss-2-4 fehlt im Simulator");
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
    let boss = boss_by_id("boss-2-4").expect("boss-2-4 fehlt im Simulator");
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
