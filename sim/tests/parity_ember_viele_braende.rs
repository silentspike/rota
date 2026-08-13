//! EMBER auf MEHREREN Braenden zugleich im Paritaets-Weg (Issue #485, AC3).
//!
//! # Warum `parity_dot_lifetime` dafuer nicht reicht — gemessen
//!
//! Jene Aufzeichnung (`dot-lifetime.json`) faehrt genau EINEN Brand. Die Gegenprobe "verlaengere nur
//! den ersten laufenden Brand" — der Fehler, vor dem AC3 ausdruecklich warnt —
//! lief am 2026-08-13 durch **alle 134** Rust-Suiten, ohne eine einzige rot zu
//! faerben. Mit einem Brand sind "der erste" und "alle" dasselbe.
//!
//! Hier laufen SMOULDER, SEARING und CINDER gleichzeitig, und EMBER trifft sie
//! 15 mal. CINDER hat einen anderen Takt (1000 statt 2000) — das trennt
//! zusaetzlich "alle gefunden" von "allen denselben Betrag gegeben".

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/ember-viele-braende.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("ember-viele-braende.json ist kein JSON");
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

/// **Die Zusicherung vor jeder Messung** — fuer EMBER einzeln.
///
/// Eine Aufzeichnung, in der EMBER nicht vorkommt, belegt fuer ihn nichts.
/// Und weil hier VIER Zauber auf der Leiste liegen — er und die drei Braende
/// —, steht die Zusicherung je Zauber: Ein gemeinsamer Fall waere schon gruen,
/// wenn nur einer von ihnen gedrueckt wird. Die drei Braende deckt
/// `drei_braende_liegen_auf_der_leiste` ab.
#[test]
fn ember_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("ember").expect("ember fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("ember liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "ember liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["ember"]
            .as_u64()
            .expect("casts.ember fehlt"),
        "Simulator und Spiel zaehlen verschieden viele EMBER-Wirkungen"
    );
}

/// ZUSICHERUNG: Es laufen wirklich DREI Braende gleichzeitig.
///
/// Ohne sie belegte die Paritaet nur, dass beide Seiten dasselbe rechnen —
/// nicht, dass sie den Mehrfach-Zugriff ueberhaupt beruehren. Genau daran ist
/// `dot-lifetime` fuer AC3 gescheitert: ein Brand, und "der erste" ist "alle".
#[test]
fn drei_braende_liegen_auf_der_leiste() {
    let json = data();
    let aufzeichnung = recording(&json);

    for name in ["smoulder", "searing", "cinder"] {
        let zauber = index_of(name).unwrap_or_else(|| panic!("{name} fehlt im Simulator"));
        let slot = aufzeichnung
            .bindings
            .iter()
            .find(|b| b.spell == zauber)
            .map(|b| b.slot)
            .unwrap_or_else(|| panic!("{name} liegt nicht auf der Leiste"));
        let druecke = aufzeichnung
            .presses
            .iter()
            .filter(|p| p.slot == slot)
            .count();
        assert!(druecke > 0, "{name} wird nie gedrueckt");
    }
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
