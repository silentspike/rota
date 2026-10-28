//! Ergebnis und Bewertung getrennt speichern (Issue #913).
//!
//! # Warum drei Speicher und nicht einer
//!
//! Die Zielfunktion — wonach ein Ergebnis bewertet wird — ist ein
//! **versioniertes Feld des Laufvertrags**, kein fuer immer festgeschriebener
//! Wert. Ein Endgame-Boss darf eine andere Sicherheit verlangen als einer der
//! ersten Instanz, und eine Rangfolge, die sich im Spiel als falsch erweist,
//! wird geaendert.
//!
//! Speichert der Ergebnisspeicher eine Bewertung mit, kostet jeder Zielwechsel
//! **jede gerechnete Kampfstunde erneut**. Getrennt kostet er Mikrosekunden:
//!
//! ```text
//! Zielfunktion geaendert
//!    -> rohes Ergebnis bleibt vollstaendig gueltig
//!    -> nur die Bewertung neu rechnen
//!    -> Suche warm fortsetzen
//! ```
//!
//! # Was hier NICHT zugesichert wird
//!
//! **Nicht**, dass drei Speicher weniger Platz brauchen — sie brauchen mehr.
//! **Nicht**, dass jede Aenderung billig ist: Aendert sich die
//! **Kampfsemantik**, sind auch die rohen Ergebnisse ungueltig, und dann hilft
//! die Trennung nicht. Genau dafuer traegt der Ergebnisschluessel einen
//! Semantik-Fingerabdruck (siehe [`ErgebnisSchluessel`]).
//!
//! # Die Sperre gegen die stille Bewertung
//!
//! Ein Feld, das jemand spaeter hinzufuegt, weil es 'praktisch' ist, hebelt die
//! ganze Trennung aus — und niemand merkt es. Dagegen stehen **zwei**
//! Mechanismen, nicht einer:
//!
//! 1. [`RoherErgebnisvektor::felder_aufgezaehlt`] destrukturiert ohne `..` —
//!    ein neues Feld bricht den Bau.
//! 2. [`Zielfunktionsfrei`] traegt **kein** Typ, der die Zielfunktion kennt.
//!    Ein Feld, das sie braucht, bricht auch dann, wenn jemand die
//!    Destrukturierung brav mitpflegt.
//!
//! # Woher die Zahlen kommen
//!
//! Nicht aus einem Nachbau: [`crate::balance::BalanceResult::ergebnisentwurf`]
//! fuellt den Entwurf aus dem **vorhandenen** Ergebnissatz und destrukturiert
//! ihn dabei ebenfalls ohne `..`. Ein Modul ohne diese Speisung waere gebaut
//! und tot.
//!
//! # Der Zielwechsel als Ablauf, nicht als Behauptung (Epic #903)
//!
//! Die drei Speicher allein sagen noch nicht, dass ein Zielwechsel nichts
//! kostet — das sagt erst der Weg, auf dem eine Suche sie benutzt.
//! [`Speicherverbund::bewerte`] ist dieser Weg:
//!
//! ```text
//! je Kandidat
//!    Schluessel aus Laufkonfiguration und Liste     ErgebnisSchluessel::fuer_lauf
//!    rohes Ergebnis holen oder EINMAL rechnen       Ergebnisspeicher::hole_oder_rechne
//!    Urteil aus dem rohen Ergebnis                  Zielfunktion::beurteile
//!    Urteil ablegen, Bestwert fortschreiben         Bewertungsspeicher, Optimalitaet
//! ```
//!
//! Die Zielfunktion kommt im ersten Schritt **nicht vor**. Genau das ist die
//! Stelle, an der ein Zielwechsel Kaempfe kosten wuerde — und an der die
//! Abnahme in `sim/tests/speicher_zielwechsel.rs` mutiert wird.
//!
//! Beide Fingerabdruecke sind **abgeleitet**, nicht vergeben:
//! [`ZielfunktionsHash::aus`] aus Zielvertrag und Laufvertrag,
//! [`SemantikHash::aus`] aus der Laufkonfiguration. Ein von Hand gesetzter
//! Fingerabdruck bewiese nur, dass zwei verschiedene Zahlen verschieden sind.

use core::cmp::Ordering;
use core::fmt;

use crate::apl::{Apl, Rule};
use crate::balance::{BalanceConfig, BalanceResult, Statistics};
use crate::data::Boss;
use crate::factors::DamageFactors;
use crate::gear::WornValues;
use crate::objective::{
    BalanceObjective, BoundsVerdict, CandidateMeasurement, CandidateVerdict, ChangeCostRule,
    CoverageObligations, Determinism, FeasibilityThresholds, LexicographicScore, RankingComparison,
    RobustnessRequirements, RunContract, TargetBand,
};

// ------------------------------------------------------------ Das Verfahren

/// FNV-1a ueber 64 Bit, jede Zahl mit ihren Bytes niederwertig zuerst.
///
/// Dieselben Konstanten wie im Fingerabdruck des Durchgangs-Akkumulators
/// (`akkumulator.rs`, #967). 64 und nicht 32 Bit wie in `checksum.rs`: Beide
/// Fingerabdruecke hier tragen `u64`, und der Aufbau-Anteil eines Schluessels
/// muss ueber sehr viele Aufbauten kollisionsarm bleiben.
///
/// Jeder Fingerabdruck beginnt mit einer **Formatkennung**. Aendert sich, was
/// eingeruehrt wird, aendert man die Kennung mit — sonst bekaemen zwei
/// verschiedene Verfahren dieselbe Zahl.
#[derive(Debug, Clone, Copy)]
struct Fnv64(u64);

impl Fnv64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn neu(format: &str) -> Self {
        let mut h = Self(Self::OFFSET);
        h.text(format);
        h
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn zahl(&mut self, wert: u64) {
        self.bytes(&wert.to_le_bytes());
    }

    fn wahrheit(&mut self, wert: bool) {
        self.zahl(u64::from(wert));
    }

    /// Ueber die **Bits**: Zwei Zahlen, die sich im letzten Bit
    /// unterscheiden, sind zwei Vertraege.
    fn gleitkomma(&mut self, wert: f64) {
        self.zahl(wert.to_bits());
    }

    /// Laenge zuerst, damit Feldgrenzen eindeutig bleiben.
    fn text(&mut self, wert: &str) {
        self.zahl(wert.len() as u64);
        self.bytes(wert.as_bytes());
    }

    /// `None` und `Some(0)` sind verschiedene Aussagen und bekommen
    /// verschiedene Zahlen.
    fn vielleicht(&mut self, wert: Option<u64>) {
        match wert {
            None => self.zahl(0),
            Some(w) => {
                self.zahl(1);
                self.zahl(w);
            }
        }
    }

    fn vielleicht_gleitkomma(&mut self, wert: Option<f64>) {
        self.vielleicht(wert.map(f64::to_bits));
    }
}

// ---------------------------------------------------------------- Fingerabdruecke

/// Fingerabdruck der **Kampfsemantik**.
///
/// Er steht im Ergebnisschluessel, nicht im Wert. Aendert sich, wie ein Kampf
/// gerechnet wird, aendert sich der Schluessel — und alte Ergebnisse werden
/// nicht mehr gefunden, statt still weiterverwendet zu werden.
///
/// **Das ist der Unterschied zwischen 'die Trennung hilft' und 'die Trennung
/// luegt'.** Ohne ihn liefe die dritte Gegenprobe aus Issue #913 ins Leere:
/// Ein Lauf mit geaenderter Semantik muss ALLE Kaempfe neu rechnen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SemantikHash(pub u64);

impl SemantikHash {
    /// Der Fingerabdruck der Kampfsemantik eines Balance-Laufs (Epic #903).
    ///
    /// # Was hier „Semantik" heisst
    ///
    /// Alles, was aus einem Aufbau und einem Boss ein Ergebnis macht: wie lange
    /// ein Kampf dauert und wann er endet, die Faktoren der Schadensformel, und
    /// ueber welche Durchgaenge gemittelt wird. Welches Feld der
    /// Laufkonfiguration wohin gehoert, steht an EINER Stelle, siehe
    /// `zerlege` — und dort bricht ein neues Feld den Bau, bis es eingeordnet
    /// ist.
    ///
    /// # Was NICHT darin steht, und warum
    ///
    /// * **Die Bewertung.** `prevented_weight` und `damage_taken_baseline`
    ///   liest allein die Suche (`rotation_search.rs`), um Schaden und
    ///   Verhindertes zu gewichten — `balance::run` liest keines von beiden.
    ///   Stuenden sie hier, kostete eine geaenderte Gewichtung jeden Kampf: der
    ///   Fehler, gegen den dieses Modul steht.
    /// * **Die Rechenaufteilung.** `threads` darf das Ergebnis nicht beruehren.
    /// * **Programm- und Datenstand.** Beide sind innerhalb eines Prozesses
    ///   fest, und der Speicher lebt nur im Prozess — ein Format auf Platte ist
    ///   ausdruecklich nicht Teil von #913. Ein dauerhafter Speicher braucht
    ///   beide im Fingerabdruck; heute faenden sie keine Stelle, an der sie sich
    ///   aendern koennten.
    #[must_use]
    pub fn aus(config: &BalanceConfig) -> Self {
        zerlege(config).semantik
    }
}

/// Wohin jedes Feld einer Laufkonfiguration im Schluessel gehoert.
struct Zerlegung {
    /// Wie aus Aufbau und Boss ein Ergebnis wird.
    semantik: SemantikHash,
    /// Ausruestung und Talente — der Teil des Aufbaus, der nicht in der
    /// Prioritaetenliste steht.
    ausstattung: u64,
    /// Gegen wen.
    boss: &'static str,
}

/// Ordnet **jedes** Feld der Laufkonfiguration einer Rolle zu.
///
/// Die Destrukturierung benutzt kein `..`: Ein neues Feld in
/// [`BalanceConfig`] bricht hier den Bau (`E0027`). Die faellige Frage lautet
/// dann nicht „eintragen", sondern **welche Rolle**: Semantik, Aufbau, Boss —
/// oder gar nicht im Schluessel, weil es nur die Bewertung oder die
/// Rechenaufteilung betrifft. Wer ein Ergebnis-bestimmendes Feld vergisst,
/// bekommt fuer zwei verschiedene Kaempfe denselben Schluessel und damit
/// still das falsche Ergebnis.
fn zerlege(config: &BalanceConfig) -> Zerlegung {
    let BalanceConfig {
        // ---- Semantik ----
        iterations,
        base_seed,
        duration_ms,
        stop_on_kill,
        factors,
        // ---- Boss: ein eigenes Feld des Schluessels ----
        boss,
        // ---- Aufbau: Ausruestung und Talente ----
        worn,
        talent_resource_permille,
        talent_dot_duration_permille,
        talent_permille,
        talent_control_gain_permille,
        talent_decay_effect_permille,
        talent_control_cost_permille,
        talent_control_effect_permille,
        talent_decay_stack_permille,
        talent_decay_duration_permille,
        talent_shield_amount_permille,
        talent_shield_duration_permille,
        talent_shield_stacks_bonus,
        talent_cooldown_reduction_permille,
        talent_cooldown_charges_bonus,
        talent_cooldown_threshold_permille,
        talent_spender_damage_permille,
        talent_build_damage_permille,
        talent_decay_cap_bonus,
        talent_decay_ward_permille,
        talent_convert_permille,
        // ---- NICHT im Schluessel ----
        // Rechenaufteilung: Jeder Durchgang haengt allein an seinem Startwert.
        threads: _,
        // Gewicht und Bezugspunkt der BEWERTUNG in der Suche, siehe
        // `SemantikHash::aus` — kein Kampf liest sie.
        prevented_weight: _,
        damage_taken_baseline: _,
    } = config;

    let mut semantik = Fnv64::neu("kampfsemantik/1");
    semantik.zahl(u64::from(*iterations));
    semantik.zahl(u64::from(*base_seed));
    semantik.vielleicht(duration_ms.map(u64::from));
    semantik.wahrheit(*stop_on_kill);
    {
        let DamageFactors {
            essence,
            talents,
            gear_effects,
            buffs,
            synergy,
            resistance,
            vulnerability,
            crit,
        } = factors;
        for faktor in [
            essence,
            talents,
            gear_effects,
            buffs,
            synergy,
            resistance,
            vulnerability,
            crit,
        ] {
            semantik.gleitkomma(*faktor);
        }
    }

    let mut ausstattung = Fnv64::neu("ausstattung/1");
    {
        let WornValues {
            catalysis,
            purity,
            yield_bonus,
            essence,
            vessel,
            ward,
            gear_effects,
        } = worn;
        for wert in [
            catalysis,
            purity,
            yield_bonus,
            essence,
            vessel,
            ward,
            gear_effects,
        ] {
            ausstattung.zahl(u64::from(*wert));
        }
    }
    for talent in [
        talent_resource_permille,
        talent_dot_duration_permille,
        talent_permille,
        talent_control_gain_permille,
        talent_decay_effect_permille,
        talent_control_cost_permille,
        talent_control_effect_permille,
        talent_decay_stack_permille,
        talent_decay_duration_permille,
        talent_shield_amount_permille,
        talent_shield_duration_permille,
        talent_shield_stacks_bonus,
        talent_cooldown_reduction_permille,
        talent_cooldown_charges_bonus,
        talent_cooldown_threshold_permille,
        talent_spender_damage_permille,
        talent_build_damage_permille,
        talent_decay_ward_permille,
        talent_convert_permille,
    ] {
        ausstattung.zahl(u64::from(*talent));
    }
    ausstattung.zahl(*talent_decay_cap_bonus as u64);

    Zerlegung {
        semantik: SemantikHash(semantik.0),
        ausstattung: ausstattung.0,
        boss: boss.id,
    }
}

/// Fingerabdruck der **Zielfunktion**.
///
/// # Warum ein Hash und keine Rangfolge
///
/// Die Wahl der Zielfunktion gehoert nicht in dieses Modul — Issue #913 macht
/// sie **billig aenderbar**, es trifft sie nicht. Hier steht deshalb nur ihre
/// Identitaet.
///
/// Dieser Typ traegt **absichtlich kein** [`Zielfunktionsfrei`]. Er ist der
/// Marker, an dem der Bau bricht, wenn jemand eine Bewertung in den
/// Ergebnisspeicher schmuggelt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZielfunktionsHash(pub u64);

impl fmt::Display for ZielfunktionsHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "zf:{:016x}", self.0)
    }
}

impl ZielfunktionsHash {
    /// Der Fingerabdruck einer Zielfunktion — aus Zielvertrag **und**
    /// Laufvertrag (Epic #903).
    ///
    /// # Warum beide
    ///
    /// Das Urteil haengt an beiden. [`BalanceObjective::judge`] liest die
    /// harten Bedingungen, [`BalanceObjective::compare`] die Rangfolge; die
    /// Stufenwerte entstehen aus Schwelle, Zufallsangabe, Zielband und
    /// Lebensgrenze des [`RunContract`]. Fehlte einer der beiden hier, liefe
    /// ein Wechsel an ihm unter dem alten Fingerabdruck — und der
    /// Bewertungsspeicher gaebe das Urteil der vorigen Zielfunktion zurueck,
    /// still und plausibel.
    ///
    /// # Jedes Feld, und keines vergessen
    ///
    /// Beide Vertraege und alle ihre Teile werden **ohne `..`** zerlegt. Ein
    /// neues Feld bricht hier den Bau, statt still nicht mitzuzaehlen.
    ///
    /// Die Rangfolge geht ueber [`crate::objective::RankingStage::rank`] ein
    /// und die Zufallsangabe ueber eine feste Zahl je Variante — nicht ueber
    /// die Reihenfolge im `enum`, die sich beim Einfuegen einer Variante
    /// verschoebe.
    #[must_use]
    pub fn aus(ziel: &BalanceObjective, vertrag: &RunContract) -> Self {
        fn schwellen(h: &mut Fnv64, werte: &FeasibilityThresholds) {
            let FeasibilityThresholds {
                min_win_probability,
                min_mechanic_probability,
            } = werte;
            h.vielleicht_gleitkomma(*min_win_probability);
            h.vielleicht_gleitkomma(*min_mechanic_probability);
        }

        let BalanceObjective {
            hard_constraints,
            lexicographic_dimensions,
            coverage_obligations,
            robustness_requirements,
            designer_change_cost,
        } = ziel;
        let RunContract {
            version,
            thresholds,
            determinism,
            target_band,
            vessel_max,
        } = vertrag;

        let mut h = Fnv64::neu("zielfunktion/1");

        // ---- Zielvertrag ----
        schwellen(&mut h, hard_constraints);
        h.zahl(lexicographic_dimensions.len() as u64);
        for stufe in lexicographic_dimensions {
            h.zahl(stufe.rank() as u64);
        }
        let CoverageObligations {
            solution_families_per_mechanic,
            every_spell_needs_a_niche,
            every_key_talent_represented,
            every_gear_archetype_represented,
        } = coverage_obligations;
        h.zahl(u64::from(*solution_families_per_mechanic));
        h.wahrheit(*every_spell_needs_a_niche);
        h.wahrheit(*every_key_talent_represented);
        h.wahrheit(*every_gear_archetype_represented);
        let RobustnessRequirements {
            min_encounters,
            min_probability_per_encounter,
        } = robustness_requirements;
        h.zahl(u64::from(*min_encounters));
        h.gleitkomma(*min_probability_per_encounter);
        let ChangeCostRule {
            cost_per_changed_value,
        } = designer_change_cost;
        h.gleitkomma(*cost_per_changed_value);

        // ---- Laufvertrag ----
        h.zahl(u64::from(*version));
        schwellen(&mut h, thresholds);
        h.zahl(match determinism {
            Determinism::Exact => 1,
            Determinism::Stochastic => 2,
        });
        match target_band {
            None => h.zahl(0),
            Some(TargetBand { low, high }) => {
                h.zahl(1);
                h.gleitkomma(*low);
                h.gleitkomma(*high);
            }
        }
        h.vielleicht(vessel_max.map(u64::from));

        Self(h.0)
    }
}

// ------------------------------------------------------------ Der Marker

/// Ein Typ, der **nichts** ueber die Zielfunktion weiss.
///
/// # Wofuer der Marker da ist
///
/// Er ist die zweite Haelfte der Sperre aus AC2. Die Destrukturierung allein
/// faengt nur **neue** Felder; sie sagt nichts darueber, ob ein Feld eine
/// Bewertung TRAEGT. Dieser Marker tut genau das: Wer ein Feld vom Typ
/// [`ZielfunktionsHash`] oder [`Bewertung`] in den rohen Vektor legt, bekommt
/// keinen uebersetzbaren Simulator.
///
/// **Bewusst nicht fuer `bool` implementiert.** Ein `bool` kann alles
/// bedeuten, auch 'war gut' — und genau das waere eine Bewertung im
/// Ergebnisspeicher. Wer einen Wahrheitswert braucht, nennt ihn beim Namen
/// (etwa [`Siegzahl`]).
pub trait Zielfunktionsfrei {}

impl Zielfunktionsfrei for u32 {}
impl Zielfunktionsfrei for u64 {}
// **Kein eigener Eintrag fuer `Millis`**: `time::Millis` ist ein Alias auf
// `u32` (`pub type Millis = u32`), kein eigener Typ. Eine zweite Umsetzung
// waere dieselbe und wird vom Uebersetzer als Konflikt gemeldet (E0119) —
// gemessen beim Bau, nicht vermutet.

/// Mittelwert und Streuung sagen, **was** passiert ist, nicht ob es gut war.
impl Zielfunktionsfrei for Statistics {}

// Die Sperre in der Gegenrichtung (Epic #903).
//
// Der Marker faengt ein Feld vom falschen Typ — solange dieser Typ den Marker
// nicht traegt. Die kuerzeste Umgehung ist deshalb EINE Zeile,
// `impl Zielfunktionsfrei for CandidateVerdict {}`, und danach ginge jedes
// Urteil als rohes Ergebnis durch, ohne dass ein Prueffall es saehe.
//
// Dieser Block macht genau diese Zeile zum Uebersetzungsfehler: Traegt einer
// der unten genannten Typen den Marker, passen BEIDE Umsetzungen von
// `OhneMarker`, der Typparameter `_` laesst sich nicht mehr erschliessen, und
// der Bau bricht mit E0283. Das Verfahren ist das von `assert_not_impl_any!`
// aus `static_assertions` — hier ohne die Kiste, weil die Bibliothek keine
// Abhaengigkeiten hat.
//
// Aufgenommen ist jeder Typ, der eine Zielfunktion KENNT oder ihr Ergebnis
// TRAEGT.
const _: fn() = || {
    trait OhneMarker<Weiche> {
        fn pruefe() {}
    }
    impl<T: ?Sized> OhneMarker<()> for T {}
    impl<T: ?Sized + Zielfunktionsfrei> OhneMarker<u8> for T {}

    let _ = <ZielfunktionsHash as OhneMarker<_>>::pruefe;
    let _ = <Bewertung as OhneMarker<_>>::pruefe;
    let _ = <CandidateVerdict as OhneMarker<_>>::pruefe;
    let _ = <LexicographicScore as OhneMarker<_>>::pruefe;
    let _ = <RankingComparison as OhneMarker<_>>::pruefe;
    let _ = <BalanceObjective as OhneMarker<_>>::pruefe;
    let _ = <RunContract as OhneMarker<_>>::pruefe;
    let _ = <Zielfunktion as OhneMarker<_>>::pruefe;
};

// ---------------------------------------------------------- Der rohe Vektor

/// Wie oft der Boss gefallen ist.
///
/// Ein eigener Typ statt `u32`, damit an der Verwendungsstelle steht, was
/// gezaehlt wird — und damit niemand versucht ist, hier ein `bool` 'gewonnen'
/// abzulegen. Ob viele Siege gut sind, entscheidet die Zielfunktion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Siegzahl(pub u32);

impl Zielfunktionsfrei for Siegzahl {}

/// Wie oft jeder Zauber gewirkt wurde — die **Ereignisse** aus AC1.
///
/// Ein eigener Typ statt `Vec<u32>`, damit der Marker daran haengen kann und
/// damit der Name sagt, was die Zahlen bedeuten. Der Index ist der
/// Zauber-Index aus [`crate::data`].
///
/// **Zielfunktionsfrei**, obwohl es verlockend waere, hier eine 'Effizienz'
/// abzulegen: Wie oft etwas gewirkt wurde, ist eine Beobachtung. Ob das gut
/// war, ist eine Bewertung.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Wirkungszahlen(pub Vec<u32>);

impl Zielfunktionsfrei for Wirkungszahlen {}

impl Wirkungszahlen {
    /// Summe ueber alle Zauber.
    #[must_use]
    pub fn gesamt(&self) -> u64 {
        self.0.iter().map(|n| u64::from(*n)).sum()
    }
}

/// Das rohe Ergebnis eines Laufs — **ohne jede Bewertung**.
///
/// # Was hier NICHT hineingehoert
///
/// Auch ein 'Gesamtwert' oder ein 'Rang' ist bereits eine Bewertung. Der
/// Speicher enthaelt, **was im Kampf passiert ist** — und nichts darueber, ob
/// das gut war. Wer eine Zahl braucht, die zwei Ergebnisse vergleicht, baut
/// sie aus diesen Feldern **beim Bewerten**, nicht hier.
///
/// Ebenfalls nicht hier: Beweismittel abgebrochener Kaempfe. Sie sind ein
/// vierter Speicher und gehoeren in ein eigenes Issue — festgehalten ist nur,
/// dass sie **nicht** hierher gehoeren.
///
/// # Warum genau diese zwoelf Felder
///
/// Sie sind die Abbildung des **vorhandenen** Ergebnissatzes
/// ([`crate::balance::BalanceResult`]) ohne dessen Laufmetadaten. Erfunden ist
/// hier nichts; was der Simulator heute misst, steht hier und sonst nirgends
/// noch einmal.
#[derive(Debug, Clone, PartialEq)]
pub struct RoherErgebnisvektor {
    /// Ausgeteilter Schaden.
    pub schaden: Statistics,
    /// **Durchgekommener** Schaden am Spieler — ohne das Abgefangene.
    pub erlittener_schaden: Statistics,
    /// Was der Boss insgesamt geschickt hat: durchgekommen PLUS abgefangen.
    pub eingehender_schaden: Statistics,
    /// Von der Schutzschicht abgefangener Schaden.
    pub absorption: Statistics,
    /// Zurueckgeholtes Leben des Spielers.
    pub heilung: Statistics,
    /// Vom BOSS zurueckgedrehter Fortschritt — **nicht** die Spielerheilung.
    pub boss_heilung: Statistics,
    /// Zeit bis zum Sieg, nur ueber die gewonnenen Durchgaenge.
    pub siegzeit_ms: Statistics,
    /// Zeit bis zum Tod des Spielers, nur ueber die verlorenen Durchgaenge.
    pub todeszeit_ms: Statistics,
    /// Wie viele Durchgaenge gerechnet wurden.
    pub durchgaenge: u32,
    /// Wie oft der Boss gefallen ist.
    pub siege: Siegzahl,
    /// Wie oft der Spieler gestorben ist.
    pub tode: u32,
    /// Wirkungen je Zauber — die Ereignisse.
    pub wirkungen: Wirkungszahlen,
}

/// Wie viele Felder das Ergebnisschema hat.
///
/// Steht als Zahl da, damit ein Pruefall sie festnageln kann. Weicht sie von
/// der Laenge aus [`RoherErgebnisvektor::felder_aufgezaehlt`] ab, ist die
/// Aufzaehlung unvollstaendig — und dann haette der Bau brechen muessen.
pub const FELDER_IM_SCHEMA: usize = 12;

impl RoherErgebnisvektor {
    /// Jedes Feld einzeln — **und hier bricht der Bau** (AC2).
    ///
    /// Die Destrukturierung nennt jedes Feld und benutzt **kein** `..`; ein
    /// neues Feld meldet `E0027`. Zusaetzlich laeuft jedes Feld durch
    /// `nur_zielfunktionsfrei`, das [`Zielfunktionsfrei`] verlangt — ein Feld,
    /// das die Zielfunktion kennt, bricht dort.
    ///
    /// **Beide Mechanismen zusammen**, weil jeder allein eine Luecke laesst:
    /// Die Destrukturierung faengt nur neue Felder, der Marker nur falsche
    /// Typen. Wer ein neues Feld hinzufuegt UND die Destrukturierung pflegt,
    /// laeuft in den zweiten; wer ein vorhandenes Feld auf einen bewertenden
    /// Typ umstellt, ebenfalls.
    ///
    /// Die zurueckgegebene Zahl je Feld ist der Mittelwert beziehungsweise die
    /// Zahl selbst; fuer die Wirkungen ihre Summe. Sie dient dem Pruefstand,
    /// nicht der Rechnung — der Speicher gibt den vollen Vektor heraus.
    #[must_use]
    pub fn felder_aufgezaehlt(&self) -> Vec<(&'static str, f64)> {
        fn nur_zielfunktionsfrei<T: Zielfunktionsfrei>(_wert: &T) {}

        let RoherErgebnisvektor {
            schaden,
            erlittener_schaden,
            eingehender_schaden,
            absorption,
            heilung,
            boss_heilung,
            siegzeit_ms,
            todeszeit_ms,
            durchgaenge,
            siege,
            tode,
            wirkungen,
        } = self;

        nur_zielfunktionsfrei(schaden);
        nur_zielfunktionsfrei(erlittener_schaden);
        nur_zielfunktionsfrei(eingehender_schaden);
        nur_zielfunktionsfrei(absorption);
        nur_zielfunktionsfrei(heilung);
        nur_zielfunktionsfrei(boss_heilung);
        nur_zielfunktionsfrei(siegzeit_ms);
        nur_zielfunktionsfrei(todeszeit_ms);
        nur_zielfunktionsfrei(durchgaenge);
        nur_zielfunktionsfrei(siege);
        nur_zielfunktionsfrei(tode);
        nur_zielfunktionsfrei(wirkungen);

        vec![
            ("schaden", schaden.mean),
            ("erlittener_schaden", erlittener_schaden.mean),
            ("eingehender_schaden", eingehender_schaden.mean),
            ("absorption", absorption.mean),
            ("heilung", heilung.mean),
            ("boss_heilung", boss_heilung.mean),
            ("siegzeit_ms", siegzeit_ms.mean),
            ("todeszeit_ms", todeszeit_ms.mean),
            ("durchgaenge", f64::from(*durchgaenge)),
            ("siege", f64::from(siege.0)),
            ("tode", f64::from(*tode)),
            ("wirkungen_gesamt", wirkungen.gesamt() as f64),
        ]
    }
}

// ------------------------------------------------------------- Der Entwurf

/// Welche Felder einem Entwurf fehlen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unvollstaendig {
    /// Die Namen der fehlenden Felder, in der Reihenfolge des Schemas.
    pub fehlend: Vec<&'static str>,
}

impl fmt::Display for Unvollstaendig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unvollstaendiger Ergebnisvektor, es fehlen: {:?}",
            self.fehlend
        )
    }
}

impl std::error::Error for Unvollstaendig {}

/// Ein Ergebnis im Aufbau — noch nicht speicherbar.
///
/// # Warum es diesen Zwischenschritt gibt (AC6)
///
/// Ein Kampf ohne vollstaendigen Vektor ist ein **Torso**. Ein spaeterer Lauf
/// mit anderer Zielfunktion faende ihn im Speicher und hielte ihn fuer ein
/// Ergebnis — der Fehler waere still und nicht mehr auffindbar.
///
/// Die Sperre steht **bevor** der erste Abbruch sie treffen kann: Einen
/// Kampfabbruch gibt es heute nicht, er entsteht erst mit den frueher
/// greifenden Schranken. Wer sie erst dann baut, baut sie einmal zu spaet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ergebnisentwurf {
    /// Ausgeteilter Schaden.
    pub schaden: Option<Statistics>,
    /// Durchgekommener Schaden.
    pub erlittener_schaden: Option<Statistics>,
    /// Durchgekommener plus abgefangener Schaden.
    pub eingehender_schaden: Option<Statistics>,
    /// Abgefangener Schaden.
    pub absorption: Option<Statistics>,
    /// Zurueckgeholtes Leben.
    pub heilung: Option<Statistics>,
    /// Vom Boss zurueckgedrehter Fortschritt.
    pub boss_heilung: Option<Statistics>,
    /// Zeit bis zum Sieg.
    pub siegzeit_ms: Option<Statistics>,
    /// Zeit bis zum Tod.
    pub todeszeit_ms: Option<Statistics>,
    /// Zahl der Durchgaenge.
    pub durchgaenge: Option<u32>,
    /// Zahl der Siege.
    pub siege: Option<Siegzahl>,
    /// Zahl der Tode.
    pub tode: Option<u32>,
    /// Wirkungen je Zauber.
    pub wirkungen: Option<Wirkungszahlen>,
}

impl Ergebnisentwurf {
    /// Schliesst den Entwurf ab — oder nennt, was fehlt.
    ///
    /// Die Reihenfolge der Pruefung ist die des Schemas, damit die Meldung
    /// stabil ist und sich vergleichen laesst. Es wird **jedes** Feld geprueft
    /// und nicht beim ersten fehlenden abgebrochen: Wer einen Torso repariert,
    /// will alle Luecken auf einmal sehen.
    ///
    /// # Errors
    ///
    /// [`Unvollstaendig`] mit den Namen der fehlenden Felder.
    pub fn abschliessen(self) -> Result<RoherErgebnisvektor, Unvollstaendig> {
        let mut fehlend: Vec<&'static str> = Vec::new();
        if self.schaden.is_none() {
            fehlend.push("schaden");
        }
        if self.erlittener_schaden.is_none() {
            fehlend.push("erlittener_schaden");
        }
        if self.eingehender_schaden.is_none() {
            fehlend.push("eingehender_schaden");
        }
        if self.absorption.is_none() {
            fehlend.push("absorption");
        }
        if self.heilung.is_none() {
            fehlend.push("heilung");
        }
        if self.boss_heilung.is_none() {
            fehlend.push("boss_heilung");
        }
        if self.siegzeit_ms.is_none() {
            fehlend.push("siegzeit_ms");
        }
        if self.todeszeit_ms.is_none() {
            fehlend.push("todeszeit_ms");
        }
        if self.durchgaenge.is_none() {
            fehlend.push("durchgaenge");
        }
        if self.siege.is_none() {
            fehlend.push("siege");
        }
        if self.tode.is_none() {
            fehlend.push("tode");
        }
        if self.wirkungen.is_none() {
            fehlend.push("wirkungen");
        }
        if !fehlend.is_empty() {
            return Err(Unvollstaendig { fehlend });
        }
        Ok(RoherErgebnisvektor {
            schaden: self.schaden.expect("oben geprueft"),
            erlittener_schaden: self.erlittener_schaden.expect("oben geprueft"),
            eingehender_schaden: self.eingehender_schaden.expect("oben geprueft"),
            absorption: self.absorption.expect("oben geprueft"),
            heilung: self.heilung.expect("oben geprueft"),
            boss_heilung: self.boss_heilung.expect("oben geprueft"),
            siegzeit_ms: self.siegzeit_ms.expect("oben geprueft"),
            todeszeit_ms: self.todeszeit_ms.expect("oben geprueft"),
            durchgaenge: self.durchgaenge.expect("oben geprueft"),
            siege: self.siege.expect("oben geprueft"),
            tode: self.tode.expect("oben geprueft"),
            wirkungen: self.wirkungen.expect("oben geprueft"),
        })
    }
}

// ---------------------------------------------------------- Ergebnisspeicher

/// Woruber ein Ergebnis eindeutig bestimmt ist.
///
/// **Der Semantik-Hash gehoert in den SCHLUESSEL**, nicht in den Wert: Aendert
/// sich, wie gerechnet wird, findet eine Abfrage das alte Ergebnis gar nicht
/// erst — statt es zurueckzugeben und dabei zu luegen.
///
/// Die Zielfunktion steht hier **nicht**. Sie hat mit dem Ergebnis nichts zu
/// tun, nur mit seiner Bewertung. Stuende sie hier, entstuende je Zielfunktion
/// ein eigener Ergebnissatz — und ein Zielwechsel koestete wieder jeden Kampf.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ErgebnisSchluessel {
    /// Welcher Aufbau gerechnet wurde (Rotation, Ausruestung, Talente).
    pub aufbau: String,
    /// Gegen welchen Boss.
    pub boss: String,
    /// Mit welcher Kampfsemantik.
    pub semantik: SemantikHash,
}

impl ErgebnisSchluessel {
    /// Der Schluessel eines Balance-Laufs — **abgeleitet, nicht vergeben**
    /// (Epic #903).
    ///
    /// # Warum nicht der Name der Liste
    ///
    /// Zwei Laeufe derselben Liste mit verschiedenen Talenten sind zwei
    /// Kaempfe. Ein Schluessel aus dem Namen allein gaebe dem zweiten das
    /// Ergebnis des ersten — still, und mit plausiblen Zahlen. Der Aufbau-Teil
    /// traegt deshalb den Namen **und** einen Fingerabdruck ueber die Regeln,
    /// die Ausruestung und die Talente.
    ///
    /// # Was der Fingerabdruck der Regeln auslaesst
    ///
    /// Nur [`Rule::line`]: die Zeile in der Quelldatei. Zwei Listen, die sich
    /// allein in Leerzeilen unterscheiden, rechnen denselben Kampf.
    ///
    /// # Was hier NICHT vorkommt
    ///
    /// Die Zielfunktion. Sie hat mit dem Ergebnis nichts zu tun — stuende sie
    /// hier, kostete jeder Zielwechsel wieder jeden Kampf.
    #[must_use]
    pub fn fuer_lauf(config: &BalanceConfig, apl: &Apl) -> Self {
        let zerlegt = zerlege(config);
        let mut aufbau = Fnv64::neu("aufbau/1");
        aufbau.zahl(zerlegt.ausstattung);
        aufbau.zahl(apl.rules.len() as u64);
        for regel in &apl.rules {
            let Rule {
                spell,
                condition,
                line: _,
            } = regel;
            aufbau.zahl(*spell as u64);
            aufbau.text(&format!("{condition:?}"));
        }
        Self {
            aufbau: format!("{}#{:016x}", apl.name, aufbau.0),
            boss: zerlegt.boss.to_string(),
            semantik: zerlegt.semantik,
        }
    }
}

/// Die rohen Ergebnisse, ohne jede Bewertung.
///
/// # Der Kampfzaehler
///
/// [`Ergebnisspeicher::gerechnete_kaempfe`] zaehlt, wie oft tatsaechlich
/// gerechnet wurde. Er ist die Messgroesse fuer AC4: Ein Zielwechsel darf ihn
/// **nicht** bewegen — ein Lauf mit geaenderter Kampfsemantik dagegen schon.
#[derive(Debug, Default)]
pub struct Ergebnisspeicher {
    eintraege: Vec<(ErgebnisSchluessel, RoherErgebnisvektor)>,
    gerechnete_kaempfe: u64,
    abgelehnte: u64,
}

impl Ergebnisspeicher {
    /// Ein leerer Speicher.
    #[must_use]
    pub fn neu() -> Self {
        Self::default()
    }

    /// Wie viele Kaempfe seit dem Anlegen wirklich gerechnet wurden.
    #[must_use]
    pub const fn gerechnete_kaempfe(&self) -> u64 {
        self.gerechnete_kaempfe
    }

    /// Wie viele Entwuerfe als unvollstaendig abgelehnt wurden.
    ///
    /// Steht getrennt, damit 'nichts gespeichert' und 'alles abgelehnt'
    /// unterscheidbar bleiben.
    #[must_use]
    pub const fn abgelehnte(&self) -> u64 {
        self.abgelehnte
    }

    /// Wie viele Ergebnisse liegen.
    #[must_use]
    pub fn anzahl(&self) -> usize {
        self.eintraege.len()
    }

    /// Legt ein abgeschlossenes Ergebnis ab.
    ///
    /// Nimmt einen **Entwurf** und lehnt ihn ab, wenn er unvollstaendig ist
    /// (AC6). Ein Torso im Speicher waere spaeter nicht mehr als solcher
    /// erkennbar.
    ///
    /// # Errors
    ///
    /// [`Unvollstaendig`], wenn dem Entwurf ein Feld fehlt. Dann wird
    /// **nichts** abgelegt.
    pub fn speichere(
        &mut self,
        schluessel: ErgebnisSchluessel,
        entwurf: Ergebnisentwurf,
    ) -> Result<(), Unvollstaendig> {
        match entwurf.abschliessen() {
            Err(fehler) => {
                self.abgelehnte += 1;
                Err(fehler)
            }
            Ok(vektor) => {
                match self.eintraege.iter_mut().find(|(k, _)| *k == schluessel) {
                    Some(vorhanden) => vorhanden.1 = vektor,
                    None => self.eintraege.push((schluessel, vektor)),
                }
                Ok(())
            }
        }
    }

    /// Holt ein Ergebnis, falls es zu genau diesem Schluessel vorliegt.
    #[must_use]
    pub fn hole(&self, schluessel: &ErgebnisSchluessel) -> Option<&RoherErgebnisvektor> {
        self.eintraege
            .iter()
            .find(|(k, _)| k == schluessel)
            .map(|(_, v)| v)
    }

    /// Holt ein Ergebnis oder rechnet es — und zaehlt nur im zweiten Fall.
    ///
    /// **Das ist die Stelle, an der AC4 gemessen wird.** Liegt das Ergebnis
    /// vor, wird `rechne` nicht aufgerufen und der Zaehler bewegt sich nicht.
    ///
    /// # Errors
    ///
    /// [`Unvollstaendig`], wenn `rechne` einen Torso liefert.
    pub fn hole_oder_rechne(
        &mut self,
        schluessel: ErgebnisSchluessel,
        rechne: impl FnOnce() -> Ergebnisentwurf,
    ) -> Result<RoherErgebnisvektor, Unvollstaendig> {
        if let Some(vorhanden) = self.hole(&schluessel) {
            return Ok(vorhanden.clone());
        }
        self.gerechnete_kaempfe += 1;
        let entwurf = rechne();
        let vektor = entwurf.abschliessen().inspect_err(|_| {
            self.abgelehnte += 1;
        })?;
        self.eintraege.push((schluessel, vektor.clone()));
        Ok(vektor)
    }

    /// Alle Schluessel, in Einfuegereihenfolge.
    ///
    /// Keine Hash-Abbildung: Die Reihenfolge ist Teil des Determinismus
    /// (GAME-DESIGN 18.3).
    pub fn schluessel(&self) -> impl Iterator<Item = &ErgebnisSchluessel> {
        self.eintraege.iter().map(|(k, _)| k)
    }
}

// --------------------------------------------------------- Bewertungsspeicher

/// Was eine Zielfunktion aus einem rohen Ergebnis macht — als **eine Zahl**.
///
/// Traegt **absichtlich kein** [`Zielfunktionsfrei`]: Dieser Typ darf nie in
/// den Ergebnisspeicher geraten.
///
/// # Eine Zahl ist nicht das Urteil des Zielvertrags
///
/// Sie war die Voreinstellung aus #913, bevor ein Zielvertrag an den Speicher
/// angeschlossen war, und bleibt die Voreinstellung der Speicher. Der
/// entschiedene Zielvertrag (#939) urteilt aber lexikographisch und mit
/// Zulaessigkeit davor — sein Urteil ist ein [`CandidateVerdict`], und beide
/// Speicher nehmen es ebenso auf (`Bewertungsspeicher<CandidateVerdict>`,
/// [`Optimalitaet::melde_urteil`]). Eine Rangfolge in eine Zahl zu falten,
/// hiesse die Gewichte zu erfinden, die der Zielvertrag ausdruecklich nicht hat.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bewertung(pub f64);

/// Woruber eine Bewertung eindeutig bestimmt ist.
///
/// **Der Zielfunktions-Hash steht im SCHLUESSEL, nicht im Wert** (AC3). Stuende
/// er im Wert, lieferte eine Abfrage die Bewertung der VORIGEN Zielfunktion
/// zurueck — und der Fehler waere still, weil die Zahl plausibel aussieht.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BewertungsSchluessel {
    /// Welches Ergebnis bewertet wurde.
    pub ergebnis: ErgebnisSchluessel,
    /// Nach welcher Zielfunktion.
    pub zielfunktion: ZielfunktionsHash,
}

/// Die Bewertungen, je Zielfunktion getrennt.
///
/// `W` ist, was abgelegt wird: voreingestellt eine [`Bewertung`], fuer den
/// entschiedenen Zielvertrag ein [`CandidateVerdict`]. Der Schluessel ist in
/// beiden Faellen derselbe — mit dem Zielfunktions-Hash darin.
#[derive(Debug)]
pub struct Bewertungsspeicher<W = Bewertung> {
    eintraege: Vec<(BewertungsSchluessel, W)>,
}

// Von Hand statt `derive`: Das abgeleitete `Default` verlangte `W: Default`,
// und ein Urteil hat keinen sinnvollen Vorgabewert — ein leerer Speicher
// braucht auch keinen.
impl<W> Default for Bewertungsspeicher<W> {
    fn default() -> Self {
        Self {
            eintraege: Vec::new(),
        }
    }
}

impl<W: Clone> Bewertungsspeicher<W> {
    /// Ein leerer Speicher.
    #[must_use]
    pub fn neu() -> Self {
        Self::default()
    }

    /// Legt eine Bewertung ab.
    pub fn setze(&mut self, schluessel: BewertungsSchluessel, wert: W) {
        match self.eintraege.iter_mut().find(|(k, _)| *k == schluessel) {
            Some(vorhanden) => vorhanden.1 = wert,
            None => self.eintraege.push((schluessel, wert)),
        }
    }

    /// Holt eine Bewertung — nur fuer **genau diese** Zielfunktion.
    #[must_use]
    pub fn hole(&self, schluessel: &BewertungsSchluessel) -> Option<W> {
        self.eintraege
            .iter()
            .find(|(k, _)| k == schluessel)
            .map(|(_, v)| v.clone())
    }

    /// Wie viele Bewertungen zu einer Zielfunktion vorliegen.
    #[must_use]
    pub fn anzahl_fuer(&self, zielfunktion: ZielfunktionsHash) -> usize {
        self.eintraege
            .iter()
            .filter(|(k, _)| k.zielfunktion == zielfunktion)
            .count()
    }

    /// Wie viele Bewertungen insgesamt vorliegen.
    #[must_use]
    pub fn anzahl(&self) -> usize {
        self.eintraege.len()
    }
}

// -------------------------------------------------------------- Sucharchiv

/// Was ein Zielwechsel mit einem Archiv-Teil macht.
///
/// # Die Entscheidung aus AC5, und warum sie so faellt
///
/// 'Unentschieden ist falsch' — deshalb steht sie im **Typ** und nicht in
/// einem Kommentar. Ein Pruefall kann eine Konstante festnageln; einen
/// Kommentar kann er nicht lesen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zielwechsel {
    /// Bleibt gueltig.
    Ueberlebt,
    /// Wird verworfen.
    Verworfen,
}

/// Welche Nischen des Suchraums schon erreicht wurden.
///
/// **Ueberlebt einen Zielwechsel.** Eine Nische ist durch das ERGEBNIS
/// definiert — welcher Bereich des Raums erreicht wurde —, nicht dadurch, ob
/// das Ergebnis gut war. Sie zu verwerfen hiesse, gerechnete Kaempfe
/// wegzuwerfen; genau das verhindert dieses Modul.
#[derive(Debug, Default)]
pub struct Nischenbelegung {
    belegt: Vec<String>,
}

impl Nischenbelegung {
    /// Wie ein Zielwechsel sie behandelt.
    pub const VERHALTEN: Zielwechsel = Zielwechsel::Ueberlebt;

    /// Merkt eine Nische als belegt.
    pub fn belege(&mut self, nische: impl Into<String>) {
        let n = nische.into();
        if !self.belegt.contains(&n) {
            self.belegt.push(n);
        }
    }

    /// Ob diese Nische belegt ist.
    #[must_use]
    pub fn ist_belegt(&self, nische: &str) -> bool {
        self.belegt.iter().any(|n| n == nische)
    }

    /// Wie viele Nischen belegt sind.
    #[must_use]
    pub fn anzahl(&self) -> usize {
        self.belegt.len()
    }
}

/// Welcher Aufbau nach der aktuellen Zielfunktion der beste war.
///
/// **Wird bei einem Zielwechsel verworfen.** 'Das beste Ergebnis' ist ohne
/// Zielfunktion nicht definiert; ein alter Bestwert unter neuem Ziel ist keine
/// Behauptung, sondern eine falsche.
///
/// `W` ist die Art der Bewertung, wie beim [`Bewertungsspeicher`]. Verglichen
/// wird je nach Art verschieden: eine [`Bewertung`] ueber ihre Zahl
/// ([`Optimalitaet::melde`]), ein [`CandidateVerdict`] ueber
/// [`BalanceObjective::compare`] ([`Optimalitaet::melde_urteil`]).
#[derive(Debug)]
pub struct Optimalitaet<W = Bewertung> {
    beste: Option<(String, W, ZielfunktionsHash)>,
}

// Von Hand statt `derive`, aus demselben Grund wie beim Bewertungsspeicher.
impl<W> Default for Optimalitaet<W> {
    fn default() -> Self {
        Self { beste: None }
    }
}

impl<W> Optimalitaet<W> {
    /// Meldet einen Kandidaten; `besser(neu, alt)` entscheidet.
    ///
    /// Gehoert der gespeicherte Bestwert zu einer **anderen** Zielfunktion,
    /// wird er ersetzt statt verglichen: Zwei Bewertungen verschiedener
    /// Zielfunktionen sind nicht vergleichbar.
    fn melde_mit(
        &mut self,
        aufbau: String,
        wert: W,
        zf: ZielfunktionsHash,
        besser: impl FnOnce(&W, &W) -> bool,
    ) {
        let ersetzen = match &self.beste {
            Some((_, alt, alte_zf)) => *alte_zf != zf || besser(&wert, alt),
            None => true,
        };
        if ersetzen {
            self.beste = Some((aufbau, wert, zf));
        }
    }
}

impl<W: Clone> Optimalitaet<W> {
    /// Der beste Aufbau **fuer genau diese Zielfunktion**.
    ///
    /// Gibt `None` zurueck, wenn der gespeicherte Bestwert zu einer anderen
    /// Zielfunktion gehoert — lieber keine Antwort als eine falsche.
    #[must_use]
    pub fn bester(&self, zf: ZielfunktionsHash) -> Option<(&str, W)> {
        match &self.beste {
            Some((aufbau, wert, gespeicherte_zf)) if *gespeicherte_zf == zf => {
                Some((aufbau.as_str(), wert.clone()))
            }
            _ => None,
        }
    }
}

impl Optimalitaet {
    /// Wie ein Zielwechsel sie behandelt — fuer **jede** Art der Bewertung.
    ///
    /// Steht an der Voreinstellung, damit `Optimalitaet::VERHALTEN` ohne
    /// Typangabe lesbar bleibt. Ausgefuehrt wird die Regel fuer jedes `W`:
    /// [`Sucharchiv::zielwechsel`] verwirft den Bestwert, und `melde_mit`
    /// ersetzt einen Bestwert fremder Zielfunktion, statt ihn zu vergleichen.
    pub const VERHALTEN: Zielwechsel = Zielwechsel::Verworfen;

    /// Meldet einen Kandidaten mit einer Zahl als Bewertung.
    pub fn melde(&mut self, aufbau: impl Into<String>, wert: Bewertung, zf: ZielfunktionsHash) {
        self.melde_mit(aufbau.into(), wert, zf, |neu, alt| neu.0 > alt.0);
    }
}

impl Optimalitaet<CandidateVerdict> {
    /// Meldet einen Kandidaten mit dem Urteil des Zielvertrags (Epic #903).
    ///
    /// # Verglichen wird ueber den Zielvertrag, nicht ueber eine Zahl
    ///
    /// [`BalanceObjective::compare`] entscheidet: erst die Zulaessigkeit, dann
    /// die Rangfolge Stufe fuer Stufe. Ersetzt wird nur bei `Greater` — bei
    /// Gleichstand bleibt der zuerst gemeldete, damit das Ergebnis an der
    /// Reihenfolge der Kandidaten haengt und nicht an einer Laune.
    ///
    /// **Ein unzulaessiger Bestwert ist moeglich**, solange kein zulaessiger
    /// Kandidat gemeldet wurde; das Urteil sagt es dann selbst
    /// (`feasible == false`). Ihn zu verschweigen hiesse, eine Regel zu
    /// erfinden, die der Zielvertrag nicht hat — dort verliert ein
    /// unzulaessiger Kandidat gegen jeden zulaessigen, und mehr nicht.
    pub fn melde_urteil(
        &mut self,
        aufbau: impl Into<String>,
        urteil: CandidateVerdict,
        ziel: &Zielfunktion,
    ) {
        self.melde_mit(aufbau.into(), urteil, ziel.fingerabdruck(), |neu, alt| {
            ziel.ziel.compare(neu, alt).ordering == Ordering::Greater
        });
    }
}

/// Das Sucharchiv: Nischenbelegung und Optimalitaet **getrennt** (AC5).
#[derive(Debug)]
pub struct Sucharchiv<W = Bewertung> {
    /// Welche Nischen erreicht wurden — ueberlebt einen Zielwechsel.
    pub nischen: Nischenbelegung,
    /// Welcher Aufbau der beste war — wird bei einem Zielwechsel verworfen.
    pub optimalitaet: Optimalitaet<W>,
}

impl<W> Default for Sucharchiv<W> {
    fn default() -> Self {
        Self {
            nischen: Nischenbelegung::default(),
            optimalitaet: Optimalitaet::default(),
        }
    }
}

impl<W> Sucharchiv<W> {
    /// Ein leeres Archiv.
    #[must_use]
    pub fn neu() -> Self {
        Self::default()
    }

    /// Vollzieht einen Zielwechsel.
    ///
    /// Die Nischenbelegung bleibt, die Optimalitaet wird verworfen — die
    /// Entscheidung aus AC5, hier ausgefuehrt statt beschrieben.
    pub fn zielwechsel(&mut self) {
        // Nischen bleiben unberuehrt: Nischenbelegung::VERHALTEN
        self.optimalitaet = Optimalitaet::default();
    }
}

// --------------------------------------------- Der Ablauf ueber allen dreien

/// Eine Zielfunktion im Sinn dieses Moduls: Zielvertrag **und** Laufvertrag.
///
/// Warum beide zusammen gehoeren, steht an [`ZielfunktionsHash::aus`]: Das
/// Urteil haengt an beiden, also auch sein Fingerabdruck.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zielfunktion {
    /// Harte Bedingungen und Rangfolge.
    pub ziel: BalanceObjective,
    /// Schwellen, Zufallsangabe, Zielband, Lebensgrenze — mit Versionsnummer.
    pub vertrag: RunContract,
}

impl Zielfunktion {
    /// Der Fingerabdruck — abgeleitet, siehe [`ZielfunktionsHash::aus`].
    #[must_use]
    pub fn fingerabdruck(&self) -> ZielfunktionsHash {
        ZielfunktionsHash::aus(&self.ziel, &self.vertrag)
    }

    /// Das Urteil ueber ein **gespeichertes** Ergebnis — ohne einen Kampf.
    ///
    /// Die Messung kommt aus [`CandidateMeasurement::from_raw_outcome`], das
    /// Urteil aus [`BalanceObjective::judge`]: dieselben Funktionen, die ein
    /// Urteil ueber einen frischen Lauf faellen, nicht ein Nachbau daneben.
    ///
    /// `aenderungskosten` und `grenzen` weiss nur der Erzeuger des Kandidaten;
    /// sie kommen deshalb von aussen und nicht aus dem Kampf.
    #[must_use]
    pub fn beurteile(
        &self,
        vektor: &RoherErgebnisvektor,
        boss: &Boss,
        aenderungskosten: Option<f64>,
        grenzen: BoundsVerdict,
    ) -> CandidateVerdict {
        self.ziel.judge(
            &CandidateMeasurement::from_raw_outcome(vektor, boss, &self.vertrag, aenderungskosten)
                .with_bounds(grenzen),
        )
    }
}

/// Ein Kandidat, wie eine Suche ihn vorlegt.
#[derive(Debug, Clone, Copy)]
pub struct Kandidat<'a> {
    /// Die Prioritaetenliste.
    pub apl: &'a Apl,
    /// Boss, Laufvertrag des Kampfs, Ausruestung, Talente.
    pub config: BalanceConfig,
    /// Was der Kandidat an den Daten aendert (Stufe 7) — das weiss nur sein
    /// Erzeuger.
    pub aenderungskosten: Option<f64>,
    /// Ob er in seiner Parameterdomaene liegt — ebenfalls Sache des
    /// Erzeugers. **Ohne diese Angabe ist jeder Kandidat unzulaessig**
    /// ([`BoundsVerdict::Unmeasured`] verletzt die harten Bedingungen).
    pub grenzen: BoundsVerdict,
}

/// Was ein Durchgang ueber die Kandidaten gekostet hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Durchgangsbilanz {
    /// Kaempfe, die **dieser** Durchgang gerechnet hat — gemessen am Zaehler
    /// des Ergebnisspeichers, vorher gegen nachher.
    pub gerechnete_kaempfe: u64,
    /// Urteile, die dieser Durchgang gefaellt hat: eines je Kandidat. Das ist
    /// die Arbeit, die ein Zielwechsel kostet — Bewerten, nicht Kaempfen.
    pub urteile: usize,
    /// Ob dieser Durchgang eine andere Zielfunktion traf als der vorige.
    pub zielwechsel: bool,
}

/// Die drei Speicher einer Suche, zusammen gefuehrt (Epic #903).
///
/// # Wofuer es den Verbund gibt
///
/// Jeder der drei Speicher ist fuer sich geprueft. Dass ein Zielwechsel
/// keinen Kampf kostet, ist aber keine Eigenschaft eines einzelnen Speichers,
/// sondern des **Wegs** durch alle drei. Dieser Weg steht hier, an einer
/// Stelle, und die Abnahme des Epics faehrt genau ihn.
#[derive(Debug, Default)]
pub struct Speicherverbund {
    /// Die rohen Ergebnisse — ohne jede Bewertung.
    pub ergebnisse: Ergebnisspeicher,
    /// Die Urteile, je Zielfunktion getrennt.
    pub urteile: Bewertungsspeicher<CandidateVerdict>,
    /// Nischenbelegung und Bestwert.
    pub archiv: Sucharchiv<CandidateVerdict>,
    /// Unter welcher Zielfunktion der vorige Durchgang lief.
    zuletzt: Option<ZielfunktionsHash>,
}

impl Speicherverbund {
    /// Ein leerer Verbund.
    #[must_use]
    pub fn neu() -> Self {
        Self::default()
    }

    /// Bewertet die Kandidaten unter dieser Zielfunktion; gerechnet wird nur,
    /// was der Ergebnisspeicher noch nicht hat.
    ///
    /// # Errors
    ///
    /// [`Unvollstaendig`], wenn ein Lauf einen Torso liefert. Ein echter
    /// Balance-Lauf tut das nie; die Sperre steht trotzdem, siehe
    /// [`Ergebnisentwurf`].
    pub fn bewerte(
        &mut self,
        kandidaten: &[Kandidat<'_>],
        ziel: &Zielfunktion,
    ) -> Result<Durchgangsbilanz, Unvollstaendig> {
        self.bewerte_mit(kandidaten, ziel, crate::balance::run)
    }

    /// Wie [`Self::bewerte`], aber mit einem anderen Laeufer.
    ///
    /// # Wofuer
    ///
    /// Fuer eine **zweite, unabhaengige Zaehlung**. Der Kampfzaehler des
    /// Ergebnisspeichers zaehlt, wo er zaehlt — haengt er einmal neben dem
    /// Aufruf, misst er eine Null, die keine ist. Ein Laeufer, der
    /// `balance::run` umhuellt und mitzaehlt, misst dort, wo der Kampf
    /// tatsaechlich passiert.
    ///
    /// # Errors
    ///
    /// Wie [`Self::bewerte`].
    pub fn bewerte_mit(
        &mut self,
        kandidaten: &[Kandidat<'_>],
        ziel: &Zielfunktion,
        mut rechne: impl FnMut(&BalanceConfig, &Apl) -> BalanceResult,
    ) -> Result<Durchgangsbilanz, Unvollstaendig> {
        let zf = ziel.fingerabdruck();
        let zielwechsel = self.zuletzt.is_some_and(|vorige| vorige != zf);
        if zielwechsel {
            self.archiv.zielwechsel();
        }
        self.zuletzt = Some(zf);

        let vorher = self.ergebnisse.gerechnete_kaempfe();
        for kandidat in kandidaten {
            // **Hier kommt die Zielfunktion nicht vor**, und das ist die
            // Zusicherung des ganzen Epics: Der Schluessel des rohen Ergebnisses
            // haengt an Liste und Laufkonfiguration, nicht am Ziel.
            let schluessel = ErgebnisSchluessel::fuer_lauf(&kandidat.config, kandidat.apl);
            let vektor = self.ergebnisse.hole_oder_rechne(schluessel.clone(), || {
                rechne(&kandidat.config, kandidat.apl).ergebnisentwurf()
            })?;
            let urteil = ziel.beurteile(
                &vektor,
                kandidat.config.boss,
                kandidat.aenderungskosten,
                kandidat.grenzen,
            );
            self.urteile.setze(
                BewertungsSchluessel {
                    ergebnis: schluessel.clone(),
                    zielfunktion: zf,
                },
                urteil.clone(),
            );
            self.archiv
                .optimalitaet
                .melde_urteil(schluessel.aufbau, urteil, ziel);
        }

        Ok(Durchgangsbilanz {
            gerechnete_kaempfe: self.ergebnisse.gerechnete_kaempfe() - vorher,
            urteile: kandidaten.len(),
            zielwechsel,
        })
    }
}
