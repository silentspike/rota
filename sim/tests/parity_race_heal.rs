//! Wettrennen und Boss-Heilung, Takt für Takt (Issue #218, AC6).
//!
//! **Beide Bausteine in EINEM Lauf.** Das Fenster liegt bei 10 s, der
//! Boss-Zauber bei 30 s. Getrennt geprüft würde nicht zeigen, dass sie sich
//! nicht ins Gehege kommen — und genau dort läuft so etwas auseinander.
//!
//! Beide wirken auf den FORTSCHRITT statt auf den Spieler: Der eine setzt den
//! ausgeteilten Schaden zeitweise auf null, der andere dreht ihn zurück. Damit
//! sind sie ein besonders guter Prüfstein für die Parität — was hier
//! auseinanderläuft, läuft am Kern der Rechnung auseinander.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const RACE_JSON: &str = include_str!("../../tests/parity/recordings/race-heal.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(RACE_JSON).expect("race-heal.json ist kein JSON");
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
fn der_pruef_boss_traegt_beide_bausteine() {
    let boss = boss_by_id("test-racer").expect(
        "Der Boss aus data/bosses/test-racer.json wurde nicht gefunden. \
         Liest build.rs das Verzeichnis noch?",
    );
    let race = boss.race.expect("test-racer hat kein Fenster");
    assert_eq!(race.at_ms, 10_000);
    assert_eq!(race.window_ms, 6_000);
    assert_eq!(race.threshold, 300);
    assert_eq!(boss.boss_heal.expect("keine Heilung").amount, 600);
}

#[test]
fn ac6_die_pruefsumme_stimmt_takt_fuer_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet = zahlen(&json, "checksums");
    let offen = zahlen(&json, "raceOpen");

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    for (tick, (links, rechts)) in result.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(
            links,
            rechts,
            "erste Abweichung im Takt {tick} ({} ms). Das Fenster stand dort im Spiel {}",
            tick as u32 * TICK_MS,
            if offen.get(tick) == Some(&1) {
                "OFFEN"
            } else {
                "zu"
            }
        );
    }

    let expected = &json["expected"];
    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"].as_u64().expect("finalChecksum")
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        expected["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum")
    );
}

#[test]
fn ac1_der_verlorene_schaden_stimmt_ueberein() {
    // **Die Zahl, um die es geht.** "Der Boss nimmt keinen Schaden" waere mit
    // mal null erfuellt — dann fehlte sie in der Fehleranalyse. Beide Seiten
    // muessen denselben Betrag als verloren fuehren, sonst meldet die
    // Auswertung im Spiel etwas anderes als der Simulator rechnet.
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    assert_eq!(
        u64::from(result.damage_lost_to_race),
        expected["damageLostToRace"]
            .as_u64()
            .expect("damageLostToRace"),
        "verschieden viel verlorener Schaden"
    );
    assert!(
        result.damage_lost_to_race > 0,
        "im Lauf ist gar kein Schaden ins Fenster gefallen — dann prueft dieser \
         Fall nichts"
    );
    assert_eq!(
        u64::from(result.damage),
        expected["damage"].as_u64().expect("damage"),
        "verschiedener ausgeteilter Schaden"
    );
}

#[test]
fn ac3_die_boss_heilung_stimmt_ueberein() {
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    assert_eq!(
        u64::from(result.boss_healed),
        expected["bossHealed"].as_u64().expect("bossHealed"),
        "verschieden viel zurueckgedrehter Fortschritt"
    );
    assert!(
        result.boss_healed > 0,
        "im Lauf ist kein Boss-Zauber durchgekommen — dann prueft dieser Fall nichts"
    );
}

/// Der Ausgang des Wettrennens, beide Seiten (Issue #639).
///
/// **Bis #639 fuehrte `fight.rs` `race_won` und las es nie.** Vier Zeilen —
/// Feld, Anlage, Ruecksetzen, Setzen — und kein einziger Leser; ein `grep`
/// ohne diese vier kam leer zurueck. Die Spielseite liefert `raceWon` seit
/// #638 ins `ReplayResult`, verglichen wurde also eine Haelfte gegen nichts.
///
/// # Warum hier und nicht in der Pruefsumme
///
/// Gemessen, nicht angenommen: `examples/parity.rs` vergleicht eine feste
/// Liste von Feldern, und `stabilised` zeigt, dass ein Ergebnisfeld allein
/// von niemandem gelesen wird — es haengt an genau einem handgeschriebenen
/// Fall (`parity_runaway.rs:170`). Ein Anschluss ohne Leser waere derselbe
/// tote Wert wie vorher, nur eine Ebene weiter.
///
/// Die Pruefsumme faerbt Takt fuer Takt und waere feiner. Bei einem Schalter,
/// der einmal von `false` auf `true` kippt und nie zurueck, traegt der
/// Endstand dieselbe Aussage — und er sagt sie an der Stelle, an der die
/// Aufzeichnung sie fuehrt.
#[test]
fn ac_der_ausgang_des_wettrennens_stimmt_ueberein() {
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    // **Die Zusicherung vor der Messung.** Gewinnt in diesem Lauf niemand,
    // vergliche der Fall darunter `false` mit `false` und bliebe gruen, auch
    // wenn die Seiten auseinanderlaufen.
    assert!(
        expected["raceWon"].as_bool().expect("raceWon"),
        "die Aufzeichnung fuehrt kein gewonnenes Rennen — dann prueft dieser Fall nichts"
    );
    assert_eq!(
        result.race_won,
        expected["raceWon"].as_bool().expect("raceWon"),
        "die Seiten sind sich ueber den Ausgang des Wettrennens nicht einig"
    );
}

#[test]
fn ac5_ein_boss_ohne_die_felder_bleibt_unberuehrt() {
    // Der Gegenfall zum ganzen Issue: `first-crucible` traegt keinen der
    // beiden Bausteine, und beide Groessen muessen null bleiben.
    let boss = boss_by_id("first-crucible").expect("first-crucible");
    assert!(boss.race.is_none());
    assert!(boss.boss_heal.is_none());
}
