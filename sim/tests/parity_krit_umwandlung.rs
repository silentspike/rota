//! Der Zufallsstrom unter Krit (Issue #532).
//!
//! # Was hier zum ersten Mal geprueft wird
//!
//! Alle bisherigen Kampf-Aufzeichnungen laufen mit Krit-Chance NULL. Sie
//! belegen die Bitgleichheit fuer den einen Fall, in dem der Zufallsstrom
//! keine Rolle spielt — er wird nie gezogen und kann nicht auseinanderlaufen.
//!
//! Ein Strom, der nie fliesst, sieht aus wie einer, der gleich fliesst.
//!
//! # Warum WEDGE
//!
//! Er macht Schaden AUSSCHLIESSLICH ueber die Umwandlung (`base_damage: 0`,
//! kein `spender`). Genau bei dieser Sorte hat Rust den Krit-Wurf nicht
//! gezogen (#520). Ein Zauber mit Grundschaden haette ihn ohnehin gezogen und
//! die Luecke verdeckt.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::gear::WornValues;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/krit-umwandlung.json");

fn worn(json: &Value) -> WornValues {
    let node = &json["worn"];
    let feld = |name: &str| node[name].as_u64().expect("worn-Feld fehlt") as u32;
    WornValues {
        catalysis: feld("catalysis"),
        purity: feld("purity"),
        yield_bonus: feld("yieldBonus"),
        essence: feld("essence"),
        vessel: feld("vessel"),
        ward: feld("ward"),
        gear_effects: feld("gearEffects"),
    }
}

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("krit-umwandlung.json ist kein JSON");
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

/// **Die Zusicherung vor jeder Messung.**
///
/// Eine Aufzeichnung, in der WEDGE nicht vorkommt, belegt fuer ihn nichts —
/// und genau dafuer entsteht sie.
#[test]
fn wedge_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let wedge = index_of("wedge").expect("wedge fehlt im Simulator");
    let wedge_slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == wedge)
        .map(|b| b.slot)
        .expect("wedge liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == wedge_slot)
        .count();
    assert!(
        druecke > 0,
        "wedge liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    // Und das Spiel hat dasselbe gezaehlt.
    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["wedge"]
            .as_u64()
            .expect("casts.wedge fehlt"),
        "Simulator und Spiel zaehlen verschieden viele WEDGE-Wirkungen"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        // **Ohne die getragenen Werte waere die Krit-Chance null** — dann
        // praefe dieser Fall genau das, was alle anderen schon tun.
        worn: worn(&data()),
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
        // **Ohne die getragenen Werte waere die Krit-Chance null** — dann
        // praefe dieser Fall genau das, was alle anderen schon tun.
        worn: worn(&data()),
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
