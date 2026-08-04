//! Zähler mit tödlicher Schwelle und exponentieller Stapel (Issue #216, AC5).
//!
//! Beide Bausteine sind **passiv**: Sie hängen allein an der Uhr, nicht am
//! Spieler. Genau deshalb sind sie ein guter Prüfstein für die Parität — was
//! hier auseinanderläuft, läuft an der Zeitrechnung auseinander und nicht an
//! einer Entscheidung.
//!
//! **Der Lauf geht über den Tod hinaus.** Der Zähler tötet bei 20 s, die
//! Aufzeichnung läuft 30 s. Die zehn Sekunden danach sind kein Beiwerk: Sie
//! belegen, dass beide Seiten nach dem Tod dasselbe tun — nämlich nichts mehr.
//! Dort liefen zwei Umsetzungen sonst auseinander, ohne dass es auffiele, weil
//! der Kampf ja ohnehin vorbei ist.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, stack_damage};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Der aufgezeichnete Lauf, wie das Spiel ihn geschrieben hat.
const DOOM_JSON: &str = include_str!("../../tests/parity/recordings/doom-stacks.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(DOOM_JSON).expect("doom-stacks.json ist kein JSON");
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

#[test]
fn der_pruef_boss_traegt_beide_bausteine_und_stammt_aus_einer_datei() {
    let boss = boss_by_id("test-doomsayer").expect(
        "Der Boss aus data/bosses/test-doomsayer.json wurde nicht gefunden. \
         Liest build.rs das Verzeichnis noch?",
    );
    let doom = boss.doom.expect("test-doomsayer hat keinen Zaehler");
    assert_eq!(doom.threshold, 100);
    assert_eq!(doom.amount, 5);
    assert_eq!(doom.clear_charges, 2);

    let stacks = boss.stacks.expect("test-doomsayer hat keinen Stapel");
    assert_eq!(stacks.base_damage, 20);
    assert_eq!(stacks.factor, 3);
}

#[test]
fn ac5_zaehler_ladungen_und_stapel_verlaufen_auf_beiden_seiten_gleich() {
    let json = data();
    let result = replay(&json);

    assert_eq!(
        result.doom,
        zahlen(&json, "doom"),
        "Spiel und Simulator fuehren den Zaehler verschieden"
    );
    assert_eq!(
        result.doom_charges,
        zahlen(&json, "doomCharges"),
        "Spiel und Simulator fuehren die Ladungen verschieden"
    );
    assert_eq!(
        result.stacks,
        zahlen(&json, "stacks"),
        "Spiel und Simulator fuehren die Stapel verschieden"
    );
    assert_eq!(
        result.player_hp,
        zahlen(&json, "playerHp"),
        "Spiel und Simulator fuehren die Lebenspunkte verschieden"
    );
}

#[test]
fn ac5_die_pruefsumme_stimmt_takt_fuer_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet = zahlen(&json, "checksums");

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    // Der erste Unterschied, nicht nur die Tatsache eines Unterschieds: Bei
    // 3000 Takten ist das der Unterschied zwischen einer Minute und einem
    // Nachmittag.
    for (tick, (links, rechts)) in result.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(
            links,
            rechts,
            "erste Abweichung im Takt {tick} ({} ms): Zaehler {}, Ladungen {}, Stapel {}",
            tick as u32 * TICK_MS,
            result.doom[tick],
            result.doom_charges[tick],
            result.stacks[tick]
        );
    }

    let expected = &json["expected"];
    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"].as_u64().expect("finalChecksum")
    );
}

#[test]
fn ac1_der_zaehler_toetet_und_zwar_nicht_ueber_den_schaden() {
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    assert_eq!(expected["diedAtMs"].as_i64(), Some(20_000));
    assert_eq!(result.died_at_ms, Some(20_000));
    assert_eq!(*result.doom.last().expect("Spur leer"), 100);

    // **Der Unterschied, der den Baustein ausmacht.** Im Takt VOR dem Tod hatte
    // der Spieler noch Leben — der Zaehler nimmt keine Lebenspunkte weg, er
    // beendet den Lauf. Ohne diese Zusicherung bliebe der Fall auch dann gruen,
    // wenn in Wahrheit der Stapel ihn erschlagen haette.
    let vorher = result.player_hp[1999];
    assert!(
        vorher > 0,
        "der Spieler war schon vor dem Zaehler tot ({vorher} Lebenspunkte) — \
         dann prueft dieser Fall den falschen Baustein"
    );
    assert_eq!(result.player_hp[2000], 0);
}

#[test]
fn ac3_der_stapel_waechst_exponentiell_und_nicht_linear() {
    let boss = boss_by_id("test-doomsayer").expect("test-doomsayer");
    let def = boss.stacks.expect("kein Stapel");

    // Erst der Nachweis, dass beide Kurven bei DIESEN Werten ueberhaupt
    // auseinanderliegen — beim ERSTEN Stapel sind sie gleich.
    let linear: Vec<u32> = (1..=4).map(|n| def.base_damage * n).collect();
    let exponentiell: Vec<u32> = (1..=4).map(|n| stack_damage(def, n)).collect();
    assert_eq!(linear, vec![20, 40, 60, 80]);
    assert_eq!(exponentiell, vec![20, 60, 180, 540]);
    assert_eq!(exponentiell[0], linear[0]);
    for i in 1..linear.len() {
        assert_ne!(exponentiell[i], linear[i], "Stapel {}", i + 1);
    }

    // Und dieselbe Reihe wie im Spiel: `stackDamage` in src/core/damage.ts.
    let json = data();
    let erwartet_stapel = zahlen(&json, "stacks");
    let result = replay(&json);
    assert_eq!(result.stacks, erwartet_stapel);
    assert_eq!(
        *result.stacks.last().expect("Spur leer"),
        def.max_stacks,
        "der Stapel hat seine Obergrenze nicht erreicht — dann prueft der \
         Vergleich die Begrenzung nicht mit"
    );
}

#[test]
fn ac2_die_ruecksetz_handlung_senkt_und_ist_begrenzt() {
    // Die Aufzeichnung loest sie nicht aus — es gibt keine Taste dafuer, und
    // welche es sein soll, entscheidet E10. Geprueft wird deshalb hier, mit
    // DENSELBEN Zahlen wie im Spiel (tests/unit/boss-doom-stacks.test.ts).
    let boss = boss_by_id("test-doomsayer").expect("test-doomsayer");
    let doom = boss.doom.expect("kein Zaehler");
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(30_000),
        ..FightConfig::default()
    });

    assert!(fight.clear_doom(), "erste Ladung");
    assert!(fight.clear_doom(), "zweite Ladung");
    // Und die Begrenzung: Die dritte gibt es nicht, und sie nimmt sich auch
    // nichts.
    assert!(!fight.clear_doom(), "es darf nur zwei Ladungen geben");
    assert_eq!(doom.clear_charges, 2);
}
