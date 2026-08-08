//! Der Verhaengnis-Zaehler, BEDIENT (Issue #561, AC4).
//!
//! # Was diese Datei absichert, das `parity_doom_stacks.rs` nicht kann
//!
//! Dort laeuft der Zaehler voll und toetet — der Lauf belegt, dass beide
//! Seiten dasselbe TUN, wenn niemand eingreift. Hier greift jemand ein:
//! `purge` senkt, die Ladung waechst nach.
//!
//! **Bis #561 gab es keinen Zauber, der `clear_doom()` ruft.** Die Methode
//! stand seit #216 da und niemand rief sie; `boss-1-2` war damit unschaffbar.
//! Ohne diese Aufzeichnung gaelte die Bitgleichheit der Senkung als
//! angenommen, nicht als geprueft — `coverage.test.ts` nennt den Zauber dann
//! beim Namen.
//!
//! # Die Nachladezeit ist auf beiden Seiten ABGELEITET
//!
//! `clear_amount / amount * per_step_ms`, kein Datenfeld. Die Pruefung sitzt
//! auf beiden Seiten IM DOOM-TAKT — eine feinere Aufloesung auf einer der
//! beiden liesse die Ladung dort frueher zurueckkommen, und ab dem ersten
//! Unterschied rechneten sie verschiedenen Schaden.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, stack_damage};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Der aufgezeichnete Lauf, wie das Spiel ihn geschrieben hat.
const DOOM_JSON: &str = include_str!("../../tests/parity/recordings/doom-purge.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(DOOM_JSON).expect("doom-purge.json ist kein JSON");
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

fn replay(json: &Value) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
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
fn die_senkung_stimmt_auf_beiden_seiten_ueberein() {
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    // **Die Zusicherung vor der Messung.** Wuerde in diesem Lauf gar nicht
    // gesenkt, verglichen die Zeilen darunter zwei unberuehrte Zaehler.
    let gewirkt = expected["casts"].as_u64().expect("casts");
    assert!(
        gewirkt > 0,
        "in dieser Aufzeichnung wird purge nie gewirkt — dann prueft der Fall nichts"
    );

    assert_eq!(
        u64::from(result.damage),
        expected["damage"].as_u64().expect("damage"),
        "verschiedener Schaden"
    );
    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"].as_u64().expect("finalChecksum"),
        "verschiedene Endsumme"
    );
}

#[test]
fn die_ladungen_laufen_takt_fuer_takt_gleich() {
    let json = data();
    let result = replay(&json);

    let erwartet = zahlen(&json, "doomCharges");
    assert_eq!(
        result.doom_charges.len(),
        erwartet.len(),
        "verschieden viele Takte in der Ladungs-Spur"
    );

    // **Die Spur ist die eigentliche Aussage.** Eine abweichende Pruefsumme
    // sagt nur, DASS etwas anders ist; hier zeigt sich, ob es der Verbrauch
    // oder das Nachladen war.
    for (tick, (ist, soll)) in result.doom_charges.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(ist, soll, "Ladungen verschieden im Takt {tick}");
    }

    // Und die Zusicherung, dass die Spur ueberhaupt etwas zeigt: Waere sie
    // konstant, liefe der Vergleich oben ueber einen unbewegten Wert.
    let verschiedene: std::collections::BTreeSet<u32> = erwartet.iter().copied().collect();
    assert!(
        verschiedene.len() > 1,
        "die Ladungs-Spur ist konstant — dann belegt sie weder Verbrauch noch Nachladen"
    );
}

#[test]
fn der_zaehler_laeuft_takt_fuer_takt_gleich() {
    let json = data();
    let result = replay(&json);
    let erwartet = zahlen(&json, "doom");
    for (tick, (ist, soll)) in result.doom.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(ist, soll, "Zaehler verschieden im Takt {tick}");
    }
}
