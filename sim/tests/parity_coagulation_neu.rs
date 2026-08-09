//! Die neuen Zauber der COAGULATION rechnen auf beiden Seiten gleich
//! (Issue #481).
//!
//! # Warum eine eigene Aufzeichnung
//!
//! `extend-effects` wirkt bewusst je EINEN CRUST und EINEN HARDEN, damit die
//! Schicht innerhalb des Laufs ablaeuft — ihr Kopfkommentar sagt es
//! ausdruecklich. Ein dritter Schild-Zauber koennte genau das kippen. Eine
//! fremde Messung fasst man nicht an, um die eigene unterzubringen.
//!
//! # Warum sie eine SAMMELSTELLE ist
//!
//! Von den fuenf neuen Zaubern der Schule steht bisher CRUCIBLE SKIN;
//! POULTICE, MEND, SUTURE und RIPOSTE kommen hinzu, ohne dass eine weitere
//! Datei entsteht. Die Zusicherung darunter steht deshalb JE ZAUBER: Ein
//! gemeinsamer Fall waere schon gruen, wenn nur einer von ihnen gedrueckt
//! wird.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/coagulation-neu.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("coagulation-neu.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
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

/// **Die Zusicherung vor jeder Messung** — fuer CRUCIBLE SKIN einzeln.
///
/// Eine Aufzeichnung, in der er nicht vorkommt, belegt fuer ihn nichts. Und
/// weil hier kuenftig mehrere Zauber in einer Datei stehen, muss das JE
/// ZAUBER dastehen.
#[test]
fn crucible_skin_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("crucible-skin").expect("crucible-skin fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("crucible-skin liegt auf keinem Platz");

    assert!(
        aufzeichnung.presses.iter().any(|p| p.slot == slot),
        "crucible-skin wird in dieser Aufzeichnung nie gedrueckt — dann belegt sie fuer ihn nichts"
    );
    assert!(
        json["expected"]["casts"]["crucible-skin"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "die Aufzeichnung zaehlt keinen einzigen Wurf von crucible-skin"
    );
}

/// Dieselbe Zusicherung fuer POULTICE (Issue #481).
#[test]
fn poultice_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("poultice").expect("poultice fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("poultice liegt auf keinem Platz");

    assert!(
        aufzeichnung.presses.iter().any(|p| p.slot == slot),
        "poultice wird in dieser Aufzeichnung nie gedrueckt — dann belegt sie fuer ihn nichts"
    );
    assert!(
        json["expected"]["casts"]["poultice"].as_u64().unwrap_or(0) > 0,
        "die Aufzeichnung zaehlt keinen einzigen Wurf von poultice"
    );
}

/// **Gewirkt heisst nicht gewirkt HAT.**
///
/// Der Fall darueber belegt, dass die Zauber gedrueckt werden. Er waere auch
/// dann gruen, wenn jeder von ihnen ins Leere liefe — und genau das war der
/// erste Entwurf dieser Aufzeichnung: `casts.crucible-skin = 1` bei
/// `absorbed = healed = converted = 0`, weil der gewaehlte Boss nie austeilt.
///
/// Die fuenf Groessen hier sind die, welche die beiden Zauber bewegen. Jede
/// steht ungleich null, und jede wird gegen die Aufzeichnung geprueft: Der
/// Simulator muss dieselbe Zahl erreichen wie das Spiel, nicht irgendeine.
#[test]
fn die_wirkungen_stehen_auf_beiden_seiten_gleich_und_ungleich_null() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        boss,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);

    let erwartet = |feld: &str| -> u32 {
        u32::try_from(
            json["expected"][feld]
                .as_u64()
                .unwrap_or_else(|| panic!("{feld} fehlt in expected")),
        )
        .expect("zu gross")
    };

    for (feld, gemessen) in [
        ("damage", ergebnis.damage),
        ("damageTaken", ergebnis.damage_taken),
        ("absorbed", ergebnis.shield_absorbed),
        ("healed", ergebnis.healed),
        ("converted", ergebnis.converted),
    ] {
        assert!(
            erwartet(feld) > 0,
            "{feld} steht in der Aufzeichnung auf null — dann vergleicht dieser Fall zwei Nullen"
        );
        assert_eq!(
            gemessen,
            erwartet(feld),
            "{feld}: Simulator und Spiel kommen auf verschiedene Werte"
        );
    }
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    // **Der Boss kommt aus der Aufzeichnung, nicht aus der Vorgabe.**
    // Gemessen: Mit `FightConfig::default()` lief dieser Fall gegen
    // `first-crucible`, der nie austeilt — beide Seiten verglichen dann eine
    // Wirkung von null gegen eine Wirkung von null und waren bitgleich.
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        boss,
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
