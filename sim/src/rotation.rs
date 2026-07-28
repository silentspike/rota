//! Die Rotationen aus Schritt 1.
//!
//! Eine Rotation ist eine geordnete Regelliste: *"Wirke A, wenn Bedingung X.
//! Sonst B, wenn Y."* (GAME-DESIGN 18.5). In E2-3 wird daraus eine eigene
//! Sprache, die sich ohne Uebersetzen aendern laesst; solange sind es
//! gewoehnliche Funktionen.
//!
//! Die vier hier stehen wortgleich in `tools/step1-check.py` und in
//! `src/devtools/reference-run.ts`. Das ist Absicht: Sie sind der Massstab, an
//! dem sich die drei Umsetzungen messen — weicht eine Zahl ab, liegt es an den
//! Regeln und nicht daran, dass jemand eine andere Rotation gespielt hat.
//!
//! > **Sie erscheinen nirgends im Spiel** (GAME-DESIGN 21.2, `START-E2.md`),
//! > auch nicht als Lernhilfe. Das Herausfinden der Reihenfolge ist der Kern
//! > des Spiels.
//!
//! Alle Schwellwerte stammen aus `data/` und nicht aus dieser Datei — die
//! Bonus-Schwelle etwa aus dem Entlade-Verhalten von PYRE. Eine hier abgetippte
//! 15 wuerde beim naechsten Balance-Schritt zu einer Rotation fuehren, die eine
//! andere Regel spielt als das Spiel.

use crate::data::{KINDLE, PYRE, SMOULDER, SPELLS};
use crate::fight::Fight;
use crate::time::{Millis, GCD_MS};

/// Ab welchem Ressourcen-Stand der Entlade-Zauber seinen Bonus gibt.
fn bonus_threshold() -> u16 {
    SPELLS[PYRE]
        .spender
        .map_or(u16::MAX, |spender| spender.bonus_threshold)
}

/// Wie lange der Brand-Effekt laeuft.
fn dot_duration_ms() -> Millis {
    SPELLS[SMOULDER].dot.map_or(0, |dot| dot.duration_ms)
}

/// Die gute Rotation — nutzt beide Entscheidungen, die Schritt 1 anbietet.
///
/// 1. SMOULDER erst erneuern, wenn er auslaeuft — keine Sekunde frueher, sonst
///    ist die Restlaufzeit verloren **und** ein GCD verschwendet
/// 2. PYRE erst ab der Bonus-Schwelle zuenden, weil dort die Verdopplung liegt
/// 3. sonst KINDLE als Fueller
///
/// Sollwert: 4.240 Schaden, Boss faellt bei 40.510 ms (`STEP1-SPEC` § 4).
#[must_use]
pub fn good(fight: &Fight) -> Option<usize> {
    // "Laeuft in weniger als einem GCD aus" ist die spaetestmoegliche
    // Erneuerung, die keinen Tick verliert.
    if fight.dot_remaining_ms(SMOULDER) < GCD_MS && fight.is_ready(SMOULDER) {
        return Some(SMOULDER);
    }
    if fight.is_ready(PYRE) && fight.resource() >= bonus_threshold() {
        return Some(PYRE);
    }
    if fight.is_ready(KINDLE) {
        return Some(KINDLE);
    }
    None
}

/// Die schlechte Rotation — plausibel falsch, nicht absichtlich dumm.
///
/// So spielt jemand, der die beiden Regeln noch nicht verstanden hat:
///
/// 1. erneuert den Brand-Effekt, sobald die halbe Laufzeit vorbei ist —
///    verschenkt Restzeit und einen GCD
/// 2. zuendet PYRE, sobald es bereit ist — nimmt den Bonus nie mit
/// 3. sonst ebenfalls KINDLE
///
/// Sollwert: 4.044 Schaden, Boss ueberlebt mit 96,3 % (`STEP1-SPEC` § 4).
#[must_use]
pub fn bad(fight: &Fight) -> Option<usize> {
    if fight.dot_remaining_ms(SMOULDER) < dot_duration_ms() / 2 && fight.is_ready(SMOULDER) {
        return Some(SMOULDER);
    }
    if fight.is_ready(PYRE) && fight.resource() > 0 {
        return Some(PYRE);
    }
    if fight.is_ready(KINDLE) {
        return Some(KINDLE);
    }
    None
}

/// Nur der DoT-Fehler: erneuert zu frueh, zuendet PYRE aber richtig.
///
/// Zeigt, was dieser eine Fehler allein kostet — laut `STEP1-SPEC` § 4 rund
/// 9,9 % ueber die volle Kampfzeit.
#[must_use]
pub fn dot_error_only(fight: &Fight) -> Option<usize> {
    if fight.dot_remaining_ms(SMOULDER) < dot_duration_ms() / 2 && fight.is_ready(SMOULDER) {
        return Some(SMOULDER);
    }
    if fight.is_ready(PYRE) && fight.resource() >= bonus_threshold() {
        return Some(PYRE);
    }
    if fight.is_ready(KINDLE) {
        return Some(KINDLE);
    }
    None
}

/// Nur der PYRE-Fehler: Brand-Effekt richtig, aber jede Zuendung zu frueh.
///
/// Der teurere der beiden Fehler — rund 24,1 % (`STEP1-SPEC` § 4).
#[must_use]
pub fn pyre_error_only(fight: &Fight) -> Option<usize> {
    if fight.dot_remaining_ms(SMOULDER) < GCD_MS && fight.is_ready(SMOULDER) {
        return Some(SMOULDER);
    }
    if fight.is_ready(PYRE) && fight.resource() > 0 {
        return Some(PYRE);
    }
    if fight.is_ready(KINDLE) {
        return Some(KINDLE);
    }
    None
}
