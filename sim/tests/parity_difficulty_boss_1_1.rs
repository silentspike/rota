//! Der erste Boss der ersten Instanz auf Heroisch und Mythisch, im
//! Paritaets-Weg (Issue #1177).
//!
//! # Was die Stufen hier dazubringen
//!
//! Normal: zwei unterbrechbare Kindling Roars je Zyklus, die Eskalation
//! verkuerzt sie. Heroisch legt eine Schwaechung darauf, die ebenfalls QUENCH
//! verlangt — dieselbe Taste fuer zwei Aufgaben. Mythisch bringt den Zaehler
//! dazu, den nur PURGE senkt. Leben und Zeitlimit folgen `data/difficulty.json`.
//!
//! # Gegen die abgeleitete Fassung, nicht gegen die Datei
//!
//! Der Boss kommt aus [`boss_at`] — dem Datensatz, den `build.rs` aus der
//! Datei und ihrer Stufe ableitet. Die Konfiguration traegt die Stufe in ihrem
//! Boss (`FightConfig::boss`, `Boss::difficulty`). Liefe dieser Fall gegen
//! Normal, endete er bei anderem Leben — die Pruefsumme meldete es im Takt des
//! Sieges.
//!
//! # Die Zusicherung misst die Rust-Seite selbst
//!
//! Die Datei nennt `afflictionsCleared` und `doomPeak` aus dem Lauf des Spiels.
//! Hier werden dieselben Groessen an den Spuren des Simulators abgelesen
//! (`afflictions_cleared`, `doom`). Stuenden beide auf null, haette die
//! Aufzeichnung die Zutat der Stufe nie beruehrt — und saehe trotzdem aus wie
//! eine gelungene.

use serde_json::Value;

use rota_sim::data::{boss_at, index_of, DIFFICULTY_HEROIC, DIFFICULTY_MYTHIC};
use rota_sim::replay::{first_divergence, Binding, Press, Recording, ReplayResult};
use rota_sim::{Fight, FightConfig, TICK_MS};

const HEROISCH: &str =
    include_str!("../../tests/parity/recordings/difficulty-boss-1-1-heroic.json");
const MYTHISCH: &str =
    include_str!("../../tests/parity/recordings/difficulty-boss-1-1-mythic.json");

fn data(text: &str) -> Value {
    let value: Value = serde_json::from_str(text).expect("keine JSON-Aufzeichnung");
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

/// Spielt die Aufzeichnung gegen die Stufe ab, die sie selbst nennt.
fn abspielen(json: &Value) -> ReplayResult {
    let stufe = u32_at(json, "difficulty");
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_at(boss_id, stufe)
        .unwrap_or_else(|| panic!("{boss_id} Stufe {stufe} fehlt im Simulator"));
    let aufzeichnung = recording(json);
    let mut fight = Fight::new(FightConfig {
        boss,
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&aufzeichnung, true)
}

fn pruefe_gleichlauf(json: &Value) {
    let ergebnis = abspielen(json);
    let erwartet = &json["expected"];
    let name = json["name"].as_str().unwrap_or("?");
    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "{name}: Schaden"
    );
    assert_eq!(
        ergebnis.accepted,
        u32_at(erwartet, "accepted"),
        "{name}: angenommen"
    );
    assert_eq!(
        ergebnis.rejected,
        u32_at(erwartet, "rejected"),
        "{name}: abgelehnt"
    );
    assert_eq!(
        ergebnis.killed_at_ms.map_or(-1, i64::from),
        erwartet["killedAtMs"].as_i64().expect("killedAtMs fehlt"),
        "{name}: Sieg"
    );
    assert_eq!(
        ergebnis.final_checksum,
        u32_at(erwartet, "finalChecksum"),
        "{name}: Schluss-Pruefsumme"
    );
    assert_eq!(
        ergebnis.sequence_checksum,
        u32_at(erwartet, "sequenceChecksum"),
        "{name}: Folgen-Pruefsumme"
    );
    let takte: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("keine Zahl")).expect("zu gross"))
        .collect();
    assert_eq!(
        first_divergence(&takte, &ergebnis.checksums, TICK_MS),
        None,
        "{name}: Simulator und Spiel laufen auseinander"
    );
    // **Die Zusicherung, an der eigenen Spur gemessen** — dieselbe Zahl wie
    // im Spiel, nicht nur „irgendetwas geschah".
    assert_eq!(
        ergebnis.afflictions_cleared,
        u32_at(erwartet, "afflictionsCleared"),
        "{name}: geklaerte Schwaechungen"
    );
    assert_eq!(
        ergebnis.doom.iter().copied().max().unwrap_or(0),
        u32_at(erwartet, "doomPeak"),
        "{name}: hoechster Zaehlerstand"
    );
}

/// **Vor jeder Messung**: Die Dateien fahren den Boss, den sie nennen, auf der
/// Stufe, die sie nennen — mit dessen Leben und Zeitlimit.
#[test]
fn die_aufzeichnungen_fahren_die_abgeleiteten_stufen() {
    for (text, stufe) in [(HEROISCH, DIFFICULTY_HEROIC), (MYTHISCH, DIFFICULTY_MYTHIC)] {
        let json = data(text);
        assert_eq!(json["bossId"].as_str(), Some("boss-1-1"));
        assert_eq!(json["difficulty"].as_u64(), Some(u64::from(stufe)));
        let boss = boss_at("boss-1-1", stufe).expect("Stufe fehlt");
        let normal = boss_at("boss-1-1", 0).expect("Normal fehlt");
        assert_eq!(boss.difficulty, stufe);
        assert_eq!(u32_at(&json["expected"], "hp"), boss.hp, "Leben der Stufe");
        assert!(boss.hp > normal.hp, "die Stufe traegt nicht mehr Leben");
        assert_eq!(
            u32_at(&json["expected"], "overpressureMs"),
            boss.overpressure_ms,
            "Zeitlimit der Stufe"
        );
        assert!(boss.overpressure_ms < normal.overpressure_ms);
        // Die Dauer der Aufzeichnung IST das Zeitlimit der Stufe — so rechnet
        // `rota-sim verify` (Zeitlimit aus dem Boss) mit derselben Grenze wie
        // dieser Fall (Dauer aus der Datei).
        assert_eq!(u32_at(&json, "ticks") * TICK_MS, boss.overpressure_ms);
    }
}

#[test]
fn heroisch_rechnet_auf_beiden_seiten_gleich() {
    pruefe_gleichlauf(&data(HEROISCH));
}

#[test]
fn mythisch_rechnet_auf_beiden_seiten_gleich() {
    pruefe_gleichlauf(&data(MYTHISCH));
}

/// **Die Zutat der Stufe kam im Lauf vor** — auf Heroisch die Schwaechung, auf
/// Mythisch zusaetzlich der Zaehler. Normal kennt beides nicht.
#[test]
fn die_zutaten_der_stufen_greifen_im_lauf() {
    let heroisch = abspielen(&data(HEROISCH));
    assert!(
        heroisch.afflicted.contains(&1),
        "auf Heroisch lag die Schwaechung nie an"
    );
    assert!(
        heroisch.afflictions_cleared > 0,
        "auf Heroisch wurde die Schwaechung nie geklaert — QUENCH trug nur eine Aufgabe"
    );
    assert_eq!(
        heroisch.doom.iter().copied().max(),
        Some(0),
        "Heroisch hat keinen Zaehler; einer stieg trotzdem"
    );

    let mythisch = abspielen(&data(MYTHISCH));
    assert!(
        mythisch.doom.iter().any(|&d| d > 0),
        "auf Mythisch stieg der Zaehler nie"
    );
    assert!(
        mythisch.afflicted.contains(&1),
        "Mythisch hat die heroische Schwaechung verloren — die Stufen sind nicht kumulativ"
    );
}
