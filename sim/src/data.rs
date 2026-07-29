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

/// Eine Phase: ab welchem Anteil der Lebenspunkte der Boss sein Verhalten
/// wechselt (GAME-DESIGN 11.1).
///
/// **Eskalierend, nicht tauschend:** Jede Phase legt eine Mechanik obendrauf,
/// statt die vorige zu ersetzen. Die Mechaniken selbst kommen mit E4-2; hier
/// steht nur, WANN gewechselt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase {
    /// Anteil der verbleibenden Lebenspunkte in Prozent, ab dem sie gilt.
    pub at_hp_percent: u32,
    /// Anzeigename der Phase.
    pub name: &'static str,
}

/// Ein Eintrag der Boss-Zeitleiste (GAME-DESIGN 11.1: „Schadens-Ereignisse als
/// **feste Zeitleiste**“).
///
/// **Fest heisst wirklich fest.** Der Boss reagiert nicht auf den Spieler, und
/// es gibt keine zufaellige Streuung — nur so sind Bestzeiten vergleichbar und
/// der Kampf ueberhaupt simulierbar (GAME-DESIGN 11.1, Absatz danach).
///
/// **Was hier absichtlich fehlt:** die Wirkung. Welcher Baustein was tut, ist
/// Gegenstand von E4-2 bis E4-6, und welcher Boss welche Eintraege bekommt,
/// gehoert nach E10 (GAME-DESIGN 22, Zeile „Boss-Zeitleisten“). Dieses Feld ist
/// das Geruest, das beides spaeter traegt: Der Zeitpunkt wird schon jetzt
/// taktgenau angesteuert und geht in die Zustands-Pruefsumme ein, damit ein
/// spaeter angehaengter Baustein nicht auf ungepruefter Zeitrechnung aufsetzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEntry {
    /// Zeitpunkt seit Kampfbeginn. Liegt auf dem Takt-Raster des Spiels.
    pub at_ms: Millis,
    /// Kennung des Mechanik-Bausteins, der hier greift.
    pub mechanic: &'static str,
}

/// Die Energieleiste — die zweite Uhr eines Bosses.
///
/// Aus GAME-DESIGN 11.2, dem ersten der neun Mechanik-Bausteine:
///
/// > Boss-Energie steigt **unabhaengig von den Lebenspunkten** und loest bei 100
/// > eine Mechanik aus. Gibt dem Kampf einen inneren Rhythmus, der nichts mit
/// > dem Lebensbalken zu tun hat.
///
/// Das „unabhaengig" ist die eigentliche Aussage: Wer schneller Schaden macht,
/// verschiebt den Ausloeser **nicht**. Damit entsteht ein zweiter Takt, gegen
/// den der Spieler planen muss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Energy {
    /// Abstand zwischen zwei Anstiegen.
    pub per_step_ms: Millis,
    /// Wie viel je Anstieg dazukommt.
    pub amount: u32,
    /// Ab diesem Stand loest sie aus.
    pub threshold: u32,
    /// Worauf sie nach dem Ausloesen faellt.
    pub reset_to: u32,
}

/// Ein Boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Boss {
    /// Bezeichner aus der Datei.
    pub id: &'static str,
    /// Anzeigename im Spiel.
    pub name: &'static str,
    /// Instanz, zu der er gehoert.
    pub instance: &'static str,
    /// Lebenspunkte.
    pub hp: u32,
    /// Zeitlimit: Ist es abgelaufen, hat der Spieler verloren.
    pub overpressure_ms: Millis,
    /// Phasen-Schwellen, absteigend. Leer heisst: keine Phasen.
    pub phases: &'static [Phase],
    /// Die zweite Uhr, falls dieser Boss eine hat.
    pub energy: Option<Energy>,
    /// Die feste Abfolge der Boss-Handlungen, aufsteigend nach Zeitpunkt.
    pub timeline: &'static [TimelineEntry],
}

impl Boss {
    /// Welche Phase bei diesem Anteil der Lebenspunkte gilt.
    ///
    /// 0 ist der Start. Die Zaehlung steigt, waehrend die Lebenspunkte fallen —
    /// der Boss geht also von Phase 0 nach oben.
    #[must_use]
    pub fn phase_at(&self, hp_percent: u32) -> usize {
        let mut phase = 0;
        for (index, schwelle) in self.phases.iter().enumerate() {
            if hp_percent <= schwelle.at_hp_percent {
                phase = index + 1;
            }
        }
        phase
    }
}

include!(concat!(env!("OUT_DIR"), "/data_generated.rs"));

/// Sucht einen Boss ueber seinen Bezeichner.
///
/// Lineare Suche ueber eine Liste mit fester Reihenfolge — bei 68 Bossen ist
/// der Unterschied zu einer Hash-Abbildung nicht messbar, und die Suche laeuft
/// ohnehin nur beim Aufbau, nie im Schritt.
#[must_use]
pub fn boss_by_id(id: &str) -> Option<&'static Boss> {
    BOSSES.iter().find(|boss| boss.id == id)
}

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
