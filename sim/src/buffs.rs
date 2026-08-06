//! Verstaerkungen und Synergie-Fenster (GAME-DESIGN 5, Issue E3-5).
//!
//! Das Gegenstueck zu `src/combat/schools/buffs.ts`. Zwei Regeln:
//!
//! | Regel                | Festlegung                                        |
//! | -------------------- | ------------------------------------------------- |
//! | **Stapel-Regel**     | Verschiedene Arten **multiplikativ**, gleiche **ersetzen** |
//! | **Synergie-Fenster** | in globalen **Wartezeiten**, nicht in Sekunden     |
//!
//! # Warum das Fenster in Wartezeiten zaehlt
//!
//! Ein Fenster von 4,5 Sekunden ist bei 1,5 s Grundtakt Platz fuer drei
//! Zauber. Sinkt die Wartezeit per Talent auf 1,0 s, waeren es viereinhalb —
//! dasselbe Fenster erlaubte eine andere Rotation, ohne dass jemand es
//! entschieden haette.
//!
//! # Die Reihenfolge der Multiplikation ist Schnittstelle
//!
//! Fliesskomma-Multiplikation ist nicht assoziativ (GAME-DESIGN 18.4).
//! Multipliziert wird deshalb in der Reihenfolge der Arten, hier wie im Spiel;
//! `tests/buffs.rs` prueft es gegen eine Datei, die aus dem Spielcode stammt.

use crate::time::Millis;

/// Wie viele Bonus-Arten gleichzeitig gefuehrt werden koennen.
///
/// Vorab angelegt, damit im Schritt nichts entsteht (GAME-DESIGN 18.4).
pub const BUFF_KIND_CAPACITY: usize = 8;

/// Keine Verstaerkung dieser Art.
const NOT_ACTIVE: i64 = -1;

/// Die laufenden Verstaerkungen.
///
/// Je Art hoechstens eine — die Stapel-Regel steckt in der Datenstruktur und
/// nicht in einer Pruefung: Zwei gleiche Arten koennen gar nicht nebeneinander
/// stehen.
#[derive(Debug, Clone)]
pub struct Buffs {
    factors: [f64; BUFF_KIND_CAPACITY],
    expires_at: [i64; BUFF_KIND_CAPACITY],
    replacements: u32,
    expirations: u32,
}

impl Default for Buffs {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffs {
    /// Legt die Verstaerkungen an — alle neutral.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            factors: [1.0; BUFF_KIND_CAPACITY],
            expires_at: [NOT_ACTIVE; BUFF_KIND_CAPACITY],
            replacements: 0,
            expirations: 0,
        }
    }

    /// Wie oft eine Art ersetzt wurde, statt danebenzustehen.
    #[must_use]
    pub const fn replacement_count(&self) -> u32 {
        self.replacements
    }

    /// Wie viele Verstaerkungen abgelaufen sind.
    #[must_use]
    pub const fn expiration_count(&self) -> u32 {
        self.expirations
    }

    /// Eine Verstaerkung anlegen, deren Fenster in **Wartezeiten** zaehlt.
    ///
    /// `gcd_length_ms` ist die **aktuelle** Wartezeit: Sinkt sie per Talent,
    /// wird das Fenster in Sekunden kuerzer — genau das ist der Zweck.
    pub fn apply_for_gcds(
        &mut self,
        kind: usize,
        factor: f64,
        gcds: u32,
        now: Millis,
        gcd_length_ms: Millis,
    ) -> bool {
        let window = i64::from(gcds) * i64::from(gcd_length_ms);
        self.apply(kind, factor, i64::from(now) + window, now)
    }

    /// Eine Verstaerkung mit ausdruecklichem Ablauf anlegen.
    ///
    /// **Gleiche Art ersetzt** — auch dann, wenn die neue schwaecher ist oder
    /// frueher ablaeuft. "Ersetzen einander" steht ohne Einschraenkung im
    /// Design; eine Regel "die staerkere gewinnt" waere eine zweite, die dort
    /// nicht steht.
    pub fn apply(&mut self, kind: usize, factor: f64, expires_at: i64, now: Millis) -> bool {
        if kind >= BUFF_KIND_CAPACITY {
            return false;
        }
        if self.is_active(kind, now) {
            self.replacements += 1;
        }
        self.factors[kind] = factor;
        self.expires_at[kind] = expires_at;
        true
    }

    /// Laeuft diese Art gerade?
    #[must_use]
    pub fn is_active(&self, kind: usize, now: Millis) -> bool {
        match self.expires_at.get(kind) {
            Some(&until) => until != NOT_ACTIVE && until > i64::from(now),
            None => false,
        }
    }

    /// Restzeit dieser Art in ms, 0 wenn sie nicht laeuft.
    #[must_use]
    pub fn remaining_ms(&self, kind: usize, now: Millis) -> Millis {
        if !self.is_active(kind, now) {
            return 0;
        }
        let left = self.expires_at[kind] - i64::from(now);
        Millis::try_from(left).unwrap_or(0)
    }

    /// Der Faktor aller laufenden Verstaerkungen — **multiplikativ**, in der
    /// Reihenfolge der Arten.
    #[must_use]
    pub fn factor(&self, now: Millis) -> f64 {
        let mut product = 1.0;
        for kind in 0..BUFF_KIND_CAPACITY {
            if !self.is_active(kind, now) {
                continue;
            }
            product *= self.factors[kind];
        }
        product
    }

    /// Wie viele Arten gerade laufen.
    #[must_use]
    pub fn active_count(&self, now: Millis) -> u32 {
        let mut count = 0;
        for kind in 0..BUFF_KIND_CAPACITY {
            if self.is_active(kind, now) {
                count += 1;
            }
        }
        count
    }

    /// Abgelaufene Verstaerkungen einsammeln.
    ///
    /// @returns wie viele Arten in diesem Aufruf abgelaufen sind
    pub fn expire(&mut self, now: Millis) -> u32 {
        let mut ended = 0;
        for kind in 0..BUFF_KIND_CAPACITY {
            let until = self.expires_at[kind];
            if until == NOT_ACTIVE || until > i64::from(now) {
                continue;
            }
            self.expires_at[kind] = NOT_ACTIVE;
            self.factors[kind] = 1.0;
            ended += 1;
            self.expirations += 1;
        }
        ended
    }

    /// Alles auf neutral zuruecksetzen.
    /// Die Summe aller offenen Fensterenden — fuer die Pruefsumme (#493).
    ///
    /// **Eine Summe statt einzelner Felder**, weil die Pruefsumme je Takt
    /// laeuft. Dieselbe Bauart wie `synergy_window_sum` (#318), und dieselbe
    /// Rechnung wie `windowSum()` auf der Spiel-Seite — nicht offene Fenster
    /// tragen 0 und nicht `NOT_ACTIVE`, sonst haenge die Summe an einer
    /// Sentinel-Zahl, die auf beiden Seiten verschieden sein duerfte.
    pub fn window_sum(&self) -> f64 {
        let mut summe = 0.0;
        for i in 0..BUFF_KIND_CAPACITY {
            if self.expires_at[i] != NOT_ACTIVE {
                summe += self.expires_at[i] as f64;
            }
        }
        summe
    }

    /// Alles auf neutral zuruecksetzen.
    pub fn reset(&mut self) {
        self.factors = [1.0; BUFF_KIND_CAPACITY];
        self.expires_at = [NOT_ACTIVE; BUFF_KIND_CAPACITY];
        self.replacements = 0;
        self.expirations = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::GCD_MS;

    #[test]
    fn fenster_haengt_an_der_wartezeit() {
        let mut voll = Buffs::new();
        voll.apply_for_gcds(0, 1.2, 3, 0, GCD_MS);
        assert_eq!(voll.remaining_ms(0, 0), 4500);

        let mut halb = Buffs::new();
        halb.apply_for_gcds(0, 1.2, 3, 0, GCD_MS / 2);
        assert_eq!(halb.remaining_ms(0, 0), 2250);
    }

    #[test]
    fn verschiedene_arten_stapeln_multiplikativ() {
        let mut buffs = Buffs::new();
        buffs.apply_for_gcds(0, 1.2, 3, 0, GCD_MS);
        buffs.apply_for_gcds(1, 1.5, 3, 0, GCD_MS);
        assert!((buffs.factor(0) - 1.8).abs() < 1e-12);
        assert_eq!(buffs.active_count(0), 2);
    }

    #[test]
    fn gleiche_arten_ersetzen_einander() {
        let mut buffs = Buffs::new();
        buffs.apply_for_gcds(0, 1.2, 3, 0, GCD_MS);
        buffs.apply_for_gcds(0, 1.2, 3, 0, GCD_MS);
        buffs.apply_for_gcds(0, 1.2, 3, 0, GCD_MS);
        assert!((buffs.factor(0) - 1.2).abs() < 1e-12);
        assert_eq!(buffs.active_count(0), 1);
        assert_eq!(buffs.replacement_count(), 2);
    }

    #[test]
    fn ohne_verstaerkung_genau_eins() {
        assert_eq!(Buffs::new().factor(0), 1.0);
    }

    #[test]
    fn laeuft_am_ende_des_fensters_ab_nicht_davor() {
        let mut buffs = Buffs::new();
        buffs.apply_for_gcds(0, 1.2, 2, 0, GCD_MS);
        assert!(buffs.is_active(0, 2999));
        assert!(!buffs.is_active(0, 3000));
    }

    #[test]
    fn weist_eine_art_ausserhalb_des_vorrats_zurueck() {
        let mut buffs = Buffs::new();
        assert!(!buffs.apply(BUFF_KIND_CAPACITY, 2.0, 1000, 0));
        assert_eq!(buffs.factor(0), 1.0);
    }
}
