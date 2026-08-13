//! Das Empfaenger-Ereignis-System (GAME-DESIGN 18.3, Issue E3-1b).
//!
//! Das Gegenstueck zu `src/combat/events.ts`. **Beide Umsetzungen muessen
//! dieselbe Folge erzeugen** — sonst ist der Abgleich wertlos.
//!
//! # "Ereignis" heisst hier dreierlei
//!
//! | Wo                     | Frage                    | Beispiel                     |
//! | ---------------------- | ------------------------ | ---------------------------- |
//! | [`crate::event`]       | **Wann** geschieht etwas | "in 2000 ms tickt der Brand" |
//! | **dieses Modul**       | **Worauf** reagiert wer  | "ein Zauber wurde gewirkt"   |
//! | `src/core/events.ts`   | Was hoert der Spieler    | "Countdown, dritter Schlag"  |
//!
//! Das erste ist eine Zeitplan-Warteschlange, das dritte gibt es nur im Spiel.
//! Hier stehen die **Regeln**: Was hier geschieht, veraendert das Ergebnis.
//!
//! # Warum die Reihenfolge festgeschrieben ist
//!
//! Aus 18.3: *"Empfaenger in fest definierter Reihenfolge — keine
//! Hash-Container. Sonst weichen Spiel und Simulator voneinander ab."*
//!
//! Sie faellt erst auf, wenn zwei Effekte auf dasselbe Ereignis hoeren — und
//! dann nicht sofort, sondern in einer Balance-Rechnung Wochen spaeter.
//!
//! # Keine Speicheranforderung beim Austragen
//!
//! Die Listen entstehen bei der Anmeldung, nicht beim Austragen
//! (GAME-DESIGN 18.4). `tests/rules_event.rs` misst es mit dem Zaehl-Allocator.

use crate::checksum::{mix_float, mix_int, FNV_OFFSET};

/// Kein Betreff — das Ereignis betrifft keinen einzelnen Zauber.
pub const NO_SUBJECT: i16 = -1;

/// Die feste Ereignismenge aus GAME-DESIGN 18.3.
///
/// **Die Kennnummern sind Schnittstelle.** Sie werden in die Pruefsumme
/// eingeruehrt; wer sie umsortiert, macht jeden Vergleich mit dem Spiel
/// ungueltig.
///
/// **Und die MENGE ist fest** — GAME-DESIGN 18.3 zaehlt sie auf. Ein siebtes
/// Ereignis fuer den Kanal-Schritt war am 2026-08-14 gebaut und ist wieder
/// entfallen: Das waere eine Design-Aenderung. Der Kanal-Schritt meldet sich
/// als `DotTick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RuleEvent {
    /// Ein Zauber wurde gewirkt. Betreff: Zauber-Kennnummer.
    SpellCast = 0,
    /// Schaden wurde verursacht. Betreff: Zauber, Wert: Schaden.
    Damage = 1,
    /// Ein Treffer war ein Krit. Ruht in Schritt 1 (PURITY = 0).
    Crit = 2,
    /// Ein Effekt ueber Zeit hat getickt.
    DotTick = 3,
    /// Der Boss betritt eine Phase. Wert: Nummer der Phase.
    BossPhase = 4,
    /// Eine Verstaerkung ist abgelaufen.
    BuffExpired = 5,
}

impl RuleEvent {
    /// Wie viele Ereignisse es gibt.
    pub const COUNT: usize = 6;

    /// Die Namen, unter denen die Daten die Ereignisse ansprechen.
    ///
    /// Wortlaut und Reihenfolge sind dieselben wie in `src/combat/events.ts`.
    pub const NAMES: [&'static str; Self::COUNT] = [
        "spell.cast",
        "damage",
        "crit",
        "dot.tick",
        "boss.phase",
        "buff.expired",
    ];

    /// Kennnummer zurueck in ein Ereignis. `None`, wenn es sie nicht gibt.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::SpellCast),
            1 => Some(Self::Damage),
            2 => Some(Self::Crit),
            3 => Some(Self::DotTick),
            4 => Some(Self::BossPhase),
            5 => Some(Self::BuffExpired),
            _ => None,
        }
    }

    /// Der Name dieses Ereignisses.
    #[must_use]
    pub const fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}

/// Wer auf ein Ereignis reagiert.
///
/// **Nur Zahlen, kein Ereignis-Objekt.** Der Empfang laeuft im Schritt; dort
/// entsteht nichts.
pub trait RuleReceiver {
    /// Ein Ereignis ist eingetreten.
    ///
    /// `subject` ist die Zauber-Kennnummer oder [`NO_SUBJECT`], `value` traegt
    /// den Schaden oder die Phasennummer, `time` die Kampfzeit in ms.
    fn on_rule_event(&mut self, event: RuleEvent, subject: i16, value: f64, time: u32);
}

/// Ein Platz in der Reihenfolge.
#[derive(Debug, Clone, Copy)]
struct Slot {
    /// Rang aus den Daten. Kleiner heisst frueher.
    order: i32,
    /// Laufende Nummer der Anmeldung. Loest Gleichstand eindeutig auf.
    seq: u32,
    /// Zeigt in [`RuleEvents::receivers`].
    index: u16,
}

/// Die Verteilung.
///
/// Je Ereignis eine zusammenhaengende Liste, nach Rang und Anmelde-Nummer
/// sortiert. Keine `HashMap`, kein `HashSet`: Deren Durchlauf-Reihenfolge ist
/// eine Zusage der Standardbibliothek, keine des Spiels.
pub struct RuleEvents {
    receivers: Vec<Box<dyn RuleReceiver>>,
    ids: Vec<String>,
    slots: [Vec<Slot>; RuleEvent::COUNT],
    next_seq: u32,
    hash: u32,
    emitted: u64,
}

impl std::fmt::Debug for RuleEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleEvents")
            .field("receivers", &self.ids)
            .field("emitted", &self.emitted)
            .field("checksum", &self.sequence_checksum())
            .finish()
    }
}

impl Default for RuleEvents {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl RuleEvents {
    /// Legt die Verteilung an, mit Platz fuer `capacity` Empfaenger je Ereignis.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            receivers: Vec::with_capacity(capacity * RuleEvent::COUNT),
            ids: Vec::with_capacity(capacity * RuleEvent::COUNT),
            slots: std::array::from_fn(|_| Vec::with_capacity(capacity)),
            next_seq: 0,
            hash: FNV_OFFSET,
            emitted: 0,
        }
    }

    /// Einen Empfaenger anmelden.
    ///
    /// Laeuft beim Aufbau des Kampfes, nicht im Schritt — hier darf Speicher
    /// entstehen.
    pub fn register(
        &mut self,
        id: &str,
        on: RuleEvent,
        order: i32,
        receiver: Box<dyn RuleReceiver>,
    ) {
        let index = u16::try_from(self.receivers.len()).expect("zu viele Empfaenger");
        self.receivers.push(receiver);
        self.ids.push(id.to_owned());
        let seq = self.next_seq;
        self.next_seq += 1;
        let list = &mut self.slots[on as usize];
        list.push(Slot { order, seq, index });
        // Stabil nach Rang, bei Gleichstand nach Anmelde-Nummer. Damit
        // verschiebt ein neuer Empfaenger die bestehenden nie relativ
        // zueinander.
        list.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.seq.cmp(&b.seq)));
    }

    /// Ein Ereignis austragen — der heisse Pfad.
    ///
    /// Die Pruefsumme laeuft **vor** den Empfaengern: Sie beschreibt, was
    /// ausgetragen wurde, nicht was daraus folgte. Sonst haenge sie davon ab,
    /// wer gerade zuhoert — und zwei Laeufe mit verschiedenen Zuhoerern waeren
    /// nicht mehr vergleichbar.
    pub fn emit(&mut self, event: RuleEvent, subject: i16, value: f64, time: u32) {
        self.hash = mix_int(self.hash, event as i32);
        self.hash = mix_int(self.hash, i32::from(subject));
        self.hash = mix_float(self.hash, value);
        self.hash = mix_int(self.hash, time as i32);
        self.emitted += 1;

        let slot_count = self.slots[event as usize].len();
        for k in 0..slot_count {
            let index = self.slots[event as usize][k].index as usize;
            self.receivers[index].on_rule_event(event, subject, value, time);
        }
    }

    /// Pruefsumme ueber die Ereignisfolge. Der Vergleichswert fuer das Spiel.
    #[must_use]
    pub const fn sequence_checksum(&self) -> u32 {
        self.hash
    }

    /// Wie viele Ereignisse ausgetragen wurden.
    #[must_use]
    pub const fn emitted_count(&self) -> u64 {
        self.emitted
    }

    /// Wie viele Empfaenger auf dieses Ereignis hoeren.
    #[must_use]
    pub fn receiver_count(&self, event: RuleEvent) -> usize {
        self.slots[event as usize].len()
    }

    /// Die Empfaenger dieses Ereignisses in ihrer Reihenfolge.
    ///
    /// Erzeugt eine Liste und gehoert deshalb nicht in den Schritt.
    #[must_use]
    pub fn receiver_ids(&self, event: RuleEvent) -> Vec<String> {
        self.slots[event as usize]
            .iter()
            .map(|slot| self.ids[slot.index as usize].clone())
            .collect()
    }

    /// Nur die Folge zuruecksetzen, die Anmeldungen behalten.
    pub fn clear_sequence(&mut self) {
        self.hash = FNV_OFFSET;
        self.emitted = 0;
    }

    /// Alles zuruecksetzen, auch die Anmeldungen.
    pub fn reset(&mut self) {
        self.receivers.clear();
        self.ids.clear();
        for list in &mut self.slots {
            list.clear();
        }
        self.next_seq = 0;
        self.clear_sequence();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Zaehler(u32);

    impl RuleReceiver for Zaehler {
        fn on_rule_event(&mut self, _event: RuleEvent, _subject: i16, _value: f64, _time: u32) {
            self.0 += 1;
        }
    }

    #[test]
    fn namen_und_kennnummern_passen_zusammen() {
        for index in 0..RuleEvent::COUNT {
            let event = RuleEvent::from_index(index).expect("Kennnummer muss es geben");
            assert_eq!(event as usize, index);
            assert_eq!(event.name(), RuleEvent::NAMES[index]);
        }
        assert!(RuleEvent::from_index(RuleEvent::COUNT).is_none());
    }

    #[test]
    fn empfaenger_bekommt_nur_sein_ereignis() {
        let mut bus = RuleEvents::with_capacity(2);
        bus.register("a", RuleEvent::Damage, 0, Box::<Zaehler>::default());
        bus.emit(RuleEvent::DotTick, 0, 40.0, 0);
        bus.emit(RuleEvent::Damage, 0, 100.0, 0);
        assert_eq!(bus.receiver_count(RuleEvent::Damage), 1);
        assert_eq!(bus.receiver_count(RuleEvent::DotTick), 0);
        assert_eq!(bus.emitted_count(), 2);
    }

    #[test]
    fn zuruecksetzen_leert_die_anmeldungen() {
        let mut bus = RuleEvents::with_capacity(1);
        bus.register("a", RuleEvent::Damage, 0, Box::<Zaehler>::default());
        bus.emit(RuleEvent::Damage, 0, 1.0, 0);
        bus.reset();
        assert_eq!(bus.receiver_count(RuleEvent::Damage), 0);
        assert_eq!(bus.emitted_count(), 0);
        assert_eq!(bus.sequence_checksum(), FNV_OFFSET);
    }
}
