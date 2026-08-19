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

/// Wert von [`Buffs::next_expiry_at`], wenn nichts laeuft.
///
/// `i64::MAX` und nicht [`NOT_ACTIVE`]: Der Vorabtest fragt `now <
/// next_expiry_at`, und mit `-1` waere er bei einem leeren Satz **immer
/// falsch** — also genau dort wirkungslos, wo er am meisten spart.
const NICHTS_LAEUFT_AB: i64 = i64::MAX;

/// Die laufenden Verstaerkungen.
///
/// Je Art hoechstens eine — die Stapel-Regel steckt in der Datenstruktur und
/// nicht in einer Pruefung: Zwei gleiche Arten koennen gar nicht nebeneinander
/// stehen.
#[derive(Debug, Clone)]
pub struct Buffs {
    factors: [f64; BUFF_KIND_CAPACITY],
    expires_at: [i64; BUFF_KIND_CAPACITY],
    /// Der frueheste Zeitpunkt, zu dem hier ueberhaupt etwas ablaufen kann
    /// (Issue #909, Punkt 1).
    ///
    /// # Wofuer
    ///
    /// [`Buffs::expire`] lief bisher bei **jedem** Zeitschritt ueber alle
    /// Arten, auch wenn nichts anstand. Gemessen ueber 145 750 400 Aufrufe im
    /// LNS-Lauf: **0,63 Prozent** raeumten etwas ab — 99,37 Prozent waren
    /// vergeblich. Und `Fight::expire_buffs` ruft acht solche Sammlungen auf,
    /// je Zeitschritt also 64 Vergleiche fuer meist nichts.
    ///
    /// # Die Zusicherung, an der alles haengt
    ///
    /// Dieser Wert ist **hoechstens** so gross wie der kleinste aktive
    /// Eintrag in `expires_at` — nie groesser. Damit ist der Vorabtest
    /// *konservativ*: Er ueberspringt nur Durchlaeufe, in denen sicher nichts
    /// ablaufen kann. Waere er zu gross, verschwaende eine Verstaerkung nicht
    /// zum richtigen Zeitpunkt, und das Ergebnis aenderte sich — genau das,
    /// was AC2 verbietet.
    ///
    /// [`Buffs::pruefe_invariante`] rechnet das nach; `expire` prueft es in
    /// Debug-Bauten bei jedem Aufruf.
    next_expiry_at: i64,
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
            next_expiry_at: NICHTS_LAEUFT_AB,
            replacements: 0,
            expirations: 0,
        }
    }

    /// Rechnet [`Buffs::next_expiry_at`] aus dem Feld neu aus.
    ///
    /// Nur dort aufgerufen, wo etwas **verschwindet** (`expire`): Beim
    /// Hinzufuegen genuegt das Minimum mit dem neuen Wert, und das ist
    /// billiger.
    fn nachrechnen(&mut self) {
        let mut frueheste = NICHTS_LAEUFT_AB;
        for kind in 0..BUFF_KIND_CAPACITY {
            let until = self.expires_at[kind];
            if until != NOT_ACTIVE && until < frueheste {
                frueheste = until;
            }
        }
        self.next_expiry_at = frueheste;
    }

    /// Ob die Zusicherung gilt: `next_expiry_at` ist hoechstens der kleinste
    /// aktive Eintrag.
    ///
    /// Steht oeffentlich, damit ein Prueffall sie an einem echten Kampf
    /// nachrechnen kann und nicht nur an von Hand gesetzten Werten. Ein
    /// `debug_assert` allein liefe in der CI mit — aber nicht im
    /// Release-Messlauf, und genau dort entstehen die Zahlen.
    #[must_use]
    pub fn pruefe_invariante(&self) -> bool {
        let mut frueheste = NICHTS_LAEUFT_AB;
        for kind in 0..BUFF_KIND_CAPACITY {
            let until = self.expires_at[kind];
            if until != NOT_ACTIVE && until < frueheste {
                frueheste = until;
            }
        }
        self.next_expiry_at <= frueheste
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
        // Minimum statt Nachrechnen: Ein neuer Eintrag kann den fruehesten
        // Zeitpunkt nur VORVERLEGEN, nie verschieben. Das haelt die
        // Zusicherung und kostet einen Vergleich statt acht.
        if expires_at != NOT_ACTIVE && expires_at < self.next_expiry_at {
            self.next_expiry_at = expires_at;
        }
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
    /// Laeuft je Takt und erzeugt deshalb nichts. Der Rueckgabewert bleibt
    /// eine Zahl — daran haelt sich auch die Maske: Sie sagt zusaetzlich,
    /// WELCHE Arten abgelaufen sind, ohne dafuer etwas anzulegen.
    ///
    /// # Warum eine Maske und keine Anzahl (Issue #682)
    ///
    /// Der Aufrufer schreibt je abgelaufener Art einen Protokoll-Eintrag.
    /// Eine blosse Anzahl truege dafuer zu wenig, und ein Eintrag, der immer
    /// dieselbe Zahl traegt, ist von einem festen Wert nicht zu
    /// unterscheiden. Das Protokoll bleibt ausserhalb dieser Datei.
    ///
    /// `count_ones()` ist die fruehere Anzahl, verlustfrei: `expires_at`
    /// haelt je Art genau einen Eintrag, es kann also hoechstens eine
    /// Verstaerkung je Art ablaufen.
    ///
    /// @returns Bitmaske der abgelaufenen Arten — Bit `k` gesetzt heisst
    ///          „Art `k` ist abgelaufen". `0` heisst „keine", wie zuvor.
    pub fn expire(&mut self, now: Millis) -> u32 {
        debug_assert!(
            self.pruefe_invariante(),
            "next_expiry_at ist groesser als der kleinste aktive Eintrag — der Vorabtest \
             wuerde einen Ablauf verschlucken und das Ergebnis aendern"
        );
        // Der Vorabtest aus Issue #909, Punkt 1. Gemessen sind 99,37 Prozent
        // der Aufrufe vergeblich; hier enden sie nach einem Vergleich statt
        // nach acht.
        if i64::from(now) < self.next_expiry_at {
            return 0;
        }

        let mut ended = 0;
        for kind in 0..BUFF_KIND_CAPACITY {
            let until = self.expires_at[kind];
            if until == NOT_ACTIVE || until > i64::from(now) {
                continue;
            }
            self.expires_at[kind] = NOT_ACTIVE;
            self.factors[kind] = 1.0;
            ended |= 1u32 << kind;
            self.expirations += 1;
        }
        // Hier MUSS nachgerechnet werden: Was verschwindet, kann den
        // fruehesten Zeitpunkt nach HINTEN schieben, und dafuer genuegt kein
        // Minimum. Der Durchlauf ist ohnehin schon gelaufen.
        self.nachrechnen();
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
        self.next_expiry_at = NICHTS_LAEUFT_AB;
        self.replacements = 0;
        self.expirations = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::GCD_MS;

    /// Die Fenstersumme traegt ERWARTETE ZAHLEN, nicht nur denselben Wert
    /// wie die Spiel-Seite (Issue #493).
    ///
    /// **Warum das ein eigener Fall ist, obwohl die Paritaet es abdeckt:**
    /// Ein Abgleich zweier Umsetzungen faengt keinen Fehler, den BEIDE
    /// teilen. Genau diese Lage hielt `perDecayStackPercent` fuenf Wochen
    /// verborgen — zwei identisch falsche Rechnungen, gruene Paritaet.
    ///
    /// Ein Fall mit ausgerechneten Zahlen kann das; ein Vergleich nicht.
    #[test]
    fn fenstersumme_zaehlt_offene_fenster_mit_ihrem_ende() {
        let mut b = Buffs::new();
        assert_eq!(b.window_sum(), 0.0, "ohne Verstaerkung ist die Summe 0");

        // Ein Fenster ueber drei Wartezeiten endet bei 4500.
        b.apply_for_gcds(0, 1.2, 3, 0, GCD_MS);
        assert_eq!(b.window_sum(), 4500.0);

        // Ein zweites, ueber zwei Wartezeiten: 4500 + 3000.
        b.apply_for_gcds(1, 1.5, 2, 0, GCD_MS);
        assert_eq!(b.window_sum(), 7500.0);

        // Nach dem Ablauf des zweiten bleibt das erste.
        b.expire(3000);
        assert_eq!(b.window_sum(), 4500.0, "das abgelaufene zaehlt weiter mit");

        // Und nach dem Ablauf beider ist sie wieder 0 — sonst wuechse sie
        // ueber den Kampf hinweg und truege geschlossene Fenster mit.
        b.expire(4500);
        assert_eq!(b.window_sum(), 0.0);
    }

    /// **Die Gegenprobe zur Summe:** Zwei verschiedene Zustaende duerfen
    /// nicht dieselbe Zahl ergeben. Sonst waere sie fuer die Pruefsumme
    /// wertlos, obwohl sie sich bewegt.
    #[test]
    fn verschiedene_fenster_ergeben_verschiedene_summen() {
        let mut frueh = Buffs::new();
        frueh.apply_for_gcds(0, 1.2, 2, 0, GCD_MS);

        let mut spaet = Buffs::new();
        spaet.apply_for_gcds(0, 1.2, 3, 0, GCD_MS);

        assert_ne!(
            frueh.window_sum(),
            spaet.window_sum(),
            "zwei verschieden lange Fenster ergeben dieselbe Summe"
        );
    }

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
