//! Gemeinsame bitgenaue Rundungsvektoren mit TypeScript (#962).

use rota_sim::factors::round_js;
use rota_sim::{compute_damage, DamageFactors, NEUTRAL};
use serde_json::Value;

mod common;

fn vectors() -> Value {
    let value: Value = serde_json::from_str(include_str!("fixtures/rounding-vectors.json"))
        .expect("Rundungsvektoren sind kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1));
    assert!(value["cases"].as_array().unwrap().len() >= 71);
    value
}

#[test]
fn gemeinsame_vektoren_beider_vorzeichen_sind_bitgleich() {
    let json = vectors();
    for case in json["cases"].as_array().unwrap() {
        let actual = round_js(common::float(&case["inputBits"]));
        if case["expectedBits"].is_null() {
            assert!(actual.is_nan(), "{}", case["name"]);
        } else {
            assert_eq!(
                actual.to_bits(),
                common::bits(&case["expectedBits"]),
                "{}: {} -> {}",
                case["name"],
                case["input"],
                case["expected"]
            );
        }
    }
}

#[test]
fn minus_eins_komma_fuenf_und_seine_nachbarn_sind_enthalten() {
    let json = vectors();
    for name in ["1.5-tie", "-1.5-below", "-1.5-tie", "-1.5-above"] {
        assert!(json["cases"]
            .as_array()
            .unwrap()
            .iter()
            .any(|case| case["name"].as_str() == Some(name)));
    }
    assert_eq!(round_js(-1.5), -1.0);
    assert_eq!((-1.5_f64).round(), -2.0);
}

#[test]
fn zentrale_schadensrundung_bleibt_im_u32_bereich_unveraendert() {
    let json = vectors();
    let mut executed = 0;
    for case in json["cases"].as_array().unwrap() {
        let input = common::float(&case["inputBits"]);
        if !(0.0..=f64::from(u32::MAX)).contains(&input) {
            continue;
        }
        let expected = common::float(&case["expectedBits"]);
        let factors = DamageFactors {
            essence: input,
            ..NEUTRAL
        };
        assert_eq!(compute_damage(1, &factors), expected as u32);
        assert_eq!(compute_damage(1, &factors), input.round() as u32);
        executed += 1;
    }
    assert!(executed >= 20);
}

#[test]
fn zwei_negative_faktoren_erzeugen_einen_negativen_zwischenwert() {
    let factors = DamageFactors {
        essence: -1.5,
        talents: -1.0,
        ..NEUTRAL
    };
    // Diese echte API-Rechnung passiert die bestehende Endwert-Zusicherung:
    // 1 * -1.5 * -1 = 1.5 -> 2. Negative Zwischenwerte sind also erreichbar.
    assert_eq!(compute_damage(1, &factors), 2);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "negativer Schaden")]
fn negative_konfiguration_erreicht_die_schadensgrenze_im_echten_kampf() {
    let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
        factors: DamageFactors {
            essence: -1.5,
            ..NEUTRAL
        },
        ..rota_sim::FightConfig::default()
    });
    fight.run(rota_sim::rotation::good);
}
