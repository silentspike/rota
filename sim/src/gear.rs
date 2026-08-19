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

/// Wie viele **Affixe** eine Qualitaet traegt (13.3).
///
/// **Affixe, nicht Werte ueberhaupt.** Die Grundwerte kommen zusaetzlich; siehe
/// [`BASE_STAT_IDS`]. Der Name weicht bewusst von der frueheren Bezeichnung
/// "Werte-Plaetze" ab — die hat die Verwechslung erst erzeugt.
///
/// **Legendary steht in 13.3 als "3+".** Das Pluszeichen ist in derselben Zeile
/// erklaert — "echter Sondereffekt". Gelesen als: drei Affixe UND zusaetzlich
/// ein Effekt, nicht als vierter Affix.
pub const QUALITY_AFFIX_SLOTS: [usize; QUALITY_COUNT] = [1, 2, 3, 3];

/// Welche Qualitaet einen Sondereffekt traegt.
pub const QUALITY_HAS_EFFECT: [bool; QUALITY_COUNT] = [false, false, false, true];

/// Das Budget einer Qualitaet auf Gegenstandsstufe 1. **Vorlaeufig (#118).**
///
/// # Die 190 steht zweimal, und das ist Absicht
///
/// **Epic und Legendary tragen dasselbe Budget** — auf jeder Stufe von 1 bis
/// 100, nicht nur zufaellig auf einer. GAME-DESIGN 13.3 gibt Legendary keine
/// zusaetzlichen Werte:
///
/// ```text
///   Epic        3 Affixe    —
///   Legendary   3+          ECHTER SONDEREFFEKT, der die Rotation aendert
/// ```
///
/// Das Pluszeichen meint den **Effekt**, nicht einen vierten Affix — siehe
/// [`QUALITY_AFFIX_SLOTS`], das `[1, 2, 3, 3]` fuehrt. Legendary unterscheidet
/// sich durch etwas, das der Gegenstand **kann**, nicht durch mehr Zahlen.
///
/// # Wer das hier "repariert", macht es kaputt
///
/// Am 2026-08-10 haben zwei Sitzungen die doppelte 190 unabhaengig
/// voneinander fuer einen Fehler gehalten. Der Kommentar sagte damals nur
/// "vorlaeufig", und die Begruendung stand bei den Affix-Plaetzen — wo sie
/// beim Aendern des Budgets niemand liest.
///
/// **Die Sorge dahinter ist berechtigt und trifft eine andere Stelle:**
/// [`QUALITY_HAS_EFFECT`] hat **hier** keinen Leser, und das ist Absicht.
///
/// Bis Issue #480 stand hier "auf keiner der beiden Seiten" — das galt damals
/// und stimmt seither nicht mehr. Die Spielseite liest es
/// (`src/gear/catalog.ts:344`) und speist damit Topf 4 ueber
/// `gearEffectsPermille`. Der Sondereffekt IST gebaut.
///
/// **Die Arbeitsteilung, die hier gilt:** `gear_effects` wird
/// HEREINGEREICHT, nicht in [`distribute_budget`] abgeleitet. Der Waechter
/// dafuer steht in `best_build.rs`:
///
/// ```text
///   assert_eq!(worn.gear_effects, 0, "Topf 4 gehoert der anderen Seite");
/// ```
///
/// Wer den Effekt hier ergaenzt, baut einen ZWEITEN Weg zu derselben Zahl.
/// Zwei Wege laufen auseinander, sobald einer nachgezogen wird — und beide
/// liefern weiter plausible Werte, weshalb es niemandem auffaellt. Wer den
/// Wert braucht, rechnet ihn beim Aufrufer mit
/// [`crate::sources::gear_effects_permille`].
///
/// **Dieser Absatz stand sechs Tage lang falsch da** und hat eine Messung in
/// die Irre gefuehrt (#791): Er begruendete eine Luecke, die es nicht mehr gab,
/// und beendete damit die Suche. Wer hier etwas liest, das eine Luecke
/// begruendet, prueft den Stand — nicht den Text.
///
/// Wer q3 hier anhebt, baut einen **zweiten** Unterschied ein, den 13.3 nicht
/// vorsieht, und verdeckt den echten. Das TS-Gegenstueck traegt denselben
/// Kommentar, und `tests/unit/gear-budget.test.ts` haelt die Gleichheit fest.
const QUALITY_BASE_BUDGET: [u64; QUALITY_COUNT] = [100, 140, 190, 190];

/// Zaehler und Nenner des Stufen-Wachstums — 21/20 sind +5 %. **Vorlaeufig.**
const GROWTH_NUMERATOR: u64 = 21;
const GROWTH_DENOMINATOR: u64 = 20;

/// Hoechste Gegenstandsstufe, die gerechnet wird.
pub const MAX_ITEM_LEVEL: u32 = 100;

/// Die eigentliche Rechnung — **eine** Formel, zwei Verwendungen.
///
/// Sie ist `const`, damit [`BUDGET_TABELLE`] sie zur Uebersetzungszeit
/// auswerten kann. Damit gibt es keine zweite Fassung in `build.rs`, die neben
/// dieser altern koennte: Tabelle und Rechnung stammen aus derselben Zeile.
const fn budget_berechnet(item_level: u32, quality: usize) -> u64 {
    if quality >= QUALITY_COUNT || item_level < 1 || item_level > MAX_ITEM_LEVEL {
        return 0;
    }
    let mut budget = QUALITY_BASE_BUDGET[quality];
    let mut stufe = 1;
    while stufe < item_level {
        budget = budget * GROWTH_NUMERATOR / GROWTH_DENOMINATOR;
        stufe += 1;
    }
    budget
}

/// Alle Auspraegungen in einem dichten Feld (Issue #908, AC6).
///
/// `(MAX_ITEM_LEVEL + 1) * QUALITY_COUNT` Eintraege zu je acht Byte. Der Platz
/// auf Index 0 bleibt leer und wird nie gelesen — er kostet vier Werte und
/// spart an jeder Lesestelle eine Verschiebung um eins.
static BUDGET_TABELLE: [[u64; QUALITY_COUNT]; MAX_ITEM_LEVEL as usize + 1] = {
    let mut tabelle = [[0u64; QUALITY_COUNT]; MAX_ITEM_LEVEL as usize + 1];
    let mut stufe = 0usize;
    while stufe <= MAX_ITEM_LEVEL as usize {
        let mut guete = 0usize;
        while guete < QUALITY_COUNT {
            tabelle[stufe][guete] = budget_berechnet(stufe as u32, guete);
            guete += 1;
        }
        stufe += 1;
    }
    tabelle
};

/// Das Budget **neu ausrechnen**, statt es nachzuschlagen.
///
/// Existiert für den Nachweis zu AC6: Ohne eine Stelle, die den Zähler
/// nachweislich bewegt, wäre die Null in [`crate::diagnostics::Counters::gear_budgets`]
/// nur die Abwesenheit einer Messung. Zugleich ist sie die **Gegenprobe** zur
/// Tabelle — zwei Wege zum selben Wert, einer davon der, den es vorher gab.
#[must_use]
pub fn budget_neu_berechnen(item_level: u32, quality: usize) -> u64 {
    #[cfg(feature = "diagnostics")]
    crate::diagnostics::note_gear_budget();

    budget_berechnet(item_level, quality)
}

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
    // **Nachgeschlagen, nicht gerechnet** (Issue #908, AC6). Die Tabelle
    // entsteht zur Uebersetzungszeit aus derselben Funktion, die auch
    // [`budget_neu_berechnen`] ruft — es gibt keine zweite Formel, die
    // auseinanderlaufen koennte.
    BUDGET_TABELLE[item_level as usize][quality]
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
    let slots = QUALITY_AFFIX_SLOTS.get(quality).copied().unwrap_or(0);
    let budget = budget_of(item_level, quality);
    let count = slots.min(out.len());
    for (index, slot) in out.iter_mut().enumerate().take(count) {
        *slot = share_of_slot(budget, slots, index);
    }
    count
}

/// Die zehn Plaetze aus 13.2.
///
/// **Die Plaetze sind Behaelter, sonst nichts** — wie in WoW und Diablo. Welche
/// Werte ein Stueck traegt, haengt an Seltenheit und Gegenstandsstufe, nicht am
/// Platz.
///
/// # Was hier bis zum 2026-07-29 stand
///
/// Eine platz-spezifische Wertezuordnung: Focus und Catalyst immer CATALYSIS,
/// Sigil und Talisman immer PURITY/YIELD, und so fort. Sie stand im Plan und im
/// Design-Dokument, mit einer Begruendung, die ueberzeugend klang.
/// **Angeordnet worden war sie nie** — auf Rueckfrage des Auftraggebers
/// durchsucht, keine einzige Fundstelle in den Gespraechen.
///
/// Aufgefallen ist es, weil zwei der zehn Plaetze (`relic`, `shard`) nie eine
/// Zuordnung bekamen und zwei der vier Kategorien ("Verteidigung",
/// "Zauber-Verstaerkung") gar keine definierten Werte waren. Beides fiel erst
/// auf, als E6-1 die Tabelle bauen sollte — und statt die Luecken zu raten,
/// wurde gefragt.
///
/// **Merksatz aus 13.2:** Eine Festlegung, die niemand angeordnet hat, sieht im
/// Dokument genauso aus wie eine, die entschieden wurde.
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

/// Die Grundwerte, die **jeder** Gegenstand traegt (13.2, 6.1).
///
/// **Sie kommen zusaetzlich zu den Affixen**, nicht aus deren Budget — wie ein
/// Diablo-Gegenstand immer Ruestung traegt. Ein Common-Stueck hat also ESSENCE,
/// VESSEL und WARD **plus** einen Affix.
///
/// Am 2026-07-29 praezisiert. Anlass war eine Rechnung, die nicht aufging: 13.3
/// gab Common genau einen "Werte-Platz", und ein Platz kann keine drei
/// Grundwerte tragen. Die Spalte meint **Affixe**.
pub const BASE_STAT_IDS: [&str; 3] = ["essence", "vessel", "ward"];

/// Was der Spieler **traegt** — die drei Sonderwerte aus 6.2, ganzzahlig.
///
/// # Warum Promille und nicht die Toepfe aus [`ValuePools`]
///
/// [`ValuePools`] fuehrt Anteile als `f64`, weil Schadensfaktoren Verhaeltnisse
/// sind. CATALYSIS wirkt aber auf **Zeiten**, und 18.4 verbietet
/// Fliesskomma-Zeit. Ein Wert, der als `f64` ankommt und fuer die Zeitrechnung
/// nach `u32` gerundet werden muesste, waere genau die Stelle, an der Spiel und
/// Simulator um ein Bit auseinanderlaufen koennen.
///
/// 6.7 nimmt die Entscheidung vorweg: **„Ganzzahlig in Promille gerechnet."**
/// Die Werte kommen deshalb gar nicht erst als `f64` herein.
///
/// | Feld            | Bedeutung                                | 1000 heisst |
/// | --------------- | ---------------------------------------- | ----------- |
/// | `catalysis`     | Tempo — verkuerzt Zeiten                 | +100 %      |
/// | `purity`        | Kritchance                               | 100 %       |
/// | `yield_bonus`   | Kritschaden **ueber der Basis** von 150 % | +100 pp    |
///
/// # Ohne getragene Ausruestung sind alle drei null
///
/// Das ist die Zusicherung aus AC7 in der Bauform: [`WornValues::none`] ist der
/// Zustand, in dem sich nichts aendern darf — keine Zeit, kein Wurf, keine
/// Pruefsumme.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WornValues {
    /// CATALYSIS in Promille.
    pub catalysis: u32,
    /// PURITY in Promille.
    pub purity: u32,
    /// YIELD in Promille, **zusaetzlich** zur Basis aus [`CRIT_BASE_PERMILLE`].
    pub yield_bonus: u32,
    /// ESSENCE in Promille — **Topf 1**, nicht Topf 4 (Issue #282).
    ///
    /// # Warum ein eigenes Feld und nicht `gear_effects`
    ///
    /// Die Leitung nach Topf 4 liegt schon, und sie zu benutzen waere eine
    /// gesparte Zeile. Sie waere auch falsch: GAME-DESIGN 13.1b trennt die
    /// Toepfe ausdruecklich — innerhalb additiv, ZWISCHEN ihnen multiplikativ.
    /// Wer ESSENCE in Topf 4 einhaengt, macht aus zwei Toepfen einen und
    /// kippt die Abwaegung, die 13.5 verlangt.
    pub essence: u32,
    /// VESSEL in Promille — Lebenspunkte, **zusaetzlich** zur Stufen-Basis.
    ///
    /// Additiv, nicht ersetzend: Wer die Stufen-Basis ersetzt, macht einen
    /// Spieler auf Stufe 100 ohne Ausruestung schwaecher als einen auf Stufe 1
    /// mit einem Fund.
    pub vessel: u32,
    /// WARD in Promille — Schadensminderung, **zusaetzlich** zu
    /// `PLAYER.wardReduction`.
    pub ward: u32,
    /// **Topf 4** in Promille: Satz-Boni **und** legendaere Effekte zusammen
    /// (13.1b).
    ///
    /// # Warum dieses Feld erst nach #177 kam
    ///
    /// #177 hat die drei Sonderwerte aus 6.2 verdrahtet — CATALYSIS auf Zeiten,
    /// PURITY und YIELD auf den Krit. Topf 4 blieb dabei liegen, obwohl der
    /// Befund im Issue ihn ausdruecklich nannte ("kein einziger Topf wird
    /// ausserhalb von gear.rs gelesen, auch POOL_GEAR_EFFECTS nicht"): Keine
    /// einzige AC hat ihn aufgefuehrt.
    ///
    /// Aufgefallen ist es bei #118, wo "bester Satz-Aufbau gegen besten
    /// Affix-Aufbau" gemessen werden soll — und die Satz-Seite gar nicht im
    /// Kampf ankam.
    ///
    /// # Eine Verdrahtung, zwei Quellen
    ///
    /// 13.1b legt Satz-Boni und legendaere Effekte in **denselben** Topf, wo
    /// sie untereinander addiert werden. Nach aussen ist es ein Faktor. Das
    /// Quellen-Modul (`crate::sources`) fuehrt sie getrennt, weil sie
    /// verschieden entstehen — hier kommen sie als eine Zahl an.
    ///
    /// **Die Lehre aus Diablo, die 13.1b zitiert:** Was multiplikativ ist, wird
    /// gejagt. Deshalb ein Topf und nicht zwei.
    pub gear_effects: u32,
}

/// Der Kritschaden ohne jeden YIELD-Beitrag: **150 %** (GAME-DESIGN 6.7).
///
/// Steht hier und nicht in `factors.rs`, weil es eine Eigenschaft des Wertes
/// ist und nicht der Formel — `factors.crit` nimmt das Ergebnis entgegen.
pub const CRIT_BASE_PERMILLE: u32 = 1500;

impl WornValues {
    /// Nichts getragen — alle drei Werte null.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            catalysis: 0,
            purity: 0,
            yield_bonus: 0,
            essence: 0,
            vessel: 0,
            ward: 0,
            gear_effects: 0,
        }
    }

    /// Ob ueberhaupt etwas getragen wird.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.catalysis == 0
            && self.purity == 0
            && self.yield_bonus == 0
            && self.essence == 0
            && self.vessel == 0
            && self.ward == 0
            && self.gear_effects == 0
    }

    /// Der Faktor von Topf 4 fuer die Schadensformel.
    ///
    /// **Eins plus die Summe** — dieselbe Regel wie in [`ValuePools::factor`],
    /// nur ganzzahlig hereingegeben. Ohne getragene Ausruestung ist es exakt
    /// 1,0, und 1,0 ist in IEEE754 die exakte Eins: Der Faktor bewegt dann
    /// keine einzige Zahl.
    #[must_use]
    pub fn gear_effect_factor(&self) -> f64 {
        1.0 + f64::from(self.gear_effects) / 1000.0
    }

    /// Der Faktor von **Topf 1** aus ESSENCE (Issue #282).
    ///
    /// Dieselbe Regel wie [`Self::gear_effect_factor`] — eins plus die Summe —
    /// aber ein **eigener** Faktor. Zwischen den Toepfen wird multipliziert
    /// (13.1b); waeren beide derselbe, waere die Rechnung additiv und die
    /// Abwaegung aus 13.5 waere gekippt.
    ///
    /// Ohne getragene Ausruestung ist es exakt 1,0, und 1,0 ist in IEEE754 die
    /// exakte Eins: Die Multiplikation bewegt dann keine einzige Zahl.
    #[must_use]
    pub fn essence_factor(&self) -> f64 {
        1.0 + f64::from(self.essence) / 1000.0
    }

    /// Die Kritchance als Wahrscheinlichkeit in `[0, 1]`.
    ///
    /// **Der Deckel bei 100 % ist keine Balance-Entscheidung, sondern eine
    /// Rechenregel:** Eine Wahrscheinlichkeit ueber eins gibt es nicht, und
    /// [`crate::rng::RandomStream::chance`] wuerde sie ohnehin als sicheren
    /// Treffer behandeln. 6.7 haelt denselben Deckel fest — „jeder Treffer
    /// kritet; darueber hinaus wirkungslos".
    #[must_use]
    pub fn crit_chance(&self) -> f64 {
        if self.purity >= 1000 {
            return 1.0;
        }
        f64::from(self.purity) / 1000.0
    }

    /// Der Faktor, mit dem ein kritischer Treffer multipliziert wird.
    ///
    /// Basis 150 %, plus YIELD. **Ohne YIELD ist es genau 1,5** — und weil 1,5
    /// in IEEE754 exakt darstellbar ist, ist der Wert auf beiden Seiten
    /// bitgleich, ohne dass es einer Zusicherung bedarf.
    #[must_use]
    pub fn crit_factor(&self) -> f64 {
        f64::from(CRIT_BASE_PERMILLE + self.yield_bonus) / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Die Gegenprobe zur Vorübersetzung** (Issue #908, AC6).
    ///
    /// Die Tabelle entsteht aus derselben `const fn`, die auch
    /// [`budget_neu_berechnen`] ruft — dieser Fall belegt also nicht, dass zwei
    /// Formeln übereinstimmen, sondern dass die **Tabellenbildung** stimmt:
    /// dass jeder Platz den Wert seiner eigenen Stufe trägt und nicht den des
    /// Nachbarn. Genau das ist der Fehler, der bei einer Index-Verschiebung
    /// entsteht und den kein Stichprobenwert findet.
    #[test]
    fn die_tabelle_traegt_an_jedem_platz_den_wert_ihrer_stufe() {
        let mut geprueft = 0;
        let mut von_null_verschieden = 0;

        for stufe in 0..=MAX_ITEM_LEVEL {
            for guete in 0..QUALITY_COUNT {
                let nachgeschlagen = budget_of(stufe, guete);
                let gerechnet = budget_neu_berechnen(stufe, guete);
                assert_eq!(
                    nachgeschlagen, gerechnet,
                    "Stufe {stufe}, Güte {guete}: Tabelle {nachgeschlagen}, \
                     gerechnet {gerechnet}"
                );
                geprueft += 1;
                if nachgeschlagen != 0 {
                    von_null_verschieden += 1;
                }
            }
        }

        assert_eq!(
            geprueft,
            (MAX_ITEM_LEVEL as usize + 1) * QUALITY_COUNT,
            "Trägerzusicherung: es wurden wirklich alle Plätze angesehen"
        );
        assert_eq!(
            von_null_verschieden,
            MAX_ITEM_LEVEL as usize * QUALITY_COUNT,
            "Trägerzusicherung: nur der Platz auf Stufe 0 ist leer — wären alle \
             null, verglichen wir Nullen mit Nullen"
        );
    }

    /// Die Verschiebung, die der Vergleich oben fangen MUSS.
    ///
    /// Nachbarstufen unterscheiden sich, sonst wäre eine um eins verrutschte
    /// Tabelle nicht von einer richtigen zu unterscheiden. Das ist keine
    /// Selbstverständlichkeit: Bei ganzzahligem Wachstum von 21/20 stehen
    /// kleine Budgets über mehrere Stufen still.
    #[test]
    fn benachbarte_stufen_tragen_verschiedene_werte() {
        let mut gleich_zu_vorgaenger = 0;
        for stufe in 2..=MAX_ITEM_LEVEL {
            if budget_of(stufe, 0) == budget_of(stufe - 1, 0) {
                gleich_zu_vorgaenger += 1;
            }
        }
        assert!(
            gleich_zu_vorgaenger < MAX_ITEM_LEVEL as usize / 2,
            "zu viele Stufen sind wertgleich ({gleich_zu_vorgaenger}) — dann \
             fände der Tabellenvergleich eine Verschiebung nicht"
        );
    }

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
        for (quality, &slots) in QUALITY_AFFIX_SLOTS.iter().enumerate() {
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
    fn die_plaetze_sind_nur_behaelter() {
        // Bis zum 2026-07-29 stand hier eine Pruefung auf die zwei Plaetze ohne
        // Zuordnung. Die Zuordnung selbst ist gestrichen — geblieben ist, dass
        // es zehn Behaelter mit Namen gibt, mehr nicht.
        assert_eq!(SLOT_NAMES.len(), SLOT_COUNT);
        assert_eq!(SLOT_NAMES[3], "relic");
        assert_eq!(SLOT_NAMES[5], "shard");
        // Und die drei Grundwerte, die jeder Gegenstand traegt — zusaetzlich zu
        // den Affixen, deshalb mehr als der eine Affix eines Common-Stuecks.
        assert_eq!(BASE_STAT_IDS.len(), 3);
        assert!(BASE_STAT_IDS.len() > QUALITY_AFFIX_SLOTS[QUALITY_COMMON]);
    }
}
