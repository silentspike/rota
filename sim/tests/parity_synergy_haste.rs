//! Tempo UND Synergie in einem Lauf (Issue #456, Schritt 2).
//!
//! # Die Luecke, die diese Datei schliesst
//!
//! Teil 1 hat die Klammerung im Spiel berichtigt: `gcds * hasted(GCD)` statt
//! `hasted(gcds * GCD)`. Die Rust-Seite wurde in Schritt 1 nachgezogen. Und
//! **nichts davon war nachweisbar**, weil keine Aufzeichnung den Fall traf:
//!
//!   synergy-window.json   canker und fester auf der Leiste — aber kein `worn`
//!   worn-values.json      catalysis 170 — aber die Leiste traegt kindle,
//!                         smoulder, pyre, quench
//!
//! Jede Haelfte gab es, nie zusammen. Beide Seiten konnten also verschieden
//! rechnen, waehrend die Paritaetspruefung gruen blieb: Sie vergleicht, was
//! aufgezeichnet ist.
//!
//! # Warum der Unterschied so leicht zu verfehlen ist
//!
//! Er betraegt **zehn Millisekunden**. Bei 300 Promille endet das Fenster neu
//! bei 2320 und alt bei 2310. Ein Druck irgendwo im Fenster liegt bei beiden
//! Rechnungen drin und zeigt nichts.
//!
//! Ueber den ganzen Lauf gibt es genau **einen** Takt, der nur im neuen Fenster
//! liegt: `2 * GCD`. Die Aufzeichnung setzt ihren zweiten Druck gezielt dorthin
//! (`recordSynergyHasted` in payloads.ts). Ohne diese Absicht traefe sie den
//! Fall so gut wie nie.
//!
//! # Und die Wahl des Tempos ist keine Geschmacksfrage
//!
//! Bei **499 von 1001** Promille-Werten ergeben beide Klammerungen dasselbe.
//! Wer einen Wert ohne Nachrechnen waehlt, bekommt mit rund fuenfzig Prozent
//! Wahrscheinlichkeit eine Aufzeichnung, die gruen ist und nichts belegt.
//! Deshalb steht die Zusicherung unten **vor** jeder Messung: Dieser Lauf kann
//! nicht gruen und wertlos zugleich sein.

use rota_sim::data::index_of;
use rota_sim::gear::WornValues;
use rota_sim::haste::hasted_ms;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::time::GCD_MS;
use rota_sim::{Fight, FightConfig};
use serde_json::Value;

/// Der unveraenderte Inhalt des Laufs mit Tempo und Synergie.
const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/synergy-haste.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("synergy-haste.json ist kein JSON");
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
/// sich selbst uebereinstimmt.
fn worn(json: &Value) -> WornValues {
    let node = &json["worn"];
    WornValues {
        catalysis: u32_at(node, "catalysis"),
        purity: u32_at(node, "purity"),
        yield_bonus: u32_at(node, "yieldBonus"),
        essence: u32_at(node, "essence"),
        vessel: u32_at(node, "vessel"),
        ward: u32_at(node, "ward"),
        gear_effects: u32_at(node, "gearEffects"),
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

/// **Die Zusicherung vor jeder Messung.**
///
/// Sie steht zuerst, weil ohne sie alles Folgende gruen und wertlos sein
/// koennte: Bei der Haelfte aller Tempo-Werte ergeben beide Klammerungen
/// dasselbe, und dann pruefte dieser Lauf die Klammerung ueberhaupt nicht.
#[test]
fn bei_diesem_tempo_unterscheiden_sich_die_beiden_klammerungen_ueberhaupt() {
    let json = data();
    let tempo = worn(&json).catalysis;

    let neu = 2 * hasted_ms(GCD_MS, tempo);
    let alt = hasted_ms(2 * GCD_MS, tempo);

    assert_ne!(
        neu, alt,
        "{tempo} Promille ist ein RUNDER Wert — beide Klammerungen ergeben {neu}. \
         Dieser Lauf kann die Klammerung nicht pruefen. 499 von 1001 Werten sind \
         so; ein anderer gehoert gewaehlt und nachgerechnet, nicht geraten."
    );

    // Und die Datei sagt dasselbe wie diese Rechnung. Stuenden dort andere
    // Zahlen, waere die Aufzeichnung mit einem anderen Tempo entstanden als dem,
    // das in ihrem `worn` steht.
    assert_eq!(
        u64::from(neu),
        json["synergyWindowMs"]
            .as_u64()
            .expect("synergyWindowMs fehlt"),
        "die Rust-Rechnung ergibt eine andere Fensterlaenge als die Datei"
    );
    assert_eq!(
        u64::from(alt),
        json["synergyWindowMsOldBracketing"]
            .as_u64()
            .expect("synergyWindowMsOldBracketing fehlt"),
        "die alte Klammerung ergibt in Rust etwas anderes als im Spiel"
    );
}

#[test]
fn der_lauf_mit_tempo_und_synergie_ergibt_denselben_schaden() {
    let json = data();
    let (recording, result) = replay_it(&json, false);
    let expected = &json["expected"];

    assert_eq!(
        result.damage,
        u32_at(expected, "damage"),
        "Simulator und Spiel rechnen mit Tempo und Synergie verschiedenen Schaden"
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

/// **Der eigentliche Nachweis.**
///
/// `synergy_used` zaehlt, wie oft FESTER im offenen Fenster landete. Genau
/// diese Zahl bricht, wenn eine Seite anders klammert — der Druck bei `2 * GCD`
/// faellt dann heraus.
#[test]
fn die_synergie_wirkt_gleich_oft_wie_im_spiel() {
    let json = data();
    let (_, result) = replay_it(&json, false);
    let expected = &json["expected"];

    let erwartet = u32_at(expected, "synergyUsed");
    assert!(
        erwartet > 0,
        "in diesem Lauf wirkt die Synergie kein einziges Mal — dann prueft er sie nicht"
    );
    assert_eq!(
        result.synergy_used, erwartet,
        "Simulator und Spiel lassen die Synergie verschieden oft wirken. Das ist \
         der Fall, gegen den diese Aufzeichnung gebaut ist: Bei abweichender \
         Klammerung faellt der Druck am Fensterende heraus."
    );
    assert_eq!(
        result.synergy_available,
        u32_at(expected, "synergyAvailable"),
        "das Fenster stand verschieden oft offen"
    );
}

/// Takt fuer Takt, nicht nur am Ende.
///
/// Ein Endergebnis kann zufaellig uebereinstimmen, waehrend der Weg dahin
/// auseinanderlief — und bei zehn Millisekunden Unterschied ist genau das die
/// wahrscheinlichere Art zu scheitern.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let (_, result) = replay_it(&json, true);

    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte aufgezeichnet"
    );
    for (tick, (links, rechts)) in result.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(
            links, rechts,
            "Takt {tick}: Simulator und Spiel laufen auseinander"
        );
    }

    assert_eq!(
        result.final_checksum,
        u32_at(&json["expected"], "finalChecksum"),
        "die Schluss-Pruefsumme weicht ab"
    );
}
