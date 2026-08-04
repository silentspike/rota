//! Gleichlauf ueber einen Lauf **mit getragener Ausruestung** (Issue #177, AC8).
//!
//! # Warum der bestehende Lauf nicht reicht
//!
//! `parity_recording.rs` spielt dieselbe Aufzeichnung ohne Ausruestung ab. Er
//! belegt damit AC7 — dass sich nichts bewegt — aber ueber die neuen Rechenwege
//! sagt er nichts: Keine Zeit wird umgerechnet, kein Wurf faellt.
//!
//! Eine Seite koennte den Tick-Abstand abrunden statt aufzurunden, oder den
//! Zufallsstrom einmal mehr drehen als die andere, und jener Lauf bliebe
//! **bitgleich**. Auffallen wuerde es erst im Spiel, bei einem Spieler mit
//! Ausruestung, und dann ohne jeden Hinweis darauf, welche Regel es war.
//!
//! Hier ist beides im Spiel:
//!
//! ```text
//! CATALYSIS 170 Promille   krumme Zwischenwerte, erzwingt die Rasterung
//! PURITY    375 Promille   zwischen beiden Kurzschluessen — es wird gewuerfelt
//! YIELD     250 Promille   Kritfaktor 1,75 statt 1,5
//! ```
//!
//! **Die Pruefsumme je Takt ist das Schaerfere am Vergleich.** Ein Endergebnis
//! kann zufaellig uebereinstimmen, waehrend der Weg dahin auseinanderlief.
//!
//! # Gemessen, nicht behauptet
//!
//! Am 2026-08-04 gegengeprueft, indem die Abklingzeit das Tempo wieder ignoriert
//! (`snap_up(now + spell.cooldown_ms)` statt `... + self.hasted(...)`):
//!
//! ```text
//! der_lauf_mit_ausruestung_ergibt_denselben_schaden        ok
//! beide_seiten_ziehen_gleich_oft_aus_dem_zufallsstrom      ok
//! die_pruefsumme_stimmt_in_jedem_einzelnen_takt            FAILED
//!   erste Abweichung in Takt 0 (0 ms)
//! ```
//!
//! **Nur die Pruefsumme faengt es.** Schaden, Sieg-Zeitpunkt und Ziehungen
//! bleiben unveraendert — in dieser Aufzeichnung ist die Wartezeit der Engpass,
//! nicht die Abklingzeit, und ein verschluckter Tempo-Wert dort bewegt keine
//! einzige Ergebniszahl. Ohne den taktweisen Vergleich waere der Fehler
//! unsichtbar geblieben, bis jemand mit anderem Zauberkasten spielt.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::gear::{WornValues, CRIT_BASE_PERMILLE};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

mod common;
use common::float;

/// Der unveraenderte Inhalt des Laufs mit Ausruestung.
const WORN_JSON: &str = include_str!("../../tests/parity/recordings/worn-values.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(WORN_JSON).expect("worn-values.json ist kein JSON");
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

/// Die getragenen Werte **aus der Datei**, nicht aus dem Simulator.
///
/// Waeren sie hier abgetippt, pruefte der Vergleich nur, ob der Simulator mit
/// sich selbst uebereinstimmt — und eine Aenderung auf der Spielseite ginge
/// unbemerkt durch. Dieselbe Ueberlegung wie bei der Tastenbelegung in
/// `parity_recording.rs`.
fn worn(json: &Value) -> WornValues {
    let node = &json["worn"];
    WornValues {
        catalysis: u32_at(node, "catalysis"),
        purity: u32_at(node, "purity"),
        yield_bonus: u32_at(node, "yieldBonus"),
    }
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

fn replay_it(json: &Value, with_checksums: bool) -> (Recording, rota_sim::ReplayResult) {
    let recording = recording(json);
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        worn: worn(json),
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, with_checksums);
    (recording, result)
}

#[test]
fn die_beiden_abgeleiteten_werte_stimmen_bitgenau_ueberein() {
    // Kritchance und Kritfaktor sind Fliesskommazahlen und gehen in den Schaden
    // ein. Sie stehen deshalb als Bitmuster in der Datei — die Regel dieses
    // Verzeichnisses seit #122, weil `serde_json` manche `f64` um ein einziges
    // Bit falsch liest.
    let json = data();
    let getragen = worn(&json);

    assert_eq!(
        CRIT_BASE_PERMILLE,
        u32_at(&json, "critBasePermille"),
        "Spiel und Simulator gehen von verschiedenen Kritschaden-Basen aus"
    );
    // **Ueber das Bitmuster, nicht ueber die Dezimalschreibweise** (#122):
    // `serde_json` liest manche `f64` um ein einziges Bit falsch, und der Krit
    // geht in den Schaden ein.
    assert_eq!(getragen.crit_chance(), float(&json["critChanceBits"]));
    assert_eq!(getragen.crit_factor(), float(&json["critFactorBits"]));

    // Und die Werte sind nicht zufaellig die neutralen: Ein Lauf mit
    // Kritchance 0 pruefte die halbe Datei nicht.
    assert!(getragen.crit_chance() > 0.0 && getragen.crit_chance() < 1.0);
    assert!(getragen.crit_factor() > 1.5);
}

#[test]
fn der_lauf_mit_ausruestung_ergibt_denselben_schaden() {
    let json = data();
    let (recording, result) = replay_it(&json, false);
    let expected = &json["expected"];

    assert_eq!(
        result.damage,
        u32_at(expected, "damage"),
        "der Simulator rechnet mit Ausruestung einen anderen Schaden als das Spiel"
    );
    assert_eq!(
        result.killed_at_ms.map(i64::from).unwrap_or(-1),
        expected["killedAtMs"].as_i64().expect("killedAtMs fehlt"),
        "der Boss faellt zu einer anderen Zeit"
    );
    assert_eq!(
        result.accepted,
        u32_at(expected, "accepted"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert!(
        !recording.presses.is_empty(),
        "eine Aufzeichnung ohne Tastendruecke prueft nichts"
    );
}

#[test]
fn beide_seiten_ziehen_gleich_oft_aus_dem_zufallsstrom() {
    // **Die Zahl, an der ein Krit-Fehler zuerst auffaellt.** Wer einmal mehr
    // oder weniger wuerfelt als die andere Seite, verschiebt jeden folgenden
    // Wurf — der Schaden liefe dann irgendwann auseinander, aber die Ursache
    // laege hundert Takte frueher.
    let json = data();
    let (_, result) = replay_it(&json, false);

    assert_eq!(
        result.random_draws,
        u32_at(&json["expected"], "randomDraws"),
        "verschieden viele Ziehungen — der Krit-Wurf faellt nicht an denselben Stellen"
    );
    // Und es wird ueberhaupt gezogen: Ein Lauf mit null Ziehungen pruefte am
    // Krit vorbei und saehe trotzdem gruen aus.
    assert!(
        result.random_draws > 0,
        "kein einziger Wurf — dieser Lauf prueft den Krit nicht"
    );
}

#[test]
fn die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    // **Der harte Pruefstein.** Ein Endergebnis sagt nur, DASS etwas nicht
    // stimmt; die Folge sagt, in welchem Takt — und damit fast immer auch,
    // welche Regel es war.
    let json = data();
    let (_, result) = replay_it(&json, true);
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl erwartet")).expect("passt nicht in 32 Bit"))
        .collect();

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    assert!(erwartet.len() > 1, "eine einzige Pruefsumme prueft nichts");

    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        panic!(
            "erste Abweichung in Takt {} ({} ms): Spiel {}, Simulator {} — {} Takte stimmten vorher",
            d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
        );
    }

    assert_eq!(
        result.final_checksum,
        u32_at(&json["expected"], "finalChecksum"),
        "Schluss-Pruefsumme"
    );
    assert_eq!(
        result.sequence_checksum,
        u32_at(&json["expected"], "sequenceChecksum"),
        "Folge-Pruefsumme"
    );
}

#[test]
fn dieser_lauf_unterscheidet_sich_wirklich_von_dem_ohne_ausruestung() {
    // **Der Gegenfall zur ganzen Datei.** Waeren die getragenen Werte
    // wirkungslos — etwa weil `worn` gar nicht bis in den Kampf durchgereicht
    // wird — kaeme derselbe Lauf heraus wie ohne Ausruestung, und alle
    // Pruefungen oben blieben gruen. Sie vergleichen nur Spiel und Simulator
    // miteinander, nicht gegen den Zustand davor.
    let json = data();
    let recording = recording(&json);

    let mit = Fight::new(FightConfig {
        seed: recording.seed,
        worn: worn(&json),
        ..FightConfig::default()
    })
    .replay(&recording, false);

    let ohne = Fight::new(FightConfig {
        seed: recording.seed,
        worn: WornValues::none(),
        ..FightConfig::default()
    })
    .replay(&recording, false);

    assert_ne!(
        mit.damage, ohne.damage,
        "die getragenen Werte bewegen den Schaden nicht"
    );
    assert_eq!(
        ohne.random_draws, 0,
        "ohne Ausruestung darf kein Wurf fallen"
    );
    assert!(mit.random_draws > 0, "mit PURITY muss gewuerfelt werden");
}
