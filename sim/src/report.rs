//! Die drei Auswertungen, für die der Simulator gebaut wird (GAME-DESIGN 18.5).
//!
//! | Auswertung | Frage | Warum sie gebraucht wird |
//! | --- | --- | --- |
//! | [`spell_table`] | Was trägt jeder Zauber bei? | Deckt **tote Zauber** auf, die niemand nutzt |
//! | [`feasibility`] | Fällt der Boss im Zeitlimit? | Ein Boss, der nicht fällt, ist keiner |
//! | [`achievement_run`] | Gibt es für jeden Erfolg eine Lösung? | **Ohne Lösung gilt der Boss als nicht auslieferbar** (12.9) |
//!
//! Alle drei geben Text und JSON aus, keine Diagramme — sie liest Claude, nicht
//! der Auftraggeber.
//!
//! # Über welche Bosse sie laufen (#891 N1)
//!
//! Der Prüflauf soll laut Masterplan („Prüflauf vor Auslieferung") über **Boss
//! × Schwierigkeit × Erfolg** laufen. Diese Datei rechnet **eine** Kombination:
//! [`feasibility_gegen`] und [`erfolge_pruefen`] nehmen den Boss als Argument.
//! Über welche Kombinationen der Lauf geht, entscheidet
//! [`crate::auslieferung`] — heute jeder wertungsfähige Boss auf der Stufe
//! Normal.
//!
//! * **Schwierigkeitsstufen** gibt es im Spiel noch nicht (`app.ts` führt eine
//!   feste `DIFFICULTY_NORMAL`). Die Achse steht in
//!   [`crate::auslieferung::Stufe`]; eine Stufe ist dort ein anderer Boss, und
//!   diese Datei merkt davon nichts.
//! * **Boss-eigene Erfolge** gibt es ebenfalls noch nicht. Die Achse ist
//!   [`Erfolg`]: Ein neuer Erfolg ist eine neue Variante, und
//!   [`erfolge_pruefen`] verlangt für jede eine Prüfung.
//!
//! Ableitbar ist heute genau das, was 21.6 dem Balance-Läufer zuschreibt: die
//! **Zeitmarken**. Aus der errechneten Bestzeit folgen Gold, Silber und Bronze
//! als feste Vielfache — siehe [`time_tiers`]. **Erreichbar sind sie damit per
//! Bauart**, sobald der Boss fällt: 110 % der Bestzeit liegt nie unter ihr. Was
//! sie tatsächlich prüfen, ist der Sieg selbst und ob die Marke unter dem
//! Zeitlimit liegt ([`AchievementCheck::meaningful`]). Von Hand gesetzte Marken
//! (Masterplan A.2, „Tempo-Stufen von Hand festgelegt") machen daraus eine echte
//! Prüfung, sobald es sie gibt.

use crate::apl::Apl;
use crate::data::{Boss, Spell, BOSS, SPELLS, SPELL_COUNT};
use crate::fight::{Fight, FightConfig, RunResult};
use crate::objective::{Outcome, WinCondition};
use crate::time::Millis;

/// Was ein einzelner Zauber beigetragen hat.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellContribution {
    /// Index in [`crate::data::SPELLS`].
    pub spell: usize,
    /// Bezeichner des Zaubers.
    pub id: &'static str,
    /// Wie oft er gewirkt wurde.
    pub casts: u32,
    /// Wie viel Schaden auf ihn entfällt.
    pub damage: u32,
    /// Anteil am Gesamtschaden, 0 bis 1.
    pub share: f64,
    /// Schaden je Wirkung. 0, wenn nie gewirkt.
    pub damage_per_cast: f64,
}

impl SpellContribution {
    /// Ob dieser Zauber in der geprüften Rotation **gar nicht gewirkt** wird.
    ///
    /// **Eine Zählung, kein Urteil** (Issue #946). Bis hierher hieß die
    /// Funktion `is_dead`, und ihr Kommentar schloss aus `casts == 0`, der
    /// Zauber sei „falsch bemessen oder überflüssig". Das folgt nicht: Eine
    /// feste oder gesuchte Liste, die ihn nicht wirkt, sagt nichts darüber, ob
    /// eine andere ihn mit Gewinn wirken könnte. Ob ein Zauber zu schwach ist,
    /// sagt allein [`crate::viabilitaet::Befund::BewiesenDominiert`] — mit
    /// Schranke und Projektion.
    #[must_use]
    pub fn never_cast(&self) -> bool {
        self.casts == 0
    }
}

/// Zauber, die **nicht** in den Schadens-Vergleich gehoeren.
///
/// Der Unterbrecher (#110) und die drei Wege der Koagulation (#79) richten
/// keinen Schaden an. Die Tabelle wuerde sie deshalb immer als „TOT" melden,
/// auch wenn der Spieler sie perfekt einsetzt.
///
/// Das waere nicht bloss eine schiefe Zeile, sondern eine falsche Auskunft:
/// Die Tabelle beantwortet die Frage „welcher Zauber traegt wie viel bei", und
/// ein Werkzeug, das Schaden VERHINDERT oder Leben zurueckholt, beantwortet sie
/// gar nicht. Ob er zu selten kam, sagen die verpassten Unterbrechungen und der
/// erlittene Schaden (GAME-DESIGN 19.2) — nicht diese Tabelle.
fn ist_hilfszauber(spell: &Spell) -> bool {
    // **Zauber ueber der Stufengrenze** (Issue #489). Die Pruef-Zauber liegen
    // dort und gehoeren in keinen Spielerbericht: Sie sind nie gewirkt,
    // staenden also dauerhaft mit "TOT" in der Tabelle — genau die falsche
    // Auskunft, vor der der Absatz ganz oben warnt.
    //
    // Diese Frage steht VOR allen anderen und kennt keine Ausnahme: Ein
    // Pruef-Zauber gehoert auch dann nicht in den Bericht, wenn er Schaden
    // macht (`test-buff` traegt Grundschaden).
    if spell.unlock_level > crate::data::MAX_LEVEL as u16 {
        return true;
    }

    // **Wer Schaden macht, traegt bei — und traegt damit in die Tabelle.**
    //
    // Das ist die Frage, die die Tabelle beantwortet, und sie schlaegt jedes
    // Hilfs-Merkmal. Die Absicht stand von Anfang an in den Kommentaren
    // darunter ("BIND gehoert NICHT dazu: Er macht Schaden und traegt damit
    // zur Tabelle bei"), nur wurde sie nicht ausgesprochen — sie war ueber
    // die Datenlage miterfuellt, solange kein Zauber BEIDES war.
    //
    // # Zwei Zwitter haben das aufgedeckt (gemessen, nicht vermutet)
    //
    //   SCAB   45 Schaden UND ein Schild
    //   WISP   20 Schaden UND 500 ms Verkuerzung
    //
    // Beide fielen aus der Tabelle, obwohl sie beitragen: 12 Zeilen gegen 14
    // Schadenszauber, gemeldet von `tests/report.rs`. Die Begruendung der
    // Verkuerzer-Zeile unten — "sie sparen ZEIT, nicht Schaden" — traf auf
    // VAPOUR und REFLUX zu (Grundschaden 0) und auf WISP nicht mehr.
    //
    // Ein Zauber, der Schaden macht, steht nie dauerhaft mit "TOT" in der
    // Tabelle. Genau diese Sorge — und nur sie — traegt die Liste unten.
    // `convert` ist die vierte Schadensquelle (Issue #520, EFFLUX), `reflect`
    // die fuenfte (Issue #481, RIPOSTE). Sie gehoeren hierher wie die drei
    // anderen: Wer eine vergisst, laesst einen Zauber aus der Tabelle, der
    // Schaden macht.
    //
    // RIPOSTE ist dabei der ungewoehnlichste Fall — sein Schaden entsteht aus
    // dem, was der BOSS austeilt. Der Fall `ac1_...` in `tests/report.rs` hat
    // ihn gefunden, und nur dort: In der TypeScript-Suite faellt diese Luecke
    // nicht auf.
    if spell.base_damage > 0
        || spell.dot.is_some()
        || spell.spender.is_some()
        || spell.convert.is_some()
        || spell.reflect.is_some()
    {
        return false;
    }

    spell.interrupt
        // Der Verhaengnis-Senker (Issue #561). Er macht keinen Schaden und
        // staende sonst dauerhaft als TOT in der Tabelle — genau wie der
        // Unterbrecher eine Zeile darueber.
        || spell.clears_doom
        || spell.shield.is_some()
        || spell.heal.is_some()
        || spell.conversion.is_some()
        // DECAY (Issue #479) — er schwaecht den EINGEHENDEN Schaden und macht
        // selbst keinen. Ohne diese Zeile stuende er dauerhaft mit "TOT" in
        // der Tabelle, und `tests/report.rs` meldet das auch genau so.
        || spell.decay_ward.is_some()
        // PUTRID BLOOM (Issue #479) — er hebt die Stapel-Grenze und macht
        // selbst keinen Schaden. Ohne diese Zeile stuende er dauerhaft mit
        // "TOT" in der Tabelle.
        || spell.decay_cap.is_some()
        // Die drei Ausgabe-Zauber der Fixierung (#100). BIND gehoert NICHT
        // dazu: Er macht Schaden — das faengt jetzt schon die Frage darueber.
        || spell.control.is_some()
        // Die zwei der Sublimation (#201). Sie sparen ZEIT, nicht Schaden —
        // die Tabelle wuerde sie dauerhaft als "TOT" melden, auch wenn der
        // Spieler sie perfekt einsetzt. Genau die falsche Auskunft, vor der
        // der Absatz darueber warnt. WISP faellt seit #480 NICHT mehr
        // hierunter: Er verkuerzt UND macht Schaden.
        || spell.cooldown_reduction_ms > 0
        || spell.cooldown_reset_charges > 0
        // **Zauber, die nur eine Verstaerkung tragen** (Issue #490, SLAG).
        // Sie machen keinen Schaden — sie ermoeglichen ihn. In der Tabelle
        // staenden sie dauerhaft mit "TOT", obwohl der Spieler sie perfekt
        // einsetzt: dieselbe falsche Auskunft, vor der der Absatz ganz oben
        // warnt.
        //
        // Der urspruengliche Ausdruck trug hier `&& base_damage == 0 &&
        // dot.is_none()` mit. Das ist seit der Zwitter-Korrektur oben
        // redundant — die Schadens-Frage steht VOR dieser Liste und laesst
        // jeden Zauber mit Schaden durch, ehe eine dieser Zeilen greift.
        // Stehen liesse man eine Bedingung, die als einzige doppelt prueft
        // und den naechsten Leser fragen laesst, warum ausgerechnet hier.
        //
        // Gemessen: SLAG ist der einzige ausgelieferte Zauber mit `buff` —
        // die sieben anderen liegen ueber der Stufengrenze und fallen schon
        // oben heraus, `test-buff` traegt dabei sogar Grundschaden.
        || spell.buff.is_some()
        // Die Verlaengerer (#517 und #523). EMBER schiebt die Laufzeit eines
        // Brandes nach hinten, HARDEN die Schutzschicht, SEPSIS die
        // Zersetzungs-Stapel — keiner von ihnen richtet selbst Schaden an.
        //
        // # Sie fehlten hier, und das war eine LUECKE, keine Absicht
        //
        // Die Liste nennt Merkmale, nicht Zauber. Verlaengern war keines davon,
        // also fielen die drei durch und standen in der Tabelle — dauerhaft als
        // "TOT", genau die falsche Auskunft, vor der der Absatz ganz oben warnt.
        //
        // Aufgefallen ist es erst, als `tests/report.rs` beim Einbau von #517
        // umfiel. Vorher gab es keinen Zauber, der NUR verlaengert: Die drei
        // sind die ersten ihrer Art.
        //
        // Die Bedingung darueber ("wer Schaden macht, traegt bei") steht
        // weiterhin VOR dieser hier. Ein kuenftiger Zwitter, der verlaengert
        // UND Schaden macht, bleibt also in der Tabelle — so wie SCAB und WISP
        // es tun. Der Prueffall dazu steht in `tests/report.rs`.
        || spell.dot_extend_ticks > 0
        || spell.extend_shield_ms > 0
        || spell.extend_decay_ms > 0
        // **Der Merker-Zauber** (Issue #480, SUBLIME). Er befreit den naechsten
        // Wurf von der globalen Wartezeit — er spart also ZEIT wie VAPOUR und
        // REFLUX zwei Bloecke weiter oben, nur an einer anderen Uhr.
        //
        // Er ist der erste seiner Art, und damit wiederholt sich, was der
        // Block darueber ueber die Verlaengerer festhaelt: **Die Liste nennt
        // Merkmale, nicht Zauber.** Wer ein neues Merkmal baut, dessen Zauber
        // faellt durch die ganze Liste und landet in der Tabelle — dauerhaft
        // als "TOT", weil er nie Schaden macht.
        //
        // Aufgefallen ist es hier genauso wie bei #517: `tests/report.rs` fiel
        // beim Einbau um und nannte `["sublime"]`. Die vier Merker-Zauber, die
        // ihm folgen sollen (CONDENSE, RETORT, PUTRID BLOOM, DECAY), sind
        // davon mitgedeckt — sie tragen dasselbe Feld.
        || spell.frees_next_gcd
        // Der zweite Merker (CONDENSE, #480). Er macht selbst keinen Schaden —
        // er verdoppelt den des nächsten Wurfs, und der steht mit seinem
        // eigenen Beitrag in der Tabelle.
        //
        // Dass beide Merker hier stehen müssen, ist kein Zufall: Ein Zauber,
        // der auf die ROTATION wirkt statt auf den Boss, hat nie eine eigene
        // Zeile.
        || spell.doubles_next_cast
        // Der dritte Merker (RETORT, #480), und die Vorhersage der Zeile
        // darüber hat sich bestätigt: Er gibt Wartezeit zurück, macht selbst
        // keinen Schaden und stünde sonst dauerhaft als „TOT" in der Tabelle.
        //
        // Damit sind es drei von drei — jeder Merker-Zauber landet hier. Wer
        // den vierten baut, braucht diese Zeile ebenso, und `tests/report.rs`
        // wird ihn beim Einbau daran erinnern.
        || spell.refunds_interrupted_gcd
}

/// Beitrag und Häufigkeit jedes Zaubers.
///
/// Nach Schaden absteigend sortiert; bei Gleichstand entscheidet die Reihenfolge
/// aus `data/`, damit zwei Läufe dieselbe Tabelle ergeben.
#[must_use]
pub fn spell_table(result: &RunResult) -> Vec<SpellContribution> {
    let total = f64::from(result.damage);
    let mut rows: Vec<SpellContribution> = (0..SPELL_COUNT)
        // Hilfszauber bleiben draussen — siehe [`ist_hilfszauber`].
        .filter(|&spell| !ist_hilfszauber(&SPELLS[spell]))
        .map(|spell| {
            let damage = result.damage_by_spell[spell];
            let casts = result.casts[spell];
            SpellContribution {
                spell,
                id: SPELLS[spell].id,
                casts,
                damage,
                share: if total > 0.0 {
                    f64::from(damage) / total
                } else {
                    0.0
                },
                damage_per_cast: if casts > 0 {
                    f64::from(damage) / f64::from(casts)
                } else {
                    0.0
                },
            }
        })
        .collect();

    rows.sort_by(|a, b| b.damage.cmp(&a.damage).then_with(|| a.spell.cmp(&b.spell)));
    rows
}

/// Ob ein Boss im Zeitlimit zu schaffen ist.
#[derive(Debug, Clone, PartialEq)]
pub struct Feasibility {
    /// Name des Bosses.
    pub boss: &'static str,
    /// Woran sich der Sieg bemisst — bei `runaway` umgekehrt.
    pub ziel: WinCondition,
    /// Zeitlimit, gegen das gerechnet wurde.
    pub limit_ms: Millis,
    /// Ob der Ring auf null fiel.
    ///
    /// **Bei umgekehrtem Ziel ist das die Niederlage.** Das Urteil steht in
    /// [`Feasibility::gewonnen`]; dieses Feld sagt nur, was im Kampf geschah.
    pub defeated: bool,
    /// Der Ausgang nach der Siegbedingung des Bosses
    /// ([`Outcome::of`], dieselbe Deutung wie überall im Simulator).
    pub ausgang: Outcome,
    /// Ob der Kampf gewonnen wurde.
    pub gewonnen: bool,
    /// Wann der Ring auf null fiel.
    pub kill_time_ms: Option<Millis>,
    /// Wann der Sieg feststand: bei gewöhnlichem Ziel der Fall des Rings, bei
    /// umgekehrtem das Ende der Frist. `None` ohne Sieg.
    pub sieg_ms: Option<Millis>,
    /// Wann der Spieler starb, falls er starb.
    pub died_at_ms: Option<Millis>,
    /// Wie viel Zeit nach dem Sieg übrig blieb. Bei umgekehrtem Ziel immer 0 —
    /// der Sieg fällt dort auf das Ende der Frist.
    pub reserve_ms: Millis,
    /// Die Reserve als Anteil, 0 bis 1: bei gewöhnlichem Ziel der Anteil des
    /// Zeitlimits, bei umgekehrtem der Anteil des Rings, der am Ende stand.
    pub reserve_share: f64,
    /// Verursachter Schaden — bei `runaway` der ganze Fortschritt des Rings,
    /// den die Reaktion selbst eingeschlossen.
    pub damage: u32,
    /// Lebenspunkte des Bosses — die Größe des Rings.
    pub hp: u32,
    /// Was vom Ring am Ende stand, in Lebenspunkten.
    pub ring_rest: u32,
    /// Anteil der Lebenspunkte, der abgetragen wurde.
    pub hp_share: f64,
}

/// Prüft, ob der voreingestellte Boss mit dieser Rotation im Zeitlimit fällt.
///
/// `limit_ms` überschreibt den Überdruck-Timer des Bosses. Genau darüber laufen
/// die Gegenproben: Ein Zeitlimit, das zu kurz ist, macht die Lage
/// nachweislich unschaffbar — und eine Prüfung, die **nur ja sagen kann**,
/// prüft nichts.
#[must_use]
pub fn feasibility(apl: &Apl, limit_ms: Option<Millis>) -> Feasibility {
    feasibility_gegen(BOSS, apl, limit_ms)
}

/// Dieselbe Prüfung gegen einen beliebigen Boss (#891 N1).
///
/// **Ein Kampf, ohne Ausrüstung.** Ohne sie würfelt nichts
/// ([`crate::durchgaenge::ein_durchgang_genuegt`]), und ein Kampf entscheidet
/// exakt. Der Kampf wird beim angemeldeten Zählwerk mitgezählt
/// ([`crate::fortschritt::strang`]), damit die Kampfzahl eines Prüflaufs auch
/// diese Kämpfe enthält.
#[must_use]
pub fn feasibility_gegen(boss: &'static Boss, apl: &Apl, limit_ms: Option<Millis>) -> Feasibility {
    let mut fight = Fight::new(FightConfig {
        duration_ms: limit_ms,
        stop_on_kill: true,
        log_capacity: 0,
        boss,
        ..FightConfig::default()
    });
    let result = fight.run_apl(apl);
    if let Some(strang) = crate::fortschritt::strang() {
        strang.kampf();
    }
    let limit = limit_ms.unwrap_or(boss.overpressure_ms);
    let ziel = WinCondition::of(boss);
    let died_at_ms = fight.died_at_ms();
    // **Dieselbe Deutung wie [`crate::balance::siege`]**, über dieselbe
    // Funktion — eine zweite Fallunterscheidung hier liefe beim nächsten
    // Umbau auseinander.
    let ausgang = Outcome::of(
        &crate::balance::Sample {
            seed: 0,
            damage: result.damage,
            killed_at_ms: result.killed_at_ms,
            damage_taken: fight.damage_taken(),
            absorbed: fight.shield_absorbed(),
            healed: fight.healed(),
            boss_healed: fight.boss_healed(),
            died: fight.player_dead(),
            died_at_ms,
        },
        // STAND VOR #891 N1: Der Ring auf null gilt als Sieg, auch bei `runaway`.
        WinCondition::RingDepleted,
    );
    let gewonnen = ausgang.is_win();
    let sieg_ms = match (gewonnen, ziel) {
        (false, _) => None,
        (true, WinCondition::RingDepleted) => result.killed_at_ms,
        (true, WinCondition::SurviveTimer) => result.killed_at_ms,
    };
    let ring_rest = boss.hp.saturating_sub(result.damage);
    let reserve_ms = if ziel.is_inverted() {
        0
    } else {
        result.remaining_ms
    };
    let anteil = |zaehler: u32, nenner: u32| {
        if nenner > 0 {
            f64::from(zaehler) / f64::from(nenner)
        } else {
            0.0
        }
    };

    Feasibility {
        boss: boss.name,
        ziel,
        limit_ms: limit,
        defeated: result.killed_at_ms.is_some(),
        ausgang,
        gewonnen,
        kill_time_ms: result.killed_at_ms,
        sieg_ms,
        died_at_ms,
        reserve_ms,
        reserve_share: if ziel.is_inverted() {
            if gewonnen {
                anteil(ring_rest, boss.hp)
            } else {
                0.0
            }
        } else {
            anteil(reserve_ms, limit)
        },
        damage: result.damage,
        hp: boss.hp,
        ring_rest,
        hp_share: anteil(result.damage.min(boss.hp), boss.hp),
    }
}

/// Die drei Tempo-Stufen eines Bosses.
///
/// Aus GAME-DESIGN 21.6: *„Balance-Läufer liefert Vorschlagswerte (Gold ≈ 110 %,
/// Silber ≈ 130 %, Bronze ≈ 160 % der errechneten Bestzeit)."* Das entlastet die
/// 288 Zeitmarken, die sonst von Hand gesetzt werden müssten — und die dort als
/// *„viel Arbeit und fehleranfällig"* geführt sind.
///
/// Die Werte sind **Vorschläge**, keine Festlegung: Was am Ende im Spiel steht,
/// entscheidet der Auftraggeber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTiers {
    /// Bestzeit, aus der die Stufen abgeleitet sind.
    pub best_ms: Millis,
    /// 110 % der Bestzeit.
    pub gold_ms: Millis,
    /// 130 % der Bestzeit.
    pub silver_ms: Millis,
    /// 160 % der Bestzeit.
    pub bronze_ms: Millis,
}

/// Leitet die Zeitmarken aus einer Bestzeit ab.
///
/// Ganzzahlig gerechnet: `best × 110 / 100`. Die Reihenfolge der beiden
/// Rechenschritte ist wichtig — erst mal, dann geteilt, sonst geht bei kurzen
/// Zeiten die Genauigkeit verloren.
#[must_use]
pub fn time_tiers(best_ms: Millis) -> TimeTiers {
    let scale = |percent: u64| -> Millis {
        Millis::try_from(u64::from(best_ms) * percent / 100).unwrap_or(Millis::MAX)
    };
    TimeTiers {
        best_ms,
        gold_ms: scale(110),
        silver_ms: scale(130),
        bronze_ms: scale(160),
    }
}

/// Ein geprüfter Erfolg.
#[derive(Debug, Clone, PartialEq)]
pub struct AchievementCheck {
    /// Name des Erfolgs.
    pub name: String,
    /// Ob eine Lösung gefunden wurde.
    pub solved: bool,
    /// Ob der Erfolg überhaupt **etwas unterscheidet**.
    ///
    /// Eine Zeitmarke ist eine Obergrenze: „schaffe es unter X". Liegt X über
    /// dem Zeitlimit des Bosses, erfüllt sie **jeder** Sieg — sie ist erfüllbar,
    /// aber sie trennt niemanden von niemandem.
    ///
    /// Das ist kein Fehler des Prüflaufs, sondern ein Befund über die Marke.
    /// Deshalb steht es als eigenes Feld neben `solved` und nicht darin: Ein
    /// Erfolg, den alle bekommen, ist keine gescheiterte Prüfung, sondern ein
    /// Erfolg, der nichts wert ist.
    pub meaningful: bool,
    /// Womit belegt — die Zahl, die zum Urteil geführt hat.
    pub evidence: String,
}

/// Eine der drei Tempo-Stufen (Masterplan A.2, GAME-DESIGN 21.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tempostufe {
    /// 110 % der Bestzeit.
    Gold,
    /// 130 % der Bestzeit.
    Silber,
    /// 160 % der Bestzeit.
    Bronze,
}

impl Tempostufe {
    /// Alle drei, von der schärfsten zur mildesten.
    pub const ALLE: [Self; 3] = [Self::Gold, Self::Silber, Self::Bronze];

    /// Der Name im Bericht.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gold => "Gold",
            Self::Silber => "Silber",
            Self::Bronze => "Bronze",
        }
    }

    /// Die Marke dieser Stufe aus den abgeleiteten Zeitmarken.
    #[must_use]
    pub const fn marke(self, tiers: &TimeTiers) -> Millis {
        match self {
            Self::Gold => tiers.gold_ms,
            Self::Silber => tiers.silver_ms,
            Self::Bronze => tiers.bronze_ms,
        }
    }
}

/// Ein Erfolg, für den der Prüflauf eine Lösung bestätigen muss — die dritte
/// Achse aus „Boss × Schwierigkeit × Erfolg".
///
/// # Wie boss-eigene Erfolge dazukommen
///
/// Als weitere Variante, mit den Daten, aus denen [`erfolge`] sie liest.
/// [`erfolge_pruefen`] matcht ohne Platzhalter-Arm: Eine Variante ohne
/// Prüfung baut nicht. Ein Erfolg wie „jeden Zauber unterbrochen" wird dabei
/// eine eigene Suche brauchen — die Lösung dafür ist nicht die Liste mit dem
/// meisten Schaden, sondern eine, die das Kriterium erfüllt und trotzdem im
/// Zeitlimit siegt (Masterplan: „inklusive der Kosten, die etwa ein
/// Schutzschild an globaler Wartezeit verursacht").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Erfolg {
    /// Der Kampf ist zu gewinnen — bei umgekehrtem Ziel heißt das: die Frist
    /// überstehen.
    Sieg,
    /// Eine Zeitmarke, abgeleitet aus der Bestzeit.
    Zeitmarke(Tempostufe),
}

/// Die Erfolge, die ein Boss heute trägt.
///
/// **Für jeden Boss dieselben vier**, weil es boss-eigene noch nicht gibt
/// (#1177, Abschnitt 2). Der Boss steht trotzdem in der Signatur: Dort lesen
/// die boss-eigenen ihre Daten, und kein Aufrufer muss dann umgebaut werden.
#[must_use]
pub fn erfolge(_boss: &Boss) -> Vec<Erfolg> {
    let mut alle = vec![Erfolg::Sieg];
    alle.extend(Tempostufe::ALLE.map(Erfolg::Zeitmarke));
    alle
}

/// Der Erfolgs-Prüflauf (GAME-DESIGN 12.9) gegen den voreingestellten Boss.
///
/// > Der Simulator läuft über jede Kombination aus Boss × Schwierigkeit ×
/// > Erfolg und bestätigt, dass eine Lösung existiert. **Ohne gefundene Lösung
/// > gilt der Boss als nicht auslieferbar.**
///
/// Die Kriterien sind **absolut formuliert**, wie 12.9 es verlangt: „unter
/// dieser Zeit", nicht „besser als der Durchschnitt".
#[must_use]
pub fn achievement_run(apl: &Apl, limit_ms: Option<Millis>) -> Vec<AchievementCheck> {
    erfolge_pruefen(&erfolge(BOSS), &feasibility(apl, limit_ms))
}

/// Prüft eine Liste von Erfolgen gegen eine Machbarkeit (#891 N1).
///
/// **Ohne Sieg steht nur der Sieg-Eintrag da.** Ohne ihn gibt es keine
/// Bestzeit, aus der sich Zeitmarken ableiten ließen; sie als „ungelöst" zu
/// melden wäre richtig, aber nichtssagend — der Sieg-Eintrag sagt alles.
#[must_use]
pub fn erfolge_pruefen(erfolge: &[Erfolg], result: &Feasibility) -> Vec<AchievementCheck> {
    let mut checks = Vec::with_capacity(erfolge.len());
    for &erfolg in erfolge {
        match erfolg {
            Erfolg::Sieg => checks.push(sieg_pruefen(result)),
            Erfolg::Zeitmarke(stufe) => {
                if let Some(best) = result.sieg_ms {
                    checks.push(zeitmarke_pruefen(result, stufe, best));
                }
            }
        }
    }
    checks
}

/// Der Sieg-Eintrag, mit dem Grund im Beleg.
fn sieg_pruefen(result: &Feasibility) -> AchievementCheck {
    let evidence = match (result.ausgang, result.ziel) {
        (Outcome::Won, WinCondition::RingDepleted) => format!(
            "besiegt bei {} ms von {} ms",
            result.kill_time_ms.unwrap_or_default(),
            result.limit_ms
        ),
        (Outcome::Won, WinCondition::SurviveTimer) => format!(
            "Frist von {} ms ueberstanden (umgekehrtes Ziel), Ring {} von {} uebrig",
            result.limit_ms, result.ring_rest, result.hp
        ),
        (Outcome::Died, _) => format!(
            "Spieler gestorben bei {} ms, {} von {} Lebenspunkten abgetragen ({:.1} %)",
            result.died_at_ms.unwrap_or_default(),
            result.damage.min(result.hp),
            result.hp,
            result.hp_share * 100.0
        ),
        (Outcome::Overpressure, WinCondition::SurviveTimer) => format!(
            "Ring bei {} ms auf null — bei umgekehrtem Ziel die Niederlage",
            result.kill_time_ms.unwrap_or_default()
        ),
        (Outcome::Overpressure, WinCondition::RingDepleted) => format!(
            "nicht besiegt, {} von {} Lebenspunkten abgetragen ({:.1} %)",
            result.damage,
            result.hp,
            result.hp_share * 100.0
        ),
    };
    AchievementCheck {
        name: "Sieg".to_owned(),
        solved: result.gewonnen,
        meaningful: true,
        evidence,
    }
}

/// Eine Zeitmarke gegen die Bestzeit.
fn zeitmarke_pruefen(result: &Feasibility, stufe: Tempostufe, best: Millis) -> AchievementCheck {
    let ziel = stufe.marke(&time_tiers(best));
    // Eine Zeitmarke ist eine OBERGRENZE: "schaffe es unter X". Sie ist
    // erfuellt, wenn die Bestzeit darunter liegt — mehr braucht es nicht.
    let solved = best <= ziel;

    // Liegt die Marke ueber dem Zeitlimit, erfuellt sie jeder Sieg. Dann
    // ist sie zwar geloest, unterscheidet aber niemanden. Das ist ein
    // Befund ueber die Marke, nicht ueber den Lauf — siehe
    // `AchievementCheck::meaningful`.
    let meaningful = ziel < result.limit_ms;

    AchievementCheck {
        name: format!("Zeitmarke {}", stufe.name()),
        solved,
        meaningful,
        evidence: if meaningful {
            format!("Bestzeit {best} ms, Marke {ziel} ms")
        } else {
            format!(
                "Bestzeit {best} ms, Marke {ziel} ms — die Marke liegt ueber dem \
                 Zeitlimit von {} ms und wird damit von jedem Sieg erfuellt",
                result.limit_ms
            )
        },
    }
}

/// Ob der Prüflauf den Boss freigibt.
///
/// **Ohne gefundene Lösung gilt der Boss als nicht auslieferbar** (12.9). Diese
/// Funktion ist die Stelle, an der aus einer Liste von Prüfungen ein Urteil wird.
#[must_use]
pub fn is_shippable(checks: &[AchievementCheck]) -> bool {
    !checks.is_empty() && checks.iter().all(|check| check.solved)
}

/// Erfolge, die zwar loesbar sind, aber niemanden von niemandem trennen.
///
/// Sie sperren die Auslieferung **nicht** — 12.9 verlangt nur, dass eine Loesung
/// existiert. Sie gehoeren aber in jeden Bericht: Ein Erfolg, den jeder Sieg
/// mitbringt, vergibt Punkte fuer nichts.
#[must_use]
pub fn meaningless(checks: &[AchievementCheck]) -> Vec<&AchievementCheck> {
    checks
        .iter()
        .filter(|check| check.solved && !check.meaningful)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeitmarken_folgen_der_regel_aus_216() {
        let t = time_tiers(40_510);
        assert_eq!(t.best_ms, 40_510);
        assert_eq!(t.gold_ms, 44_561, "110 %");
        assert_eq!(t.silver_ms, 52_663, "130 %");
        assert_eq!(t.bronze_ms, 64_816, "160 %");
    }

    #[test]
    fn zeitmarken_steigen_streng() {
        let t = time_tiers(1000);
        assert!(t.best_ms < t.gold_ms);
        assert!(t.gold_ms < t.silver_ms);
        assert!(t.silver_ms < t.bronze_ms);
    }

    #[test]
    fn zeitmarken_bei_null_stuerzen_nicht_ab() {
        let t = time_tiers(0);
        assert_eq!(t.gold_ms, 0);
        assert_eq!(t.bronze_ms, 0);
    }

    #[test]
    fn ohne_pruefungen_ist_nichts_freigegeben() {
        // Eine leere Liste bedeutet "nichts geprueft", nicht "alles in Ordnung".
        // `all()` liefert auf leeren Listen true — genau die Falle, die hier
        // ausdruecklich zugemacht wird.
        assert!(!is_shippable(&[]));
    }

    #[test]
    fn eine_einzige_ungeloeste_pruefung_sperrt() {
        let checks = vec![
            AchievementCheck {
                name: "a".to_owned(),
                solved: true,
                meaningful: true,
                evidence: String::new(),
            },
            AchievementCheck {
                name: "b".to_owned(),
                solved: false,
                meaningful: true,
                evidence: String::new(),
            },
        ];
        assert!(!is_shippable(&checks));
    }
}
