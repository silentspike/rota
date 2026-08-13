//! Saetze (GAME-DESIGN 13.5, Issue E6-3).
//!
//! Das Gegenstueck zu `src/gear/sets.ts`.
//!
//! | Regel | Festlegung |
//! | --- | --- |
//! | Nur im Endgame | waehrend des Aufstiegs nur Einzelstuecke |
//! | Drei Bonus-Stufen | zwei, vier und sechs Teile |
//! | Frei kombinierbar | Teile passen auf beliebige Plaetze; gezaehlt wird nur die ANZAHL |
//! | Werte unten, Rotation oben | Werte-Boni auf den unteren Stufen |
//! | Ein Topf mit den Legendaeren | additiv untereinander (13.1b) |
//!
//! # Warum nur gezaehlt und nicht zugeordnet wird
//!
//! "Satz-Teile passen auf BELIEBIGE Plaetze" steht hier in der Bauform: Dieses
//! Modul kennt keine Plaetze. Es bekommt eine Liste "welcher Satz sitzt auf
//! Platz i" und zaehlt. Eine platz-abhaengige Regel liesse sich gar nicht
//! ausdruecken.
//!
//! # Die Schwelle gilt "ab", nicht "genau"
//!
//! Fuenf Teile geben denselben Bonus wie vier. Eine Pruefung auf `== 4` waere
//! die naheliegende Schreibweise und gaebe bei fuenf Teilen NICHTS — der
//! Spieler verloere einen Bonus, weil er ein Teil zu viel traegt.
//!
//! # Was hier NICHT steht
//!
//! **Welche 18 Boni es gibt.** Das ist Katalog-Inhalt und steht vor E10 aus
//! (GAME-DESIGN 22). Dieses Modul nimmt sie als Daten entgegen.

/// Kein Satz — ein gewoehnliches Stueck.
pub const NO_SET: i32 = -1;

/// Die drei Bonus-Stufen (13.5). **Nicht vorlaeufig.**
pub const SET_TIERS: [u32; 3] = [2, 4, 6];

/// Wie viele Saetze es gibt — einer je Endgame-Instanz.
pub const SET_COUNT: usize = 6;

/// Ein Werte-Bonus: ein Anteil fuer den Ausruestungs-Topf.
pub const BONUS_VALUE: u8 = 0;
/// Ein Rotations-Effekt: veraendert, wie die Rotation funktioniert.
pub const BONUS_ROTATION: u8 = 1;

/// Ein Bonus einer Stufe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetBonus {
    /// [`BONUS_VALUE`] oder [`BONUS_ROTATION`].
    pub kind: u8,
    /// Bei einem Werte-Bonus der Anteil, sonst 0.
    pub share: f64,
    /// Bei einem Rotations-Effekt seine Kennung, sonst leer.
    pub effect: &'static str,
}

/// Ein Satz: drei Stufen, eine Schule.
#[derive(Debug, Clone, Copy)]
pub struct SetDef {
    /// Kennnummer, zugleich Index in der Teile-Zaehlung.
    pub id: usize,
    /// Auf welche Schule er zielt (13.5).
    pub school: &'static str,
    /// Die Boni zu [`SET_TIERS`], in derselben Reihenfolge.
    pub tiers: [SetBonus; SET_TIERS.len()],
}

/// Zaehlt, wie viele Teile je Satz getragen werden.
///
/// Gibt zurueck, wie viele Plaetze ueberhaupt ein Satz-Teil tragen.
pub fn count_pieces(equipped: &[i32], out: &mut [u32]) -> u32 {
    for slot in out.iter_mut() {
        *slot = 0;
    }
    let mut getragen = 0;
    for set in equipped {
        if *set == NO_SET {
            continue;
        }
        let Ok(index) = usize::try_from(*set) else {
            continue;
        };
        if let Some(zaehler) = out.get_mut(index) {
            *zaehler += 1;
            getragen += 1;
        }
    }
    getragen
}

/// Wie viele Bonus-Stufen bei `pieces` Teilen ausgeloest sind.
///
/// **Die Schwelle gilt "ab", nicht "genau".**
#[must_use]
pub fn tiers_reached(pieces: u32) -> usize {
    SET_TIERS.iter().filter(|s| pieces >= **s).count()
}

/// Ist die Stufe `tier` bei `pieces` Teilen ausgeloest?
#[must_use]
pub fn tier_active(pieces: u32, tier: usize) -> bool {
    SET_TIERS.get(tier).is_some_and(|s| pieces >= *s)
}

/// Der gesamte **Werte**-Anteil aller ausgeloesten Satz-Boni.
///
/// Eine Summe und kein Faktor: Satz-Boni liegen im selben Topf wie die
/// legendaeren Effekte und sind dort additiv (13.1b).
#[must_use]
pub fn value_share_of(sets: &[SetDef], pieces: &[u32]) -> f64 {
    let mut summe = 0.0;
    for satz in sets {
        let getragen = pieces.get(satz.id).copied().unwrap_or(0);
        for (tier, bonus) in satz.tiers.iter().enumerate() {
            if bonus.kind == BONUS_VALUE && tier_active(getragen, tier) {
                summe += bonus.share;
            }
        }
    }
    summe
}

/// Die Kennungen aller ausgeloesten **Rotations**-Effekte.
///
/// In fester Reihenfolge — erst nach Satz, dann nach Stufe —, damit Spiel und
/// Simulator dieselbe Folge sehen (18.4).
#[must_use]
pub fn active_rotation_effects(sets: &[SetDef], pieces: &[u32]) -> Vec<&'static str> {
    let mut effekte = Vec::new();
    for satz in sets {
        let getragen = pieces.get(satz.id).copied().unwrap_or(0);
        for (tier, bonus) in satz.tiers.iter().enumerate() {
            if bonus.kind == BONUS_ROTATION && tier_active(getragen, tier) {
                effekte.push(bonus.effect);
            }
        }
    }
    effekte
}

/// Der Rotations-Effekt des schulunabhaengigen Sechsers (`docs/SATZ-KATALOG.md`).
///
/// > Der globale Cooldown sinkt um [`WHOLE_WORK_GCD_REDUCTION_MS`].
///
/// Gegenstueck: `SET_EFFECT_WHOLE_WORK` in `src/gear/sets.ts`.
pub const SET_EFFECT_WHOLE_WORK: &str = "whole-work";

/// Der Rotations-Effekt des PUTREFACTION-Sechsers (`docs/SATZ-KATALOG.md`).
///
/// > Zersetzungs-Stapel verfallen nicht, solange ein DoT laeuft.
///
/// Gegenstueck: `SET_EFFECT_SLOW_ROT` in `src/gear/sets.ts`.
pub const SET_EFFECT_SLOW_ROT: &str = "slow-rot";
/// Der Rotations-Effekt des COAGULATION-Sechsers (`docs/SATZ-KATALOG.md`).
///
/// > Ueberschuessiger Schutz ueber der Obergrenze wird zu Schaden.
///
/// Die Obergrenze ist eine STAPEL-Grenze; der Ueberschuss ist der Betrag, den
/// ein Druck auf eine volle Schicht beigetragen haette.
///
/// Gegenstueck: `SET_EFFECT_SEALED_VESSEL` in `src/gear/sets.ts`.
pub const SET_EFFECT_SEALED_VESSEL: &str = "sealed-vessel";
/// Der Rotations-Effekt des FIXATION-Sechsers (`docs/SATZ-KATALOG.md`).
///
/// > Der erste Kontroll-Ausgeber nach jedem Phasenwechsel kostet nichts.
///
/// „Kostet nichts" hebt den ABZUG auf, nicht die VORAUSSETZUNG: Die Punkte
/// muessen da sein, werden aber nicht ausgegeben.
///
/// Gegenstueck: `SET_EFFECT_LONG_GRIP` in `src/gear/sets.ts`.
pub const SET_EFFECT_LONG_GRIP: &str = "long-grip";

/// Um wie viel `The Whole Work` die Wartezeit senkt, in Millisekunden.
///
/// **Startwert, keine Balance-Entscheidung** (Orchestrator, 2026-08-11). Der
/// Prueflauf aus #118 misst, ob er traegt.
pub const WHOLE_WORK_GCD_REDUCTION_MS: u32 = 100;

/// Der Rotations-Effekt des SUBLIMATION-Sechsers (`docs/SATZ-KATALOG.md`).
///
/// > Der Ruecksetzer bekommt eine Ladung mehr.
///
/// Der Katalog wollte urspruenglich „auf die zwei laengsten Abklingzeiten
/// statt auf eine, macht lange Zauber planbar statt zufaellig". Der
/// Ruecksetzer ist aber ein AUSWAHLMODUS — der Spieler waehlt, nichts ist
/// zufaellig, und es gibt keine Auswahl nach Laenge. Entschieden
/// (Auftraggeber, 2026-08-13): +1 Ladung ueber denselben Summenweg wie
/// Zauber-Eigenschaft und Talent. Der Spieler waehlt zweimal statt einmal.
///
/// Gegenstueck: `SET_EFFECT_PERPETUAL_DRAFT` in `src/gear/sets.ts`.
pub const SET_EFFECT_PERPETUAL_DRAFT: &str = "perpetual-draft";

/// Wie viele Ladungen `Perpetual Draft` dem Ruecksetzer zulegt.
///
/// Gegenstueck: `PERPETUAL_DRAFT_EXTRA_CHARGE` in `src/gear/sets.ts`. Beide
/// Seiten muessen dieselbe Zahl nennen, sonst laufen Spiel und Simulator
/// auseinander, ohne dass eine Pruefsumme das zeigt — der Wert steht erst
/// im Vergleich der Auswahl-Zahlen, nicht im Schaden.
pub const PERPETUAL_DRAFT_EXTRA_CHARGE: u8 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    fn wert(share: f64) -> SetBonus {
        SetBonus {
            kind: BONUS_VALUE,
            share,
            effect: "",
        }
    }
    fn rotation(effect: &'static str) -> SetBonus {
        SetBonus {
            kind: BONUS_ROTATION,
            share: 0.0,
            effect,
        }
    }
    /// Ein Rotations-Bonus, der **faelschlich** einen Anteil traegt.
    ///
    /// Gebraucht fuer die Deckelungs-Pruefung: `rotation()` setzt `share` auf
    /// 0, und damit ist jede Pruefung der Filterung gruen, ohne sie zu
    /// beruehren — sie misst dann die Null, nicht das `kind`.
    fn rotation_mit_wert(effect: &'static str) -> SetBonus {
        SetBonus {
            kind: BONUS_ROTATION,
            share: 0.5,
            effect,
        }
    }
    fn satz_mit_falschem_sechser() -> SetDef {
        SetDef {
            id: 0,
            school: "CALCINATION",
            tiers: [wert(0.1), wert(0.2), rotation_mit_wert("test-a-sechser")],
        }
    }
    fn satz_a() -> SetDef {
        SetDef {
            id: 0,
            school: "CALCINATION",
            tiers: [wert(0.1), wert(0.2), rotation("test-a-sechser")],
        }
    }

    #[test]
    fn die_schwelle_gilt_ab_nicht_genau() {
        assert_eq!(tiers_reached(3), 1);
        assert_eq!(tiers_reached(5), tiers_reached(4));
        assert_eq!(tiers_reached(7), 3);
    }

    #[test]
    fn ohne_satz_gibt_es_nichts() {
        // Der Vergleichsfall — ohne ihn belegte der Rest nur, dass ueberhaupt
        // etwas passiert.
        let pieces = [0u32; SET_COUNT];
        assert_eq!(value_share_of(&[satz_a()], &pieces), 0.0);
        assert!(active_rotation_effects(&[satz_a()], &pieces).is_empty());
    }

    #[test]
    fn die_oberste_stufe_traegt_keine_werte() {
        // 13.5: Rotations-Effekte auf der sechsten Stufe, Werte darunter.
        //
        // **Diese Zusicherung ist umgezogen, nicht verschwunden.** Bis zur
        // Zusammenfuehrung der zwei Wege (2026-08-11) deckelte
        // `gear_effects_permille` selbst die Stufenzahl. Jetzt bekommt es einen
        // fertigen Anteil und kann nicht mehr wissen, woher er kommt — die
        // Deckelung sitzt hier, weil `value_share_of` nur Werte-Boni summiert.
        //
        // **Mit einem Satz, dessen sechste Stufe einen Anteil TRAEGT.** Mit
        // `satz_a()` waere der Fall gruen, ohne die Filterung zu beruehren:
        // `rotation()` setzt `share` auf 0, also kaeme auch ohne jede Pruefung
        // des `kind` dieselbe Zahl heraus. Auf der TS-Seite ist genau das
        // gemessen worden — die Mutation liess den Fall in seiner ersten
        // Fassung gruen.
        let mut alle_sechs = [0u32; SET_COUNT];
        alle_sechs[0] = 6;
        let mut nur_vier = [0u32; SET_COUNT];
        nur_vier[0] = 4;
        let satz = satz_mit_falschem_sechser();
        assert!(
            (value_share_of(&[satz], &alle_sechs) - value_share_of(&[satz], &nur_vier)).abs()
                < 1e-12,
            "die sechste Stufe traegt einen Werte-Bonus, obwohl sie es nicht darf"
        );
        // ZUSICHERUNG: Bei vier Teilen ist ueberhaupt etwas da — sonst
        // vergliche der Fall zwei Nullen und waere auch im Schadensfall gruen.
        assert!(value_share_of(&[satz], &nur_vier) > 0.0);
    }

    #[test]
    fn mehr_teile_wirken_staerker() {
        let mut zwei = [0u32; SET_COUNT];
        zwei[0] = 2;
        let mut vier = [0u32; SET_COUNT];
        vier[0] = 4;
        // Als Ungleichung, nicht gegen eine Zahl: 0 == 0 waere gruen.
        assert!(value_share_of(&[satz_a()], &vier) > value_share_of(&[satz_a()], &zwei));
        assert!(value_share_of(&[satz_a()], &zwei) > 0.0);
    }

    #[test]
    fn der_platz_spielt_keine_rolle() {
        let mut vorne = [0u32; SET_COUNT];
        let mut hinten = [0u32; SET_COUNT];
        count_pieces(&[0, 0, NO_SET, NO_SET, NO_SET], &mut vorne);
        count_pieces(&[NO_SET, NO_SET, NO_SET, 0, 0], &mut hinten);
        assert_eq!(vorne, hinten);
    }

    #[test]
    fn eine_unbekannte_kennung_wird_verworfen() {
        let mut pieces = [0u32; SET_COUNT];
        let getragen = count_pieces(&[SET_COUNT as i32, -5, 0], &mut pieces);
        assert_eq!(getragen, 1);
        assert_eq!(pieces[0], 1);
    }
}
