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
//! | drei echte Läufe mit einem, zwei, drei Teilnehmern | drei **Deployments** mit einem, zwei, drei Teilnehmern |
//! | gleicher Ergebnis-Fingerabdruck | gleicher **Kampagnen**-Fingerabdruck |
//!
//! **Teilnehmer sind Arbeitsprozesse auf node3**, jeder mit eigener
//! Kernbindung — Entscheidung des Auftraggebers vom 2026-10-28 („2 Prozesse auf
//! node3"). Hier stand „mit einem, zwei, drei Rechnern"; „drei Rechner" heißt
//! seitdem drei Prozesse. Kein weiterer Rechner; das Netz zwischen zwei
//! Maschinen prüft die Abnahme nicht.
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
    CampaignDefinition, CampaignRunContract, ComputeRequirement, CpuReservation, CpuRole,
    DateTimeUtc, DeadlinePolicy, DeployedHost, Hash, HostRequirement, NetworkRequirement,
    ReferenceProfile, ResultGuarantee, RunDeployment, WorkUnits, REFERENCE_PROFILE_STANDARD,
};

const LAPTOP: CpuReservation = CpuReservation {
    physical_cores: 8,
    threads_per_core: 2,
    reserved_physical_cores: 2,
};

const LAPTOP_RECHNET: ComputeRequirement = ComputeRequirement {
    guaranteed_cpu_quota: Some(1.0),
    cpu: LAPTOP,
};

const REFERENZ_HOST: HostRequirement = HostRequirement {
    name: "laptop",
    cpu_role: CpuRole::Compute(LAPTOP_RECHNET),
    gpu_duty_cycle: 0.0,
    memory_limit_mib: 4096,
    thermal_throttling: false,
};

/// Der Referenzrechner mit anderen CPU-Angaben, sonst unverändert.
const fn rechnet(rechnen: ComputeRequirement) -> HostRequirement {
    HostRequirement {
        cpu_role: CpuRole::Compute(rechnen),
        ..REFERENZ_HOST
    }
}

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

/// Physische Kerne von node3 — die Grenze, in die alle Teilnehmer passen.
const NODE3_KERNE: u16 = 8;

/// Ein Teilnehmer der Abnahme: ein Arbeitsprozess auf node3, gebunden an
/// `kerne` eigene physische Kerne samt ihren SMT-Geschwistern.
///
/// Entscheidung des Auftraggebers vom 2026-10-28: „2 Prozesse auf node3" —
/// getrennte Arbeitsprozesse, jeder mit eigener Kernbindung, kein weiterer
/// Rechner.
fn prozess(name: &'static str, kerne: u16) -> DeployedHost {
    DeployedHost {
        name,
        cgroup_quota: 1.0,
        runtime_perf_profile: 1.0,
        cpu: CpuReservation {
            physical_cores: kerne,
            threads_per_core: 2,
            reserved_physical_cores: 0,
        },
    }
}

/// Ein, zwei, drei Teilnehmer — mit ABSICHTLICH verschiedener Kernbindung,
/// damit die Deployments sich nicht nur in der Länge unterscheiden.
///
/// Bis zum 2026-10-28 waren es Rechner („laptop" mit 8, „node3" mit 12,
/// „dritter" mit 16 Kernen). Seitdem teilen sich die Teilnehmer die Kerne von
/// node3; die Schätzung ist deshalb überall dieselbe.
fn deployments() -> [RunDeployment; 3] {
    [
        RunDeployment {
            available_hosts: vec![prozess("node3/1", 8)],
            estimated_eta: Duration::from_secs(7200),
        },
        RunDeployment {
            available_hosts: vec![prozess("node3/1", 5), prozess("node3/2", 3)],
            estimated_eta: Duration::from_secs(7200),
        },
        RunDeployment {
            available_hosts: vec![
                prozess("node3/1", 4),
                prozess("node3/2", 3),
                prozess("node3/3", 1),
            ],
            estimated_eta: Duration::from_secs(7200),
        },
    ]
}

/// **AC2** — dieselbe Kampagne, ein, zwei, drei Teilnehmer, ein Fingerabdruck.
#[test]
fn ac2_der_fingerabdruck_haengt_nicht_am_deployment() {
    let k = kampagne();
    let d = deployments();

    // ZUSICHERUNG: Die drei Deployments müssen sich WIRKLICH unterscheiden.
    // Ohne sie verglichen wir dreimal dasselbe, und die Gleichheit wäre trivial
    // wahr — der Fall bliebe grün, auch wenn jemand alle drei gleich baut. Die
    // Summe der Stränge ist dafür kein Maß mehr: Alle drei teilen sich node3.
    let straenge_je_teilnehmer: Vec<Vec<u32>> = d
        .iter()
        .map(|x| {
            x.available_hosts
                .iter()
                .map(|t| t.cpu.compute_threads())
                .collect()
        })
        .collect();
    assert_eq!(
        straenge_je_teilnehmer,
        vec![vec![16], vec![10, 6], vec![8, 6, 2]],
        "Die drei Deployments unterscheiden sich nicht wie erwartet — dann \
         prüft dieser Fall nichts."
    );

    // Kein weiterer Rechner: Jeder Teilnehmer ist ein Prozess auf node3, und
    // zusammen binden sie höchstens dessen Kerne.
    for x in &d {
        for t in &x.available_hosts {
            assert!(
                t.name.starts_with("node3/"),
                "Teilnehmer {} ist kein Prozess auf node3",
                t.name
            );
        }
        let kerne: u32 = x
            .available_hosts
            .iter()
            .map(|t| u32::from(t.cpu.physical_cores))
            .sum();
        assert!(
            kerne <= u32::from(NODE3_KERNE),
            "{kerne} gebundene Kerne passen nicht auf node3 ({NODE3_KERNE})"
        );
    }

    let abdruecke: Vec<Hash> = d
        .iter()
        .map(|_| CampaignRunContract::from_campaign(&k).campaign_definition_hash)
        .collect();
    assert_eq!(
        abdruecke[0], abdruecke[1],
        "ein Teilnehmer gegen zwei: verschiedene Fingerabdrücke"
    );
    assert_eq!(
        abdruecke[1], abdruecke[2],
        "zwei Teilnehmer gegen drei: verschiedene Fingerabdrücke"
    );

    // Und die Arbeitsmenge selbst ist dieselbe, nicht nur ihr Abdruck.
    for d_i in &d {
        let vertrag = CampaignRunContract::from_campaign(&k);
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

    for (feld, abdruck) in &abweichungen {
        assert_ne!(
            *abdruck, basis,
            "Das Feld {feld} geht nicht in den Fingerabdruck ein — es lässt \
             sich ändern, ohne dass der Abdruck es meldet."
        );
    }

    // Die Zahl steht in der MELDUNG, nicht in der Bedingung: Eine feste Zahl
    // hier bräche bei jedem neuen Feld, statt es zu prüfen. Bis zum
    // 2026-10-29 dreizehn; Pflichtprojektionen, Abdeckung und die sechs
    // Qualitätsvorgaben sind mit dem Zuschnitt in #891 aus der Kampagne
    // entfernt, weil kein Produktionspfad sie las.
    assert_eq!(
        abweichungen.len(),
        5,
        "Es wurden {} Felder geprüft. Kommt ein Feld zur Kampagne dazu, gehört \
         es hier dazu — sonst wächst die Struktur schneller als ihr Schutz.",
        abweichungen.len()
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
    let vertrag = CampaignRunContract::from_campaign(&kampagne());
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

/// **AC6** — die Bedeutung der Frist ist benannt.
///
/// Bis zum 2026-10-29 hieß der Fall `…_und_die_zaehlung_steht_fest` und
/// prüfte dazu die drei Festlegungen zur Zählung der Rechenzeit. Sie standen
/// als eigenes Vertragsfeld, das kein Produktionspfad las, und sind mit dem
/// Zuschnitt in #891 entfernt.
#[test]
fn ac6_beide_fristarten_sind_unterscheidbar() {
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

    let v_zwei = CampaignRunContract::from_campaign(&zwei);
    let v_vier = CampaignRunContract::from_campaign(&vier);
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
    let vertrag = CampaignRunContract::from_campaign(&k);

    // Was der Vertrag aus der Kampagne übernimmt, kommt unverändert an. Wer es
    // dort liest, prüft `ac0_jedes_vertragsfeld_hat_einen_leser_im_produktionspfad`.
    assert_eq!(vertrag.target_wall_time, k.target_wall_time);
    assert_eq!(vertrag.deterministic_work_budget, k.canonical_work_budget);
    assert_eq!(vertrag.campaign_definition_hash, k.fingerprint());
    assert!(vertrag.hard_deadline.is_none());
}

/// Die Quelltexte, in denen die Leser des Laufvertrags stehen.
const KAMPAGNE_RS: &str = include_str!("../src/kampagne.rs");
const MACHBARKEIT_RS: &str = include_str!("../src/machbarkeit.rs");

/// Jedes Feld des Laufvertrags und die Funktion, die es im Produktionspfad
/// liest: (Feld, Datei, Funktion, Zugriff).
///
/// Ein Feld ohne Zeile hier ist ein Feld ohne Leser. Bis zum 2026-10-29 trug
/// der Vertrag vierzehn Felder, und zehn davon las niemand außer
/// Fingerabdruck, JSON und Herkunftsvergleich (Zuschnitt in #891). Ein solches
/// Feld verhält sich in jedem Test genau wie eines, das korrekt gelesen wird.
const LESER: &[(&str, &str, &str, &str)] = &[
    (
        "target_wall_time",
        "src/machbarkeit.rs",
        "preflight",
        "contract.target_wall_time",
    ),
    (
        "campaign_definition_hash",
        "src/kampagne.rs",
        "origin",
        "contract.campaign_definition_hash",
    ),
    (
        "deterministic_work_budget",
        "src/machbarkeit.rs",
        "preflight",
        "contract.deterministic_work_budget",
    ),
    (
        "hard_deadline",
        "src/machbarkeit.rs",
        "preflight",
        "contract.hard_deadline",
    ),
];

/// Funktionen, die einen Wert nur abbilden oder weiterreichen. Sie zählen
/// nicht als Leser: Ein Feld, das nur in Fingerabdruck und JSON eingeht,
/// ändert nichts daran, was ein Lauf tut.
const KEIN_LESER: &[&str] = &[
    "to_json",
    "fingerprint",
    "from_campaign",
    "with_hard_deadline",
    "eq",
    "fmt",
];

fn quelltext(datei: &str) -> &'static str {
    match datei {
        "src/kampagne.rs" => KAMPAGNE_RS,
        "src/machbarkeit.rs" => MACHBARKEIT_RS,
        _ => panic!("{datei}: kein eingebundener Quelltext"),
    }
}

/// Ein Block ohne Kommentare und ohne den Inhalt von Zeichenketten und
/// Zeichenliteralen; die Zeilen bleiben erhalten. Ein Feldname im Kommentar
/// oder in einer Meldung darf weder als Feld noch als Lesezugriff zählen.
fn nur_code(quelle: &str) -> String {
    let z: Vec<char> = quelle.chars().collect();
    let mut code = String::with_capacity(quelle.len());
    let mut i = 0;
    while i < z.len() {
        match z[i] {
            '/' if z.get(i + 1) == Some(&'/') => {
                while i < z.len() && z[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '"' => {
                i += 1;
                while i < z.len() && z[i] != '"' {
                    if z[i] == '\\' {
                        i += 1;
                    } else if z[i] == '\n' {
                        code.push('\n');
                    }
                    i += 1;
                }
                code.push_str("\"\"");
            }
            // Zeichenliteral mit Escape, etwa '\\' oder '\u{1f}'.
            '\'' if z.get(i + 1) == Some(&'\\') => {
                i += 3;
                while i < z.len() && z[i] != '\'' {
                    i += 1;
                }
                code.push_str("' '");
            }
            // Einfaches Zeichenliteral wie '"'; eine Lebensdauer wie 'a bleibt.
            '\'' if z.get(i + 2) == Some(&'\'') => {
                i += 2;
                code.push_str("' '");
            }
            c => code.push(c),
        }
        i += 1;
    }
    code
}

/// Der Block, dessen Kopf `kopf` enthält: vom Kopf bis zur schließenden
/// Klammer auf der Einrückung des Kopfes. Der Kopf muss genau einmal außerhalb
/// eines Kommentars stehen, sonst liest der Wächter womöglich den falschen.
fn block<'a>(quelle: &'a str, kopf: &str) -> &'a str {
    let treffer: Vec<usize> = quelle
        .match_indices(kopf)
        .map(|(i, _)| i)
        .filter(|&i| {
            let zeile = quelle[..i].rfind('\n').map_or(0, |z| z + 1);
            !quelle[zeile..i].trim_start().starts_with("//")
        })
        .collect();
    assert_eq!(treffer.len(), 1, "»{kopf}« steht {} Mal", treffer.len());
    let beginn = treffer[0];
    let zeile = quelle[..beginn].rfind('\n').map_or(0, |z| z + 1);
    let davor = &quelle[zeile..beginn];
    let einzug = davor.len() - davor.trim_start().len();
    let schluss = format!("\n{}}}\n", " ".repeat(einzug));
    let ende = quelle[beginn..]
        .find(&schluss)
        .unwrap_or_else(|| panic!("Ende von »{kopf}« nicht gefunden"));
    &quelle[beginn..beginn + ende]
}

/// Die Feldnamen eines Struct-Rumpfs, ohne Kommentare und Attribute.
fn feldnamen(rumpf: &str) -> Vec<String> {
    nur_code(rumpf)
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|z| !z.is_empty() && !z.starts_with("#["))
        .filter_map(|z| {
            let z = z
                .strip_prefix("pub(crate) ")
                .or_else(|| z.strip_prefix("pub "))
                .unwrap_or(z);
            let (name, rest) = z.split_once(':')?;
            let bezeichner = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c == '_' || c.is_ascii_lowercase() || c.is_ascii_digit());
            (bezeichner && !rest.starts_with(':')).then(|| name.to_owned())
        })
        .collect()
}

/// Liest der Rumpf `zugriff` — ohne Kommentare, Zeichenketten und Leerraum,
/// damit ein umbrochener Zugriff nicht unsichtbar wird?
fn liest(rumpf: &str, zugriff: &str) -> bool {
    let code: String = nur_code(rumpf)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let gesucht: String = zugriff.chars().filter(|c| !c.is_whitespace()).collect();
    code.contains(&gesucht)
}

/// **AC0, „jedes Feld hat einen Leser"** — der Laufvertrag trägt nur Felder,
/// die ein Produktionspfad liest.
///
/// Gelesen wird der Quelltext, weil Rust keine Felder aufzählen kann. Die
/// Feldmenge muss der Liste [`LESER`] GLEICH sein, nicht nur in ihr enthalten:
/// Fände der Wächter das Struct nicht oder läse null Felder, wäre „jedes
/// gefundene Feld hat einen Leser" trivial wahr.
#[test]
fn ac0_jedes_vertragsfeld_hat_einen_leser_im_produktionspfad() {
    let felder = feldnamen(block(KAMPAGNE_RS, "pub struct CampaignRunContract {"));
    let eingetragen: Vec<&str> = LESER.iter().map(|l| l.0).collect();

    let ohne_leser: Vec<&str> = felder
        .iter()
        .map(String::as_str)
        .filter(|f| !eingetragen.contains(f))
        .collect();
    assert!(
        ohne_leser.is_empty(),
        "Felder des Laufvertrags ohne Leser im Produktionspfad: {ohne_leser:?}. \
         Ein neues Feld braucht eine Funktion außerhalb der Tests, die es liest \
         und danach handelt — Fingerabdruck und JSON zählen nicht. Gibt es \
         keine, gehört das Feld nicht in den Vertrag."
    );
    let verschwunden: Vec<&str> = eingetragen
        .iter()
        .filter(|f| !felder.iter().any(|g| g == **f))
        .copied()
        .collect();
    assert!(
        verschwunden.is_empty(),
        "Eingetragene Leser für Felder, die der Vertrag nicht hat: {verschwunden:?} \
         (gefunden: {felder:?})"
    );

    for (feld, datei, funktion, zugriff) in LESER {
        assert!(
            !KEIN_LESER.contains(funktion),
            "{feld}: {funktion} bildet nur ab und ist kein Leser"
        );
        let rumpf = block(quelltext(datei), &format!("fn {funktion}("));
        assert!(
            liest(rumpf, zugriff),
            "{feld}: {datei} {funktion} liest »{zugriff}« nicht"
        );
    }
}

/// Gegenprobe des Wächters an einem erfundenen Quelltext: Ein Feldname oder
/// Zugriff in Kommentar oder Zeichenkette zählt nicht, ein umbrochener Zugriff
/// schon — auch hinter Zeichenliteralen, die ein Anführungszeichen enthalten.
#[test]
fn ac0_waechter_kontrolle_kommentare_zaehlen_nicht() {
    const PROBE: &str = "\
/// pub erfunden: u8,
pub struct Probe {
    /// pub auch_erfunden: u8,
    pub echt: u8, // pub im_kommentar: u8,
    #[allow(dead_code)]
    intern: std::time::Duration,
}

pub fn leser(contract: &Probe) -> u8 {
    // contract.intern
    let text: &'static str = \"// contract.intern\";
    let zeichen = ['\"', '\\\\', '\\u{1f}'];
    contract
        .echt
}
";
    assert_eq!(
        feldnamen(block(PROBE, "pub struct Probe {")),
        vec!["echt", "intern"]
    );
    let rumpf = block(PROBE, "fn leser(");
    assert!(
        liest(rumpf, "contract.echt"),
        "umbrochener Zugriff übersehen"
    );
    assert!(
        !liest(rumpf, "contract.intern"),
        "Zugriff aus Kommentar oder Zeichenkette gezählt"
    );
}

/// **AC3** — das Referenzprofil nennt Mindestressourcen, nicht Erreichbarkeit.
///
/// Der Unterschied zwischen `None` und einer Zahl ist hier inhaltlich:
/// ausdrückliches „best effort" gegen eine Zusicherung. Ein Profil, das nur
/// Namen führte, bestünde diesen Fall nicht.
///
/// „Jeder Rechner hat nutzbare Stränge" gilt seit dem 2026-10-28 nur noch für
/// die, deren **CPU rechnet**: Ein Koordinator hat bewusst keine (im Profil
/// „standard" der Laptop, Entscheidung des Auftraggebers). Damit die
/// Präzisierung den Fall nicht aushöhlt, muss mindestens eine CPU rechnen —
/// sonst bestünde ein Profil aus lauter Koordinatoren die Schleife trivial und
/// hätte nichts, woraus ein Budget folgen könnte. Die Grafik-Einschaltdauer
/// steht bei jedem Rechner und muss überall ein Anteil sein.
#[test]
fn ac3_das_referenzprofil_nennt_mindestressourcen() {
    let profil = kampagne().reference_profile;
    assert!(!profil.hosts.is_empty(), "Das Profil führt keinen Rechner.");
    assert!(
        profil.hosts.iter().any(|h| h.compute_threads() > 0),
        "Im Profil rechnet kein Rechner."
    );

    for h in profil.hosts {
        assert!(
            h.memory_limit_mib > 0,
            "{}: kein Speicherbudget — dann sagt der Eintrag nur, dass es die \
             Maschine gibt.",
            h.name
        );
        if let CpuRole::Compute(rechnen) = h.cpu_role {
            assert!(
                rechnen.cpu.compute_threads() > 0,
                "{}: rechnet ohne nutzbare Stränge",
                h.name
            );
        }
        assert!(
            h.gpu_duty_cycle.is_finite() && (0.0..=1.0).contains(&h.gpu_duty_cycle),
            "{}: Grafik-Einschaltdauer {} ist kein Anteil",
            h.name,
            h.gpu_duty_cycle
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
            rechnet(ComputeRequirement {
                guaranteed_cpu_quota: Some(2.0),
                ..LAPTOP_RECHNET
            }),
        ),
        (
            "best effort",
            rechnet(ComputeRequirement {
                guaranteed_cpu_quota: None,
                ..LAPTOP_RECHNET
            }),
        ),
        (
            "memory",
            HostRequirement {
                memory_limit_mib: 8192,
                ..REFERENZ_HOST
            },
        ),
        (
            "physical cores",
            rechnet(ComputeRequirement {
                cpu: CpuReservation {
                    physical_cores: 9,
                    ..LAPTOP
                },
                ..LAPTOP_RECHNET
            }),
        ),
        (
            "threads per core",
            rechnet(ComputeRequirement {
                cpu: CpuReservation {
                    threads_per_core: 1,
                    ..LAPTOP
                },
                ..LAPTOP_RECHNET
            }),
        ),
        (
            "reserved cores",
            rechnet(ComputeRequirement {
                cpu: CpuReservation {
                    reserved_physical_cores: 1,
                    ..LAPTOP
                },
                ..LAPTOP_RECHNET
            }),
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
        // Seit 2026-10-28: Ob die CPU eines Rechners rechnet, gehört zur
        // Identität. Die CPU des Laptops im Profil „standard" ist danach
        // Koordinator geworden.
        (
            "cpu role",
            HostRequirement {
                cpu_role: CpuRole::Coordinator,
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
    static OHNE_QUOTE: &[HostRequirement] = &[rechnet(ComputeRequirement {
        guaranteed_cpu_quota: None,
        ..LAPTOP_RECHNET
    })];
    static NULL_QUOTE: &[HostRequirement] = &[rechnet(ComputeRequirement {
        guaranteed_cpu_quota: Some(0.0),
        ..LAPTOP_RECHNET
    })];
    // Ein Koordinator ist kein rechnender Rechner mit lauter Nullen: null
    // nutzbare Stränge heißen „alle Kerne reserviert", eine Quote von 0.0
    // „garantiert nichts" — beides sind Aussagen über einen RECHNENDEN.
    static KOORDINATOR: &[HostRequirement] = &[HostRequirement {
        cpu_role: CpuRole::Coordinator,
        ..REFERENZ_HOST
    }];
    static LAUTER_NULLEN: &[HostRequirement] = &[rechnet(ComputeRequirement {
        guaranteed_cpu_quota: Some(0.0),
        cpu: CpuReservation {
            physical_cores: 0,
            threads_per_core: 0,
            reserved_physical_cores: 0,
        },
    })];
    // Die GPU eines Koordinators gehört zur Identität (Entscheidung „NUR GPU",
    // 2026-10-28): Ein Profil, dessen Laptop-GPU ein Viertel der Zeit rechnet,
    // ist ein anderes als eines ohne.
    static KOORDINATOR_MIT_GPU: &[HostRequirement] = &[HostRequirement {
        cpu_role: CpuRole::Coordinator,
        gpu_duty_cycle: 0.25,
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

    let mut koordinator = basis;
    koordinator.reference_profile.hosts = KOORDINATOR;
    let mut nullen = basis;
    nullen.reference_profile.hosts = LAUTER_NULLEN;
    assert_eq!(nullen.reference_profile.hosts[0].compute_threads(), 0);
    assert_eq!(koordinator.reference_profile.hosts[0].compute_threads(), 0);
    assert_ne!(
        koordinator.fingerprint(),
        nullen.fingerprint(),
        "Ein Koordinator und ein rechnender Rechner ohne Stränge sind derselbe \
         Fingerabdruck."
    );

    let mut mit_gpu = basis;
    mit_gpu.reference_profile.hosts = KOORDINATOR_MIT_GPU;
    assert_eq!(mit_gpu.reference_profile.hosts[0].compute_threads(), 0);
    assert_ne!(
        mit_gpu.fingerprint(),
        koordinator.fingerprint(),
        "Die Grafik-Einschaltdauer eines Koordinators fehlt im Fingerabdruck."
    );
}

/// Die CPU-Rolle steht im Fingerabdruck VOR den CPU-Angaben — und das trägt.
///
/// Die beiden Profile sind so gebaut, dass sie ohne die Rollenkennung
/// denselben Mischstrom ergäben, Wert für Wert. Wo im einen der Koordinator
/// „a" Grafik, Speicher und Drosselung einrührt (0, 0, 2, 0), rührt im anderen
/// der rechnende „a" Quote-Kennung, Kerne, Stränge und Reservierung ein — mit
/// denselben Zahlen. Seine Grafik-Bits (1 und 0),
/// sein Speicher (98) und seine Drosselung (0) sind im ersten Profil Länge,
/// Name und Quote-Kennung des rechnenden „b"; dessen Kerne, Stränge und
/// Reservierung (1, 0, 98) sind im zweiten Länge und Name des Koordinators
/// „b". Ein Koordinator hat weniger Felder als ein rechnender Rechner; ohne
/// Kennung entscheidet nichts, wo sie enden.
#[test]
fn ac2_die_rolle_trennt_koordinator_und_rechner_auch_bei_passenden_werten() {
    static KOORDINATOR_ZUERST: &[HostRequirement] = &[
        HostRequirement {
            name: "a",
            cpu_role: CpuRole::Coordinator,
            gpu_duty_cycle: 0.0,
            memory_limit_mib: 2,
            thermal_throttling: false,
        },
        HostRequirement {
            name: "b",
            cpu_role: CpuRole::Compute(ComputeRequirement {
                guaranteed_cpu_quota: None,
                cpu: CpuReservation {
                    physical_cores: 1,
                    threads_per_core: 0,
                    reserved_physical_cores: 98,
                },
            }),
            gpu_duty_cycle: 0.0,
            memory_limit_mib: 4096,
            thermal_throttling: false,
        },
    ];
    static RECHNER_ZUERST: &[HostRequirement] = &[
        HostRequirement {
            name: "a",
            cpu_role: CpuRole::Compute(ComputeRequirement {
                guaranteed_cpu_quota: None,
                cpu: CpuReservation {
                    physical_cores: 0,
                    threads_per_core: 2,
                    reserved_physical_cores: 0,
                },
            }),
            // Niederwertige Hälfte 1, höherwertige 0: im anderen Profil die
            // Länge von „b".
            gpu_duty_cycle: f64::from_bits(1),
            memory_limit_mib: 98,
            thermal_throttling: false,
        },
        HostRequirement {
            name: "b",
            cpu_role: CpuRole::Coordinator,
            gpu_duty_cycle: 0.0,
            memory_limit_mib: 4096,
            thermal_throttling: false,
        },
    ];
    let mut eins = kampagne();
    eins.reference_profile.hosts = KOORDINATOR_ZUERST;
    let mut zwei = kampagne();
    zwei.reference_profile.hosts = RECHNER_ZUERST;
    assert_ne!(
        eins.fingerprint(),
        zwei.fingerprint(),
        "Ohne Rollenkennung fallen die beiden Profile zusammen."
    );
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

/// **AC3** — das Referenzprofil „standard" ist Laptop + node3: node3 rechnet,
/// die CPU des Laptops koordiniert.
///
/// Rechner: Entscheidung des Auftraggebers vom 2026-10-28. Der Plan nannte
/// „Laptop + proxmox + gonet". Zusatzrechner ist nur noch node3 (Proxmox-Knoten
/// `ovh-pve1`, AMD Ryzen 7 9700X, 8 Kerne/16 Stränge); gonet und 10.0.0.69
/// gehören nicht mehr dazu.
///
/// Rollen: zweite Entscheidung desselben Tages, auf die Frage, ob der Laptop
/// mitrechnet oder nur koordiniert: „Nur koordinieren". **Dieser Fall ist
/// deshalb bewusst umgedreht.** Bis dahin verlangte er wörtlich, dass der
/// Laptop mit zwölf Strängen rechnet (`balanced`, zwei physische Kerne samt
/// Geschwistern frei, Quote 12), und dass jeder Rechner des Profils nutzbare
/// Stränge hat. Jetzt verlangt er das Gegenteil für die CPU des Laptops, und
/// die Strang-Bedingung gilt für jeden Rechner, dessen CPU rechnet —
/// abgesichert dadurch, dass die Liste dieser Rechner genau node3 ist.
///
/// GPU: dritte Entscheidung desselben Abends, „NUR GPU" — die GPU des Laptops
/// darf rechnen, auf ausdrücklichen Aufruf. Deshalb prüft der Fall die
/// Grafik-Einschaltdauer bei **jedem** Rechner und verlangt nicht, dass der
/// Koordinator keine hat. Im Referenzlauf ist sie 0 %.
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

    // Der Koordinator läuft auf dem ersten Rechner (`ReferenceProfile::hosts`),
    // und im Profil standard rechnet seine CPU nicht.
    let laptop = &p.hosts[0];
    assert_eq!(
        laptop.cpu_role,
        CpuRole::Coordinator,
        "Die CPU des Laptops rechnet mit — entschieden ist „Nur koordinieren“"
    );
    assert_eq!(laptop.compute_threads(), 0);

    let rechnend: Vec<&str> = p
        .hosts
        .iter()
        .filter(|h| matches!(h.cpu_role, CpuRole::Compute(_)))
        .map(|h| h.name)
        .collect();
    assert_eq!(
        rechnend,
        vec!["node3"],
        "Im Profil standard rechnet nicht genau die CPU von node3"
    );

    for h in p.hosts {
        assert!(h.memory_limit_mib > 0, "{}: kein Speicherbudget", h.name);
        assert!(
            !h.thermal_throttling,
            "{}: aktives Drosseln erlaubt",
            h.name
        );
        assert!(
            h.gpu_duty_cycle.is_finite() && (0.0..=1.0).contains(&h.gpu_duty_cycle),
            "{}: Grafik-Einschaltdauer {} ist kein Anteil",
            h.name,
            h.gpu_duty_cycle
        );
        let CpuRole::Compute(rechnen) = h.cpu_role else {
            continue;
        };
        assert!(
            rechnen.cpu.compute_threads() > 0,
            "{}: rechnet ohne Stränge",
            h.name
        );
        if let Some(quote) = rechnen.guaranteed_cpu_quota {
            assert!(
                quote.is_finite() && quote > 0.0,
                "{}: Quote {quote} ist keine Zusicherung",
                h.name
            );
            assert!(
                quote <= f64::from(rechnen.cpu.compute_threads()),
                "{}: Quote {quote} über den nutzbaren Strängen",
                h.name
            );
        }
    }

    // Die entschiedenen Werte, wörtlich (Orchestrator, 2026-10-28). Die
    // allgemeinen Eigenschaften oben bestünde auch ein Profil mit erfundenen
    // Zahlen; erst diese Zeilen machen eine stille Änderung sichtbar.
    //
    // Speicher ist auf BEIDEN Rechnern eine Obergrenze, keine Reservierung:
    // auf dem Laptop die Vorgabe „unter 5 GB“, auf node3 die Vorgabe des
    // Auftraggebers vom 2026-10-28, dass der Container nur belegt, was er
    // gerade braucht. Auch der Koordinator belegt Speicher.
    assert_eq!(laptop.memory_limit_mib, 4096, "Vorgabe „unter 5 GB“");
    // Die GPU des Laptops darf rechnen, aber nur auf ausdrücklichen Aufruf —
    // und es gibt noch keinen GPU-Pfad. Der Referenzlauf rechnet also ohne sie.
    assert_eq!(
        laptop.gpu_duty_cycle, 0.0,
        "Im Referenzlauf rechnet die GPU des Laptops von selbst mit"
    );

    let node3 = &p.hosts[1];
    let CpuRole::Compute(n3) = node3.cpu_role else {
        panic!("node3 rechnet nicht: {:?}", node3.cpu_role);
    };
    assert_eq!(
        n3.guaranteed_cpu_quota, None,
        "node3 ist ausdrücklich best effort — keine garantierte Quote"
    );
    assert_eq!(
        n3.cpu,
        CpuReservation {
            physical_cores: 8,
            threads_per_core: 2,
            reserved_physical_cores: 0,
        },
        "16 Stränge, nichts reserviert: die Laptop-Grenze gilt auf den Nodes nicht"
    );
    assert_eq!(
        node3.memory_limit_mib, 4096,
        "Speicher höchstens 4096 MiB, nichts reserviert"
    );
    assert_eq!(node3.gpu_duty_cycle, 0.0, "node3 hat nur BMC-Grafik");

    // Das Profil rechnet mit genau den Strängen von node3.
    let straenge: u32 = p.hosts.iter().map(HostRequirement::compute_threads).sum();
    assert_eq!(
        straenge,
        n3.cpu.compute_threads(),
        "Im Profil rechnen mehr als die Stränge von node3"
    );
    assert_eq!(straenge, 16);

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
