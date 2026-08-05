//! Eine aufgezeichnete Runde, in der die Synergie **wirklich gewirkt wird**
//! (Issue #318, AC7).
//!
//! # Warum es diese Datei braucht
//!
//! Die dreizehn bestehenden Aufzeichnungen belegen die Synergie nicht: CANKER
//! und FESTER werden in keiner von ihnen gedrueckt. Sie sind gruen, **weil** die
//! Mechanik dort nie anlaeuft — dieselbe Lage, in der eine fertige Mechanik ohne
//! Nutzer als fertig gilt (#201, #219, #223).
//!
//! Gemessen, bevor diese Datei entstand: In allen Aufzeichnungen zusammen
//! wurden acht der fuenfzehn Zauber je gedrueckt, canker und fester keinmal.
//!
//! # Was sie zeigen KANN, und das ist die Bedingung
//!
//! Ohne die Synergie-Rechnung liefert dieselbe Tastenfolge andere Zahlen:
//!
//! | | Schaden | Pruefsumme |
//! | --- | --- | --- |
//! | ohne die Rechnung | 924 | 307633569 |
//! | mit der Rechnung | 990 | 3225501345 |
//!
//! Eine Aufzeichnung, die den Unterschied nicht zeigen kann, ist nur eine
//! weitere gruene Datei.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const SYNERGY_JSON: &str = include_str!("../../tests/parity/recordings/synergy-window.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(SYNERGY_JSON).expect("synergy-window.json ist kein JSON");
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

fn replay(json: &Value, with_checksums: bool) -> (Recording, rota_sim::ReplayResult) {
    let recording = recording(json);
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, with_checksums);
    (recording, result)
}

#[test]
fn die_runde_wirkt_das_paar_ueberhaupt() {
    // **Die Vorbedingung, ohne die alles Weitere nichts belegt.** Eine
    // Aufzeichnung, in der CANKER und FESTER nicht vorkommen, waere gruen und
    // sagte ueber die Synergie nichts — genau die Lage, aus der dieses Issue
    // entstanden ist.
    let json = data();
    let recording = recording(&json);

    let canker = index_of("canker").expect("CANKER fehlt");
    let fester = index_of("fester").expect("FESTER fehlt");
    let gelegt: Vec<usize> = recording.bindings.iter().map(|b| b.spell).collect();
    assert!(
        gelegt.contains(&canker) && gelegt.contains(&fester),
        "die Leiste der Aufzeichnung fuehrt das Paar nicht: {gelegt:?}"
    );

    // Und beide werden auch GEDRUECKT, nicht nur gebunden. Der Unterschied ist
    // der Befund aus #340: acht Zauber stehen in den Aufzeichnungen, sieben
    // werden nie gedrueckt.
    let mut canker_druecke = 0;
    let mut fester_druecke = 0;
    for press in &recording.presses {
        match recording.spell_for_slot(press.slot) {
            Some(spell) if spell == canker => canker_druecke += 1,
            Some(spell) if spell == fester => fester_druecke += 1,
            _ => {}
        }
    }
    assert!(
        canker_druecke > 0 && fester_druecke > 0,
        "gedrueckt: CANKER {canker_druecke}, FESTER {fester_druecke} — eine \
         Aufzeichnung, in der einer von beiden fehlt, kann kein Fenster nutzen"
    );
}

#[test]
fn die_synergie_wird_in_der_runde_genutzt() {
    // **Nicht „es gibt Gelegenheiten", sondern „sie werden genutzt".** Ein Lauf
    // mit offenen Fenstern und null Nutzungen liefe durch jede Pruefsumme, ohne
    // dass der Bonus je in die Rechnung kaeme.
    let json = data();
    let expected = &json["expected"];
    let genutzt = expected["synergyUsed"].as_u64().expect("synergyUsed fehlt");
    let moeglich = expected["synergyAvailable"]
        .as_u64()
        .expect("synergyAvailable fehlt");

    assert!(
        genutzt > 0,
        "kein Fenster genutzt — dann belegt die Aufzeichnung die Rechnung nicht"
    );
    assert!(
        genutzt <= moeglich,
        "mehr genutzt ({genutzt}) als moeglich ({moeglich})"
    );

    // **Und der SIMULATOR muss auf dieselbe Zahl kommen.** Ohne diesen
    // Vergleich prueft der Fall nur, was in der Datei steht — die das Spiel
    // geschrieben hat. Der Simulator kaeme mit null genutzten Fenstern durch,
    // solange der Schaden zufaellig stimmt.
    let (_, result) = replay(&json, false);
    assert_eq!(
        u64::from(result.synergy_used),
        genutzt,
        "der Simulator nutzt {} Fenster, das Spiel {genutzt}",
        result.synergy_used
    );
    assert_eq!(u64::from(result.synergy_available), moeglich);
}

#[test]
fn spiel_und_simulator_rechnen_dieselbe_runde() {
    let json = data();
    let (recording, result) = replay(&json, false);
    let expected = &json["expected"];

    assert!(!recording.presses.is_empty());
    assert_eq!(
        u64::from(result.damage),
        expected["damage"].as_u64().expect("damage fehlt"),
        "der Gesamtschaden weicht ab"
    );
    assert_eq!(
        u64::from(result.accepted),
        expected["accepted"].as_u64().expect("accepted fehlt")
    );
    assert_eq!(
        u64::from(result.rejected),
        expected["rejected"].as_u64().expect("rejected fehlt")
    );
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let (_, result) = replay(&json, true);
    let expected: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect();

    assert!(!expected.is_empty());

    if let Some(d) = first_divergence(&expected, &result.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             uebereinstimmende Takte davor: {}",
            d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
        );
    }
    assert_eq!(result.checksums.len(), expected.len());
}
