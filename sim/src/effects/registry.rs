//! Die Registry der Pflichtanalysen — erzeugt, nicht gezaehlt.
//!
//! # Warum hier keine Zahl steht
//!
//! Eine Konstante wie `ANALYSEN: usize = 8` altert bei jeder Ergaenzung, und
//! zwar **still**: Wer eine neunte Analyse hinzufuegt und die Zahl vergisst,
//! bekommt eine Pruefung, die weiterhin acht bestaetigt. Genau diese Sorte
//! Drift ist der Anlass fuer Issue #911.
//!
//! Deshalb entsteht die Liste aus dem Makro [`registry!`], und ihre Laenge ist
//! [`REGISTRY`]`.len()` — ein abgeleiteter Wert, kein gepflegter.
//!
//! Die Pruefungen lauten entsprechend „**jede registrierte** Analyse", nie
//! „alle acht". Die Zahl gehoert in die **Meldung**, damit ein Befund lesbar
//! ist, und nie in die **Bedingung**, damit er nicht altert.

use super::{Bounds, RandomnessSemantics};
use super::{ConservativeFallback, EffectNode, LocalMonotonicityHint, StateMask};
use core::fmt;

/// Der Name einer Analyse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPassId(pub &'static str);

impl fmt::Display for AnalysisPassId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Wie eine Analyse ueber einen Knoten entscheidet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Die Optimierung ist erlaubt.
    Permits,
    /// Die Optimierung ist ausgeschlossen — aus einer **bekannten** Eigenschaft.
    Forbids,
    /// Der konservative Rueckfall greift, weil eine Angabe fehlt.
    ///
    /// Der Unterschied zu [`Verdict::Forbids`] ist wichtig: Dort weiss man,
    /// dass es nicht geht. Hier weiss man **nichts**, und deshalb geht es
    /// ebenfalls nicht. Beides fuehrt zum langsamen Pfad, aber nur eines davon
    /// ist ein Hinweis auf eine Luecke in den Daten.
    Conservative,
}

/// Was eine Analyse ueber den Knoten herausgefunden hat.
///
/// Ohne diesen Anteil waere das Ergebnis ein blosses Ja/Nein, und die Analysen
/// waeren nicht unterscheidbar — der Fehler, gegen den dieses Modul gebaut ist,
/// nur eine Ebene hoeher.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Evidence {
    /// Keine ueber das Verdikt hinausgehende Angabe.
    Keine,
    /// Welche Ereigniskanaele der Effekt braucht.
    Kanaele(StateMask),
    /// Welche Zufallsquelle beteiligt ist.
    Zufall(RandomnessSemantics),
    /// Die Schranke, soweit bekannt.
    Schranke(Bounds),
    /// Der Monotonie-Hinweis, ausdruecklich ohne Zertifikats-Anspruch.
    Monotonie(LocalMonotonicityHint),
    /// Die Obergrenze fuer die Zahl moeglicher Aktionen.
    Aktionsgrenze(Option<u32>),
}

/// Das Ergebnis einer Analyse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalysisResult {
    /// Welche Analyse geantwortet hat.
    pub pass: AnalysisPassId,
    /// Das Verdikt.
    pub verdict: Verdict,
    /// Welche konservativen Zustaende dadurch greifen.
    pub fallback: ConservativeFallback,
    /// Was herausgefunden wurde.
    pub evidence: Evidence,
    /// Warum — im Klartext, fuer die Meldung.
    pub reason: &'static str,
}

impl AnalysisResult {
    /// Der Rueckfall: alle sechs konservativen Zustaende greifen.
    pub const fn konservativ(
        pass: AnalysisPassId,
        evidence: Evidence,
        reason: &'static str,
    ) -> AnalysisResult {
        AnalysisResult {
            pass,
            verdict: Verdict::Conservative,
            fallback: ConservativeFallback::ALLE,
            evidence,
            reason,
        }
    }

    /// Die Optimierung ist erlaubt.
    pub const fn erlaubt(
        pass: AnalysisPassId,
        evidence: Evidence,
        reason: &'static str,
    ) -> AnalysisResult {
        AnalysisResult {
            pass,
            verdict: Verdict::Permits,
            fallback: ConservativeFallback::KEINER,
            evidence,
            reason,
        }
    }

    /// Ausgeschlossen aus bekannter Eigenschaft — nicht aus Unwissen.
    pub const fn verboten(
        pass: AnalysisPassId,
        evidence: Evidence,
        reason: &'static str,
    ) -> AnalysisResult {
        AnalysisResult {
            pass,
            verdict: Verdict::Forbids,
            fallback: ConservativeFallback::ALLE,
            evidence,
            reason,
        }
    }

    /// Ob der langsame Pfad gilt — aus welchem der beiden Gruende auch immer.
    pub const fn langsam(&self) -> bool {
        !matches!(self.verdict, Verdict::Permits)
    }
}

/// Eine Pflichtanalyse ueber der Wirkungsbeschreibung.
///
/// # Die Zusicherung haengt an der Umsetzung, nicht am Trait
///
/// Jede Umsetzung von [`EffectAnalysisPass::analyze`] behandelt
/// [`super::EffectOp`] mit einem vollstaendigen `match` **ohne Auffangzweig**.
/// Ein neuer Baustein laesst deshalb jede einzelne Analyse nicht uebersetzen,
/// bis er eingeordnet ist — das ist der Mechanismus hinter AC2, und er ist ein
/// Uebersetzungsfehler, kein Testfehler.
pub trait EffectAnalysisPass {
    /// Der Name dieser Analyse.
    const ID: AnalysisPassId;

    /// Was diese Analyse ueber den Knoten sagt.
    fn analyze(&self, node: &EffectNode) -> AnalysisResult;
}

/// Der objektsichere Anteil von [`EffectAnalysisPass`].
///
/// Notwendig, weil ein Trait mit assoziierter Konstante nicht objektsicher ist
/// — die Registry braucht aber eine Liste ueber verschiedene Analysen hinweg.
/// Die Umsetzung ist eine Deckenimplementierung: Wer [`EffectAnalysisPass`]
/// erfuellt, ist automatisch hier drin und kann nicht vergessen werden.
pub trait DynAnalysisPass: Sync {
    /// Der Name dieser Analyse.
    fn id(&self) -> AnalysisPassId;
    /// Was diese Analyse ueber den Knoten sagt.
    fn analyze_dyn(&self, node: &EffectNode) -> AnalysisResult;
}

impl<T: EffectAnalysisPass + Sync> DynAnalysisPass for T {
    fn id(&self) -> AnalysisPassId {
        T::ID
    }

    fn analyze_dyn(&self, node: &EffectNode) -> AnalysisResult {
        self.analyze(node)
    }
}

/// Erzeugt die Registry aus einer Aufzaehlung von Analysen.
///
/// Der einzige Ort, an dem die Liste steht. Ihre **Laenge** entsteht daraus und
/// wird nirgends von Hand gefuehrt.
#[macro_export]
macro_rules! registry {
    ($($pass:path),* $(,)?) => {
        /// Alle Pflichtanalysen.
        ///
        /// Erzeugt vom Makro `registry!`. Die Zahl der Eintraege ist
        /// `REGISTRY.len()` und steht bewusst an keiner Stelle als Konstante.
        pub static REGISTRY: &[&dyn $crate::effects::registry::DynAnalysisPass] =
            &[$(&$pass),*];
    };
}

/// Laesst **jede registrierte** Analyse ueber einen Knoten urteilen.
///
/// Die Schleife laeuft ueber [`REGISTRY`](super::passes::REGISTRY) und nicht
/// ueber eine gezaehlte Liste — kommt eine Analyse dazu, ist sie ohne weiteres
/// Zutun dabei.
pub fn alle_analysen(node: &EffectNode) -> Vec<AnalysisResult> {
    super::passes::REGISTRY
        .iter()
        .map(|pass| pass.analyze_dyn(node))
        .collect()
}

/// Ob **jede** registrierte Analyse in den konservativen Rueckfall geht.
///
/// Das ist die Pruefform aus AC3: nicht „alle acht", sondern jede registrierte.
/// Die Zahl wandert in die Meldung des Aufrufers, nicht in diese Bedingung.
pub fn alle_konservativ(node: &EffectNode) -> bool {
    let ergebnisse = alle_analysen(node);
    !ergebnisse.is_empty()
        && ergebnisse
            .iter()
            .all(|r| matches!(r.verdict, Verdict::Conservative))
}
