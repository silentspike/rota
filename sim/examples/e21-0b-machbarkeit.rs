//! Frisch gemessene Vorabprüfung eines ausdrücklich skalaren Beispielauftrags.
//!
//! Seriell auf dem vorkonfigurierten Buildserver bauen und messen:
//! ```text
//! cargo remote --no-copy-lock -- run --release -p rota-sim --example e21-0b-machbarkeit -- --mandatory 1000000 --target-time 1ms
//! cargo remote --no-copy-lock -- run --release -p rota-sim --example e21-0b-machbarkeit -- --mandatory 1000 --target-time 1h
//! ```
//!
//! Keine alte Profildatei wird eingelesen. Gemessen werden echte skalare Kämpfe
//! mit aktivem Krit-Zufall; die deterministische Ein-Durchgang-Abkürzung wäre
//! für die gezählte Einheit falsch. Dieses Beispiel prüft nur vorab. Es ist
//! kein vollständiger Kampagnen-Runner und gibt kein Balance-Ergebnis aus.
//!
//! # Exit-Codes
//!
//! ```text
//! 0  PASS              die Pflichtarbeit passt (ebenso --help)
//! 1  nicht erfüllbar   gemessen und geprüft; die Meldung nennt drei Wege
//! 2  BLOCKED           keine Machbarkeitsaussage: Messung instabil oder
//!                      ungültig, Eingabe falsch
//! ```
//!
//! Bis zum 29.10. endete auch BLOCKED mit 1. Ein Aufrufer konnte „passt nicht“
//! dann nicht von „nicht gemessen“ trennen — #949 verlangt die drei Zustände
//! getrennt, und nicht gelaufen ist kein Ergebnis.

use std::process::ExitCode;
use std::time::Duration;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig};
use rota_sim::data::boss_by_id;
use rota_sim::gear::WornValues;
use rota_sim::hostcaps::HostCaps;
use rota_sim::kalibrierung::{
    kalibriere, schluessel_erheben, Arbeitsart, Profil, Profilschluessel, RateModel,
};
use rota_sim::kampagne::{
    AuditPolicy, CampaignDefinition, CampaignRunContract, ConfidenceSpec, CoverageSpec,
    DeadlinePolicy, Mission, ProjectionSet, QualityTargets, ReferenceProfile, WorkUnits,
};
use rota_sim::machbarkeit::{preflight, CalibratedHost, FeasibilityError, WorkEstimate};

const BOSS: &str = "boss-1-1";
const BATCH: u32 = 20_000;
const APL: &str = include_str!("../apl/step1-good.apl");

/// Wie die Vorabprüfung ausgeht. Jeder Ausgang hat genau einen Exit-Code.
enum Ausgang {
    /// Die Pflichtarbeit passt.
    Pass,
    /// Gemessen und geprüft, und sie passt nicht; die Meldung nennt die Wege.
    NichtErfuellbar(String),
    /// Keine Machbarkeitsaussage, mit Grund.
    Blocked(String),
}

impl Ausgang {
    const fn exit_code(&self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::NichtErfuellbar(_) => 1,
            Self::Blocked(_) => 2,
        }
    }
}

/// Jeder Fehler vor der Entscheidung — Eingabe, Herkunft, Messung — blockiert.
impl From<String> for Ausgang {
    fn from(grund: String) -> Self {
        Self::Blocked(format!("BLOCKED: {grund}"))
    }
}

impl From<&str> for Ausgang {
    fn from(grund: &str) -> Self {
        grund.to_owned().into()
    }
}

/// Nur eine Überschreitung der Zielzeit ist eine Machbarkeitsaussage.
///
/// Die übrigen Fehler der Vorabprüfung sagen, dass sie keine treffen konnte:
/// ungültige Eingabe oder Messung, fehlende Rate, Pflichtkosten über dem
/// eingefrorenen Vertrag (in diesem Beispiel entsteht das Budget aus der
/// Auftragsmenge). Die Zuordnung ist erschöpfend: Eine neue Fehlerart bricht
/// den Bau, statt still einen Exit-Code zu erben.
fn ausgang_aus(fehler: &FeasibilityError) -> Ausgang {
    match fehler {
        FeasibilityError::TimeBudgetExceeded { .. } => Ausgang::NichtErfuellbar(fehler.to_string()),
        FeasibilityError::InvalidInput(_)
        | FeasibilityError::MissingRate(_)
        | FeasibilityError::WorkBudgetExceeded { .. } => fehler.to_string().into(),
    }
}

/// Was das Programm vom Rechner erhebt.
///
/// Im Betrieb ist es der Rechner, auf dem es läuft ([`Dieser`]). Die Prüffälle
/// unten setzen einen festen ein: Ihr Exit-Code soll an der Entscheidung
/// hängen, nicht an Last und Ausstattung des Bau-Rechners. Fehlt dort etwa
/// `scaling_governor`, verweigert die Vorabprüfung jede Aussage
/// („Profilherkunft unvollständig“).
trait Rechner {
    /// Host, Artefakt, Kernel und Taktregime; vor und nach der Messung erhoben.
    fn schluessel(&self, artefakt: &str) -> Profilschluessel;
    /// Die Ausstattung.
    fn ausstattung(&self) -> HostCaps;
    /// Eine frische adaptive Kalibrierung genau dieses Auftrags.
    fn kalibriere(&self, config: &BalanceConfig, apl: &Apl) -> RateModel;
}

/// Der Rechner, auf dem das Programm läuft.
struct Dieser;

impl Rechner for Dieser {
    fn schluessel(&self, artefakt: &str) -> Profilschluessel {
        schluessel_erheben(artefakt)
    }

    fn ausstattung(&self) -> HostCaps {
        HostCaps::erheben()
    }

    fn kalibriere(&self, config: &BalanceConfig, apl: &Apl) -> RateModel {
        kalibriere(|| {
            std::hint::black_box(run(config, apl));
            f64::from(BATCH)
        })
    }
}

fn duration_ms(text: &str) -> Result<u64, String> {
    for (suffix, multiplier) in [("ms", 1_u64), ("s", 1000), ("m", 60_000), ("h", 3_600_000)] {
        if let Some(number) = text.strip_suffix(suffix) {
            let value = number
                .parse::<u64>()
                .map_err(|_| format!("ungültige Zielzeit: {text}"))?;
            return value
                .checked_mul(multiplier)
                .filter(|value| *value > 0)
                .ok_or_else(|| format!("Zielzeit muss positiv sein und in u64 passen: {text}"));
        }
    }
    Err(format!(
        "Zielzeit braucht eine Einheit: {text}; etwa 1ms, 30s oder 2h"
    ))
}

/// Ein gemeldeter Wert der Kalibrierung; `unbekannt`, wenn er nicht erhoben
/// werden konnte — das ist etwas anderes als null.
fn gemeldet(wert: Option<f64>) -> String {
    wert.map_or_else(|| "unbekannt".to_owned(), |w| format!("{w:.6}"))
}

fn artifact_hash() -> Result<String, String> {
    let path = std::env::current_exe().map_err(|error| error.to_string())?;
    let output = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|error| format!("Artefakt-Hash: {error}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let hash = text.split_whitespace().next().unwrap_or("");
    if !output.status.success() || hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Artefakt-Hash konnte nicht erhoben werden".to_owned());
    }
    Ok(hash.to_owned())
}

/// Der ganze Ablauf bis zur Entscheidung; `Ok` ist PASS.
fn pruefe(argumente: &[String], rechner: &impl Rechner) -> Result<(), Ausgang> {
    let mut mandatory = 1_000_000_u64;
    let mut optional = 0_u64;
    let mut target_ms = 1000_u64;
    let mut hard_deadline = false;
    let mut arguments = argumente.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!("e21-0b-machbarkeit [--mandatory N] [--optional N] [--target-time 1ms|30s|2h] [--hard-deadline]\n\
                    Einheit: echter skalarer Kampf. Fest: boss-1-1, step1-good.apl, 60s,\n\
                    stop_on_kill=false, purity=375, seed=5, ein Rechenstrang.\n\
                    Vor jeder Prüfung wird dieser Auftrag frisch adaptiv kalibriert.\n\
                    --hard-deadline verlangt ausdrücklich einen späteren Teilauftrag.\n\
                    Dieses Beispiel prüft nur lokal vorab; zusätzliche Hosts akzeptiert\n\
                    die Bibliotheks-API mit passenden vollständigen Messprofilen.\n\
                    Es startet keine vollständige Kampagne.\n\
                    Exit 0 PASS, 1 nicht erfüllbar (mit drei Wegen), 2 BLOCKED (keine Aussage).");
                return Ok(());
            }
            "--hard-deadline" => hard_deadline = true,
            "--mandatory" | "--optional" | "--target-time" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("Wert fehlt nach {argument}"))?;
                if argument == "--target-time" {
                    target_ms = duration_ms(value)?;
                } else {
                    let number = value
                        .parse::<u64>()
                        .map_err(|_| format!("ungültige Einheiten: {value}"))?;
                    if argument == "--mandatory" {
                        mandatory = number;
                    } else {
                        optional = number;
                    }
                }
            }
            _ => return Err(format!("unbekannte Angabe: {argument}; siehe --help").into()),
        }
    }
    if mandatory == 0 {
        return Err("Der Beispielauftrag braucht mindestens einen Pflichtkampf".into());
    }
    let work_budget = mandatory
        .checked_add(optional)
        .ok_or("Arbeitsmenge überschreitet u64")?;
    let artifact = artifact_hash()?;
    let expected_key = rechner.schluessel(&artifact);
    let mut caps = rechner.ausstattung();
    let mut worn = WornValues::none();
    worn.purity = 375;
    if rota_sim::durchgaenge::ein_durchgang_genuegt(rota_sim::data::EFFECT_NODES, &worn) {
        return Err("Der Messauftrag würde Durchgänge überspringen".into());
    }
    let apl = parse(AplName("e21-0b fixed workload"), APL).map_err(|error| error.to_string())?;
    let config = BalanceConfig {
        boss: boss_by_id(BOSS).ok_or("Beispielboss fehlt")?,
        iterations: BATCH,
        base_seed: 5,
        threads: 1,
        duration_ms: Some(60_000),
        stop_on_kill: false,
        worn,
        ..BalanceConfig::default()
    };
    println!("Kalibrierung vor dem Auftrag: host={} sha256={}\n\
        scalar_full_fight: boss-1-1, step1-good.apl, 60s, stop_on_kill=false, purity=375, seed=5, threads=1, batch={BATCH}",
        expected_key.host, artifact);
    let rate = rechner.kalibriere(&config, &apl);
    println!(
        "Messbeleg: rate={:.6} Kaempfe/s spread={:.6} samples={} duration_ms={} reason={:?} takt_drift={} psi_cpu={}",
        rate.rate_per_s,
        rate.spread,
        rate.samples,
        rate.duration_ms,
        rate.reason,
        gemeldet(rate.takt_drift),
        gemeldet(rate.psi_cpu)
    );
    if !rate.reason.brauchbar() {
        return Err(format!(
            "Kalibrierung nicht stabil ({:?}); keine Machbarkeitsaussage",
            rate.reason
        )
        .into());
    }
    caps.calibrated_fights_per_s = rate.rate_per_s;
    caps.calibration_spread = rate.spread;
    caps.measured_at = rate.measured_at;
    let profile = Profil {
        schluessel: rechner.schluessel(&artifact),
        caps,
        raten: vec![(Arbeitsart::ScalarFullFight, rate)],
    };
    // Für DIESEN expliziten Beispielauftrag gilt 1 WU je echtem Kampf.
    // Das ist keine Interpretation eines allgemeinen Kampagnenbudgets.
    let campaign = CampaignDefinition {
        profile_version: 1,
        reference_profile: ReferenceProfile {
            name: "e21-0b-beispiel",
            version: 1,
            hosts: &[],
            network: None,
        },
        // Seit #914 gehört die Zielzeit zur Kampagne; der Vertrag übernimmt sie.
        target_wall_time: Duration::from_millis(target_ms),
        canonical_work_budget: WorkUnits(work_budget),
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
    };
    let mut contract =
        CampaignRunContract::from_campaign(&campaign, Mission::SingleBoss { boss: BOSS });
    if hard_deadline {
        contract = contract.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
            remaining: Duration::from_millis(target_ms),
        });
    }
    let work = WorkEstimate {
        kind: Arbeitsart::ScalarFullFight,
        mandatory,
        optional,
        work_units_per_unit: WorkUnits(1),
    };
    let estimate = preflight(
        &contract,
        &[work],
        &[CalibratedHost {
            profile: &profile,
            expected_key: &expected_key,
        }],
    )
    .map_err(|fehler| ausgang_aus(&fehler))?;
    println!("Vorabprüfung PASS: Pflicht={mandatory} Kaempfe/{:.3}ms, optional={optional} Kaempfe/{:?}ms, Ziel={}ms, WU={}, Teilauftrag={}",
        estimate.mandatory_ms, estimate.optional_ms, estimate.target_ms,
        estimate.mandatory_work_units.get(), estimate.partial_requested);
    println!("Dies ist eine Schätzung vor dem Start, kein Kampagnenergebnis.");
    Ok(())
}

/// Meldet den Ausgang und liefert seinen Exit-Code.
fn beende(ausgang: Result<(), Ausgang>) -> u8 {
    let ausgang = ausgang.err().unwrap_or(Ausgang::Pass);
    match &ausgang {
        Ausgang::Pass => {}
        Ausgang::NichtErfuellbar(meldung) | Ausgang::Blocked(meldung) => eprintln!("{meldung}"),
    }
    ausgang.exit_code()
}

fn main() -> ExitCode {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(beende(pruefe(&argumente, &Dieser)))
}

#[cfg(test)]
mod tests {
    //! Exit-Codes am gestarteten Prozess (#949).
    //!
    //! Jeder Fall startet dieses Prüfprogramm ein zweites Mal als eigenen
    //! Prozess, und darin nur den Hilfsfall [`als_programm`]. Der ruft dieselben
    //! zwei Funktionen wie `main`, `pruefe` und `beende`, und beendet den
    //! Prozess mit deren Code. Gelesen werden Exit-Code und Ausgabe des Kindes.
    //!
    //! **Was hier nicht läuft:** `main` selbst — ein Prüfprogramm hat seinen
    //! eigenen Einstieg —, also die Zeile, die die Argumente liest, und der
    //! echte Rechner. Der feste ersetzt Schlüssel, Ausstattung und
    //! Kalibrierung; die echte Kalibrierung belegen die node3-Läufe in
    //! `docs/MACHBARKEIT.md`.

    use std::process::Command;

    use rota_sim::hostcaps::IsaSet;
    use rota_sim::kalibrierung::Abbruchgrund;

    use super::{
        beende, pruefe, Apl, BalanceConfig, HostCaps, Profilschluessel, RateModel, Rechner,
    };

    /// Die Argumente des Kindes, getrennt durch das Steuerzeichen US.
    const ARGUMENTE: &str = "E21_0B_ARGUMENTE";
    /// Welchen Messbeleg der feste Rechner liefert.
    const MESSBELEG: &str = "E21_0B_MESSBELEG";

    /// Ein Rechner mit festem Schlüssel, fester Ausstattung und festem
    /// Messbeleg.
    struct Fest(RateModel);

    impl Rechner for Fest {
        fn schluessel(&self, artefakt: &str) -> Profilschluessel {
            Profilschluessel {
                host: "fest".to_owned(),
                artefakt: artefakt.to_owned(),
                kernel: "fest".to_owned(),
                taktregime: "fest".to_owned(),
            }
        }

        fn ausstattung(&self) -> HostCaps {
            HostCaps {
                threads_total: 16,
                threads_usable: 16,
                l2_per_core: 0,
                l3: 0,
                ram_total: 0,
                ram_free: 0,
                isa: IsaSet::default(),
                gpu: None,
                calibrated_fights_per_s: 0.0,
                calibration_spread: 0.0,
                measured_at: 0,
            }
        }

        fn kalibriere(&self, _: &BalanceConfig, _: &Apl) -> RateModel {
            self.0
        }
    }

    /// Der Messbeleg von node3, Lauf 1 vom 29.10.: stabil, Taktdrift
    /// 0,222755. An ihm scheiterte die Abnahme mit „ungültige oder doppelte
    /// Rate“.
    const NODE3_LAUF_1: RateModel = RateModel {
        rate_per_s: 169_979.531_806,
        spread: 0.026_107,
        samples: 43,
        duration_ms: 5122,
        reason: Abbruchgrund::Stabil,
        measured_at: 0,
        psi_cpu: Some(0.0),
        takt_drift: Some(0.222_755),
    };

    fn messbeleg(name: &str) -> RateModel {
        match name {
            "node3-lauf-1" => NODE3_LAUF_1,
            "hoechstdauer" => RateModel {
                reason: Abbruchgrund::Hoechstdauer,
                ..NODE3_LAUF_1
            },
            "streuung-nan" => RateModel {
                spread: f64::NAN,
                ..NODE3_LAUF_1
            },
            anderer => panic!("unbekannter Messbeleg {anderer}"),
        }
    }

    /// Der Prozess, den die Prüffälle starten — selbst keiner.
    #[test]
    #[ignore = "Hilfsprozess der Exit-Code-Prüffälle, nur von ihnen gestartet"]
    fn als_programm() {
        let argumente: Vec<String> = std::env::var(ARGUMENTE)
            .expect("nur über die Exit-Code-Prüffälle gestartet")
            .split('\u{1f}')
            .filter(|teil| !teil.is_empty())
            .map(str::to_owned)
            .collect();
        let beleg = messbeleg(&std::env::var(MESSBELEG).expect("Messbeleg fehlt"));
        std::process::exit(i32::from(beende(pruefe(&argumente, &Fest(beleg)))));
    }

    struct Kind {
        code: Option<i32>,
        stdout: String,
        stderr: String,
    }

    fn starte(argumente: &[&str], beleg: &str) -> Kind {
        let ausgabe = Command::new(std::env::current_exe().expect("eigenes Prüfprogramm"))
            .args([
                "--exact",
                "tests::als_programm",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(ARGUMENTE, argumente.join("\u{1f}"))
            .env(MESSBELEG, beleg)
            .output()
            .expect("Kindprozess startet");
        Kind {
            code: ausgabe.status.code(),
            stdout: String::from_utf8_lossy(&ausgabe.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&ausgabe.stderr).into_owned(),
        }
    }

    /// Das Kind hat `pruefe` bis zur Messung durchlaufen — sonst belegt sein
    /// Exit-Code nichts über die Entscheidung.
    fn hat_gemessen(kind: &Kind) {
        assert!(
            kind.stdout
                .contains("Messbeleg: rate=169979.531806 Kaempfe/s")
                && kind.stdout.contains("takt_drift=0.222755 psi_cpu=0.000000"),
            "das Kind hat nicht gemessen:\n{}\n{}",
            kind.stdout,
            kind.stderr
        );
    }

    #[test]
    fn exit_0_wenn_die_pflicht_passt_auch_bei_taktdrift() {
        let kind = starte(
            &["--mandatory", "1000", "--target-time", "1h"],
            "node3-lauf-1",
        );
        hat_gemessen(&kind);
        assert_eq!(kind.code, Some(0), "{}", kind.stderr);
        assert!(
            kind.stdout
                .contains("Vorabprüfung PASS: Pflicht=1000 Kaempfe")
                && kind.stdout.contains("Teilauftrag=false"),
            "{}",
            kind.stdout
        );
        assert!(kind.stderr.is_empty(), "{}", kind.stderr);
    }

    #[test]
    fn exit_0_mit_teilauftrag_bei_ausdruecklicher_frist() {
        let argumente = [
            "--mandatory",
            "1000000",
            "--target-time",
            "1ms",
            "--hard-deadline",
        ];
        let kind = starte(&argumente, "node3-lauf-1");
        hat_gemessen(&kind);
        assert_eq!(kind.code, Some(0), "{}", kind.stderr);
        assert!(kind.stdout.contains("Teilauftrag=true"), "{}", kind.stdout);
    }

    #[test]
    fn exit_1_nennt_alle_drei_wege() {
        let kind = starte(
            &["--mandatory", "1000000", "--target-time", "1ms"],
            "node3-lauf-1",
        );
        hat_gemessen(&kind);
        assert_eq!(kind.code, Some(1), "{}", kind.stderr);
        for teil in [
            "Auftrag in 1 ms nicht erfüllbar.",
            "Wege:",
            "weitere passend kalibrierte Rechner zulassen (--hosts …)",
            "Zielzeit erhöhen (--target-time mindestens",
            "Teilergebnis ausdrücklich akzeptieren (--hard-deadline; liefert AnytimePartial)",
        ] {
            assert!(kind.stderr.contains(teil), "{teil} fehlt:\n{}", kind.stderr);
        }
        assert!(!kind.stderr.contains("BLOCKED"), "{}", kind.stderr);
    }

    #[test]
    fn exit_2_bei_instabiler_kalibrierung() {
        let kind = starte(
            &["--mandatory", "1000", "--target-time", "1h"],
            "hoechstdauer",
        );
        assert_eq!(kind.code, Some(2), "{}", kind.stderr);
        assert!(
            kind.stderr.contains(
                "BLOCKED: Kalibrierung nicht stabil (Hoechstdauer); keine Machbarkeitsaussage"
            ),
            "{}",
            kind.stderr
        );
        assert!(!kind.stdout.contains("PASS"), "{}", kind.stdout);
    }

    #[test]
    fn exit_2_bei_ungueltiger_rate_nennt_das_kriterium() {
        let kind = starte(
            &["--mandatory", "1000", "--target-time", "1h"],
            "streuung-nan",
        );
        assert_eq!(kind.code, Some(2), "{}", kind.stderr);
        assert!(
            kind.stderr.contains(
                "BLOCKED: Vorabprüfung: ungültige Rate scalar_full_fight auf fest: Streuung nicht endlich und nicht negativ: NaN"
            ),
            "{}",
            kind.stderr
        );
    }

    #[test]
    fn exit_2_bei_falscher_eingabe() {
        let kind = starte(&["--target-time", "5"], "node3-lauf-1");
        assert_eq!(kind.code, Some(2), "{}", kind.stderr);
        assert!(
            kind.stderr
                .contains("BLOCKED: Zielzeit braucht eine Einheit: 5"),
            "{}",
            kind.stderr
        );
        assert!(!kind.stdout.contains("Messbeleg"), "{}", kind.stdout);
    }
}
