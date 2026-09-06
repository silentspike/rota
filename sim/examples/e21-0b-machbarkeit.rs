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

use std::process::ExitCode;

use rota_sim::apl::{parse, AplName};
use rota_sim::balance::{run, BalanceConfig};
use rota_sim::data::boss_by_id;
use rota_sim::gear::WornValues;
use rota_sim::hostcaps::HostCaps;
use rota_sim::kalibrierung::{kalibriere, schluessel_erheben, Arbeitsart, Profil};
use rota_sim::kampagne::{
    AuditPolicy, CampaignDefinition, CampaignRunContract, ConfidenceSpec, CoverageSpec,
    DeadlinePolicy, Mission, ProjectionSet, QualityTargets, ReferenceProfile, WorkUnits,
};
use rota_sim::machbarkeit::{preflight, CalibratedHost, WorkEstimate};

const BOSS: &str = "boss-1-1";
const BATCH: u32 = 20_000;
const APL: &str = include_str!("../apl/step1-good.apl");

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

fn execute() -> Result<(), String> {
    let mut mandatory = 1_000_000_u64;
    let mut optional = 0_u64;
    let mut target_ms = 1000_u64;
    let mut hard_deadline = false;
    let mut arguments = std::env::args().skip(1);
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
                    Es startet keine vollständige Kampagne.");
                return Ok(());
            }
            "--hard-deadline" => hard_deadline = true,
            "--mandatory" | "--optional" | "--target-time" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("Wert fehlt nach {argument}"))?;
                if argument == "--target-time" {
                    target_ms = duration_ms(&value)?;
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
            _ => return Err(format!("unbekannte Angabe: {argument}; siehe --help")),
        }
    }
    if mandatory == 0 {
        return Err("Der Beispielauftrag braucht mindestens einen Pflichtkampf".to_owned());
    }
    let work_budget = mandatory
        .checked_add(optional)
        .ok_or("Arbeitsmenge überschreitet u64")?;
    let artifact = artifact_hash()?;
    let expected_key = schluessel_erheben(&artifact);
    let mut caps = HostCaps::erheben();
    let mut worn = WornValues::none();
    worn.purity = 375;
    if rota_sim::durchgaenge::ein_durchgang_genuegt(rota_sim::data::EFFECT_NODES, &worn) {
        return Err("BLOCKED: Der Messauftrag würde Durchgänge überspringen".to_owned());
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
    let rate = kalibriere(|| {
        std::hint::black_box(run(&config, &apl));
        f64::from(BATCH)
    });
    println!(
        "Messbeleg: rate={:.6} Kaempfe/s spread={:.6} samples={} duration_ms={} reason={:?}",
        rate.rate_per_s, rate.spread, rate.samples, rate.duration_ms, rate.reason
    );
    if !rate.reason.brauchbar() {
        return Err(format!(
            "BLOCKED: Kalibrierung nicht stabil ({:?}); keine Machbarkeitsaussage",
            rate.reason
        ));
    }
    caps.calibrated_fights_per_s = rate.rate_per_s;
    caps.calibration_spread = rate.spread;
    caps.measured_at = rate.measured_at;
    let profile = Profil {
        schluessel: schluessel_erheben(&artifact),
        caps,
        raten: vec![(Arbeitsart::ScalarFullFight, rate)],
    };
    // Für DIESEN expliziten Beispielauftrag gilt 1 WU je echtem Kampf.
    // Das ist keine Interpretation eines allgemeinen Kampagnenbudgets.
    let campaign = CampaignDefinition {
        profile_version: 1,
        reference_profile: ReferenceProfile {
            version: 1,
            hosts: &[],
        },
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
    let mut contract = CampaignRunContract::from_campaign(
        &campaign,
        Mission::SingleBoss { boss: BOSS },
        target_ms,
    );
    if hard_deadline {
        contract = contract.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
            remaining_ms: target_ms,
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
    .map_err(|error| error.to_string())?;
    println!("Vorabprüfung PASS: Pflicht={mandatory} Kaempfe/{:.3}ms, optional={optional} Kaempfe/{:?}ms, Ziel={}ms, WU={}, Teilauftrag={}",
        estimate.mandatory_ms, estimate.optional_ms, estimate.target_ms,
        estimate.mandatory_work_units.get(), estimate.partial_requested);
    println!("Dies ist eine Schätzung vor dem Start, kein Kampagnenergebnis.");
    Ok(())
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
