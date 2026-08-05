//! Die letzten drei Bausteine, Takt für Takt (Issue #240, AC6).
//!
//! **Alle drei in EINEM Lauf.** Die Schwächung greift bei 8 s und wird von
//! QUENCH entfernt, die Wirkzeit der Zeitleiste schrumpft mit jedem der sieben
//! Zyklen, und die Reaktion sinkt, bis CRUST sie hebt. Getrennt geprüft würde
//! nicht zeigen, dass sie einander nicht ins Gehege kommen — und genau dort
//! läuft so etwas auseinander.
//!
//! Zwei Fallen, die beim Bauen dieses Prüffalls zugeschlagen haben und die er
//! deshalb ausdrücklich abdeckt:
//!
//! 1. **Die Reaktion muss im Konstruktor stehen, nicht erst in `begin_fight`.**
//!    Die Wiedergabe ruft es nicht. Im Spiel war sie zuerst nur dort gesetzt —
//!    die erste Vergleichsdatei trug `reactionFailed: true` und eine Reihe aus
//!    lauter Nullen, weil sie im ersten Schritt unter null fiel.
//! 2. **Das Kippen wird in `i64` gerechnet, nicht in `u32`.** Ein
//!    `saturating_sub` bliebe bei 0 stehen, und die Bedingung „unter
//!    `fail_below`" wäre bei einer Schwelle von 0 nie erfüllt. Der Datensatz
//!    dieses Bosses hat genau diese Schwelle.
//!
//! Was hier NICHT vorkommt, ist das Kippen selbst: CRUST hat 6 s Abklingzeit,
//! wer stabilisiert, hält die Reaktion. Ein Lauf, der beides zeigte, müsste
//! den Spieler mittendrin unmotiviert aufhören lassen. Der Fall steht deshalb
//! unten als eigener Prüffall, mit derselben gerechneten Zahl wie im Spiel
//! (`tests/unit/boss-trial-blocks.test.ts`, AC1): 51000 ms.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const TRIAL_JSON: &str = include_str!("../../tests/parity/recordings/trial-blocks.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(TRIAL_JSON).expect("trial-blocks.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
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
                slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(spell_id).unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt")),
            }
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick")).expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("ticks zu gross"),
        bindings,
        presses,
    }
}

fn replay(json: &Value) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
}

fn zahlen(json: &Value, feld: &str) -> Vec<u32> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect()
}

fn ganze(json: &Value, feld: &str) -> Vec<i64> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| v.as_i64().expect("ganze Zahl"))
        .collect()
}

#[test]
fn der_pruef_boss_traegt_alle_drei_bausteine() {
    let boss = boss_by_id("test-crucible-trial").expect(
        "Der Boss aus data/bosses/test-crucible-trial.json wurde nicht gefunden. \
         Liest build.rs das Verzeichnis noch?",
    );
    let stab = boss.stabilise.expect("keine kippende Reaktion");
    assert_eq!(stab.start_value, 1000);
    assert_eq!(stab.drop_amount, 20);
    assert_eq!(stab.fail_below, 0);
    let esc = boss.escalation.expect("keine Eskalation");
    assert_eq!(esc.cast_time_shrink_ms, 500);
    assert_eq!(esc.min_cast_time_ms, 1000);
    let aff = boss.affliction.expect("keine Schwaechung");
    assert_eq!(aff.at_ms, 8_000);
    // **Mehrere Wege, nicht einer** (11.2, und 11.3 verbietet, eine Schule zu
    // verlangen). Heilung steht bewusst NICHT darin — das ist der Baustein.
    assert!(aff.cleared_by.len() >= 2);
    assert!(!aff.cleared_by.contains(&"salve"));
}

#[test]
fn ac6_die_pruefsumme_stimmt_takt_fuer_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet = zahlen(&json, "checksums");
    let reaktion = zahlen(&json, "reaction");
    let geschwaecht = zahlen(&json, "afflicted");
    let landet = ganze(&json, "castLandsAtMs");

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    assert!(
        erwartet.len() > 8_000,
        "der Lauf ist zu kurz, um alle drei Bausteine zu zeigen"
    );

    for (tick, (links, rechts)) in result.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(
            links,
            rechts,
            "erste Abweichung im Takt {tick} ({} ms). Im Spiel stand dort: \
             Reaktion {}, Schwaechung {}, Boss-Zauber trifft bei {}",
            tick as u32 * TICK_MS,
            reaktion.get(tick).copied().unwrap_or(0),
            if geschwaecht.get(tick) == Some(&1) {
                "LIEGT"
            } else {
                "weg"
            },
            landet.get(tick).copied().unwrap_or(-1),
        );
    }

    let expected = &json["expected"];
    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"].as_u64().expect("finalChecksum")
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        expected["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum")
    );
}

#[test]
fn ac6_die_drei_spuren_stimmen_einzeln_ueberein() {
    // **Die Pruefsumme allein genuegt nicht.** Sie sagt, DASS ein Takt anders
    // ist. Welcher der drei Bausteine es war, sagt erst der Vergleich der
    // Spuren — und der ist billig, weil sie ohnehin mitlaufen.
    let json = data();
    let result = replay(&json);

    assert_eq!(result.reaction, zahlen(&json, "reaction"), "Reaktion (B6)");
    assert_eq!(
        result.afflicted,
        zahlen(&json, "afflicted"),
        "Schwaechung (B8)"
    );
    assert_eq!(
        result.cast_lands_at,
        ganze(&json, "castLandsAtMs"),
        "Trefferzeitpunkt des Boss-Zaubers (B7)"
    );
}

#[test]
fn ac6_die_endstaende_stimmen_ueberein() {
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    assert_eq!(
        u64::from(result.final_reaction),
        expected["finalReaction"].as_u64().expect("finalReaction"),
        "verschiedener Stand der Reaktion"
    );
    assert_eq!(
        u64::from(result.stabilised_count),
        expected["stabilisedCount"]
            .as_u64()
            .expect("stabilisedCount"),
        "verschieden oft stabilisiert"
    );
    assert_eq!(
        u64::from(result.afflictions_cleared),
        expected["afflictionsCleared"]
            .as_u64()
            .expect("afflictionsCleared"),
        "verschieden oft entfernt"
    );
    assert_eq!(
        u64::from(result.cycle),
        expected["cycle"].as_u64().expect("cycle"),
        "verschieden viele Zyklen"
    );
    assert_eq!(
        result.reaction_failed,
        expected["reactionFailed"]
            .as_bool()
            .expect("reactionFailed")
    );

    // **Ohne diese drei prueft der Fall nichts.** Ein Lauf, in dem nie
    // stabilisiert und nie entfernt wurde, waere mit lauter Nullen auf beiden
    // Seiten gleich — und genau so sah die erste Fassung dieser Datei aus.
    assert!(
        result.stabilised_count > 0,
        "im Lauf wurde nie stabilisiert"
    );
    assert!(
        result.afflictions_cleared > 0,
        "im Lauf wurde die Schwaechung nie entfernt"
    );
    assert!(result.cycle > 1, "die Zeitleiste hat sich nicht wiederholt");
}

#[test]
fn ac1_die_reaktion_kippt_zur_selben_zeit_wie_im_spiel() {
    // **Der Fall, der nicht in die Aufzeichnung passt** — er verlangt, dass
    // der Spieler NICHTS tut. Die Zahl ist gerechnet und steht genauso im
    // Prueffall des Spiels: 1000 bei 20 je Sekunde, der einundfuenfzigste
    // Schritt faellt unter null.
    //
    // Er deckt die u32-Falle ab: Mit `saturating_sub` bliebe die Reaktion bei
    // 0 stehen und kippte NIE, weil `fail_below` hier 0 ist.
    let boss = boss_by_id("test-crucible-trial").expect("test-crucible-trial");
    let stab = boss.stabilise.expect("keine Reaktion");
    let erwartet = (stab.start_value / stab.drop_amount + 1) * stab.drop_per_step_ms;
    assert_eq!(erwartet, 51_000);

    let leer = Recording {
        name: "kein-eingriff".to_owned(),
        seed: 0,
        ticks: 6_000,
        bindings: Vec::new(),
        presses: Vec::new(),
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(leer.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let result = fight.replay(&leer, false);

    assert!(result.reaction_failed, "ohne Eingriff muss sie kippen");
    assert_eq!(
        result.died_at_ms,
        Some(erwartet),
        "sie kippt zu einer anderen Zeit als im Spiel"
    );
    assert_eq!(result.final_reaction, stab.fail_below);
}

#[test]
fn ac5_ein_boss_ohne_die_felder_bleibt_unberuehrt() {
    // Der Gegenfall zum ganzen Issue: `first-crucible` traegt keinen der drei
    // Bausteine, und alle drei Groessen muessen ruhen.
    let boss = boss_by_id("first-crucible").expect("first-crucible");
    assert!(boss.stabilise.is_none());
    assert!(boss.escalation.is_none());
    assert!(boss.affliction.is_none());
}
