//! Die wiederholte Zeitleiste rechnet auf beiden Seiten gleich (Issue #174).
//!
//! # Warum mindestens zwei Zyklen
//!
//! Eine Wiederholungs-Regel, die nur eine Seite kennt, bricht die Paritaet
//! **erst nach dem ersten Zyklus**. Ein Lauf, der einen Zyklus abdeckt, liefe
//! gruen — und die Abweichung fiele erst auf, wenn jemand laenger uebt.
//!
//! Deshalb steht hier nicht nur die Pruefsumme je Takt, sondern eine harte
//! Untergrenze fuer die Zahl der durchlaufenen Zyklen. Faellt sie unter zwei,
//! bricht der Fall, statt still weniger zu pruefen als sein Name verspricht.
//!
//! # Der Zaehler ist Fortschritt, nicht Platz in der Liste
//!
//! Bei wiederholter Zeitleiste laeuft `timeline_index` ueber die Laenge der
//! Liste hinaus. Das geht in die Zustands-Pruefsumme ein — beide Seiten
//! muessen also nicht nur dieselben Eintraege ausloesen, sondern auch gleich
//! zaehlen.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/timeline-loop.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("timeline-loop.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn zahlen(json: &Value, feld: &str) -> Vec<u32> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect()
}

fn nachspielen(json: &Value) -> (Fight, rota_sim::ReplayResult) {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| Binding {
            slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
            spell: index_of(b["spellId"].as_str().expect("spellId")).expect("Zauber unbekannt"),
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
    let ergebnis = fight.replay(&recording, true);
    (fight, ergebnis)
}

#[test]
fn ac5_der_simulator_kennt_die_wiederholung_ueberhaupt() {
    // **Die Ursache zuerst.** Kennt der Simulator das Feld nicht, waere alles
    // Weitere ein Vergleich zweier Laeufe, die beide nach dem ersten Zyklus
    // aufhoeren — und die stimmen dann natuerlich ueberein.
    let boss = boss_by_id("test-metronome").expect("test-metronome fehlt");
    assert!(
        boss.timeline_loop_ms.is_some(),
        "der Prueff-Boss hat keine Wiederholung — dann prueft diese Datei nichts"
    );
    assert!(!boss.timeline.is_empty());
}

#[test]
fn ac5_der_lauf_deckt_mindestens_zwei_zyklen_ab() {
    // **Ohne diesen Fall belegt die Pruefsumme nichts ueber die Wiederholung.**
    // Eine Regel, die nur eine Seite kennt, laeuft im ERSTEN Zyklus noch gleich.
    let json = data();
    let (fight, _) = nachspielen(&json);
    let boss = boss_by_id("test-metronome").expect("test-metronome");
    let laenge = boss.timeline.len();

    let zyklen = fight.timeline_index() / laenge;
    assert!(
        zyklen >= 2,
        "der Lauf hat nur {zyklen} Zyklen durchlaufen ({} Eintraege bei einer \
         Liste von {laenge}) — zu wenig, um die Wiederholung zu belegen",
        fight.timeline_index()
    );
}

#[test]
fn ac5_die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let (_, ergebnis) = nachspielen(&json);
    let soll = zahlen(&json, "checksums");

    assert!(!soll.is_empty());
    if let Some(d) = first_divergence(&soll, &ergebnis.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} ({} ms)\n  \
             Spiel: {}   Simulator: {}\n  gleiche Takte davor: {}\n  \
             Liegt der Takt hinter dem ersten Zyklus, kennt eine Seite die \
             Wiederholung nicht.",
            d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
        );
    }
    assert_eq!(ergebnis.checksums.len(), soll.len());
}

#[test]
fn ac5_der_fortschrittszaehler_laeuft_auf_beiden_seiten_gleich() {
    // Die Pruefsumme enthaelt den Zaehler, dieser Fall zeigt ihn einzeln: Wo
    // die Spuren auseinandergehen, steht der Grund — nicht nur, DASS etwas
    // abweicht.
    let json = data();
    let (_, ergebnis) = nachspielen(&json);
    let soll = zahlen(&json, "timelineIndex");
    let ist: Vec<u32> = ergebnis.timeline_index.clone();

    assert_eq!(ist.len(), soll.len(), "verschieden viele Takte");
    for (tick, (a, b)) in soll.iter().zip(ist.iter()).enumerate() {
        assert_eq!(
            a,
            b,
            "Takt {tick} ({} ms): Spiel {a}, Simulator {b}",
            tick as u32 * TICK_MS
        );
    }
    // Und er ist wirklich ueber die Liste hinausgelaufen.
    let laenge = boss_by_id("test-metronome").expect("boss").timeline.len() as u32;
    assert!(
        *ist.last().expect("Takte") > laenge,
        "der Zaehler blieb bei {} und damit innerhalb der Liste ({laenge}) — \
         dann gab es gar keine Wiederholung",
        ist.last().expect("Takte")
    );
}

#[test]
fn ac5_die_kennzahlen_stimmen_ebenfalls() {
    // Die Gegenprobe zur Pruefsumme: Nicht nur der Zustand stimmt ueberein,
    // sondern auch das, was er bewirkt hat.
    let json = data();
    let (fight, ergebnis) = nachspielen(&json);
    let erwartet = &json["expected"];

    for (feld, gemessen) in [
        ("damageDealt", u64::from(fight.damage())),
        ("damageTaken", u64::from(fight.damage_taken())),
        ("interruptsLanded", u64::from(fight.interrupts_landed())),
        ("interruptsMissed", u64::from(ergebnis.interrupts_missed)),
        ("finalTimelineIndex", fight.timeline_index() as u64),
        ("finalPlayerHp", u64::from(fight.player_hp())),
    ] {
        assert_eq!(
            gemessen,
            erwartet[feld]
                .as_u64()
                .unwrap_or_else(|| panic!("{feld} fehlt")),
            "{feld} weicht ab"
        );
    }
}

#[test]
fn ac5_ohne_wiederholung_endet_die_zeitleiste_wie_bisher() {
    // **Der Gegenfall, und er ist nicht verhandelbar:** Die Wiederholung gilt
    // nur fuer Bosse, die sie ausweisen. Wuerde sie fuer alle gelten, aenderte
    // sich das Verhalten jedes ausgelieferten Bosses — und die Referenz unten
    // waere nicht das Einzige, was sich bewegt.
    let boss = boss_by_id("test-warden").expect("test-warden");
    assert!(
        boss.timeline_loop_ms.is_none(),
        "ein ausgelieferter Prueff-Boss hat unerwartet eine Wiederholung"
    );
    let laenge = boss.timeline.len();

    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(120_000),
        ..FightConfig::default()
    });
    let bind = index_of("bind").expect("bind");
    let presses = (0..12_000).map(|tick| Press { tick, slot: 0 }).collect();
    let recording = Recording {
        name: "ohne-wiederholung".to_owned(),
        seed: 0,
        ticks: 12_000,
        bindings: vec![Binding {
            slot: 0,
            spell: bind,
        }],
        presses,
    };
    fight.replay(&recording, false);

    assert_eq!(
        fight.timeline_index(),
        laenge,
        "der Zaehler ist ueber die Liste hinausgelaufen, obwohl dieser Boss \
         sich nicht wiederholt"
    );
}

#[test]
fn ac6_die_referenz_rechnung_ist_unberuehrt() {
    let mut fight = Fight::new(FightConfig::default());
    fn gute_rotation(f: &Fight) -> Option<usize> {
        let kindle = index_of("kindle").expect("kindle");
        let smoulder = index_of("smoulder").expect("smoulder");
        let pyre = index_of("pyre").expect("pyre");
        if f.dot_remaining_ms(smoulder) < 1500 && f.is_ready(smoulder) {
            Some(smoulder)
        } else if f.is_ready(pyre) && f.resource() >= 15 {
            Some(pyre)
        } else if f.is_ready(kindle) {
            Some(kindle)
        } else {
            None
        }
    }
    let ergebnis = fight.run(gute_rotation as fn(&Fight) -> Option<usize>);
    assert_eq!(
        ergebnis.damage, 4240,
        "die Referenz-Rechnung hat sich bewegt"
    );
    assert_eq!(ergebnis.killed_at_ms, Some(40_510));
}
