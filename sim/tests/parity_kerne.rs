//! CLASP, LATCH und PLATE im Paritaets-Weg (Issue #519).
//!
//! # Warum es diese Datei braucht
//!
//! `coverage.test.ts` verlangt, dass jeder ausgelieferte Zauber in einer
//! Aufzeichnung **gewirkt** wird. Die Aufzeichnung allein reicht nicht: Liest
//! keine Rust-Datei sie, vergleicht niemand die beiden Seiten — der Befund aus
//! #456.
//!
//! **Diese Datei fehlte, und ich habe sie vergessen.** `kern-zauber.json` lag
//! seit #519 unter `recordings/`, wurde exportiert und von `freshness`
//! geprueft — aber auf der Rust-Seite von niemandem gelesen. Aufgefallen ist es
//! erst, als dieselbe Luecke bei `fixation-spells.json` einen echten
//! Paritaetsfehler verdeckt hatte.
//!
//! # Was sie zeigt
//!
//! CLASP sammelt Kontroll-Punkte, LATCH gibt sie fuer eine Verzoegerung aus,
//! PLATE legt eine grosse Schicht. Drei verschiedene Wirkungen aus zwei
//! Schulen in einem Lauf.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/kern-zauber.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("kern-zauber.json ist kein JSON");
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

/// **Die Zusicherung vor jeder Messung.**
///
/// Eine Aufzeichnung, in der KERN nicht vorkommt, belegt fuer ihn nichts —
/// und genau dafuer entsteht sie.
#[test]
fn clasp_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let clasp = index_of("clasp").expect("clasp fehlt im Simulator");
    let clasp_slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == clasp)
        .map(|b| b.slot)
        .expect("clasp liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == clasp_slot)
        .count();
    assert!(
        druecke > 0,
        "clasp liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    // Und das Spiel hat dasselbe gezaehlt.
    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["clasp"]
            .as_u64()
            .expect("casts.clasp fehlt"),
        "Simulator und Spiel zaehlen verschieden viele KERN-Wirkungen"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
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
    let mut fight = Fight::new(FightConfig {
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
