//! Die NEUEN Zauber der COAGULATION rechnen auf beiden Seiten gleich
//! (Issue #481).
//!
//! # Warum eine eigene Aufzeichnung
//!
//! `extend-effects` wirkt bewusst je EINEN CRUST und EINEN HARDEN, damit die
//! Schicht innerhalb des Laufs ablaeuft — ihr Kopfkommentar sagt es
//! ausdruecklich. Ein dritter Schild-Zauber koennte genau das kippen. Eine
//! fremde Messung fasst man nicht an, um die eigene unterzubringen.
//!
//! # Warum EIN LAUF JE ZAUBER — und nicht alle in einem
//!
//! Der erste Entwurf wirkte alle neuen Zauber in einem Durchgang. Das ging
//! zwei Zauber lang gut und brach beim dritten:
//!
//! > `healed` ist eine SUMME ueber alle Heiler. Zwei Heiler in einem Lauf sind
//! > kommunizierende Roehren — heilt der eine weniger, holt der andere den
//! > Rest, bis der Deckel erreicht ist. Dann steht dieselbe Zahl da, egal was
//! > in den Datensaetzen steht.
//!
//! Gemessen am 2026-08-09: Mit CRUCIBLE SKIN, POULTICE und MEND in einem Lauf
//! bewegte **keine** der Zahlen mehr eine Groesse — weder MENDs
//! `perMissingPercent` noch POULTICEs `amount`. Die Aufzeichnung fuehrte die
//! Werte mit, ohne sie zu pruefen.
//!
//! Mit einem Lauf je Zauber ist `healed` eindeutig einem Zauber zuzuordnen.
//! Dasselbe Muster nutzt `parity_coagulation.rs` fuer die drei ALTEN Wege.
//!
//! Der Lauf `none` ist die Messlatte: Er zeigt, dass der Boss ueberhaupt weh
//! tut. Ohne ihn koennten alle Zauber wirkungslos sein, ohne dass es auffaellt.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/coagulation-neu.json");

/// Die Zauber, fuer die diese Aufzeichnung je einen Lauf fuehrt.
///
/// **Die Liste ist abgeschrieben, und das ist hier richtig.** Sie leitet sich
/// NICHT aus der Datei her: Eine Liste, die sich aus der Umsetzung ergibt,
/// waere auch dann gruen, wenn ein Lauf verschwaende. Wer einen der fuenf
/// neuen Zauber baut, traegt ihn hier ein — und bekommt einen roten Fall,
/// solange die Aufzeichnung ihn nicht wirkt.
const NEUE_ZAUBER: [&str; 3] = ["crucible-skin", "poultice", "mend"];

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("coagulation-neu.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

fn lauf<'a>(json: &'a Value, weg: &str) -> &'a Value {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|r| r["way"].as_str() == Some(weg))
        .unwrap_or_else(|| panic!("kein Lauf fuer Weg {weg}"))
}

/// Spielt einen Lauf im Simulator nach.
///
/// Die Druecke stehen in der Datei, statt hier aus der Regel nachgebaut zu
/// werden. Eine nachgebaute Regel, die minimal abweicht, ergaebe eine
/// Abweichung, die wie ein Rechenfehler aussieht.
fn replay(json: &Value, weg: &str) -> rota_sim::ReplayResult {
    let eintrag = lauf(json, weg);
    // **Der Boss kommt aus der Aufzeichnung, nicht aus der Vorgabe.** Gemessen:
    // Mit `FightConfig::default()` lief dieser Fall gegen `first-crucible`, der
    // nie austeilt — beide Seiten verglichen dann eine Wirkung von null gegen
    // eine Wirkung von null und waren bitgleich.
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(eintrag["ticks"].as_u64().expect("ticks fehlen")).expect("zu gross");

    let bindings = eintrag["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| {
            let id = b["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(b["slot"].as_u64().expect("slot fehlt")).expect("slot > 255"),
                spell: index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt")),
            }
        })
        .collect();
    let presses = eintrag["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick fehlt")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot fehlt")).expect("slot > 255"),
        })
        .collect();

    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(
        &Recording {
            name: format!("coagulation-neu-{weg}"),
            seed: 0,
            ticks,
            bindings,
            presses,
        },
        true,
    )
}

fn erwartet(json: &Value, weg: &str, feld: &str) -> u32 {
    u32::try_from(
        lauf(json, weg)["expected"][feld]
            .as_u64()
            .unwrap_or_else(|| panic!("{weg}/{feld} fehlt in expected")),
    )
    .expect("zu gross")
}

/// **Die Zusicherung vor jeder Messung** — je Zauber einzeln.
///
/// Eine Aufzeichnung, in der ein Zauber nicht vorkommt, belegt fuer ihn
/// nichts. Ein gemeinsamer Fall waere schon gruen, wenn nur einer von ihnen
/// gedrueckt wird.
#[test]
fn jeder_neue_zauber_wird_in_seinem_lauf_wirklich_gewirkt() {
    let json = data();

    for zauber in NEUE_ZAUBER {
        let eintrag = lauf(&json, zauber);
        let index = index_of(zauber).unwrap_or_else(|| panic!("{zauber} fehlt im Simulator"));
        let slot = eintrag["bindings"]
            .as_array()
            .expect("bindings fehlen")
            .iter()
            .find(|b| b["spellId"].as_str() == Some(zauber))
            .and_then(|b| b["slot"].as_u64())
            .unwrap_or_else(|| panic!("{zauber} liegt auf keinem Platz"));
        assert!(
            index_of(zauber).is_some(),
            "{zauber} kennt der Simulator nicht"
        );
        assert!(
            eintrag["presses"]
                .as_array()
                .expect("presses fehlen")
                .iter()
                .any(|p| p["slot"].as_u64() == Some(slot)),
            "{zauber} wird in seinem Lauf nie gedrueckt — dann belegt er nichts"
        );
        assert!(
            erwartet(&json, zauber, "casts") > 0,
            "der Lauf {zauber} zaehlt keinen einzigen Wurf"
        );
        // Der Index wird gebraucht, damit ein umbenannter Zauber hier auffaellt
        // und nicht erst in der Pruefsumme.
        assert!(index < 255, "{zauber} hat einen unmoeglichen Index");
    }
}

/// **Gewirkt heisst nicht gewirkt HAT.**
///
/// Der Fall darueber belegt, dass die Zauber gedrueckt werden. Er waere auch
/// dann gruen, wenn jeder von ihnen ins Leere liefe — und genau das war der
/// erste Entwurf dieser Aufzeichnung: `casts.crucible-skin = 1` bei
/// `absorbed = healed = converted = 0`, weil der gewaehlte Boss nie austeilt.
#[test]
fn jeder_zauber_bewegt_in_seinem_lauf_die_groesse_seines_weges() {
    let json = data();

    // Was jeder Zauber bewegen MUSS. Die Zuordnung ist die Aussage: CRUCIBLE
    // SKIN traegt alle drei Wege, POULTICE und MEND nur die Heilung.
    let pflicht: [(&str, &[&str]); 3] = [
        ("crucible-skin", &["absorbed", "healed", "converted"]),
        ("poultice", &["healed"]),
        ("mend", &["healed"]),
    ];

    for (zauber, groessen) in pflicht {
        for feld in groessen {
            assert!(
                erwartet(&json, zauber, feld) > 0,
                "{zauber}: {feld} steht auf null — dann vergleicht dieser Fall zwei Nullen"
            );
            // Und dieselbe Groesse steht im Lauf OHNE Zauber auf null. Ohne
            // diese Haelfte koennte sie von etwas anderem stammen.
            if *feld != "healed" || zauber == "crucible-skin" {
                assert_eq!(
                    erwartet(&json, "none", feld),
                    0,
                    "{feld} bewegt sich auch ohne {zauber} — dann misst der Lauf nicht ihn"
                );
            }
        }
    }
}

/// Die Messlatte: Ohne Verteidigung tut der Boss weh.
#[test]
fn der_lauf_ohne_zauber_zeigt_dass_der_boss_ueberhaupt_austeilt() {
    let json = data();
    assert!(
        erwartet(&json, "none", "damageTaken") > 0,
        "der Boss teilt gar nicht aus — dann belegt kein Lauf etwas"
    );
    for zauber in NEUE_ZAUBER {
        assert!(
            erwartet(&json, zauber, "playerHp") > erwartet(&json, "none", "playerHp"),
            "{zauber} laesst den Spieler nicht besser dastehen als gar nichts"
        );
    }
}

/// Die Groessen stimmen auf beiden Seiten — je Lauf.
#[test]
fn simulator_und_spiel_kommen_je_lauf_auf_dieselben_zahlen() {
    let json = data();

    for weg in ["none", "crucible-skin", "poultice", "mend"] {
        let ergebnis = replay(&json, weg);
        for (feld, gemessen) in [
            ("damage", ergebnis.damage),
            ("damageTaken", ergebnis.damage_taken),
            ("absorbed", ergebnis.shield_absorbed),
            ("healed", ergebnis.healed),
            ("converted", ergebnis.converted),
        ] {
            assert_eq!(
                gemessen,
                erwartet(&json, weg, feld),
                "{weg}/{feld}: Simulator und Spiel kommen auf verschiedene Werte"
            );
        }
    }
}

/// Takt fuer Takt, nicht nur am Ende — je Lauf.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt_jedes_laufs() {
    let json = data();

    for weg in ["none", "crucible-skin", "poultice", "mend"] {
        let ergebnis = replay(&json, weg);
        let erwartete: Vec<u32> = lauf(&json, weg)["checksums"]
            .as_array()
            .expect("checksums fehlen")
            .iter()
            .map(|v| {
                u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross")
            })
            .collect();

        // `first_divergence` nennt den Takt statt nur „ungleich" — bei
        // zweitausend Eintraegen ist das der Unterschied zwischen einem Hinweis
        // und einer Suche.
        assert_eq!(
            first_divergence(&erwartete, &ergebnis.checksums, TICK_MS),
            None,
            "Simulator und Spiel laufen im Lauf {weg} auseinander"
        );
    }
}
