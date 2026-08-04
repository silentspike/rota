//! Zufall — **je Quelle ein eigener Strom**, und derselbe Erzeuger wie im Spiel.
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
//! # Warum mulberry32 und nicht irgendein guter Erzeuger
//!
//! Weil das Spiel ihn benutzt. `src/core/rng.ts` sagt es selbst:
//!
//! > klein, schnell, ohne Speicheranforderung je Ziehung und ueber
//! > Sprachgrenzen hinweg in wenigen Zeilen identisch nachzubauen — **die
//! > Bedingung dafuer, dass der Rust-Simulator dieselbe Folge liefert**.
//!
//! Ein anderer Erzeuger waere hier besonders heimtueckisch: In Schritt 1 zieht
//! niemand (PURITY = 0, kein Krit — `STEP1-SPEC` § 2), also bestuende ein
//! Simulator mit fremdem Erzeuger heute **jeden** gemeinsamen Pruefall. Reissen
//! wuerde es erst ab Schritt 3, mit dem ersten Krit — und dann sucht man den
//! Fehler im Kampfablauf, wo er nicht ist.
//!
//! `tests/parity_primitives.rs` prueft die Folge deshalb gegen Werte, die aus
//! dem Spielcode selbst stammen: `tests/parity/recordings/primitives.json`.
//!
//! Die Stroeme sind von Anfang an angelegt, obwohl noch niemand zieht — sie
//! nachtraeglich einzuziehen hiesse, jede bis dahin aufgezeichnete Runde zu
//! entwerten.

/// Woher eine Zufallszahl stammt. Jede Quelle hat ihren eigenen Strom.
///
/// Die Zahlenwerte sind Teil der Schnittstelle: Sie bestimmen die Reihenfolge in
/// der Zustands-Pruefsumme und muessen zu `STREAM_CRIT`, `STREAM_DOT_TICK` und
/// `STREAM_HEAL` aus `src/core/rng.ts` passen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Ob ein Treffer kritisch ist (ab Schritt 3).
    Crit = 0,
    /// Streuung auf DoT-Ticks.
    DotTick = 1,
    /// Heilung.
    ///
    /// **Bleibt reserviert, obwohl heute nichts daraus zieht.** Alle drei
    /// Koagulations-Wege sind deterministisch (Design 21.1 verwirft
    /// Zufalls-Procs ausser Krit). Ihn zu entfernen und spaeter wieder
    /// einzufuegen wuerde jede aufgezeichnete Pruefsumme zweimal verschieben.
    Heal = 2,
}

/// Anzahl der Stroeme.
pub const STREAM_COUNT: usize = 3;

impl Stream {
    /// Alle Stroeme in fester Reihenfolge — fuer Tests und Berichte.
    pub const ALL: [Stream; STREAM_COUNT] = [Stream::Crit, Stream::DotTick, Stream::Heal];

    /// Name des Stroms, wie ihn `STREAM_NAMES` in `src/core/rng.ts` fuehrt.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Stream::Crit => "crit",
            Stream::DotTick => "dotTick",
            Stream::Heal => "heal",
        }
    }
}

/// Ein einzelner Strom.
///
/// Der Zustand ist eine einzige 32-Bit-Zahl — damit vollstaendig speicherbar und
/// exakt wiederherstellbar.
#[derive(Debug, Clone, Copy)]
pub struct RandomStream {
    seed: u32,
    state: u32,
    draws: u32,
}

impl RandomStream {
    /// Legt einen Strom mit dem gegebenen Startwert an.
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            state: seed,
            draws: 0,
        }
    }

    /// Der Startwert.
    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }

    /// Zahl der Ziehungen. In Schritt 1 muss sie 0 bleiben.
    #[must_use]
    pub const fn draws(&self) -> u32 {
        self.draws
    }

    /// Die naechste Zahl, roh ueber 32 Bit.
    ///
    /// Das Spiel gibt an dieser Stelle `wert / 4294967296` zurueck; die rohe Zahl
    /// ist derselbe Schritt ohne die Division und laesst sich exakt vergleichen.
    pub fn next_u32(&mut self) -> u32 {
        self.draws += 1;
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    /// Die naechste Zahl in `[0, 1)` — wie `next()` im Spiel.
    pub fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }

    /// Trifft eine Wahrscheinlichkeit in `[0, 1]`.
    ///
    /// Die beiden Kurzschluesse sind **nicht** nur Bequemlichkeit: Ein Wurf,
    /// dessen Ergebnis feststeht, wuerde den Strom weiterdrehen und damit alles
    /// Nachfolgende verschieben. `src/core/rng.ts` macht es genauso.
    pub fn chance(&mut self, probability: f64) -> bool {
        if probability <= 0.0 {
            return false;
        }
        if probability >= 1.0 {
            return true;
        }
        self.next_f64() < probability
    }

    /// Setzt den Strom auf seinen Startwert zurueck.
    pub const fn reset(&mut self) {
        self.state = self.seed;
        self.draws = 0;
    }
}

/// Alle Stroeme eines Kampfes, aus einem einzigen Startwert abgeleitet.
#[derive(Debug, Clone)]
pub struct Streams {
    seed: u32,
    streams: [RandomStream; STREAM_COUNT],
}

impl Streams {
    /// Legt die Stroeme zu einem Startwert an.
    ///
    /// Die Ableitung ist dieselbe wie in `src/core/rng.ts`:
    /// `(seed * 0x9e3779b1 + i * 0x85ebca6b) mod 2^32`. Ohne diese Streuung
    /// lieferten alle drei Stroeme dieselbe Folge.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        let mut streams = [RandomStream::new(0); STREAM_COUNT];
        for (i, slot) in streams.iter_mut().enumerate() {
            let offset = (i as u32).wrapping_mul(0x85eb_ca6b);
            *slot = RandomStream::new(seed.wrapping_mul(0x9e37_79b1).wrapping_add(offset));
        }
        Self { seed, streams }
    }

    /// Der Startwert, aus dem diese Stroeme entstanden sind.
    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }

    /// Ein Strom, schreibend.
    pub fn stream(&mut self, stream: Stream) -> &mut RandomStream {
        &mut self.streams[stream as usize]
    }

    /// Startwert eines einzelnen Stroms, wie ihn die Ableitung ergeben hat.
    ///
    /// Fuer den Abgleich mit dem Spiel: Stimmt schon dieser Wert nicht, liegt
    /// der Fehler in der Ableitung und nicht im Erzeuger — ein Unterschied, der
    /// bei der Suche Stunden spart.
    #[must_use]
    pub fn stream_seed(&self, stream: Stream) -> u32 {
        self.streams[stream as usize].seed()
    }

    /// Zieht die naechste Zahl aus einem Strom.
    pub fn next_u32(&mut self, stream: Stream) -> u32 {
        self.streams[stream as usize].next_u32()
    }

    /// Trifft eine Wahrscheinlichkeit aus dem angegebenen Strom.
    pub fn chance(&mut self, stream: Stream, probability: f64) -> bool {
        self.streams[stream as usize].chance(probability)
    }

    /// Wie oft aus einem Strom gezogen wurde.
    #[must_use]
    pub fn draws(&self, stream: Stream) -> u32 {
        self.streams[stream as usize].draws()
    }

    /// Ziehungen je Strom, in der Reihenfolge der Schnittstelle — so gehen sie
    /// in die Zustands-Pruefsumme ein.
    #[must_use]
    pub fn draw_counts(&self) -> [u32; STREAM_COUNT] {
        let mut out = [0; STREAM_COUNT];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.streams[i].draws();
        }
        out
    }

    /// Ziehungen ueber alle Stroeme. In Schritt 1 muss das 0 sein.
    #[must_use]
    pub fn total_draws(&self) -> u32 {
        self.streams.iter().map(RandomStream::draws).sum()
    }

    /// Setzt alle Stroeme auf ihre Startwerte zurueck.
    pub fn reset(&mut self) {
        for stream in &mut self.streams {
            stream.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleicher_startwert_gleiche_folge() {
        let mut a = Streams::new(42);
        let mut b = Streams::new(42);
        for _ in 0..64 {
            assert_eq!(a.next_u32(Stream::Crit), b.next_u32(Stream::Crit));
        }
    }

    #[test]
    fn verschiedene_startwerte_verschiedene_folge() {
        let mut a = Streams::new(1);
        let mut b = Streams::new(2);
        assert_ne!(a.next_u32(Stream::Crit), b.next_u32(Stream::Crit));
    }

    #[test]
    fn die_drei_stroeme_liefern_verschiedene_folgen() {
        // Ohne die Streuung im Startwert waeren alle drei gleich — und die
        // Trennung der Stroeme waere nur auf dem Papier vorhanden.
        let mut s = Streams::new(7);
        let crit = s.next_u32(Stream::Crit);
        let dot = s.next_u32(Stream::DotTick);
        let heal = s.next_u32(Stream::Heal);
        assert_ne!(crit, dot);
        assert_ne!(dot, heal);
        assert_ne!(crit, heal);
    }

    #[test]
    fn stroeme_laufen_unabhaengig() {
        // Die Regel aus 18.4 in ihrer kuerzesten Form: Ziehungen aus dem einen
        // Strom duerfen den anderen nicht bewegen.
        let mut ohne = Streams::new(7);
        let mut mit = Streams::new(7);

        let erwartet: Vec<u32> = (0..8).map(|_| ohne.next_u32(Stream::DotTick)).collect();

        for _ in 0..100 {
            mit.next_u32(Stream::Crit);
        }
        let tatsaechlich: Vec<u32> = (0..8).map(|_| mit.next_u32(Stream::DotTick)).collect();

        assert_eq!(erwartet, tatsaechlich);
    }

    #[test]
    fn ziehungen_in_null_eins() {
        let mut s = Streams::new(3);
        for _ in 0..256 {
            let value = s.stream(Stream::Crit).next_f64();
            assert!(
                (0.0..1.0).contains(&value),
                "{value} liegt ausserhalb [0, 1)"
            );
        }
    }

    #[test]
    fn sichere_entscheidungen_ziehen_nicht() {
        let mut s = Streams::new(3);
        assert!(!s.chance(Stream::Crit, 0.0));
        assert!(!s.chance(Stream::Crit, -1.0));
        assert!(s.chance(Stream::Crit, 1.0));
        assert!(s.chance(Stream::Crit, 2.0));
        assert_eq!(
            s.draws(Stream::Crit),
            0,
            "eine Entscheidung, die feststeht, darf den Strom nicht weiterdrehen"
        );
    }

    #[test]
    fn ziehungen_werden_je_strom_gezaehlt() {
        let mut s = Streams::new(9);
        s.next_u32(Stream::Crit);
        s.next_u32(Stream::Crit);
        s.next_u32(Stream::Heal);
        assert_eq!(s.draw_counts(), [2, 0, 1]);
        assert_eq!(s.total_draws(), 3);
    }

    /// **Waechter gegen die Rueckkehr des Beute-Stroms** (Issue E6-2).
    ///
    /// Wer ihn hier eintraegt — aus dem naheliegenden Reflex, alle
    /// Zufallsquellen an einer Stelle zu haben — verschiebt stillschweigend
    /// JEDE aufgezeichnete Pruefsumme: 23 463 Zeilen in fuenf
    /// Vergleichsdateien, am 2026-08-04 gemessen.
    #[test]
    fn der_beute_strom_gehoert_nicht_zu_den_kampf_stroemen() {
        assert_eq!(
            STREAM_COUNT, 3,
            "Der Beute-Strom gehoert NICHT zu den Kampf-Stroemen (Entscheidung vom \
             2026-08-04, GAME-DESIGN 20.2). Beute wird beim SIEG gewuerfelt, nicht im \
             Kampf — sie kann nichts verschieben. Ihn hier einzutragen haengt jedem Takt \
             eine Null an die Zustands-Pruefsumme und verschiebt alle aufgezeichneten \
             Laeufe. Er steht in src/loot/stream.ts."
        );
        let namen: Vec<&str> = Stream::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(namen, vec!["crit", "dotTick", "heal"]);
        assert!(!namen.contains(&"loot"));
    }

    #[test]
    fn zuruecksetzen_stellt_die_folge_wieder_her() {
        let mut s = Streams::new(11);
        let erste: Vec<u32> = (0..4).map(|_| s.next_u32(Stream::Crit)).collect();
        s.reset();
        let zweite: Vec<u32> = (0..4).map(|_| s.next_u32(Stream::Crit)).collect();
        assert_eq!(erste, zweite);
        assert_eq!(s.draws(Stream::Crit), 4);
    }
}
