//! Der 1. Boss der fuenften Instanz im Paritaets-Weg.
//!
//! # Was er verlangt
//!
//! DIE GETEILTE ANTWORT: `energy` und `affliction` zugleich. Das Gift nennt in
//! `clearedBy` genau QUENCH und TEMPER — dieselben zwei Zeilen, die man vor der
//! Entladung fuer Schutz braeuchte. Wer reinigt, steht bei 100 ohne Vorbereitung
//! da.
//!
//! # Warum QUENCH hier die Signatur traegt, obwohl es nur EINMAL vorkommt
//!
//! Dieser Boss wirkt keinen unterbrechbaren Zauber — seine Zeitleiste besteht
//! aus zwei `strike`. Ein QUENCH kann also nicht vom Unterbrechen stammen,
//! sondern nur aus der Reinigungs-Zeile der Rotation. Die Eins ist deshalb
//! aussagekraeftiger als eine groessere Zahl bei einem Boss, der beides zulaesst:
//! Sie ist eindeutig zuzuordnen.
//!
//! # Was diese Datei NICHT belegt
//!
//! Ob die Entladung von `energy` je ausgeloest hat. Die Aufzeichnung fuehrt
//! `casts` und Schaden, nicht `bossEnergyTriggers` — die Zahl steht in keinem
//! Feld, das hier zu lesen waere. Rechnerisch faellt sie bei 6 Punkten je
//! Sekunde und Schwelle 100 etwa alle 16,7 s, und der Kampf dauert laenger.
//! **Rechnerisch ist nicht gemessen**, und deshalb steht hier keine Zusicherung
//! darauf.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance5-boss-1.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance5-boss-1.json ist kein JSON");
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
    // **Der Traeger der Signatur**, und er ist hier eindeutig: Dieser Boss wirkt
    // keinen unterbrechbaren Zauber, also stammt das QUENCH aus der Reinigung.
    assert_eq!(
        json["expected"]["casts"]["quench"].as_u64(),
        Some(1),
        "quench wird nicht 1-mal gewirkt — das Gift wird nicht mehr abgetragen"
    );
    assert_eq!(
        json["expected"]["casts"]["kindle"].as_u64(),
        Some(23),
        "kindle wird nicht 23-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4684),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    // **Der Spieler ueberlebt.** Stuerbe er, endete die Aufzeichnung vorzeitig
    // und pruefte den Rest des Kampfes nicht mehr.
    assert_eq!(
        json["expected"]["diedAtMs"].as_i64(),
        Some(-1),
        "der Spieler stirbt — die Reinigung greift nicht mehr"
    );
    // **Der Kampf reicht ueber die neunte Sekunde hinaus.** Das Gift beginnt bei
    // 8000 ms; ein Lauf, der vorher endete, haette den Baustein nie beruehrt.
    assert!(
        json["expected"]["killedAtMs"].as_i64().unwrap_or(-1) > 8000,
        "der Kampf endet vor dem Einsetzen des Giftes — die Signatur kommt nicht vor"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-5-1"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-1").expect("boss-5-1 fehlt im Simulator");
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

/// Takt fuer Takt — hier haengen die Gift-Ticks und die Energieleiste drin.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-1").expect("boss-5-1 fehlt im Simulator");
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
