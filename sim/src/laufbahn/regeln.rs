//! Die Regeln UEBER dem Kampf, wie das Spiel sie rechnet (Issue #922).
//!
//! # Warum hier ein Nachbau steht
//!
//! Die Regeln leben in TypeScript (`src/loot/`, `src/progression/`,
//! `src/gear/catalog.ts`, `src/loot/estimate.ts`,
//! `src/devtools/expected-gear.ts`), und der Simulator kann sie nicht
//! aufrufen: Er ist eine Rust-Bibliothek ohne Laufzeit fuer JavaScript. Ein
//! Datenexport reicht auch nicht, weil die Schleife die Regeln mit Eingaben
//! ruft, die erst im Lauf entstehen — eine Stufe nach dem vierten Sieg, ein
//! Material-Vorrat nach dem neunten.
//!
//! Ein Nachbau antwortet plausibel aus einer zweiten Quelle. **Deshalb steht
//! jede Funktion hier unter einer Paritaets-Aufzeichnung**:
//! `tests/parity/recordings/laufbahn-regeln.json` entsteht aus den
//! Spielfunktionen selbst (`tests/parity/laufbahn-regeln.ts`), und
//! `sim/tests/parity_laufbahn_regeln.rs` haelt jede Zeile dagegen. Wer eine
//! Regel im Spiel aendert, bekommt `freshness.test.ts` rot; wer sie hier
//! aendert, `parity_laufbahn_regeln.rs`.
//!
//! # Die Zahlen kommen aus `data/`, wo das Spiel sie auch liest
//!
//! Katalog, Fallrate, Ertrag, Erfahrung, Instanzen und Boss-Stufen erzeugt
//! `build.rs` in `laufbahn_generated.rs`. Was in TypeScript als Konstante im
//! Code steht (Gewichtstabelle, Material je Qualitaet, Kosten je Stufe), steht
//! hier als Konstante mit Verweis — die Aufzeichnung deckt jede Zeile davon.

use crate::gear::{
    distribute_budget, WornValues, MAX_ITEM_LEVEL, QUALITY_COMMON, QUALITY_COUNT, QUALITY_EPIC,
    QUALITY_HAS_EFFECT, QUALITY_LEGENDARY, QUALITY_RARE,
};
use crate::rng::RandomStream;
use crate::sources::{base_permille, gear_effects_permille};

// ------------------------------------------------------------------ Daten

/// Feld von [`WornValues`]: CATALYSIS.
pub const FELD_CATALYSIS: usize = 0;
/// Feld von [`WornValues`]: PURITY.
pub const FELD_PURITY: usize = 1;
/// Feld von [`WornValues`]: YIELD (`yield_bonus`).
pub const FELD_YIELD: usize = 2;
/// Feld von [`WornValues`]: ESSENCE.
pub const FELD_ESSENCE: usize = 3;
/// Feld von [`WornValues`]: VESSEL. **Nicht oeffentlich**: Kein Affix fuellt
/// es (`AFFIX_FELDER` in `build.rs` kennt die Art nicht), nur die Grundwerte
/// in [`worn_from`] — und die schreiben das Feld direkt.
const FELD_VESSEL: usize = 4;
/// Feld von [`WornValues`]: WARD — nicht oeffentlich, aus demselben Grund.
const FELD_WARD: usize = 5;
/// Feld von [`WornValues`]: Topf 4 (`gear_effects`).
pub const FELD_GEAR_EFFECTS: usize = 6;

/// Ein Eintrag aus `data/items.json` — die **Vorlage** eines Fundes.
///
/// Stufe und Qualitaet hier sind die der Vorlage. Ein Fund traegt seine
/// eigenen (`rollLoot`: „Aus `itemLevelOf`, nicht aus dem Katalog").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vorlage {
    /// Kennung, wie im Spielstand.
    pub id: &'static str,
    /// Auf welchem der zehn Plaetze das Stueck getragen wird.
    pub slot: usize,
    /// Stufe der Vorlage — Rueckfall, wenn ein Stueck keine eigene traegt.
    pub item_level: u32,
    /// Qualitaet der Vorlage — Rueckfall wie oben.
    pub quality: usize,
    /// Welche Felder von [`WornValues`] die Affix-Plaetze fuellen, in ihrer
    /// Reihenfolge (`FELD_*`).
    pub affixe: &'static [usize],
    /// An welchen Boss das Stueck gebunden ist, oder `""` fuer ueberall.
    pub gebunden_an: &'static str,
    /// Satz-Kennung, `-1` fuer kein Satz-Teil.
    pub satz: i32,
}

/// Eine Instanz aus `data/instances.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instanz {
    /// Kennung, wie sie am Boss steht (`instance`).
    pub id: &'static str,
    /// Die Nummer, von der Beute-Stufe und Qualitaets-Obergrenze abhaengen.
    pub number: u32,
    /// Reihenfolge in der Auswahl.
    pub order: u32,
    /// Welche Instanz abgeschlossen sein muss, oder `None`.
    pub requires: Option<&'static str>,
}

include!(concat!(env!("OUT_DIR"), "/laufbahn_generated.rs"));

/// Die Stufe eines Bosses aus `data/bosses/*.json`, Rueckfall 1.
#[must_use]
pub fn boss_stufe(boss_id: &str) -> u32 {
    crate::data::BOSSES
        .iter()
        .position(|b| b.id == boss_id)
        .and_then(|i| BOSS_STUFEN.get(i).copied())
        .unwrap_or(1)
}

/// Die Instanz einer Kennung, oder `None`.
#[must_use]
pub fn instanz(id: &str) -> Option<&'static Instanz> {
    INSTANZEN.iter().find(|i| i.id == id)
}

/// `Math.round` aus JavaScript: halbe Werte nach OBEN, und `-0` fuer
/// `[-0.5, 0)`.
///
/// `f64::round` rundet halbe Werte von null WEG — bei `-2.5` ergibt das `-3`
/// statt `-2`, und bei `-0.3` `-0` wie hier, bei `-0.5` aber `-1` statt `-0`.
/// Der Vergleich in `compareItems` rundet genau dort (`~-0%`), und ein
/// Bitmuster `-0` gegen `-1` waere ein anderes Urteil.
#[must_use]
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    if (-0.5..0.0).contains(&x) {
        return -0.0;
    }
    let unten = x.floor();
    if x - unten >= 0.5 {
        unten + 1.0
    } else {
        unten
    }
}

// ------------------------------------------------------- Beute (rates.ts)

/// Wie viele Instanzen es gibt (8 Aufstieg + 6 Endgame).
pub const INSTANCE_COUNT: i32 = 14;
/// Die erste Endgame-Instanz.
pub const FIRST_ENDGAME_INSTANCE: i32 = 9;
/// Die drei Schwierigkeitsstufen.
pub const DIFFICULTY_COUNT: i32 = 3;
/// Ab welcher Instanz Epic fallen kann. **Vorlaeufig (#118)**, wie im Spiel.
pub const EPIC_FROM_INSTANCE: i32 = 4;
/// Die Summe aller Gewichte.
pub const WEIGHT_TOTAL: u32 = 10_000;
/// Zeilen der Gewichtstabelle.
pub const PROGRESS_STEPS: usize = 6;
/// Die Gewichte je Fortschrittsstufe — `PROGRESS_WEIGHTS` in `rates.ts`.
pub const PROGRESS_WEIGHTS: [[u32; QUALITY_COUNT]; PROGRESS_STEPS] = [
    [8500, 1500, 0, 0],
    [8000, 2000, 0, 0],
    [7500, 2200, 300, 0],
    [7000, 2500, 500, 0],
    [6500, 2800, 700, 0],
    [6000, 3000, 900, 100],
];

/// Die Instanz geklemmt auf `[1, INSTANCE_COUNT]`, wie `rates.ts` es tut.
const fn instanz_geklemmt(instance: i32) -> i32 {
    if instance < 1 {
        1
    } else if instance > INSTANCE_COUNT {
        INSTANCE_COUNT
    } else {
        instance
    }
}

const fn schwierigkeit_geklemmt(difficulty: i32) -> i32 {
    if difficulty < 0 {
        0
    } else if difficulty > DIFFICULTY_COUNT - 1 {
        DIFFICULTY_COUNT - 1
    } else {
        difficulty
    }
}

/// Die hoechste Qualitaet, die in dieser Instanz fallen kann — die
/// **Design-Regel** aus 13.4, nicht die Balance.
#[must_use]
pub const fn quality_cap_of(instance: i32) -> usize {
    if instance < 1 || instance > INSTANCE_COUNT {
        return QUALITY_COMMON;
    }
    if instance >= FIRST_ENDGAME_INSTANCE {
        return QUALITY_LEGENDARY;
    }
    if instance >= EPIC_FROM_INSTANCE {
        return QUALITY_EPIC;
    }
    QUALITY_RARE
}

/// Die Fortschrittsstufe eines Kampfes (`progressOf`).
#[must_use]
pub const fn progress_of(instance: i32, difficulty: i32, endless_level: i32) -> usize {
    let i = instanz_geklemmt(instance);
    let d = schwierigkeit_geklemmt(difficulty);
    let e = if endless_level > 0 { endless_level } else { 0 };
    let roh = i - 1 + 4 * d + e;
    let stufe = (roh / 4) as usize;
    if stufe > PROGRESS_STEPS - 1 {
        PROGRESS_STEPS - 1
    } else {
        stufe
    }
}

/// Die Gewichte fuer einen Kampf (`weightsFor`). Das Gewicht einer
/// gesperrten Qualitaet faellt der hoechsten erlaubten zu.
#[must_use]
pub fn weights_for(instance: i32, difficulty: i32, endless_level: i32) -> [u32; QUALITY_COUNT] {
    let zeile = PROGRESS_WEIGHTS[progress_of(instance, difficulty, endless_level)];
    let deckel = quality_cap_of(instance);
    let mut raus = [0u32; QUALITY_COUNT];
    let mut gesperrt = 0;
    for (quality, gewicht) in zeile.iter().enumerate() {
        if quality > deckel {
            gesperrt += gewicht;
        } else {
            raus[quality] = *gewicht;
        }
    }
    raus[deckel] += gesperrt;
    raus
}

/// Welche Qualitaet ein Wurf aus `[0, 1)` ergibt (`qualityOfRoll`).
#[must_use]
pub fn quality_of_roll(weights: &[u32; QUALITY_COUNT], roll: f64) -> usize {
    let gezogen = (roll * f64::from(WEIGHT_TOTAL)).floor();
    let mut grenze = 0u32;
    for (quality, gewicht) in weights.iter().enumerate() {
        grenze += gewicht;
        if gezogen < f64::from(grenze) {
            return quality;
        }
    }
    QUALITY_COMMON
}

/// Die Gegenstandsstufe eines Bosses (`itemLevelOf`).
#[must_use]
pub const fn item_level_of(instance: i32, difficulty: i32, endless_level: i32) -> u32 {
    let i = instanz_geklemmt(instance);
    let d = schwierigkeit_geklemmt(difficulty);
    let e = if endless_level > 0 { endless_level } else { 0 };
    ((i - 1) * 6 + d * 3 + e + 1) as u32
}

/// Darf dieses Stueck bei diesem Boss fallen? (`mayDropAt`)
#[must_use]
pub fn may_drop_at(bound_to_boss: &str, boss_id: &str) -> bool {
    bound_to_boss.is_empty() || bound_to_boss == boss_id
}

/// Was ein Sieg ueber den Fund weiss (`DropContext`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fundort<'a> {
    /// Instanz 1 bis 14.
    pub instance: i32,
    /// 0 normal, 1 heroisch, 2 mythisch.
    pub difficulty: i32,
    /// Endlos-Stufe, 0 ausserhalb.
    pub endless_level: i32,
    /// Welcher Boss gefallen ist.
    pub boss_id: &'a str,
}

/// Ein Fund: welche Vorlage, auf welcher Stufe, in welcher Qualitaet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fund {
    /// Index in den uebergebenen Katalog.
    pub vorlage: usize,
    /// Aus [`item_level_of`], nicht aus der Vorlage.
    pub item_level: u32,
    /// Gewuerfelte Qualitaet.
    pub quality: usize,
}

/// Der Startwert des Stroms `index` zu einem Kampf-Startwert —
/// `seedForStream` in `src/core/rng.ts`.
///
/// Dieselbe Rechnung wie in [`crate::rng::Streams::new`]; dort steht sie im
/// Konstruktor und laesst sich nicht aufrufen. Dass beide gleich rechnen,
/// haelt der Prueffall `strom_startwert_rechnet_wie_die_kampfstroeme` fest.
#[must_use]
pub const fn strom_startwert(seed: u32, index: u32) -> u32 {
    seed.wrapping_mul(0x9e37_79b1)
        .wrapping_add(index.wrapping_mul(0x85eb_ca6b))
}

/// Der Index, den der Beute-Strom in den Kampfstroemen haette
/// (`LOOT_STREAM_INDEX`) — er steht dort bewusst NICHT (`rng.rs`).
pub const LOOT_STREAM_INDEX: u32 = 3;

/// Der Beute-Strom eines Kampfes (`lootStreamFor`).
#[must_use]
pub const fn loot_stream_for(fight_seed: u32) -> RandomStream {
    RandomStream::new(strom_startwert(fight_seed, LOOT_STREAM_INDEX))
}

/// Welche Vorlagen bei diesem Boss fallen duerfen, als Indizes
/// (`candidatesFor`).
#[must_use]
pub fn candidates_for(boss_id: &str, katalog: &[Vorlage]) -> Vec<usize> {
    katalog
        .iter()
        .enumerate()
        .filter(|(_, v)| may_drop_at(v.gebunden_an, boss_id))
        .map(|(i, _)| i)
        .collect()
}

/// Was dieser Sieg fallen laesst, oder `None` (`rollLoot`).
///
/// **Die Reihenfolge der Zuege ist Teil der Zusicherung** und dieselbe wie im
/// Spiel: faellt etwas, welche Qualitaet, welcher Eintrag. Drei Zuege, auch
/// wenn nichts faellt.
pub fn roll_loot(
    stream: &mut RandomStream,
    ort: &Fundort<'_>,
    katalog: &[Vorlage],
    rate_per_mille: u32,
) -> Option<Fund> {
    let faellt = (stream.next_f64() * 1000.0).floor() < f64::from(rate_per_mille);
    let gewichte = weights_for(ort.instance, ort.difficulty, ort.endless_level);
    let quality = quality_of_roll(&gewichte, stream.next_f64());
    let auswahl = candidates_for(ort.boss_id, katalog);
    let gewaehlt = if auswahl.is_empty() {
        None
    } else {
        Some((stream.next_f64() * auswahl.len() as f64).floor() as usize)
    };
    if !faellt {
        return None;
    }
    let index = gewaehlt?.min(auswahl.len() - 1);
    Some(Fund {
        vorlage: auswahl[index],
        item_level: item_level_of(ort.instance, ort.difficulty, ort.endless_level),
        quality,
    })
}

// ---------------------------------------------- Material und Aufwertung

/// Material je nicht behaltenem Stueck, nach Qualitaet. **Vorlaeufig (#118).**
pub const MATERIAL_PER_QUALITY: [u64; QUALITY_COUNT] = [1, 3, 9, 27];
/// Was eine Stufe Aufwertung kostet. **Vorlaeufig (#118).**
pub const MATERIAL_PER_LEVEL: u64 = 4;

/// Das Material eines nicht behaltenen Stuecks (`materialFor`).
#[must_use]
pub fn material_for(quality: i64) -> u64 {
    material_for_mit(&MATERIAL_PER_QUALITY, quality)
}

/// [`material_for`] mit einer anderen Tabelle — der Einhaengepunkt fuer eine
/// geaenderte Aufwert-Regel (AC5). Mit [`MATERIAL_PER_QUALITY`] dieselbe
/// Funktion; nur so deckt die Aufzeichnung auch den Weg der Schleife.
#[must_use]
pub fn material_for_mit(tabelle: &[u64; QUALITY_COUNT], quality: i64) -> u64 {
    usize::try_from(quality)
        .ok()
        .and_then(|q| tabelle.get(q).copied())
        .unwrap_or(0)
}

/// Was eine Aufwertung ergeben hat (`UpgradeResult`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aufwertung {
    /// Die erreichte Gegenstandsstufe.
    pub level: i64,
    /// Wie viel Material tatsaechlich verbraucht wurde.
    pub spent: u64,
    /// Ob die Obergrenze des Inhalts die Aufwertung angehalten hat.
    pub capped_by_content: bool,
}

/// Wertet ein Stueck auf, so weit Material und **Inhalt** es zulassen
/// (`upgradeItem`).
///
/// **Die Obergrenze ist die Regel aus 13.6**: hoechstens bis zur Stufe, die im
/// aktuell gespielten Inhalt ohnehin faellt. Sie haelt auch bei beliebig viel
/// Material — an ihr haengt der Prueffall
/// `aufwertung_bleibt_unter_der_inhaltsstufe` in `sim/tests/laufbahn.rs`.
#[must_use]
pub fn upgrade_item(
    item_level: i64,
    material: i64,
    instance: i32,
    difficulty: i32,
    endless_level: i32,
) -> Aufwertung {
    upgrade_item_mit(
        MATERIAL_PER_LEVEL,
        item_level,
        material,
        instance,
        difficulty,
        endless_level,
    )
}

/// [`upgrade_item`] mit anderen Kosten je Stufe — Einhaengepunkt wie
/// [`material_for_mit`]. Die Obergrenze ist **nicht** einhaengbar: Sie ist
/// die Regel aus 13.6, keine Balance-Zahl.
///
/// # Panics
///
/// Bei Kosten 0 — eine Stufe umsonst hiesse unbegrenzt viele.
#[must_use]
pub fn upgrade_item_mit(
    kosten_je_stufe: u64,
    item_level: i64,
    material: i64,
    instance: i32,
    difficulty: i32,
    endless_level: i32,
) -> Aufwertung {
    assert!(
        kosten_je_stufe > 0,
        "eine Aufwertungsstufe kostet mindestens 1"
    );
    let grenze = i64::from(item_level_of(instance, difficulty, endless_level));
    let start = item_level.max(0);
    let vorrat = material.max(0).unsigned_abs();
    if start >= grenze {
        return Aufwertung {
            level: start,
            spent: 0,
            capped_by_content: true,
        };
    }
    let bezahlbar = vorrat / kosten_je_stufe;
    let moeglich = (grenze - start).unsigned_abs();
    let stufen = bezahlbar.min(moeglich);
    Aufwertung {
        level: start + stufen as i64,
        spent: stufen * kosten_je_stufe,
        capped_by_content: stufen == moeglich,
    }
}

/// Die Obergrenze im aktuellen Inhalt (`upgradeCapOf`).
#[must_use]
pub const fn upgrade_cap_of(instance: i32, difficulty: i32, endless_level: i32) -> u32 {
    item_level_of(instance, difficulty, endless_level)
}

/// Die Stufe, mit der gerechnet wird: Fundstufe plus Aufwertung, geklemmt
/// (`effectiveLevelOf`). `None` heisst: Feld fehlt im Spielstand.
#[must_use]
pub fn effective_level_of(
    item_level: Option<i64>,
    upgrade_steps: Option<i64>,
    vorlage: i64,
) -> u32 {
    let basis = item_level.unwrap_or(vorlage);
    if basis <= 0 {
        return 0;
    }
    let schritte = upgrade_steps.filter(|s| *s > 0).unwrap_or(0);
    let stufe = (basis + schritte).min(i64::from(MAX_ITEM_LEVEL));
    u32::try_from(stufe).unwrap_or(MAX_ITEM_LEVEL)
}

// ------------------------------------------------------------ Punkte

/// Was ein Lauf beisteuert (`PointsSource`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Punktequelle {
    /// Gewonnen?
    pub victory: bool,
    /// Restlaufzeit beim Sieg in ms.
    pub remaining_ms: f64,
    /// Ring-Rest, nur bei einem `runaway`-Boss.
    pub held_percent: Option<f64>,
}

/// Die Rechnung in ihren Teilen (`PointsBreakdown`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Punkte {
    /// Die Groesse, aus der gerechnet wurde, in Sekunden-Einheiten.
    pub units: f64,
    /// Was die Kurve daraus macht, vor dem Stufenabstand.
    pub base: i64,
    /// Was die Kurve aus dem bisherigen Bestwert macht, oder 0.
    pub previous_base: i64,
    /// Ob nur der Rest-Ertrag fuers Wiederholen gezahlt wurde.
    pub repeat_only: bool,
    /// Der Abstandsfaktor in Promille.
    pub falloff_per_mille: u32,
    /// Was ausgezahlt wird.
    pub awarded: i64,
}

/// Der Stufenabstand, der den Ertrag mindert — nur nach oben (`levelGap`).
#[must_use]
pub fn level_gap(boss_level: i64, player_level: i64) -> i64 {
    (player_level.max(1) - boss_level.max(1)).max(0)
}

/// Der Abstandsfaktor in Promille (`rewardFalloffPerMille`).
#[must_use]
pub fn reward_falloff_per_mille(boss_level: i64, player_level: i64) -> u32 {
    usize::try_from(level_gap(boss_level, player_level))
        .ok()
        .and_then(|a| FALLOFF_PER_MILLE.get(a).copied())
        .unwrap_or(0)
}

/// Einen Ertrag um den Abstand mindern (`applyRewardFalloff`).
#[must_use]
pub fn apply_reward_falloff(amount: i64, boss_level: i64, player_level: i64) -> i64 {
    let promille = f64::from(reward_falloff_per_mille(boss_level, player_level));
    ((amount as f64 * promille) / 1000.0).trunc() as i64
}

/// `Math.pow` aus JavaScript fuer die Punktekurve.
///
/// **Fuer den Exponenten 2 die Multiplikation**, und das ist kein Abkuerzen:
/// `Math.pow(x, 2)` im Spiel IST `x * x` — gemessen am 2026-10-29 in Node
/// v24.14.1 (V8, dieselbe Maschine wie im Browser): 5 340 001 Werte, darunter
/// jede Restzeit von 0 bis 60 s im Viertel-Millisekunden-Raster, null
/// Abweichungen. `f64::powf` ruft die C-Bibliothek, die das nicht zusagt.
/// Fuer jeden anderen Exponenten gilt, was `emit_levels` in `build.rs`
/// schreibt: Potenzen sind zwischen zwei Sprachen nicht bitgleich garantiert —
/// die Aufzeichnung wuerde es zeigen.
fn potenz(x: f64, exponent: f64) -> f64 {
    if exponent == 2.0 {
        x * x
    } else {
        x.powf(exponent)
    }
}

fn einheiten(ist_ring: bool, wert: f64) -> f64 {
    if ist_ring {
        wert.max(0.0) * HELD_POINTS_PER_PERCENT
    } else {
        wert.max(0.0) / 1000.0
    }
}

fn kurve(units: f64) -> i64 {
    js_round(POINTS_PER_SECOND * potenz(units, POINTS_EXPONENT)) as i64
}

/// Was ein Lauf einbringt, in seine Teile zerlegt (`pointsBreakdown`).
///
/// `previous_best` ist der bisherige Bestwert als ROHWERT (ms bzw. Prozent).
#[must_use]
pub fn points_breakdown(
    source: &Punktequelle,
    boss_level: i64,
    player_level: i64,
    previous_best: Option<f64>,
) -> Punkte {
    let ist_ring = source.held_percent.is_some();
    let units = if source.victory {
        einheiten(ist_ring, source.held_percent.unwrap_or(source.remaining_ms))
    } else {
        0.0
    };
    let base = kurve(units);
    let previous_base = match previous_best {
        Some(wert) if source.victory => kurve(einheiten(ist_ring, wert)),
        _ => 0,
    };
    let differenz = (base - previous_base).max(0);
    let repeat_only = source.victory && differenz == 0;
    let roh = if repeat_only {
        ((base as f64 * REPEAT_PER_MILLE) / 1000.0).trunc() as i64
    } else {
        differenz
    };
    Punkte {
        units,
        base,
        previous_base,
        repeat_only,
        falloff_per_mille: reward_falloff_per_mille(boss_level, player_level),
        awarded: apply_reward_falloff(roh, boss_level, player_level),
    }
}

// --------------------------------------------------------- Erfahrung

/// Die Stufe, mit der jeder Spielstand beginnt.
pub const START_LEVEL: u32 = 1;

/// Stand des Aufstiegs (`LevelState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stufenstand {
    /// Erreichte Stufe.
    pub level: u32,
    /// Erfahrung auf dieser Stufe.
    pub xp: i64,
}

/// Erfahrung fuer einen Sieg (`xpForVictory`).
#[must_use]
pub fn xp_for_victory(boss_level: i64, player_level: i64) -> i64 {
    let stufe = boss_level.max(1);
    apply_reward_falloff(stufe * XP_PER_BOSS_LEVEL as i64, boss_level, player_level)
}

/// Was bis zur naechsten Stufe fehlt, oder `None` fuer „unendlich"
/// (`xpToNext`).
#[must_use]
pub fn xp_to_next(level: i64) -> Option<i64> {
    if level >= i64::from(crate::data::MAX_LEVEL) {
        return None;
    }
    let index = usize::try_from(level.max(1) - 1).ok()?;
    XP_TO_NEXT_LEVEL.get(index).map(|b| *b as i64)
}

/// Erfahrung gutschreiben, mit Aufstieg (`grantXp`).
#[must_use]
pub fn grant_xp(state: Stufenstand, amount: i64) -> Stufenstand {
    if amount <= 0 {
        return state;
    }
    let max = crate::data::MAX_LEVEL;
    let mut level = state.level.max(START_LEVEL);
    let mut xp = state.xp + amount;
    while level < max {
        let Some(bedarf) = xp_to_next(i64::from(level)) else {
            break;
        };
        if xp < bedarf {
            break;
        }
        xp -= bedarf;
        level += 1;
    }
    if level >= max {
        return Stufenstand { level: max, xp: 0 };
    }
    Stufenstand { level, xp }
}

// ------------------------------------------------------- Freischaltung

/// Welche Instanzen offen sind (`freigeschalteteInstanzen`), als Indizes in
/// [`INSTANZEN`].
///
/// `besiegt(i)` sagt, ob der Boss `crate::data::BOSSES[i]` gefallen ist — auf
/// welcher Stufe, ist gleich (das Spiel loest die Kennung aus dem Schluessel).
/// **Vollstaendig heisst jeder Boss**, und eine Instanz ohne Bosse gilt nicht
/// als abgeschlossen.
#[must_use]
pub fn freigeschaltete_instanzen(besiegt: impl Fn(usize) -> bool) -> Vec<usize> {
    let abgeschlossen = |id: &str| {
        let mut drin = crate::data::BOSSES
            .iter()
            .enumerate()
            .filter(|(_, b)| b.instance == id)
            .peekable();
        drin.peek().is_some() && drin.all(|(i, _)| besiegt(i))
    };
    INSTANZEN
        .iter()
        .enumerate()
        .filter(|(_, i)| i.requires.is_none_or(&abgeschlossen))
        .map(|(index, _)| index)
        .collect()
}

// ----------------------------------------------------- Getragene Werte

/// ESSENCE eines Gegenstands auf Stufe 1 — `ESSENCE_AT_LEVEL_1` in
/// `src/gear/sources.ts`. **Unkalibriert**, wie dort.
pub const ESSENCE_AT_LEVEL_1: u32 = 10;
/// VESSEL auf Stufe 1 — `VESSEL_AT_LEVEL_1`.
pub const VESSEL_AT_LEVEL_1: u32 = 10;
/// WARD auf Stufe 1 — `WARD_AT_LEVEL_1`.
pub const WARD_AT_LEVEL_1: u32 = 10;

/// Ein besessenes Stueck (`OwnedItemLike`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stueck {
    /// Index in [`KATALOG`].
    pub vorlage: usize,
    /// Gewuerfelte Stufe.
    pub item_level: u32,
    /// Gewuerfelte Qualitaet.
    pub quality: usize,
    /// Aufwertungsstufen auf diesem Stueck.
    pub upgrade_steps: u32,
    /// Auf welchem Platz es liegt, `None` im Beutel.
    pub platz: Option<usize>,
}

impl Stueck {
    /// Die Stufe, mit der gerechnet wird.
    #[must_use]
    pub fn effektive_stufe(&self) -> u32 {
        effective_level_of(
            Some(i64::from(self.item_level)),
            Some(i64::from(self.upgrade_steps)),
            KATALOG
                .get(self.vorlage)
                .map_or(0, |v| i64::from(v.item_level)),
        )
    }
}

fn feld_mut(werte: &mut WornValues, feld: usize) -> &mut u32 {
    match feld {
        FELD_CATALYSIS => &mut werte.catalysis,
        FELD_PURITY => &mut werte.purity,
        FELD_YIELD => &mut werte.yield_bonus,
        FELD_ESSENCE => &mut werte.essence,
        FELD_VESSEL => &mut werte.vessel,
        FELD_WARD => &mut werte.ward,
        _ => &mut werte.gear_effects,
    }
}

/// Die getragenen Werte aus den ausgeruesteten Stuecken (`wornFrom`).
///
/// # Panics
///
/// Bei einem Satz-Teil. Die Satz-Tabelle steht hier nicht; `build.rs` bricht
/// den Bau, sobald ein Spielboss eines fallen lassen kann, und ein Aufrufer,
/// der eines von Hand hineinreicht, soll das laut erfahren statt mit einem
/// Satz-Bonus von null zu rechnen.
#[must_use]
pub fn worn_from(stuecke: &[Stueck]) -> WornValues {
    let mut summe = WornValues::none();
    let mut getragen = 0u32;
    let mut legendaere = 0u32;
    for stueck in stuecke {
        if stueck.platz.is_none() {
            continue;
        }
        let Some(def) = KATALOG.get(stueck.vorlage) else {
            continue;
        };
        assert!(
            def.satz == -1,
            "worn_from: \"{}\" ist ein Satz-Teil; die Fortschritts-Simulation kennt die \
             Satz-Tabelle nicht",
            def.id
        );
        getragen += 1;
        let stufe = stueck.effektive_stufe();
        let guete = stueck.quality;
        summe.essence += base_permille(stufe, ESSENCE_AT_LEVEL_1);
        summe.vessel += base_permille(stufe, VESSEL_AT_LEVEL_1);
        summe.ward += base_permille(stufe, WARD_AT_LEVEL_1);
        let mut anteile = [0u64; 4];
        let anzahl = distribute_budget(stufe, guete, &mut anteile);
        for (anteil, feld) in anteile.iter().take(anzahl).zip(def.affixe) {
            *feld_mut(&mut summe, *feld) += u32::try_from(*anteil).unwrap_or(u32::MAX);
        }
        if QUALITY_HAS_EFFECT.get(guete) == Some(&true) {
            legendaere += 1;
        }
    }
    // Topf 4: legendaere Effekte und Satz-Boni, additiv. Satz-Teile gibt es
    // hier nicht (siehe oben), der Satz-Anteil ist also die gerundete Null aus
    // `Math.round(valueShareOf(SETS, …) * 1000)`.
    summe.gear_effects += gear_effects_permille(legendaere, 0);
    if getragen == 0 {
        WornValues::none()
    } else {
        summe
    }
}

/// Der geschaetzte Schaden je Wirkung (`estimateDamagePerHit`).
///
/// Dieselbe Reihenfolge der Faktoren wie im Spiel — sie ist Teil der
/// Bitgleichheit.
#[must_use]
pub fn estimate_damage_per_hit(worn: &WornValues) -> f64 {
    let p = worn.crit_chance();
    let werte = worn.essence_factor() * worn.gear_effect_factor();
    werte * ((1.0 - p) * 1.0 + p * worn.crit_factor())
}

/// Ein Fund ist besser als das Getragene.
pub const COMPARE_BETTER: i32 = 1;
/// Gleich.
pub const COMPARE_EQUAL: i32 = 0;
/// Schlechter.
pub const COMPARE_WORSE: i32 = -1;

/// Das Ergebnis des automatischen Vergleichs (`Comparison`, ohne die Texte).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vergleich {
    /// [`COMPARE_BETTER`], [`COMPARE_EQUAL`] oder [`COMPARE_WORSE`].
    pub verdict: i32,
    /// Der geschaetzte Unterschied in Prozent, auf eine Nachkommastelle.
    pub damage_percent: f64,
}

fn compare_items(mit_fund: f64, mit_getragenem: f64) -> Vergleich {
    if !(mit_fund.is_finite() && mit_getragenem.is_finite() && mit_getragenem > 0.0) {
        return Vergleich {
            verdict: COMPARE_EQUAL,
            damage_percent: 0.0,
        };
    }
    let anteil = (mit_fund - mit_getragenem) / mit_getragenem;
    let prozent = js_round(anteil * 1000.0) / 10.0;
    let verdict = if prozent > 0.0 {
        COMPARE_BETTER
    } else if prozent < 0.0 {
        COMPARE_WORSE
    } else {
        COMPARE_EQUAL
    };
    Vergleich {
        verdict,
        damage_percent: prozent,
    }
}

/// Wie ein Stueck im Beutel gegen das abschneidet, was auf seinem Platz
/// liegt (`compareBagItem`). **Ein echter Tausch**: Die ganze Ausruestung
/// wird mit und ohne ihn gerechnet.
#[must_use]
pub fn compare_bag_item(stuecke: &[Stueck], bag_index: usize) -> Option<Vergleich> {
    let fund = stuecke.get(bag_index)?;
    if fund.platz.is_some() {
        return None;
    }
    let def = KATALOG.get(fund.vorlage)?;
    let nach_tausch: Vec<Stueck> = stuecke
        .iter()
        .enumerate()
        .map(|(i, besitz)| {
            if i == bag_index {
                Stueck {
                    platz: Some(def.slot),
                    ..*besitz
                }
            } else if besitz.platz == Some(def.slot) {
                Stueck {
                    platz: None,
                    ..*besitz
                }
            } else {
                *besitz
            }
        })
        .collect();
    Some(compare_items(
        estimate_damage_per_hit(&worn_from(&nach_tausch)),
        estimate_damage_per_hit(&worn_from(stuecke)),
    ))
}

// ---------------------------------------------- Der erwartete Stand

/// Die Schwierigkeit des erwarteten Stands (20.5: normal).
pub const ERWARTETE_SCHWIERIGKEIT: i32 = 0;

/// Ein Stueck je Platz, alle `-mix` (`STUECK_JE_PLATZ` in
/// `src/devtools/expected-gear.ts`).
pub const STUECK_JE_PLATZ: [&str; 10] = [
    "focus-mix",
    "catalyst-mix",
    "sigil-mix",
    "relic-mix",
    "grimoire-mix",
    "shard-mix",
    "talisman-mix",
    "phylactery-mix",
    "anchor-mix",
    "prism-mix",
];

/// Wie viele der `plaetze` Stuecke ueber `common` liegen (`bessereStuecke`).
#[must_use]
pub fn bessere_stuecke(instanz: i32, plaetze: usize) -> usize {
    let gewichte = weights_for(instanz, ERWARTETE_SCHWIERIGKEIT, 0);
    let gesamt: u32 = gewichte.iter().sum();
    if gesamt == 0 {
        return 0;
    }
    let anteil = f64::from(gesamt - gewichte[QUALITY_COMMON]) / f64::from(gesamt);
    (plaetze as f64 * anteil).floor() as usize
}

/// Die Ausruestung, mit der ein Spieler die Instanz NACH `vorstufe` betritt
/// (`erwarteterStand`) — die **gesetzte** Annahme aus 20.5, gegen die die
/// Fortschritts-Simulation die gemessene haelt.
#[must_use]
pub fn erwarteter_stand(vorstufe: i32, plaetze: usize) -> Vec<Stueck> {
    let item_level = item_level_of(vorstufe, ERWARTETE_SCHWIERIGKEIT, 0);
    let bessere = bessere_stuecke(vorstufe, plaetze);
    let mut raus = Vec::with_capacity(plaetze.min(STUECK_JE_PLATZ.len()));
    for platz in 0..plaetze {
        let Some(id) = STUECK_JE_PLATZ.get(platz) else {
            continue;
        };
        let Some(vorlage) = KATALOG.iter().position(|v| v.id == *id) else {
            continue;
        };
        raus.push(Stueck {
            vorlage,
            item_level,
            quality: if platz < bessere {
                quality_cap_of(vorstufe)
            } else {
                QUALITY_COMMON
            },
            upgrade_steps: 0,
            platz: Some(platz),
        });
    }
    raus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{Stream, Streams};

    #[test]
    fn strom_startwert_rechnet_wie_die_kampfstroeme() {
        // Die drei Kampfstroeme sind Index 0 bis 2 derselben Ableitung; der
        // Beute-Strom ist Index 3. Stimmen 0 bis 2 nicht, stimmt 3 auch nicht.
        for seed in [0u32, 1, 2, 42, 123_456_789, u32::MAX] {
            let stroeme = Streams::new(seed);
            for (index, strom) in Stream::ALL.iter().enumerate() {
                assert_eq!(
                    strom_startwert(seed, index as u32),
                    stroeme.stream_seed(*strom),
                    "Startwert {seed}, Strom {index}"
                );
            }
        }
    }

    #[test]
    fn js_round_rundet_wie_javascript() {
        // Halbe Werte nach oben, nicht von null weg — und -0 im Bereich davor.
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(-0.5).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(-0.3).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(-0.51), -1.0);
        assert_eq!(js_round(0.49), 0.0);
        // Gegen `f64::round` an der Stelle, an der die beiden auseinanderlaufen
        // — sonst bewiese der Fall nichts ueber den Unterschied.
        assert_ne!(js_round(-2.5), (-2.5f64).round());
    }

    #[test]
    fn die_gewichte_summieren_immer_auf_die_summe() {
        for instanz in -1..=15 {
            for schwierigkeit in -1..=3 {
                for endlos in [-1, 0, 1, 7, 20] {
                    let summe: u32 = weights_for(instanz, schwierigkeit, endlos).iter().sum();
                    assert_eq!(summe, WEIGHT_TOTAL, "{instanz}/{schwierigkeit}/{endlos}");
                }
            }
        }
    }
}
