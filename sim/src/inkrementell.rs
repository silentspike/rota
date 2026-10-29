//! Inkrementelle Auswertung: der Startpunkt wird berechnet, nicht abgeleitet
//! (Issue #917, E11.4).
//!
//! # Was hier geschieht
//!
//! Eine Suchrunde bewertet viele Abwandlungen **derselben** Basisliste — beim
//! Greedy-Einfügen jede offene Zeile an jeder Stelle. Statt jede Abwandlung von
//! vorn zu rechnen, zeichnet [`Basislauf::aufzeichnen`] den Kampf der Basisliste
//! einmal auf: vor jeder Entscheidung einen [`DecisionCheckpoint`]. Danach wird
//! für eine Abwandlung der erste Punkt **berechnet**, an dem sie überhaupt
//! wirken kann ([`Basislauf::startpunkt`]):
//!
//! ```text
//! Abwandlung
//!    ↓
//! ersten beeinflussbaren Prüfpunkt suchen
//!    ↓
//! keiner vorhanden?  → Ergebnis der Basis unverändert übernehmen
//!    ↓
//! sonst den Zustand dort laden und nur das Suffix rechnen
//! ```
//!
//! # Warum berechnet und nicht aus der Position abgeleitet
//!
//! Die naheliegende Annahme lautete „Änderung an Position p, also ab Prüfpunkt p
//! weiterrechnen“. Sie ist gemessen widerlegt: Eine Zeile an Position 8 kann
//! bereits Entscheidung 0 ändern, wenn die acht Zeilen darüber in diesem Moment
//! alle nicht greifen — 436 von 440 Varianten wichen **vor** der
//! Einfügeposition ab (`docs/SIM-ARCHITEKTUR-PLAN.md`, E11.4). Position ist nicht
//! Entscheidungsnummer.
//!
//! # Der Vertrag, der für jede Abwandlung gilt
//!
//! ```text
//! berechneter Startprüfpunkt  ≤  tatsächlicher erster Divergenzpunkt
//! optimierter Suchlauf        =  bitgleich dasselbe Ergebnis
//! ```
//!
//! Ein zu früher Prüfpunkt kostet nur Leistung. **Ein zu später liefert ein
//! falsches Ergebnis, und zwar still** — der Suffix-Lauf beginnt in einem Zustand,
//! den die Abwandlung nie erreicht hätte. `tests/inkrementell.rs` prüft den
//! Vertrag je Änderungsart gegen die Vollsimulation.
//!
//! # Wie der Startpunkt berechnet wird
//!
//! Jede Abwandlung ist eine neue Regelliste über das **Universum** der
//! Aufzeichnung: erst die Regeln der Basisliste, dann der Kandidatenvorrat. Zwei
//! Dinge können sich an einem Prüfpunkt ändern, und beide stehen in ihm:
//!
//! 1. **Die Wahl.** Die abgewandelte Liste wählt die erste Regel, deren Bedingung
//!    gilt (`valid_rules`) und deren Zauber bereit ist (`ready_spells`). Ist das
//!    ein anderer Zauber als in der Basis, weicht der Kampf hier ab. Solange er
//!    nicht abgewichen ist, ist der Zustand derselbe — die Bits gelten also auch
//!    für die Abwandlung.
//! 2. **Der Weckruf danach.** Eine Prioritätenliste plant nach der letzten
//!    Entscheidung eines Zeitpunkts einen Weckruf auf den frühesten Umschlag
//!    **aller** ihrer Regeln, gültig oder nicht ([`crate::apl::Apl::next_wakeup`]).
//!    Eine eingefügte Zeile mit zeitabhängiger Bedingung kann so einen
//!    Entscheidungspunkt schaffen, den die Basis nicht hatte; eine entfernte einen
//!    tilgen. Der Prüfpunkt trägt deshalb, welche Regel vor dem nächsten Ereignis
//!    wecken wollte — weicht der nächste Entscheidungszeitpunkt ab, ist das eine
//!    Abweichung, auch wenn dort am Ende nichts gewirkt würde.
//!
//! Aus 1. folgen die Zeilen der Tabelle in AC2, jede als Sonderfall: Eine
//! eingefügte Zeile wirkt am ersten Punkt, an dem ihre Bedingung gilt, ihr Zauber
//! bereit ist und keine höher stehende Regel greift; eine entfernte am ersten
//! Punkt, an dem sie gewählt wurde; zwei benachbarte getauschte am ersten Punkt,
//! an dem beide gültig und bereit waren. Die allgemeine Form deckt darüber hinaus
//! Tausch über Abstand, gekoppelte Abwandlungen und Zerstören-und-Reparieren ab,
//! ohne eigene Regel je Operator.
//!
//! **Zahlenänderungen gehören nicht hierher** — sie berühren den Kampf an
//! Stellen, die kein Prüfpunkt trägt (Out of scope von #917, der Ledger aus #976).

use std::cell::RefCell;

use crate::apl::{Apl, Rule};
use crate::bitmenge::{RuleBits, SpellBits};
use crate::data::{SPELLS, SPELL_COUNT};
use crate::fight::{CompactFightState, Decide, Fight, FightConfig, SearchFight, Zielwerte};
use crate::time::Millis;

/// Eine Regel im Universum einer Aufzeichnung: zuerst die Basisliste, dann der
/// Vorrat.
pub type RuleId = u16;

/// Was bis zu einem Prüfpunkt zum Ziel beigetragen ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zwischenstand {
    /// Die Kampfzeit des Prüfpunkts.
    pub time: Millis,
    /// Bis hierher angerichteter Schaden.
    pub damage: u32,
    /// Bis hierher eingesteckter Schaden.
    pub damage_taken: u32,
}

/// Ein Prüfpunkt unmittelbar vor einer Entscheidung (Issue #917, AC0).
///
/// `valid_rules` und `ready_spells` sind nicht optional: Ohne sie lässt sich
/// der erste beeinflussbare Punkt nicht finden, und es bliebe nur die
/// Vollsimulation.
#[derive(Debug, Clone)]
pub struct DecisionCheckpoint {
    /// Der Suchzustand vor dieser Entscheidung — hier setzt ein Suffix-Lauf auf.
    pub state: CompactFightState,
    /// Die Regel der Basisliste, die hier gewählt wurde; `None` heißt warten.
    pub selected_rule: Option<RuleId>,
    /// Welche Regeln des Universums hier gültig waren: Bedingung erfüllt, ohne
    /// Rücksicht auf die Bereitschaft ihres Zaubers.
    pub valid_rules: RuleBits,
    /// Welche Zauber hier bereit waren ([`Fight::is_ready`]).
    pub ready_spells: SpellBits,
    /// Wie viele Ereignisse bis hier abgearbeitet waren — so viel Arbeit
    /// wiederholt ein Suffix-Lauf ab hier nicht.
    pub event_cursor: u32,
    /// Was bis hier zum Ziel beigetragen ist.
    pub accumulated_result: Zwischenstand,
    /// Nur an der letzten Entscheidung eines Zeitpunkts: der Weckruf danach.
    weckruf: Option<Weckruf>,
}

/// Was nach der letzten Entscheidung eines Zeitpunkts über den nächsten
/// Entscheidungszeitpunkt bestimmt.
#[derive(Debug, Clone)]
struct Weckruf {
    /// Das nächste eingeplante Ereignis, höchstens das Kampfende. Ein Weckruf
    /// zählt nur, wenn er davor liegt (`Fight::schedule_decision_wakeup`).
    grenze: Millis,
    /// Je Regel, die vor der Grenze wecken will, ihr Zeitpunkt.
    beitraege: Vec<(RuleId, Millis)>,
    /// Der nächste Entscheidungszeitpunkt der Basis.
    basis_naechste: Millis,
}

impl Weckruf {
    /// Der nächste Entscheidungszeitpunkt einer Liste, deren Regeln `drin`
    /// enthält. Das Kampfende steht als `grenze`.
    fn naechste(&self, drin: impl Fn(RuleId) -> bool) -> Millis {
        self.beitraege
            .iter()
            .filter(|(regel, _)| drin(*regel))
            .map(|&(_, zeit)| zeit)
            .min()
            .map_or(self.grenze, |zeit| zeit.min(self.grenze))
    }
}

/// Zeichnet beim Lauf einer Basisliste vor jeder Entscheidung einen Prüfpunkt
/// auf.
///
/// Eine Hülle um die Liste: Sie entscheidet **genau wie die Liste** und plant
/// dieselben Weckrufe — der aufgezeichnete Lauf ist bitgleich mit dem gewöhnlichen
/// (`tests/inkrementell.rs`, `die_aufzeichnung_aendert_den_lauf_nicht`).
struct Aufzeichner<'a> {
    basis: &'a Apl,
    universum: &'a [Rule],
    punkte: RefCell<Vec<DecisionCheckpoint>>,
}

impl Decide for Aufzeichner<'_> {
    fn decide(&self, fight: &Fight) -> Option<usize> {
        let mut valid_rules = RuleBits::mit_breite(self.universum.len());
        for (id, regel) in self.universum.iter().enumerate() {
            if regel.condition.holds(fight) {
                valid_rules.setzen(id);
            }
        }
        let mut ready_spells = SpellBits::leer();
        for zauber in 0..SPELL_COUNT {
            if fight.is_ready(zauber) {
                ready_spells.setzen(zauber);
            }
        }
        let selected = (0..self.basis.rules.len()).find(|&id| {
            valid_rules.enthaelt(id) && ready_spells.enthaelt(self.universum[id].spell)
        });
        let wahl = self.basis.decide(fight);
        // **Die Bits müssen die Wahl tragen**, sonst rechnet die Suche nach dem
        // Startpunkt mit etwas anderem als der Kampf. Kein `debug_assert`: Die
        // Suche läuft im Release-Bau.
        assert_eq!(
            wahl,
            selected.map(|id| self.universum[id].spell),
            "die aufgezeichneten Bits erklären die Wahl der Basisliste nicht (bei {} ms)",
            fight.time()
        );
        self.punkte.borrow_mut().push(DecisionCheckpoint {
            state: fight.such_zustand(),
            selected_rule: selected.map(regel_id),
            valid_rules,
            ready_spells,
            event_cursor: fight.ereignisse(),
            accumulated_result: Zwischenstand {
                time: fight.time(),
                damage: fight.damage(),
                damage_taken: fight.damage_taken(),
            },
            weckruf: None,
        });
        wahl
    }

    fn next_wakeup(&self, fight: &Fight) -> Option<Millis> {
        let jetzt = fight.time();
        let ende = jetzt + fight.fight_remaining_ms();
        let grenze = fight
            .naechstes_ereignis()
            .map_or(ende, |naechstes| naechstes.min(ende));
        let gcd = fight.gcd_ready_at();
        // Der Beitrag einer Regel, wie `Fight::schedule_decision_wakeup` ihn
        // bildet: ihr Umschlag, frühestens am Ende der Wartezeit — und für eine
        // Regel ohne Wartezeit auch darin.
        let mut beitraege = Vec::new();
        for (id, regel) in self.universum.iter().enumerate() {
            let Some(umschlag) = regel.umschlag(fight) else {
                continue;
            };
            let mit_wartezeit = umschlag.max(gcd);
            let beitrag = if gcd > jetzt && !SPELLS[regel.spell].triggers_gcd {
                mit_wartezeit.min(umschlag)
            } else {
                mit_wartezeit
            };
            if beitrag < grenze {
                beitraege.push((regel_id(id), beitrag));
            }
        }
        let basis_laenge = self.basis.rules.len();
        let mut weckruf = Weckruf {
            grenze,
            beitraege,
            basis_naechste: grenze,
        };
        weckruf.basis_naechste = weckruf.naechste(|id| usize::from(id) < basis_laenge);
        let mut punkte = self.punkte.borrow_mut();
        let letzte = punkte
            .last_mut()
            .expect("vor dem Weckruf steht immer eine Entscheidung");
        letzte.weckruf = Some(weckruf);
        self.basis.next_wakeup(fight)
    }

    fn next_wakeup_ohne_gcd(&self, fight: &Fight) -> Option<Millis> {
        self.basis.next_wakeup_ohne_gcd(fight)
    }
}

fn regel_id(index: usize) -> RuleId {
    RuleId::try_from(index).expect("mehr Regeln im Universum, als eine RuleId fasst")
}

/// Wie eine Abwandlung ausgewertet wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auswertung {
    /// Kein Prüfpunkt ist beeinflussbar: Das Ergebnis der Basis gilt
    /// unverändert — der Zweig „ändert nichts“.
    Unveraendert(Zielwerte),
    /// Ab Prüfpunkt `ab` wurde neu gerechnet.
    Suffix {
        /// Der Prüfpunkt, ab dem gerechnet wurde.
        ab: usize,
        /// Wie weit der Kampf dort war.
        praefix: Zwischenstand,
        /// Das Ergebnis des Suffix-Laufs.
        ziel: Zielwerte,
    },
}

impl Auswertung {
    /// Das Ergebnis, gleich auf welchem Weg.
    #[must_use]
    pub const fn ziel(&self) -> Zielwerte {
        match *self {
            Self::Unveraendert(ziel) | Self::Suffix { ziel, .. } => ziel,
        }
    }

    /// Ob der Zweig „ändert nichts“ griff.
    #[must_use]
    pub const fn unveraendert(&self) -> bool {
        matches!(self, Self::Unveraendert(_))
    }
}

/// Der aufgezeichnete Lauf einer Basisliste (Issue #917).
#[derive(Debug, Clone)]
pub struct Basislauf {
    config: FightConfig,
    universum: Vec<Rule>,
    basis_laenge: usize,
    punkte: Vec<DecisionCheckpoint>,
    ereignisse: u32,
    ziel: Zielwerte,
}

impl Basislauf {
    /// Rechnet die Basisliste einmal durch und zeichnet vor jeder Entscheidung
    /// einen Prüfpunkt auf.
    ///
    /// Das Universum sind die Regeln von `basis` und danach die von `vorrat` —
    /// jede Abwandlung, die nachher ausgewertet wird, setzt sich aus ihnen
    /// zusammen. Eine Regel, die hier nicht steht, hat keine Bits und kann nicht
    /// ausgewertet werden.
    #[must_use]
    pub fn aufzeichnen(config: FightConfig, basis: &Apl, vorrat: &[Rule]) -> Self {
        let universum: Vec<Rule> = basis.rules.iter().chain(vorrat).cloned().collect();
        let aufzeichner = Aufzeichner {
            basis,
            universum: &universum,
            punkte: RefCell::new(Vec::new()),
        };
        let mut kampf = SearchFight::new(config);
        let ziel = kampf.run_mit(&aufzeichner);
        let ereignisse = kampf.kampf().ereignisse();
        let punkte = aufzeichner.punkte.into_inner();
        Self {
            config,
            universum,
            basis_laenge: basis.rules.len(),
            punkte,
            ereignisse,
            ziel,
        }
    }

    /// Die Regeln der Basisliste als Universums-Kennungen, in ihrer Reihenfolge.
    #[must_use]
    pub fn basis(&self) -> Vec<RuleId> {
        (0..self.basis_laenge).map(regel_id).collect()
    }

    /// Die Kennung der Vorrats-Regel `index`.
    ///
    /// # Panics
    ///
    /// Wenn der Vorrat keine Regel `index` hat.
    #[must_use]
    pub fn vorrat(&self, index: usize) -> RuleId {
        assert!(
            self.basis_laenge + index < self.universum.len(),
            "Vorrats-Regel {index} gibt es nicht"
        );
        regel_id(self.basis_laenge + index)
    }

    /// Die Prüfpunkte in ihrer Reihenfolge.
    #[must_use]
    pub fn punkte(&self) -> &[DecisionCheckpoint] {
        &self.punkte
    }

    /// Die Regel mit der Kennung `id`.
    ///
    /// # Panics
    ///
    /// Wenn `id` nicht im Universum steht.
    #[must_use]
    pub fn regel(&self, id: RuleId) -> &Rule {
        &self.universum[usize::from(id)]
    }

    /// Welcher Zauber an Prüfpunkt `punkt` gewirkt wurde; `None` heißt warten.
    ///
    /// # Panics
    ///
    /// Wenn es den Prüfpunkt nicht gibt.
    #[must_use]
    pub fn gewaehlter_zauber(&self, punkt: usize) -> Option<usize> {
        self.punkte[punkt]
            .selected_rule
            .map(|id| self.regel(id).spell)
    }

    /// Das Ergebnis der Basisliste.
    #[must_use]
    pub const fn ziel(&self) -> Zielwerte {
        self.ziel
    }

    /// Wie viele Ereignisse der ganze Basislauf abgearbeitet hat.
    #[must_use]
    pub const fn ereignisse(&self) -> u32 {
        self.ereignisse
    }

    /// Ein Suchkampf mit der Konfiguration dieser Aufzeichnung — der Kampf, in
    /// den [`Basislauf::auswerten`] einen Prüfpunkt lädt.
    #[must_use]
    pub fn suchkampf(&self) -> SearchFight {
        SearchFight::new(self.config)
    }

    /// Die Liste aus Universums-Kennungen als Prioritätenliste.
    #[must_use]
    pub fn apl(&self, liste: &[RuleId]) -> Apl {
        Apl {
            name: "abwandlung".to_owned(),
            rules: liste
                .iter()
                .map(|&id| self.universum[usize::from(id)].clone())
                .collect(),
        }
    }

    /// Der erste Prüfpunkt, an dem die Liste `liste` vom Basislauf abweichen
    /// kann — oder `None`, wenn es keinen gibt.
    ///
    /// Siehe den Modulkopf für die Rechnung. Gefunden wird der Punkt, an dem
    /// die Abwandlung **anders wählt** oder nach dem sie **zu einer anderen Zeit
    /// erneut entscheidet**; bis dorthin ist ihr Kampf derselbe.
    ///
    /// # Panics
    ///
    /// Wenn `liste` eine Kennung enthält, die nicht im Universum steht.
    #[must_use]
    pub fn startpunkt(&self, liste: &[RuleId]) -> Option<usize> {
        let mut drin = RuleBits::mit_breite(self.universum.len());
        for &id in liste {
            drin.setzen(usize::from(id));
        }
        for (index, punkt) in self.punkte.iter().enumerate() {
            let wahl = liste
                .iter()
                .map(|&id| usize::from(id))
                .find(|&id| {
                    punkt.valid_rules.enthaelt(id)
                        && punkt.ready_spells.enthaelt(self.universum[id].spell)
                })
                .map(|id| self.universum[id].spell);
            let basiswahl = punkt
                .selected_rule
                .map(|id| self.universum[usize::from(id)].spell);
            if wahl != basiswahl {
                return Some(index);
            }
            if let Some(weckruf) = &punkt.weckruf {
                if weckruf.naechste(|id| drin.enthaelt(usize::from(id))) != weckruf.basis_naechste {
                    return Some(index);
                }
            }
        }
        None
    }

    /// Wertet die Liste `liste` aus: unverändert, wenn kein Prüfpunkt
    /// beeinflussbar ist, sonst als Suffix-Lauf ab dem ersten.
    ///
    /// `kampf` muss aus derselben Konfiguration stammen
    /// ([`Basislauf::suchkampf`]); er wird nur für den Suffix-Lauf gebraucht.
    ///
    /// # Panics
    ///
    /// Wenn `kampf` eine andere Konfiguration hat, oder wie
    /// [`Basislauf::startpunkt`].
    pub fn auswerten(&self, liste: &[RuleId], kampf: &mut SearchFight) -> Auswertung {
        match self.startpunkt(liste) {
            None => Auswertung::Unveraendert(self.ziel),
            Some(ab) => {
                let punkt = &self.punkte[ab];
                let ziel = kampf.fortsetzen(&punkt.state, &self.apl(liste));
                Auswertung::Suffix {
                    ab,
                    praefix: punkt.accumulated_result,
                    ziel,
                }
            }
        }
    }
}
