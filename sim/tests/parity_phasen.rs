//! Ein Phasenwechsel mit gebundenem Ereignis — beide Seiten gleich (#557).
//!
//! # Warum es diese Datei geben MUSS
//!
//! Vor #557 war `bossPhase` ein Zustand ohne Wirkung: gesetzt, in die
//! Pruefsumme geruehrt, von keiner Regel gelesen. Ein Boss mit zwei
//! Phasenschwellen verlief bitgleich zu demselben Boss mit `phases: []` —
//! gemessen an 1000 Durchgaengen.
//!
//! Diese Aufzeichnung faehrt `test-phasenbindung`: ein Ereignis mit
//! `phase: 1`, das erst ab der zweiten Phase feuert. Beide Seiten muessen an
//! derselben Stelle dieselbe Entscheidung treffen — feuern oder verfallen.
//!
//! # Die Regel, die hier mitgeprueft wird
//!
//! Ein uebersprungenes Ereignis VERFAELLT, es wird nicht nachgeholt. Der
//! Index laeuft weiter, die Wirkung entfaellt. Ein aufgestautes Ereignis
//! fiele beim Phasenwechsel gesammelt mit allen anderen.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/phasen-bindung.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("phasen-bindung.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn zahlen(json: &Value, feld: &str) -> Vec<u32> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect()
}

fn nachspielen(json: &Value) -> (Fight, rota_sim::ReplayResult) {
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
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, true);
    (fight, ergebnis)
}

#[test]
fn ac3_der_phasenwechsel_findet_ueberhaupt_statt() {
    // **Die Ursache zuerst.** Bleibt der Boss in Phase 0, verfaellt das
    // gebundene Ereignis und der Lauf zeigt nichts.
    let json = data();
    let hoechste = json["expected"]["hoechstePhase"]
        .as_u64()
        .expect("hoechstePhase");
    assert!(
        hoechste > 0,
        "die hoechste erreichte Phase ist {hoechste} — dann faehrt diese \
         Aufzeichnung keinen Wechsel, und das gebundene Ereignis verfaellt"
    );
}

#[test]
fn ac3_die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    // `bossPhase` steckt in der Pruefsumme. Eine Seite, die den Wechsel zu
    // einem anderen Takt vollzieht — oder das Ereignis feuern laesst, wo die
    // andere es verfallen laesst — faellt hier auf.
    let json = data();
    let (_, ergebnis) = nachspielen(&json);
    let soll = zahlen(&json, "checksums");

    assert!(!soll.is_empty());
    if let Some(d) = first_divergence(&soll, &ergebnis.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} ({} ms)\n  \
             Spiel: {}   Simulator: {}\n  gleiche Takte davor: {}",
            d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
        );
    }
    assert_eq!(ergebnis.checksums.len(), soll.len());
}

#[test]
fn ac3_beide_seiten_ergeben_denselben_gesamtschaden() {
    let json = data();
    let (fight, _) = nachspielen(&json);
    assert_eq!(
        u64::from(fight.damage()),
        json["expected"]["damageDealt"]
            .as_u64()
            .expect("damageDealt"),
        "der Gesamtschaden weicht ab"
    );
}

/// Das gebundene Ereignis hat wirklich getroffen.
///
/// # Warum die Pruefsumme das nicht schon erledigt
///
/// Sie zeigt Gleichlauf, nicht WIRKUNG. Waeren beide Seiten gleich falsch —
/// beide ohne Bindung, beide mit — liefe sie durch. Der eingesteckte Schaden
/// trennt die Faelle: Mit gebundenem Ereignis ist er hoeher als ohne.
///
/// Gemessen am Balance-Laeufer: 1040 mit Phasen, 320 mit `phases: []`.
#[test]
fn ac3_das_gebundene_ereignis_hat_getroffen() {
    let json = data();
    let (fight, _) = nachspielen(&json);
    let soll = json["expected"]["damageTaken"]
        .as_u64()
        .expect("damageTaken");
    assert!(
        soll > 320,
        "der Spieler steckte nur {soll} ein — das ist der Wert OHNE das \
         gebundene Ereignis. Dann ist die Phase nicht erreicht worden, und \
         der Fall belegt die Bindung nicht"
    );
    assert_eq!(
        u64::from(fight.damage_taken()),
        soll,
        "die Seiten rechnen verschieden viel eingesteckten Schaden"
    );
}
