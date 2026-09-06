//! Independent exact-rational references, not a Monte-Carlo accuracy estimate.

use rota_sim::verteilung::{
    verteilung_ueber_geprueft, NumerikFehler, QuantilGrenze, StatistikGrenze, Treffer, Verteilung,
};

fn reference() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/verteilung-965-reference.json")).unwrap()
}

fn endpoint(row: &serde_json::Value, name: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(row[name].as_str().unwrap(), 16).unwrap())
}

fn encloses_exact_reference(bound: StatistikGrenze, row: &serde_json::Value) {
    let lo = endpoint(row, "lower_bits");
    let hi = endpoint(row, "upper_bits");
    assert!(
        bound.untergrenze <= lo,
        "{bound:?} does not enclose rational lower {lo:e}"
    );
    assert!(
        bound.obergrenze >= hi,
        "{bound:?} does not enclose rational upper {hi:e}"
    );
    assert!(bound.absoluter_fehler >= (bound.wert - lo).abs().max((hi - bound.wert).abs()));
}

fn binary_hits(n: u32) -> Vec<Treffer> {
    (0..n)
        .map(|i| Treffer {
            ohne_krit: 0,
            mit_krit: 1 << i,
        })
        .collect()
}

#[test]
fn sixteen_thousand_states_obey_mass_and_every_statistic_bound() {
    let reference = reference();
    let n = reference["n"].as_u64().unwrap() as u32;
    let threshold = reference["threshold"].as_u64().unwrap();
    let v = verteilung_ueber_geprueft(&binary_hits(n), 0.1, &[threshold]).unwrap();
    let proof = v.massen_nachweis().unwrap();
    assert_eq!(proof.zustaende, 16_384);
    assert_eq!(proof.schritte, u64::from(n));
    assert_eq!(proof.zustandsuebergaenge, 2 * ((1_u64 << n) - 1));
    assert!(proof.gesamtmasse.untergrenze <= 1.0 && proof.gesamtmasse.obergrenze >= 1.0);
    assert!((proof.gesamtmasse.wert - 1.0).abs() <= proof.gesamtmasse.absoluter_fehler);

    // Each rational mass is enclosed by adjacent binary64 values generated
    // with Python Fraction. Summing the FARTHER endpoint distances outward
    // bounds the true L1 error from above, including this test's summation.
    let mut l1_upper = 0.0_f64;
    let mut rounded_states = 0;
    for (state, measured) in v.massen() {
        let row = &reference["masses_by_popcount"][state.schaden.count_ones() as usize];
        let lo = endpoint(row, "lower_bits");
        let hi = endpoint(row, "upper_bits");
        let error = (measured - lo).abs().max((hi - measured).abs()).next_up();
        l1_upper = (l1_upper + error).next_up();
        if measured < lo || measured > hi {
            rounded_states += 1;
        }
    }
    assert!(
        rounded_states > 0,
        "reference must observe real accumulated roundoff"
    );
    assert!(
        l1_upper <= proof.l1_fehler,
        "observed L1 <= {l1_upper:e}, proof {proof:?}"
    );
    assert!(
        proof.l1_fehler < 1e-9,
        "a unit-scale bound is not useful on this control"
    );

    encloses_exact_reference(v.erwarteter_schaden_grenze().unwrap(), &reference["mean"]);
    encloses_exact_reference(v.varianz_grenze().unwrap(), &reference["variance"]);
    encloses_exact_reference(
        v.phase_erreicht_grenze(1).unwrap(),
        &reference["win_probability"],
    );
    encloses_exact_reference(
        v.siegwahrscheinlichkeit_grenze(threshold).unwrap(),
        &reference["win_probability"],
    );
    let stddev = v.standardabweichung_grenze().unwrap();
    assert!(
        stddev.untergrenze
            <= endpoint(&reference["variance"], "lower_bits")
                .sqrt()
                .next_down()
    );
    assert!(
        stddev.obergrenze
            >= endpoint(&reference["variance"], "upper_bits")
                .sqrt()
                .next_up()
    );
    for quantile in reference["quantiles"].as_array().unwrap() {
        let q = quantile["probability"].as_f64().unwrap();
        let exact = quantile["damage"].as_u64().unwrap();
        let bracket = v.quantil_grenze(q).unwrap();
        assert!(bracket.unterer_schaden <= exact && exact <= bracket.oberer_schaden);
    }
    println!("states={}, transitions={}, rounded_states={rounded_states}, measured_L1_upper={l1_upper:e}, certified_L1={:e}",
        proof.zustaende, proof.zustandsuebergaenge, proof.l1_fehler);
}

#[test]
fn exact_u128_reference_proves_l1_for_16384_rounding_states() {
    // 15^14 exceeds the binary64 significand; this is not an exact-arithmetic
    // toy. All exact mass numerators over 2^56 still fit in u128.
    let n = 14;
    let v = verteilung_ueber_geprueft(&binary_hits(n), 1.0 / 16.0, &[8192]).unwrap();
    let proof = v.massen_nachweis().unwrap();
    assert_eq!(proof.zustaende, 16_384);
    assert!(15_u128.pow(n) > 1_u128 << 53);

    // Every nonzero computed mass here, and the final error bound, is an
    // exact integer multiple of 2^-108. Decode IEEE bits, without a float
    // multiplication/conversion or a second numerical DP in the reference.
    fn scaled(value: f64) -> u128 {
        assert!(value.is_finite() && value > 0.0);
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
        let shift = exponent - 52 + 108;
        assert!((0..=74).contains(&shift));
        u128::from((bits & ((1_u64 << 52) - 1)) | (1_u64 << 52)) << shift
    }

    let mut l1_exact_scaled = 0_u128;
    let mut nonzero_errors = 0;
    let mut total_numerator = 0_u128;
    let mut quantiles = [None; 3];
    for (state, mass) in v.massen() {
        let numerator = 15_u128.pow(n - state.schaden.count_ones());
        let error = scaled(mass).abs_diff(numerator << 52);
        l1_exact_scaled += error;
        nonzero_errors += usize::from(error != 0);
        total_numerator += numerator;
        for (i, threshold) in [1_u128 << 54, 1_u128 << 55, 3_u128 << 54]
            .into_iter()
            .enumerate()
        {
            if quantiles[i].is_none() && total_numerator >= threshold {
                quantiles[i] = Some(state.schaden);
            }
        }
    }
    assert_eq!(total_numerator, 1_u128 << 56);
    assert!(nonzero_errors > 0);
    assert!(l1_exact_scaled > 0);
    assert!(l1_exact_scaled <= scaled(proof.l1_fehler));
    for (i, q) in [0.25, 0.5, 0.75].into_iter().enumerate() {
        let exact = quantiles[i].unwrap();
        let b = v.quantil_grenze(q).unwrap();
        assert!(b.unterer_schaden <= exact && b.oberer_schaden >= exact);
    }
    let expected_mean = 16383.0 / 16.0;
    let expected_variance = 15.0 * f64::from((4_u32.pow(n) - 1) / 3) / 256.0;
    for (b, exact) in [
        (v.erwarteter_schaden_grenze().unwrap(), expected_mean),
        (v.varianz_grenze().unwrap(), expected_variance),
        (v.phase_erreicht_grenze(1).unwrap(), 1.0 / 16.0),
    ] {
        assert!(b.untergrenze <= exact && b.obergrenze >= exact);
    }
    println!("exact u128 reference: {nonzero_errors} rounded states, L1 numerator {l1_exact_scaled} / 2^108");
}

#[test]
fn one_state_does_not_hide_a_thousand_rounding_merge_steps() {
    let hit = Treffer {
        ohne_krit: 1,
        mit_krit: 1,
    };
    let one = verteilung_ueber_geprueft(&[hit], 0.1, &[]).unwrap();
    let many = verteilung_ueber_geprueft(&vec![hit; 1000], 0.1, &[]).unwrap();
    let a = one.massen_nachweis().unwrap();
    let b = many.massen_nachweis().unwrap();
    assert_eq!((a.zustaende, b.zustaende), (1, 1));
    assert_eq!((a.zustandsuebergaenge, b.zustandsuebergaenge), (2, 2000));
    assert!(b.l1_fehler > a.l1_fehler * 100.0);
    let mean = many.erwarteter_schaden_grenze().unwrap();
    assert!(mean.untergrenze <= 1000.0 && mean.obergrenze >= 1000.0);
    let var = many.varianz_grenze().unwrap();
    assert_eq!(var.untergrenze, 0.0);
    assert!(var.obergrenze >= 0.0 && var.absoluter_fehler.is_finite());
    let sigma = many.standardabweichung_grenze().unwrap();
    assert_eq!(sigma.untergrenze, 0.0);
    assert!(sigma.obergrenze.is_finite());
}

#[test]
fn quantile_jump_has_damage_bounds_instead_of_a_mass_epsilon() {
    let hit = Treffer {
        ohne_krit: 0,
        mit_krit: 1_000_000_000,
    };
    let v = verteilung_ueber_geprueft(&[hit], 0.5, &[]).unwrap();
    assert_eq!(
        v.quantil_grenze(0.5).unwrap(),
        QuantilGrenze {
            unterer_schaden: 0,
            oberer_schaden: 1_000_000_000,
        }
    );
    assert_eq!(
        v.quantil_grenze(0.25).unwrap(),
        QuantilGrenze {
            unterer_schaden: 0,
            oberer_schaden: 0
        }
    );
    assert_eq!(
        v.quantil_grenze(0.75).unwrap(),
        QuantilGrenze {
            unterer_schaden: 1_000_000_000,
            oberer_schaden: 1_000_000_000,
        }
    );
    for q in [0.0, 1.0] {
        let x = if q == 0.0 { 0 } else { 1_000_000_000 };
        assert_eq!(
            v.quantil_grenze(q).unwrap(),
            QuantilGrenze {
                unterer_schaden: x,
                oberer_schaden: x
            }
        );
    }
    // Neighboring exactly representable probabilities have medians a billion
    // damage apart. Both must remain inside the mass-uncertain brackets.
    let below = verteilung_ueber_geprueft(&[hit], 0.5_f64.next_down(), &[]).unwrap();
    let above = verteilung_ueber_geprueft(&[hit], 0.5_f64.next_up(), &[]).unwrap();
    assert_eq!(below.quantil_grenze(0.5).unwrap().unterer_schaden, 0);
    assert_eq!(
        above.quantil_grenze(0.5).unwrap().oberer_schaden,
        1_000_000_000
    );
}

#[test]
fn underflow_retains_reachable_support_and_an_absolute_mass_bound() {
    let hits = [Treffer {
        ohne_krit: 0,
        mit_krit: 1,
    }; 2];
    let v = verteilung_ueber_geprueft(&hits, f64::MIN_POSITIVE, &[]).unwrap();
    assert_eq!(v.zustaende(), 3);
    assert_eq!(v.massen().find(|(z, _)| z.schaden == 2).unwrap().1, 0.0);
    assert!(v.massen_nachweis().unwrap().l1_fehler > 0.0);
    assert!(v.siegwahrscheinlichkeit_grenze(2).unwrap().obergrenze > 0.0);
    assert_eq!(
        v.quantil_grenze(1.0).unwrap(),
        QuantilGrenze {
            unterer_schaden: 2,
            oberer_schaden: 2
        }
    );
    // Even a subnormal INPUT is included, not silently read as zero (DAZ).
    let smallest = verteilung_ueber_geprueft(&hits[..1], f64::from_bits(1), &[]).unwrap();
    assert_eq!(
        smallest
            .massen()
            .find(|(z, _)| z.schaden == 1)
            .unwrap()
            .1
            .to_bits(),
        1
    );
}

#[test]
fn invalid_probabilities_and_uncertified_distributions_are_rejected() {
    for p in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.1] {
        assert_eq!(
            verteilung_ueber_geprueft(&[], p, &[]).unwrap_err(),
            NumerikFehler::Wahrscheinlichkeit
        );
        assert_eq!(
            Verteilung::am_anfang().quantil_grenze(p).unwrap_err(),
            NumerikFehler::Wahrscheinlichkeit
        );
    }
    assert_eq!(
        Verteilung::default().massen_nachweis().unwrap_err(),
        NumerikFehler::KeinNachweis
    );
    for p in [0.0, 1.0] {
        let v = verteilung_ueber_geprueft(&binary_hits(8), p, &[]).unwrap();
        assert_eq!(v.zustaende(), 1);
        assert_eq!(v.massen_nachweis().unwrap().l1_fehler, 0.0);
    }
}

#[test]
fn integer_rng_quantization_is_not_claimed_as_numerical_roundoff() {
    // The threshold's probability under a UNIFORM u32 model. This does not
    // establish uniformity or independence of the production FNV generator.
    let p = 0.1;
    let effective = rota_sim::verteilung::schwelle(p) as f64 / 4_294_967_296.0;
    let hit = [Treffer {
        ohne_krit: 0,
        mit_krit: 1,
    }];
    let model = verteilung_ueber_geprueft(&hit, p, &[]).unwrap();
    let rng_model = verteilung_ueber_geprueft(&hit, effective, &[]).unwrap();
    let a = model.siegwahrscheinlichkeit_grenze(1).unwrap();
    let b = rng_model.siegwahrscheinlichkeit_grenze(1).unwrap();
    assert!(
        b.untergrenze > a.obergrenze,
        "different models must not be hidden inside roundoff"
    );
    assert!(effective - p > 9e-11);
}

#[test]
fn an_event_is_evaluated_once_per_state() {
    let v = verteilung_ueber_geprueft(&binary_hits(4), 0.5, &[]).unwrap();
    let calls = std::cell::Cell::new(0);
    let bound = v
        .wahrscheinlichkeit_grenze(|z| {
            calls.set(calls.get() + 1);
            z.schaden < 8
        })
        .unwrap();
    assert_eq!(calls.get(), v.zustaende());
    assert!(bound.untergrenze <= 0.5 && bound.obergrenze >= 0.5);
}
