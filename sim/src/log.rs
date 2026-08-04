//! Das Kampfprotokoll — ein Ringpuffer mit fester Groesse.
//!
//! GAME-DESIGN 18.2 zaehlt die Datenmenge des Protokolls zu den vier
//! Leistungs-Gefahren und schreibt als Gegenmassnahme vor:
//!
//! > **Vorab angelegter Ringpuffer mit gepackten Zahlen** (Zeitpunkt,
//! > Ereignisart, Zauber, Wert). ~10.000 Eintraege bei einem 10-Minuten-Kampf
//! > ≈ 160 KB, keine Objekte.
//!
//! Fuer den Simulator zaehlt vor allem der zweite Teil: Waechst das Protokoll
//! im Kampf mit, fordert es Speicher an — und dann ist die Regel *"keine
//! Speicheranforderung im Schritt"* verletzt. Ein Ringpuffer schreibt
//! stattdessen die aeltesten Eintraege ueber.
//!
//! Bei einem Lauf mit hunderttausend Durchgaengen interessiert das Protokoll
//! ohnehin nicht — dann wird es mit Kapazitaet 0 angelegt und kostet gar nichts.

use crate::time::Millis;

/// Art eines Protokoll-Eintrags — **ein gemeinsamer Nummernraum mit dem
/// Spiel** (Issue E3-6).
///
/// Bis dahin fuehrten beide Seiten eigene Nummern, und ab der 2 bedeutete
/// dieselbe Zahl etwas anderes: Eine 3 hiess hier "Entladung", im Spiel
/// "DoT-Tick"; eine 4 hier "Boss gefallen", dort "Phasenwechsel". Verglichen
/// hat das nie jemand — die Auswertung in Epic E7 liest aber genau diese
/// Daten, und eine verschobene Bedeutung ergaebe eine Fehleranalyse, die
/// plausibel aussieht und falsch ist.
///
/// **Jede Seite benutzt nur ihre Teilmenge.** Die Luecken bei 3, 6 und 7
/// gehoeren dem Spiel (Krit, Phasenwechsel, Verstaerkung laeuft ab); leere
/// Varianten zu fuehren, die der Simulator nie schreibt, waere toter Code.
///
/// | Nr. | Art          | Spiel | Simulator |
/// | --- | ------------ | ----- | --------- |
/// | 0   | `cast`       | ja    | ja        |
/// | 1   | `damage`     | ja    | ja        |
/// | 2   | `dotTick`    | ja    | ja        |
/// | 3   | `crit`       | ja    | —         |
/// | 4   | `spend`      | —     | ja        |
/// | 5   | `kill`       | —     | ja        |
/// | 6   | `phase`      | ja    | —         |
/// | 7   | `buffExpire` | ja    | —         |
///
/// `tests/log_kinds.rs` prueft die Tabelle gegen eine Datei, die aus dem
/// Spielcode stammt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogKind {
    /// Ein Zauber wurde gewirkt.
    Cast = 0,
    /// Ein Zauber hat beim Wirken Schaden verursacht.
    Damage = 1,
    /// Ein Brand-Effekt hat getickt.
    DotTick = 2,
    /// Ein Entlade-Zauber hat die Ressource verbraucht.
    Spend = 4,
    /// Der Boss ist gefallen.
    Kill = 5,
    /// Der Boss hat die Phase gewechselt. `value` ist die neue Phase.
    ///
    /// **Nummer 6 gehoerte bisher allein dem Spiel** (E3-6). Der Simulator
    /// schreibt sie seit Issue #96 ebenfalls — dieselbe Bedeutung, dieselbe
    /// Zahl. Genau dafuer wurde der gemeinsame Raum eingerichtet.
    BossPhase = 6,
    /// Die Energieleiste des Bosses hat **ausgeloest**. `value` zaehlt die
    /// Ausloesungen.
    ///
    /// Heisst im gemeinsamen Raum `bossEnergyTriggered`, und das `Triggered`
    /// ist wesentlich: Dies ist ein **Vorgang**, kein Zustand. Geschrieben
    /// wird nur im Moment des Ausloesens, nicht bei jedem Anstieg — der Stand
    /// der Leiste ist [`Fight::energy`] und etwas anderes. Beim ersten Lesen
    /// wurden die beiden verwechselt; daher die zwei unterscheidbaren Namen.
    ///
    /// Neu im gemeinsamen Raum (Issue #96). Nur der Simulator schreibt sie —
    /// wie `spend` und `kill` auch.
    BossEnergy = 8,
    /// Ein Eintrag der Boss-Zeitleiste ist faellig geworden. `value` ist sein
    /// Index in `Boss::timeline`.
    ///
    /// Was der Eintrag *bewirkt*, entscheidet sich in E4-2 bis E4-6 — hier
    /// steht vorerst nur, dass er zur richtigen Zeit gegriffen hat.
    BossTimeline = 9,
    /// Der Spieler hat Schaden genommen. `value` ist der Betrag **nach** WARD.
    ///
    /// Neu im gemeinsamen Raum (Issue #110).
    PlayerDamage = 10,
    /// Der Spieler ist gestorben. `value` ist der insgesamt erlittene Schaden.
    ///
    /// Keine Strafe (GAME-DESIGN A.3) — der Eintrag steht fuer die Auswertung,
    /// nicht fuer eine Buchung.
    PlayerDeath = 11,
    /// Der Spieler hat einen Boss-Zauber unterbrochen. `value` zaehlt sie.
    InterruptLanded = 12,
    /// Die Schutzschicht hat Schaden aufgefangen (COAGULATION, Weg 1).
    /// `value` ist der abgefangene Betrag.
    ShieldAbsorb = 13,
    /// Der Spieler wurde geheilt (Weg 2). `value` ist der zurueckgeholte Betrag.
    ///
    /// **Nicht der Betrag aus den Daten**, sondern der wirklich angekommene:
    /// Eine Heilung ueber die volle Lebensanzeige hinaus zaehlt nur, was
    /// gefehlt hat. Sonst saehe ein Lauf mit voller Anzeige nach mehr Heilung
    /// aus, als er gebraucht hat.
    Healed = 14,
    /// Ausgeteilter Schaden wurde in Schutz verwandelt (Weg 3).
    ConvertedToShield = 15,
    /// Kontroll-Punkte wurden ausgegeben (FIXATION, Issue #100).
    /// `value` ist der Betrag der Wirkung — Millisekunden oder Prozentpunkte.
    ControlSpent = 16,
}

/// Ein Eintrag. Gepackte Zahlen, keine Zeichenketten, kein Zeiger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Kampfzeit des Ereignisses.
    pub time: Millis,
    /// Art des Eintrags.
    pub kind: LogKind,
    /// Index des beteiligten Zaubers; `u8::MAX`, wenn keiner beteiligt ist.
    pub spell: u8,
    /// Schaden, verbrauchte Ressource oder 0.
    pub value: u32,
}

/// Kein Zauber beteiligt.
pub const NO_SPELL: u8 = u8::MAX;

/// Ringpuffer fester Groesse.
#[derive(Debug)]
pub struct CombatLog {
    entries: Vec<Entry>,
    /// Naechste Schreibstelle.
    head: usize,
    /// Wie viele Eintraege insgesamt geschrieben wurden — auch die bereits
    /// ueberschriebenen. Ohne diesen Zaehler saehe ein uebergelaufenes
    /// Protokoll aus wie ein vollstaendiges.
    written: u64,
}

impl CombatLog {
    /// Legt ein Protokoll mit Platz fuer `capacity` Eintraege an.
    ///
    /// `capacity == 0` schaltet das Protokoll ab: Es wird nichts angefordert und
    /// nichts geschrieben.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            head: 0,
            written: 0,
        }
    }

    /// Schreibt einen Eintrag.
    pub fn push(&mut self, time: Millis, kind: LogKind, spell: u8, value: u32) {
        let capacity = self.entries.capacity();
        if capacity == 0 {
            return;
        }
        let entry = Entry {
            time,
            kind,
            spell,
            value,
        };
        if self.entries.len() < capacity {
            // Waechst nur bis zur vorab angeforderten Kapazitaet — dieser Zweig
            // fordert deshalb keinen Speicher an.
            self.entries.push(entry);
        } else {
            self.entries[self.head] = entry;
        }
        self.head = (self.head + 1) % capacity;
        self.written += 1;
    }

    /// Die vorhandenen Eintraege in Schreibreihenfolge.
    ///
    /// Ist das Protokoll uebergelaufen, fehlen die aeltesten. Die Reihenfolge
    /// stimmt trotzdem: Nach dem Kopf kommt der aelteste erhaltene Eintrag.
    #[must_use]
    pub fn entries(&self) -> Vec<Entry> {
        if !self.overflowed() {
            return self.entries.clone();
        }
        let mut out = Vec::with_capacity(self.entries.len());
        out.extend_from_slice(&self.entries[self.head..]);
        out.extend_from_slice(&self.entries[..self.head]);
        out
    }

    /// Wie viele Eintraege insgesamt geschrieben wurden, auch ueberschriebene.
    #[must_use]
    pub fn written(&self) -> u64 {
        self.written
    }

    /// Ob Eintraege verloren gegangen sind.
    #[must_use]
    pub fn overflowed(&self) -> bool {
        self.written > self.entries.len() as u64
    }

    /// Wie viele Eintraege gerade vorliegen.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Ob nichts geschrieben wurde.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Leert das Protokoll, behaelt aber den Speicher.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.head = 0;
        self.written = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schreibt_in_reihenfolge() {
        let mut log = CombatLog::with_capacity(4);
        for i in 0..3u32 {
            log.push(i * 100, LogKind::Damage, 0, i);
        }
        let e = log.entries();
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].value, 0);
        assert_eq!(e[2].value, 2);
        assert!(!log.overflowed());
    }

    #[test]
    fn ueberschreibt_die_aeltesten() {
        let mut log = CombatLog::with_capacity(3);
        for i in 0..5u32 {
            log.push(i * 100, LogKind::Damage, 0, i);
        }
        let e = log.entries();
        assert_eq!(e.len(), 3, "mehr als die Kapazitaet wird nie aufbewahrt");
        assert_eq!(
            e.iter().map(|x| x.value).collect::<Vec<_>>(),
            vec![2, 3, 4],
            "die drei juengsten, in Schreibreihenfolge"
        );
        assert!(log.overflowed());
        assert_eq!(log.written(), 5);
    }

    #[test]
    fn kapazitaet_null_schreibt_nichts() {
        let mut log = CombatLog::with_capacity(0);
        log.push(0, LogKind::Damage, 0, 42);
        assert!(log.is_empty());
        assert_eq!(log.written(), 0);
    }

    #[test]
    fn leeren_behaelt_den_speicher() {
        let mut log = CombatLog::with_capacity(16);
        for i in 0..16u32 {
            log.push(i, LogKind::Cast, 0, 0);
        }
        let vorher = log.entries.capacity();
        log.clear();
        assert!(log.is_empty());
        assert_eq!(log.entries.capacity(), vorher);
    }
}
