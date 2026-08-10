//! **Die fehlgeschlagene Kontroll-Ausgabe** (gemessen bei Issue #480)
//!
//! `control_failed` war die dritte von drei Größen, die `ReplayResult` ausgibt
//! und die kein Paritätsfall gelesen hat. Bei den beiden anderen genügte ein
//! Vergleich — die Werte standen längst in Aufzeichnungen.
//!
//! Hier nicht: Über alle 50 nachspielbaren Runden gemessen schlug in **keiner
//! einzigen** eine Kontrolle fehl. Die Größe war nicht bloss unverglichen,
//! sondern gar nicht erreichbar. Deshalb eine eigene Aufzeichnung.
//!
//! LATCH kostet 2 Punkte und wird im Takt 0 geworfen, wenn der Vorrat auf 0
//! steht. Danach die Gegenrichtung: BIND baut auf, der zweite LATCH gelingt.
//! Eine Aufzeichnung mit nur einem Fehlschlag liesse offen, ob überhaupt je
//! etwas gelingt.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/kontrolle-fehlschlag.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("kontrolle-fehlschlag.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn aufbau(json: &Value) -> (Recording, &'static rota_sim::data::Boss, u32) {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| Binding {
            slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
            spell: index_of(b["spellId"].as_str().expect("spellId")).expect("Zauber unbekannt"),
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();
    let recording = Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: 0,
        ticks,
        bindings,
        presses,
    };
    (recording, boss, ticks)
}

#[test]
fn ac0_der_lauf_enthaelt_beide_richtungen() {
    // **Die Zusicherung vor allem anderen.** Ein Lauf ohne Fehlschlag
    // vergliche zwei Nullen; einer ohne gelungene Ausgabe belegte nur, dass
    // Ausgaben scheitern.
    let json = data();
    let fehl = json["expected"]["controlFailed"]
        .as_u64()
        .expect("controlFailed fehlt");
    assert!(
        fehl > 0,
        "in dieser Runde schlägt keine Kontrolle fehl — dann vergleichen die \
         Fälle darunter zwei Nullen und belegen nichts"
    );

    let ausgegeben = json["expected"]["controlSpent"]
        .as_u64()
        .expect("controlSpent fehlt");
    assert!(
        ausgegeben > 0,
        "in dieser Runde gelingt keine einzige Ausgabe — dann zeigt der Lauf \
         nur die eine Richtung"
    );
}

#[test]
fn ac1_beide_seiten_zaehlen_dieselben_fehlschlaege() {
    let json = data();
    let (recording, boss, ticks) = aufbau(&json);
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);

    assert_eq!(
        u64::from(ergebnis.control_failed),
        json["expected"]["controlFailed"]
            .as_u64()
            .expect("controlFailed"),
        "Simulator und Spiel zählen verschieden viele fehlgeschlagene Ausgaben"
    );
    assert_eq!(
        u64::from(ergebnis.control_spent),
        json["expected"]["controlSpent"]
            .as_u64()
            .expect("controlSpent"),
        "Simulator und Spiel geben verschieden viele Punkte aus"
    );
}

#[test]
fn ac2_die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let (recording, boss, ticks) = aufbau(&json);
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|c| u32::try_from(c.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect();
    assert!(!erwartet.is_empty(), "keine Prüfsummen in der Aufzeichnung");

    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, true);

    assert_eq!(
        ergebnis.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    for (i, (a, b)) in ergebnis.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(a, b, "Prüfsumme weicht in Takt {i} ab");
    }
}
