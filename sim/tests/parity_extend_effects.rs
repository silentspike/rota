//! HARDEN und SEPSIS im Paritaets-Weg (Issue #523).
//!
//! # Warum es diese Datei braucht
//!
//! Wie bei #517: seit dem Einbau ungedeckt. Eine Regel, die keine
//! Aufzeichnung ausloest, ist fuer die Paritaet unsichtbar.
//!
//! # Was sie zeigt
//!
//! Einen laufenden Ablauf verlaengern, ohne ihn neu zu setzen — bei HARDEN
//! die Schutzschicht, bei SEPSIS **alle** Zersetzungs-Stapel um denselben
//! Betrag. Beim zweiten haengt die Verfallsreihenfolge daran: `decay_expiry_ms`
//! ist sortiert, und ein Zuschlag auf nur einen Eintrag braeche die Ordnung,
//! ohne die Stapelzahl zu veraendern.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/extend-effects.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("extend-effects.json ist kein JSON");
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

/// **Die Zusicherung vor jeder Messung** — fuer HARDEN einzeln.
///
/// Eine Aufzeichnung, in der HARDEN nicht vorkommt, belegt fuer ihn
/// nichts. Und weil hier 2 Zauber in einer Datei stehen, muss das
/// **je Zauber** dastehen: Ein gemeinsamer Fall waere schon gruen, wenn nur
/// einer von ihnen gedrueckt wird.
#[test]
fn harden_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("harden").expect("harden fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("harden liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "harden liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["harden"]
            .as_u64()
            .expect("casts.harden fehlt"),
        "Simulator und Spiel zaehlen verschieden viele HARDEN-Wirkungen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer SEPSIS einzeln.
///
/// Eine Aufzeichnung, in der SEPSIS nicht vorkommt, belegt fuer ihn
/// nichts. Und weil hier 2 Zauber in einer Datei stehen, muss das
/// **je Zauber** dastehen: Ein gemeinsamer Fall waere schon gruen, wenn nur
/// einer von ihnen gedrueckt wird.
#[test]
fn sepsis_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("sepsis").expect("sepsis fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("sepsis liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "sepsis liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["sepsis"]
            .as_u64()
            .expect("casts.sepsis fehlt"),
        "Simulator und Spiel zaehlen verschieden viele SEPSIS-Wirkungen"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);
    let erwartet = &json["expected"];

    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
    );
    assert_eq!(
        ergebnis.accepted,
        u32_at(erwartet, "accepted"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert_eq!(
        ergebnis.final_checksum,
        u32_at(erwartet, "finalChecksum"),
        "die Schluss-Pruefsumme weicht ab"
    );
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);

    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();

    // `first_divergence` nennt den Takt statt nur „ungleich" — bei dreitausend
    // Eintraegen ist das der Unterschied zwischen einem Hinweis und einer
    // Suche.
    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
