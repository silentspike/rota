//! Overdrive, Takt für Takt gegen das Spiel (#1177, Abschnitt 2).
//!
//! Die Runde stammt aus `tests/parity/overdrive-payload.ts`: die Rotation des
//! Spiels, mitgeschrieben, mit einem Druck auf G und zwei verworfenen. Was sie
//! an welchem Takt berührt, steht dort in der Tabelle; hier wird sie nur
//! abgespielt und Takt für Takt verglichen.
//!
//! **Die Regel-Zahlen kommen aus der Datei** und werden mit denen des
//! Simulators verglichen, nicht abgeschrieben: Kennung des Drucks, Dauer und
//! Stärke. Läge eine Seite daneben, fiele es hier mit Namen auf — und nicht
//! erst als Prüfsumme an einem Takt, der auf keine Ursache zeigt.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::overdrive::{Overdrive, OVERDRIVE_PRESS};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const OVERDRIVE_JSON: &str = include_str!("../../tests/parity/recordings/overdrive.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(OVERDRIVE_JSON).expect("overdrive.json ist kein JSON");
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

fn replay(json: &Value) -> (Fight, rota_sim::ReplayResult) {
    let recording = recording(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, true);
    (fight, result)
}

fn zahlen(json: &Value, feld: &str) -> Vec<i64> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| v.as_i64().expect("Zahl"))
        .collect()
}

#[test]
fn beide_seiten_lesen_dieselben_regel_zahlen() {
    let json = data();
    let regeln = &json["overdrive"];
    assert_eq!(regeln["press"].as_u64(), Some(u64::from(OVERDRIVE_PRESS)));
    assert_eq!(
        regeln["durationMs"].as_u64(),
        Some(u64::from(Overdrive::duration_ms()))
    );
    assert_eq!(
        regeln["catalysisPermille"].as_u64(),
        Some(u64::from(Overdrive::catalysis_bonus()))
    );
}

#[test]
fn der_fall_kann_den_unterschied_ueberhaupt_zeigen() {
    // **Die Zusicherung, die dieser Datei ihren Wert gibt.** Ohne sie wäre alles
    // Weitere auch an einer Runde grün, in der nie gezündet wurde.
    let json = data();
    let rest = zahlen(&json, "overdriveRemaining");
    let wirksam = rest.iter().filter(|&&v| v > 0).count();
    assert_eq!(
        wirksam,
        (Overdrive::duration_ms() / TICK_MS) as usize,
        "Overdrive wirkte nicht genau seine Dauer lang"
    );
    let drucke_auf_g = json["presses"]
        .as_array()
        .expect("presses")
        .iter()
        .filter(|p| p["slot"].as_u64() == Some(u64::from(OVERDRIVE_PRESS)))
        .count();
    assert_eq!(drucke_auf_g, 3, "gemessen: ein Zünden, zwei verworfene Drücke");

    let erwartet = &json["expected"];
    assert_eq!(erwartet["overdriveIgnitedAtMs"].as_i64(), Some(4000));
    assert_eq!(
        erwartet["castsCancelled"].as_u64(),
        Some(1),
        "das Zünden bricht die Wirkzeit von test-cast ab"
    );
    assert!(
        erwartet["rejected"].as_u64().unwrap_or(0) >= 2,
        "die zwei späteren Drücke auf G sind Fehleingaben"
    );
    assert_eq!(erwartet["quenchCasts"].as_u64(), Some(1), "der Grenz-Takt fehlt");
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let (_, result) = replay(&json);
    let erwartet: Vec<u32> = zahlen(&json, "checksums")
        .into_iter()
        .map(|v| u32::try_from(v).expect("32 Bit"))
        .collect();
    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: Spiel {}, Simulator {}",
        erwartet.len(),
        result.checksums.len()
    );
    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        let rest = zahlen(&json, "overdriveRemaining");
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             Overdrive-Rest dort im Spiel: {:?} ms",
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
            rest.get(d.tick as usize),
        );
    }
}

#[test]
fn der_schluss_stand_stimmt() {
    let json = data();
    let (fight, result) = replay(&json);
    let erwartet = &json["expected"];
    assert_eq!(Some(i64::from(result.damage)), erwartet["damage"].as_i64());
    assert_eq!(Some(u64::from(result.accepted)), erwartet["accepted"].as_u64());
    assert_eq!(Some(u64::from(result.rejected)), erwartet["rejected"].as_u64());
    assert_eq!(
        Some(u64::from(result.casts_cancelled)),
        erwartet["castsCancelled"].as_u64()
    );
    assert_eq!(
        Some(u64::from(result.final_checksum)),
        erwartet["finalChecksum"].as_u64()
    );
    assert_eq!(
        Some(u64::from(result.sequence_checksum)),
        erwartet["sequenceChecksum"].as_u64()
    );
    assert_eq!(
        fight.overdrive().ignited_at().map(i64::from),
        erwartet["overdriveIgnitedAtMs"].as_i64(),
        "Zündzeitpunkt"
    );
    let crust = index_of("crust").expect("crust");
    let smoulder = index_of("smoulder").expect("smoulder");
    assert_eq!(
        Some(u64::from(result.casts[crust])),
        erwartet["crustCasts"].as_u64()
    );
    assert_eq!(
        Some(u64::from(fight.dot_tick_count(smoulder))),
        erwartet["smoulderTicks"].as_u64()
    );
}
