//! Der Kampagnenvertrag lässt sich lesen, was er schreibt (Issue #914).
//!
//! # Warum der Test einen anderen Weg nimmt als der Code
//!
//! [`rota_sim::kampagne`] schreibt sein JSON **von Hand**, weil die Bibliothek
//! abhängigkeitsfrei bleiben muss (`sim/Cargo.toml`: eine Kiste, die im Schritt
//! Speicher anfordert, unterliefe den Determinismus).
//!
//! Dieser Test liest es mit `serde_json` zurück — einer Testabhängigkeit, die
//! in der Bibliothek nicht vorkommt. Das ist Absicht und dasselbe Muster, mit
//! dem das Projekt seine einkompilierten Daten prüft: **Wer mit demselben Code
//! schreibt und liest, prüft seinen eigenen Fehler mit.** Ein handgeschriebener
//! Textbaustein, der kein gültiges JSON ergibt, fiele beim Zurücklesen mit dem
//! eigenen Parser nie auf.

use std::time::Duration;

use rota_sim::kampagne::{
    AuditPolicy, CampaignDefinition, CampaignRunContract, ConfidenceSpec, CoverageSpec,
    CpuReservation, DateTimeUtc, DeadlinePolicy, HostRequirement, JsonError, Mission,
    NetworkRequirement, ProjectionSet, QualityTargets, ReferenceProfile, ResultGuarantee,
    WorkUnits, REFERENCE_PROFILE_STANDARD,
};

static HOSTS: &[HostRequirement] = &[
    HostRequirement {
        name: "laptop",
        guaranteed_cpu_quota: Some(1.0),
        guaranteed_memory_mib: 4096,
        cpu: CpuReservation {
            physical_cores: 8,
            threads_per_core: 2,
            reserved_physical_cores: 2,
        },
        gpu_duty_cycle: 0.0,
        thermal_throttling: false,
    },
    // Der zweite Rechner hat ausdrücklich KEINE Zusicherung — der Fall, den ein
    // Profil mit nur einem Eintrag nie prüfen würde.
    HostRequirement {
        name: "best-effort",
        guaranteed_cpu_quota: None,
        guaranteed_memory_mib: 8192,
        cpu: CpuReservation {
            physical_cores: 16,
            threads_per_core: 2,
            reserved_physical_cores: 2,
        },
        gpu_duty_cycle: 0.5,
        thermal_throttling: false,
    },
];

fn kampagne() -> CampaignDefinition {
    CampaignDefinition {
        profile_version: 3,
        reference_profile: ReferenceProfile {
            name: "json-probe",
            version: 2,
            hosts: HOSTS,
            network: Some(NetworkRequirement {
                max_rtt_ms: 250,
                min_bandwidth_mibit_s: Some(50),
            }),
        },
        target_wall_time: Duration::new(7200, 5),
        canonical_work_budget: WorkUnits(1_234_567),
        exact_obligations: ProjectionSet { required: 12 },
        coverage_spec: CoverageSpec { strength: 2 },
        quality_targets: QualityTargets {
            minimum_niche_coverage: 0.95,
            independent_restarts: 8,
            plateau_work_units: WorkUnits(50_000),
            maximum_bound_gap: None,
            confidence_requirement: ConfidenceSpec { level: 0.99 },
            final_audit: AuditPolicy::ScalarReplayAll,
        },
    }
}

/// Das erzeugte JSON ist gültig und trägt die Werte, die hineingegeben wurden.
#[test]
fn die_kampagne_schreibt_gueltiges_json_mit_ihren_werten() {
    let k = kampagne();
    let text = k.to_json().expect("gültiger Vertrag");
    let v: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("kein gültiges JSON: {e}\n{text}"));

    assert_eq!(v["profileVersion"], 3);
    assert_eq!(v["fingerprint"], k.fingerprint().get());
    assert_eq!(v["referenceProfile"]["name"], "json-probe");
    assert_eq!(v["referenceProfile"]["version"], 2);
    assert_eq!(v["referenceProfile"]["network"]["maxRttMs"], 250);
    assert_eq!(v["referenceProfile"]["network"]["minBandwidthMibitS"], 50);

    // Eine nicht gemessene Bandbreite steht als `null` im Text, nicht als 0 —
    // „unbekannt" gegen „null verlangt". Gegenprobe ist die 50 darüber.
    let profil = REFERENCE_PROFILE_STANDARD;
    let standard = CampaignDefinition {
        reference_profile: profil,
        ..k
    };
    let s: serde_json::Value =
        serde_json::from_str(&standard.to_json().expect("gültig")).expect("JSON");
    assert_eq!(s["referenceProfile"]["name"], "standard");
    assert_eq!(s["referenceProfile"]["network"]["maxRttMs"], 250);
    assert!(
        s["referenceProfile"]["network"]["minBandwidthMibitS"].is_null(),
        "{}",
        s["referenceProfile"]["network"]
    );
    assert!(s["referenceProfile"]["hosts"][1]["guaranteedCpuQuota"].is_null());

    // Sekunden UND Nanosekunden: Eine Zielzeit in Millisekunden verlöre die 5 ns
    // hier, und zwei Profile mit verschiedenem Fingerabdruck schrieben
    // denselben Text.
    assert_eq!(v["targetWallTime"]["secs"], 7200);
    assert_eq!(v["targetWallTime"]["nanos"], 5);
    assert_eq!(v["canonicalWorkBudget"], 1_234_567);
    assert_eq!(v["exactObligations"], 12);
    assert_eq!(v["coverageStrength"], 2);
    assert_eq!(v["minimumNicheCoverage"], 0.95);
    assert_eq!(v["independentRestarts"], 8);
    assert_eq!(v["plateauWorkUnits"], 50_000);
    assert_eq!(v["confidence"], 0.99);
    assert_eq!(v["finalAudit"], "scalarReplayAll");

    let hosts = v["referenceProfile"]["hosts"]
        .as_array()
        .expect("hosts ist keine Liste");
    assert_eq!(hosts.len(), 2, "Beide Rechner müssen im Text stehen.");
    assert_eq!(hosts[0]["name"], "laptop");
    assert_eq!(
        hosts[0]["computeThreads"], 12,
        "Die Anzeige rechnet die SMT-Geschwister nicht mit."
    );
    assert_eq!(hosts[0]["totalThreads"], 16);
    assert_eq!(hosts[0]["gpuDutyCycle"], 0.0);
    assert_eq!(hosts[1]["gpuDutyCycle"], 0.5);
    assert!(
        hosts[0].get("maxRttMs").is_none(),
        "Das Netz steht am Profil, nicht mehr am Rechner"
    );
}

/// **Ein fehlender Wert wird `null`, nicht `0`.**
///
/// Drei Aussagen, die im JSON auseinandergehalten werden müssen:
/// `null` = ausdrücklich best effort · eine Zahl = Zusicherung · `0.0` =
/// garantiert nichts. Wer den fehlenden Wert als Null schreibt, macht aus der
/// ersten die dritte.
#[test]
fn ein_fehlender_wert_wird_null_und_nicht_null_komma_null() {
    let text = kampagne().to_json().expect("gültiger Vertrag");
    let v: serde_json::Value = serde_json::from_str(&text).expect("kein gültiges JSON");

    assert!(
        v["maximumBoundGap"].is_null(),
        "Eine fehlende Schranke steht nicht als null im Text: {}",
        v["maximumBoundGap"]
    );

    let hosts = v["referenceProfile"]["hosts"].as_array().unwrap();
    assert!(
        hosts[1]["guaranteedCpuQuota"].is_null(),
        "Best effort steht nicht als null im Text: {}",
        hosts[1]["guaranteedCpuQuota"]
    );
    // GEGENPROBE: Der Rechner MIT Zusicherung darf nicht auch null sein — sonst
    // wäre der Fall oben auch dann grün, wenn das Feld immer null wird.
    assert_eq!(
        hosts[0]["guaranteedCpuQuota"], 1.0,
        "Auch die vorhandene Zusicherung steht als null — dann unterscheidet \
         der Text die beiden Fälle gar nicht."
    );

    // Und eine gesetzte Schranke von 0.0 muss als Zahl erscheinen, nicht als
    // null. Das ist die Richtung, die beim Schreiben am leichtesten kippt.
    let mut mit_null = kampagne();
    mit_null.quality_targets.maximum_bound_gap = Some(0.0);
    let v2: serde_json::Value =
        serde_json::from_str(&mit_null.to_json().expect("gültiger Vertrag")).unwrap();
    assert_eq!(
        v2["maximumBoundGap"], 0.0,
        "Eine ausdrücklich verlangte Schranke von 0.0 wurde zu null."
    );
    assert!(!v2["maximumBoundGap"].is_null());
}

/// Der Vertrag ohne Frist schreibt `null` — nicht eine sehr grosse Zahl (AC5).
#[test]
fn ohne_frist_steht_null_im_vertrag() {
    let v0 = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance);
    let v: serde_json::Value =
        serde_json::from_str(&v0.to_json().expect("gültiger Vertrag")).expect("kein gültiges JSON");

    assert!(
        v["hardDeadline"].is_null(),
        "Ohne ausdrücklichen Wunsch steht eine Frist im Vertrag: {}",
        v["hardDeadline"]
    );
    assert_eq!(v["campaignDefinitionHash"], kampagne().fingerprint().get());
    assert_eq!(v["deterministicWorkBudget"], 1_234_567);
    assert_eq!(v["targetWallTime"]["secs"], 7200);
    assert_eq!(v["targetWallTime"]["nanos"], 5);
    assert_eq!(v["mission"]["kind"], "fullBalance");

    // Die drei Festlegungen stehen im Text, nicht nur im Typ (AC6).
    assert_eq!(v["runtimeAccounting"]["countsPausedTime"], false);
    assert_eq!(v["runtimeAccounting"]["countsCoordinatorOutage"], false);
    assert_eq!(v["runtimeAccounting"]["countsHostWait"], false);
}

/// Beide Fristarten sind im Text unterscheidbar (AC6).
///
/// Zwei Fristen, die als dieselbe Zeile erscheinen, wären beim Zurücklesen
/// nicht mehr auseinanderzuhalten — und die Bedeutung ist genau das, was AC6
/// benannt haben will.
#[test]
fn beide_fristarten_stehen_unterscheidbar_im_text() {
    let basis = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance);

    let wanduhr = basis.with_hard_deadline(DeadlinePolicy::WallClockUtc {
        deadline: DateTimeUtc {
            unix_seconds: 1_800_000_000,
        },
    });
    let rechenzeit = basis.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: Duration::new(7200, 123_456_789),
    });

    let a: serde_json::Value =
        serde_json::from_str(&wanduhr.to_json().expect("gültiger Vertrag")).unwrap();
    let b: serde_json::Value =
        serde_json::from_str(&rechenzeit.to_json().expect("gültiger Vertrag")).unwrap();

    assert_eq!(a["hardDeadline"]["kind"], "wallClockUtc");
    assert_eq!(
        a["hardDeadline"]["deadline"]["unixSeconds"],
        1_800_000_000_i64
    );
    assert_eq!(b["hardDeadline"]["kind"], "activeRuntime");
    assert_eq!(b["hardDeadline"]["remaining"]["secs"], 7200);
    assert_eq!(b["hardDeadline"]["remaining"]["nanos"], 123_456_789);
    assert_ne!(
        a["hardDeadline"], b["hardDeadline"],
        "Die beiden Fristarten sehen im Text gleich aus."
    );
}

/// Alle drei Missionsarten sind im Text unterscheidbar.
#[test]
fn jede_mission_steht_unterscheidbar_im_text() {
    let k = kampagne();
    let texte: Vec<serde_json::Value> = [
        Mission::FullBalance,
        Mission::SingleInstance { instance: 5 },
        Mission::SingleBoss { boss: "boss-1-2" },
    ]
    .into_iter()
    .map(|m| {
        let vertrag = CampaignRunContract::from_campaign(&k, m);
        serde_json::from_str(&vertrag.to_json().expect("gültiger Vertrag"))
            .expect("kein gültiges JSON")
    })
    .collect();

    assert_eq!(texte[0]["mission"]["kind"], "fullBalance");
    assert_eq!(texte[1]["mission"]["kind"], "singleInstance");
    assert_eq!(texte[1]["mission"]["instance"], 5);
    assert_eq!(texte[2]["mission"]["kind"], "singleBoss");
    assert_eq!(texte[2]["mission"]["boss"], "boss-1-2");

    // Die drei Missionen dürfen nicht denselben Text ergeben.
    assert_ne!(texte[0]["mission"], texte[1]["mission"]);
    assert_ne!(texte[1]["mission"], texte[2]["mission"]);
}

/// Alle JSON-Steuerzeichen, Anführungszeichen, Backslash und nicht-ASCII
/// müssen bytegetreu rücklesbar bleiben. Rusts Debug-Format kann das nicht.
#[test]
fn namen_mit_steuerzeichen_und_unicode_sind_gueltige_json_strings() {
    const NAME: &str = concat!(
        "\0\u{1}\u{2}\u{3}\u{4}\u{5}\u{6}\u{7}\u{8}\t\n\u{b}\u{c}\r\u{e}\u{f}",
        "\u{10}\u{11}\u{12}\u{13}\u{14}\u{15}\u{16}\u{17}",
        "\u{18}\u{19}\u{1a}\u{1b}\u{1c}\u{1d}\u{1e}\u{1f}",
        "\"\\/ä🎵\u{7f}\u{2028}"
    );
    let host = HostRequirement {
        name: NAME,
        ..HOSTS[0]
    };
    let text = host.to_json().expect("endliche Quote");
    let gelesen: serde_json::Value = serde_json::from_str(&text).expect("JSON-Hostname");
    assert_eq!(gelesen["name"].as_str(), Some(NAME));

    let vertrag =
        CampaignRunContract::from_campaign(&kampagne(), Mission::SingleBoss { boss: NAME });
    let text = vertrag.to_json().expect("endlicher Vertrag");
    let gelesen: serde_json::Value = serde_json::from_str(&text).expect("JSON-Bossname");
    assert_eq!(gelesen["mission"]["boss"].as_str(), Some(NAME));
}

/// Ungültige Zahlen werden nicht zu `null`: Das hieße bei maximumBoundGap,
/// die Schranke aufzuheben, statt einen fehlerhaften Vertrag zurückzuweisen.
#[test]
fn nichtendliche_qualitaetsvorgaben_werden_mit_feldname_abgelehnt() {
    for wert in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut k = kampagne();
        k.quality_targets.minimum_niche_coverage = wert;
        assert_eq!(
            k.to_json(),
            Err(JsonError::NonFinite {
                field: "minimumNicheCoverage"
            })
        );

        let mut k = kampagne();
        k.quality_targets.maximum_bound_gap = Some(wert);
        assert_eq!(
            k.to_json(),
            Err(JsonError::NonFinite {
                field: "maximumBoundGap"
            })
        );

        let mut k = kampagne();
        k.quality_targets.confidence_requirement.level = wert;
        assert_eq!(
            k.to_json(),
            Err(JsonError::NonFinite {
                field: "confidence"
            })
        );

        // Der Laufvertrag ist öffentlich veränderbar. Seine Ausgabe darf
        // sich deshalb nicht allein auf die vorherige Kampagne verlassen.
        let basis = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance);
        let mut vertrag = basis;
        vertrag.minimum_niche_coverage = wert;
        assert_eq!(
            vertrag.to_json(),
            Err(JsonError::NonFinite {
                field: "minimumNicheCoverage"
            })
        );
        let mut vertrag = basis;
        vertrag.maximum_bound_gap = Some(wert);
        assert_eq!(
            vertrag.to_json(),
            Err(JsonError::NonFinite {
                field: "maximumBoundGap"
            })
        );
        let mut vertrag = basis;
        vertrag.confidence_requirement.level = wert;
        assert_eq!(
            vertrag.to_json(),
            Err(JsonError::NonFinite {
                field: "confidence"
            })
        );
    }
}

#[test]
fn nichtendliche_cpu_quote_ist_keine_fehlende_zusicherung() {
    for wert in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let host = HostRequirement {
            guaranteed_cpu_quota: Some(wert),
            ..HOSTS[0]
        };
        assert_eq!(
            host.to_json(),
            Err(JsonError::NonFinite {
                field: "guaranteedCpuQuota"
            })
        );
    }
    static UNGUELTIG: &[HostRequirement] = &[HostRequirement {
        guaranteed_cpu_quota: Some(f64::NAN),
        ..HOSTS[0]
    }];
    let mut k = kampagne();
    k.reference_profile.hosts = UNGUELTIG;
    assert_eq!(
        k.to_json(),
        Err(JsonError::NonFinite {
            field: "guaranteedCpuQuota"
        })
    );
}

#[test]
fn endliche_randwerte_bleiben_zahlen_und_cpu_vorgaben_bleiben_erhalten() {
    for wert in [0.0, -0.0, f64::MIN_POSITIVE, f64::MAX] {
        let host = HostRequirement {
            guaranteed_cpu_quota: Some(wert),
            ..HOSTS[0]
        };
        let gelesen: serde_json::Value = serde_json::from_str(&host.to_json().unwrap()).unwrap();
        let quote = gelesen["guaranteedCpuQuota"]
            .as_f64()
            .expect("Zahl statt null");
        assert!(quote.is_finite());
        assert_eq!(quote.to_bits(), wert.to_bits());
        assert_eq!(gelesen["physicalCores"], 8);
        assert_eq!(gelesen["threadsPerCore"], 2);
        assert_eq!(gelesen["reservedPhysicalCores"], 2);
    }
}

#[test]
fn nichtendliche_grafik_einschaltdauer_wird_abgelehnt() {
    for wert in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let host = HostRequirement {
            gpu_duty_cycle: wert,
            ..HOSTS[0]
        };
        assert_eq!(
            host.to_json(),
            Err(JsonError::NonFinite {
                field: "gpuDutyCycle"
            })
        );
    }
}

/// **AC5** — jede der fünf Garantiearten steht unterscheidbar im Text, und das
/// Teilergebnis nennt beide Zahlen.
///
/// Ein Teilergebnis, dessen Text nur die erledigte Arbeit nennt, liest sich
/// beim Zurücklesen wie ein fertiges Ergebnis. Deshalb steht `complete` mit im
/// Text — berechnet, nicht nachgetragen.
#[test]
fn jede_ergebnisgarantie_steht_unterscheidbar_im_text() {
    let arten = [
        ResultGuarantee::CertifiedExact,
        ResultGuarantee::BitExactOptimized,
        ResultGuarantee::BoundedApproximation {
            lower: 0.25,
            upper: 0.75,
        },
        ResultGuarantee::StatisticalEstimate { confidence: 0.99 },
        ResultGuarantee::AnytimePartial {
            work_completed: WorkUnits(400_000),
            work_planned: WorkUnits(1_000_000),
        },
    ];
    let texte: Vec<serde_json::Value> = arten
        .iter()
        .map(|g| {
            let text = g.to_json().expect("endliche Garantie");
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("kein JSON: {e}\n{text}"))
        })
        .collect();

    let kinds: Vec<&str> = texte
        .iter()
        .map(|v| v["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(
        kinds,
        vec![
            "certifiedExact",
            "bitExactOptimized",
            "boundedApproximation",
            "statisticalEstimate",
            "anytimePartial"
        ]
    );
    assert_eq!(texte[2]["lower"], 0.25);
    assert_eq!(texte[2]["upper"], 0.75);
    assert_eq!(texte[3]["confidence"], 0.99);
    assert_eq!(texte[4]["workCompleted"], 400_000);
    assert_eq!(texte[4]["workPlanned"], 1_000_000);

    // Nur das abgeschnittene Teilergebnis ist unvollständig.
    let vollstaendig: Vec<bool> = texte
        .iter()
        .map(|v| v["complete"].as_bool().expect("complete"))
        .collect();
    assert_eq!(vollstaendig, vec![true, true, true, true, false]);

    // GEGENPROBE: Ein Teilergebnis, das sein Budget erreicht hat, meldet sich
    // weiter als `anytimePartial` — aber vollständig.
    let erreicht = ResultGuarantee::AnytimePartial {
        work_completed: WorkUnits(1_000_000),
        work_planned: WorkUnits(1_000_000),
    };
    let v: serde_json::Value = serde_json::from_str(&erreicht.to_json().unwrap()).unwrap();
    assert_eq!(v["kind"], "anytimePartial");
    assert_eq!(v["complete"], true);
}

#[test]
fn nichtendliche_garantiewerte_werden_mit_feldname_abgelehnt() {
    for wert in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            ResultGuarantee::BoundedApproximation {
                lower: wert,
                upper: 1.0
            }
            .to_json(),
            Err(JsonError::NonFinite { field: "lower" })
        );
        assert_eq!(
            ResultGuarantee::BoundedApproximation {
                lower: 0.0,
                upper: wert
            }
            .to_json(),
            Err(JsonError::NonFinite { field: "upper" })
        );
        assert_eq!(
            ResultGuarantee::StatisticalEstimate { confidence: wert }.to_json(),
            Err(JsonError::NonFinite {
                field: "confidence"
            })
        );
    }
}
