//! Der 3. Boss der fuenften Instanz im Paritaets-Weg.
//!
//! # Was er verlangt
//!
//! DIE BESCHLEUNIGTE EINDAEMMUNG: `runaway` und `escalation` zugleich. Der Ring
//! fuellt sich von selbst, und die Verkuerzung greift an beidem — der
//! Schutzzauber wird billiger in Zeit, der Zerfall haeufiger.
//!
//! # CRUST ist hier der Traeger, und das ist keine Geschmacksfrage
//!
//! Die Bedingung aus #394: Ein Boss mit umgekehrtem Ziel braucht einen
//! Stabilisierer, der OHNE eingehenden Schaden wirkt. CRUST erfuellt das, TEMPER
//! nicht — er wandelt ausgeteilten Schaden in Schutz, und damit stabilisierte
//! man DURCH Schaden. Der Simulator sagt es an der Stelle, an der es wirkt
//! (`stabilise` in fight.rs).
//!
//! Steht CRUST hier auf null, ist die halbe Aufgabe des Bosses nicht gefahren
//! worden — und der Lauf saehe trotzdem aus wie ein gelungener. Deshalb ist die
//! Acht unten eine Zusicherung und keine Beobachtung.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance5-boss-3.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance5-boss-3.json ist kein JSON");
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

/// **Die Zusicherung vor jeder Messung** — mit Zahlen, nicht mit Anwesenheit.
#[test]
fn die_aufzeichnung_ist_noch_dieselbe() {
    let json = data();
    // **Der Traeger der Signatur.** Acht Stabilisierungen; faellt die Zahl auf
    // null, faehrt die Aufzeichnung den Boss nur noch von aussen, und `runaway`
    // waere eine Zahl in den Daten statt einer Anforderung im Kampf.
    assert_eq!(
        json["expected"]["casts"]["crust"].as_u64(),
        Some(8),
        "crust wird nicht 8-mal gewirkt — es wird nicht mehr stabilisiert"
    );
    // **TEMPER darf NICHT vorkommen.** Er stabilisiert durch ausgeteilten
    // Schaden und hoebe die Bedingung des Bausteins auf (#394). Er steht nicht
    // auf der Leiste; diese Zeile faengt den Tag, an dem er dazukaeme.
    assert_eq!(
        json["expected"]["casts"]["temper"].as_u64(),
        Some(0),
        "temper wird gewirkt — damit stabilisiert der Lauf DURCH Schaden"
    );
    // **Die Unterbrechung.** `escalation` wirkt auf den Quickening Chant.
    assert_eq!(
        json["expected"]["casts"]["quench"].as_u64(),
        Some(3),
        "quench wird nicht 3-mal gewirkt — die Unterbrechung kommt nicht mehr vor"
    );
    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4834),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    assert_eq!(
        json["expected"]["diedAtMs"].as_i64(),
        Some(-1),
        "der Spieler stirbt — das Stabilisieren haelt den Ring nicht mehr"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-5-3"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-3").expect("boss-5-3 fehlt im Simulator");
    let mut fight = Fight::new(FightConfig {
        boss,
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

/// Takt fuer Takt — hier haengen der wachsende Ring und die Verkuerzung drin.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-3").expect("boss-5-3 fehlt im Simulator");
    let mut fight = Fight::new(FightConfig {
        boss,
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

    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
