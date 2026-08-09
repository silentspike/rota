//! Schreiben beide Seiten dieselben Protokoll-Arten zur selben ZEIT? (Issue #676)
//!
//! # Was dieser Fall kann, was `parity_log_kinds_written` nicht kann
//!
//! Jener Fall (Issue #151) vergleicht, wie **oft** eine Art vorkommt. Das ist
//! der richtige erste Schritt und war lange der einzige. Er hat zwei Grenzen,
//! und beide sind hier geschlossen:
//!
//! **1. Er vergleicht die Anzahl, nicht den Zeitpunkt.** Zwei Seiten können
//! eine Art gleich oft schreiben und die Einträge an verschiedenen Takten
//! setzen. Eine Auswertung, die nach Gelegenheiten sucht, läse dann dieselbe
//! Summe aus verschobenen Ereignissen.
//!
//! **2. Er fährt einen Boss, der Art 8 nicht auslösen kann.**
//! `log-parity.json` läuft gegen `test-warden` mit `energy: null`. Die
//! Energieleiste löst dort nie aus, `bossEnergyTriggered` steht auf beiden
//! Seiten auf null, und der Vergleich ist grün — auch als das Spiel die Art
//! noch überhaupt nicht schrieb. **Eine Art, die kein Lauf hervorruft, ist
//! nicht geprüft, sondern unauslösbar.**
//!
//! Das ist dieselbe Lage wie bei ANCHOR in Issue #667: Der Zauber stand mit
//! `casts: 1` in den Aufzeichnungen, seine Wirkung prüfte trotzdem niemand,
//! weil `boss-1-2` die nötige Phasenschwelle nie erreicht. Dort entstand
//! `test-phase`, hier `log-takte.json` gegen `test-vessel`.

use std::collections::BTreeMap;

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/log-takte.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("log-takte.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

/// Spielt die aufgezeichnete Runde nach und sammelt die Zeitpunkte je Art.
fn nachspielen(json: &Value) -> (Fight, BTreeMap<&'static str, Vec<u32>>) {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| {
            let id = b["spellId"].as_str().expect("spellId");
            Binding {
                slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt")),
            }
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
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, false);

    let mut takte: BTreeMap<&'static str, Vec<u32>> = BTreeMap::new();
    for eintrag in fight.log().entries() {
        takte
            .entry(eintrag.kind.name())
            .or_default()
            .push(eintrag.time);
    }
    (fight, takte)
}

/// Die Zeitpunkte, die das Spiel aufgezeichnet hat.
fn vom_spiel(json: &Value) -> BTreeMap<String, Vec<u32>> {
    json["takte"]
        .as_object()
        .expect("takte fehlen")
        .iter()
        .map(|(name, liste)| {
            let werte = liste
                .as_array()
                .expect("Takte sind keine Liste")
                .iter()
                .map(|w| u32::try_from(w.as_u64().expect("Zahl")).expect("32 Bit"))
                .collect();
            (name.clone(), werte)
        })
        .collect()
}

#[test]
fn ac0_die_lage_loest_ereignisart_8_ueberhaupt_aus() {
    // **Diese Zusicherung steht vor allem anderen.** Löst die Energieleiste
    // nicht aus, vergleichen alle Fälle darunter zwei leere Listen und sind
    // grün, ohne etwas über Art 8 zu sagen — exakt der Zustand, den dieses
    // Issue behebt.
    let json = data();
    let erwartet = json["expected"]["bossEnergyTriggers"]
        .as_u64()
        .expect("bossEnergyTriggers fehlt");
    assert!(
        erwartet > 0,
        "die Aufzeichnung hat keinen einzigen Energie-Ausloeser — sie kann \
         ueber Art 8 nichts belegen"
    );

    let (_, simulator) = nachspielen(&json);
    let sim_takte = simulator.get("bossEnergyTriggered");
    assert!(
        sim_takte.is_some_and(|t| !t.is_empty()),
        "der Simulator schreibt bossEnergyTriggered in dieser Lage nicht"
    );
    let spiel = vom_spiel(&json);
    assert!(
        spiel
            .get("bossEnergyTriggered")
            .is_some_and(|t| !t.is_empty()),
        "das Spiel schreibt bossEnergyTriggered in dieser Lage nicht — \
         genau der Befund aus #676, vor #679"
    );
}

#[test]
fn ac1_beide_seiten_schreiben_zur_selben_zeit() {
    let json = data();
    let (_, simulator) = nachspielen(&json);
    let spiel = vom_spiel(&json);

    // Nur die Arten, die BEIDE Seiten führen sollen. `spend` und `kill`
    // gehören allein dem Simulator — sie hier zu fordern hiesse, das Spiel
    // muesse den Boss-Tod protokollieren.
    let nur_simulator = ["spend", "kill", "controlSpent"];

    let mut verglichen = 0;
    for (art, sim_zeiten) in &simulator {
        if nur_simulator.contains(art) {
            continue;
        }
        let leer: Vec<u32> = Vec::new();
        let spiel_zeiten = spiel.get(*art).unwrap_or(&leer);
        assert_eq!(
            sim_zeiten, spiel_zeiten,
            "{art}: die Zeitpunkte gehen auseinander.\n  Simulator {sim_zeiten:?}\n  Spiel     {spiel_zeiten:?}"
        );
        verglichen += 1;
    }

    // **Ohne diese Zeile prueft der Test nichts.** Waere die Sammlung leer,
    // liefe die Schleife gar nicht und der Fall bliebe gruen.
    assert!(
        verglichen >= 4,
        "es wurden nur {verglichen} Arten verglichen — zu wenige, um etwas zu belegen"
    );
}

#[test]
fn ac2_die_kennzahlen_stimmen_ebenfalls_ueberein() {
    // Die Gegenprobe zu den Zeitpunkten: nicht nur WANN geschrieben wird,
    // sondern dass dabei dasselbe geschieht.
    let json = data();
    let (fight, _) = nachspielen(&json);
    let erwartet = &json["expected"];

    for (feld, gemessen) in [
        ("damageDealt", u64::from(fight.damage())),
        ("damageTaken", u64::from(fight.damage_taken())),
    ] {
        assert_eq!(
            gemessen,
            erwartet[feld]
                .as_u64()
                .unwrap_or_else(|| panic!("{feld} fehlt")),
            "{feld} weicht ab"
        );
    }
}
