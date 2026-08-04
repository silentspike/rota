//! Der Cooldown-Ruecksetzer als **Auswahlmodus** (Issue E3-3c).
//!
//! Das Gegenstueck zu dem, was `src/core/session.ts` fuer das Spiel tut. Der
//! Ruecksetzer ist kein passiver Effekt auf den naechsten gewirkten Zauber,
//! sondern fragt: **welchen?**
//!
//! ```text
//!   1. Ruecksetzer druecken   -> Modus wird scharf, kostet einen vollen GCD
//!   2. Zauber waehlen         -> dessen Abklingzeit faellt auf null,
//!                                gewirkt wird er NICHT
//!   3. Wer ihn wirken will    -> drueckt ihn danach erneut
//! ```
//!
//! Drei Aktionen muessen sich gegen drei gewoehnliche Wirkungen rechnen —
//! Entscheidung des Auftraggebers vom 2026-07-29.
//!
//! # Die Regeln, und woher sie kommen
//!
//! | Regel | Herkunft |
//! | --- | --- |
//! | waehlt nie sich selbst — und verpufft dabei | sonst kettet er sich endlos; erster Zug einer Optimums-Suche |
//! | **jede** falsche Wahl verbraucht ihn | "pech fuer den user, muss er lernen besser zu spielen oder timen" (Auftraggeber, 2026-07-29) |
//! | die Auswahl loest die volle Wartezeit aus | folgt aus AC4 ("kein Doppel-Wirken im selben Takt") und aus "drei Aktionen" |
//! | ein Zauber ohne Abklingzeit verpufft ihn | ausdrueckliche Festlegung |
//! | die Wartezeit wird nicht mit zurueckgesetzt | ausdrueckliche Festlegung |
//!
//! # Warum das hier steht und nicht im Kampf
//!
//! Der Modus ist **Eingabe-Zustand**, nicht Kampf-Zustand: Er entscheidet, was
//! ein Tastendruck bedeutet, nicht was ein Zauber tut. Im Spiel sitzt er
//! deshalb in der Session und nicht im Kampf; hier steht er als eigenes Modul,
//! damit beide Seiten dieselbe Regel an derselben Stelle haben.

use crate::data::SPELL_COUNT;
use crate::time::Millis;

/// Kein Zauber.
pub const NO_SPELL: i16 = -1;

/// Was ein Druck im Auswahlmodus bewirkt hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Der Modus stand nicht scharf — der Druck ist eine gewoehnliche Wirkung.
    NotArmed,
    /// Zurueckgesetzt. Der Zauber ist bereit und wurde **nicht** gewirkt.
    Applied,
    /// **Verpufft.** Zwei Faelle, eine Regel: der Ruecksetzer auf sich selbst,
    /// oder ein Zauber, der gar nicht auf Abklingzeit liegt. In beiden ist er
    /// verbraucht und gewirkt wird nichts.
    Wasted,
}

/// Der Auswahlmodus.
#[derive(Debug, Clone, Copy, Default)]
pub struct CooldownReset {
    armed_for: i16,
    applied: u32,
    wasted: u32,
}

impl CooldownReset {
    /// Legt den Modus an — nicht scharf.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            armed_for: NO_SPELL,
            applied: 0,
            wasted: 0,
        }
    }

    /// Steht der Modus scharf?
    #[must_use]
    pub const fn is_armed(&self) -> bool {
        self.armed_for != NO_SPELL
    }

    /// Welcher Ruecksetzer scharf steht, sonst [`NO_SPELL`].
    #[must_use]
    pub const fn armed_for(&self) -> i16 {
        self.armed_for
    }

    /// Wie oft eine Auswahl eine Abklingzeit zurueckgesetzt hat.
    #[must_use]
    pub const fn applied_count(&self) -> u32 {
        self.applied
    }

    /// Wie oft eine Auswahl verpufft ist.
    #[must_use]
    pub const fn wasted_count(&self) -> u32 {
        self.wasted
    }

    /// Den Modus scharf stellen — **nach** der Wirkung des Ruecksetzers.
    ///
    /// Vorher waere falsch: Dann waehlte sein eigener Druck bereits.
    pub fn arm(&mut self, spell: usize) {
        self.armed_for = i16::try_from(spell).unwrap_or(NO_SPELL);
    }

    /// Ein Druck im Auswahlmodus.
    ///
    /// Setzt bei Erfolg `ready_at[spell]` auf 0 und loest die Wartezeit aus.
    /// Beides tut der Aufrufer nicht selbst, damit die Regel an einer Stelle
    /// steht.
    pub fn choose(
        &mut self,
        spell: usize,
        now: Millis,
        ready_at: &mut [Millis; SPELL_COUNT],
        gcd_ready_at: &mut Millis,
        gcd_length_ms: Millis,
    ) -> Choice {
        if !self.is_armed() {
            return Choice::NotArmed;
        }
        // **Jede Wahl verbraucht den Ruecksetzer** — auch eine falsche.
        let selbst = i16::try_from(spell).unwrap_or(NO_SPELL) == self.armed_for;
        self.armed_for = NO_SPELL;
        // Die Auswahl kostet eine Aktion — sonst faellt der Nachdruck in
        // denselben Takt, und aus drei Aktionen werden zwei.
        *gcd_ready_at = now + gcd_length_ms;

        match ready_at.get_mut(spell) {
            Some(slot) if *slot > now && !selbst => {
                *slot = 0;
                self.applied += 1;
                Choice::Applied
            }
            _ => {
                self.wasted += 1;
                Choice::Wasted
            }
        }
    }

    /// Den Modus loeschen — bei einem neuen Durchgang.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::GCD_MS;

    fn leer() -> [Millis; SPELL_COUNT] {
        [0; SPELL_COUNT]
    }

    #[test]
    fn ohne_scharfen_modus_geschieht_nichts() {
        let mut modus = CooldownReset::new();
        let mut ready = leer();
        let mut gcd = 0;
        assert_eq!(
            modus.choose(0, 0, &mut ready, &mut gcd, GCD_MS),
            Choice::NotArmed
        );
        assert_eq!(gcd, 0, "ohne Auswahl keine Wartezeit");
    }

    #[test]
    fn der_gewaehlte_zauber_ist_danach_bereit_und_ungewirkt() {
        let mut modus = CooldownReset::new();
        modus.arm(2);
        let mut ready = leer();
        ready[0] = 10_000;
        let mut gcd = 0;

        assert_eq!(
            modus.choose(0, 3000, &mut ready, &mut gcd, GCD_MS),
            Choice::Applied
        );
        assert_eq!(ready[0], 0, "die Abklingzeit muss gefallen sein");
        assert_eq!(modus.applied_count(), 1);
        assert!(!modus.is_armed(), "der Modus ist verbraucht");
    }

    #[test]
    fn er_waehlt_nie_sich_selbst_und_verpufft_dabei() {
        // Der erste Anlauf liess den Modus scharf — hergeleitet aus
        // GAME-DESIGN 8.2 ("Fehleingabe wird verworfen"). Die Herleitung war
        // schluessig und trotzdem falsch: 8.2 meint eine Taste OHNE Zauber,
        // also eine Eingabe ins Leere. Eine Selbstwahl trifft einen Zauber,
        // nur den falschen — ein Spielfehler. Der Auftraggeber hat die Linie
        // vorgegeben: "pech fuer den user".
        let mut modus = CooldownReset::new();
        modus.arm(2);
        let mut ready = leer();
        ready[2] = 10_000;
        let mut gcd = 0;

        assert_eq!(
            modus.choose(2, 3000, &mut ready, &mut gcd, GCD_MS),
            Choice::Wasted
        );
        assert_eq!(ready[2], 10_000, "seine eigene Abklingzeit bleibt stehen");
        assert!(!modus.is_armed(), "auch die Selbstwahl verbraucht ihn");
        assert_eq!(gcd, 3000 + GCD_MS, "und sie kostet eine Aktion");
    }

    #[test]
    fn ein_bereiter_zauber_verpufft_ihn() {
        let mut modus = CooldownReset::new();
        modus.arm(2);
        let mut ready = leer();
        let mut gcd = 0;

        assert_eq!(
            modus.choose(0, 3000, &mut ready, &mut gcd, GCD_MS),
            Choice::Wasted
        );
        assert_eq!(modus.wasted_count(), 1);
        assert!(!modus.is_armed(), "verbraucht ist verbraucht");
        assert_eq!(gcd, 3000 + GCD_MS, "auch die verpuffte Auswahl kostet");
    }

    #[test]
    fn die_auswahl_loest_die_volle_wartezeit_aus() {
        // Folgt aus AC4 ("kein Doppel-Wirken im selben Takt") und aus der
        // Rechnung "drei Aktionen gegen drei gewoehnliche Wirkungen".
        let mut modus = CooldownReset::new();
        modus.arm(2);
        let mut ready = leer();
        ready[0] = 10_000;
        let mut gcd = 0;

        modus.choose(0, 3000, &mut ready, &mut gcd, GCD_MS);
        assert_eq!(gcd, 4500);
    }

    #[test]
    fn die_wartezeit_wird_nicht_mit_zurueckgesetzt() {
        let mut modus = CooldownReset::new();
        modus.arm(2);
        let mut ready = leer();
        ready[0] = 10_000;
        // Eine laufende Wartezeit bis 5000 — die Auswahl bei 3000 setzt sie
        // NEU, aber nie auf null.
        let mut gcd = 5000;
        modus.choose(0, 3000, &mut ready, &mut gcd, GCD_MS);
        assert!(gcd > 3000, "die Wartezeit darf nicht verschwinden");
    }
}
