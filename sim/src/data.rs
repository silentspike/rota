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
    /// Ob dieser Zauber einen laufenden Boss-Zauber **unterbricht**
    /// (GAME-DESIGN 11.2, Baustein 9).
    ///
    /// Steht in den Daten und nicht im Code: Ein Zauber, der das kann, ist ein
    /// Datensatz wie jeder andere (Leitplanke 6). Er ist zugleich die **einzige
    /// Ausnahme** von der globalen Wartezeit im ganzen Spiel (GAME-DESIGN 6.6)
    /// — aber auch das steht als `triggers_gcd: false` in denselben Daten und
    /// nicht als Sonderfall in der Kampfschleife.
    pub interrupt: bool,
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
/// **Seit Issue #110 hat ein Eintrag Wirkung.** In #96 war dies noch ein
/// leeres Geruest: Der Zeitpunkt wurde taktgenau angesteuert, aber es geschah
/// nichts. Jetzt teilt der Boss damit Schaden aus — sofort ([`TimelineKind::Strike`])
/// oder nach einer Wirkzeit, die sich unterbrechen laesst
/// ([`TimelineKind::InterruptibleCast`]).
///
/// Was **weiterhin** fehlt, sind die uebrigen sieben Mechanik-Bausteine
/// (E4-3 und spaeter) und die Zeitleisten der einzelnen Bosse, die nach E10
/// gehoeren (GAME-DESIGN 22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEntry {
    /// Zeitpunkt seit Kampfbeginn. Liegt auf dem Takt-Raster des Spiels.
    ///
    /// Bei einem Zauber ist das der Zeitpunkt, zu dem er **beginnt** — nicht
    /// der, zu dem er trifft. Der Treffer liegt `cast_time_ms` spaeter, und
    /// dazwischen liegt das Fenster, in dem unterbrochen werden kann.
    pub at_ms: Millis,
    /// Kennung des Mechanik-Bausteins, der hier greift.
    pub mechanic: &'static str,
    /// Anzeigename der Faehigkeit. Englisch, wie alle Spieltexte.
    pub name: &'static str,
    /// Was hier geschieht.
    pub kind: TimelineKind,
    /// Schaden am Spieler, **vor** WARD.
    pub damage: u32,
    /// Wirkzeit eines Boss-Zaubers. 0 bei allem, was sofort trifft.
    pub cast_time_ms: Millis,
}

/// Was ein Zeitleisten-Eintrag tut.
///
/// Nur zwei Arten, und das ist Absicht: Die uebrigen der neun Mechanik-
/// Bausteine (GAME-DESIGN 11.2) kommen in E4-3 und spaeter. Wer hier eine
/// dritte Art braucht, fuegt sie hinzu — das Geruest aus #96 traegt sie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineKind {
    /// Trifft sofort. Nicht zu verhindern, nur auszuhalten.
    Strike,
    /// Ein Zauber mit Wirkzeit. **Unterbrechbar**, solange er laeuft
    /// (GAME-DESIGN 11.2, Baustein 9).
    ///
    /// Kommt er durch, trifft er mit vollem Schaden; wird er unterbrochen,
    /// richtet er gar nichts an. Ein Mittelding gibt es nicht — das ist die
    /// Entscheidung, die den Baustein ausmacht.
    InterruptibleCast,
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

/// Die Grundwerte des Spielers (GAME-DESIGN 6.1).
///
/// **Berechnet, nicht gespeichert.** Der Spielstand haelt sie nicht — sie
/// ergeben sich aus Ausruestung und Talenten, und beides gibt es noch nicht
/// (E5, E6). Bis dahin stehen sie in `data/player/base.json`.
///
/// > **Die Zahlen sind noch nicht Design.** GAME-DESIGN Teil F.6 fuehrt sie
/// > als „vor E4 auszuplanen"; entschieden ist davon nichts. Sie tragen die
/// > Prueffaelle aus Issue #110 und sonst nichts.
///
/// # Verhaeltnis zur Ausruestung (E6)
///
/// `src/gear/slots.ts` fuehrt mit `BASE_STAT_IDS` die **Kennungen** der
/// Grundwerte; diese Datei fuehrt ihre **Werte**. Zwei verschiedene Aussagen,
/// kein Widerspruch.
///
/// **Diese Werte sind der STARTWERT.** Ausruestung wird darauf **addiert**,
/// sie ersetzt ihn nicht. Heute traegt kein Stueck etwas, also faellt die
/// Frage nicht auf; wer sie zusammenfuehrt, findet die Antwort hier und in
/// `src/core/data.ts`.
///
/// Eine Formel von WARD-Punkten zu Minderungsanteil fehlt dabei noch, und
/// zwar mit Absicht — sie ist Design und steht in Kapitel 22 als offen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerBase {
    /// VESSEL — Lebenspunkte.
    pub vessel: u32,
    /// WARD — Anteil, um den eingehender Schaden gemindert wird.
    ///
    /// Ein Anteil und keine Ruestungszahl: Eine Ruestungsformel waere eine
    /// Design-Entscheidung, und die ist nicht gefallen. Ein Anteil traegt den
    /// Prueffall, ohne etwas vorwegzunehmen.
    pub ward_reduction: f64,
}

impl PlayerBase {
    /// Wie viel Schaden nach WARD ankommt.
    ///
    /// **Einmal gerundet, am Ende** — dieselbe Regel wie bei ausgehendem
    /// Schaden (GAME-DESIGN 6.3). Zweimal zu runden ergaebe je nach
    /// Reihenfolge andere Zahlen, und genau daran laufen zwei Umsetzungen
    /// auseinander.
    #[must_use]
    pub fn incoming(&self, raw: u32) -> u32 {
        let gemindert = f64::from(raw) * (1.0 - self.ward_reduction);
        // Nie unter null, und nie mehr als der Rohwert: Ein WARD ueber 1.0
        // wuerde den Spieler sonst heilen.
        gemindert.round().max(0.0) as u32
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
        // Drei Schadenszauber, dazu seit Issue #110 der Unterbrecher. Er zaehlt
        // nicht zu den dreien aus `STEP1-SPEC`: Er richtet keinen Schaden an
        // und kommt in keiner der vier Pruefrotationen vor.
        assert_eq!(SPELL_COUNT, 4);
        assert_eq!(index_of("kindle"), Some(KINDLE));
        assert_eq!(index_of("smoulder"), Some(SMOULDER));
        assert_eq!(index_of("pyre"), Some(PYRE));
        assert_eq!(index_of("gibtesnicht"), None);

        // Nicht ueber `base_damage` gezaehlt: SMOULDER ist ein Brand-Effekt und
        // PYRE ein Entlader, beide haben 0 Sofortschaden. Der Unterbrecher
        // unterscheidet sich nicht durch den Schaden, sondern durch die Rolle.
        let kampfzauber = SPELLS.iter().filter(|s| !s.interrupt).count();
        assert_eq!(kampfzauber, 3, "Schritt 1 hat genau drei Kampfzauber");
    }

    #[test]
    fn der_unterbrecher_ist_die_einzige_ausnahme_von_der_wartezeit() {
        // GAME-DESIGN 6.6: **Genau eine** Ausnahme. Ein zweiter Zauber ohne
        // globale Wartezeit waere eine Design-Aenderung, die sich in eine
        // Datendatei einschleichen koennte, ohne dass jemand sie beschliesst.
        let ohne_gcd: Vec<&str> = SPELLS
            .iter()
            .filter(|s| !s.triggers_gcd)
            .map(|s| s.id)
            .collect();
        assert_eq!(ohne_gcd, vec!["quench"]);

        let unterbrecher: Vec<&str> = SPELLS
            .iter()
            .filter(|s| s.interrupt)
            .map(|s| s.id)
            .collect();
        assert_eq!(unterbrecher, vec!["quench"]);
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
