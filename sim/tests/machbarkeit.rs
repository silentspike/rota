//! Rechenproben mit ausdrücklich synthetischen Raten; keine Host-Benchmarks.
//! Der echte Messauftrag steht im Beispiel `e21-0b-machbarkeit`.

use std::time::Duration;

use rota_sim::hostcaps::{HostCaps, IsaSet};
use rota_sim::kalibrierung::{Abbruchgrund, Arbeitsart, Profil, Profilschluessel, RateModel};
use rota_sim::kampagne::{
    AuditPolicy, CampaignDefinition, CampaignRunContract, ConfidenceSpec, CoverageSpec,
    DateTimeUtc, DeadlinePolicy, Mission, ProjectionSet, QualityTargets, ReferenceProfile,
    WorkUnits,
};
use rota_sim::machbarkeit::{preflight, CalibratedHost, FeasibilityError, WorkEstimate};

fn contract(target_ms: u64) -> CampaignRunContract {
    contract_for(Duration::from_millis(target_ms))
}

/// Die Zielzeit gehört seit #914 zur Kampagne; der Vertrag übernimmt sie.
fn contract_for(target_wall_time: Duration) -> CampaignRunContract {
    CampaignRunContract::from_campaign(
        &CampaignDefinition {
            profile_version: 1,
            reference_profile: ReferenceProfile {
                name: "fixture-only",
                version: 1,
                hosts: &[],
                network: None,
            },
            target_wall_time,
            canonical_work_budget: WorkUnits(1_000_000),
            exact_obligations: ProjectionSet { required: 1 },
            coverage_spec: CoverageSpec { strength: 0 },
            quality_targets: QualityTargets {
                minimum_niche_coverage: 0.0,
                independent_restarts: 1,
                plateau_work_units: WorkUnits(0),
                maximum_bound_gap: None,
                confidence_requirement: ConfidenceSpec { level: 0.99 },
                final_audit: AuditPolicy::ScalarReplayAll,
            },
        },
        Mission::SingleBoss {
            boss: "fixture-only",
        },
    )
}

fn rate(value: f64) -> RateModel {
    RateModel {
        rate_per_s: value,
        spread: 0.01,
        samples: 8,
        duration_ms: 6000,
        reason: Abbruchgrund::Stabil,
        measured_at: 42,
        psi_cpu: None,
        takt_drift: Some(0.01),
    }
}

fn profile(name: &str, value: f64) -> Profil {
    Profil {
        schluessel: Profilschluessel {
            host: name.to_owned(),
            artefakt: "synthetic-artifact".to_owned(),
            kernel: "synthetic-kernel".to_owned(),
            taktregime: "synthetic-regime".to_owned(),
        },
        caps: HostCaps {
            threads_total: 16,
            threads_usable: 12,
            l2_per_core: 0,
            l3: 0,
            ram_total: 1024,
            ram_free: 1024,
            isa: IsaSet::default(),
            gpu: None,
            calibrated_fights_per_s: value,
            calibration_spread: 0.01,
            measured_at: 42,
        },
        raten: vec![(Arbeitsart::ScalarFullFight, rate(value))],
    }
}

fn host(profile: &Profil) -> CalibratedHost<'_> {
    CalibratedHost {
        profile,
        expected_key: &profile.schluessel,
    }
}

fn work() -> WorkEstimate {
    WorkEstimate {
        kind: Arbeitsart::ScalarFullFight,
        mandatory: 100,
        optional: 0,
        work_units_per_unit: WorkUnits(1),
    }
}

#[test]
fn zu_kurze_zielzeit_bricht_ab_und_nennt_alle_drei_auswege() {
    let contract = contract(999);
    let before = contract.to_json().expect("finite fixture");
    let profile = profile("fixture", 100.0);
    let error = preflight(&contract, &[work()], &[host(&profile)]).expect_err("too short");
    assert!(matches!(error, FeasibilityError::TimeBudgetExceeded { .. }));
    let message = error.to_string();
    for path in [
        "--hosts",
        "--target-time",
        "--hard-deadline",
        "AnytimePartial",
    ] {
        assert!(message.contains(path), "missing {path}: {message}");
    }
    assert_eq!(before, contract.to_json().expect("unchanged fixture"));
}

#[test]
fn ausreichende_und_exakt_knappe_zielzeit_sind_beide_zulaessig() {
    let profile = profile("fixture", 100.0);
    for target in [1000, 1001, 10_000] {
        let estimate = preflight(&contract(target), &[work()], &[host(&profile)]).expect("fits");
        assert_eq!(estimate.mandatory_ms, 1000.0);
        assert!(!estimate.partial_requested);
        assert_eq!(estimate.mandatory_work_units, WorkUnits(100));
    }
}

#[test]
fn optionale_arbeit_verhindert_den_start_der_passenden_pflicht_nicht() {
    let profile = profile("fixture", 100.0);
    let mut work = work();
    work.optional = 10_000;
    let estimate = preflight(&contract(1000), &[work], &[host(&profile)]).expect("mandatory fits");
    assert_eq!(estimate.optional_ms, Some(100_000.0));
    assert_eq!(estimate.mandatory_work_units, WorkUnits(100));
}

#[test]
fn weitere_rechner_aendern_eta_aber_nicht_pflichtmenge_oder_vertrag() {
    let first = profile("one", 100.0);
    let second = profile("two", 100.0);
    let contract = contract(500);
    let before = contract.to_json().expect("finite fixture");
    assert!(preflight(&contract, &[work()], &[host(&first)]).is_err());
    let estimate = preflight(&contract, &[work()], &[host(&first), host(&second)]).expect("fits");
    assert_eq!(estimate.mandatory_ms, 500.0);
    assert_eq!(estimate.mandatory_work_units, WorkUnits(100));
    assert_eq!(before, contract.to_json().expect("unchanged fixture"));
}

#[test]
fn explizite_frist_ist_nur_eine_zulassung_fuer_ein_teilergebnis() {
    let profile = profile("fixture", 100.0);
    let contract = contract(999).with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: Duration::from_millis(999),
    });
    let estimate = preflight(&contract, &[work()], &[host(&profile)]).expect("explicit partial");
    assert!(estimate.partial_requested);
    assert_eq!(estimate.mandatory_ms, 1000.0);
    assert_eq!(estimate.mandatory_work_units, WorkUnits(100));
}

#[test]
fn arbeitsklassen_werden_in_zeit_umgerechnet_bevor_addiert_wird() {
    let mut profile = profile("fixture", 100.0);
    profile.raten.push((Arbeitsart::CritDp, rate(10.0)));
    let dp = WorkEstimate {
        kind: Arbeitsart::CritDp,
        mandatory: 100,
        optional: 0,
        work_units_per_unit: WorkUnits(7),
    };
    let estimate = preflight(&contract(11_000), &[work(), dp], &[host(&profile)]).expect("fits");
    assert_eq!(estimate.mandatory_ms, 11_000.0);
    assert_eq!(estimate.mandatory_work_units, WorkUnits(800));
    assert!(preflight(&contract(10_999), &[work(), dp], &[host(&profile)]).is_err());
}

#[test]
fn fehlende_pflichtrate_ist_fehler_fehlende_optionale_rate_bleibt_unbekannt() {
    let profile = profile("fixture", 100.0);
    let mut dp = WorkEstimate {
        kind: Arbeitsart::CritDp,
        mandatory: 1,
        optional: 0,
        work_units_per_unit: WorkUnits(1),
    };
    assert_eq!(
        preflight(&contract(10_000), &[work(), dp], &[host(&profile)]),
        Err(FeasibilityError::MissingRate(Arbeitsart::CritDp))
    );
    dp.mandatory = 0;
    dp.optional = 1;
    let estimate =
        preflight(&contract(1000), &[work(), dp], &[host(&profile)]).expect("mandatory fits");
    assert_eq!(estimate.optional_ms, None);
}

#[test]
fn raten_ohne_gueltigen_messbeleg_werden_zurueckgewiesen() {
    let mut bad = Vec::new();
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        bad.push(rate(value));
    }
    // `0.03` stand hier bis zum 28.10. als „Streuung an der Schwelle". Seitdem
    // hängt die Schwelle an der Unsicherheit des Mittels (#912): An ihr liegt
    // bei acht Abschnitten eine Streuung von `0.03 * sqrt(7)`.
    let an_der_schwelle = 0.03 * 7f64.sqrt();
    for spread in [-0.1, an_der_schwelle, f64::NAN, f64::INFINITY] {
        bad.push(RateModel {
            spread,
            ..rate(100.0)
        });
    }
    bad.push(RateModel {
        samples: 3,
        ..rate(100.0)
    });
    bad.push(RateModel {
        duration_ms: 4999,
        ..rate(100.0)
    });
    bad.push(RateModel {
        reason: Abbruchgrund::Druck,
        ..rate(100.0)
    });
    bad.push(RateModel {
        takt_drift: Some(0.11),
        ..rate(100.0)
    });
    bad.push(RateModel {
        psi_cpu: Some(f64::NAN),
        ..rate(100.0)
    });
    for invalid in bad {
        let mut profile = profile("fixture", 100.0);
        profile.raten[0].1 = invalid;
        assert!(
            preflight(&contract(10_000), &[work()], &[host(&profile)]).is_err(),
            "{invalid:?}"
        );
    }
}

#[test]
fn stabile_rate_mit_breiter_abschnittsstreuung_wird_angenommen() {
    // Seit dem 28.10. hängt `Stabil` an der Unsicherheit des MITTELS, nicht an
    // der Streuung der Abschnitte (#912, Entscheidung des Orchestrators). Auf
    // node3 streuen die Abschnitte des Vollkampfs um rund 10 %, das Mittel aus
    // gut 500 ist auf ein halbes Prozent bekannt. Eine solche Rate ist eine
    // Kalibrierung — wer sie hier verwirft, verwirft jede auf dem Zielrechner.
    let mut profile = profile("fixture", 100.0);
    profile.raten[0].1 = RateModel {
        spread: 0.10,
        samples: 516,
        ..rate(100.0)
    };
    assert!(
        preflight(&contract(10_000), &[work()], &[host(&profile)]).is_ok(),
        "eine stabile Rate mit breiten Abschnitten wurde verworfen"
    );
}

#[test]
fn stabile_rate_mit_taktdrift_wird_angenommen() {
    // Die Kalibrierung meldet die Taktdrift und entscheidet nicht über sie
    // (`kalibrierung.rs`, `kalibriere`): Der Takt schwankt im Leerlauf um über
    // hundert Prozent, und was er an der Rate bewegt, steht schon in der
    // Streuung. Entscheidung des Orchestrators vom 29.10.: Die Vorabprüfung
    // folgt ihr. Bis dahin verwarf sie ab `TAKT_DRIFT_MAX`.
    //
    // Der zweite Fall ist der Messbeleg, an dem es auffiel: node3, 29.10.,
    // `e21-0b-machbarkeit --mandatory 1000000 --target-time 1ms`, Lauf 1 von
    // 6 — „ungültige oder doppelte Rate“ bei einer stabilen Messung.
    let node3 = RateModel {
        rate_per_s: 169_979.531_806,
        spread: 0.026_107,
        samples: 43,
        duration_ms: 5122,
        reason: Abbruchgrund::Stabil,
        measured_at: 42,
        psi_cpu: Some(0.0),
        takt_drift: Some(0.222_755),
    };
    for (fall, gemessen) in [
        (
            "Drift 0.15",
            RateModel {
                takt_drift: Some(0.15),
                ..rate(100.0)
            },
        ),
        ("node3 Lauf 1", node3),
    ] {
        let mut profile = profile("fixture", gemessen.rate_per_s);
        profile.raten[0].1 = gemessen;
        let ergebnis = preflight(&contract(10_000), &[work()], &[host(&profile)]);
        assert!(
            ergebnis.is_ok(),
            "{fall}: eine stabile Rate wurde wegen der Taktdrift verworfen: {ergebnis:?}"
        );
    }
}

#[test]
fn anderer_host_artefakt_kernel_oder_taktregime_ist_inkompatibel() {
    let profile = profile("fixture", 100.0);
    for field in 0..4 {
        let mut expected = profile.schluessel.clone();
        match field {
            0 => expected.host.push_str("-changed"),
            1 => expected.artefakt.push_str("-changed"),
            2 => expected.kernel.push_str("-changed"),
            _ => expected.taktregime.push_str("-changed"),
        }
        let candidate = CalibratedHost {
            profile: &profile,
            expected_key: &expected,
        };
        let error =
            preflight(&contract(10_000), &[work()], &[candidate]).expect_err("incompatible");
        assert!(error.to_string().contains("inkompatibles Profil"));
    }
}

#[test]
fn doppelte_hosts_und_raten_erhoehen_die_kapazitaet_nicht() {
    let mut profile = profile("fixture", 100.0);
    assert!(preflight(
        &contract(10_000),
        &[work()],
        &[host(&profile), host(&profile)]
    )
    .is_err());
    profile
        .raten
        .push((Arbeitsart::ScalarFullFight, rate(100.0)));
    assert!(preflight(&contract(10_000), &[work()], &[host(&profile)]).is_err());
}

#[test]
fn avx_rate_braucht_passende_isa_und_keine_kerne_ist_keine_kapazitaet() {
    let mut profile = profile("fixture", 100.0);
    profile.raten.push((Arbeitsart::AvxFullFight, rate(100.0)));
    assert!(preflight(&contract(10_000), &[work()], &[host(&profile)]).is_err());
    profile.raten.pop();
    profile.caps.threads_usable = 0;
    assert!(preflight(&contract(10_000), &[work()], &[host(&profile)]).is_err());
}

#[test]
fn wu_kosten_sind_ganzzahlig_und_werden_nicht_aus_raten_abgeleitet() {
    let profile = profile("fixture", 100.0);
    let mut work = work();
    work.work_units_per_unit = WorkUnits(10_001);
    let error = preflight(&contract(10_000), &[work], &[host(&profile)]).expect_err("WU exceed");
    assert!(matches!(error, FeasibilityError::WorkBudgetExceeded { .. }));
    work.work_units_per_unit = WorkUnits(u64::MAX);
    assert!(preflight(&contract(10_000), &[work], &[host(&profile)]).is_err());
    work.work_units_per_unit = WorkUnits(0);
    assert!(preflight(&contract(10_000), &[work], &[host(&profile)]).is_err());
}

#[test]
fn leere_auftraege_und_nicht_endliche_eta_sind_fehler() {
    let profile = profile("fixture", f64::MIN_POSITIVE);
    assert!(preflight(&contract(1000), &[], &[host(&profile)]).is_err());
    assert!(preflight(&contract(0), &[work()], &[host(&profile)]).is_err());
    assert!(preflight(&contract(1000), &[work()], &[host(&profile)]).is_err());
}

#[test]
fn restfrist_gilt_auch_wenn_die_zielzeit_viel_groesser_ist() {
    let profile = profile("fixture", 100.0);
    for (remaining_ms, partial) in [(1, true), (999, true), (1000, false), (1001, false)] {
        let contract = contract(10_000).with_hard_deadline(DeadlinePolicy::ActiveRuntime {
            remaining: Duration::from_millis(remaining_ms),
        });
        let estimate =
            preflight(&contract, &[work()], &[host(&profile)]).expect("explicit deadline");
        assert_eq!(estimate.mandatory_ms, 1000.0);
        assert_eq!(
            estimate.partial_requested, partial,
            "remaining_ms={remaining_ms}"
        );
    }
}

#[test]
fn ausdrueckliche_lange_frist_ist_trotz_knapper_zielzeit_kein_teilergebnis() {
    let profile = profile("fixture", 100.0);
    let contract = contract(999).with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: Duration::from_millis(1000),
    });
    let estimate =
        preflight(&contract, &[work()], &[host(&profile)]).expect("actual deadline fits");
    assert!(!estimate.partial_requested);
    assert_eq!(
        estimate.target_ms, 999,
        "the softer goal is still reported unchanged"
    );
}

#[test]
fn abgelaufene_und_ohne_aktuelles_utc_nicht_pruefbare_fristen_sind_fehler() {
    let profile = profile("fixture", 100.0);
    for policy in [
        DeadlinePolicy::ActiveRuntime {
            remaining: Duration::from_millis(0),
        },
        DeadlinePolicy::WallClockUtc {
            deadline: DateTimeUtc { unix_seconds: 1 },
        },
        DeadlinePolicy::WallClockUtc {
            deadline: DateTimeUtc {
                unix_seconds: i64::MAX,
            },
        },
    ] {
        let contract = contract(10_000).with_hard_deadline(policy);
        let error =
            preflight(&contract, &[work()], &[host(&profile)]).expect_err("deadline not usable");
        assert!(matches!(error, FeasibilityError::InvalidInput(_)));
    }
}

#[test]
fn ein_unteilbarer_kampf_wird_auf_zwei_hosts_nicht_doppelt_so_schnell() {
    let one = profile("one", 100.0);
    let two = profile("two", 100.0);
    let work = WorkEstimate {
        mandatory: 1,
        optional: 1,
        ..work()
    };
    assert!(preflight(&contract(9), &[work], &[host(&one), host(&two)]).is_err());
    let estimate =
        preflight(&contract(10), &[work], &[host(&one), host(&two)]).expect("one whole fight fits");
    assert_eq!(estimate.mandatory_ms, 10.0);
    assert_eq!(estimate.optional_ms, Some(10.0));
}

#[test]
fn weniger_jobs_als_hosts_und_angefangene_letzte_wellen_bleiben_ganzzahlig() {
    let one = profile("one", 100.0);
    let two = profile("two", 100.0);
    let three = profile("three", 100.0);
    for (jobs, expected_ms) in [(1, 10), (2, 10), (3, 20), (5, 30)] {
        let work = WorkEstimate {
            mandatory: jobs,
            ..work()
        };
        let estimate = preflight(&contract(expected_ms), &[work], &[host(&one), host(&two)])
            .expect("whole waves fit");
        assert_eq!(estimate.mandatory_ms, expected_ms as f64);
        assert!(preflight(
            &contract(expected_ms - 1),
            &[work],
            &[host(&one), host(&two)]
        )
        .is_err());
    }
    let work = WorkEstimate {
        mandatory: 2,
        ..work()
    };
    let estimate = preflight(
        &contract(10),
        &[work],
        &[host(&one), host(&two), host(&three)],
    )
    .expect("two jobs on three hosts");
    assert_eq!(estimate.mandatory_ms, 10.0);
}

#[test]
fn ungleiche_hosts_werden_nach_tatsaechlichen_jobabschluessen_belegt() {
    let slow = profile("slow", 100.0);
    let fast = profile("fast", 200.0);
    for hosts in [[host(&slow), host(&fast)], [host(&fast), host(&slow)]] {
        for (jobs, expected_ms) in [(1, 5), (2, 10), (3, 10), (4, 15), (5, 20)] {
            let work = WorkEstimate {
                mandatory: jobs,
                ..work()
            };
            let estimate = preflight(&contract(expected_ms), &[work], &hosts)
                .expect("heterogeneous whole jobs fit");
            assert_eq!(estimate.mandatory_ms, expected_ms as f64, "jobs={jobs}");
            assert!(preflight(&contract(expected_ms - 1), &[work], &hosts).is_err());
        }
    }
}

#[test]
fn kleine_heterogene_belegungen_stimmen_mit_vollstaendiger_aufzaehlung_ueberein() {
    let a = profile("a", 100.0); // 10 ms per whole job
    let b = profile("b", 200.0); // 5 ms per whole job
    let c = profile("c", 50.0); // 20 ms per whole job
    for count in 1..=12_u64 {
        let mut exact = u64::MAX;
        for on_a in 0..=count {
            for on_b in 0..=count - on_a {
                let on_c = count - on_a - on_b;
                exact = exact.min((on_a * 10).max(on_b * 5).max(on_c * 20));
            }
        }
        let work = WorkEstimate {
            mandatory: count,
            ..work()
        };
        let estimate = preflight(&contract(exact), &[work], &[host(&a), host(&b), host(&c)])
            .expect("enumerated optimum fits");
        assert_eq!(estimate.mandatory_ms, exact as f64, "count={count}");
    }
}

#[test]
fn u64_mengen_brauchen_keine_schleife_pro_job() {
    let a = profile("a", 1_000_000.0);
    let b = profile("b", 2_000_000.0);
    let mut contract = contract(u64::MAX);
    contract.deterministic_work_budget = WorkUnits(u64::MAX);
    let work = WorkEstimate {
        mandatory: u64::MAX,
        ..work()
    };
    let estimate =
        preflight(&contract, &[work], &[host(&a), host(&b)]).expect("large finite job set");
    assert_eq!(estimate.mandatory_work_units, WorkUnits(u64::MAX));
    assert!(estimate.mandatory_ms.is_finite() && estimate.mandatory_ms > 0.0);
    let only_fast = preflight(&contract, &[work], &[host(&b)]).expect("single host also finite");
    assert!(estimate.mandatory_ms <= only_fast.mandatory_ms);
}

#[test]
fn grosse_frist_wird_durch_f64_rundung_nicht_groesser() {
    let profile = profile("fixture", 1000.0); // exactly one millisecond per job
    let count = (1_u64 << 53) + 4;
    let work = WorkEstimate {
        mandatory: count,
        ..work()
    };
    let mut contract = contract(count - 1);
    contract.deterministic_work_budget = WorkUnits(count);
    assert!(matches!(
        preflight(&contract, &[work], &[host(&profile)]),
        Err(FeasibilityError::TimeBudgetExceeded { .. })
    ));
}

/// Seit #914 sind Zielzeit und Restfrist `Duration`s. Die Vorabprüfung rechnet
/// in ganzen Millisekunden; eine Angabe mit Bruchteil wird abgelehnt statt
/// still abgerundet.
///
/// Abrunden wäre nicht harmlos: Bei 1000 ms + 1 ns gälte eine Pflichtzeit von
/// 1000,0000005 ms als Überschreitung, obwohl sie passt — und die Meldung nennte
/// eine Zielzeit, die niemand angegeben hat.
#[test]
fn zeiten_ohne_ganze_millisekunden_werden_nicht_still_gerundet() {
    let profile = profile("fixture", 100.0);
    let krumm = Duration::from_millis(1000) + Duration::from_nanos(1);
    let error = preflight(&contract_for(krumm), &[work()], &[host(&profile)])
        .expect_err("Zielzeit mit Bruchteil");
    assert!(
        matches!(&error, FeasibilityError::InvalidInput(text) if text.contains("Zielzeit")),
        "{error:?}"
    );

    let mit_frist =
        contract(10_000).with_hard_deadline(DeadlinePolicy::ActiveRuntime { remaining: krumm });
    let error =
        preflight(&mit_frist, &[work()], &[host(&profile)]).expect_err("Restfrist mit Bruchteil");
    assert!(
        matches!(&error, FeasibilityError::InvalidInput(text) if text.contains("Restfrist")),
        "{error:?}"
    );

    // Jenseits von u64 Millisekunden gibt es keine Zahl, mit der gerechnet
    // werden könnte.
    let zu_gross = Duration::from_secs(u64::MAX);
    assert!(matches!(
        preflight(&contract_for(zu_gross), &[work()], &[host(&profile)]),
        Err(FeasibilityError::InvalidInput(_))
    ));

    // GEGENPROBE: dieselben Werte in ganzen Millisekunden sind zulässig.
    assert!(preflight(&contract(1000), &[work()], &[host(&profile)]).is_ok());
    let ganz = contract(10_000).with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: Duration::from_millis(1000),
    });
    assert!(preflight(&ganz, &[work()], &[host(&profile)]).is_ok());
}
