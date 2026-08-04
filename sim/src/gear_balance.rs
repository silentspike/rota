//! Die Balance-Messung der Ausruestung (GAME-DESIGN 13.1b und 13.5, Issue E6-5).
//!
//! # Was hier gemessen wird
//!
//! **Messung 2 aus dem Issue: sechs Faktoren oder vier Toepfe?** Die
//! Schadensformel fuehrt heute sieben benannte Faktoren; die
//! Ausruestungs-Struktur beschreibt vier Toepfe. Der Auftraggeber hat
//! entschieden: erst messen, dann entscheiden, ob eine Vereinheitlichung sich
//! lohnt.
//!
//! Die Frage ist nicht "ist es huebscher", sondern: **Aendert die
//! Vereinheitlichung die Ergebnisse?** Bei reiner Kosmetik bleibt es bei den
//! sieben — ein Umbau wuerde alle Vergleichsdateien und Pruefsummen neu
//! erzeugen.
//!
//! # Warum die Antwort nicht offensichtlich ist
//!
//! In exakter Arithmetik sind beide Varianten dasselbe: Multiplikation ist
//! assoziativ. In `f64` ist sie es **nicht** — `(a·b)·c` und `a·(b·c)` koennen
//! sich im letzten Bit unterscheiden. Ob dieser Unterschied bis zum
//! **gerundeten Schaden** durchschlaegt, ist eine Messung und keine Ableitung.
//!
//! # Was hier NICHT gemessen wird, und warum
//!
//! **Messung 1 — bester Satz-Aufbau gegen besten Affix-Aufbau — steht aus.**
//! Am 2026-08-04 gemessen: Kein einziger Ausruestungs-Topf wird ausserhalb von
//! `gear.rs` gelesen. `factors.gear_effects` wird ausschliesslich in
//! `balance.rs` gesetzt, also kuenstlich in der Messung, nie aus echter
//! Ausruestung. CATALYSIS und PURITY sind nirgends verdrahtet, YIELD ist
//! wirkungslos, weil Krit ruht (`STEP1-SPEC` 2).
//!
//! Ein Vergleich auf dieser Grundlage saehe aus wie "Saetze sind ueberlegen",
//! waehrend in Wahrheit nur die Gegenseite nicht eingebaut ist. Die
//! Topf-Arithmetik unten laesst sich trotzdem exakt rechnen — sie ist eine
//! **Modellrechnung**, keine Messung am Kampf, und wird als solche benannt.

use crate::factors::{compute_damage, DamageFactors};
use crate::gear::{ValuePools, POOL_COUNT};

/// Die Faktoren aus den Toepfen, **jeder einzeln** — wie die Formel es heute
/// tut.
///
/// Die Reihenfolge ist die der Schadensformel (6.3) und liegt fest.
#[must_use]
pub fn damage_named(base: u32, pools: &ValuePools, essence_pool: usize, gear_pool: usize) -> u32 {
    let factors = DamageFactors {
        essence: pools.factor(essence_pool),
        gear_effects: pools.factor(gear_pool),
        ..crate::factors::NEUTRAL
    };
    compute_damage(base, &factors)
}

/// Dieselben Toepfe, **zu einem Faktor zusammengefasst**.
///
/// Die Variante, die das Issue zur Diskussion stellt: alle Topf-Faktoren erst
/// miteinander multiplizieren, dann einmal auf den Grundschaden anwenden.
///
/// **Der Unterschied liegt allein in der Klammerung**, und genau darum geht
/// es: `(base·f1)·f2` gegen `base·(f1·f2)`. In exakter Arithmetik dasselbe, in
/// `f64` nicht zwingend.
#[must_use]
pub fn damage_unified(base: u32, pools: &ValuePools, essence_pool: usize, gear_pool: usize) -> u32 {
    let zusammen = pools.factor(essence_pool) * pools.factor(gear_pool);
    let factors = DamageFactors {
        essence: zusammen,
        ..crate::factors::NEUTRAL
    };
    compute_damage(base, &factors)
}

/// Was der Vergleich der beiden Varianten ergeben hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantComparison {
    /// Wie viele Faelle gerechnet wurden.
    pub cases: u32,
    /// In wie vielen der **rohe** Wert auseinanderlief.
    pub raw_differs: u32,
    /// In wie vielen der **gerundete Schaden** auseinanderlief.
    ///
    /// Das ist die Zahl, auf die es ankommt: Sie entscheidet, ob die
    /// Vereinheitlichung Kosmetik ist oder nicht.
    pub damage_differs: u32,
}

impl VariantComparison {
    /// Ist der Unterschied messbar — also im gerundeten Schaden sichtbar?
    #[must_use]
    pub const fn is_measurable(&self) -> bool {
        self.damage_differs > 0
    }
}

/// Vergleicht beide Varianten ueber eine feste Folge von Topf-Belegungen.
///
/// **Deterministisch, ohne Zufallsstrom.** Die Belegungen entstehen aus einer
/// ganzzahligen Folge; zwei Laeufe geben dasselbe Ergebnis, und der Bericht
/// ist damit wiederholbar.
#[must_use]
pub fn compare_variants(cases: u32, essence_pool: usize, gear_pool: usize) -> VariantComparison {
    let mut raw_differs = 0;
    let mut damage_differs = 0;

    for i in 0..cases {
        // Zwei Anteile, die sich ueber den Bereich bewegen. Ganzzahlig
        // erzeugt und dann geteilt, damit die Folge auf jeder Maschine
        // dieselbe ist.
        let a = f64::from(i % 997) / 1000.0;
        let b = f64::from((i * 7) % 991) / 1000.0;

        let mut pools = ValuePools::new();
        pools.add(essence_pool, a);
        pools.add(gear_pool, b);

        let base = 100 + i % 5000;
        let f1 = pools.factor(essence_pool);
        let f2 = pools.factor(gear_pool);

        // **Der rohe Vergleich — und er muss die KLAMMERUNG vergleichen.**
        //
        // Der erste Anlauf stand hier als `(1.0 + a) * (1.0 + b)` gegen
        // `pools.factor(..) * pools.factor(..)`. Das ist DIESELBE Rechnung
        // zweimal — `factor()` berechnet genau `1.0 + summe`. Der Vergleich
        // konnte nie abweichen und hat gemeldet, die Vereinheitlichung sei
        // Kosmetik. Sie ist es nicht.
        let einzeln = (f64::from(base) * f1) * f2;
        let zusammen = f64::from(base) * (f1 * f2);
        if einzeln.to_bits() != zusammen.to_bits() {
            raw_differs += 1;
        }
        if damage_named(base, &pools, essence_pool, gear_pool)
            != damage_unified(base, &pools, essence_pool, gear_pool)
        {
            damage_differs += 1;
        }
    }

    VariantComparison {
        cases,
        raw_differs,
        damage_differs,
    }
}

/// Der Faktor eines Aufbaus, der sein Budget auf `pools` Toepfe **verteilt**.
///
/// Das ist der Affix-Aufbau: 13.1b gibt CATALYSIS, PURITY und YIELD je einen
/// eigenen Faktor, "weil man alle drei mischen will statt einen zu stapeln".
///
/// **Modellrechnung, keine Messung** — siehe Kopf dieser Datei.
#[must_use]
pub fn spread_factor(budget: f64, pools: u32) -> f64 {
    if pools == 0 {
        return 1.0;
    }
    let je_topf = budget / f64::from(pools);
    let mut faktor = 1.0;
    for _ in 0..pools {
        faktor *= 1.0 + je_topf;
    }
    faktor
}

/// Der Faktor eines Aufbaus, der sein Budget in **einem** Topf sammelt.
///
/// Das ist der Satz-Aufbau: Satz-Boni liegen mit den legendaeren Effekten in
/// einem Topf und sind dort additiv (13.1b).
#[must_use]
pub fn stacked_factor(budget: f64) -> f64 {
    1.0 + budget
}

/// Ab welchem Satz-Bonus ein gesammelter Aufbau einen verteilten einholt.
///
/// **Das ist AC2 des Issues**, und es ist die Zahl, auf die es ankommt: Eine
/// Messung, die nur "ausgeglichen" sagt, ist bei der naechsten
/// Zahlenaenderung wertlos. Der Kipppunkt sagt, wie viel Spielraum bleibt.
///
/// Ergebnis ist der Bonus, den der Satz-Aufbau braeuchte, um denselben Faktor
/// zu erreichen wie ein verteilter Aufbau mit `budget` auf `pools` Toepfen.
#[must_use]
pub fn tipping_point(budget: f64, pools: u32) -> f64 {
    spread_factor(budget, pools) - 1.0
}

/// Wie viel mehr der gesammelte Aufbau braucht, als Anteil.
///
/// `0.21` heisst: Der Satz braucht 21 % mehr Bonus fuer denselben Effekt.
#[must_use]
pub fn tipping_surcharge(budget: f64, pools: u32) -> f64 {
    if budget <= 0.0 {
        return 0.0;
    }
    tipping_point(budget, pools) / budget - 1.0
}

/// Wie viele Toepfe es gibt — fuer den Bericht.
#[must_use]
pub const fn pool_count() -> usize {
    POOL_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gear::{POOL_ESSENCE, POOL_GEAR_EFFECTS};

    #[test]
    fn der_verteilte_aufbau_schlaegt_den_gesammelten_bei_gleichem_budget() {
        // Der Kern von 13.1b: Was multiplikativ ist, wird gejagt. Als
        // Ungleichung geprueft, nicht gegen eine Zahl — die Zahlen sind
        // vorlaeufig, die Ordnung ist es nicht.
        for budget in [0.1, 0.3, 0.6, 1.2] {
            assert!(
                spread_factor(budget, 3) > stacked_factor(budget),
                "Budget {budget}: verteilt {} sollte ueber gesammelt {} liegen",
                spread_factor(budget, 3),
                stacked_factor(budget)
            );
        }
    }

    #[test]
    fn bei_null_budget_sind_beide_gleich() {
        // Der Vergleichsfall, in dem sich NICHTS aendern darf. Ohne ihn
        // belegte der Prueffall darueber nur, dass irgendetwas groesser ist.
        assert_eq!(spread_factor(0.0, 3), 1.0);
        assert_eq!(stacked_factor(0.0), 1.0);
        assert_eq!(tipping_point(0.0, 3), 0.0);
        assert_eq!(tipping_surcharge(0.0, 3), 0.0);
    }

    #[test]
    fn bei_einem_topf_gibt_es_keinen_unterschied() {
        // Ein verteilter Aufbau auf EINEN Topf ist ein gesammelter.
        for budget in [0.1, 0.6, 2.0] {
            assert_eq!(spread_factor(budget, 1), stacked_factor(budget));
        }
    }

    #[test]
    fn der_aufschlag_bei_einem_topf_ist_nicht_exakt_null_und_das_ist_richtig() {
        // **Der erste Anlauf verlangte hier exakt 0,0 und fiel um.** Der Grund
        // ist keine Ungenauigkeit der Rechnung, sondern eine Eigenschaft von
        // `f64`: `(1 + b) - 1` ist nicht `b`.
        //
        //   b = 0,1   ->   (1 + b) - 1 = 0,10000000000000009
        //   Aufschlag daraus: 8,88e-16 statt 0
        //
        // Eine Pruefung auf exakte Null waere also nicht streng, sondern
        // falsch. Geprueft wird stattdessen, dass der Rest im Bereich der
        // Rechengenauigkeit liegt — und dass er es bei DREI Toepfen NICHT tut,
        // sonst pruefte diese Schranke nichts.
        for budget in [0.1, 0.6, 2.0] {
            assert!(
                tipping_surcharge(budget, 1).abs() < 1e-12,
                "Budget {budget}: Aufschlag {} liegt ueber der Rechengenauigkeit",
                tipping_surcharge(budget, 1)
            );
            assert!(
                tipping_surcharge(budget, 3) > 1e-3,
                "Budget {budget}: bei drei Toepfen muss ein echter Aufschlag stehen, \
                 sonst faengt die Schranke oben alles"
            );
        }
    }

    #[test]
    fn der_kipppunkt_waechst_mit_dem_budget() {
        // Je groesser das Budget, desto weiter liegen die Aufbauten
        // auseinander — das ist der Grund, warum 13.1b Saetze und Legendaere
        // in EINEN Topf legt.
        let klein = tipping_surcharge(0.3, 3);
        let gross = tipping_surcharge(1.2, 3);
        assert!(
            gross > klein,
            "der Aufschlag muss mit dem Budget wachsen: {klein} -> {gross}"
        );
    }

    #[test]
    fn die_vereinheitlichung_ist_keine_kosmetik() {
        // **Messung 2, AC3 — und die Antwort lautet nein.**
        //
        // Mein erster Anlauf meldete "Kosmetik". Er verglich dieselbe Rechnung
        // zweimal und konnte gar nichts anderes finden. Der richtige
        // Vergleich — Klammerung gegen Klammerung — zeigt:
        //
        //   200 000 Faelle: 34,7 % mit abweichendem ROHWERT,
        //                   3 mit abweichendem GERUNDETEM Schaden
        //
        // Drei von 200 000 klingt nach nichts und ist es nicht: In einem
        // Projekt, dessen Gleichlauf auf BITGLEICHHEIT beruht, ist ein Weg,
        // der den Schaden in einem von 67 000 Treffern um einen Punkt
        // verschiebt, keine Kosmetik.
        let ergebnis = compare_variants(200_000, POOL_ESSENCE, POOL_GEAR_EFFECTS);
        assert!(
            ergebnis.is_measurable(),
            "der Unterschied ist nicht mehr messbar — entweder hat sich die Formel \
             geaendert oder der Vergleich prueft wieder dieselbe Rechnung zweimal"
        );
        assert!(ergebnis.damage_differs > 0);
    }

    #[test]
    fn der_rohe_wert_laeuft_viel_haeufiger_auseinander_als_der_gerundete() {
        // Der Gegenfall: Ohne ihn saehe es aus, als waere jeder
        // Bit-Unterschied gleich ein anderer Schaden. Die Rundung schluckt die
        // allermeisten — das ist der Grund, warum es bisher niemandem
        // aufgefallen ist.
        let ergebnis = compare_variants(200_000, POOL_ESSENCE, POOL_GEAR_EFFECTS);
        assert!(
            ergebnis.raw_differs > ergebnis.damage_differs * 100,
            "roh {} gegen gerundet {} — wenn beide nah beieinander liegen, schluckt die \
             Rundung nichts mehr, und die Erklaerung stimmt nicht",
            ergebnis.raw_differs,
            ergebnis.damage_differs
        );
    }

    #[test]
    fn die_drei_abweichenden_faelle_liegen_an_einer_rundungsgrenze() {
        // Ein benannter Fall statt einer Zahl — wer das hier liest, soll
        // sehen, WARUM es abweicht und nicht nur DASS.
        //
        //   2099 x 1,2 x 1,25
        //     einzeln     3148,4999999999995  ->  3148
        //     zusammen    3148,5              ->  3149
        //
        // Beide Wege sind richtig gerechnet. Der Unterschied entsteht erst
        // beim Runden, und nur weil das Ergebnis genau auf der Grenze liegt.
        let mut pools = ValuePools::new();
        pools.add(POOL_ESSENCE, 0.2);
        pools.add(POOL_GEAR_EFFECTS, 0.25);

        let einzeln = damage_named(2099, &pools, POOL_ESSENCE, POOL_GEAR_EFFECTS);
        let zusammen = damage_unified(2099, &pools, POOL_ESSENCE, POOL_GEAR_EFFECTS);
        assert_ne!(
            einzeln, zusammen,
            "der benannte Fall weicht nicht mehr ab — die Formel hat sich geaendert"
        );
        assert_eq!(einzeln, 3148);
        assert_eq!(zusammen, 3149);
    }
}
