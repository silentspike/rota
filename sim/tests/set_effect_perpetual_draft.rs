//! Der Satz-Effekt `perpetual-draft` wirkt auch im Simulator (13.5, #480).
//!
//! Der Ruecksetzer bekommt [`PERPETUAL_DRAFT_EXTRA_CHARGE`] Ladungen mehr.
//!
//! # Derselbe Aufbau wie `talent_charges.rs`, aus demselben Grund
//!
//! Der Ruecksetzer ist ein AUSWAHLMODUS: Er stellt scharf, und erst der
//! naechste Druck sagt, welchen Zauber er meint. Bedient wird diese Wahl nur
//! im Abspieler (`fight.rs`, `if self.reset_mode.is_armed()`), nicht in
//! `Fight::run`. Ein Aufbau ueber `run` bliebe deshalb stumm — mit und ohne
//! Effekt dieselben Zahlen, und zwar weil gar nichts zurueckgesetzt wird.
//!
//! Gemessen wird der HOECHSTE Wert der Ladungs-Spur, nicht der letzte: Die
//! Ladungen zaehlen waehrend des Laufs herunter, am Ende steht regelmaessig 0,
//! und zwei Nullen zu vergleichen sagte nichts.
//!
//! Die Aufzeichnung `alembic-zwei-ladungen.json` gibt es bereits und sie fuehrt
//! die Ladungs-Spur mit — es braucht also weder eine neue noch einen Export.
//! ALEMBIC traegt zwei Ladungen aus der ZAUBER-Eigenschaft; daran zeigt sich,
//! ob der Effekt sich addiert oder die Eigenschaft ersetzt.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::sets::PERPETUAL_DRAFT_EXTRA_CHARGE;

const JSON: &str = include_str!("../../tests/parity/recordings/alembic-zwei-ladungen.json");

/// Der hoechste Stand der Ladungs-Spur, mit oder ohne den Effekt.
fn hoechste_ladung(perpetual_draft: bool, talent_bonus: u32) -> u32 {
    let json: Value = serde_json::from_str(JSON).expect("alembic-zwei-ladungen.json ist kein JSON");
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks fehlen")).expect("zu viele");
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));

    let bindings: Vec<Binding> = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| Binding {
            slot: u8::try_from(b["slot"].as_u64().expect("slot fehlt")).expect("zu gross"),
            spell: index_of(b["spellId"].as_str().expect("spellId fehlt"))
                .expect("Zauber der Aufzeichnung fehlt im Katalog"),
        })
        .collect();
    let presses: Vec<Press> = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick fehlt")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot fehlt")).expect("zu gross"),
        })
        .collect();

    let recording = Recording {
        name: String::from("alembic-zwei-ladungen"),
        seed: 0,
        ticks,
        presses,
        bindings,
    };
    let mut fight = Fight::new(FightConfig {
        boss,
        perpetual_draft,
        talent_cooldown_charges_bonus: talent_bonus,
        ..FightConfig::default()
    });
    fight
        .replay(&recording, true)
        .reset_charges_left
        .into_iter()
        .max()
        .expect("die Ladungs-Spur ist leer")
}

#[test]
fn der_traeger_stellt_ueberhaupt_ladungen_scharf() {
    // **Zuerst der Traeger, dann die Wirkung.** Bliebe die Spur auf 0,
    // verglichen die Faelle unten zwei Nullen.
    assert!(
        hoechste_ladung(false, 0) > 0,
        "die Ladungs-Spur bleibt bei 0 — der Ruecksetzer wurde nie scharf \
         gestellt, und ein Satz-Effekt darauf koennte sich nicht zeigen"
    );
}

#[test]
fn mit_dem_effekt_steht_eine_ladung_mehr_scharf() {
    let ohne = hoechste_ladung(false, 0);
    let mit = hoechste_ladung(true, 0);

    // Gegen die exakte Differenz, nicht gegen "mehr": Das trennt einen
    // Zuschlag von einem Faktor. Bei zwei Grund-Ladungen sieht ein
    // verdoppelnder Fehler genauso aus wie +2.
    assert_eq!(
        mit,
        ohne + u32::from(PERPETUAL_DRAFT_EXTRA_CHARGE),
        "mit dem Effekt {mit}, ohne {ohne} — erwartet genau eine Ladung mehr"
    );
}

#[test]
fn die_zauber_eigenschaft_wird_ergaenzt_nicht_ersetzt() {
    // ALEMBIC traegt zwei Ladungen. Wird die Eigenschaft ersetzt statt
    // ergaenzt, stuende hier 1 statt 3 — und der Fall oben bliebe trotzdem
    // gruen, weil er nur die Differenz kennt.
    assert_eq!(hoechste_ladung(false, 0), 2, "ALEMBIC traegt zwei Ladungen");
    assert_eq!(hoechste_ladung(true, 0), 3);
}

#[test]
fn talent_und_satz_effekt_addieren_sich() {
    // Die eigentliche Auflage: EINE Summe mit drei Speisungen. Zwei aus dem
    // Zauber, fuenf aus dem Talent, eine aus dem Satz.
    assert_eq!(hoechste_ladung(true, 5), 8);
    // ZUSICHERUNG: Ohne den Satz-Effekt sind es genau die sieben aus Zauber
    // und Talent — sonst bewiese die 8 oben nicht, WOHER der Zuwachs kommt.
    assert_eq!(hoechste_ladung(false, 5), 7);
}
