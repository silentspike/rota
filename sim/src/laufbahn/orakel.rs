//! Das Kampf-Orakel der Fortschritts-Simulation — **Fassung A** (#922, AC1/AC2).
//!
//! # Die Frage, die es beantwortet
//!
//! **„Schafft ein durchschnittlicher Spieler das?"** — genauer: ein Spieler,
//! der jeden Boss mit derselben, fest vorgegebenen Referenz-Spielweise
//! angeht, ohne Talente, auf Kampfstufe 1, und nur seine Ausruestung wechselt.
//!
//! Nicht „ist es schaffbar?". Das waere Fassung B: je Zelle eine eigene Suche
//! nach der besten Spielweise, also ein voller Suchlauf statt eines Kampfes.
//! Beide Fragen sind berechtigt; sie unterschiedslos „Orakel" zu nennen waere
//! die Verwechslung, vor der der Plan warnt (E11.4e). Diese Datei baut nur A,
//! und jede Ausgabe, die auf ihr steht, traegt den Satz aus
//! [`FASSUNG_A`] mit.
//!
//! # Der Schluessel
//!
//! ```text
//! A   ProgressOracleKey { gear_phenotype, fixed_reference_build, boss, difficulty, ladder_level }
//!     OrakelSchluessel  { ausstattung,    referenz,              boss, schwierigkeit, leiterstufe }
//! ```
//!
//! Der Phaenotyp ist [`WornValues`] **exakt** — kein Raster. Eine Zelle zu
//! einem gerundeten Phaenotyp waere schneller und gaebe fuer zwei verschiedene
//! Ausruestungen dieselbe Antwort; die Kontrollprobe aus dem Plan (eine Zelle
//! direkt nachrechnen und vergleichen) faende dann jede Abweichung als Fehler.
//!
//! Schwierigkeit und Leiterstufe stehen im Schluessel, obwohl der Kampf sie
//! nicht kennt (`rates.ts` fuehrt drei Stufen, der Simulator eine). Das
//! Orakel **lehnt** deshalb alles ausser `(0, 0)` ab, statt die Felder still
//! zu ueberlesen — wer sie fuellt, bekaeme sonst normale Zellen fuer mythische
//! Fragen.
//!
//! # Wo die Zellen liegen
//!
//! Im vorhandenen [`Ergebnisspeicher`] aus `speicher.rs` (#913), mit dessen
//! Schluessel [`ErgebnisSchluessel::fuer_lauf`] — Aufbau (Liste, Ausruestung,
//! Talente), Boss, Kampfsemantik. Kein zweiter Speicher daneben. Was die
//! Schleife ueber dem Kampf aendert (Fallrate, Beute, Aufwertung,
//! Freischaltung, Strategie), steht in keinem dieser Felder: Darauf beruht die
//! Invalidierungsregel in `laufbahn.rs`.
//!
//! # Was die Zelle traegt
//!
//! Was der Ergebnisspeicher haelt — Siege, Durchgaenge, die mittlere Siegzeit
//! und den mittleren Schaden. Mit **einem** Durchgang (der Voreinstellung, wie
//! im Plan: „einmal gerechnet, danach nur nachgeschlagen") ist das genau ein
//! Kampf. Mit mehreren wird die Restzeit zum Mittel ueber die gewonnenen
//! Durchgaenge und der Ring-Rest eines `runaway`-Bosses zum Mittel ueber alle
//! — der Speicher fuehrt keine Einzelwerte. Das ist eine Naeherung, und sie
//! steht in jeder Ausgabe mit der Zahl der Durchgaenge daneben.

use crate::apl::Apl;
use crate::balance::{self, BalanceConfig};
use crate::data::{boss_by_id, Boss};
use crate::gear::WornValues;
use crate::speicher::{ErgebnisSchluessel, Ergebnisspeicher, RoherErgebnisvektor};

/// Der Satz, der jede Ausgabe auf diesem Orakel beschriftet (AC2).
pub const FASSUNG_A: &str = "Fassung A — feste Referenz-Spielweise: beantwortet \
\"schafft ein durchschnittlicher Spieler das?\", nicht \"ist es schaffbar?\"";

/// Die feste Referenz-Spielweise — `fixed_reference_build` aus Fassung A.
///
/// Eine Liste fuer alle Bosse, wahlweise je Boss ersetzt. **Ohne Talente und
/// auf Kampfstufe 1**: Beides legt der Balance-Laeufer fest
/// (`balance.rs`, `kampf_konfiguration`), und das Orakel rechnet ueber ihn.
#[derive(Debug, Clone)]
pub struct Referenzspielweise {
    /// Die Liste fuer jeden Boss ohne eigene.
    pub liste: Apl,
    /// Eigene Listen je Boss-Kennung.
    pub je_boss: Vec<(String, Apl)>,
}

impl Referenzspielweise {
    /// Eine Liste fuer alle Bosse.
    #[must_use]
    pub const fn fuer_alle(liste: Apl) -> Self {
        Self {
            liste,
            je_boss: Vec::new(),
        }
    }

    /// Die Liste, mit der gegen diesen Boss gekaempft wird.
    #[must_use]
    pub fn liste_fuer(&self, boss_id: &str) -> &Apl {
        self.je_boss
            .iter()
            .find(|(id, _)| id == boss_id)
            .map_or(&self.liste, |(_, apl)| apl)
    }
}

/// Der Schluessel einer Orakel-Zelle — Fassung A (AC1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrakelSchluessel {
    /// `gear_phenotype`: die getragenen Werte, exakt.
    pub ausstattung: WornValues,
    /// Gegen welchen Boss.
    pub boss: &'static str,
    /// `difficulty` — nur 0 wird gerechnet.
    pub schwierigkeit: u32,
    /// `ladder_level` — nur 0 wird gerechnet.
    pub leiterstufe: u32,
}

/// Was eine Zelle sagt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zelle {
    /// Wie oft der Boss gefallen ist (bei `runaway`: gehalten wurde).
    pub siege: u32,
    /// Wie viele Durchgaenge gerechnet wurden.
    pub durchgaenge: u32,
    /// Mittlere Kampfzeit der gewonnenen Durchgaenge (siehe Kopf zu `runaway`).
    pub siegzeit_ms: f64,
    /// Mittlerer ausgeteilter Schaden ueber alle Durchgaenge.
    pub schaden: f64,
    /// Wie oft der Spieler gestorben ist.
    pub tode: u32,
}

impl Zelle {
    /// Aus dem rohen Ergebnisvektor — die einzige Quelle einer Zelle.
    #[must_use]
    pub fn aus(vektor: &RoherErgebnisvektor) -> Self {
        Self {
            siege: vektor.siege.0,
            durchgaenge: vektor.durchgaenge,
            siegzeit_ms: vektor.siegzeit_ms.mean,
            schaden: vektor.schaden.mean,
            tode: vektor.tode,
        }
    }

    /// Anteil der gewonnenen Durchgaenge.
    #[must_use]
    pub fn siegquote(&self) -> f64 {
        if self.durchgaenge == 0 {
            return 0.0;
        }
        f64::from(self.siege) / f64::from(self.durchgaenge)
    }
}

/// Warum eine Zelle nicht gerechnet werden kann.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrakelFehler {
    /// Den Boss gibt es nicht.
    UnbekannterBoss(String),
    /// Schwierigkeit oder Leiterstufe ungleich 0 — der Kampf kennt sie nicht.
    NichtImKampf {
        /// Die verlangte Schwierigkeit.
        schwierigkeit: u32,
        /// Die verlangte Leiterstufe.
        leiterstufe: u32,
    },
}

impl core::fmt::Display for OrakelFehler {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnbekannterBoss(id) => write!(f, "unbekannter Boss \"{id}\""),
            Self::NichtImKampf {
                schwierigkeit,
                leiterstufe,
            } => write!(
                f,
                "Schwierigkeit {schwierigkeit} / Leiterstufe {leiterstufe}: der Simulator \
                 fuehrt eine feste Stufe, nur 0/0 ist rechenbar"
            ),
        }
    }
}

impl std::error::Error for OrakelFehler {}

/// Was die Schleife vom Kampf braucht.
///
/// Ein Trait, damit ein Prueffall die Schleife von einem Kampf trennen kann,
/// dessen Ausgang fuer die gepruefte Eigenschaft keine Rolle spielt — die
/// Fundrate haengt an den Siegen, nicht daran, wie knapp sie waren.
pub trait Kampforakel {
    /// Die Zelle zu diesem Schluessel.
    ///
    /// # Errors
    ///
    /// [`OrakelFehler`], wenn die Zelle nicht rechenbar ist.
    fn zelle(&mut self, schluessel: &OrakelSchluessel) -> Result<Zelle, OrakelFehler>;

    /// Wie viele Zellen wirklich gerechnet wurden.
    fn gerechnet(&self) -> u64;

    /// Wie oft insgesamt gefragt wurde.
    fn abgefragt(&self) -> u64;
}

/// Die Einstellungen, unter denen eine Zelle gerechnet wird — die
/// Kampfsemantik des Orakels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kampfeinstellung {
    /// Durchgaenge je Zelle (Voreinstellung 1: ein Kampf je Zelle).
    pub durchgaenge: u32,
    /// Startwert des ersten Durchgangs.
    pub startwert: u32,
}

impl Default for Kampfeinstellung {
    fn default() -> Self {
        Self {
            durchgaenge: 1,
            startwert: 0,
        }
    }
}

/// Die Laufkonfiguration einer Zelle.
///
/// **Ein Strang**: Die Schleife verteilt ganze Laufbahnen auf Straenge, nicht
/// die Durchgaenge einer Zelle — `threads` steht ausdruecklich nicht im
/// Ergebnisschluessel (`speicher.rs`, `zerlege`).
#[must_use]
pub fn laufkonfiguration(
    boss: &'static Boss,
    ausstattung: WornValues,
    einstellung: Kampfeinstellung,
) -> BalanceConfig {
    BalanceConfig {
        iterations: einstellung.durchgaenge,
        base_seed: einstellung.startwert,
        threads: 1,
        duration_ms: None,
        stop_on_kill: true,
        boss,
        worn: ausstattung,
        ..BalanceConfig::default()
    }
}

/// Das Orakel ueber dem Ergebnisspeicher.
#[derive(Debug)]
pub struct SpeicherOrakel<'a> {
    referenz: &'a Referenzspielweise,
    einstellung: Kampfeinstellung,
    speicher: Ergebnisspeicher,
    abgefragt: u64,
}

impl<'a> SpeicherOrakel<'a> {
    /// Ein Orakel mit leerem Speicher.
    #[must_use]
    pub fn neu(referenz: &'a Referenzspielweise, einstellung: Kampfeinstellung) -> Self {
        Self::mit_speicher(referenz, einstellung, Ergebnisspeicher::neu())
    }

    /// Ein Orakel auf einem vorhandenen Speicher — fuer die Kontrollprobe zu
    /// AC5, in der zwei Laeufe mit verschiedener Fallrate denselben Speicher
    /// teilen.
    #[must_use]
    pub const fn mit_speicher(
        referenz: &'a Referenzspielweise,
        einstellung: Kampfeinstellung,
        speicher: Ergebnisspeicher,
    ) -> Self {
        Self {
            referenz,
            einstellung,
            speicher,
            abgefragt: 0,
        }
    }

    /// Der Speicher, fuer die Weitergabe an einen zweiten Lauf.
    #[must_use]
    pub fn speicher(self) -> Ergebnisspeicher {
        self.speicher
    }

    /// Der Speicher, zum Nachsehen.
    #[must_use]
    pub const fn speicher_ansehen(&self) -> &Ergebnisspeicher {
        &self.speicher
    }

    /// Der Ergebnisschluessel, unter dem diese Zelle liegt.
    ///
    /// # Errors
    ///
    /// [`OrakelFehler`] wie bei [`Kampforakel::zelle`].
    pub fn ergebnisschluessel(
        &self,
        schluessel: &OrakelSchluessel,
    ) -> Result<ErgebnisSchluessel, OrakelFehler> {
        let (config, apl) = self.lauf(schluessel)?;
        Ok(ErgebnisSchluessel::fuer_lauf(&config, apl))
    }

    /// Laufkonfiguration und Liste einer Zelle. Die Liste haengt am Leben der
    /// Referenz, nicht an `self` — sonst sperrte sie den Speicher, waehrend
    /// [`Kampforakel::zelle`] ihn fuellt.
    fn lauf(
        &self,
        schluessel: &OrakelSchluessel,
    ) -> Result<(BalanceConfig, &'a Apl), OrakelFehler> {
        let referenz: &'a Referenzspielweise = self.referenz;
        if schluessel.schwierigkeit != 0 || schluessel.leiterstufe != 0 {
            return Err(OrakelFehler::NichtImKampf {
                schwierigkeit: schluessel.schwierigkeit,
                leiterstufe: schluessel.leiterstufe,
            });
        }
        let boss = boss_by_id(schluessel.boss)
            .ok_or_else(|| OrakelFehler::UnbekannterBoss(schluessel.boss.to_owned()))?;
        Ok((
            laufkonfiguration(boss, schluessel.ausstattung, self.einstellung),
            referenz.liste_fuer(schluessel.boss),
        ))
    }
}

impl Kampforakel for SpeicherOrakel<'_> {
    fn zelle(&mut self, schluessel: &OrakelSchluessel) -> Result<Zelle, OrakelFehler> {
        self.abgefragt += 1;
        let (config, apl) = self.lauf(schluessel)?;
        let ergebnis = ErgebnisSchluessel::fuer_lauf(&config, apl);
        let vektor = self
            .speicher
            .hole_oder_rechne(ergebnis, || balance::run(&config, apl).ergebnisentwurf())
            // `ergebnisentwurf` fuellt jedes Feld; ein Torso hier hiesse, dass
            // der Balance-Laeufer einen unvollstaendigen Satz liefert — das
            // soll laut sein, nicht eine Zelle mit Nullen.
            .unwrap_or_else(|fehler| panic!("Orakel-Zelle unvollstaendig: {fehler}"));
        Ok(Zelle::aus(&vektor))
    }

    fn gerechnet(&self) -> u64 {
        self.speicher.gerechnete_kaempfe()
    }

    fn abgefragt(&self) -> u64 {
        self.abgefragt
    }
}
