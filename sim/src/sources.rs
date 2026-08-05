//! Die fuenf Quellen der Sonderwerte (GAME-DESIGN 6.2, 12.5, 13.3, 13.5;
//! Issue #178).
//!
//! Das Gegenstueck zu `src/gear/sources.ts` — Zeile fuer Zeile dieselbe
//! Rechnung, ganzzahlig in Promille.
//!
//! # Die Reihenfolge ist umgekehrt zur naheliegenden
//!
//! **Nicht** "Deckel durch zehn Plaetze = Wert je Platz". Diese Rechnung saehe
//! nur die Affixe und unterstellte, dass sie die einzige Quelle sind.
//!
//! # Wofuer die Summe da ist
//!
//! Der Tempo-Deckel wird **gemessen, nicht gesetzt** (6.7, Fassung vom
//! 2026-08-04). Die Summe aus dieser Datei ist die Obergrenze, **unter** der ein
//! gemessener Deckel liegen muss:
//!
//! * Liegt er **darueber**, ist er unerreichbar und damit wirkungslos.
//! * Liegt er **genau darauf**, ist "alles auf Tempo" wieder die einzig
//!   richtige Antwort statt einer Entscheidung.
//! * Nur **darunter** entsteht die Frage "was lasse ich dafuer weg".
//!
//! # Dauerhaft und kurzzeitig sind zwei verschiedene Summen
//!
//! 6.2 trennt sie ausdruecklich: CATALYSIS "waechst **dauerhaft** ueber Talente
//! und Ausruestung, **kurzzeitig** ueber Overdrive". Ein Deckel greift im
//! Spitzenmoment anders als im Dauerzustand.
//!
//! # Die Zahlen sind vorlaeufig, ihre Struktur nicht

use crate::gear::{distribute_budget, WornValues, QUALITY_EPIC};

/// Affixe auf getragenen Gegenstaenden.
pub const SOURCE_AFFIXES: usize = 0;
/// Der globale Talentbaum (12.5).
pub const SOURCE_TALENTS: usize = 1;
/// Selbst-Verstaerkungen aus Zaubern — vor allem Overdrive (7.4).
pub const SOURCE_SPELLS: usize = 2;
/// Legendaere Sondereffekte (13.3).
pub const SOURCE_LEGENDARY: usize = 3;
/// Satz-Boni (13.5).
pub const SOURCE_SETS: usize = 4;
/// Wie viele Quellen es gibt.
pub const SOURCE_COUNT: usize = 5;

/// Die Namen in der Reihenfolge der Kennnummern — Teil der Schnittstelle.
pub const SOURCE_NAMES: [&str; SOURCE_COUNT] =
    ["affixes", "talents", "spells", "legendary", "sets"];

/// CATALYSIS — Tempo.
pub const STAT_CATALYSIS: usize = 0;
/// PURITY — Kritchance.
pub const STAT_PURITY: usize = 1;
/// YIELD — Kritschaden.
pub const STAT_YIELD: usize = 2;
/// Wie viele Sonderwerte es gibt (6.2).
pub const STAT_COUNT: usize = 3;

/// Die Namen in der Reihenfolge der Kennnummern.
pub const STAT_NAMES: [&str; STAT_COUNT] = ["catalysis", "purity", "yield"];

/// Wie viele **Budget-Punkte** ein Promille eines Sonderwerts kostet.
/// **Vorlaeufig.**
///
/// # Warum ausgerechnet 250 — zwei unabhaengige Gruende
///
/// **Erstens nennt 6.7 die Groessenordnung selbst.** Als Beispiel fuer einen
/// Fund steht dort "+3 % Tempo". Ein Epic-Affix auf der hoechsten
/// Gegenstandsstufe traegt 7527 Budget-Punkte; geteilt durch 250 sind das genau
/// 30 Promille, also **3 %**.
///
/// **Zweitens macht erst dieser Wert den einzigen festen Deckel des Spiels
/// erreichbar.** 6.7 sagt ueber die 100 % bei PURITY: "Das ist ein erreichbares
/// Ziel, auf das sich ein ganzer Aufbau ausrichten laesst." Mit einem Umrechner
/// von 500 kaeme ein voller Aufbau auf 680 Promille und bliebe **unter** dem
/// Deckel — das Ziel waere unerreichbar und der Satz im Design falsch.
pub const BUDGET_PER_PERMILLE: u64 = 250;

/// Was ein **einzelner Affix** einer Qualitaet auf einer Gegenstandsstufe
/// Was ein Gegenstand dieser Stufe an **je einem Grundwert** traegt, in
/// Promille (Issue #282).
///
/// Das Gegenstueck zu `basePermille` in `src/gear/sources.ts` — beide Seiten
/// rechnen dieselbe Zahl aus derselben Kurve.
///
/// # Warum das neben [`affix_permille`] steht und nicht darin
///
/// Grundwerte und Affixe sind zwei verschiedene Quellen (GAME-DESIGN 13.5):
/// Affixe haengen an Budget, Stufe **und** Qualitaet, Grundwerte **nur** an
/// der Gegenstandsstufe.
///
/// > „Ein Gegenstand traegt IMMER die drei Grundwerte … so wie ein
/// > Diablo-Gegenstand immer Ruestung traegt." (GAME-DESIGN 1346, 1359)
///
/// Deshalb nimmt diese Funktion **keine** Qualitaet entgegen — nicht aus
/// Vergesslichkeit, sondern weil sie keine hat.
///
/// # Was sie NICHT tut
///
/// **Sie ist nicht fuer Affixe.** Dafuer steht [`affix_permille`] daneben. Der
/// Name hier ist kuerzer, und das ist der einzige Grund, aus dem jemand die
/// falsche greifen wuerde.
///
/// Sie liefert auch keine Summe ueber alle Plaetze: Es ist der Wert **eines**
/// Gegenstands, je **einem** Grundwert.
///
/// # Die Basis kommt herein, sie steht nicht hier
///
/// Drei Grundwerte, drei Basen — dieselbe Zahl bedeutet bei multiplikativem
/// Schaden, additivem Leben und der Minderung etwas Verschiedenes. Die Werte
/// stehen auf der TypeScript-Seite als `ESSENCE_AT_LEVEL_1`,
/// `VESSEL_AT_LEVEL_1` und `WARD_AT_LEVEL_1` und sind **unkalibriert**;
/// kalibriert werden sie am Skalierungsfaktor aus dem Balance-Laeufer
/// (`docs/ENTSCHEIDUNGEN-OFFEN.md`, Punkt 7).
#[must_use]
pub fn base_permille(item_level: u32, base_at_level_1: u32) -> u32 {
    if !(1..=crate::gear::MAX_ITEM_LEVEL).contains(&item_level) {
        return 0;
    }
    let promille = crate::data::LEVEL_FACTOR_PER_MILLE
        .get(item_level as usize - 1)
        .copied()
        .unwrap_or(1000);
    u32::try_from(u64::from(base_at_level_1) * u64::from(promille) / 1000).unwrap_or(u32::MAX)
}

/// traegt, in Promille. Abgeleitet aus dem Budget, nicht daneben gesetzt.
#[must_use]
pub fn affix_permille(item_level: u32, quality: usize) -> u32 {
    let mut slots = [0u64; 4];
    let count = distribute_budget(item_level, quality, &mut slots);
    if count == 0 {
        return 0;
    }
    (slots[0] / BUDGET_PER_PERMILLE) as u32
}

/// Das Maximum aus **Affixen**: jeder Platz mit der besten Qualitaet, jeder
/// Affix auf denselben Wert.
#[must_use]
pub fn max_from_affixes(item_level: u32) -> u32 {
    let mut slots = [0u64; 4];
    let per_item = distribute_budget(item_level, QUALITY_EPIC, &mut slots);
    let mut sum = 0u64;
    for slot in slots.iter().take(per_item) {
        sum += slot / BUDGET_PER_PERMILLE;
    }
    (sum * crate::gear::SLOT_COUNT as u64) as u32
}

/// Die Maxima der vier uebrigen Quellen, je Sonderwert, in Promille.
/// **Alle Zahlen vorlaeufig.**
///
/// | Quelle    | Begruendung der Groessenordnung |
/// | --------- | ------------------------------- |
/// | Talente   | 12.5: ~50 Punkte auf sieben Baeume, "niemals alles" |
/// | Zauber    | 7.4: Overdrive, "kurzer, **starker** Tempo-Schub" — nur Tempo |
/// | Legendaer | 13.3: der Sondereffekt "veraendert die Rotation", ein Wert-Bonus ist die Ausnahme |
/// | Saetze    | 13.5: Werte-Boni auf den **unteren** Stufen, Rotation auf der sechsten |
const MAX_PERMILLE: [[u32; STAT_COUNT]; SOURCE_COUNT] = [
    [0, 0, 0],       // Affixe — kommen aus dem Budget
    [100, 100, 200], // Talente
    [300, 0, 0],     // Zauber (Overdrive) — nur Tempo, nur kurzzeitig
    [50, 50, 100],   // legendaere Effekte
    [80, 80, 160],   // Satz-Boni, zwei Stufen zusammen
];

/// Welche Quellen **dauerhaft** wirken.
///
/// Der Overdrive ist der einzige befristete Beitrag (6.2, 7.4).
#[must_use]
pub const fn is_permanent(source: usize) -> bool {
    source != SOURCE_SPELLS
}

/// Das Maximum einer Quelle fuer einen Sonderwert, in Promille.
#[must_use]
pub fn max_of(source: usize, stat: usize, item_level: u32) -> u32 {
    if source >= SOURCE_COUNT || stat >= STAT_COUNT {
        return 0;
    }
    if source == SOURCE_AFFIXES {
        return max_from_affixes(item_level);
    }
    MAX_PERMILLE[source][stat]
}

/// Die Summe aller **dauerhaften** Quellen — die Zahl, unter der ein gemessener
/// Deckel liegen muss.
#[must_use]
pub fn permanent_max(stat: usize, item_level: u32) -> u32 {
    let mut sum = 0;
    for source in 0..SOURCE_COUNT {
        if is_permanent(source) {
            sum += max_of(source, stat, item_level);
        }
    }
    sum
}

/// Die Summe **aller** Quellen, einschliesslich der befristeten.
#[must_use]
pub fn total_max(stat: usize, item_level: u32) -> u32 {
    let mut sum = 0;
    for source in 0..SOURCE_COUNT {
        sum += max_of(source, stat, item_level);
    }
    sum
}

/// Was ein **einzelner legendaerer Effekt** zu Topf 4 beitraegt, in Promille.
///
/// **Vorlaeufig — der Wert wird gemessen (#118).** Eine Begruendung, warum
/// ausgerechnet diese Zahl, gibt es nicht und soll es nicht geben: Eine Zahl
/// mit Begruendungs-Anschein waere schlimmer als eine offen gesetzte (6.7).
pub const LEGENDARY_EFFECT_PERMILLE: u32 = 200;

/// Was ein **Satz-Bonus** je Werte-Stufe zu Topf 4 beitraegt, in Promille.
/// **Vorlaeufig — der Wert wird gemessen (#118).**
pub const SET_BONUS_PERMILLE: u32 = 150;

/// Wie viele Satz-Stufen Werte-Boni tragen (13.5).
///
/// Die oberste traegt Rotations-Effekte statt Werten. Abgeleitet aus
/// [`crate::sets::SET_TIERS`], nicht abgetippt.
pub const SET_VALUE_TIERS: u32 = crate::sets::SET_TIERS.len() as u32 - 1;

/// Topf 4 aus getragenen legendaeren Effekten und Satz-Stufen.
///
/// **Beide Quellen, eine Zahl** — der Zweck des gemeinsamen Topfes (13.1b):
/// Innerhalb wird addiert, also konkurrieren sie, statt sich zu multiplizieren.
#[must_use]
pub fn gear_effects_permille(legendaries: u32, set_tiers: u32) -> u32 {
    legendaries * LEGENDARY_EFFECT_PERMILLE + set_tiers.min(SET_VALUE_TIERS) * SET_BONUS_PERMILLE
}

/// Der Aufbau, der **alles** auf einen Wert setzt.
///
/// Ohne Overdrive gerechnet — ein Dauerwert, der einen befristeten Spitzenwert
/// enthaelt, waere eine falsche Zahl.
#[must_use]
pub fn pure_build(stat: usize, item_level: u32) -> WornValues {
    let alles = permanent_max(stat, item_level);
    WornValues {
        catalysis: if stat == STAT_CATALYSIS { alles } else { 0 },
        purity: if stat == STAT_PURITY { alles } else { 0 },
        yield_bonus: if stat == STAT_YIELD { alles } else { 0 },
        // Ein reiner Aufbau setzt genau EINEN Sonderwert. Die Grundwerte
        // stehen daneben und werden von der Verteilung nicht beruehrt.
        essence: 0,
        vessel: 0,
        ward: 0,
        gear_effects: 0,
    }
}

/// Der **Satz-Aufbau**: dasselbe Budget, aber vollstaendig in Topf 4.
///
/// # Was hier gegen was antritt (13.5, Issue #118)
///
/// > Satz-Boni und **Affixe** muessen **etwa gleichwertig** sein. Kippt es in
/// > die eine Richtung, tragen alle nur Saetze; kippt es in die andere, sind
/// > die Saetze wertlos.
///
/// Der Unterschied ist nicht die Menge, sondern die **Verrechnung**: Ein
/// Affix-Aufbau verteilt sein Budget auf drei Wirkwege, die sich
/// multiplizieren; ein Satz-Aufbau sammelt alles in einem Topf, in dem
/// addiert wird (13.1b).
///
/// **Beide bekommen deshalb dasselbe Budget** — sonst verglichen man nicht
/// zwei Wege, sondern zwei Budgets.
#[must_use]
pub fn set_build(reference_stat: usize, item_level: u32) -> WornValues {
    WornValues {
        gear_effects: permanent_max(reference_stat, item_level),
        ..WornValues::none()
    }
}

/// Der gleichmaessig gemischte Aufbau — **dasselbe Budget** wie
/// [`pure_build`] fuer denselben Bezugswert.
///
/// Die erste Fassung nahm das groesste der drei Maxima und gab dem gemischten
/// Aufbau damit mehr Budget; er haette gewonnen, ohne dass die Verteilung etwas
/// damit zu tun gehabt haette.
#[must_use]
pub fn mixed_build(reference_stat: usize, item_level: u32) -> WornValues {
    let drittel = permanent_max(reference_stat, item_level) / STAT_COUNT as u32;
    WornValues {
        catalysis: drittel,
        purity: drittel,
        yield_bonus: drittel,
        // Auch der gemischte Aufbau verteilt nur das Affix-Budget.
        essence: 0,
        vessel: 0,
        ward: 0,
        gear_effects: 0,
    }
}

/// Was ein Aufbau insgesamt einsetzt — fuer den fairen Vergleich.
#[must_use]
pub const fn build_total(build: &WornValues) -> u32 {
    build.catalysis + build.purity + build.yield_bonus + build.gear_effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gear::{MAX_ITEM_LEVEL, QUALITY_COMMON};

    #[test]
    fn ein_epic_affix_traegt_die_drei_prozent_aus_6_7() {
        assert_eq!(affix_permille(MAX_ITEM_LEVEL, QUALITY_EPIC), 30);
    }

    #[test]
    fn keine_quelle_bleibt_bei_allen_drei_werten_leer() {
        for (source, name) in SOURCE_NAMES.iter().enumerate() {
            let summe: u32 = (0..STAT_COUNT)
                .map(|stat| max_of(source, stat, MAX_ITEM_LEVEL))
                .sum();
            assert!(summe > 0, "Quelle {name} traegt nirgends etwas bei");
        }
    }

    #[test]
    fn der_overdrive_wirkt_nur_auf_tempo() {
        assert!(max_of(SOURCE_SPELLS, STAT_CATALYSIS, MAX_ITEM_LEVEL) > 0);
        assert_eq!(max_of(SOURCE_SPELLS, STAT_PURITY, MAX_ITEM_LEVEL), 0);
        assert_eq!(max_of(SOURCE_SPELLS, STAT_YIELD, MAX_ITEM_LEVEL), 0);
    }

    #[test]
    fn der_purity_deckel_bleibt_erreichbar() {
        // 6.7: "Das ist ein erreichbares Ziel, auf das sich ein ganzer Aufbau
        // ausrichten laesst." Diese Pruefung hat die Wahl des Umrechners
        // bestimmt — mit 500 Punkten je Promille waere das Ziel unerreichbar.
        assert!(permanent_max(STAT_PURITY, MAX_ITEM_LEVEL) >= 1000);
        // Und nicht mit einer Hand voll Gegenstaende: Affixe allein reichen nicht.
        assert!(max_of(SOURCE_AFFIXES, STAT_PURITY, MAX_ITEM_LEVEL) < 1000);
    }

    #[test]
    fn dauerhaft_und_kurzzeitig_sind_verschieden() {
        assert!(!is_permanent(SOURCE_SPELLS));
        assert!(
            total_max(STAT_CATALYSIS, MAX_ITEM_LEVEL)
                > permanent_max(STAT_CATALYSIS, MAX_ITEM_LEVEL)
        );
        // Bei den Kritwerten fallen beide zusammen — der Overdrive wirkt dort nicht.
        assert_eq!(
            total_max(STAT_PURITY, MAX_ITEM_LEVEL),
            permanent_max(STAT_PURITY, MAX_ITEM_LEVEL)
        );
    }

    #[test]
    fn beide_aufbauten_setzen_dasselbe_budget_ein() {
        for (stat, name) in STAT_NAMES.iter().enumerate() {
            let rein = build_total(&pure_build(stat, MAX_ITEM_LEVEL));
            let gemischt = build_total(&mixed_build(stat, MAX_ITEM_LEVEL));
            assert!(gemischt <= rein, "Bezugswert {name}");
            assert!(
                rein - gemischt < STAT_COUNT as u32,
                "Bezugswert {name}: {rein} gegen {gemischt}"
            );
        }
    }

    #[test]
    fn die_beiden_quellen_von_topf_4_addieren_sich() {
        // **AC3 von #195.** Ein Prueffall, der nur zeigt "beide zusammen sind
        // staerker", belegt nichts — das gilt additiv wie multiplikativ. Die
        // Zahlen muessen AUSEINANDERLIEGEN.
        assert_eq!(
            gear_effects_permille(1, 1),
            LEGENDARY_EFFECT_PERMILLE + SET_BONUS_PERMILLE
        );

        let additiv = 1.0 + 0.2 + 0.15;
        let multiplikativ = 1.2 * 1.15;
        assert_ne!(
            additiv, multiplikativ,
            "die beiden Faelle sind nicht unterscheidbar — der Prueffall taugt nichts"
        );

        let worn = WornValues {
            gear_effects: gear_effects_permille(1, 1),
            ..WornValues::none()
        };
        assert!((worn.gear_effect_factor() - additiv).abs() < 1e-12);
        assert!((worn.gear_effect_factor() - multiplikativ).abs() > 1e-3);
    }

    #[test]
    fn die_oberste_satz_stufe_traegt_keine_werte() {
        // 13.5: Rotations-Effekte auf der sechsten Stufe, Werte darunter.
        assert_eq!(
            gear_effects_permille(0, 99),
            SET_VALUE_TIERS * SET_BONUS_PERMILLE
        );
        assert_eq!(SET_VALUE_TIERS, 2);
    }

    #[test]
    fn ein_common_affix_ist_groesser_als_ein_epic_affix() {
        // Ein Common traegt EINEN Affix mit dem ganzen Budget; ein Epic teilt
        // seins auf drei. Drei Epic-Affixe sind zusammen trotzdem mehr.
        let common = affix_permille(MAX_ITEM_LEVEL, QUALITY_COMMON);
        let epic = affix_permille(MAX_ITEM_LEVEL, QUALITY_EPIC);
        assert!(common > epic);
        assert!(epic * 3 > common);
    }
}
