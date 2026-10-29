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
    CampaignDefinition, CampaignRunContract, ComputeRequirement, CpuReservation, CpuRole,
    DateTimeUtc, DeadlinePolicy, HostRequirement, JsonError, NetworkRequirement, ReferenceProfile,
    ResultGuarantee, WorkUnits, REFERENCE_PROFILE_STANDARD,
};

const LAPTOP_RECHNET: ComputeRequirement = ComputeRequirement {
    guaranteed_cpu_quota: Some(1.0),
    cpu: CpuReservation {
        physical_cores: 8,
        threads_per_core: 2,
        reserved_physical_cores: 2,
    },
};

const LAPTOP: HostRequirement = HostRequirement {
    name: "laptop",
    cpu_role: CpuRole::Compute(LAPTOP_RECHNET),
    gpu_duty_cycle: 0.0,
    memory_limit_mib: 4096,
    thermal_throttling: false,
};

/// Der erste Testrechner mit anderen CPU-Angaben, sonst unverändert.
const fn rechnet(rechnen: ComputeRequirement) -> HostRequirement {
    HostRequirement {
        cpu_role: CpuRole::Compute(rechnen),
        ..LAPTOP
    }
}

static HOSTS: &[HostRequirement] = &[
    LAPTOP,
    // Der zweite Rechner hat ausdrücklich KEINE Zusicherung — der Fall, den ein
    // Profil mit nur einem Eintrag nie prüfen würde.
    HostRequirement {
        name: "best-effort",
        cpu_role: CpuRole::Compute(ComputeRequirement {
            guaranteed_cpu_quota: None,
            cpu: CpuReservation {
                physical_cores: 16,
                threads_per_core: 2,
                reserved_physical_cores: 2,
            },
        }),
        gpu_duty_cycle: 0.5,
        memory_limit_mib: 8192,
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

    let hosts = v["referenceProfile"]["hosts"]
        .as_array()
        .expect("hosts ist keine Liste");
    assert_eq!(hosts.len(), 2, "Beide Rechner müssen im Text stehen.");
    assert_eq!(hosts[0]["name"], "laptop");
    assert_eq!(hosts[0]["cpuRole"], "compute");
    assert_eq!(hosts[1]["cpuRole"], "compute");
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
    // Der Speicher ist eine Obergrenze und heißt im Text auch so. Ein Schlüssel
    // „guaranteed…“ behauptete eine Reservierung, die es nicht gibt.
    assert_eq!(hosts[0]["memoryLimitMib"], 4096);
    assert_eq!(hosts[1]["memoryLimitMib"], 8192);
    assert!(
        hosts[0].get("guaranteedMemoryMib").is_none(),
        "Der Text nennt den Speicher noch als zugesichert"
    );
}

/// **AC3** — im Profil „standard" rechnet nur node3 auf der CPU; die CPU des
/// Laptops koordiniert, seine GPU darf rechnen.
///
/// Zwei Entscheidungen des Auftraggebers vom 2026-10-28. Auf die Frage, ob der
/// Laptop weiter mitrechnet oder nur koordiniert: „Nur koordinieren" — bis dahin
/// rechnete er mit zwölf Strängen mit (Betriebsprofil `balanced`). Auf die
/// Frage, ob er außer Koordinieren rechnen darf: „NUR GPU" — GPU-Arbeit ja, auf
/// ausdrücklichen Aufruf; die CPU nur als Rückfall.
///
/// **Bewusst umgestellt.** Dieser Fall verlangte zuerst, dass der Koordinator
/// auch keine Grafik-Einschaltdauer führt — also „der Laptop rechnet gar
/// nichts". Das hat die zweite Entscheidung aufgehoben: Die Rolle gilt der CPU
/// (`cpuRole`), die Grafik-Einschaltdauer steht bei jedem Rechner.
///
/// Der Fall liest den **Text**, weil ein Leser des Profils die Rolle dort finden
/// muss. Ein Koordinator, der nur an fehlenden oder leeren Zahlen zu erkennen
/// wäre, sähe für einen Leser, der `null` als „best effort" liest, wie ein
/// rechnender Rechner ohne Zusicherung aus.
#[test]
fn ac3_im_profil_standard_rechnet_nur_node3_der_laptop_koordiniert() {
    let standard = CampaignDefinition {
        reference_profile: REFERENCE_PROFILE_STANDARD,
        ..kampagne()
    };
    let v: serde_json::Value =
        serde_json::from_str(&standard.to_json().expect("gültig")).expect("JSON");
    let hosts = v["referenceProfile"]["hosts"]
        .as_array()
        .expect("hosts ist keine Liste");
    assert_eq!(hosts.len(), 2, "{hosts:?}");

    let laptop = &hosts[0];
    assert_eq!(laptop["name"], "laptop");
    assert_eq!(laptop["cpuRole"], "coordinator", "{laptop}");
    assert!(
        laptop.get("role").is_none(),
        "Eine Rolle ohne „cpu“ läse sich als „rechnet gar nicht“: {laptop}"
    );
    assert_eq!(
        laptop["computeThreads"], 0,
        "Die CPU des Koordinators rechnet mit: {laptop}"
    );
    // Keine CPU-Quote für Rechenarbeit — auch keine leere. Ein `null` an dieser
    // Stelle hieße „best effort", also: rechnet ohne Zusicherung.
    for feld in [
        "guaranteedCpuQuota",
        "totalThreads",
        "physicalCores",
        "threadsPerCore",
        "reservedPhysicalCores",
    ] {
        assert!(
            laptop.get(feld).is_none(),
            "Der Koordinator führt {feld}: {laptop}"
        );
    }
    // Die GPU darf rechnen, also steht ihre Einschaltdauer da — als Zahl. Im
    // Referenzlauf ist sie 0: Es gibt keinen GPU-Pfad, und GPU-Arbeit läuft nur
    // auf ausdrücklichen Aufruf.
    assert_eq!(
        laptop["gpuDutyCycle"], 0.0,
        "Der Koordinator führt keine Grafik-Einschaltdauer: {laptop}"
    );
    assert_eq!(laptop["memoryLimitMib"], 4096);
    assert_eq!(laptop["thermalThrottling"], false);

    // GEGENPROBE im selben Text: Der rechnende Rechner trägt seine Angaben
    // weiter. Ohne sie wäre der Fall auch grün, wenn `to_json` die
    // Rechenangaben für JEDEN Rechner wegließe.
    let node3 = &hosts[1];
    assert_eq!(node3["name"], "node3");
    assert_eq!(node3["cpuRole"], "compute", "{node3}");
    assert_eq!(node3["gpuDutyCycle"], 0.0);
    assert_eq!(node3["computeThreads"], 16);
    assert_eq!(node3["totalThreads"], 16);
    assert_eq!(node3["reservedPhysicalCores"], 0);
    assert!(
        node3["guaranteedCpuQuota"].is_null(),
        "node3 ist best effort: {node3}"
    );

    let straenge: u64 = hosts
        .iter()
        .map(|h| {
            h["computeThreads"]
                .as_u64()
                .expect("computeThreads ist keine Zahl")
        })
        .sum();
    assert_eq!(
        straenge, 16,
        "Im Profil rechnen mehr als die Stränge von node3"
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
}

/// Der Vertrag ohne Frist schreibt `null` — nicht eine sehr grosse Zahl (AC5).
#[test]
fn ohne_frist_steht_null_im_vertrag() {
    let v0 = CampaignRunContract::from_campaign(&kampagne());
    let v: serde_json::Value = serde_json::from_str(&v0.to_json()).expect("kein gültiges JSON");

    assert!(
        v["hardDeadline"].is_null(),
        "Ohne ausdrücklichen Wunsch steht eine Frist im Vertrag: {}",
        v["hardDeadline"]
    );
    assert_eq!(v["campaignDefinitionHash"], kampagne().fingerprint().get());
    assert_eq!(v["deterministicWorkBudget"], 1_234_567);
    assert_eq!(v["targetWallTime"]["secs"], 7200);
    assert_eq!(v["targetWallTime"]["nanos"], 5);
}

/// Beide Fristarten sind im Text unterscheidbar (AC6).
///
/// Zwei Fristen, die als dieselbe Zeile erscheinen, wären beim Zurücklesen
/// nicht mehr auseinanderzuhalten — und die Bedeutung ist genau das, was AC6
/// benannt haben will.
#[test]
fn beide_fristarten_stehen_unterscheidbar_im_text() {
    let basis = CampaignRunContract::from_campaign(&kampagne());

    let wanduhr = basis.with_hard_deadline(DeadlinePolicy::WallClockUtc {
        deadline: DateTimeUtc {
            unix_seconds: 1_800_000_000,
        },
    });
    let rechenzeit = basis.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: Duration::new(7200, 123_456_789),
    });

    let a: serde_json::Value = serde_json::from_str(&wanduhr.to_json()).unwrap();
    let b: serde_json::Value = serde_json::from_str(&rechenzeit.to_json()).unwrap();

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
}

#[test]
fn nichtendliche_cpu_quote_ist_keine_fehlende_zusicherung() {
    for wert in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let host = rechnet(ComputeRequirement {
            guaranteed_cpu_quota: Some(wert),
            ..LAPTOP_RECHNET
        });
        assert_eq!(
            host.to_json(),
            Err(JsonError::NonFinite {
                field: "guaranteedCpuQuota"
            })
        );
    }
    static UNGUELTIG: &[HostRequirement] = &[rechnet(ComputeRequirement {
        guaranteed_cpu_quota: Some(f64::NAN),
        ..LAPTOP_RECHNET
    })];
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
        let host = rechnet(ComputeRequirement {
            guaranteed_cpu_quota: Some(wert),
            ..LAPTOP_RECHNET
        });
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

/// Gilt für jede CPU-Rolle: Auch ein Koordinator schreibt seine
/// Grafik-Einschaltdauer, also darf auch seine kein `NaN` sein.
#[test]
fn nichtendliche_grafik_einschaltdauer_wird_abgelehnt() {
    for cpu_role in [CpuRole::Compute(LAPTOP_RECHNET), CpuRole::Coordinator] {
        for wert in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let host = HostRequirement {
                cpu_role,
                gpu_duty_cycle: wert,
                ..LAPTOP
            };
            assert_eq!(
                host.to_json(),
                Err(JsonError::NonFinite {
                    field: "gpuDutyCycle"
                }),
                "{cpu_role:?}"
            );
        }
    }
}

/// Die GPU eines Koordinators darf rechnen (Entscheidung des Auftraggebers
/// „NUR GPU", 2026-10-28) — ihr Anteil steht dann im Text, die CPU-Angaben
/// nicht.
#[test]
fn ein_koordinator_mit_gpu_schreibt_ihre_einschaltdauer() {
    let host = HostRequirement {
        cpu_role: CpuRole::Coordinator,
        gpu_duty_cycle: 0.25,
        ..LAPTOP
    };
    let gelesen: serde_json::Value =
        serde_json::from_str(&host.to_json().expect("endlich")).expect("JSON");
    assert_eq!(gelesen["cpuRole"], "coordinator");
    assert_eq!(gelesen["computeThreads"], 0);
    assert_eq!(gelesen["gpuDutyCycle"], 0.25, "{gelesen}");
    assert!(gelesen.get("guaranteedCpuQuota").is_none(), "{gelesen}");
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
