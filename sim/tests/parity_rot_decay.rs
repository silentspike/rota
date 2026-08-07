//! ROT im Paritaets-Weg (Issue #477, Rezept aus #479).
//!
//! # Warum es diese Datei braucht
//!
//! `coverage.test.ts` verlangt, dass jeder ausgelieferte Zauber in einer
//! Aufzeichnung **gewirkt** wird — sonst gilt die Bitgleichheit fuer ihn als
//! angenommen. Die Aufzeichnung allein reicht dafuer aber nicht: Sie liegt
//! unter `recordings/`, und wenn keine Rust-Datei sie liest, vergleicht
//! niemand die beiden Seiten.
//!
//! Genau diese Luecke war der Befund aus #456: `synergy-window.json` stand
//! seit #318 da und in keinem Frische-Fall. Eine Aufzeichnung ohne
//! Gegenstueck sieht aus wie Absicherung und ist keine.
//!
//! `rot-decay.json` war der zweite Fall derselben Art: Sie kam mit #498 und
//! stand seither ohne Rust-Gegenstueck da. Aufgefallen ist sie nicht beim
//! Lesen, sondern durch den Waechter `gegenstueck.test.ts` — beim ersten
//! Nachzug, der ihn ueberhaupt fahren liess.
//!
//! # Was sie zeigt
//!
//! ROT, CANKER und KINDLE ueber 3000 Takte (7 / 7 / 6 Wirkungen, gezaehlt in
//! `expected.casts`). ROT ist der erste DoT einer Schule OHNE eigene
//! Ressource: Statt `resourceGainPerTick` traegt er `decayPerTick: 1` und
//! legt seine Stapel auf dem BOSS ab, wo sie einzeln verfallen. CANKER liest
//! sie (`decay.stacks: 1`).
//!
//! Beide Wege liegen damit auseinander — der eine erhoeht eine Ressource des
//! Spielers, der andere einen Zaehler am Ziel. Zwei Seiten, die dieselbe
//! Zahl am selben Ort fuehren muessen, ohne dass eine gemeinsame Ressource
//! sie zusammenhaelt.
//!
//! # Gegenprobe, damit gruen hier etwas heisst
//!
//! Am 2026-08-07 gemessen: `checksums[1700] + 1` und `expected.damage + 1`
//! in der Aufzeichnung, dann laufen lassen. Beide Faelle melden sich, und
//! der Takt-Fall nennt die Stelle statt nur „ungleich":
//!
//! ```text
//! Divergence { tick: 1700, time_ms: 17000,
//!              expected: 2060739798, actual: 2060739797, matched_ticks: 1700 }
//! ```
//!
//! Ohne diesen Beleg saehe die Datei genauso aus, wenn sie gar nichts
//! pruefte.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/rot-decay.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("rot-decay.json ist kein JSON");
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
/// Eine Aufzeichnung, in der ROT nicht vorkommt, belegt fuer ihn nichts —
/// und genau dafuer entsteht sie.
#[test]
fn rot_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let rot = index_of("rot").expect("rot fehlt im Simulator");
    let rot_slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == rot)
        .map(|b| b.slot)
        .expect("rot liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == rot_slot)
        .count();
    assert!(
        druecke > 0,
        "rot liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    // Und das Spiel hat dasselbe gezaehlt.
    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["rot"]
            .as_u64()
            .expect("casts.rot fehlt"),
        "Simulator und Spiel zaehlen verschieden viele ROT-Wirkungen"
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
