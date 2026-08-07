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

use crate::data::{SPELLS, SPELL_COUNT};
use crate::rules_event::{RuleEvent, RuleReceiver};
use crate::time::Millis;

/// Was der Kampf nach dem Austragen auszufuehren hat.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PendingShorten {
    /// Um wie viel verkuerzt werden soll. 0 heisst: nichts zu tun.
    pub amount: Millis,
    /// Dieser Zauber bleibt aussen vor — der wirkende.
    pub except: i16,
    /// Nur Abklingzeiten unter dieser Grenze (ASCEND, #518). 0 = alle.
    pub below: Millis,
    /// Nur die laengste (DECANT, #518).
    pub longest_only: bool,
}

/// Der angesetzte, noch nicht ausgefuehrte Auftrag. Geteilt mit dem Kampf.
pub type PendingSlot = Rc<Cell<PendingShorten>>;

/// Verkuerzt alle laufenden Abklingzeiten, wenn ein Zauber gewirkt wird.
pub struct SublimationMechanic {
    /// Verkuerzung je Zauber, aus den Daten. 0 = dieser Zauber verkuerzt nichts.
    per_cast: [Millis; SPELL_COUNT],
    /// Je Zauber: nur Abklingzeiten unter dieser Grenze (ASCEND, #518).
    below: [Millis; SPELL_COUNT],
    /// Je Zauber: nur die laengste (DECANT, #518).
    longest_only: [bool; SPELL_COUNT],
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
        // **Die Auswahl-Tabellen entstehen HIER, nicht beim Aufrufer**
        // (Issue #518). `per_cast` kommt von aussen, weil `fight.rs` es aus
        // den Zauber-Daten fuellt; die zwei neuen kaemen aus derselben
        // Quelle, und drei Tabellen durch die Signatur zu reichen macht
        // jeden kuenftigen Aufrufer zum Mitwisser.
        //
        // Die Mechanik LIEFERT den Auftrag nur — angewandt wird er in
        // `fight.rs`. Damit ist das Kopieren hier harmlos: kein
        // Kampfzustand, keine Rechnung.
        let mut below = [0; SPELL_COUNT];
        let mut longest_only = [false; SPELL_COUNT];
        for (index, spell) in SPELLS.iter().enumerate() {
            below[index] = spell.cooldown_reduction_below_ms;
            longest_only[index] = spell.cooldown_reduction_longest_only;
        }

        Self {
            per_cast,
            below,
            longest_only,
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
        // **Die Auswahl kommt vom ZAUBER** (Issue #518). `shorten_cooldowns`
        // kennt den Ausloeser nicht — sie bekommt einen Index zum
        // Ueberspringen und sonst nichts.
        self.pending.set(PendingShorten {
            amount,
            except: subject,
            below: self.below.get(index).copied().unwrap_or(0),
            longest_only: self.longest_only.get(index).copied().unwrap_or(false),
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

    // **Die Auswahl VOR der Schleife** (Issue #518, DECANT). Wer den
    // laengsten treffen will, muss ihn erst finden — die Schleife sieht
    // immer nur einen Eintrag.
    //
    // Bei Gleichstand gewinnt der NIEDRIGERE Index. Das ist eine
    // Entscheidung, keine Selbstverstaendlichkeit: Ohne sie haenge das
    // Ergebnis an der Laufrichtung, und das waere ein stiller
    // Paritaets-Bruch, sobald eine Seite umbaut.
    let mut ziel = usize::MAX;
    if order.longest_only {
        let mut laengste = now;
        for (index, slot) in ready_at.iter().enumerate() {
            if index == except {
                continue;
            }
            if *slot > laengste {
                laengste = *slot;
                ziel = index;
            }
        }
        if ziel == usize::MAX {
            return 0;
        }
    }

    let mut touched = 0;
    for (index, slot) in ready_at.iter_mut().enumerate() {
        if index == except || *slot <= now {
            continue;
        }
        if order.longest_only && index != ziel {
            continue;
        }
        // **Ein STRENGES Kleiner** (Issue #518, ASCEND): Eine Abklingzeit
        // GENAU auf der Grenze bleibt unberuehrt. "Unter 10 s" heisst
        // woertlich unter, nicht "bis einschliesslich".
        if order.below > 0 && slot.saturating_sub(now) >= order.below {
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
                except: 0,
                below: 0,
                longest_only: false,
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
                below: 0,
                longest_only: false,
            },
            3000,
        );
        assert_eq!(touched, 2, "zwei laufende, der dritte ist ausgenommen");
        assert_eq!(ready_at[0], 8_000);
        assert_eq!(ready_at[1], 18_000);
        assert_eq!(ready_at[2], 1_500, "der wirkende Zauber bleibt unberuehrt");
    }

    // ------------------------------------------------------------------
    // Der UMFANG der Verkuerzung (Issue #518)
    //
    // Bis hierher setzt jeder Fall `below: 0` und `longest_only: false` — die
    // zwei Felder kamen also in der Datei vor, ohne je scharf gestellt zu
    // sein. Gemessen beim Nachziehen von #518 auf main: acht Vorkommen im
    // Testmodul, alle mit dem Standardwert. Damit war die KERNMECHANIK des
    // Issues auf der Simulator-Seite nur ueber die Aufzeichnung abgesichert,
    // und die hatte bis dahin keinen eigenen Prueffall.
    //
    // Vorkommen ist keine Pruefung.
    // ------------------------------------------------------------------

    #[test]
    fn ascend_trifft_nur_abklingzeiten_unter_der_schwelle() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 5_000; // 2 000 ms Restzeit -> unter der Schwelle
        ready_at[1] = 20_000; // 17 000 ms Restzeit -> darueber
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 1000,
                except: -1,
                below: 10_000,
                longest_only: false,
            },
            3000,
        );
        assert_eq!(touched, 1, "nur die kurze darf getroffen sein");
        assert_eq!(ready_at[0], 4_000);
        assert_eq!(
            ready_at[1], 20_000,
            "die lange Abklingzeit liegt ueber der Schwelle und bleibt"
        );
    }

    /// Die Grenze ist ein STRENGES Kleiner — genau darauf heisst unberuehrt.
    ///
    /// „Unter 10 s" steht so im Design und ist woertlich gemeint. Ein Fall,
    /// der nur 2 000 gegen 17 000 prueft, laesst offen, wohin der Gleichstand
    /// faellt — und genau dort trennen sich `<` und `<=`, ohne dass eine
    /// Aufzeichnung es je zeigen wuerde.
    #[test]
    fn genau_auf_der_schwelle_bleibt_unberuehrt() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 13_000; // Restzeit bei now = 3000 ist GENAU 10 000
        ready_at[1] = 12_999; // eine Millisekunde darunter
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 1000,
                except: -1,
                below: 10_000,
                longest_only: false,
            },
            3000,
        );
        assert_eq!(touched, 1, "nur der eine Takt unter der Grenze");
        assert_eq!(ready_at[0], 13_000, "genau auf der Grenze: unberuehrt");
        assert_eq!(ready_at[1], 11_999);
    }

    #[test]
    fn decant_trifft_nur_die_laengste_abklingzeit() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 10_000;
        ready_at[1] = 30_000; // die laengste
        ready_at[2] = 20_000;
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 2000,
                except: -1,
                below: 0,
                longest_only: true,
            },
            3000,
        );
        assert_eq!(touched, 1, "genau eine, nicht alle drei");
        assert_eq!(ready_at[1], 28_000, "die laengste wurde verkuerzt");
        assert_eq!(ready_at[0], 10_000);
        assert_eq!(ready_at[2], 20_000);
    }

    /// Bei Gleichstand gewinnt der NIEDRIGERE Index.
    ///
    /// Das ist eine Entscheidung, keine Selbstverstaendlichkeit: Ohne sie
    /// haengt das Ergebnis an der Laufrichtung der Schleife, und das waere ein
    /// stiller Paritaets-Bruch, sobald eine Seite umbaut. Der Kommentar an
    /// `shorten_cooldowns` haelt die Regel fest — dieser Fall haelt sie fest,
    /// wenn jemand den Kommentar nicht liest.
    #[test]
    fn bei_gleichstand_gewinnt_der_niedrigere_index() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[2] = 30_000;
        ready_at[5] = 30_000; // gleich lang
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 2000,
                except: -1,
                below: 0,
                longest_only: true,
            },
            3000,
        );
        assert_eq!(touched, 1);
        assert_eq!(ready_at[2], 28_000, "der niedrigere Index gewinnt");
        assert_eq!(ready_at[5], 30_000);
    }

    /// Der wirkende Zauber ist auch dann ausgenommen, wenn er die laengste
    /// Abklingzeit hat.
    ///
    /// Sonst verkuerzte DECANT sich selbst — und zwar ausgerechnet dann, wenn
    /// er als einziger eine lange Wartezeit traegt. Die Ausnahme muss VOR der
    /// Auswahl greifen, nicht erst in der Schleife: Sonst faende die Auswahl
    /// ihn, die Schleife uebersprænge ihn, und es wuerde gar nichts verkuerzt.
    #[test]
    fn der_wirkende_zauber_gewinnt_die_auswahl_nicht() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 40_000; // der wirkende, laengste
        ready_at[1] = 30_000; // die laengste der uebrigen
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 2000,
                except: 0,
                below: 0,
                longest_only: true,
            },
            3000,
        );
        assert_eq!(touched, 1, "die Auswahl darf nicht ins Leere laufen");
        assert_eq!(ready_at[0], 40_000, "der wirkende bleibt unberuehrt");
        assert_eq!(ready_at[1], 28_000);
    }

    /// Beides zusammen: nur die laengste, und nur wenn sie unter der Schwelle
    /// liegt.
    ///
    /// Kein ausgelieferter Zauber traegt heute beide Felder — aber die
    /// Rechnung kennt die Kombination, und was die Rechnung kann, gehoert
    /// gemessen. Sonst steht die Antwort erst fest, wenn jemand den Zauber
    /// baut, und dann ist sie zufaellig.
    #[test]
    fn schwelle_und_laengste_zusammen() {
        let mut ready_at = [0; SPELL_COUNT];
        ready_at[0] = 30_000; // die laengste, aber UEBER der Schwelle
        ready_at[1] = 8_000; // darunter, aber nicht die laengste
        let touched = shorten_cooldowns(
            &mut ready_at,
            PendingShorten {
                amount: 1000,
                except: -1,
                below: 10_000,
                longest_only: true,
            },
            3000,
        );
        assert_eq!(
            touched, 0,
            "die Auswahl nimmt die laengste, die Schwelle weist sie ab — dann \
             bleibt nichts uebrig, und es wird auf die zweitlaengste NICHT \
             ausgewichen"
        );
        assert_eq!(ready_at[0], 30_000);
        assert_eq!(ready_at[1], 8_000);
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
                below: 0,
                longest_only: false,
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
                below: 0,
                longest_only: false,
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
