//! Zufall — **je Quelle ein eigener Strom**.
//!
//! Aus GAME-DESIGN 18.4:
//!
//! > Krit, DoT-Ticks und Heilung brauchen **je einen eigenen Strom** — sonst
//! > verschiebt eine zusaetzliche Ziehung alles Nachfolgende.
//!
//! Das ist keine Feinheit, sondern der Unterschied zwischen einem brauchbaren
//! und einem unbrauchbaren Simulator. Mit einem gemeinsamen Strom aendert ein
//! einziger zusaetzlicher Krit-Wurf — etwa weil ein Talent einen Treffer mehr
//! erzeugt — saemtliche folgenden DoT-Ticks. Zwei Laeufe, die sich nur in einem
//! Talent unterscheiden sollten, unterscheiden sich dann ueberall, und die
//! gemessene Wirkung des Talents ist Rauschen.
//!
//! **In Schritt 1 zieht noch niemand.** Auf Stufe 1 ist PURITY = 0, also gibt es
//! keinen Krit (`STEP1-SPEC` § 2). Die Stroeme sind trotzdem von Anfang an
//! angelegt — sie nachtraeglich einzuziehen hiesse, jede bis dahin
//! aufgezeichnete Runde zu entwerten.
//!
//! # Warum ein eigener Generator statt einer fertigen Kiste
//!
//! Der Simulator muss **ueber Jahre** dieselbe Zahlenfolge liefern: Aufgezeichnete
//! Runden sind Regressionstests (GAME-DESIGN 19.1), und die verlieren ihren Wert,
//! sobald eine Abhaengigkeit ihren Generator austauscht. SplitMix64 ist
//! veroeffentlicht, in 20 Zeilen vollstaendig, und liefert auf jeder Maschine
//! dasselbe Ergebnis.

/// Woher eine Zufallszahl stammt. Jede Quelle hat ihren eigenen Strom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Ob ein Treffer kritisch ist (ab Schritt 3).
    Crit = 0,
    /// Streuung auf DoT-Ticks.
    DotTick = 1,
    /// Heilung.
    Heal = 2,
}

/// Anzahl der Stroeme.
pub const STREAM_COUNT: usize = 3;

impl Stream {
    /// Alle Stroeme in fester Reihenfolge — fuer Tests und Berichte.
    pub const ALL: [Stream; STREAM_COUNT] = [Stream::Crit, Stream::DotTick, Stream::Heal];
}

/// Die Zufallsstroeme eines Kampfes.
///
/// Wird beim Aufbau des Kampfes angelegt und danach nur noch fortgeschrieben —
/// keine Speicheranforderung im Schritt.
#[derive(Debug, Clone)]
pub struct Streams {
    /// Startwert, aus dem alle Stroeme abgeleitet wurden.
    seed: u64,
    /// Zustand je Strom.
    state: [u64; STREAM_COUNT],
    /// Wie oft aus jedem Strom gezogen wurde.
    draws: [u32; STREAM_COUNT],
}

/// Streuwert je Strom, damit zwei Stroeme aus demselben Startwert nicht
/// dieselbe Folge liefern. Die Zahlen sind beliebig, aber **fest**: Sie zu
/// aendern hiesse, jede aufgezeichnete Runde ungueltig zu machen.
const STREAM_SALT: [u64; STREAM_COUNT] = [
    0x9E37_79B9_7F4A_7C15, // Crit
    0xBF58_476D_1CE4_E5B9, // DotTick
    0x94D0_49BB_1331_11EB, // Heal
];

impl Streams {
    /// Legt die Stroeme zu einem Startwert an.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut state = [0u64; STREAM_COUNT];
        for (i, slot) in state.iter_mut().enumerate() {
            // Der Startwert wird je Strom gestreut und einmal durchgemischt.
            // Ohne das Durchmischen laegen die Anfangszustaende dicht beieinander,
            // und die ersten Ziehungen waeren zwischen den Stroemen sichtbar aehnlich.
            *slot = mix(seed ^ STREAM_SALT[i]);
        }
        Self {
            seed,
            state,
            draws: [0; STREAM_COUNT],
        }
    }

    /// Der Startwert, aus dem diese Stroeme entstanden sind.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Zieht die naechste Zahl aus einem Strom.
    pub fn next_u64(&mut self, stream: Stream) -> u64 {
        let i = stream as usize;
        self.state[i] = self.state[i].wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.draws[i] += 1;
        mix(self.state[i])
    }

    /// Entscheidet mit einer Wahrscheinlichkeit in Zehntelprozent (0…1000).
    ///
    /// Ganzzahlig gerechnet, damit dieselbe Entscheidung auf jeder Maschine
    /// gleich ausfaellt — ein Vergleich zweier Fliesskommazahlen waere hier
    /// genau die Art von Detail, die Spiel und Simulator auseinanderlaufen laesst.
    pub fn chance_per_mille(&mut self, stream: Stream, per_mille: u32) -> bool {
        if per_mille == 0 {
            // Wichtig: **keine Ziehung**. Ein Wurf, dessen Ergebnis feststeht,
            // wuerde den Strom weiterdrehen und damit alles Folgende verschieben.
            return false;
        }
        if per_mille >= 1000 {
            return true;
        }
        (self.next_u64(stream) % 1000) < u64::from(per_mille)
    }

    /// Wie oft aus einem Strom gezogen wurde.
    #[must_use]
    pub fn draws(&self, stream: Stream) -> u32 {
        self.draws[stream as usize]
    }

    /// Ziehungen ueber alle Stroeme. In Schritt 1 muss das 0 sein.
    #[must_use]
    pub fn total_draws(&self) -> u32 {
        self.draws.iter().sum()
    }
}

/// SplitMix64 — der Mischschritt.
///
/// Quelle: Steele, Lea, Flood, "Fast Splittable Pseudorandom Number Generators"
/// (2014). Die Konstanten sind Teil des Verfahrens und duerfen nicht angepasst
/// werden.
const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleicher_startwert_gleiche_folge() {
        let mut a = Streams::new(42);
        let mut b = Streams::new(42);
        for _ in 0..64 {
            assert_eq!(a.next_u64(Stream::Crit), b.next_u64(Stream::Crit));
        }
    }

    #[test]
    fn verschiedene_startwerte_verschiedene_folge() {
        let mut a = Streams::new(1);
        let mut b = Streams::new(2);
        assert_ne!(a.next_u64(Stream::Crit), b.next_u64(Stream::Crit));
    }

    #[test]
    fn stroeme_laufen_unabhaengig() {
        // Das ist die Regel aus 18.4 in ihrer kuerzesten Form: Ziehungen aus dem
        // einen Strom duerfen den anderen nicht bewegen.
        let mut ohne = Streams::new(7);
        let mut mit = Streams::new(7);

        let erwartet: Vec<u64> = (0..8).map(|_| ohne.next_u64(Stream::DotTick)).collect();

        for _ in 0..100 {
            mit.next_u64(Stream::Crit);
        }
        let tatsaechlich: Vec<u64> = (0..8).map(|_| mit.next_u64(Stream::DotTick)).collect();

        assert_eq!(erwartet, tatsaechlich);
    }

    #[test]
    fn null_prozent_zieht_nicht() {
        let mut s = Streams::new(3);
        assert!(!s.chance_per_mille(Stream::Crit, 0));
        assert_eq!(
            s.draws(Stream::Crit),
            0,
            "eine Entscheidung, die feststeht, darf den Strom nicht weiterdrehen"
        );
    }

    #[test]
    fn hundert_prozent_zieht_nicht() {
        let mut s = Streams::new(3);
        assert!(s.chance_per_mille(Stream::Crit, 1000));
        assert_eq!(s.draws(Stream::Crit), 0);
    }

    #[test]
    fn ziehungen_werden_je_strom_gezaehlt() {
        let mut s = Streams::new(9);
        s.next_u64(Stream::Crit);
        s.next_u64(Stream::Crit);
        s.next_u64(Stream::Heal);
        assert_eq!(s.draws(Stream::Crit), 2);
        assert_eq!(s.draws(Stream::DotTick), 0);
        assert_eq!(s.draws(Stream::Heal), 1);
        assert_eq!(s.total_draws(), 3);
    }
}
