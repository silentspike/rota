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
