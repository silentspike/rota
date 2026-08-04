//! Die Schadensformel und ihre Faktoren (GAME-DESIGN 6.3).
//!
//! ```text
//! Schaden = Grundschaden
//!         × ESSENCE-Faktor
//!         × Talent-Faktoren
//!         × Ausrüstungs-Sondereffekte
//!         × aktive Verstärkungen
//!         × Synergie-Faktoren
//!         × Widerstandsfaktor des Bosses
//!         × Krit
//! ```
//!
//! **Die Reihenfolge der Multiplikationen ist festgelegt und für Spiel und
//! Simulator identisch.** Fließkomma-Multiplikation ist nicht assoziativ:
//! `(a × b) × c` und `a × (b × c)` können sich im letzten Bit unterscheiden.
//! Bei zehntausenden Treffern je Balance-Lauf summiert sich das zu einer
//! Abweichung, die niemand mehr zuordnen kann.
//!
//! Diese Datei ist deshalb die Zwillingsschwester von `src/core/damage.ts` —
//! Feld für Feld, Zeile für Zeile. `tests/parity_factors.rs` prüft das gegen
//! Werte aus dem Spielcode.
//!
//! # Wofür der Balance-Läufer sie braucht
//!
//! Issue #56 fragt nach Skalierungsfaktoren: *„Wie viel bringt ein Punkt
//! CATALYSIS?"* Die Umrechnung eines Wert-**Punkts** in einen **Faktor** ist im
//! Design noch nicht festgelegt (GAME-DESIGN 22: Talent-Inhalte vor E5,
//! Gegenstands-Katalog vor E6). Messbar ist deshalb die Empfindlichkeit
//! gegenüber den **Faktoren** — und sobald die Umrechnung feststeht, ist der
//! Wert je Punkt eine Multiplikation davon.

/// Die Faktoren in der Reihenfolge, in der sie angewandt werden.
///
/// `f64` und nicht ganzzahlig: Anders als die Zeit sind Schadensfaktoren
/// Verhältnisse, und ein Tempo-Faktor von 1,17 (GAME-DESIGN 18.4 nennt genau
/// dieses Beispiel) ließe sich ganzzahlig nicht ausdrücken. Gerundet wird
/// einmal am Ende.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageFactors {
    /// Aus dem Wert ESSENCE (GAME-DESIGN 6.1).
    pub essence: f64,
    /// Aus dem Talentbaum.
    pub talents: f64,
    /// Legendaere Effekte **und** Satz-Boni — Topf 4 aus GAME-DESIGN 13.1b.
    ///
    /// **Warum an dieser Stelle:** Keiner der bestehenden Faktoren konnte ihn
    /// aufnehmen. `essence` machte ihn additiv mit ESSENCE, `talents` additiv
    /// mit den Talenten — und 13.1 trennt beide Systeme ausdruecklich. `buffs`
    /// IST Topf 3, und zwischen Toepfen wird multipliziert. Es blieb nur ein
    /// eigener Faktor. Die Position gruppiert die Formel: erst die
    /// Spieler-Faktoren, dann der Zustand, dann der Boss, zuletzt Krit.
    ///
    /// Heute steht er bei 1,0 und bewegt keine einzige Zahl. Genau deshalb war
    /// Vorsicht geboten — eine Aenderung, die nichts bewegt, geht sonst
    /// unbemerkt durch.
    pub gear_effects: f64,
    /// Laufende Verstaerkungen auf dem Spieler.
    pub buffs: f64,
    /// Offene Synergie-Fenster (GAME-DESIGN 5).
    pub synergy: f64,
    /// Widerstand des Bosses gegen die Schule (GAME-DESIGN 11.4).
    pub resistance: f64,
    /// Verwundbarkeit des Bosses — der siebte Faktor (GAME-DESIGN 6.3).
    ///
    /// Traegt alles, was den Boss fuer Schaden **aus jeder Quelle**
    /// empfaenglicher macht; heute die Zersetzungs-Stapel (4.1).
    ///
    /// **Die Position ist eine Festlegung:** `… × resistance × vulnerability ×
    /// crit`. Fliesskomma-Multiplikation ist nicht assoziativ.
    pub vulnerability: f64,
    /// 1,0 ohne Krit, sonst der YIELD-Faktor.
    pub crit: f64,
}

/// Alle Faktoren neutral — der Zustand auf Stufe 1 ohne Ausruestung.
///
/// Das ist der Zustand von Schritt 1 (`STEP1-SPEC` § 2: „alle Faktoren = 1,0").
/// Mit ihm rechnet die Formel dasselbe wie gar keine Formel, und der Gleichlauf
/// mit dem Spiel aus #54 bleibt unberuehrt — geprueft in `tests/parity_factors.rs`.
pub const NEUTRAL: DamageFactors = DamageFactors {
    essence: 1.0,
    talents: 1.0,
    gear_effects: 1.0,
    buffs: 1.0,
    synergy: 1.0,
    resistance: 1.0,
    vulnerability: 1.0,
    crit: 1.0,
};

impl Default for DamageFactors {
    fn default() -> Self {
        NEUTRAL
    }
}

impl DamageFactors {
    /// Ob alle Faktoren neutral sind.
    ///
    /// Dann darf die Rechnung abgekuerzt werden, ohne dass sich etwas aendert —
    /// und genau das tut [`compute_damage`].
    #[must_use]
    pub fn is_neutral(&self) -> bool {
        *self == NEUTRAL
    }
}

/// Schaden eines einzelnen Treffers.
///
/// Gerundet wird **einmal am Ende**, nicht zwischendurch: Zwischenrundungen
/// waeren eine zweite Stelle, an der Spiel und Simulator auseinanderlaufen
/// koennten. Der Rundungspunkt gehoert damit zur gemeinsamen Schnittstelle.
///
/// # Zur Rundung
///
/// JavaScript rundet mit `Math.round` zur naechsten ganzen Zahl und die Haelfte
/// **aufwaerts** (Richtung +∞). Rust rundet die Haelfte **vom Nullpunkt weg**.
/// Fuer positive Werte ist das dasselbe; Schaden ist nie negativ, und
/// `debug_assert` haelt das fest, falls sich das je aendert.
#[must_use]
pub fn compute_damage(base_damage: u32, factors: &DamageFactors) -> u32 {
    // Der haeufigste Fall in Schritt 1, und zugleich der wichtigste: Ohne
    // Faktoren gibt es keine Fliesskomma-Rechnung und damit auch keine
    // Moeglichkeit, dass sie von der des Spiels abweicht.
    if factors.is_neutral() {
        return base_damage;
    }

    let mut value = f64::from(base_damage);
    value *= factors.essence;
    value *= factors.talents;
    value *= factors.gear_effects;
    value *= factors.buffs;
    value *= factors.synergy;
    value *= factors.resistance;
    value *= factors.vulnerability;
    value *= factors.crit;

    debug_assert!(
        value >= 0.0,
        "negativer Schaden — die Rundung wuerde von JavaScript abweichen"
    );
    // `as u32` saettigt bei 0 und bei u32::MAX; beides waere ein Fehler weiter
    // oben, aber kein Grund, hier abzustuerzen.
    value.round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutrale_faktoren_lassen_den_grundschaden_unveraendert() {
        for base in [0, 1, 40, 100, 360, 4240] {
            assert_eq!(compute_damage(base, &NEUTRAL), base);
        }
    }

    #[test]
    fn die_reihenfolge_der_multiplikationen_steht_fest() {
        // **Was dieser Waechter leistet — und was nicht.**
        //
        // Er prueft, dass ALLE Faktoren angewandt werden. Wer einen vergisst
        // oder doppelt anwendet, faellt hier auf.
        //
        // Eine **Umstellung** der Reihenfolge faengt er NICHT, und der
        // urspruengliche Kommentar hier behauptete das Gegenteil. Gemessen am
        // 2026-07-29 (Issue E6-1): ueber 36 356 232 Kombinationen im
        // u32-Bereich gibt es keinen Wertesatz, bei dem eine Umstellung den
        // GERUNDETEN Schaden bewegt. `compute_damage` rundet am Ende, und bei
        // u32-Betraegen ist der Fliesskomma-Unterschied rund eine Million mal
        // kleiner als eine halbe Einheit.
        //
        // Der Gleichlauf ist trotzdem gesichert, nur anderswo:
        // `tests/unit/damage-order.test.ts` auf der Spielseite kann es, weil
        // JavaScript keine u32-Grenze kennt und dort Betraege erreichbar sind,
        // bei denen die Rundung kippt. Dort steht die Gegenprobe, die hier
        // nicht moeglich ist.
        let factors = DamageFactors {
            essence: 1.1,
            talents: 1.03,
            // **Ausdruecklich nicht 1,0.** Ein neutraler Wert liesse sich
            // beliebig verschieben, ohne dass dieser Waechter etwas merkt —
            // er wuerde die Position des neuen Faktors also gar nicht sichern.
            gear_effects: 1.19,
            buffs: 1.07,
            synergy: 1.13,
            resistance: 0.97,
            crit: 2.0,
        };
        let mut erwartet = 100.0_f64;
        erwartet *= 1.1;
        erwartet *= 1.03;
        erwartet *= 1.19;
        erwartet *= 1.07;
        erwartet *= 1.13;
        erwartet *= 0.97;
        erwartet *= 2.0;
        assert_eq!(compute_damage(100, &factors), erwartet.round() as u32);
    }

    #[test]
    fn einmal_gerundet_wird_am_ende() {
        // 100 × 1,004 = 100,4 -> 100. Wuerde zwischendurch gerundet, kaeme bei
        // zwei Faktoren zu je 1,002 etwas anderes heraus.
        let einmal = DamageFactors {
            essence: 1.004,
            ..NEUTRAL
        };
        let zweimal = DamageFactors {
            essence: 1.002,
            talents: 1.002,
            ..NEUTRAL
        };
        assert_eq!(compute_damage(100, &einmal), 100);
        assert_eq!(compute_damage(100, &zweimal), 100);
    }

    #[test]
    fn die_haelfte_wird_aufgerundet() {
        // Wie `Math.round` in JavaScript. 40 × 1,0125 = 40,5 -> 41.
        let factors = DamageFactors {
            essence: 1.0125,
            ..NEUTRAL
        };
        assert_eq!(compute_damage(40, &factors), 41);
    }

    #[test]
    fn ein_faktor_null_ergibt_null() {
        let factors = DamageFactors {
            resistance: 0.0,
            ..NEUTRAL
        };
        assert_eq!(compute_damage(4240, &factors), 0);
    }
}
