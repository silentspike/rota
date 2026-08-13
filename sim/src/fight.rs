//! Der Kampf — ereignisgesteuert.
//!
//! # Warum ereignisgesteuert und nicht im Takt
//!
//! Das Spiel rechnet in einem festen Logik-Takt von 10 ms (GAME-DESIGN 18.2).
//! Bei 60 Sekunden Kampf sind das 6.000 Schritte, von denen rund 5.950 nichts
//! tun. Fuer das Spiel ist das richtig: Der Takt macht das Verhalten
//! geraeteunabhaengig. Fuer den Simulator waere es Verschwendung — er soll
//! hunderttausende Durchgaenge rechnen (GAME-DESIGN 18.5).
//!
//! Dieser Simulator springt deshalb von Ereignis zu Ereignis. Ein Durchgang
//! ueber 60 Sekunden kostet damit rund 90 statt 6.000 Schritte.
//!
//! # Wie er trotzdem dasselbe rechnet
//!
//! Ein anderes Rechenverfahren darf **kein anderes Ergebnis** liefern
//! (GAME-DESIGN 18.4, gemeinsame Pruefaelle). Drei Stellen entscheiden darueber:
//!
//! **1. Die Reihenfolge innerhalb einer Millisekunde.** Die Referenz arbeitet je
//! Takt in drei Phasen: erst die passiven Wirkungen (`step_passive`: DoT-Tick,
//! dann Verfall), dann die Pruefung, ob der Boss gefallen ist, dann die
//! Entscheidung des Spielers. [`Fight::run`] arbeitet genauso — nur eben je
//! Ereigniszeitpunkt statt je Takt.
//!
//! **2. Das Takt-Raster.** Das Spiel kann nur zu Takt-Grenzen handeln. Alle
//! Zeitpunkte, die aus einer Abklingzeit entstehen, laufen deshalb durch
//! [`snap_up`]. In Schritt 1 aendert das nichts (alles ist ein Vielfaches von
//! 500 ms); ab Schritt 3 mit Tempo-Faktoren ist es der Unterschied zwischen
//! Gleichlauf und tagelanger Suche.
//!
//! **3. Wann der Sieg bemerkt wird.** Beide vorhandenen Umsetzungen — die
//! Referenz `tools/step1-check.py` und das Spiel `src/devtools/reference-run.ts` —
//! pruefen den Boss-Tod **vor** der Wirk-Phase des Takts. Ein toedlicher Zauber
//! bei 40.500 ms wird deshalb erst im Takt 40.510 ms bemerkt, und genau diese
//! Zahl steht als Sollwert in `STEP1-SPEC` § 4. Ein DoT-Tick dagegen faellt in
//! die passive Phase und wird **im selben Takt** bemerkt. Beides bildet
//! [`Fight::cast`] ab, indem es nach einem toedlichen Zauber einen Weckruf auf
//! den naechsten Takt setzt.
//!
//! Das ist keine Nachbesserung zugunsten einer Wunschzahl, sondern die Regel
//! selbst: Der Sieg gilt in dem Logik-Takt als festgestellt, in dem das Spiel
//! ihn feststellt.
//!
//! # Wo das Verfahren an seine Grenze kommt
//!
//! Die Rotation wird **nur an Ereigniszeitpunkten** befragt. Fuer eine Rotation,
//! die den Zustand abfragt — Abklingzeit bereit? Ressource ueber der Schwelle?
//! Restlaufzeit unter einem GCD? — ist das vollstaendig, denn der Zustand
//! aendert sich ausschliesslich an Ereignissen. Alle vier Rotationen aus
//! `STEP1-SPEC` § 4 sind so gebaut.
//!
//! Fuer eine Rotation, die auf die **Uhr** schaut, waere es unvollstaendig:
//! Faellt der Zeitpunkt, zu dem ihre Bedingung wahr wird, zwischen zwei
//! Ereignisse, bemerkt der Simulator ihn nicht — die Referenz mit ihrem
//! 10-ms-Takt sehr wohl.
//!
//! **Seit E2-3 ist das geloest**, aber nicht hier: [`Decide::next_wakeup`] laesst
//! die Entscheidung selbst sagen, wann sie erneut geprueft werden will, und
//! [`Fight::schedule_decision_wakeup`] plant den Weckruf ein. Eine
//! Prioritaetenliste ([`crate::apl`]) kennt ihre Bedingungen und rechnet den
//! Umschlagpunkt aus; eine Rotation als blosse Funktion kann das nicht und
//! bekommt deshalb weiterhin keine Weckrufe — fuer die vier zustandsabhaengigen
//! Pruefrotationen ist das genau richtig.
//!
//! `tests/edge_cases.rs` haelt beide Seiten fest: dass eine Funktion die Grenze
//! hat, und `tests/apl.rs`, dass eine Prioritaetenliste sie nicht mehr hat.

use crate::buffs::{Buffs, BUFF_KIND_CAPACITY};
use crate::checksum::{mix_named_float, mix_named_int, FNV_OFFSET};
use crate::cooldown_reset::{Choice, CooldownReset};
use crate::data::{
    stack_damage, stat_at_level, Boss, BossHealTrigger, BuffTarget, Control, ControlEffect, Doom,
    Dot, Resource, Spell, TimelineEntry, TimelineKind, BOSS, PLAYER, RESOURCE, SPELLS, SPELL_COUNT,
};
use crate::event::{EventKind, EventQueue};
use crate::factors::{compute_damage, DamageFactors, NEUTRAL};
use crate::gear::WornValues;
use crate::haste::hasted_ms;
use crate::log::{CombatLog, LogKind, NO_SPELL};
use crate::replay::{Recording, ReplayResult};
use crate::rng::{Stream, Streams};
use crate::rules_event::{RuleEvent, RuleEvents, RuleReceiver};
use crate::sets::WHOLE_WORK_GCD_REDUCTION_MS;
use crate::stacks::{PendingGain, StackMechanic};
use crate::sublimation::{shorten_cooldowns, PendingShorten, PendingSlot, SublimationMechanic};
use crate::time::{snap_up, Millis, GCD_MS, TICK_MS};

/// Kampfzustand „laeuft" — der Wert, den `src/core/combat.ts` als
/// `FIGHT_RUNNING` fuehrt.
///
/// **Er aendert sich waehrend eines Durchgangs nicht**, und das ist kein
/// Versehen: Weder `src/devtools/reference-run.ts` noch
/// `src/devtools/recording.ts` rufen `checkDefeat` auf — beide pruefen den
/// Boss-Tod selbst und schreiben den Zustand dabei nicht fort. Da die
/// Zustands-Pruefsumme dieses Feld einruehrt, muss der Simulator es genauso
/// halten; ein hier gesetzter Sieg-Zustand ergaebe ab dem Sieg-Takt eine
/// abweichende Pruefsumme, ohne dass eine Regel anders gerechnet waere.
pub const FIGHT_RUNNING: i32 = 0;

/// Eine Rotation: Welcher Zauber soll jetzt gewirkt werden?
///
/// `None` heisst bewusst warten. Neben dieser einfachen Form gibt es seit E2-3
/// die Prioritaetenliste als eigene Sprache ([`crate::apl`]); beide Wege laufen
/// ueber [`Decide`] durch dieselbe Kampfschleife.
pub type Rotation = fn(&Fight) -> Option<usize>;

/// Wer entscheidet, was gewirkt wird.
///
/// Zwei Umsetzungen: eine gewoehnliche Funktion (die vier Pruefrotationen) und
/// [`crate::apl::Apl`], die geschriebene Prioritaetenliste.
pub trait Decide {
    /// Welcher Zauber jetzt gewirkt werden soll, oder `None` fuer warten.
    fn decide(&self, fight: &Fight) -> Option<usize>;

    /// Wann die Entscheidung erneut geprueft werden will, weil eine ihrer
    /// Bedingungen **allein durch verstreichende Zeit** umschlagen kann.
    ///
    /// Der Kampf springt von Ereignis zu Ereignis. Ohne diese Auskunft wuerde
    /// eine Bedingung wie „Restlaufzeit unter einem GCD" erst beim naechsten
    /// ohnehin anstehenden Ereignis bemerkt — und das kann Sekunden spaeter
    /// sein. Das war der Vorabfund aus Issue #53; hier wird er behoben.
    ///
    /// Die Voreinstellung `None` ist fuer zustandsabhaengige Rotationen richtig:
    /// Deren Bedingungen aendern sich nur, wenn ohnehin ein Ereignis eintritt.
    fn next_wakeup(&self, _fight: &Fight) -> Option<Millis> {
        None
    }
}

impl Decide for Rotation {
    fn decide(&self, fight: &Fight) -> Option<usize> {
        self(fight)
    }
}

/// Wie viele Ereignisse gleichzeitig offen sein koennen.
///
/// Ueberschlag fuer Schritt 1: je Zauber ein Weckruf aus der Abklingzeit, einer
/// aus der globalen Wartezeit, ein DoT-Tick, ein Verfall-Schritt — zu keinem
/// Zeitpunkt mehr als eine Handvoll. 256 ist grosszuegig bemessen und kostet
/// einmalig 4 KB. Der Wert ist wichtig, weil die Schlange bei Ueberlauf
/// **nachfordern** wuerde, und das waere eine Speicheranforderung im Schritt.
const EVENT_CAPACITY: usize = 256;

/// Wie lange die Phasen-Senkung aus ANCHOR gilt (GAME-DESIGN 4.2b).
///
/// **Vorlaeufig, nicht balanciert.** Die Frist ist Katalog-Inhalt und steht in
/// GAME-DESIGN F.6 als „vor E10 auszuplanen". Sie traegt die Prueffaelle und
/// sonst nichts.
///
/// Sie steht hier und nicht in den Zauber-Daten, weil sie eine Eigenschaft der
/// WIRKUNG ist und nicht des Zaubers: Jeder kuenftige Zauber mit `holdPhase`
/// soll dieselbe Frist haben, sonst waere „hinauszoegern" bei jedem etwas
/// anderes.
/// Oeffentlich, damit Prueffaelle gegen sie rechnen statt 8000 zu verdrahten:
/// Eine Aenderung der Frist muss die Faelle mitnehmen, nicht stillschweigend
/// an ihnen vorbeilaufen.
pub const PHASE_HOLD_DURATION_MS: Millis = 8_000;

// -- PUTREFACTION (Issue #97) --
//
// **Alle drei Werte sind vorlaeufig und werden gemessen** (GAME-DESIGN 4.1,
// Teil F.6). Das Spiel fuehrt dieselben drei in `combat.ts`; die Pruefsumme je
// Takt haelt beide Seiten zusammen.

/// Wie viele Stapel hoechstens gleichzeitig liegen. Wird gemessen.
pub const MAX_DECAY_STACKS: usize = 5;

/// Wie viele Stapel die **Struktur** fasst — Feldlaenge und Pruefsummen-Schleife
/// (PUTRID BLOOM, Issue #479).
///
/// `MAX_DECAY_STACKS` ist die GRENZE, ab der kein Zauber mehr nachlegt; PUTRID
/// BLOOM hebt sie fuer eine Weile. Waere sie zugleich die Feldlaenge, aenderte
/// sich mit ihr die Zahl der Werte in der Pruefsumme — und eine Pruefsumme,
/// deren Laenge vom Spielverlauf abhaengt, ist keine mehr.
///
/// Muss mit `DECAY_STACK_CAPACITY` im Spiel uebereinstimmen (`combat.ts`).
pub const DECAY_STACK_CAPACITY: usize = 8;

/// Um wie viel Prozent ein offenes Synergie-Fenster den Schaden erhoeht
/// (GAME-DESIGN 5 und 1193, #318).
///
/// **Gesetzt, nicht gemessen** — wie `DECAY_STACK_PERCENT`. GAME-DESIGN regelt
/// Fenster und Stapel-Regel und schweigt zur Hoehe; die bestimmt der
/// Balance-Laeufer. Der Wert steht WOERTLICH gleich im Spiel
/// (`SYNERGY_BONUS_PERCENT` in combat.ts) — laufen die beiden auseinander,
/// meldet es die Pruefsumme ab Takt 0.
pub const SYNERGY_BONUS_PERCENT: u32 = 25;

/// Um wie viel Prozent ein Stapel den Schaden aller Quellen erhoeht.
///
/// **Entschieden, nicht offen** — Issue #224, 2026-08-05. Bis Issue #482 stand
/// hier 3 mit dem Vermerk "Wird gemessen"; die Messung hatte laengst
/// stattgefunden, nur die Umsetzung fehlte.
///
/// GAME-DESIGN 221 rechnet vor: Der Break-even, ab dem sich der Platz auf der
/// Leiste rechnet, liegt bei 5,4 % je Stapel. Mit 3 % waere PUTREFACTION nie
/// die beste Wahl.
///
/// Spiegelbild zu `DECAY_STACK_PERCENT` in `src/core/combat.ts`.
pub const DECAY_STACK_PERCENT: u32 = 6;

/// Wie lange ein einzelner Stapel haelt. Wird gemessen.
pub const DECAY_STACK_DURATION_MS: Millis = 12_000;

/// Einstellungen eines Durchgangs.
#[derive(Debug, Clone, Copy)]
pub struct FightConfig {
    /// Startwert der Zufallsstroeme.
    ///
    /// 32 Bit, weil das Spiel 32 Bit fuehrt (`src/core/rng.ts`). Ein breiterer
    /// Startwert liesse sich hier annehmen und waere im Spiel nicht darstellbar —
    /// aufgezeichnete Runden waeren dann nicht mehr uebertragbar.
    pub seed: u32,
    /// Kampfdauer. `None` heisst: der Ueberdruck-Timer des Bosses.
    pub duration_ms: Option<Millis>,
    /// Ob der Kampf endet, sobald der Boss faellt.
    ///
    /// `false` wird fuer den Vergleich zweier Rotationen ueber die volle
    /// Kampfzeit gebraucht (`STEP1-SPEC` § 4) — sonst rechnet die bessere
    /// Rotation kuerzer und sieht dadurch schlechter aus, als sie ist.
    pub stop_on_kill: bool,
    /// Wie viele Protokoll-Eintraege aufbewahrt werden. 0 schaltet das
    /// Protokoll ab.
    pub log_capacity: usize,
    /// Die Faktoren der Schadensformel (GAME-DESIGN 6.3).
    ///
    /// In Schritt 1 durchweg neutral (`STEP1-SPEC` § 2). Der Balance-Laeufer
    /// veraendert je einen davon, um die Empfindlichkeit zu messen.
    pub factors: DamageFactors,
    /// Gegen welchen Boss gerechnet wird.
    ///
    /// Voreinstellung ist der Boss aus `data/bosses/`, der als Standard
    /// gekennzeichnet ist. Mit 68 Bossen (GAME-DESIGN 11.7) wird das der
    /// haeufigste Schalter des Balance-Laeufers.
    pub boss: &'static Boss,
    /// Die getragenen Sonderwerte (GAME-DESIGN 6.7, Issue #177).
    ///
    /// **Voreinstellung ist [`WornValues::none`]**, und daran haengt die
    /// Zusicherung aus AC7: Solange nichts getragen wird, laeuft der Kampf
    /// durch dieselben Zweige wie vorher — keine Zeit wird umgerechnet, kein
    /// Zufallsstrom gedreht, keine Pruefsumme bewegt.
    pub worn: WornValues,
    /// Ob das Schluesseltalent "split-spender" verteilt ist (Issue #418).
    ///
    /// **Ein Wahrheitswert, keine Zahl.** Die Talente kommen sonst nur als
    /// fertiger Faktor herein (`factors.talents`) — das reicht fuer die drei
    /// Wege aus GAME-DESIGN 12.5, die alle Zahlen erhoehen. Ein
    /// Schluesseltalent aendert stattdessen, WAS geschieht, und laesst sich
    /// deshalb nicht als Faktor uebergeben.
    ///
    /// Dieselbe Bauart wie `worn`: ein Datenfeld, das durch Spiel, Simulator,
    /// Aufzeichnung und Wiedergabe geht. Die Alternative — eine abgewandelte
    /// Zauberliste — geht auf dieser Seite nicht: `SPELLS` ist statisch und
    /// wird an vierzig Stellen direkt gelesen.
    pub split_spender: bool,
    /// Ob der Satz-Effekt "ashen-regimen" wirkt (13.5, Issue #480).
    ///
    /// Die sechste Satz-Stufe traegt keinen Werte-Bonus, sondern eine Kennung,
    /// auf die der Kampf reagiert — dieselbe Bauart wie ein Schluesseltalent,
    /// und deshalb dasselbe Feld daneben. Er laesst PYROCLASM ein Drittel der
    /// angesammelten Hitze stehen, statt sie vollstaendig zu verbrauchen.
    ///
    /// **Als Wahrheitswert und nicht als Kennungsliste**: Der Simulator laedt
    /// `data/sets.json` nicht; welche Effekte ein Aufbau ausloest, rechnet das
    /// Spiel und reicht das Ergebnis herein — wie bei `worn`.
    pub ashen_regimen: bool,
    /// Ob der Satz-Effekt "whole-work" wirkt (13.5, Issue #480).
    ///
    /// Der globale Cooldown sinkt um [`WHOLE_WORK_GCD_REDUCTION_MS`]. Er ist
    /// der einzige der sechs ohne Schulbindung — er wirkt auf jede
    /// Zusammenstellung gleich.
    pub whole_work: bool,
    /// Ob der Satz-Effekt "slow-rot" wirkt (13.5, Issue #480).
    ///
    /// Zersetzungs-Stapel verfallen nicht, solange ein DoT laeuft.
    pub slow_rot: bool,
    /// Ob der Satz-Effekt "sealed-vessel" wirkt (13.5, Issue #480).
    ///
    /// Ueberschuessiger Schutz ueber der Stapel-Grenze wird zu Schaden.
    pub sealed_vessel: bool,
    /// Ob der Satz-Effekt "long-grip" wirkt (13.5, Issue #480).
    ///
    /// Der erste Kontroll-Ausgeber nach jedem Phasenwechsel kostet nichts —
    /// der ABZUG entfaellt, die Voraussetzung bleibt.
    pub long_grip: bool,
    /// Die verteilten Talente als Promille-Summe (Issue #417).
    ///
    /// **Als fertige Summe und nicht als Verteilung**, anders als im Spiel: Der
    /// Simulator kennt keine Talent-Kennungen und braucht sie nicht — was er
    /// braucht, ist die Zahl, die daraus folgt. Eine zweite Nachbildung der
    /// Baumstruktur waere eine zweite Stelle, an der beide Seiten auseinander
    /// laufen koennen.
    ///
    /// **Voreinstellung 0**, also Faktor 1,0 — dieselbe Zusicherung wie bei
    /// `worn`: Ohne Talente laeuft der Kampf durch dieselben Zweige wie vorher,
    /// und keine Pruefsumme bewegt sich.
    pub talent_permille: u32,
    /// Promille-Summe der Talente auf den **Ressourcen-Gewinn** (Issue #419).
    ///
    /// Die zweite von drei Talent-Groessen. Wie `talent_permille` kommt die
    /// Summe von aussen: Der Baum steht im Spiel, und eine zweite Nachbildung
    /// waere eine zweite Fehlerquelle.
    ///
    /// **Voreinstellung 0**, also Faktor 1,0 — ohne Talente laeuft der Kampf
    /// durch dieselben Zweige wie vorher, und keine Pruefsumme bewegt sich.
    pub talent_resource_permille: u32,
    /// Promille-Summe der Talente auf die **Laufzeit** der Brand-Effekte.
    ///
    /// Sie verlaengert die DAUER, nicht den Abstand: Ein laengerer Brand tickt
    /// OEFTER, er tickt nicht langsamer.
    pub talent_dot_duration_permille: u32,
    /// Promille auf den Aufbau der Kontroll-Punkte (FIXATION, #419).
    /// Die Wirkstelle steht seit GRIPLOCK; dies ist die Talent-Speisung.
    pub talent_control_gain_permille: u32,
    /// Promille auf die Wertigkeit eines Faeulnis-Stapels (PUTREFACTION).
    pub talent_decay_effect_permille: u32,
    /// Promille je Rang auf den Preis der Kontroll-Wirkungen (FIXATION, #419).
    /// **Er TEILT** — siehe `talent_control_cost_factor`.
    pub talent_control_cost_permille: u32,
    /// Promille auf die Staerke der Kontroll-Wirkungen (FIXATION).
    pub talent_control_effect_permille: u32,
    /// Promille auf die Zahl gelegter Faeulnis-Stapel (PUTREFACTION).
    pub talent_decay_stack_permille: u32,
    /// Promille auf die Haltbarkeit der Faeulnis-Stapel (PUTREFACTION).
    pub talent_decay_duration_permille: u32,
    /// Promille auf die Dicke der Schutzschicht (COAGULATION).
    pub talent_shield_amount_permille: u32,
    /// Promille auf die Dauer der Schutzschicht (COAGULATION).
    pub talent_shield_duration_permille: u32,
    /// ZUSCHLAG auf die Stapel-Grenze der Schutzschicht (COAGULATION).
    ///
    /// **Eine ganze Zahl, kein Promille-Wert** — als einziger der sieben. Ein
    /// Faktor auf `max_stacks: 3` ergaebe Bruchteile, und eine halbe
    /// Stapel-Erlaubnis gibt es nicht.
    pub talent_shield_stacks_bonus: u32,
    /// Promille auf den Abzug an Abklingzeit (SUBLIMATION).
    pub talent_cooldown_reduction_permille: u32,
    /// ZUSCHLAG auf die Ruecksetz-Ladungen (SUBLIMATION).
    ///
    /// **Eine ganze Zahl**, zweite dieser Art nach `talent_shield_stacks_bonus`
    /// und aus demselben Grund: Ladungen sind zaehlbar, halbe gibt es nicht.
    pub talent_cooldown_charges_bonus: u32,
    /// Promille auf die Schwelle, ab der der Abzug greift (SUBLIMATION).
    pub talent_cooldown_threshold_permille: u32,
    /// Stufe des Spielers (Issue #248).
    ///
    /// Sie wirkt auf die Grundwerte: Ein Kampf auf Stufe 50 hat andere
    /// Lebenspunkte als einer auf Stufe 1. Kennte der Simulator sie nicht,
    /// rechnete er ab E10 gegen einen anderen Spieler als das Spiel.
    ///
    /// **Nicht die Erfahrung.** Wie lange jemand fuer diese Stufe gebraucht
    /// hat, beruehrt keinen Takt.
    pub player_level: u32,
}

impl Default for FightConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            duration_ms: None,
            stop_on_kill: true,
            log_capacity: 4096,
            factors: NEUTRAL,
            boss: BOSS,
            worn: WornValues::none(),
            split_spender: false,
            ashen_regimen: false,
            whole_work: false,
            slow_rot: false,
            sealed_vessel: false,
            long_grip: false,
            talent_permille: 0,
            talent_resource_permille: 0,
            talent_dot_duration_permille: 0,
            talent_control_gain_permille: 0,
            talent_decay_effect_permille: 0,
            talent_control_cost_permille: 0,
            talent_control_effect_permille: 0,
            talent_decay_stack_permille: 0,
            talent_decay_duration_permille: 0,
            talent_shield_amount_permille: 0,
            talent_shield_duration_permille: 0,
            talent_shield_stacks_bonus: 0,
            talent_cooldown_reduction_permille: 0,
            talent_cooldown_charges_bonus: 0,
            talent_cooldown_threshold_permille: 0,
            player_level: 1,
        }
    }
}

/// Das Ergebnis eines Durchgangs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    /// Gesamtschaden.
    pub damage: u32,
    /// Kampfzeit des Sieges, oder `None`, wenn der Boss ueberlebt hat.
    pub killed_at_ms: Option<Millis>,
    /// Verbleibende Zeit bis zum Ueberdruck, wenn der Boss gefallen ist.
    pub remaining_ms: Millis,
    /// Wirkungen je Zauber.
    pub casts: [u32; SPELL_COUNT],
    /// Schaden je Zauber.
    pub damage_by_spell: [u32; SPELL_COUNT],
    /// Durch zu fruehes Erneuern verfallene Ticks, je Zauber.
    pub lost_dot_ticks: [u32; SPELL_COUNT],
    /// **Abgegebene** Ticks je Zauber (Issue #177).
    ///
    /// Der Kampf fuehrt diese Zahl seit E3-1b mit und gab sie bislang nicht
    /// heraus — sie ging nur in die Zustands-Pruefsumme ein. Mit CATALYSIS wird
    /// sie zur Messgroesse: „Die Dauer bleibt, die Ticks werden mehr" (6.7) ist
    /// ueber den Schaden allein nicht pruefbar, weil dort der Krit mitspielt.
    pub dot_tick_counts: [u32; SPELL_COUNT],
    /// Am Deckel verpuffte Ressource.
    pub resource_overflow: u32,
    /// Zeit, in der gewartet wurde, obwohl gewirkt werden durfte.
    pub idle_ms: Millis,
    /// Ziehungen aus den Zufallsstroemen. In Schritt 1 muss das 0 sein.
    pub random_draws: u32,
}

/// Ein Kampf-Durchgang.
///
/// # `Debug` ist hier ein WERKZEUG, keine Bequemlichkeit (Issue #349)
///
/// Rust hat keine Reflexion — es gibt keinen Weg, ueber die Felder zu laufen,
/// wie `Object.keys` es auf der Spielseite tut. `#[derive(Debug)]` ist der
/// naechstbeste: Der Uebersetzer schreibt die Auflistung, und sie enthaelt
/// **jedes** Feld, auch das von morgen.
///
/// `tests/reset_completeness.rs` vergleicht damit einen zurueckgesetzten
/// gegen einen frischen `Fight`. Ohne diese Ableitung braeuchte der Waechter
/// eine von Hand gepflegte Liste — und wer ein Feld hinzufuegt und die Liste
/// vergisst, hat genau den Fehler gebaut, den sie verhindern soll.
///
/// Es kostet zur Laufzeit nichts: Die Auflistung entsteht nur, wenn sie
/// jemand anfordert, und das tut ausser dem Waechter niemand.
#[derive(Debug)]
pub struct Fight {
    // -- Zeit --
    time: Millis,
    duration_ms: Millis,
    stop_on_kill: bool,

    // -- Boss --
    boss: &'static Boss,
    damage: u32,
    /// Wie viel Ring der SPIELER heruntergedrueckt hat (Issue #256, AC3).
    ///
    /// `damage` mischt seit #247 zwei Herkuenfte: Die davonlaufende Reaktion
    /// senkt den Ring von selbst. Gemessen gegen `test-runaway` steht bei
    /// einem Lauf ohne einen einzigen Tastendruck `damage = 1020`, davon
    /// null vom Spieler — die Fehleranalyse meldete ihm sonst 1020 Punkte
    /// als seinen Fehler.
    ///
    /// Nur was ankommt: Schaden im Unverwundbarkeits-Fenster senkt den Ring
    /// nicht und steht in `damage_lost_to_race`.
    player_damage: u32,
    killed_at: Option<Millis>,
    /// Welche Phase gerade laeuft. 0 ist der Start.
    phase: usize,
    /// Stand der Energieleiste — der zweiten Uhr.
    energy: u32,
    /// Wie oft die Energieleiste ausgeloest hat.
    energy_triggers: u32,
    /// Stand des toedlichen Zaehlers (Issue #216).
    doom: u32,
    /// Verbleibende Anwendungen der Ruecksetz-Handlung.
    doom_charges: u32,
    /// Wann die naechste verbrauchte Senkungs-Ladung zurueckkommt (#561).
    ///
    /// `None` heisst: keine wartet. Wird beim VERBRAUCH gesetzt, nicht beim
    /// Aufbau — ein Kampf ohne Senkung hat damit denselben Verlauf wie vorher.
    next_doom_charge_at: Option<Millis>,
    /// Zahl der exponentiellen Stapel (Issue #216).
    stacks: u32,
    /// Kampfzeit, zu der das Unverwundbarkeits-Fenster endet (Issue #218).
    race_ends_at: Option<Millis>,
    /// Ob das Fenster schon gelaufen ist — es beginnt genau einmal.
    race_done: bool,
    /// Wie weit das Ziel geladen ist.
    race_progress: u32,
    /// Ob das Ziel zerstoert wurde, statt dass die Zeit ablief.
    race_won: bool,
    /// Schaden, der im Fenster verpuffte. Verloren, nicht ausgeteilt.
    damage_lost_to_race: u32,
    /// Wie viel Fortschritt der Boss zurueckgedreht hat.
    boss_healed: u32,
    /// Wie viel Fortschritt der Spieler zurueckgedraengt hat (#247).
    stabilised: u32,
    /// Ob die Schwaechung gerade liegt (Baustein 8, Issue #240).
    afflicted: bool,
    /// Wie oft sie entfernt wurde.
    ///
    /// Traegt den Unterschied zwischen "nie gelegen" und "gelegen und
    /// entfernt": Im Zustand sehen die beiden gleich aus. Sie ist ausserdem
    /// die Bedingung dafuer, dass die Schwaechung **einmal** greift und nicht
    /// nach jedem Entfernen erneut.
    afflictions_cleared: u32,
    /// Wie weit die Boss-Zeitleiste abgearbeitet ist.
    ///
    /// Zeigt auf den **naechsten** faelligen Eintrag. Steht er am Ende der
    /// Liste, ist die Zeitleiste durch.
    timeline_index: usize,
    /// Welcher Boss-Zauber gerade laeuft, als Index in `Boss::timeline`.
    ///
    /// `None` heisst: keiner. Solange einer laeuft, kann er unterbrochen
    /// werden — danach nicht mehr, auch nicht im Takt des Treffers.
    casting: Option<usize>,
    /// Der Widerstands-Faktor je Zauber (Issue #328).
    ///
    /// Vorgerechnet beim Aufbau: Der Boss wechselt zwischen den Kaempfen, nicht
    /// in ihnen. Eine Suche je Wirkung waere eine Abbildung im Schritt
    /// (GAME-DESIGN 18.4).
    resistance_by_spell: [f64; SPELL_COUNT],
    /// Welcher Zauber des SPIELERS gerade gewirkt wird (Issue #304).
    ///
    /// `None` heisst: keiner. Bis #304 gab es diesen Zustand nicht — `casting`
    /// eine Zeile darueber gehoert dem Boss, und `cast_time_ms` am
    /// Spieler-Zauber wurde eingelesen und nie gelesen.
    player_casting: Option<usize>,
    /// Wann die laufende Wirkzeit des Spielers endet.
    ///
    /// Ohne laufende Wirkzeit 0 und nicht -1: `Millis` ist vorzeichenlos.
    /// Welcher Ruhewert hier steht, ist gleichgueltig — die Pruefsumme liest
    /// ihn nur, solange `player_casting` gesetzt ist, genau wie bei
    /// `cast_lands_at`.
    player_cast_ends_at: Millis,
    /// Welcher Wirkvorgang gemeint ist — gegen veraltete Weckrufe in der
    /// Schlange, dieselbe Vorsichtsmassnahme wie `dot_generation`.
    player_cast_generation: u16,
    /// Wie oft ein Boss-Angriff eine Wirkzeit abgebrochen hat.
    casts_interrupted: u32,
    /// Wie oft ein eigener Druck eine laufende Wirkzeit abgebrochen hat.
    ///
    /// **Getrennt vom Zaehler des Boss-Abbruchs**, obwohl beide dasselbe Feld
    /// raeumen: Das eine hat der Boss getan, das andere der Spieler selbst.
    casts_cancelled: u32,
    /// Wie oft ein Boss-Zauber durchgekommen ist.
    interrupts_missed: u32,
    /// Wie oft der Spieler einen unterbrochen hat.
    interrupts_landed: u32,

    // -- Spieler --
    /// Volle Lebenspunkte auf der Stufe dieses Laufs (Issue #248).
    ///
    /// **Einmal gerechnet, dreimal gebraucht**: Startwert, Zuruecksetzen und
    /// die Obergrenze der Heilung. Die Rechnung an drei Stellen zu wiederholen
    /// waere dieselbe Formel an drei Orten, und eine davon liefe irgendwann
    /// weg.
    vessel_max: u32,
    /// Verbleibende Lebenspunkte. Beginnt bei `vessel_max`.
    player_hp: u32,
    /// Kampfzeit des Spielertods, oder `None`.
    ///
    /// **Keine Strafe** (GAME-DESIGN A.3): Der Versuch ist vorbei, die
    /// Auswertung erscheint wie bei jedem anderen Ende, und man startet neu.
    died_at: Option<Millis>,
    /// Gesamter erlittener Schaden, **nach** WARD.
    ///
    /// Zaehlt, was den Spieler wirklich getroffen hat — was die Schutzschicht
    /// abgefangen hat, steht in `shield_absorbed`.
    damage_taken: u32,

    // -- COAGULATION (Issue #79) --
    /// Stand der Schutzschicht: wie viel Schaden sie noch aufnehmen kann.
    shield: u32,
    /// Wann sie verfaellt, wenn sie nicht vorher aufgebraucht wird.
    shield_expires_at: Option<Millis>,
    /// Wie viele Anwendungen gerade uebereinanderliegen.
    shield_stacks: u32,
    /// Wie viel Schaden die Schutzschicht insgesamt abgefangen hat.
    shield_absorbed: u32,
    /// Wie viel Leben insgesamt zurueckgeholt wurde.
    healed: u32,
    /// Heilung ueber Zeit (POULTICE, Issue #481) — Gesamtbetrag, Schrittzahl,
    /// Abstand, getane Schritte, schon gutgeschriebener Betrag, Anwendung.
    ///
    /// **Kumulativ statt „Betrag je Schritt".** `amount * getan / schritte`,
    /// ganzzahlig geteilt, minus dem schon Gutgeschriebenen — so bleibt die
    /// Summe exakt `amount` und jeder Schritt ganzzahlig, auch wenn der Betrag
    /// nicht glatt durch die Schrittzahl geht. Ein vorab gerechneter Betrag je
    /// Schritt waere bei 1800/6 zufaellig glatt und beim naechsten Zauber eine
    /// Fliesskommazahl, die beide Seiten unabhaengig runden muessten
    /// (GAME-DESIGN 18.4).
    heal_over_time_amount: u32,
    heal_over_time_steps: u32,
    heal_over_time_interval_ms: Millis,
    heal_over_time_done: u32,
    heal_over_time_credited: u32,
    heal_over_time_generation: u16,
    /// Bis wann die Schaden-in-Schutz-Umwandlung laeuft (Weg 3).
    conversion_until: Option<Millis>,
    /// Bis wann DECAYs Schwaechung laeuft (Issue #479).
    ///
    /// Dieselbe Bauart wie `conversion_until` (TEMPER): ein Zustand fuer eine
    /// Weile. Er steht in keiner Pruefsumme und muss es nicht — seine Wirkung
    /// zeigt sich im Leben des Spielers, und das mischt.
    decay_ward_until: Option<Millis>,
    /// Promille je Stapel, solange sie laeuft.
    decay_ward_permille: u32,
    /// Anteil, den sie umwandelt, in Prozent. 0 heisst: keine laeuft.
    conversion_percent: u32,
    /// Wie viel Schutz die Umwandlung insgesamt erzeugt hat.
    converted: u32,
    /// Der Rueckschlag (RIPOSTE, Issue #481) — bis wann er laeuft, sein Anteil,
    /// und was er insgesamt zurueckgegeben hat.
    ///
    /// Getrennt von der Umwandlung, obwohl beide zu Weg 3 gehoeren: Sie haengen
    /// an entgegengesetzten Enden (ausgeteilter Schaden gegen eingehenden).
    reflect_until: Option<Millis>,
    reflect_percent: u32,
    reflected: u32,
    /// Welchem Zauber der zurueckgegebene Schaden gehoert.
    ///
    /// **Er braucht einen Traeger, keinen Platzhalter.** Der erste Entwurf
    /// buchte ihn auf `usize::MAX` — das haette `damage_by_spell[index]` zur
    /// Laufzeit ueber die Grenze geschickt. Der Bau war gruen; gefunden wurde
    /// es beim Lesen von `deal`.
    reflect_spell: usize,

    // -- FIXATION (Issue #100) --
    /// Vorrat an Kontroll-Punkten.
    ///
    /// **Zum Ausgeben, nicht zum Sammeln** (GAME-DESIGN 4.1): Sie verfallen
    /// nicht wie die Hitze, sondern warten darauf, dass der Spieler sich
    /// entscheidet. Das ist der Unterschied zwischen einem Planungswerkzeug
    /// und einer Ressource, die einen Rhythmus vorgibt.
    control: u32,
    /// Schaden aus umgewandelten Kontroll-Punkten, gemerkt bis zur Rechnung
    /// (Issue #478, WEDGE).
    ///
    /// **Gemerkt und nicht sofort ausgeteilt:** Die Kontroll-Wirkung laeuft
    /// vor der Schadensrechnung, dort stehen die Faktoren noch nicht. Ein
    /// Treffer an dieser Stelle ginge an Krit, Widerstand und Verstaerkung
    /// vorbei — er waere der einzige Schaden im Spiel, den kein Faktor
    /// beruehrt.
    pending_control_damage: u32,
    /// Ablaufzeiten der Zersetzungs-Stapel, **aufsteigend** — aeltester vorn.
    ///
    /// **Jeder Stapel hat seine eigene Uhr** (GAME-DESIGN 4.1). Feld fester
    /// Laenge, kein Vec: Der Kampf fordert keinen Speicher an (18.2).
    // **Die KAPAZITAET, nicht die Grenze** (PUTRID BLOOM, #479).
    decay_expiry: [Millis; DECAY_STACK_CAPACITY],
    /// Um wie viele Plaetze die Grenze angehoben ist. 0 = keine Anhebung.
    decay_cap_bonus: usize,
    /// Bis wann die Anhebung gilt.
    decay_cap_until: Option<Millis>,
    /// Wie viele Stapel gerade liegen.
    decay_count: usize,
    /// Bis wann das Synergie-Fenster dieses Zaubers offen steht (#318).
    ///
    /// Index ist der **Partner**, nicht der Ausloeser: Gefragt wird beim
    /// Wirken "steht MIR ein Fenster offen".
    ///
    /// **0 heisst: kein Fenster.** Dieselbe Bauart wie `decay_expiry`, und
    /// zulaessig, weil ein Fenster nie bei 0 endet — es steht immer mindestens
    /// eine Wartezeit lang offen.
    synergy_window_ends: [Millis; SPELL_COUNT],
    synergy_used: u32,
    synergy_available: u32,
    /// Wie viele Punkte insgesamt ausgegeben wurden.
    control_spent: u32,
    /// Um wie viel der **naechste** Zeitleisten-Eintrag nach hinten geschoben
    /// ist (STAY, #100).
    ///
    /// **Nur der eine, nicht die folgenden** — Entscheidung des Auftraggebers
    /// vom 2026-08-04. Der Wortlaut in GAME-DESIGN 4.1 sagt „einen Schlag
    /// verzoegern", Singular.
    ///
    /// Der Unterschied ist nicht kosmetisch: Schlaegt der Boss bei 10, 17 und
    /// 24 s und der Spieler verzoegert den ersten um 3 s, ergibt das 13, 17,
    /// 24 — der Abstand zum zweiten schrumpft von sieben auf vier Sekunden.
    /// Verzoegern verschafft Luft und **erkauft sie mit Gedraenge danach**.
    ///
    /// Die verworfene Fassung haette alle folgenden mitgeschoben (13, 20, 27).
    /// Mehrfach eingesetzt haette sie den Boss immer weiter nach hinten
    /// gedraengt, ohne dass es etwas kostet ausser Punkten — ein anderer
    /// Zauber.
    next_strike_delay: Millis,
    /// Wie lange der NAECHSTE Boss-Zauber unterbrechbar ist (PIN, Issue #478).
    ///
    /// `0` heisst "kein Merker". Der Wert ist eine FENSTERLAENGE: Ein `Strike`
    /// traegt keine Wirkzeit, also gibt PIN ihm eine — sonst waere er
    /// unterbrechbar auf dem Papier und nie in der Hand.
    ///
    /// Dieselbe Bauart wie `next_strike_delay` (STAY): beim Ausloesen des
    /// naechsten Eintrags gelesen und verbraucht, nicht waehrend eines
    /// laufenden.
    pin_window: Millis,
    /// Um wie viele Prozentpunkte die Phasen-Schwelle gesenkt ist (ANCHOR).
    phase_hold_percent: u32,
    /// Bis wann die Senkung gilt (GAME-DESIGN 4.2b).
    ///
    /// **„Hinauszoegern" heisst befristet, nicht dauerhaft.** Sie endet, was
    /// zuerst eintritt: Die Frist laeuft ab, oder der naechste Wechsel hat
    /// stattgefunden — dann ist sie verbraucht.
    ///
    /// Eine Senkung, die den ganzen Kampf haelt, verschoebe bei einem Boss mit
    /// zwei Schwellen **beide**. Ein einziger Druck zu Kampfbeginn waere die
    /// beste Spielweise, und danach gaebe es nichts mehr zu entscheiden — das
    /// widerspraeche „Planungswerkzeug, keine passive Wirkung".
    phase_hold_until: Option<Millis>,
    /// Wann der laufende Boss-Zauber trifft.
    ///
    /// Wird mitgefuehrt, weil STALL ihn **verschiebt** (FIXATION, #100): Ohne
    /// diesen Wert liesse sich nicht sagen, wohin.
    cast_lands_at: Millis,
    /// Wie oft eine Wirkung mangels Punkten ins Leere lief.
    ///
    /// **Kein Fehler, sondern eine vertane Wirkung.** Reichen die Punkte
    /// nicht, geschieht nichts — der Zauber nimmt sich nicht, was er nicht hat.
    control_failed: u32,

    // -- Spieler --
    gcd_ready_at: Millis,
    /// **Der erste MERKER des Datenmodells** (SUBLIME, Issue #480).
    ///
    /// Steht auf `true`, wenn ein Zauber mit `frees_next_gcd` geworfen wurde
    /// und der nächste deshalb keine globale Wartezeit auslöst. Er lebt
    /// genau bis zu diesem nächsten Wurf.
    ///
    /// # Warum er in der Prüfsumme steht
    ///
    /// Ein Kampfzustand, der nicht in der Prüfsumme steht, ist von der
    /// Parität **nicht erfasst**: Beide Seiten könnten ihn verschieden
    /// führen — einer verbraucht ihn beim nächsten Wurf, der andere beim
    /// übernächsten —, und keine Prüfung würde es bemerken. Die Zahlen
    /// stimmten weiter, bis irgendwann eine Rotation auseinanderliefe.
    ///
    /// Das gilt für jeden Merker, der noch kommt (CONDENSE, RETORT,
    /// PUTRID BLOOM, DECAY). Wer einen baut, hängt ihn dort an.
    next_cast_free_gcd: bool,
    /// **Der zweite MERKER** (CONDENSE, Issue #480).
    ///
    /// Steht auf `true`, wenn ein Zauber mit `doubles_next_cast` geworfen
    /// wurde und der nächste deshalb zweimal wirkt. Er lebt genau bis zu
    /// diesem nächsten Wurf, wie sein Nachbar darüber — und aus demselben
    /// Grund steht er in der Prüfsumme.
    next_cast_doubles: bool,
    /// **Der dritte MERKER** (RETORT, Issue #480): Ist gerüstet.
    ///
    /// Steht auf `true`, wenn RETORT geworfen wurde und der nächste vom BOSS
    /// abgebrochene Wurf deshalb keine Wartezeit kostet.
    ///
    /// # Vorbereitung statt Rettung — bewusst anders als das Raster
    ///
    /// `docs/ZAUBER-RASTER.md` beschreibt ihn reaktiv: „ein unterbrochener
    /// Zauber ist sofort wieder bereit". Gebaut ist er präventiv, und das ist
    /// eine Entscheidung des Auftraggebers, keine Ungenauigkeit.
    ///
    /// Der Grund ist Design-Gesetz 6.6: Es lässt **genau eine** Ausnahme von
    /// der globalen Wartezeit zu, und die heisst Unterbrechen. Reaktiv gebaut
    /// bräuchte RETORT dieselbe Ausnahme — er müsste während der Wartezeit
    /// werfbar sein, die er verkürzen soll. Gemessen: mit `triggersGcd: true`
    /// wurde sein Druck abgelehnt (1 angenommen, 1 abgelehnt, 0 Würfe).
    ///
    /// Präventiv gebaut löst er selbst eine Wartezeit aus wie jeder andere
    /// Zauber, und die Regel bleibt unberührt. Spielbar ist das wegen
    /// Design-Gesetz 6 (Determinismus): Der Spieler SIEHT die Unterbrechung
    /// kommen, also ist es eine Planungsaufgabe und kein Reflex.
    ///
    /// # Ein Ja/Nein, keine Zahl
    ///
    /// Zweimal rüsten ist einmal gerüstet — der Merker zählt nicht hoch. Beim
    /// Rüsten ist der abgebrochene Wurf noch gar nicht gelaufen; es gibt also
    /// keinen Betrag, den man sich merken könnte.
    ///
    /// Steht aus demselben Grund in der Prüfsumme wie seine zwei Nachbarn.
    retort_armed: bool,
    /// Der Auswahlmodus des Ruecksetzers (Issue #223).
    ///
    /// Bis heute war er gebaut und unbenutzt — `cooldown_reset.rs` stand
    /// vollstaendig da, und kein Aufruf im Kampf rief ihn. Mit REFLUX (#201)
    /// gibt es einen Zauber, der ihn scharf stellt.
    reset_mode: CooldownReset,
    cooldown_ready_at: [Millis; SPELL_COUNT],
    resource: u16,
    dot_expires_at: [Option<Millis>; SPELL_COUNT],
    dot_generation: [u16; SPELL_COUNT],
    /// Die laufenden Verstaerkungen (GAME-DESIGN 5, Issue #420).
    buffs: Buffs,
    /// Verstaerkungen auf andere Groessen als Schaden (Issue #489).
    ///
    /// **Je Zielgroesse eine eigene Instanz.** Eine gemeinsame mit
    /// Arten-Bereichen waere eine Verabredung, die beim dritten Zauber
    /// bricht, ohne dass etwas rot wird. Getrennte Speicher machen die
    /// Verwechslung unmoeglich statt verboten.
    control_buffs: Buffs,
    resource_buffs: Buffs,
    /// Die beiden Talent-Faktoren, die ZUR LAUFZEIT gebraucht werden (#419).
    ///
    /// Anders als `talent_permille`, das einmal beim Aufbau in die
    /// Schadensfaktoren eingeht: Diese beiden wirken bei jedem
    /// Ressourcen-Gewinn und bei jedem gesetzten Brand — sie muessen also im
    /// Kampf stehen, nicht in der Konfiguration.
    talent_resource_factor: f64,
    talent_dot_duration_factor: f64,
    talent_control_gain_factor: f64,
    talent_decay_effect_factor: f64,
    talent_control_cost_factor: f64,
    talent_control_effect_factor: f64,
    talent_decay_stack_factor: f64,
    talent_decay_duration_factor: f64,
    talent_shield_amount_factor: f64,
    talent_shield_duration_factor: f64,
    talent_shield_stacks_bonus: u32,
    talent_cooldown_reduction_factor: f64,
    talent_cooldown_charges_bonus: u32,
    talent_cooldown_threshold_permille: u32,
    tick_buffs: Buffs,
    stack_buffs: Buffs,
    cd_reduce_buffs: Buffs,
    convert_buffs: Buffs,
    cap_buffs: Buffs,

    // -- Zaehlwerk --
    casts: [u32; SPELL_COUNT],
    damage_by_spell: [u32; SPELL_COUNT],
    lost_dot_ticks: [u32; SPELL_COUNT],
    dot_tick_counts: [u32; SPELL_COUNT],
    resource_overflow: u32,
    idle_ms: Millis,

    // -- Regeln --
    factors: DamageFactors,
    /// Die getragenen Sonderwerte (GAME-DESIGN 6.7).
    worn: WornValues,
    split_spender: bool,
    ashen_regimen: bool,
    whole_work: bool,
    slow_rot: bool,
    sealed_vessel: bool,
    long_grip: bool,
    /// Steht ein kostenloser Kontroll-Ausgeber bereit? (THE LONG GRIP)
    ///
    /// **Ein Ja/Nein, keine Zahl**: Zwei Phasenwechsel ohne Ausgeber dazwischen
    /// sind ein freier Ausgeber, nicht zwei.
    free_control_spend_armed: bool,
    /// Wie oft der Effekt gegriffen hat — fuer die Pruefung.
    free_control_spends: u32,

    // -- Werkzeuge --
    queue: EventQueue,
    log: CombatLog,
    rng: Streams,

    /// Was die Schul-Mechanik angesetzt, der Kampf aber noch nicht gebucht hat
    /// (Issue E3-2). Siehe [`crate::stacks`] fuer den Grund der Teilung.
    pending_gain: PendingGain,
    /// Wie oft Stapel angesetzt wurden.
    stack_gains: std::rc::Rc<std::cell::Cell<u32>>,
    /// Verkuerzung der Abklingzeiten, angesetzt und noch nicht ausgefuehrt
    /// (Issue E3-3). Siehe [`crate::sublimation`] fuer den Grund der Teilung.
    pending_shorten: PendingSlot,
    /// Wie oft die Sublimation angesetzt hat.
    sublimation_uses: std::rc::Rc<std::cell::Cell<u32>>,

    /// Die Regel-Ereignisse (GAME-DESIGN 18.3, Issue E3-1b).
    ///
    /// **Erst der Schaden, dann "Zauber gewirkt"** — Entscheidung des
    /// Auftraggebers vom 2026-07-29. Ein Empfaenger reagiert auf eine
    /// abgeschlossene Handlung und beeinflusst den naechsten Zauber, nicht den
    /// laufenden. Dieselbe Reihenfolge im Spiel (`src/core/combat.ts`).
    events: RuleEvents,
}

/// Der Widerstands-Faktor **je Zauber** (Issue #328).
///
/// # Am ZAUBER, nicht am Kampf — entschieden am 2026-08-05
///
/// Bis dahin war es eine einzelne Zahl: `boss.resistance_for(SCHOOL)`. Sie
/// multiplizierte jeden Spielerschaden gleich, und etwas, das alles gleich
/// skaliert, kann keine Reihenfolge aendern. Gemessen mit dem Rotations-Laeufer:
/// An einem Boss mit 0,75 und einem mit 1,25 kam **dieselbe** beste Liste
/// heraus, 23340 gegen 38900 — beides 31120 vor dem Faktor.
///
/// Die Aufloesung steht im SSOT: "Wer **trotz Widerstand** Calcination spielt,
/// sammelt weiter normal Hitze — macht damit aber weniger Schaden." Das setzt
/// voraus, dass jemand mit einer anderen Schule nicht weniger macht.
///
/// **Schulunabhaengige Zauber tragen 1,0** (GAME-DESIGN 7.4): Wer keiner Schule
/// angehoert, den kann keine Schule abhalten.
///
/// # Zwei Dinge, die hier zusammenkommen
///
/// | | |
/// | --- | --- |
/// | `regler` | der Faktor aus `BalanceConfig` — womit der Balance-Laeufer die Formel vermisst |
/// | Boss | der Widerstand aus den Boss-Daten, den auch das Spiel anwendet |
///
/// **Deshalb multipliziert und nicht gesetzt.** Das Spiel kennt keinen Regler.
/// Wuerde der Simulator ebenfalls setzen, naehme er dem Laeufer sein Werkzeug —
/// ausgerechnet dem, der belegen soll, was die Widerstaende bewirken.
///
/// # DIE BEDINGUNG, AUF DER DIE BITGLEICHHEIT RUHT
///
/// Beide Rechnungen stimmen ueberein, **solange eine der zwei Zahlen 1,0 ist**.
/// Sind es beide nicht, rechnet der Simulator `regler x boss` und das Spiel nur
/// `boss` — und kein Paritaets-Weg faehrt diese Kombination, also meldete sie
/// niemand.
///
/// Bis zum 2026-08-05 stand die Bedingung als KOMMENTAR da. Ein Satz, auf den
/// sich die Bitgleichheit stuetzt und den niemand prueft, ist dieselbe Klasse
/// wie #331 — nur mit einer anderen Zahl. Deshalb bricht dieser Aufruf ab,
/// statt still etwas zu rechnen, das dem Spiel nicht entspricht.
///
/// **Kein `debug_assert`**: Der Balance-Laeufer ist genau die Stelle, die den
/// Regler setzt, und er laeuft mit `--release`. Ein `debug_assert` schwiege
/// dort — also an der einzigen Stelle, an der die Kombination entstehen kann.
fn resistance_by_spell(regler: f64, boss: &Boss) -> [f64; SPELL_COUNT] {
    let mut tabelle = [1.0_f64; SPELL_COUNT];
    let mut i = 0;
    while i < SPELL_COUNT {
        let vom_boss = match SPELLS[i].school {
            Some(schule) => boss.resistance_for(schule),
            None => 1.0,
        };
        // **Vergleich auf die exakte Eins, mit Absicht.** 1,0 ist in IEEE754
        // exakt darstellbar, und gemeint ist genau der neutrale Wert — nicht
        // "ungefaehr neutral". Ein Regler von 1,0000001 IST eine Abweichung.
        #[allow(clippy::float_cmp)]
        let einer_ist_neutral = regler == 1.0 || vom_boss == 1.0;
        assert!(
            einer_ist_neutral,
            "Balance-Regler ({regler}) UND Boss-Widerstand ({vom_boss}) sind bei \
             Zauber \"{}\" beide gesetzt. Der Simulator rechnete dann {}, das \
             Spiel nur {vom_boss} — die Bitgleichheit waere hin, und kein \
             Paritaets-Weg wuerde es melden. Entweder den Regler auf 1,0 lassen \
             oder gegen einen Boss ohne Widerstand messen.",
            SPELLS[i].id,
            regler * vom_boss
        );
        tabelle[i] = regler * vom_boss;
        i += 1;
    }
    tabelle
}

impl Fight {
    /// Legt einen Durchgang an. Fordert allen benoetigten Speicher **jetzt** an.
    #[must_use]
    pub fn new(config: FightConfig) -> Self {
        // Einmal gerechnet, zweimal gebraucht — Startleben und Deckel sind
        // derselbe Wert, und getrennte Ausdruecke waeren zwei Stellen, die
        // auseinanderlaufen koennen.
        let vessel_max = vessel_with_gear(
            stat_at_level(PLAYER.vessel, config.player_level),
            config.worn.vessel,
        );
        let mut fight = Self {
            time: 0,
            duration_ms: config.duration_ms.unwrap_or(config.boss.overpressure_ms),
            stop_on_kill: config.stop_on_kill,
            boss: config.boss,
            damage: 0,
            player_damage: 0,
            killed_at: None,
            phase: 0,
            energy: 0,
            energy_triggers: 0,
            doom: 0,
            doom_charges: config.boss.doom.map_or(0, |d| d.clear_charges),
            next_doom_charge_at: None,
            stacks: 0,
            race_ends_at: None,
            race_done: false,
            race_progress: 0,
            race_won: false,
            damage_lost_to_race: 0,
            boss_healed: 0,
            stabilised: 0,
            afflicted: false,
            afflictions_cleared: 0,
            timeline_index: 0,
            casting: None,
            player_casting: None,
            player_cast_ends_at: 0,
            player_cast_generation: 0,
            casts_interrupted: 0,
            casts_cancelled: 0,
            interrupts_missed: 0,
            interrupts_landed: 0,
            // **Ausruestung kommt DAZU, nicht davor** (Issue #282, AC3).
            // Wer die Stufen-Basis ersetzt statt sie zu erhoehen, macht einen
            // Spieler auf Stufe 100 ohne Ausruestung schwaecher als einen auf
            // Stufe 1 mit einem Fund.
            //
            // Ohne getragene Ausruestung ist der Faktor exakt 1,0 und die
            // Multiplikation bewegt keine Zahl — belegt in AC5.
            vessel_max,
            // **Derselbe Wert wie `vessel_max`, nicht die Basis darunter.**
            //
            // Hier stand `stat_at_level(...)` — die Lebenspunkte OHNE
            // Ausruestung, waehrend `vessel_max` sie MIT bekam. Der Spieler
            // startete also verwundeter, als er sein durfte, und weil
            // `player_hp` in die Pruefsumme eingeht, brach der Gleichlauf in
            // Takt 0.
            //
            // Die beiden Zeilen sahen aus wie ein Paar und WAREN eins, solange
            // es keine Ausruestung gab. Auf der Spiel-Seite ist es eine
            // einzige Zeile (`playerHpValue = vesselMax`) — dort gab es beim
            // Uebertragen nichts zu vergessen. **Eine Asymmetrie, die auf
            // einer Seite zwei Zeilen ist und auf der anderen eine, faellt
            // beim Uebertragen durch.**
            player_hp: vessel_max,
            died_at: None,
            damage_taken: 0,
            shield: 0,
            shield_expires_at: None,
            buffs: Buffs::new(),
            control_buffs: Buffs::new(),
            resource_buffs: Buffs::new(),
            talent_resource_factor: 1.0 + f64::from(config.talent_resource_permille) / 1000.0,
            talent_dot_duration_factor: 1.0
                + f64::from(config.talent_dot_duration_permille) / 1000.0,
            talent_control_gain_factor: 1.0
                + f64::from(config.talent_control_gain_permille) / 1000.0,
            talent_decay_effect_factor: 1.0
                + f64::from(config.talent_decay_effect_permille) / 1000.0,
            talent_control_cost_factor: 1.0
                + f64::from(config.talent_control_cost_permille) / 1000.0,
            talent_control_effect_factor: 1.0
                + f64::from(config.talent_control_effect_permille) / 1000.0,
            talent_decay_stack_factor: 1.0 + f64::from(config.talent_decay_stack_permille) / 1000.0,
            talent_decay_duration_factor: 1.0
                + f64::from(config.talent_decay_duration_permille) / 1000.0,
            talent_shield_amount_factor: 1.0
                + f64::from(config.talent_shield_amount_permille) / 1000.0,
            talent_shield_duration_factor: 1.0
                + f64::from(config.talent_shield_duration_permille) / 1000.0,
            talent_shield_stacks_bonus: config.talent_shield_stacks_bonus,
            talent_cooldown_reduction_factor: 1.0
                + f64::from(config.talent_cooldown_reduction_permille) / 1000.0,
            talent_cooldown_charges_bonus: config.talent_cooldown_charges_bonus,
            // **Als Promille, nicht als Faktor** — anders als die acht darueber.
            // Der Wert wird an `SublimationMechanic::new` weitergereicht, und
            // die rechnet den Faktor selbst; zweimal umgerechnet driftete er.
            talent_cooldown_threshold_permille: config.talent_cooldown_threshold_permille,
            tick_buffs: Buffs::new(),
            stack_buffs: Buffs::new(),
            cd_reduce_buffs: Buffs::new(),
            convert_buffs: Buffs::new(),
            cap_buffs: Buffs::new(),
            shield_stacks: 0,
            shield_absorbed: 0,
            healed: 0,
            heal_over_time_amount: 0,
            heal_over_time_steps: 0,
            heal_over_time_interval_ms: 0,
            heal_over_time_done: 0,
            heal_over_time_credited: 0,
            heal_over_time_generation: 0,
            conversion_until: None,
            decay_ward_until: None,
            decay_ward_permille: 0,
            conversion_percent: 0,
            converted: 0,
            reflect_until: None,
            reflect_percent: 0,
            reflected: 0,
            reflect_spell: 0,
            control: 0,
            pending_control_damage: 0,
            next_strike_delay: 0,
            pin_window: 0,
            phase_hold_percent: 0,
            phase_hold_until: None,
            cast_lands_at: 0,
            control_spent: 0,
            decay_expiry: [0; DECAY_STACK_CAPACITY],
            decay_cap_bonus: 0,
            decay_cap_until: None,
            decay_count: 0,
            synergy_window_ends: [0; SPELL_COUNT],
            synergy_used: 0,
            synergy_available: 0,
            control_failed: 0,
            gcd_ready_at: 0,
            next_cast_free_gcd: false,
            next_cast_doubles: false,
            retort_armed: false,
            reset_mode: CooldownReset::new(),
            cooldown_ready_at: [0; SPELL_COUNT],
            resource: 0,
            dot_expires_at: [None; SPELL_COUNT],
            dot_generation: [0; SPELL_COUNT],
            casts: [0; SPELL_COUNT],
            damage_by_spell: [0; SPELL_COUNT],
            lost_dot_ticks: [0; SPELL_COUNT],
            dot_tick_counts: [0; SPELL_COUNT],
            resource_overflow: 0,
            idle_ms: 0,
            // **Topf 4 kommt beim Aufbau in die Faktoren**, nicht im Takt.
            //
            // Er aendert sich waehrend eines Kampfes nicht: Ausruestung wird
            // zwischen den Kaempfen gewechselt (13.1). Ihn je Treffer neu zu
            // rechnen waere Arbeit ohne Ergebnis — und `DamageFactors` traegt
            // ihn ohnehin als eigenes Feld (6.3).
            //
            // **Ohne getragene Ausruestung ist der Faktor exakt 1,0**, und 1,0
            // ist in IEEE754 die exakte Eins. Die Multiplikation in
            // `compute_damage` bewegt dann keine einzige Zahl — und
            // `is_neutral()` greift weiterhin, weil das Feld unveraendert bei
            // 1,0 steht.
            factors: DamageFactors {
                essence: config.factors.essence * config.worn.essence_factor(),
                gear_effects: config.factors.gear_effects * config.worn.gear_effect_factor(),
                // **Der Talent-Faktor** (Issue #417). Dieselbe Rechnung wie
                // `talentFactor` im Spiel: 1 + Promille / 1000, aus ganzen
                // Zahlen gebildet. Die Summe kommt von aussen, weil der Baum
                // dort steht und eine zweite Nachbildung eine zweite
                // Fehlerquelle waere.
                talents: config.factors.talents
                    * (1.0 + f64::from(config.talent_permille) / 1000.0),
                // **Der Widerstand des Bosses gegen die eigene Schule**
                // (GAME-DESIGN 11.4, Issue #328). Er kommt beim Aufbau in die
                // Faktoren, nicht im Schritt: Der Boss wechselt zwischen den
                // Kaempfen, nicht in ihnen — dieselbe Stelle wie im Spiel
                // (`combat.ts`, `resistanceFactor`).
                //
                // **Der Widerstand steht hier NICHT mehr** (Issue #328). Er
                // haengt am ZAUBER und wird vor jeder Rechnung gesetzt; was
                // hier bleibt, gilt fuer den ganzen Kampf.
                resistance: 1.0,
                ..config.factors
            },
            resistance_by_spell: resistance_by_spell(config.factors.resistance, config.boss),
            worn: config.worn,
            split_spender: config.split_spender,
            ashen_regimen: config.ashen_regimen,
            whole_work: config.whole_work,
            slow_rot: config.slow_rot,
            sealed_vessel: config.sealed_vessel,
            long_grip: config.long_grip,
            free_control_spend_armed: false,
            free_control_spends: 0,
            queue: EventQueue::with_capacity(EVENT_CAPACITY),
            log: CombatLog::with_capacity(config.log_capacity),
            rng: Streams::new(config.seed),
            // **Ohne Vorab-Speicher.** Der erste Versuch legte vier Plaetze je
            // Ereignis an — und `tests/allocation.rs` schlug sofort an: Der
            // Aufbau darf ohne Protokoll genau eine Anforderung kosten, die
            // Ereignis-Schlange. Vorab-Speicher fuer Empfaenger, die es noch
            // nicht gibt, kauft auch nichts: Angemeldet wird beim Aufbau, nicht
            // im Schritt (GAME-DESIGN 18.4 verlangt nur Letzteres).
            events: RuleEvents::default(),
            pending_gain: PendingGain::default(),
            stack_gains: std::rc::Rc::default(),
            pending_shorten: PendingSlot::default(),
            sublimation_uses: std::rc::Rc::default(),
        };
        fight.register_school_mechanics();
        fight
    }

    /// Die Schul-Mechanik anmelden (Issue E3-2).
    ///
    /// Eine Zeile fuer alle Schulen: Welcher Vorrat gefuellt wird und wie viel,
    /// steht in den Daten des jeweiligen Zaubers.
    fn register_school_mechanics(&mut self) {
        let mechanic = StackMechanic::new(
            std::rc::Rc::clone(&self.pending_gain),
            std::rc::Rc::clone(&self.stack_gains),
        );
        self.events
            .register("stacks:heat", RuleEvent::SpellCast, 0, Box::new(mechanic));
        let second = StackMechanic::new(
            std::rc::Rc::clone(&self.pending_gain),
            std::rc::Rc::clone(&self.stack_gains),
        );
        self.events
            .register("stacks:heat", RuleEvent::DotTick, 0, Box::new(second));

        // SUBLIMATION (Issue E3-3), Rang 10: nach der Stapel-Mechanik. Die
        // Verkuerzungen kommen aus den Zauber-Daten; wer nichts verkuerzt,
        // steht mit 0 in der Tabelle.
        let mut per_cast = [0; SPELL_COUNT];
        // **Der Talent-Faktor gehoert an DIESE Stelle, nicht ans Ablesen.** Die
        // Tabelle wird einmal gebaut und danach je Wirkung gelesen; ein Faktor
        // beim Lesen liefe zwar auch, aber die Spiel-Seite rechnet ihn in
        // `reductionOfSpell` ebenfalls auf den Rohwert. Gleiche Stelle, gleiche
        // Rundung — sonst driften die Seiten um einen Millisekunden-Rest.
        let faktor = self.talent_cooldown_reduction_factor;
        for (index, spell) in SPELLS.iter().enumerate() {
            per_cast[index] = if spell.cooldown_reduction_ms == 0 {
                0
            } else {
                (f64::from(spell.cooldown_reduction_ms) * faktor).round() as u32
            };
        }
        self.events.register(
            "sublimation",
            RuleEvent::SpellCast,
            10,
            Box::new(SublimationMechanic::new(
                per_cast,
                std::rc::Rc::clone(&self.pending_shorten),
                std::rc::Rc::clone(&self.sublimation_uses),
                self.talent_cooldown_threshold_permille,
            )),
        );
    }

    /// Wie oft die Sublimation angesetzt hat.
    #[must_use]
    pub fn sublimation_use_count(&self) -> u32 {
        self.sublimation_uses.get()
    }

    /// Wie oft die Schul-Mechanik Stapel angesetzt hat.
    #[must_use]
    pub fn stack_gain_count(&self) -> u32 {
        self.stack_gains.get()
    }

    /// Ein Regel-Ereignis austragen und einsammeln, was die Schul-Mechanik
    /// dabei angesetzt hat.
    ///
    /// **Die einzige Stelle, an der gebucht wird.** Der Empfaenger kann nicht
    /// selbst buchen (siehe [`crate::stacks`]); dass es hier und nur hier
    /// geschieht, haelt die Regel trotzdem an einem Ort.
    fn emit_rule(&mut self, event: RuleEvent, subject: i16, value: f64) {
        let now = self.time;
        self.events.emit(event, subject, value, now);
        let gained = self.pending_gain.replace(0);
        if gained > 0 {
            self.add_resource(gained);
        }
        let mut order = self.pending_shorten.replace(PendingShorten::default());
        if order.amount > 0 {
            // **Der Faktor aus `cd_reduce_buffs`** (DISTIL, E10). Er greift
            // HIER und nicht in `SublimationMechanic`: Die Mechanik liefert
            // den ROHBETRAG aus ihrer Tabelle, angewandt wird er an dieser
            // Stelle — genau wie `pending_gain` durch `add_resource` laeuft.
            //
            // Ein erster Befund hielt einen Schnittstellen-Umbau fuer noetig,
            // weil `sublimation.rs` den Wert aus einer kopierten Tabelle holt
            // und `Fight` nicht sieht. Das stimmt — aber es ist gleichgueltig,
            // weil die WIRKUNG hier liegt. Ich hatte auf die Stelle gesehen,
            // wo der Wert HERKOMMT, statt auf die, wo er WIRKT.
            order.amount =
                (f64::from(order.amount) * self.cd_reduce_buffs.factor(now)).round() as Millis;
            shorten_cooldowns(&mut self.cooldown_ready_at, order, now);
        }
    }

    /// Einen Empfaenger fuer ein Regel-Ereignis anmelden (Issue E3-1b).
    ///
    /// Laeuft beim Aufbau, nicht im Schritt.
    pub fn on_rule_event(
        &mut self,
        id: &str,
        on: RuleEvent,
        order: i32,
        receiver: Box<dyn RuleReceiver>,
    ) {
        self.events.register(id, on, order, receiver);
    }

    /// Die Regel-Ereignisse dieses Durchgangs — Folge, Anzahl, Empfaenger.
    #[must_use]
    pub const fn rule_events(&self) -> &RuleEvents {
        &self.events
    }

    // ---------------------------------------------------------------- Ablauf

    /// Rechnet den Kampf mit der angegebenen Rotation durch.
    ///
    /// Ein bereits gelaufener Kampf muss vorher mit [`Fight::reset`]
    /// zurueckgesetzt werden.
    pub fn run(&mut self, rotation: Rotation) -> RunResult {
        self.run_with(&rotation)
    }

    /// Rechnet den Kampf mit einer geschriebenen Prioritaetenliste durch.
    ///
    /// Nutzt dieselbe Schleife wie [`Fight::run`] — es gibt nur **eine**
    /// Regelrechnung, nicht zwei, die auseinanderlaufen koennten.
    pub fn run_apl(&mut self, apl: &crate::apl::Apl) -> RunResult {
        self.run_with(apl)
    }

    fn run_with<D: Decide + ?Sized>(&mut self, decider: &D) -> RunResult {
        // Der Verfall laeuft ab Kampfbeginn, unabhaengig davon, ob ueberhaupt
        // Ressource da ist — so macht es die Referenz, und das ist auch richtig:
        // Der Takt haengt am Kampf, nicht am Fuellstand.
        if RESOURCE.decay_per_step_ms > 0 {
            self.queue
                .push(RESOURCE.decay_per_step_ms, EventKind::ResourceDecay);
        }
        self.schedule_first_energy_tick();
        self.schedule_first_doom_tick();
        self.schedule_first_stack_tick();
        self.schedule_first_runaway_tick();
        self.schedule_first_affliction();
        self.schedule_next_timeline_step();
        // Die erste Entscheidung faellt bei 0.
        self.queue.push(0, EventKind::Wakeup);

        while let Some(now) = self.queue.peek_time() {
            if now >= self.duration_ms {
                break;
            }
            self.time = now;

            // Phase 1 — alle passiven Wirkungen dieses Zeitpunkts, in der
            // Rangfolge aus `event.rs`. Erst wenn keine mehr offen ist, darf
            // geprueft und entschieden werden; sonst saehe die Entscheidung
            // einen halb fortgeschriebenen Zustand.
            while self.queue.peek_time() == Some(now) {
                let event = self.queue.pop().expect("peek_time hat bestaetigt");
                self.apply(event.kind);
            }

            // Phase 2 — Sieg-Pruefung, an derselben Stelle wie in der Referenz.
            if self.killed_at.is_none() && self.damage >= self.boss.hp {
                self.killed_at = Some(now);
                self.log.push(now, LogKind::Kill, NO_SPELL, self.damage);
                if self.stop_on_kill {
                    break;
                }
            }

            // Der Spielertod beendet den Versuch — **immer**, auch wenn
            // `stop_on_kill` aus ist (GAME-DESIGN A.3: Versuch vorbei, keine
            // Strafe). Der Unterschied zum Sieg ist Absicht: Zwei Rotationen
            // ueber die volle Zeit zu vergleichen ergibt Sinn, ein toter
            // Spieler, der weiterwirkt, nicht.
            if self.died_at.is_some() {
                break;
            }
            self.expire_coagulation();
            self.expire_fixation();
            self.expire_putrefaction();
            self.expire_buffs();
            self.advance_race(now);
            self.check_phase();

            // Phase 3 — die Entscheidung des Spielers.
            if self.gcd_ready_at <= now {
                match decider.decide(self) {
                    Some(index) => self.cast(index),
                    None => {
                        // Gewartet, obwohl gewirkt werden durfte. Die Referenz
                        // zaehlt dafuer je leerem Takt 10 ms; hier ist es
                        // dieselbe Spanne am Stueck, bis das naechste Ereignis
                        // den Zustand aendern kann.
                        let next = self.queue.peek_time().unwrap_or(self.duration_ms);
                        self.idle_ms += next.min(self.duration_ms) - now;
                    }
                }
            }

            self.schedule_decision_wakeup(decider, now);
        }

        self.result()
    }

    /// Spielt eine aufgezeichnete Runde ab (GAME-DESIGN 18.4).
    ///
    /// Anders als [`Fight::run`] entscheidet hier nicht der Simulator, was
    /// gewirkt wird — die Tastendruecke stehen fest. Genau das macht den
    /// Pruefall aussagekraeftig: Geprueft wird die **Regelrechnung**, nicht die
    /// Rotationswahl.
    ///
    /// # Warum das hier Takt fuer Takt laeuft
    ///
    /// Der uebrige Simulator springt von Ereignis zu Ereignis; das geht hier
    /// nicht, denn die Aufzeichnung ist taktgebunden und die Pruefsumme wird je
    /// Takt verglichen. Beide Wege benutzen aber **dieselben** Zustandsuebergaenge
    /// ([`Fight::apply`], [`Fight::cast`]) — es gibt nur eine Regelrechnung, nicht
    /// zwei, die auseinanderlaufen koennten.
    ///
    /// Der Ablauf ist der von `replayRecording` in `src/devtools/recording.ts`:
    /// je Takt erst die passiven Wirkungen, dann die Sieg-Pruefung mit Abbruch,
    /// dann die Tastendruecke, dann die Pruefsumme.
    pub fn replay(&mut self, recording: &Recording, checksum_per_tick: bool) -> ReplayResult {
        if RESOURCE.decay_per_step_ms > 0 {
            self.queue
                .push(RESOURCE.decay_per_step_ms, EventKind::ResourceDecay);
        }
        self.schedule_first_energy_tick();
        self.schedule_first_doom_tick();
        self.schedule_first_stack_tick();
        self.schedule_first_runaway_tick();
        self.schedule_first_affliction();
        self.schedule_next_timeline_step();

        let mut checksums = Vec::with_capacity(if checksum_per_tick {
            recording.ticks as usize
        } else {
            0
        });
        // Die drei Zustandsspuren. Wie `checksums` nur auf Verlangen, und mit
        // derselben Kapazitaet vorbelegt — im Schritt wird nichts angefordert.
        let mut phases = Vec::with_capacity(checksums.capacity());
        let mut energy = Vec::with_capacity(checksums.capacity());
        let mut doom = Vec::with_capacity(checksums.capacity());
        let mut doom_charges = Vec::with_capacity(checksums.capacity());
        let mut stacks_trace = Vec::with_capacity(checksums.capacity());
        let mut afflicted_trace = Vec::with_capacity(checksums.capacity());
        let mut reset_charges_trace = Vec::with_capacity(checksums.capacity());
        let mut cast_lands_trace: Vec<i64> = Vec::with_capacity(checksums.capacity());
        let mut timeline_index = Vec::with_capacity(checksums.capacity());
        let mut player_hp = Vec::with_capacity(checksums.capacity());
        let mut shield_spur = Vec::with_capacity(checksums.capacity());
        let mut sequence: u32 = 0;
        let mut accepted = 0;
        let mut rejected = 0;
        let mut press_index = 0;

        for tick in 0..recording.ticks {
            let now = tick * TICK_MS;
            self.time = now;

            // Phase 1 — passive Wirkungen dieses Takts.
            while let Some(due) = self.queue.peek_time() {
                if due > now {
                    break;
                }
                let event = self.queue.pop().expect("peek_time hat bestaetigt");
                self.apply(event.kind);
            }

            self.expire_coagulation();
            self.expire_fixation();
            self.expire_putrefaction();
            self.expire_buffs();
            self.advance_race(now);
            self.check_phase();

            // Phase 2 — Sieg-Pruefung. Das Spiel bricht hier ab, und die
            // Pruefsumme dieses Takts wird noch geschrieben.
            if self.killed_at.is_none() && self.damage >= self.boss.hp {
                self.killed_at = Some(now);
                self.log.push(now, LogKind::Kill, NO_SPELL, self.damage);
                let hash = self.checksum_at(now);
                sequence = sequence.wrapping_mul(31).wrapping_add(hash);
                if checksum_per_tick {
                    checksums.push(hash);
                    phases.push(self.phase as u32);
                    energy.push(self.energy);
                    doom.push(self.doom);
                    doom_charges.push(self.doom_charges);
                    stacks_trace.push(self.stacks);
                    timeline_index.push(self.timeline_index as u32);
                    player_hp.push(self.player_hp);
                    shield_spur.push(self.shield);
                    afflicted_trace.push(u32::from(self.afflicted));
                    reset_charges_trace.push(u32::from(self.reset_mode.charges_left()));
                    cast_lands_trace
                        .push(self.casting.map_or(-1, |_| i64::from(self.cast_lands_at)));
                }
                break;
            }

            // Der Spielertod endet genauso — mit der Pruefsumme dieses Takts,
            // damit beide Seiten denselben letzten Eintrag haben.
            if self.died_at.is_some() {
                let hash = self.checksum_at(now);
                sequence = sequence.wrapping_mul(31).wrapping_add(hash);
                if checksum_per_tick {
                    checksums.push(hash);
                    phases.push(self.phase as u32);
                    energy.push(self.energy);
                    doom.push(self.doom);
                    doom_charges.push(self.doom_charges);
                    stacks_trace.push(self.stacks);
                    timeline_index.push(self.timeline_index as u32);
                    player_hp.push(self.player_hp);
                    shield_spur.push(self.shield);
                    afflicted_trace.push(u32::from(self.afflicted));
                    reset_charges_trace.push(u32::from(self.reset_mode.charges_left()));
                    cast_lands_trace
                        .push(self.casting.map_or(-1, |_| i64::from(self.cast_lands_at)));
                }
                break;
            }

            // Phase 3 — die Tastendruecke dieses Takts.
            while press_index < recording.presses.len() {
                let press = recording.presses[press_index];
                if press.tick > tick {
                    break;
                }
                press_index += 1;
                match recording.spell_for_slot(press.slot) {
                    // Unbelegter Platz: Fehleingabe, nichts wird gemerkt.
                    None => rejected += 1,
                    Some(index) => {
                        // **Drei Ausgaenge, nicht zwei** (Issue #223). Steht
                        // der Auswahlmodus scharf, WAEHLT dieser Druck, statt
                        // zu wirken — dieselbe Regel wie `Session.apply` auf
                        // der Spielseite, und dieselbe Reihenfolge.
                        //
                        // Der Waechter, der bis hierher stand, faellt damit:
                        // Aufzeichnungen duerfen REFLUX enthalten.
                        if self.reset_mode.is_armed() {
                            // Die Wahl verbraucht den Ruecksetzer in jedem
                            // Fall — auch eine falsche. `choose` setzt die
                            // Abklingzeit und die Wartezeit selbst, damit die
                            // Regel an einer Stelle steht.
                            match self.reset_mode.choose(
                                index,
                                now,
                                &mut self.cooldown_ready_at,
                                &mut self.gcd_ready_at,
                                GCD_MS,
                            ) {
                                // **Eine Auswahl zaehlt NICHT als Wirkung**,
                                // weder als angenommene noch als abgelehnte.
                                // `Session.apply` erhoeht `accepted` nur im
                                // Zweig `if (!chose)`; sonst stuende in der
                                // Auswertung eine Wirkung, die nie stattfand.
                                //
                                // Gemessen, nicht gelesen: Meine erste Fassung
                                // zaehlte sie mit und kam auf 40 statt 38
                                // angenommene Druecke bei 40 Druecken und zwei
                                // Auswahlen. Die Pruefsumme merkte davon
                                // nichts — sie fuehrt die Zaehler nicht.
                                Choice::Applied => {}
                                // Eine verpuffte Wahl dagegen ist eine
                                // Fehleingabe wie jede andere — auf der
                                // Spielseite `accepted = false` und damit
                                // `rejected++`.
                                Choice::Wasted => rejected += 1,
                                // Kann hier nicht auftreten: `is_armed` wurde
                                // gerade geprueft. Behandelt statt ignoriert,
                                // damit eine spaetere Aenderung an `choose`
                                // nicht still durchfaellt.
                                Choice::NotArmed => rejected += 1,
                            }
                        } else if self.is_ready(index) {
                            self.cast(index);
                            accepted += 1;
                            // **NACH der Wirkung scharf stellen**, nie davor.
                            // Vorher waere falsch: Dann waehlte sein eigener
                            // Druck bereits, und der Ruecksetzer wuerde nie
                            // gewirkt. Dieselbe Reihenfolge wie in
                            // `Session.apply`; AC4 von #223 haelt sie fest.
                            // Der Zuschlag greift NACH der Pruefung, nicht in
                            // ihr — sonst waere jeder Zauber ein Ruecksetzer,
                            // sobald ein Rang verteilt ist.
                            if SPELLS[index].cooldown_reset_charges > 0 {
                                self.reset_mode.arm(
                                    index,
                                    SPELLS[index].cooldown_reset_charges.saturating_add(
                                        // `cooldown_reset_charges` ist u8, der
                                        // Zuschlag u32 wie die acht anderen
                                        // Talent-Felder. Geklemmt statt
                                        // gecastet: 300 Raenge waeren sonst 44.
                                        u8::try_from(self.talent_cooldown_charges_bonus)
                                            .unwrap_or(u8::MAX),
                                    ),
                                );
                            }
                        } else {
                            // Zu frueh gedrueckt. Das Spiel merkt sich den Druck
                            // ausdruecklich **nicht** (GAME-DESIGN 21.1: keine
                            // Eingabe-Vormerkung) — er ist damit verloren.
                            rejected += 1;
                        }
                    }
                }
            }

            let hash = self.checksum_at(now);
            sequence = sequence.wrapping_mul(31).wrapping_add(hash);
            if checksum_per_tick {
                checksums.push(hash);
                phases.push(self.phase as u32);
                energy.push(self.energy);
                doom.push(self.doom);
                doom_charges.push(self.doom_charges);
                stacks_trace.push(self.stacks);
                timeline_index.push(self.timeline_index as u32);
                player_hp.push(self.player_hp);
                shield_spur.push(self.shield);
                afflicted_trace.push(u32::from(self.afflicted));
                reset_charges_trace.push(u32::from(self.reset_mode.charges_left()));
                cast_lands_trace.push(self.casting.map_or(-1, |_| i64::from(self.cast_lands_at)));
            }
        }

        let final_time = self
            .killed_at
            .unwrap_or_else(|| recording.ticks.saturating_sub(1) * TICK_MS);

        ReplayResult {
            damage: self.damage,
            killed_at_ms: self.killed_at,
            accepted,
            rejected,
            casts: self.casts.to_vec(),
            final_checksum: self.checksum_at(final_time),
            sequence_checksum: sequence,
            synergy_used: self.synergy_used,
            synergy_available: self.synergy_available,
            checksums,
            random_draws: self.rng.total_draws(),
            phases,
            energy,
            damage_lost_to_race: self.damage_lost_to_race,
            boss_healed: self.boss_healed,
            stabilised: self.stabilised,
            race_won: self.race_won,
            doom,
            doom_charges,
            stacks: stacks_trace,
            timeline_index,
            player_hp,
            damage_taken: self.damage_taken,
            died_at_ms: self.died_at,
            control_spent: self.control_spent,
            control_failed: self.control_failed,
            interrupts_landed: self.interrupts_landed,
            interrupts_missed: self.interrupts_missed,
            casts_interrupted: self.casts_interrupted,
            casts_cancelled: self.casts_cancelled,
            damage_by_spell: self.damage_by_spell,
            shield: shield_spur,
            afflicted: afflicted_trace,
            reset_charges_left: reset_charges_trace,
            cast_lands_at: cast_lands_trace,
            afflictions_cleared: self.afflictions_cleared,
            cycle: self.cycle(),
            shield_absorbed: self.shield_absorbed,
            healed: self.healed,
            converted: self.converted,
            reflected: self.reflected,
        }
    }

    /// Setzt den Kampf zurueck, **ohne** Speicher freizugeben oder neu
    /// anzufordern. So kostet ein zweiter Durchgang nichts.
    pub fn reset(&mut self, seed: u32) {
        self.time = 0;
        self.damage = 0;
        self.killed_at = None;
        self.phase = 0;
        self.energy = 0;
        self.energy_triggers = 0;
        self.doom = 0;
        self.doom_charges = self.boss.doom.map_or(0, |d| d.clear_charges);
        self.stacks = 0;
        self.race_ends_at = None;
        self.race_done = false;
        self.race_progress = 0;
        self.race_won = false;
        self.damage_lost_to_race = 0;
        self.boss_healed = 0;
        self.stabilised = 0;
        self.afflicted = false;
        self.afflictions_cleared = 0;
        self.timeline_index = 0;
        self.casting = None;
        // **Und der Faktor dazu**, nicht nur der Zustand: `reset` leert die
        // Liste, die Rechnung stuende ohne die zweite Zeile weiter auf dem
        // letzten Wert. Genau dieser Fehler ist beim Synergie-Fenster passiert
        // (#318) — dort blieb nach einem Leistenwechsel "1 of 1" stehen, weil
        // der Zustand geraeumt wurde und die Rechnung nicht.
        self.buffs.reset();
        self.control_buffs.reset();
        self.resource_buffs.reset();
        self.tick_buffs.reset();
        self.stack_buffs.reset();
        self.cd_reduce_buffs.reset();
        self.convert_buffs.reset();
        self.cap_buffs.reset();
        self.factors.buffs = crate::factors::NEUTRAL.buffs;
        // **Der Anteil des SPIELERS am Ring-Verlust** (Issue #349). Er stand
        // bis zum 2026-08-05 nicht hier — gefunden feldgenau ueber
        // `record_fields`, das die Pruefsumme mit ihren Namen aufzeichnet:
        //
        //     ABWEICHUNG [115] playerDamage: nach reset 4240, frisch 0
        //
        // Es ist das SPIEGELBILD des Falls, den Worker-2 gefunden hat: Dort
        // raeumt der Simulator die Zersetzungs-Stapel und das Spiel nicht,
        // hier raeumt das Spiel `playerDamageValue` und der Simulator nicht.
        // Beide Male unsichtbar, weil kein Paritaets-Weg zweimal faehrt.
        self.player_damage = 0;
        self.stack_gains.set(0);
        // Ein neuer Lauf faengt ohne laufende Wirkzeit an (Issue #304). Ohne
        // diese Zeilen bliebe der Zustand stehen, und der naechste Kampf
        // begaenne dauerhaft blockiert.
        self.player_casting = None;
        self.player_cast_ends_at = 0;
        self.player_cast_generation = 0;
        self.casts_interrupted = 0;
        self.casts_cancelled = 0;
        self.interrupts_missed = 0;
        self.interrupts_landed = 0;
        self.player_hp = self.vessel_max;
        self.died_at = None;
        self.damage_taken = 0;
        self.shield = 0;
        self.shield_expires_at = None;
        self.shield_stacks = 0;
        self.shield_absorbed = 0;
        self.healed = 0;
        // **Auch die laufende Heilung endet mit der Runde.** Ohne diese Zeilen
        // liefe ein POULTICE aus der vorigen Runde in die naechste hinein und
        // heilte dort weiter — sichtbar erst an einer Anzeige, die ohne Zauber
        // steigt.
        //
        // Die Anwendungsnummer geht dabei auf 0 zurueck, nicht hoch: `reset`
        // leert die Schlange (`self.queue.clear()`), es kann also kein Weckruf
        // aus der alten Runde uebrig sein. Genauso halten es `dot_generation`
        // und `player_cast_generation` — und `reset_completeness` besteht
        // darauf, dass nach dem Zuruecksetzen KEIN Feld anders steht als in
        // einem frischen Kampf.
        self.heal_over_time_amount = 0;
        self.heal_over_time_steps = 0;
        self.heal_over_time_interval_ms = 0;
        self.heal_over_time_done = 0;
        self.heal_over_time_credited = 0;
        self.heal_over_time_generation = 0;
        self.conversion_until = None;
        self.conversion_percent = 0;
        // DECAY (#479) — eine Schwaechung, die einen Kampf ueberlebte,
        // schuetzte im naechsten gratis.
        self.decay_ward_until = None;
        self.decay_ward_permille = 0;
        self.converted = 0;
        self.reflect_until = None;
        self.reflect_percent = 0;
        self.reflected = 0;
        self.reflect_spell = 0;
        self.control = 0;
        self.pending_control_damage = 0;
        self.next_strike_delay = 0;
        // PIN (#478) — ein Merker, der einen Kampf ueberlebte, machte im
        // naechsten einen Zauber unterbrechbar, den niemand bezahlt hat.
        self.pin_window = 0;
        self.phase_hold_percent = 0;
        self.phase_hold_until = None;
        self.cast_lands_at = 0;
        self.control_spent = 0;
        self.decay_expiry = [0; DECAY_STACK_CAPACITY];
        // PUTRID BLOOM (#479) — eine Anhebung, die einen Kampf ueberlebte, gaebe
        // im naechsten Plaetze, die niemand bezahlt hat.
        self.decay_cap_bonus = 0;
        self.decay_cap_until = None;
        self.decay_count = 0;
        // **Die Synergie-Fenster und ihre Zaehler** (#318). Im Browser
        // gemessen: Ohne das traegt der naechste Lauf die Ausbeute des vorigen.
        self.synergy_window_ends = [0; SPELL_COUNT];
        self.synergy_used = 0;
        self.synergy_available = 0;
        self.control_failed = 0;
        self.gcd_ready_at = 0;
        self.next_cast_free_gcd = false;
        self.next_cast_doubles = false;
        self.retort_armed = false;
        self.reset_mode.reset();
        self.cooldown_ready_at = [0; SPELL_COUNT];
        self.resource = 0;
        self.dot_expires_at = [None; SPELL_COUNT];
        self.dot_generation = [0; SPELL_COUNT];
        self.casts = [0; SPELL_COUNT];
        self.damage_by_spell = [0; SPELL_COUNT];
        self.lost_dot_ticks = [0; SPELL_COUNT];
        self.dot_tick_counts = [0; SPELL_COUNT];
        self.resource_overflow = 0;
        self.idle_ms = 0;
        self.queue.clear();
        self.log.clear();
        self.rng = Streams::new(seed);
        // Nur die Folge, nicht die Anmeldungen: Ein neuer Durchgang behaelt
        // seine Empfaenger und beginnt lediglich wieder bei null Ereignissen.
        self.events.clear_sequence();
    }

    /// Plant den ersten Anstieg der Energieleiste ein, falls der Boss eine hat.
    fn schedule_first_energy_tick(&mut self) {
        if let Some(energy) = self.boss.energy {
            if energy.per_step_ms > 0 && energy.per_step_ms < self.duration_ms {
                self.queue.push(energy.per_step_ms, EventKind::EnergyTick);
            }
        }
    }

    /// Plant den ersten Anstieg des toedlichen Zaehlers ein (Issue #216).
    fn schedule_first_doom_tick(&mut self) {
        if let Some(doom) = self.boss.doom {
            if doom.per_step_ms > 0 && doom.per_step_ms < self.duration_ms {
                self.queue.push(doom.per_step_ms, EventKind::DoomTick);
            }
        }
    }

    /// Plant den ersten Schritt der davonlaufenden Reaktion ein (Issue #247).
    fn schedule_first_runaway_tick(&mut self) {
        if let Some(run) = self.boss.runaway {
            if run.per_step_ms > 0 && run.per_step_ms < self.duration_ms {
                self.queue.push(run.per_step_ms, EventKind::RunawayTick);
            }
        }
    }

    /// Plant den ersten Stapel ein (Issue #216).
    fn schedule_first_stack_tick(&mut self) {
        if let Some(stacks) = self.boss.stacks {
            if stacks.per_step_ms > 0 && stacks.per_step_ms < self.duration_ms {
                self.queue.push(stacks.per_step_ms, EventKind::StackTick);
            }
        }
    }

    /// Plant den Beginn der Schwaechung ein (Issue #240).
    ///
    /// **Der Beginn, nicht der erste Schaden.** Sie greift zu `at_ms` und
    /// tickt von da an; der erste Tick faellt eine Schrittweite spaeter. Wer
    /// hier gleich `at_ms + per_step_ms` einplante, verschoebe den Beginn und
    /// damit auch den Zeitpunkt, ab dem sie sich entfernen laesst.
    fn schedule_first_affliction(&mut self) {
        if let Some(aff) = self.boss.affliction {
            if aff.at_ms < self.duration_ms {
                self.queue.push(aff.at_ms, EventKind::AfflictionTick);
            }
        }
    }

    /// Plant den naechsten faelligen Eintrag der Boss-Zeitleiste ein.
    ///
    /// Immer nur **einen**. Alle auf einmal einzuplanen waere kuerzer, wuerde
    /// aber die Ereignis-Schlange mit Eintraegen fuellen, die ein frueher
    /// Sieg ohnehin nie erreicht — und ihre Kapazitaet ist bewusst fest
    /// ([`EVENT_CAPACITY`]).
    /// Welcher Eintrag beim Fortschrittsstand `index` ansteht, und wann.
    ///
    /// **Der Zaehler ist Fortschritt, nicht Platz in der Liste.** Wiederholt
    /// sich die Zeitleiste (`timeline_loop_ms`, Issue #174), laeuft er darueber
    /// hinaus: Eintrag `i` im Zyklus `z` ist faellig bei `z * loop + at_ms`.
    ///
    /// Ohne Wiederholung endet die Zeitleiste am letzten Eintrag — dann gibt
    /// diese Funktion `None`, und der Kampf plant nichts mehr ein.
    ///
    /// **Bei leerer Liste immer `None`.** Das ist der Fall der stummen Puppe,
    /// an der maximale Rotationen gemessen werden: Eine Wiederholung ueber
    /// nichts darf nicht kreisen, nichts ausloesen und nichts kosten.
    fn timeline_at(&self, index: usize) -> Option<(TimelineEntry, Millis)> {
        let len = self.boss.timeline.len();
        if len == 0 {
            return None;
        }
        match self.boss.timeline_loop_ms {
            None => self.boss.timeline.get(index).map(|e| (*e, e.at_ms)),
            Some(loop_ms) => {
                let entry = self.boss.timeline[index % len];
                let zyklus = Millis::try_from(index / len).unwrap_or(Millis::MAX);
                Some((
                    entry,
                    zyklus.saturating_mul(loop_ms).saturating_add(entry.at_ms),
                ))
            }
        }
    }

    fn schedule_next_timeline_step(&mut self) {
        let Some((_, at_ms)) = self.timeline_at(self.timeline_index) else {
            return;
        };
        // **Die Verzoegerung aus STAY gilt nur fuer diesen einen Eintrag**
        // (#100). Die folgenden stehen, wo sie stehen — der Abstand dahinter
        // schrumpft, und genau das ist der Preis.
        let faellig = at_ms + self.next_strike_delay;
        if faellig < self.duration_ms {
            self.queue.push(
                faellig,
                EventKind::TimelineStep {
                    index: u32::try_from(self.timeline_index).unwrap_or(u32::MAX),
                },
            );
        }
    }

    /// Ein Eintrag der Zeitleiste ist faellig (GAME-DESIGN 11.1).
    ///
    /// **Noch ohne Wirkung.** Die neun Mechanik-Bausteine kommen in E4-2 bis
    /// E4-6, die Zeitleisten der einzelnen Bosse in E10. Was hier schon steht,
    /// ist der Zeitpunkt: Er wird taktgenau angesteuert, protokolliert und geht
    /// ueber den Zeiger in die Zustands-Pruefsumme ein. Ein spaeter angehaengter
    /// Baustein setzt damit auf nachgewiesener Zeitrechnung auf statt auf einer
    /// erst dann erstmals gepruefen.
    fn timeline_step(&mut self, index: usize) {
        // Ein veralteter Eintrag: Der Zeiger ist bereits weiter. Kann heute
        // nicht vorkommen, weil immer nur einer offen ist — aber wenn E4-2
        // Eintraege ueberspringbar macht, faengt es das hier ab, statt den
        // Zeiger rueckwaerts laufen zu lassen.
        if index != self.timeline_index {
            return;
        }
        // **Verschoben statt faellig** (STAY, #100). Wurde der Eintrag
        // nachtraeglich verzoegert, steht der neue Weckruf schon in der
        // Schlange; dieser hier ist der alte und laeuft ins Leere. Ihn zu
        // loeschen waere teurer als ihn verfallen zu lassen — dasselbe
        // Vorgehen wie bei veralteten DoT-Ticks und beim Boss-Zauber.
        let Some((entry, at_ms)) = self.timeline_at(index) else {
            return;
        };
        if self.time < at_ms + self.next_strike_delay {
            return;
        }
        // **Die Phasen-Bindung** (Issue #557). Bis hierher war `phase` ein
        // Zustand ohne Wirkung: gesetzt, in die Pruefsumme gerueht, von keiner
        // Regel gelesen. Ein Boss mit zwei Phasenschwellen verlief bitgleich
        // zu demselben Boss mit `phases: []`.
        //
        // **Das Ereignis VERFAELLT, es wird nicht nachgeholt** — der Index
        // laeuft weiter, die Wirkung entfaellt. Ein aufgestautes Ereignis
        // fiele beim Phasenwechsel gesammelt mit allen anderen, und aus einer
        // lernbaren Abfolge wuerde eine Lawine zum Schwellenzeitpunkt.
        //
        // Dieselbe Stelle und dieselbe Reihenfolge wie in
        // `Combat.stepPassive`: vor dem Log, damit ein uebersprungenes
        // Ereignis nicht als gefeuert erscheint.
        if entry.phase != u32::MAX && (self.phase as u32) < entry.phase {
            self.timeline_index += 1;
            self.next_strike_delay = 0;
            self.schedule_next_timeline_step();
            return;
        }

        self.log
            .push(self.time, LogKind::BossTimeline, NO_SPELL, index as u32);

        // **Hier bekommt die Zeitleiste ihre Wirkung** (Issue #110). In #96
        // wurde der Zeitpunkt nur angesteuert und protokolliert.
        // **PIN wird HIER gelesen und HIER verbraucht** (#478) — im
        // Ausloese-Moment, nicht waehrend einer laufenden Wirkzeit. Eine Stelle
        // frueher gelesen, befreite PIN den gerade laufenden Zauber: sich
        // selbst statt den naechsten, und der Fall bliebe GRUEN, weil ja etwas
        // unterbrechbar wurde.
        //
        // Verbraucht wird er in BEIDEN Zweigen. Traf er einen ohnehin
        // unterbrechbaren Eintrag, ist er verpufft — das ist die Entscheidung,
        // die der Zauber abverlangt, und keine Buchhaltung, die davor bewahrt.
        let pin_fenster = self.pin_window;
        self.pin_window = 0;
        match entry.kind {
            // Trifft sofort — es sei denn, PIN hat ihm ein Fenster gegeben.
            // Dann laeuft er wie ein Zauber und ist zu unterbrechen.
            TimelineKind::Strike if pin_fenster > 0 => {
                self.casting = Some(index);
                self.cast_lands_at = self.time + pin_fenster;
                self.queue.push(
                    self.cast_lands_at,
                    EventKind::BossCastLands {
                        index: u32::try_from(index).unwrap_or(u32::MAX),
                    },
                );
            }
            // Trifft sofort. Nicht zu verhindern, nur auszuhalten.
            TimelineKind::Strike => self.hit_player(entry.damage),
            // Beginnt zu wirken. Ob er trifft, entscheidet sich erst, wenn die
            // Wirkzeit um ist — bis dahin kann der Spieler ihn abbrechen.
            //
            // **Die eigene Wirkzeit bleibt, auch unter PIN.** Ein
            // schrumpfendes Fenster (#240) ist eine Eigenschaft des Bosses;
            // PIN darf es nicht heimlich verlaengern.
            TimelineKind::InterruptibleCast => {
                self.casting = Some(index);
                self.cast_lands_at =
                    self.time + self.escalated_cast_time(entry.cast_time_ms, index);
                self.queue.push(
                    self.cast_lands_at,
                    EventKind::BossCastLands {
                        index: u32::try_from(index).unwrap_or(u32::MAX),
                    },
                );
            }
        }

        // **Der Abbruch, und zwar nur durch bestimmte Angriffe** (GAME-DESIGN
        // 7.1, Issue #304). Welche das sind, steht in den BOSS-Daten und nicht
        // hier — dieselbe Stelle und dieselbe Reihenfolge wie in
        // `Combat::stepPassive`: nach dem Schlag, vor dem Vorruecken.
        if entry.interrupts_player && self.cancel_player_cast() {
            self.casts_interrupted += 1;
            // **Die Rüstung greift** (RETORT, #480).
            //
            // **Der Rest der Wartezeit wird erlassen, nicht die volle.** Die
            // schon verstrichene Zeit hat der Spieler durchlebt — sie ist
            // nicht verloren. Verloren ist nur, was er ohne Gegenleistung
            // noch abwarten müsste. „Der Abbruch kostet keine Wartezeit"
            // heisst: ab hier ist er frei.
            //
            // **Hier und nicht in `cancel_player_cast`**: Die Funktion wird
            // auch vom Selbst-Abbruch gerufen. Wer sich rüstet und dann selbst
            // abbricht, hätte sich die Wartezeit gekauft — die Rüstung bleibt
            // dort stehen und wird nicht verbraucht.
            if self.take_retort_armed() {
                self.gcd_ready_at = self.time;
                self.queue.push(self.gcd_ready_at, EventKind::Wakeup);
            }
        }

        // Verbraucht: Der naechste Eintrag faengt wieder bei seinem eigenen
        // Zeitpunkt an.
        self.next_strike_delay = 0;
        self.timeline_index += 1;
        self.schedule_next_timeline_step();
    }

    /// Die laufende Wirkzeit des Spielers abbrechen (Issue #304, AC4).
    ///
    /// **Die Abklingzeit laeuft nicht an**, und das steht hier nicht als Zeile,
    /// sondern als Abwesenheit einer: Sie wird in `resolve_player_cast`
    /// gesetzt, und die laeuft bei einem Abbruch nie.
    /// Verbraucht den Merker aus SUBLIME, falls er steht (Issue #480).
    ///
    /// Gibt `true` zurück, wenn dieser Wurf von der Wartezeit befreit war —
    /// und löscht ihn dabei. **Einmal gesetzt, einmal verbraucht**: Ein
    /// Merker, der stehen bliebe, machte aus einem einzelnen freien Wurf eine
    /// dauerhafte Befreiung.
    ///
    /// Er wird an genau zwei Stellen abgefragt, weil der GCD an genau zwei
    /// Stellen gesetzt wird: einmal für Zauber mit Wirkzeit, einmal für
    /// sofortige.
    fn take_free_gcd(&mut self) -> bool {
        if self.next_cast_free_gcd {
            self.next_cast_free_gcd = false;
            true
        } else {
            false
        }
    }

    /// Verbraucht den Merker aus CONDENSE, falls er steht (Issue #480).
    ///
    /// **Er wird auch dann verbraucht, wenn der Zauber gar keinen Schaden
    /// macht.** Ein Hilfszauber wirkt dann zweimal ohne sichtbaren
    /// Unterschied — und der Merker ist weg. Die Alternative wäre, ihn über
    /// Hilfszauber hinweg aufzuheben, bis ein Schadenszauber kommt: Das hiesse
    /// "der nächste SCHADENSZAUBER wirkt zweimal", und das steht so weder im
    /// Raster noch auf der Zauberleiste.
    fn take_double_cast(&mut self) -> bool {
        if self.next_cast_doubles {
            self.next_cast_doubles = false;
            true
        } else {
            false
        }
    }

    /// Verbraucht die Rüstung, falls sie steht (RETORT, Issue #480).
    ///
    /// **Einmal gerüstet, einmal verbraucht** — wie bei den zwei anderen
    /// Merkern. Eine Rüstung, die stehen bliebe, machte aus einer einzelnen
    /// Vorbereitung eine dauerhafte Vergünstigung.
    fn take_retort_armed(&mut self) -> bool {
        if self.retort_armed {
            self.retort_armed = false;
            true
        } else {
            false
        }
    }

    /// Bricht eine laufende Wirkzeit ab. **Von zwei Wegen gerufen, und der
    /// Unterschied ist Absicht** (RETORT, Issue #480).
    ///
    /// | Aufrufer | was er ist | setzt RETORTs Merker |
    /// |---|---|---|
    /// | `timeline_step`, bei `entry.interrupts_player` | der Boss stört | **ja** |
    /// | `cast`, bei einem zweiten Druck | der Spieler entscheidet | **nein** |
    ///
    /// Deshalb steht `interrupted_gcd_ms` **beim Aufrufer** und nicht hier
    /// drin: Von hier aus liesse sich nicht unterscheiden, wer gerufen hat.
    ///
    /// # Warum der Selbst-Abbruch leer ausgeht
    ///
    /// RETORT ist ein Gegenmittel gegen den Boss, kein Werkzeug zum
    /// Umdisponieren. Bekäme der Spieler die Wartezeit auch für den eigenen
    /// Abbruch zurück, wäre die beste Spielweise, sich selbst zu
    /// unterbrechen — ein Zauber, der das Ausnutzen der eigenen Fehlplanung
    /// belohnt, ist kein Gegenmittel. Der Boss-Abbruch ist ein äusseres
    /// Ereignis, auf das reagiert wird; der Selbst-Abbruch ist eine Wahl,
    /// und dafür gibt es keine Entschädigung.
    ///
    /// **Das ist kein vergessener Fall.** Wer hier eine Zeile ergänzt, die
    /// den Merker für beide Wege setzt, kehrt die Entscheidung um.
    /// `sim/tests/retort_wirkung.rs` hält sie fest: Ein Fall fährt den
    /// Selbst-Abbruch und belegt, dass RETORT danach nichts erstattet.
    fn cancel_player_cast(&mut self) -> bool {
        if self.player_casting.is_none() {
            return false;
        }
        self.player_casting = None;
        self.player_cast_ends_at = 0;
        true
    }

    /// Ein Boss-Zauber ist fertig gewirkt.
    ///
    /// Er trifft **nur**, wenn er noch laeuft. Wurde er unterbrochen, steht
    /// `casting` auf `None` oder auf einem anderen Eintrag, und dieser Aufruf
    /// verpufft — der eingeplante Treffer aus der Schlange zu entfernen waere
    /// teurer als ihn hier verfallen zu lassen (dasselbe Vorgehen wie bei den
    /// veralteten DoT-Ticks, `generation`).
    fn boss_cast_lands(&mut self, index: usize) {
        if self.casting != Some(index) {
            return;
        }
        // **Verschoben statt faellig** (STALL, #100). Der urspruenglich
        // eingeplante Weckruf laeuft ins Leere; der neue steht schon in der
        // Schlange. Ihn zu loeschen waere teurer als ihn verfallen zu lassen —
        // dasselbe Vorgehen wie bei veralteten DoT-Ticks.
        if self.time < self.cast_lands_at {
            return;
        }
        self.casting = None;
        self.interrupts_missed += 1;
        // Ueber `timeline_at`, nicht ueber den Platz in der Liste: Bei
        // wiederholter Zeitleiste liegt der Zaehler darueber (#174).
        if let Some((entry, _)) = self.timeline_at(index) {
            self.hit_player(entry.damage);
        }
        // **Ein Fehler macht Fortschritt rueckgaengig** (11.2, Baustein 5;
        // Issue #218). Derselbe Ausloeser wie auf der Spielseite: der
        // durchgekommene Boss-Zauber. WIE VIEL steht im Boss-Datensatz —
        // fehlt das Feld, heilt nichts.
        //
        // **Seit #609 sagt der Datensatz auch, WORAN sie haengt** — bis dahin
        // war dies die einzige Stelle.
        if self.boss.boss_heal.map(|h| h.on) == Some(BossHealTrigger::Interrupt) {
            self.heal_boss();
        }
    }

    /// Das Unverwundbarkeits-Fenster fortschreiben (Baustein 4; Issue #218).
    ///
    /// **Eine Stelle fuer beide Schleifen.** `run_with` und `replay` haben je
    /// eine eigene Takt-Schleife; die Regel zweimal hinzuschreiben hiesse,
    /// dieselbe Sonderregel an zwei Orten zu pflegen — und einer laeuft
    /// irgendwann weg.
    ///
    /// Genau das ist mir hier passiert: Mein erster Einbau traf nur `run_with`,
    /// und die Paritaet meldete 0 verlorenen Schaden gegen 380 im Spiel.
    ///
    /// Das Fenster beginnt GENAU EINMAL. Eines, das sich wiederholt, waere die
    /// eskalierende Wiederholung (Baustein 7) — ein anderer Baustein.
    fn advance_race(&mut self, now: Millis) {
        let Some(race) = self.boss.race else {
            return;
        };
        if !self.race_done && now >= race.at_ms {
            self.race_done = true;
            self.race_ends_at = Some(now + race.window_ms);
        }
        if let Some(ende) = self.race_ends_at {
            if now >= ende {
                self.race_ends_at = None;
            }
        }
    }

    /// Den Boss um den Betrag aus den Daten heilen (Baustein 5; Issue #218).
    ///
    /// Der Fortschritt faellt nie unter null: Ein Boss, der mehr "geheilt"
    /// wird, als je Schaden gefallen ist, haette negativen Fortschritt, und
    /// jede Prozentrechnung darueber waere unsinnig.
    fn heal_boss(&mut self) -> u32 {
        let Some(heal) = self.boss.boss_heal else {
            return 0;
        };
        let wirklich = self.undo_progress(heal.amount);
        self.boss_healed += wirklich;
        wirklich
    }

    /// Fortschritt am Ring zurueckdrehen — **die einzige Stelle, die ihn senkt**.
    ///
    /// Zwei Dinge drehen zurueck und haben nichts miteinander zu tun ausser
    /// dieser Zeile: die Boss-Heilung (Baustein 5, #218) als Strafe und das
    /// Stabilisieren (Baustein 6, #247) als Antwort des Spielers. Die
    /// Untergrenze gilt fuer beide — ein Boss mit negativem Fortschritt haette
    /// einen Ring ueber 100 %.
    ///
    /// Spiegelbild zu `undoProgress` auf der Spielseite, wo ein Prueffall die
    /// Schreibstellen zaehlt.
    fn undo_progress(&mut self, amount: u32) -> u32 {
        let wirklich = amount.min(self.damage);
        self.damage -= wirklich;
        wirklich
    }

    /// Die davonlaufende Reaktion zurueckdraengen (Baustein 6, Issue #247).
    ///
    /// **Schutz und Heilung um IHREN EIGENEN Betrag.** Eine zweite Zahl
    /// "stabilisiert um X" neben `shield.amount` waere dieselbe Groesse an zwei
    /// Orten. TEMPER bleibt draussen: Er wandelt ausgeteilten Schaden in
    /// Schutz — damit stabilisierte man DURCH Schaden machen, und die
    /// Bedingung des Bausteins waere aufgehoben.
    fn stabilise(&mut self, amount: u32) {
        if self.boss.runaway.is_none() {
            return;
        }
        self.stabilised += self.undo_progress(amount);
    }

    /// Die drei Wege der Koagulation, soweit sie beim Wirken greifen
    /// (GAME-DESIGN 4.1, Issue #79).
    ///
    /// Weg 3 wirkt hier nur als **Anschalter**: Die Umwandlung selbst
    /// geschieht spaeter, bei jedem ausgeteilten Schaden.
    fn apply_coagulation(&mut self, index: usize, spell: &Spell) {
        // Weg 1 — Schutzschicht aufbauen.
        if let Some(shield) = spell.shield {
            // **Stapeln bis zur Grenze, dann nicht weiter.** Ein vierter Druck
            // auf eine volle Schicht erneuert die Laufzeit, erhoeht aber den
            // Betrag nicht — sonst waere die Obergrenze wirkungslos.
            // **Die drei COAGULATION-Wege** (Issue #419). Einmal gerechnet:
            // `shield.amount` wird zweimal gelesen (Schicht und Stabilisierung).
            let dicke = (f64::from(shield.amount) * self.talent_shield_amount_factor) as u32;
            let grenze = shield.max_stacks + self.talent_shield_stacks_bonus;
            if self.shield_stacks >= grenze && self.sealed_vessel {
                // **THE SEALED VESSEL** (13.5, COAGULATION-Sechser): Was die
                // volle Schicht nicht mehr aufnimmt, geht als Schaden hinaus.
                //
                // Der Betrag, den dieser Druck BEIGETRAGEN HAETTE — also
                // `dicke` mitsamt Talent-Faktor, nicht `shield.amount`.
                // Er ersetzt nichts: Laufzeit und Stapelstand bleiben.
                self.deal(index, dicke, LogKind::Damage);
            }
            if self.shield_stacks < grenze {
                self.shield_stacks += 1;
                self.shield = self.shield.saturating_add(dicke);
                // **Der Schutz draengt die Reaktion zurueck** (Baustein 6,
                // #247), um seinen EIGENEN Betrag. Nur beim Stapeln, nicht beim
                // blossen Erneuern — sonst waere die Obergrenze wirkungslos.
                // Mit der GEHOBENEN Dicke — sonst stabilisiert die Schicht
                // nach dem alten Wert (beides gruen, beides falsch).
                self.stabilise(dicke);
            }
            // **Dauer UND Wakeup mit demselben Wert** (#419). Wer nur die Dauer
            // hebt, bekommt eine Schicht, die im Simulator laenger haelt als im
            // Spiel — mein DECAY-Fehler, eine Schule weiter.
            let dauer =
                (f64::from(shield.duration_ms) * self.talent_shield_duration_factor) as Millis;
            self.shield_expires_at = Some(self.time + dauer);
            self.queue.push(self.time + dauer, EventKind::Wakeup);
            // **Kein eigener Cast-Eintrag hier** (Issue #151). Er stand bis
            // dahin daneben und ergab fuer CRUST ZWEI Zeilen: diese und die
            // regulaere am Ende von `cast`. Das Spiel schreibt nur eine —
            // gefunden, weil der Prueffall die ZAHL je Art vergleicht und
            // nicht ihr Vorkommen.
            //
            // Der Aufbau der Schicht braucht auch keine eigene Art: Was sie
            // leistet, steht in `ShieldAbsorb`, sobald sie etwas auffaengt.
        }

        // Weg 2 — Heilung.
        //
        // **Der Betrag steht beim WIRKEN fest** (MEND, Issue #481). Fehlt dem
        // Spieler nichts, bleibt es beim Grundbetrag; je mehr fehlt, desto mehr
        // kommt dazu. Die Reihenfolge ist die Aussage: erst der Betrag, dann
        // die Verteilung. Anders herum muesste jeder Heil-Takt den Zustand von
        // damals kennen — und ein Zauber, der ueber sechs Sekunden heilt,
        // wuerde unterwegs staerker, waehrend der Boss zuschlaegt.
        // **Die Schutzschicht geht als Ganzes ueber** (SUTURE, Issue #481). Sie
        // wird verbraucht, BEVOR der Betrag feststeht — stuende sie noch, wenn
        // die Heilung ankommt, gaebe der Zauber beides.
        let aus_schicht = spell
            .heal
            .filter(|h| h.from_shield_percent > 0)
            .map_or(0, |h| {
                self.verbrauche_schicht_fuer_heilung(h.from_shield_percent)
            });
        let gewirkter_betrag = spell.heal.map_or(0, |heal| {
            heal.amount + self.fehlend_prozent() * heal.per_missing_percent + aus_schicht
        });
        // **Heilung ueber Zeit laeuft an, statt sofort anzukommen** (POULTICE,
        // Issue #481). Sie ersetzt die Sofortheilung fuer diesen Zauber; beides
        // zugleich waere doppelt.
        if let Some(heal) = spell.heal.filter(|h| h.over_ms > 0) {
            let abstand = heal.tick_interval_ms.max(TICK_MS);
            self.heal_over_time_amount = gewirkter_betrag;
            // Abgeleitet, nicht konfiguriert: Die Schrittzahl folgt aus Dauer
            // und Abstand. Eine eigene Zahl dafuer koennte still von beiden
            // abweichen. `build.rs` haelt fest, dass die Teilung aufgeht.
            self.heal_over_time_steps = (heal.over_ms / abstand).max(1);
            self.heal_over_time_interval_ms = abstand;
            self.heal_over_time_done = 0;
            self.heal_over_time_credited = 0;
            self.heal_over_time_generation = self.heal_over_time_generation.wrapping_add(1);
            self.queue.push(
                self.time + abstand,
                EventKind::HealTick {
                    generation: self.heal_over_time_generation,
                },
            );
        } else if spell.heal.is_some() {
            // **Nur bis zur vollen Anzeige.** Was darueber hinausginge, ist
            // keine Heilung, sondern verpufft — und es als Heilung zu zaehlen
            // liesse einen Lauf mit voller Anzeige nach mehr Wirkung aussehen,
            // als er gebraucht hat.
            let fehlend = self.vessel_max.saturating_sub(self.player_hp);
            let wirklich = gewirkter_betrag.min(fehlend);
            if wirklich > 0 {
                self.player_hp += wirklich;
                self.healed = self.healed.saturating_add(wirklich);
                // Auch die Heilung draengt zurueck — mit dem Betrag, der
                // WIRKLICH angekommen ist.
                self.stabilise(wirklich);
                self.log
                    .push(self.time, LogKind::Healed, index as u8, wirklich);
            }
        }

        // Weg 3 — die Umwandlung anschalten.
        if let Some(conversion) = spell.conversion {
            self.conversion_percent = conversion.share_percent;
            self.conversion_until = Some(self.time + conversion.duration_ms);
            self.queue
                .push(self.time + conversion.duration_ms, EventKind::Wakeup);
        }
        // PUTRID BLOOM (Issue #479) — die Grenze anheben. Liegende Stapel
        // bleiben liegen, wenn sie auslaeuft: Die Grenze entscheidet nur, ob
        // NACHGELEGT werden darf.
        if let Some(decay_cap) = spell.decay_cap {
            self.decay_cap_bonus = decay_cap.bonus as usize;
            self.decay_cap_until = Some(self.time + decay_cap.duration_ms);
            self.queue
                .push(self.time + decay_cap.duration_ms, EventKind::Wakeup);
        }
        // DECAY (Issue #479) — die Schwaechung anschalten. Die STAPELZAHL wird
        // hier NICHT festgehalten: Sie wird beim Treffer gelesen, sonst zaehlte
        // der Aufbau nach dem Wurf nicht mehr.
        //
        // **Das Wakeup gehoert dazu**, wie bei der Umwandlung darueber: Ohne
        // es wuerde der Ablauf erst beim naechsten Ereignis bemerkt, und die
        // Schwaechung haelte laenger als die Daten sagen — im Spiel dagegen
        // laeuft die Uhr Takt fuer Takt. Genau daran liefen die Seiten
        // auseinander.
        if let Some(decay_ward) = spell.decay_ward {
            self.decay_ward_permille = decay_ward.permille_per_stack;
            self.decay_ward_until = Some(self.time + decay_ward.duration_ms);
            self.queue
                .push(self.time + decay_ward.duration_ms, EventKind::Wakeup);
        }

        // Weg 3, andere Richtung — den Rueckschlag anschalten (RIPOSTE, #481).
        if let Some(reflect) = spell.reflect {
            self.reflect_percent = reflect.share_percent;
            self.reflect_until = Some(self.time + reflect.duration_ms);
            self.reflect_spell = index;
            self.queue
                .push(self.time + reflect.duration_ms, EventKind::Wakeup);
        }
    }

    /// Gibt einen Teil des eingehenden Schadens an den Boss zurueck (RIPOSTE).
    ///
    /// **Ganzzahlig und abgerundet**, wie bei der Umwandlung: Der Anteil steht
    /// als Prozentzahl in den Daten, damit hier `schaden * prozent / 100`
    /// moeglich ist.
    ///
    /// Der zurueckgegebene Schaden laeuft durch `deal` wie jeder andere — ein
    /// zweiter Weg an der Buchung vorbei waere eine Stelle zu viel.
    fn reflect_damage(&mut self, angekommen: u32) {
        if self.reflect_percent == 0 || angekommen == 0 {
            return;
        }
        if self.reflect_until.is_none_or(|bis| self.time > bis) {
            return;
        }
        let zurueck = angekommen * self.reflect_percent / 100;
        if zurueck == 0 {
            return;
        }
        self.reflected = self.reflected.saturating_add(zurueck);
        self.deal(self.reflect_spell, zurueck, LogKind::Damage);
    }

    /// Die drei Wirkungen aus GAME-DESIGN 4.1.
    ///
    /// Hier greift FIXATION wirklich in den **Kampfverlauf** ein — das ist,
    /// was sie von jeder anderen Schule unterscheidet.
    fn apply_control_effect(&mut self, control: Control) {
        // **Der Wirkungs-Weg wirkt am EINGANG** (Issue #419, FIXATION). Der
        // Faktor auf `amount` gilt damit fuer alle fuenf Wirkungen und fuer
        // `AllThree`, das drei davon nacheinander aufruft — eine Zeile statt
        // fuenf, die auseinanderlaufen koennen.
        let control = Control {
            amount: (f64::from(control.amount) * self.talent_control_effect_factor).round() as u32,
            // **Die Phasen-Senkung traegt denselben Faktor, aber ihre eigene
            // Zahl** (Issue #478). Bei 0 gilt `amount` — dieselbe Regel wie in
            // `combat.ts`, damit beide Seiten bei einem Zauber ohne
            // `amountPercent` dasselbe rechnen.
            amount_percent: if control.amount_percent > 0 {
                (f64::from(control.amount_percent) * self.talent_control_effect_factor).round()
                    as u32
            } else {
                (f64::from(control.amount) * self.talent_control_effect_factor).round() as u32
            },
            ..control
        };
        // **CAPSTONE loest alle drei auf einmal aus** (Issue #478). Ein `match`
        // waehlt genau einen Zweig — deshalb ruft `AllThree` die drei
        // einzelnen auf, statt ihren Inhalt zu wiederholen. Wer den Inhalt
        // kopierte, haette drei Stellen, die auseinanderlaufen koennen.
        if control.effect == ControlEffect::AllThree {
            for effekt in [
                ControlEffect::SlowCast,
                ControlEffect::DelayStrike,
                ControlEffect::HoldPhase,
            ] {
                self.apply_control_effect(Control {
                    effect: effekt,
                    ..control
                });
            }
            return;
        }

        match control.effect {
            // Ist oben schon behandelt; der Zweig haelt das `match`
            // vollstaendig, ohne einen Auffangzweig zu brauchen.
            ControlEffect::AllThree => {}
            // **Der Notausgang** (Issue #478, WEDGE): Punkte in Schaden.
            // Alle Punkte, nicht nur die Kosten — `cost` ist der
            // Mindesteinsatz, sonst braechte er bei vollem Vorrat genauso
            // viel wie bei knappem.
            ControlEffect::ConvertToDamage => {
                let rest = self.control;
                self.control = 0;
                self.control_spent = self.control_spent.saturating_add(rest);
                self.pending_control_damage = (control.cost + rest) * control.amount;
            }
            // Einen Boss-Zauber verlangsamen: Die Wirkzeit dehnt sich, und
            // damit wird das Fenster zum Unterbrechen groesser.
            ControlEffect::SlowCast => {
                if let Some(index) = self.casting {
                    self.cast_lands_at += control.amount;
                    self.queue.push(
                        self.cast_lands_at,
                        EventKind::BossCastLands {
                            index: u32::try_from(index).unwrap_or(u32::MAX),
                        },
                    );
                    self.log
                        .push(self.time, LogKind::ControlSpent, NO_SPELL, control.amount);
                }
            }
            // Einen Schlag verzoegern: Der naechste Zeitleisten-Eintrag rueckt
            // nach hinten.
            ControlEffect::DelayStrike => {
                // Nur der naechste Eintrag. Zweimal vor demselben Schlag
                // gewirkt summiert sich — das ist derselbe Schlag, nicht zwei.
                self.next_strike_delay = self.next_strike_delay.saturating_add(control.amount);
                self.schedule_next_timeline_step();
                self.log
                    .push(self.time, LogKind::ControlSpent, NO_SPELL, control.amount);
            }
            // Den naechsten Boss-Zauber unterbrechbar machen (PIN, #478).
            //
            // **Kein `schedule_next_timeline_step`.** Anders als DelayStrike
            // verschiebt PIN keinen Zeitpunkt — der Eintrag faellt, wann er
            // faellt, und aendert nur seine Art. Ein Neu-Ansteuern haette hier
            // nichts zu tun und koennte die Warteschlange stoeren.
            ControlEffect::MakeInterruptible => {
                self.pin_window = control.amount;
                self.log
                    .push(self.time, LogKind::ControlSpent, NO_SPELL, control.amount);
            }
            // Eine Phase hinauszoegern: Die Schwelle sinkt voruebergehend.
            //
            // **Sie nimmt einen Wechsel nicht zurueck.** Die Phase kann nur
            // steigen (`check_phase`) — laeuft die Fixierung waehrend einer
            // hoeheren Phase aus, faellt der Boss nicht zurueck.
            ControlEffect::HoldPhase => {
                // `amount_percent` statt `amount`: Bei CAPSTONE sind das 10
                // Prozentpunkte statt seiner 2000 Millisekunden. Bei jedem
                // anderen Zauber stehen in beiden Feldern dieselbe Zahl.
                self.phase_hold_percent = control.amount_percent;
                // **Dieselbe Frist fuer alle**, auch fuer CAPSTONE mit seinem
                // zweiten Betrags-Feld: Sie gehoert zur WIRKUNG, nicht zum
                // Zauber — sonst hiesse "hinauszoegern" bei jedem etwas
                // anderes.
                self.phase_hold_until = Some(self.time + PHASE_HOLD_DURATION_MS);
                self.queue
                    .push(self.time + PHASE_HOLD_DURATION_MS, EventKind::Wakeup);
                self.log.push(
                    self.time,
                    LogKind::ControlSpent,
                    NO_SPELL,
                    control.amount_percent,
                );
            }
        }
    }

    /// Kontroll-Punkte aufbauen und ausgeben (FIXATION, Issue #100).
    ///
    /// **Der Spieler entscheidet, wofuer** — deshalb steht die Wirkung am
    /// Zauber und nicht an einer Automatik. Reichen die Punkte nicht,
    /// geschieht nichts; der Zauber ist vertan, aber er nimmt sich nicht, was
    /// er nicht hat.
    /// Eine Umwandlung aus einer schulfremden Quelle (Issue #520, EFFLUX).
    ///
    /// **Eigene Verpackung, geteilte Senke** — wie im Spiel
    /// (`combat.ts`, `applyConversion`). `apply_fixation` verbraucht
    /// Kontroll-Punkte; ein SUBLIMATION-Zauber dort loeschte sie dem Spieler.
    /// Die Senke `pending_control_damage` dagegen ist schulneutral.
    ///
    /// **Ganze Sekunden**, wie drueben: `per_second` ist Schaden je Sekunde,
    /// und Millisekunden ergaeben Nachkommastellen an einer Stelle, an der
    /// GAME-DESIGN 18.4 ganze Zahlen verlangt.
    fn apply_conversion(&mut self, spell: &Spell, now: Millis) {
        let Some(convert) = spell.convert else {
            return;
        };
        let mut rest_ms: Millis = 0;
        for &bereit_ab in &self.cooldown_ready_at {
            if bereit_ab > now {
                rest_ms += bereit_ab - now;
            }
        }
        self.pending_control_damage = (rest_ms / 1000) * convert.per_second;
    }

    fn apply_fixation(&mut self, spell: &Spell) {
        if spell.control_gain > 0 {
            // Ohne Obergrenze: Kontroll-Punkte verfallen nicht (anders als
            // Hitze), und eine Grenze waere eine Design-Entscheidung, die
            // nicht gefallen ist. Faellt sie spaeter, ist es eine Zahl in den
            // Daten.
            // **Zauber-Buff UND Talent** (#419), multiplikativ: zwei Toepfe.
            let faktor = self.control_buffs.factor(self.time) * self.talent_control_gain_factor;
            let gewinn = (f64::from(spell.control_gain) * faktor).round() as u32;
            self.control = self.control.saturating_add(gewinn);
        }

        let Some(control) = spell.control else {
            return;
        };
        // **Der Preis-Weg TEILT** (#419) und wird auf mindestens 1 geklemmt —
        // ein Ausgeber, der nichts kostet, waere keiner.
        let preis =
            ((f64::from(control.cost) / self.talent_control_cost_factor).round() as u32).max(1);
        if self.control < preis {
            self.control_failed += 1;
            return;
        }
        // **THE LONG GRIP** (13.5): Der ABZUG entfaellt, nicht die
        // Voraussetzung — die Pruefung oben laeuft unveraendert.
        if self.long_grip && self.free_control_spend_armed {
            self.free_control_spend_armed = false;
            self.free_control_spends += 1;
        } else {
            self.control -= preis;
        }
        self.control_spent = self.control_spent.saturating_add(preis);
        self.apply_control_effect(control);
    }

    /// Wandelt ausgeteilten Schaden in Schutz um, solange Weg 3 laeuft.
    ///
    /// **Ganzzahlig und abgerundet.** Der Anteil steht als Prozentzahl in den
    /// Daten (`share_percent`), damit hier eine ganzzahlige Rechnung moeglich
    /// ist: `schaden * prozent / 100`. Ein Bruch als Faktor muesste von beiden
    /// Seiten unabhaengig gerundet werden — genau daran laufen zwei
    /// Umsetzungen auseinander (GAME-DESIGN 18.4).
    fn convert_damage_to_shield(&mut self, damage: u32) {
        if self.conversion_percent == 0 || damage == 0 {
            return;
        }
        if self.conversion_until.is_none_or(|bis| self.time > bis) {
            return;
        }
        // **Der Faktor aus `convert_buffs`** (QUICKSILVER, E10). Gelesen bei
        // JEDEM Schaden, solange die Umwandlung laeuft — nur deshalb traegt
        // ein zeitlich begrenzter Faktor hier. Spiegelbild von
        // `Combat.applyConversion` im Spiel.
        let satz = (f64::from(self.conversion_percent) * self.convert_buffs.factor(self.time))
            .round() as u32;
        let gewonnen = damage * satz / 100;
        if gewonnen == 0 {
            return;
        }
        self.shield = self.shield.saturating_add(gewonnen);
        self.converted = self.converted.saturating_add(gewonnen);
        self.log
            .push(self.time, LogKind::ConvertedToShield, NO_SPELL, gewonnen);
    }

    /// Laesst die Schutzschicht und die Umwandlung ablaufen.
    ///
    /// Steht bei den passiven Wirkungen, weil beides allein an der Uhr haengt —
    /// wie die Energieleiste des Bosses.
    /// Legt Zersetzungs-Stapel an (PUTREFACTION, GAME-DESIGN 4.1).
    ///
    /// **Die Obergrenze verbraucht nichts** — eine Wirkung darueber erneuert
    /// insbesondere nicht die Uhr des aeltesten Stapels.
    fn apply_putrefaction(&mut self, spell: &Spell) {
        self.add_decay_stacks(spell.decay_stacks);
    }

    /// Stapel legen — die **eine** Stelle, an der das geschieht (Issue #498).
    ///
    /// Bis ROT gab es nur einen Weg: beim Wirken, ueber `spell.decay_stacks`.
    /// Ein Effekt ueber Zeit legt sie jetzt auch, ueber `dot.decay_per_tick`.
    /// Beide teilen sich diese Funktion, damit Obergrenze, Verfallszeit und
    /// die Weckuhr **nicht zweimal** dastehen.
    ///
    /// Spiegelbild zu `addDecayStacks` in `src/core/combat.ts`.
    fn add_decay_stacks(&mut self, anzahl: u32) {
        // **Der Aufbau-Weg wirkt HIER** (#419), nicht an den zwei Aufrufern:
        // `decay.stacks` und `dot.decay_per_tick` laufen beide durch diese
        // Stelle. Abgerundet — aufrunden hiesse, dass jeder erste Rang sofort
        // einen Stapel schenkt.
        let anzahl = (f64::from(anzahl) * self.talent_decay_stack_factor) as u32;
        if anzahl == 0 {
            return;
        }
        for _ in 0..anzahl {
            if self.decay_count >= self.decay_cap() {
                break;
            }
            // **Der Haltbarkeits-Weg** (#419) — er verlaengert die Uhr JEDES neu
            // gelegten Stapels; liegende bleiben, wie sie sind.
            let haltbar =
                (f64::from(DECAY_STACK_DURATION_MS) * self.talent_decay_duration_factor) as Millis;
            self.decay_expiry[self.decay_count] = self.time + haltbar;
            self.decay_count += 1;
            self.queue
                .push(self.time + DECAY_STACK_DURATION_MS, EventKind::Wakeup);
        }
        self.update_vulnerability();
    }

    /// Laesst abgelaufene Stapel verfallen — **den aeltesten zuerst**.
    /// Abgelaufene Verstaerkungen wegraeumen und den Faktor nachziehen (#420).
    ///
    /// **Die zweite Zeile ist der ganze Punkt.** `Buffs::expire` raeumt nur den
    /// eigenen Zustand; ohne das Nachziehen bliebe `factors.buffs` auf dem
    /// alten Wert stehen, und eine abgelaufene Verstaerkung wirkte fuer den
    /// Rest des Kampfes weiter.
    fn expire_buffs(&mut self) {
        let now = self.time;
        // Alle fuenf, nicht nur die Schadens-Verstaerkungen (#489).
        let masken = [
            self.control_buffs.expire(now),
            self.resource_buffs.expire(now),
            self.tick_buffs.expire(now),
            self.stack_buffs.expire(now),
            self.cd_reduce_buffs.expire(now),
            self.convert_buffs.expire(now),
            self.cap_buffs.expire(now),
            self.buffs.expire(now),
        ];
        for maske in masken {
            self.log_expired(maske);
        }

        // **Die achte liest ihren Rueckgabewert schon immer.** `== 0` heisst
        // weiterhin „nichts abgelaufen" — die Maske hat dieselbe Null wie die
        // fruehere Anzahl, deshalb bleibt der Vergleich richtig (#682).
        if masken[7] == 0 {
            return;
        }
        self.factors.buffs = self.buffs.factor(now);
    }

    /// Schreibt je abgelaufener Verstaerkung einen Eintrag der Art 7 (#682).
    ///
    /// Steht hier und nicht in [`Buffs::expire`]: Die Klasse laeuft je Takt,
    /// darf nichts erzeugen und kennt das Protokoll nicht. Sie gibt eine
    /// Maske zurueck, der Aufrufer schreibt.
    ///
    /// Der Wert ist die ART, nicht die Anzahl — sie bewegt sich, und ein
    /// Eintrag, der immer dieselbe Zahl traegt, ist von einem festen Wert
    /// nicht zu unterscheiden.
    fn log_expired(&mut self, maske: u32) {
        if maske == 0 {
            return;
        }
        for kind in 0..BUFF_KIND_CAPACITY {
            if maske & (1u32 << kind) == 0 {
                continue;
            }
            self.log.push(
                self.time,
                LogKind::BuffExpire,
                NO_SPELL,
                u32::try_from(kind).unwrap_or(0),
            );
        }
    }

    /// Die Stapel-Grenze, die JETZT gilt (PUTRID BLOOM, Issue #479).
    ///
    /// **Gedeckelt auf die Kapazitaet** — darueber hinaus gaebe es keinen Platz
    /// im Feld. Das Spiel rechnet dieselbe Zeile (`combat.ts`, `decayCap`).
    fn decay_cap(&self) -> usize {
        let offen = matches!(self.decay_cap_until, Some(bis) if self.time <= bis);
        let gehoben = MAX_DECAY_STACKS + if offen { self.decay_cap_bonus } else { 0 };
        gehoben.min(DECAY_STACK_CAPACITY)
    }

    /// Laeuft gerade irgendein DoT auf dem Ziel?
    ///
    /// **Als eigene Abfrage und nicht inline**, weil `slow-rot` sie braucht und
    /// die Bedingung sonst an der Verfallsstelle stuende — dort, wo sie beim
    /// naechsten Umbau uebersehen wird. Gegenstueck: `hasRunningDot` im Spiel.
    fn has_running_dot(&self) -> bool {
        self.dot_expires_at
            .iter()
            .any(|ende| matches!(ende, Some(bis) if *bis > self.time))
    }

    fn expire_putrefaction(&mut self) {
        // **SLOW ROT** (13.5, PUTREFACTION-Sechser): Solange ein DoT laeuft,
        // verfaellt kein Zersetzungs-Stapel.
        //
        // **Aufgeschoben, nicht aufgehoben.** Die Verfallszeiten bleiben
        // stehen; endet der letzte DoT, holt der naechste Durchlauf alles nach,
        // was inzwischen faellig wurde. Ein Zuruecksetzen der Zeiten waere eine
        // Verlaengerung und damit ein zweiter Effekt.
        if self.slow_rot && self.has_running_dot() {
            return;
        }
        let mut verfallen = 0;
        while verfallen < self.decay_count && self.decay_expiry[verfallen] <= self.time {
            verfallen += 1;
        }
        if verfallen == 0 {
            return;
        }
        for i in verfallen..self.decay_count {
            self.decay_expiry[i - verfallen] = self.decay_expiry[i];
        }
        self.decay_count -= verfallen;
        self.update_vulnerability();
    }

    /// Setzt den Synergie-Faktor, wenn diesem Zauber ein Fenster offen steht
    /// (GAME-DESIGN 5, #318).
    ///
    /// Der Bonus liegt **neben** dem Stapel-Bonus, nicht darin: `vulnerability`
    /// hebt den Schaden aller Quellen, sobald Stapel liegen. Eine Gegenprobe,
    /// die den Stapelstand mitvariiert, misst also die Stapel und nicht die
    /// Synergie.
    ///
    /// **Hier stand bis Issue #484, der Bonus liege neben
    /// `per_decay_stack_percent`.** Das war falsch: Dieses Feld wird auf keiner
    /// der beiden Seiten je auf den Schaden angewandt — es gibt kein „neben",
    /// weil es nichts gibt, wozu es danebenlaege. Der Satz stand an genau der
    /// Stelle, an der jemand nachsieht.
    fn apply_synergy_bonus(&mut self, index: usize) {
        let endet = self.synergy_window_ends[index];
        let offen = endet != 0 && self.time <= endet;
        // **Ganzzahlig bis zur letzten Teilung** — dieselbe Rechnung wie im
        // Spiel (18.4).
        self.factors.synergy = if offen {
            f64::from(100 + SYNERGY_BONUS_PERCENT) / 100.0
        } else {
            1.0
        };
        if offen {
            self.synergy_used += 1;
            // **Das Fenster ist verbraucht.** Sonst zaehlte jede weitere
            // Wirkung darin erneut, und die Ausbeute stiege ueber die Zahl der
            // Gelegenheiten.
            self.synergy_window_ends[index] = 0;
        }
    }

    /// Oeffnet dem Partner ein Fenster (#318).
    ///
    /// **Die Laenge steht in WARTEZEITEN**, nicht in Millisekunden. Die
    /// entstehen hier, durch dieselbe `hasted()`-Stelle wie jede andere Zeit —
    /// GAME-DESIGN 327: "In Wartezeiten gerechnet, nicht in Sekunden, sonst
    /// verschiebt sich das Verhaeltnis bei hohem Tempo."
    fn open_synergy_window(&mut self, spell: &Spell) {
        if spell.synergy_partner.is_empty() || spell.synergy_window_gcds == 0 {
            return;
        }
        let Some(partner) = SPELLS.iter().position(|s| s.id == spell.synergy_partner) else {
            return;
        };
        // **Die Klammerung ist der Unterschied** (Issue #456, gefunden bei
        // #420): `gcds * hasted(GCD)` und `hasted(gcds * GCD)` sind bei
        // ganzzahliger Rechnung NICHT dasselbe. `hasted` teilt und rastet auf
        // die 10-ms-Marke auf; zwei aufgerundete Wartezeiten sind laenger als
        // eine aufgerundete Doppel-Wartezeit.
        //
        // Bei CANKER (zwei Wartezeiten) fehlten dem Spieler dadurch zehn
        // Millisekunden, und der zweite Zauber verpasste das Fenster, das ihm
        // nach GAME-DESIGN 327 zusteht — aus "zwei Zauber" wurde einer.
        //
        // Bei Tempo 0, 376 und 500 Promille geht die Rundung zufaellig auf.
        // Ein Prueffall, der einen dieser Werte waehlt, ist gruen ohne etwas
        // zu belegen.
        let laenge = Millis::from(spell.synergy_window_gcds) * self.hasted(GCD_MS);
        self.synergy_window_ends[partner] = self.time + laenge;
        // **Wird der Partner im Fenster ueberhaupt bereit?** Ein Fenster, das
        // aufgeht waehrend er auf Abklingzeit liegt und bis zum Ende nicht
        // bereit wird, ist keine Gelegenheit — die Zahl bestrafte den Spieler
        // sonst fuer eine Abklingzeit statt fuer sein Spiel.
        if self.cooldown_ready_at[partner].saturating_sub(self.time) <= laenge {
            self.synergy_available += 1;
        }
    }

    /// Die Summe aller offenen Fensterenden — fuer die Pruefsumme (#318).
    ///
    /// **Eine Summe statt einer Schleife im Takt-Vergleich**: GAME-DESIGN 18.2
    /// erlaubt im laufenden Bild keine Erzeugung, und die Spielseite rechnet
    /// dieselbe Summe.
    fn synergy_window_sum(&self) -> f64 {
        let mut summe = 0.0;
        for endet in self.synergy_window_ends.iter() {
            if *endet != 0 {
                summe += *endet as f64;
            }
        }
        summe
    }

    /// Rechnet die Verwundbarkeit aus den liegenden Stapeln.
    ///
    /// **Ganzzahlig bis zur letzten Teilung** — dieselbe Rechnung wie im Spiel
    /// (18.4).
    ///
    /// # Ihr Ergebnis erreicht heute KEINE Schadensrechnung
    ///
    /// Gemessen mit einer Mutation: Setzt man hier `99.0` statt der Rechnung,
    /// bleiben bei 121 gelaufenen Suiten **null** Fälle rot. Dieselbe Mutation
    /// in [`Self::apply_decay_stack_bonus`] färbt 274 — der Kontrollversuch
    /// schliesst also aus, dass die Verwundbarkeit gar nicht geprüft würde.
    ///
    /// **Die beiden Zahlen altern, das Verhältnis nicht.** Beim Schreiben
    /// dieses Kommentars standen 116 Suiten und 262 rote Fälle da; zwei
    /// gemergte Zauber später sind es 121 und 274. Die Suitenzahl ist die
    /// Zusicherung, dass der Lauf stattgefunden hat — ein Bau, der gar nicht
    /// läuft, meldet null rote Fälle genauso wie ein grüner. Was trägt, ist
    /// **null gegen alles andere**.
    ///
    /// Der Grund ist die Reihenfolge: `apply_decay_stack_bonus` setzt dasselbe
    /// Feld **unbedingt neu** und rechnet es von Grund auf, und sie läuft vor
    /// jeder Schadensrechnung — in `resolve_player_cast` wie in `dot_tick`.
    /// Der hier gesetzte Wert steht nur zwischen einem Stapel-Ereignis und dem
    /// nächsten Wurf, und in dieser Zeit fällt kein Schaden.
    ///
    /// # Warum sie trotzdem bleibt
    ///
    /// Auf der Spiel-Seite hält `updateVulnerability` den Getter
    /// `combat.vulnerability` aktuell, den zwei Prüffälle lesen
    /// (`putrefaction.test.ts`, `rot-decay-per-tick.test.ts`). Sie zu
    /// entfernen wäre ein Eingriff auf beiden Seiten für null Gewinn —
    /// entschieden nach der Messung, nicht nach Gefühl (Issue #480).
    ///
    /// **Wer hier etwas ändert, ändert nichts am Schaden.** Wer den Schaden
    /// treffen will, ändert `apply_decay_stack_bonus`.
    fn update_vulnerability(&mut self) {
        // Der Faktor wiegt den EINZELNEN Stapel schwerer, er legt keinen
        // dazu — die Stapelzahl bleibt unberuehrt (Issue #489, VIRULENCE).
        let je_stapel = (f64::from(DECAY_STACK_PERCENT)
            * (self.stack_buffs.factor(self.time) * self.talent_decay_effect_factor))
            .round() as u32;
        let prozent = 100 + self.decay_count as u32 * je_stapel;
        self.factors.vulnerability = f64::from(prozent) / 100.0;
    }

    /// Die Verwundbarkeit **fuer die Wirkung eines bestimmten Zaubers** — mit
    /// seinem eigenen Zuschlag je Stapel (`per_decay_stack_percent`, #484).
    ///
    /// # Warum ADDITIV im selben Topf
    ///
    /// GAME-DESIGN 1405 fuehrt die Zersetzungs-Stapel ausdruecklich im
    /// additiven Topf. Ein neunter Multiplikator nur fuer einen Zauber waere
    /// der Fehler, den Diablo IV mit der Verwundbarkeit gemacht hat.
    ///
    /// # Warum JE WIRKUNG
    ///
    /// Der Topf ist global, das Feld gilt fuer EINEN Zauber. Deshalb hier vor
    /// der Rechnung gesetzt und beim naechsten Zauber ueberschrieben — genau
    /// wie `resistance` (#328). Dieselbe Rechnung wie im Spiel
    /// (`applyDecayStackBonus`), ganzzahlig bis zur letzten Teilung (18.4).
    /// Ueber alle Braende laufen, die **gerade brennen** (Issue #517).
    ///
    /// Der Baustein, den EMBER und PYROCLASM sich teilen — nicht ein
    /// Rechenschritt, sondern diese Iteration. Bis hierher wurde
    /// `dot_expires_at` ausschliesslich je Index gelesen und geschrieben.
    /// Die Schutzschicht laenger halten (HARDEN, Issue #523).
    ///
    /// Der Ablauf rueckt nach hinten, der aufgebrauchte Betrag bleibt. Ein
    /// abgelaufener oder nie gesetzter Schild wird NICHT wiederbelebt —
    /// verlaengern ist nicht neu setzen.
    ///
    /// **Es gibt hier keinen Takt.** Bei den Braenden musste die Tick-Folge
    /// mitgedacht werden (#517); der Schild haelt nur Betrag und Ablauf. Das
    /// steht als Satz da, damit das Fehlen nicht wie ein Versehen aussieht.
    fn extend_shield(&mut self, ms: Millis) {
        if ms == 0 || self.shield == 0 {
            return;
        }
        let Some(expires) = self.shield_expires_at else {
            return;
        };
        if self.time > expires {
            return;
        }
        self.shield_expires_at = Some(expires + ms);
    }

    /// Die Zersetzungs-Stapel laenger halten (SEPSIS, Issue #523).
    ///
    /// **Alle, und um denselben Betrag.** `decay_expiry` ist aufsteigend
    /// sortiert, und das Nachruecken beim Verfall setzt das voraus. Ein
    /// Zuschlag auf nur einen Eintrag stellte einen spaeteren Ablauf vor einen
    /// frueheren — dann verfallen Stapel in falscher Reihenfolge, ohne dass
    /// die Stapelzahl es verraet. Dieselbe Regel wie im Spiel.
    fn extend_decay_stacks(&mut self, ms: Millis) {
        if ms == 0 {
            return;
        }
        for i in 0..self.decay_count {
            self.decay_expiry[i] += ms;
        }
    }

    fn active_dot_indices(&self) -> Vec<usize> {
        (0..SPELL_COUNT)
            .filter(|&i| SPELLS[i].dot.is_some() && self.dot_active(i))
            .collect()
    }

    /// Jeden laufenden Brand um `ticks` Tick-Intervalle verlaengern (EMBER).
    ///
    /// # Die Tick-Folge bleibt unberuehrt — hier wie im Spiel
    ///
    /// Der Simulator plant Ticks ueber die Ereignis-Schlange, nicht ueber
    /// einen Speicher: Beim Abarbeiten prueft er `next <= expires` und legt
    /// nur dann den naechsten. Eine laengere Laufzeit ergibt damit von selbst
    /// einen Tick mehr — dieselbe Wirkung wie das unberuehrte
    /// `dotNextTickAtMs` im Spiel, ueber einen anderen Weg.
    fn extend_active_dots(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }
        for i in self.active_dot_indices() {
            let Some(dot) = SPELLS[i].dot.as_ref() else {
                continue;
            };
            let dazu = ticks * self.dot_interval(dot);
            if let Some(expires) = self.dot_expires_at[i] {
                self.dot_expires_at[i] = Some(expires + dazu);
            }
        }
    }

    /// Alle laufenden Braende verbrauchen und ihren Rest als Schaden geben
    /// (PYROCLASM).
    ///
    /// Restdauer mal Schaden je Tick — genau das, was sie noch gemacht
    /// haetten. Nach der Erneuerungsregel ist Restzeit verloren, wenn zu frueh
    /// erneuert wird; dieser Zauber kehrt dieselbe Regel um.
    ///
    /// Ganzzahlig bis zur letzten Teilung (18.4): erst die offenen Ticks, dann
    /// der Schaden — dieselbe Reihenfolge wie im Spiel.
    fn consume_active_dots(&mut self) -> u32 {
        let mut summe = 0;
        for i in self.active_dot_indices() {
            let Some(dot) = SPELLS[i].dot.as_ref() else {
                continue;
            };
            let rest = self.dot_expires_at[i].map_or(0, |e| e.saturating_sub(self.time));
            let offene_ticks = rest / self.dot_interval(dot);
            let factors = self.factors_for(false);
            summe += compute_damage(dot.tick_damage * offene_ticks, &factors);
            // Der Brand ist weg. Ein Tick-Ereignis, das noch in der Schlange
            // liegt, verfaellt beim Abarbeiten von selbst — `dot_active` ist
            // dann falsch.
            self.dot_expires_at[i] = None;
        }
        summe
    }

    fn apply_decay_stack_bonus(&mut self, index: usize) {
        let pro_stapel = SPELLS[index].per_decay_stack_percent;
        // **Der Faktor aus `stack_buffs` gehoert auch hierher** (Issue #489).
        // Diese Methode SETZT die Verwundbarkeit neu — ohne ihn wuerfe sie weg,
        // was `update_vulnerability` eingerechnet hat, und eine
        // Stapel-Verstaerkung verschwaende beim naechsten Wirken spurlos.
        //
        // Gemessen auf der Spiel-Seite beim Zusammenfuehren von #484 und #489:
        // `buff-targets` mass 1,06 gegen 1,06. Beide Vorhaben waren fuer sich
        // gruen; erst zusammen zeigte sich, dass das eine die Wirkung des
        // anderen ueberschreibt. Hier stand derselbe Fehler.
        //
        // Ganzzahlig bis zur letzten Teilung (18.4) — auf die Prozentpunkte,
        // nicht auf das Ergebnis, wie in `update_vulnerability`.
        let je_stapel = (f64::from(DECAY_STACK_PERCENT + pro_stapel)
            * (self.stack_buffs.factor(self.time) * self.talent_decay_effect_factor))
            .round() as u32;
        let prozent = 100 + self.decay_count as u32 * je_stapel;
        self.factors.vulnerability = f64::from(prozent) / 100.0;
    }

    /// Die Kontroll-Punkte der FIXATION (fuer die APL, Issue #526).
    #[must_use]
    pub fn control_points(&self) -> u32 {
        self.control
    }

    /// Wie oft THE LONG GRIP einen Ausgeber umsonst gemacht hat (13.5).
    #[must_use]
    pub fn free_control_spends(&self) -> u32 {
        self.free_control_spends
    }

    /// Wie viele Zersetzungs-Stapel gerade auf dem Boss liegen.
    #[must_use]
    pub fn decay_stacks(&self) -> usize {
        self.decay_count
    }

    /// Wann der Stapel an Platz `index` verfaellt. 0 fuer unbelegt.
    #[must_use]
    pub fn decay_expiry_at(&self, index: usize) -> Millis {
        if index < self.decay_count {
            self.decay_expiry[index]
        } else {
            0
        }
    }

    fn expire_fixation(&mut self) {
        // Die Frist ist um, ohne dass ein Wechsel stattgefunden hat.
        if self.phase_hold_until.is_some_and(|bis| self.time > bis) {
            self.phase_hold_percent = 0;
            self.phase_hold_until = None;
        }
    }

    fn expire_coagulation(&mut self) {
        if self.shield_expires_at.is_some_and(|bis| self.time > bis) {
            self.shield = 0;
            self.shield_stacks = 0;
            self.shield_expires_at = None;
        }
        if self.conversion_until.is_some_and(|bis| self.time > bis) {
            self.conversion_percent = 0;
            self.conversion_until = None;
        }
        if self.reflect_until.is_some_and(|bis| self.time > bis) {
            self.reflect_percent = 0;
            self.reflect_until = None;
        }
    }

    /// Schaden am Spieler, **nach** WARD (GAME-DESIGN 6.1).
    ///
    /// Der Tod wird hier festgestellt und nicht erst in der Kampfschleife: Ein
    /// Spieler mit null Lebenspunkten, der noch einen Takt weiterwirkt, waere
    /// eine stille Abweichung zwischen beiden Umsetzungen.
    fn hit_player(&mut self, raw: u32) {
        if self.died_at.is_some() {
            return;
        }
        // WARD aus der getragenen Ausruestung kommt zur Basis dazu (#282, AC4).
        // **DECAY geht in DIESELBE Promille-Summe** (Issue #479) — und damit
        // unter denselben Deckel. Die Schwaechung ist kein eigener Faktor
        // neben WARD, sondern mehr WARD; sonst liesse sich `ward_cap_permille`
        // ueber die Faeulnis umgehen.
        //
        // **Die Stapelzahl wird HIER gelesen, nicht beim Wirken.**
        // **Erst verfallen lassen, dann zaehlen** (Issue #479, Weg c).
        //
        // Das Spiel ruft `expirePutrefaction` VOR der Zeitleiste, der
        // Simulator DANACH (fight.rs, Ereignisschleife). Bei Gleichstand —
        // 12000 ms ist zugleich Verfallszeit und Landepunkt — zaehlte der
        // Simulator hier einen Stapel mehr.
        //
        // Der Aufruf steht LOKAL an der Lesestelle, nicht in der Schleife:
        // Eine Umstellung dort bricht `parity_rot_decay`, weil ROT seine
        // Stapel ueber DoT-Ticks legt und die andere Reihenfolge erwartet.
        // Gemessen, nicht vermutet.
        self.expire_putrefaction();
        let decay_permille = match self.decay_ward_until {
            Some(bis) if self.time <= bis => {
                self.decay_ward_permille * u32::try_from(self.decay_count).unwrap_or(0)
            }
            _ => 0,
        };
        let nach_ward = PLAYER.incoming_with_gear(raw, self.worn.ward + decay_permille);

        // **Die Schutzschicht nimmt den Schlag ZUERST** (COAGULATION, Weg 1).
        //
        // Das ist die ganze Mechanik: Sie muss VOR dem Schlag stehen. Eine
        // Schicht, die erst danach greift, waere eine Heilung mit anderem
        // Namen — und die gibt es in dieser Schule als eigenen Weg.
        //
        // **Nach WARD, nicht davor.** WARD ist eine Eigenschaft des Spielers
        // und mindert jeden Schlag; die Schicht faengt auf, was danach uebrig
        // bleibt. Andersherum gerechnet wuerde WARD den Schutz mitmindern und
        // damit doppelt zaehlen.
        let abgefangen = self.shield.min(nach_ward);
        if abgefangen > 0 {
            self.shield -= abgefangen;
            self.shield_absorbed = self.shield_absorbed.saturating_add(abgefangen);
            self.log
                .push(self.time, LogKind::ShieldAbsorb, NO_SPELL, abgefangen);
            if self.shield == 0 {
                // Aufgebraucht: Die Stapel sind weg, nicht nur der Betrag.
                self.shield_stacks = 0;
                self.shield_expires_at = None;
            }
        }

        let angekommen = nach_ward - abgefangen;
        self.damage_taken = self.damage_taken.saturating_add(angekommen);
        self.player_hp = self.player_hp.saturating_sub(angekommen);
        self.log
            .push(self.time, LogKind::PlayerDamage, NO_SPELL, angekommen);
        if self.player_hp == 0 {
            self.died_at = Some(self.time);
            self.log
                .push(self.time, LogKind::PlayerDeath, NO_SPELL, self.damage_taken);
        }
        // **Der Rueckschlag haengt am ANGEKOMMENEN Schaden**, nicht am rohen
        // (RIPOSTE, Issue #481). Ein Schlag, den die Schicht ganz auffaengt,
        // gibt nichts zurueck: Sonst waeren Schicht und Rueckschlag zusammen
        // doppelt stark.
        self.reflect_damage(angekommen);
    }

    /// Bricht einen laufenden Boss-Zauber ab (GAME-DESIGN 11.2, Baustein 9).
    ///
    /// Gibt zurueck, ob ueberhaupt etwas zu unterbrechen war. Ein Unterbrechen
    /// ins Leere ist **kein Fehler**, sondern eine vertane Abklingzeit — der
    /// Zauber wird gewirkt, er trifft nur nichts.
    fn interrupt_boss_cast(&mut self) -> bool {
        if self.casting.take().is_some() {
            self.interrupts_landed += 1;
            true
        } else {
            false
        }
    }

    fn apply(&mut self, kind: EventKind) {
        match kind {
            EventKind::DotTick { spell, generation } => self.dot_tick(spell as usize, generation),
            EventKind::ResourceDecay => self.decay(),
            EventKind::HealTick { generation } => self.heal_tick(generation),
            EventKind::EnergyTick => self.energy_tick(),
            EventKind::DoomTick => self.doom_tick(),
            EventKind::StackTick => self.stack_tick(),
            EventKind::RunawayTick => self.runaway_tick(),
            EventKind::AfflictionTick => self.affliction_tick(),
            EventKind::TimelineStep { index } => self.timeline_step(index as usize),
            EventKind::BossCastLands { index } => self.boss_cast_lands(index as usize),
            EventKind::PlayerCastLands { spell, generation } => {
                self.player_cast_lands(spell as usize, generation);
            }
            // Ein Weckruf traegt keine Wirkung. Er sorgt nur dafuer, dass der
            // Simulator zu diesem Zeitpunkt ueberhaupt aufwacht — weil dann eine
            // Abklingzeit abgelaufen ist oder der Sieg zu pruefen war.
            EventKind::Wakeup => {}
        }
    }

    /// Eine Zeit unter dem Einfluss des getragenen CATALYSIS (GAME-DESIGN 6.7).
    ///
    /// **Die einzige Stelle, an der Tempo auf eine Zeit trifft.** Jeder Wirkort
    /// geht durch sie; wer einen neuen hinzufuegt, ruft sie auf und bekommt das
    /// Takt-Raster und die Untergrenze geschenkt.
    ///
    /// Ohne getragene Ausruestung gibt sie `base_ms` unveraendert zurueck —
    /// siehe [`hasted_ms`], dort haengt AC7 daran.
    /// Die Laenge der globalen Wartezeit — mit dem Satz-Effekt `whole-work`.
    ///
    /// **An EINER Stelle abgeleitet, nicht an jeder Ausloesestelle gerechnet.**
    /// Die Wartezeit wird an zwei Stellen gesetzt; ein Abzug an jeder davon
    /// waeren zwei Stellen, an denen er fehlen kann. Im Spiel steht dieselbe
    /// Ableitung in `Combat.setSetEffects` ueber `GlobalCooldown.setLength`.
    /// Die Wartezeit-Laenge fuer Prueffaelle.
    ///
    /// `gcd_length` ist privat, weil sie nur intern gebraucht wird — ein
    /// Prueffall, der die Laenge selbst ablesen will, braucht trotzdem einen
    /// Weg dorthin. Ohne ihn koennte er sie nur ueber die Wurfzahl erschliessen,
    /// und dann waere nicht zu trennen, ob die Laenge oder die Abklingzeit die
    /// Bremse ist.
    #[must_use]
    pub fn gcd_length_for_test(&self) -> Millis {
        self.gcd_length()
    }

    fn gcd_length(&self) -> Millis {
        if self.whole_work {
            GCD_MS - Millis::from(WHOLE_WORK_GCD_REDUCTION_MS)
        } else {
            GCD_MS
        }
    }

    fn hasted(&self, base_ms: Millis) -> Millis {
        hasted_ms(base_ms, self.worn.catalysis)
    }

    /// Der Tick-Abstand eines Effekts ueber Zeit, unter dem Einfluss von Tempo.
    ///
    /// Steht als eigene Methode da, weil der Abstand an **vier** Stellen
    /// gebraucht wird: beim ersten Tick, beim naechsten, beim Zaehlen der durch
    /// Erneuern verlorenen Ticks — und in den Pruefungen. Vier Handabschriften
    /// derselben Rechnung waeren genau die Klasse Fehler, die #134, #138, #159
    /// und #173 hinterlassen haben.
    fn dot_interval(&self, dot: &Dot) -> Millis {
        // **Der Faktor TEILT hier, statt zu multiplizieren** (Issue #489,
        // FORGE). Er verkuerzt den ABSTAND, macht die Ticks also HAEUFIGER:
        // Ein Faktor 2 heisst "doppelt so oft", nicht "halb so oft".
        //
        // Die einzige der vier Zielgroessen, bei der das so ist. Wer die Zeile
        // nach dem Muster der anderen drei liest, dreht die Wirkung um — und
        // der Fehler faellt spaet auf, weil der Tick-SCHADEN unveraendert
        // bleibt und nur die Gesamtsumme wandert.
        let faktor = self.tick_buffs.factor(self.time);
        let roh = if faktor == 1.0 {
            dot.tick_interval_ms
        } else {
            (f64::from(dot.tick_interval_ms) / faktor).round() as Millis
        };
        self.hasted(roh)
    }

    /// Ein Kritwurf fuer **eine Wirkung** (GAME-DESIGN 6.5).
    ///
    /// > Echte Wahrscheinlichkeit pro Zauber, pro DoT-Tick und pro Heilung.
    /// > Kein Pech-Schutz, kein fester Startwert, keine feste Reihenfolge.
    ///
    /// # Wann gewuerfelt wird — und warum nicht oefter
    ///
    /// **Ein Wurf je Wirkung, nicht je Teilbetrag.** Ein Zauber mit Grund- und
    /// Entladeschaden kritet als Ganzes oder gar nicht. Das folgt derselben
    /// Festlegung, die weiter unten ein einziges Schaden-Ereignis je Wirkung
    /// meldet: Zwei Wuerfe waeren zwei Erzaehlungen desselben Treffers.
    ///
    /// # Ohne PURITY faellt gar kein Wurf
    ///
    /// [`crate::rng::RandomStream::chance`] schliesst bei einer
    /// Wahrscheinlichkeit von 0 kurz, **ohne den Strom zu drehen** — der
    /// Kommentar dort nennt genau diesen Grund. Damit bleibt der Ziehungszaehler
    /// unveraendert, und mit ihm die Zustands-Pruefsumme. **Daran haengt AC7**,
    /// und zwar an einer Stelle, die man leicht uebersieht: Ein Wurf, dessen
    /// Ergebnis feststeht, waere rechnerisch folgenlos und trotzdem sichtbar.
    ///
    /// # Der Strom kommt vom Aufrufer, seit Issue #602
    ///
    /// Bis zum 2026-08-07 zog diese Funktion **immer** aus `Stream::Crit` —
    /// auch fuer den Krit-Wurf eines DoT-Ticks. Beide Seiten taten das gleich,
    /// die Paritaet war intakt, und genau deshalb hat es niemand gesehen.
    ///
    /// GAME-DESIGN 18.4 verlangt „je einen eigenen Strom", und der Nebensatz
    /// sagt warum: „sonst verschiebt eine zusaetzliche Ziehung alles
    /// Nachfolgende." Zog ein Tick aus `Stream::Crit`, verschob jeder
    /// zusaetzliche Tick die Krit-Wuerfe aller Zauber. Die Regel schuetzt nicht
    /// die Paritaet, sondern die Vergleichbarkeit zweier Laeufe mit
    /// verschiedenen Rotationen.
    fn roll_crit(&mut self, stream: Stream) -> bool {
        self.rng.chance(stream, self.worn.crit_chance())
    }

    /// Die Faktoren fuer einen Treffer, je nachdem ob der Wurf sass.
    fn factors_for(&self, crit: bool) -> DamageFactors {
        if crit {
            DamageFactors {
                crit: self.worn.crit_factor(),
                ..self.factors
            }
        } else {
            self.factors
        }
    }

    fn dot_tick(&mut self, index: usize, generation: u16) {
        // Veraltet: Der Effekt wurde inzwischen erneuert, und `refreshMode:
        // "reset"` laesst die alte Anwendung verfallen (`STEP1-SPEC` 1.2).
        if generation != self.dot_generation[index] {
            return;
        }
        if !self.dot_active(index) {
            return;
        }
        let Some(dot) = SPELLS[index].dot else {
            return;
        };

        // **Ein eigener Wurf je Tick** (GAME-DESIGN 6.5: „pro Zauber, pro
        // DoT-Tick und pro Heilung"). Nicht der Wurf der Wirkung, die ihn
        // gesetzt hat — sonst kritete ein DoT ueber seine ganze Laufzeit
        // entweder immer oder nie, und der Glueckanteil, den 6.5 ausdruecklich
        // will, verschwaende sich in einer einzigen Entscheidung.
        // **Aus Stream::DotTick, nicht aus Stream::Crit** (Issue #602): Ein
        // Tick darf die Krit-Wuerfe der gewirkten Zauber nicht verschieben.
        let kritet = self.roll_crit(Stream::DotTick);
        // Auch ein Brand-Tick gehoert seinem Zauber und damit dessen Schule
        // (Issue #328). Ohne diese Zeile truege er den Widerstand des zuletzt
        // gewirkten Zaubers — nicht falsch gerechnet, sondern falsch
        // zugeordnet, und das verraet keine Summe.
        self.factors.resistance = self.resistance_by_spell[index];
        // Der Stapel-Zuschlag DIESES Zaubers (#484) — aus demselben Grund wie
        // der Widerstand daneben: Er gehoert der Wirkung, nicht dem Kampf.
        self.apply_decay_stack_bonus(index);
        let factors = self.factors_for(kritet);
        let tick_damage = self.deal(
            index,
            compute_damage(dot.tick_damage, &factors),
            LogKind::DotTick,
        );
        // Ein Tick ist **ein** Ereignis, nicht zwei. Wer allen Schaden zaehlen
        // will, meldet sich fuer beide Arten an — so bleibt die Folge eindeutig.
        self.emit_rule(RuleEvent::DotTick, index as i16, f64::from(tick_damage));
        // **Das Ereignis, das bis #177 ruhte.** Sein Kommentar in
        // `rules_event.rs` sagte "Ruht in Schritt 1 (PURITY = 0)" — mit
        // getragenem PURITY hat es einen Fall.
        if kritet {
            self.emit_rule(RuleEvent::Crit, index as i16, f64::from(tick_damage));
        }
        self.dot_tick_counts[index] += 1;
        // **Zersetzungs-Stapel je Tick** (Issue #498, ROT). Sie laufen NICHT
        // ueber das Ereignis wie der Hitze-Gewinn: Stapel sind keine
        // Schul-Ressource, sondern liegen auf dem Boss — und die Stelle, die
        // sie beim Wirken legt, sitzt ebenfalls direkt hier.
        self.add_decay_stacks(dot.decay_per_tick);
        // Der Hitze-Gewinn stand bis E3-2 hier. Er haengt jetzt am Ereignis.

        // Der naechste Tick nur, wenn er noch in die Laufzeit faellt. Das `<=`
        // ist dieselbe Entscheidung wie das `>=` in [`Fight::dot_active`]:
        // 12 s Laufzeit bei 2 s Abstand ergeben **6** Ticks, der letzte genau
        // beim Ablauf (`STEP1-SPEC` 1.2).
        let next = self.time + self.dot_interval(&dot);
        if self.dot_expires_at[index].is_some_and(|expires| next <= expires) {
            self.queue.push(
                next,
                EventKind::DotTick {
                    spell: index as u8,
                    generation,
                },
            );
        }
    }

    /// Nimmt die liegende Schutzschicht weg und gibt zurueck, was davon als
    /// Heilung ankommt (SUTURE, Issue #481).
    ///
    /// **Die Schicht ist danach ganz weg, nicht anteilig.** Das Raster sagt
    /// „wandelt Schicht in Heilung um"; eine halbe Schicht stehen zu lassen
    /// waere ein anderer Zauber. Der Prozentsatz bestimmt, wie viel bei der
    /// Umwandlung ANKOMMT — nicht, wie viel verbraucht wird.
    fn verbrauche_schicht_fuer_heilung(&mut self, prozent: u32) -> u32 {
        let liegend = self.shield;
        if liegend == 0 {
            return 0;
        }
        self.shield = 0;
        self.shield_stacks = 0;
        self.shield_expires_at = None;
        liegend * prozent / 100
    }

    /// Wie viel Prozent der Anzeige gerade fehlen, abgerundet (MEND, #481).
    ///
    /// **Ganzzahlige Division, kein Runden.** Sie entspricht genau dem
    /// `Math.floor` der TypeScript-Seite; `round` haette bei `.5` eine eigene
    /// Regel, die dort nachzubauen waere.
    ///
    /// Am Wert DIESER Stufe gemessen, nicht am Grundwert — sonst zeigte ein
    /// Spieler auf Stufe 30 mit vollem Leben ein Defizit an.
    fn fehlend_prozent(&self) -> u32 {
        if self.vessel_max == 0 {
            return 0;
        }
        self.vessel_max.saturating_sub(self.player_hp) * 100 / self.vessel_max
    }

    /// Ein Schritt einer Heilung ueber Zeit kommt an (POULTICE, Issue #481).
    ///
    /// Deckelt an der vollen Anzeige — was darueber hinausginge, verpufft, genau
    /// wie bei der Sofortheilung. Und derselbe Nachlauf: zurueckdraengen
    /// (Baustein 6) und ins Protokoll, mit dem Betrag, der WIRKLICH angekommen
    /// ist.
    fn heal_tick(&mut self, generation: u16) {
        // Ein Weckruf aus einer frueheren Anwendung — ein zweiter Wurf hat die
        // Heilung inzwischen neu aufgesetzt, oder die Runde ist vorbei.
        if generation != self.heal_over_time_generation
            || self.heal_over_time_done >= self.heal_over_time_steps
        {
            return;
        }
        self.heal_over_time_done += 1;
        let soll =
            self.heal_over_time_amount * self.heal_over_time_done / self.heal_over_time_steps;
        let faellig = soll - self.heal_over_time_credited;
        self.heal_over_time_credited = soll;
        let fehlend = self.vessel_max.saturating_sub(self.player_hp);
        let wirklich = faellig.min(fehlend);
        if wirklich > 0 {
            self.player_hp += wirklich;
            self.healed = self.healed.saturating_add(wirklich);
            self.stabilise(wirklich);
            self.log
                .push(self.time, LogKind::Healed, NO_SPELL, wirklich);
        }
        // Der naechste Schritt nur, solange noch einer aussteht.
        if self.heal_over_time_done < self.heal_over_time_steps {
            self.queue.push(
                self.time + self.heal_over_time_interval_ms,
                EventKind::HealTick { generation },
            );
        }
    }

    /// Die Energieleiste steigt (GAME-DESIGN 11.2, Baustein 1).
    ///
    /// **Unabhaengig vom Schaden des Spielers** — das ist die eigentliche
    /// Aussage des Bausteins. Wer schneller zuschlaegt, verschiebt den
    /// Ausloeser nicht; damit entsteht ein zweiter Takt, gegen den geplant
    /// werden muss.
    fn energy_tick(&mut self) {
        let Some(energy) = self.boss.energy else {
            return;
        };

        self.energy = self.energy.saturating_add(energy.amount);
        if self.energy >= energy.threshold {
            self.energy_triggers += 1;
            self.energy = energy.reset_to;
            self.log.push(
                self.time,
                LogKind::BossEnergy,
                NO_SPELL,
                self.energy_triggers,
            );
            // **Und hier tut sie etwas** (Issue #568). Bis dahin endete der
            // Zweig mit dem Zaehler und dem Eintrag im Verlauf: Die Leiste
            // stieg, erreichte 100, sprang zurueck — und keine Zeile im Kampf
            // fragte danach. `test-vessel` lieferte mit und ohne `energy`
            // bitgleich 8080/1600/0.
            //
            // Der Auftraggeber hat entschieden: ein **Boss-Schlag ausserhalb
            // der Zeitleiste**. Deshalb derselbe Weg wie ein `Strike` dort
            // (`hit_player`) — nur kommt der Zeitpunkt aus der Leiste statt
            // aus `at_ms`.
            //
            // **Nach dem Log-Eintrag**, damit der Verlauf den Ausloeser vor
            // seiner Wirkung zeigt: Toetet der Schlag den Spieler, steht der
            // Grund davor und nicht dahinter.
            self.hit_player(energy.on_trigger.damage);
        }

        let next = self.time + energy.per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::EnergyTick);
        }
    }

    /// Der toedliche Zaehler steigt (GAME-DESIGN 11.2, Baustein 2; Issue #216).
    ///
    /// Er springt **nicht** zurueck wie die Energieleiste — bei der Schwelle
    /// ist der Lauf vorbei. Der Tod geht dabei NICHT durch `hit_player`:
    /// WARD und Schutzschicht wuerden dort mindern, und ein Zaehler, den eine
    /// Schutzschicht ueberlebt, waere keiner.
    fn doom_tick(&mut self) {
        let Some(doom) = self.boss.doom else {
            return;
        };
        if self.died_at.is_some() {
            return;
        }

        // **Verbrauchte Senkungs-Ladungen wachsen nach** (Issue #561). Bis
        // dahin war `clear_charges` ein Vorrat fuer den ganzen Kampf: zwei
        // Senkungen a 40 gegen 360 Anstieg — der Zauber waere wirkungslos
        // gewesen, egal wie oft man ihn drueckt.
        //
        // **Die Pruefung sitzt IM DOOM-TAKT, nicht in einer eigenen Uhr.** Auf
        // der Spielseite steht sie an derselben Stelle. Eine feinere Aufloesung
        // auf einer der beiden Seiten liesse die Ladung dort frueher
        // zurueckkommen — und zwei Seiten, die verschieden oft nachladen,
        // rechnen ab dem ersten Unterschied verschiedenen Schaden.
        if let Some(faellig) = self.next_doom_charge_at {
            if self.time >= faellig && self.doom_charges < doom.clear_charges {
                self.doom_charges += 1;
                self.next_doom_charge_at = if self.doom_charges < doom.clear_charges {
                    Some(faellig + Self::recharge_ms(doom))
                } else {
                    None
                };
            }
        }

        self.doom = self.doom.saturating_add(doom.amount);
        if self.doom >= doom.threshold {
            self.doom = doom.threshold;
            self.player_hp = 0;
            self.died_at = Some(self.time);
            // **Auch dieser Tod steht im Protokoll** (Issue #693).
            //
            // Bis hierher setzten ZWEI Wege `died_at`, und nur einer schrieb
            // den Eintrag: der Schadens-Tod in `hit_player`. Wer den Verlauf
            // eines Verhängnis-Todes las, fand den Spieler tot und nirgends
            // vermerkt, dass er gestorben war.
            //
            // **Der Wert ist derselbe wie am anderen Weg: `damage_taken`.**
            // Das ist gewollt und war die Entscheidung an AC1. Der Eintrag
            // sagt „der Spieler ist gestorben, mit so viel genommenem
            // Schaden" — eine Aussage, die bei beiden Todesarten dasselbe
            // bedeutet. Den Verhängnis-Stand hier einzusetzen wäre die
            // Verwechslung, die #512 und #568 gekostet hat: dieselbe Zahl
            // hieße je nach Todesart etwas anderes.
            //
            // Die TODESART steht damit nicht im Verlauf, und das ist bekannt:
            // Es gibt überhaupt keine Protokoll-Art für das Verhängnis, auch
            // nicht für Tick oder Schwelle. Soll sie hinein, braucht sie eine
            // eigene Art — das ist ein eigenes Issue, nicht dieses.
            //
            // Der Wert ist nie 0: Gemessen an allen drei Bossen mit
            // Verhängnis nimmt der Spieler vorher Schaden, nur keinen
            // tödlichen (boss-1-2 280, test-doomsayer 2800, test-stacked 160).
            self.log
                .push(self.time, LogKind::PlayerDeath, NO_SPELL, self.damage_taken);
            return;
        }

        let next = self.time + doom.per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::DoomTick);
        }
    }

    /// Ein exponentieller Stapel kommt dazu (Baustein 3; Issue #216).
    ///
    /// Anders als der Zaehler toetet er nicht bei einem Wert, sondern macht den
    /// Schaden unbezahlbar: 1 -> 3 -> 9. Er geht deshalb durch `hit_player` wie
    /// jeder andere Schlag — WARD und Schutzschicht wirken, und genau das ist
    /// die Antwort, die dem Spieler bleibt.
    fn stack_tick(&mut self) {
        let Some(stacks) = self.boss.stacks else {
            return;
        };

        if self.stacks < stacks.max_stacks {
            self.stacks += 1;
        }
        let damage = stack_damage(stacks, self.stacks);
        self.hit_player(damage);

        let next = self.time + stacks.per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::StackTick);
        }
    }

    /// Wie viele Zyklen die Zeitleiste gelaufen ist (Baustein 7, Issue #240).
    ///
    /// **Abgeleitet, nicht mitgezaehlt** — genau wie im Spiel. Ein eigenes Feld
    /// waere eine zweite Wahrheit ueber dieselbe Groesse, und die zweite laeuft
    /// beim Nachziehen weg. Der Zeiger weiss es bereits.
    fn cycle(&self) -> u32 {
        let laenge = self.boss.timeline.len();
        if laenge == 0 || self.boss.timeline_loop_ms.is_none() {
            return 0;
        }
        u32::try_from(self.timeline_index / laenge).unwrap_or(u32::MAX)
    }

    /// Die Wirkzeit nach der Eskalation (Baustein 7, Issue #240).
    ///
    /// **Der Zyklus kommt aus dem Fortschrittszaehler des Eintrags, nicht aus
    /// dem Stand der Zeitleiste.** Beim ersten Zauber zeigt der Zeiger bereits
    /// auf den naechsten Eintrag; wer ihn liest, verkuerzt schon den ersten
    /// Zauber um einen Zyklus. Im Spiel ist mir genau das passiert, und
    /// aufgefallen ist es nur, weil die erwartete Reihe VOR dem Vergleich
    /// ausgerechnet war.
    ///
    /// Ohne Wiederholung gibt es keine Zyklen — dann bleibt die Wirkzeit, wie
    /// sie in der Zeitleiste steht.
    fn escalated_cast_time(&self, base: Millis, entry_index: usize) -> Millis {
        let Some(esc) = self.boss.escalation else {
            return base;
        };
        let laenge = self.boss.timeline.len();
        if laenge == 0 || self.boss.timeline_loop_ms.is_none() {
            return base;
        }
        let zyklus = entry_index / laenge;
        if zyklus == 0 {
            return base;
        }
        let gekuerzt = i64::from(base) - i64::from(esc.cast_time_shrink_ms) * zyklus as i64;
        if gekuerzt < i64::from(esc.min_cast_time_ms) {
            esc.min_cast_time_ms
        } else {
            Millis::try_from(gekuerzt).unwrap_or(esc.min_cast_time_ms)
        }
    }

    /// Die Schwaechung entfernen (Baustein 8, Issue #240).
    ///
    /// **Heilung steht bewusst nicht in `cleared_by`** — das ist der ganze
    /// Baustein. Wer sie eintraegt, hat ihn in eine gewoehnliche Schwaechung
    /// zurueckverwandelt.
    fn clear_affliction(&mut self, spell_id: &str) {
        let Some(aff) = self.boss.affliction else {
            return;
        };
        if !self.afflicted || !aff.cleared_by.contains(&spell_id) {
            return;
        }
        self.afflicted = false;
        self.afflictions_cleared += 1;
    }

    /// Die davonlaufende Reaktion senkt den Ring (Baustein 6, Issue #247).
    ///
    /// **Sie ist kein Schaden.** Deshalb geht sie durch `book_progress` und
    /// nicht durch `deal`: Das Unverwundbarkeits-Fenster haelt sie nicht auf,
    /// sie taucht nicht in der Schadenstabelle auf, und sie speist die
    /// Umwandlung nicht. Liefe sie in den Fenster-Zweig, zaehlte sie ausserdem
    /// als "im Fenster verloren" — die Fehleranalyse meldete dem Spieler dann
    /// Schaden, den er nie gemacht hat.
    fn runaway_tick(&mut self) {
        let Some(run) = self.boss.runaway else {
            return;
        };
        if self.killed_at.is_some() || self.died_at.is_some() {
            return;
        }

        self.book_progress(run.amount, false);

        let next = self.time + run.per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::RunawayTick);
        }
    }

    /// Die Schwaechung greift oder tickt (Baustein 8, Issue #240).
    ///
    /// Beides in einem Ereignis, weil beides an derselben Uhr haengt: Der
    /// erste Aufruf legt sie auf, jeder weitere macht Schaden. Sie geht durch
    /// `hit_player` — WARD und Schutzschicht mindern ihn also. Was sie NICHT
    /// entfernt, ist Heilung, und darin liegt der ganze Baustein.
    fn affliction_tick(&mut self) {
        let Some(aff) = self.boss.affliction else {
            return;
        };

        if !self.afflicted {
            // **Genau einmal.** Ist sie schon einmal entfernt worden, kommt sie
            // nicht wieder — sonst waere das Entfernen eine Verzoegerung und
            // keine Loesung.
            if self.afflictions_cleared > 0 {
                return;
            }
            self.afflicted = true;
        } else {
            self.hit_player(aff.damage);
            // **Der zweite Ausloeser von Baustein 5** (Issue #609). Dieselbe
            // Stelle wie auf der Spielseite: neben dem Schaden der Schwaechung,
            // je Schritt und nicht einmal.
            //
            // Die beiden Seiten sind hier verschieden GEBAUT — das Spiel
            // schleift, der Simulator plant Ereignisse ein —, aber sie muessen
            // gleich oft zurueckdrehen. Genau dafuer steht der Wert in der
            // Aufzeichnung.
            if self.boss.boss_heal.map(|h| h.on) == Some(BossHealTrigger::Affliction) {
                self.heal_boss();
            }
        }

        let next = self.time + aff.per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::AfflictionTick);
        }
    }

    /// Die begrenzt verfuegbare Handlung, die den Zaehler senkt.
    ///
    /// Gibt zurueck, ob sie gegriffen hat. **Ohne Ladung geschieht nichts** —
    /// sie nimmt sich nicht, was sie nicht hat.
    ///
    /// WELCHE TASTE SIE AUSLOEST, STEHT HIER NICHT: Dieses Issue baut den
    /// Mechanismus, die Bindung an einen Zauber entscheidet E10.
    pub fn clear_doom(&mut self) -> bool {
        let Some(doom) = self.boss.doom else {
            return false;
        };
        if self.doom_charges == 0 || self.died_at.is_some() {
            return false;
        }
        self.doom_charges -= 1;
        // Die Uhr laeuft ab dem VERBRAUCH, nicht ab Kampfbeginn — sonst
        // bekaeme ein Spieler, der lange nicht gedrueckt hat, seine Ladung
        // sofort zurueck, und die Wartezeit waere keine.
        if self.next_doom_charge_at.is_none() {
            self.next_doom_charge_at = Some(self.time + Self::recharge_ms(doom));
        }
        self.doom = self.doom.saturating_sub(doom.clear_amount);
        true
    }

    /// Wie lange eine verbrauchte Senkungs-Ladung zum Nachladen braucht
    /// (Issue #561).
    ///
    /// **Abgeleitet, kein eigenes Datenfeld.** Es ist genau die Zeit, in der
    /// der Zaehler um `clear_amount` steigt — bei `boss-1-2` also
    /// 40/6 * 1000 = 6667 ms. Eine Zahl, die aus dem Gleichgewicht zwischen
    /// Anstieg und Senkung folgt, gilt fuer JEDEN doom-Boss und kann nicht
    /// vergessen werden; ein Feld haette bei jedem neuen von Hand passen
    /// muessen (zu wenig: unschaffbar, zu viel: Kulisse).
    ///
    /// Spiegelbild zu `Combat.rechargeMs()` in `src/core/combat.ts`.
    fn recharge_ms(doom: Doom) -> Millis {
        if doom.amount == 0 {
            return 0;
        }
        let genau =
            f64::from(doom.clear_amount) / f64::from(doom.amount) * f64::from(doom.per_step_ms);
        genau.round() as Millis
    }

    /// Prueft, ob der Boss die Phase wechselt.
    ///
    /// An derselben Stelle wie die Sieg-Pruefung: nach den passiven Wirkungen,
    /// vor der Entscheidung des Spielers. Das ist kein Zufall — beide haengen
    /// am selben Wert, den Lebenspunkten, und muessen deshalb denselben
    /// Zeitpunkt sehen.
    ///
    /// **Eskalierend, nicht springend:** Die Phase kann nur steigen. Heilt der
    /// Boss spaeter einmal (GAME-DESIGN 11.2, Baustein 5), faellt sie nicht
    /// zurueck — eine schon gezeigte Mechanik verschwindet nicht wieder.
    fn check_phase(&mut self) {
        if self.boss.phases.is_empty() {
            return;
        }
        // **Die gesenkte Schwelle aus ANCHOR** (#100): Der Boss muss tiefer
        // fallen, bevor er wechselt. Gerechnet wird, indem der Anteil
        // scheinbar hoeher liegt — dasselbe Ergebnis, ohne die Boss-Daten
        // anzufassen, die statisch einkompiliert sind.
        let anteil = self.boss_hp_percent() + self.phase_hold_percent;
        let neue = self.boss.phase_at(anteil);
        if neue > self.phase {
            self.phase = neue;
            // **THE LONG GRIP wird hier scharf gestellt** (13.5): beim
            // WECHSEL, nicht beim Erreichen einer Phase.
            self.free_control_spend_armed = true;
            // **Verbraucht** (GAME-DESIGN 4.2b): Die Senkung galt fuer diesen
            // einen Wechsel. Der uebernaechste bekommt wieder die volle
            // Schwelle — sonst verschoebe ein Druck alle folgenden mit.
            self.phase_hold_percent = 0;
            self.phase_hold_until = None;
            self.log
                .push(self.time, LogKind::BossPhase, NO_SPELL, neue as u32);
        }
    }

    fn decay(&mut self) {
        self.resource = self.resource.saturating_sub(RESOURCE.decay_amount);
        let next = self.time + RESOURCE.decay_per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::ResourceDecay);
        }
    }

    /// Wirkt einen Zauber.
    ///
    /// Die Reihenfolge der Wirkungen ist die der Referenz und nicht beliebig:
    /// Ein Entlade-Zauber liest die Ressource, **bevor** ein Gewinn desselben
    /// Zaubers sie erhoehen koennte. In Schritt 1 gibt es keinen Zauber, der
    /// beides tut — die Festlegung verhindert, dass der erste solche Zauber
    /// stillschweigend anders rechnet als im Spiel.
    fn cast(&mut self, index: usize) {
        let spell: &Spell = &SPELLS[index];
        let now = self.time;

        // **Jede Wirkung bricht einen laufenden Zauber ab** (GAME-DESIGN 7.1,
        // entschieden am 2026-08-05) — auch das Unterbrechen.
        //
        // Die Stelle ist der Beginn von `cast` und nicht frueher: Die Aufrufer
        // pruefen Wartezeit und Abklingzeit VOR diesem Aufruf (`is_ready` in
        // `replay` und in der Rotation). Wer hier ankommt, wirkt auch — und ein
        // Druck, der zu keiner Wirkung fuehrt, darf nichts abbrechen.
        //
        // Auf der Spielseite steht derselbe Aufruf in `Combat.cast`, dort NACH
        // den zwei Pruefungen, weil sie in derselben Methode liegen.
        if self.cancel_player_cast() {
            self.casts_cancelled += 1;
        }

        // **Der Zauber mit Wirkzeit endet hier** (GAME-DESIGN 7.1, Issue #304).
        //
        // Die globale Wartezeit laeuft ab dem BEGINN — 7.1 misst die Wirkzeit
        // in Wartezeiten, und dieser Vergleich ergibt nur einen Sinn, wenn
        // beide zugleich laufen. Die ABKLINGZEIT laeuft dagegen nicht an; sie
        // steht in `resolve_player_cast`, und daran haengt die zweite Haelfte
        // von 7.1: "Bei Abbruch geht der Zauber verloren, die Abklingzeit
        // laeuft aber nicht an."
        let cast_time = self.hasted(spell.cast_time_ms);
        if cast_time > 0 {
            self.player_cast_generation = self.player_cast_generation.wrapping_add(1);
            self.player_casting = Some(index);
            self.player_cast_ends_at = snap_up(now + cast_time);
            // **Der Merker geht VOR der Eigenschaft** (SUBLIME, #480): Ein
            // Zauber, der die Wartezeit sonst auslöste, tut es einmal nicht.
            if spell.triggers_gcd && !self.take_free_gcd() {
                self.gcd_ready_at = snap_up(now + self.gcd_length());
                self.queue.push(self.gcd_ready_at, EventKind::Wakeup);
            }
            self.queue.push(
                self.player_cast_ends_at,
                EventKind::PlayerCastLands {
                    spell: index as u8,
                    generation: self.player_cast_generation,
                },
            );
            return;
        }

        self.resolve_player_cast(index, false);
    }

    /// Die Wirkzeit eines Spieler-Zaubers ist um (Issue #304).
    ///
    /// Er wirkt **nur**, wenn er noch laeuft. Wurde er abgebrochen, steht
    /// `player_casting` auf `None` oder auf einem anderen Vorgang, und dieser
    /// Aufruf verpufft — dasselbe Vorgehen wie bei veralteten DoT-Ticks und
    /// beim Boss-Zauber.
    fn player_cast_lands(&mut self, index: usize, generation: u16) {
        if self.player_casting != Some(index) || self.player_cast_generation != generation {
            return;
        }
        if self.time < self.player_cast_ends_at {
            return;
        }
        self.player_casting = None;
        self.player_cast_ends_at = 0;
        self.resolve_player_cast(index, true);
    }

    /// Die Wirkung eines Zaubers — der Teil, der bei einer Wirkzeit **am Ende**
    /// faellt (Issue #304, AC2).
    ///
    /// Bis #304 stand das alles im Rumpf von `cast`. Herausgezogen und nicht
    /// kopiert: Ein Sofort-Zauber und ein Zauber mit Wirkzeit teilen sich diese
    /// eine Stelle. **Der Kritwurf gehoert hierher und nicht an den Beginn** —
    /// ein Wurf beim Druecken drehte den Zufallsstrom zu einem anderen
    /// Zeitpunkt als das Spiel.
    fn resolve_player_cast(&mut self, index: usize, wirkzeit_lief: bool) {
        let spell: &Spell = &SPELLS[index];
        let now = self.time;

        self.casts[index] += 1;

        // **Wirkt dieser Wurf zweimal?** (CONDENSE, Issue #480.)
        //
        // Der Merker wird HIER verbraucht, vor jeder Wirkung — und das Setzen
        // steht wie bei SUBLIME weit unten, hinter allem. Stünde beides
        // beieinander, verdoppelte CONDENSE sich selbst statt den nächsten.
        //
        // `casts` bleibt bei EINS. Es ist ein Wurf, der zweimal wirkt, kein
        // zweiter Wurf: eine Abklingzeit, eine Wartezeit, ein Cast-Eintrag im
        // Log. Wer hier mitzählte, machte aus CONDENSE einen Rücksetzer — und
        // genau davon unterscheidet ihn das Raster.
        let anwendungen = if self.take_double_cast() { 2 } else { 1 };

        // **Erst der Schaden, dann die Wirkung.** Bis zu Issue E3-1b stand der
        // Cast-Eintrag hier oben und damit VOR dem Schaden — das Spiel
        // (`src/core/combat.ts`) schreibt seit E1-7 umgekehrt. Zwei
        // Erzaehlungen desselben Kampfes, die sich widersprechen, sind beim
        // Abgleich schlimmer als eine; deshalb ist der Simulator angeglichen
        // worden und nicht das Spiel (Entscheidung des Auftraggebers,
        // 2026-07-29). Verglichen wurde bis dahin nur Zahlenwerk, weshalb es
        // niemandem auffiel.
        let mut cast_damage = 0;

        // Unterbrechen (GAME-DESIGN 11.2, Baustein 9).
        //
        // **Kein Sonderfall in der Wartezeit-Rechnung.** Dass dieser Zauber
        // keine globale Wartezeit ausloest, steht weiter unten als
        // `spell.triggers_gcd` und kommt aus den Daten — hier waere eine
        // zweite Stelle, an der dieselbe Regel haengt, und zwei Stellen laufen
        // auseinander.
        if spell.interrupt && self.interrupt_boss_cast() {
            self.log.push(
                now,
                LogKind::InterruptLanded,
                index as u8,
                self.interrupts_landed,
            );
        }

        // **Das Verhaengnis senken** (Baustein 2; Issue #561). Dieselbe Bauart
        // wie das Unterbrechen daroeber: Ob der Zauber es tut, steht in den
        // Daten; WIE VIEL er senkt, steht beim Boss (`clear_amount`).
        //
        // Spiegelbild zu `if (spell.clearsDoom === true) this.clearDoom();`
        // in `src/core/combat.ts`.
        if spell.clears_doom {
            self.clear_doom();
        }

        // Die drei Wege der Koagulation (Issue #79). Sie stehen VOR dem
        // Schaden, weil ein Zauber, der beides taete, erst schuetzen und dann
        // zuschlagen soll — die Reihenfolge ist heute ohne Wirkung (keiner
        // macht beides) und deshalb jetzt festzulegen und nicht dann.
        self.apply_coagulation(index, spell);
        self.apply_putrefaction(spell);
        self.apply_fixation(spell);
        self.apply_conversion(spell, now);
        // **Erst pruefen, dann oeffnen** (#318). Ein Zauber, der sein eigener
        // Partner waere, nutzte sonst das Fenster, das er im selben Zug
        // aufmacht.
        self.apply_synergy_bonus(index);
        self.open_synergy_window(spell);

        // **Ein Wurf fuer die ganze Wirkung**, und er faellt genau dann, wenn
        // der Zauber **laut seinen Daten** Schaden austeilen kann — nicht
        // danach, ob am Ende ein Betrag herauskommt.
        //
        // Der Unterschied betrifft den Entlade-Zauber ohne Punkte: Er wuerfelt,
        // obwohl sein Schaden null ist. Das ist Absicht. Die Bedingung ist aus
        // den Daten ablesbar und haengt nicht am Zustand — damit ist sie auf
        // beiden Seiten trivial gleichzuhalten. Eine Bedingung ueber den
        // ausgerechneten Betrag waere es nicht, und ein Zufallsstrom, der auf
        // den zwei Seiten verschieden oft gedreht wird, laesst den ganzen
        // folgenden Kampf auseinanderlaufen.
        // **Die Umwandlung zaehlt mit** (Issue #520): WEDGE und EFFLUX
        // fuehren weder base_damage noch spender, teilen aber Schaden aus.
        // Fehlte sie hier, wuerde der Krit-Wurf auf dieser Seite nicht
        // gezogen — und ein Wuerfel, der verschieden oft gedreht wird, laesst
        // den ganzen folgenden Kampf auseinanderlaufen.
        let teilt_schaden_aus =
            spell.base_damage > 0 || spell.spender.is_some() || self.pending_control_damage > 0;
        let kritet = teilt_schaden_aus && self.roll_crit(Stream::Crit);
        // Der Widerstand dieses Zaubers, vor der Rechnung (Issue #328).
        // Dieselbe Stelle wie im Spiel (`Combat.resolvePlayerCast`).
        self.factors.resistance = self.resistance_by_spell[index];
        // Der Stapel-Zuschlag DIESES Zaubers (#484) — aus demselben Grund wie
        // der Widerstand daneben: Er gehoert der Wirkung, nicht dem Kampf.
        self.apply_decay_stack_bonus(index);
        // EMBER (Issue #517), VOR der Schadensrechnung: Ein Brand, den dieser
        // Zauber selbst legt, soll nicht schon verlaengert werden.
        if spell.dot_extend_ticks > 0 {
            self.extend_active_dots(spell.dot_extend_ticks);
        }
        // HARDEN und SEPSIS (#523), aus demselben Grund an derselben Stelle.
        if spell.extend_shield_ms > 0 {
            self.extend_shield(spell.extend_shield_ms);
        }
        if spell.extend_decay_ms > 0 {
            self.extend_decay_stacks(spell.extend_decay_ms);
        }
        let factors = self.factors_for(kritet);
        // **`factors` ist hier bereits kopiert.** Die Verstaerkung unten
        // aendert `self.factors.buffs`, aber nicht mehr diese Kopie — genau so
        // soll es sein: Ein Zauber verstaerkt die FOLGENDEN, nicht sich selbst.
        // Im Spiel ergibt sich dasselbe daraus, dass der Buff nach dem Schaden
        // angelegt wird (`Combat.resolvePlayerCast`).

        // **Der umgewandelte Schaden geht durch dieselben Faktoren** (#478):
        // Krit, Widerstand, Verstaerkung. Ein Treffer, den kein Faktor
        // beruehrt, waere der einzige im Spiel.
        // **Die Schleife für CONDENSE** (#480). Sie läuft einmal, oder zweimal
        // wenn der Merker stand.
        //
        // Sie umschliesst genau die MENGEN — die vier Schadensquellen. Die
        // ZUSTÄNDE darüber (unterbrechen, Verhängnis senken, Synergie-Fenster)
        // und darunter (Verstärkung, Brand) stehen bewusst ausserhalb: Zweimal
        // zu unterbrechen unterbricht nicht doppelt, und ein Fenster zweimal zu
        // öffnen verschöbe seine Dauer.
        //
        // **Ein Krit-Wurf für beide Anwendungen**, nicht zwei. Er fällt oben
        // vor der Schleife — dieselbe Regel, nach der das Schlüsseltalent aus
        // Issue #418 gebaut ist, das denselben Satz trägt ("wirkt dafür
        // ZWEIMAL"). Ein Zufallsstrom, der auf den zwei Seiten verschieden oft
        // gedreht wird, lässt den ganzen folgenden Kampf auseinanderlaufen.
        for _ in 0..anwendungen {
            if self.pending_control_damage > 0 {
                let roh = self.pending_control_damage;
                self.pending_control_damage = 0;
                cast_damage += self.deal(index, compute_damage(roh, &factors), LogKind::Damage);
            }

            if spell.base_damage > 0 {
                cast_damage += self.deal(
                    index,
                    compute_damage(spell.base_damage, &factors),
                    LogKind::Damage,
                );
            }

            if let Some(spender) = spell.spender {
                // **Das erste Schluesseltalent** (Issue #418, GAME-DESIGN 13.09):
                // "Der Entlade-Zauber verbraucht nur die HAELFTE der Hitze, wirkt
                // dafuer ZWEIMAL."
                //
                // Eine Verzweigung und kein Faktor — ein Schluesseltalent, das
                // sich als Faktor ausdruecken liesse, waere keines.
                let geteilt = self.split_spender;
                // Ganzzahlig geteilt, der Rest bleibt liegen (18.4): Aus 15 werden
                // zweimal 7, eine Hitze bleibt uebrig.
                let pro_schlag = if geteilt {
                    self.resource / 2
                } else {
                    self.resource
                };
                let schlaege = if geteilt { 2 } else { 1 };

                for _ in 0..schlaege {
                    // Der Vorrat wird VOR dem zweiten Schlag verbraucht, sonst
                    // rechnete der zweite mit demselben Stand wie der erste.
                    let spent = if geteilt {
                        pro_schlag.min(self.resource)
                    } else {
                        self.resource
                    };
                    if geteilt && spent == 0 {
                        break;
                    }

                    let mut damage = spender.damage_per_point * u32::from(spent);
                    if spent >= spender.bonus_threshold {
                        damage *= spender.bonus_multiplier;
                    }
                    self.log
                        .push(now, LogKind::Spend, index as u8, u32::from(spent));
                    cast_damage +=
                        self.deal(index, compute_damage(damage, &factors), LogKind::Damage);
                    if geteilt {
                        self.resource -= spent;
                    } else {
                        // **ASHEN REGIMEN** (13.5, Sechser-Bonus): Ein Drittel
                        // der angesammelten Hitze bleibt stehen.
                        //
                        // Abgerundet, der Rest verfaellt (18.4) — bei 20 Hitze
                        // bleiben 6 stehen, nicht 6,67. Nach dem Schaden, nicht
                        // davor: `spent` ist oben schon eingegangen; der Effekt
                        // schenkt Vorrat fuer den NAECHSTEN Wurf.
                        self.resource = if self.ashen_regimen { spent / 3 } else { 0 };
                    }
                }
            }

            // PYROCLASM (#517): der Rest der laufenden Braende, in denselben
            // Betrag wie der Grundschaden — dieselbe Stelle wie im Spiel.
            if spell.consumes_dots {
                let rest = self.consume_active_dots();
                if rest > 0 {
                    cast_damage += self.deal(index, rest, LogKind::Damage);
                }
            }
        }

        // NECROSIS (#479): raeumt NACH der eigenen Berechnung ab, dieselbe
        // Stelle wie im Spiel (`combat.ts`, `consumeDecayStacks`). Der
        // Schadensfaktor steht oben schon fest und traegt den Stapel-Bonus —
        // der Entlader rechnet also MIT den Stapeln und nimmt sie erst dann
        // weg. Er addiert nichts: Sein Ertrag steckt im Faktor, nicht in
        // einem zweiten Betrag.
        if spell.consumes_decay_stacks && self.decay_count > 0 {
            self.decay_count = 0;
            self.update_vulnerability();
        }

        // **Ein Schaden-Ereignis je Wirkung, mit der Summe** — nicht eines je
        // Teilbetrag. Das Spiel rechnet Grund- und Entladeschaden zusammen und
        // meldet einmal; ein Zauber, der beides hat, ergaebe hier sonst zwei
        // Ereignisse und dort eines. In Schritt 1 gibt es keinen solchen
        // Zauber — genau deshalb ist es jetzt festzulegen und nicht dann.
        if cast_damage > 0 {
            self.emit_rule(RuleEvent::Damage, index as i16, f64::from(cast_damage));
        }
        // **Hier stand bis #177: "Krit ruht in Schritt 1 (PURITY = 0); ein
        // Ausloeser, der nie zutrifft, waere toter Code."** Der Ort war richtig
        // benannt; jetzt hat er einen Fall. Der Ausloeser steht NACH
        // `RuleEvent::Damage`, weil ein Krit eine Aussage ueber einen Treffer
        // ist, den es schon gegeben hat.
        if kritet && cast_damage > 0 {
            self.emit_rule(RuleEvent::Crit, index as i16, f64::from(cast_damage));
        }

        // Die Verstaerkung, nach dem Schaden und vor dem Brand-Effekt — dieselbe
        // Stelle wie im Spiel (Issue #420).
        if let Some(buff) = spell.buff {
            // **`self.hasted(GCD_MS)`, nicht `hasted(gcds * GCD_MS)`.** Die
            // beiden sind bei ganzzahliger Rechnung NICHT dasselbe: `hasted`
            // teilt und rastet auf, und drei aufgerundete Wartezeiten koennen
            // laenger sein als eine aufgerundete Dreifach-Wartezeit. Das Spiel
            // rechnet `gcds * hasted(GCD_MS)` (`Combat.resolvePlayerCast`), und
            // eine andere Klammerung hier waere ein Gleichlauf-Bruch, den erst
            // ein Lauf mit Tempo zeigt.
            // **Der Vorrat haengt an der Zielgroesse** (Issue #489). Die
            // Stapel-Regel (GAME-DESIGN 328) gilt damit INNERHALB einer
            // Groesse, nicht ueber alle hinweg: Eine Verstaerkung auf den
            // Punktegewinn kann eine auf den Schaden nicht verdraengen.
            let fenster = self.hasted(GCD_MS);
            let vorrat = match buff.target {
                BuffTarget::Damage => &mut self.buffs,
                BuffTarget::Control => &mut self.control_buffs,
                BuffTarget::Resource => &mut self.resource_buffs,
                BuffTarget::Tick => &mut self.tick_buffs,
                BuffTarget::Stack => &mut self.stack_buffs,
                BuffTarget::CdReduce => &mut self.cd_reduce_buffs,
                BuffTarget::Convert => &mut self.convert_buffs,
                BuffTarget::Cap => &mut self.cap_buffs,
            };
            vorrat.apply_for_gcds(buff.kind, buff.factor, buff.gcds, now, fenster);
            // Nur der Schadens-Faktor wird gespiegelt; die anderen vier werden
            // an ihrer Rechenstelle gelesen, damit kein zweiter Zustand
            // nachzuziehen ist.
            self.factors.buffs = self.buffs.factor(now);
        }

        if let Some(dot) = spell.dot {
            if self.dot_active(index) {
                // Erneuern verwirft die Restzeit (GAME-DESIGN 7.2). Was dabei an
                // Ticks verloren geht, ist die Kennzahl, an der die Fehleranalyse
                // den haeufigsten Anfaengerfehler festmacht.
                let remaining = self.dot_expires_at[index].unwrap_or(now) - now;
                self.lost_dot_ticks[index] += remaining / self.dot_interval(&dot);
            }
            self.dot_generation[index] = self.dot_generation[index].wrapping_add(1);
            // **Die Dauer bleibt unangetastet** (GAME-DESIGN 6.7). Tempo macht
            // die Ticks dichter, nicht das Fenster kuerzer — sonst verschoebe
            // sich der beste Erneuerungszeitpunkt (7.2) bei jedem
            // Ausruestungswechsel.
            // **Die Laufzeit traegt den Talent-Faktor** (#419, Weg „hold").
            // Laenger heisst MEHR Ticks, nicht langsamere — den Abstand
            // bewegt `tick_buffs` an anderer Stelle. Abgerundet, wie im Spiel:
            // `Math.floor` und `as u32` sind hier dasselbe.
            let dauer = (f64::from(dot.duration_ms) * self.talent_dot_duration_factor) as Millis;
            self.dot_expires_at[index] = Some(now + dauer);
            let first = snap_up(now + self.dot_interval(&dot));
            self.queue.push(
                first,
                EventKind::DotTick {
                    spell: index as u8,
                    generation: self.dot_generation[index],
                },
            );
        }

        // **Alle Abklingzeiten skalieren mit** (GAME-DESIGN 6.7) — anders als
        // in WoW, wo eine Ausnahmeliste je Faehigkeit entscheidet.
        // Abklingzeiten sind bei uns die alleinige Schwierigkeit (2.1); eine
        // Liste, welcher Zauber mitskaliert, waere eine zweite Regel zum
        // Auswendiglernen.
        let ready_at = snap_up(now + self.hasted(spell.cooldown_ms));
        self.cooldown_ready_at[index] = ready_at;
        self.queue.push(ready_at, EventKind::Wakeup);

        // **Nur wenn sie nicht schon laeuft** (Issue #304): Bei einer Wirkzeit
        // ist die Wartezeit beim BEGINN gestartet worden. Ein zweiter Start
        // hier verlaengerte sie um die volle Laenge.
        if spell.triggers_gcd && !wirkzeit_lief && !self.take_free_gcd() {
            self.gcd_ready_at = snap_up(now + self.gcd_length());
            self.queue.push(self.gcd_ready_at, EventKind::Wakeup);
        }

        // **Rüsten** (RETORT, #480): Der nächste vom BOSS abgebrochene Wurf
        // kostet keine Wartezeit.
        //
        // Steht bei den zwei anderen Merkern, hinter allem, was sie
        // verbrauchen könnte — aus demselben Grund wie dort. RETORTs eigene
        // Wartezeit ist eine Zeile höher schon gestartet und bleibt: Er zahlt
        // wie jeder andere Zauber, und genau deshalb bleibt Design-Gesetz 6.6
        // unberührt.
        if spell.refunds_interrupted_gcd {
            self.retort_armed = true;
        }

        // **Den NÄCHSTEN Wurf befreien** (SUBLIME, #480).
        //
        // Diese Zeile steht bewusst HINTER beiden GCD-Stellen. Ein erster
        // Entwurf setzte sie oben zu den übrigen Wirkungen — dort hätte
        // SUBLIME als Sofort-Zauber seinen eigenen Merker eine Zeile später
        // wieder verbraucht und sich selbst befreit statt den nächsten.
        //
        // Der Fehler wäre grün geblieben: Die Wartezeit fiele ja aus, nur bei
        // der falschen Zeile.
        if spell.frees_next_gcd {
            self.next_cast_free_gcd = true;
        }

        // **Den NÄCHSTEN Wurf verdoppeln** (CONDENSE, #480), aus demselben
        // Grund an derselben Stelle wie der Merker darüber: hinter allem, was
        // ihn verbrauchen könnte.
        if spell.doubles_next_cast {
            self.next_cast_doubles = true;
        }

        // Der Sieg wird erst in der Pruef-Phase des **naechsten** Takts bemerkt,
        // weil die Pruefung vor der Wirk-Phase steht. Ohne diesen Weckruf
        // koennte der Simulator bis zum naechsten regulaeren Ereignis
        // weiterspringen und einen zu spaeten Zeitpunkt melden.
        if self.damage >= self.boss.hp {
            self.queue.push(now + TICK_MS, EventKind::Wakeup);
        }

        // Zuletzt: Die Wirkung ist abgeschlossen — Schaden, Effekt ueber Zeit,
        // Vorrat, Abklingzeit und Wartezeit stehen.
        self.log.push(now, LogKind::Cast, index as u8, 0);
        // **Das Ereignis feuert so oft, wie der Wurf gewirkt hat** (CONDENSE,
        // #480) — der Log-Eintrag darüber genau einmal.
        //
        // Daran hängen die beiden Mechaniken, die Mengen aufbauen: `stacks:heat`
        // (der Vorrat) und `sublimation` (die Verkürzung aller Abklingzeiten).
        // Sie zweimal ansetzen zu lassen ist dieselbe Regel wie beim Schaden
        // oben — und sie kostet keine Aufzählung, weil sie an der Stelle
        // ansetzt, an der die Mengen ohnehin entstehen.
        for _ in 0..anwendungen {
            self.emit_rule(RuleEvent::SpellCast, index as i16, f64::from(cast_damage));
        }

        // **Die Schwaechung ohne Heil-Loesung** (Baustein 8, Issue #240).
        // Steht dieser Zauber in `cleared_by`, ist sie weg — und Heilung steht
        // dort bewusst NICHT, das ist der ganze Baustein.
        //
        // An derselben Stelle wie im Spiel: nach dem Ereignis, als letztes.
        self.clear_affliction(spell.id);
    }

    /// Traegt Schaden ein und gibt den Betrag zurueck.
    fn deal(&mut self, index: usize, amount: u32, kind: LogKind) -> u32 {
        // **Im Unverwundbarkeits-Fenster kommt nichts am Boss an** (11.2,
        // Baustein 4; Issue #218) — aber der Schaden ist nicht weg. Er laedt
        // das Ziel und zaehlt als VERLOREN; das ist nicht dasselbe wie mal
        // null, und die Zahl gehoert in die Fehleranalyse (GAME-DESIGN 15.3).
        //
        // Hier und nur hier, aus demselben Grund wie die Umwandlung unten:
        // Jeder ausgeteilte Schaden laeuft durch diese Funktion. Auf der
        // Spielseite tut das `bookProgress`, und ein Prueffall haelt dort
        // fest, dass es nur eine Stelle gibt.
        if self.race_ends_at.is_some() {
            self.damage_lost_to_race += amount;
            if let Some(race) = self.boss.race {
                self.race_progress += amount;
                if self.race_progress >= race.threshold {
                    self.race_won = true;
                    self.race_ends_at = None;
                }
            }
            self.log.push(self.time, kind, index as u8, amount);
            return amount;
        }
        self.book_progress(amount, true);
        self.damage_by_spell[index] += amount;
        self.log.push(self.time, kind, index as u8, amount);
        // **Hier und nur hier** speist Weg 3 (Issue #79). Jeder ausgeteilte
        // Schaden laeuft durch diese Funktion — Sofortschaden, Entladung und
        // Brand-Tick. Die Umwandlung an den drei Aufrufstellen einzeln
        // anzuhaengen waere dieselbe Regel an drei Orten, und einer davon
        // liefe irgendwann weg.
        self.convert_damage_to_shield(amount);
        amount
    }

    /// Fortschritt am Ring buchen — **die einzige Stelle, die ihn erhoeht**.
    ///
    /// Seit #247 gibt es Fortschritt ohne Zauber: Die davonlaufende Reaktion
    /// senkt den Ring von selbst. Sie kommt hier durch dieselbe Tuer wie
    /// `deal`, damit es nicht zwei Stellen gibt, die kuenftige Sonderregeln
    /// einzeln kennen muessten. Auf der Spielseite haelt ein Prueffall genau
    /// das fest (`progress-writer-source.test.ts`).
    ///
    /// Was NICHT hierher gehoert und deshalb in `deal` bleibt: das
    /// Unverwundbarkeits-Fenster, die Schadenstabelle, das Protokoll und die
    /// Umwandlung. Alle vier gelten fuer ausgeteilten SCHADEN — die Reaktion
    /// ist keiner.
    fn book_progress(&mut self, amount: u32, from_player: bool) {
        self.damage += amount;
        // **Die Herkunft wird HIER festgehalten, nicht spaeter erschlossen.**
        // Nach der Buchung ist sie fort: Der Ring weiss nicht mehr, wer ihn
        // gesenkt hat. Dasselbe Argument wie auf der Spielseite in
        // `bookProgress`.
        if from_player {
            self.player_damage += amount;
        }
    }

    fn add_resource(&mut self, amount: u16) {
        // **Vor der Obergrenze, nicht danach** (Issue #489): Was ueber `cap`
        // hinausgeht, zaehlt als Ueberlauf — die Verstaerkung darf also den
        // Ueberlauf mit erhoehen, sonst verschwaende sie sich lautlos.
        let amount = if amount == 0 {
            0
        } else {
            // **Zwei Faktoren, EINE Rundung** (#419). Der Buff ist ein
            // zeitlich begrenztes Fenster (FLASHPOINT), der Talent-Faktor die
            // dauerhafte Verteilung des Spielers. Zweimal nacheinander zu
            // runden ergaebe bei 3 x 1,5 x 1,5 je nach Reihenfolge 6 oder 7 —
            // und das Spiel muesste dieselbe Reihenfolge treffen. Dort steht
            // dieselbe Zeile (`combat.ts`, `gainOfSpell`).
            (f64::from(amount)
                * self.resource_buffs.factor(self.time)
                * self.talent_resource_factor)
                .round() as u16
        };
        let before = self.resource;
        // **Die Grenze, wie sie JETZT gilt** (SLAG, Issue #490). Auf dieser
        // Seite gibt es nur EINE Lesestelle — das Entwickler-Werkzeug
        // (`setResource`) existiert im Simulator nicht. Im Spiel sind es zwei,
        // und dort rechnet ein Getter sie an einer Stelle.
        let cap = (f64::from(RESOURCE.cap) * self.cap_buffs.factor(self.time)).round() as u16;
        // **Ein Erzeuger nimmt NIE weg** (Issue #549).
        //
        // Faellt der Deckel unter den stehenden Vorrat — SLAG hebt ihn von 20
        // auf 30, das Fenster laeuft ab —, dann haette `min` gesenkt:
        //
        //     vorrat 24, Deckel wieder 20, BELLOWS gibt 8
        //       min(24 + 8, 20) = 20      netto MINUS vier
        //       max(24, min(...)) = 24    er bleibt, wo er ist
        //
        // Hier stuerzte der Simulator ab, waehrend das Spiel still falsch
        // zaehlte: `self.resource - before` lief als u16 ueber. Reproduziert
        // mit BELLOWS auf Tick 610 gegen 590.
        //
        // Der Absturz verschwindet damit als NEBENWIRKUNG, nicht als
        // Behebung. Ein `saturating_sub` haette ihn auch abgefangen und die
        // Frage offengelassen, was gelten soll — und die zwei Seiten haetten
        // weiter verschiedene Zahlen fuehren koennen.
        self.resource = (before + amount).min(cap).max(before);
        // Der Zaehler nimmt nur, was von DIESER Erzeugung nicht ankam. Die
        // Klammer ist mit der Regel oben nie mehr negativ.
        self.resource_overflow += u32::from(amount - (self.resource - before));
    }

    fn result(&self) -> RunResult {
        RunResult {
            damage: self.damage,
            killed_at_ms: self.killed_at,
            remaining_ms: self.killed_at.map_or(0, |t| self.duration_ms - t),
            casts: self.casts,
            damage_by_spell: self.damage_by_spell,
            lost_dot_ticks: self.lost_dot_ticks,
            dot_tick_counts: self.dot_tick_counts,
            resource_overflow: self.resource_overflow,
            idle_ms: self.idle_ms,
            random_draws: self.rng.total_draws(),
        }
    }

    // ------------------------------------------------- Abfragen fuer Rotationen

    /// Aktuelle Kampfzeit.
    #[must_use]
    pub fn time(&self) -> Millis {
        self.time
    }

    /// Aktueller Stand der Schul-Ressource.
    #[must_use]
    pub fn resource(&self) -> u16 {
        self.resource
    }

    /// Bisher verursachter Schaden.
    #[must_use]
    pub fn damage(&self) -> u32 {
        self.damage
    }

    /// Ob ein Zauber jetzt gewirkt werden kann: eigene Abklingzeit **und**
    /// globale Wartezeit abgelaufen.
    #[must_use]
    pub fn is_ready(&self, index: usize) -> bool {
        // **Hier stand bis zum 2026-08-05 eine Sperre** — "waehrend einer
        // Wirkzeit ist keiner bereit". Sie ist mit der Regel entfallen: Die
        // Wirkzeit sperrt nichts, jede Wirkung bricht sie ab (GAME-DESIGN 7.1,
        // "Was einen laufenden Zauber abbricht"). Dieselbe Stelle wie in
        // `Combat.isReady`.
        if self.cooldown_ready_at[index] > self.time {
            return false;
        }
        // **Wer keine Wartezeit ausloest, wird auch von keiner gesperrt**
        // (GAME-DESIGN 6.6/10.4, Issue #110).
        //
        // Bis hierher galt die Sperre fuer jeden Zauber. Fuer die drei aus
        // Schritt 1 war das richtig — sie loesen alle eine aus. Fuer den
        // Unterbrecher waere es die Ausnahme mit falschem Vorzeichen: Er
        // loeste zwar keine Wartezeit aus, koennte aber waehrend einer
        // laufenden nicht gewirkt werden. Gebraucht wird er genau dann.
        if !SPELLS[index].triggers_gcd {
            return true;
        }
        self.gcd_ready_at <= self.time
    }

    /// Ob ein Brand-Effekt gerade laeuft.
    ///
    /// `>=` und nicht `>`: Der letzte Tick faellt mit dem Ablauf zusammen
    /// (`STEP1-SPEC` 1.2, dort am 2026-07-28 zugunsten von 6 Ticks entschieden).
    #[must_use]
    pub fn dot_active(&self, index: usize) -> bool {
        self.dot_expires_at[index].is_some_and(|expires| expires >= self.time)
    }

    /// Restlaufzeit eines Brand-Effekts; 0, wenn er nicht laeuft.
    #[must_use]
    pub fn dot_remaining_ms(&self, index: usize) -> Millis {
        self.dot_expires_at[index].map_or(0, |expires| expires.saturating_sub(self.time))
    }

    /// Plant einen Weckruf ein, wenn die Entscheidung einen braucht.
    ///
    /// Nur, wenn er **frueher** liegt als das naechste ohnehin anstehende
    /// Ereignis — sonst fuellte sich die Schlange mit Weckrufen, die nichts
    /// bringen, und die Kapazitaet aus [`EVENT_CAPACITY`] waere schneller
    /// erreicht als noetig.
    ///
    /// Vor Ablauf der globalen Wartezeit ist eine erneute Pruefung ebenfalls
    /// zwecklos: Gewirkt werden kann ohnehin nicht.
    fn schedule_decision_wakeup<D: Decide + ?Sized>(&mut self, decider: &D, now: Millis) {
        let Some(wanted) = decider.next_wakeup(self) else {
            return;
        };
        let at = wanted.max(self.gcd_ready_at);
        if at <= now || at >= self.duration_ms {
            return;
        }
        if self.queue.peek_time().is_none_or(|next| at < next) {
            self.queue.push(at, EventKind::Wakeup);
        }
    }

    /// Restliche Abklingzeit eines Zaubers **jetzt**.
    #[must_use]
    pub fn cooldown_remaining_ms(&self, index: usize) -> Millis {
        self.cooldown_remaining_ms_at(index, self.time)
    }

    /// Ob die eigene Abklingzeit eines Zaubers abgelaufen ist.
    ///
    /// **Ohne** die globale Wartezeit — anders als [`Fight::is_ready`].
    /// Entscheidungen fallen ohnehin nur, wenn der GCD bereit ist; eine
    /// Prioritaetenliste soll deshalb nach der Abklingzeit fragen koennen, ohne
    /// die Wartezeit jedes Mal mitzudenken.
    #[must_use]
    pub fn cooldown_ready(&self, index: usize) -> bool {
        self.cooldown_ready_at[index] <= self.time
    }

    /// Verbleibende Lebenspunkte des Bosses.
    #[must_use]
    pub fn boss_hp_remaining(&self) -> u32 {
        self.boss.hp.saturating_sub(self.damage)
    }

    /// Verbleibende Lebenspunkte in Prozent, ganzzahlig abgerundet.
    ///
    /// Ganzzahlig, weil die Prioritaetenliste ganzzahlig rechnet — und weil ein
    /// Fliesskomma-Vergleich an dieser Stelle zwischen zwei Umsetzungen
    /// unterschiedlich ausfallen koennte.
    #[must_use]
    pub fn boss_hp_percent(&self) -> u32 {
        if self.boss.hp == 0 {
            return 0;
        }
        (self.boss_hp_remaining() * 100) / self.boss.hp
    }

    /// Um wie viele Prozentpunkte die Phasen-Schwelle gerade gesenkt ist.
    ///
    /// Fuer die Prueffaelle zu #100: Ohne diesen Zugang kann ein Fall nicht
    /// pruefen, dass die **Ursache** eintritt — dass die Senkung zum
    /// betrachteten Zeitpunkt ueberhaupt anliegt. Ein Fall, der das nicht
    /// pruefen kann, ist auch dann gruen, wenn er ins Leere laeuft.
    #[must_use]
    pub fn phase_hold_percent(&self) -> u32 {
        self.phase_hold_percent
    }

    /// Zeit bis zum Ueberdruck.
    #[must_use]
    pub fn fight_remaining_ms(&self) -> Millis {
        self.duration_ms.saturating_sub(self.time)
    }

    /// Restliche Abklingzeit eines Zaubers zu einem beliebigen Zeitpunkt.
    ///
    /// Der Zeitpunkt wird uebergeben statt aus dem Zustand genommen, weil die
    /// Pruefsumme ihn fuer **jeden Takt** braucht — auch fuer die, an denen der
    /// ereignisgesteuerte Simulator gar nicht anhaelt.
    #[must_use]
    pub fn cooldown_remaining_ms_at(&self, index: usize, time_ms: Millis) -> Millis {
        self.cooldown_ready_at[index].saturating_sub(time_ms)
    }

    /// Restlaufzeit eines Brand-Effekts zu einem beliebigen Zeitpunkt.
    #[must_use]
    pub fn dot_remaining_ms_at(&self, index: usize, time_ms: Millis) -> Millis {
        self.dot_expires_at[index].map_or(0, |expires| expires.saturating_sub(time_ms))
    }

    /// Wie oft ein Zauber gewirkt wurde.
    #[must_use]
    pub fn cast_count(&self, index: usize) -> u32 {
        self.casts[index]
    }

    /// Wie oft ein Brand-Effekt getickt hat.
    #[must_use]
    pub fn dot_tick_count(&self, index: usize) -> u32 {
        self.dot_tick_counts[index]
    }

    /// Durch zu fruehes Erneuern verfallene Ticks.
    #[must_use]
    pub fn lost_dot_ticks(&self, index: usize) -> u32 {
        self.lost_dot_ticks[index]
    }

    /// Am Deckel verpuffte Ressource.
    #[must_use]
    pub fn resource_overflow(&self) -> u32 {
        self.resource_overflow
    }

    /// Wie oft ein eigener Druck eine Wirkzeit abgebrochen hat.
    ///
    /// Für Prüffälle, die belegen wollen, dass ein Selbst-Abbruch WIRKLICH
    /// stattgefunden hat. Ohne diese Zusicherung misst ein solcher Fall
    /// womöglich einen Boss-Abbruch — und wäre rot aus dem falschen Grund.
    #[must_use]
    pub fn self_cancelled_count(&self) -> u32 {
        self.casts_cancelled
    }

    /// Ob RETORTs Rüstung gerade steht (Issue #480).
    ///
    /// Für Prüffälle: Die Rüstung ist von aussen sonst nur mittelbar sichtbar
    /// — über eine Wartezeit, die beim nächsten Boss-Abbruch ausfällt. Ein
    /// Fall, der belegen will, dass sie NICHT verbraucht wurde, käme damit
    /// nicht aus.
    #[must_use]
    pub fn retort_armed(&self) -> bool {
        self.retort_armed
    }

    /// Wann die globale Wartezeit ablaeuft.
    #[must_use]
    pub fn gcd_ready_at(&self) -> Millis {
        self.gcd_ready_at
    }

    /// Zustands-Pruefsumme zu einem Zeitpunkt (GAME-DESIGN 19.1).
    ///
    /// **Die Reihenfolge der Felder ist Teil der Schnittstelle** und stammt aus
    /// `combatChecksum` in `src/core/checksum.ts`. Ein Feld dazwischenzuschieben
    /// oder zwei zu vertauschen aendert jede Summe — dann meldet der Abgleich
    /// bei jedem Takt eine Abweichung und bei keinem eine echte.
    ///
    /// Welcher Wert als ganze Zahl und welcher als Fliesskommazahl eingeht, ist
    /// ebenfalls vorgegeben: Im Spiel sind Schaden, Wartezeit und Restzeiten
    /// `number`, also Fliesskomma, auch wenn sie ganzzahlige Werte tragen.
    #[must_use]
    pub fn checksum_at(&self, time_ms: Millis) -> u32 {
        let mut h = FNV_OFFSET;
        h = mix_named_int(h, "time", time_ms as i32);
        h = mix_named_float(h, "damage", f64::from(self.damage));
        h = mix_named_int(h, "resource", i32::from(self.resource));
        h = mix_named_int(h, "resourceOverflow", self.resource_overflow as i32);
        h = mix_named_int(h, "fightState", FIGHT_RUNNING);
        h = mix_named_float(h, "gcdReadyAt", f64::from(self.gcd_ready_at));
        for index in 0..SPELL_COUNT {
            h = mix_named_float(
                h,
                "cooldownRemaining",
                f64::from(self.cooldown_remaining_ms_at(index, time_ms)),
            );
            h = mix_named_float(
                h,
                "dotRemaining",
                f64::from(self.dot_remaining_ms_at(index, time_ms)),
            );
            h = mix_named_int(h, "casts", self.casts[index] as i32);
            h = mix_named_int(h, "dotTicks", self.dot_tick_counts[index] as i32);
            h = mix_named_int(h, "lostDotTicks", self.lost_dot_ticks[index] as i32);
        }
        for draws in self.rng.draw_counts() {
            h = mix_named_int(h, "randomDraws", draws as i32);
        }
        // **Angehaengt, nicht eingeschoben** (Issue #96). Die bestehenden
        // Positionen bleiben dadurch unveraendert; die Reihenfolge ist ohnehin
        // willkuerlich und muss nur auf beiden Seiten gleich sein.
        //
        // Ohne diese beiden Felder waere die Pruefsumme fuer einen Boss mit
        // Phasen blind: Zwei Laeufe in verschiedenen Phasen saehen gleich aus,
        // und der Abgleich meldete Uebereinstimmung, wo keine ist.
        h = mix_named_int(h, "bossPhase", self.phase as i32);
        h = mix_named_int(h, "bossEnergy", self.energy as i32);
        h = mix_named_int(h, "timelineIndex", self.timeline_index as i32);
        // **Angehaengt, nicht eingeschoben** (Issue #110). Ohne die
        // Lebenspunkte waere die Pruefsumme fuer einen Boss, der Schaden
        // austeilt, blind: Zwei Laeufe mit verschiedenem erlittenem Schaden
        // saehen gleich aus.
        h = mix_named_int(h, "playerHp", self.player_hp as i32);
        // Und der laufende Boss-Zauber, damit ein verpasstes Unterbrechen
        // auffaellt, bevor der Treffer ueberhaupt faellt.
        h = mix_named_int(h, "casting", self.casting.map_or(-1, |i| i as i32));
        // **Angehaengt, nicht eingeschoben** (Issue #79). Ohne den Schutz waere
        // die Pruefsumme fuer jeden Kampf mit Schutzschicht blind: Zwei Laeufe
        // mit verschieden viel aufgefangenem Schaden saehen gleich aus, solange
        // die Lebenspunkte zufaellig uebereinstimmen.
        h = mix_named_int(h, "shield", self.shield as i32);
        h = mix_named_int(h, "shieldStacks", self.shield_stacks as i32);
        // Die laufende Umwandlung gehoert dazu, nicht nur ihr Ergebnis: Sie
        // entscheidet ueber jeden kuenftigen Schlag, und ein Unterschied darin
        // faellt sonst erst auf, wenn er sich in Zahlen niedergeschlagen hat.
        h = mix_named_int(h, "conversionPercent", self.conversion_percent as i32);
        // **Der erste Merker in der Prüfsumme** (SUBLIME, Issue #480).
        //
        // ANGEHÄNGT, nicht eingeschoben — dieselbe Regel wie bei `control`
        // seit #100: Ein Einschub deutet jede bestehende Reihenfolge um.
        //
        // Warum ein Merker überhaupt hierher gehört: Er ist ein Kampfzustand,
        // der zwischen zwei Würfen lebt. Stünde er nicht hier, könnten beide
        // Seiten ihn verschieden führen — einer verbraucht ihn beim nächsten
        // Wurf, der andere beim übernächsten —, und keine Prüfung bemerkte es.
        // Die Zahlen stimmten weiter, bis irgendwann eine Rotation
        // auseinanderliefe.
        //
        // Für die vier Merker, die folgen (CONDENSE, RETORT, PUTRID BLOOM,
        // DECAY): hier anhängen, in derselben Reihenfolge auf beiden Seiten.
        h = mix_named_int(h, "nextCastFreeGcd", i32::from(self.next_cast_free_gcd));
        // Der zweite Merker, unmittelbar neben dem ersten (CONDENSE, #480).
        h = mix_named_int(h, "nextCastDoubles", i32::from(self.next_cast_doubles));
        // Der dritte Merker, neben den zwei anderen (RETORT, #480). Als
        // Millisekunden-Zahl statt Ja/Nein — die Begründung steht am Feld.
        h = mix_named_int(h, "retortArmed", i32::from(self.retort_armed));
        // **Angehaengt, nicht eingeschoben** (Issue #100). Ohne den
        // Kontroll-Vorrat waere die Pruefsumme fuer jeden Kampf mit Fixierung
        // blind, und die gesenkte Phasen-Schwelle entscheidet ueber jeden
        // kuenftigen Wechsel.
        h = mix_named_int(h, "control", self.control as i32);
        h = mix_named_int(h, "phaseHoldPercent", self.phase_hold_percent as i32);
        // **Angehaengt, nicht eingeschoben** (Issue #216, Zaehler und Stapel).
        // Ohne sie waere die Pruefsumme fuer einen Boss mit diesen Bausteinen
        // blind: Zwei Laeufe mit verschiedenem Zaehlerstand saehen gleich aus,
        // solange die Lebenspunkte uebereinstimmen — und beim Zaehler stimmen
        // sie ueberein, denn er nimmt keine.
        h = mix_named_int(h, "doom", self.doom as i32);
        h = mix_named_int(h, "doomCharges", self.doom_charges as i32);
        h = mix_named_int(h, "bossStacks", self.stacks as i32);
        // **Angehaengt, nicht eingeschoben** (Issue #97) — und zwar ans ECHTE
        // Ende. Mein erster Versuch setzte den Block hinter
        // `phase_hold_percent`, also VOR doom/doom_charges/stacks aus #216;
        // das Spiel haengt hinter stacks an. Dieselben Felder, andere
        // Reihenfolge — die Pruefsumme wich schon in Takt 0 ab, ohne dass ein
        // einziger Stapel lag.
        //
        // Die Zahl der Stapel allein genuegt nicht: Zwei Laeufe mit gleich
        // vielen, aber verschieden alten Stapeln laufen gleich weiter, bis der
        // erste verfaellt.
        h = mix_named_int(h, "decayStacks", self.decay_count as i32);
        // **Ueber die KAPAZITAET, nicht die Grenze** (PUTRID BLOOM, #479).
        for i in 0..DECAY_STACK_CAPACITY {
            h = mix_named_int(h, "decayExpiry", self.decay_expiry_at(i) as i32);
        }
        // **Angehaengt, nicht eingeschoben** (Issue #218). Ohne sie waere die
        // Pruefsumme fuer einen Boss mit Unverwundbarkeits-Fenster blind: Zwei
        // Laeufe mit verschieden viel verlorenem Schaden saehen gleich aus,
        // solange der ausgeteilte uebereinstimmt — und im Fenster stimmt er
        // ueberein, denn dort waechst er gar nicht.
        h = mix_named_int(h, "raceOpen", i32::from(self.race_ends_at.is_some()));
        h = mix_named_int(h, "raceProgress", self.race_progress as i32);
        h = mix_named_int(h, "damageLostToRace", self.damage_lost_to_race as i32);
        h = mix_named_int(h, "bossHealed", self.boss_healed as i32);
        // **Angehaengt, nicht eingeschoben** (Issue #240, die letzten drei
        // Bausteine) — dieselbe Folge wie in `src/core/checksum.ts`.
        //
        // Baustein 7: Die Eskalation wirkt ausschliesslich auf diesen
        // Zeitpunkt. Ohne ihn fiele eine falsch gerechnete Verkuerzung erst
        // auf, wenn der Treffer faellt — Takte spaeter, an einer Stelle, die
        // nicht mehr auf die Ursache zeigt. Denselben Wert verschiebt die
        // Fixierung (#100), fuer die er bisher ebenfalls fehlte.
        //
        // **Nur solange einer laeuft**, siehe `src/core/checksum.ts`: Ohne
        // laufenden Zauber traegt das Feld dort -1 und hier 0.
        h = mix_named_int(
            h,
            "castLandsAt",
            self.casting.map_or(-1, |_| self.cast_lands_at as i32),
        );
        // Baustein 8: Ob die Schwaechung liegt, entscheidet ueber jeden
        // weiteren Takt. Der Zaehler traegt den Unterschied zwischen "nie
        // gelegen" und "gelegen und entfernt".
        h = mix_named_int(h, "afflicted", i32::from(self.afflicted));
        h = mix_named_int(h, "afflictionsCleared", self.afflictions_cleared as i32);
        // Issue #256: Der Anteil des SPIELERS am Ring-Verlust. `damage` steht
        // schon an Position 2, mischt aber seit #247 zwei Herkuenfte. Zwei
        // Laeufe mit gleichem `damage` und verschiedenem Spieler-Anteil saehen
        // ohne dieses Feld identisch aus.
        //
        // **Angehaengt, nicht eingeschoben** — wie jedes Feld seit #96. Der
        // Name kommt aus #236 dazu, und er muss WOERTLICH derselbe sein wie
        // auf der Spielseite: Der Gleichlauf-Waechter vergleicht die Namen.
        h = mix_named_float(h, "playerDamage", f64::from(self.player_damage));
        // Issue #304: die laufende Wirkzeit des SPIELERS. Bis hierher trug die
        // Pruefsumme nur den Zustand des Bosses; fuer den Spieler gab es
        // keinen, weil eine Wirkzeit nie lief.
        //
        // **Ohne diese zwei Felder waere die Pruefung an ihrer empfindlichsten
        // Stelle blind:** Ein Zauber, der auf einer Seite eine Wirkzeit hat und
        // auf der anderen sofort trifft, ergaebe in jedem Takt dazwischen
        // dieselbe Summe — gleicher Schaden (noch keiner), gleiche Abklingzeit
        // (noch keine), gleiche Wartezeit. Erst der Treffer liefe auseinander,
        // und die Pruefsumme zeigte auf den Takt des Treffers statt auf den des
        // Fehlers.
        //
        // **Angehaengt, nicht eingeschoben** — dieselbe Folge und dieselben
        // Namen wie in `src/core/checksum.ts`.
        h = mix_named_int(
            h,
            "playerCasting",
            self.player_casting.map_or(-1, |i| i as i32),
        );
        // Dieselbe Konvention wie `castLandsAt`: ohne laufende Wirkzeit ein
        // fester Wert auf beiden Seiten.
        h = mix_named_int(
            h,
            "playerCastEndsAt",
            self.player_casting
                .map_or(-1, |_| self.player_cast_ends_at as i32),
        );
        // Issue #318: Die Synergie-Fenster. Ohne sie liefe ein Simulator, der
        // das Fenster anders lang rechnet oder es nicht verbraucht, still
        // auseinander — der Schaden wiche erst dann ab, wenn der Partner
        // wirklich im Fenster wirkt, und das kann Takte spaeter sein oder nie.
        //
        // **Angehaengt, nicht eingeschoben** — ans ECHTE Ende, hinter
        // playerDamage. Die Namen sind woertlich dieselben wie auf der
        // Spielseite; der Gleichlauf-Waechter vergleicht sie.
        h = mix_named_int(h, "synergyUsed", self.synergy_used as i32);
        h = mix_named_int(h, "synergyAvailable", self.synergy_available as i32);
        h = mix_named_float(h, "synergyOpenUntil", self.synergy_window_sum());
        // **Angehaengt, nicht eingeschoben** (#493). Ohne dieses Feld ist die
        // Pruefsumme fuer ein laufendes Verstaerkungs-Fenster blind: Zwei
        // Laeufe mit gleichem Schaden, einer im Fenster und einer danach,
        // ergaben dieselbe Summe. Gemessen auf der Spiel-Seite.
        //
        // Alle fuenf Fenster in EINER Zahl, wie dort — jedes weitere Feld
        // verschoebe die Bit-Folge auf beiden Seiten.
        h = mix_named_float(
            h,
            "buffSum",
            self.buffs.window_sum()
                + self.control_buffs.window_sum()
                + self.resource_buffs.window_sum()
                + self.tick_buffs.window_sum()
                + self.stack_buffs.window_sum()
                + self.cd_reduce_buffs.window_sum()
                + self.convert_buffs.window_sum()
                + self.cap_buffs.window_sum(),
        );
        // **Angehaengt, nicht eingeschoben** (RIPOSTE, Issue #481). Das
        // laufende Rueckschlag-Fenster gehoert hierher aus demselben Grund wie
        // `conversionPercent`: Es entscheidet ueber jeden kuenftigen Schlag.
        //
        // Der ZAEHLER `reflected` steht hier NICHT — er ist kumulativ und hat
        // eine eigene Leitung ins Ergebnis, genau wie `converted`.
        h = mix_named_int(h, "reflectPercent", self.reflect_percent as i32);
        h
    }

    /// Welche Phase gerade laeuft. 0 ist der Start.
    #[must_use]
    pub fn phase(&self) -> usize {
        self.phase
    }

    /// Stand der Energieleiste.
    #[must_use]
    pub fn energy(&self) -> u32 {
        self.energy
    }

    /// Wie oft die Energieleiste ausgeloest hat.
    #[must_use]
    pub fn energy_triggers(&self) -> u32 {
        self.energy_triggers
    }

    /// Wie weit die Boss-Zeitleiste abgearbeitet ist.
    #[must_use]
    pub fn timeline_index(&self) -> usize {
        self.timeline_index
    }

    /// Verbleibende Lebenspunkte des Spielers.
    #[must_use]
    pub fn player_hp(&self) -> u32 {
        self.player_hp
    }

    /// Gesamter erlittener Schaden, **nach** WARD.
    #[must_use]
    pub fn damage_taken(&self) -> u32 {
        self.damage_taken
    }

    /// Ist der Spieler tot? Der Versuch endet dann (GAME-DESIGN A.3).
    #[must_use]
    pub fn player_dead(&self) -> bool {
        self.died_at.is_some()
    }

    /// Welcher Boss-Zauber gerade laeuft, als Index in `Boss::timeline`.
    #[must_use]
    pub fn casting(&self) -> Option<usize> {
        self.casting
    }

    /// Wie oft der Spieler einen Boss-Zauber unterbrochen hat.
    #[must_use]
    pub fn interrupts_landed(&self) -> u32 {
        self.interrupts_landed
    }

    /// Stand der Schutzschicht (COAGULATION, Weg 1).
    #[must_use]
    pub fn shield(&self) -> u32 {
        self.shield
    }

    /// Wie viele Anwendungen der Schutzschicht uebereinanderliegen.
    #[must_use]
    pub fn shield_stacks(&self) -> u32 {
        self.shield_stacks
    }

    /// Laeuft das Fenster der Verstaerkung, die dieser Zauber legt? (#553)
    ///
    /// # Die Uebersetzung, die es vorher nicht gab
    ///
    /// Verstaerkungen liegen nach ART und Zielgroesse (Issue #489) — acht
    /// getrennte Vorraete, in jedem `factors[kind]`. Eine Rotation kennt aber
    /// Zauber: `kindle if buff.flashpoint.active` ist die Frage, die ein
    /// Spieler stellt, und sie war bis #553 nicht stellbar.
    ///
    /// Traegt der Zauber keine Verstaerkung, ist die Antwort `false`. Das ist
    /// KEIN Fehler: Eine Bedingung auf einen Zauber ohne Fenster greift nie,
    /// und das ist eine gueltige (wenn auch nutzlose) Rotation. Ein Fehler
    /// waere hier eine Aussage ueber die Absicht des Schreibers, die der
    /// Parser nicht treffen kann.
    #[must_use]
    pub fn buff_active(&self, spell: usize) -> bool {
        let Some(buff) = SPELLS.get(spell).and_then(|s| s.buff) else {
            return false;
        };
        self.buff_store(buff.target).is_active(buff.kind, self.time)
    }

    /// Wie lange das Fenster dieses Zaubers noch laeuft, in ms (#553).
    ///
    /// `0`, wenn es nicht laeuft oder der Zauber keines legt.
    #[must_use]
    pub fn buff_remaining_ms(&self, spell: usize) -> Millis {
        let Some(buff) = SPELLS.get(spell).and_then(|s| s.buff) else {
            return 0;
        };
        self.buff_store(buff.target)
            .remaining_ms(buff.kind, self.time)
    }

    /// Der Vorrat zu einer Zielgroesse.
    ///
    /// Dieselbe Zuordnung wie beim Anlegen in `cast` — sie steht hier ein
    /// zweites Mal, weil dort ein `&mut` gebraucht wird und hier ein `&`.
    /// Wer eine Zielgroesse ergaenzt, muss BEIDE Stellen anfassen; der
    /// Uebersetzer erzwingt es, weil `match` auf `BuffTarget` vollstaendig
    /// sein muss.
    const fn buff_store(&self, target: BuffTarget) -> &Buffs {
        match target {
            BuffTarget::Damage => &self.buffs,
            BuffTarget::Control => &self.control_buffs,
            BuffTarget::Resource => &self.resource_buffs,
            BuffTarget::Tick => &self.tick_buffs,
            BuffTarget::Stack => &self.stack_buffs,
            BuffTarget::CdReduce => &self.cd_reduce_buffs,
            BuffTarget::Convert => &self.convert_buffs,
            BuffTarget::Cap => &self.cap_buffs,
        }
    }

    /// Wie viel Schaden die Schutzschicht insgesamt aufgefangen hat.
    #[must_use]
    pub fn shield_absorbed(&self) -> u32 {
        self.shield_absorbed
    }

    /// Der Verwundbarkeits-Faktor aus den Faeulnis-Stapeln (Issue #738).
    ///
    /// **Das Gegenstueck zu `combat.vulnerability`** auf der Spiel-Seite
    /// (`combat.ts:2868`), das dort von drei Pruefdateien gelesen wird. Auf
    /// dieser Seite gab es keines — und die Paritaet uebertraegt `buffs`
    /// takt-fuer-takt, `vulnerability` gar nicht. Zwei Faktoren derselben
    /// Familie, einer verglichen, der andere nicht.
    #[must_use]
    pub fn vulnerability(&self) -> f64 {
        self.factors.vulnerability
    }

    /// Wie viel Leben insgesamt zurueckgeholt wurde (Weg 2).
    #[must_use]
    pub fn healed(&self) -> u32 {
        self.healed
    }

    /// Wie viel Fortschritt der BOSS zurueckgedreht hat (Baustein 5, #218).
    ///
    /// **Nicht zu verwechseln mit [`healed`](Self::healed)** — das ist die
    /// Heilung des SPIELERS. Diese Verwechslung hat am 2026-08-07 zu der
    /// Aussage gefuehrt, boss-1-4 heile nicht, weil der Laeufer fuer
    /// "Leben zurueckgeholt" 0.0 zeigte: Er zeigte die Spielerzahl.
    pub fn boss_healed(&self) -> u32 {
        self.boss_healed
    }

    /// Wie viel Schutz die Umwandlung erzeugt hat (Weg 3).
    #[must_use]
    pub fn converted(&self) -> u32 {
        self.converted
    }

    /// Der Boss, gegen den gerechnet wird.
    #[must_use]
    pub fn boss(&self) -> &'static Boss {
        self.boss
    }

    /// Die Schul-Mechanik.
    #[must_use]
    pub fn resource_data(&self) -> &'static Resource {
        &RESOURCE
    }

    /// Das Kampfprotokoll.
    #[must_use]
    pub fn log(&self) -> &CombatLog {
        &self.log
    }

    /// Die Zufallsstroeme — schreibend, weil Ziehen den Zustand fortschreibt.
    pub fn rng(&mut self) -> &mut Streams {
        &mut self.rng
    }

    /// Ziehungen aus einem Strom.
    #[must_use]
    pub fn draws(&self, stream: Stream) -> u32 {
        self.rng.draws(stream)
    }
}

/// Die Lebenspunkte aus Stufe **und** Ausruestung (Issue #282, AC3).
///
/// # Warum eine eigene Funktion und keine Zeile im Aufbau
///
/// Beide Seiten muessen dieselbe Rundung treffen. Steht die Rechnung an einer
/// Stelle, ist sie eine Zeile zum Vergleichen; steht sie zweimal im Aufbau,
/// muss man sie suchen.
///
/// **Ganzzahlig gerundet, nicht in Fliesskomma gehalten:** Lebenspunkte sind
/// ganze Zahlen, und ein Rest, der erst spaeter abgeschnitten wird, ist ein
/// Rest, den die andere Seite anders abschneiden koennte.
#[must_use]
pub fn vessel_with_gear(base: u32, worn_permille: u32) -> u32 {
    if worn_permille == 0 {
        return base;
    }
    u32::try_from(u64::from(base) * (1000 + u64::from(worn_permille)) / 1000).unwrap_or(u32::MAX)
}
