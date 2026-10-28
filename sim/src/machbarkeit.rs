//! Vorabprüfung mit expliziten Arbeitsklassen (E21.0b, #949).
//!
//! Einheiten werden nur durch die Rate derselben Arbeitsklasse geteilt. Erst
//! die geschätzten Zeiten werden addiert. Die Umrechnung in die eingefrorene
//! Währung `WorkUnits` ist eine eigene Eingabe und folgt niemals aus einer Rate.
//!
//! Die Prüfung startet keinen Kampf und verändert keinen Vertrag. Sie schätzt
//! auf Basis der übergebenen, aktuell kompatiblen Messungen. Eine Rate gilt für
//! den gemessenen Host-Auftrag, nicht pro Kern. Übertragung, künftige Fremdlast
//! und die Vollständigkeit der vom Aufrufer benannten Pflichtaufträge sind hier
//! nicht bewiesen. Der vollständige Kampagnen-Runner ist ein eigener Anschluss.

use std::fmt;

use crate::kalibrierung::{
    Arbeitsart, Profil, Profilschluessel, RateModel, ABSCHNITTE_MIN, MINDESTDAUER, STREUUNG_MAX,
    TAKT_DRIFT_MAX,
};
use crate::kampagne::{CampaignRunContract, DeadlinePolicy, WorkUnits};

/// Einheiten genau einer gemessenen Arbeitsklasse, getrennt nach Pflicht/Suche.
#[derive(Debug, Clone, Copy)]
pub struct WorkEstimate {
    /// Arbeitsklasse und damit die Einheit der zu verwendenden Rate.
    pub kind: Arbeitsart,
    /// Vollständig zu prüfende Einheiten.
    pub mandatory: u64,
    /// Zusätzliche Einheiten, deren Dauer die Pflichtzulassung nicht sperrt.
    pub optional: u64,
    /// Eingefrorener Umrechnungskurs, etwa 8 WU je DP-Zustand.
    pub work_units_per_unit: WorkUnits,
}

/// Messung und unabhängig erhobener aktueller Schlüssel desselben Rechners.
#[derive(Debug, Clone, Copy)]
pub struct CalibratedHost<'a> {
    /// Vollständige Messbelege; alte reine Durchsatzzahlen genügen nicht.
    pub profile: &'a Profil,
    /// Aktuelle Identität aus Host, Binärartefakt, Kernel und Taktregime.
    pub expected_key: &'a Profilschluessel,
}

/// Schätzung, keine Garantie eines abgeschlossenen Laufs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feasibility {
    /// Geschätzte Pflichtzeit auf der angegebenen Rechnermenge.
    pub mandatory_ms: f64,
    /// `None`, wenn für mindestens eine optionale Klasse keine Rate vorliegt.
    pub optional_ms: Option<f64>,
    /// Ganzzahlig ermittelte Pflichtkosten in der eingefrorenen Währung.
    pub mandatory_work_units: WorkUnits,
    /// Unverändert aus dem Laufvertrag übernommene Zielzeit, in ganzen
    /// Millisekunden (eine Zielzeit mit Bruchteil wird abgelehnt, nicht
    /// gerundet).
    pub target_ms: u64,
    /// Die Pflichtzeit überschreitet das ausdrücklich verlangte verbleibende
    /// Fristbudget. Ein späterer Runner muss `AnytimePartial` liefern.
    pub partial_requested: bool,
}

/// Ein Start darf bei diesen Fehlern nicht als vollständiger Auftrag erfolgen.
#[derive(Debug, Clone, PartialEq)]
pub enum FeasibilityError {
    /// Ungültige Auftragsdaten oder fehlender/inkompatibler Messbeleg.
    InvalidInput(String),
    /// Für eine Pflichtklasse existiert keine passende Rate.
    MissingRate(Arbeitsart),
    /// Pflichtkosten überschreiten die unveränderliche Vertragsmenge.
    WorkBudgetExceeded {
        /// Kosten der angegebenen Pflichtaufträge.
        required: WorkUnits,
        /// Eingefrorene Obergrenze aus dem Vertrag.
        available: WorkUnits,
    },
    /// Geschätzte Pflichtzeit überschreitet das Ziel ohne Teilauftragsfreigabe.
    TimeBudgetExceeded {
        /// Aus passenden Raten geschätzte Pflichtzeit.
        required_ms: f64,
        /// Verlangte Zielzeit.
        target_ms: u64,
    },
}

impl fmt::Display for FeasibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "Vorabprüfung: {message}"),
            Self::MissingRate(kind) => write!(f,
                "Vorabprüfung: keine gültige gemessene Rate für Pflichtarbeit {}. Zuerst diesen Auftrag kalibrieren.",
                kind.schluessel()),
            Self::WorkBudgetExceeded { required, available } => write!(f,
                "Pflichtarbeit ({} WU) überschreitet den eingefrorenen Vertrag ({} WU). Auftrag/Profil ausdrücklich korrigieren; keine stille Kürzung.",
                required.get(), available.get()),
            Self::TimeBudgetExceeded { required_ms, target_ms } => write!(f,
                "Auftrag in {target_ms} ms nicht erfüllbar.\n  Pflichtarbeit: {required_ms:.3} ms auf der aktuellen Rechnermenge (Schätzung).\n  Wege:\n    weitere passend kalibrierte Rechner zulassen (--hosts …)\n    Zielzeit erhöhen (--target-time mindestens {:.3}ms)\n    Teilergebnis ausdrücklich akzeptieren (--hard-deadline; liefert AnytimePartial)",
                required_ms.ceil()),
        }
    }
}

impl std::error::Error for FeasibilityError {}

fn invalid(message: impl Into<String>) -> FeasibilityError {
    FeasibilityError::InvalidInput(message.into())
}

fn valid_rate(rate: &RateModel) -> bool {
    rate.rate_per_s.is_finite()
        && rate.rate_per_s > 0.0
        && rate.spread.is_finite()
        && rate.spread >= 0.0
        && rate.spread < STREUUNG_MAX
        && rate.samples >= ABSCHNITTE_MIN
        && u128::from(rate.duration_ms) >= MINDESTDAUER.as_millis()
        && rate.reason.brauchbar()
        && rate
            .takt_drift
            .is_none_or(|drift| drift.is_finite() && (0.0..=TAKT_DRIFT_MAX).contains(&drift))
        && rate
            .psi_cpu
            .is_none_or(|psi| psi.is_finite() && (0.0..=100.0).contains(&psi))
}

/// Nächster größerer positiver Fließkommawert, auch bei einer Untergrenze 0.
fn upward(value: f64) -> f64 {
    if value.is_finite() {
        f64::from_bits(value.to_bits() + 1)
    } else {
        value
    }
}

/// Dauer einer tatsächlich ganzzahligen Belegung, ohne Abrundung der ETA.
fn host_job_ms(count: u64, rate: f64) -> f64 {
    if count == 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let rounded = count as f64;
    // Oberhalb 2^53 ist nicht jede Jobzahl als f64 darstellbar. Die Belegung
    // selbst bleibt u64; nur ihre Zeit wird nach oben eingeschlossen.
    let count_upper = if (rounded as u64) < count {
        upward(rounded)
    } else {
        rounded
    };
    let scaled = count_upper * 1000.0;
    let numerator = if count_upper.mul_add(1000.0, -scaled) > 0.0 {
        upward(scaled)
    } else {
        scaled
    };
    let duration = numerator / rate;
    if rate.mul_add(duration, -numerator) < 0.0 {
        upward(duration)
    } else {
        duration
    }
}

/// Addiert nichtnegative Zeiten, ohne einen positiven Rundungsrest zu verlieren.
fn add_ms(left: f64, right: f64) -> f64 {
    let sum = left + right;
    let larger = left.max(right);
    let smaller = left.min(right);
    if smaller - (sum - larger) > 0.0 {
        upward(sum)
    } else {
        sum
    }
}

/// Große ganzzahlige Fristen werden nicht versehentlich nach oben erweitert.
fn exceeds_time(estimate: f64, budget: u64) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let rounded = budget as f64;
    #[allow(clippy::cast_precision_loss)]
    let beyond_u64 = u64::MAX as f64;
    let lower = if rounded >= beyond_u64 || (rounded as u64) > budget {
        f64::from_bits(rounded.to_bits() - 1)
    } else {
        rounded
    };
    estimate > lower
}

/// Konstruiert eine zulässige ganzzahlige Belegung, nicht N/ΣRate Teiljobs.
fn whole_jobs_ms(count: u64, rates: &[f64]) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let fastest = rates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index)
        .expect("nonempty validated rates");
    let scale = rates[fastest];
    let normalized_sum: f64 = rates.iter().map(|rate| rate / scale).sum();
    let mut remaining = count;
    #[allow(clippy::cast_precision_loss)]
    let approximate_count = count as f64;
    let mut assigned: Vec<u64> = rates
        .iter()
        .map(|rate| {
            // Numerische Rundung darf höchstens die Platzierung ändern. Es
            // werden exakt count Jobs zugeteilt, niemals Bruchteile oder mehr.
            let share = (approximate_count * (rate / scale) / normalized_sum).floor() as u64;
            let jobs = share.min(remaining);
            remaining -= jobs;
            jobs
        })
        .collect();
    // Nach der proportionalen Grundbelegung fehlen rechnerisch weniger als
    // H Jobs. Wähle jeweils den nächsten tatsächlichen Jobabschluss. Die
    // Grenze H schützt auch bei extremen f64-Rundungen vor einer O(N)-Schleife.
    for _ in 0..rates.len() {
        if remaining == 0 {
            break;
        }
        let next = rates
            .iter()
            .enumerate()
            .min_by(|(a, rate_a), (b, rate_b)| {
                host_job_ms(assigned[*a] + 1, **rate_a)
                    .total_cmp(&host_job_ms(assigned[*b] + 1, **rate_b))
            })
            .map(|(index, _)| index)
            .expect("nonempty validated rates");
        assigned[next] += 1;
        remaining -= 1;
    }
    // Bei Zahlen oberhalb f64-Integerpräzision kann ein numerischer Rest
    // bleiben. Er wird als ganzer Block zugeteilt und vollständig berechnet.
    // Das ist eine konservative zulässige Belegung, keine Optimalitätszusage.
    assigned[fastest] += remaining;
    assigned
        .iter()
        .zip(rates)
        .map(|(&jobs, &rate)| host_job_ms(jobs, rate))
        .fold(0.0, f64::max)
}

/// Prüft Pflichtkosten und Zielzeit, bevor ein Aufrufer Arbeit startet.
///
/// Die Klassen werden sequenziell geschätzt; innerhalb einer Klasse werden
/// ganze Jobs auf Hosts verteilt. Fehlende optionale Raten blockieren keine
/// Pflichtarbeit. Fehlerhafte vorhandene Messungen werden immer abgelehnt.
/// `ActiveRuntime` wird gegen die tatsächliche Restfrist geprüft; bei deren
/// Überschreitung ist nur ein ausdrücklich markierter Teilauftrag zulässig.
/// `WallClockUtc` wird abgelehnt, weil diese API kein aktuelles UTC erhält.
/// Eine Frist hebt weder ungültige Eingaben noch den eingefrorenen WU-Vertrag auf.
pub fn preflight(
    contract: &CampaignRunContract,
    work: &[WorkEstimate],
    hosts: &[CalibratedHost<'_>],
) -> Result<Feasibility, FeasibilityError> {
    let target_ms = whole_ms(contract.target_wall_time, "Zielzeit")?;
    if target_ms == 0 {
        return Err(invalid("Zielzeit muss positiv sein"));
    }
    let remaining_deadline_ms = match contract.hard_deadline {
        None => None,
        Some(DeadlinePolicy::ActiveRuntime { remaining }) => {
            let remaining_ms = whole_ms(remaining, "Restfrist")?;
            if remaining_ms == 0 {
                return Err(invalid("ActiveRuntime-Frist ist bereits abgelaufen"));
            }
            Some(remaining_ms)
        }
        Some(DeadlinePolicy::WallClockUtc { .. }) => {
            return Err(invalid("WallClockUtc wird ohne explizites aktuelles UTC nicht unterstützt; ActiveRuntime verwenden"));
        }
    };
    if work.is_empty() {
        return Err(invalid("keine expliziten Arbeitsaufträge angegeben"));
    }
    let mut mandatory_work = 0_u64;
    let mut work_kinds = Vec::new();
    for item in work {
        if work_kinds.contains(&item.kind) {
            return Err(invalid(format!(
                "doppelte Arbeitsklasse {}",
                item.kind.schluessel()
            )));
        }
        work_kinds.push(item.kind);
        if item.work_units_per_unit.get() == 0 {
            return Err(invalid("WU je Einheit muss positiv sein"));
        }
        mandatory_work = item
            .mandatory
            .checked_mul(item.work_units_per_unit.get())
            .and_then(|value| mandatory_work.checked_add(value))
            .ok_or_else(|| invalid("Pflichtkosten überschreiten u64"))?;
    }
    if mandatory_work > contract.deterministic_work_budget.get() {
        return Err(FeasibilityError::WorkBudgetExceeded {
            required: WorkUnits(mandatory_work),
            available: contract.deterministic_work_budget,
        });
    }

    let mut seen_hosts = Vec::new();
    for host in hosts {
        let profile = host.profile;
        if profile.schluessel != *host.expected_key {
            return Err(invalid(format!(
                "inkompatibles Profil für {}: Host/Artefakt/Kernel/Taktregime geändert",
                profile.schluessel.host
            )));
        }
        if [
            &profile.schluessel.host,
            &profile.schluessel.artefakt,
            &profile.schluessel.kernel,
            &profile.schluessel.taktregime,
        ]
        .iter()
        .any(|part| part.trim().is_empty() || part.as_str() == "unbekannt")
        {
            return Err(invalid("Profilherkunft unvollständig"));
        }
        if seen_hosts.contains(&profile.schluessel.host.as_str()) {
            return Err(invalid(format!(
                "Rechner {} doppelt angegeben",
                profile.schluessel.host
            )));
        }
        seen_hosts.push(profile.schluessel.host.as_str());
        if profile.caps.threads_usable == 0
            || profile.caps.threads_usable > profile.caps.threads_total
        {
            return Err(invalid(format!(
                "ungültige nutzbare CPU-Menge für {}",
                profile.schluessel.host
            )));
        }
        let mut seen_kinds = Vec::new();
        for (kind, rate) in &profile.raten {
            if seen_kinds.contains(kind) || !valid_rate(rate) {
                return Err(invalid(format!(
                    "ungültige oder doppelte Rate {} auf {}",
                    kind.schluessel(),
                    profile.schluessel.host
                )));
            }
            seen_kinds.push(*kind);
            if *kind == Arbeitsart::AvxFullFight && !profile.caps.isa.vektorisierbar() {
                return Err(invalid(format!(
                    "AVX-Rate auf ungeeignetem Rechner {}",
                    profile.schluessel.host
                )));
            }
        }
    }

    let mut mandatory_ms = 0.0;
    let mut optional_ms = Some(0.0);
    for item in work {
        if item.mandatory == 0 && item.optional == 0 {
            continue;
        }
        let rates: Vec<f64> = hosts
            .iter()
            .flat_map(|host| &host.profile.raten)
            .filter(|(kind, _)| *kind == item.kind)
            .map(|(_, measured)| measured.rate_per_s)
            .collect();
        if rates.is_empty() {
            if item.mandatory > 0 {
                return Err(FeasibilityError::MissingRate(item.kind));
            }
            optional_ms = None;
            continue;
        }
        let required = whole_jobs_ms(item.mandatory, &rates);
        let optional = whole_jobs_ms(item.optional, &rates);
        mandatory_ms = add_ms(mandatory_ms, required);
        optional_ms = optional_ms.map(|value| add_ms(value, optional));
    }
    if !mandatory_ms.is_finite() || optional_ms.is_some_and(|value| !value.is_finite()) {
        return Err(invalid("geschätzte Dauer ist nicht endlich"));
    }
    let partial_requested =
        remaining_deadline_ms.is_some_and(|remaining| exceeds_time(mandatory_ms, remaining));
    if exceeds_time(mandatory_ms, target_ms) && remaining_deadline_ms.is_none() {
        return Err(FeasibilityError::TimeBudgetExceeded {
            required_ms: mandatory_ms,
            target_ms,
        });
    }
    Ok(Feasibility {
        mandatory_ms,
        optional_ms,
        mandatory_work_units: WorkUnits(mandatory_work),
        target_ms,
        partial_requested,
    })
}

/// Eine Vertragsdauer in ganzen Millisekunden — oder ein benannter Fehler.
///
/// Seit #914 sind Zielzeit und Restfrist `Duration`s, die Prüfung rechnet in
/// Millisekunden. **Abrunden wäre nicht harmlos:** Bei 1000 ms + 1 ns gälte
/// eine Pflichtzeit knapp über 1000 ms als Überschreitung, obwohl sie passt,
/// und die Meldung nennte eine Zielzeit, die niemand angegeben hat.
fn whole_ms(value: std::time::Duration, what: &str) -> Result<u64, FeasibilityError> {
    if !value.subsec_nanos().is_multiple_of(1_000_000) {
        return Err(invalid(format!(
            "{what} muss in ganzen Millisekunden angegeben sein: {value:?}"
        )));
    }
    u64::try_from(value.as_millis())
        .map_err(|_| invalid(format!("{what} überschreitet u64 Millisekunden: {value:?}")))
}
