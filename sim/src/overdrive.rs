//! Overdrive — der Tempo-Schub auf eigener Taste (#1177, Abschnitt 2).
//!
//! Das Gegenstück zu `src/combat/overdrive.ts`, Zeile für Zeile dieselbe Regel.
//!
//! > **Overdrive** (statt „Bloodlust"): außerhalb der Schulen, belegt keinen
//! > Leisten-Platz, eigene Taste **G**, frei zündbar, einmal pro Kampf, kurzer
//! > starker Tempo-Schub. […] der verlässliche Angelpunkt fürs Zusammenlegen
//! > der großen Fähigkeiten.
//! >
//! > — Masterplan A.4, Zeile „Overdrive + Überdruck"
//!
//! # Was „Tempo" hier heißt
//!
//! CATALYSIS. GAME-DESIGN 6.2 sagt es wörtlich: CATALYSIS „wächst dauerhaft
//! über Talente und Ausrüstung, kurzzeitig über Overdrive". Overdrive geht
//! deshalb durch **dieselbe** Stelle wie getragenes Tempo
//! (`Fight::hasted`) und wirkt dort, wo Tempo wirkt — auf Abklingzeiten,
//! Wirkzeiten, Kanalisierungen und DoT-Abstände. **Nicht auf die globale
//! Wartezeit:** Die sinkt seit dem 2026-07-29 ausschließlich über den
//! Talentbaum (6.2, 6.7).
//!
//! # Die drei Zustände
//!
//! | Zustand | `ignited_at` | Wirkung |
//! | --- | --- | --- |
//! | bereit | `None` | keine |
//! | aktiv | `Some(t)`, `now < t + Dauer` | `+overdrive_catalysis_permille` |
//! | verbraucht | `Some(t)`, `now >= t + Dauer` | keine, und kein zweites Mal |
//!
//! Ein vierter Zustand fehlt mit Absicht: Es gibt keine Abklingzeit, nach der
//! Overdrive zurückkäme. „Einmal pro Kampf" heißt, dass nur ein neuer Kampf ihn
//! zurückgibt — der Simulator baut dafür einen neuen [`crate::Fight`].
//!
//! # Die Zahlen sind vorläufig
//!
//! Dauer und Stärke stehen in `data/player/base.json`, mit ihrer Herleitung.
//! Gemessen sind sie nicht; das ist Sache des Balance-Läufers.

use crate::data::PLAYER;
use crate::time::Millis;

/// Die Kennung des Overdrive-Drucks in einer Aufzeichnung.
///
/// Ein Druck nennt sonst einen **Leisten-Platz**: 13 Stellen mal zwei Ebenen,
/// also 0 bis 25. Overdrive liegt außerhalb der Leiste — er bekommt die erste
/// Nummer dahinter. So bleibt ein Druck ein Druck (Takt und Kennung, dieselbe
/// Form in jeder Datei), und kein Platz der Leiste ändert seine Bedeutung.
///
/// Dieselbe Zahl steht in `src/input/keys.ts` als `OVERDRIVE_PRESS`; die
/// Aufzeichnung `overdrive.json` trägt sie mit, und
/// `sim/tests/parity_overdrive.rs` vergleicht beide.
pub const OVERDRIVE_PRESS: u8 = 26;

/// Der Zustand von Overdrive in einem Kampf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Overdrive {
    /// Wann gezündet wurde. `None` heißt: noch bereit.
    ignited_at: Option<Millis>,
}

impl Overdrive {
    /// Ein bereiter Overdrive, wie zu Beginn jedes Kampfes.
    #[must_use]
    pub const fn new() -> Self {
        Self { ignited_at: None }
    }

    /// Wie lange er wirkt, in ms — aus `data/player/base.json`.
    #[must_use]
    pub fn duration_ms() -> Millis {
        PLAYER.overdrive_duration_ms
    }

    /// Wie viel CATALYSIS er solange dazugibt, in Promille.
    #[must_use]
    pub fn catalysis_bonus() -> u32 {
        PLAYER.overdrive_catalysis_permille
    }

    /// Zünden. `false`, wenn er in diesem Kampf schon gezündet wurde.
    ///
    /// **Ein zweiter Druck tut nichts** — er verlängert nicht, er setzt nicht
    /// neu, er verschiebt nicht. Sonst wäre „einmal pro Kampf" eine Frage des
    /// Zeitpunkts, zu dem man zum zweiten Mal drückt.
    pub fn ignite(&mut self, now: Millis) -> bool {
        if self.ignited_at.is_some() {
            return false;
        }
        self.ignited_at = Some(now);
        true
    }

    /// Wann gezündet wurde, oder `None`.
    #[must_use]
    pub const fn ignited_at(&self) -> Option<Millis> {
        self.ignited_at
    }

    /// Noch nicht gezündet?
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.ignited_at.is_none()
    }

    /// Wann die Wirkung endet, oder `None`, solange nicht gezündet.
    #[must_use]
    pub fn ends_at(&self) -> Option<Millis> {
        self.ignited_at
            .map(|at| at.saturating_add(Self::duration_ms()))
    }

    /// Wirkt er gerade?
    ///
    /// **Halboffen, `[Zündung, Zündung + Dauer)`.** Im Takt der Zündung wirkt
    /// er schon — ein Zauber, der im selben Takt nach dem Druck auf G gewirkt
    /// wird, bekommt das Tempo —, im Takt des Endes nicht mehr. Genau
    /// `Dauer / Takt` Takte also, keiner mehr und keiner weniger.
    #[must_use]
    pub fn is_active(&self, now: Millis) -> bool {
        match (self.ignited_at, self.ends_at()) {
            (Some(at), Some(ends)) => now >= at && now < ends,
            _ => false,
        }
    }

    /// Restlaufzeit in ms; 0, wenn er nicht wirkt.
    #[must_use]
    pub fn remaining_ms(&self, now: Millis) -> Millis {
        if !self.is_active(now) {
            return 0;
        }
        self.ends_at().map_or(0, |ends| ends - now)
    }

    /// Was er in diesem Moment zu CATALYSIS beiträgt, in Promille.
    #[must_use]
    pub fn catalysis_permille(&self, now: Millis) -> u32 {
        if self.is_active(now) {
            Self::catalysis_bonus()
        } else {
            0
        }
    }
}
