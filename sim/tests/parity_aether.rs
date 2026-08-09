//! **AETHERs Fenster, auf beiden Seiten gleich** (Issue #480)
//!
//! AETHER verdoppelt für acht Wartezeiten, was die Verkürzer abziehen —
//! dieselbe Bauart wie DISTIL, nur länger und mit viermal der Abklingzeit.
//!
//! # Warum diese Aufzeichnung eigenständig ist
//!
//! Der erste Entwurf legte AETHER in die bestehende `sublimation`-Runde. Das
//! ging schief, und der Grund gehört hierher, damit es niemand wiederholt:
//!
//! ```text
//!   parity_sublimation_run misst an VAPOURs Restzeit einen Sprung von
//!   genau 1010 ms (10 ms Uhr + 1000 ms VAPOUR).
//!
//!   Mit AETHER in derselben Runde verdoppelt sich der Betrag — der Fall
//!   fand "kein einziger Fall der Restzeit betraegt 1010 ms" und verlor
//!   seinen Traeger.
//! ```
//!
//! Das ist dieselbe Klasse, die WISP aus jener Aufzeichnung vertrieben hat,
//! aber nicht derselbe Mechanismus: WISP verdrängte einen anderen Zauber,
//! AETHER verändert **die gemessene Größe selbst**.
//!
//! # Was hier geprüft wird
//!
//! Die Prüfsumme je Takt und die Restzeit von KINDLE. An KINDLE wird
//! gemessen, weil VAPOUR sich nicht selbst verkürzt: Wer an der eigenen
//! Restzeit des Verkürzers misst, sieht die Wirkung nie.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/aether-fenster.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("aether-fenster.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn nachspielen(json: &Value) -> Fight {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| {
            let id = b["spellId"].as_str().expect("spellId");
            Binding {
                slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt")),
            }
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
    fight.replay(&recording, false);
    fight
}

#[test]
fn ac0_der_lauf_wirft_aether_ueberhaupt() {
    // **Die Zusicherung vor allem anderen.** Steht hier eine Null, misst der
    // Fall eine Runde ohne den Zauber, um den es geht — und wäre grün, weil
    // dann eben nichts zu verstärken war.
    let json = data();
    let wuerfe = json["expected"]["aetherCasts"]
        .as_u64()
        .expect("aetherCasts fehlt");
    assert!(
        wuerfe > 0,
        "die Aufzeichnung wirft AETHER kein einziges Mal — sie belegt nichts über ihn"
    );
    let verkuerzer = json["expected"]["vapourCasts"]
        .as_u64()
        .expect("vapourCasts fehlt");
    assert!(
        verkuerzer > 0,
        "die Aufzeichnung wirft VAPOUR nie — dann gibt es keine Verkürzung, \
         die AETHER verdoppeln könnte"
    );
}

#[test]
fn ac1_beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let fight = nachspielen(&json);
    let erwartet = json["expected"]["damageDealt"]
        .as_u64()
        .expect("damageDealt fehlt");
    assert_eq!(
        u64::from(fight.damage()),
        erwartet,
        "der Schaden geht auseinander"
    );
}

#[test]
fn ac2_die_pruefsumme_stimmt_in_jedem_takt() {
    // Der eigentliche Paritäts-Nachweis: Bit für Bit, Takt für Takt.
    let json = data();
    let boss_id = json["bossId"].as_str().expect("bossId");
    let boss = boss_by_id(boss_id).expect("Boss");
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|c| u32::try_from(c.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect();

    assert!(
        !erwartet.is_empty(),
        "keine Prüfsummen in der Aufzeichnung — der Fall vergleicht nichts"
    );

    let bindings: Vec<Binding> = json["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .map(|b| Binding {
            slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot"),
            spell: index_of(b["spellId"].as_str().expect("spellId")).expect("Zauber"),
        })
        .collect();
    let presses: Vec<Press> = json["presses"]
        .as_array()
        .expect("presses")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick")).expect("tick"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot"),
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
    // `true` schaltet die Prüfsumme je Takt ein — ohne sie bleibt die Liste
    // leer, und der Vergleich unten verglich zwei Nichtse.
    let ergebnis = fight.replay(&recording, true);

    assert_eq!(
        ergebnis.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: {} gegen {}",
        ergebnis.checksums.len(),
        erwartet.len()
    );
    for (i, (a, b)) in ergebnis.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(a, b, "Prüfsumme weicht in Takt {i} ab");
    }
}
