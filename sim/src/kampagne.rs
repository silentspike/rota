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
//! Der Architekturplan vergab diesen Namen bis zum 2026-08-22 **zweimal**: an
//! den Zielvertrag (Schwellen 99 % / 99,9 %, gebaut in [`crate::objective`] aus
//! #939) und an den Laufvertrag hier. Die Feldmengen haben null Überschneidung.
//! Umbenannt wurde der beschriebene, nicht der gebaute — deshalb trägt der
//! Laufvertrag den Namen [`CampaignRunContract`], und seine **Feldnamen und
//! -typen** stehen wörtlich wie im Plan (`docs/SIM-ARCHITEKTUR-PLAN.md`, E21.0).
//! Das einzige Feld darüber hinaus ist `runtime_accounting`: AC6 verlangt die
//! drei Festlegungen ausdrücklich „im Vertrag".
//!
//! # Das Referenzprofil „standard"
//!
//! [`REFERENCE_PROFILE_STANDARD`] ist **Laptop + node3**. Der Plan nannte
//! „Laptop + proxmox + gonet"; der Auftraggeber hat die Zusatzrechner am
//! 2026-10-28 neu festgelegt: nur node3 (Proxmox-Knoten `ovh-pve1`, AMD Ryzen 7
//! 9700X). gonet und 10.0.0.69 gehören nicht mehr dazu.
//!
//! # Was hier NICHT steht
//!
//! Kein Rechenweg. Dieses Modul beschreibt Läufe, es rechnet keine. Die
//! Machbarkeitsprüfung („passt die Pflichtarbeit ins Budget?") steht in
//! [`crate::machbarkeit`]; der Budgetverteiler und das Qualitätszertifikat
//! sind eigene Issues.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;
use std::time::Duration;

use crate::checksum::{mix_int, FNV_OFFSET};
use crate::hostcaps::kernliste;

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

/// Der Fingerabdruck einer Kampagne — im Plan der Typ `Hash`.
///
/// FNV-1a über 32 Bit wie im übrigen Projekt ([`crate::checksum`]). Ein eigener
/// Typ statt eines blanken `u32`, weil in diesem Modul auch Versionen, Mengen
/// und CPU-Nummern `u32` sind — und ein Fingerabdruck, der sich mit einer
/// Versionsnummer vergleichen lässt, wird irgendwann mit einer verglichen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash(pub u32);

impl Hash {
    /// Der Wert als Zahl.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Ein Zeitpunkt in koordinierter Weltzeit — im Plan der Typ `DateTimeUtc`.
///
/// Sekunden seit der Unix-Epoche, ohne Schaltsekunden, so wie
/// `SystemTime::UNIX_EPOCH` zählt. **Kein `SystemTime`**, weil der Wert in den
/// Vertrag geschrieben und wieder gelesen wird: Ein `SystemTime` hat keine
/// feste Darstellung, und ein Vertrag darf nicht an der Uhr des Rechners
/// hängen, der ihn liest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTimeUtc {
    /// Sekunden seit 1970-01-01T00:00:00Z.
    pub unix_seconds: i64,
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
    /// Die Stärke t: Für welche Größe von Kombinationen wird Abdeckung
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
    /// „Um 08:00 muss es fertig sein" — ein Termin in Weltzeit.
    ///
    /// Richtig, wenn ein Termin dahintersteht. **Nicht** reproduzierbar: Zwei
    /// Läufe derselben Kampagne enden bei verschiedener Startzeit an
    /// verschiedenen Stellen.
    WallClockUtc {
        /// Der Termin.
        deadline: DateTimeUtc,
    },
    /// „Zwei Stunden RECHENZEIT" — zählt nur, was wirklich gerechnet wurde.
    ///
    /// Für ein reproduzierbares Teilergebnis die sauberere Wahl.
    ActiveRuntime {
        /// Verbleibende Rechenzeit.
        remaining: Duration,
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
/// Dies ist die rechnerische Vorgabe für homogene SMT-Topologien, wie sie das
/// Referenzprofil für jeden Rechner führt — auch für einen, dessen Topologie
/// der Koordinator gar nicht lesen kann. **Welche** CPUs es sind, liefert
/// [`CpuTopology::assign`] aus der Topologie der laufenden Maschine.
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

/// Ein logischer Prozessor, wie ihn `/sys/devices/system/cpu/cpuN/topology`
/// meldet (AC4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalCpu {
    /// Die Nummer, unter der der Kernel ihn führt — die Zahl für `taskset -c`.
    pub cpu: u32,
    /// `physical_package_id`: der Sockel.
    pub package: u32,
    /// `core_id`: der physische Kern, **nur innerhalb eines Pakets eindeutig**.
    pub core: u32,
}

/// Ein physischer Kern samt seinen SMT-Geschwistern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalCore {
    /// Der Sockel.
    pub package: u32,
    /// `core_id` innerhalb des Sockels.
    pub core: u32,
    /// Die CPU-Nummern dieses Kerns, aufsteigend.
    pub cpus: Vec<u32>,
}

/// Warum aus einer Topologie keine eindeutige Belegung folgt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// Eine sysfs-Datei fehlt oder ist nicht lesbar.
    Unreadable {
        /// Die Datei.
        path: String,
    },
    /// Eine sysfs-Datei enthält keine Zahl oder keine Kernliste.
    Malformed {
        /// Die Datei.
        path: String,
    },
    /// Keine einzige CPU.
    Empty,
    /// Dieselbe CPU-Nummer steht zweimal in der Eingabe.
    DuplicateCpu {
        /// Die doppelte Nummer.
        cpu: u32,
    },
    /// `thread_siblings_list` widerspricht der Zuordnung über Paket und Kern.
    ///
    /// Dann ist eine der beiden Kernel-Angaben falsch, und eine Reservierung
    /// „samt SMT-Geschwistern" ist nicht mehr eindeutig.
    SiblingMismatch {
        /// Die CPU, deren Geschwisterliste nicht passt.
        cpu: u32,
    },
    /// Nicht alle Kerne haben gleich viele Stränge (etwa P- und E-Kerne).
    ///
    /// [`CpuReservation`] setzt das voraus; die CPU-Nummern aus
    /// [`CpuTopology::assign`] bleiben trotzdem eindeutig.
    Heterogeneous,
    /// Mehr Kerne oder Stränge, als [`CpuReservation`] zählen kann.
    TooManyCpus,
    /// Die Reservierung ließe keinen Kern zum Rechnen übrig.
    ReservationTooLarge {
        /// Verlangte Reservierung.
        reserved: u16,
        /// Vorhandene physische Kerne.
        physical_cores: usize,
    },
}

impl std::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable { path } => write!(f, "{path}: nicht lesbar"),
            Self::Malformed { path } => write!(f, "{path}: keine Zahl oder Kernliste"),
            Self::Empty => f.write_str("keine CPU in der Topologie"),
            Self::DuplicateCpu { cpu } => write!(f, "CPU {cpu} doppelt angegeben"),
            Self::SiblingMismatch { cpu } => write!(
                f,
                "CPU {cpu}: thread_siblings_list widerspricht Paket und core_id"
            ),
            Self::Heterogeneous => {
                f.write_str("Kerne mit verschieden vielen Strängen: keine einheitliche Anzahl")
            }
            Self::TooManyCpus => f.write_str("mehr Kerne oder Stränge als u16 fasst"),
            Self::ReservationTooLarge {
                reserved,
                physical_cores,
            } => write!(
                f,
                "{reserved} von {physical_cores} physischen Kernen reserviert: keiner bleibt zum Rechnen"
            ),
        }
    }
}

impl std::error::Error for TopologyError {}

/// Die Kerntopologie einer Maschine — woraus die CPU-Nummern folgen (AC4).
///
/// # Warum aus der Topologie und nicht aus der Anzahl
///
/// „Zwölf von sechzehn Strängen" heißt auf dem Laptop CPUs 0–11, weil sein
/// Kernel die Geschwister nebeneinander nummeriert (`0-1`, `2-3`, …). Ein
/// Kernel, der erst alle ersten Stränge und dann alle zweiten zählt (`0,8`,
/// `1,9`, …), hat dieselben zwölf Stränge unter den Nummern 0–5 und 8–13. Wer
/// die Nummern aus der Anzahl ableitet, reserviert dort zwei Stränge statt
/// zwei Kerne — genau die Lesart, die AC4 ausschließt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTopology {
    /// Aufsteigend nach (Paket, Kern).
    cores: Vec<PhysicalCore>,
}

impl CpuTopology {
    /// Die Topologie aus einzelnen logischen Prozessoren.
    ///
    /// # Errors
    /// Eine leere Eingabe oder eine doppelte CPU-Nummer.
    pub fn from_cpus(cpus: &[LogicalCpu]) -> Result<Self, TopologyError> {
        if cpus.is_empty() {
            return Err(TopologyError::Empty);
        }
        let mut gesehen = BTreeSet::new();
        let mut kerne: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
        for c in cpus {
            if !gesehen.insert(c.cpu) {
                return Err(TopologyError::DuplicateCpu { cpu: c.cpu });
            }
            kerne.entry((c.package, c.core)).or_default().push(c.cpu);
        }
        let cores = kerne
            .into_iter()
            .map(|((package, core), mut cpus)| {
                cpus.sort_unstable();
                PhysicalCore {
                    package,
                    core,
                    cpus,
                }
            })
            .collect();
        Ok(Self { cores })
    }

    /// Die Topologie der laufenden Maschine, aus `/sys/devices/system/cpu`.
    ///
    /// # Errors
    /// Siehe [`CpuTopology::from_sysfs_at`].
    pub fn from_sysfs() -> Result<Self, TopologyError> {
        Self::from_sysfs_at(Path::new("/sys/devices/system/cpu"))
    }

    /// Die Topologie aus einem sysfs-Verzeichnis.
    ///
    /// Gelesen werden nur die CPUs aus `online` — eine abgeschaltete CPU hat
    /// ein Verzeichnis, aber keinen Platz in einer Belegung. Je CPU zählen
    /// `physical_package_id`, `core_id` und `thread_siblings_list`; die dritte
    /// Angabe ist die Gegenprobe zu den ersten beiden.
    ///
    /// Das Verzeichnis ist ein Parameter, damit der Leser an einem Nachbau
    /// prüfbar ist: Auf der echten Maschine steht immer dieselbe Topologie, und
    /// ein Fehler im Zusammenfügen fiele dort nie auf.
    ///
    /// # Errors
    /// Fehlende oder unlesbare Dateien, Inhalte ohne Zahl, widersprüchliche
    /// Geschwisterlisten.
    pub fn from_sysfs_at(root: &Path) -> Result<Self, TopologyError> {
        let online = sysfs_list(&root.join("online"))?;
        let mut cpus = Vec::with_capacity(online.len());
        let mut geschwister: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &cpu in &online {
            let dir = root.join(format!("cpu{cpu}/topology"));
            let package = sysfs_number(&dir.join("physical_package_id"))?;
            let core = sysfs_number(&dir.join("core_id"))?;
            let mut liste = sysfs_list(&dir.join("thread_siblings_list"))?;
            liste.retain(|g| online.contains(g));
            liste.sort_unstable();
            geschwister.insert(cpu, liste);
            cpus.push(LogicalCpu { cpu, package, core });
        }
        let topologie = Self::from_cpus(&cpus)?;
        for kern in &topologie.cores {
            for cpu in &kern.cpus {
                if geschwister.get(cpu) != Some(&kern.cpus) {
                    return Err(TopologyError::SiblingMismatch { cpu: *cpu });
                }
            }
        }
        Ok(topologie)
    }

    /// Die physischen Kerne, aufsteigend nach (Paket, Kern).
    #[must_use]
    pub fn cores(&self) -> &[PhysicalCore] {
        &self.cores
    }

    /// Dieselbe Reservierung als Anzahl — der Eintrag fürs Referenzprofil.
    ///
    /// # Errors
    /// Kerne mit verschieden vielen Strängen, eine Reservierung ohne
    /// verbleibenden Kern, oder Zahlen jenseits von `u16`.
    pub fn reservation(
        &self,
        reserved_physical_cores: u16,
    ) -> Result<CpuReservation, TopologyError> {
        self.pruefe_reservierung(reserved_physical_cores)?;
        let straenge = self.cores[0].cpus.len();
        if self.cores.iter().any(|k| k.cpus.len() != straenge) {
            return Err(TopologyError::Heterogeneous);
        }
        Ok(CpuReservation {
            physical_cores: u16::try_from(self.cores.len())
                .map_err(|_| TopologyError::TooManyCpus)?,
            threads_per_core: u16::try_from(straenge).map_err(|_| TopologyError::TooManyCpus)?,
            reserved_physical_cores,
        })
    }

    /// Welche CPUs rechnen und welche frei bleiben.
    ///
    /// Reserviert werden die **letzten** `reserved_physical_cores` Kerne in der
    /// Reihenfolge (Paket, Kern), jeder **samt allen seinen Strängen** — auf
    /// dem Laptop die Kerne 6–7, wie die Anzeige in AC4 sie nennt. Die
    /// rechnenden Kerne behalten ihre SMT-Geschwister
    /// (`use_smt_siblings_of_compute_cores: true`, Plan E15.1, „Regelung: von
    /// außen und von selbst").
    ///
    /// # Errors
    /// Eine Reservierung, die keinen Kern zum Rechnen übrig ließe.
    pub fn assign(&self, reserved_physical_cores: u16) -> Result<CpuAssignment, TopologyError> {
        self.pruefe_reservierung(reserved_physical_cores)?;
        let grenze = self.cores.len() - usize::from(reserved_physical_cores);
        let (rechnend, reserviert) = self.cores.split_at(grenze);
        let mut compute_cpus: Vec<u32> = rechnend
            .iter()
            .flat_map(|k| k.cpus.iter().copied())
            .collect();
        compute_cpus.sort_unstable();
        Ok(CpuAssignment {
            compute_cpus,
            reserved_cores: reserviert.to_vec(),
            multi_package: self
                .cores
                .iter()
                .any(|k| k.package != self.cores[0].package),
        })
    }

    fn pruefe_reservierung(&self, reserved: u16) -> Result<(), TopologyError> {
        if usize::from(reserved) >= self.cores.len() {
            return Err(TopologyError::ReservationTooLarge {
                reserved,
                physical_cores: self.cores.len(),
            });
        }
        Ok(())
    }
}

/// Die tatsächliche Aufteilung einer Maschine in rechnende und freie CPUs
/// (AC4).
///
/// Die Felder sind privat, damit es keine Belegung gibt, die nicht aus einer
/// Topologie stammt — eine von Hand gebaute Liste wäre genau die Behauptung
/// ohne Quelle, gegen die AC4 steht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuAssignment {
    compute_cpus: Vec<u32>,
    reserved_cores: Vec<PhysicalCore>,
    multi_package: bool,
}

impl CpuAssignment {
    /// Die CPUs, auf denen gerechnet wird, aufsteigend.
    #[must_use]
    pub fn compute_cpus(&self) -> &[u32] {
        &self.compute_cpus
    }

    /// Die reservierten physischen Kerne.
    #[must_use]
    pub fn reserved_cores(&self) -> &[PhysicalCore] {
        &self.reserved_cores
    }

    /// Die CPUs der reservierten Kerne, aufsteigend.
    #[must_use]
    pub fn reserved_cpus(&self) -> Vec<u32> {
        let mut cpus: Vec<u32> = self
            .reserved_cores
            .iter()
            .flat_map(|k| k.cpus.iter().copied())
            .collect();
        cpus.sort_unstable();
        cpus
    }

    /// Die Anzeige aus AC4 — mit den Nummern, nicht mit einer Bruchzahl.
    ///
    /// ```text
    /// Compute:     CPUs 0–11
    /// Reserviert:  physische Kerne 6–7 samt SMT-Geschwistern (CPUs 12–15)
    /// ```
    ///
    /// Auf einer Maschine mit mehreren Sockeln steht jeder Kern als
    /// `Paket/Kern`, weil `core_id` sich je Sockel wiederholt.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut text = format!(
            "Compute:     CPUs {}\n",
            bereiche(&self.compute_cpus, "–", ", ")
        );
        if self.reserved_cores.is_empty() {
            text.push_str("Reserviert:  keine");
            return text;
        }
        let kerne = if self.multi_package {
            self.reserved_cores
                .iter()
                .map(|k| format!("{}/{}", k.package, k.core))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            let ids: Vec<u32> = self.reserved_cores.iter().map(|k| k.core).collect();
            bereiche(&ids, "–", ", ")
        };
        write!(
            text,
            "Reserviert:  physische Kerne {kerne} samt SMT-Geschwistern (CPUs {})",
            bereiche(&self.reserved_cpus(), "–", ", ")
        )
        .expect("String ist beschreibbar");
        text
    }

    /// Die Belegung als JSON — die Nummern gehören in den Bericht (AC4).
    ///
    /// Neben den Listen steht jeweils die kernelübliche Kurzform, wie sie
    /// `taskset -c` erwartet und `/proc/self/status` als `Cpus_allowed_list`
    /// zurückmeldet.
    #[must_use]
    pub fn to_json(&self) -> String {
        let kerne = self
            .reserved_cores
            .iter()
            .map(|k| {
                format!(
                    "{{\"package\":{},\"core\":{},\"cpus\":{}}}",
                    k.package,
                    k.core,
                    json_u32_array(&k.cpus)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"computeCpus\":{},\"computeCpuList\":{},\
             \"reservedCores\":[{kerne}],\"reservedCpuList\":{}}}",
            json_u32_array(&self.compute_cpus),
            json_string(&cpu_list(&self.compute_cpus)),
            json_string(&cpu_list(&self.reserved_cpus())),
        )
    }
}

/// CPU-Nummern in der kernelüblichen Kurzform, etwa `0-5,8-13`.
///
/// Die Form, die `taskset -c` annimmt und `Cpus_allowed_list` meldet — und die
/// [`kernliste`] zurückliest. Die Eingabe wird sortiert und von Doppelten
/// befreit; eine leere Eingabe ergibt eine leere Zeichenkette.
#[must_use]
pub fn cpu_list(cpus: &[u32]) -> String {
    let mut sortiert = cpus.to_vec();
    sortiert.sort_unstable();
    sortiert.dedup();
    bereiche(&sortiert, "-", ",")
}

/// Fasst aufsteigende Nummern zu Bereichen zusammen.
fn bereiche(nummern: &[u32], bis: &str, trenner: &str) -> String {
    let mut teile: Vec<String> = Vec::new();
    let mut i = 0;
    while i < nummern.len() {
        let anfang = nummern[i];
        let mut ende = anfang;
        while i + 1 < nummern.len() && nummern[i + 1] == ende + 1 {
            i += 1;
            ende = nummern[i];
        }
        teile.push(if anfang == ende {
            anfang.to_string()
        } else {
            format!("{anfang}{bis}{ende}")
        });
        i += 1;
    }
    teile.join(trenner)
}

fn sysfs_text(path: &Path) -> Result<String, TopologyError> {
    std::fs::read_to_string(path).map_err(|_| TopologyError::Unreadable {
        path: path.display().to_string(),
    })
}

fn sysfs_number(path: &Path) -> Result<u32, TopologyError> {
    sysfs_text(path)?
        .trim()
        .parse()
        .map_err(|_| TopologyError::Malformed {
            path: path.display().to_string(),
        })
}

fn sysfs_list(path: &Path) -> Result<Vec<u32>, TopologyError> {
    let falsch = || TopologyError::Malformed {
        path: path.display().to_string(),
    };
    kernliste(&sysfs_text(path)?)
        .ok_or_else(falsch)?
        .into_iter()
        .map(|k| u32::try_from(k).map_err(|_| falsch()))
        .collect()
}

/// Was ein Rechner mindestens mitbringen muss (AC3).
///
/// **„Rechner erreichbar" genügt nicht.** In einer geteilten Umgebung sagt
/// Erreichbarkeit nichts über die Zeit: Eine Maschine, die antwortet und dabei
/// 5 % ihrer Rechenzeit bekommt, ist für einen Balance-Lauf so gut wie
/// abwesend — nur merkt es niemand, weil sie ja „da" ist.
///
/// Die Netzvorgabe steht seit #914 am [`ReferenceProfile`] und nicht mehr hier:
/// AC3 führt „Netz" als eigene Zeile, und der Rechner, auf dem der Koordinator
/// läuft, hat keine Verbindung zu sich selbst, für die eine Umlaufzeit gälte.
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
    /// Obergrenze für den Speicher des Laufs auf diesem Rechner, in Mebibyte.
    ///
    /// **Eine Obergrenze, keine Reservierung:** Der Lauf belegt nur, was er
    /// gerade braucht, und niemand hält ihm den Rest frei (Vorgabe des
    /// Auftraggebers vom 2026-10-28 für node3; auf dem Laptop ist die Quelle
    /// die Vorgabe „unter 5 GB"). Bis #914 hieß das Feld
    /// `guaranteed_memory_mib` und behauptete eine Zusicherung, die es auf
    /// keinem der beiden Rechner gibt.
    pub memory_limit_mib: u32,
    /// Die CPU-Aufteilung dieses Rechners.
    pub cpu: CpuReservation,
    /// Grafik-Einschaltdauer: welcher Anteil der Zeit die Grafik rechnet,
    /// 0.0 bis 1.0 (AC3).
    ///
    /// Steht hier, weil sich auf dem Laptop CPU, iGPU und dGPU ein thermisches
    /// Budget teilen: Grafikarbeit **senkt den CPU-Takt** (Plan E15.0b). Ein
    /// Budget, das mit laufender Grafik abgeleitet wurde, gilt nicht für einen
    /// Lauf ohne — und umgekehrt.
    pub gpu_duty_cycle: f64,
    /// Wird die Maschine im Betrieb aktiv gedrosselt?
    ///
    /// Steht hier, weil eine gedrosselte Maschine dieselbe Arbeit in anderer
    /// Zeit leistet — und das Referenzprofil Zeit **ableitet**, nicht misst.
    pub thermal_throttling: bool,
}

/// Was das Netz zwischen Koordinator und Zusatzrechnern mindestens leisten
/// muss (AC3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkRequirement {
    /// Obergrenze für die Umlaufzeit, in Millisekunden.
    pub max_rtt_ms: u32,
    /// Untergrenze für die Bandbreite, in Mebibit je Sekunde.
    ///
    /// `None` heißt **nicht gemessen** — nicht „keine Untergrenze" und nicht
    /// „best effort". Eine erfundene kleine Zahl sähe wie eine Vorgabe aus, und
    /// niemand prüfte sie nach. Wird sie erhoben, ist das eine neue Fassung des
    /// Profils mit neuer Version.
    pub min_bandwidth_mibit_s: Option<u32>,
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
    /// Name des Profils, etwa `standard`.
    pub name: &'static str,
    /// Version dieses Profils.
    pub version: u32,
    /// Die Rechner, aus denen abgeleitet wurde. Der erste ist der, auf dem der
    /// Koordinator läuft.
    pub hosts: &'static [HostRequirement],
    /// Was das Netz zu den übrigen Rechnern leisten muss.
    ///
    /// `None` heißt: kein Netz im Profil, weil es nur einen Rechner führt.
    /// Das ist eine andere Aussage als eine Vorgabe aus Nullen.
    pub network: Option<NetworkRequirement>,
}

/// Die Rechner des Referenzprofils „standard": Laptop + node3.
///
/// Rechner: Entscheidung des Auftraggebers vom 2026-10-28. Werte: entschieden
/// vom Orchestrator am selben Tag. Herkunft je Wert am Eintrag.
static STANDARD_HOSTS: &[HostRequirement] = &[
    // Der Laptop: AMD Ryzen 9 5900HS, 8 physische Kerne × 2 Stränge (sysfs am
    // 2026-10-28: thread_siblings_list 0-1 … 14-15). Profil `balanced`: zwei
    // physische Kerne samt Geschwistern bleiben frei (Plan, Referenzprofil
    // „standard" unter „Was dieser Plan zusichert"), also zwölf
    // Stränge — die CPU-Quote ist genau diese zwölf. Speicher höchstens 4096 MiB
    // nach der Vorgabe „Speichergrenzen unter 5 GB" für diese Maschine — eine
    // Obergrenze, keine Reservierung. Grafik 0 %, weil es keinen GPU-Pfad gibt;
    // das Budget stammt allein aus der CPU. Der Koordinator läuft hier.
    HostRequirement {
        name: "laptop",
        guaranteed_cpu_quota: Some(12.0),
        memory_limit_mib: 4096,
        cpu: CpuReservation {
            physical_cores: 8,
            threads_per_core: 2,
            reserved_physical_cores: 2,
        },
        gpu_duty_cycle: 0.0,
        thermal_throttling: false,
    },
    // node3: Proxmox-Knoten `ovh-pve1`, AMD Ryzen 7 9700X, 8 Kerne/16 Stränge,
    // nichts reserviert (die Laptop-Grenze gilt auf node3 nicht, Plan E15.1,
    // „Regelung: von außen und von selbst"). CPU ausdrücklich best effort: keine garantierte Quote, wie der
    // Plan es für Zusatzrechner vorsieht („cores großzügig, cpuunits niedrig").
    // Speicher höchstens 4096 MiB als OBERGRENZE, nicht reserviert: Der
    // Container belegt nur, was er gerade braucht (Vorgabe des Auftraggebers
    // vom 2026-10-28) — auf ovh-pve1 waren an dem Tag nur rund 12 GB frei,
    // 13 VMs laufen. Der Simulations-Container entsteht erst mit #912.
    HostRequirement {
        name: "node3",
        guaranteed_cpu_quota: None,
        memory_limit_mib: 4096,
        cpu: CpuReservation {
            physical_cores: 8,
            threads_per_core: 2,
            reserved_physical_cores: 0,
        },
        gpu_duty_cycle: 0.0,
        thermal_throttling: false,
    },
];

/// Das Referenzprofil „standard" (AC3): Laptop + node3.
///
/// Der Plan nannte bis zum 2026-10-28 „Laptop + proxmox + gonet"
/// (`docs/SIM-ARCHITEKTUR-PLAN.md`, Block „Referenzprofil standard"); die
/// Zusatzrechner hat der Auftraggeber an diesem Tag auf node3 festgelegt.
///
/// Das Netz ist die eine Verbindung Laptop → node3. Umlaufzeit höchstens
/// 250 ms: gemessen am 2026-10-28 mit `ping -c 10` zur Bau-VM auf node3
/// (10.0.0.237), min 48 / Mittel 92 / max 183 ms. Die Bandbreite ist **nicht
/// gemessen** und steht deshalb als unbekannt; erhoben wird sie mit #912.
pub const REFERENCE_PROFILE_STANDARD: ReferenceProfile = ReferenceProfile {
    name: "standard",
    version: 1,
    hosts: STANDARD_HOSTS,
    network: Some(NetworkRequirement {
        max_rtt_ms: 250,
        min_bandwidth_mibit_s: None,
    }),
};

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
///
/// Deshalb steht die Zielzeit **hier** und nicht nur im Vertrag: Das Budget ist
/// für eine Zielzeit unter dem Referenzprofil bemessen, und
/// [`CampaignRunContract::from_campaign`] übernimmt beide. Eine andere
/// Zielzeit geht nur über [`ProfileCatalog::derive_for_target_time`] — mit
/// einer neuen Versionsnummer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CampaignDefinition {
    /// Version dieser Kampagne. Ändert sich die Arbeitsmenge, ändert sich diese
    /// Zahl.
    pub profile_version: u32,
    /// Das Referenzprofil, aus dem das Budget abgeleitet wurde.
    pub reference_profile: ReferenceProfile,
    /// Die Zielzeit, für die das Budget unter dem Referenzprofil bemessen ist.
    pub target_wall_time: Duration,
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
    /// Format 3 (#914) bindet zusätzlich die Zielzeit ein, Name und Netz des
    /// Referenzprofils und die Grafik-Einschaltdauer jedes Rechners; die
    /// Netzvorgabe je Rechner aus Format 2 ist ans Profil gewandert. Die
    /// aktuelle Hardware im Deployment bleibt ausgeschlossen.
    #[must_use]
    pub fn fingerprint(&self) -> Hash {
        let profil = &self.reference_profile;
        let mut h = mix_int(FNV_OFFSET, 3);
        h = mix_int(h, self.profile_version as i32);
        h = mix_text(h, profil.name);
        h = mix_int(h, profil.version as i32);
        h = match profil.network {
            Some(netz) => {
                let h = mix_int(mix_int(h, 1), netz.max_rtt_ms as i32);
                // „nicht gemessen" und „0 verlangt" getrennt, wie bei der Quote.
                match netz.min_bandwidth_mibit_s {
                    Some(b) => mix_int(mix_int(h, 1), b as i32),
                    None => mix_int(h, 0),
                }
            }
            None => mix_int(h, 0),
        };
        h = mix_u64(h, profil.hosts.len() as u64);
        for host in profil.hosts {
            h = mix_text(h, host.name);
            h = match host.guaranteed_cpu_quota {
                Some(quota) => mix_f64(mix_int(h, 1), quota),
                None => mix_int(h, 0),
            };
            h = mix_int(h, host.memory_limit_mib as i32);
            h = mix_int(h, i32::from(host.cpu.physical_cores));
            h = mix_int(h, i32::from(host.cpu.threads_per_core));
            h = mix_int(h, i32::from(host.cpu.reserved_physical_cores));
            h = mix_f64(h, host.gpu_duty_cycle);
            h = mix_int(h, i32::from(host.thermal_throttling));
        }
        h = mix_duration(h, self.target_wall_time);
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
        Hash(mix_int(h, audit_code(self.quality_targets.final_audit)))
    }

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
    /// Fließkommazahlen stehen in der kürzesten verlustfrei rücklesbaren Form
    /// (`{:?}`). Zu runden wäre hier falsch: Die letzte Stelle ist die, an der
    /// sich eine veränderte Vorgabe verrät. Aus demselben Grund steht eine
    /// Dauer als Sekunden und Nanosekunden, nicht als Millisekunden.
    ///
    /// # Errors
    /// Nichtendliche Zahlen werden mit Feldname abgelehnt. Sie dürfen weder
    /// ungültiges JSON erzeugen noch als fehlende Vorgabe (`null`) erscheinen.
    pub fn to_json(&self) -> Result<String, JsonError> {
        let profil = &self.reference_profile;
        let hosts = profil
            .hosts
            .iter()
            .map(HostRequirement::to_json)
            .collect::<Result<Vec<_>, _>>()?
            .join(",");
        let netz = match profil.network {
            // Eine nicht gemessene Bandbreite ist `null`, nicht 0: Die 0 wäre
            // eine Vorgabe („keine Untergrenze verlangt"), `null` ist keine.
            Some(n) => format!(
                "{{\"maxRttMs\":{},\"minBandwidthMibitS\":{}}}",
                n.max_rtt_ms,
                n.min_bandwidth_mibit_s
                    .map_or_else(|| "null".to_owned(), |b| b.to_string())
            ),
            None => "null".to_owned(),
        };
        let q = &self.quality_targets;
        Ok(format!(
            "{{\"profileVersion\":{},\"fingerprint\":{},\
             \"referenceProfile\":{{\"name\":{},\"version\":{},\"hosts\":[{}],\"network\":{}}},\
             \"targetWallTime\":{},\
             \"canonicalWorkBudget\":{},\"exactObligations\":{},\
             \"coverageStrength\":{},\"minimumNicheCoverage\":{},\
             \"independentRestarts\":{},\"plateauWorkUnits\":{},\
             \"maximumBoundGap\":{},\"confidence\":{},\"finalAudit\":{:?}}}",
            self.profile_version,
            self.fingerprint().get(),
            json_string(profil.name),
            profil.version,
            hosts,
            netz,
            json_duration(self.target_wall_time),
            self.canonical_work_budget.get(),
            self.exact_obligations.required,
            self.coverage_spec.strength,
            json_number(q.minimum_niche_coverage, "minimumNicheCoverage")?,
            q.independent_restarts,
            q.plateau_work_units.get(),
            // `null` und eine Zahl sind verschiedene Aussagen — "keine Schranke
            // verlangt" gegen "Schranke muss exakt sein". Eine 0.0 an dieser
            // Stelle wäre die zweite, gemeint ist beim fehlenden Wert die erste.
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
    /// Eine vorhandene CPU-Quote und die Grafik-Einschaltdauer müssen endlich
    /// sein.
    pub fn to_json(&self) -> Result<String, JsonError> {
        Ok(format!(
            "{{\"name\":{},\"guaranteedCpuQuota\":{},\
             \"memoryLimitMib\":{},\"computeThreads\":{},\
             \"totalThreads\":{},\"physicalCores\":{},\"threadsPerCore\":{},\
             \"reservedPhysicalCores\":{},\"gpuDutyCycle\":{},\
             \"thermalThrottling\":{}}}",
            json_string(self.name),
            // `null` heißt hier ausdrücklich "best effort", nicht "unbekannt".
            // Eine 0.0 wäre die dritte, ganz andere Aussage: garantiert nichts.
            json_optional_number(self.guaranteed_cpu_quota, "guaranteedCpuQuota")?,
            self.memory_limit_mib,
            self.cpu.compute_threads(),
            self.cpu.total_threads(),
            self.cpu.physical_cores,
            self.cpu.threads_per_core,
            self.cpu.reserved_physical_cores,
            json_number(self.gpu_duty_cycle, "gpuDutyCycle")?,
            self.thermal_throttling,
        ))
    }
}

/// Warum eine Kampagne nicht als versioniertes Profil gilt (AC7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    /// Unter dieser Version liegt schon ein Profil mit anderem Inhalt.
    ///
    /// Das ist das „nachträgliche Umrechnen", das AC7 ausschließt: dieselbe
    /// Versionsnummer, eine andere Arbeitsmenge oder Zielzeit.
    VersionFrozen {
        /// Die vergebene Version.
        profile_version: u32,
        /// Der Fingerabdruck, unter dem sie eingefroren ist.
        frozen: Hash,
        /// Der Fingerabdruck der vorgelegten Fassung.
        presented: Hash,
    },
    /// Diese Version ist nicht eingefroren.
    UnknownVersion {
        /// Die verlangte Version.
        profile_version: u32,
    },
    /// Kein eingefrorenes Profil hat diesen Fingerabdruck.
    UnknownOrigin {
        /// Der Fingerabdruck aus dem Vertrag.
        campaign_definition_hash: Hash,
    },
    /// Der Vertrag nennt ein eingefrorenes Profil, weicht aber in einem Feld
    /// ab, das er aus ihm übernimmt.
    ContractDiverges {
        /// Das abweichende Vertragsfeld.
        field: &'static str,
    },
    /// Die höchste Versionsnummer ist vergeben.
    VersionsExhausted,
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionFrozen {
                profile_version,
                frozen,
                presented,
            } => write!(
                f,
                "Profil {profile_version} ist eingefroren ({}) — die vorgelegte Fassung ({}) \
                 braucht eine neue Version",
                frozen.get(),
                presented.get()
            ),
            Self::UnknownVersion { profile_version } => {
                write!(f, "Profil {profile_version} ist nicht eingefroren")
            }
            Self::UnknownOrigin {
                campaign_definition_hash,
            } => write!(
                f,
                "kein eingefrorenes Profil mit Fingerabdruck {}",
                campaign_definition_hash.get()
            ),
            Self::ContractDiverges { field } => {
                write!(f, "Vertragsfeld {field} weicht von seinem Profil ab")
            }
            Self::VersionsExhausted => f.write_str("keine freie Profilversion mehr"),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Die versionierten Profile — und ihr Schutz (AC7).
///
/// # Warum ein Katalog und nicht der Typ
///
/// Die Felder von [`CampaignDefinition`] sind öffentlich, und das bleibt so:
/// Eine Kampagne ist ein Datensatz, den Werkzeuge bauen und lesen. Jeder kann
/// deshalb eine Definition mit Version 1 und einem anderen Budget erzeugen. Was
/// „Version 1" bedeutet, hält der Katalog fest — als Fingerabdruck — und eine
/// abweichende Fassung unter derselben Nummer weist er zurück.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileCatalog {
    frozen: Vec<CampaignDefinition>,
}

impl ProfileCatalog {
    /// Ein leerer Katalog.
    #[must_use]
    pub const fn new() -> Self {
        Self { frozen: Vec::new() }
    }

    /// Eine Fassung unter ihrer Version einfrieren.
    ///
    /// Dieselbe Fassung ein zweites Mal ist kein Fehler: Einfrieren ist eine
    /// Feststellung, kein Ereignis.
    ///
    /// # Errors
    /// [`ProfileError::VersionFrozen`], wenn unter der Version schon ein anderer
    /// Inhalt liegt. Der Katalog bleibt dann unverändert.
    pub fn freeze(&mut self, definition: CampaignDefinition) -> Result<Hash, ProfileError> {
        let presented = definition.fingerprint();
        if let Some(vorhanden) = self.get(definition.profile_version) {
            let frozen = vorhanden.fingerprint();
            return if frozen == presented {
                Ok(presented)
            } else {
                Err(ProfileError::VersionFrozen {
                    profile_version: definition.profile_version,
                    frozen,
                    presented,
                })
            };
        }
        self.frozen.push(definition);
        Ok(presented)
    }

    /// Die eingefrorene Fassung einer Version.
    #[must_use]
    pub fn get(&self, profile_version: u32) -> Option<&CampaignDefinition> {
        self.frozen
            .iter()
            .find(|d| d.profile_version == profile_version)
    }

    /// Ist diese Fassung die eingefrorene ihrer Version?
    ///
    /// # Errors
    /// Unbekannte Version oder abweichender Inhalt.
    pub fn verify(&self, definition: &CampaignDefinition) -> Result<(), ProfileError> {
        let vorhanden =
            self.get(definition.profile_version)
                .ok_or(ProfileError::UnknownVersion {
                    profile_version: definition.profile_version,
                })?;
        let frozen = vorhanden.fingerprint();
        let presented = definition.fingerprint();
        if frozen == presented {
            Ok(())
        } else {
            Err(ProfileError::VersionFrozen {
                profile_version: definition.profile_version,
                frozen,
                presented,
            })
        }
    }

    /// Ein NEUES Profil für eine neue Zielzeit (AC7).
    ///
    /// Die Basis bleibt unverändert im Katalog; das neue Profil übernimmt alles
    /// außer Zielzeit und Budget und bekommt die nächste freie Version.
    ///
    /// **Das Budget ist eine Eingabe.** Es folgt aus dem Referenzprofil, und
    /// diese Ableitung ist nicht Gegenstand dieses Moduls — vor allem folgt es
    /// nie aus der Kalibrierung des Rechners, auf dem gerade jemand ableitet.
    ///
    /// # Errors
    /// Unbekannte Basis oder keine freie Version.
    pub fn derive_for_target_time(
        &mut self,
        base_version: u32,
        target_wall_time: Duration,
        canonical_work_budget: WorkUnits,
    ) -> Result<CampaignDefinition, ProfileError> {
        let basis = *self.get(base_version).ok_or(ProfileError::UnknownVersion {
            profile_version: base_version,
        })?;
        let naechste = self
            .frozen
            .iter()
            .map(|d| d.profile_version)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ProfileError::VersionsExhausted)?;
        let neu = CampaignDefinition {
            profile_version: naechste,
            target_wall_time,
            canonical_work_budget,
            ..basis
        };
        self.freeze(neu)?;
        Ok(neu)
    }

    /// Aus welchem eingefrorenen Profil stammt dieser Vertrag?
    ///
    /// Das ist der Leser von `campaign_definition_hash`: Er sucht das Profil
    /// zum Fingerabdruck und prüft **jedes** Feld, das der Vertrag aus ihm
    /// übernommen hat. Ein Vertrag, dessen Budget nachträglich geändert wurde,
    /// trägt sonst einen gültigen Fingerabdruck neben einer fremden
    /// Arbeitsmenge. Mission, Frist und Zählweise sind Laufsache und gehören
    /// nicht zur Herkunft.
    ///
    /// # Errors
    /// Unbekannter Fingerabdruck oder ein abweichendes übernommenes Feld.
    pub fn origin(
        &self,
        contract: &CampaignRunContract,
    ) -> Result<&CampaignDefinition, ProfileError> {
        let d = self
            .frozen
            .iter()
            .find(|d| d.fingerprint() == contract.campaign_definition_hash)
            .ok_or(ProfileError::UnknownOrigin {
                campaign_definition_hash: contract.campaign_definition_hash,
            })?;
        let q = &d.quality_targets;
        let felder = [
            (
                "target_wall_time",
                contract.target_wall_time == d.target_wall_time,
            ),
            (
                "deterministic_work_budget",
                contract.deterministic_work_budget == d.canonical_work_budget,
            ),
            (
                "exact_obligations",
                contract.exact_obligations == d.exact_obligations,
            ),
            (
                "interaction_coverage",
                contract.interaction_coverage == d.coverage_spec,
            ),
            (
                "minimum_niche_coverage",
                gleiche_bits(contract.minimum_niche_coverage, q.minimum_niche_coverage),
            ),
            (
                "independent_restarts",
                contract.independent_restarts == q.independent_restarts,
            ),
            (
                "plateau_work_units",
                contract.plateau_work_units == q.plateau_work_units,
            ),
            (
                "maximum_bound_gap",
                match (contract.maximum_bound_gap, q.maximum_bound_gap) {
                    (None, None) => true,
                    (Some(a), Some(b)) => gleiche_bits(a, b),
                    _ => false,
                },
            ),
            (
                "confidence_requirement",
                gleiche_bits(
                    contract.confidence_requirement.level,
                    q.confidence_requirement.level,
                ),
            ),
            ("final_audit", contract.final_audit == q.final_audit),
        ];
        match felder.iter().find(|(_, gleich)| !gleich) {
            Some((field, _)) => Err(ProfileError::ContractDiverges { field }),
            None => Ok(d),
        }
    }
}

/// Gleitkommawerte über ihre Bits vergleichen — wie der Fingerabdruck.
fn gleiche_bits(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
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
            "{{\"mission\":{},\"targetWallTime\":{},\
             \"campaignDefinitionHash\":{},\"deterministicWorkBudget\":{},\
             \"hardDeadline\":{},\"runtimeAccounting\":{},\
             \"exactObligations\":{},\"coverageStrength\":{},\
             \"minimumNicheCoverage\":{},\"independentRestarts\":{},\
             \"plateauWorkUnits\":{},\"maximumBoundGap\":{},\
             \"confidence\":{},\"finalAudit\":{:?}}}",
            mission_json(self.mission),
            json_duration(self.target_wall_time),
            self.campaign_definition_hash.get(),
            self.deterministic_work_budget.get(),
            // Fehlt die Frist, steht `null` — nicht etwa eine sehr große Zahl.
            // Ein Vorgabewert an dieser Stelle wäre eine Frist, die niemand
            // verlangt hat (AC5).
            match self.hard_deadline {
                Some(DeadlinePolicy::WallClockUtc { deadline }) => format!(
                    "{{\"kind\":\"wallClockUtc\",\"deadline\":{{\"unixSeconds\":{}}}}}",
                    deadline.unix_seconds
                ),
                Some(DeadlinePolicy::ActiveRuntime { remaining }) => format!(
                    "{{\"kind\":\"activeRuntime\",\"remaining\":{}}}",
                    json_duration(remaining)
                ),
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

/// Eine Dauer verlustfrei: ganze Sekunden und der Nanosekundenrest.
fn json_duration(value: Duration) -> String {
    format!(
        "{{\"secs\":{},\"nanos\":{}}}",
        value.as_secs(),
        value.subsec_nanos()
    )
}

fn json_u32_array(values: &[u32]) -> String {
    let teile: Vec<String> = values.iter().map(u32::to_string).collect();
    format!("[{}]", teile.join(","))
}

/// Der Name der Auditregel im JSON — stabil, unabhängig von der Reihenfolge im
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

/// Eine Fließkommazahl über ihre Bits einrühren.
const fn mix_f64(hash: u32, value: f64) -> u32 {
    mix_u64(hash, value.to_bits())
}

/// Eine Dauer über Sekunden und Nanosekunden einrühren — beide, sonst wären
/// 2 h und 2 h + 1 ns derselbe Fingerabdruck.
const fn mix_duration(hash: u32, value: Duration) -> u32 {
    mix_int(mix_u64(hash, value.as_secs()), value.subsec_nanos() as i32)
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
///
/// Die Quoten und Leistungsprofile aus dem Plan (`cgroup_quotas`,
/// `runtime_perf_profiles`) stehen je Rechner in [`DeployedHost`] statt in zwei
/// Listen neben `available_hosts`: Drei parallele Listen können verschieden lang
/// sein, ein Eintrag je Rechner nicht.
#[derive(Debug, Clone, PartialEq)]
pub struct RunDeployment {
    /// Die Rechner, die diesem Lauf zur Verfügung stehen.
    pub available_hosts: Vec<DeployedHost>,
    /// Geschätzte Dauer — eine **Schätzung**, kein Vertrag.
    pub estimated_eta: Duration,
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

/// Der Laufvertrag (AC0) — Feldnamen und -typen wörtlich aus dem Plan.
///
/// Der Name unterscheidet ihn vom Zielvertrag [`crate::objective::RunContract`]
/// aus #939, der im Plan bis zum 2026-08-22 denselben Namen trug und etwas
/// völlig anderes beschreibt — siehe Modulkopf.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CampaignRunContract {
    /// Wozu dieser Lauf da ist.
    pub mission: Mission,
    /// Die angestrebte Wanduhrzeit — ein **Ziel**, keine Frist. Übernommen aus
    /// der Kampagne, für die das Budget bemessen ist.
    pub target_wall_time: Duration,
    /// Woher das Budget stammt: der Fingerabdruck der Kampagne.
    ///
    /// Ohne dieses Feld ließe sich hinterher nicht mehr feststellen, gegen
    /// welche Arbeitsmenge ein Ergebnis entstanden ist. Geprüft wird es von
    /// [`ProfileCatalog::origin`].
    pub campaign_definition_hash: Hash,
    /// Die eingefrorene Arbeitsmenge, übernommen aus der Kampagne.
    pub deterministic_work_budget: WorkUnits,
    /// Ein echter Zeitabbruch — **nur auf ausdrücklichen Wunsch** (AC5).
    ///
    /// `None` ist die Voreinstellung, und das ist eine inhaltliche Festlegung:
    /// Ein Lauf bricht nach erledigter Arbeit ab, nicht nach Ablauf einer Uhr.
    pub hard_deadline: Option<DeadlinePolicy>,
    /// Wie Rechenzeit gezählt wird, falls eine Frist gesetzt ist (AC6: die drei
    /// Festlegungen stehen „im Vertrag").
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
    /// Arbeitsmenge **und Zielzeit** werden übernommen, nicht gerechnet — und
    /// der Fingerabdruck wandert mit, damit später belegbar bleibt, woher sie
    /// kamen. Bis #914 war die Zielzeit ein Parameter; damit entstanden Verträge
    /// mit verschiedener Zielzeit aus derselben versionierten Kampagne. Das
    /// Deployment ist hier kein Parameter, aus demselben Grund wie bei
    /// [`CampaignDefinition::fingerprint`].
    #[must_use]
    pub fn from_campaign(campaign: &CampaignDefinition, mission: Mission) -> Self {
        Self {
            mission,
            target_wall_time: campaign.target_wall_time,
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

    /// Die Garantie eines Laufs, den seine Frist abgeschnitten hat (AC5).
    ///
    /// `None`, wenn **keine** Frist verlangt war: Dann darf kein Lauf als
    /// abgeschnitten gelten, und wer trotzdem ein Teilergebnis meldet, hat
    /// einen Abbruch eingeführt, den der Vertrag nicht kennt. Die geplante
    /// Arbeit ist die des Vertrags — nicht eine, die der Aufrufer nennt.
    #[must_use]
    pub const fn anytime_partial(&self, work_completed: WorkUnits) -> Option<ResultGuarantee> {
        match self.hard_deadline {
            Some(_) => Some(ResultGuarantee::AnytimePartial {
                work_completed,
                work_planned: self.deterministic_work_budget,
            }),
            None => None,
        }
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

    /// Die Garantie als JSON (AC5).
    ///
    /// `complete` steht mit im Text und ist **berechnet**: Ein Teilergebnis,
    /// dessen Text nur die erledigte Arbeit nennt, liest sich beim Zurücklesen
    /// wie ein fertiges Ergebnis.
    ///
    /// # Errors
    /// Nichtendliche Schranken oder Sicherheiten werden mit Feldname abgelehnt.
    pub fn to_json(self) -> Result<String, JsonError> {
        let complete = self.is_complete();
        Ok(match self {
            Self::CertifiedExact => {
                format!("{{\"kind\":\"certifiedExact\",\"complete\":{complete}}}")
            }
            Self::BitExactOptimized => {
                format!("{{\"kind\":\"bitExactOptimized\",\"complete\":{complete}}}")
            }
            Self::BoundedApproximation { lower, upper } => format!(
                "{{\"kind\":\"boundedApproximation\",\"lower\":{},\"upper\":{},\
                 \"complete\":{complete}}}",
                json_number(lower, "lower")?,
                json_number(upper, "upper")?,
            ),
            Self::StatisticalEstimate { confidence } => format!(
                "{{\"kind\":\"statisticalEstimate\",\"confidence\":{},\"complete\":{complete}}}",
                json_number(confidence, "confidence")?,
            ),
            Self::AnytimePartial {
                work_completed,
                work_planned,
            } => format!(
                "{{\"kind\":\"anytimePartial\",\"workCompleted\":{},\"workPlanned\":{},\
                 \"complete\":{complete}}}",
                work_completed.get(),
                work_planned.get(),
            ),
        })
    }
}
