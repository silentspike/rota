//! Das Arbeitsbudget gehört zur Kampagne, nicht zur Tagesform (Issue #914).
//!
//! # Was diese Prüfstrecke belegt — und was sie NICHT belegt
//!
//! Sie prüft die **Strukturen**, weil dieses Issue Strukturen baut und
//! ausdrücklich keinen Rechenweg (Umsetzungsgrenze im Issue). Konkret heißt das
//! für das schwerste Kriterium:
//!
//! | AC2 verlangt | hier geprüft |
//! | --- | --- |
//! | drei echte Läufe mit einem, zwei, drei Rechnern | drei **Deployments** mit einem, zwei, drei Rechnern |
//! | gleicher Ergebnis-Fingerabdruck | gleicher **Kampagnen**-Fingerabdruck |
//!
//! **Nicht belegt ist damit, dass ein echter verteilter Lauf dasselbe
//! Ergebnis liefert.** Das kann erst das Verteiler-Issue, das den Rechenweg
//! baut; hier gibt es keinen, der abweichen könnte. Belegt ist die Eigenschaft,
//! an der es hinge: dass die Arbeitsmenge das Deployment nicht kennt.
//!
//! Der Unterschied ist keine Formalie. Wer ihn wegließe, hätte einen grünen
//! Haken an einem Kriterium, dessen eigentliche Frage offen bleibt.

use std::time::Duration;

use rota_sim::kampagne::{
    AuditPolicy, CampaignDefinition, CampaignRunContract, ConfidenceSpec, CoverageSpec,
    CpuReservation, DateTimeUtc, DeadlinePolicy, DeployedHost, Hash, HostRequirement, Mission,
    NetworkRequirement, ProjectionSet, QualityTargets, ReferenceProfile, ResultGuarantee,
    RunDeployment, RuntimeAccounting, WorkUnits, REFERENCE_PROFILE_STANDARD,
};

const LAPTOP: CpuReservation = CpuReservation {
    physical_cores: 8,
    threads_per_core: 2,
    reserved_physical_cores: 2,
};

const REFERENZ_HOST: HostRequirement = HostRequirement {
    name: "laptop",
    guaranteed_cpu_quota: Some(1.0),
    guaranteed_memory_mib: 4096,
    cpu: LAPTOP,
    gpu_duty_cycle: 0.0,
    thermal_throttling: false,
};

const NETZ: NetworkRequirement = NetworkRequirement {
    max_rtt_ms: 5,
    min_bandwidth_mibit_s: Some(100),
};

const ZWEI_STUNDEN: Duration = Duration::from_secs(7200);

static PROFIL_HOSTS: &[HostRequirement] = &[REFERENZ_HOST];

fn kampagne() -> CampaignDefinition {
    CampaignDefinition {
        profile_version: 1,
        reference_profile: ReferenceProfile {
            name: "pruefprofil",
            version: 1,
            hosts: PROFIL_HOSTS,
            network: Some(NETZ),
        },
        target_wall_time: ZWEI_STUNDEN,
        canonical_work_budget: WorkUnits(1_000_000),
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

fn host(name: &'static str, kerne: u16) -> DeployedHost {
    DeployedHost {
        name,
        cgroup_quota: 1.0,
        runtime_perf_profile: 1.0,
        cpu: CpuReservation {
            physical_cores: kerne,
            threads_per_core: 2,
            reserved_physical_cores: 2,
        },
    }
}

/// Ein, zwei, drei Rechner — mit ABSICHTLICH verschiedener Ausstattung, damit
/// die Deployments sich nicht nur in der Länge unterscheiden.
fn deployments() -> [RunDeployment; 3] {
    [
        RunDeployment {
            available_hosts: vec![host("laptop", 8)],
            estimated_eta: Duration::from_secs(7200),
        },
        RunDeployment {
            available_hosts: vec![host("laptop", 8), host("node3", 12)],
            estimated_eta: Duration::from_secs(3600),
        },
        RunDeployment {
            available_hosts: vec![host("laptop", 8), host("node3", 12), host("dritter", 16)],
            estimated_eta: Duration::from_secs(2400),
        },
    ]
}

/// **AC2** — dieselbe Kampagne, drei Rechnerzahlen, ein Fingerabdruck.
#[test]
fn ac2_der_fingerabdruck_haengt_nicht_am_deployment() {
    let k = kampagne();
    let d = deployments();

    // ZUSICHERUNG: Die drei Deployments müssen sich WIRKLICH unterscheiden.
    // Ohne sie verglichen wir dreimal dasselbe, und die Gleichheit wäre trivial
    // wahr — der Fall bliebe grün, auch wenn jemand alle drei gleich baut.
    let stränge: Vec<u64> = d.iter().map(RunDeployment::total_compute_threads).collect();
    assert_eq!(
        stränge,
        vec![12, 12 + 20, 12 + 20 + 28],
        "Die drei Deployments unterscheiden sich nicht wie erwartet — dann \
         prüft dieser Fall nichts."
    );

    let abdruecke: Vec<Hash> = d
        .iter()
        .map(|_| {
            CampaignRunContract::from_campaign(&k, Mission::FullBalance).campaign_definition_hash
        })
        .collect();
    assert_eq!(
        abdruecke[0], abdruecke[1],
        "ein Rechner gegen zwei: verschiedene Fingerabdrücke"
    );
    assert_eq!(
        abdruecke[1], abdruecke[2],
        "zwei Rechner gegen drei: verschiedene Fingerabdrücke"
    );

    // Und die Arbeitsmenge selbst ist dieselbe, nicht nur ihr Abdruck.
    for d_i in &d {
        let vertrag = CampaignRunContract::from_campaign(&k, Mission::FullBalance);
        assert_eq!(
            vertrag.deterministic_work_budget,
            k.canonical_work_budget,
            "Bei {} Strängen weicht das Budget ab.",
            d_i.total_compute_threads()
        );
    }
}

/// **AC2, Kontrollprobe** — ein veränderter Vertrag MUSS einen anderen
/// Fingerabdruck liefern.
///
/// Das Issue verlangt sie ausdrücklich: „Bleibt er gleich, prüft AC2 nichts."
/// Ein Fingerabdruck, der immer denselben Wert liefert, bestünde den Fall
/// darüber mühelos.
///
/// Geprüft wird **jedes** Feld einzeln, nicht eines stellvertretend. Ein Feld,
/// das nicht eingeht, ist genau die stille Lücke, die später jemanden ein Feld
/// ändern lässt, ohne dass sich der Abdruck rührt.
#[test]
fn ac2_kontrolle_jedes_feld_aendert_den_fingerabdruck() {
    let basis = kampagne().fingerprint();

    let mut abweichungen: Vec<(&str, Hash)> = Vec::new();

    let mut k = kampagne();
    k.profile_version = 2;
    abweichungen.push(("profile_version", k.fingerprint()));

    let mut k = kampagne();
    k.reference_profile.version = 2;
    abweichungen.push(("reference_profile.version", k.fingerprint()));

    let mut k = kampagne();
    k.target_wall_time = Duration::from_secs(7201);
    abweichungen.push(("target_wall_time", k.fingerprint()));

    // Die Nanosekunden gehen getrennt ein: Eine Zielzeit, die nur Sekunden
    // einrührte, hielte 2 h und 2 h + 1 ns für dasselbe Profil.
    let mut k = kampagne();
    k.target_wall_time = ZWEI_STUNDEN + Duration::from_nanos(1);
    abweichungen.push(("target_wall_time nanos", k.fingerprint()));

    let mut k = kampagne();
    k.canonical_work_budget = WorkUnits(1_000_001);
    abweichungen.push(("canonical_work_budget", k.fingerprint()));

    let mut k = kampagne();
    k.exact_obligations.required = 13;
    abweichungen.push(("exact_obligations", k.fingerprint()));

    let mut k = kampagne();
    k.coverage_spec.strength = 3;
    abweichungen.push(("coverage_spec", k.fingerprint()));

    let mut k = kampagne();
    k.quality_targets.minimum_niche_coverage = 0.96;
    abweichungen.push(("minimum_niche_coverage", k.fingerprint()));

    let mut k = kampagne();
    k.quality_targets.independent_restarts = 9;
    abweichungen.push(("independent_restarts", k.fingerprint()));

    let mut k = kampagne();
    k.quality_targets.plateau_work_units = WorkUnits(50_001);
    abweichungen.push(("plateau_work_units", k.fingerprint()));

    let mut k = kampagne();
    k.quality_targets.maximum_bound_gap = Some(0.0);
    abweichungen.push(("maximum_bound_gap None->Some(0.0)", k.fingerprint()));

    let mut k = kampagne();
    k.quality_targets.confidence_requirement.level = 0.999;
    abweichungen.push(("confidence_requirement", k.fingerprint()));

    let mut k = kampagne();
    k.quality_targets.final_audit = AuditPolicy::ScalarReplayDecisive;
    abweichungen.push(("final_audit", k.fingerprint()));

    for (feld, abdruck) in &abweichungen {
        assert_ne!(
            *abdruck, basis,
            "Das Feld {feld} geht nicht in den Fingerabdruck ein — es lässt \
             sich ändern, ohne dass der Abdruck es meldet."
        );
    }

    // Die Zahl steht in der MELDUNG, nicht in der Bedingung: Eine feste Zahl
    // hier bräche bei jedem neuen Feld, statt es zu prüfen.
    assert_eq!(
        abweichungen.len(),
        13,
        "Es wurden {} Felder geprüft. Kommt ein Feld zur Kampagne dazu, gehört \
         es hier dazu — sonst wächst die Struktur schneller als ihr Schutz.",
        abweichungen.len()
    );
}

/// **AC2, die schärfere Kontrolle** — `Some(0.0)` und `None` sind verschiedene
/// Aussagen.
///
/// „Keine Schranke verlangt" gegen „Schranke muss exakt eingehalten werden".
/// Ein Fingerabdruck, der den fehlenden Wert wie die 0.0 behandelt, wirft beide
/// zusammen — und der Unterschied fällt erst auf, wenn ein Lauf eine Schranke
/// einhält, die nie verlangt war.
#[test]
fn ac2_fehlender_wert_ist_nicht_der_wert_null() {
    let mut ohne = kampagne();
    ohne.quality_targets.maximum_bound_gap = None;
    let mut mit_null = kampagne();
    mit_null.quality_targets.maximum_bound_gap = Some(0.0);

    assert_ne!(
        ohne.fingerprint(),
        mit_null.fingerprint(),
        "None und Some(0.0) ergeben denselben Fingerabdruck."
    );
}

/// **AC4** — „Kerne minus zwei" ist dreideutig, die Angabe hier ist es nicht.
///
/// Bei 8 physischen Kernen und 16 Strängen sind die drei möglichen Lesarten
/// sechs, vierzehn und zwölf. Verbindlich ist **zwölf**: zwei physische Kerne
/// samt ihren SMT-Geschwistern.
#[test]
fn ac4_die_reservierung_nimmt_die_smt_geschwister_mit() {
    assert_eq!(LAPTOP.total_threads(), 16, "8 Kerne mal 2 Stränge");
    assert_eq!(
        LAPTOP.compute_threads(),
        12,
        "Zwei reservierte Kerne nehmen ihre Geschwister mit: 16 minus 4, nicht \
         minus 2. Kommt hier 14 heraus, werden nur zwei STRÄNGE reserviert."
    );

    // Eine Maschine ohne SMT: dort sind alle drei Lesarten dieselbe Zahl, und
    // deshalb belegt sie nichts — sie steht hier als Gegenprobe, dass die
    // Rechnung nicht einfach immer durch zwei teilt.
    let ohne_smt = CpuReservation {
        physical_cores: 8,
        threads_per_core: 1,
        reserved_physical_cores: 2,
    };
    assert_eq!(ohne_smt.compute_threads(), 6);
}

/// Die Struktur kennt nur Anzahlen. AC4s tatsächliche CPU-IDs bleiben ohne
/// Topologie und Affinitätsnachweis offen; die Anzeige darf sie nicht erfinden.
#[test]
fn ac4_die_anzeige_trennt_rechenvorgabe_von_unbekannter_kernbindung() {
    let text = LAPTOP.describe();
    assert!(text.contains("12 Stränge (rechnerisch)"), "{text}");
    assert!(
        text.contains("2 von 8 physischen Kernen samt SMT-Geschwistern"),
        "{text}"
    );
    assert!(
        text.contains("CPU-IDs und tatsächliche Kernbindung: nicht ermittelt"),
        "{text}"
    );
    assert!(!text.contains("0-11") && !text.contains("6-7"), "{text}");
}

/// **AC5** — ein Zeitabbruch ist nicht die Voreinstellung.
#[test]
fn ac5_ohne_ausdruecklichen_wunsch_gibt_es_keine_frist() {
    let vertrag = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance);
    assert!(
        vertrag.hard_deadline.is_none(),
        "Der abgeleitete Vertrag bringt eine Frist mit, ohne dass sie verlangt \
         wurde."
    );

    let mit = vertrag.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: ZWEI_STUNDEN,
    });
    assert!(
        mit.hard_deadline.is_some(),
        "Eine ausdrücklich verlangte Frist kommt nicht an."
    );
}

/// **AC5** — das Teilergebnis meldet sich als solches und nennt beide Zahlen.
#[test]
fn ac5_ein_abgeschnittener_lauf_traegt_eine_eigene_garantie() {
    let teil = ResultGuarantee::AnytimePartial {
        work_completed: WorkUnits(400_000),
        work_planned: WorkUnits(1_000_000),
    };
    assert!(
        !teil.is_complete(),
        "Ein Lauf mit 400k von 1M erledigten Einheiten gilt als vollständig."
    );

    let ganz = ResultGuarantee::AnytimePartial {
        work_completed: WorkUnits(1_000_000),
        work_planned: WorkUnits(1_000_000),
    };
    assert!(
        ganz.is_complete(),
        "Ein Lauf, der sein Budget erreicht hat, gilt als unvollständig — dann \
         wäre jeder Lauf mit Frist ein Teilergebnis, auch der folgenlose."
    );

    // GEGENPROBE: Die anderen vier Arten dürfen nicht plötzlich unvollständig
    // sein. Ohne sie wäre `is_complete` auch dann grün, wenn es schlicht
    // `matches!(self, AnytimePartial{..})` verneinte. Bis #914 standen hier
    // nur drei der vier — `BoundedApproximation` fehlte.
    assert!(ResultGuarantee::CertifiedExact.is_complete());
    assert!(ResultGuarantee::BitExactOptimized.is_complete());
    assert!(ResultGuarantee::BoundedApproximation {
        lower: 1.0,
        upper: 2.0
    }
    .is_complete());
    assert!(ResultGuarantee::StatisticalEstimate { confidence: 0.5 }.is_complete());
}

/// **AC6** — die Bedeutung der Frist ist benannt, und die drei Festlegungen
/// stehen im Vertrag.
#[test]
fn ac6_beide_fristarten_sind_unterscheidbar_und_die_zaehlung_steht_fest() {
    let wanduhr = DeadlinePolicy::WallClockUtc {
        deadline: DateTimeUtc {
            unix_seconds: 1_800_000_000,
        },
    };
    let rechenzeit = DeadlinePolicy::ActiveRuntime {
        remaining: ZWEI_STUNDEN,
    };
    assert_ne!(
        wanduhr, rechenzeit,
        "Die beiden Fristarten sind nicht unterscheidbar."
    );

    let z = RuntimeAccounting::REPRODUCIBLE;
    assert!(!z.counts_paused_time, "Pausezeit zählt mit.");
    assert!(
        !z.counts_coordinator_outage,
        "Ein Koordinatorausfall zählt mit."
    );
    assert!(!z.counts_host_wait, "Wartezeit auf Rechner zählt mit.");
}

/// **AC7** — die Zielzeit gehört zur Kampagne, und eine andere ist ein anderes
/// Profil.
///
/// Bis #914 nahm `from_campaign` die Zielzeit als Parameter: Zwei Verträge mit
/// zwei und vier Stunden entstanden aus DERSELBEN versionierten Kampagne. Das
/// Budget blieb dabei gleich, aber nichts sagte, für welche Zielzeit es
/// bemessen war. Jetzt übernimmt der Vertrag sie, und eine andere Zielzeit
/// ändert den Fingerabdruck der Kampagne — das Versionieren und den Schutz
/// prüft `tests/kampagne_versionen.rs`.
#[test]
fn ac7_die_zielzeit_kommt_aus_der_kampagne_und_ist_teil_ihrer_identitaet() {
    let zwei = kampagne();
    let mut vier = kampagne();
    vier.target_wall_time = Duration::from_secs(14_400);

    let v_zwei = CampaignRunContract::from_campaign(&zwei, Mission::FullBalance);
    let v_vier = CampaignRunContract::from_campaign(&vier, Mission::FullBalance);
    assert_eq!(v_zwei.target_wall_time, ZWEI_STUNDEN);
    assert_eq!(v_vier.target_wall_time, Duration::from_secs(14_400));
    assert_eq!(
        v_zwei.deterministic_work_budget, v_vier.deterministic_work_budget,
        "Die doppelte Zielzeit hat das Budget verändert. Genau das ist der \
         Fehler, gegen den dieses Issue steht."
    );
    assert_ne!(
        v_zwei.campaign_definition_hash, v_vier.campaign_definition_hash,
        "Zwei Zielzeiten, ein Fingerabdruck — dann wären es dasselbe Profil."
    );
}

/// **AC0/AC1** — die Trennung ist an der Signatur ablesbar, nicht nur an der
/// Absicht.
///
/// Der Vertrag entsteht aus der Kampagne allein. Gäbe es einen
/// Deployment-Parameter, könnte eine spätere Zeile ihn einrühren, und niemand
/// sähe es dem Aufruf an.
#[test]
fn ac1_der_vertrag_entsteht_aus_der_kampagne_allein() {
    let k = kampagne();
    let vertrag = CampaignRunContract::from_campaign(&k, Mission::FullBalance);

    // Jedes Feld der Kampagne hat einen Leser im Vertrag (AC0).
    assert_eq!(vertrag.target_wall_time, k.target_wall_time);
    assert_eq!(vertrag.deterministic_work_budget, k.canonical_work_budget);
    assert_eq!(vertrag.exact_obligations, k.exact_obligations);
    assert_eq!(vertrag.interaction_coverage, k.coverage_spec);
    assert_eq!(
        vertrag.minimum_niche_coverage,
        k.quality_targets.minimum_niche_coverage
    );
    assert_eq!(
        vertrag.independent_restarts,
        k.quality_targets.independent_restarts
    );
    assert_eq!(
        vertrag.plateau_work_units,
        k.quality_targets.plateau_work_units
    );
    assert_eq!(
        vertrag.maximum_bound_gap,
        k.quality_targets.maximum_bound_gap
    );
    assert_eq!(
        vertrag.confidence_requirement,
        k.quality_targets.confidence_requirement
    );
    assert_eq!(vertrag.final_audit, k.quality_targets.final_audit);
    assert_eq!(vertrag.campaign_definition_hash, k.fingerprint());
    assert_eq!(vertrag.mission, Mission::FullBalance);
}

/// **AC3** — das Referenzprofil nennt Mindestressourcen, nicht Erreichbarkeit.
///
/// Der Unterschied zwischen `None` und einer Zahl ist hier inhaltlich:
/// ausdrückliches „best effort" gegen eine Zusicherung. Ein Profil, das nur
/// Namen führte, bestünde diesen Fall nicht.
#[test]
fn ac3_das_referenzprofil_nennt_mindestressourcen() {
    let profil = kampagne().reference_profile;
    assert!(!profil.hosts.is_empty(), "Das Profil führt keinen Rechner.");

    for h in profil.hosts {
        assert!(
            h.guaranteed_memory_mib > 0,
            "{}: kein Speicherbudget — dann sagt der Eintrag nur, dass es die \
             Maschine gibt.",
            h.name
        );
        assert!(
            h.cpu.compute_threads() > 0,
            "{}: keine nutzbaren Stränge",
            h.name
        );
        assert!(
            !h.thermal_throttling,
            "{}: Das Referenzprofil erlaubt aktives Drosseln. Dann leistet die \
             Referenzmaschine je nach Tag verschieden viel, und ein daraus \
             abgeleitetes Budget ist genau das, was dieses Issue verhindert.",
            h.name
        );
    }
}

/// Eine gleiche Versionsnummer darf geänderte Herkunftsdaten nicht verdecken.
/// Jeder einzelne Inhalt des Referenzrechners beeinflusst die Identität;
/// das aktuelle Deployment gehört weiterhin nicht dazu (Kontrolle oben).
#[test]
fn ac2_jedes_referenzfeld_aendert_den_fingerabdruck_ohne_neue_versionsnummer() {
    static VARIANTEN: &[(&str, HostRequirement)] = &[
        (
            "name",
            HostRequirement {
                name: "node3",
                ..REFERENZ_HOST
            },
        ),
        (
            "quota",
            HostRequirement {
                guaranteed_cpu_quota: Some(2.0),
                ..REFERENZ_HOST
            },
        ),
        (
            "best effort",
            HostRequirement {
                guaranteed_cpu_quota: None,
                ..REFERENZ_HOST
            },
        ),
        (
            "memory",
            HostRequirement {
                guaranteed_memory_mib: 8192,
                ..REFERENZ_HOST
            },
        ),
        (
            "physical cores",
            HostRequirement {
                cpu: CpuReservation {
                    physical_cores: 9,
                    ..LAPTOP
                },
                ..REFERENZ_HOST
            },
        ),
        (
            "threads per core",
            HostRequirement {
                cpu: CpuReservation {
                    threads_per_core: 1,
                    ..LAPTOP
                },
                ..REFERENZ_HOST
            },
        ),
        (
            "reserved cores",
            HostRequirement {
                cpu: CpuReservation {
                    reserved_physical_cores: 1,
                    ..LAPTOP
                },
                ..REFERENZ_HOST
            },
        ),
        (
            "gpu duty cycle",
            HostRequirement {
                gpu_duty_cycle: 0.25,
                ..REFERENZ_HOST
            },
        ),
        (
            "throttling",
            HostRequirement {
                thermal_throttling: true,
                ..REFERENZ_HOST
            },
        ),
    ];
    let basis = kampagne();
    for (feld, host) in VARIANTEN {
        let mut anders = basis;
        anders.reference_profile.hosts = std::slice::from_ref(host);
        assert_ne!(
            anders.fingerprint(),
            basis.fingerprint(),
            "Referenzfeld {feld} fehlt"
        );
        assert_eq!(anders.canonical_work_budget, basis.canonical_work_budget);
    }
}

#[test]
fn ac2_referenzrechner_anzahl_und_fehlende_quote_bleiben_unterscheidbar() {
    static DOPPELT: &[HostRequirement] = &[REFERENZ_HOST, REFERENZ_HOST];
    static OHNE_QUOTE: &[HostRequirement] = &[HostRequirement {
        guaranteed_cpu_quota: None,
        ..REFERENZ_HOST
    }];
    static NULL_QUOTE: &[HostRequirement] = &[HostRequirement {
        guaranteed_cpu_quota: Some(0.0),
        ..REFERENZ_HOST
    }];
    let basis = kampagne();
    let mut doppelt = basis;
    doppelt.reference_profile.hosts = DOPPELT;
    assert_ne!(doppelt.fingerprint(), basis.fingerprint());
    let mut leer = basis;
    leer.reference_profile.hosts = &[];
    assert_ne!(leer.fingerprint(), basis.fingerprint());
    let mut ohne = basis;
    ohne.reference_profile.hosts = OHNE_QUOTE;
    let mut null = basis;
    null.reference_profile.hosts = NULL_QUOTE;
    assert_ne!(ohne.fingerprint(), null.fingerprint());
}

#[test]
fn ac4_leere_und_ueberreservierte_vorgaben_erfinden_keine_cpu_bereiche() {
    for (kerne, reserviert) in [(0, 0), (8, 8), (8, 9), (0, u16::MAX)] {
        let cpu = CpuReservation {
            physical_cores: kerne,
            threads_per_core: 2,
            reserved_physical_cores: reserviert,
        };
        assert_eq!(cpu.compute_threads(), 0);
        let text = cpu.describe();
        assert!(text.contains("Compute: 0 Stränge (rechnerisch)"), "{text}");
        assert!(!text.contains("CPUs 0-"), "{text}");
        assert_eq!(
            text.contains("mehr Kerne reserviert als vorhanden"),
            reserviert > kerne
        );
    }
    let ohne_smt_anzahl = CpuReservation {
        threads_per_core: 0,
        ..LAPTOP
    };
    assert!(ohne_smt_anzahl.describe().contains("null Stränge"));
    let ohne_reservierung = CpuReservation {
        reserved_physical_cores: 0,
        ..LAPTOP
    };
    assert!(ohne_reservierung
        .describe()
        .contains("0 von 8 physischen Kernen"));
}

#[test]
fn ac4_strangzahlen_werden_vor_der_multiplikation_verbreitert() {
    let cpu = CpuReservation {
        physical_cores: u16::MAX,
        threads_per_core: u16::MAX,
        reserved_physical_cores: 0,
    };
    assert_eq!(cpu.total_threads(), 4_294_836_225);
    assert_eq!(cpu.compute_threads(), 4_294_836_225);
    assert!(cpu.describe().contains("4294836225 Stränge"));
    let mut host = host("gross", 1);
    host.cpu = cpu;
    let deployment = RunDeployment {
        available_hosts: vec![host, host],
        estimated_eta: Duration::ZERO,
    };
    assert_eq!(deployment.total_compute_threads(), 8_589_672_450);
}

/// Name und Netz des Referenzprofils gehören zur Identität wie seine Rechner.
///
/// Das Netz steht seit #914 am Profil und nicht mehr an jedem Rechner: AC3
/// führt es als eigene Zeile, und im Profil „standard" gibt es genau eine
/// Verbindung — vom Laptop, auf dem der Koordinator läuft, zu node3.
#[test]
fn ac2_name_und_netz_des_referenzprofils_aendern_den_fingerabdruck() {
    let basis = kampagne();
    let mut varianten: Vec<(&str, CampaignDefinition)> = Vec::new();

    let mut k = basis;
    k.reference_profile.name = "anderes";
    varianten.push(("name", k));
    let mut k = basis;
    k.reference_profile.network = Some(NetworkRequirement {
        max_rtt_ms: 6,
        ..NETZ
    });
    varianten.push(("rtt", k));
    let mut k = basis;
    k.reference_profile.network = Some(NetworkRequirement {
        min_bandwidth_mibit_s: Some(101),
        ..NETZ
    });
    varianten.push(("bandwidth", k));
    let mut k = basis;
    k.reference_profile.network = Some(NetworkRequirement {
        min_bandwidth_mibit_s: None,
        ..NETZ
    });
    varianten.push(("bandwidth unbekannt", k));
    let mut k = basis;
    k.reference_profile.network = None;
    varianten.push(("kein Netz", k));

    for (feld, k) in &varianten {
        assert_ne!(
            k.fingerprint(),
            basis.fingerprint(),
            "Profilfeld {feld} fehlt im Fingerabdruck"
        );
        assert_eq!(k.canonical_work_budget, basis.canonical_work_budget);
    }

    // `None` und ein Netz aus Nullen sind verschiedene Aussagen: „kein Netz im
    // Profil" gegen „null Millisekunden Umlaufzeit verlangt".
    let mut ohne = basis;
    ohne.reference_profile.network = None;
    let mut nullen = basis;
    nullen.reference_profile.network = Some(NetworkRequirement {
        max_rtt_ms: 0,
        min_bandwidth_mibit_s: Some(0),
    });
    assert_ne!(ohne.fingerprint(), nullen.fingerprint());

    // Dasselbe für die Bandbreite allein: „nicht gemessen" gegen „null
    // verlangt" — die zweite wäre eine Vorgabe, die erste ist keine.
    let mut unbekannt = basis;
    unbekannt.reference_profile.network = Some(NetworkRequirement {
        min_bandwidth_mibit_s: None,
        ..NETZ
    });
    let mut null_bandbreite = basis;
    null_bandbreite.reference_profile.network = Some(NetworkRequirement {
        min_bandwidth_mibit_s: Some(0),
        ..NETZ
    });
    assert_ne!(unbekannt.fingerprint(), null_bandbreite.fingerprint());
}

/// **AC3** — das Referenzprofil „standard" ist Laptop + node3.
///
/// Entscheidung des Auftraggebers vom 2026-10-28: Der Plan nannte
/// „Laptop + proxmox + gonet". Zusatzrechner ist nur noch node3 (Proxmox-Knoten
/// `ovh-pve1`, AMD Ryzen 7 9700X, 8 Kerne/16 Stränge); gonet und 10.0.0.69
/// gehören nicht mehr dazu.
#[test]
fn ac3_das_referenzprofil_standard_ist_laptop_und_node3() {
    let p = REFERENCE_PROFILE_STANDARD;
    assert_eq!(p.name, "standard");
    let namen: Vec<&str> = p.hosts.iter().map(|h| h.name).collect();
    assert_eq!(
        namen,
        vec!["laptop", "node3"],
        "Das Profil standard führt andere Rechner als Laptop + node3"
    );

    let laptop = &p.hosts[0];
    assert_eq!(
        laptop.cpu,
        CpuReservation {
            physical_cores: 8,
            threads_per_core: 2,
            reserved_physical_cores: 2,
        },
        "balanced: zwei physische Kerne samt Geschwistern bleiben frei (Plan Z. 46)"
    );
    assert_eq!(laptop.cpu.compute_threads(), 12);

    for h in p.hosts {
        assert!(
            h.guaranteed_memory_mib > 0,
            "{}: kein Speicherbudget",
            h.name
        );
        assert!(h.cpu.compute_threads() > 0, "{}: keine Stränge", h.name);
        assert!(
            h.gpu_duty_cycle.is_finite() && (0.0..=1.0).contains(&h.gpu_duty_cycle),
            "{}: Grafik-Einschaltdauer {} ist kein Anteil",
            h.name,
            h.gpu_duty_cycle
        );
        if let Some(quote) = h.guaranteed_cpu_quota {
            assert!(
                quote.is_finite() && quote > 0.0,
                "{}: Quote {quote} ist keine Zusicherung",
                h.name
            );
            assert!(
                quote <= f64::from(h.cpu.compute_threads()),
                "{}: Quote {quote} über den nutzbaren Strängen",
                h.name
            );
        }
        assert!(
            !h.thermal_throttling,
            "{}: aktives Drosseln erlaubt",
            h.name
        );
    }

    // Die entschiedenen Werte, wörtlich (Orchestrator, 2026-10-28). Die
    // allgemeinen Eigenschaften oben bestünde auch ein Profil mit erfundenen
    // Zahlen; erst diese Zeilen machen eine stille Änderung sichtbar.
    assert_eq!(laptop.guaranteed_cpu_quota, Some(12.0));
    assert_eq!(laptop.guaranteed_memory_mib, 4096, "Vorgabe „unter 5 GB“");
    assert_eq!(
        laptop.gpu_duty_cycle, 0.0,
        "kein GPU-Pfad, also keine Grafik"
    );

    let node3 = &p.hosts[1];
    assert_eq!(
        node3.guaranteed_cpu_quota, None,
        "node3 ist ausdrücklich best effort — keine garantierte Quote"
    );
    assert_eq!(
        node3.cpu,
        CpuReservation {
            physical_cores: 8,
            threads_per_core: 2,
            reserved_physical_cores: 0,
        },
        "16 Stränge, nichts reserviert: die Laptop-Grenze gilt auf den Nodes nicht"
    );
    assert_eq!(node3.guaranteed_memory_mib, 4096, "Speicher hart 4096 MiB");
    assert_eq!(node3.gpu_duty_cycle, 0.0);

    let netz = p
        .network
        .expect("Laptop + node3 hat eine Verbindung, also eine Netzvorgabe");
    assert_eq!(netz.max_rtt_ms, 250, "gemessen 48/92/183 ms zur Bau-VM");
    // NICHT gemessen — und deshalb keine Zahl. Eine Untergrenze von 1 Mibit/s
    // sähe wie eine Vorgabe aus und wäre eine Erfindung. Erhoben wird sie mit
    // #912; dann bekommt das Profil eine neue Version.
    assert_eq!(
        netz.min_bandwidth_mibit_s, None,
        "Die Bandbreiten-Untergrenze von node3 ist nicht gemessen"
    );
}
