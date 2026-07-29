//! Die Ereignis-Warteschlange.
//!
//! GAME-DESIGN 18.5 nennt als erstes von SimulationCraft uebernommenes Konzept:
//!
//! > **Ereignisgesteuert** statt fester Takt — zwischen zwei Tastendruecken
//! > passiert ausser DoT-Ticks nichts.
//!
//! Ein Kampf ueber 60 Sekunden hat bei 10 ms Takt 6.000 Schritte, von denen rund
//! 5.950 nichts tun. Der Simulator springt stattdessen von Ereignis zu Ereignis.
//!
//! # Warum die Reihenfolge so streng festgelegt ist
//!
//! Aus 18.4: *"Feste Ereignis-Reihenfolge, keine Hash-Container. Sonst weichen
//! Spiel und Simulator ab — und man sucht tagelang."*
//!
//! Treffen mehrere Ereignisse auf dieselbe Millisekunde, entscheidet die
//! Reihenfolge ueber das Ergebnis. Ein Beispiel aus Schritt 1: Bei 12.000 ms
//! fallen ein DoT-Tick (+1 Hitze) und ein Verfall-Schritt (−1 Hitze) zusammen.
//! Beide Reihenfolgen ergeben denselben Hitze-Stand — aber nur, weil hier kein
//! Deckel greift. Steht die Hitze bei 20, verpufft der Tick in der einen
//! Reihenfolge und wird in der anderen mitgenommen. Deshalb ist die Rangfolge
//! Teil der Regeln und nicht dem Zufall der Sammlung ueberlassen.
//!
//! Die Rangfolge bildet die Takt-Schleife des Spiels ab
//! (`src/devtools/reference-run.ts`, `tools/step1-check.py`):
//! erst die passiven Wirkungen, dann die Entscheidung des Spielers.

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::time::Millis;

/// Was zu einem Zeitpunkt geschieht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// Ein Brand-Effekt tickt.
    DotTick {
        /// Index des Zaubers in `data::SPELLS`.
        spell: u8,
        /// Welche Anwendung des Effekts gemeint ist.
        ///
        /// Beim Erneuern verfaellt der alte Effekt (`refreshMode: "reset"`),
        /// seine bereits eingeplanten Ticks liegen aber noch in der Schlange.
        /// Ohne diesen Zaehler wuerde ein alter Tick, der zufaellig auf denselben
        /// Zeitpunkt faellt wie ein neuer, **doppelt** zuschlagen.
        generation: u16,
    },
    /// Die Ressource verfaellt um einen Schritt.
    ResourceDecay,
    /// Die Energieleiste des Bosses steigt um einen Schritt.
    ///
    /// Die zweite Uhr (GAME-DESIGN 11.2, Baustein 1). Sie steht **nach** dem
    /// Ressourcen-Verfall in der Rangfolge, weil beides passive Wirkungen sind
    /// und die Reihenfolge festliegen muss — nicht, weil eine wichtiger waere.
    EnergyTick,
    /// Nur aufwachen: Zu diesem Zeitpunkt kann sich eine Entscheidung ergeben
    /// (Abklingzeit oder globale Wartezeit ist abgelaufen), oder es ist zu
    /// pruefen, ob der Boss gefallen ist.
    Wakeup,
}

impl EventKind {
    /// Rang innerhalb derselben Millisekunde. Kleiner heisst frueher.
    ///
    /// Passive Wirkungen zuerst, in der Reihenfolge der Referenz
    /// (`step_passive`: erst der Tick, dann der Verfall), danach alles, was zu
    /// einer Entscheidung fuehren kann.
    const fn rank(self) -> u8 {
        match self {
            EventKind::DotTick { .. } => 0,
            EventKind::ResourceDecay => 1,
            EventKind::EnergyTick => 2,
            EventKind::Wakeup => 3,
        }
    }
}

/// Ein eingeplantes Ereignis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Wann es faellig ist.
    pub time: Millis,
    /// Was geschieht.
    pub kind: EventKind,
    /// Einfuege-Nummer. Loest Gleichstand auf, damit die Reihenfolge auch bei
    /// gleicher Zeit **und** gleichem Rang eindeutig ist.
    seq: u32,
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time
            .cmp(&other.time)
            .then_with(|| self.kind.rank().cmp(&other.kind.rank()))
            .then_with(|| self.seq.cmp(&other.seq))
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Die Warteschlange.
///
/// Eine Halde mit **vorab angeforderter** Kapazitaet: Solange sie nicht
/// ueberlaeuft, fordert weder `push` noch `pop` Speicher an. Das ist die Regel
/// *"Keine Speicheranforderung im Schritt, zusammenhaengende Listen"* aus 18.4;
/// `tests/allocation.rs` misst nach, dass sie eingehalten wird.
#[derive(Debug)]
pub struct EventQueue {
    heap: BinaryHeap<Reverse<Event>>,
    next_seq: u32,
}

impl EventQueue {
    /// Legt eine Schlange mit Platz fuer `capacity` gleichzeitig offene
    /// Ereignisse an.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: BinaryHeap::with_capacity(capacity),
            next_seq: 0,
        }
    }

    /// Plant ein Ereignis ein.
    pub fn push(&mut self, time: Millis, kind: EventKind) {
        let event = Event {
            time,
            kind,
            seq: self.next_seq,
        };
        self.next_seq += 1;
        self.heap.push(Reverse(event));
    }

    /// Zeitpunkt des naechsten faelligen Ereignisses, ohne es zu entnehmen.
    #[must_use]
    pub fn peek_time(&self) -> Option<Millis> {
        self.heap.peek().map(|Reverse(e)| e.time)
    }

    /// Entnimmt das naechste faellige Ereignis.
    pub fn pop(&mut self) -> Option<Event> {
        self.heap.pop().map(|Reverse(e)| e)
    }

    /// Wie viele Ereignisse offen sind.
    #[must_use]
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Ob die Schlange leer ist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Platz, der ohne neue Speicheranforderung zur Verfuegung steht.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.heap.capacity()
    }

    /// Leert die Schlange, behaelt aber den Speicher.
    pub fn clear(&mut self) {
        self.heap.clear();
        self.next_seq = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frueheste_zeit_kommt_zuerst() {
        let mut q = EventQueue::with_capacity(8);
        q.push(3000, EventKind::Wakeup);
        q.push(1000, EventKind::ResourceDecay);
        q.push(2000, EventKind::Wakeup);

        assert_eq!(q.pop().map(|e| e.time), Some(1000));
        assert_eq!(q.pop().map(|e| e.time), Some(2000));
        assert_eq!(q.pop().map(|e| e.time), Some(3000));
        assert!(q.pop().is_none());
    }

    #[test]
    fn bei_gleicher_zeit_entscheidet_der_rang() {
        let mut q = EventQueue::with_capacity(8);
        // Bewusst in der falschen Reihenfolge eingefuegt.
        q.push(2000, EventKind::Wakeup);
        q.push(2000, EventKind::ResourceDecay);
        q.push(
            2000,
            EventKind::DotTick {
                spell: 1,
                generation: 0,
            },
        );

        assert!(matches!(
            q.pop().map(|e| e.kind),
            Some(EventKind::DotTick { .. })
        ));
        assert_eq!(q.pop().map(|e| e.kind), Some(EventKind::ResourceDecay));
        assert_eq!(q.pop().map(|e| e.kind), Some(EventKind::Wakeup));
    }

    #[test]
    fn bei_gleichem_rang_entscheidet_die_einfuege_reihenfolge() {
        let mut q = EventQueue::with_capacity(8);
        q.push(
            500,
            EventKind::DotTick {
                spell: 7,
                generation: 0,
            },
        );
        q.push(
            500,
            EventKind::DotTick {
                spell: 9,
                generation: 0,
            },
        );

        assert_eq!(
            q.pop().map(|e| e.kind),
            Some(EventKind::DotTick {
                spell: 7,
                generation: 0
            })
        );
        assert_eq!(
            q.pop().map(|e| e.kind),
            Some(EventKind::DotTick {
                spell: 9,
                generation: 0
            })
        );
    }

    #[test]
    fn peek_time_entnimmt_nicht() {
        let mut q = EventQueue::with_capacity(4);
        q.push(1234, EventKind::Wakeup);
        assert_eq!(q.peek_time(), Some(1234));
        assert_eq!(q.peek_time(), Some(1234));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn leeren_behaelt_den_speicher() {
        let mut q = EventQueue::with_capacity(64);
        let vorher = q.capacity();
        for t in 0..50 {
            q.push(t * 10, EventKind::Wakeup);
        }
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.capacity(), vorher, "clear darf nicht neu anfordern");
    }
}
