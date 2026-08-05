//! Die neuen Kalzinations-Zauber rechnen auf beiden Seiten gleich (Issue #268).
//!
//! # Warum dieser Weg nötig war
//!
//! `coverage.test.ts` (Issue #340) verlangt, dass jeder ausgelieferte Zauber
//! in **einem** Paritäts-Weg gewirkt wird. Ohne ihn gilt die Bitgleichheit für
//! `bellows` und `searing` als *angenommen* — und eine Annahme ist genau das,
//! was die Vergleichsdateien ersetzen sollen.
//!
//! **Meine eigene Messung hätte das nicht geleistet.** Der Rotations-Läufer
//! aus #237 zeigt, dass beide Zauber gewählt werden — aber er läuft nur auf
//! dieser Seite. Er beantwortet „wird er gewählt", nicht „rechnen beide Seiten
//! ihn gleich". Der Wächter hat den Unterschied benannt, bevor ich ihn
//! gemeldet hätte.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/new-calcination.json");

fn data() -> Value {
    serde_json::from_str(JSON).expect("new-calcination.json ist kein JSON")
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

#[test]
fn beide_neuen_zauber_werden_ueberhaupt_gewirkt() {
    // **Der Gegenfall zuerst.** Wird einer von beiden nie gedrückt, ist der
    // Prüfsummen-Vergleich unten grün, ohne etwas über ihn zu sagen — genau
    // die Lücke, wegen der dieser Weg entstanden ist.
    let json = data();
    let casts = &json["expected"]["casts"];
    for id in ["bellows", "searing"] {
        let n = casts[id].as_u64().unwrap_or(0);
        assert!(
            n > 0,
            "{id} wird in dieser Aufzeichnung nicht gewirkt — dann belegt sie \
             fuer ihn nichts"
        );
    }
}

#[test]
fn die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    let json = data();
    let recording = recording(&json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect();

    // Auch hier: eine leere Liste waere Takt fuer Takt gleich.
    assert!(
        erwartet.len() > 100,
        "nur {} Takte aufgezeichnet — zu wenige fuer eine Aussage",
        erwartet.len()
    );

    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, true);

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: Simulator {}, Spiel {}",
        result.checksums.len(),
        erwartet.len()
    );
    for (takt, (hier, dort)) in result.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(
            hier,
            dort,
            "erste Abweichung in Takt {takt} ({} ms): Spiel {dort}, Simulator {hier}",
            takt as u32 * TICK_MS
        );
    }
}
