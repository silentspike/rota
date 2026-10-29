//! Was die Prüffälle der inkrementellen Auswertung teilen (Issue #917).
//!
//! # Wogegen geprüft wird
//!
//! Jede Abwandlung wird **zweimal voll** gerechnet: einmal als aufgezeichneter
//! Suchkampf (daraus die tatsächliche erste Abweichung), einmal als Prüfkampf
//! ([`AuditFight`]) — das ist der Kampf, den jede Paritäts-Aufzeichnung fährt.
//! Erst gegen beide wird die inkrementelle Auswertung gehalten.

use rota_sim::apl::Rule;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::fight::{AuditFight, FightConfig, SearchFight, Zielwerte};
use rota_sim::gear::WornValues;
use rota_sim::inkrementell::{Auswertung, Basislauf, RuleId};
use rota_sim::rotation_search::{all_candidates, boss_kandidaten, Candidate};

/// Die Kampfdauer der Prüffälle — die aus Anhang B.2.1.
pub const DAUER_MS: u32 = 60_000;

/// Der Kampf, wie der Balance-Läufer ihn baut: ohne Talente, ohne Protokoll,
/// über die volle Zeit.
pub fn kampf(boss: &str, seed: u32) -> FightConfig {
    FightConfig {
        seed,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        log_capacity: 0,
        boss: boss_by_id(boss).unwrap_or_else(|| panic!("Boss {boss} fehlt in den Daten")),
        ..FightConfig::default()
    }
}

/// Derselbe Kampf mit getragenem PURITY: Er würfelt.
pub fn wuerfelnder_kampf(boss: &str, seed: u32) -> FightConfig {
    FightConfig {
        worn: WornValues {
            purity: 400,
            ..WornValues::none()
        },
        ..kampf(boss, seed)
    }
}

/// Der Kandidatenvorrat der Suche an diesem Boss — dieselben Zeilen, die
/// `greedy_zuege` einfügt, samt der boss-abhängigen.
pub fn vorrat(config: &FightConfig) -> Vec<Rule> {
    let pool = all_candidates();
    let balance = BalanceConfig {
        boss: config.boss,
        ..BalanceConfig::default()
    };
    let ergaenzt = boss_kandidaten(&balance, &pool);
    pool.iter().chain(&ergaenzt).map(Candidate::rule).collect()
}

/// Ein reproduzierbarer Zufall (splitmix64).
pub struct Zufall(u64);

impl Zufall {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn bis(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z % n as u64) as usize
    }
}

/// Die erste Entscheidung, an der zwei Läufe auseinandergehen: andere Zeit
/// oder anderer Zauber. `None`, wenn beide dieselbe Folge haben.
pub fn erste_abweichung(a: &Basislauf, b: &Basislauf) -> Option<usize> {
    let n = a.punkte().len().min(b.punkte().len());
    for j in 0..n {
        let zeit_a = a.punkte()[j].accumulated_result.time;
        let zeit_b = b.punkte()[j].accumulated_result.time;
        if zeit_a != zeit_b || a.gewaehlter_zauber(j) != b.gewaehlter_zauber(j) {
            return Some(j);
        }
    }
    (a.punkte().len() != b.punkte().len()).then_some(n)
}

/// Was eine geprüfte Abwandlung ergab.
#[derive(Debug, Clone, Copy)]
pub struct Befund {
    /// Der berechnete Startpunkt.
    pub berechnet: Option<usize>,
    /// Die tatsächliche erste Abweichung, aus der Vollsimulation.
    pub tatsaechlich: Option<usize>,
    /// Wie die Auswertung lief.
    pub auswertung: Auswertung,
}

/// Prüft eine Abwandlung gegen die Vollsimulation — der Vertrag aus AC1:
///
/// ```text
/// berechneter Startprüfpunkt  ≤  tatsächlicher erster Divergenzpunkt
/// optimierter Suchlauf        =  bitgleich dasselbe Ergebnis
/// ```
///
/// Dazu die Schärfe: Der Startpunkt liegt höchstens **einen** Prüfpunkt vor
/// der Abweichung — genau einen, wenn die Abwandlung nur einen anderen
/// Entscheidungszeitpunkt schafft, null, wenn sie anders wählt.
pub fn pruefe(
    basis: &Basislauf,
    config: FightConfig,
    liste: &[RuleId],
    suche: &mut SearchFight,
    was: &str,
) -> Befund {
    let apl = basis.apl(liste);
    let voll = Basislauf::aufzeichnen(config, &apl, &[]);
    let mut pruef = AuditFight::new(config);
    pruef.run_apl(&apl);
    let pruef_ziel: Zielwerte = pruef.zielwerte();
    assert_eq!(
        voll.ziel(),
        pruef_ziel,
        "{was}: Such- und Prüfkampf rechnen verschieden"
    );

    let tatsaechlich = erste_abweichung(basis, &voll);
    let berechnet = basis.startpunkt(liste);
    let auswertung = basis.auswerten(liste, suche);
    assert_eq!(
        auswertung.ziel(),
        pruef_ziel,
        "{was}: inkrementell ({auswertung:?}) gegen Vollsimulation, Startpunkt {berechnet:?}, \
         Abweichung {tatsaechlich:?}"
    );
    match (berechnet, tatsaechlich) {
        (None, None) => {}
        (None, Some(t)) => {
            panic!("{was}: „ändert nichts“ gemeldet, aber der Kampf weicht bei Prüfpunkt {t} ab")
        }
        (Some(b), None) => {
            panic!("{was}: Startpunkt {b} berechnet, aber die Vollsimulation weicht nirgends ab")
        }
        (Some(b), Some(t)) => {
            assert!(
                b <= t,
                "{was}: Startpunkt {b} liegt HINTER der ersten Abweichung {t} — ein \
                 stiller Ergebnisfehler"
            );
            assert!(
                t <= b + 1,
                "{was}: Startpunkt {b} liegt {} Prüfpunkte vor der Abweichung {t}",
                t - b
            );
        }
    }
    Befund {
        berechnet,
        tatsaechlich,
        auswertung,
    }
}

/// Zählt, was eine Reihe von Prüfungen ergab — für die Meldung je Operator.
#[derive(Debug, Default, Clone, Copy)]
pub struct Zaehlung {
    pub geprueft: usize,
    pub unveraendert: usize,
    pub genau: usize,
    pub einen_frueher: usize,
    /// Ereignisse, die kein Suffix-Lauf wiederholen musste.
    pub gesparte_ereignisse: u64,
    /// Ereignisse, die eine Vollsimulation je Abwandlung gerechnet hätte.
    pub ereignisse_voll: u64,
}

impl Zaehlung {
    pub fn nimm(&mut self, basis: &Basislauf, befund: &Befund) {
        self.geprueft += 1;
        self.ereignisse_voll += u64::from(basis.ereignisse());
        match befund.auswertung {
            Auswertung::Unveraendert(_) => {
                self.unveraendert += 1;
                self.gesparte_ereignisse += u64::from(basis.ereignisse());
            }
            Auswertung::Suffix { ab, .. } => {
                self.gesparte_ereignisse += u64::from(basis.punkte()[ab].event_cursor);
            }
        }
        match (befund.berechnet, befund.tatsaechlich) {
            (Some(b), Some(t)) if b == t => self.genau += 1,
            (Some(_), Some(_)) => self.einen_frueher += 1,
            _ => {}
        }
    }

    pub fn melden(&self, operator: &str) {
        let anteil = |x: usize| 100.0 * x as f64 / self.geprueft.max(1) as f64;
        println!(
            "{operator}: {} geprüft, unverändert {} ({:.1} %), Startpunkt = Abweichung {}, \
             einen früher {}, gesparte Ereignisse {:.1} %",
            self.geprueft,
            self.unveraendert,
            anteil(self.unveraendert),
            self.genau,
            self.einen_frueher,
            100.0 * self.gesparte_ereignisse as f64 / self.ereignisse_voll.max(1) as f64
        );
    }
}

/// Fügt `neu` an Stelle `stelle` ein.
pub fn eingefuegt(liste: &[RuleId], stelle: usize, neu: RuleId) -> Vec<RuleId> {
    let mut v = liste.to_vec();
    v.insert(stelle, neu);
    v
}

/// Entfernt die Zeile an Stelle `stelle`.
pub fn entfernt(liste: &[RuleId], stelle: usize) -> Vec<RuleId> {
    let mut v = liste.to_vec();
    v.remove(stelle);
    v
}

/// Tauscht die Zeilen an `a` und `b`.
pub fn getauscht(liste: &[RuleId], a: usize, b: usize) -> Vec<RuleId> {
    let mut v = liste.to_vec();
    v.swap(a, b);
    v
}
