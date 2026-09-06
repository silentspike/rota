//! Wie viel Arbeit zu einer Kampagne gehört — hardwareunabhängig (Issue #914).
//!
//! # Die Gefahr, gegen die dieses Modul gebaut ist
//!
//! Ein harter Zwei-Stunden-Abbruch ist **nicht reproduzierbar**. Fällt ein
//! Zusatzrechner aus, werden weniger Kandidaten geprüft; wird der Laptop
//! gedrosselt, endet der Lauf woanders. Das Ergebnis hinge an der Tageslast und
//! wäre als Balance-Grundlage wertlos.
//!
//! Der naheliegende Ausweg wäre, die Rechner zu kalibrieren und daraus das
//! Budget zu rechnen. **Das ist derselbe Fehler in anderer Form:** Ist nur der
//! Laptop erreichbar, entsteht aus denselben zwei Stunden ein kleineres Budget
//! — und damit wieder ein anderes Ergebnis. Der Plan hat diese Vorfassung am
//! 2026-08-15 ausdrücklich zurückgenommen.
//!
//! ```text
//! Die Kalibrierung bestimmt:   WO läuft die Arbeit · WIE LANGE dauert sie
//! Sie bestimmt NICHT:          WIE VIEL Arbeit zur Standardkampagne gehört
//! ```
//!
//! Deshalb sind es **zwei** Strukturen: [`CampaignDefinition`] ist versioniert
//! und hardwareunabhängig, [`RunDeployment`] beschreibt den einzelnen Lauf. Die
//! Trennung ist nicht Geschmack, sondern der ganze Zweck — und sie ist an
//! [`CampaignDefinition::fingerprint`] messbar, die das Deployment gar nicht
//! sehen kann, weil sie es nicht als Parameter nimmt.
//!
//! # Warum der Typ nicht `RunContract` heißt
//!
//! Der Architekturplan vergibt diesen Namen **zweimal**: an Zeile 2582 für den
//! Zielvertrag (Schwellen 99 % / 99,9 %, gebaut in [`crate::objective`] aus
//! #939) und an Zeile 4436 für den Laufvertrag hier. Die Feldmengen haben null
//! Überschneidung. Das gebaute umzubenennen hieße, fremden Code und zwei fremde
//! Prüfdateien für einen Namen anzufassen, den dieses Issue neu vergibt —
//! deshalb trägt das neue den unterscheidenden Namen
//! [`CampaignRunContract`], und die **Feldnamen** stammen wörtlich aus dem Plan.
//!
//! # Was hier NICHT steht
//!
//! Kein Rechenweg. Dieses Modul beschreibt Läufe, es rechnet keine. Die
//! Machbarkeitsprüfung („passt die Pflichtarbeit ins Budget?"), der
//! Budgetverteiler und das Qualitätszertifikat sind eigene Issues.

use std::fmt::Write;

use crate::checksum::{mix_int, FNV_OFFSET};

/// Ein Vertragswert, den JSON nicht als Zahl darstellen kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonError {
    /// NaN oder Unendlich an der benannten Stelle. `None` ist davon getrennt
    /// und bleibt bei optionalen Angaben das gültige JSON `null`.
    NonFinite {
        /// Feld des Vertrags oder Referenzrechners.
        field: &'static str,
    },
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "{field}: Zahl muss endlich sein"),
        }
    }
}

impl std::error::Error for JsonError {}

/// Eine Arbeitseinheit — die hardwareunabhängige Währung dieses Moduls.
///
/// **Bewusst keine Zeit und keine Kampfzahl.** Eine Zeitangabe wäre
/// maschinenabhängig, und die Zahl der Kämpfe wäre es indirekt auch, sobald ein
/// schnellerer Pfad dieselbe Aussage mit weniger Kämpfen erreicht. Eine
/// Arbeitseinheit ist ein abstraktes Maß, das die Kampagne festlegt und kein
/// Rechner verhandelt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkUnits(pub u64);

impl WorkUnits {
    /// Der Wert als Zahl.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Wozu ein Lauf da ist.
///
/// Die Mission steht im Vertrag, weil zwei Läufe mit gleichem Budget und
/// verschiedener Mission **nicht** vergleichbar sind — und ein Vergleich, den
/// niemand verboten hat, wird irgendwann gezogen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mission {
    /// Die vollständige Balance-Kampagne über alle Instanzen.
    FullBalance,
    /// Eine einzelne Instanz.
    SingleInstance {
        /// Nummer der Instanz.
        instance: u8,
    },
    /// Ein gezielter Nachlauf über einen einzelnen Boss.
    SingleBoss {
        /// Kennung des Bosses, wie in den Bossdaten.
        boss: &'static str,
    },
}

/// Was **vollständig** gerechnet werden muss, nicht nur gesucht.
///
/// Die Zahl ist eine Menge von Projektionen, keine Obergrenze: Sie sind
/// Pflicht, und ein Lauf, der sie nicht erledigt hat, ist nicht fertig — egal
/// wie viel Budget er sonst verbraucht hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectionSet {
    /// Wie viele Pflichtprojektionen zu dieser Kampagne gehören.
    pub required: u16,
}

/// Die vorgeschriebene Wechselwirkungsabdeckung (t-Wege, siehe E21.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverageSpec {
    /// Die Stärke t: Für welche Grösse von Kombinationen wird Abdeckung
    /// verlangt.
    pub strength: u8,
}

/// Welche statistische Sicherheit ein Ergebnis tragen muss.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceSpec {
    /// Verlangte Sicherheit, etwa 0.99.
    pub level: f64,
}

/// Was am Ende eines Laufs nachgewiesen wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditPolicy {
    /// Der skalare Referenzpfad läuft über jede finale Aussage nach.
    ScalarReplayAll,
    /// Nur über die Aussagen, die eine Entscheidung tragen.
    ScalarReplayDecisive,
}

/// Was eine Frist **bedeutet** (AC6).
///
/// Ein prozesslokaler Zeitzähler überlebt keinen Neustart und ist damit für
/// einen Lauf, der pausiert und fortgesetzt werden kann, unbrauchbar. Deshalb
/// ist die Bedeutung benannt statt unterstellt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeadlinePolicy {
    /// „Um 08:00 muss es fertig sein" — Wanduhr in UTC, als Sekunden seit der
    /// Epoche.
    ///
    /// Richtig, wenn ein Termin dahintersteht. **Nicht** reproduzierbar: Zwei
    /// Läufe derselben Kampagne enden bei verschiedener Startzeit an
    /// verschiedenen Stellen.
    WallClockUtc {
        /// Sekunden seit der Unix-Epoche.
        deadline_unix_s: i64,
    },
    /// „Zwei Stunden RECHENZEIT" — zählt nur, was wirklich gerechnet wurde.
    ///
    /// Für ein reproduzierbares Teilergebnis die sauberere Wahl.
    ActiveRuntime {
        /// Verbleibende Rechenzeit in Millisekunden.
        remaining_ms: u64,
    },
}

/// Die drei Festlegungen, ohne die `ActiveRuntime` mehrdeutig bleibt.
///
/// Der Plan verlangt sie ausdrücklich: Zählt Pausezeit? Zählt ein
/// Koordinatorausfall? Zählt Wartezeit auf Zusatzrechner? **Ohne Antwort misst
/// jeder Lauf etwas anderes und nennt es gleich.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeAccounting {
    /// Zählt die Zeit, in der ein Lauf ausdrücklich pausiert ist?
    pub counts_paused_time: bool,
    /// Zählt die Zeit, in der der Koordinator ausgefallen ist?
    pub counts_coordinator_outage: bool,
    /// Zählt die Wartezeit auf Zusatzrechner, die noch nicht antworten?
    pub counts_host_wait: bool,
}

impl RuntimeAccounting {
    /// Die reproduzierbare Wahl: Es zählt nur, was gerechnet wurde.
    ///
    /// Alle drei stehen auf `false`, und das ist kein Vorgabewert aus Bequemheit
    /// — jede `true`-Antwort macht die verbrauchte Frist von etwas abhängig, das
    /// nicht die Arbeit ist.
    pub const REPRODUCIBLE: Self = Self {
        counts_paused_time: false,
        counts_coordinator_outage: false,
        counts_host_wait: false,
    };
}

/// Wie viele physische Kerne für einen Lauf vorgesehen sind (AC4).
///
/// # Warum „Kerne minus zwei" nicht genügt
///
/// Auf einer Maschine mit 8 physischen Kernen und 16 Strängen ist die Angabe
/// **dreideutig**: sechs Stränge, vierzehn oder zwölf. Verbindlich ist das
/// Letzte — zwei physische Kerne **samt ihren SMT-Geschwistern**, also zwölf
/// nutzbare Stränge.
///
/// Dies ist eine rechnerische Vorgabe für homogene SMT-Topologien. Die Struktur
/// kennt weder CPU-IDs noch Geschwisterzuordnungen und setzt keine Affinität.
/// Die tatsächliche Kernbindung muss der spätere Verteiler nachweisen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuReservation {
    /// Physische Kerne der Maschine.
    pub physical_cores: u16,
    /// Stränge je physischem Kern (1 ohne SMT, sonst meist 2).
    pub threads_per_core: u16,
    /// Wie viele physische Kerne reserviert bleiben — samt Geschwistern.
    pub reserved_physical_cores: u16,
}

impl CpuReservation {
    /// Wie viele Stränge die Maschine insgesamt hat.
    #[must_use]
    pub const fn total_threads(self) -> u32 {
        self.physical_cores as u32 * self.threads_per_core as u32
    }

    /// Wie viele Stränge dem Rechnen bleiben.
    ///
    /// Reserviert werden **ganze** Kerne: Ein reservierter Kern nimmt seine
    /// SMT-Geschwister mit. Genau hier entsteht der Unterschied zwischen zwölf
    /// und vierzehn.
    #[must_use]
    pub const fn compute_threads(self) -> u32 {
        let frei = self
            .physical_cores
            .saturating_sub(self.reserved_physical_cores);
        frei as u32 * self.threads_per_core as u32
    }

    /// Rechnerische Aufteilung ohne Behauptung einer tatsächlichen Kernbindung.
    /// Auch eine leere oder widersprüchliche Vorgabe bleibt darstellbar; eine
    /// übergroße Reservierung wird ausdrücklich als ungültig ausgewiesen.
    #[must_use]
    pub fn describe(self) -> String {
        let mut text = format!(
            "Compute: {} Stränge (rechnerisch)\n\
             Reservierung: {} von {} physischen Kernen samt SMT-Geschwistern\n\
             CPU-IDs und tatsächliche Kernbindung: nicht ermittelt",
            self.compute_threads(),
            self.reserved_physical_cores,
            self.physical_cores,
        );
        if self.reserved_physical_cores > self.physical_cores {
            text.push_str("\nUngültig: mehr Kerne reserviert als vorhanden");
        }
        if self.physical_cores > 0 && self.threads_per_core == 0 {
            text.push_str("\nUngültig: vorhandene Kerne haben null Stränge");
        }
        text
    }
}

/// Was ein Rechner mindestens mitbringen muss (AC3).
///
/// **„Rechner erreichbar" genügt nicht.** In einer geteilten Umgebung sagt
/// Erreichbarkeit nichts über die Zeit: Eine Maschine, die antwortet und dabei
/// 5 % ihrer Rechenzeit bekommt, ist für einen Balance-Lauf so gut wie
/// abwesend — nur merkt es niemand, weil sie ja „da" ist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostRequirement {
    /// Name des Rechners im Referenzprofil.
    pub name: &'static str,
    /// Zugesicherter Anteil an Rechenzeit, 1.0 = ein voller Strang.
    ///
    /// `None` heißt ausdrücklich **best effort** — nicht „unbekannt". Der
    /// Unterschied gehört benannt, weil ein unbekannter Wert später geschätzt
    /// wird und ein ausdrückliches „best effort" nicht.
    pub guaranteed_cpu_quota: Option<f64>,
    /// Zugesicherter Speicher in Mebibyte.
    pub guaranteed_memory_mib: u32,
    /// Die CPU-Aufteilung dieses Rechners.
    pub cpu: CpuReservation,
    /// Obergrenze für die Umlaufzeit im Netz, in Millisekunden.
    pub max_rtt_ms: u32,
    /// Untergrenze für die Bandbreite, in Mebibit je Sekunde.
    pub min_bandwidth_mibit_s: u32,
    /// Wird die Maschine im Betrieb aktiv gedrosselt?
    ///
    /// Steht hier, weil eine gedrosselte Maschine dieselbe Arbeit in anderer
    /// Zeit leistet — und das Referenzprofil Zeit **ableitet**, nicht misst.
    pub thermal_throttling: bool,
}

/// Das Referenzprofil: die Maschinen, aus denen das kanonische Budget abgeleitet
/// wurde (AC3).
///
/// Es ist **Herkunftsangabe, keine Anforderung an den Lauf.** Ein Lauf auf
/// schwächerer Hardware ist erlaubt und dauert länger; er rechnet dieselbe
/// Arbeitsmenge. Genau deshalb steht das Profil in der versionierten Definition
/// und nicht im Deployment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceProfile {
    /// Version dieses Profils.
    pub version: u32,
    /// Die Rechner, aus denen abgeleitet wurde.
    pub hosts: &'static [HostRequirement],
}

/// Welche Qualität ein Ergebnis erreichen muss.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityTargets {
    /// Mindestanteil abgedeckter Nischen.
    pub minimum_niche_coverage: f64,
    /// Wie viele unabhängige Neustarts die Suche fährt.
    pub independent_restarts: u16,
    /// Nach wie vielen Arbeitseinheiten ohne Verbesserung abgebrochen wird —
    /// die Abbruchregel, endlich benannt.
    pub plateau_work_units: WorkUnits,
    /// Erlaubter Abstand zwischen unterer und oberer Schranke, falls verlangt.
    pub maximum_bound_gap: Option<f64>,
    /// Verlangte statistische Sicherheit.
    pub confidence_requirement: ConfidenceSpec,
    /// Was am Ende nachgewiesen wird.
    pub final_audit: AuditPolicy,
}

/// Die versionierte, hardwareunabhängige Kampagne (AC1, AC7).
///
/// # Warum eine neue Zielzeit ein NEUES Profil erzeugt
///
/// Ein bereits versioniertes Profil hat feste Werteinheiten und wird **nicht**
/// nachträglich umgerechnet. Sonst hätten zwei Ergebnisse denselben
/// Versionsstempel und verschiedene Arbeitsmengen — und der Stempel wäre wertlos
/// genau in dem Moment, in dem ihn jemand braucht.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CampaignDefinition {
    /// Version dieser Kampagne. Ändert sich die Arbeitsmenge, ändert sich diese
    /// Zahl.
    pub profile_version: u32,
    /// Das Referenzprofil, aus dem das Budget abgeleitet wurde.
    pub reference_profile: ReferenceProfile,
    /// Die eingefrorene Arbeitsmenge.
    ///
    /// **Aus dem Referenzprofil abgeleitet, dann fest.** Nicht aus der
    /// Kalibrierung des laufenden Rechners — das ist der Fehler, gegen den
    /// dieses ganze Modul steht.
    pub canonical_work_budget: WorkUnits,
    /// Was vollständig gerechnet werden muss.
    pub exact_obligations: ProjectionSet,
    /// Die vorgeschriebene Wechselwirkungsabdeckung.
    pub coverage_spec: CoverageSpec,
    /// Welche Qualität erreicht werden muss.
    pub quality_targets: QualityTargets,
}

impl CampaignDefinition {
    /// Der Fingerabdruck der Kampagne — die Zahl, an der AC2 hängt.
    ///
    /// # Warum das Deployment kein Parameter ist
    ///
    /// Das ist die Zusicherung selbst, nicht ihre Umsetzung: Eine Funktion, die
    /// das [`RunDeployment`] gar nicht **sehen** kann, kann es auch nicht
    /// versehentlich einrühren. Ein späterer Zusatzparameter wäre eine sichtbare
    /// Signaturänderung und keine stille Regression.
    ///
    /// Das Verfahren ist FNV-1a über 32 Bit wie im übrigen Projekt
    /// ([`crate::checksum`]) — kein zweites Hash-Verfahren, damit niemand zwei
    /// Fingerabdrücke desselben Gegenstands vergleicht.
    ///
    /// Gleitkommawerte gehen über ihre **Bits** ein: Zwei Zahlen, die sich im
    /// letzten Bit unterscheiden, drucken dieselbe Zeile und müssen trotzdem
    /// verschiedene Fingerabdrücke ergeben.
    ///
    /// Format 2 bindet auch den vollständigen Inhalt des Referenzprofils ein,
    /// einschließlich Reihenfolge und Anzahl seiner Rechner. Die Herkunft ist
    /// Teil der versionierten Definition; die aktuelle Hardware im Deployment
    /// bleibt ausgeschlossen. Das ändert weder Budget noch Arbeitsmenge.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h = mix_int(FNV_OFFSET, 2);
        h = mix_int(h, self.profile_version as i32);
        h = mix_int(h, self.reference_profile.version as i32);
        h = mix_u64(h, self.reference_profile.hosts.len() as u64);
        for host in self.reference_profile.hosts {
            h = mix_text(h, host.name);
            h = match host.guaranteed_cpu_quota {
                Some(quota) => mix_f64(mix_int(h, 1), quota),
                None => mix_int(h, 0),
            };
            h = mix_int(h, host.guaranteed_memory_mib as i32);
            h = mix_int(h, i32::from(host.cpu.physical_cores));
            h = mix_int(h, i32::from(host.cpu.threads_per_core));
            h = mix_int(h, i32::from(host.cpu.reserved_physical_cores));
            h = mix_int(h, host.max_rtt_ms as i32);
            h = mix_int(h, host.min_bandwidth_mibit_s as i32);
            h = mix_int(h, i32::from(host.thermal_throttling));
        }
        h = mix_u64(h, self.canonical_work_budget.get());
        h = mix_int(h, i32::from(self.exact_obligations.required));
        h = mix_int(h, i32::from(self.coverage_spec.strength));
        h = mix_f64(h, self.quality_targets.minimum_niche_coverage);
        h = mix_int(h, i32::from(self.quality_targets.independent_restarts));
        h = mix_u64(h, self.quality_targets.plateau_work_units.get());
        // Ein fehlender Wert und der Wert 0.0 sind verschiedene Aussagen —
        // "keine Schranke verlangt" gegen "Schranke muss exakt sein". Ohne die
        // Kennzeichnung wären beide derselbe Fingerabdruck.
        h = match self.quality_targets.maximum_bound_gap {
            Some(g) => mix_f64(mix_int(h, 1), g),
            None => mix_int(h, 0),
        };
        h = mix_f64(h, self.quality_targets.confidence_requirement.level);
        mix_int(h, audit_code(self.quality_targets.final_audit))
    }
}

impl CampaignDefinition {
    /// Die Kampagne als JSON — samt ihrem Fingerabdruck.
    ///
    /// Von Hand geschrieben statt über eine Serialisierungs-Kiste, weil die
    /// Bibliothek **abhängigkeitsfrei** bleibt (siehe `sim/Cargo.toml`): Eine
    /// Kiste, die im Schritt Speicher anfordert, würde den Determinismus
    /// unterlaufen, der der Zweck dieses Simulators ist.
    ///
    /// **Der Fingerabdruck steht mit im Text.** Ohne ihn ließe sich eine
    /// zurückgelesene Kampagne nicht dagegen prüfen, ob sie unterwegs verändert
    /// wurde — und genau das ist der Fall, für den es ihn gibt.
    ///
    /// Fliesskommazahlen stehen in der kürzesten verlustfrei rücklesbaren Form
    /// (`{:?}`). Zu runden wäre hier falsch: Die letzte Stelle ist die, an der
    /// sich eine veränderte Vorgabe verrät.
    ///
    /// # Errors
    /// Nichtendliche Zahlen werden mit Feldname abgelehnt. Sie dürfen weder
    /// ungültiges JSON erzeugen noch als fehlende Vorgabe (`null`) erscheinen.
    pub fn to_json(&self) -> Result<String, JsonError> {
        let hosts = self
            .reference_profile
            .hosts
            .iter()
            .map(HostRequirement::to_json)
            .collect::<Result<Vec<_>, _>>()?
            .join(",");
        let q = &self.quality_targets;
        Ok(format!(
            "{{\"profileVersion\":{},\"fingerprint\":{},\
             \"referenceProfile\":{{\"version\":{},\"hosts\":[{}]}},\
             \"canonicalWorkBudget\":{},\"exactObligations\":{},\
             \"coverageStrength\":{},\"minimumNicheCoverage\":{},\
             \"independentRestarts\":{},\"plateauWorkUnits\":{},\
             \"maximumBoundGap\":{},\"confidence\":{},\"finalAudit\":{:?}}}",
            self.profile_version,
            self.fingerprint(),
            self.reference_profile.version,
            hosts,
            self.canonical_work_budget.get(),
            self.exact_obligations.required,
            self.coverage_spec.strength,
            json_number(q.minimum_niche_coverage, "minimumNicheCoverage")?,
            q.independent_restarts,
            q.plateau_work_units.get(),
            // `null` und eine Zahl sind verschiedene Aussagen — "keine Schranke
            // verlangt" gegen "Schranke muss exakt sein". Eine 0.0 an dieser
            // Stelle waere die zweite, gemeint ist beim fehlenden Wert die erste.
            json_optional_number(q.maximum_bound_gap, "maximumBoundGap")?,
            json_number(q.confidence_requirement.level, "confidence")?,
            audit_name(q.final_audit),
        ))
    }
}

impl HostRequirement {
    /// Einen Rechner des Referenzprofils als JSON.
    ///
    /// # Errors
    /// Eine vorhandene CPU-Quote muss endlich sein.
    pub fn to_json(&self) -> Result<String, JsonError> {
        Ok(format!(
            "{{\"name\":{},\"guaranteedCpuQuota\":{},\
             \"guaranteedMemoryMib\":{},\"computeThreads\":{},\
             \"totalThreads\":{},\"physicalCores\":{},\"threadsPerCore\":{},\
             \"reservedPhysicalCores\":{},\"maxRttMs\":{},\
             \"minBandwidthMibitS\":{},\"thermalThrottling\":{}}}",
            json_string(self.name),
            // `null` heisst hier ausdruecklich "best effort", nicht "unbekannt".
            // Eine 0.0 waere die dritte, ganz andere Aussage: garantiert nichts.
            json_optional_number(self.guaranteed_cpu_quota, "guaranteedCpuQuota")?,
            self.guaranteed_memory_mib,
            self.cpu.compute_threads(),
            self.cpu.total_threads(),
            self.cpu.physical_cores,
            self.cpu.threads_per_core,
            self.cpu.reserved_physical_cores,
            self.max_rtt_ms,
            self.min_bandwidth_mibit_s,
            self.thermal_throttling,
        ))
    }
}

impl CampaignRunContract {
    /// Den Laufvertrag als JSON.
    ///
    /// `campaignDefinitionHash` ist das Feld, das später belegt, **gegen welche
    /// Arbeitsmenge** ein Ergebnis entstanden ist. Ein Ergebnis ohne diese Zahl
    /// ist mit keinem anderen vergleichbar.
    ///
    /// # Errors
    /// Nichtendliche Qualitätsvorgaben werden mit Feldname abgelehnt.
    pub fn to_json(&self) -> Result<String, JsonError> {
        Ok(format!(
            "{{\"mission\":{},\"targetWallTimeMs\":{},\
             \"campaignDefinitionHash\":{},\"deterministicWorkBudget\":{},\
             \"hardDeadline\":{},\"runtimeAccounting\":{},\
             \"exactObligations\":{},\"coverageStrength\":{},\
             \"minimumNicheCoverage\":{},\"independentRestarts\":{},\
             \"plateauWorkUnits\":{},\"maximumBoundGap\":{},\
             \"confidence\":{},\"finalAudit\":{:?}}}",
            mission_json(self.mission),
            self.target_wall_time_ms,
            self.campaign_definition_hash,
            self.deterministic_work_budget.get(),
            // Fehlt die Frist, steht `null` — nicht etwa eine sehr grosse Zahl.
            // Ein Vorgabewert an dieser Stelle waere eine Frist, die niemand
            // verlangt hat (AC5).
            match self.hard_deadline {
                Some(DeadlinePolicy::WallClockUtc { deadline_unix_s }) =>
                    format!("{{\"kind\":\"wallClockUtc\",\"deadlineUnixS\":{deadline_unix_s}}}"),
                Some(DeadlinePolicy::ActiveRuntime { remaining_ms }) =>
                    format!("{{\"kind\":\"activeRuntime\",\"remainingMs\":{remaining_ms}}}"),
                None => "null".to_string(),
            },
            format_args!(
                "{{\"countsPausedTime\":{},\"countsCoordinatorOutage\":{},\
                 \"countsHostWait\":{}}}",
                self.runtime_accounting.counts_paused_time,
                self.runtime_accounting.counts_coordinator_outage,
                self.runtime_accounting.counts_host_wait,
            ),
            self.exact_obligations.required,
            self.interaction_coverage.strength,
            json_number(self.minimum_niche_coverage, "minimumNicheCoverage")?,
            self.independent_restarts,
            self.plateau_work_units.get(),
            json_optional_number(self.maximum_bound_gap, "maximumBoundGap")?,
            json_number(self.confidence_requirement.level, "confidence")?,
            audit_name(self.final_audit),
        ))
    }
}

/// Die Mission als JSON-Wert.
fn mission_json(m: Mission) -> String {
    match m {
        Mission::FullBalance => "{\"kind\":\"fullBalance\"}".to_string(),
        Mission::SingleInstance { instance } => {
            format!("{{\"kind\":\"singleInstance\",\"instance\":{instance}}}")
        }
        Mission::SingleBoss { boss } => {
            format!("{{\"kind\":\"singleBoss\",\"boss\":{}}}", json_string(boss))
        }
    }
}

/// Rusts Debug-Escapes (`\\0`, `\\u{...}`) sind kein JSON. Nur die
/// vorgeschriebenen Steuerzeichen werden maskiert; Unicode bleibt UTF-8.
fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0000}'..='\u{001f}' => {
                write!(out, "\\u{:04x}", ch as u32).expect("String ist beschreibbar");
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn json_number(value: f64, field: &'static str) -> Result<String, JsonError> {
    if !value.is_finite() {
        return Err(JsonError::NonFinite { field });
    }
    Ok(format!("{value:?}"))
}

fn json_optional_number(value: Option<f64>, field: &'static str) -> Result<String, JsonError> {
    match value {
        Some(number) => json_number(number, field),
        None => Ok("null".to_owned()),
    }
}

/// Der Name der Auditregel im JSON — stabil, unabhaengig von der Reihenfolge im
/// `enum`.
const fn audit_name(policy: AuditPolicy) -> &'static str {
    match policy {
        AuditPolicy::ScalarReplayAll => "scalarReplayAll",
        AuditPolicy::ScalarReplayDecisive => "scalarReplayDecisive",
    }
}

/// Eine 64-Bit-Zahl über zwei 32-Bit-Hälften einrühren, niederwertige zuerst.
const fn mix_u64(hash: u32, value: u64) -> u32 {
    let low = value as u32;
    let high = (value >> 32) as u32;
    mix_int(mix_int(hash, low as i32), high as i32)
}

/// Länge und UTF-8-Bytes binden auch leere Namen und Feldgrenzen eindeutig.
fn mix_text(hash: u32, value: &str) -> u32 {
    value
        .bytes()
        .fold(mix_u64(hash, value.len() as u64), |h, byte| {
            mix_int(h, i32::from(byte))
        })
}

/// Eine Fliesskommazahl über ihre Bits einrühren.
const fn mix_f64(hash: u32, value: f64) -> u32 {
    mix_u64(hash, value.to_bits())
}

/// Jede Variante bekommt eine feste Zahl.
///
/// **Nicht die Reihenfolge im `enum` benutzen:** Eine später eingefügte Variante
/// verschöbe alle folgenden und änderte jeden bereits vergebenen Fingerabdruck,
/// ohne dass sich eine Kampagne geändert hätte.
const fn audit_code(policy: AuditPolicy) -> i32 {
    match policy {
        AuditPolicy::ScalarReplayAll => 1,
        AuditPolicy::ScalarReplayDecisive => 2,
    }
}

/// Ein einzelner Rechner, wie er in diesem Lauf tatsächlich zur Verfügung steht.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeployedHost {
    /// Name des Rechners.
    pub name: &'static str,
    /// Tatsächlich zugeteilter Anteil an Rechenzeit.
    pub cgroup_quota: f64,
    /// Gemessene Leistung relativ zum Referenzprofil, 1.0 = wie dort.
    pub runtime_perf_profile: f64,
    /// Die CPU-Aufteilung dieses Rechners in diesem Lauf.
    pub cpu: CpuReservation,
}

/// Wie ein einzelner Lauf verteilt wird (AC1) — dynamisch, nicht versioniert.
///
/// **Diese Struktur darf das Ergebnis nicht beeinflussen.** Sie bestimmt, wo
/// gerechnet wird und wie lange es dauert; wie viel gerechnet wird, steht in der
/// [`CampaignDefinition`]. Wer beides vermischt, macht das Ergebnis von der
/// Tagesform abhängig — und merkt es nicht, weil jeder einzelne Lauf für sich
/// stimmig aussieht.
#[derive(Debug, Clone, PartialEq)]
pub struct RunDeployment {
    /// Die Rechner, die diesem Lauf zur Verfügung stehen.
    pub available_hosts: Vec<DeployedHost>,
    /// Geschätzte Dauer in Millisekunden — eine **Schätzung**, kein Vertrag.
    pub estimated_eta_ms: u64,
}

impl RunDeployment {
    /// Die Summe der nutzbaren Stränge über alle Rechner.
    #[must_use]
    pub fn total_compute_threads(&self) -> u64 {
        self.available_hosts
            .iter()
            .map(|h| u64::from(h.cpu.compute_threads()))
            .sum()
    }
}

/// Der Laufvertrag (AC0).
///
/// Der Name unterscheidet ihn vom Zielvertrag [`crate::objective::RunContract`]
/// aus #939, der denselben Namen im Plan trägt und etwas völlig anderes
/// beschreibt — siehe Modulkopf.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CampaignRunContract {
    /// Wozu dieser Lauf da ist.
    pub mission: Mission,
    /// Die angestrebte Wanduhrzeit in Millisekunden — ein **Ziel**, keine Frist.
    pub target_wall_time_ms: u64,
    /// Woher das Budget stammt: der Fingerabdruck der Kampagne.
    ///
    /// Ohne dieses Feld ließe sich hinterher nicht mehr feststellen, gegen
    /// welche Arbeitsmenge ein Ergebnis entstanden ist.
    pub campaign_definition_hash: u32,
    /// Die eingefrorene Arbeitsmenge, übernommen aus der Kampagne.
    pub deterministic_work_budget: WorkUnits,
    /// Ein echter Zeitabbruch — **nur auf ausdrücklichen Wunsch** (AC5).
    ///
    /// `None` ist die Voreinstellung, und das ist eine inhaltliche Festlegung:
    /// Ein Lauf bricht nach erledigter Arbeit ab, nicht nach Ablauf einer Uhr.
    pub hard_deadline: Option<DeadlinePolicy>,
    /// Wie Rechenzeit gezählt wird, falls eine Frist gesetzt ist.
    pub runtime_accounting: RuntimeAccounting,
    /// Was vollständig gerechnet werden muss.
    pub exact_obligations: ProjectionSet,
    /// Die vorgeschriebene Wechselwirkungsabdeckung.
    pub interaction_coverage: CoverageSpec,
    /// Mindestanteil abgedeckter Nischen.
    pub minimum_niche_coverage: f64,
    /// Wie viele unabhängige Neustarts.
    pub independent_restarts: u16,
    /// Die Abbruchregel bei ausbleibender Verbesserung.
    pub plateau_work_units: WorkUnits,
    /// Erlaubter Schrankenabstand, falls verlangt.
    pub maximum_bound_gap: Option<f64>,
    /// Verlangte statistische Sicherheit.
    pub confidence_requirement: ConfidenceSpec,
    /// Was am Ende nachgewiesen wird.
    pub final_audit: AuditPolicy,
}

impl CampaignRunContract {
    /// Einen Vertrag aus einer Kampagne ableiten.
    ///
    /// Die Arbeitsmenge wird **übernommen**, nicht gerechnet — und der
    /// Fingerabdruck wandert mit, damit später belegbar bleibt, woher sie kam.
    /// Das Deployment ist hier kein Parameter, aus demselben Grund wie bei
    /// [`CampaignDefinition::fingerprint`].
    #[must_use]
    pub fn from_campaign(
        campaign: &CampaignDefinition,
        mission: Mission,
        target_wall_time_ms: u64,
    ) -> Self {
        Self {
            mission,
            target_wall_time_ms,
            campaign_definition_hash: campaign.fingerprint(),
            deterministic_work_budget: campaign.canonical_work_budget,
            hard_deadline: None,
            runtime_accounting: RuntimeAccounting::REPRODUCIBLE,
            exact_obligations: campaign.exact_obligations,
            interaction_coverage: campaign.coverage_spec,
            minimum_niche_coverage: campaign.quality_targets.minimum_niche_coverage,
            independent_restarts: campaign.quality_targets.independent_restarts,
            plateau_work_units: campaign.quality_targets.plateau_work_units,
            maximum_bound_gap: campaign.quality_targets.maximum_bound_gap,
            confidence_requirement: campaign.quality_targets.confidence_requirement,
            final_audit: campaign.quality_targets.final_audit,
        }
    }

    /// Einen Zeitabbruch ausdrücklich verlangen (AC5).
    ///
    /// Es gibt bewusst **keinen** Weg, eine Frist als Voreinstellung zu
    /// bekommen: Sie muss durch diesen Aufruf gehen, und der steht im Diff.
    #[must_use]
    pub const fn with_hard_deadline(mut self, policy: DeadlinePolicy) -> Self {
        self.hard_deadline = Some(policy);
        self
    }
}

/// Welche Art von Zusage ein Ergebnis trägt.
///
/// Die ersten vier stammen aus Teil 4 des Plans; [`ResultGuarantee::AnytimePartial`]
/// ist die fünfte aus diesem Issue — im Betrieb die häufigste, weil sie jeden
/// Lauf betrifft, der nach Zeit abgeschnitten wurde.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultGuarantee {
    /// Exakt, mit nachgewiesener Voraussetzung.
    CertifiedExact,
    /// Bitgleich zum skalaren Pfad, nur schneller gerechnet.
    BitExactOptimized,
    /// Genähert, mit Schranken.
    BoundedApproximation {
        /// Untere Schranke.
        lower: f64,
        /// Obere Schranke.
        upper: f64,
    },
    /// Geschätzt, mit Sicherheit.
    StatisticalEstimate {
        /// Erreichte Sicherheit.
        confidence: f64,
    },
    /// **Teilergebnis** eines abgeschnittenen Laufs (AC5).
    ///
    /// Es meldet sich als solches und nennt beide Zahlen. Ein Teilergebnis, das
    /// nur die erledigte Arbeit nennt, sieht aus wie ein fertiges — der
    /// Vergleich mit der geplanten Arbeit ist die ganze Aussage.
    AnytimePartial {
        /// Tatsächlich erledigte Arbeit.
        work_completed: WorkUnits,
        /// Geplante Arbeit laut Vertrag.
        work_planned: WorkUnits,
    },
}

impl ResultGuarantee {
    /// Ist dieses Ergebnis vollständig?
    ///
    /// Ein `AnytimePartial` ist genau dann vollständig, wenn es sein Budget
    /// erreicht hat — dann war der Zeitabbruch folgenlos.
    #[must_use]
    pub fn is_complete(self) -> bool {
        match self {
            Self::AnytimePartial {
                work_completed,
                work_planned,
            } => work_completed >= work_planned,
            _ => true,
        }
    }
}
