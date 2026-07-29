//! SUBLIMATION — Verkuerzung aller Abklingzeiten (GAME-DESIGN 4.1, Issue E3-3).
//!
//! Das Gegenstueck zu `src/combat/schools/sublimation.ts`. Ein Empfaenger auf
//! `spell.cast`, der alle **laufenden** Abklingzeiten um einen Betrag
//! verkuerzt. Der Betrag steht in den Zauber-Daten, nicht im Code.
//!
//! # Zwei Festlegungen, bestaetigt am 2026-07-29
//!
//! 1. **Der wirkende Zauber verkuerzt seine EIGENE Abklingzeit nicht.** Taete
//!    er es, waere er schlicht ein Zauber mit kuerzerer Abklingzeit — die
//!    Mechanik waere eine umstaendliche Schreibweise fuer
//!    `cooldown_ms - cooldown_reduction_ms`. Und es widerspraeche der
//!    Reihenfolge (B) aus E3-1b: Ein Empfaenger beeinflusst den naechsten
//!    Zauber, nicht den laufenden.
//! 2. **Die globale Wartezeit ist keine Abklingzeit.** Sie sinkt nur ueber den
//!    Talentbaum, nie ueber Ausruestung oder Zauberwirkungen
//!    (GAME-DESIGN 6.2).
//!
//! # Warum der Empfaenger nicht selbst verkuerzt
//!
//! Dieselbe Bindung wie bei [`crate::stacks`]: Der Empfaenger liegt in der
//! Verteilung, die im Kampf liegt; waehrend `emit` laeuft, ist der Kampf
//! bereits ausgeliehen. Er setzt deshalb an, der Kampf fuehrt aus.
//!
//! # Woher die Betraege kommen
//!
//! **Als Tabelle vom Aufrufer**, nicht aus `SPELLS` gelesen. Grund: Das Feld
//! `cooldown_reduction_ms` an [`crate::data::Spell`] fehlt noch — es laesst
//! sich nur zusammen mit `build.rs` einfuehren, weil dort jede Feldzuweisung
//! einzeln ausgeschrieben wird (gemessen: drei `E0063`, wenn man nur `data.rs`
//! anfasst). Bis dahin ist dieses Modul vollstaendig und pruefbar, und der
//! Anschluss ist eine Zeile.

use std::cell::Cell;
use std::rc::Rc;

use crate::data::SPELL_COUNT;
use crate::rules_event::{RuleEvent, RuleReceiver};
use crate::time::Millis;

/// Was der Kampf nach dem Austragen auszufuehren hat.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PendingShorten {
    /// Um wie viel verkuerzt werden soll. 0 heisst: nichts zu tun.
    pub amount: Millis,
    /// Dieser Zauber bleibt aussen vor — der wirkende.
    pub except: i16,
}

/// Der angesetzte, noch nicht ausgefuehrte Auftrag. Geteilt mit dem Kampf.
pub type PendingSlot = Rc<Cell<PendingShorten>>;

/// Verkuerzt alle laufenden Abklingzeiten, wenn ein Zauber gewirkt wird.
pub struct SublimationMechanic {
    /// Verkuerzung je Zauber, aus den Daten. 0 = dieser Zauber verkuerzt nichts.
    per_cast: [Millis; SPELL_COUNT],
    pending: PendingSlot,
    /// Wie oft die Mechanik angesetzt hat. Fuer die Pruefung.
    applications: Rc<Cell<u32>>,
}

impl SublimationMechanic {
    /// Legt die Mechanik mit der Verkuerzungs-Tabelle an.
    #[must_use]
    pub fn new(
        per_cast: [Millis; SPELL_COUNT],
        pending: PendingSlot,
        applications: Rc<Cell<u32>>,
    ) -> Self {
        Self {
            per_cast,
            pending,
            applications,
        }
    }
}

impl RuleReceiver for SublimationMechanic {
    fn on_rule_event(&mut self, event: RuleEvent, subject: i16, _value: f64, _time: u32) {
        if event != RuleEvent::SpellCast {
            return;
        }
        let Ok(index) = usize::try_from(subject) else {
            return;
        };
        let amount = self.per_cast.get(index).copied().unwrap_or(0);
        if amount == 0 {
            return;
        }
        self.applications.set(self.applications.get() + 1);
        self.pending.set(PendingShorten {
            amount,
            except: subject,
        });
    }
}

/// Alle laufenden Abklingzeiten verkuerzen.
///
/// **Begrenzt auf den Jetzt-Zeitpunkt statt abgezogen.** Ein Zauber, der
/// bereits bereit ist, bleibt unberuehrt; sonst entstuende eine "negative
/// Restzeit", die den naechsten Wirk-Versuch um genau diesen Betrag frueher
/// zuliesse, ohne dass es je eine Abklingzeit gegeben haette.
///
/// Steht hier und nicht im Kampf, damit beide Seiten dieselbe Rechnung an
/// derselben Stelle haben — `tests/sublimation.rs` prueft sie ohne Kampf.
///
/// @returns wie viele Abklingzeiten tatsaechlich verkuerzt wurden
pub fn shorten_cooldowns(
    ready_at: &mut [Millis; SPELL_COUNT],
    order: PendingShorten,
    now: Millis,
) -> u32 {
    if order.amount == 0 {
        return 0;
    }
    let except = usize::try_from(order.except).unwrap_or(usize::MAX);
    let mut touched = 0;
    for (index, slot) in ready_at.iter_mut().enumerate() {
        if index == except || *slot <= now {
            continue;
        }
        *slot = slot.saturating_sub(order.amount).max(now);
        touched += 1;
    }
    touched
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mechanic(
        per_cast: [Millis; SPELL_COUNT],
    ) -> (SublimationMechanic, PendingSlot, Rc<Cell<u32>>) {
        let pending: PendingSlot = Rc::default();
        let applications = Rc::new(Cell::new(0));
        let m = SublimationMechanic::new(per_cast, Rc::clone(&pending), Rc::clone(&applications));
        (m, pending, applications)
    }

    #[test]
    fn setzt_nur_an_wenn_der_zauber_etwas_verkuerzt() {
        let mut per_cast = [0; SPELL_COUNT];
        per_cast[0] = 2000;
        let (mut m, pending, applications) = mechanic(per_cast);

        m.on_rule_event(RuleEvent::SpellCast, 1, 0.0, 0);
        assert_eq!(pending.get(), PendingShorten::default());
        assert_eq!(applications.get(), 0);

        m.on_rule_event(RuleEvent::SpellCast, 0, 0.0, 0);
        assert_eq!(
            pending.get(),
            PendingShorten {
                amount: 2000,
                except: 0
            }
        );
        assert_eq!(applications.get(), 1);
    }

    #[test]
    fn reagiert_nicht_auf_fremde_ereignisse() {
        let mut per_cast = [0; SPELL_COUNT];
        per_cast[0] = 2000;
        let (mut m, pending, _) = mechanic(per_cast);
        m.on_rule_event(RuleEvent::Damage, 0, 100.0, 0);
        m.on_rule_event(RuleEvent::DotTick, 0, 40.0, 0);
        assert_eq!(pending.get(), PendingShorten::default());
    }

    #[test]
    fn verkuerzt_jede_laufende_abklingzeit() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 10_000;
        ready_at[1] = 20_000;
        ready_at[2] = 1_500;
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 2000,
                except: 2,
            },
            3000,
        );
        assert_eq!(touched, 2, "zwei laufende, der dritte ist ausgenommen");
        assert_eq!(ready_at[0], 8_000);
        assert_eq!(ready_at[1], 18_000);
        assert_eq!(ready_at[2], 1_500, "der wirkende Zauber bleibt unberuehrt");
    }

    #[test]
    fn laesst_einen_bereiten_zauber_unberuehrt() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 1000; // laengst abgelaufen bei now = 3000
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 2000,
                except: -1,
            },
            3000,
        );
        assert_eq!(touched, 0);
        assert_eq!(ready_at[0], 1000);
    }

    #[test]
    fn begrenzt_auf_jetzt_statt_abzuziehen() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 9_500; // noch 500 ms bei now = 9000
        shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 2000,
                except: -1,
            },
            9000,
        );
        assert_eq!(
            ready_at[0], 9000,
            "keine negative Restzeit — auf JETZT begrenzt"
        );
    }

    #[test]
    fn tut_nichts_ohne_auftrag() {
        let mut ready_at = [5000; SPELL_COUNT];
        let touched = shorten_cooldowns(&mut ready_at, PendingShorten::default(), 0);
        assert_eq!(touched, 0);
        assert_eq!(ready_at, [5000; SPELL_COUNT]);
    }
}
