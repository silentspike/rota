//! Zauber- und Boss-Daten — einkompiliert aus `data/`.
//!
//! Die Werte in diesem Modul stammen **nicht** aus dieser Quelldatei. Sie werden
//! von `build.rs` aus `data/spells/calcination.json` und
//! `data/bosses/first-crucible.json` erzeugt, und zwar bei jedem Bau neu
//! (`cargo:rerun-if-changed`). Wer eine Zahl aendern will, aendert die JSON-Datei;
//! hier stehen nur die Formen, in die sie gegossen wird.
//!
//! Damit gilt fuer den Simulator dasselbe wie fuers Spiel: `data/` ist die
//! einzige Wahrheit (GAME-DESIGN 18.3, `STEP1-SPEC` § 5 — das Format ist
//! ausdruecklich eine Schnittstelle zwischen beiden Umsetzungen).

use crate::time::Millis;

/// Eine Schul-Mechanik: die Ressource, die sich auflaedt und wieder verfaellt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resource {
    /// Bezeichner aus der Datei, z. B. `heat`.
    pub id: &'static str,
    /// Anzeigename im Spiel (englisch, siehe Design-Gesetz 5).
    pub name: &'static str,
    /// Obergrenze. Was darueber hinaus erzeugt wird, verpufft.
    pub cap: u16,
    /// Abstand zwischen zwei Verfall-Schritten.
    pub decay_per_step_ms: Millis,
    /// Wie viel je Verfall-Schritt abgeht.
    pub decay_amount: u16,
    /// Ab hier zeigt das HUD eine Markierung — fuer den Simulator nur mitgefuehrt.
    pub highlight_at: u16,
}

/// Ein Brand-Effekt: Schaden ueber Zeit statt beim Wirken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dot {
    /// Schaden je Tick.
    pub tick_damage: u32,
    /// Abstand zwischen zwei Ticks.
    pub tick_interval_ms: Millis,
    /// Gesamtlaufzeit ab dem Wirken.
    pub duration_ms: Millis,
    /// Ressourcen-Gewinn je Tick.
    pub resource_gain_per_tick: u16,
}

/// Ein Entlade-Zauber: verbraucht die Ressource und schlaegt dafuer zu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spender {
    /// Schaden je Ressourcen-Punkt.
    pub damage_per_point: u32,
    /// Ab diesem Stand greift der Bonus — **ab**, nicht darueber.
    pub bonus_threshold: u16,
    /// Womit der Schaden bei erreichtem Bonus vervielfacht wird.
    pub bonus_multiplier: u32,
}

/// Ein Zauber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spell {
    /// Bezeichner aus der Datei, z. B. `kindle`.
    pub id: &'static str,
    /// Anzeigename im Spiel.
    pub name: &'static str,
    /// Ab welcher Spielerstufe der Zauber verfuegbar ist.
    pub unlock_level: u16,
    /// Sofortschaden beim Wirken.
    pub base_damage: u32,
    /// Eigene Abklingzeit.
    pub cooldown_ms: Millis,
    /// Wirkzeit. In Schritt 1 durchweg 0 (Sofort-Zauber).
    pub cast_time_ms: Millis,
    /// Ob der Zauber die globale Wartezeit ausloest.
    pub triggers_gcd: bool,
    /// Ressourcen-Gewinn beim Wirken.
    pub resource_gain: u16,
    /// Brand-Effekt, falls der Zauber einen hat.
    pub dot: Option<Dot>,
    /// Entlade-Verhalten, falls der Zauber die Ressource verbraucht.
    pub spender: Option<Spender>,
    /// Um wie viel dieser Zauber **alle laufenden Abklingzeiten** verkuerzt
    /// (SUBLIMATION, GAME-DESIGN 4.1, Issue E3-3).
    ///
    /// **0 heisst: verkuerzt nichts.** Das Feld ist in der Datei optional; die
    /// drei Zauber der Kalzination fuehren es nicht. Ein Pflichtfeld waere hier
    /// falsch — ein Zauber ohne Sublimations-Wirkung soll es gar nicht erst
    /// nennen muessen.
    pub cooldown_reduction_ms: Millis,
}

/// Ein Boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Boss {
    /// Bezeichner aus der Datei.
    pub id: &'static str,
    /// Anzeigename im Spiel.
    pub name: &'static str,
    /// Lebenspunkte.
    pub hp: u32,
    /// Zeitlimit: Ist es abgelaufen, hat der Spieler verloren.
    pub overpressure_ms: Millis,
}

include!(concat!(env!("OUT_DIR"), "/data_generated.rs"));

/// Sucht einen Zauber ueber seinen Bezeichner.
///
/// Lineare Suche ueber eine Liste mit fester Reihenfolge — **absichtlich keine
/// Hash-Abbildung** (GAME-DESIGN 18.3). Bei drei bis sechzig Zaubern ist der
/// Unterschied nicht messbar, und die Suche laeuft ohnehin nur beim Aufbau,
/// nie im Schritt.
#[must_use]
pub fn index_of(id: &str) -> Option<usize> {
    let mut i = 0;
    while i < SPELL_COUNT {
        if SPELLS[i].id == id {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn die_drei_zauber_aus_schritt_1_sind_da() {
        assert_eq!(SPELL_COUNT, 3, "Schritt 1 hat genau drei Zauber");
        assert_eq!(index_of("kindle"), Some(KINDLE));
        assert_eq!(index_of("smoulder"), Some(SMOULDER));
        assert_eq!(index_of("pyre"), Some(PYRE));
        assert_eq!(index_of("gibtesnicht"), None);
    }

    #[test]
    fn die_reihenfolge_ist_die_der_datei() {
        // Sie ist Teil des Determinismus: Spiel und Simulator muessen dieselbe
        // Reihenfolge sehen, sonst weichen Ereignisse bei Gleichstand ab.
        assert_eq!(SPELLS[0].id, "kindle");
        assert_eq!(SPELLS[1].id, "smoulder");
        assert_eq!(SPELLS[2].id, "pyre");
    }

    #[test]
    fn die_schul_mechanik_ist_die_hitze() {
        assert_eq!(SCHOOL, "CALCINATION");
        assert_eq!(RESOURCE.id, "heat");
    }
}
