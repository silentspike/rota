//! Die davonlaufende Reaktion mit Spielerschaden, Takt fuer Takt (Issue #256, AC3).
//!
//! WARUM ES DIESE DATEI GIBT: EINE GEGENPROBE, DIE NICHT ANSCHLUG
//! --------------------------------------------------------------
//! Beim Einbau von `player_damage` habe ich in `runaway_tick` die falsche
//! Herkunft gesetzt — `true` statt `false`, also die Reaktion als Leistung des
//! Spielers gebucht. Die Rust-Tests blieben gruen.
//!
//! Der Grund war strukturell: Bis zum 2026-08-05 hat **keine einzige
//! Vergleichsdatei einen Boss mit `runaway` gefahren**. `parity_runaway.rs`
//! prueft Zahlen und keine Pruefsummen; alle Pruefsummen-Dateien laufen gegen
//! Bosse ohne Reaktion. `runaway_tick` kam dort schlicht nie vor.
//!
//! Das ist die Frage aus der vierten Gestalt, eine Ebene hoeher gestellt:
//! Koennte diese Pruefung den Unterschied ueberhaupt zeigen? Sie konnte es
//! nicht, und deshalb steht hier eine, die es kann.
//!
//! **Beide Herkuenfte in EINEM Lauf.** Die gute Rotation macht Schaden, und der
//! Ring faellt zusaetzlich von selbst — gemessen 1040 Ring-Verlust, davon 620
//! vom Spieler. Ein Lauf mit nur einer Herkunft koennte die Trennung nicht
//! pruefen: Waeren beide Anteile gleich gross oder einer davon null, saehe eine
//! Umsetzung ohne Trennung genauso aus.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const RUNAWAY_JSON: &str = include_str!("../../tests/parity/recordings/runaway-progress.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(RUNAWAY_JSON).expect("runaway-progress.json ist kein JSON");
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
fn der_fall_kann_den_unterschied_ueberhaupt_zeigen() {
    // **Die Zusicherung, die diesem Pruefstand seinen Wert gibt.** Ohne sie
    // waere alles Weitere gruen, auch wenn der Lauf nur eine Herkunft haette —
    // und genau das war der Zustand, den diese Datei behebt.
    let json = data();
    let ring = zahlen(&json, "ringDamage");
    let vom_spieler = zahlen(&json, "playerDamage");

    let letzter_ring = *ring.last().expect("kein Ring-Verlust aufgezeichnet");
    let letzter_spieler = *vom_spieler
        .last()
        .expect("kein Spieler-Anteil aufgezeichnet");

    assert!(
        letzter_spieler > 0,
        "ohne Spielerschaden pruefte der Fall die Trennung nicht"
    );
    assert!(
        letzter_ring > letzter_spieler,
        "ohne Reaktions-Anteil ebenfalls nicht: Ring {letzter_ring}, Spieler {letzter_spieler}"
    );
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet = zahlen(&json, "checksums");

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: Spiel {}, Simulator {}",
        erwartet.len(),
        result.checksums.len()
    );

    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        let ring = zahlen(&json, "ringDamage");
        let vom_spieler = zahlen(&json, "playerDamage");
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             Ring dort: {:?}, davon vom Spieler: {:?}",
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
            ring.get(d.tick as usize),
            vom_spieler.get(d.tick as usize),
        );
    }
}

#[test]
fn der_schluss_stand_stimmt() {
    let json = data();
    let result = replay(&json);
    let erwartet = &json["expected"];

    assert_eq!(
        u64::from(result.damage),
        erwartet["damage"].as_u64().expect("damage fehlt"),
        "verschiedener Ring-Verlust"
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        erwartet["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum fehlt"),
        "verschiedene Folge-Pruefsumme"
    );
    assert_eq!(
        u64::from(result.final_checksum),
        erwartet["finalChecksum"]
            .as_u64()
            .expect("finalChecksum fehlt"),
        "verschiedene Schluss-Pruefsumme"
    );
}

#[test]
fn der_boss_traegt_die_reaktion_auch_im_simulator() {
    // Sonst pruefte alles oben einen Boss ohne Reaktion — dieselbe Falle wie
    // ein Gegenfall am falschen Datensatz.
    let boss = boss_by_id("test-runaway").expect("test-runaway fehlt");
    let run = boss.runaway.expect("test-runaway hat keine Reaktion");
    assert_eq!(run.per_step_ms, 1_000);
    assert_eq!(run.amount, 60);
}
