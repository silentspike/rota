//! Gemeinsames Handwerkszeug der Gleichlauf-Pruefungen (Issue #122).
//!
//! # Warum Gleitkommazahlen ueber ihr Bitmuster verglichen werden
//!
//! **`serde_json` liest manche `f64` mit einem Bit Abweichung.** Gemessen am
//! 2026-07-29 beim Bau von #114 und vom Orchestrator unabhaengig nachgemessen:
//!
//! ```text
//!   1.6500000000000001   std=3ffa666666666667   serde=3ffa666666666666  ABWEICHUNG
//!   1.0000000000000002   std=3ff0000000000001   serde=3ff0000000000001  gleich
//!   2.1450000000000005   std=400128f5c28f5c2a   serde=400128f5c28f5c2a  gleich
//!   2.475                std=4003cccccccccccd   serde=4003cccccccccccd  gleich
//! ```
//!
//! Die Rechnung stimmt, `str::parse::<f64>` stimmt, und `serde_json` ueber den
//! Umweg Text-plus-parse stimmt auch. Nur `as_f64()` verliert das Bit — und
//! **nicht systematisch**: drei der vier Zahlen oben gehen durch.
//!
//! Damit war jeder Vergleich ueber die Dezimalschreibweise **gruen aus
//! Glueck**, nicht aus Konstruktion. Die naechste Zahl, die jemand in eine
//! Vergleichsdatei schreibt, konnte ihn stillschweigend entwerten: Er meldete
//! dann eine Abweichung, die es nicht gibt, oder verdeckte eine, die es gibt.
//!
//! Dieselbe Klasse wie der Protokoll-Fund in #105 — kein Fehler im Lauf,
//! sondern eine gestellte Falle fuer spaeter.
//!
//! # Die Regel
//!
//! **Jede Gleitkommazahl, die ein Prueffall aus einer Vergleichsdatei liest,
//! wird ueber ihr Bitmuster verglichen, nie ueber `as_f64()`.** Die Datei fuehrt
//! sie zweimal: dezimal, damit ein Diff lesbar bleibt, und als 16 Hexzeichen,
//! weil nur das den Vergleich traegt. Erzeugt werden beide von `bitsOf()` in
//! `tests/parity/payloads.ts`.
//!
//! Das gilt auch fuer **Eingabe**werte. Laese der Simulator die ein Bit
//! daneben, rechnete er mit anderen Zahlen als das Spiel — und meldete eine
//! Abweichung, die er selbst erzeugt hat.
//!
//! # Nicht betroffen
//!
//! `balance.rs` vergleicht mit einer Toleranz von `1e-9`. Ein Bit liegt weit
//! darunter; dort ist `as_f64()` unbedenklich.

// Jede Prueffall-Datei bindet dieses Modul einzeln ein und benutzt davon nur,
// was sie braucht — ungenutzte Helfer sind hier der Normalfall, kein Fund.
#![allow(dead_code)]

use serde_json::Value;

/// Die Bits einer Gleitkommazahl aus einer Vergleichsdatei.
///
/// Erwartet das Feld mit den 16 Hexzeichen, **nicht** das dezimale.
///
/// # Panics
///
/// Wenn das Feld fehlt, keine Zeichenkette ist oder sich nicht als
/// Sedezimalzahl lesen laesst. Ein Prueffall, der stattdessen still auf einen
/// Ersatzwert ausweicht, waere genau der Zustand, den #122 beseitigt.
#[must_use]
pub fn bits(value: &Value) -> u64 {
    let text = value
        .as_str()
        .expect("Bitmuster als Zeichenkette erwartet — fehlt das Feld in der Vergleichsdatei?");
    u64::from_str_radix(text, 16).expect("16 Hexzeichen erwartet")
}

/// Die Gleitkommazahl hinter einem Bitmuster.
///
/// Fuer **Eingabe**werte: Der Simulator muss mit derselben Zahl rechnen, die
/// das Spiel benutzt hat, nicht mit der naechstgelegenen.
#[must_use]
pub fn float(value: &Value) -> f64 {
    f64::from_bits(bits(value))
}
