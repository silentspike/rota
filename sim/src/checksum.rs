//! Zustands-Pruefsumme je Takt (GAME-DESIGN 19.1).
//!
//! **Wozu:** Weichen Spiel und Simulator voneinander ab, sagt die Folge der
//! Pruefsummen, **in welchem Takt** es passiert ist — statt nur, dass der
//! Endschaden nicht stimmt. Bei 6.000 Takten je Kampf ist das der Unterschied
//! zwischen einer Minute und einem Nachmittag.
//!
//! Das Verfahren ist FNV-1a ueber 32 Bit und stammt aus `src/core/checksum.ts`.
//! Beide Umsetzungen muessen **bitgleich** rechnen, sonst ist die ganze Anzeige
//! wertlos: Eine Pruefsumme, die sich schon im Verfahren unterscheidet, meldet
//! bei jedem Takt eine Abweichung und bei keinem eine echte.
//!
//! Drei Stellen sind dabei heikel, und alle drei sind hier bewusst so geschrieben:
//!
//! 1. **Vorzeichen.** JavaScript rechnet `value | 0` in eine 32-Bit-Zahl mit
//!    Vorzeichen um und schiebt danach mit `>>>` **ohne** Vorzeichen. In Rust
//!    heisst das: als `u32` betrachten und logisch schieben.
//! 2. **Fliesskomma.** Eingerührt werden die **Bits**, nicht der gerundete Wert —
//!    und zwar niederwertiges Wort zuerst. Das entspricht der `Int32Array`-Sicht
//!    auf einen `Float64Array` auf allen Maschinen, auf denen ROTA laeuft.
//! 3. **Reihenfolge der Felder.** Sie ist Teil der Schnittstelle. Ein Feld
//!    dazwischenzuschieben aendert jede Pruefsumme.
//!
//! `tests/parity_primitives.rs` prueft alle drei gegen Werte, die aus dem
//! Spielcode selbst stammen.

/// Anfangswert von FNV-1a ueber 32 Bit.
pub const FNV_OFFSET: u32 = 0x811c_9dc5;
/// Multiplikator von FNV-1a ueber 32 Bit.
pub const FNV_PRIME: u32 = 0x0100_0193;

/// Eine ganze Zahl einruehren — vier Bytes, niederwertiges zuerst.
#[must_use]
pub const fn mix_int(hash: u32, value: i32) -> u32 {
    let mut h = hash;
    // `value as u32` entspricht dem `| 0` samt anschliessendem `>>> 8` in
    // JavaScript: Ab dem ersten Schieben ist der Wert vorzeichenlos.
    let mut v = value as u32;
    let mut byte = 0;
    while byte < 4 {
        h ^= v & 0xff;
        h = h.wrapping_mul(FNV_PRIME);
        v >>= 8;
        byte += 1;
    }
    h
}

/// Eine Fliesskommazahl einruehren — ueber ihre Bit-Darstellung.
///
/// Zwei Zahlen, die sich im letzten Bit unterscheiden, muessen verschiedene
/// Pruefsummen ergeben; genau diese Unterschiede sucht der Abgleich.
#[must_use]
pub const fn mix_float(hash: u32, value: f64) -> u32 {
    let bits = value.to_bits();
    let low = bits as u32;
    let high = (bits >> 32) as u32;
    mix_int(mix_int(hash, low as i32), high as i32)
}

// Die Namensfolge des laufenden Aufrufs — **nur fuer den Gleichlauf-Waechter**
// (Issue #236).
//
// # Warum zur Laufzeit und nicht aus dem Quelltext
//
// Eine Quelltext-Stelle ist nicht ein Wert. `checksum_at` mischt in einer
// Schleife ueber die Zauber fuenf Werte **je Zauber**. Wer die Namen aus dem
// Quelltext zieht, zaehlt Zeilen und nicht Werte — und meldet eine Abweichung
// zwischen den Seiten, die keine ist. Genau daran ist der erste Versuch eines
// solchen Waechters gescheitert.
//
// Zur Laufzeit ist ausserdem gleichgueltig, ob ein Wert direkt oder ueber eine
// lokale Variable hereinkommt: gemischt ist gemischt.
//
// # Im Kampf kostet das nichts
//
// Aufgezeichnet wird nur, wenn die Aufzeichnung laeuft. Sonst ist es ein
// Vergleich gegen `None` und kein Eintrag — der Kampf darf im Schritt nichts
// anfordern (GAME-DESIGN 18.4).
thread_local! {
    static RECORDING: std::cell::RefCell<Option<Vec<&'static str>>> =
        const { std::cell::RefCell::new(None) };
}

/// Zeichnet die Namensfolge eines Aufrufs auf und gibt sie zurueck.
pub fn record_field_order<F: FnOnce()>(run: F) -> Vec<&'static str> {
    RECORDING.with(|r| *r.borrow_mut() = Some(Vec::new()));
    run();
    RECORDING.with(|r| r.borrow_mut().take().unwrap_or_default())
}

/// Eine ganze Zahl einruehren, unter einem Namen.
///
/// **Der Name geht NICHT in die Pruefsumme ein.** Taete er es, aenderte sich
/// jede Pruefsumme im Projekt, und jede eingecheckte Vergleichsdatei waere
/// falsch. Er dient allein dazu, die Reihenfolge benennbar zu machen: Meldet
/// der Waechter "an Position 9 steht hier `casts` und dort `dot_ticks`", ist
/// die Stelle gefunden — statt "Pruefsumme stimmt nicht", was in #97 eine
/// Stunde Suche bedeutet hat.
#[must_use]
pub fn mix_named_int(hash: u32, name: &'static str, value: i32) -> u32 {
    RECORDING.with(|r| {
        if let Some(liste) = r.borrow_mut().as_mut() {
            liste.push(name);
        }
    });
    mix_int(hash, value)
}

/// Dasselbe fuer eine Fliesskommazahl. Der Name geht auch hier nicht ein.
#[must_use]
pub fn mix_named_float(hash: u32, name: &'static str, value: f64) -> u32 {
    RECORDING.with(|r| {
        if let Some(liste) = r.borrow_mut().as_mut() {
            liste.push(name);
        }
    });
    mix_float(hash, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleiche_eingabe_gleiche_summe() {
        assert_eq!(mix_int(FNV_OFFSET, 42), mix_int(FNV_OFFSET, 42));
        assert_eq!(mix_float(FNV_OFFSET, 1.5), mix_float(FNV_OFFSET, 1.5));
    }

    #[test]
    fn ein_bit_unterschied_aendert_die_summe() {
        assert_ne!(mix_int(FNV_OFFSET, 0), mix_int(FNV_OFFSET, 1));
        // Die kleinste darstellbare Aenderung an einer Fliesskommazahl.
        let a = 1.0_f64;
        let b = f64::from_bits(a.to_bits() + 1);
        assert_ne!(mix_float(FNV_OFFSET, a), mix_float(FNV_OFFSET, b));
    }

    #[test]
    fn die_reihenfolge_zaehlt() {
        let vorwaerts = mix_int(mix_int(FNV_OFFSET, 1), 2);
        let rueckwaerts = mix_int(mix_int(FNV_OFFSET, 2), 1);
        assert_ne!(vorwaerts, rueckwaerts);
    }

    #[test]
    fn negative_zahlen_laufen_nicht_ueber() {
        // In JavaScript ist das der Fall, an dem sich Vorzeichen-Fehler zeigen.
        // Hier genuegt: es rechnet und liefert etwas anderes als der Betrag.
        assert_ne!(mix_int(FNV_OFFSET, -1), mix_int(FNV_OFFSET, 1));
        assert_ne!(mix_int(FNV_OFFSET, -40510), mix_int(FNV_OFFSET, 40510));
    }

    #[test]
    fn null_und_minus_null_sind_verschieden() {
        // Ein Fliesskomma-Detail, das ueber die Bits sichtbar wird und ueber den
        // Wert nicht: -0.0 == 0.0, aber die Bitmuster unterscheiden sich.
        assert_ne!(mix_float(FNV_OFFSET, 0.0), mix_float(FNV_OFFSET, -0.0));
    }
}
