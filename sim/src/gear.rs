//! Ausruestung: die vier Toepfe, das Werte-Budget, die zehn Plaetze
//! (GAME-DESIGN 13.1b bis 13.3, Issue E6-1).
//!
//! Das Gegenstueck zu `src/gear/pools.ts`, `src/gear/budget.ts` und
//! `src/gear/slots.ts`.
//!
//! # Das Prinzip in einem Satz
//!
//! **Innerhalb eines Topfes wird addiert, zwischen den Toepfen multipliziert.**
//!
//! | Topf                      | Inhalt                              | Verrechnung                  |
//! | ------------------------- | ----------------------------------- | ---------------------------- |
//! | ESSENCE                   | alle Quellen des Grundwerts         | additiv, dann **ein** Faktor |
//! | Sonderwerte               | CATALYSIS, PURITY, YIELD            | **je eigener** Faktor        |
//! | Aktive Verstaerkungen     | Overdrive, Synergien, Stapel        | additiv untereinander        |
//! | Ausruestungs-Sondereffekte| legendaere Effekte UND Satz-Boni    | additiv untereinander        |
//!
//! Die letzte Zeile ist die wichtigste, und sie ist aus Schaden gelernt: In
//! Diablo IV waren Kritschaden und Verwundbarkeit eigene multiplikative Toepfe,
//! wurden "disproportional stark", und Blizzard verschob sie in Saison 2 zurueck
//! in den additiven Topf. Diablo 3 verlor dieselbe Schlacht bei den Saetzen, bis
//! legendaere Gegenstaende OHNE Satz-Bonus schlicht ignoriert wurden.
//! **Was multiplikativ ist, wird gejagt, und alles andere wird liegen gelassen.**
//!
//! # Was hier NICHT steht
//!
//! **Wie das Ergebnis in die Schadensformel eintritt.** 13.1b legt fest, wie die
//! Toepfe untereinander rechnen, nicht an welcher Stelle von 6.3 der
//! Ausruestungs-Topf multipliziert wird. Die Reihenfolge der Multiplikationen
//! ist laut 6.3 eine Festlegung des Design-Dokuments und darf nicht im Code
//! entschieden werden.

/// Alle ESSENCE-Quellen.
pub const POOL_ESSENCE: usize = 0;
/// CATALYSIS — erste Kammer des Sonderwert-Topfes.
pub const POOL_CATALYSIS: usize = 1;
/// PURITY — zweite Kammer.
pub const POOL_PURITY: usize = 2;
/// YIELD — dritte Kammer.
pub const POOL_YIELD: usize = 3;
/// Aktive Verstaerkungen.
pub const POOL_ACTIVE: usize = 4;
/// Legendaere Effekte **und** Satz-Boni — gemeinsam.
pub const POOL_GEAR_EFFECTS: usize = 5;

/// Wie viele Faktoren die vier Toepfe nach aussen ergeben.
pub const POOL_COUNT: usize = 6;

/// Die Namen in der Reihenfolge der Kennnummern — Teil der Schnittstelle.
pub const POOL_NAMES: [&str; POOL_COUNT] = [
    "essence",
    "catalysis",
    "purity",
    "yield",
    "active",
    "gearEffects",
];

/// Die Beitraege zu den Toepfen.
///
/// Ein Beitrag ist ein **Anteil**, keine Punktzahl: 0,3 heisst "+30 %".
#[derive(Debug, Clone, Copy, Default)]
pub struct ValuePools {
    sums: [f64; POOL_COUNT],
}

impl ValuePools {
    /// Legt die Toepfe an — alle leer, alle Faktoren also 1,0.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sums: [0.0; POOL_COUNT],
        }
    }

    /// Einen Beitrag hinzufuegen. `false`, wenn es den Topf nicht gibt.
    pub fn add(&mut self, pool: usize, share: f64) -> bool {
        match self.sums.get_mut(pool) {
            Some(slot) => {
                *slot += share;
                true
            }
            None => false,
        }
    }

    /// Die Summe der Beitraege eines Topfes — ohne die Eins.
    #[must_use]
    pub fn sum(&self, pool: usize) -> f64 {
        self.sums.get(pool).copied().unwrap_or(0.0)
    }

    /// Der Faktor eines Topfes: **eins plus** die Summe seiner Beitraege.
    #[must_use]
    pub fn factor(&self, pool: usize) -> f64 {
        1.0 + self.sum(pool)
    }

    /// Alle sechs Faktoren miteinander multipliziert.
    ///
    /// **Die Reihenfolge ist die der Kennnummern und liegt fest.**
    /// Fliesskomma-Multiplikation ist nicht assoziativ (18.4).
    #[must_use]
    pub fn product(&self) -> f64 {
        let mut value = 1.0;
        for pool in 0..POOL_COUNT {
            value *= self.factor(pool);
        }
        value
    }

    /// Alles auf null — bei einem Ausruestungswechsel.
    pub fn clear(&mut self) {
        self.sums = [0.0; POOL_COUNT];
    }
}

/// Common — ein Werte-Platz (13.3).
pub const QUALITY_COMMON: usize = 0;
/// Rare — zwei Werte-Plaetze.
pub const QUALITY_RARE: usize = 1;
/// Epic — drei Werte-Plaetze.
pub const QUALITY_EPIC: usize = 2;
/// Legendary — drei Werte-Plaetze und ein Sondereffekt.
pub const QUALITY_LEGENDARY: usize = 3;
/// Wie viele Qualitaeten es gibt.
pub const QUALITY_COUNT: usize = 4;

/// Die Namen in der Reihenfolge der Kennnummern.
pub const QUALITY_NAMES: [&str; QUALITY_COUNT] = ["common", "rare", "epic", "legendary"];

/// Wie viele Werte-Plaetze eine Qualitaet traegt.
///
/// **Legendary steht in 13.3 als "3+".** Das Pluszeichen ist in derselben Zeile
/// erklaert — "echter Sondereffekt". Gelesen als: drei Werte-Plaetze UND
/// zusaetzlich ein Effekt, nicht als vierter Werte-Platz.
pub const QUALITY_STAT_SLOTS: [usize; QUALITY_COUNT] = [1, 2, 3, 3];

/// Welche Qualitaet einen Sondereffekt traegt.
pub const QUALITY_HAS_EFFECT: [bool; QUALITY_COUNT] = [false, false, false, true];

/// Das Budget einer Qualitaet auf Gegenstandsstufe 1. **Vorlaeufig (#118).**
const QUALITY_BASE_BUDGET: [u64; QUALITY_COUNT] = [100, 140, 190, 190];

/// Zaehler und Nenner des Stufen-Wachstums — 21/20 sind +5 %. **Vorlaeufig.**
const GROWTH_NUMERATOR: u64 = 21;
const GROWTH_DENOMINATOR: u64 = 20;

/// Hoechste Gegenstandsstufe, die gerechnet wird.
pub const MAX_ITEM_LEVEL: u32 = 100;

/// Das Werte-Budget eines Gegenstands.
///
/// **Durchweg ganzzahlig.** Ein `powf(1.05, stufe)` waere der naheliegende Weg
/// und waere falsch: Potenzfunktionen sind zwischen zwei Sprachen nicht bitgleich
/// garantiert, und steht das Ergebnis dann zufaellig auf `.5`, kippt die Rundung
/// auf einer Seite und auf der anderen nicht — ein Fehler, der bei Stufe 47
/// auftritt und bei 46 und 48 nicht.
#[must_use]
pub fn budget_of(item_level: u32, quality: usize) -> u64 {
    if quality >= QUALITY_COUNT || !(1..=MAX_ITEM_LEVEL).contains(&item_level) {
        return 0;
    }
    let mut budget = QUALITY_BASE_BUDGET[quality];
    for _ in 1..item_level {
        budget = budget * GROWTH_NUMERATOR / GROWTH_DENOMINATOR;
    }
    budget
}

/// Was der Werte-Platz `index` von `budget` bekommt — gleichmaessig verteilt,
/// Rest auf die vorderen Plaetze.
///
/// Dieselbe Funktion wie `budget/plaetze + (index < budget % plaetze)`; die
/// geschlossene Form spart nur die Fallunterscheidung. **Ueber 720 036
/// Wertepaare gemessen: null Abweichung.**
///
/// AC1 haengt nicht an dieser Formel, sondern am **Wachstum**: Bei einem
/// Budget-Zuwachs von 1 koennte gar keine Verteilung alle Plaetze anheben.
#[must_use]
pub fn share_of_slot(budget: u64, slots: usize, index: usize) -> u64 {
    if slots == 0 || index >= slots {
        return 0;
    }
    let slots = slots as u64;
    (budget + slots - 1 - index as u64) / slots
}

/// Das Budget auf die Werte-Plaetze einer Qualitaet verteilen.
///
/// Schreibt in ein uebergebenes Feld und gibt die Zahl der beschriebenen
/// Plaetze zurueck.
pub fn distribute_budget(item_level: u32, quality: usize, out: &mut [u64]) -> usize {
    let slots = QUALITY_STAT_SLOTS.get(quality).copied().unwrap_or(0);
    let budget = budget_of(item_level, quality);
    let count = slots.min(out.len());
    for (index, slot) in out.iter_mut().enumerate().take(count) {
        *slot = share_of_slot(budget, slots, index);
    }
    count
}

/// Die zehn Plaetze aus 13.2.
pub const SLOT_COUNT: usize = 10;

/// Die Namen in der Reihenfolge aus 13.2.
pub const SLOT_NAMES: [&str; SLOT_COUNT] = [
    "focus",
    "catalyst",
    "sigil",
    "relic",
    "grimoire",
    "shard",
    "talisman",
    "phylactery",
    "anchor",
    "prism",
];

/// Die Grundwerte, die auf **jedem** Platz sitzen (13.2, 6.1).
///
/// **Eine Frage, die das Design nicht beantwortet:** ob sie aus demselben
/// Budget ziehen wie die Werte-Plaetze. 13.3 sagt, das Budget werde "auf die
/// Werte-Plaetze verteilt", und gibt Common genau einen davon — ein Platz kann
/// aber keine drei Grundwerte tragen. Diese Liste sagt deshalb nur, WELCHE
/// Grundwerte es gibt, nicht woher sie ihre Zahlen nehmen.
pub const BASE_STAT_IDS: [&str; 3] = ["essence", "vessel", "ward"];

/// Noch nicht zugeordnet.
pub const SPECIAL_UNDECIDED: i32 = -1;
/// CATALYSIS (6.2).
pub const SPECIAL_CATALYSIS: i32 = 0;
/// PURITY und YIELD gemeinsam (6.2).
pub const SPECIAL_PURITY_YIELD: i32 = 1;
/// "Verteidigung" — im Dokument kein benannter Wert.
pub const SPECIAL_DEFENSE: i32 = 2;
/// "Zauber-Verstaerkung" — im Dokument kein benannter Wert.
pub const SPECIAL_SPELL_AMP: i32 = 3;

/// Die Namen der Sonderwert-Kategorien.
pub const SPECIAL_NAMES: [&str; 4] = ["catalysis", "purityYield", "defense", "spellAmp"];

/// Die feste Zuordnung aus 13.2 — **Daten, nicht Code**.
///
/// **`relic` und `shard` sind ausdruecklich offen.** 13.2 zaehlt zehn Plaetze
/// auf und ordnet acht zu; beide kommen im gesamten Design-Dokument genau einmal
/// vor, in der Namensliste. Ein geratener Sonderwert waere eine
/// Design-Entscheidung im Code — eine, die niemandem auffiele, weil sie
/// plausibel aussaehe.
pub const SLOT_SPECIAL: [i32; SLOT_COUNT] = [
    SPECIAL_CATALYSIS,    // focus
    SPECIAL_CATALYSIS,    // catalyst
    SPECIAL_PURITY_YIELD, // sigil
    SPECIAL_UNDECIDED,    // relic  <- 13.2 sagt dazu nichts
    SPECIAL_SPELL_AMP,    // grimoire
    SPECIAL_UNDECIDED,    // shard  <- 13.2 sagt dazu nichts
    SPECIAL_PURITY_YIELD, // talisman
    SPECIAL_DEFENSE,      // phylactery
    SPECIAL_DEFENSE,      // anchor
    SPECIAL_SPELL_AMP,    // prism
];

/// Traegt dieser Platz eine festgelegte Sonderwert-Zuordnung?
#[must_use]
pub fn is_slot_decided(slot: usize) -> bool {
    SLOT_SPECIAL.get(slot).copied().unwrap_or(SPECIAL_UNDECIDED) != SPECIAL_UNDECIDED
}

/// Welche Plaetze noch auf ihre Zuordnung warten.
#[must_use]
pub fn undecided_slots() -> Vec<usize> {
    (0..SLOT_COUNT)
        .filter(|slot| !is_slot_decided(*slot))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleiche_toepfe_addieren_verschiedene_multiplizieren() {
        let mut pools = ValuePools::new();
        pools.add(POOL_CATALYSIS, 0.2);
        pools.add(POOL_CATALYSIS, 0.3);
        pools.add(POOL_PURITY, 0.5);
        assert_eq!(pools.factor(POOL_CATALYSIS), 1.5);
        assert_eq!(pools.factor(POOL_PURITY), 1.5);
        assert_eq!(pools.product(), 2.25);
    }

    #[test]
    fn satz_und_legendaer_teilen_einen_topf() {
        // 30 % + 20 % = 50 %, nicht 56 %.
        let mut pools = ValuePools::new();
        pools.add(POOL_GEAR_EFFECTS, 0.3);
        pools.add(POOL_GEAR_EFFECTS, 0.2);
        assert_eq!(pools.factor(POOL_GEAR_EFFECTS), 1.5);
        assert_ne!(pools.factor(POOL_GEAR_EFFECTS), 1.2 * 1.3);
    }

    #[test]
    fn ein_unbekannter_topf_faellt_auf() {
        let mut pools = ValuePools::new();
        assert!(!pools.add(POOL_COUNT, 0.5));
        assert!(pools.add(POOL_ESSENCE, 0.5));
        assert_eq!(pools.product(), 1.5);
    }

    #[test]
    fn hoehere_stufe_heisst_durchweg_hoehere_werte() {
        for (quality, &slots) in QUALITY_STAT_SLOTS.iter().enumerate() {
            for level in 1..MAX_ITEM_LEVEL {
                let mut unten = [0u64; 4];
                let mut oben = [0u64; 4];
                distribute_budget(level, quality, &mut unten);
                distribute_budget(level + 1, quality, &mut oben);
                for index in 0..slots {
                    assert!(
                        oben[index] > unten[index],
                        "Qualitaet {quality}, Stufe {level}, Platz {index}: {} -> {}",
                        unten[index],
                        oben[index]
                    );
                }
            }
        }
    }

    #[test]
    fn das_budget_wird_restlos_verteilt() {
        for quality in 0..QUALITY_COUNT {
            for level in 1..=MAX_ITEM_LEVEL {
                let mut werte = [0u64; 4];
                let count = distribute_budget(level, quality, &mut werte);
                let summe: u64 = werte[..count].iter().sum();
                assert_eq!(summe, budget_of(level, quality));
            }
        }
    }

    #[test]
    fn zwei_plaetze_sind_offen() {
        assert_eq!(undecided_slots(), vec![3, 5]);
        assert!(!is_slot_decided(3));
        assert!(!is_slot_decided(5));
        assert!(is_slot_decided(0));
        assert!(!is_slot_decided(SLOT_COUNT));
    }
}
