//! Beide Seiten rechnen denselben Verwundbarkeits-Faktor (Issue #738).
//!
//! # Die Lücke, die diese Datei schliesst
//!
//! Die Faktoren-Spur `factorAt` uebertrug `buffs` takt-fuer-takt und
//! `vulnerability` gar nicht — zwei Faktoren derselben Familie, einer
//! verglichen, der andere nicht. Auf der Spiel-Seite gibt es
//! `combat.vulnerability` seit Langem und drei Pruefdateien lesen ihn; auf
//! dieser Seite gab es bis #738 kein Gegenstueck.
//!
//! # Warum eine EIGENE Aufzeichnung
//!
//! `buff-combat` traegt die Spur auch, faehrt aber keine Zersetzung: Ihr
//! `vulnerability` ist konstant 1,0. Sie ist damit die **Gegenprobe** — ein
//! Lauf ohne Zersetzung, der gruen bleibt — und kann genau deshalb den
//! Wirkungsnachweis nicht fuehren. Ein Wert, der nie von 1 abweicht, faerbt
//! bei keiner Mutation.

use std::fs;

use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

fn daten() -> serde_json::Value {
    let pfad = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/parity/recordings/decay-factor.json"
    );
    serde_json::from_str(&fs::read_to_string(pfad).expect("decay-factor.json fehlt"))
        .expect("decay-factor.json ist kein JSON")
}

/// Spielt die Aufzeichnung ab und liefert den Faktor am ENDE.
///
/// **Nur der Endwert, nicht je Takt** — und das ist eine Einschraenkung, keine
/// Wahl: `Fight::replay` fuehrt Pruefsummen je Takt, aber keine Faktoren-Spur.
/// Sie dort einzuhaengen waere der Abspieler-Umbau, den dieser Schritt
/// vermeiden soll. Der letzte Messpunkt der Aufzeichnung liegt bei Takt 599,
/// also am selben Ort.
fn faktor_am_ende(json: &serde_json::Value) -> f64 {
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks fehlen")).expect("zu viele");
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));

    let mut bindings: Vec<Binding> = Vec::new();
    let mut presses: Vec<Press> = Vec::new();
    for eintrag in json["presses"].as_array().expect("presses fehlen") {
        let id = eintrag["spellId"].as_str().expect("spellId fehlt");
        let index = SPELLS
            .iter()
            .position(|s| s.id == id)
            .unwrap_or_else(|| panic!("Zauber {id} gibt es nicht"));
        let slot = match bindings.iter().position(|b| b.spell == index) {
            Some(s) => u8::try_from(s).expect("mehr Plaetze als eine Leiste fasst"),
            None => {
                let neu = u8::try_from(bindings.len()).expect("mehr Plaetze als eine Leiste fasst");
                bindings.push(Binding {
                    slot: neu,
                    spell: index,
                });
                neu
            }
        };
        presses.push(Press {
            tick: u32::try_from(eintrag["tick"].as_u64().expect("tick fehlt")).expect("zu gross"),
            slot,
        });
    }

    let recording = Recording {
        name: String::from("decay-factor"),
        seed: 0,
        ticks,
        presses,
        bindings,
    };
    let mut fight = Fight::new(FightConfig {
        boss,
        ..FightConfig::default()
    });
    fight.replay(&recording, true);
    fight.vulnerability()
}

#[test]
fn die_aufzeichnung_traegt_ueberhaupt_zersetzung() {
    // **Die Zusicherung im Datensatz.** Ohne sie verglichen die Faelle unten
    // zweimal 1,0 gegeneinander und waeren gruen, ohne etwas zu belegen —
    // genau die Lage, in der `buff-combat` steckt.
    let json = daten();
    let spur: Vec<f64> = json["factorAt"]
        .as_array()
        .expect("factorAt fehlt")
        .iter()
        .map(|p| p["vulnerability"].as_f64().expect("vulnerability fehlt"))
        .collect();

    assert!(
        spur.iter().any(|&v| v > 1.0),
        "die Verwundbarkeit bleibt in der ganzen Spur bei 1,0 — dann faehrt \
         die Aufzeichnung ihre Zersetzung gar nicht mit: {spur:?}"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_faktor() {
    let json = daten();
    let spur = json["factorAt"].as_array().expect("factorAt fehlt");
    let letzter = spur.last().expect("factorAt ist leer")["vulnerability"]
        .as_f64()
        .expect("vulnerability fehlt");

    let gemessen = faktor_am_ende(&json);

    assert!(
        (gemessen - letzter).abs() < 1e-9,
        "am Ende rechnet der Simulator {gemessen}, das Spiel {letzter}"
    );
}
