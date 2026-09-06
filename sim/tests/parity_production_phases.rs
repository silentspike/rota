//! Production phase bindings, with the actual progression equipment (#1057).
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::gear::WornValues;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};
use serde_json::Value;

fn number(node: &Value, key: &str) -> u32 {
    u32::try_from(
        node[key]
            .as_u64()
            .unwrap_or_else(|| panic!("missing {key}")),
    )
    .expect("number out of range")
}

#[test]
fn production_bosses_match_every_tick_and_damage_taken() {
    let data: Value = serde_json::from_str(include_str!(
        "../../tests/parity/recordings/production-phase-bindings.json"
    ))
    .unwrap();
    assert_eq!(number(&data, "schema"), 1);
    let runs = data["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 4);
    let mut taken = Vec::new();
    for run in runs {
        assert_eq!(number(run, "tickMs"), TICK_MS);
        assert_eq!(run["longGrip"], false);
        let id = run["bossId"].as_str().unwrap();
        let original = boss_by_id(id).unwrap();
        let boss = if run["phasesEnabled"].as_bool().unwrap() {
            original
        } else {
            let mut copy = *original;
            copy.phases = &[];
            Box::leak(Box::new(copy))
        };
        let recording = Recording {
            name: run["name"].as_str().unwrap().to_owned(),
            seed: number(run, "seed"),
            ticks: number(run, "ticks"),
            bindings: run["bindings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| Binding {
                    slot: u8::try_from(number(b, "slot")).unwrap(),
                    spell: index_of(b["spellId"].as_str().unwrap()).unwrap(),
                })
                .collect(),
            presses: run["presses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Press {
                    tick: number(p, "tick"),
                    slot: u8::try_from(number(p, "slot")).unwrap(),
                })
                .collect(),
        };
        let worn = &run["worn"];
        let mut fight = Fight::new(FightConfig {
            boss,
            seed: recording.seed,
            duration_ms: Some(recording.ticks * TICK_MS),
            player_level: number(run, "playerLevel"),
            worn: WornValues {
                catalysis: number(worn, "catalysis"),
                purity: number(worn, "purity"),
                yield_bonus: number(worn, "yieldBonus"),
                essence: number(worn, "essence"),
                vessel: number(worn, "vessel"),
                ward: number(worn, "ward"),
                gear_effects: number(worn, "gearEffects"),
            },
            ..FightConfig::default()
        });
        let actual = fight.replay(&recording, true);
        let expected = &run["expected"];
        assert_eq!(
            actual.damage,
            number(expected, "damage"),
            "{} damage",
            recording.name
        );
        assert_eq!(
            actual.damage_taken,
            number(expected, "damageTaken"),
            "{} damage taken",
            recording.name
        );
        assert_eq!(actual.accepted, number(expected, "accepted"));
        assert_eq!(actual.rejected, number(expected, "rejected"));
        assert_eq!(
            actual.killed_at_ms.map(i64::from).unwrap_or(-1),
            expected["killedAtMs"].as_i64().unwrap()
        );
        assert_eq!(
            actual.died_at_ms.map(i64::from).unwrap_or(-1),
            expected["diedAtMs"].as_i64().unwrap()
        );
        assert_eq!(actual.final_checksum, number(expected, "finalChecksum"));
        assert_eq!(
            actual.sequence_checksum,
            number(expected, "sequenceChecksum")
        );
        for (spell, count) in expected["casts"].as_object().unwrap() {
            assert_eq!(
                u64::from(actual.casts[index_of(spell).unwrap()]),
                count.as_u64().unwrap()
            );
        }
        let checksums: Vec<u32> = run["checksums"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| u32::try_from(n.as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(
            actual.checksums, checksums,
            "{} tick checksums",
            recording.name
        );
        taken.push(actual.damage_taken);
    }
    assert_eq!(taken[0] - taken[1], 245, "boss-1-2 bound strike");
    assert_eq!(taken[2] - taken[3], 228, "boss-3-4 bound strike");
}
