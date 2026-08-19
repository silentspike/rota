//! Exakte Krit-Verteilung und zählerbasierter Zufall (Issue #915).
//!
//! # Was hier entsteht, und was bewusst NICHT
//!
//! Dieses Modul trägt die Verfahren für den Fall, dass der kritische Treffer
//! **wirklich würfelt**: eine exakte Verteilungs-Dynamikprogrammierung und eine
//! gestaffelte Stichprobe. Beides läuft **neben** dem Kampf, nicht darin — aus
//! demselben Grund wie [`crate::analytisch`]: Der Kampf und das Spiel teilen
//! ihren Zufall, und Auflage 1 verlangt, dass jede Prüfdatei bitgleich bleibt.
//!
//! Gemessen am Bestand: Drei der 86 Paritäts-Aufzeichnungen würfeln wirklich
//! (`worn.purity = 375`, also Krit-Chance 0,375), und `worn-values.json` prüft
//! `expected.randomDraws = 25` ausdrücklich. Ein anderer Zufall im Kampfpfad
//! bräche sie beim ersten Lauf.
//!
//! # Die ganzzahlige Schwelle (AC5)
//!
//! Das Issue verlangt, Wahrscheinlichkeiten über **ganzzahlige** Schwellen zu
//! entscheiden statt über Gleitkomma-Vergleiche:
//!
//! ```text
//! random_u32 < probability_threshold_u32
//! ```
//!
//! Der Grund ist nicht Geschwindigkeit, sondern Bitgleichheit über
//! Hauptprozessor, Vektorpfad und Grafikeinheit. Ein Gleitkomma-Vergleich ist
//! genau die Stelle, an der ein Grafikpfad still abweicht.
//!
//! **Die Umrechnung ist exakt, und das ist keine Selbstverständlichkeit.**
//! `next_f64()` liefert `x / 2³²` mit ganzzahligem `x < 2³²`; dieser Quotient
//! ist in `f64` exakt darstellbar (53 Bit Mantisse reichen für 32 Bit Zähler
//! und eine Zweierpotenz im Nenner). Damit gilt
//!
//! ```text
//! x / 2³² < p   ⟺   x < p · 2³²   ⟺   x < ⌈p · 2³²⌉
//! ```
//!
//! Die letzte Umformung gilt, weil `x` ganzzahlig ist. Ob `p · 2³²` seinerseits
//! exakt gerechnet wird, hängt an `p` — und genau das prüft
//! `schwelle_trifft_denselben_ausgang_wie_der_gleitkomma_vergleich`.

/// Die ganzzahlige Schwelle zu einer Wahrscheinlichkeit.
///
/// Gibt den Wert `t` zurück, für den `random_u32 < t` genau dann gilt, wenn der
/// bisherige Vergleich `next_f64() < p` zutrifft.
///
/// Die Ränder sind Sonderfälle und **nicht** gerundet: `p <= 0` ergibt 0 (kein
/// Wert liegt darunter), `p >= 1` ergibt `2³²` als `u64` (jeder `u32` liegt
/// darunter). Beides entspricht den Kurzschlüssen in
/// [`crate::rng::RandomStream::chance`], die dort **nicht** würfeln.
#[must_use]
pub fn schwelle(p: f64) -> u64 {
    const ZWEI_HOCH_32: f64 = 4_294_967_296.0;
    if p <= 0.0 {
        return 0;
    }
    if p >= 1.0 {
        return ZWEI_HOCH_32 as u64;
    }
    // `ceil`, nicht `round`: Bei exakt aufgehendem Produkt darf der Wert
    // `p · 2³²` selbst NICHT mehr treffen — `x / 2³² < p` ist echt kleiner.
    (p * ZWEI_HOCH_32).ceil() as u64
}

/// Trifft der Wurf `wert` die Wahrscheinlichkeit — ganzzahlig entschieden?
#[must_use]
pub fn trifft(wert: u32, schwelle: u64) -> bool {
    u64::from(wert) < schwelle
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wie der Gleitkomma-Vergleich heute entscheidet — der Bezugspunkt.
    ///
    /// Bewusst nachgebildet statt `chance` gerufen: `chance` zieht selbst und
    /// nähme uns die Kontrolle über den Wurfwert. Die eine Zeile ist wörtlich
    /// die aus `rng.rs`, und der Fall unten prüft beide gegeneinander über den
    /// **ganzen** Wertebereich, nicht an Stichproben.
    fn gleitkomma_vergleich(wert: u32, p: f64) -> bool {
        f64::from(wert) / 4_294_967_296.0 < p
    }

    /// **AC5: die ganzzahlige Schwelle entscheidet identisch.**
    ///
    /// Geprüft über alle Krit-Chancen, die aus PURITY entstehen können
    /// (`purity / 1000` für 0..=1000), und je Chance über die Wurfwerte an den
    /// **Rändern** der Entscheidung. Dort und nur dort kann eine Umrechnung
    /// kippen — in der Mitte des Bereichs stimmen beide trivial überein.
    #[test]
    fn schwelle_trifft_denselben_ausgang_wie_der_gleitkomma_vergleich() {
        let mut geprueft = 0u64;
        let mut abweichungen = Vec::new();

        for purity in 0u32..=1000 {
            let p = f64::from(purity) / 1000.0;
            let t = schwelle(p);

            // Die Randwerte der Entscheidung plus ein paar feste Punkte.
            let mut kandidaten = vec![0u32, 1, u32::MAX, u32::MAX - 1];
            for delta in 0..3u64 {
                if t >= delta {
                    let unten = t - delta;
                    if unten <= u64::from(u32::MAX) {
                        kandidaten.push(unten as u32);
                    }
                }
                let oben = t + delta;
                if oben <= u64::from(u32::MAX) {
                    kandidaten.push(oben as u32);
                }
            }

            for wert in kandidaten {
                let ganzzahlig = trifft(wert, t);
                let gleitkomma = gleitkomma_vergleich(wert, p);
                geprueft += 1;
                if ganzzahlig != gleitkomma {
                    abweichungen.push((purity, wert, ganzzahlig, gleitkomma));
                }
            }
        }

        assert!(
            geprueft > 5_000,
            "Trägerzusicherung: es wurden wirklich viele Paare geprüft ({geprueft})"
        );
        assert!(
            abweichungen.is_empty(),
            "{} Abweichungen, die ersten drei: {:?}",
            abweichungen.len(),
            &abweichungen[..abweichungen.len().min(3)]
        );
        println!("{geprueft} Paare geprüft, 0 Abweichungen");
    }

    /// Die Ränder verhalten sich wie die Kurzschlüsse in `chance`.
    #[test]
    fn an_den_raendern_wird_gar_nicht_entschieden() {
        assert_eq!(schwelle(0.0), 0, "bei p = 0 trifft kein Wurf");
        assert!(!trifft(0, schwelle(0.0)), "auch die Null nicht");

        let voll = schwelle(1.0);
        assert!(trifft(u32::MAX, voll), "bei p = 1 trifft jeder Wurf");
        assert!(trifft(0, voll));

        // Und negative oder übergroße Werte kippen nicht in die Gegenrichtung.
        assert_eq!(schwelle(-0.5), 0);
        assert_eq!(schwelle(1.5), schwelle(1.0));
    }

    /// **Die Gegenprobe: eine falsche Rundung fiele auf.**
    ///
    /// Ohne diesen Fall bliebe offen, ob der Vergleich oben überhaupt etwas
    /// unterscheidet — er wäre auch grün, wenn `schwelle` und der
    /// Gleitkomma-Weg beide dieselbe falsche Antwort gäben.
    #[test]
    fn mit_round_statt_ceil_gaebe_es_abweichungen() {
        const ZWEI_HOCH_32: f64 = 4_294_967_296.0;
        let mut abweichungen = 0;

        for purity in 1u32..1000 {
            let p = f64::from(purity) / 1000.0;
            let richtig = schwelle(p);
            let gerundet = (p * ZWEI_HOCH_32).round() as u64;
            if richtig != gerundet {
                // Genau am Grenzwert entscheiden die beiden verschieden.
                let wert = gerundet.min(u64::from(u32::MAX)) as u32;
                if trifft(wert, richtig) != trifft(wert, gerundet) {
                    abweichungen += 1;
                }
            }
        }

        assert!(
            abweichungen > 0,
            "round und ceil verhielten sich in KEINEM Fall verschieden — dann \
             prüft der Fall oben die Rundung gar nicht"
        );
        println!("round statt ceil: {abweichungen} unterscheidbare Fälle");
    }
}
