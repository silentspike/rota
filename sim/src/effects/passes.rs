//! Die acht Pflichtanalysen ueber der Wirkungsbeschreibung.
//!
//! # Was sie gemeinsam haben
//!
//! Jede beantwortet **eine** der acht Fragen aus Issue #911, und jede tut das
//! mit einem vollstaendigen `match` ueber [`EffectOp`] **ohne Auffangzweig**.
//! Daran haengt die einzige harte Zusicherung dieses Moduls: Ein neuer
//! Baustein laesst jede dieser Dateien nicht mehr uebersetzen, bis er ueberall
//! eingeordnet ist.
//!
//! # Die Reihenfolge der Pruefung ist nicht beliebig
//!
//! Jeder Pass fragt **zuerst** nach einer Luecke und geht dann in den
//! Rueckfall. Erst danach urteilt er ueber den Baustein. Andersherum waere ein
//! Knoten ohne Semantik im guenstigen Fall schnell — und „unbekannt" waere
//! damit schneller als „bekannt", also genau falsch herum.
//!
//! # Drei der acht sind BESCHREIBEND, nicht entscheidend
//!
//! [`InvalidationPass`], [`EventChannelPass`] und [`RandomSourcePass`]
//! beantworten keine Ja/Nein-Frage, sondern eine nach dem **Was**: welche
//! Ergebnisse ungueltig werden, welche Kanaele der Kampf braucht, welche
//! Zufallsquelle aktiv ist. Ihr Verdikt ist deshalb ueber alle vollstaendig
//! beschriebenen Knoten hinweg konstant [`Permits`](super::registry::Verdict);
//! ihre eigentliche Aussage steht in der [`Evidence`].
//!
//! **Wer sie am Verdikt misst, misst neben der Wirkung vorbei.** Genau das ist
//! beim Bau passiert: Ein Prueffall, der „unterscheidet jede Analyse etwas?"
//! ueber das Verdikt fragte, meldete diese drei als stumpf — sie waren es
//! nicht. Der Fall vergleicht jetzt das vollstaendige Ergebnis, und die
//! Unterscheidung liegt dort, wo sie hingehoert.
//!
//! Fuer die uebrigen fuenf traegt das Verdikt die Antwort, und der Unterschied
//! zwischen `Forbids` und `Conservative` ist dort wesentlich: bekannt
//! ausgeschlossen gegen unbekannt und deshalb ausgeschlossen. Siehe
//! [`Verdict`](super::registry::Verdict).

use super::registry::{AnalysisPassId, AnalysisResult, EffectAnalysisPass, Evidence};
use super::{
    Bounds, EffectNode, EffectOp, MonotonicityDomain, RandomnessSemantics, StateMask,
    TimingSemantics,
};

// Die Registry. Sie steht am Anfang der Datei, damit die Liste beim Lesen vor
// den Umsetzungen kommt — und sie ist der einzige Ort, an dem die Analysen
// aufgezaehlt werden. Ihre Laenge ist `REGISTRY.len()`, nirgends eine Zahl.
crate::registry! {
    AnalyticCritPass,
    ActionBoundPass,
    KernelChoicePass,
    CacheReusePass,
    InvalidationPass,
    EventChannelPass,
    RandomSourcePass,
    MonotonicityPass,
}

/// Hilfsgrund fuer den Rueckfall, damit die Meldung ueberall gleich lautet.
const LUECKE: &str =
    "Angabe fehlt — konservativ: alle Abhaengigkeiten, kein Ledger, keine Marginalisierung, \
     kein Dominanz-Pruning, keine enge Schranke, Rueckfall-Kernel";

// ------------------------------------------------------- 1. Krit analytisch

/// Darf der kritische Treffer analytisch statt gewuerfelt behandelt werden?
///
/// Analytisch heisst: mit dem Erwartungswert rechnen statt zu ziehen. Das ist
/// nur erlaubt, wenn am Ergebnis **keine Verzweigung** haengt — sonst ist der
/// Erwartungswert des Ergebnisses nicht das Ergebnis des Erwartungswerts.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalyticCritPass;

impl EffectAnalysisPass for AnalyticCritPass {
    const ID: AnalysisPassId = AnalysisPassId("analytic-crit");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(Self::ID, Evidence::Keine, LUECKE);
        }
        if node.control_flow_effect.verzweigt() {
            return AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "am Ergebnis haengt eine Verzweigung — der Erwartungswert traegt hier nicht",
            );
        }
        match node.op {
            EffectOp::DealDamage | EffectOp::ApplyDot => {
                if node.randomness.ist_deterministisch() {
                    AnalysisResult::erlaubt(
                        Self::ID,
                        Evidence::Zufall(node.randomness),
                        "kein Zufall beteiligt — es gibt gar nichts zu marginalisieren",
                    )
                } else {
                    AnalysisResult::erlaubt(
                        Self::ID,
                        Evidence::Zufall(node.randomness),
                        "Schaden ohne Verzweigung — der Krit darf analytisch behandelt werden",
                    )
                }
            }
            EffectOp::Heal | EffectOp::Shield | EffectOp::ConvertDamage => {
                AnalysisResult::verboten(
                    Self::ID,
                    Evidence::Zufall(node.randomness),
                    "Erhalt des Spielers entscheidet ueber Abbruch — nicht marginalisierbar",
                )
            }
            EffectOp::ApplyBuff
            | EffectOp::ModifyResource
            | EffectOp::ReduceCooldown
            | EffectOp::ResetCooldown
            | EffectOp::FreeNextGcd => AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "veraendert, WELCHE Aktionen folgen — der Trace haengt daran",
            ),
            EffectOp::ChangeBossState | EffectOp::ChangePhase => AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "Bosszustand verzweigt den Kampf",
            ),
            EffectOp::CustomSignature(_) => AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "einmalige Bossregel — analytisch nur mit eigenem Nachweis",
            ),
        }
    }
}

// -------------------------------------------------- 2. Obergrenze Aktionen

/// Welche Obergrenze gilt fuer die Zahl moeglicher Aktionen?
///
/// `None` heisst **unbekannt**, nicht „unbegrenzt". Ein Effekt, der Wartezeiten
/// verkuerzt oder Wuerfe verdoppelt, hebt jede vorher gerechnete Grenze auf —
/// und wer das uebersieht, dimensioniert Puffer zu klein.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionBoundPass;

impl EffectAnalysisPass for ActionBoundPass {
    const ID: AnalysisPassId = AnalysisPassId("action-bound");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(Self::ID, Evidence::Aktionsgrenze(None), LUECKE);
        }
        match node.op {
            EffectOp::ReduceCooldown | EffectOp::ResetCooldown | EffectOp::FreeNextGcd => {
                // Diese drei erzeugen zusaetzliche Aktionen im selben Zeitraum.
                // Ohne enge Schranke ist die Zahl der Aktionen unbeschraenkt.
                if node.numeric_bounds.ist_eng() {
                    AnalysisResult::erlaubt(
                        Self::ID,
                        Evidence::Schranke(node.numeric_bounds),
                        "erzeugt Zusatz-Aktionen, aber mit enger Schranke",
                    )
                } else {
                    AnalysisResult::konservativ(
                        Self::ID,
                        Evidence::Aktionsgrenze(None),
                        "erzeugt Zusatz-Aktionen ohne enge Schranke — keine Obergrenze ableitbar",
                    )
                }
            }
            EffectOp::DealDamage
            | EffectOp::ApplyDot
            | EffectOp::ApplyBuff
            | EffectOp::ModifyResource
            | EffectOp::Heal
            | EffectOp::Shield
            | EffectOp::ConvertDamage => AnalysisResult::erlaubt(
                Self::ID,
                Evidence::Schranke(node.numeric_bounds),
                "veraendert die Zahl moeglicher Aktionen nicht",
            ),
            EffectOp::ChangeBossState | EffectOp::ChangePhase => AnalysisResult::konservativ(
                Self::ID,
                Evidence::Aktionsgrenze(None),
                "Phasenwechsel kann die Kampfdauer aendern — Grenze nicht ableitbar",
            ),
            EffectOp::CustomSignature(_) => AnalysisResult::konservativ(
                Self::ID,
                Evidence::Aktionsgrenze(None),
                "einmalige Bossregel — Obergrenze nur mit eigenem Nachweis",
            ),
        }
    }
}

// ------------------------------------------------------------ 3. Kernelwahl

/// Welcher spezialisierte Kernel passt?
///
/// Ein spezialisierter Kernel setzt voraus, dass der Effekt ueber viele
/// Durchgaenge **gleich** laeuft. Alles, was am laufenden Zustand haengt oder
/// verzweigt, faellt auf den allgemeinen Kernel zurueck.
#[derive(Debug, Clone, Copy, Default)]
pub struct KernelChoicePass;

impl EffectAnalysisPass for KernelChoicePass {
    const ID: AnalysisPassId = AnalysisPassId("kernel-choice");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(Self::ID, Evidence::Keine, LUECKE);
        }
        if matches!(node.timing, TimingSemantics::Live) {
            return AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "liest bei jedem Tick neu — kein Kernel mit festgehaltenen Werten",
            );
        }
        match node.op {
            EffectOp::DealDamage => AnalysisResult::erlaubt(
                Self::ID,
                Evidence::Schranke(node.numeric_bounds),
                "gleichfoermiger Schaden — spezialisierter Kernel moeglich",
            ),
            EffectOp::ApplyDot | EffectOp::ApplyBuff | EffectOp::Shield => AnalysisResult::erlaubt(
                Self::ID,
                Evidence::Keine,
                "festgehaltene Werte — Tick-Kernel moeglich",
            ),
            EffectOp::ModifyResource | EffectOp::Heal => AnalysisResult::erlaubt(
                Self::ID,
                Evidence::Schranke(node.numeric_bounds),
                "einfache Zustandsaenderung",
            ),
            EffectOp::ReduceCooldown
            | EffectOp::ResetCooldown
            | EffectOp::FreeNextGcd
            | EffectOp::ConvertDamage => AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "greift in die Ablaufsteuerung — allgemeiner Kernel",
            ),
            EffectOp::ChangeBossState | EffectOp::ChangePhase => AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "Bosszustand — allgemeiner Kernel",
            ),
            EffectOp::CustomSignature(_) => AnalysisResult::konservativ(
                Self::ID,
                Evidence::Keine,
                "einmalige Bossregel — allgemeiner Rueckfall-Kernel",
            ),
        }
    }
}

// ------------------------------------------------- 4. Rechenweg wiederverwenden

/// Darf ein zwischengespeicherter Rechenweg wiederverwendet werden?
///
/// Nur, wenn der Effekt seine Eingangswerte **festhaelt**. Ein lebender Wert
/// macht jeden gespeicherten Weg wertlos, und der Fehler waere still: Das
/// Ergebnis sieht plausibel aus und gehoert zu einem anderen Zustand.
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheReusePass;

impl EffectAnalysisPass for CacheReusePass {
    const ID: AnalysisPassId = AnalysisPassId("cache-reuse");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(Self::ID, Evidence::Keine, LUECKE);
        }
        let festgehalten = matches!(
            node.timing,
            TimingSemantics::Snapshot | TimingSemantics::Immediate
        );
        match node.op {
            EffectOp::DealDamage | EffectOp::Heal | EffectOp::Shield => {
                if festgehalten {
                    AnalysisResult::erlaubt(
                        Self::ID,
                        Evidence::Keine,
                        "Momentaufnahme — der Rechenweg gilt weiter",
                    )
                } else {
                    AnalysisResult::verboten(
                        Self::ID,
                        Evidence::Keine,
                        "lebender Wert — jeder gespeicherte Weg waere veraltet",
                    )
                }
            }
            EffectOp::ApplyDot | EffectOp::ApplyBuff => {
                if festgehalten {
                    AnalysisResult::erlaubt(
                        Self::ID,
                        Evidence::Keine,
                        "beim Auflegen festgehalten — wiederverwendbar",
                    )
                } else {
                    AnalysisResult::verboten(
                        Self::ID,
                        Evidence::Keine,
                        "liest bei jedem Tick neu — nicht wiederverwendbar",
                    )
                }
            }
            EffectOp::ModifyResource
            | EffectOp::ReduceCooldown
            | EffectOp::ResetCooldown
            | EffectOp::FreeNextGcd
            | EffectOp::ConvertDamage => AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "aendert den Zustand, auf dem spaetere Wege beruhen",
            ),
            EffectOp::ChangeBossState | EffectOp::ChangePhase => AnalysisResult::verboten(
                Self::ID,
                Evidence::Keine,
                "Bosszustand — spaetere Wege beruhen darauf",
            ),
            EffectOp::CustomSignature(_) => AnalysisResult::konservativ(
                Self::ID,
                Evidence::Keine,
                "einmalige Bossregel — kein Ledger, keine Wiederverwendung",
            ),
        }
    }
}

// ----------------------------------------------------------- 5. Invalidierung

/// Welche Ergebnisse werden ungueltig, wenn eine Zahl sich aendert?
///
/// Die Antwort ist die Schreibmaske des Effekts — **erzeugt** aus den Daten und
/// nicht aus einer gepflegten Zuordnung. Genau hier war der Drift am
/// gefaehrlichsten: Eine vergessene Zeile laesst ein Ergebnis stehen, das nicht
/// mehr gilt.
#[derive(Debug, Clone, Copy, Default)]
pub struct InvalidationPass;

impl EffectAnalysisPass for InvalidationPass {
    const ID: AnalysisPassId = AnalysisPassId("invalidation");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(
                Self::ID,
                Evidence::Kanaele(StateMask::UNKNOWN),
                LUECKE,
            );
        }
        // Der Rumpf ist fuer alle Bausteine derselbe — aber er wird
        // **aufgezaehlt** und nicht mit einem Auffangzweig erschlagen. Ein
        // neuer Baustein muss auch hier eine Entscheidung bekommen, und sei sie
        // dieselbe wie nebenan.
        let masken = match node.op {
            EffectOp::DealDamage
            | EffectOp::ApplyDot
            | EffectOp::ApplyBuff
            | EffectOp::ModifyResource
            | EffectOp::ReduceCooldown
            | EffectOp::ResetCooldown
            | EffectOp::FreeNextGcd
            | EffectOp::Heal
            | EffectOp::Shield
            | EffectOp::ConvertDamage
            | EffectOp::ChangeBossState
            | EffectOp::ChangePhase => node.writes,
            EffectOp::CustomSignature(_) => {
                // Auch eine Signatur MIT Semantik schreibt nur, was sie angibt.
                // Ohne Semantik faengt `hat_luecke` sie oben ab.
                node.writes
            }
        };
        if masken.is_empty() {
            AnalysisResult::erlaubt(
                Self::ID,
                Evidence::Kanaele(masken),
                "schreibt nichts — macht keine Ergebnisse ungueltig",
            )
        } else {
            AnalysisResult::erlaubt(
                Self::ID,
                Evidence::Kanaele(masken),
                "die Schreibmaske ist die Invalidierungsmenge",
            )
        }
    }
}

// -------------------------------------------------------- 6. Ereigniskanaele

/// Welche Ereigniskanaele braucht der Kampf?
///
/// Die Vereinigung aus Lese- und Schreibmaske. Ein Kanal, den kein Effekt
/// beruehrt, muss nicht bedient werden — aber die Entscheidung darueber faellt
/// aus den Knoten und nicht aus einer Liste.
#[derive(Debug, Clone, Copy, Default)]
pub struct EventChannelPass;

impl EffectAnalysisPass for EventChannelPass {
    const ID: AnalysisPassId = AnalysisPassId("event-channels");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(
                Self::ID,
                Evidence::Kanaele(StateMask::UNKNOWN),
                LUECKE,
            );
        }
        let kanaele = match node.op {
            EffectOp::DealDamage => node.reads.union(node.writes).union(StateMask::BOSS_HP),
            EffectOp::ApplyDot => node.reads.union(node.writes).union(StateMask::DOTS),
            EffectOp::ApplyBuff => node.reads.union(node.writes).union(StateMask::BUFFS),
            EffectOp::ModifyResource => node.reads.union(node.writes).union(StateMask::RESOURCE),
            EffectOp::ReduceCooldown | EffectOp::ResetCooldown => {
                node.reads.union(node.writes).union(StateMask::COOLDOWNS)
            }
            EffectOp::FreeNextGcd => node.reads.union(node.writes).union(StateMask::GCD),
            EffectOp::Heal => node.reads.union(node.writes).union(StateMask::PLAYER_HP),
            EffectOp::Shield => node.reads.union(node.writes).union(StateMask::SHIELD),
            EffectOp::ConvertDamage => node
                .reads
                .union(node.writes)
                .union(StateMask::SHIELD)
                .union(StateMask::PLAYER_HP),
            EffectOp::ChangeBossState => {
                node.reads.union(node.writes).union(StateMask::BOSS_SYSTEMS)
            }
            EffectOp::ChangePhase => node.reads.union(node.writes).union(StateMask::PHASE),
            EffectOp::CustomSignature(_) => node.reads.union(node.writes),
        };
        AnalysisResult::erlaubt(
            Self::ID,
            Evidence::Kanaele(kanaele),
            "die Kanaele folgen aus Lese- und Schreibmaske des Bausteins",
        )
    }
}

// --------------------------------------------------------- 7. Zufallsquellen

/// Welche Zufallsquellen sind ueberhaupt aktiv?
///
/// Wichtig fuer die Zusicherung, dass ein Lauf ohne Krit **null** Ziehungen
/// macht. Diese Zahl steht in der Doku des Simulators und waere ohne Ableitung
/// eine Behauptung.
#[derive(Debug, Clone, Copy, Default)]
pub struct RandomSourcePass;

impl EffectAnalysisPass for RandomSourcePass {
    const ID: AnalysisPassId = AnalysisPassId("random-sources");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(
                Self::ID,
                Evidence::Zufall(RandomnessSemantics::Unknown),
                LUECKE,
            );
        }
        // Die Quelle steht am Knoten. Der `match` entscheidet, ob der Baustein
        // ueberhaupt eine haben DARF — ein Baustein, der keine haben kann, mit
        // gesetzter Quelle waere ein Datenfehler.
        let darf_zufall = match node.op {
            EffectOp::DealDamage | EffectOp::ApplyDot | EffectOp::Heal => true,
            EffectOp::ApplyBuff
            | EffectOp::ModifyResource
            | EffectOp::ReduceCooldown
            | EffectOp::ResetCooldown
            | EffectOp::FreeNextGcd
            | EffectOp::Shield
            | EffectOp::ConvertDamage => false,
            EffectOp::ChangeBossState | EffectOp::ChangePhase => true,
            EffectOp::CustomSignature(_) => true,
        };
        if !darf_zufall && !node.randomness.ist_deterministisch() {
            return AnalysisResult::konservativ(
                Self::ID,
                Evidence::Zufall(node.randomness),
                "Baustein kann keinen Zufall tragen, traegt aber eine Quelle — Datenfehler",
            );
        }
        AnalysisResult::erlaubt(
            Self::ID,
            Evidence::Zufall(node.randomness),
            "die aktive Quelle steht am Knoten",
        )
    }
}

// -------------------------------------------------------------- 8. Monotonie

/// Ist die Wirkung monoton — mehr Wert, besseres Ergebnis?
///
/// # Diese Analyse liefert einen Hinweis, kein Zertifikat
///
/// Sie antwortet **nur** fuer [`MonotonicityDomain::ExpectedDamageStableTrace`]
/// und ausdruecklich nicht fuer den Sieg: An einem Boss, bei dem ein Ring auf
/// null die Niederlage ist, kann mehr Schaden schlechter sein. Das Zertifikat
/// entsteht erst aus Wirkungsbeschreibung, Bosssemantik, Zielfunktion und
/// Trace-Region zusammen — und damit nicht hier.
#[derive(Debug, Clone, Copy, Default)]
pub struct MonotonicityPass;

impl EffectAnalysisPass for MonotonicityPass {
    const ID: AnalysisPassId = AnalysisPassId("monotonicity");

    fn analyze(&self, node: &EffectNode) -> AnalysisResult {
        if node.hat_luecke() {
            return AnalysisResult::konservativ(
                Self::ID,
                Evidence::Monotonie(super::LocalMonotonicityHint::UNKNOWN),
                LUECKE,
            );
        }
        // Sobald der Sieg im Spiel ist, sagt der Hinweis nichts mehr — auch
        // dann nicht, wenn er fuer den stabilen Trace gesetzt ist.
        if node.control_flow_effect.branches_on_victory {
            return AnalysisResult::konservativ(
                Self::ID,
                Evidence::Monotonie(node.monotonicity),
                "am Sieg haengt eine Verzweigung — der Hinweis traegt dafuer nicht",
            );
        }
        let traegt = node
            .monotonicity
            .traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace);
        match node.op {
            EffectOp::DealDamage | EffectOp::ApplyDot => {
                if traegt {
                    AnalysisResult::erlaubt(
                        Self::ID,
                        Evidence::Monotonie(node.monotonicity),
                        "Hinweis fuer erwarteten Schaden bei stabilem Trace — kein Zertifikat",
                    )
                } else {
                    AnalysisResult::konservativ(
                        Self::ID,
                        Evidence::Monotonie(node.monotonicity),
                        "kein Hinweis fuer diesen Bezug",
                    )
                }
            }
            EffectOp::ModifyResource | EffectOp::ReduceCooldown | EffectOp::ResetCooldown => {
                if traegt {
                    AnalysisResult::erlaubt(
                        Self::ID,
                        Evidence::Monotonie(node.monotonicity),
                        "mehr Ressource oder kuerzere Wartezeit ordnet den Trace nicht um",
                    )
                } else {
                    AnalysisResult::konservativ(
                        Self::ID,
                        Evidence::Monotonie(node.monotonicity),
                        "kein Hinweis fuer diesen Bezug",
                    )
                }
            }
            EffectOp::ApplyBuff | EffectOp::FreeNextGcd | EffectOp::Heal | EffectOp::Shield => {
                AnalysisResult::konservativ(
                    Self::ID,
                    Evidence::Monotonie(node.monotonicity),
                    "wirkt ueber den Trace, nicht ueber den Wert — Hinweis reicht nicht",
                )
            }
            EffectOp::ConvertDamage => AnalysisResult::verboten(
                Self::ID,
                Evidence::Monotonie(node.monotonicity),
                "wandelt Schaden um — mehr kann weniger sein",
            ),
            EffectOp::ChangeBossState | EffectOp::ChangePhase => AnalysisResult::verboten(
                Self::ID,
                Evidence::Monotonie(node.monotonicity),
                "Phasen und Bosszustaende sind nicht monoton",
            ),
            EffectOp::CustomSignature(_) => AnalysisResult::konservativ(
                Self::ID,
                Evidence::Monotonie(node.monotonicity),
                "einmalige Bossregel — Monotonie nur mit eigenem Nachweis",
            ),
        }
    }
}

/// Wie viele Analysen registriert sind.
///
/// **Abgeleitet, nicht gepflegt.** Diese Funktion existiert, damit die Zahl in
/// Meldungen stehen kann, ohne dass irgendwo eine Konstante altert. Sie gehoert
/// in die Meldung, nie in die Bedingung.
pub fn anzahl_analysen() -> usize {
    REGISTRY.len()
}

/// Die Namen aller registrierten Analysen, in Registrierungsreihenfolge.
pub fn namen() -> Vec<AnalysisPassId> {
    REGISTRY.iter().map(|p| p.id()).collect()
}

/// Ein Knoten, der alles ueber sich sagt — fuer Pruefstaende und als Vorlage.
///
/// Bewusst hier und nicht im Prueffall: Wer einen neuen Baustein einordnet,
/// braucht genau einen vollstaendig ausgefuellten Knoten zum Danebenlegen.
pub fn beispielknoten(op: EffectOp) -> EffectNode {
    EffectNode {
        op,
        trigger: super::Trigger::OnCast,
        target: super::Target::Boss,
        reads: StateMask::RESOURCE,
        writes: StateMask::BOSS_HP,
        timing: TimingSemantics::Immediate,
        stacking: super::StackingSemantics::None,
        randomness: RandomnessSemantics::Deterministic,
        monotonicity: super::LocalMonotonicityHint {
            direction: super::MonotonicDirection::Increasing,
            domain: MonotonicityDomain::ExpectedDamageStableTrace,
        },
        numeric_bounds: Bounds::exact(0, 1_000_000),
        control_flow_effect: super::ControlFlowEffect::NONE,
        origin: "beispiel",
    }
}
