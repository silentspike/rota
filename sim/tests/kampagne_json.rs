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

use rota_sim::kampagne::{
    AuditPolicy, CampaignDefinition, CampaignRunContract, ConfidenceSpec, CoverageSpec,
    CpuReservation, DeadlinePolicy, HostRequirement, Mission, ProjectionSet, QualityTargets,
    ReferenceProfile, WorkUnits,
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
        max_rtt_ms: 5,
        min_bandwidth_mibit_s: 100,
        thermal_throttling: false,
    },
    // Der zweite Rechner hat ausdrücklich KEINE Zusicherung — der Fall, den ein
    // Profil mit nur einem Eintrag nie prüfen würde.
    HostRequirement {
        name: "gonet",
        guaranteed_cpu_quota: None,
        guaranteed_memory_mib: 8192,
        cpu: CpuReservation {
            physical_cores: 16,
            threads_per_core: 2,
            reserved_physical_cores: 2,
        },
        max_rtt_ms: 20,
        min_bandwidth_mibit_s: 1000,
        thermal_throttling: false,
    },
];

fn kampagne() -> CampaignDefinition {
    CampaignDefinition {
        profile_version: 3,
        reference_profile: ReferenceProfile {
            version: 2,
            hosts: HOSTS,
        },
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
    let text = k.to_json();
    let v: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("kein gültiges JSON: {e}\n{text}"));

    assert_eq!(v["profileVersion"], 3);
    assert_eq!(v["fingerprint"], k.fingerprint());
    assert_eq!(v["referenceProfile"]["version"], 2);
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
}

/// **Ein fehlender Wert wird `null`, nicht `0`.**
///
/// Drei Aussagen, die im JSON auseinandergehalten werden müssen:
/// `null` = ausdrücklich best effort · eine Zahl = Zusicherung · `0.0` =
/// garantiert nichts. Wer den fehlenden Wert als Null schreibt, macht aus der
/// ersten die dritte.
#[test]
fn ein_fehlender_wert_wird_null_und_nicht_null_komma_null() {
    let text = kampagne().to_json();
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
    let v2: serde_json::Value = serde_json::from_str(&mit_null.to_json()).unwrap();
    assert_eq!(
        v2["maximumBoundGap"], 0.0,
        "Eine ausdrücklich verlangte Schranke von 0.0 wurde zu null."
    );
    assert!(!v2["maximumBoundGap"].is_null());
}

/// Der Vertrag ohne Frist schreibt `null` — nicht eine sehr grosse Zahl (AC5).
#[test]
fn ohne_frist_steht_null_im_vertrag() {
    let v0 = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance, 7_200_000);
    let v: serde_json::Value = serde_json::from_str(&v0.to_json()).expect("kein gültiges JSON");

    assert!(
        v["hardDeadline"].is_null(),
        "Ohne ausdrücklichen Wunsch steht eine Frist im Vertrag: {}",
        v["hardDeadline"]
    );
    assert_eq!(v["campaignDefinitionHash"], kampagne().fingerprint());
    assert_eq!(v["deterministicWorkBudget"], 1_234_567);
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
    let basis = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance, 7_200_000);

    let wanduhr = basis.with_hard_deadline(DeadlinePolicy::WallClockUtc {
        deadline_unix_s: 1_800_000_000,
    });
    let rechenzeit = basis.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining_ms: 7_200_000,
    });

    let a: serde_json::Value = serde_json::from_str(&wanduhr.to_json()).unwrap();
    let b: serde_json::Value = serde_json::from_str(&rechenzeit.to_json()).unwrap();

    assert_eq!(a["hardDeadline"]["kind"], "wallClockUtc");
    assert_eq!(a["hardDeadline"]["deadlineUnixS"], 1_800_000_000_i64);
    assert_eq!(b["hardDeadline"]["kind"], "activeRuntime");
    assert_eq!(b["hardDeadline"]["remainingMs"], 7_200_000);
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
        let vertrag = CampaignRunContract::from_campaign(&k, m, 7_200_000);
        serde_json::from_str(&vertrag.to_json()).expect("kein gültiges JSON")
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
