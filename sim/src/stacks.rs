//! Die Stapel-Mechanik der Schulen (GAME-DESIGN 4.1, Issue E3-2).
//!
//! Das Gegenstueck zu `src/combat/schools/stacks.ts`. Der Hitze-Gewinn stand
//! bis E3-2 fest verdrahtet in [`crate::fight::Fight::cast`] und
//! `Fight::dot_tick`; er haengt jetzt an den Ereignissen `spell.cast` und
//! `dot.tick`. **Was ein Zauber beisteuert, steht in seinen Daten**
//! (`resource_gain`, `resource_gain_per_tick`) — ein neuer Zauber bleibt damit
//! ein Datensatz (Leitplanke 6).
//!
//! # Warum der Gewinn hier NICHT gebucht wird
//!
//! In TypeScript ruft der Empfaenger direkt in den Kampf zurueck. In Rust geht
//! das nicht: Der Empfaenger liegt **in** der Verteilung, und die liegt im
//! Kampf — waehrend `emit` laeuft, ist der Kampf bereits ausgeliehen.
//!
//! Deshalb die Teilung, die ohnehin die richtige ist:
//!
//! | Wer          | Was                                                       |
//! | ------------ | --------------------------------------------------------- |
//! | Mechanik     | **wie viel** — gelesen aus den Daten des Zaubers           |
//! | Kampf        | **buchen** — Obergrenze, Verpufftes, Pruefsumme            |
//!
//! Der angesetzte Betrag steht in einer geteilten Zelle; der Kampf holt ihn
//! unmittelbar nach dem Austragen ab. Eine Anforderung beim Aufbau, keine im
//! Schritt (GAME-DESIGN 18.4).
//!
//! # Was hier nicht steht
//!
//! Der **Verfall**. Er haengt an der Uhr, nicht an einem Ereignis — im
//! Simulator ein eingeplantes [`crate::event::EventKind::ResourceDecay`]. Ihn
//! hierher zu ziehen hiesse, eine Zeitregel als Reaktion zu verkleiden.
//!
//! Die **Entladung**. Sie ist Teil des Zaubers und liest den Vorrat, bevor der
//! eigene Gewinn ihn erhoehen kann (`STEP1-SPEC` 1.3).

use std::cell::Cell;
use std::rc::Rc;

use crate::data::{SPELLS, SPELL_COUNT};
use crate::rules_event::{RuleEvent, RuleReceiver};

/// Der angesetzte, noch nicht gebuchte Gewinn.
///
/// Geteilt zwischen Mechanik und Kampf. `Cell` statt `RefCell`: Es geht um
/// eine Zahl, und der Zugriff ist nie verschachtelt.
pub type PendingGain = Rc<Cell<u16>>;

/// Sammelt Stapel, wenn gewirkt wird oder ein Effekt ueber Zeit tickt.
pub struct StackMechanic {
    /// Gewinn je Zauber bei einer Wirkung, aus den Daten.
    per_cast: [u16; SPELL_COUNT],
    /// Gewinn je Zauber und Tick, aus den Daten.
    per_tick: [u16; SPELL_COUNT],
    pending: PendingGain,
    /// Wie oft Stapel angesetzt wurden. Fuer die Pruefung.
    gains: Rc<Cell<u32>>,
}

impl StackMechanic {
    /// Legt die Mechanik an und liest die Gewinne aus den Zauber-Daten.
    #[must_use]
    pub fn new(pending: PendingGain, gains: Rc<Cell<u32>>) -> Self {
        let mut per_cast = [0; SPELL_COUNT];
        let mut per_tick = [0; SPELL_COUNT];
        for (index, spell) in SPELLS.iter().enumerate() {
            per_cast[index] = spell.resource_gain;
            // **Ein Kanalzauber wird hier abgezweigt** (Issue #323). Sein
            // Schritt meldet sich als `DotTick` — die Ereignismenge ist fest
            // (GAME-DESIGN 18.3) —, aber was er beisteuert, steht in
            // `resource_gain` und nicht in `dot.resource_gain_per_tick`: Er hat
            // gar keinen DoT. Ohne das bekam er gemessen 0 Vorrat, waehrend
            // KINDLE 3 brachte.
            //
            // **Der ROHWERT wird geteilt, nicht der fertige Betrag.** Die
            // Faktoren (FLASHPOINT-Fenster, Talent-Verteilung) kommen erst in
            // `Fight::add_resource` dazu; das Spiel rechnet sie schon in
            // `gainOfChannelStep` ein. Fuer einen ungeteilten Betrag ist das
            // gleichwertig — mit einer Teilung dazwischen nicht mehr: Bei 5
            // Rohgewinn ueber 2 Schritte und Faktor 1,5 gaebe "erst Faktor,
            // dann teilen" eine 4, "erst teilen, dann Faktor" eine 3. Beide
            // Seiten teilen deshalb vor den Faktoren.
            per_tick[index] = if spell.channel_tick_ms > 0 && spell.channel_ms > 0 {
                let schritte = spell.channel_ms / spell.channel_tick_ms;
                if schritte > 0 {
                    u16::try_from(u32::from(spell.resource_gain) / schritte).unwrap_or(0)
                } else {
                    0
                }
            } else {
                spell.dot.map_or(0, |dot| dot.resource_gain_per_tick)
            };
        }
        Self {
            per_cast,
            per_tick,
            pending,
            gains,
        }
    }
}

impl RuleReceiver for StackMechanic {
    fn on_rule_event(&mut self, event: RuleEvent, subject: i16, _value: f64, _time: u32) {
        let Ok(index) = usize::try_from(subject) else {
            return;
        };
        let amount = match event {
            RuleEvent::SpellCast => self.per_cast.get(index).copied().unwrap_or(0),
            RuleEvent::DotTick => self.per_tick.get(index).copied().unwrap_or(0),
            _ => 0,
        };
        if amount == 0 {
            return;
        }
        self.gains.set(self.gains.get() + 1);
        self.pending.set(self.pending.get() + amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::index_of;

    #[test]
    fn liest_den_gewinn_aus_den_daten() {
        let pending: PendingGain = Rc::new(Cell::new(0));
        let gains = Rc::new(Cell::new(0));
        let mut mechanic = StackMechanic::new(Rc::clone(&pending), Rc::clone(&gains));

        let kindle = index_of("kindle").expect("KINDLE muss es geben");
        mechanic.on_rule_event(RuleEvent::SpellCast, kindle as i16, 0.0, 0);
        assert_eq!(pending.get(), SPELLS[kindle].resource_gain);
        assert_eq!(gains.get(), 1);
    }

    #[test]
    fn reagiert_nicht_auf_fremde_ereignisse() {
        let pending: PendingGain = Rc::new(Cell::new(0));
        let mut mechanic = StackMechanic::new(Rc::clone(&pending), Rc::new(Cell::new(0)));
        mechanic.on_rule_event(RuleEvent::Damage, 0, 100.0, 0);
        mechanic.on_rule_event(RuleEvent::BossPhase, -1, 2.0, 0);
        assert_eq!(pending.get(), 0);
    }

    #[test]
    fn haelt_einen_zauber_ohne_gewinn_bei_null() {
        let pending: PendingGain = Rc::new(Cell::new(0));
        let mut mechanic = StackMechanic::new(Rc::clone(&pending), Rc::new(Cell::new(0)));
        // PYRE ist der Entlade-Zauber und bringt selbst nichts ein.
        let pyre = index_of("pyre").expect("PYRE muss es geben");
        mechanic.on_rule_event(RuleEvent::SpellCast, pyre as i16, 0.0, 0);
        assert_eq!(pending.get(), 0);
    }
}
