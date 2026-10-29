//! Die Stufen-Tabelle im Paritaets-Weg (Issue #1177).
//!
//! # Was hier verglichen wird
//!
//! Spiel und Simulator leiten Heroisch und Mythisch GETRENNT ab: das Spiel zur
//! Laufzeit (`deriveStage` in `src/core/difficulty.ts`), der Simulator zur
//! Bauzeit (`leite_stufe_ab` in `sim/build.rs`). Dieselbe Regel in zwei
//! Sprachen ist genau die Lage, in der zwei Seiten still auseinanderlaufen.
//!
//! `difficulty-stages.json` traegt fuer JEDEN Boss mit Stufen beide Fassungen
//! Feld fuer Feld, so wie das Spiel sie liest. Dieser Fall haelt jedes Feld
//! gegen [`boss_at`]. Die Kampf-Aufzeichnungen (`parity_difficulty_boss_1_1.rs`)
//! pruefen einen Boss im Lauf; dieser hier alle, ohne Lauf.

use serde_json::Value;

use rota_sim::data::{boss_at, BossHealTrigger, TimelineKind, BOSS_STAGES, STAGED_BOSS_COUNT};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/difficulty-stages.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("difficulty-stages.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

fn zahl(node: &Value, key: &str, wer: &str) -> u64 {
    node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{wer}: {key} fehlt oder ist keine Zahl"))
}

fn stufen() -> Vec<Value> {
    data()["stages"].as_array().expect("stages fehlt").clone()
}

/// **Die Zusicherung vor dem Vergleich**: Es gibt etwas zu vergleichen, und
/// beide Seiten fuehren dieselbe MENGE.
#[test]
fn beide_seiten_kennen_dieselben_stufen() {
    let liste = stufen();
    // 21 Bosse auf je zwei Stufen ueber Normal. Gezaehlt an der AUFZEICHNUNG,
    // nicht an der Konstanten: `STAGED_BOSS_COUNT >= 21` waere zur
    // Uebersetzungszeit entschieden und prueft nichts, was sich noch aendern
    // koennte (clippy lehnt es mit -D warnings ab).
    let mut kennungen: Vec<&str> = liste.iter().filter_map(|e| e["id"].as_str()).collect();
    kennungen.sort_unstable();
    kennungen.dedup();
    assert!(
        kennungen.len() >= 21,
        "die Aufzeichnung fuehrt nur {} Bosse mit Stufen",
        kennungen.len()
    );
    assert_eq!(
        liste.len(),
        STAGED_BOSS_COUNT * 2,
        "das Spiel fuehrt {} Stufen, der Simulator {}",
        liste.len(),
        STAGED_BOSS_COUNT * 2
    );
    for stufen in &BOSS_STAGES {
        let id = stufen[0].id;
        for (index, boss) in stufen.iter().enumerate() {
            let erwartet = index as u32 + 1;
            assert_eq!(
                boss.id, id,
                "die Fassungen eines Bosses tragen eine Kennung"
            );
            assert_eq!(boss.difficulty, erwartet, "{id}: Stufe {erwartet}");
            assert!(
                liste.iter().any(|e| e["id"].as_str() == Some(id)
                    && e["difficulty"].as_u64() == Some(u64::from(erwartet))),
                "{id} Stufe {erwartet} fehlt in der Aufzeichnung des Spiels"
            );
        }
    }
}

#[test]
fn das_spiel_meldet_kein_problem() {
    for eintrag in stufen() {
        let probleme = eintrag["problems"].as_array().expect("problems fehlt");
        assert!(
            probleme.is_empty(),
            "{} Stufe {}: {probleme:?}",
            eintrag["id"],
            eintrag["difficulty"]
        );
    }
}

#[test]
fn jede_stufe_ist_auf_beiden_seiten_derselbe_datensatz() {
    for e in stufen() {
        let id = e["id"].as_str().expect("id fehlt");
        let stufe = u32::try_from(zahl(&e, "difficulty", id)).expect("Stufe zu gross");
        let wer = format!("{id} Stufe {stufe}");
        let boss = boss_at(id, stufe).unwrap_or_else(|| panic!("{wer} fehlt im Simulator"));

        assert_eq!(u64::from(boss.hp), zahl(&e, "hp", &wer), "{wer}: hp");
        assert_eq!(
            u64::from(boss.overpressure_ms),
            zahl(&e, "overpressureMs", &wer),
            "{wer}: overpressureMs"
        );
        assert_eq!(
            boss.timeline_loop_ms.map(u64::from),
            e["timelineLoopMs"].as_u64(),
            "{wer}: timelineLoopMs"
        );

        // ---- Die Zeitleiste, Eintrag fuer Eintrag ----
        let leiste = e["timeline"].as_array().expect("timeline fehlt");
        assert_eq!(
            boss.timeline.len(),
            leiste.len(),
            "{wer}: Laenge der Zeitleiste"
        );
        for (eintrag, spiel) in boss.timeline.iter().zip(leiste) {
            let w = format!("{wer} bei {} ms", eintrag.at_ms);
            assert_eq!(
                u64::from(eintrag.at_ms),
                zahl(spiel, "atMs", &w),
                "{w}: atMs"
            );
            assert_eq!(
                Some(eintrag.mechanic),
                spiel["mechanic"].as_str(),
                "{w}: mechanic"
            );
            let art = match eintrag.kind {
                TimelineKind::Strike => "strike",
                TimelineKind::InterruptibleCast => "interruptibleCast",
            };
            assert_eq!(Some(art), spiel["mechanic"].as_str(), "{w}: Art");
            assert_eq!(Some(eintrag.name), spiel["name"].as_str(), "{w}: name");
            assert_eq!(
                u64::from(eintrag.damage),
                zahl(spiel, "damage", &w),
                "{w}: damage"
            );
            assert_eq!(
                u64::from(eintrag.cast_time_ms),
                zahl(spiel, "castTimeMs", &w),
                "{w}: castTimeMs"
            );
            assert_eq!(
                Some(eintrag.interrupts_player),
                spiel["interruptsPlayer"].as_bool(),
                "{w}: interruptsPlayer"
            );
            // `u32::MAX` heisst „an keine Phase gebunden“ (`TimelineEntry::phase`),
            // im Spiel fehlt das Feld — die Aufzeichnung schreibt dann `null`.
            let gebunden = (eintrag.phase != u32::MAX).then(|| u64::from(eintrag.phase));
            assert_eq!(gebunden, spiel["phase"].as_u64(), "{w}: phase");
            assert_eq!(
                gebunden.is_none(),
                spiel["phase"].is_null(),
                "{w}: phase gebunden oder nicht"
            );
        }

        // ---- Die Bausteine: da oder nicht da, und mit denselben Zahlen ----
        match (&boss.energy, e["energy"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["energy"];
                assert_eq!(u64::from(x.per_step_ms), zahl(s, "perStepMs", &wer));
                assert_eq!(u64::from(x.amount), zahl(s, "amount", &wer));
                assert_eq!(u64::from(x.threshold), zahl(s, "threshold", &wer));
                assert_eq!(u64::from(x.reset_to), zahl(s, "resetTo", &wer));
                assert_eq!(Some(x.on_trigger.name), s["onTrigger"]["name"].as_str());
                assert_eq!(
                    u64::from(x.on_trigger.damage),
                    zahl(&s["onTrigger"], "damage", &wer)
                );
            }
            (rust, spiel_leer) => panic!("{wer}: energy {rust:?} gegen leer={spiel_leer}"),
        }
        match (&boss.doom, e["doom"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["doom"];
                assert_eq!(u64::from(x.per_step_ms), zahl(s, "perStepMs", &wer));
                assert_eq!(u64::from(x.amount), zahl(s, "amount", &wer));
                assert_eq!(u64::from(x.threshold), zahl(s, "threshold", &wer));
                assert_eq!(u64::from(x.clear_amount), zahl(s, "clearAmount", &wer));
                assert_eq!(u64::from(x.clear_charges), zahl(s, "clearCharges", &wer));
            }
            (rust, spiel_leer) => panic!("{wer}: doom {rust:?} gegen leer={spiel_leer}"),
        }
        match (&boss.stacks, e["stacks"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["stacks"];
                assert_eq!(u64::from(x.per_step_ms), zahl(s, "perStepMs", &wer));
                assert_eq!(u64::from(x.base_damage), zahl(s, "baseDamage", &wer));
                assert_eq!(u64::from(x.factor), zahl(s, "factor", &wer));
                assert_eq!(u64::from(x.max_stacks), zahl(s, "maxStacks", &wer));
            }
            (rust, spiel_leer) => panic!("{wer}: stacks {rust:?} gegen leer={spiel_leer}"),
        }
        match (&boss.race, e["race"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["race"];
                assert_eq!(u64::from(x.at_ms), zahl(s, "atMs", &wer));
                assert_eq!(u64::from(x.window_ms), zahl(s, "windowMs", &wer));
                assert_eq!(u64::from(x.threshold), zahl(s, "threshold", &wer));
            }
            (rust, spiel_leer) => panic!("{wer}: race {rust:?} gegen leer={spiel_leer}"),
        }
        match (&boss.affliction, e["affliction"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["affliction"];
                assert_eq!(u64::from(x.at_ms), zahl(s, "atMs", &wer));
                assert_eq!(u64::from(x.per_step_ms), zahl(s, "perStepMs", &wer));
                assert_eq!(u64::from(x.damage), zahl(s, "damage", &wer));
                let spiel: Vec<&str> = s["clearedBy"]
                    .as_array()
                    .expect("clearedBy fehlt")
                    .iter()
                    .filter_map(Value::as_str)
                    .collect();
                assert_eq!(x.cleared_by, spiel.as_slice(), "{wer}: clearedBy");
            }
            (rust, spiel_leer) => panic!("{wer}: affliction {rust:?} gegen leer={spiel_leer}"),
        }
        match (&boss.escalation, e["escalation"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["escalation"];
                assert_eq!(
                    u64::from(x.cast_time_shrink_ms),
                    zahl(s, "castTimeShrinkMs", &wer)
                );
                assert_eq!(
                    u64::from(x.min_cast_time_ms),
                    zahl(s, "minCastTimeMs", &wer)
                );
            }
            (rust, spiel_leer) => panic!("{wer}: escalation {rust:?} gegen leer={spiel_leer}"),
        }
        match (&boss.boss_heal, e["bossHeal"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["bossHeal"];
                assert_eq!(u64::from(x.amount), zahl(s, "amount", &wer));
                let ausloeser = match x.on {
                    BossHealTrigger::Interrupt => "interrupt",
                    BossHealTrigger::Affliction => "affliction",
                };
                assert_eq!(Some(ausloeser), s["on"].as_str(), "{wer}: bossHeal.on");
            }
            (rust, spiel_leer) => panic!("{wer}: bossHeal {rust:?} gegen leer={spiel_leer}"),
        }
        match (&boss.runaway, e["runaway"].is_null()) {
            (None, true) => {}
            (Some(x), false) => {
                let s = &e["runaway"];
                assert_eq!(u64::from(x.per_step_ms), zahl(s, "perStepMs", &wer));
                assert_eq!(u64::from(x.amount), zahl(s, "amount", &wer));
            }
            (rust, spiel_leer) => panic!("{wer}: runaway {rust:?} gegen leer={spiel_leer}"),
        }
    }
}

/// **Die Gegenprobe**: Der Vergleich trifft wirklich die Stufe, nicht Normal.
///
/// Zoege `boss_at` fuer jede Stufe die Normal-Fassung heran, schluege der Fall
/// darueber schon beim ersten Lebenswert an — dieser Fall haelt fest, dass er
/// das KANN: Die Tabelle des Spiels traegt auf Heroisch mehr Leben als Normal.
#[test]
fn die_tabelle_unterscheidet_sich_von_normal() {
    let normal = boss_at("boss-1-1", 0).expect("boss-1-1 fehlt");
    let heroisch = stufen()
        .into_iter()
        .find(|e| e["id"].as_str() == Some("boss-1-1") && e["difficulty"].as_u64() == Some(1))
        .expect("boss-1-1 heroisch fehlt in der Aufzeichnung");
    assert!(
        zahl(&heroisch, "hp", "boss-1-1 heroisch") > u64::from(normal.hp),
        "Heroisch traegt nicht mehr Leben als Normal — der Vergleich trennte die Stufen nicht"
    );
    assert!(normal.affliction.is_none());
    assert!(!heroisch["affliction"].is_null());
}

/// Ein Pruef-Boss hat keine Stufen — und bekommt keine untergeschoben.
#[test]
fn ein_pruef_boss_hat_keine_heroische_fassung() {
    assert!(boss_at("test-doomsayer", 0).is_some());
    assert!(boss_at("test-doomsayer", 1).is_none());
    assert!(
        boss_at("boss-1-1", 3).is_none(),
        "eine vierte Stufe gibt es nicht"
    );
}
