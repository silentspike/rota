/**
 * Die Kampfregeln (GAME-DESIGN 6, 7, 18.3, STEP1-SPEC).
 *
 * Ganzzahlige Zeit in Millisekunden, feste Reihenfolge der Rechenschritte. Diese
 * Datei ist die Seite des Spiels an der Schnittstelle, die es sich mit dem
 * Rust-Simulator teilt — jede Regel hier muss dort gleich lauten.
 *
 * **Reihenfolge je Logik-Takt**, wie in `tools/step1-check.py`:
 *
 *   1. laufende Wirkungen (DoT-Ticks, Verfall des Vorrats)
 *   2. Prüfung, ob der Boss gefallen ist
 *   3. Tastendrücke dieses Takts
 *
 * Die Reihenfolge ist nicht beliebig: Ein DoT-Tick, der den Boss legt, muss zählen,
 * bevor im selben Takt noch etwas gewirkt wird.
 */
import { computeDamage, NEUTRAL_FACTORS, stackDamage, type DamageFactors } from './damage.js';
import { BIG_HIT_HP_SHARE, HIT_DIRECT, HIT_DOT, type HitFn } from './hits.js';
import {
  EVENT_BOSS_ENERGY_TRIGGERED,
  EVENT_BUFF_EXPIRE,
  EVENT_BOSS_TIMELINE_FIRED,
  EVENT_CAST,
  EVENT_CONTROL_SPENT,
  EVENT_DAMAGE,
  EVENT_DOT_TICK,
  EVENT_HEALED,
  EVENT_INTERRUPT_LANDED,
  EVENT_PHASE,
  EVENT_PLAYER_DAMAGE,
  EVENT_SHIELD_ABSORB,
  NO_SPELL_REF,
  type CombatLog,
} from './log.js';
import {
  PLAYER,
  statAtLevel,
  type BossDef,
  type SchoolDef,
  type SpellDef,
  type TimelineEntryDef,
  type DotDef,
  type BuffDef,
} from './data.js';
import { GCD_MS, GlobalCooldown } from './gcd.js';
import { RandomStreams, STREAM_CRIT, STREAM_DOT_TICK } from './rng.js';
import { hastedMs } from './haste.js';
import {
  NOTHING_WORN,
  critChance,
  critFactor,
  essenceFactor,
  vesselWithGear,
  gearEffectFactor,
  type WornValues,
} from '../gear/worn.js';
import {
  PERPETUAL_DRAFT_EXTRA_CHARGE,
  SET_EFFECT_ASHEN_REGIMEN,
  SET_EFFECT_PERPETUAL_DRAFT,
  SET_EFFECT_SLOW_ROT,
  SET_EFFECT_SEALED_VESSEL,
  SET_EFFECT_LONG_GRIP,
  SET_EFFECT_WHOLE_WORK,
  WHOLE_WORK_GCD_REDUCTION_MS,
} from '../gear/sets.js';
import {
  hasKeystone,
  KEYSTONE_SPLIT_SPENDER,
  KEYSTONE_AUTO_RESET,
  KEYSTONE_DECAY_ECHO,
  KEYSTONE_PARTIAL_CONTROL,
  KEYSTONE_SHIELD_REFUND,
  NO_TALENTS,
  talentFactor,
  talentSpenderDamageFactor,
  talentResourceFactor,
  talentDotDurationFactor,
  talentShieldAmountFactor,
  talentShieldDurationFactor,
  talentShieldStacksBonus,
  talentCooldownReductionFactor,
  talentCooldownChargesBonus,
  talentCooldownThresholdFactor,
  talentControlCostFactor,
  talentControlEffectFactor,
  talentControlGainFactor,
  talentDecayEffectFactor,
  talentDecayStackPermille,
  talentDecayDurationFactor,
  type TalentAllocation,
} from '../progression/talents.js';

/**
 * Die Faktoren, wie `Combat` sie INTERN haelt (Issue #196).
 *
 * `DamageFactors` ist nach aussen unveraenderlich, und das bleibt so: Wer sie
 * bekommt, soll sie nicht anfassen koennen. Innerhalb dieser Klasse muessen
 * sie sich aber bewegen duerfen -- sonst waere ein Faktor, der sich im Kampf
 * aendert, gar nicht abbildbar.
 *
 * Der Schreibzugriff bleibt trotzdem auf EINE Stelle beschraenkt: Ein
 * Prueffall liest diese Datei und faengt jede andere.
 */
type MutableFactors = { -readonly [K in keyof DamageFactors]: DamageFactors[K] };
import {
  RULE_CRIT,
  RULE_DAMAGE,
  RULE_DOT_TICK,
  RULE_SPELL_CAST,
  RuleEvents,
} from '../combat/events.js';
import { Buffs, BUFF_KIND_CAPACITY } from '../combat/schools/buffs.js';
import { TICK_MS } from './loop.js';
import { registerStackMechanic, type StackMechanic } from '../combat/schools/stacks.js';
import { registerSublimation, type SublimationMechanic } from '../combat/schools/sublimation.js';

/** Ausgang eines Wirk-Versuchs. */
export const CAST_OK = 0;
export const CAST_UNKNOWN = 1;
export const CAST_ON_COOLDOWN = 2;
export const CAST_ON_GCD = 3;
export const CAST_FIGHT_OVER = 4;
/**
 * Der Zauber hat **begonnen** — seine Wirkzeit läuft (Issue #304).
 *
 * Ein eigener Ausgang und nicht `CAST_OK`, weil die beiden verschiedene Dinge
 * behaupten: `CAST_OK` heißt „gewirkt, Schaden steht", dieser hier heißt
 * „angefangen, Ausgang offen". Wer sie zusammenlegte, bekäme ein `outcome.damage`
 * von 0, das nach einem Treffer ohne Wirkung aussieht.
 *
 * **Für die Eingabe zählen beide als angenommen** — der Druck war richtig, und
 * ein rotes Blinken wäre eine Lüge. Deshalb `CAST_ACCEPTED` als gemeinsame
 * Prüfung an den zwei Stellen, die das entscheiden.
 */
export const CAST_STARTED = 5;
/**
 * **Hier stand `CAST_BUSY`** — der Ausgang „es läuft bereits eine Wirkzeit".
 *
 * Er ist am 2026-08-05 entfallen, weil der Auftraggeber die Regel umgedreht
 * hat: Die Wirkzeit sperrt **nichts**. Jede Wirkung bricht einen laufenden
 * Zauber ab, auch das Unterbrechen (GAME-DESIGN 7.1, „Was einen laufenden
 * Zauber abbricht"). Ein Druck während der Wirkzeit ist damit kein
 * abgewiesener Druck mehr, sondern eine Entscheidung — und sie kostet den
 * laufenden Zauber.
 *
 * Die Nummer 6 bleibt frei und wird nicht nachbesetzt: Diese Werte stehen in
 * keiner eingecheckten Datei, aber eine wiederverwendete Nummer wäre bei der
 * nächsten Suche eine falsche Fährte.
 */

/** Ob dieser Ausgang ein angenommener Tastendruck ist (Issue #304). */
export function isCastAccepted(result: number): boolean {
  return result === CAST_OK || result === CAST_STARTED;
}

/** Stand des Kampfes. */
export const FIGHT_RUNNING = 0;
export const FIGHT_WON = 1;
export const FIGHT_OVERPRESSURE = 2;
/**
 * Der Spieler ist gestorben (GAME-DESIGN A.3, Issue #581).
 *
 * **Ein eigenes Ende, kein Sonderfall des Überdrucks.** GAME-DESIGN nennt in
 * Kapitel 10 vier davon — „Sieg · Spieler-Tod · Zeitablauf (Überdruck) ·
 * Abbruch" —, und der Tod steht dort neben dem Zeitablauf, nicht darunter.
 *
 * Der Kommentar an `checkDefeat` warnt vor einem eigenen Zustand („eine dritte
 * Wahrheit über dasselbe Ende"). Er meint die Umkehrung durch `runaway`: Dort
 * führen zwei Wege zu DEMSELBEN Ende, und ein eigener Zustand hätte nichts
 * unterschieden. Hier ist es umgekehrt — ein anderes Ende, das die Auswertung
 * bereits kennt: `OUTCOME_DEATH` und die Überschrift `VESSEL LOST` stehen seit
 * jeher da und waren nur nie erreichbar.
 */
export const FIGHT_DIED = 3;

export const NOT_ACTIVE = -1;

// -- PUTREFACTION (Issue #97) --
//
// Die drei Größen der Zersetzungs-Stapel. **Alle drei sind vorläufig und
// werden gemessen** (GAME-DESIGN 4.1, Teil F.6): Wie viel ein Stapel trägt,
// wie viele es höchstens gibt und wie lange sie halten, bestimmt der
// Balance-Läufer — nicht diese Datei. Sie tragen bis dahin die Prüffälle.
//
// Der Simulator führt dieselben drei Werte in `fight.rs`; die Prüfsumme je
// Takt hält beide Seiten zusammen.

/** Wie viele Stapel höchstens gleichzeitig liegen. Wird gemessen (#118). */
export const MAX_DECAY_STACKS = 5;

/**
 * Wie viele Stapel die **Struktur** fasst — Feldlänge und Prüfsummen-Schleife
 * (PUTRID BLOOM, Issue #479).
 *
 * # Warum zwei Konstanten, wo bisher eine reichte
 *
 * `MAX_DECAY_STACKS` ist die **Grenze**: Ab ihr legt kein Zauber mehr nach.
 * PUTRID BLOOM hebt genau diese Grenze für eine Weile — und wäre sie zugleich
 * die Feldlänge, änderte sich mit ihr die Zahl der Werte, die in die Prüfsumme
 * gehen. Eine Prüfsumme, deren LÄNGE vom Spielverlauf abhängt, ist keine mehr:
 * Zwei Läufe wären nicht vergleichbar, sobald einer den Zauber wirkt.
 *
 * Deshalb ist die Struktur **fest** und die Grenze beweglich. Die unbelegten
 * Plätze mischen als 0 — das tat der Bau schon vorher so
 * (`decayExpiryAt` gibt oberhalb der Zahl 0 zurück), es fällt hier nur nicht
 * mehr mit der Grenze zusammen.
 *
 * # Warum acht
 *
 * Sie muss über jedem Wert liegen, den die Grenze je erreichen kann: fünf plus
 * der größte Zuschlag, den ein Zauber gibt. PUTRID BLOOM gibt heute zwei; acht
 * lässt Luft für einen zweiten Zauber derselben Art, ohne dass die Struktur
 * erneut wächst — und jedes Wachstum kostet einen Neu-Export ALLER
 * Aufzeichnungen.
 *
 * **Der Parser hält das nach**: `data.ts` weist einen `decayCap`-Zuschlag
 * zurück, der über die Kapazität hinausführt. Eine Zahl, die nur hier steht,
 * wäre eine Zusicherung ohne Wächter.
 */
export const DECAY_STACK_CAPACITY = 8;

/**
 * Um wie viel Prozent ein Stapel den Schaden aller Quellen erhöht.
 *
 * **Entschieden, nicht offen** — Issue #224, 2026-08-05. Bis Issue #482 stand
 * hier 3 mit dem Vermerk „Wird gemessen"; die Messung hatte längst
 * stattgefunden, nur die Umsetzung fehlte.
 *
 * # Warum 6 und nicht 3
 *
 * GAME-DESIGN 221 rechnet es vor: Der Break-even, ab dem sich der Platz auf
 * der Leiste überhaupt rechnet, liegt bei **5,4 %** je Stapel. Mit 3 % bliebe
 * PUTREFACTION **nie** die beste Wahl — und das ist etwas anderes als „allein
 * gespielt schwach", was das Design über die Schule sagt.
 *
 * # Der Wert steht auf BEIDEN Seiten
 *
 * `DECAY_STACK_PERCENT` in `sim/src/fight.rs` trägt dieselbe Zahl. Laufen sie
 * auseinander, meldet es die Prüfsumme ab dem ersten Takt, in dem ein Stapel
 * liegt — vier Aufzeichnungen kommen dorthin.
 */
export const DECAY_STACK_PERCENT = 6;

/**
 * Um wie viel Prozent ein offenes Synergie-Fenster den Schaden erhöht
 * (GAME-DESIGN 5 und 1193, Issue #318).
 *
 * **Gesetzt, nicht gemessen.** GAME-DESIGN regelt Fenster und Stapel-Regel und
 * schweigt zur Höhe — die bestimmt der Balance-Läufer, wie die Punktekurve.
 * 25 % liegt in der Mitte des Korridors, den die Widerstände benutzen (20–30 %),
 * und ist damit vergleichbar.
 *
 * Prüfbar wird der Wert, sobald ein Boss die Schulwahl belohnt (#328); #237
 * kann ihn dann fahren.
 */
export const SYNERGY_BONUS_PERCENT = 25;

/** Wie lange ein einzelner Stapel hält. Wird gemessen. */
export const DECAY_STACK_DURATION_MS = 12_000;

/**
 * Wie lange die Phasen-Senkung aus ANCHOR gilt (GAME-DESIGN 4.2b).
 *
 * **Vorläufig, nicht balanciert** — Katalog-Inhalt, GAME-DESIGN F.6 setzt die
 * Zahl auf E10 an. Sie steht hier und nicht in den Zauber-Daten, weil sie eine
 * Eigenschaft der WIRKUNG ist und nicht des Zaubers: Jeder künftige Zauber mit
 * `holdPhase` soll dieselbe Frist haben, sonst wäre „hinauszögern" bei jedem
 * etwas anderes. Dieselbe Zahl steht in `sim/src/fight.rs`.
 */
export const PHASE_HOLD_DURATION_MS = 8_000;

/**
 * Ein Zauber-Index, für den Übersetzer von einer beliebigen Zahl unterscheidbar
 * (Issue #372).
 *
 * # Warum eine Marke und nicht `number`
 *
 * Ein Zauber-Index, eine Belegungsnummer, eine Kampfzeit und eine Schadenszahl
 * sind alle `number`. Der Übersetzer kann sie nicht auseinanderhalten, und
 * deshalb brauchte es bis #369 ein Suchmuster, um handgeschriebene Indizes zu
 * finden — eins, das 24 Stellen meldete, von denen 8 echt waren, und 13
 * versteckte Konstanten übersah.
 *
 * **Eine Allaussage über Quelltext belegt man nicht durch Suchen, sondern indem
 * man sie erzwingt.** Mit dieser Marke wird jede rohe Zahl an einer
 * Zauber-Stelle ein Übersetzungsfehler — auch die, die kein Muster kennt.
 *
 * Zur Laufzeit ist sie nicht vorhanden: `SpellIndex` ist eine `number`, die
 * Marke existiert nur im Typsystem. Keine Aufzeichnung bewegt sich dadurch.
 */
declare const spellIndexMarke: unique symbol;
export type SpellIndex = number & { readonly [spellIndexMarke]: 'spell' };

/**
 * Eine Zahl als Zauber-Index kennzeichnen — **die sichtbare Ausnahme**.
 *
 * Wer sie ruft, umgeht die Prüfung des Übersetzers und sagt es damit. Der
 * einzige gültige Grund ist eine Zahl, die gerade KEINEN Zauber treffen soll
 * (`cast(99, …)` prüft die Abweisung eines unbekannten Index).
 */
export function alsSpellIndex(zahl: number): SpellIndex {
  return zahl as SpellIndex;
}

/**
 * Der leere Platz — **mit der Marke, nicht als nackte `-1`** (Issue #442).
 *
 * Er steht im selben Wertebereich wie ein Zauber-Index und wird an denselben
 * Stellen gereicht. Ohne die Marke müsste jeder Aufrufer von `bind(slot,
 * NO_SPELL)` ein `alsSpellIndex` schreiben — und eine Umgehung, die man an
 * fünfzehn Stellen wiederholt, hebt den Sinn des Typs auf.
 *
 * **Er steht seit #444 HIER und nicht mehr in `session.ts`**, und das ist der
 * Grund, warum dieses Issue überhaupt baubar war: `combat.ts` braucht ihn an
 * fünf Stellen, an denen vorher `-1` stand. Ein Import aus `session.ts` wäre
 * ein Zirkelbezug gewesen — die Datei holt bereits aus dieser hier. Eine rohe
 * Umschreibung an jeder der fünf Stellen hätte die Umgehungen vermehrt statt
 * sie zu verringern, also das Gegenteil des Ziels bewirkt.
 *
 * `session.ts` reicht ihn weiter, damit jede bestehende Importstelle gilt.
 */
export const NO_SPELL = alsSpellIndex(-1);

export interface CombatOptions {
  readonly school: SchoolDef;
  readonly boss: BossDef;
  /** Startwert der Zufallsströme. In Schritt 1 zieht noch niemand daraus. */
  readonly seed?: number;
  /**
   * Die getragenen Sonderwerte (GAME-DESIGN 6.7, Issue #177).
   *
   * **Fehlt der Wert, wird nichts getragen** — und daran hängt die Zusicherung
   * aus AC7: Der Kampf läuft dann durch dieselben Zweige wie vorher, keine Zeit
   * wird umgerechnet, kein Zufallsstrom gedreht, keine Prüfsumme bewegt.
   */
  readonly worn?: WornValues;
  /**
   * Die verteilten Talente (Issue #417).
   *
   * Wie `worn` eine Größe, die für den ganzen Kampf gilt und sich darin nicht
   * ändert: Umskillen ist zwischen Kämpfen kostenlos (GAME-DESIGN 12.4), nicht
   * währenddessen.
   */
  readonly talents?: TalentAllocation;
  /**
   * Die Stufe des Spielers (Issue #289).
   *
   * **Fehlt sie, gilt Stufe 1** — und daran hängt der Gegenfall: Der
   * Referenzlauf und jede eingecheckte Vergleichsdatei geben keine Stufe an.
   * Wäre die Vorgabe eine andere, wanderte jede Aufzeichnung.
   *
   * Bis #289 kannte der Kampf die Stufe gar nicht: `statAtLevel` gab es seit
   * #248 mit siebzehn Prüffällen und keinem Aufrufer, und ein Spieler auf
   * Stufe 100 war genauso stark wie einer auf Stufe 1 — während das Player
   * Home ihm „Stat growth x4.116" zeigte.
   */
  readonly level?: number;
}

/**
 * Was ein Wirken bewirkt hat. **Ein einziges, wiederverwendetes Objekt** — der
 * Kampf darf im laufenden Bild nichts erzeugen (GAME-DESIGN 18.2).
 */
export interface CastOutcome {
  result: number;
  /**
   * Welcher Zauber gewirkt wurde — **mit der Marke** (Issue #444).
   *
   * `NO_SPELL`, wenn der Versuch keinen Zauber traf. Bis hierher stand hier
   * eine nackte Zahl, und ein Aufrufer konnte einen Platz-Index einsetzen,
   * ohne dass etwas es gemeldet hätte.
   */
  spellIndex: SpellIndex;
  /** Zugefügter Schaden, Direkt- und Entladeschaden zusammen. */
  damage: number;
  /** Vorrat vor dem Wirken — für die Schadens-Herleitung (GAME-DESIGN 19.1). */
  resourceBefore: number;
  /** Hat der Entlade-Zauber seinen Bonus mitgenommen? */
  bonusApplied: boolean;
}

export class Combat {
  readonly school: SchoolDef;
  /**
   * Der Boss dieses Kampfes.
   *
   * **Nicht mehr `readonly` seit Issue #613**, weil der Spieler zwischen den
   * Kämpfen einen anderen wählt. Geändert wird er ausschließlich über
   * {@link switchBoss} — direkt zuzuweisen ließe die Widerstände des alten
   * stehen, siehe dort.
   */
  boss: BossDef;
  readonly spells: readonly SpellDef[];
  readonly gcd = new GlobalCooldown();
  readonly random: RandomStreams;

  /**
   * Empfänger für Treffer (GAME-DESIGN 9 „Wucht").
   *
   * Bleibt `null`, wenn niemand zuhört — dann kostet der Kampf keinen Aufruf.
   */
  onHit: HitFn | null = null;

  /**
   * Das Kampfprotokoll (GAME-DESIGN 18.2). Bleibt `null`, wenn niemand mitschreibt.
   *
   * Geschrieben wird in **Kampfzeit**, nicht in der durchlaufenden Uhr: Ein
   * Protokoll, dessen Nullpunkt vom Laden der Seite abhängt, lässt sich zwischen
   * zwei Läufen nicht vergleichen.
   */
  log: CombatLog | null = null;

  /**
   * Das Ereignis-System (GAME-DESIGN 18.3, Issue E3-1b).
   *
   * **Erst der Schaden, dann die Wirkung** — Entscheidung des Auftraggebers
   * vom 2026-07-29. „Zauber gewirkt" meldet eine **abgeschlossene** Handlung:
   * Ein Empfänger beeinflusst den nächsten Zauber, nicht den laufenden. Der
   * Zersetzungs-Stapel der PUTREFACTION wirkt damit ab dem nächsten Treffer,
   * nicht auf den auslösenden.
   *
   * Der Grund ist kein Geschmack: Bei der umgekehrten Reihenfolge könnte ein
   * Effekt den Schaden verändern, der ihn ausgelöst hat, und solche
   * Rückkopplungen lassen sich in einer Optimums-Suche später nicht mehr
   * auseinanderdividieren.
   *
   * Dieselbe Reihenfolge wie das Kampfprotokoll — zwei Erzählungen desselben
   * Kampfes, die auseinanderliefen, wären beim Abgleich schlimmer als eine.
   */
  readonly events = new RuleEvents();

  /** Wiederverwendetes Ergebnis-Objekt, siehe `CastOutcome`. */
  private readonly outcome: CastOutcome = {
    result: CAST_OK,
    spellIndex: NO_SPELL,
    damage: 0,
    resourceBefore: 0,
    bonusApplied: false,
  };

  /**
   * Ein ZWEITES Ergebnis-Objekt, für die Wirkung nach Ablauf einer Wirkzeit.
   *
   * Nicht dasselbe wie `outcome`: Beide fallen im selben Takt an — erst die
   * Auflösung in `stepPassive`, dann der Tastendruck. Ein gemeinsames Objekt
   * überschriebe das eine mit dem anderen, und der Aufrufer von `cast()`
   * bekäme Zahlen aus einer fremden Wirkung.
   *
   * Wie `outcome` einmalig angelegt: Der Kampf darf im Takt nichts erzeugen
   * (GAME-DESIGN 18.2).
   */
  private readonly deferredOutcome: CastOutcome = {
    result: CAST_OK,
    spellIndex: NO_SPELL,
    damage: 0,
    resourceBefore: 0,
    bonusApplied: false,
  };

  private readonly indexById = new Map<string, number>();
  private readonly cooldownReadyAtMs: Float64Array;
  private readonly castCounts: Int32Array;
  /**
   * Bis wann das Synergie-Fenster dieses Zaubers offen steht (Issue #318).
   *
   * Index ist der **Partner**, nicht der Auslöser: Gefragt wird beim Wirken
   * „steht MIR ein Fenster offen", und das ist eine Abfrage ohne Suche.
   */
  private readonly synergyWindowEndsMs: Float64Array;
  private synergyUsedCount = 0;
  private synergyAvailableCount = 0;
  private readonly damageBySpell: Float64Array;
  private readonly dotExpiresAtMs: Float64Array;
  private readonly dotNextTickAtMs: Float64Array;

  /**
   * Bis wann ein **erloschener** Brand noch Zersetzungs-Stapel legt (#419,
   * PUTREFACTIONs Schlüsseltalent).
   *
   * Der Wert ist die Frist des jüngsten Stapels, der beim letzten Tick lag —
   * fortgeschrieben, solange der Brand läuft, sodass beim Erlöschen von selbst
   * der richtige Endpunkt dasteht. `NOT_ACTIVE` heißt: kein Nachlauf.
   *
   * **Gemessen in Kampfzeit** (`fightElapsedMs`), wie `decayExpiryMs`, aus dem
   * er stammt — nicht in der Zeit des Aufrufers.
   */
  private readonly decayEchoUntilMs: Float64Array;

  /**
   * Der gesammelte REST des Stapel-Aufbaus, in Promille und **je Quelle**
   * (#419).
   *
   * Was an einem ganzen Stapel fehlt, wartet hier auf den naechsten Wurf
   * derselben Quelle. Je Quelle, weil sonst ein Zauber den Rest eines anderen
   * ueber die Schwelle schoebe — dann entschiede die Reihenfolge im Kampf
   * ueber den Stapel, und die Rotation waere nicht mehr planbar.
   */
  private readonly decayRest: Float64Array;
  private readonly dotTicksLost: Int32Array;
  private readonly dotTickCounts: Int32Array;

  /**
   * Der Widerstands-Faktor **je Zauber** (GAME-DESIGN 11.4, Issue #328).
   *
   * # Am ZAUBER, nicht am Kampf — entschieden am 2026-08-05
   *
   * Bis dahin stand hier eine einzelne Zahl: `boss.resistances[school]` mit der
   * Schule des KAMPFES. Sie multiplizierte jeden Spielerschaden gleich, und
   * etwas, das alles gleich skaliert, kann keine Reihenfolge ändern. Gemessen:
   * Der Rotations-Läufer fand an einem Boss mit 0,75 und einem mit 1,25
   * **dieselbe** beste Liste, 23340 gegen 38900 — beides 31120 vor dem Faktor.
   *
   * Die Auflösung steht im SSOT, und sie ergibt nur so einen Sinn:
   *
   * > Wer **trotz Widerstand** Calcination spielt, sammelt weiter normal Hitze
   * > — macht damit aber weniger Schaden.
   *
   * „Trotz" setzt voraus, dass jemand mit einer anderen Schule **nicht**
   * weniger macht. Und Leitplanke 2 („jeder Zauber muss irgendwo die beste
   * Wahl sein") braucht einen Faktor, der die Zauber verschieden trifft.
   *
   * # Vorgerechnet, nicht im Takt gesucht
   *
   * Eine Float64Array über die Zauber-Indizes: Der Boss wechselt zwischen den
   * Kämpfen, nicht in ihnen. Eine Suche in `boss.resistances` je Wirkung wäre
   * eine Abbildung im Takt (GAME-DESIGN 18.2/18.4).
   *
   * **Schulunabhängige Zauber tragen 1,0.** QUENCH gehört keiner Schule an
   * (7.4), also kann keine Schule ihn abhalten — das ist dieselbe Trennung,
   * die ihn schon von der globalen Wartezeit ausnimmt.
   */
  private readonly resistanceOfSpell: Float64Array;
  /**
   * Die Schadensfaktoren dieses Kampfes.
   *
   * **Geschrieben wird ausschliesslich ueber `setDamageFactor`** (Issue #196).
   * Ein Prueffall liest diese Datei und schlaegt an, wenn irgendwo sonst
   * hineingeschrieben wird — der Grund steht bei `factorsOnCrit`.
   */
  private readonly factors: MutableFactors;
  /**
   * Ablaufzeiten der Zersetzungs-Stapel, **aufsteigend** — der älteste vorn.
   *
   * **Jeder Stapel hat seine eigene Uhr** (GAME-DESIGN 4.1). Ein gemeinsamer
   * Verfall war die verworfene Bauart: Er hätte jeden Zwischenfall im Kampf
   * den ganzen Aufbau gekostet.
   *
   * Ein Feld fester Länge statt einer wachsenden Liste: Der Kampf darf im
   * laufenden Bild nichts erzeugen (18.2).
   */
  // **Die KAPAZITAET, nicht die Grenze** (PUTRID BLOOM, #479). Das Feld ist
  // fest; wie viele Plaetze davon belegt werden duerfen, sagt `decayCap`.
  private readonly decayExpiryMs = new Float64Array(DECAY_STACK_CAPACITY);

  /** Wie viele Stapel gerade liegen. */
  private decayCountValue = 0;

  /** Die getragenen Sonderwerte (GAME-DESIGN 6.7). */
  private worn: WornValues;

  /**
   * Die Rotations-Effekte der sechsten Satz-Stufen (13.5, Issue #480).
   *
   * Kennungen, keine Zahlen — sie stehen nicht in `WornValues`, weil sie dort
   * ein Feld wären, das niemand liest. Der Kampf fragt sie über
   * `hasSetEffect` ab, wie ein Schlüsseltalent.
   */
  private setEffects: string[] = [];

  /**
   * Die laufenden Verstärkungen (GAME-DESIGN 5, Issue #420).
   *
   * **Vorab angelegt und nie ersetzt**, wie jede andere Mechanik hier: Ein
   * `new Buffs()` im laufenden Takt verstieße gegen 18.2. `reset()` leert das
   * Objekt, statt ein neues zu setzen.
   */
  private readonly buffs = new Buffs();
  /**
   * Verstärkungen auf andere Größen als Schaden (Issue #489).
   *
   * **Je Zielgröße eine eigene Instanz, nicht eine gemeinsame mit mehr
   * Arten.** GAME-DESIGN 328 sagt: gleiche Art ersetzt einander. Solange
   * alles auf den Schaden wirkte, war „gleiche Art" eindeutig; mit mehreren
   * Zielgrößen wäre es eine Verabredung („Arten 0–7 Schaden, 8–15 Punkte"),
   * die am Tag ihrer Einführung stimmt und beim dritten Zauber gebrochen ist,
   * ohne dass etwas rot wird.
   *
   * Getrennte Instanzen machen die Verwechslung **unmöglich** statt verboten:
   * Eine Verstärkung auf den Punktegewinn kann eine auf den Schaden gar nicht
   * verdrängen, weil sie in einem anderen Speicher liegt.
   */
  /**
   * Die zuletzt gesehene Uhrzeit — für die Verstärkungs-Faktoren (#489).
   *
   * **Warum ein mitgeführtes Feld und nicht ein Parameter.** Die vier
   * Zielgrößen werden an Stellen gelesen, die heute keine Zeit kennen:
   * `gainOfSpell` ist eine Abfrage über Zauber-Daten, `dotInterval` rechnet
   * einen Abstand, und `updateVulnerability` läuft aus mehreren Richtungen.
   * Sie alle um einen Parameter zu erweitern hieße, ihn durch ein Dutzend
   * Aufrufer zu fädeln — und jede vergessene Stelle wäre ein stiller
   * Rückfall auf den Faktor 1.
   *
   * Der Preis ist ein Feld, das in `reset()` mitmuss. Das fängt der Wächter
   * aus #349, der genau dafür gebaut ist.
   */
  private nowMsForBuffs = 0;
  private readonly controlBuffs = new Buffs();
  private readonly resourceBuffs = new Buffs();
  private readonly tickBuffs = new Buffs();
  private readonly stackBuffs = new Buffs();
  /** Verstärkt, was ein Zauber an Abklingzeit verkürzt (DISTIL, E10). */
  private readonly cdReduceBuffs = new Buffs();
  /** Verstärkt die Umwandlungsrate Schaden → Schild (QUICKSILVER, E10). */
  private readonly convertBuffs = new Buffs();
  /** Hebt die Obergrenze des Vorrats befristet an (SLAG, Issue #490). */
  private readonly capBuffs = new Buffs();
  /**
   * Die verteilten Talente dieses Kampfes (Issue #417).
   *
   * **Nur gelesen, nie geschrieben** — deshalb steht sie nicht in `reset()`:
   * Ein neuer Kampf bekommt eine neue Zuweisung über die Optionen, und
   * Umskillen zwischen Kämpfen ist damit von selbst vollständig. Sobald ein
   * Talent Kampfzustand anlegt (Issue #418), gilt das nicht mehr.
   */
  private talents: TalentAllocation;
  /**
   * Dieselben Faktoren, aber mit gesetztem Krit.
   *
   * **Vorab angelegt, weil im laufenden Takt nichts entstehen darf** (18.2).
   * Der naheliegende Weg wäre `{ ...this.factors, crit: … }` im Treffer
   * gewesen — ein Objekt je Treffer, bei einem zehnminütigen Kampf
   * zehntausende.
   *
   * SIE WIRD NACHGEZOGEN, NICHT EINGEFROREN (Issue #196)
   * ----------------------------------------------------
   * Die frühere Begründung lautete, die Vorab-Fassung sei möglich, „weil
   * `worn` sich während eines Kampfes nicht ändert". Das galt, solange
   * **alle** Faktoren bei Kampfbeginn feststanden. Sobald einer sich im Kampf
   * bewegt — Verwundbarkeits-Stapel, Zeit-begrenzte Wirkungen —, rechnete
   * jeder kritische Treffer mit dem Stand von Sekunde null, und zwar still:
   * Ein gewöhnlicher Treffer wäre richtig, ein kritischer falsch.
   *
   * Die Optimierung bleibt. Nachgezogen wird bei jeder **Änderung**, nicht bei
   * jedem Treffer — das sind ein paar Schreibvorgänge je Kampf statt
   * zehntausend Objekten.
   *
   * `crit` selbst steht hier und nur hier: Er ist abgeleitet, nicht gesetzt.
   */
  private factorsOnCrit: MutableFactors;

  private resourceValue = 0;
  private resourceOverflowValue = 0;
  private nextDecayAtMs: number;
  /** Welche Phase gerade läuft. 0 ist der Start (GAME-DESIGN 11.1). */
  private bossPhaseValue = 0;
  /** Stand der Energieleiste — der zweiten Uhr (GAME-DESIGN 11.2). */
  private bossEnergyValue = 0;
  /** Wann die Energieleiste das nächste Mal steigt. */
  private nextEnergyAtMs: number;
  /** Wie oft sie ausgelöst hat. */
  private bossEnergyTriggersValue = 0;

  /**
   * Wie weit die Boss-Zeitleiste abgearbeitet ist — Zeiger auf den **nächsten**
   * fälligen Eintrag (GAME-DESIGN 11.1).
   */
  private timelineIndexValue = 0;

  /**
   * Welcher Boss-Zauber gerade läuft, als Index in `boss.timeline`.
   *
   * `-1` heißt: keiner. Solange einer läuft, kann er unterbrochen werden —
   * danach nicht mehr, auch nicht im Takt des Treffers.
   */
  private castingValue = -1;

  /** Kampfzeit, zu der der laufende Boss-Zauber trifft. */
  private castLandsAtMsValue = -1;

  /**
   * Kampfzeit, zu der der laufende Boss-Zauber begonnen hat (Issue #327).
   *
   * **Nur für den Balken**, siehe `castingStartedAtMs`. Er steht hier, weil er
   * sich nach einer Verschiebung durch STALL nicht mehr aus dem Ende
   * zurückrechnen lässt.
   */
  private castingStartedAtMsValue = -1;

  /**
   * Welcher Zauber des SPIELERS gerade gewirkt wird, als Index in `spells`.
   *
   * `-1` heißt: keiner. **Bis Issue #304 gab es diesen Zustand nicht** — die
   * beiden Felder darüber gehören dem Boss, und `castTimeMs` am Spieler-Zauber
   * wurde geladen und nie gelesen (alle fünfzehn standen auf 0).
   *
   * Solange er gesetzt ist, blockiert er jede weitere Wirkung (GAME-DESIGN
   * 7.1). Er ist die einzige Stelle, an der eine Spieler-Handlung Zeit braucht;
   * alles andere geschieht im Takt des Tastendrucks.
   */
  private playerCastingValue: SpellIndex = NO_SPELL;

  /**
   * Uhrzeit des Spiels, zu der die laufende Wirkzeit endet.
   *
   * **Uhrzeit, nicht Kampfzeit** — anders als `castLandsAtMsValue`, das ab
   * Kampfbeginn zählt. Der Unterschied ist Absicht: Die Spieler-Wirkzeit wird
   * gegen `timeMs` verglichen, das durch `cast()` und `stepPassive` läuft, und
   * eine zweite Zeitrechnung an derselben Stelle wäre die nächste stille
   * Abweichung.
   */
  private playerCastEndsAtMsValue = -1;

  /**
   * Wann die nächste Teilwirkung einer Kanalisierung fällt; -1 heißt keine
   * (Issue #323).
   *
   * # Warum die Kanalisierung DIESELBEN Cast-Felder benutzt
   *
   * `playerCasting`, `playerCastEndsAtMs` und `playerCastStartedAtMs` tragen
   * sie mit. Das ist keine Sparsamkeit, sondern die Regel: „Jede Wirkung des
   * Spielers bricht einen laufenden Cast **oder eine Kanalisierung** ab"
   * (GAME-DESIGN 7.2). Läge sie in eigenen Feldern, bräuchte jeder Abbruchweg
   * eine zweite Zeile — und die eine, die man vergisst, wäre der Fehler.
   *
   * Der Unterschied steht allein in der AUFLÖSUNG: Ein Cast löst am Ende
   * einmal auf, eine Kanalisierung je Raster-Schritt ein Stück.
   *
   * # Warum der Abbruch keine anteilige Rechnung braucht
   *
   * Weil der Anteil schon ausgezahlt ist. Fällt die Wirkung wirklich laufend
   * an, ist „der erreichte Anteil bleibt" kein Sonderfall, sondern das, was
   * ohnehin passiert ist. Eine Rechnung „wie weit bin ich gekommen" beim
   * Abbruch wäre eine zweite Quelle für dieselbe Zahl.
   */
  private playerChannelNextTickAtMsValue = -1;

  /**
   * Uhrzeit, zu der die laufende Wirkzeit begonnen hat.
   *
   * **Nur für den Balken**, und deshalb bewusst NICHT in der Prüfsumme: Der
   * Wert ist vollständig aus `playerCastEndsAtMsValue` und der bereits
   * verrechneten Dauer bestimmt und beeinflusst keine Rechnung. Ein Feld, das
   * nichts entscheidet, macht die Prüfsumme nicht schärfer — es macht nur die
   * Feldfolge länger, die beide Seiten gleich halten müssen.
   */
  private playerCastStartedAtMsValue = -1;

  /**
   * Wie oft ein Boss-Angriff eine laufende Wirkzeit abgebrochen hat.
   *
   * Ein Zähler und keine Vermutung: „Der Zauber ist verloren" lässt sich sonst
   * nur daran ablesen, dass etwas NICHT geschehen ist — und das ist dieselbe
   * Aussage wie „der Spieler hat ihn nie gedrückt".
   */
  private castsInterruptedValue = 0;

  /**
   * Wie oft der Spieler eine eigene laufende Wirkzeit durch einen weiteren
   * Druck abgebrochen hat.
   *
   * **Getrennt vom Zähler des Boss-Abbruchs**, obwohl beide dasselbe Feld
   * räumen: Die zwei sind für den Spieler völlig verschiedene Dinge. Das eine
   * hat der Boss getan, das andere er selbst — und eine Fehleranalyse, die
   * beides zusammenzählte, könnte ihm nicht sagen, welches davon ein Fehler
   * war.
   */
  private castsCancelledValue = 0;

  /** Verbleibende Lebenspunkte des Spielers (Issue #110). */
  private playerHpValue: number;

  /** Stand des Zählers, der bei seiner Schwelle tötet (Issue #216). */
  private doomValue = 0;

  /** Kampfzeit des nächsten Zähler-Schritts. */
  private nextDoomAtMs = 0;
  /**
   * Wann die nächste verbrauchte Senkungs-Ladung zurückkommt (Issue #561).
   *
   * `0` heißt: keine wartet — der Vorrat ist voll. Der Wert wird beim
   * Verbrauch gesetzt, nicht beim Aufbau, damit ein Kampf ohne Senkung
   * denselben Verlauf hat wie vorher.
   */
  private nextDoomChargeAtMs = 0;

  /** Wie oft die Rücksetz-Handlung noch zur Verfügung steht. */
  private doomChargesValue = 0;

  /** Zahl der Stapel (Issue #216). */
  private stacksValue = 0;

  /** Kampfzeit, zu der das Unverwundbarkeits-Fenster endet, oder -1 (#218). */
  private raceEndsAtMs = -1;

  /** Ob das Fenster schon gelaufen ist — es beginnt genau einmal. */
  private raceDone = false;

  /** Schaden, der im Fenster verpuffte. Verloren, nicht ausgeteilt. */
  private damageLostToRaceValue = 0;

  /**
   * Wie weit das Ziel im laufenden Fenster geladen ist (Issue #218).
   *
   * Es ist die Energieleiste mit umgekehrter Fuellrichtung: Dort steigt sie
   * von selbst, hier fuellt der Spieler sie mit Schaden.
   */
  private raceProgressValue = 0;

  /** Ob das Ziel zerstoert wurde, statt dass die Zeit ablief. */
  private raceWonValue = false;

  /** Wie viel Fortschritt der Boss zurueckgedreht hat (Issue #218). */
  private bossHealedValue = 0;

  /** Ob die Schwaechung liegt (Baustein 8). */
  private afflictedValue = false;

  /** Kampfzeit des naechsten Schwaechungs-Ticks. */
  private nextAfflictionAtMs = 0;

  /** Wie oft sie entfernt wurde. */
  private afflictionsClearedValue = 0;

  /** Kampfzeit des nächsten Stapel-Schritts. */
  private nextStackAtMs = 0;
  /**
   * Wann die davonlaufende Reaktion das nächste Mal steigt (Issue #247).
   *
   * Im Konstruktor gesetzt, nicht erst in `beginFight` — die Wiedergabe ruft
   * es nicht. Dieselbe Begründung wie bei Energieleiste, Zähler und Stapel;
   * im verworfenen Anlauf hat genau das eine Vergleichsdatei aus lauter Nullen
   * erzeugt.
   */
  private nextRunawayAtMs = 0;
  /** Wie viel Fortschritt der Spieler insgesamt zurückgedrängt hat (#247). */
  private stabilisedValue = 0;

  /** Kampfzeit des Spielertods, oder -1. */
  private diedAtMsValue = -1;
  /**
   * **Der erste MERKER des Datenmodells** (SUBLIME, Issue #480).
   *
   * Steht auf `true`, wenn ein Zauber mit `freesNextGcd` geworfen wurde und
   * der nächste deshalb keine globale Wartezeit auslöst. Er lebt genau bis zu
   * diesem nächsten Wurf.
   *
   * # Warum er in der Prüfsumme steht
   *
   * Ein Kampfzustand, der nicht in der Prüfsumme steht, ist von der Parität
   * **nicht erfasst**: Beide Seiten könnten ihn verschieden führen — einer
   * verbraucht ihn beim nächsten Wurf, der andere beim übernächsten —, und
   * keine Prüfung würde es bemerken. Die Zahlen stimmten weiter, bis
   * irgendwann eine Rotation auseinanderliefe.
   *
   * Das gilt für jeden Merker, der noch kommt (CONDENSE, RETORT, PUTRID
   * BLOOM, DECAY). Wer einen baut, hängt ihn dort an.
   */
  private nextCastFreeGcdValue = false;

  /**
   * Der zweite Merker (CONDENSE, #480): Der nächste Wurf wirkt zweimal.
   *
   * Steht neben seinem Nachbarn darüber und nicht am Ende der Felder —
   * Merker sind eine eigene Sorte Zustand, und vier verstreute wären
   * schwerer zu pflegen als eine Gruppe.
   */
  private nextCastDoublesValue = false;

  /**
   * **Der dritte Merker** (RETORT, #480): Ist gerüstet.
   *
   * Steht auf `true`, wenn RETORT geworfen wurde und der nächste vom BOSS
   * abgebrochene Wurf deshalb keine Wartezeit kostet.
   *
   * **Vorbereitung statt Rettung** — bewusst anders als das Raster, das ihn
   * reaktiv beschreibt. Reaktiv gebaut bräuchte er eine zweite Ausnahme von
   * Design-Gesetz 6.6; gemessen wurde sein Druck dann abgelehnt. Die
   * ausführliche Begründung steht am Gegenstück in `sim/src/fight.rs`.
   *
   * Ein Ja/Nein, keine Zahl: Zweimal rüsten ist einmal gerüstet.
   */
  private retortArmedValue = false;

  /** Gesamter erlittener Schaden, **nach** WARD. */
  private damageTakenValue = 0;

  /** Wie oft der Spieler einen Boss-Zauber unterbrochen hat. */
  private interruptsLandedValue = 0;

  /** Wie oft einer durchgekommen ist. */
  private interruptsMissedValue = 0;

  // -- COAGULATION (Issue #79) --

  /** Stand der Schutzschicht: wie viel Schaden sie noch aufnehmen kann. */
  private shieldValue = 0;

  /** Wann sie verfällt, wenn sie nicht vorher aufgebraucht wird. -1 = keine. */
  private shieldExpiresAtMs = -1;
  /**
   * Wie dick die zuletzt gelegte Schicht war — für COAGULATIONs
   * Schlüsseltalent (#419).
   *
   * Ohne diesen Merker wüsste der Bruch nicht, wovon er die Hälfte nachlegen
   * soll: `shieldValue` ist in dem Moment gerade 0.
   */
  private shieldLastAmount = 0;
  /**
   * Ob dieser Schild seine Nachlage schon bekommen hat.
   *
   * **Die Begrenzung ist der Kern des Talents.** Ohne sie legte jeder Bruch
   * nach, der nächste wieder, und die Schicht wäre nicht mehr aufzubrauchen.
   */
  private shieldRefundUsed = false;

  /**
   * Heilung über Zeit (POULTICE, Issue #481) — Gesamtbetrag, Anzahl der
   * Schritte, wie viele davon getan sind, nächster Takt.
   *
   * Vier Felder statt eines Objekts: Im Takt entsteht damit nichts (GAME-DESIGN
   * 18.2), und der Zustand ist an derselben Stelle sichtbar wie die
   * Schutzschicht darüber.
   *
   * **Warum ein Zähler statt eines Betrags je Takt.** Der erste Entwurf trug
   * `perTick = amount / schritte` — bei POULTICE (1800 über 6000 ms) ergibt das
   * glatte 30, und der Fall war grün. Beim nächsten Zauber mit einem nicht
   * teilbaren Betrag wäre daraus eine Fließkommazahl geworden, die beide Seiten
   * unabhängig hätten runden müssen: genau der Fall, den GAME-DESIGN 18.4
   * ausschließt. Kumulativ gerechnet — `amount * getan / schritte`, ganzzahlig
   * geteilt, minus dem schon Gutgeschriebenen — bleibt die Summe exakt `amount`
   * und jeder einzelne Schritt ganzzahlig, für jeden Betrag und jede Dauer.
   */
  private healOverTimeAmount = 0;
  private healOverTimeSteps = 0;
  private healOverTimeIntervalMs = 0;
  private healOverTimeDone = 0;
  private healOverTimeCredited = 0;
  private healOverTimeNextAtMs = 0;

  /** Wie viele Anwendungen gerade übereinanderliegen. */
  private shieldStacksValue = 0;

  /** Wie viel Schaden die Schutzschicht insgesamt abgefangen hat. */
  private shieldAbsorbedValue = 0;

  /** Wie viel Leben insgesamt zurückgeholt wurde. */
  private healedValue = 0;

  /** Bis wann die Schaden-in-Schutz-Umwandlung läuft. -1 = keine. */
  private conversionUntilMs = -1;

  /** Anteil, den sie umwandelt, in Prozent. 0 heißt: keine läuft. */
  private conversionPercentValue = 0;

  /**
   * Um wie viele Plätze die Stapel-Grenze angehoben ist (PUTRID BLOOM, #479).
   *
   * `0` heißt: die Grenze ist `MAX_DECAY_STACKS`. Der Zuschlag gilt nur,
   * solange `decayCapUntilMs` nicht abgelaufen ist — dieselbe Bauart wie die
   * Umwandlung von TEMPER eine Zeile höher.
   */
  private decayCapBonusValue = 0;

  /** Bis wann der Zuschlag gilt. -1 = keiner. */
  private decayCapUntilMs = -1;

  /**
   * Die Grenze, die JETZT gilt.
   *
   * **Gedeckelt auf die Kapazität**, und das ist keine Vorsicht, sondern
   * Notwendigkeit: Darüber hinaus gäbe es keinen Platz im Feld. Der Parser
   * weist solche Daten schon zurück; diese Zeile ist die zweite Sperre für den
   * Fall, dass der Zuschlag später aus einer anderen Quelle kommt.
   */
  private get decayCap(): number {
    const offen = this.decayCapUntilMs >= 0 && this.nowMsForBuffs <= this.decayCapUntilMs;
    return Math.min(DECAY_STACK_CAPACITY, MAX_DECAY_STACKS + (offen ? this.decayCapBonusValue : 0));
  }

  /**
   * Bis wann DECAYs Schwächung läuft. -1 = keine (Issue #479).
   *
   * **Dieselbe Bauart wie `conversionUntilMs`** (TEMPER): ein Zustand für eine
   * Weile, gemessen in Kampfzeit. Er steht in keiner Prüfsumme und muss es
   * nicht — seine Wirkung zeigt sich in `playerHp`, und das mischt
   * (`checksum.ts`). Genau so trägt `conversionUntilMs` seit #79.
   */
  private decayWardUntilMs = -1;

  /** Promille je Stapel, solange die Schwächung läuft. 0 heißt: keine. */
  private decayWardPermilleValue = 0;

  /** Wie viel Schutz die Umwandlung insgesamt erzeugt hat. */
  private convertedValue = 0;

  /**
   * Der Rückschlag (RIPOSTE, Issue #481) — bis wann er läuft, sein Anteil,
   * und was er insgesamt zurückgegeben hat.
   *
   * Getrennt von der Umwandlung, obwohl beide zu Weg 3 gehören: Sie hängen an
   * entgegengesetzten Enden (ausgeteilter Schaden gegen eingehenden), und ein
   * gemeinsamer Zustand hieße, dass ein Zauber nicht nur einen von beiden
   * anschalten könnte.
   */
  private reflectUntilMs = -1;
  private reflectPercentValue = 0;
  private reflectedValue = 0;
  /**
   * Welchem Zauber der zurückgegebene Schaden gehört.
   *
   * **Er braucht einen Träger, keinen Platzhalter.** Der erste Entwurf buchte
   * ihn auf `-1`; im Simulator hätte dieselbe Zeile `damage_by_spell[index]`
   * über die Array-Grenze geschickt, und hier wäre er in `damageOf` gar nicht
   * aufgetaucht. Ein Schaden ohne Urheber ist in der Fehleranalyse (GAME-DESIGN
   * 15.3) nicht wiederzufinden.
   */
  private reflectSpellIndex = alsSpellIndex(0);

  // -- FIXATION (Issue #100) --

  /**
   * Vorrat an Kontroll-Punkten.
   *
   * **Zum Ausgeben, nicht zum Sammeln** (GAME-DESIGN 4.1): Sie verfallen nicht
   * wie die Hitze, sondern warten darauf, dass der Spieler sich entscheidet.
   */
  private controlValue = 0;
  /**
   * Schaden aus umgewandelten Kontroll-Punkten, gemerkt bis zur Rechnung
   * (Issue #478, WEDGE).
   *
   * **Warum gemerkt und nicht sofort ausgeteilt:** `applyFixation` läuft vor
   * der Schadensrechnung, und dort stehen die Faktoren noch nicht. Ein Treffer
   * an dieser Stelle ginge an Krit, Widerstand und Verstärkung vorbei — er
   * wäre der einzige Schaden im Spiel, den kein Faktor berührt.
   */
  private pendingConvertedDamage = 0;

  /** Wie viele Punkte insgesamt ausgegeben wurden. */
  private controlSpentValue = 0;

  /**
   * Steht ein kostenloser Kontroll-Ausgeber bereit? (THE LONG GRIP, 13.5)
   *
   * Wird bei jedem Phasenwechsel gesetzt und vom ersten Ausgeber danach
   * verbraucht. **Ein Ja/Nein, keine Zahl**: Zwei Wechsel ohne Ausgeber
   * dazwischen sind ein freier Ausgeber, nicht zwei.
   */
  private freeControlSpendArmed = false;

  /** Wie oft der Effekt gegriffen hat — für Anzeige und Prüfung. */
  private freeControlSpends = 0;

  /**
   * Wie oft eine Wirkung mangels Punkten ins Leere lief.
   *
   * **Kein Fehler, sondern eine vertane Wirkung.** Reichen die Punkte nicht,
   * geschieht nichts — der Zauber nimmt sich nicht, was er nicht hat.
   */
  private controlFailedValue = 0;

  /**
   * Um wie viel der **nächste** Zeitleisten-Eintrag nach hinten geschoben ist.
   *
   * Nur der eine, nicht die folgenden — Entscheidung des Auftraggebers vom
   * 2026-08-04. Verzögern verschafft Luft und **erkauft sie mit Gedränge
   * danach**.
   */
  private nextStrikeDelayMs = 0;

  /**
   * Ob der **nächste** Boss-Zauber unterbrechbar wird, auch wenn er es von
   * Haus aus nicht ist (PIN, Issue #478).
   *
   * # Warum „der nächste" wörtlich zu nehmen ist
   *
   * Gelesen wird der Merker in `stepTimeline`, im Moment, in dem ein Eintrag
   * **auslöst** — nicht während einer laufenden Wirkzeit. Wer ihn eine Zeile
   * früher läse, machte den GERADE LAUFENDEN Zauber unterbrechbar; PIN
   * befreite dann sich selbst statt den folgenden, und der Fall bliebe grün,
   * weil ja etwas unterbrechbar wurde. Worker-2 hat genau diese Falle bei
   * seinem Merker getroffen.
   *
   * **Dieselbe Bauart wie `nextStrikeDelayMs`** (STAY, Issue #100): ein Merker
   * auf dem nächsten Zeitleisten-Eintrag, beim Auslösen verbraucht.
   *
   * # Warum eine ZEIT und kein Ja/Nein
   *
   * Ein `strike` trägt `castTimeMs: 0` — der Parser verlangt für
   * `interruptibleCast` ausdrücklich mehr als null (`data.ts`). Ein Schlag,
   * den PIN nur umetikettierte, wäre im selben Augenblick vorbei, in dem er
   * beginnt: unterbrechbar auf dem Papier, nie unterbrechbar in der Hand.
   * Also gibt PIN ihm ein Fenster, und dessen Länge steht in den DATEN.
   *
   * `0` heißt „kein Merker".
   */
  private pinWindowMsValue = 0;

  /** Um wie viele Prozentpunkte die Phasen-Schwelle gesenkt ist (ANCHOR). */
  private phaseHoldPercentValue = 0;

  /**
   * Bis wann die Senkung gilt (GAME-DESIGN 4.2b). -1 = keine.
   *
   * **„Hinauszögern" heißt befristet, nicht dauerhaft.** Sie endet, was zuerst
   * eintritt: Die Frist läuft ab, oder der nächste Wechsel hat stattgefunden —
   * dann ist sie verbraucht.
   */
  private phaseHoldUntilMs = -1;
  /** Die Stufe des Spielers; ohne Angabe 1 (Issue #289). */
  private levelValue: number;
  /**
   * Die Lebenspunkte dieser Stufe — der Deckel fürs Heilen.
   *
   * **Nicht `PLAYER.vessel`.** Der ist der Grundwert aus `base.json`; was
   * dieser Spieler hat, sagt `statAtLevel`.
   */
  private vesselMax: number;
  private damageDealtValue = 0;
  /**
   * Wie viel Ring der SPIELER heruntergedrückt hat (Issue #256, AC3).
   *
   * **Ein zweiter Zähler, weil einer die Herkunft nicht mehr hergibt.** Seit
   * #247 senkt die davonlaufende Reaktion den Ring von selbst, und beides
   * landete in `damageDealtValue`. Gemessen am 2026-08-05 gegen
   * `test-runaway`:
   *
   * | Lauf | damageDealt | davon Reaktion | davon Spieler |
   * | --- | --- | --- | --- |
   * | nichts gedrückt | 1020 | 1020 | 0 |
   * | KINDLE auf Cooldown | 1020 | 420 | 600 |
   *
   * Ohne die Trennung meldete die Fehleranalyse dem Nichtstuer 1020 Punkte
   * als seinen Fehler — dieselbe verkehrte Aussage, die AC1 eine Ebene höher
   * gerade repariert hat.
   *
   * **Nur was ankommt.** Schaden, den das Unverwundbarkeits-Fenster schluckt,
   * senkt den Ring nicht und steht deshalb nicht hier; er hat mit
   * `damageLostToRaceValue` seine eigene Zahl.
   *
   * **Ein Stabilisierer senkt ihn nicht.** `undoProgress` drängt den Ring
   * zurück, aber wer 600 Punkte angerichtet und 300 repariert hat, hat 600
   * angerichtet — die Reparatur ist eine eigene Leistung und gehört in eine
   * eigene Zeile, nicht in eine Verrechnung, die beides unsichtbar macht.
   */
  private playerDamageValue = 0;
  private stateValue = FIGHT_RUNNING;
  private endedAtMs = NOT_ACTIVE;
  private fightStartedAtMs = 0;
  /**
   * Ob `beginFight` gelaufen ist (Issue #433).
   *
   * **Warum das nicht aus `fightStartedAtMs` ablesbar ist.** Dort steht die
   * Startzeit, und `0` bedeutet zwei verschiedene Dinge: „noch nicht begonnen"
   * und „bei Zeit 0 begonnen". Die Sperren gegen Ändern im Kampf lasen
   * `fightStartedAtMs > 0` und ließen deshalb jeden Kampf durch, der bei 0
   * anfängt.
   *
   * Im Spiel läuft vorher ein Countdown, also ist die Startzeit dort nie 0. Im
   * Prüfstand ist 0 der naheliegende Wert — die Sperre war also genau dort
   * offen, wo sie am ehesten gebraucht wird.
   *
   * `stateValue` allein reicht nicht: Es steht von Anfang an auf
   * `FIGHT_RUNNING`, auch vor dem ersten Takt.
   */
  private fightHasBegun = false;

  constructor(options: CombatOptions) {
    this.school = options.school;
    this.boss = options.boss;
    this.spells = options.school.spells;
    this.random = new RandomStreams(options.seed ?? 0);

    const count = this.spells.length;
    this.cooldownReadyAtMs = new Float64Array(count);
    this.castCounts = new Int32Array(count);
    this.synergyWindowEndsMs = new Float64Array(count).fill(NOT_ACTIVE);
    this.damageBySpell = new Float64Array(count);
    this.dotExpiresAtMs = new Float64Array(count).fill(NOT_ACTIVE);
    this.dotNextTickAtMs = new Float64Array(count).fill(NOT_ACTIVE);
    this.decayEchoUntilMs = new Float64Array(count).fill(NOT_ACTIVE);
    this.decayRest = new Float64Array(count);
    this.dotTicksLost = new Int32Array(count);
    this.dotTickCounts = new Int32Array(count);
    for (let i = 0; i < count; i++) this.indexById.set((this.spells[i] as SpellDef).id, i);

    this.nextDecayAtMs = options.school.resource.decayPerStepMs;
    // Genauso wie der Verfall im Konstruktor: Der Replay ruft `beginFight`
    // nicht, und ein Startwert von 0 ließe die Leiste schon im Takt 0 steigen —
    // der Simulator plant den ersten Anstieg bei `perStepMs`. Genau so eine
    // Kleinigkeit lässt beide Seiten auseinanderlaufen.
    this.nextEnergyAtMs = options.boss.energy?.perStepMs ?? 0;
    // Dieselbe Begründung wie eine Zeile darüber: Der Replay ruft `beginFight`
    // nicht, und ein Startwert von 0 ließe beide schon im Takt 0 steigen.
    this.nextDoomAtMs = options.boss.doom?.perStepMs ?? 0;
    this.nextStackAtMs = options.boss.stacks?.perStepMs ?? 0;
    this.nextRunawayAtMs = options.boss.runaway?.perStepMs ?? 0;
    this.doomChargesValue = options.boss.doom?.clearCharges ?? 0;
    // **Aus der Stufe, nicht aus der Datei** (Issue #289). `statAtLevel` gibt
    // auf Stufe 1 exakt den Grundwert zurück — dort bewegt sich nichts.
    this.levelValue = options.level ?? 1;
    // **Ausrüstung kommt DAZU, nicht davor** (Issue #282, AC3). Wer die
    // Stufen-Basis ersetzt statt sie zu erhöhen, macht einen Spieler auf
    // Stufe 100 ohne Ausrüstung schwächer als einen auf Stufe 1 mit einem
    // Fund. Ohne getragene Ausrüstung bewegt die Rechnung keine Zahl.
    this.vesselMax = vesselWithGear(
      statAtLevel(PLAYER.vessel, this.levelValue),
      (options.worn ?? NOTHING_WORN).vessel,
    );
    this.playerHpValue = this.vesselMax;
    this.resistanceOfSpell = new Float64Array(this.spells.length);
    for (let i = 0; i < this.spells.length; i++) {
      const schule = this.spells[i]?.school;
      this.resistanceOfSpell[i] =
        schule === null || schule === undefined ? 1 : (options.boss.resistances[schule] ?? 1);
    }
    this.worn = options.worn ?? NOTHING_WORN;
    this.talents = options.talents ?? NO_TALENTS;
    // **Topf 4 kommt beim Aufbau in die Faktoren**, nicht im Takt: Ausrüstung
    // wird zwischen den Kämpfen gewechselt (13.1). Ohne getragene Ausrüstung
    // ist der Faktor exakt 1,0 — die Multiplikation bewegt dann keine Zahl.
    // **Der Widerstand steht hier NICHT mehr** (Issue #328). Er hängt am
    // Zauber und wird deshalb vor jeder Wirkung gesetzt, nicht einmal beim
    // Aufbau. Was hier bleibt, ist alles, was für den ganzen Kampf gilt.
    this.factors = {
      ...NEUTRAL_FACTORS,
      // **Topf 1 ebenso**, und als EIGENER Faktor: Zwischen den Toepfen wird
      // multipliziert (13.1b). Waeren ESSENCE und Topf 4 derselbe Faktor,
      // waere die Rechnung additiv und die Abwaegung aus 13.5 gekippt.
      //
      // `resistance` stand hier bis #328 daneben und ist dort weggefallen —
      // er haengt am Zauber, nicht am Kampf. ESSENCE bleibt, weil Ausruestung
      // fuer den ganzen Kampf gilt.
      essence: NEUTRAL_FACTORS.essence * essenceFactor(this.worn),
      gearEffects: gearEffectFactor(this.worn),
      // **Topf 2, und ausdrücklich nicht bei ESSENCE** (Issue #417): 13.1b
      // trennt die Töpfe — innerhalb additiv, zwischen ihnen multiplikativ.
      // Talente dort einzurechnen wäre eine gesparte Zeile und machte aus zwei
      // Töpfen einen. Die Gegenprobe dazu steht in `talents.test.ts`.
      talents: talentFactor(this.talents),
    };
    this.factorsOnCrit = { ...this.factors, crit: critFactor(this.worn) };

    // Die Schul-Mechanik hört ab hier auf Ereignisse (E3-2). Eine Zeile für
    // alle Schulen: Welcher Vorrat gefüllt wird und wie viel, steht in den
    // Daten des jeweiligen Zaubers.
    this.stacks = registerStackMechanic(this);
    this.sublimation = registerSublimation(this);
  }

  /** Die Stapel-Mechanik dieser Schule. Öffentlich für Prüfung und Werkzeuge. */
  readonly stacks: StackMechanic;

  /** Die Verkürzung der Abklingzeiten (E3-3). */
  readonly sublimation: SublimationMechanic;

  // ---------------------------------------------------------------- Abfragen

  indexOf(spellId: string): SpellIndex {
    // **Das `-1` ist die einzige rohe Zahl, die hier entstehen darf** — und sie
    // bedeutet „gibt es nicht". Sie durch einen Wurf zu ersetzen wäre eine
    // Verhaltensänderung; heute verlassen sich Aufrufer darauf (Issue #372).
    return alsSpellIndex(this.indexById.get(spellId) ?? -1);
  }

  spellAt(index: SpellIndex): SpellDef | undefined {
    return this.spells[index];
  }

  get damageDealt(): number {
    return this.damageDealtValue;
  }

  /**
   * Der Anteil am Ring-Verlust, der vom Spieler stammt (Issue #256).
   *
   * Bei einem gewöhnlichen Boss ist das der Fortschritt selbst; bei einem mit
   * `runaway` ist es der Teil des Verlusts, den er sich selbst zugefügt hat.
   */
  get playerDamage(): number {
    return this.playerDamageValue;
  }

  get bossHpRemaining(): number {
    const left = this.boss.hp - this.damageDealtValue;
    return left > 0 ? left : 0;
  }

  /** Anteil der verbleibenden Lebenspunkte, 0 bis 1. Für den Reaktionsgefäß-Ring. */
  get bossHpFraction(): number {
    return this.bossHpRemaining / this.boss.hp;
  }

  /** Der Vorrat der Schul-Mechanik — in Schritt 1 die Hitze. */
  get resource(): number {
    return this.resourceValue;
  }

  /** Wie viel Vorrat an der Obergrenze verpufft ist. */
  /**
   * Welche Phase gerade läuft. 0 ist der Start.
   *
   * Geht in die Zustands-Prüfsumme ein (Issue #96): Ohne sie wären zwei Läufe
   * in verschiedenen Phasen nicht zu unterscheiden.
   */
  get bossPhase(): number {
    return this.bossPhaseValue;
  }

  /** Stand der Energieleiste. Geht ebenfalls in die Prüfsumme ein. */
  get bossEnergy(): number {
    return this.bossEnergyValue;
  }

  /**
   * Die acht Schadensfaktoren, wie sie gerade stehen (Issue #425).
   *
   * # Wozu ein Lesezugriff, den das Spiel nicht braucht
   *
   * Damit ein Prüffall je Topf messen kann, ob ein Aufbau ihn wirklich füllt.
   * Über den Schaden geht das nicht: Der sagt, DASS etwas fehlt, nie WELCHES —
   * und genau diese Unterscheidung ist der Zweck von #425.
   *
   * **Für einen EINZELNEN Faktor bleibt der Schaden der bessere Nachweis**
   * (so gebaut in #417 und #433): Er belegt, dass der Wert ankommt, nicht nur
   * dass er berechnet wurde. Dieser Zugriff beantwortet die andere Frage —
   * „welcher Eingang wurde nie gefüllt" — und sollte nicht zum allgemeinen Weg
   * werden, Faktoren zu prüfen.
   *
   * # Warum `DamageFactors` und nicht der interne Typ
   *
   * `this.factors` ist `MutableFactors` — dieselbe Form mit abgestreiften
   * `readonly`-Markierungen, damit `setDamageFactor` schreiben kann. Gäbe
   * dieser Zugriff sie unverändert heraus, könnte jeder Aufrufer von außen
   * hineinschreiben (`combat.damageFactors.essence = 99`) — ein Loch in der
   * Determinismus-Zusicherung, aufgerissen ausgerechnet von dem Zugriff, der
   * sie prüfen soll.
   *
   * **Der Typ, nicht eine Kopie.** Eine Kopie schützte auch gegen Absicht,
   * kostete aber ein Objekt je Aufruf, und GAME-DESIGN 18.2 verbietet
   * Erzeugung im laufenden Bild. Wer die Markierung mit `as` umgeht, tut das
   * sichtbar — derselbe Schutzgrad wie bei `SpellIndex`.
   */
  get damageFactors(): DamageFactors {
    return this.factors;
  }

  /** Wie oft die Energieleiste ausgelöst hat. */
  /** PRUEF-ZUGANG, erscheint nicht (Issue #294): Boss-Innenleben. */
  get bossEnergyTriggers(): number {
    return this.bossEnergyTriggersValue;
  }

  /**
   * Welcher Eintrag beim Fortschrittsstand `index` ansteht.
   *
   * **Der Zähler ist Fortschritt, nicht Platz in der Liste.** Wiederholt sich
   * die Zeitleiste (`timelineLoopMs`, Issue #174), läuft er darüber hinaus:
   * Eintrag `i` im Zyklus `z` ist fällig bei `z * loop + atMs`.
   *
   * **Bei leerer Liste immer `undefined`.** Das ist der Fall der stummen
   * Puppe, an der maximale Rotationen gemessen werden: Eine Wiederholung über
   * nichts darf nicht kreisen, nichts auslösen und nichts kosten.
   *
   * Wortgleich mit `Fight::timeline_at` im Simulator — eine Regel, die nur
   * eine Seite kennt, bricht die Parität erst nach dem ersten Zyklus.
   */
  private timelineEntryAt(index: number): TimelineEntryDef | undefined {
    const laenge = this.boss.timeline.length;
    if (laenge === 0) return undefined;
    return this.boss.timelineLoopMs === null
      ? this.boss.timeline[index]
      : this.boss.timeline[index % laenge];
  }

  /**
   * Wann der Eintrag beim Stand `index` fällig ist. `-1`, wenn keiner ansteht.
   *
   * **Zwei Methoden mit einfachen Rückgabewerten statt einer mit einem
   * Objekt.** Der erste Entwurf gab `{ eintrag, atMs }` zurück und erzeugte
   * damit bei jedem Aufruf ein Objekt — im laufenden Kampf, was GAME-DESIGN
   * 18.2 ausdrücklich verbietet. Gefunden beim Bauen von AC4.
   */
  private timelineDueAt(index: number): number {
    const laenge = this.boss.timeline.length;
    if (laenge === 0) return -1;
    const loop = this.boss.timelineLoopMs;
    if (loop === null) {
      const eintrag = this.boss.timeline[index];
      return eintrag === undefined ? -1 : eintrag.atMs;
    }
    return Math.floor(index / laenge) * loop + this.boss.timeline[index % laenge]!.atMs;
  }

  /** Wie weit die Boss-Zeitleiste abgearbeitet ist. */
  get timelineIndex(): number {
    return this.timelineIndexValue;
  }

  /** Verbleibende Lebenspunkte des Spielers. */
  /** Die Stufe, mit der dieser Kampf rechnet (Issue #289). */
  get level(): number {
    return this.levelValue;
  }

  /**
   * Die Stufe setzen — **zwischen den Kämpfen**, nicht im Takt (Issue #289).
   *
   * Der Kampf entsteht einmal beim Start des Spiels, die Stufe ändert sich
   * später. Ohne diesen Weg rechnete jeder Kampf mit der Stufe, die beim
   * Programmstart galt — und der Aufstieg wäre erst nach einem Neuladen
   * spürbar.
   *
   * **Die Lebenspunkte werden mitgezogen, aber nur außerhalb eines Kampfes.**
   * Mitten im Kampf die Obergrenze zu heben hieße, dem Spieler Punkte zu
   * schenken, die er sich nicht erkämpft hat.
   */
  /**
   * Die Talent-Verteilung ändern (Issue #417).
   *
   * **Nur zwischen Kämpfen**, wie GAME-DESIGN 12.4 es verlangt: „Umskillen
   * jederzeit kostenlos **zwischen den Kämpfen**". Mitten im Kampf zu wechseln
   * hiesse, den Schaden rückwirkend anders zu rechnen als die bereits
   * gefallenen Treffer — und die Prüfsumme wüsste nichts davon.
   *
   * Die Faktoren werden dabei neu gebildet, nicht nachgerechnet: Es ist
   * dieselbe Zeile wie beim Aufbau, damit es keine zweite Rechenstelle gibt.
   */
  /**
   * Die getragenen Werte ändern (Issue #421a).
   *
   * **Nur zwischen Kämpfen**, wie beim Umskillen: Mitten im Kampf zu wechseln
   * hiesse, den Schaden rückwirkend anders zu rechnen als die bereits
   * gefallenen Treffer.
   *
   * `vesselMax` zieht mit, weil VESSEL die Lebenspunkte hebt — und wie bei
   * `setPlayerLevel` nur ausserhalb des Kampfes: Mitten drin die Obergrenze zu
   * heben hiesse, Punkte zu schenken.
   */
  setWorn(worn: WornValues): void {
    // **Vollsperre: im Kampf passiert hier gar nichts.** Getragene Werte
    // ändern `essence` und `gearEffects`, und die gehen in jeden Treffer ein.
    // Mitten im Kampf gesetzt, wären bereits gefallene Treffer anders
    // gerechnet als die folgenden — derselbe Lauf mit zwei Rechnungen.
    if (this.stateValue === FIGHT_RUNNING && this.fightHasBegun) return;
    this.worn = worn;
    this.setDamageFactor('essence', NEUTRAL_FACTORS.essence * essenceFactor(worn));
    this.setDamageFactor('gearEffects', gearEffectFactor(worn));
    // **Die Krit-Fassung als Ganzes neu, nicht Feld für Feld** — und das ist
    // keine Stilfrage: `crit` ist in `setDamageFactor` ausdrücklich
    // ausgenommen, weil es nur in dieser Fassung existiert. Ein einzelnes Feld
    // hier zu beschreiben wäre eine dritte Schreibstelle, und der Wächter aus
    // #196 hat genau das gefangen — zum zweiten Mal an diesem Abend, nach #417.
    //
    // Dieselbe Form wie beim Aufbau. Sie ist hier zulässig, weil zwischen den
    // Kämpfen kein laufzeitgesetzter Faktor offen steht.
    this.factorsOnCrit = { ...this.factors, crit: critFactor(worn) };
    const vorher = this.vesselMax;
    this.vesselMax = vesselWithGear(statAtLevel(PLAYER.vessel, this.levelValue), worn.vessel);
    if (this.playerHpValue >= vorher || this.playerHpValue === 0) {
      this.playerHpValue = this.vesselMax;
    }
  }

  /** Die getragenen Werte — für Anzeige und Prüfung. */
  get wornValues(): WornValues {
    return this.worn;
  }

  /**
   * Ob ein Schlüsseltalent dieses Verhalten freigeschaltet hat (Issue #418).
   *
   * **Aus der Verteilung abgeleitet, nicht mitgeführt.** Ein zweites Feld
   * `private splitSpender = false` müsste in `setTalents` und in `reset`
   * nachgezogen werden — zwei Stellen, an denen es auseinanderlaufen kann.
   * Die Ableitung kann das nicht: Sie liest immer den Stand, der gilt.
   */
  hasKeystone(keystone: string): boolean {
    return hasKeystone(this.talents, keystone);
  }

  /**
   * Ob ein **Rotations-Effekt** eines Satzes wirkt (13.5, Issue #480).
   *
   * Die sechste Satz-Stufe trägt keinen Werte-Bonus, sondern eine Kennung, auf
   * die der Kampf reagiert — dieselbe Bauart wie ein Schlüsseltalent, und
   * deshalb dieselbe Abfrage daneben.
   *
   * **Nicht abgeleitet, sondern übergeben** — anders als `hasKeystone`. Der
   * Grund: Die Talente stehen im Kampf, der Besitz nicht. `rotationEffectsFrom`
   * rechnet ihn aus den getragenen Stücken, und das Ergebnis kommt herein wie
   * `worn`. Zwei Wege in den Kampf, weil es zwei Quellen sind.
   */
  hasSetEffect(effect: string): boolean {
    return this.setEffects.includes(effect);
  }

  /**
   * Die Rotations-Effekte setzen — wie `setWorn`, mit derselben Sperre.
   *
   * Ein Effekt, der mitten im Kampf dazukommt, teilte den Lauf in zwei
   * Rechnungen; die Prüfsumme beschriebe danach keinen der beiden.
   */
  setSetEffects(effects: readonly string[]): void {
    if (this.stateValue === FIGHT_RUNNING && this.fightHasBegun) return;
    this.setEffects = [...effects];
    // **THE WHOLE WORK** (13.5, Sechser ohne Schulbindung): Die Wartezeit
    // sinkt um 100 ms.
    //
    // **Hier abgeleitet und nicht an der Auslösestelle gerechnet.** Die
    // Wartezeit wird an drei Stellen ausgelöst (`gcd.trigger`); ein Abzug an
    // jeder davon wären drei Stellen, an denen er fehlen kann. `setLength`
    // gibt es seit #177 genau dafür — sie hatte bis hierher **keinen
    // Aufrufer ausserhalb der Prüffälle**, dieselbe Klasse wie
    // `activeRotationEffects` vor #774.
    //
    // Die Länge steht damit an EINER Stelle, und `buffs.ts` rechnet
    // Verstärkungs-Dauern in GCDs bereits gegen sie — der Effekt schlägt dort
    // also mit durch, was gewollt ist und in `whole-work.test.ts` gemessen
    // wird.
    this.gcd.setLength(
      this.setEffects.includes(SET_EFFECT_WHOLE_WORK)
        ? GCD_MS - WHOLE_WORK_GCD_REDUCTION_MS
        : GCD_MS,
    );
  }

  /** Die aktiven Rotations-Effekte — für Anzeige und Prüfung. */
  get activeSetEffects(): readonly string[] {
    return this.setEffects;
  }

  setTalents(allocation: TalentAllocation): void {
    // **Vollsperre, aus demselben Grund wie bei `setWorn`:** Talente setzen
    // einen Schadensfaktor, und ein Faktor, der sich mitten im Kampf ändert,
    // teilt den Lauf in zwei Rechnungen.
    if (this.stateValue === FIGHT_RUNNING && this.fightHasBegun) return;
    this.talents = allocation;
    // **Über `setDamageFactor`, nicht über eine Zuweisung** (Issue #196): Nur
    // so zieht die Krit-Fassung mit. Meine erste Fassung schrieb beide Felder
    // von Hand — der Quelltext-Wächter aus #196 hat sie gefangen, und genau
    // dafür wurde er gebaut. Die Auflage in #249 nennt denselben Fall: Der
    // gewöhnliche Treffer bliebe richtig, nur der kritische rechnete falsch.
    this.setDamageFactor('talents', talentFactor(this.talents));
  }

  /** Die aktuelle Verteilung — für Anzeige und Prüfung. */
  get talentAllocation(): TalentAllocation {
    return this.talents;
  }

  setPlayerLevel(level: number): void {
    const stufe = Math.max(1, Math.trunc(level));
    if (stufe === this.levelValue) return;
    this.levelValue = stufe;
    const vorher = this.vesselMax;
    this.vesselMax = statAtLevel(PLAYER.vessel, stufe);
    // **Teilsperre, und das ist Absicht.** Stufe und `vesselMax` sind oben
    // schon gesetzt — sie beschreiben den Spieler und dürfen sich auch im
    // Kampf ändern. Nur das Auffüllen unterbleibt: Mitten im Kampf die
    // Obergrenze zu heben UND die Punkte mitzugeben hieße, dem Spieler Leben
    // zu schenken, das er sich nicht erkämpft hat.
    //
    // Die Zeile sieht aus wie die beiden Vollsperren und wirkt anders, weil
    // sie woanders steht. Wer sie nach oben zöge, machte daraus still eine
    // Vollsperre.
    if (this.stateValue === FIGHT_RUNNING && this.fightHasBegun) return;
    // Außerhalb des Kampfes: voll, wie beim Aufbau.
    if (this.playerHpValue >= vorher || this.playerHpValue === 0) {
      this.playerHpValue = this.vesselMax;
    }
  }

  /**
   * Der Anteil, den WARD vom eingehenden Schaden nimmt.
   *
   * **Er skaliert NICHT mit der Stufe**, und das ist gemessen, nicht
   * angenommen: `statAtLevel(0.2, 30)` ist null — die Funktion rechnet
   * ganzzahlig, und ein Anteil wird dabei zu nichts. Der Simulator lässt ihn
   * ebenfalls unverändert (`fight.rs`, `PLAYER.incoming`).
   *
   * Sachlich dieselbe Antwort: Ein Prozentsatz, der mitwächst, erreicht
   * irgendwann 100 % und macht den Spieler unverwundbar. Was mitwächst, ist
   * der abgefangene Schaden — und der ergibt sich aus dem größeren Pool.
   */
  get wardReduction(): number {
    return PLAYER.wardReduction;
  }

  get playerHp(): number {
    return this.playerHpValue;
  }

  /**
   * Nimmt die liegende Schutzschicht weg und gibt zurück, was davon als
   * Heilung ankommt (SUTURE, Issue #481).
   *
   * **Die Schicht ist danach ganz weg, nicht anteilig.** Das Raster sagt
   * „wandelt Schicht in Heilung um"; eine halbe Schicht stehen zu lassen wäre
   * ein anderer Zauber. Der Prozentsatz bestimmt, wie viel bei der Umwandlung
   * ANKOMMT — nicht, wie viel verbraucht wird.
   *
   * Ganzzahlig abgerundet, damit Spiel und Simulator dieselbe Zahl bekommen.
   */
  private verbraucheSchichtFuerHeilung(prozent: number): number {
    const liegend = this.shieldValue;
    if (liegend <= 0) return 0;
    this.shieldValue = 0;
    this.shieldStacksValue = 0;
    this.shieldExpiresAtMs = -1;
    return Math.floor((liegend * prozent) / 100);
  }

  /**
   * Wie viel Prozent der Anzeige gerade fehlen, abgerundet (MEND, Issue #481).
   *
   * **Ganzzahlig und abgerundet, nicht gerundet.** Beide Umsetzungen müssen
   * dieselbe Zahl bekommen, und `Math.floor` hat in Rust eine exakte
   * Entsprechung: die ganzzahlige Division. `Math.round` hätte bei `.5` eine
   * eigene Regel, die dort nachzubauen wäre.
   *
   * Am Wert DIESER Stufe gemessen, nicht am Grundwert — sonst zeigte ein
   * Spieler auf Stufe 30 mit vollem Leben ein Defizit an.
   */
  private fehlendProzent(): number {
    if (this.vesselMax <= 0) return 0;
    const fehlend = Math.max(0, this.vesselMax - this.playerHpValue);
    return Math.floor((fehlend * 100) / this.vesselMax);
  }

  /**
   * Ob das Unverwundbarkeits-Fenster gerade offen ist (Issue #218).
   *
   * **Der Spieler muss das sehen** — sonst hält er die ausbleibende Wirkung
   * für einen Rechenfehler (GAME-DESIGN 11.7: Information vorzuenthalten ist
   * die billigste Form von Schwierigkeit).
   */
  get raceOpen(): boolean {
    return this.raceEndsAtMs >= 0;
  }

  /**
   * Wie viele Zyklen die Zeitleiste gelaufen ist (Baustein 7; Issue #240).
   *
   * **Abgeleitet, nicht mitgezählt.** Ein eigenes Feld wäre eine zweite
   * Wahrheit über dieselbe Größe — und die zweite läuft beim Nachziehen weg.
   * Der Zeiger weiß es bereits.
   */
  /**
   * PRUEF-ZUGANG, erscheint nicht (Issue #294).
   *
   * Boss-Innenleben. Was der Spieler davon braucht, zeigt der Ring im Kampf.
   */
  get cycle(): number {
    const laenge = this.boss.timeline.length;
    if (laenge === 0 || this.boss.timelineLoopMs === null) return 0;
    return Math.floor(this.timelineIndexValue / laenge);
  }

  /** Ob die Schwächung gerade liegt (Baustein 8). */
  get afflicted(): boolean {
    return this.afflictedValue;
  }

  /** Wie oft sie entfernt wurde. */
  get afflictionsCleared(): number {
    return this.afflictionsClearedValue;
  }

  /** Wie weit das Ziel im laufenden Fenster geladen ist (Issue #218). */
  get raceProgress(): number {
    return this.raceProgressValue;
  }

  /** Ob das Ziel zerstört wurde — der Erfolgsfall des Wettrennens. */
  /** PRUEF-ZUGANG, erscheint nicht (Issue #294): Der Ausgang steht im Ergebnis. */
  get raceWon(): boolean {
    return this.raceWonValue;
  }

  /** Schaden, der im Fenster verpuffte — verloren, nicht ausgeteilt. */
  get damageLostToRace(): number {
    return this.damageLostToRaceValue;
  }

  /** Wie viel Fortschritt der Boss zurückgedreht hat. */
  get bossHealed(): number {
    return this.bossHealedValue;
  }

  /** Stand des tödlichen Zählers (Issue #216). 0, wenn der Boss keinen hat. */
  get doom(): number {
    return this.doomValue;
  }

  /** Verbleibende Anwendungen der Rücksetz-Handlung. */
  get doomCharges(): number {
    return this.doomChargesValue;
  }

  /**
   * Zahl der exponentiellen Stapel des Bosses (Issue #216).
   *
   * `bossStacks` und nicht `stacks`: Der Name war vergeben — die Schule fuehrt
   * unter `stacks` ihre eigene Stapel-Mechanik. Zwei verschiedene Dinge unter
   * einem Namen waeren im Kampf nicht auseinanderzuhalten.
   */
  get bossStacks(): number {
    return this.stacksValue;
  }

  /**
   * Die begrenzt verfügbare Handlung, die den Zähler senkt (11.2, Baustein 2).
   *
   * Gibt zurück, ob sie gegriffen hat. **Ohne Ladung geschieht nichts** — sie
   * nimmt sich nicht, was sie nicht hat, dieselbe Regel wie bei den
   * Kontroll-Punkten der Fixierung.
   *
   * WELCHE TASTE SIE AUSLÖST, STEHT HIER NICHT. Dieses Issue baut den
   * Mechanismus; die Bindung an einen Zauber entscheidet E10 mit den
   * Signatur-Regeln (Frage an den Auftraggeber, Meldung 1177).
   */
  clearDoom(): boolean {
    const doom = this.boss.doom;
    if (doom === null || this.doomChargesValue <= 0 || this.diedAtMsValue >= 0) return false;
    this.doomChargesValue--;
    // **Die Uhr läuft ab dem Verbrauch, nicht ab dem Kampfbeginn.** Wer spät
    // senkt, wartet ab dann — sonst bekäme ein Spieler, der lange nicht
    // gedrückt hat, seine Ladung sofort zurück und die Wartezeit wäre keine.
    if (this.nextDoomChargeAtMs === 0)
      this.nextDoomChargeAtMs = this.nowMsForBuffs + this.rechargeMs();
    const left = this.doomValue - doom.clearAmount;
    this.doomValue = left > 0 ? left : 0;
    return true;
  }

  /**
   * Wie lange eine verbrauchte Senkungs-Ladung zum Nachladen braucht
   * (Issue #561).
   *
   * # Warum die Zahl aus den Bossdaten FOLGT statt ein eigenes Feld zu sein
   *
   * Sie ist genau die Zeit, in der der Zähler um `clearAmount` steigt — also
   * so lang, dass eine Ladung nachwächst, während der Schaden nachwächst, den
   * sie abträgt. Damit ist der Vorrat selbstregulierend: Bei einem Boss, der
   * schneller lädt, kommen die Ladungen schneller zurück.
   *
   * Ein eigenes Datenfeld hätte bei **jedem** neuen `doom`-Boss von Hand
   * passen müssen — und genau das war der Einwand gegen den festen Vorrat:
   * zu wenig heißt unschaffbar, zu viel heißt Kulisse. Eine abgeleitete Zahl
   * kann nicht vergessen werden.
   *
   * Bei `boss-1-2` sind das 40/6 × 1000 ms ≈ **6667 ms**.
   */
  private rechargeMs(): number {
    const doom = this.boss.doom;
    if (doom === null || doom.amount <= 0) return 0;
    return Math.round((doom.clearAmount / doom.amount) * doom.perStepMs);
  }

  /** Wie viel Fortschritt der Spieler zurückgedrängt hat (Baustein 6; #247). */
  /** ANGEZEIGT seit #294: "Pushed back", nur bei einem `runaway`-Boss. */
  get stabilised(): number {
    return this.stabilisedValue;
  }

  /** Welcher Boss-Zauber gerade läuft, oder -1. */
  get casting(): number {
    return this.castingValue;
  }

  /**
   * Wie der laufende Boss-Zauber heißt, oder `null` (Issue #327).
   *
   * **Ein schmaler Zugang statt `timelineEntryAt` öffentlich zu machen.**
   * Dieselbe Regel wie beim Vorbereitungs-Bildschirm in #328, der fertige
   * Zeilen bekommt und nicht den Boss: Wer der Anzeige den ganzen Eintrag
   * gäbe, lüde sie ein, mehr daraus zu lesen — Schaden, Wirkzeit, Mechanik —
   * und die nächste Anzeige träfe die Entscheidungen dann noch einmal.
   *
   * Englisch, wie alle Spieltexte: Der Name kommt aus dem Boss-Datensatz.
   */
  get castingName(): string | null {
    if (this.castingValue < 0) return null;
    return this.timelineEntryAt(this.castingValue)?.name ?? null;
  }

  /**
   * Wann der laufende Boss-Zauber BEGONNEN hat — Kampfzeit seit Beginn
   * (Issue #327).
   *
   * # Warum das ein eigenes Feld braucht und keine Rechnung ist
   *
   * Ein Balken braucht Anfang und Ende. Das Ende steht in `castLandsAtMs`, der
   * Anfang wäre der Zeitpunkt des Zeitleisten-Eintrags — nur ist er nach der
   * ersten Verschiebung nicht mehr zurückzurechnen:
   *
   * | Wer | Was |
   * | --- | --- |
   * | Eskalation (#240) | verkürzt die DAUER, beim Start |
   * | STALL / FIXATION (#100) | verschiebt das ENDE, **während** er läuft |
   *
   * „Ende minus Dauer" ergäbe nach einem STALL einen Anfang, den es nie gab —
   * und der Balken spränge zurück, statt langsamer zu werden.
   *
   * # Nicht in der Prüfsumme, und das ist eine Entscheidung
   *
   * Er beeinflusst keine Rechnung: Kein Treffer, keine Abklingzeit, kein
   * Zustand hängt an ihm. Ein Feld, das nichts entscheidet, macht die
   * Prüfsumme nicht schärfer — es verlängert nur die Feldfolge, die beide
   * Seiten gleich halten müssen.
   *
   * **Die Grenze verläuft nicht bei „neuer Zustand", sondern bei „entscheidet
   * etwas".** Das ist der Unterschied zu `playerCastingValue` aus #304: Der
   * bestimmt, was ein Tastendruck abbricht und welche Wirkung verlorengeht —
   * er gehört in die Prüfsumme, und zwar auf beiden Seiten. Ein
   * Anfangszeitpunkt für einen Balken bestimmt nichts.
   *
   * Wer diese Grenze bei „neu" statt bei „entscheidend" zieht, mischt
   * entweder zu viel ein (und macht jede Vergleichsdatei bei jeder Anzeige
   * ungültig) oder zu wenig (und lässt Spiel und Simulator auseinanderlaufen,
   * ohne dass es jemand meldet).
   *
   * Die Probe darauf ist `npm run parity:export`: Schreibt es nichts, hat der
   * Simulator nichts zu lernen.
   */
  get castingStartedAtMs(): number {
    return this.castingStartedAtMsValue;
  }

  /**
   * Wann der laufende Boss-Zauber trifft — Kampfzeit seit Beginn.
   *
   * **Nicht für die Anzeige, sondern für die Prüfsumme** (GAME-DESIGN 19.1).
   * Ohne diesen Zugriff wäre sie für zwei Dinge blind, die beide genau hier
   * wirken: die Eskalation, die das Fenster mit jedem Zyklus verkürzt
   * (Baustein 7), und die Fixierung, die es verlängert. Bei beiden bliebe der
   * Unterschied verborgen, bis der Treffer fällt — und dann sagt die Prüfsumme
   * nicht mehr, in welchem Takt gerechnet wurde, sondern nur noch, dass am Ende
   * etwas nicht stimmt.
   *
   * Der Wert ist bedeutungslos, solange `casting` auf -1 steht.
   */
  get castLandsAtMs(): number {
    return this.castLandsAtMsValue;
  }

  /** Welchen Zauber der Spieler gerade wirkt, oder -1 (Issue #304). */
  get playerCasting(): SpellIndex {
    return this.playerCastingValue;
  }

  /** Wie oft ein Boss-Angriff eine Wirkzeit abgebrochen hat (Issue #304). */
  get castsInterrupted(): number {
    return this.castsInterruptedValue;
  }

  /** Wie oft ein eigener Druck eine Wirkzeit abgebrochen hat (Issue #304). */
  get castsCancelled(): number {
    return this.castsCancelledValue;
  }

  /**
   * Wann die laufende Wirkzeit endet — Uhrzeit des Spiels, oder -1.
   *
   * Geht wie `castLandsAtMs` in die Prüfsumme ein: Eine Wirkzeit, die auf einer
   * Seite anders gerechnet wird (Tempo, Fixierung), fiele sonst erst auf, wenn
   * der Schaden fällt — also Takte später, an einer Stelle, die nicht mehr auf
   * die Ursache zeigt.
   */
  get playerCastEndsAtMs(): number {
    return this.playerCastEndsAtMsValue;
  }

  /**
   * Wie weit die laufende Wirkzeit fortgeschritten ist, 0 bis 1.
   *
   * **Für den Balken** (AC1). Er rechnet nicht selbst: Die Dauer steckt in
   * `playerCastStartedAtMsValue`, und sie ist bereits mit Tempo verrechnet. Wer
   * sie in der Anzeige aus `spell.castTimeMs` neu herleitete, hätte dieselbe
   * Regel an zwei Orten — und der Balken liefe bei getragenem CATALYSIS anders
   * als der Zauber.
   *
   * Ohne laufende Wirkzeit: 0.
   */
  playerCastProgress(nowMs: number): number {
    if (this.playerCastingValue < 0) return 0;
    const dauer = this.playerCastEndsAtMsValue - this.playerCastStartedAtMsValue;
    if (dauer <= 0) return 1;
    const vergangen = nowMs - this.playerCastStartedAtMsValue;
    if (vergangen <= 0) return 0;
    return vergangen >= dauer ? 1 : vergangen / dauer;
  }

  /** Ist der Spieler tot? Der Versuch endet dann (GAME-DESIGN A.3). */
  /**
   * PRUEF-ZUGANG, erscheint nicht (Issue #294).
   *
   * Der Tod steht als "VESSEL LOST" im Ergebnis — eine zweite Anzeige dafuer
   * waere dieselbe Aussage zweimal.
   */
  get playerDead(): boolean {
    return this.diedAtMsValue >= 0;
  }

  /** Kampfzeit des Spielertods, oder -1. */
  /** PRUEF-ZUGANG, erscheint nicht (Issue #294): Die Zeit steht als `elapsedMs`. */
  get diedAtMs(): number {
    return this.diedAtMsValue;
  }

  /** Gesamter erlittener Schaden, **nach** WARD. */
  /** ANGEZEIGT seit #294: "Damage taken" bei den Kennzahlen. */
  get damageTaken(): number {
    return this.damageTakenValue;
  }

  /** Wie oft der Spieler einen Boss-Zauber unterbrochen hat. */
  /** ANGEZEIGT seit #294: "Interrupts landed" im aufgeklappten Bereich. */
  get interruptsLanded(): number {
    return this.interruptsLandedValue;
  }

  /** Wie oft einer durchgekommen ist. */
  get interruptsMissed(): number {
    return this.interruptsMissedValue;
  }

  /**
   * Schaden am Spieler, **nach** WARD (GAME-DESIGN 6.1).
   *
   * Der Tod wird hier festgestellt und nicht erst in der Kampfschleife: Ein
   * Spieler mit null Lebenspunkten, der noch einen Takt weiterwirkt, wäre eine
   * stille Abweichung zum Simulator.
   */
  /** Stand der Schutzschicht (COAGULATION, Weg 1). */
  get shield(): number {
    return this.shieldValue;
  }

  /** Wie viele Anwendungen der Schutzschicht übereinanderliegen. */
  get shieldStacks(): number {
    return this.shieldStacksValue;
  }

  /** Wie viel Schaden die Schutzschicht insgesamt aufgefangen hat. */
  /** ANGEZEIGT seit #294: "Shield absorbed" im aufgeklappten Bereich. */
  get shieldAbsorbed(): number {
    return this.shieldAbsorbedValue;
  }

  /** Wie viel Leben insgesamt zurückgeholt wurde (Weg 2). */
  get healed(): number {
    return this.healedValue;
  }

  /** Anteil der laufenden Umwandlung in Prozent; 0 heißt: keine (Weg 3). */
  get conversionPercent(): number {
    return this.conversionPercentValue;
  }

  /** Wie viel Schutz die Umwandlung erzeugt hat. */
  /** ANGEZEIGT seit #294: "Converted" im aufgeklappten Bereich. */
  get converted(): number {
    return this.convertedValue;
  }

  /** Anteil des laufenden Rückschlags in Prozent; 0 heißt: keiner (RIPOSTE). */
  get reflectPercent(): number {
    return this.reflectPercentValue;
  }

  /** Wie viel Schaden der Rückschlag insgesamt an den Boss zurückgegeben hat. */
  get reflected(): number {
    return this.reflectedValue;
  }

  /** Vorrat an Kontroll-Punkten (FIXATION, Issue #100). */
  get control(): number {
    return this.controlValue;
  }

  /** Wie viele Kontroll-Punkte insgesamt ausgegeben wurden. */
  get controlSpent(): number {
    return this.controlSpentValue;
  }

  /** Wie oft THE LONG GRIP einen Ausgeber umsonst gemacht hat (13.5). */
  get freeControlSpendCount(): number {
    return this.freeControlSpends;
  }

  /**
   * Verbraucht den Merker aus SUBLIME, falls er steht (Issue #480).
   *
   * **Einmal gesetzt, einmal verbraucht**: Ein Merker, der stehen bliebe,
   * machte aus einem einzelnen freien Wurf eine dauerhafte Befreiung.
   */
  private takeFreeGcd(): boolean {
    if (!this.nextCastFreeGcdValue) return false;
    this.nextCastFreeGcdValue = false;
    return true;
  }

  /**
   * Ob der nächste Wurf von der globalen Wartezeit befreit ist (#480).
   *
   * **Diese beiden stehen bewusst HIER und nicht bei `playerDead`.** Ein
   * erster Entwurf setzte sie dorthin — und schnitt damit den Kommentarblock
   * ab, den `measured-sizes-source.test.ts` über `get playerDead(` liest. Der
   * Wächter wurde rot, ohne dass an `playerDead` etwas geändert war.
   *
   * Ein Einschub deutet nicht nur Reihenfolgen um, sondern auch Kommentare,
   * die jemand als Datenquelle nutzt.
   */
  get nextCastFreeGcd(): boolean {
    return this.nextCastFreeGcdValue;
  }

  /**
   * Verbraucht den Merker aus CONDENSE, falls er steht (#480).
   *
   * Einmal gesetzt, einmal verbraucht — wie bei SUBLIME. Ein Merker, der
   * stehen bliebe, machte aus einer einzelnen Verdopplung eine dauerhafte.
   */
  private takeDoubleCast(): boolean {
    if (!this.nextCastDoublesValue) return false;
    this.nextCastDoublesValue = false;
    return true;
  }

  /** Ob der nächste Wurf zweimal wirkt (#480). */
  get nextCastDoubles(): boolean {
    return this.nextCastDoublesValue;
  }

  /**
   * Verbraucht die Rüstung, falls sie steht (#480).
   *
   * Einmal gerüstet, einmal verbraucht — wie bei den zwei Merkern darüber.
   * Eine Rüstung, die stehen bliebe, machte aus einer Vorbereitung eine
   * dauerhafte Vergünstigung.
   */
  private takeRetortArmed(): boolean {
    if (!this.retortArmedValue) return false;
    this.retortArmedValue = false;
    return true;
  }

  /** Ob RETORTs Rüstung steht — für die Prüfsumme und für Prüffälle (#480). */
  get retortArmed(): boolean {
    return this.retortArmedValue;
  }

  /** Wie oft eine Kontroll-Wirkung mangels Punkten ins Leere lief. */
  /** ANGEZEIGT seit #294: "Control wasted" im aufgeklappten Bereich. */
  get controlFailed(): number {
    return this.controlFailedValue;
  }

  /** Um wie viele Prozentpunkte die Phasen-Schwelle gesenkt ist. */
  get phaseHoldPercent(): number {
    return this.phaseHoldPercentValue;
  }

  /**
   * Kontroll-Punkte aufbauen und ausgeben (FIXATION, GAME-DESIGN 4.1).
   *
   * **Der Spieler entscheidet, wofür** — deshalb steht die Wirkung am Zauber
   * und nicht an einer Automatik. Reichen die Punkte nicht, geschieht nichts.
   */
  /**
   * Eine Umwandlung aus einer schulfremden Quelle (Issue #520, EFFLUX).
   *
   * # Warum das nicht in `applyFixation` steht
   *
   * Dort läuft FIXATIONs Punkte-Buchhaltung: `controlValue -= cost` und, bei
   * `convertToDamage`, `this.controlValue = 0`. Ein SUBLIMATION-Zauber, der
   * dort durchginge, würde dem Spieler auf einer gemischten Leiste die
   * Kontroll-Punkte **löschen** — kein Buchungsfehler, sondern ein Verlust.
   *
   * # Was geteilt wird: die SENKE
   *
   * `pendingConvertedDamage` ist schulneutral — sie nimmt einen Betrag und
   * macht Schaden daraus, durch dieselben Faktoren wie jeder Treffer
   * (Krit, Widerstand, Verstärkung). Nur der Weg dorthin ist je Schule
   * verschieden. Deshalb schreibt diese Stelle in dieselbe Senke statt eine
   * zweite Rechnung aufzumachen.
   */
  private applyConversion(spell: SpellDef, timeMs: number): void {
    const convert = spell.convert;
    if (convert === undefined) return;

    // **Ganze Sekunden.** `perSecond` ist Schaden je Sekunde; Millisekunden
    // durchzureichen ergäbe Nachkommastellen an einer Stelle, an der
    // GAME-DESIGN 18.4 ganze Zahlen verlangt — und der Simulator rechnet in
    // `u32`.
    //
    // **Der leere Fall ist kein Sonderfall:** Liegt keine Abklingzeit an, ist
    // die Summe null und der Schaden null. Der Zauber bricht nicht, er bringt
    // nichts — genau das, was ein Notausgang tun soll, wenn es nichts
    // umzuwandeln gibt.
    const restMs = this.remainingCooldownSumMs(timeMs);
    this.pendingConvertedDamage = Math.floor(restMs / 1000) * convert.perSecond;
  }

  private applyFixation(spell: SpellDef, timeMs: number): void {
    if (spell.controlGain !== undefined && spell.controlGain > 0) {
      // **Der Faktor aus `controlBuffs`** (Issue #489, GRIPLOCK). Ganzzahlig
      // gerundet, weil Kontroll-Punkte ganze Zahlen sind — ein halber Punkt
      // wäre eine Größe, die das Spiel nirgends anzeigt.
      // **Zauber-Buff UND Talent** (Issue #419). GRIPLOCK speist den einen,
      // der Aufbau-Weg den anderen — multiplikativ, weil sie aus zwei Töpfen
      // kommen (13.1b): ein Verstärker im Kampf und eine dauerhafte Wahl.
      const faktor = this.controlBuffs.factor(timeMs) * talentControlGainFactor(this.talents);
      this.controlValue += Math.round(spell.controlGain * faktor);
    }

    const control = spell.control;
    if (control === undefined) return;
    // **Der Preis-Weg TEILT** (Issue #419, FIXATION). Als einziger der sieben:
    // Ein Weg „billiger ausgeben" senkt den Preis, ein Faktor 1,2 darauf
    // multipliziert hieße teurer.
    //
    // **Geklemmt auf mindestens 1** — ein Ausgeber, der nichts kostet, wäre
    // keiner, und `controlValue >= 0` wäre bei Preis 0 immer wahr. Die
    // Klemmung steht hier und nicht im Parser: Sie folgt aus der Verteilung,
    // nicht aus den Daten.
    const preis = Math.max(1, Math.round(control.cost / talentControlCostFactor(this.talents)));
    // **FIXATIONs Schlüsseltalent** (#419): Der Zauber scheitert nicht mehr,
    // sondern wirkt mit dem, was da ist.
    //
    // Ohne das Talent ist der Druck verloren, sobald der Vorrat nicht reicht.
    // Mit ihm wird der Rest eingesetzt — und die Wirkung sinkt im selben
    // Verhältnis (siehe `anteilNenner` unten). Das ist der Unterschied zu
    // einem Rabatt: Billiger wäre ein Faktor, anteilig ist eine andere
    // Rechnung, und genau das verlangt #418 von einem Schlüsseltalent.
    //
    // **Bei leerem Vorrat scheitert er weiterhin.** Ein Zauber, der aus nichts
    // etwas macht, wäre kein Schlüsseltalent, sondern ein Fehler.
    const anteilig = this.hasKeystone(KEYSTONE_PARTIAL_CONTROL);
    if (this.controlValue < preis && (!anteilig || this.controlValue <= 0)) {
      this.controlFailedValue++;
      return;
    }
    // **Der Einsatz ist geklemmt** (#419, `partial-control`): Ohne das Talent
    // ist er der volle Preis, mit ihm bei knappem Vorrat weniger — und die
    // Wirkung sinkt unten im selben Verhältnis.
    const einsatz = Math.min(preis, this.controlValue);
    // **THE LONG GRIP** (13.5, FIXATION-Sechser): Der erste Ausgeber nach
    // einem Phasenwechsel kostet nichts.
    //
    // **Der ABZUG entfällt, nicht die VORAUSSETZUNG.** Die Prüfung oben läuft
    // unverändert: Man braucht die Punkte, gibt sie aber nicht aus. Die
    // Klemmung auf Preis 1 begründet sich damit, dass `controlValue >= 0` bei
    // Preis 0 immer wahr wäre — ein Effekt, der die Prüfung überspränge,
    // umginge sie statt sie zu berücksichtigen.
    //
    // **Ein Ja/Nein, keine Zahl** — wie die drei Merker aus E10: Zwei
    // Phasenwechsel ohne Ausgeber dazwischen sind ein freier Ausgeber, nicht
    // zwei. Wer sie sammeln könnte, hortete Wechsel statt den Moment zu
    // treffen, und der Effekt belohnte das Gegenteil seiner Begründung.
    const umsonst = this.hasSetEffect(SET_EFFECT_LONG_GRIP) && this.freeControlSpendArmed;
    if (umsonst) {
      this.freeControlSpendArmed = false;
      this.freeControlSpends++;
    } else {
      // **Der EINSATZ, nicht der Preis** — sonst zöge ein anteiliger Ausgeber
      // (#419) den vollen Preis ab, den er gar nicht hatte.
      this.controlValue -= einsatz;
    }
    this.controlSpentValue += einsatz;
    // **Der Mindesteinsatz** (Issue #683). Jeder Kontroll-Zauber geht hier
    // durch; die beiden Stellen weiter unten nehmen bei zwei Wirkungen
    // zusätzlich den REST und schreiben ihn eigens.
    this.log?.push(this.fightElapsedMs(timeMs), EVENT_CONTROL_SPENT, NO_SPELL_REF, einsatz);

    // Die drei Wirkungen aus 4.1. Hier greift FIXATION wirklich in den
    // Kampfverlauf ein — das ist, was sie von jeder anderen Schule
    // unterscheidet.
    // **CAPSTONE loest alle drei aus** (Issue #478): der große Moment der
    // Schule. Kein neuer Mechanismus — er ruft, was die drei Ausgeber je
    // einzeln tun, und verbraucht dafür alle Punkte.
    // **Der Wirkungs-Weg** (Issue #419, FIXATION). Einmal ausgerechnet und an
    // ALLEN fünf Wirkungen benutzt — jede Wirkung einzeln zu multiplizieren
    // liefe bei einer späteren Änderung auseinander, und `allThree` löst drei
    // davon gleichzeitig aus.
    // **Der Anteil steckt IN der einen Rundung**, nicht daneben (GAME-DESIGN
    // 18.4, ganzzahlig bis zur letzten Teilung). Wer erst rundet und dann
    // teilt, bekommt auf der Rust-Seite eine andere Zahl — und die
    // Bitgleichheit reisst an einer Stelle, die niemand mit dem Talent in
    // Verbindung braechte.
    const staerke = Math.round(
      (control.amount * talentControlEffectFactor(this.talents) * einsatz) / preis,
    );
    // **Die Phasen-Senkung hat ihre EIGENE Stärke** (Issue #478).
    //
    // `slowCast` und `delayStrike` rechnen in Millisekunden, `holdPhase` in
    // Prozentpunkten. Für die drei Spezialisten fällt das nicht auf — jeder
    // trägt genau eine Einheit. CAPSTONE löst als einziger alle drei aus, und
    // seine 2000 Millisekunden landeten bis hierher im Prozent-Feld.
    //
    // Gemessen an `test-phase`, wann der Wechsel fiel:
    //
    // ```text
    //   ohne Zauber    6010 ms
    //   ANCHOR (10 %)  9010 ms   der Boss muss 10 % tiefer fallen
    //   CAPSTONE      14010 ms   genau PHASE_HOLD_DURATION_MS nach dem Wurf
    // ```
    //
    // 2000 Prozentpunkte sind keine Senkung, sondern eine Abschaltung: Der
    // Wechsel fiel erst, als die Frist ablief. Der Plan verlangt „die
    // Phasen-Schwelle SINKT vorübergehend" und „alle kaufen einen einzelnen
    // Moment, keinen Zustand" — bei vollständiger Blockade muss man den
    // Moment nicht TREFFEN, man muss nur irgendwann drücken.
    //
    // **Der Rückfall auf `staerke` ist kein Rest, sondern die Regel für alle
    // anderen**: ANCHOR trägt nur `amountPercent`, also steht seine Zahl in
    // beiden Feldern, und für ihn ändert sich nichts.
    // **Auch sie sinkt anteilig** (#419). Ohne das wäre das Schlüsseltalent
    // bei ANCHOR ein reiner Rabatt: halber Einsatz, volle Senkung.
    const senkung =
      control.amountPercent > 0
        ? Math.round(
            (control.amountPercent * talentControlEffectFactor(this.talents) * einsatz) / preis,
          )
        : staerke;
    const alleDrei = control.effect === 'allThree';
    if (alleDrei) {
      // **Alle Punkte, wie bei WEDGE.** Das Raster sagt „verbraucht alle
      // Punkte" — `cost` ist der Mindesteinsatz. Bliebe der Rest liegen,
      // brächte der große Moment der Schule bei vollem Vorrat genauso viel
      // wie bei knappem.
      // **Der Rest bei ALLE DREI** (Issue #683). Eigener Eintrag, nicht mit
      // dem Mindesteinsatz verrechnet: Wer den Verlauf liest, soll sehen,
      // dass hier über die Kosten hinaus geleert wurde.
      this.log?.push(
        this.fightElapsedMs(timeMs),
        EVENT_CONTROL_SPENT,
        NO_SPELL_REF,
        this.controlValue,
      );
      this.controlSpentValue += this.controlValue;
      this.controlValue = 0;
    }

    if (control.effect === 'slowCast' || alleDrei) {
      // Der laufende Boss-Zauber trifft später; das Unterbrechen-Fenster wird
      // größer.
      if (this.castingValue >= 0) this.castLandsAtMsValue += staerke;
    }
    if (control.effect === 'delayStrike' || alleDrei) {
      // **Nur der nächste Eintrag.** Die folgenden stehen, wo sie stehen —
      // der Abstand dahinter schrumpft, und genau das ist der Preis.
      this.nextStrikeDelayMs += staerke;
    }
    if (control.effect === 'makeInterruptible') {
      // **PIN** (Issue #478): Der nächste Boss-Zauber wird unterbrechbar, auch
      // wenn er es nicht war. Damit verbindet sich die Schule mit QUENCH — wer
      // unterbrechen kann, braucht etwas zum Unterbrechen.
      //
      // **NICHT gegen den laufenden.** Der Merker wird erst gelesen, wenn der
      // nächste Eintrag auslöst; ein `castingValue >= 0` wie bei `slowCast`
      // wäre hier falsch. Ein Zauber, der schon läuft, ist entweder ohnehin
      // unterbrechbar oder trifft sofort — in beiden Fällen kommt PIN zu spät,
      // und das ist die Entscheidung, die er dem Spieler abverlangt.
      this.pinWindowMsValue = staerke;
    }
    if (control.effect === 'convertToDamage') {
      // **Der Notausgang** (Issue #478, WEDGE): Gegen einen Boss ohne
      // Mechanik gibt es nichts zu verlangsamen, zu verzögern oder
      // hinauszuzögern — die drei anderen Wirkungen laufen dort ins Leere,
      // und FIXATION wäre die einzige Schule, die an manchen Bossen gar
      // nicht spielbar ist.
      // **Alle Punkte, nicht nur die Kosten.** `cost` ist der Mindesteinsatz
      // — was darüber liegt, geht mit. Sonst wäre WEDGE ein Zauber, der bei
      // vollem Vorrat genauso viel bringt wie bei knappem, und „aufsparen"
      // hätte keinen Sinn.
      const rest = this.controlValue;
      this.controlValue = 0;
      this.controlSpentValue += rest;
      // **Der Rest bei UMWANDLUNG** (Issue #683). Dieselbe Überlegung wie
      // oben — und `rest` kann 0 sein, wenn der Vorrat genau den Kosten
      // entsprach. Dann steht hier eine 0, und das ist richtig: Der Weg
      // wurde genommen, er hat nur nichts mehr vorgefunden.
      this.log?.push(this.fightElapsedMs(timeMs), EVENT_CONTROL_SPENT, NO_SPELL_REF, rest);
      // `preis` statt `control.cost`: Wer billiger ausgibt, wandelt auch
      // weniger um — sonst wäre der Preis-Weg zugleich ein Schadens-Weg.
      this.pendingConvertedDamage = (preis + rest) * staerke;
    }
    if (control.effect === 'holdPhase' || alleDrei) {
      // Die Schwelle sinkt vorübergehend. **Sie nimmt einen Wechsel nicht
      // zurück** — die Phase kann nur steigen.
      //
      // `senkung` statt `staerke`: siehe die Begründung oben. Für ANCHOR sind
      // beide gleich, für CAPSTONE nicht.
      this.phaseHoldPercentValue = senkung;
      // **Dieselbe Frist für alle**, auch für CAPSTONE, der als einziger ein
      // zweites Betrags-Feld hat. Der Plan: „Die Frist gilt für ALLE ZAUBER
      // GLEICH — sie gehört zur Wirkung, nicht zum einzelnen Zauber; sonst
      // hieße ‚hinauszögern' bei jedem etwas anderes."
      this.phaseHoldUntilMs = timeMs - this.fightStartedAtMs + PHASE_HOLD_DURATION_MS;
    }
  }

  /**
   * Die drei Wege der Koagulation, soweit sie beim Wirken greifen
   * (GAME-DESIGN 4.1, Issue #79).
   *
   * Weg 3 wirkt hier nur als **Anschalter**: Die Umwandlung selbst geschieht
   * später, bei jedem ausgeteilten Schaden.
   */
  private applyCoagulation(spell: SpellDef, timeMs: number, index: SpellIndex): void {
    const seitBeginn = timeMs - this.fightStartedAtMs;

    // Weg 1 — Schutzschicht aufbauen.
    const shield = spell.shield;
    if (shield !== undefined) {
      // **Stapeln bis zur Grenze, dann nicht weiter.** Ein weiterer Druck auf
      // eine volle Schicht erneuert die Laufzeit, erhöht aber den Betrag
      // nicht — sonst wäre die Obergrenze wirkungslos.
      // **Die drei COAGULATION-Wege** (Issue #419). Alle drei einmal
      // ausgerechnet und dann benutzt — `shield.amount` wird unten ZWEIMAL
      // gelesen (Schicht und Stabilisierung), und zwei getrennte Rechnungen
      // liefen bei einer späteren Änderung auseinander.
      const dicke = Math.floor(shield.amount * talentShieldAmountFactor(this.talents));
      const grenze = shield.maxStacks + talentShieldStacksBonus(this.talents);
      if (this.shieldStacksValue >= grenze && this.hasSetEffect(SET_EFFECT_SEALED_VESSEL)) {
        // **THE SEALED VESSEL** (13.5, COAGULATION-Sechser): Was die volle
        // Schicht nicht mehr aufnimmt, geht als Schaden hinaus.
        //
        // **Der Betrag, den dieser Druck BEIGETRAGEN HÄTTE** — also `dicke`
        // mitsamt Talent-Faktor, nicht `shield.amount`. Sonst zahlte der
        // Sechser nach dem alten Wert aus, während die Schicht nach dem neuen
        // rechnet: beides plausibel, eines falsch.
        //
        // Er ersetzt nichts: Die Laufzeit wird unten weiter erneuert, und der
        // Stapelstand bleibt an der Grenze. Der Effekt fügt nur hinzu, was
        // vorher niemand bekam.
        this.bookProgress(dicke, index, timeMs);
        this.reportHit(dicke, HIT_DIRECT, false, timeMs);
        this.log?.push(this.fightElapsedMs(timeMs), EVENT_DAMAGE, index, dicke);
      }
      if (this.shieldStacksValue < grenze) {
        this.shieldStacksValue++;
        this.shieldValue += dicke;
        // Für die Nachlage (#419): Der Bruch braucht die Dicke, und jede neu
        // gelegte Schicht bekommt ihre eigene Nachlage.
        this.shieldLastAmount = dicke;
        this.shieldRefundUsed = false;
        // **Der Schutz drängt die Reaktion zurück** (Baustein 6; #247), um
        // seinen EIGENEN Betrag. Nur beim Stapeln, nicht beim bloßen Erneuern:
        // Sonst wäre die Obergrenze wirkungslos, und das gilt hier wie für den
        // Schutz selbst.
        //
        // **Mit der GEHOBENEN Dicke**, nicht mit `shield.amount`: Sonst wäre
        // die Schicht dicker und stabilisierte nach dem alten Wert — beides
        // grün, beides falsch.
        this.stabilise(dicke);
      }
      this.shieldExpiresAtMs =
        seitBeginn + Math.floor(shield.durationMs * talentShieldDurationFactor(this.talents));
    }

    // Weg 2 — Heilung.
    const heal = spell.heal;
    // **Der Betrag steht beim WIRKEN fest** (MEND, Issue #481). Fehlt dem
    // Spieler nichts, bleibt es beim Grundbetrag; je mehr fehlt, desto mehr
    // kommt dazu. Ganzzahlig gerechnet und in Prozent, damit Spiel und
    // Simulator dieselbe Zahl bekommen (GAME-DESIGN 18.4).
    //
    // Die Reihenfolge ist die Aussage: erst der Betrag, dann die Verteilung.
    // Anders herum müsste jeder Heil-Takt den Zustand von damals kennen — und
    // ein Zauber, der über sechs Sekunden heilt, würde unterwegs stärker,
    // während der Boss zuschlägt.
    //
    // **Die Schutzschicht geht als Ganzes über** (SUTURE, Issue #481). Sie
    // wird verbraucht, BEVOR der Betrag feststeht — stünde sie noch, wenn die
    // Heilung ankommt, gäbe der Zauber beides.
    const ausSchicht =
      heal !== undefined && heal.fromShieldPercent !== undefined && heal.fromShieldPercent > 0
        ? this.verbraucheSchichtFuerHeilung(heal.fromShieldPercent)
        : 0;
    const gewirkterBetrag =
      heal === undefined
        ? 0
        : heal.amount + this.fehlendProzent() * (heal.perMissingPercent ?? 0) + ausSchicht;
    // **Heilung über Zeit läuft an, statt sofort anzukommen** (POULTICE,
    // Issue #481). Sie ersetzt die Sofortheilung für diesen Zauber; beides
    // zugleich wäre doppelt.
    if (heal !== undefined && heal.overMs !== undefined && heal.overMs > 0) {
      // Abgeleitet, nicht konfiguriert: Die Schrittzahl folgt aus Dauer und
      // Takt. Eine eigene Zahl dafür könnte still von beiden abweichen.
      const abstand = Math.max(TICK_MS, heal.tickIntervalMs ?? TICK_MS);
      this.healOverTimeAmount = gewirkterBetrag;
      this.healOverTimeSteps = Math.max(1, Math.round(heal.overMs / abstand));
      this.healOverTimeIntervalMs = abstand;
      this.healOverTimeDone = 0;
      this.healOverTimeCredited = 0;
      this.healOverTimeNextAtMs = seitBeginn + abstand;
    } else if (heal !== undefined) {
      // **Nur bis zur vollen Anzeige.** Was darüber hinausginge, verpufft — es
      // als Heilung zu zählen ließe einen Lauf mit voller Anzeige nach mehr
      // Wirkung aussehen, als er gebraucht hat.
      // Gedeckelt am Wert DIESER Stufe: Sonst heilte ein Spieler auf Stufe 30
      // nur bis zum Grundwert, und die gewonnenen Punkte wären nach dem
      // ersten Schaden fort.
      const fehlend = Math.max(0, this.vesselMax - this.playerHpValue);
      const wirklich = Math.min(gewirkterBetrag, fehlend);
      if (wirklich > 0) {
        this.playerHpValue += wirklich;
        this.healedValue += wirklich;
        // Auch die Heilung drängt zurück — mit dem Betrag, der WIRKLICH
        // angekommen ist. Was über die volle Anzeige hinausginge, verpufft
        // hier wie dort.
        this.stabilise(wirklich);
        this.log?.push(this.fightElapsedMs(timeMs), EVENT_HEALED, NO_SPELL_REF, wirklich);
      }
    }

    // Weg 3 — die Umwandlung anschalten.
    const conversion = spell.conversion;
    if (conversion !== undefined) {
      this.conversionPercentValue = conversion.sharePercent;
      this.conversionUntilMs = seitBeginn + conversion.durationMs;
    }

    // Weg 3, andere Richtung — den Rückschlag anschalten (RIPOSTE, #481).
    const reflect = spell.reflect;
    if (reflect !== undefined) {
      this.reflectPercentValue = reflect.sharePercent;
      this.reflectUntilMs = seitBeginn + reflect.durationMs;
      this.reflectSpellIndex = index;
    }

    // PUTRID BLOOM (Issue #479) — die Grenze anheben.
    //
    // **Liegende Stapel bleiben liegen, wenn die Anhebung ausläuft.** Die
    // Grenze entscheidet nur, ob NACHGELEGT werden darf (`decayCount >=
    // decayCap`); sie wirft nichts weg. Wer unter der Anhebung sieben Stapel
    // aufbaut, behält sie bis zu ihrem eigenen Verfall — sonst wäre das
    // Auslaufen ein zweiter, unsichtbarer Verfall, und die Uhren am Stapel
    // logen.
    const decayCap = spell.decayCap;
    if (decayCap !== undefined) {
      this.decayCapBonusValue = decayCap.bonus;
      this.decayCapUntilMs = seitBeginn + decayCap.durationMs;
    }

    // DECAY (Issue #479) — die Schwächung anschalten. Die STAPELZAHL wird
    // hier NICHT festgehalten: Sie wird beim Treffer gelesen, nicht beim
    // Wirken. Wer sie hier einfröre, machte aus „schwächt nach Stapelzahl"
    // ein „schwächt nach der Stapelzahl von vorhin" — und der Aufbau nach dem
    // Wurf zählte nicht mehr.
    const decayWard = spell.decayWard;
    if (decayWard !== undefined) {
      this.decayWardPermilleValue = decayWard.permillePerStack;
      this.decayWardUntilMs = seitBeginn + decayWard.durationMs;
    }
  }

  /**
   * Wandelt ausgeteilten Schaden in Schutz um, solange Weg 3 läuft.
   *
   * **Ganzzahlig und abgerundet.** Der Anteil steht als Prozentzahl in den
   * Daten, damit hier `Math.floor(schaden * prozent / 100)` möglich ist. Ein
   * Bruch als Faktor müsste von beiden Seiten unabhängig gerundet werden —
   * genau daran laufen zwei Umsetzungen auseinander (GAME-DESIGN 18.4).
   */
  private convertDamageToShield(damage: number, timeMs: number): void {
    if (this.conversionPercentValue === 0 || damage <= 0) return;
    if (timeMs - this.fightStartedAtMs > this.conversionUntilMs) return;
    // **Der Faktor aus `convertBuffs`** (QUICKSILVER, E10). Gelesen bei jedem
    // Schaden, solange die Umwandlung läuft — dieselbe Lage wie oben.
    const satz = Math.round(
      this.conversionPercentValue * this.convertBuffs.factor(this.nowMsForBuffs),
    );
    const gewonnen = Math.floor((damage * satz) / 100);
    if (gewonnen <= 0) return;
    this.shieldValue += gewonnen;
    this.convertedValue += gewonnen;
  }

  /**
   * Lässt die Schutzschicht und die Umwandlung ablaufen.
   *
   * Steht bei den passiven Wirkungen, weil beides allein an der Uhr hängt —
   * wie die Energieleiste des Bosses.
   */
  /**
   * Legt Zersetzungs-Stapel an (PUTREFACTION, GAME-DESIGN 4.1).
   *
   * **Die Obergrenze verbraucht nichts.** Eine Wirkung über der Grenze erhöht
   * nichts und nimmt auch nichts weg — sie erneuert insbesondere nicht die Uhr
   * des ältesten Stapels. Sonst hielte man den Aufbau ewig oben, ohne je
   * nachzulegen, und „verfallen langsam" wäre wirkungslos.
   */
  private applyPutrefaction(index: number, spell: SpellDef, timeMs: number): void {
    const decay = spell.decay;
    if (decay === undefined) return;
    this.addDecayStacks(index, decay.stacks, timeMs);
  }

  /**
   * Stapel legen — die **eine** Stelle, an der das geschieht (Issue #498).
   *
   * Bis ROT gab es nur einen Weg: beim Wirken, über `spell.decay.stacks`. Ein
   * Effekt über Zeit legt sie jetzt auch, über `dot.decayPerTick`. Beide Wege
   * teilen sich diese Funktion, damit Obergrenze, Verfallszeit und die
   * Neuberechnung der Verwundbarkeit **nicht zweimal** dastehen.
   *
   * Genau daran ist die Schule schon einmal gescheitert: `perDecayStackPercent`
   * wird an einer Stelle gelesen und an keiner angewandt (#484). Eine zweite
   * Kopie dieser Schleife wäre dieselbe Klasse Fehler mit einer anderen Zahl.
   */
  private addDecayStacks(quelle: number, anzahl: number, timeMs: number): void {
    // **Der Aufbau-Weg wirkt HIER, nicht an den zwei Aufrufern** (#419,
    // PUTREFACTION). `decay.stacks` und `dot.decayPerTick` laufen beide durch
    // diese Stelle; den Faktor dort zweimal anzuwenden hieße, ihn bei einem
    // dritten Weg zu vergessen.
    //
    // # Der Rest wird gesammelt, nicht weggeworfen
    //
    // Hier stand bis zum 2026-08-13 `Math.floor(anzahl * faktor)`, und das war
    // **gemessen wirkungslos**: Alle drei Quellen legen genau EINEN Stapel
    // (canker 1, blight 1, rot 1), und bei Basis 1 ändert ein Faktor erst ab
    // 2,0 etwas:
    //
    // ```text
    //   floor(1 × 1,3) = 1     floor(1 × 1,5) = 1     floor(1 × 2,0) = 2
    // ```
    //
    // `putrefaction-build` trägt bei vollem Ausbau 1,5 — der Weg tat also
    // nichts. Aufrunden verbietet sich (jeder erste Rang schenkte sofort einen
    // Stapel), deshalb sammelt der Kampf jetzt den Rest: Was am ganzen Stapel
    // fehlt, wartet auf den nächsten Wurf. Bei Faktor 1,5 legt jeder zweite
    // Wurf einen zusätzlichen.
    //
    // **Ganzzahlig, nicht in Fließkomma** (18.4). Der Rest läuft über den
    // ganzen Kampf mit; zwei Umsetzungen, die ihn in `f64` führen, laufen
    // irgendwann in der letzten Stelle auseinander, und Bit-Parität ist nicht
    // „fast gleich".
    //
    // **Je Quelle, nicht global.** Sonst schöbe ein Zauber den Rest eines
    // anderen über die Schwelle, und die Reihenfolge im Kampf entschiede über
    // den Stapel — planbar wäre das nicht mehr (Gesetz 6).
    if (anzahl > 0) {
      const gesamt =
        anzahl * (1000 + talentDecayStackPermille(this.talents)) + (this.decayRest[quelle] ?? 0);
      anzahl = Math.floor(gesamt / 1000);
      this.decayRest[quelle] = gesamt - anzahl * 1000;
    }
    if (anzahl <= 0) return;
    const seitBeginn = this.fightElapsedMs(timeMs);
    for (let i = 0; i < anzahl; i++) {
      if (this.decayCountValue >= this.decayCap) break;
      // Ans Ende, weil der neue am spätesten verfällt — die Liste bleibt damit
      // aufsteigend, ohne je zu sortieren.
      // **Der Haltbarkeits-Weg** (#419). Er verlängert die Uhr JEDES neu
      // gelegten Stapels; liegende bleiben, wie sie sind — sonst würde ein
      // Talent rückwirkend wirken.
      this.decayExpiryMs[this.decayCountValue] =
        seitBeginn + Math.floor(DECAY_STACK_DURATION_MS * talentDecayDurationFactor(this.talents));
      this.decayCountValue++;
    }
    this.updateVulnerability();
  }

  /**
   * Schreibt den Endpunkt des Nachlaufs fort (#419, `KEYSTONE_DECAY_ECHO`).
   *
   * **Nur für Brände, die überhaupt Stapel legen.** Ein Brand ohne
   * `decayPerTick` bekommt keinen Endpunkt und damit keinen Nachlauf — sonst
   * schaltete die Schleife nach seinem Ende einen Tick-Zeiger weiter, der
   * nichts bewirkt.
   *
   * **Der jüngste Stapel gibt das Ende**, nicht der älteste: Er ist der letzte,
   * der vergeht. Liegt gerade keiner, gibt es nichts nachzuhalten.
   */
  private merkeDecayEcho(index: number, decayPerTick: number): void {
    if (decayPerTick <= 0) return;
    this.decayEchoUntilMs[index] =
      this.decayCountValue > 0
        ? (this.decayExpiryMs[this.decayCountValue - 1] ?? NOT_ACTIVE)
        : NOT_ACTIVE;
  }

  /**
   * Ob ein erloschener Brand noch nachläuft — und räumt ihn ab, wenn nicht.
   *
   * **Hier endet die Selbstverstärkung.** Der Endpunkt stammt aus der Lage beim
   * letzten lebenden Tick; was der Nachlauf selbst legt, verschiebt ihn nicht,
   * weil `merkeDecayEcho` nur im lebenden Zweig aufgerufen wird. Ohne diese
   * Trennung hielte ein einziger Brand seine Stapel für immer: Jeder neue
   * Stapel schöbe die Frist vor sich her, und die Zersetzung liefe ohne
   * weiteren Tastendruck endlos weiter.
   */
  private imDecayEcho(index: number, timeMs: number): boolean {
    if (!this.hasKeystone(KEYSTONE_DECAY_ECHO)) return false;
    const bis = this.decayEchoUntilMs[index] ?? NOT_ACTIVE;
    if (bis === NOT_ACTIVE) return false;
    // Auch die Stapel selbst müssen noch liegen: Wer sie anderweitig verliert
    // (Phasenwechsel, Verzehr), verliert den Nachlauf mit ihnen.
    if (this.decayCountValue <= 0 || this.fightElapsedMs(timeMs) > bis) {
      this.decayEchoUntilMs[index] = NOT_ACTIVE;
      return false;
    }
    return true;
  }

  /**
   * Lässt abgelaufene Stapel verfallen — **den ältesten zuerst**.
   *
   * Die Liste steht aufsteigend, also stehen die abgelaufenen vorn. Was bleibt,
   * rückt auf; die jüngeren behalten ihre eigene Restzeit.
   */
  /**
   * Abgelaufene Verstärkungen wegräumen und den Faktor nachziehen (#420).
   *
   * **Die zweite Zeile ist der ganze Punkt.** `buffs.expire` räumt nur den
   * eigenen Zustand; ohne das Nachziehen bliebe `factors.buffs` auf dem alten
   * Wert stehen, und eine abgelaufene Verstärkung wirkte für den Rest des
   * Kampfes weiter. Der Prüffall dazu misst deshalb den Schaden **nach** dem
   * Ablauf, nicht nur währenddessen — ein Messpunkt innerhalb des Fensters
   * sähe genau gleich aus.
   */
  private expireBuffs(timeMs: number): void {
    // **Alle fünf, nicht nur die Schadens-Verstärkungen** (Issue #489). Ein
    // Fenster, das nie abläuft, wirkt für den Rest des Kampfes weiter — und
    // bei den vier neuen Größen fiele das nicht am Schaden auf.
    this.logExpired(this.controlBuffs.expire(timeMs), timeMs);
    this.logExpired(this.resourceBuffs.expire(timeMs), timeMs);
    this.logExpired(this.tickBuffs.expire(timeMs), timeMs);
    this.logExpired(this.stackBuffs.expire(timeMs), timeMs);
    this.logExpired(this.cdReduceBuffs.expire(timeMs), timeMs);
    this.logExpired(this.convertBuffs.expire(timeMs), timeMs);
    this.logExpired(this.capBuffs.expire(timeMs), timeMs);

    // **Die achte, und sie liest ihren Rückgabewert schon immer.** `=== 0`
    // heisst weiterhin „nichts abgelaufen" — die Maske hat dieselbe Null wie
    // die frühere Anzahl, deshalb bleibt der Vergleich richtig (#682).
    const abgelaufen = this.buffs.expire(timeMs);
    this.logExpired(abgelaufen, timeMs);
    if (abgelaufen === 0) return;
    this.updateBuffFactor(timeMs);
  }

  /**
   * Schreibt je abgelaufener Verstärkung einen Eintrag der Art 7 (Issue #682).
   *
   * # Warum hier und nicht in `Buffs.expire`
   *
   * `expire` läuft je Takt und darf **nichts erzeugen** — das steht über der
   * Methode und ist der Grund, warum sie eine Zahl zurückgibt statt einer
   * Liste. Die Klasse kennt das Protokoll nicht und soll es nicht kennen.
   * Also liest der Aufrufer die Maske und schreibt hier.
   *
   * # Ein Eintrag JE ART, nicht einer je Sammlung
   *
   * Acht Sammlungen rufen `expire` auf. Ein Eintrag je Aufruf trüge eine
   * Zahl, die von der Sammlung abhängt statt vom Ereignis — wer den Verlauf
   * liest, sähe „drei liefen aus" und wüsste nicht, welche. Der Wert ist
   * deshalb die ART, und die bewegt sich: Ein Eintrag, der immer dieselbe
   * Zahl trägt, ist von einem festen Wert nicht zu unterscheiden.
   */
  private logExpired(maske: number, timeMs: number): void {
    if (maske === 0 || this.log === null) return;
    for (let kind = 0; kind < BUFF_KIND_CAPACITY; kind++) {
      if ((maske & (1 << kind)) === 0) continue;
      this.log.push(this.fightElapsedMs(timeMs), EVENT_BUFF_EXPIRE, NO_SPELL_REF, kind);
    }
  }

  /** Trägt den Gesamtfaktor der laufenden Verstärkungen in die Rechnung. */
  /** Welcher Speicher zu einer Zielgröße gehört (Issue #489). */
  private buffSpeicherFuer(ziel: BuffDef['target']): Buffs {
    switch (ziel) {
      case 'control':
        return this.controlBuffs;
      case 'resource':
        return this.resourceBuffs;
      case 'tick':
        return this.tickBuffs;
      case 'stack':
        return this.stackBuffs;
      case 'cdreduce':
        return this.cdReduceBuffs;
      case 'convert':
        return this.convertBuffs;
      case 'cap':
        return this.capBuffs;
      default:
        // `damage` und `undefined` — so waren alle Verstärkungen vor #489
        // gemeint, und ein fehlendes Feld soll weiter das Alte tun.
        return this.buffs;
    }
  }

  private updateBuffFactor(timeMs: number): void {
    this.setDamageFactor('buffs', this.buffs.factor(timeMs));
  }

  /**
   * Läuft gerade irgendein DoT auf dem Ziel?
   *
   * **Als eigene Abfrage und nicht inline**, weil `SLOW_ROT` sie braucht und
   * die Bedingung sonst an der Verfallsstelle stünde — dort, wo sie beim
   * nächsten Umbau übersehen wird. `dotRemainingMs` beantwortet dieselbe Frage
   * je Zauber; hier geht es um „mindestens einer".
   */
  private hasRunningDot(timeMs: number): boolean {
    for (let i = 0; i < this.dotExpiresAtMs.length; i++) {
      const expires = this.dotExpiresAtMs[i] ?? NOT_ACTIVE;
      if (expires !== NOT_ACTIVE && expires > timeMs) return true;
    }
    return false;
  }

  private expirePutrefaction(timeMs: number): void {
    // **SLOW ROT** (13.5, PUTREFACTION-Sechser): Solange ein DoT läuft,
    // verfällt kein Zersetzungs-Stapel.
    //
    // **Aufgeschoben, nicht aufgehoben.** Die Verfallszeiten bleiben stehen;
    // endet der letzte DoT, holt der nächste Durchlauf alles nach, was
    // inzwischen fällig geworden ist. Ein Zurücksetzen der Zeiten wäre eine
    // Verlängerung und damit ein zweiter Effekt.
    //
    // Der Aufbau vor dem Schadensfenster wird damit zur Kernentscheidung statt
    // zum Wettlauf.
    if (this.hasSetEffect(SET_EFFECT_SLOW_ROT) && this.hasRunningDot(timeMs)) return;
    const seitBeginn = this.fightElapsedMs(timeMs);
    let verfallen = 0;
    while (verfallen < this.decayCountValue && (this.decayExpiryMs[verfallen] ?? 0) <= seitBeginn) {
      verfallen++;
    }
    if (verfallen === 0) return;
    for (let i = verfallen; i < this.decayCountValue; i++) {
      this.decayExpiryMs[i - verfallen] = this.decayExpiryMs[i] ?? 0;
    }
    this.decayCountValue -= verfallen;
    this.updateVulnerability();
  }

  /**
   * Räumt ALLE Fäulnis-Stapel ab — der Entlader der Schule (NECROSIS,
   * Issue #479).
   *
   * # Warum das kein neues Zustandsfeld braucht
   *
   * Die Stapel sind ein **Boss-Zustand**, kein Spieler-Guthaben: Sie stehen
   * schon in `decayCountValue` und damit in der Prüfsumme. Der erste Entwurf
   * ging über `spender` — den Weg, den Ressourcen nehmen — und scheiterte an
   * `sim/build.rs:706`, weil PUTREFACTION überhaupt keine Ressource führt.
   * Der Abbruch war richtig: Er hat eine Mechanik verhindert, die es an der
   * Stelle nicht gibt.
   *
   * # Warum hier nichts verschoben wird
   *
   * Anders als beim Verfall (`expirePutrefaction`) fallen nicht die ältesten
   * Stapel weg, sondern alle. Die Ablaufzeiten in `decayExpiryMs` bleiben
   * stehen und werden nie gelesen, solange `decayCountValue` auf 0 steht —
   * jede Schleife darüber läuft gegen die Zählung, nicht gegen die Länge des
   * Feldes.
   */
  private consumeDecayStacks(): void {
    if (this.decayCountValue === 0) return;
    this.decayCountValue = 0;
    this.updateVulnerability();
  }

  /**
   * Rechnet die Verwundbarkeit aus den liegenden Stapeln.
   *
   * **Über `setDamageFactor`** (Issue #196): Nur so zieht die vorab angelegte
   * Krit-Fassung mit. Direkt auf `this.factors` zu schreiben ließe jeden
   * kritischen Treffer mit dem alten Wert rechnen — der Quelltext-Wächter aus
   * #196 fängt das, und genau dafür wurde er gebaut.
   *
   * **Ganzzahlig bis zur letzten Teilung.** `(100 + n · p) / 100` ist auf
   * beiden Seiten dieselbe Rechnung; `1 + n · p / 100` wäre es nicht zwingend
   * (18.4).
   */
  /**
   * Den Synergie-Faktor setzen, wenn diesem Zauber ein Fenster offen steht
   * (GAME-DESIGN 5, Issue #318).
   *
   * # Der Faktor ist additiv im Topf „aktive Verstärkungen"
   *
   * GAME-DESIGN 1193 ordnet Synergien demselben Topf zu wie Overdrive und die
   * Zersetzungs-Stapel, „additiv untereinander". Zwei offene Fenster addieren
   * ihre Boni zu EINEM Faktor, statt übereinander zu multiplizieren — das ist
   * die Stapel-Regel aus 327, angewandt.
   *
   * # Der Bonus liegt NEBEN dem Stapel-Bonus, nicht darin
   *
   * Eine Gegenprobe, die den Stapelstand mitvariiert, misst die Stapel und
   * nicht die Synergie: `vulnerability` hebt den Schaden **aller** Quellen,
   * sobald Stapel liegen (`updateVulnerability`).
   *
   * **Hier stand bis Issue #484 zusätzlich, FESTER werde „ohnehin stärker, je
   * mehr Zersetzungs-Stapel liegen (`perDecayStackPercent`)".** Das war falsch,
   * und zwar nicht nur ungenau: Das Feld wird geladen, geprüft und für die
   * Zuordnung gelesen (`indicators.ts`, `mechanicId === 'decay'`) — aber auf
   * keiner der beiden Seiten je auf den Schaden angewandt. Es gibt kein
   * „neben", weil es nichts gibt, wozu es danebenläge.
   *
   * Der Satz war gefährlich, weil er an genau der Stelle stand, an der jemand
   * nachsieht — und die allgemeine Verwundbarkeit seinen Ausfall maskiert:
   * „mit Stapeln macht FESTER mehr Schaden" ist wahr, kommt aber aus der
   * anderen Mechanik.
   */
  private applySynergyBonus(index: number, timeMs: number): void {
    const seitBeginn = this.fightElapsedMs(timeMs);
    const endet = this.synergyWindowEndsMs[index] ?? NOT_ACTIVE;
    const offen = endet !== NOT_ACTIVE && seitBeginn <= endet;
    // **Ganzzahlig bis zur letzten Teilung** (GAME-DESIGN 18.4): (100 + n) / 100
    // und nicht 1 + n / 100 — sonst hängt das Ergebnis an der Reihenfolge der
    // Gleitkomma-Schritte, und die beiden Seiten laufen auseinander.
    this.setDamageFactor('synergy', offen ? (100 + SYNERGY_BONUS_PERCENT) / 100 : 1);
    if (offen) {
      this.synergyUsedCount++;
      // **Das Fenster ist verbraucht.** Sonst zählte jede weitere Wirkung im
      // selben Fenster erneut als genutzte Synergie, und die Ausbeute stiege
      // über die Zahl der Gelegenheiten.
      this.synergyWindowEndsMs[index] = NOT_ACTIVE;
    }
  }

  /**
   * Das Fenster für den Partner öffnen (Issue #318).
   *
   * # Die Länge steht in WARTEZEITEN, nicht in Millisekunden
   *
   * `windowGcds` ist eine Anzahl; die Millisekunden entstehen hier, durch
   * dieselbe `hasted()`-Stelle wie jede andere Zeit. Damit schrumpft das
   * Fenster bei hohem CATALYSIS mit — GAME-DESIGN 327: „In Wartezeiten
   * gerechnet, nicht in Sekunden, sonst verschiebt sich das Verhältnis bei
   * hohem Tempo."
   *
   * ## Die Klammerung ist der ganze Unterschied (Issue #428)
   *
   *     gcds × hasted(GCD_MS)     n echte Wartezeiten      <- richtig
   *     hasted(gcds × GCD_MS)     eine beschleunigte Dauer <- war hier
   *
   * Beides liest sich wie „das Fenster in Wartezeiten", und dieser Kommentar
   * behauptete es auch — nur rechnete die Zeile darunter das andere. `hasted`
   * rundet **einmal** ab (`Math.floor`); wer erst multipliziert, verliert die
   * Rundung der einzelnen Wartezeit und landet bis zu 10 ms zu tief.
   *
   * Zehn Millisekunden sind genau ein Takt — und damit die Grenze zwischen
   * „der zweite Zauber ist noch drin" und „er ist draußen". Aus zwei Zaubern
   * wurde bei Tempo einer, also genau die Verschiebung, die der Design-Satz
   * verbietet.
   *
   * Bei 0, 376 und 500 ‰ ergeben beide Klammerungen dasselbe — dort trifft
   * sich die Rundung zufällig. Ein Prüffall an einem dieser Werte wäre grün,
   * ohne etwas zu belegen; deshalb misst `synergy-window.test.ts` bei 100 und
   * 300 ‰.
   *
   * # Warum `synergyAvailable` an der Bereitschaft hängt
   *
   * Ein Fenster, das aufgeht, während der Partner auf Abklingzeit liegt und
   * auch bis zum Ende nicht bereit wird, ist **keine Gelegenheit**. Zählte es
   * mit, sänke die Ausbeute durch etwas, das der Spieler gar nicht hätte
   * nutzen können — die Zahl bestrafte ihn für eine Abklingzeit statt für sein
   * Spiel.
   */
  private openSynergyWindow(spell: SpellDef, timeMs: number): void {
    const synergy = spell.synergy;
    if (synergy === undefined) return;
    const partner = this.indexOf(synergy.partner);
    if (partner < 0) return;
    const seitBeginn = this.fightElapsedMs(timeMs);
    const laenge = synergy.windowGcds * this.hasted(GCD_MS);
    this.synergyWindowEndsMs[partner] = seitBeginn + laenge;
    // **Wird der Partner im Fenster überhaupt bereit?** `cooldownReadyAtMs`
    // steht in Kampfzeit wie `timeMs`, das Fensterende in Zeit seit
    // Kampfbeginn — deshalb die Differenz und nicht der rohe Vergleich.
    const bereitAb = this.cooldownReadyAtMs[partner] ?? 0;
    if (bereitAb - timeMs <= laenge) this.synergyAvailableCount++;
  }

  /** Wie oft ein Fenster offen stand, das der Partner hätte nutzen können. */
  get synergyAvailable(): number {
    return this.synergyAvailableCount;
  }

  /** Wie oft der Partner im offenen Fenster gewirkt wurde. */
  get synergyUsed(): number {
    return this.synergyUsedCount;
  }

  /**
   * Die Summe aller offenen Fensterenden — für die Prüfsumme (Issue #318).
   *
   * **Eine Summe statt einer Schleife**, weil die Prüfsumme je Takt läuft und
   * GAME-DESIGN 18.2 dort keine Erzeugung erlaubt. Sie unterscheidet dieselben
   * Zustände wie die einzelnen Werte: Ein Fenster, das anderswo endet, ändert
   * die Summe.
   *
   * Nicht offene Fenster tragen 0 und nicht `NOT_ACTIVE` — sonst hinge die
   * Summe an einer Sentinel-Zahl, die auf den beiden Seiten verschieden sein
   * dürfte.
   */
  /**
   * Die Fenstersummen ALLER Verstärkungen, für die Prüfsumme (Issue #489).
   *
   * **Alle fünf zusammen in einer Zahl**, nicht fünf Felder: Die Prüfsumme
   * unterscheidet dieselben Zustände, und jedes weitere Feld verschiebt die
   * Bit-Folge auf beiden Seiten. Ein Fenster, das anderswo endet, ändert die
   * Summe — gleich in welcher Zielgröße.
   */
  get buffWindowSum(): number {
    return (
      this.buffs.windowSum() +
      this.controlBuffs.windowSum() +
      this.resourceBuffs.windowSum() +
      this.tickBuffs.windowSum() +
      this.stackBuffs.windowSum() +
      this.cdReduceBuffs.windowSum() +
      this.convertBuffs.windowSum() +
      this.capBuffs.windowSum()
    );
  }

  get synergyWindowSum(): number {
    let summe = 0;
    for (let i = 0; i < this.synergyWindowEndsMs.length; i++) {
      const endet = this.synergyWindowEndsMs[i] ?? NOT_ACTIVE;
      if (endet !== NOT_ACTIVE) summe += endet;
    }
    return summe;
  }

  /**
   * Rechnet die Verwundbarkeit aus den liegenden Stapeln.
   *
   * # Ihr Ergebnis erreicht heute KEINE Schadensrechnung
   *
   * Gemessen mit einer Mutation: Setzt man hier `99` statt der Rechnung,
   * werden genau **zwei** Fälle rot — und beide lesen den Getter
   * `combat.vulnerability`, keiner rechnet Schaden damit
   * (`putrefaction.test.ts`, `rot-decay-per-tick.test.ts`). Dieselbe Mutation
   * in `applyDecayStackBonus` färbt 214 Fälle.
   *
   * **Die 214 altert, die 2 nicht.** Beim Schreiben waren es 210; jeder neue
   * Prüffall hebt sie. Die 2 dagegen sind namentlich bekannt und bleiben, bis
   * jemand einen dritten Leser des Getters baut.
   *
   * Der Grund ist die Reihenfolge: `applyDecayStackBonus` setzt dasselbe Feld
   * **unbedingt neu** und rechnet es von Grund auf, und sie läuft vor jeder
   * Schadensrechnung. Der hier gesetzte Wert steht nur zwischen einem
   * Stapel-Ereignis und dem nächsten Wurf, und in dieser Zeit fällt kein
   * Schaden.
   *
   * # Warum sie trotzdem bleibt
   *
   * Sie hält den Getter aktuell, den die zwei Prüffälle lesen. Sie zu
   * entfernen wäre ein Eingriff auf beiden Seiten für null Gewinn —
   * entschieden nach der Messung, nicht nach Gefühl (Issue #480). Der
   * Simulator trägt denselben Hinweis an `update_vulnerability`.
   *
   * **Wer hier etwas ändert, ändert nichts am Schaden.** Wer den Schaden
   * treffen will, ändert `applyDecayStackBonus`.
   */
  private updateVulnerability(): void {
    // **Der Faktor aus `stackBuffs`** (Issue #489, VIRULENCE): Er ändert, was
    // ein Stapel WERT ist, nicht wie viele liegen. Ganzzahlig bis zur letzten
    // Teilung (18.4) — deshalb auf die Prozentpunkte, nicht auf das Ergebnis.
    const jeStapel = Math.round(
      DECAY_STACK_PERCENT *
        this.stackBuffs.factor(this.nowMsForBuffs) *
        talentDecayEffectFactor(this.talents),
    );
    this.setDamageFactor('vulnerability', (100 + this.decayCountValue * jeStapel) / 100);
  }

  /**
   * Die Verwundbarkeit **für die Wirkung eines bestimmten Zaubers** — mit
   * seinem eigenen Zuschlag je Stapel (`perDecayStackPercent`, Issue #484).
   *
   * # Warum ADDITIV im selben Topf und nicht als eigener Faktor
   *
   * GAME-DESIGN 1405 führt die Zersetzungs-Stapel ausdrücklich im additiven
   * Topf („Aktive Verstärkungen: Overdrive, Synergien, Zersetzungs-Stapel —
   * additiv untereinander"). Ein zauber-eigener Zuschlag auf diese Stapel
   * gehört in denselben Topf.
   *
   * Die Begründung dort ist die stärkere: *„Was multiplikativ ist, wird
   * gejagt."* Ein neunter Faktor nur für einen Zauber wäre der Fehler, den
   * Diablo IV mit der Verwundbarkeit gemacht und in Saison 2 zurückgenommen
   * hat. Gemessen für FESTER bei fünf Stapeln: additiv 2,15 gegen
   * multiplikativ 2,30 — bei zehn Stapeln wächst der Abstand.
   *
   * # Warum JE WIRKUNG und nicht je Kampf
   *
   * Der Verwundbarkeits-Topf ist **global** — er hebt den Schaden aller
   * Quellen. `perDecayStackPercent` gilt aber für **einen** Zauber. Deshalb
   * wird er hier vor der Rechnung gesetzt und beim nächsten Zauber wieder
   * überschrieben, genau wie `resistance` (Issue #328). Wer ihn dauerhaft in
   * `updateVulnerability` schriebe, gäbe den Zuschlag allen anderen mit.
   */
  /**
   * Die Schutzschicht länger halten (HARDEN, Issue #523).
   *
   * # Verlängern ist nicht Neusetzen
   *
   * Der Ablauf rückt nach hinten, `shieldValue` bleibt unangetastet. Ein
   * Schild, der schon die Hälfte abgefangen hat, behält seinen Rest — wer ihn
   * neu setzte, machte aus einem Verlängerer einen zweiten CRUST.
   *
   * # Es gibt hier KEINEN Takt, den man mitdenken müsste
   *
   * Bei den Bränden musste `dotNextTickAtMs` beim Verlängern unberührt bleiben
   * (#517) — sonst brennt ein Brand länger und tickt seltener. Der Schild hat
   * keine Entsprechung: Er hält nur einen Betrag und einen Ablauf, und beim
   * Ablauf wird der Betrag genullt.
   *
   * **Das steht hier als Satz, damit das Fehlen nicht wie ein Versehen
   * aussieht.** Wer #517 kennt, sucht sonst nach der Zeile, die es nicht gibt.
   *
   * Ein abgelaufener oder nie gesetzter Schild wird NICHT wiederbelebt: Ohne
   * laufende Schicht gibt es nichts zu verlängern.
   */
  extendShield(timeMs: number, ms: number): void {
    if (ms <= 0) return;
    const seitBeginn = this.fightElapsedMs(timeMs);
    if (this.shieldExpiresAtMs < 0 || seitBeginn > this.shieldExpiresAtMs) return;
    if (this.shieldValue <= 0) return;
    this.shieldExpiresAtMs += ms;
  }

  /**
   * Die Zersetzungs-Stapel länger halten (SEPSIS, Issue #523).
   *
   * # WELCHE Stapel — die Antwort folgt aus der Struktur, nicht aus Geschmack
   *
   * **Alle, und um denselben Betrag.** Das ist keine Vorliebe, sondern die
   * einzige Wahl, die die Ordnung erhält:
   *
   * `decayExpiryMs` ist **aufsteigend sortiert**, und das Nachrücken beim
   * Verfall setzt das voraus (siehe die Schleife in `expireDecayStacks`: sie
   * zählt von vorn, solange der vorderste abgelaufen ist). Verlängerte man nur
   * den ältesten oder nur den jüngsten, stünde ein späterer Ablauf vor einem
   * früheren — und dann verfallen Stapel in der falschen Reihenfolge, **ohne
   * dass die Stapelzahl es verrät**.
   *
   * Genau diese Klasse hat der ROT-Prüffall aus #498 gefangen: Die Summe
   * stimmt, die Abläufe nicht.
   *
   * Ein gleicher Zuschlag auf alle erhält die Ordnung dagegen zwingend — die
   * Differenzen zwischen den Einträgen bleiben unverändert.
   *
   * # Auch hier gibt es keinen Takt
   *
   * Die Stapel ticken nicht; sie liegen und verfallen. Die
   * `dotNextTickAtMs`-Falle aus #517 hat keine Entsprechung.
   */
  extendDecayStacks(ms: number): void {
    if (ms <= 0) return;
    for (let i = 0; i < this.decayCountValue; i++) {
      this.decayExpiryMs[i] = (this.decayExpiryMs[i] ?? 0) + ms;
    }
  }

  /**
   * Der Auswahlmodus des Cooldown-Rücksetzers (Issue #528).
   *
   * # Warum die Regel HIER liegt und nicht mehr in `Session`
   *
   * Sie lag bis #528 in `src/core/session.ts` — im Spiel eine Schicht über
   * ihrer eigenen Wirkung, im Simulator dagegen im Kampf (`fight.rs`,
   * `reset_mode`). Die Reihenfolge war auf beiden Seiten sorgfältig
   * gleichgehalten, die **Schicht** nicht.
   *
   * # Was hier stand, war beim Schreiben schon falsch (berichtigt in #528 AC5)
   *
   * Der Satz lautete: „Jede Aufzeichnung fährt `Combat` direkt — `new Session`
   * kommt in keiner vor." Gemessen in `tests/parity/payloads.ts`:
   *
   * ```text
   * new Combat(    36
   * new Session(    2      resetRunPayload, cooldownResetPayload
   * ```
   *
   * Die ältere der beiden stammt vom 2026-08-05, dieser Kommentar vom
   * 2026-08-07 — er war zwei Tage zu spät und trotzdem falsch.
   *
   * **Und die Schlussfolgerung trägt nicht.** Eine Regel ausserhalb von
   * `Combat` ist NICHT grundsätzlich unsichtbar — sie ist es, solange keine
   * Aufzeichnung sie fährt. Gemessen: Wird der `armResetIfNeeded`-Aufruf aus
   * `alembicPayload` entfernt und neu exportiert, fallen 4 von 5 Fällen in
   * `sim/tests/parity_alembic.rs`, darunter die Prüfsumme je Takt.
   *
   * Der Rücksetzer war unsichtbar, WEIL ihn keine Aufzeichnung fuhr — nicht,
   * weil er hier lag. Der Umzug war trotzdem richtig: Er macht die Regel dort
   * prüfbar, wo sie wirkt. Nur die Begründung war es nicht.
   *
   * Der Bruch fiel auf, als jemand REFLUX in eine Aufzeichnung nehmen wollte.
   *
   * Und die Wirkung tritt beim **nächsten gewirkten Zauber** ein — das ist ein
   * Kampfereignis, kein Sitzungsereignis (WORKER-GUIDE 8.38: der Zustand
   * gehört dorthin abgelesen, wo die Wirkung eintritt).
   */
  private resetArmedForValue: number = -1;

  /**
   * Wie viele Auswahlen noch offen sind (Issue #521).
   *
   * # Warum ein Zähler neben dem Index und nicht statt seiner
   *
   * Der Index sagt, WER scharf gestellt hat — das braucht die Prüfung „der
   * Rücksetzer wählt sich nicht selbst" (AC4), und zwar bei **jeder** der
   * Auswahlen, nicht nur bei der ersten. Der Zähler sagt, wie oft noch
   * gewählt werden darf. Zwei Fragen, zwei Felder.
   */
  private resetChargesLeftValue = 0;

  /** Steht der Auswahlmodus scharf? Für Anzeige und Prüfung. */
  get resetArmed(): boolean {
    return this.resetArmedForValue >= 0;
  }

  /** Welcher Rücksetzer scharf steht, oder -1. */
  get resetArmedFor(): number {
    return this.resetArmedForValue;
  }

  /**
   * Der Zauber mit der längsten Restzeit — ohne den Rücksetzer selbst.
   *
   * `-1`, wenn keiner auf Abklingzeit steht. **Bei Gleichstand der kleinere
   * Platz**, damit beide Seiten dieselbe Wahl treffen: Der Simulator läuft
   * über dieselbe Reihenfolge, und „irgendeiner davon" wäre der erste Riss in
   * der Bitgleichheit.
   *
   * Der Rücksetzer selbst bleibt außen vor — er steht in dem Moment gerade
   * frisch auf Abklingzeit und wäre sonst regelmäßig sein eigenes Ziel.
   */
  private laengsteRestzeit(ausser: number, timeMs: number): number {
    let ziel = -1;
    let laengste = 0;
    for (let i = 0; i < this.spells.length; i++) {
      if (i === ausser) continue;
      const rest = this.cooldownRemainingMs(i, timeMs);
      if (rest > laengste) {
        laengste = rest;
        ziel = i;
      }
    }
    return ziel;
  }

  /** Wie viele Auswahlen noch offen sind (Issue #521). */
  get resetChargesLeft(): number {
    return this.resetChargesLeftValue;
  }

  /**
   * Nach einer Wirkung scharf stellen, wenn der Zauber ein Rücksetzer ist.
   *
   * **NACH der Wirkung, nie davor** — sonst wählte sein eigener Druck bereits,
   * und der Rücksetzer würde nie gewirkt.
   */
  armResetIfNeeded(index: number, timeMs: number): void {
    const ladungen = this.spells[index]?.cooldownResetCharges ?? 0;
    if (ladungen <= 0) return;

    // **SUBLIMATIONs Schlüsseltalent** (#419): Der Rücksetzer wählt nicht
    // mehr, sondern trifft die längste Restzeit von selbst.
    //
    // Wie bei `split-spender` steht hier eine Verzweigung und kein Faktor: Es
    // ändert sich, WAS geschieht — der Auswahlmodus entfällt, und mit ihm der
    // zusätzliche Tastendruck. Ein Talent, das sich als Zahl ausdrücken ließe,
    // wäre kein Schlüsseltalent.
    //
    // **Die Ladungen fallen mit weg**, und das ist Absicht: Sie zählen
    // Auswahlen, und es gibt keine mehr. Ein Zuschlag auf etwas, das nicht
    // stattfindet, wäre eine Zahl ohne Wirkung — genau das, was #418 an einem
    // Schlüsseltalent ausschließt.
    if (this.hasKeystone(KEYSTONE_AUTO_RESET)) {
      const ziel = this.laengsteRestzeit(index, timeMs);
      if (ziel >= 0) this.clearCooldown(ziel);
      return;
    }

    this.resetArmedForValue = index;
    // **Überschreiben, nicht aufaddieren.** Wer einen zweiten Rücksetzer
    // wirkt, während der erste noch scharf steht, hat den ersten verschenkt —
    // dieselbe Regel wie bisher, nur jetzt mit Zahl. Aufaddieren wäre eine
    // Vorratshaltung, die es in diesem Spiel nirgends gibt.
    // **Der Zuschlag greift erst hier, nicht bei der Prüfung oben.** Wer ihn
    // in `ladungen` zöge, machte aus jedem Zauber einen Rücksetzer, sobald ein
    // Rang verteilt ist — `ladungen <= 0` wäre dann nie wahr.
    //
    // `Perpetual Draft` ist der dritte Summand und ausdrücklich kein vierter
    // Rechenweg (Auftraggeber, 2026-08-13): eine Summe mit drei Speisungen —
    // Zauber-Eigenschaft, Talent, Satz-Effekt —, nicht drei Rechnungen. Er
    // steht deshalb hier und nicht in einer eigenen Abfrage weiter unten.
    this.resetChargesLeftValue =
      ladungen +
      talentCooldownChargesBonus(this.talents) +
      (this.hasSetEffect(SET_EFFECT_PERPETUAL_DRAFT) ? PERPETUAL_DRAFT_EXTRA_CHARGE : 0);
  }

  /**
   * Ein Druck im Auswahlmodus: Er WÄHLT, statt zu wirken.
   *
   * @returns ob die Wahl eine Abklingzeit zurückgesetzt hat. `false` heisst
   *   verpufft — die Ladung ist trotzdem verbraucht.
   */
  chooseForReset(index: SpellIndex, timeMs: number): boolean {
    // **Jede Wahl verbraucht eine Ladung** — auch eine falsche.
    const selbst = index === this.resetArmedForValue;
    this.resetChargesLeftValue--;
    // Erst wenn keine Ladung mehr offen ist, fällt der Auswahlmodus. Bei einer
    // einzigen Ladung ist das genau das Verhalten von vorher.
    if (this.resetChargesLeftValue <= 0) {
      this.resetArmedForValue = -1;
      this.resetChargesLeftValue = 0;
    }
    // Die Wartezeit läuft für die Auswahl wie für eine Wirkung.
    this.gcd.trigger(timeMs);
    if (selbst || this.isOffCooldown(index, timeMs)) return false;
    this.clearCooldown(index);
    return true;
  }

  /**
   * Setzt den Talent-Topf für DIESEN Zauber (GLOBAL „release", Issue #419).
   *
   * Ein Entlader — `spender` oder `consumesDecayStacks` — trägt den Zuschlag
   * des globalen Weges, jeder andere Zauber nicht.
   *
   * # Warum an jeder Wirkung neu, auch ohne Entlader
   *
   * Der Faktor ist EIN Feld für alle Zauber. Setzte ihn nur der Entlader,
   * bliebe sein Zuschlag danach stehen und färbte jeden folgenden Treffer —
   * ein Brand-Tick, ein Füller, alles. Deshalb schreibt diese Methode auch den
   * gewöhnlichen Fall zurück, genau wie `applyDecayStackBonus` daneben.
   *
   * # Addiert, nicht multipliziert
   *
   * 13.1b: innerhalb eines Topfes additiv. `talentFactor` ist `1 + Summe/1000`;
   * der Zuschlag kommt als `Faktor − 1` obendrauf und landet damit in derselben
   * Summe. Multiplizieren machte aus einem Topf zwei.
   */
  private applySpenderBonus(index: number): void {
    const spell = this.spells[index];
    const istEntlader = spell?.spender !== undefined || spell?.consumesDecayStacks === true;
    const zuschlag = istEntlader ? talentSpenderDamageFactor(this.talents) - 1 : 0;
    this.setDamageFactor('talents', talentFactor(this.talents) + zuschlag);
  }

  private applyDecayStackBonus(index: number): void {
    const proStapel = this.spells[index]?.perDecayStackPercent ?? 0;
    // **Der Faktor aus `stackBuffs` gehört auch hierher** (Issue #489). Diese
    // Methode SETZT die Verwundbarkeit neu — ohne ihn würfe sie weg, was
    // `updateVulnerability` eingerechnet hat, und eine Stapel-Verstärkung
    // verschwände beim nächsten Wirken spurlos.
    //
    // Gemessen: Genau das ist beim Zusammenführen von #484 und #489
    // aufgefallen — `buff-targets.test.ts` mass 1,06 gegen 1,06. Beide
    // Vorhaben waren für sich grün; erst zusammen zeigte sich, dass das eine
    // die Wirkung des anderen überschreibt. Kein Merge-Werkzeug fängt das:
    // Die Dateien berühren einander nicht.
    //
    // Ganzzahlig bis zur letzten Teilung (18.4) — auf die Prozentpunkte, nicht
    // auf das Ergebnis, genau wie in `updateVulnerability`.
    const jeStapel = Math.round(
      (DECAY_STACK_PERCENT + proStapel) *
        this.stackBuffs.factor(this.nowMsForBuffs) *
        talentDecayEffectFactor(this.talents),
    );
    this.setDamageFactor('vulnerability', (100 + this.decayCountValue * jeStapel) / 100);
  }

  /** Wie viele Zersetzungs-Stapel gerade auf dem Boss liegen. */
  /**
   * Die Stapel-Grenze, die JETZT gilt — für die Anzeige (PUTRID BLOOM, #479).
   *
   * Das HUD zeigt „3 / 5". Hebt der Zauber die Grenze, muss dort die GEHOBENE
   * Zahl stehen; sonst stünde der Spieler bei „6 / 5" und hielte die Anzeige
   * für kaputt statt den Zauber für wirksam.
   */
  get decayStackCap(): number {
    return this.decayCap;
  }

  get decayStacks(): number {
    return this.decayCountValue;
  }

  /**
   * Wann der Stapel an Platz `index` verfällt, in Kampfzeit. 0 für unbelegt.
   *
   * Für die Prüfsumme: Die Zahl der Stapel allein genügt nicht — zwei Läufe
   * mit gleich vielen, aber verschieden alten Stapeln laufen gleich weiter,
   * bis der erste verfällt.
   */
  decayExpiryAt(index: number): number {
    return index < this.decayCountValue ? (this.decayExpiryMs[index] ?? 0) : 0;
  }

  /** Die Verwundbarkeit, die daraus folgt. 1,0 ohne Stapel. */
  get vulnerability(): number {
    return this.factors.vulnerability;
  }

  private expireCoagulation(timeMs: number): void {
    const seitBeginn = timeMs - this.fightStartedAtMs;
    // Die Frist der Phasen-Senkung ist um, ohne dass ein Wechsel stattfand.
    if (this.phaseHoldUntilMs >= 0 && seitBeginn > this.phaseHoldUntilMs) {
      this.phaseHoldPercentValue = 0;
      this.phaseHoldUntilMs = -1;
    }
    if (this.shieldExpiresAtMs >= 0 && seitBeginn > this.shieldExpiresAtMs) {
      this.shieldValue = 0;
      this.shieldStacksValue = 0;
      this.shieldExpiresAtMs = -1;
    }
    if (this.conversionUntilMs >= 0 && seitBeginn > this.conversionUntilMs) {
      this.conversionPercentValue = 0;
      this.conversionUntilMs = -1;
    }
    // Dasselbe für den Rückschlag (RIPOSTE, #481). Ohne diese vier Zeilen
    // liefe er bis zum Ende der Runde weiter — ein Fenster, das nie zugeht.
    if (this.reflectUntilMs >= 0 && seitBeginn > this.reflectUntilMs) {
      this.reflectPercentValue = 0;
      this.reflectUntilMs = -1;
    }
  }

  private hitPlayer(raw: number, timeMs: number): void {
    if (this.diedAtMsValue >= 0) return;
    // **Einmal gerundet, am Ende** — dieselbe Regel wie bei ausgehendem
    // Schaden (GAME-DESIGN 6.3) und dieselbe Zeile wie `PlayerBase::incoming`
    // im Simulator. Zweimal zu runden ergäbe je nach Reihenfolge andere
    // Zahlen, und genau daran laufen zwei Umsetzungen auseinander.
    // WARD aus der getragenen Ausrüstung kommt zur Basis dazu (#282, AC4).
    // **Additiv, nicht multiplikativ:** Beides ist Minderung, also derselbe
    // Topf — innerhalb eines Topfes wird addiert (13.1b).
    //
    // **Hier stehen ZWEI Deckel übereinander, und sie sagen Verschiedenes**
    // (Issue #466, GAME-DESIGN „Die Deckel"):
    //
    //   wardCapPermille   750 ‰, GESETZT vom Auftraggeber. Eine Balance-Zahl.
    //                     Sie könnte auch bei 400 oder 900 stehen, ohne dass
    //                     die Formel etwas dagegen hätte. Ohne sie kann eine
    //                     hinreichend gute Ausrüstung den Spieler
    //                     unverwundbar machen — und damit verlöre die
    //                     COAGULATION-Schule ihren Gegenstand.
    //
    //   1                 eine RECHENREGEL. Über eins würde der Spieler von
    //                     jedem Treffer geheilt. Sie folgt aus der Formel und
    //                     ist nicht verhandelbar.
    //
    // Der zweite ist heute wirkungslos, weil 750 ‰ darunter liegt — und steht
    // trotzdem da. Wer den Balance-Deckel eines Tages über 1000 ‰ setzt, soll
    // nicht erst durch geheilte Spieler darauf stossen. Ihn wegzulassen hiesse,
    // sich auf den Wert einer Datendatei zu verlassen.
    // **DECAY geht in dieselbe Summe** (Issue #479) — und damit auch unter
    // denselben Deckel. Das ist die Entscheidung: Die Schwächung ist kein
    // eigener Faktor neben WARD, sondern mehr WARD. Andernfalls ließe sich der
    // Balance-Deckel (`wardCapPermille`) über die Fäulnis umgehen, und die
    // Grenze, die COAGULATION ihren Gegenstand lässt, wäre für PUTREFACTION
    // aufgehoben.
    //
    // **Die Stapelzahl wird HIER gelesen, nicht beim Wirken.** Wer nach dem
    // Wurf weiter aufbaut, mindert mehr — „nach Stapelzahl" heißt nach der
    // jetzigen, nicht nach der von damals.
    // **In EINE Promille-Summe, dann EINMAL geteilt.** `(a + b) / 1000` und
    // `a/1000 + b/1000` sind in Fliesskomma nicht dasselbe, und die Rust-Seite
    // rechnet die erste Form (`incoming_with_gear` nimmt EINEN Promille-Wert).
    // Zwei Divisionen hier ergäben eine Abweichung, die erst in der Prüfsumme
    // auffiele und wie ein Rechenfehler im Kampf aussähe.
    const seitBeginnFuerDecay = this.fightElapsedMs(timeMs);
    const decayPermille =
      this.decayWardUntilMs >= 0 && seitBeginnFuerDecay <= this.decayWardUntilMs
        ? this.decayWardPermilleValue * this.decayCountValue
        : 0;
    const roh = PLAYER.wardReduction + (this.worn.ward + decayPermille) / 1000;
    const minderung = Math.min(1, Math.min(PLAYER.wardCapPermille / 1000, roh));
    const nachWard = Math.max(0, Math.round(raw * (1 - minderung)));

    // **Die Schutzschicht nimmt den Schlag ZUERST** (COAGULATION, Weg 1,
    // Issue #79).
    //
    // Das ist die ganze Mechanik: Sie muss VOR dem Schlag stehen. Eine
    // Schicht, die erst danach greift, wäre eine Heilung mit anderem Namen —
    // und die gibt es in dieser Schule als eigenen Weg.
    //
    // **Nach WARD, nicht davor.** WARD ist eine Eigenschaft des Spielers und
    // mindert jeden Schlag; die Schicht fängt auf, was danach übrig bleibt.
    // Andersherum gerechnet würde WARD den Schutz mitmindern und damit doppelt
    // zählen. Dieselbe Reihenfolge wie in `Fight::hit_player`.
    const abgefangen = Math.min(this.shieldValue, nachWard);
    if (abgefangen > 0) {
      this.shieldValue -= abgefangen;
      this.shieldAbsorbedValue += abgefangen;
      this.log?.push(this.fightElapsedMs(timeMs), EVENT_SHIELD_ABSORB, NO_SPELL_REF, abgefangen);
      if (this.shieldValue === 0) {
        // **COAGULATIONs Schlüsseltalent** (#419): Der brechende Schild legt
        // eine halbe Schicht nach — einmal je gelegtem Schild.
        //
        // Abgerundet (GAME-DESIGN 18.4) und nur, wenn dabei etwas übrig
        // bleibt: Eine Nachlage von 0 wäre keine, würde aber den Merker
        // verbrauchen und sähe im Verlauf wie eine aus.
        const nachlage = this.hasKeystone(KEYSTONE_SHIELD_REFUND)
          ? Math.floor(this.shieldLastAmount / 2)
          : 0;
        //
        // **Kein `return` hier.** Der Schlag, der die Schicht bricht, kann
        // mehr tragen, als sie auffängt; der Rest muss weiter unten beim
        // Spieler ankommen. Ein früher Ausstieg verschluckte ihn — der Schild
        // wäre dann nicht nur eine Nachlage wert, sondern ein Freischlag.
        if (nachlage > 0 && !this.shieldRefundUsed) {
          this.shieldRefundUsed = true;
          this.shieldValue = nachlage;
          this.shieldStacksValue = 1;
        } else {
          // Aufgebraucht: Die Stapel sind weg, nicht nur der Betrag.
          this.shieldStacksValue = 0;
          this.shieldExpiresAtMs = -1;
        }
      }
    }

    const angekommen = nachWard - abgefangen;
    this.damageTakenValue += angekommen;
    this.log?.push(this.fightElapsedMs(timeMs), EVENT_PLAYER_DAMAGE, NO_SPELL_REF, angekommen);
    this.playerHpValue = Math.max(0, this.playerHpValue - angekommen);
    if (this.playerHpValue === 0) {
      this.diedAtMsValue = timeMs - this.fightStartedAtMs;
    }
    // **Der Rückschlag hängt am ANGEKOMMENEN Schaden**, nicht am rohen
    // (RIPOSTE, Issue #481). Ein Schlag, den die Schicht ganz auffängt, gibt
    // nichts zurück: Sonst wären Schicht und Rückschlag zusammen doppelt
    // stark, und ein Spieler mit voller Schicht bekäme Schaden geschenkt, den
    // er gar nicht genommen hat.
    this.reflectDamage(angekommen, timeMs);
  }

  /**
   * Gibt einen Teil des eingehenden Schadens an den Boss zurück (RIPOSTE).
   *
   * **Ganzzahlig und abgerundet**, wie bei der Umwandlung nebenan: Der Anteil
   * steht als Prozentzahl in den Daten, damit hier
   * `Math.floor(schaden * prozent / 100)` möglich ist.
   *
   * Der zurückgegebene Schaden zählt als Schaden des SPIELERS — er geht durch
   * `bookProgress` wie jeder andere und bewegt damit auch den Fortschritt.
   * Ein zweiter Weg an der Buchung vorbei wäre eine Stelle zu viel.
   */
  private reflectDamage(angekommen: number, timeMs: number): void {
    if (this.reflectPercentValue === 0 || angekommen <= 0) return;
    if (timeMs - this.fightStartedAtMs > this.reflectUntilMs) return;
    const zurueck = Math.floor((angekommen * this.reflectPercentValue) / 100);
    if (zurueck <= 0) return;
    this.reflectedValue += zurueck;
    this.bookProgress(zurueck, this.reflectSpellIndex, timeMs);
  }

  /**
   * Bricht einen laufenden Boss-Zauber ab (GAME-DESIGN 11.2, Baustein 9).
   *
   * Gibt zurück, ob überhaupt etwas zu unterbrechen war. Ein Unterbrechen ins
   * Leere ist **kein Fehler**, sondern eine vertane Abklingzeit.
   */
  private interruptBossCast(timeMs: number): boolean {
    if (this.castingValue < 0) return false;
    this.castingValue = -1;
    this.castLandsAtMsValue = -1;
    // **Sofort, nicht zum geplanten Trefferzeitpunkt** (Issue #327, AC2): Der
    // Balken verschwindet mit dem Unterbrechen. Ein Balken, der stehen bliebe,
    // behauptete eine Wirkung, die nicht mehr kommt.
    this.castingStartedAtMsValue = -1;
    this.interruptsLandedValue++;
    // **Das Spiel schreibt es jetzt selbst** (Issue #151). Bis dahin führte es
    // nur der Simulator — und die Fehleranalyse läuft im Browser, nicht im
    // Simulator. Ein Protokoll, in dem die Größe überall fehlt, kann eine
    // Gelegenheit nicht belegen.
    this.log?.push(
      this.fightElapsedMs(timeMs),
      EVENT_INTERRUPT_LANDED,
      NO_SPELL_REF,
      this.interruptsLandedValue,
    );
    return true;
  }

  /** Verbleibende Lebenspunkte in Prozent, ganzzahlig abgerundet. */
  /** PRUEF-ZUGANG, erscheint nicht (Issue #294): Der Ring zeigt es im Kampf. */
  get bossHpPercent(): number {
    if (this.boss.hp <= 0) return 0;
    const übrig = this.boss.hp - this.damageDealtValue;
    return Math.floor((Math.max(0, übrig) * 100) / this.boss.hp);
  }

  get resourceOverflow(): number {
    return this.resourceOverflowValue;
  }

  get fightState(): number {
    return this.stateValue;
  }

  /** Kampfzeit, zu der der Kampf endete, sonst `NOT_ACTIVE`. */
  get endedAt(): number {
    return this.endedAtMs;
  }

  /**
   * Zeitpunkt auf der durchlaufenden Uhr, zu dem der Kampf begann.
   *
   * **Zwei Uhren, eine Zeitachse:** Die Abklingzeiten laufen schon in der
   * Vorbereitung mit (GAME-DESIGN 10), die Kampfuhr beginnt erst mit dem Ende des
   * Countdowns. Beide messen dieselbe durchlaufende Zeit — nur der Nullpunkt der
   * Kampfuhr liegt später. Eine zweite, angehaltene Zeitachse wäre die Sorte
   * Sonderfall, an der später jede Aufzeichnung scheitert.
   */
  get fightStartedAt(): number {
    return this.fightStartedAtMs;
  }

  /** Verstrichene **Kampfzeit** — die Zahl, die der Spieler auf der Uhr sieht. */
  fightElapsedMs(timeMs: number): number {
    const elapsed = timeMs - this.fightStartedAtMs;
    return elapsed > 0 ? elapsed : 0;
  }

  /** Restzeit bis zum Überdruck. */
  overpressureRemainingMs(timeMs: number): number {
    const left = this.boss.overpressureMs - this.fightElapsedMs(timeMs);
    return left > 0 ? left : 0;
  }

  /**
   * Den Kampf beginnen lassen.
   *
   * Setzt den Nullpunkt der Kampfuhr und den Verfall-Plan des Vorrats. Die
   * Abklingzeiten laufen unverändert weiter — was in der Vorbereitung gewirkt
   * wurde, bleibt gewirkt.
   */
  beginFight(timeMs: number): void {
    this.fightStartedAtMs = timeMs;
    this.fightHasBegun = true;
    this.nextDecayAtMs = timeMs + this.school.resource.decayPerStepMs;
    this.nextEnergyAtMs = timeMs + (this.boss.energy?.perStepMs ?? 0);
    this.nextDoomAtMs = timeMs + (this.boss.doom?.perStepMs ?? 0);
    this.nextStackAtMs = timeMs + (this.boss.stacks?.perStepMs ?? 0);
    this.nextRunawayAtMs = timeMs + (this.boss.runaway?.perStepMs ?? 0);
    this.stabilisedValue = 0;
    // Das Fenster haengt an der KAMPFuhr, nicht an der durchlaufenden.
    this.raceDone = false;
    this.raceEndsAtMs = -1;
    this.afflictedValue = false;
    this.nextAfflictionAtMs = 0;
  }

  castCount(index: SpellIndex): number {
    return this.castCounts[index] ?? 0;
  }

  damageOf(index: SpellIndex): number {
    return this.damageBySpell[index] ?? 0;
  }

  /** Verfallene DoT-Ticks durch zu frühes Erneuern (GAME-DESIGN 7.2). */
  lostDotTicks(index: number): number {
    return this.dotTicksLost[index] ?? 0;
  }

  dotTickCount(index: number): number {
    return this.dotTickCounts[index] ?? 0;
  }

  cooldownRemainingMs(index: number, timeMs: number): number {
    const left = (this.cooldownReadyAtMs[index] ?? 0) - timeMs;
    return left > 0 ? left : 0;
  }

  /**
   * Restlaufzeit eines DoT.
   *
   * Grundlage der wichtigsten Entscheidung in Schritt 1: so spät wie möglich
   * erneuern, aber nicht zu spät (GAME-DESIGN 7.2).
   */
  dotRemainingMs(index: SpellIndex, timeMs: number): number {
    const expires = this.dotExpiresAtMs[index] ?? NOT_ACTIVE;
    if (expires === NOT_ACTIVE) return 0;
    const left = expires - timeMs;
    return left > 0 ? left : 0;
  }

  /**
   * Eine Zeit unter dem Einfluss des getragenen CATALYSIS (GAME-DESIGN 6.7).
   *
   * **Die einzige Stelle, an der Tempo auf eine Zeit trifft.** Jeder Wirkort
   * geht durch sie; wer einen neuen hinzufügt, ruft sie auf und bekommt das
   * Takt-Raster und die Untergrenze geschenkt.
   *
   * Ohne getragene Ausrüstung gibt sie `baseMs` unverändert zurück — siehe
   * `hastedMs`, dort hängt AC7 daran.
   */
  private hasted(baseMs: number): number {
    return hastedMs(baseMs, this.worn.catalysis);
  }

  /**
   * Der Tick-Abstand eines Effekts über Zeit, unter dem Einfluss von Tempo.
   *
   * Steht als eigene Methode da, weil der Abstand an **drei** Stellen gebraucht
   * wird: beim Setzen, beim nächsten Tick und beim Zählen der durch Erneuern
   * verlorenen Ticks. Drei Handabschriften derselben Rechnung wären genau die
   * Klasse Fehler, die #134, #138, #159 und #173 hinterlassen haben.
   */
  private dotInterval(dot: { readonly tickIntervalMs: number }): number {
    // **Der Faktor aus `tickBuffs`** (Issue #489, FORGE). Er verkürzt den
    // ABSTAND, macht die Ticks also HÄUFIGER — deshalb geteilt, nicht
    // multipliziert. Ein Faktor 2 heißt „doppelt so oft", nicht „halb so oft".
    //
    // **Die einzige der vier Zielgrößen, bei der der Faktor teilt.** Bei
    // Punktegewinn, Hitze-Aufbau und Stapel-Wert multipliziert er. Wer die
    // Zeile nach dem Muster der anderen drei liest, dreht die Wirkung um —
    // und ein Fehler hier fällt spät auf, weil der Tick-SCHADEN unverändert
    // bleibt und nur die Gesamtsumme wandert.
    const faktor = this.tickBuffs.factor(this.nowMsForBuffs);
    const abstand = faktor === 1 ? dot.tickIntervalMs : Math.round(dot.tickIntervalMs / faktor);
    return this.hasted(abstand);
  }

  /**
   * Über alle Brände laufen, die **gerade brennen** (Issue #517).
   *
   * # Warum es diese Schleife vorher nicht gab
   *
   * `dotExpiresAtMs` wurde bis hierher ausschliesslich **je Index** gelesen und
   * geschrieben. Die einzige Sammel-Operation war `fill(NOT_ACTIVE)` beim
   * Kampfende — und die kennt keine Bedingung. Wer etwas mit *allen laufenden*
   * Bränden tun wollte, hatte keine Stelle dafür.
   *
   * Genau das teilen sich EMBER und PYROCLASM: **nicht** einen Rechenschritt,
   * sondern diese Iteration. EMBER verlängert, was sie findet; PYROCLASM liest
   * und räumt ab. Wer sie für einen baut, hat sie für den anderen.
   */
  private forEachActiveDot(
    timeMs: number,
    tue: (index: number, dot: DotDef, laeuftNochMs: number) => void,
  ): void {
    for (let i = 0; i < this.spells.length; i++) {
      const dot = this.spells[i]?.dot;
      if (dot === undefined) continue;
      if (!this.isDotActive(i, timeMs)) continue;
      tue(i, dot, this.dotRemainingMs(alsSpellIndex(i), timeMs));
    }
  }

  /**
   * Jeden laufenden Brand um ein Tick-Intervall verlängern (EMBER, #517).
   *
   * # `dotNextTickAtMs` bleibt bewusst unberührt
   *
   * Der Takt läuft **unabhängig** von der Laufzeit: Die Tick-Schleife setzt
   * `nextTick + dotInterval` und fragt getrennt, ob der Brand noch aktiv ist.
   * Wer beim Verlängern auch den Takt verschöbe, liesse den Brand länger
   * brennen und dabei **seltener** ticken — der Zuwachs käme nie an.
   *
   * Nichts zu tun ist hier die richtige Handlung, und deshalb steht sie als
   * Satz da statt als fehlende Zeile.
   */
  extendActiveDots(timeMs: number, ticks: number): void {
    if (ticks <= 0) return;
    this.forEachActiveDot(timeMs, (i, dot) => {
      const bisher = this.dotExpiresAtMs[i] ?? NOT_ACTIVE;
      this.dotExpiresAtMs[i] = bisher + ticks * this.dotInterval(dot);
    });
  }

  /**
   * Alle laufenden Brände verbrauchen und ihren Rest als Schaden geben
   * (PYROCLASM, #517).
   *
   * # Restdauer × Schaden je Tick — und warum das keine neue Zahl ist
   *
   * Es ist genau der Schaden, den die Brände noch gemacht hätten. Nach der
   * Erneuerungsregel ist verbleibende Restzeit **verloren**, wenn zu früh
   * erneuert wird; PYROCLASM kehrt dieselbe Regel um. Der Spieler kennt sie
   * bereits — früh zünden verschenkt, spät zünden riskiert das Auslaufen.
   *
   * Ein Brand, der schon abgelaufen ist, trägt **null** bei: `forEachActiveDot`
   * sieht ihn gar nicht, und `dotRemainingMs` gäbe ohnehin 0.
   */
  consumeActiveDots(timeMs: number): number {
    let summe = 0;
    this.forEachActiveDot(timeMs, (i, dot, laeuftNochMs) => {
      const abstand = this.dotInterval(dot);
      // Ganzzahlig bis zur letzten Teilung (18.4): erst die Ticks, dann der
      // Schaden. Andersherum trüge die Rundung sich durch jeden Brand.
      const offeneTicks = Math.floor(laeuftNochMs / abstand);
      summe += computeDamage(dot.tickDamage * offeneTicks, this.factorsFor(false));
      this.dotExpiresAtMs[i] = NOT_ACTIVE;
      this.dotNextTickAtMs[i] = NOT_ACTIVE;
    });
    return summe;
  }

  /**
   * Ein Kritwurf für **eine Wirkung** (GAME-DESIGN 6.5).
   *
   * > Echte Wahrscheinlichkeit pro Zauber, pro DoT-Tick und pro Heilung. Kein
   * > Pech-Schutz, kein fester Startwert, keine feste Reihenfolge.
   *
   * Gibt zurück, **ob** der Wurf saß. Die Faktoren holt der Aufrufer über
   * `factorsFor` — getrennt, weil das Ergebnis auch für die Trefferanzeige und
   * das Regel-Ereignis `RULE_CRIT` gebraucht wird und ein zweiter Wurf dafür
   * eine zweite Wahrheit wäre.
   *
   * # Ohne PURITY fällt gar kein Wurf
   *
   * `RandomStream.chance` schließt bei einer Wahrscheinlichkeit von 0 kurz,
   * **ohne den Strom zu drehen**. Damit bleibt der Ziehungszähler unverändert,
   * und mit ihm die Zustands-Prüfsumme. **Daran hängt AC7**, und zwar an einer
   * Stelle, die man leicht übersieht: Ein Wurf, dessen Ergebnis feststeht, wäre
   * rechnerisch folgenlos und trotzdem sichtbar.
   */
  /**
   * # Der Strom kommt vom Aufrufer, seit Issue #602
   *
   * Bis zum 2026-08-07 zog diese Funktion **immer** aus `STREAM_CRIT` — auch
   * für den Krit-Wurf eines DoT-Ticks. Beide Seiten taten das gleich, die
   * Parität war intakt, und genau deshalb hat es niemand gesehen.
   *
   * GAME-DESIGN 18.4 verlangt „je einen eigenen Strom", und der Nebensatz sagt
   * warum: *„sonst verschiebt eine zusätzliche Ziehung alles Nachfolgende."*
   * Zog ein Tick aus `STREAM_CRIT`, verschob **jeder zusätzliche Tick die
   * Krit-Würfe aller Zauber**. Die Regel schützt nicht die Parität, sondern die
   * Vergleichbarkeit zweier Läufe mit verschiedenen Rotationen — wer einen DoT
   * hinzufügte, maß im Balance-Vergleich teilweise verschobenen Zufall statt
   * der Rotation.
   */
  private rollCrit(stream: number): boolean {
    return this.random.stream(stream).chance(critChance(this.worn));
  }

  /** Die Faktoren für einen Treffer, je nachdem ob der Wurf saß. */
  private factorsFor(crit: boolean): DamageFactors {
    return crit ? this.factorsOnCrit : this.factors;
  }

  /**
   * Einen Schadensfaktor während des Kampfes setzen (Issue #196).
   *
   * **Die einzige Stelle, die `factors` schreibt.** Sie zieht die Krit-Fassung
   * mit — genau hier und nicht beim Treffer, damit im laufenden Takt nichts
   * entsteht (GAME-DESIGN 18.2).
   *
   * `crit` ist ausgenommen: Er ist aus `worn` abgeleitet und gehört nicht zu
   * den Faktoren, die eine Mechanik setzt. Wäre er hier setzbar, könnte
   * jemand die Krit-Fassung über diesen Weg still von der Basis abkoppeln.
   */
  setDamageFactor(kind: Exclude<keyof DamageFactors, 'crit'>, value: number): void {
    this.factors[kind] = value;
    // Das Mitziehen ist der ganze Punkt dieser Methode. Fehlt diese Zeile,
    // rechnet jeder kritische Treffer mit dem alten Wert weiter.
    this.factorsOnCrit[kind] = value;
  }

  /**
   * Ob die Krit-Fassung noch zur Basis passt.
   *
   * Für Prüffälle und Werkzeuge: Sie muss in **allen** Feldern außer `crit`
   * mit `factors` übereinstimmen. Weicht sie ab, rechnen gewöhnlicher und
   * kritischer Treffer mit verschiedenen Grundlagen — und der Unterschied
   * fällt nur bei einem Prüffall auf, der beides zugleich abdeckt.
   */
  /**
   * PRUEF-ZUGANG, erscheint nicht (Issue #294).
   *
   * Der Name sagt es: Er vergleicht zwei interne Fassungen derselben
   * Faktoren. Fuer den Spieler gibt es dazu nichts zu sehen.
   */
  get critFactorsMirrorBase(): boolean {
    for (const key of Object.keys(this.factors) as (keyof DamageFactors)[]) {
      if (key === 'crit') continue;
      if (this.factors[key] !== this.factorsOnCrit[key]) return false;
    }
    return true;
  }

  isDotActive(index: number, timeMs: number): boolean {
    // Größer-gleich: Der letzte Tick fällt mit dem Ablauf zusammen. Eine Dauer von
    // 12 s bei 2 s Tick-Abstand ergibt 6 Ticks (bei 2, 4, 6, 8, 10, 12 s), nicht 5.
    // Entschieden am 2026-07-28 (STEP1-SPEC § 1.2): Der Spieler sieht 12,0 s
    // Restlaufzeit — bei 5 Ticks wären die letzten zwei Sekunden wirkungslos, und
    // "so spät wie möglich erneuern" (GAME-DESIGN 7.2) hieße heimlich "bei 10 s".
    return (this.dotExpiresAtMs[index] ?? NOT_ACTIVE) >= timeMs;
  }

  /** Ist der Zauber selbst bereit? Die globale Wartezeit zählt hier nicht mit. */
  isOffCooldown(index: SpellIndex, timeMs: number): boolean {
    return (this.cooldownReadyAtMs[index] ?? 0) <= timeMs;
  }

  /** Kann der Zauber jetzt gewirkt werden? */
  isReady(index: SpellIndex, timeMs: number): boolean {
    // **Hier stand bis zum 2026-08-05 eine Sperre** — „während einer Wirkzeit
    // ist keiner bereit". Sie ist mit der Regel entfallen: Die Wirkzeit sperrt
    // nichts, und ein Zauber, der während ihr gedrückt wird, wirkt (GAME-DESIGN
    // 7.1). Die Leiste zeigt deshalb weiter an, was wirklich drückbar ist.
    if (!this.isOffCooldown(index, timeMs)) return false;
    // Dieselbe Ausnahme wie in `cast` (GAME-DESIGN 6.6, Issue #110). Sie steht
    // hier ein zweites Mal, weil die Zauberleiste `isReady` fragt und nicht
    // `cast`: Ohne sie zeigte sie den Unterbrecher als gesperrt an, während er
    // in Wahrheit wirkbar ist — und der Spieler drückte ihn nicht.
    const spell = this.spells[index];
    if (spell !== undefined && !spell.triggersGcd) return true;
    return this.gcd.isReady(timeMs);
  }

  // ------------------------------------------------------------------- Takt

  /**
   * Laufende Wirkungen eines Takts: DoT-Ticks und Verfall des Vorrats.
   *
   * Beide laufen unabhängig von Tastendrücken weiter — der Verfall folgt einem
   * festen Plan ab Kampfbeginn, nicht dem letzten Wirken.
   */
  stepPassive(timeMs: number): void {
    // **Vor der Abbruchbedingung**, damit die Uhr auch dann stimmt, wenn der
    // Kampf gerade endet — sonst rechneten die Faktoren mit einer alten Zeit.
    this.nowMsForBuffs = timeMs;
    if (this.stateValue !== FIGHT_RUNNING) return;
    // **Der Spielertod beendet den Versuch** (GAME-DESIGN A.3: „Versuch vorbei,
    // keine Strafe"), und zwar hier — nicht erst in den einzelnen Wirkungen.
    //
    // Bis zum 2026-08-07 setzte der Tod nur `diedAtMsValue`. Mehrere Stellen
    // fragten es ab (`hitPlayer`, `doom` und weitere), das Wirken selbst nicht:
    // hp, doom und erlittener Schaden standen, ausgeteilter Schaden wuchs
    // weiter. Im `doom-stacks`-Lauf gemessen — 2120 bei 20000 ms, 2780 am Ende,
    // 660 Schaden von einem toten Spieler (Issue #567).
    //
    // Die Rust-Seite tut dasselbe eine Ebene höher: `fight.rs` bricht die
    // Takt-Schleife ab (`if self.died_at.is_some() { break; }`), also passiert
    // dort nach dem Tod GAR nichts mehr. Diese Zeile ist ihr Spiegelbild — sie
    // steht deshalb neben der Kampfende-Abfrage und nicht in `cast` allein.
    if (this.diedAtMsValue >= 0) return;

    for (let i = 0; i < this.spells.length; i++) {
      const spell = this.spells[i] as SpellDef;
      const dot = spell.dot;
      if (dot === undefined) continue;
      const nextTick = this.dotNextTickAtMs[i] ?? NOT_ACTIVE;
      if (nextTick === NOT_ACTIVE || timeMs < nextTick) continue;

      // **Der Nachlauf eines erloschenen Brands** (#419, PUTREFACTIONs
      // Schlüsseltalent). Er steht VOR der Schadensrechnung und steigt mit
      // `continue` aus: Ein toter Brand legt Stapel, aber er schlägt nicht
      // mehr zu. Trüge er auch Schaden, wäre er eine verlängerte Laufzeit —
      // und die gibt es schon als „Lasting Rot".
      if (!this.isDotActive(i, timeMs)) {
        if (!this.imDecayEcho(i, timeMs)) continue;
        this.addDecayStacks(i, dot.decayPerTick ?? 0, timeMs);
        this.dotNextTickAtMs[i] = nextTick + this.dotInterval(dot);
        continue;
      }

      // **Ein eigener Wurf je Tick** (GAME-DESIGN 6.5: „pro Zauber, pro
      // DoT-Tick und pro Heilung"). Nicht der Wurf der Wirkung, die ihn gesetzt
      // hat — sonst kritete ein DoT über seine ganze Laufzeit entweder immer
      // oder nie, und der Glücksanteil, den 6.5 ausdrücklich will, verschwände
      // in einer einzigen Entscheidung.
      // **Aus STREAM_DOT_TICK, nicht aus STREAM_CRIT** (Issue #602): Ein Tick
      // darf die Krit-Würfe der gewirkten Zauber nicht verschieben.
      const kritet = this.rollCrit(STREAM_DOT_TICK);
      // Auch ein Brand-Tick gehört seinem Zauber und damit dessen Schule
      // (Issue #328). Ohne diese Zeile trüge er den Widerstand des zuletzt
      // gewirkten Zaubers — und das wäre nicht falsch berechnet, sondern
      // falsch zugeordnet: ein Fehler, den keine Summe verrät.
      this.setDamageFactor('resistance', this.resistanceOfSpell[i] ?? 1);
      // Und derselbe Zuschlag wie beim Wirken: Ein Tick gehoert seinem Zauber
      // (#328), also traegt er auch dessen perDecayStackPercent (#484).
      this.applyDecayStackBonus(i);
      // **Auch hier, obwohl ein Brand nie ein Entlader ist** (#419): Die
      // Methode SETZT den Topf. Bliebe sie hier weg, truege jeder Tick nach
      // einem Entlader dessen Zuschlag weiter.
      this.applySpenderBonus(i);
      const damage = computeDamage(dot.tickDamage, this.factorsFor(kritet));
      // **Der Schleifenindex trägt die Marke** (Issue #444): `i` läuft über
      // dieselbe Liste, aus der `indexOf` seine Werte nimmt — es IST ein
      // Zauber-Index, ihm fehlt nur der Typ. Die Umschreibung steht deshalb
      // hier und nicht in der Schleifenbedingung: Dort bräuchte es zwei
      // (Start und Fortschaltung) für dieselbe Aussage.
      this.bookProgress(damage, alsSpellIndex(i), timeMs);
      // Auch ein Brand-Tick speist Weg 3 (Issue #79) — im Simulator läuft
      // jeder ausgeteilte Schaden durch `deal`, hier gibt es zwei Stellen.
      this.convertDamageToShield(damage, timeMs);
      this.reportHit(damage, HIT_DOT, kritet, timeMs);
      this.log?.push(this.fightElapsedMs(timeMs), EVENT_DOT_TICK, i, damage);
      // **Das Ereignis, das bis #177 ruhte.** Der Kommentar an der Cast-Stelle
      // sagte, es fehle nicht aus Nachlässigkeit — es hatte nur nie einen Fall.
      // Mit getragenem PURITY hat es einen.
      if (kritet) this.events.emit(RULE_CRIT, i, damage, timeMs);
      // Ein Tick ist **ein** Ereignis, nicht zwei. Wer allen Schaden zählen
      // will, meldet sich für beide Arten an — so bleibt die Folge eindeutig.
      this.events.emit(RULE_DOT_TICK, i, damage, timeMs);
      this.dotTickCounts[i] = (this.dotTickCounts[i] ?? 0) + 1;
      // **Zersetzungs-Stapel je Tick** (Issue #498, ROT). Sie laufen NICHT
      // über das Ereignis wie der Hitze-Gewinn: Stapel sind keine
      // Schul-Ressource, sondern liegen auf dem Boss — und die Stelle, die
      // sie beim Wirken legt (`applyPutrefaction`), sitzt ebenfalls direkt im
      // Kampf. Zwei Wege für dieselbe Größe wären eine Stelle zu viel.
      this.addDecayStacks(i, dot.decayPerTick ?? 0, timeMs);
      // **Den Endpunkt des Nachlaufs fortschreiben, solange der Brand lebt**
      // (#419). Beim Erlöschen steht damit von selbst die Frist des jüngsten
      // Stapels da — es braucht keine Stelle, die das Ende des Brands abpasst.
      this.merkeDecayEcho(i, dot.decayPerTick ?? 0);
      // Der Hitze-Gewinn stand bis E3-2 hier. Er hängt jetzt am Ereignis
      // `dot.tick` (`src/combat/schools/stacks.ts`) — was ein Zauber
      // beisteuert, steht in seinen Daten, nicht im Kampfcode.
      this.dotNextTickAtMs[i] = nextTick + this.dotInterval(dot);
    }

    const resource = this.school.resource;
    if (resource.decayPerStepMs > 0 && timeMs >= this.nextDecayAtMs) {
      const left = this.resourceValue - resource.decayAmount;
      this.resourceValue = left > 0 ? left : 0;
      this.nextDecayAtMs += resource.decayPerStepMs;
    }

    // **Heilung über Zeit** (POULTICE, Issue #481). Sie läuft im selben Takt
    // wie die übrigen wiederkehrenden Größen und deckelt an der vollen
    // Anzeige — was darüber hinausginge, verpufft, genau wie bei der
    // Sofortheilung.
    while (this.healOverTimeDone < this.healOverTimeSteps && timeMs >= this.healOverTimeNextAtMs) {
      this.healOverTimeDone++;
      const soll = Math.floor(
        (this.healOverTimeAmount * this.healOverTimeDone) / this.healOverTimeSteps,
      );
      const faellig = soll - this.healOverTimeCredited;
      this.healOverTimeCredited = soll;
      const fehlend = Math.max(0, this.vesselMax - this.playerHpValue);
      const wirklich = Math.min(faellig, fehlend);
      if (wirklich > 0) {
        this.playerHpValue += wirklich;
        this.healedValue += wirklich;
        // **Derselbe Nachlauf wie bei der Sofortheilung**, mit dem Betrag, der
        // WIRKLICH angekommen ist: zurückdrängen (Baustein 6) und ins
        // Protokoll. Ein Heil-Takt ist damit ein Ereignis wie ein Brand-Takt —
        // ihn stumm zu lassen hieße, dieselbe Wirkung je nach Zauber einmal zu
        // melden und einmal nicht.
        this.stabilise(wirklich);
        this.log?.push(this.fightElapsedMs(timeMs), EVENT_HEALED, NO_SPELL_REF, wirklich);
      }
      this.healOverTimeNextAtMs += this.healOverTimeIntervalMs;
    }

    // Die zweite Uhr (GAME-DESIGN 11.2, Baustein 1). Sie steigt **unabhängig
    // vom Schaden des Spielers** — wer schneller zuschlägt, verschiebt den
    // Auslöser nicht. Genau das macht sie zu einem zweiten Takt, gegen den
    // geplant werden muss.
    const energy = this.boss.energy;
    if (energy !== null && energy.perStepMs > 0) {
      while (timeMs >= this.nextEnergyAtMs) {
        this.bossEnergyValue += energy.amount;
        if (this.bossEnergyValue >= energy.threshold) {
          this.bossEnergyTriggersValue++;
          this.bossEnergyValue = energy.resetTo;
          // **Und hier tut sie etwas** (Issue #568). Bis dahin endete der
          // Zweig mit dem Hochzählen: Die Leiste stieg, erreichte 100, sprang
          // zurück — und keine Zeile im Kampf fragte danach. `test-vessel`
          // lieferte mit und ohne `energy` bitgleich 8080/1600/0.
          //
          // Der Auftraggeber hat entschieden: ein **Boss-Schlag außerhalb der
          // Zeitleiste**. Deshalb derselbe Weg wie ein `strike` dort
          // (`hitPlayer`) — nur kommt der Zeitpunkt aus der Leiste statt aus
          // `atMs`. Genau das ist der „innere Rhythmus, der nichts mit dem
          // Lebensbalken zu tun hat" (GAME-DESIGN 11.2, Baustein 1).
          //
          // # Der Eintrag steht VOR dem Schlag, und das aus zwei Gründen
          //
          // Erstens fachlich: Tötet der Schlag den Spieler, steht der Grund
          // im Verlauf davor und nicht dahinter.
          //
          // Zweitens Parität: `fight.rs` schreibt ihn an genau dieser Stelle.
          // Wichen die Seiten hier ab, färbte die Prüfung takt-genau — und
          // man suchte den Fehler dort, wo er sich zeigt, statt dort, wo er
          // sitzt.
          //
          // **Diese Ereignisart wurde bis #568 auf dieser Seite NIE
          // geschrieben.** Art 8 war vergeben, in `log.ts` dokumentiert, von
          // `log-kinds` gedeckt und von `freshness` gezählt — nur schrieb sie
          // niemand. Die Parität kann das nicht fangen: Der Verlauf steht in
          // keiner Aufzeichnung, die diesen Boss fährt. Deshalb hängt an
          // dieser Zeile ein eigener Prüffall
          // (`tests/unit/energie-ausloeser.test.ts`), nicht die Parität.
          // Wie viele andere Arten dasselbe Bild zeigen, ist ungemessen und
          // steht als Issue #676.
          this.log?.push(
            this.fightElapsedMs(timeMs),
            EVENT_BOSS_ENERGY_TRIGGERED,
            NO_SPELL_REF,
            this.bossEnergyTriggersValue,
          );
          this.hitPlayer(energy.onTrigger.damage, timeMs);
        }
        this.nextEnergyAtMs += energy.perStepMs;
      }
    }

    // Der Zähler, der bei seiner Schwelle tötet (11.2, Baustein 2; #216).
    //
    // Er steigt wie die Energieleiste von selbst — aber er springt **nicht**
    // zurück. Wer die Schwelle erreicht, ist tot, und zwar durch den Zähler
    // und nicht durch Zeitablauf: `hitPlayer` mit den vollen Lebenspunkten
    // wäre der falsche Weg, weil WARD und Schutzschicht dann noch mindern
    // würden. Ein Zähler, den eine Schutzschicht überlebt, wäre keiner.
    const doom = this.boss.doom;
    if (doom !== null && doom.perStepMs > 0 && this.diedAtMsValue < 0) {
      // **Verbrauchte Ladungen wachsen nach** (Issue #561). Bis dahin war
      // `clearCharges` ein Vorrat für den ganzen Kampf: zwei Senkungen à 40
      // gegen 360 Anstieg — der Zauber wäre wirkungslos gewesen, egal wie oft
      // man ihn drückt. Die Begrenzung liegt jetzt in der Wartezeit, nicht in
      // einem Vorrat.
      //
      // `clearCharges` bleibt die **Obergrenze**: Mehr als der Boss vorsieht
      // liegt nie bereit, gespart wird also nicht.
      while (timeMs >= this.nextDoomAtMs) {
        // **Die Prüfung sitzt IM Doom-Takt, nicht in einer eigenen Uhr.** Die
        // Rust-Seite ist ereignisbasiert und kommt hier alle `perStepMs`
        // vorbei; prüfte das Spiel bei jedem Zehntel-Takt, käme die Ladung
        // dort früher zurück. **Zwei Seiten, die verschieden oft nachladen,
        // rechnen ab dem ersten Unterschied verschiedenen Schaden** — und das
        // ist genau die Stelle, an der Umbauten auseinanderlaufen.
        //
        // Die Nachladezeit wird damit auf den Takt gerundet: 6667 ms bei
        // `perStepMs` 1000 heißt, die Ladung kommt beim Tick nach 7000 zurück.
        if (this.nextDoomChargeAtMs > 0 && this.nextDoomAtMs >= this.nextDoomChargeAtMs) {
          if (this.doomChargesValue < doom.clearCharges) {
            this.doomChargesValue++;
            this.nextDoomChargeAtMs =
              this.doomChargesValue < doom.clearCharges
                ? this.nextDoomChargeAtMs + this.rechargeMs()
                : 0;
          }
        }
        this.doomValue += doom.amount;
        this.nextDoomAtMs += doom.perStepMs;
        if (this.doomValue >= doom.threshold) {
          this.doomValue = doom.threshold;
          this.playerHpValue = 0;
          this.diedAtMsValue = timeMs - this.fightStartedAtMs;
          // KEIN Protokoll-Eintrag: Das Protokoll traegt die Fehleranalyse,
          // und ein Schadens-Ereignis ueber 0 waere dort eine Zeile, die nach
          // Schaden aussieht und keiner ist. Eine eigene Art im gemeinsamen
          // Nummernraum (#134) waere die richtige Loesung — sie gehoert in das
          // Issue, das den Zaehler an einen Boss haengt, nicht hierher.
          break;
        }
      }
    }

    // Die davonlaufende Reaktion (11.2, Baustein 6; #247).
    //
    // **Sie senkt den RING**, nicht einen zweiten Wert — der Ring ist die
    // Reaktion (9.1). Der Fortschritt geht deshalb durch `bookProgress`, wie
    // jeder andere auch: EINE Buchungsstelle, die das Fenster und die
    // Boss-Heilung kennt.
    //
    // `vomSpieler: false` — sie ist kein Schaden. Das Fenster hält sie nicht
    // auf, und sie darf nicht als "im Fenster verloren" erscheinen.
    //
    // Zwischen Zaehler und Schwaechung, dieselbe Stelle wie in `fight.rs`.
    const runaway = this.boss.runaway;
    if (runaway !== null && runaway.perStepMs > 0 && this.stateValue === FIGHT_RUNNING) {
      while (timeMs >= this.nextRunawayAtMs) {
        this.bookProgress(runaway.amount, NO_SPELL, timeMs, false);
        this.nextRunawayAtMs += runaway.perStepMs;
      }
    }

    // Die Schwaechung ohne Heil-Loesung (11.2, Baustein 8; #240).
    const aff = this.boss.affliction;
    if (aff !== null) {
      const seitBeginnFuerAff = timeMs - this.fightStartedAtMs;
      if (
        !this.afflictedValue &&
        this.afflictionsClearedValue === 0 &&
        seitBeginnFuerAff >= aff.atMs
      ) {
        this.afflictedValue = true;
        this.nextAfflictionAtMs = timeMs + aff.perStepMs;
      }
      if (this.afflictedValue) {
        while (timeMs >= this.nextAfflictionAtMs) {
          this.hitPlayer(aff.damage, timeMs);
          // **Der zweite Auslöser von Baustein 5** (Issue #609): Wer die
          // Schwächung nicht klärt, verliert nicht nur Leben, sondern
          // Fortschritt. Das ist derselbe Satz aus 11.2 — „ein Fehler macht
          // Fortschritt rückgängig" —, nur ist der Fehler hier eine verpasste
          // REINIGUNG statt einer verpassten Unterbrechung.
          //
          // **Je Schritt, nicht einmal.** Ein einmaliges Zurückdrehen wäre
          // eine Strafe für das Bekommen, nicht für das Nicht-Klären; wer
          // sofort reinigt, zahlte dasselbe wie wer es laufen lässt. Der Preis
          // muss mit der Zeit wachsen, sonst stellt er keine Wahl.
          if (this.boss.bossHeal?.on === 'affliction') this.healBoss();
          this.nextAfflictionAtMs += aff.perStepMs;
        }
      }
    }

    // Das Wettrennen (11.2, Baustein 4; #218).
    //
    // Es beginnt GENAU EINMAL, zu dem Zeitpunkt aus den Daten. Ein Fenster,
    // das sich wiederholt, waere die eskalierende Wiederholung (Baustein 7) —
    // ein anderer Baustein mit anderer Wirkung.
    const race = this.boss.race;
    if (race !== null) {
      const seitBeginnFuerRace = timeMs - this.fightStartedAtMs;
      if (!this.raceDone && seitBeginnFuerRace >= race.atMs) {
        this.raceDone = true;
        this.raceEndsAtMs = timeMs + race.windowMs;
      }
      // Abgelaufen. Der andere Ausgang — vorzeitig beendet — laeuft ueber
      // `endRace()`; beide muessen belegt sein (AC2).
      if (this.raceEndsAtMs >= 0 && timeMs >= this.raceEndsAtMs) {
        this.raceEndsAtMs = -1;
      }
    }

    // Der exponentielle Stapel (11.2, Baustein 3; #216).
    //
    // Anders als der Zähler tötet er nicht bei einem Wert, sondern macht den
    // Schaden unbezahlbar: 1 → 3 → 9. Er geht deshalb durch `hitPlayer` wie
    // jeder andere Schlag — WARD und Schutzschicht wirken, und genau das ist
    // die Antwort, die dem Spieler bleibt.
    const stacks = this.boss.stacks;
    if (stacks !== null && stacks.perStepMs > 0) {
      while (timeMs >= this.nextStackAtMs) {
        if (this.stacksValue < stacks.maxStacks) this.stacksValue++;
        this.hitPlayer(stackDamage(stacks, this.stacksValue), timeMs);
        this.nextStackAtMs += stacks.perStepMs;
      }
    }

    // Die feste Zeitleiste. Sie hängt weder am Schaden noch an der Phase,
    // sondern allein an der Uhr — deshalb steht sie hier bei den passiven
    // Wirkungen. Die `while`-Schleife holt mehrere Einträge nach, falls ein
    // Takt über sie hinwegspringt; im 10-ms-Takt kann das nur passieren, wenn
    // zwei Einträge auf denselben Zeitpunkt fallen, und dann ist es richtig so.
    //
    // **Seit #110 hat ein Eintrag Wirkung**: Ein Schlag trifft sofort, ein
    // Zauber beginnt zu laufen und trifft erst nach seiner Wirkzeit.
    //
    this.expireCoagulation(timeMs);
    this.expirePutrefaction(timeMs);
    this.expireBuffs(timeMs);

    // `atMs` zählt **ab Kampfbeginn**, `timeMs` ist die Uhr des Spiels. Beginnt
    // ein Kampf nicht bei null, ist der Unterschied der ganze Versatz — und
    // zwar ein stiller: Die Einträge kämen zu früh, aber der Kampf liefe weiter.
    const seitBeginn = timeMs - this.fightStartedAtMs;

    // Zuerst der laufende Boss-Zauber: Ist seine Wirkzeit um, trifft er.
    // **Vor** dem Vorrücken der Zeitleiste, damit ein Treffer und ein neuer
    // Eintrag im selben Takt in derselben Reihenfolge geschehen wie im
    // Simulator — dort steht `BossCastLands` vor `TimelineStep` in der
    // Rangfolge der Ereignisse.
    if (this.castingValue >= 0 && seitBeginn >= this.castLandsAtMsValue) {
      // Über `timelineAt`, nicht über den Platz in der Liste: Bei wiederholter
      // Zeitleiste liegt der Zähler darüber (#174).
      const laufend = this.timelineEntryAt(this.castingValue);
      this.castingValue = -1;
      this.castLandsAtMsValue = -1;
      this.castingStartedAtMsValue = -1;
      this.interruptsMissedValue++;
      if (laufend !== undefined) this.hitPlayer(laufend.damage, timeMs);
      // **Ein Fehler macht Fortschritt rueckgaengig** (11.2, Baustein 5;
      // Issue #218). Der Auslöser ist der durchgekommene Boss-Zauber — kein
      // neuer Fehler-Begriff, und `interruptsMissed` zaehlt ihn ohnehin.
      //
      // Ob und wie viel er heilt, sagt der BOSS-Datensatz. Eine feste
      // Kopplung gaelte fuer alle Bosse; 11.2 sagt "nur fuer einzelne, sonst
      // frustrierend".
      //
      // **Seit #609 sagt der Datensatz auch, WORAN sie haengt.** Bis dahin war
      // dies die einzige Stelle — jeder Boss, der Fortschritt kosten sollte,
      // verlangte damit dieselbe Taste.
      if (this.boss.bossHeal?.on === 'interrupt') this.healBoss();
    }

    for (;;) {
      // `-1` heißt: nichts steht mehr an. Ohne Wiederholung ist die
      // Zeitleiste zu Ende — mit Wiederholung ist sie leer, und dann darf
      // hier nichts kreisen (#174, der Fall der stummen Puppe).
      const faelligAtMs = this.timelineDueAt(this.timelineIndexValue);
      if (faelligAtMs < 0) break;
      if (seitBeginn < faelligAtMs + this.nextStrikeDelayMs) break;
      const eintrag = this.timelineEntryAt(this.timelineIndexValue)!;
      const index = this.timelineIndexValue;

      // **Die Phasen-Bindung** (Issue #557). Bis hierher war `bossPhase` ein
      // Zustand ohne Wirkung: gesetzt, in die Prüfsumme gerührt, von keiner
      // Regel gelesen. Ein Boss mit zwei Phasenschwellen verlief bitgleich zu
      // demselben Boss mit `phases: []` — gemessen an 1000 Durchgängen.
      //
      // # Das Ereignis VERFÄLLT, es wird nicht nachgeholt
      //
      // Der Index läuft weiter, die Wirkung entfällt. Das ist die Regel, und
      // sie folgt aus der Zeitleiste selbst: Ein aufgestautes Ereignis fiele
      // beim Phasenwechsel gesammelt mit allen anderen, und aus einer
      // lernbaren Abfolge würde eine Lawine zum Schwellenzeitpunkt.
      //
      // # Warum eine UNTERGRENZE und kein Gleichheitstest
      //
      // Phasen laufen nur vorwärts (`if (neue > this.bossPhaseValue)` weiter
      // unten). Ein Ereignis, das an eine übersprungene Phase gebunden wäre,
      // käme nie wieder — `phase: 1` heisst deshalb „ab der zweiten Phase".
      if (eintrag.phase !== undefined && this.bossPhaseValue < eintrag.phase) {
        this.timelineIndexValue++;
        this.nextStrikeDelayMs = 0;
        continue;
      }

      this.log?.push(this.fightElapsedMs(timeMs), EVENT_BOSS_TIMELINE_FIRED, NO_SPELL_REF, index);
      // Verbraucht: Der nächste Eintrag fängt wieder bei seinem eigenen
      // Zeitpunkt an (STAY, Issue #100).
      this.nextStrikeDelayMs = 0;
      this.timelineIndexValue++;
      // **PIN wird HIER gelesen und HIER verbraucht** (Issue #478) — im
      // Auslöse-Moment des Eintrags, nicht während einer laufenden Wirkzeit.
      // Eine Zeile früher gelesen, machte er den gerade laufenden Zauber
      // unterbrechbar: PIN befreite sich selbst statt den nächsten, und der
      // Fall bliebe GRÜN, weil ja etwas unterbrechbar wurde.
      //
      // Verbraucht wird er unabhängig davon, ob er etwas bewirkt hat. Traf er
      // einen Eintrag, der ohnehin unterbrechbar ist, ist er verpufft — das
      // ist die Entscheidung, die der Zauber dem Spieler abverlangt, und
      // keine Buchhaltung, die ihn davor bewahrt.
      const pinFenster = this.pinWindowMsValue;
      this.pinWindowMsValue = 0;
      if (eintrag.mechanic === 'interruptibleCast' || pinFenster > 0) {
        // Ob er trifft, entscheidet sich erst, wenn die Wirkzeit um ist — bis
        // dahin kann der Spieler ihn abbrechen.
        this.castingValue = index;
        // Der Anfang für den Balken (Issue #327) — die Kampfzeit, zu der der
        // Eintrag fällig wurde, samt der Verzögerung, die STAY ihm mitgegeben
        // hat. Genau dieser Wert ist später nicht mehr herzuleiten.
        this.castingStartedAtMsValue = faelligAtMs + this.nextStrikeDelayMs;
        // **Die eskalierende Wiederholung** (11.2, Baustein 7; Issue #240):
        // Mit jedem Zyklus schrumpft das Fenster zum Unterbrechen. Die
        // Verkuerzung steht in den DATEN — sonst waere sie eine Regel des
        // Spiels statt eine Eigenschaft dieses Bosses.
        // **Das Fenster von PIN gilt nur für einen Eintrag, der KEINES hat.**
        // Traf er einen ohnehin unterbrechbaren, bleibt dessen eigene Wirkzeit
        // samt Eskalation stehen — PIN darf einen schrumpfenden Zauber nicht
        // heimlich wieder verlängern (#240: die Verkürzung ist eine Eigenschaft
        // des Bosses, keine Regel des Spiels).
        this.castLandsAtMsValue =
          eintrag.mechanic === 'interruptibleCast'
            ? faelligAtMs + this.escalatedCastTime(eintrag.castTimeMs, index)
            : faelligAtMs + pinFenster;
      } else {
        // Trifft sofort. Nicht zu verhindern, nur auszuhalten.
        this.hitPlayer(eintrag.damage, timeMs);
      }
      // **Der Abbruch, und zwar nur durch bestimmte Angriffe** (GAME-DESIGN
      // 7.1: „nur durch bestimmte, vorher erkennbare Boss-Angriffe. Normale
      // Treffer stören nicht"). Welche das sind, steht in den BOSS-Daten und
      // nicht hier — sonst bräche jeder Schlag jeden Zauber ab, und aus einer
      // lernbaren Mechanik würde Zufallsärger.
      //
      // Steht NACH dem Schlag: Der Treffer fällt in jedem Fall, der Abbruch ist
      // seine Nebenwirkung.
      if (eintrag.interruptsPlayer && this.cancelPlayerCast()) {
        this.castsInterruptedValue++;
        // **Die Rüstung greift** (RETORT, #480): Der Rest der Wartezeit wird
        // erlassen, nicht die volle — die verstrichene Zeit hat der Spieler
        // durchlebt.
        //
        // Hier und NICHT in `cancelPlayerCast`: Die Methode wird auch vom
        // Selbst-Abbruch gerufen. Wer sich rüstet und dann selbst abbricht,
        // hätte sich die Wartezeit gekauft.
        if (this.takeRetortArmed()) this.gcd.refund(this.gcd.remainingMs(timeMs));
      }
    }

    // **Die eigene Wirkzeit zuletzt** (Issue #304): nach allen Handlungen des
    // Bosses und vor den Tastendrücken dieses Takts. Ein abbrechender Schlag im
    // selben Takt muss den Zauber noch erwischen können — stünde die Auflösung
    // davor, käme der Schaden durch und der Abbruch ginge ins Leere.
    //
    // Im Simulator ist es derselbe Platz: `PlayerCastLands` steht zwischen
    // `BossCastLands` und `Wakeup` in der Rangfolge.
    // **Die Teilwirkungen einer Kanalisierung** (Issue #323), VOR der
    // Cast-Auflösung und an derselben Stelle in der Rangfolge: Ein
    // abbrechender Schlag dieses Takts hat oben schon geräumt, und dann fällt
    // hier nichts mehr — genau das ist „der erreichte Anteil".
    //
    // Eine `while`-Schleife und kein `if`: Bei einem Raster unter der
    // Takt-Länge fielen sonst Schritte aus. Sie endet sicher, weil jeder
    // Durchlauf den nächsten Termin um mindestens ein Raster nach hinten
    // schiebt.
    while (
      this.playerChannelNextTickAtMsValue >= 0 &&
      timeMs >= this.playerChannelNextTickAtMsValue &&
      this.playerChannelNextTickAtMsValue <= this.playerCastEndsAtMsValue
    ) {
      const index = this.playerCastingValue;
      const spell = index >= 0 ? this.spells[index] : undefined;
      if (spell === undefined) break;
      this.resolveChannelTick(index, spell, this.playerChannelNextTickAtMsValue);
      this.playerChannelNextTickAtMsValue += this.hasted(spell.channelTickMs);
    }

    if (this.playerCastingValue >= 0 && timeMs >= this.playerCastEndsAtMsValue) {
      const index = this.playerCastingValue;
      const spell = this.spells[index];
      const warKanal = spell !== undefined && spell.channelMs > 0;
      this.playerCastingValue = NO_SPELL;
      this.playerCastEndsAtMsValue = -1;
      this.playerCastStartedAtMsValue = -1;
      this.playerChannelNextTickAtMsValue = -1;
      // **Eine Kanalisierung löst am Ende NICHTS aus.** Ihre Wirkung ist über
      // die Dauer bereits gefallen; ein Aufruf hier gäbe sie ein zweites Mal,
      // in voller Höhe. Der Unterschied zwischen den beiden Typen ist genau
      // diese Zeile.
      if (spell !== undefined && !warKanal) {
        this.resolvePlayerCast(index, spell, timeMs, this.deferredOutcome, true);
      }
    }

    // Phasenwechsel an der Lebenspunkt-Schwelle. Die Phase kann nur STEIGEN:
    // Heilt der Boss später einmal (Baustein 5), verschwindet eine schon
    // gezeigte Mechanik nicht wieder.
    if (this.boss.phases.length > 0) {
      // Die gesenkte Schwelle aus ANCHOR (#100): Der Boss muss tiefer fallen,
      // bevor er wechselt. Gerechnet über den Anteil, nicht über die
      // Boss-Daten — die gehören dem Boss, nicht dem Kampf.
      const prozent = this.bossHpPercent + this.phaseHoldPercentValue;
      let neue = 0;
      for (let i = 0; i < this.boss.phases.length; i++) {
        const schwelle = this.boss.phases[i];
        if (schwelle !== undefined && prozent <= schwelle.atHpPercent) neue = i + 1;
      }
      if (neue > this.bossPhaseValue) {
        this.bossPhaseValue = neue;
        // **THE LONG GRIP wird hier scharf gestellt** (13.5): beim WECHSEL,
        // nicht beim Erreichen einer Phase. Ein Boss ohne Phasen kommt nie
        // hierher, und der Effekt greift dort zu Recht nie.
        this.freeControlSpendArmed = true;
        // **Verbraucht** (GAME-DESIGN 4.2b): Die Senkung galt für diesen einen
        // Wechsel. Der übernächste bekommt wieder die volle Schwelle — sonst
        // verschöbe ein Druck alle folgenden mit.
        this.phaseHoldPercentValue = 0;
        this.phaseHoldUntilMs = -1;
        // **Nummer 6 gehört seit #96 BEIDEN Seiten** — der Simulator schrieb
        // sie, das Spiel nicht. Gefunden, weil der Prüffall die Zahl je Art
        // vergleicht: Simulator 1, Spiel 0.
        //
        // Steht NACH dem Verbrauch, in derselben Reihenfolge wie
        // `Fight::check_phase` im Simulator. Am Zustand ändert das nichts —
        // aber wo die beiden Seiten dieselbe Stelle verschieden ordnen, wird
        // die nächste Ergänzung dort zur stillen Abweichung.
        this.log?.push(this.fightElapsedMs(timeMs), EVENT_PHASE, NO_SPELL_REF, neue);
      }
    }
  }

  /**
   * Ausgeteilten Schaden auf den Fortschritt buchen (Issue #218).
   *
   * **Die einzige Stelle, die `damageDealtValue` erhöht.** Vorher waren es
   * zwei — Wirkung und Brand-Tick —, und beide hätten die Fenster-Regel
   * einzeln kennen müssen. Genau das Muster, das der Krit-Wächter (#196) für
   * die Schadensfaktoren verhindert: Wer eine dritte Stelle anlegt, umgeht
   * die Regel, und der Fehler ist still.
   *
   * Im Unverwundbarkeits-Fenster kommt der Schaden **nicht** am Boss an und
   * zählt als verloren. Das ist nicht dasselbe wie `× 0`: Die Zahl bleibt
   * erhalten und gehört in die Fehleranalyse (GAME-DESIGN 15.3) — wer sie
   * verschwinden ließe, nähme dem Spieler die einzige Auskunft, die ihn beim
   * nächsten Versuch besser macht.
   *
   * SEIT #247 GIBT ES FORTSCHRITT OHNE ZAUBER: Die davonlaufende Reaktion
   * senkt den Ring von selbst. Sie kommt hier durch dieselbe Tür wie alles
   * andere — eine eigene Schreibstelle wäre genau die dritte, die dieser
   * Kommentar seit #218 verbietet.
   *
   * Zwei Dinge musste sie mitdenken, und beide stehen deshalb als Parameter da:
   *
   * - **`vomSpieler`.** Das Unverwundbarkeits-Fenster schützt den Boss vor
   *   SCHADEN. Eine Reaktion, die von selbst kippt, ist keiner — sie läuft im
   *   Fenster weiter. Liefe sie in den Zweig, zählte sie außerdem als „im
   *   Fenster verloren", und die Fehleranalyse (15.3) meldete dem Spieler
   *   Schaden, den er nie gemacht hat.
   * - **`spellIndex < 0`.** Ohne Zauber gibt es keine Spalte in der
   *   Schadenstabelle; ein negativer Index legte dort ein Feld an, das keiner
   *   Zeile entspricht.
   *
   * @returns ob der Schaden am Boss angekommen ist
   */
  private bookProgress(
    damage: number,
    spellIndex: SpellIndex,
    timeMs: number,
    vomSpieler = true,
  ): boolean {
    if (vomSpieler && this.raceEndsAtMs >= 0) {
      // **Verloren am Boss, aber nicht wirkungslos.** Der Schaden laedt das
      // Ziel; erreicht es die Schwelle, ist es zerstoert und das Fenster endet
      // frueh. Genau das macht "der beste Moment fuer Overdrive" wahr.
      this.damageLostToRaceValue += damage;
      const race = this.boss.race;
      if (race !== null) {
        this.raceProgressValue += damage;
        if (this.raceProgressValue >= race.threshold) {
          this.raceWonValue = true;
          this.raceEndsAtMs = -1;
        }
      }
      return false;
    }
    void timeMs;
    this.damageDealtValue += damage;
    // **Die Herkunft wird HIER festgehalten, nicht später erschlossen.** Nach
    // der Buchung ist sie fort: Der Ring weiß nicht mehr, wer ihn gesenkt hat.
    if (vomSpieler) this.playerDamageValue += damage;
    if (spellIndex >= 0) {
      this.damageBySpell[spellIndex] = (this.damageBySpell[spellIndex] ?? 0) + damage;
    }
    return true;
  }

  /**
   * Den Boss um den Betrag aus den Daten heilen (Baustein 5; Issue #218).
   *
   * > Ein Fehler macht Fortschritt rückgängig. Härteste Form von Konsequenz.
   *
   * **Ausgelöst vom durchgekommenen Boss-Zauber** — kein neuer Fehler-Begriff.
   * Wie viel er heilt, sagt der Boss-Datensatz; fehlt das Feld, heilt nichts.
   * Eine feste Kopplung gälte für alle Bosse, und 11.2 sagt „nur für einzelne".
   *
   * Der Fortschritt fällt nie unter null: Ein Boss, der mehr „geheilt" wird,
   * als je Schaden gefallen ist, hätte negativen Fortschritt, und jede
   * Prozentrechnung darüber wäre unsinnig.
   *
   * @returns wie viel wirklich zurückgedreht wurde
   */
  healBoss(): number {
    const heal = this.boss.bossHeal;
    if (heal === null) return 0;
    const wirklich = this.undoProgress(heal.amount);
    this.bossHealedValue += wirklich;
    return wirklich;
  }

  /**
   * Fortschritt am Ring zurückdrehen — **die einzige Stelle, die ihn senkt**.
   *
   * Zwei Dinge drehen zurück, und sie haben nichts miteinander zu tun außer
   * dieser Zeile:
   *
   * | Wer | Warum |
   * | --- | --- |
   * | `healBoss` | Ein Fehler macht Fortschritt rückgängig (Baustein 5, #218) |
   * | `stabilise` | Der Spieler hält die Reaktion (Baustein 6, #247) |
   *
   * Das eine ist Strafe, das andere Antwort — aber beide senken denselben
   * Wert, und die Untergrenze gilt für beide. Sie zweimal zu schreiben wäre
   * dasselbe Muster, das `bookProgress` seit #218 verhindert; der Prüffall
   * `progress-writer-source` zählt die Schreibstellen und bleibt dadurch bei
   * vier.
   *
   * **Nie unter null:** Ein Boss mit negativem Fortschritt hätte einen Ring
   * über 100 %, und jede Prozentrechnung darüber wäre unsinnig.
   *
   * @returns wie viel wirklich zurückgedreht wurde
   */
  private undoProgress(amount: number): number {
    if (amount <= 0 || this.damageDealtValue <= 0) return 0;
    const wirklich = Math.min(amount, this.damageDealtValue);
    this.damageDealtValue -= wirklich;
    return wirklich;
  }

  /**
   * Die davonlaufende Reaktion zurückdrängen (11.2, Baustein 6; Issue #247).
   *
   * > **Statt** Schaden zu machen muss eine kippende Reaktion stabilisiert
   * > werden.
   *
   * **Schutz und Heilung heben den Ring um IHREN EIGENEN Betrag.** Eine zweite
   * Zahl „stabilisiert um X" neben `shield.amount` wäre dieselbe Größe an zwei
   * Orten, und die zweite liefe beim nächsten Balancing weg.
   *
   * TEMPER BLEIBT DRAUSSEN, und das ist die Grenze des Bausteins, keine Lücke:
   * Er wandelt ausgeteilten Schaden in Schutz. Würde dieser Schutz
   * stabilisieren, dann stabilisierte man **durch Schaden machen** — und die
   * Bedingung, die den ganzen Kampf ausmacht, wäre aufgehoben. GAME-DESIGN 4.1
   * nennt die drei Wege „nebeneinander": drei Spielweisen, nicht drei Antworten
   * auf jede Frage.
   *
   * Ohne das Boss-Feld geschieht nichts — sonst hätte jeder gewöhnliche Kampf
   * einen versteckten Weg, Fortschritt zu verlieren.
   */
  private stabilise(amount: number): void {
    if (this.boss.runaway === null) return;
    this.stabilisedValue += this.undoProgress(amount);
  }

  /**
   * Die Wirkzeit nach der Zahl der Zyklen (11.2, Baustein 7; Issue #240).
   *
   * `minCastTimeMs` ist keine Feinheit: Ohne Untergrenze schrumpfte das
   * Fenster irgendwann auf einen Takt, und der Baustein wäre kein Anspruch
   * mehr, sondern eine Unmöglichkeit — 11.7 verwirft genau das.
   */
  private escalatedCastTime(base: number, eintragIndex: number): number {
    const esc = this.boss.escalation;
    const laenge = this.boss.timeline.length;
    if (esc === null || laenge === 0 || this.boss.timelineLoopMs === null) return base;
    // **Der Zyklus DIESES Eintrags**, nicht der Stand des Zeigers. Der ist an
    // dieser Stelle bereits erhoeht — mein erster Entwurf las ihn und gab dem
    // ERSTEN Zauber schon die Verkuerzung des zweiten Zyklus. Gemessen:
    // [2500, 2000, 1500, 1000] statt [3000, 2500, 2000, 1500].
    const zyklus = Math.floor(eintragIndex / laenge);
    if (zyklus <= 0) return base;
    const gekuerzt = base - esc.castTimeShrinkMs * zyklus;
    return gekuerzt < esc.minCastTimeMs ? esc.minCastTimeMs : gekuerzt;
  }

  /**
   * Die Schwächung entfernen (11.2, Baustein 8; Issue #240).
   *
   * **Heilung steht bewusst nicht in `clearedBy`** — das ist der ganze
   * Baustein. Wer sie einträgt, hat ihn in eine gewöhnliche Schwächung
   * zurückverwandelt.
   *
   * @returns ob etwas zu entfernen war
   */
  private clearAffliction(spellId: string): boolean {
    const def = this.boss.affliction;
    if (def === null || !this.afflictedValue) return false;
    if (!def.clearedBy.includes(spellId)) return false;
    this.afflictedValue = false;
    this.afflictionsClearedValue++;
    return true;
  }

  /**
   * Prüft, ob der Boss gefallen ist. Nach `stepPassive`, vor dem Wirken.
   *
   * **BEIDE AUSGÄNGE KEHREN SICH UM, wenn der Boss das umgekehrte Ziel trägt**
   * (11.2, Baustein 6; Issue #247):
   *
   * | | Ring auf 0 | Zeit abgelaufen |
   * | --- | --- | --- |
   * | gewöhnlich | Sieg | Niederlage |
   * | `runaway` | Niederlage | Sieg |
   *
   * Nicht eine Bedingung, sondern zwei — sonst wäre es keine Umkehrung des
   * ZIELS, sondern nur ein anderer Verlustweg. Der Sieg ist dort das
   * Überstehen der Frist, wie beim Vorbild aus 11.2; die Fiktion trägt es von
   * selbst, weil der Überdruck laut 7.4 entsteht, **weil** die Reaktion außer
   * Kontrolle gerät (GAME-DESIGN 7.4). Wer sie hält, bei dem entsteht er nicht.
   *
   * Die Zustände bleiben dieselben: `FIGHT_OVERPRESSURE` ist in beiden Fällen
   * die Niederlage. Ein eigener Zustand wäre eine dritte Wahrheit über dasselbe
   * Ende, und die Auswertung müsste sie überall mitführen.
   */
  checkDefeat(timeMs: number): boolean {
    if (this.stateValue !== FIGHT_RUNNING) return true;
    const umgekehrt = this.boss.runaway !== null;
    if (this.damageDealtValue >= this.boss.hp) {
      this.stateValue = umgekehrt ? FIGHT_OVERPRESSURE : FIGHT_WON;
      this.endedAtMs = timeMs;
      return true;
    }
    // **Der Spielertod, und er steht NACH dem Boss-Tod** (GAME-DESIGN A.3,
    // Issue #581).
    //
    // Die Reihenfolge ist nicht frei: `Fight::replay` prüft in `fight.rs`
    // erst `killed_at`, dann `died_at`. Fallen beide im selben Takt, gewinnt
    // der Spieler — auf beiden Seiten dieselbe Entscheidung, sonst liefen sie
    // genau in dem Takt auseinander, den niemand vorhersieht.
    //
    // Bis zum 2026-08-07 fehlte dieser Zweig ganz: `checkDefeat` hieß so und
    // prüfte den Tod des Spielers nicht. Er wurde in `hitPlayer` gesetzt und
    // von keiner Stelle als Ende gelesen — ein toter Spieler sammelte weiter
    // Schaden und gewann. Gemessen an `boss-2-2`: Tod bei 15000 ms, Boss
    // gefallen bei 52510.
    //
    // **Die Umkehrung durch `runaway` gilt hier nicht.** Sie dreht das ZIEL um
    // (Ring auf 0 gegen Frist überstehen); der eigene Tod ist in beiden Fällen
    // dasselbe Ende. Ein `umgekehrt ? … : …` an dieser Stelle hieße, dass ein
    // toter Spieler gewinnen kann, wenn der Boss davonläuft.
    if (this.diedAtMsValue >= 0) {
      this.stateValue = FIGHT_DIED;
      this.endedAtMs = timeMs;
      return true;
    }
    // Der Überdruck bemisst sich an der KAMPFuhr, nicht an der durchlaufenden:
    // Die Vorbereitung darf nichts von der Kampfzeit abziehen.
    if (this.fightElapsedMs(timeMs) >= this.boss.overpressureMs) {
      this.stateValue = umgekehrt ? FIGHT_WON : FIGHT_OVERPRESSURE;
      this.endedAtMs = timeMs;
      return true;
    }
    return false;
  }

  // ----------------------------------------------------------------- Wirken

  /**
   * Einen Zauber wirken.
   *
   * Seit Issue #304 sind das **zwei** Wege: Ein Sofort-Zauber wirkt hier und
   * jetzt, ein Zauber mit Wirkzeit beginnt nur — seine Wirkung fällt in
   * `resolvePlayerCast`, wenn die Zeit um ist. Was dazwischen liegt, entscheidet
   * allein `spell.castTimeMs`; im Code steht keine Liste, welcher Zauber wie
   * läuft (Leitplanke 6).
   *
   * @returns Das wiederverwendete Ergebnis-Objekt. Wer es behalten will, kopiert
   *   die Felder heraus — beim nächsten Aufruf steht etwas anderes darin.
   */
  cast(index: SpellIndex, timeMs: number): CastOutcome {
    this.nowMsForBuffs = timeMs;
    const outcome = this.outcome;
    outcome.spellIndex = index;
    outcome.damage = 0;
    outcome.resourceBefore = this.resourceValue;
    outcome.bonusApplied = false;

    const spell = this.spells[index];
    if (spell === undefined) {
      outcome.result = CAST_UNKNOWN;
      return outcome;
    }
    if (this.stateValue !== FIGHT_RUNNING) {
      outcome.result = CAST_FIGHT_OVER;
      return outcome;
    }
    // **Ein toter Spieler wirkt nicht** (GAME-DESIGN A.3, Issue #567) — und das
    // Ergebnis ist `CAST_FIGHT_OVER`, weil es für den Drückenden dasselbe ist:
    // Der Versuch ist vorbei. Ein eigenes Ergebnis dafür wäre eine
    // Design-Entscheidung; A.3 verlangt sie nicht.
    //
    // Diese Abfrage steht ZUSÄTZLICH zu der in `stepPassive`, nicht statt ihr:
    // `cast` kommt vom Tastendruck und läuft nicht über den Takt. Wer nur eine
    // von beiden setzt, hat die Regel halb umgesetzt — genau das war der
    // Befund, aus dem dieses Issue entstand.
    if (this.diedAtMsValue >= 0) {
      outcome.result = CAST_FIGHT_OVER;
      return outcome;
    }
    // **Wer keine Wartezeit auslöst, wird auch von keiner gesperrt**
    // (GAME-DESIGN 6.6/10.4, Issue #110).
    //
    // Bis hierher galt die Sperre für jeden Zauber. Für die drei aus Schritt 1
    // war das richtig — sie lösen alle eine aus. Für den Unterbrecher wäre es
    // die Ausnahme mit falschem Vorzeichen: Er löste zwar keine Wartezeit aus,
    // könnte aber während einer laufenden nicht gewirkt werden. Gebraucht wird
    // er genau dann.
    if (spell.triggersGcd && !this.gcd.isReady(timeMs)) {
      outcome.result = CAST_ON_GCD;
      return outcome;
    }
    if (!this.isOffCooldown(index, timeMs)) {
      outcome.result = CAST_ON_COOLDOWN;
      return outcome;
    }

    // **Ab hier wird gewirkt — und jede Wirkung bricht einen laufenden Zauber
    // ab** (GAME-DESIGN 7.1, „Was einen laufenden Zauber abbricht", entschieden
    // am 2026-08-05):
    //
    // > Jede Wirkung des Spielers bricht einen laufenden Cast oder eine
    // > Kanalisierung ab. Jede, ohne Ausnahme — auch das Unterbrechen.
    //
    // **Die Stelle ist mit Absicht HIER und nicht weiter oben.** Ein Druck, den
    // die Wartezeit oder die Abklingzeit abweist, führt zu keiner Wirkung — er
    // darf also auch nichts abbrechen. Stünde der Abbruch vor den zwei
    // Prüfungen, kostete ein Fehlgriff den laufenden Zauber, und aus einer
    // Entscheidung würde eine Falle.
    //
    // Folge, die man kennen muss: Solange die globale Wartezeit läuft, kann nur
    // ein Zauber ohne sie (der Unterbrecher) abbrechen. Bei 1500 ms Wartezeit
    // und 2000 ms Wirkzeit bleiben den übrigen 500 ms Fenster.
    if (this.cancelPlayerCast()) this.castsCancelledValue++;

    // **Der Zauber mit Wirkzeit endet hier** (GAME-DESIGN 7.1, Issue #304).
    //
    // Die globale Wartezeit läuft ab dem BEGINN — 7.1 misst die Wirkzeit in
    // Wartezeiten („1,5–2,5 s, also ein bis knapp zwei globale Wartezeiten"),
    // und dieser Vergleich ergibt nur einen Sinn, wenn beide zugleich laufen.
    //
    // Die ABKLINGZEIT läuft dagegen NICHT an. Sie gehört zur Wirkung und steht
    // deshalb in `resolvePlayerCast` — daran hängt die zweite Hälfte von
    // GAME-DESIGN 7.1: „Bei Abbruch geht der Zauber verloren, die Abklingzeit
    // läuft aber nicht an."
    // **Die Kanalisierung** (Issue #323, GAME-DESIGN 7.2). Sie steht VOR der
    // Wirkzeit, weil der Parser beides zugleich ausschliesst — die Reihenfolge
    // ist damit keine Entscheidung, sondern nur eine Lesart.
    //
    // **Design-Gesetz 1:** EIN Tastendruck startet sie, dann läuft sie von
    // selbst. Es gibt hier keinen gehaltenen Zustand und keine zweite Taste;
    // was folgt, hängt allein an `timeMs`.
    const kanal = this.hasted(spell.channelMs);
    if (kanal > 0) {
      this.playerCastingValue = index;
      this.playerCastStartedAtMsValue = timeMs;
      this.playerCastEndsAtMsValue = timeMs + kanal;
      // Die erste Teilwirkung fällt NACH dem ersten Raster-Schritt, nicht
      // sofort: Sonst bekäme ein Kanal, der im selben Moment abbricht, schon
      // eine Wirkung — und „nach 0 s" ergäbe mehr als null.
      this.playerChannelNextTickAtMsValue = timeMs + this.hasted(spell.channelTickMs);
      if (spell.triggersGcd && !this.takeFreeGcd()) this.gcd.trigger(timeMs);
      outcome.result = CAST_STARTED;
      return outcome;
    }

    const wirkzeit = this.hasted(spell.castTimeMs);
    if (wirkzeit > 0) {
      this.playerCastingValue = index;
      this.playerCastStartedAtMsValue = timeMs;
      this.playerCastEndsAtMsValue = timeMs + wirkzeit;
      // **Der Merker geht VOR der Eigenschaft** (SUBLIME, #480).
      if (spell.triggersGcd && !this.takeFreeGcd()) this.gcd.trigger(timeMs);
      // **Kein Protokoll-Eintrag für den Beginn.** Das Protokoll trägt die
      // Fehleranalyse und hat einen gemeinsamen Nummernraum mit dem Simulator
      // (#134); eine neue Art dafür verschöbe jede eingecheckte
      // Vergleichsdatei. Der Eintrag fällt mit der Wirkung — dort, wo auch der
      // Schaden steht.
      outcome.result = CAST_STARTED;
      return outcome;
    }

    this.resolvePlayerCast(index, spell, timeMs, outcome);
    outcome.result = CAST_OK;
    return outcome;
  }

  /**
   * Die Wirkung eines Zaubers — der Teil, der bei einer Wirkzeit **am Ende**
   * fällt (Issue #304, AC2).
   *
   * Bis #304 stand das alles im Rumpf von `cast()`. Herausgezogen und nicht
   * kopiert: Ein Sofort-Zauber und ein Zauber mit Wirkzeit teilen sich diese
   * eine Stelle, sonst gäbe es zwei Schadensrechnungen, die auseinanderlaufen.
   *
   * **Der Kritwurf gehört hierher und nicht an den Beginn.** Ein Wurf beim
   * Drücken drehte den Zufallsstrom zu einem anderen Zeitpunkt als der
   * Simulator, und ein abgebrochener Zauber hätte einen Wurf verbraucht, dessen
   * Ergebnis niemand je sieht — beides ließe jeden folgenden Kampf
   * auseinanderlaufen.
   */
  private resolvePlayerCast(
    index: SpellIndex,
    spell: SpellDef,
    timeMs: number,
    outcome: CastOutcome,
    wirkzeitLief = false,
  ): void {
    // Unterbrechen (GAME-DESIGN 11.2, Baustein 9).
    //
    // **Kein Sonderfall in der Wartezeit-Rechnung.** Dass dieser Zauber keine
    // globale Wartezeit auslöst, steht weiter unten als `spell.triggersGcd` und
    // kommt aus den Daten — hier wäre eine zweite Stelle, an der dieselbe Regel
    // hängt, und zwei Stellen laufen auseinander.
    // **Wirkt dieser Wurf zweimal?** (CONDENSE, Issue #480.)
    //
    // Der Merker wird HIER verbraucht, vor jeder Wirkung — das Setzen steht
    // wie bei SUBLIME weit unten, hinter allem. Stünden beide beieinander,
    // verdoppelte CONDENSE sich selbst statt den nächsten.
    //
    // Die Wurfzahl bleibt bei EINS: ein Wurf, der zweimal wirkt, kein zweiter
    // Wurf. Eine Abklingzeit, eine Wartezeit, ein Cast-Eintrag im Log.
    const anwendungen = this.takeDoubleCast() ? 2 : 1;

    if (spell.interrupt === true) this.interruptBossCast(timeMs);

    // **Das Verhängnis senken** (Baustein 2; Issue #561). Genau dieselbe Bauart
    // eine Zeile höher: Ob der Zauber es tut, steht in den Daten; WIE VIEL er
    // senkt, steht beim Boss (`clearAmount`).
    //
    // `clearDoom()` gab es seit #216 und niemand rief es. Der Zähler lief voll
    // und tötete, ohne dass eine Taste dagegen existierte — `boss-1-2` war
    // damit unschaffbar, und das ist kein Prüffall-Problem, sondern ein
    // kaputtes Spiel.
    if (spell.clearsDoom === true) this.clearDoom();

    // Die drei Wege der Koagulation (Issue #79). Sie stehen VOR dem Schaden,
    // weil ein Zauber, der beides täte, erst schützen und dann zuschlagen
    // soll — heute ohne Wirkung (keiner macht beides) und deshalb jetzt
    // festzulegen und nicht dann.
    this.applyCoagulation(spell, timeMs, index);
    this.applyFixation(spell, timeMs);
    this.applyConversion(spell, timeMs);
    this.applyPutrefaction(index, spell, timeMs);
    this.openSynergyWindow(spell, timeMs);

    // **Ein Wurf für die ganze Wirkung**, und er fällt genau dann, wenn der
    // Zauber **laut seinen Daten** Schaden austeilen kann — nicht danach, ob am
    // Ende ein Betrag herauskommt.
    //
    // Der Unterschied betrifft den Entlade-Zauber ohne Punkte: Er würfelt,
    // obwohl sein Schaden null ist. Das ist Absicht. Die Bedingung ist aus den
    // Daten ablesbar und hängt nicht am Zustand — damit ist sie auf beiden
    // Seiten trivial gleichzuhalten. Eine Bedingung über den ausgerechneten
    // Betrag wäre es nicht, und ein Zufallsstrom, der auf den zwei Seiten
    // verschieden oft gedreht wird, lässt den ganzen folgenden Kampf
    // auseinanderlaufen.
    // **Erst prüfen, dann öffnen** (Issue #318). Ein Zauber, der sein eigener
    // Partner wäre, nutzte sonst das Fenster, das er im selben Zug aufmacht.
    this.applySynergyBonus(index, timeMs);

    // WEDGE teilt Schaden aus, ohne `baseDamage` oder `spender` zu führen —
    // seine Quelle sind die Kontroll-Punkte (Issue #478).
    const teiltSchadenAus =
      spell.baseDamage > 0 || spell.spender !== undefined || this.pendingConvertedDamage > 0;
    const kritet = teiltSchadenAus ? this.rollCrit(STREAM_CRIT) : false;
    // **Der Widerstand dieses Zaubers, vor der Rechnung** (Issue #328). Über
    // `setDamageFactor`, damit die Krit-Fassung mitzieht — dieselbe Stelle,
    // die #196 als einzige Schreibstelle festgelegt hat.
    this.setDamageFactor('resistance', this.resistanceOfSpell[index] ?? 1);
    // Der Stapel-Zuschlag DIESES Zaubers, aus demselben Grund wie der
    // Widerstand daneben: Er gehoert der Wirkung, nicht dem Kampf (#484).
    this.applyDecayStackBonus(index);
    // Und der Entlader-Zuschlag des globalen Weges (#419), aus demselben Grund
    // an derselben Stelle.
    this.applySpenderBonus(index);
    const castFactors = this.factorsFor(kritet);

    // EMBER und PYROCLASM (Issue #517). Beide VOR der Schadensrechnung: EMBER,
    // damit ein Brand, den dieser Zauber selbst legt, nicht schon verlaengert
    // wird; PYROCLASM, weil sein Beitrag in denselben Betrag gehoert wie der
    // Grundschaden.
    if (spell.dotExtendTicks !== undefined) this.extendActiveDots(timeMs, spell.dotExtendTicks);
    // HARDEN und SEPSIS (#523), aus demselben Grund an derselben Stelle.
    if (spell.extendShieldMs !== undefined) this.extendShield(timeMs, spell.extendShieldMs);
    if (spell.extendDecayMs !== undefined) this.extendDecayStacks(spell.extendDecayMs);

    // **Die Schleife für CONDENSE** (#480): einmal, oder zweimal wenn der
    // Merker stand.
    //
    // Sie umschliesst genau die MENGEN — die vier Schadensquellen und ihre
    // Buchung. Die ZUSTÄNDE davor (unterbrechen, Verhängnis, Synergie-Fenster)
    // und danach (Verstärkung, Brand) bleiben ausserhalb: Zweimal zu
    // unterbrechen unterbricht nicht doppelt, und ein Fenster zweimal zu
    // öffnen verschöbe seine Dauer.
    //
    // **Der Krit-Wurf fällt oben, EINMAL für beide Anwendungen** — dieselbe
    // Regel wie beim Schlüsseltalent aus #418, das denselben Satz trägt.
    //
    // **Warum die Buchung mit hineingehört, obwohl sie hier eine Summe
    // bucht**: Der Simulator ruft `deal` je Schadensquelle einzeln und legt
    // damit je Quelle einen Log-Eintrag an. Dass beide Seiten heute gleich
    // viele Einträge schreiben, liegt allein daran, dass kein Zauber ZWEI
    // Quellen führt — der Kommentar über dem Schadens-Ereignis im Simulator
    // sagt es ausdrücklich. CONDENSE erzeugt genau diesen Fall zum ersten Mal.
    // Bliebe die Buchung draussen, schriebe der Simulator zwei Einträge und
    // das Spiel einen.
    let gesamtschaden = 0;
    for (let anwendung = 0; anwendung < anwendungen; anwendung++) {
      let damage = 0;
      if (spell.baseDamage > 0) damage += computeDamage(spell.baseDamage, castFactors);
      if (spell.consumesDots === true) damage += this.consumeActiveDots(timeMs);
      // **Der umgewandelte Schaden geht durch dieselben Faktoren** (Issue #478):
      // Krit, Widerstand, Verstärkung. Ein Treffer, den kein Faktor berührt,
      // wäre der einzige im Spiel — und niemand fände es, weil er trotzdem
      // Schaden macht.
      if (this.pendingConvertedDamage > 0) {
        damage += computeDamage(this.pendingConvertedDamage, castFactors);
        this.pendingConvertedDamage = 0;
      }

      const spender = spell.spender;
      if (spender !== undefined && spender.resource === this.school.resource.id) {
        // **Das erste Schlüsseltalent** (Issue #418, GAME-DESIGN 13.09): „Der
        // Entlade-Zauber verbraucht nur die HÄLFTE der Hitze, wirkt dafür
        // ZWEIMAL."
        //
        // Es trägt keine Zahl, sondern schaltet ein Verhalten um — deshalb
        // steht hier eine Verzweigung und kein Faktor. Ein Schlüsseltalent, das
        // sich als Faktor ausdrücken ließe, wäre keines.
        const geteilt = this.hasKeystone(KEYSTONE_SPLIT_SPENDER);
        // **Ganzzahlig geteilt, und der Rest bleibt liegen** (GAME-DESIGN 18.4).
        // Bei 15 Hitze verbraucht jede Hälfte 7, eine bleibt übrig — nicht 7,5.
        const proSchlag = geteilt ? Math.floor(this.resourceValue / 2) : this.resourceValue;
        const schlaege = geteilt ? 2 : 1;

        for (let i = 0; i < schlaege; i++) {
          // **Der Vorrat wird VOR dem zweiten Schlag verbraucht**, sonst
          // rechnete der zweite mit demselben Stand wie der erste — und aus
          // „halb, dafür zweimal" würde „ganz, dafür zweimal".
          const amount = geteilt ? Math.min(proSchlag, this.resourceValue) : this.resourceValue;
          if (geteilt && amount === 0) break;

          let raw = spender.damagePerPoint * amount;
          if (amount >= spender.bonusThreshold) {
            // "ab 15 Hitze", nicht "über 14": Die Schwelle selbst zählt mit.
            raw *= spender.bonusMultiplier;
            outcome.bonusApplied = true;
          }
          damage += computeDamage(raw, castFactors);
          if (geteilt) this.resourceValue -= amount;
          else if (spender.consumesAll) {
            // **ASHEN REGIMEN** (13.5, Sechser-Bonus): Ein Drittel der
            // angesammelten Hitze bleibt stehen, statt vollständig zu
            // verbrauchen.
            //
            // **Abgerundet, und der Rest verfällt** (GAME-DESIGN 18.4): Bei
            // 20 Hitze bleiben 6 stehen, nicht 6,67. Ein Bruchteil hier
            // driftete über hunderte Würfe, und Spiel und Simulator müssten
            // auf dieselbe Zahl kommen.
            //
            // **Nach dem Schaden, nicht davor**: `amount` ist oben schon in
            // den Treffer eingegangen. Der Effekt schenkt Vorrat für den
            // NÄCHSTEN Wurf; er verbilligt diesen hier nicht.
            this.resourceValue = this.hasSetEffect(SET_EFFECT_ASHEN_REGIMEN)
              ? Math.floor(amount / 3)
              : 0;
          }
        }
      }

      if (damage > 0) {
        this.bookProgress(damage, index, timeMs);
        this.convertDamageToShield(damage, timeMs);
        this.reportHit(damage, HIT_DIRECT, kritet, timeMs);
        this.log?.push(this.fightElapsedMs(timeMs), EVENT_DAMAGE, index, damage);
      }
      gesamtschaden += damage;
    }

    // **NECROSIS räumt NACH seiner eigenen Berechnung ab** (Issue #479).
    //
    // `castFactors` steht oben schon fest und trägt den Stapel-Bonus aus
    // `perDecayStackPercent`. Der Entlader rechnet also MIT den Stapeln und
    // nimmt sie erst danach weg — andersherum wäre er der einzige Zauber der
    // Schule, der von der Fäulnis nichts hat.
    //
    // Er addiert nichts zu `damage`: Sein Ertrag steckt bereits im Faktor,
    // nicht in einem zweiten Betrag. Das unterscheidet ihn von PYROCLASM in
    // der Schleife, dessen Brände einen eigenen Betrag beisteuern.
    //
    // **Er steht NACH der CONDENSE-Schleife und nicht darin** (#480): Stapel
    // abzuräumen ist ein ZUSTAND, keine Menge. Ein zweiter Durchlauf fände
    // ohnehin nichts mehr vor — aber die Stelle folgt der Regel, nicht dem
    // Zufall, dass es hier egal wäre. Stünde er DAVOR, rechnete keine der
    // beiden Anwendungen mit den Stapeln, und der Absatz oben wäre verletzt.
    if (spell.consumesDecayStacks === true) this.consumeDecayStacks();

    // **Ein Schaden-Ereignis je WURF, mit der Summe** — nicht eines je
    // Anwendung. Genau so steht es im Simulator, und der Kommentar dort
    // begründet es: Das Spiel meldet einmal, auch wenn mehrere Quellen
    // beigetragen haben. Deshalb steht dieses Ereignis ausserhalb der
    // Schleife, während die Treffer-Buchung darin bleibt.
    const damage = gesamtschaden;
    if (damage > 0) {
      this.events.emit(RULE_DAMAGE, index, damage, timeMs);
      // **Hier stand bis #177: „Krit ruht in Schritt 1, ein Auslöser wäre
      // toter Code — die Kennung steht bereit, der Ort ist benannt."** Der Ort
      // war richtig benannt; jetzt hat er einen Fall. Der Auslöser steht NACH
      // `RULE_DAMAGE`, weil ein Krit eine Aussage über einen Treffer ist, den
      // es schon gegeben hat.
      if (kritet) this.events.emit(RULE_CRIT, index, damage, timeMs);
    }

    // Die Verstärkung wird NACH dem Schaden angelegt (Issue #420): Ein Zauber
    // verstärkt die folgenden, nicht sich selbst. Andernfalls trüge der
    // auslösende Zauber seinen eigenen Bonus, und das Fenster wäre in Wahrheit
    // einen Zauber länger als die Daten sagen.
    const buff = spell.buff;
    if (buff !== undefined) {
      // **Die Fensterlänge geht durch `hasted`**, wie beim Synergie-Fenster
      // (Zeile 1489). Genau darin liegt der Zweck der Zählung in Wartezeiten:
      // Wer schneller wirkt, hat ein kürzeres Fenster in Sekunden — und
      // trotzdem dieselbe Zahl Zauber darin (GAME-DESIGN 5).
      // **Die Zielgröße wählt den Speicher** (Issue #489). Fehlt sie, ist es
      // der Schaden — so waren alle Verstärkungen vor #489 gemeint.
      const speicher = this.buffSpeicherFuer(buff.target);
      speicher.applyForGcds(buff.kind, buff.factor, buff.gcds, timeMs, this.hasted(GCD_MS));
      // Nur der Schadens-Faktor geht in `factors`; die drei anderen werden an
      // ihrer Rechenstelle gelesen.
      if (speicher === this.buffs) this.updateBuffFactor(timeMs);
    }

    const dot = spell.dot;
    if (dot !== undefined) {
      // Erneuern setzt zurück: Die verbleibende Restzeit ist verloren, und mit
      // ihr die Ticks, die darin noch gekommen wären (GAME-DESIGN 7.2).
      if (this.isDotActive(index, timeMs)) {
        const remaining = this.dotRemainingMs(index, timeMs);
        this.dotTicksLost[index] =
          (this.dotTicksLost[index] ?? 0) + Math.floor(remaining / this.dotInterval(dot));
      }
      // **Die Dauer bleibt unangetastet** (GAME-DESIGN 6.7). Tempo macht die
      // Ticks dichter, nicht das Fenster kürzer — sonst verschöbe sich der
      // beste Erneuerungszeitpunkt (7.2) bei jedem Ausrüstungswechsel.
      // **Die Laufzeit trägt den Talent-Faktor** (#419, Weg „hold"). Länger
      // heißt MEHR Ticks, nicht langsamere — der Abstand bleibt unberührt,
      // den bewegt `tickBuffs` an anderer Stelle.
      //
      // Ganzzahlig abgerundet: Eine Laufzeit ist eine Zeit auf dem Takt-Raster,
      // und `Math.floor` hat in Rust eine exakte Entsprechung.
      this.dotExpiresAtMs[index] =
        timeMs + Math.floor(dot.durationMs * talentDotDurationFactor(this.talents));
      this.dotNextTickAtMs[index] = timeMs + this.dotInterval(dot);
    }

    // **Alle Abklingzeiten skalieren mit** (GAME-DESIGN 6.7) — anders als in
    // WoW, wo eine Ausnahmeliste je Fähigkeit entscheidet. Abklingzeiten sind
    // bei uns die alleinige Schwierigkeit (2.1); eine Liste, welcher Zauber
    // mitskaliert, wäre eine zweite Regel zum Auswendiglernen.
    this.cooldownReadyAtMs[index] = timeMs + this.hasted(spell.cooldownMs);
    // **Nur wenn sie nicht schon läuft** (Issue #304): Bei einer Wirkzeit ist
    // die Wartezeit beim BEGINN gestartet worden. Ein zweiter Start hier
    // verlängerte sie um die volle Länge, und aus „ein bis knapp zwei
    // Wartezeiten" (7.1) würden drei.
    if (spell.triggersGcd && !wirkzeitLief && !this.takeFreeGcd()) this.gcd.trigger(timeMs);

    // **Rüsten** (RETORT, #480): Der nächste vom BOSS abgebrochene Wurf
    // kostet keine Wartezeit.
    //
    // Steht bei den zwei anderen Merkern, hinter allem, was sie verbrauchen
    // könnte. RETORTs eigene Wartezeit ist eine Zeile höher schon gestartet
    // und bleibt — er zahlt wie jeder andere Zauber, und genau deshalb bleibt
    // Design-Gesetz 6.6 unberührt.
    if (spell.refundsInterruptedGcd === true) this.retortArmedValue = true;

    // **Den NÄCHSTEN Wurf befreien** (SUBLIME, #480). Bewusst HINTER beiden
    // GCD-Stellen: Weiter oben hätte SUBLIME als Sofort-Zauber seinen eigenen
    // Merker eine Zeile später verbraucht und sich selbst befreit statt den
    // nächsten. Der Fehler wäre grün geblieben — die Wartezeit fiele ja aus,
    // nur bei der falschen Zeile.
    if (spell.freesNextGcd === true) this.nextCastFreeGcdValue = true;
    if (spell.doublesNextCast === true) this.nextCastDoublesValue = true;
    this.castCounts[index] = (this.castCounts[index] ?? 0) + 1;

    this.log?.push(this.fightElapsedMs(timeMs), EVENT_CAST, index, damage);
    // Zuletzt: Die Wirkung ist abgeschlossen — Schaden, Effekt über Zeit,
    // Vorrat, Abklingzeit und Wartezeit stehen.
    //
    // **Das Ereignis feuert so oft, wie der Wurf gewirkt hat** (CONDENSE,
    // #480) — der Cast-Eintrag darüber genau einmal. Daran hängen die beiden
    // Mechaniken, die Mengen aufbauen: der Vorrat und die Verkürzung aller
    // Abklingzeiten. Sie zweimal ansetzen zu lassen ist dieselbe Regel wie
    // beim Schaden, und sie kostet keine Aufzählung — sie setzt dort an, wo
    // die Mengen ohnehin entstehen.
    for (let anwendung = 0; anwendung < anwendungen; anwendung++) {
      this.events.emit(RULE_SPELL_CAST, index, damage, timeMs);
    }

    // **Die Schwaechung ohne Heil-Loesung** (11.2, Baustein 8; Issue #240).
    // Steht dieser Zauber in `clearedBy`, ist sie weg — und Heilung steht dort
    // bewusst NICHT, das ist der ganze Baustein.
    this.clearAffliction(spell.id);

    outcome.damage = damage;
  }

  /**
   * Die laufende Wirkzeit abbrechen (GAME-DESIGN 7.1, Issue #304, AC4).
   *
   * > Bei Abbruch geht der Zauber verloren, **die Abklingzeit läuft aber nicht
   * > an.**
   *
   * **Die zweite Hälfte ist die eigentliche Regel**, und sie steht hier nicht
   * als Zeile, sondern als Abwesenheit einer: Die Abklingzeit wird in
   * `resolvePlayerCast` gesetzt, und die läuft bei einem Abbruch nie. Wer sie
   * beim Beginn setzte, müsste sie hier von Hand zurücknehmen — und genau das
   * wäre die Stelle, die man vergisst.
   *
   * # ZWEI AUFRUFER, und der zweite ist der häufigere
   *
   * | Auslöser | wo |
   * | --- | --- |
   * | Ein Boss-Angriff mit `interruptsPlayer` | `stepPassive`, in der Zeitleiste |
   * | **Jede eigene Wirkung** | `cast`, nach Wartezeit und Abklingzeit |
   *
   * Der zweite kam am 2026-08-05 dazu, als der Auftraggeber die Regel
   * umgedreht hat. Beide räumen dasselbe Feld; **gezählt wird getrennt**, weil
   * „der Boss hat es genommen" und „ich habe es weggeworfen" für den Spieler
   * nichts miteinander zu tun haben.
   *
   * # Wo eine Kanalisierung ansetzen wird (Issue #323)
   *
   * Sie verliert bei Abbruch **nicht** alles, sondern behält den erreichten
   * Anteil. Diese Methode ist die Stelle dafür: Sie kennt den Zauber und den
   * Zeitpunkt, und `playerCastStartedAtMsValue` sagt, wie weit er gekommen ist.
   * Ein Kanal löste hier anteilig auf, statt nur zu räumen — deshalb steht das
   * Räumen an EINER Stelle und nicht an den zwei Aufrufern.
   *
   * @returns true, wenn tatsächlich etwas abgebrochen wurde.
   */
  private cancelPlayerCast(): boolean {
    if (this.playerCastingValue < 0) return false;
    this.playerCastingValue = NO_SPELL;
    this.playerCastEndsAtMsValue = -1;
    this.playerCastStartedAtMsValue = -1;
    // **Nur räumen, nichts nachrechnen** (Issue #323). Der erreichte Anteil
    // einer Kanalisierung ist bereits ausgezahlt — Schritt für Schritt, in
    // `stepPassive`. Wer hier „wie weit bin ich gekommen" rechnete, hätte
    // dieselbe Zahl aus zwei Quellen, und die zweite wäre die falsche.
    //
    // **Diese Zeile hält den Zustand sauber, sie bewirkt den Abbruch nicht.**
    // Gemessen: Nimmt man sie heraus, bleiben alle zwölf Fälle grün — die
    // Schleife oben steigt schon aus, weil `playerCastingValue` geräumt ist.
    // Sie steht hier, damit das Feld nicht auf einen vergangenen Termin zeigt,
    // während nichts läuft; wer es je ohne den Cast-Zustand liest, bekäme
    // sonst eine Zahl, die nichts mehr bedeutet.
    this.playerChannelNextTickAtMsValue = -1;
    return true;
  }

  /**
   * Eine Teilwirkung einer Kanalisierung (Issue #323, GAME-DESIGN 7.2).
   *
   * # Der Anteil kommt aus der Dauer, nicht aus einer zweiten Zahl
   *
   * `baseDamage` ist die **Gesamt**-Wirkung; sie wird auf die Raster-Schritte
   * verteilt. Eine eigene Zahl „Schaden je Schritt" in den Daten wäre eine
   * zweite Quelle für dieselbe Aussage — und die beiden liefen bei der ersten
   * Änderung auseinander.
   *
   * **Ganzzahlig abgerundet, der Rest verfällt** (GAME-DESIGN 18.4). Bei 500
   * über 3 Schritte gibt jeder 166, nicht 166,67. Der Parser verlangt
   * `channelMs % channelTickMs === 0`, damit wenigstens die Schrittzahl
   * eindeutig ist; die Rundung des Betrags bleibt und ist auf beiden Seiten
   * dieselbe Formel.
   *
   * # Warum hier nicht `resolvePlayerCast` steht
   *
   * Der löst die **ganze** Wirkung auf: Synergie-Fenster, Koagulation,
   * Verhängnis, Unterbrechen. Das je Schritt zu tun hiesse, einen
   * fünfschrittigen Kanal fünf Fenster öffnen zu lassen. Eine Kanalisierung
   * teilt ihren SCHADEN, nicht ihre Ereignisse.
   */
  private resolveChannelTick(index: number, spell: SpellDef, timeMs: number): void {
    const schritte = Math.floor(spell.channelMs / spell.channelTickMs);
    if (schritte <= 0 || spell.baseDamage <= 0) return;
    // **Ein eigener Wurf je Schritt**, und aus `STREAM_DOT_TICK` — dieselbe
    // Regel und derselbe Strom wie beim Brand-Tick (GAME-DESIGN 6.5, Issue
    // #602). Ein Wurf für die ganze Kanalisierung kritete sie entweder ganz
    // oder gar nicht; ein Wurf aus `STREAM_CRIT` verschöbe die Würfe der
    // gewirkten Zauber und liesse jeden folgenden Kampf auseinanderlaufen.
    const kritet = this.rollCrit(STREAM_DOT_TICK);
    // Auch eine Teilwirkung gehört ihrem Zauber und damit dessen Schule
    // (Issue #328) — sonst trüge sie den Widerstand des zuletzt gewirkten.
    this.setDamageFactor('resistance', this.resistanceOfSpell[index] ?? 1);
    const anteil = Math.floor(spell.baseDamage / schritte);
    const damage = computeDamage(anteil, this.factorsFor(kritet));
    this.bookProgress(damage, alsSpellIndex(index), timeMs);
    this.convertDamageToShield(damage, timeMs);
    this.reportHit(damage, HIT_DOT, kritet, timeMs);
    // **Dieselbe Protokoll-Art wie ein Brand-Tick** und keine neue: Das
    // Protokoll hat einen gemeinsamen Nummernraum mit dem Simulator (#134),
    // und eine neue Art verschöbe jede eingecheckte Vergleichsdatei. Die
    // Aussage ist dieselbe — laufender Schaden, der nicht vom Drücken kommt.
    this.log?.push(this.fightElapsedMs(timeMs), EVENT_DOT_TICK, index, damage);
    if (kritet) this.events.emit(RULE_CRIT, index, damage, timeMs);
    this.events.emit(RULE_DOT_TICK, index, damage, timeMs);
  }

  /**
   * Einen Treffer melden.
   *
   * Ob er „groß" ist, entscheidet der Kampf und nicht die Anzeige: Es ist eine
   * Aussage über den Schaden, nicht über den Geschmack.
   */
  private reportHit(damage: number, kind: number, isCrit: boolean, timeMs: number): void {
    const sink = this.onHit;
    if (sink === null) return;
    sink(damage, kind, isCrit, damage >= this.boss.hp * BIG_HIT_HP_SHARE, timeMs);
  }

  /**
   * Stapel hinzufügen — der Zugriff, den die Schul-Mechanik benutzt.
   *
   * Deckelt selbst und zählt mit, was an der Obergrenze verpufft. Öffentlich
   * seit E3-2: Der Gewinn entsteht nicht mehr im Kampfcode, sondern in einem
   * Empfänger (`src/combat/schools/stacks.ts`).
   */
  gainResource(amount: number): void {
    this.addResource(amount);
  }

  /** Was ein Zauber bei einer Wirkung beisteuert — aus seinen Daten. */
  gainOfSpell(spellIndex: number, resourceId: string): number {
    const roh = this.spells[spellIndex]?.resourceGain?.[resourceId] ?? 0;
    // **Der Faktor aus `resourceBuffs`** (Issue #489, FLASHPOINT). Ganzzahlig,
    // wie der Vorrat selbst.
    if (roh === 0) return 0;
    // **Zwei Faktoren, und sie kommen aus verschiedenen Welten** (#419).
    // `resourceBuffs` ist ein zeitlich begrenztes Fenster aus einem Zauber
    // (FLASHPOINT); `talentResourceFactor` ist die dauerhafte Verteilung des
    // Spielers. Sie multiplizieren sich, wie alle Faktoren dieses Spiels.
    //
    // **Einmal gerundet, nicht zweimal.** Zwei Rundungen nacheinander ergeben
    // bei 3 × 1,5 × 1,5 je nach Reihenfolge 6 oder 7 — und der Simulator
    // müsste dieselbe Reihenfolge treffen. Ein Produkt, eine Rundung.
    const faktor =
      this.resourceBuffs.factor(this.nowMsForBuffs) * talentResourceFactor(this.talents);
    return Math.round(roh * faktor);
  }

  /** Was ein Zauber je Tick seines Effekts über Zeit beisteuert. */
  gainOfTick(spellIndex: number, resourceId: string): number {
    return this.spells[spellIndex]?.dot?.resourceGainPerTick[resourceId] ?? 0;
  }

  /** Kennung des Vorrats dieser Schule. */
  get resourceId(): string {
    return this.school.resource.id;
  }

  /** Was ein Zauber laut seinen Daten an Abklingzeit verkürzt (E3-3). */
  reductionOfSpell(spellIndex: number): number {
    const roh = this.spells[spellIndex]?.cooldownReductionMs ?? 0;
    // **Der Faktor aus `cdReduceBuffs`** (DISTIL, E10). Hier wird bei JEDEM
    // Wirken neu geholt — deshalb trägt ein zeitlich begrenzter Faktor an
    // dieser Stelle. Gemessen vor dem Bau: `shield.durationMs` tut das
    // NICHT (einmal beim Anlegen), und HARDEN gehört deshalb nicht hierher.
    return roh === 0
      ? 0
      : Math.round(
          roh *
            this.cdReduceBuffs.factor(this.nowMsForBuffs) *
            talentCooldownReductionFactor(this.talents),
        );
  }

  /**
   * Die **Auswahl**, die dieser Zauber beim Verkürzen trifft (Issue #518).
   *
   * `shortenCooldowns` kennt den Auslöser nicht — sie bekommt einen Index
   * zum Überspringen und sonst nichts. Ohne diesen Getter wären die zwei
   * neuen Parameter vorhanden und unerreichbar: die Klasse aus #420/#421.
   */
  reductionScopeOfSpell(spellIndex: number): { belowMs: number; longestOnly: boolean } {
    const spell = this.spells[spellIndex];
    return {
      // **Die Schwelle wächst mit, der Abzug bleibt.** Ein höherer Wert lässt
      // den Abzug auf mehr Zauber greifen (`ascend` trifft alles unter 10 s),
      // ohne den Abzug selbst zu ändern — das ist der Unterschied zum Weg
      // `reduction`, und die Kreuzprobe misst genau ihn.
      belowMs: Math.round(
        (spell?.cooldownReductionBelowMs ?? 0) * talentCooldownThresholdFactor(this.talents),
      ),
      longestOnly: spell?.cooldownReductionLongestOnly === true,
    };
  }

  /**
   * Die Abklingzeit **eines** Zaubers auf null setzen.
   *
   * Für den Cooldown-Rücksetzer (SUBLIMATION, Issue E3-3c). Er ist ein
   * Auswahlmodus: Der Spieler sagt ausdrücklich, welchen Zauber er meint —
   * deshalb genau einer und nicht alle, und deshalb hier statt in
   * `setCooldownsReady`, das ein Entwicklungs-Werkzeug ist.
   *
   * @returns false, wenn es den Zauber nicht gibt.
   */
  clearCooldown(index: number): boolean {
    if (index < 0 || index >= this.cooldownReadyAtMs.length) return false;
    this.cooldownReadyAtMs[index] = 0;
    return true;
  }

  /**
   * Alle laufenden Abklingzeiten verkürzen (SUBLIMATION, GAME-DESIGN 4.1).
   *
   * **Nur laufende.** Ein Zauber, der bereit ist, bleibt unberührt — sonst
   * entstünde eine „negative Restzeit", die den nächsten Wirk-Versuch um genau
   * diesen Betrag früher zuließe, ohne dass es je eine Abklingzeit gegeben
   * hätte. Die Zeit wird deshalb auf den Jetzt-Zeitpunkt begrenzt, nicht
   * einfach abgezogen.
   *
   * @returns wie viele Abklingzeiten tatsächlich verkürzt wurden
   */
  /**
   * Die Summe aller verbleibenden Abklingzeiten in Millisekunden (Issue #520).
   *
   * **Was NICHT mitzählt:** Zauber, die bereit sind. Ihre Restzeit ist null,
   * nicht negativ — sonst zöge ein lange bereiter Zauber die Summe herunter,
   * und EFFLUX brächte in einer ruhigen Phase weniger als mitten im Kampf.
   */
  remainingCooldownSumMs(timeMs: number): number {
    let summe = 0;
    for (let i = 0; i < this.cooldownReadyAtMs.length; i++) {
      const bereitAb = this.cooldownReadyAtMs[i] ?? 0;
      if (bereitAb > timeMs) summe += bereitAb - timeMs;
    }
    return summe;
  }

  shortenCooldowns(
    amount: number,
    timeMs: number,
    exceptIndex: number,
    grenze = 0,
    nurLaengste = false,
  ): number {
    if (amount <= 0) return 0;

    // **Die Auswahl VOR der Schleife** (Issue #518, DECANT). Wer den
    // längsten treffen will, muss ihn erst finden — die Schleife selbst
    // kann es nicht, sie sieht immer nur einen Eintrag.
    //
    // Bei Gleichstand gewinnt der NIEDRIGERE Index. Das ist eine
    // Entscheidung, keine Selbstverständlichkeit: Ohne sie hinge das
    // Ergebnis davon ab, ob die Schleife vorwärts oder rückwärts läuft, und
    // das wäre ein stiller Paritäts-Bruch, sobald eine Seite umbaut.
    let ziel = -1;
    if (nurLaengste) {
      let laengste = timeMs;
      for (let i = 0; i < this.cooldownReadyAtMs.length; i++) {
        if (i === exceptIndex) continue;
        const readyAt = this.cooldownReadyAtMs[i] ?? 0;
        if (readyAt > laengste) {
          laengste = readyAt;
          ziel = i;
        }
      }
      if (ziel < 0) return 0;
    }

    let touched = 0;
    for (let i = 0; i < this.cooldownReadyAtMs.length; i++) {
      if (i === exceptIndex) continue;
      if (nurLaengste && i !== ziel) continue;
      const readyAt = this.cooldownReadyAtMs[i] ?? 0;
      if (readyAt <= timeMs) continue;

      // **Die Schwelle ist ein STRENGES Kleiner** (Issue #518, ASCEND).
      // Eine Abklingzeit GENAU auf der Schwelle bleibt unberührt. Die Regel
      // musste in eine Richtung fallen; so gelesen heißt „unter 10 s"
      // wörtlich unter, nicht „bis einschließlich".
      if (grenze > 0 && readyAt - timeMs >= grenze) continue;

      const shortened = readyAt - amount;
      this.cooldownReadyAtMs[i] = shortened > timeMs ? shortened : timeMs;
      touched++;
    }
    return touched;
  }

  /**
   * Die Obergrenze des Vorrats, wie sie JETZT gilt (SLAG, Issue #490).
   *
   * **Eine Stelle, nicht zwei.** `resource.cap` wurde an zwei Orten gelesen
   * (Auffüllen und Setzen); ein Faktor an nur einem hätte eine Grenze
   * ergeben, die beim Auffüllen anders ist als beim Setzen — und der
   * Unterschied fiele erst auf, wenn jemand das Werkzeug benutzt.
   */
  private get resourceCap(): number {
    return Math.round(this.school.resource.cap * this.capBuffs.factor(this.nowMsForBuffs));
  }

  /**
   * Vorrat erzeugen (Issue #549).
   *
   * # Ein Erzeuger nimmt NIE weg
   *
   * Das ist die Regel, und sie steht hier, weil sie nirgends geschrieben
   * stand — sie folgte nur aus `min(before + amount, cap)` und war dort
   * falsch, sobald der Deckel unter den stehenden Vorrat fällt.
   *
   * Dieser Zustand ist erreichbar: SLAG hebt die Grenze von 20 auf 30, wer im
   * Fenster über 20 sammelt und es ablaufen lässt, steht darüber. Ein Wurf
   * hätte dann **gesenkt**:
   *
   * ```text
   * vorrat 24, Deckel wieder 20, BELLOWS gibt 8
   *   min(24 + 8, 20) = 20      acht erzeugt, vier verloren, netto MINUS vier
   *   max(24, min(...)) = 24    er bleibt, wo er ist
   * ```
   *
   * Ein Aufbau-Zauber, nach dem der Spieler weniger hat als vorher, ist in
   * keinem Regelwerk richtig. Der Verfall baut den Überstand ohnehin ab —
   * gemessen: 24 auf 20 in vier Sekunden —, es braucht dafür keine zweite
   * Mechanik.
   *
   * **Unter dem Deckel ändert sich nichts.** Dort ist `before` immer kleiner
   * als das Ergebnis, das `max` also wirkungslos.
   *
   * # Was der Überlauf-Zähler zählt
   *
   * Nur, was von DIESER Erzeugung nicht ankam. Die vier Punkte, die der
   * fallende Deckel kostet, waren bereits angekommen — sie gehen nicht durch
   * die Erzeugung verloren, und „Überlauf" hat sie noch nie gemeint.
   *
   * Mit der Regel oben ist `this.resourceValue - before` nie negativ; vorher
   * sprang der Zähler um 12, wo 8 erzeugt wurden.
   */
  private addResource(amount: number): void {
    if (amount === 0) return;
    const cap = this.resourceCap;
    const before = this.resourceValue;
    const raised = before + amount;
    const gekappt = raised > cap ? cap : raised;
    this.resourceValue = gekappt > before ? gekappt : before;
    this.resourceOverflowValue += amount - (this.resourceValue - before);
  }

  // ------------------------------------------------------------ Zustand setzen

  /**
   * Verbleibende Lebenspunkte setzen — Entwicklungs-Werkzeug (GAME-DESIGN 19.1:
   * „Zustand setzen: Boss-Leben"). Erspart drei Minuten Spielzeit für eine
   * Prüfung kurz vor dem Ende.
   */
  setBossHpRemaining(hp: number): void {
    // Zustand setzen ist ein Werkzeug, mit dem sich das Spiel steuern lässt —
    // und damit genau das, was GAME-DESIGN 19.3 aus der Auslieferung heraushält.
    if (!__ROTA_DEVTOOLS__) return;
    const clamped = hp < 0 ? 0 : hp > this.boss.hp ? this.boss.hp : hp;
    this.damageDealtValue = this.boss.hp - clamped;
    // `playerDamageValue` bleibt, wo es ist: Wer den Ring per Werkzeug setzt,
    // hat ihn nicht heruntergespielt. Ihn mitzuziehen hieße, dem Spieler einen
    // Fehler zuzuschreiben, den ein Tastendruck in der Konsole gemacht hat.
  }

  /** Alle Abklingzeiten sofort bereit — Entwicklungs-Werkzeug (GAME-DESIGN 19.1). */
  setCooldownsReady(): void {
    if (!__ROTA_DEVTOOLS__) return;
    this.cooldownReadyAtMs.fill(0);
    this.gcd.reset();
  }

  /** Vorrat setzen — Entwicklungs-Werkzeug (GAME-DESIGN 19.1). */
  setResource(value: number): void {
    if (!__ROTA_DEVTOOLS__) return;
    const cap = this.resourceCap;
    this.resourceValue = value < 0 ? 0 : value > cap ? cap : value;
  }

  /**
   * Einen anderen Boss übernehmen und den Kampf frisch aufsetzen (Issue #613).
   *
   * # Warum nicht einfach ein neuer `Combat`
   *
   * `Session` bekommt den Kampf im Konstruktor und hält ihn `readonly`; in
   * `app.ts` steht `this.combat` an 29 Stellen. Ein Neuaufbau zöge Sitzung,
   * Anzeige und Trefferanzeige mit — deutlich mehr, als der Wechsel wert ist.
   *
   * # Warum es trotzdem nicht bei `this.boss = boss` bleibt
   *
   * **`reset()` allein genügt nicht.** Es leitet die fünf Zeitgeber aus
   * `this.boss` neu ab — also nach dem Umsetzen aus dem richtigen Boss. Die
   * **Widerstände** stehen aber in `resistanceOfSpell`, und das wird nur im
   * Konstruktor gefüllt. Ohne die Schleife hier trüge der neue Boss die
   * Widerstände des alten: Jeder Zauber rechnete mit einem Faktor, der zu
   * einem Kampf gehört, den der Spieler verlassen hat.
   *
   * Das ist die „zweite Wahrheit", die gegen einen Setter spricht — sie ist
   * real, deshalb steht sie hier an einer Stelle statt beim Aufrufer.
   * `tests/unit/boss-switch.test.ts` misst genau das.
   */
  switchBoss(boss: BossDef): void {
    this.boss = boss;
    for (let i = 0; i < this.spells.length; i++) {
      const schule = this.spells[i]?.school;
      this.resistanceOfSpell[i] =
        schule === null || schule === undefined ? 1 : (boss.resistances[schule] ?? 1);
    }
    this.reset();
  }

  reset(): void {
    this.cooldownReadyAtMs.fill(0);
    this.castCounts.fill(0);
    // **Die Synergie-Fenster und ihre Zähler** (Issue #318). Ohne diese Zeilen
    // trägt der nächste Lauf die Ausbeute des vorigen — im Browser gemessen:
    // Eine Leiste OHNE das Paar zeigte weiter „1 of 1", weil `used` und
    // `available` den Reset überlebten.
    //
    // Ein Prüffall findet das nicht: Er baut für jeden Fall eine frische
    // Sitzung, und dort ist der Zustand ohnehin leer. Sichtbar wird es erst
    // beim zweiten Lauf im selben Spiel.
    this.synergyWindowEndsMs.fill(NOT_ACTIVE);
    this.synergyUsedCount = 0;
    this.synergyAvailableCount = 0;
    this.damageBySpell.fill(0);
    this.dotExpiresAtMs.fill(NOT_ACTIVE);
    this.dotNextTickAtMs.fill(NOT_ACTIVE);
    this.dotTicksLost.fill(0);
    this.dotTickCounts.fill(0);
    this.gcd.reset();
    // **Ein neuer Versuch fängt ohne laufenden BOSS-Zauber an** (Issue #327).
    //
    // Gemessen am 2026-08-05, vor dieser Zeile: Nach `reset()` stand `casting`
    // weiterhin auf 2 und `castLandsAtMs` auf 7000 — der Zustand überlebte den
    // Versuch. Der Simulator räumt ihn seit jeher (`Fight::reset`,
    // `self.casting = None`), das Spiel nicht. Zwei Seiten, zwei Antworten,
    // und keine Prüfung dazwischen: Keine Aufzeichnung setzt mitten in einem
    // Boss-Zauber zurück, also hat es nie jemand gemerkt.
    //
    // Aufgefallen ist es erst, als der Balken aus diesem Issue den Zustand
    // ZEIGT: Nach einem Neustart stünde dort ein Zauber, den es nicht mehr
    // gibt. Eine Anzeige findet, was keine Rechnung verrät.
    this.castingValue = -1;
    this.castLandsAtMsValue = -1;
    this.castingStartedAtMsValue = -1;
    // Ein neuer Versuch fängt ohne laufende Wirkzeit an (Issue #304). Ohne
    // diese Zeilen bliebe der Zustand über den Versuch hinaus stehen: Der
    // erste Druck des nächsten Kampfes bräche einen Zauber ab, den es nicht
    // mehr gibt, und die Zahl der Abbrüche stimmte von Anfang an nicht.
    this.playerCastingValue = NO_SPELL;
    this.playerCastEndsAtMsValue = -1;
    this.playerCastStartedAtMsValue = -1;
    this.castsInterruptedValue = 0;
    this.castsCancelledValue = 0;
    this.random.reset();
    this.resourceValue = 0;
    this.resourceOverflowValue = 0;
    // **Und der Faktor dazu**, nicht nur der Zustand: `buffs.reset()` leert die
    // Liste, die Rechnung stünde ohne die zweite Zeile weiter auf dem letzten
    // Wert. Genau dieser Fehler ist beim Synergie-Fenster passiert (#318) —
    // dort blieb nach einem Leistenwechsel „1 of 1" stehen, und kein Prüffall
    // fand es, weil jeder frisch baut statt zurückzusetzen.
    this.nowMsForBuffs = 0;
    this.buffs.reset();
    this.controlBuffs.reset();
    this.resourceBuffs.reset();
    this.tickBuffs.reset();
    this.stackBuffs.reset();
    this.cdReduceBuffs.reset();
    this.convertBuffs.reset();
    this.capBuffs.reset();
    this.setDamageFactor('buffs', NEUTRAL_FACTORS.buffs);
    this.nextDecayAtMs = this.school.resource.decayPerStepMs;
    this.damageDealtValue = 0;
    this.playerDamageValue = 0;
    this.stateValue = FIGHT_RUNNING;
    this.endedAtMs = NOT_ACTIVE;
    this.fightStartedAtMs = 0;
    this.fightHasBegun = false;
    // Nur die Folge, nicht die Anmeldungen: Ein neuer Versuch behält seine
    // Zauber und Talente — er beginnt lediglich wieder bei null Ereignissen.
    this.events.clearSequence();

    // ------------------------------------------------------------------
    // Der zweite Lauf (Issue #349)
    // ------------------------------------------------------------------
    //
    // **Hier fehlten 26 Felder**, gefunden vom Wächter in
    // `tests/unit/reset-completeness.test.ts` — nicht von Hand aufgezählt,
    // sondern gemessen: Er vergleicht den zurückgesetzten Kampf gegen einen
    // frisch gebauten und meldet jedes Feld, das abweicht.
    //
    // Und es war nicht latent. Das Spiel baut den Kampf genau einmal
    // (`app.ts`), „Retry" führt hierher. Vor dieser Ergänzung begann der
    // zweite Versuch mit angeschlagenem Leben, einer Zeitleiste bei Eintrag 2
    // und liegenden Zersetzungs-Stapeln:
    //
    //     LAUF 1 Ende:  HP 4552  Zeitleiste 2  Zersetzung 5  erlitten 448
    //     nach reset(): HP 4552  Zeitleiste 2  Zersetzung 5  erlitten 448
    //     frisch:       HP 5000  Zeitleiste 0  Zersetzung 0  erlitten 0
    //
    // **Die Werte spiegeln den Konstruktor**, nicht die Null.** Mehrere Uhren
    // beginnen bei ihrem Takt und nicht bei 0 — der Replay ruft `beginFight`
    // nicht, und ein Startwert von 0 ließe sie schon im Takt 0 auslösen. Wer
    // hier stumpf nullt, baut genau den Unterschied ein, den die Kommentare
    // an den Feldern seit #96 verhindern.
    this.bossPhaseValue = 0;
    this.freeControlSpendArmed = false;
    this.freeControlSpends = 0;
    this.bossEnergyValue = 0;
    this.bossEnergyTriggersValue = 0;
    this.timelineIndexValue = 0;
    this.nextEnergyAtMs = this.boss.energy?.perStepMs ?? 0;
    this.nextDoomAtMs = this.boss.doom?.perStepMs ?? 0;
    this.nextStackAtMs = this.boss.stacks?.perStepMs ?? 0;
    this.nextRunawayAtMs = this.boss.runaway?.perStepMs ?? 0;
    this.nextAfflictionAtMs = 0;
    this.nextStrikeDelayMs = 0;
    // PIN (#478) — ein Merker, der einen Kampf überlebte, machte im nächsten
    // einen Zauber unterbrechbar, den niemand dafür bezahlt hat.
    this.pinWindowMsValue = 0;

    this.playerHpValue = this.vesselMax;
    this.diedAtMsValue = -1;
    this.damageTakenValue = 0;
    this.interruptsLandedValue = 0;
    this.interruptsMissedValue = 0;

    this.doomValue = 0;
    this.doomChargesValue = this.boss.doom?.clearCharges ?? 0;
    this.nextDoomChargeAtMs = 0;
    this.stacksValue = 0;

    this.raceDone = false;
    this.raceWonValue = false;
    this.raceEndsAtMs = -1;
    this.raceProgressValue = 0;
    this.damageLostToRaceValue = 0;
    this.bossHealedValue = 0;
    this.stabilisedValue = 0;

    this.afflictedValue = false;
    this.afflictionsClearedValue = 0;

    this.decayCountValue = 0;
    this.decayExpiryMs.fill(0);
    // **Der gesammelte Rest endet mit dem Kampf** (#419). Ein Rest, der ihn
    // überlebte, machte den ersten Wurf des nächsten Kampfes von der
    // Vorgeschichte abhängig — zwei gleiche Eröffnungen legten verschieden
    // viele Stapel, je nachdem was vorher lief. Das wäre gegen die
    // Planbarkeit aus Gesetz 6.
    this.decayRest.fill(0);
    // Der Auswahlmodus gehoert zum Kampf und endet mit ihm (#528).
    //
    // **Und offene Ladungen enden hier mit** (Issue #521, AC3). Es gibt
    // KEINEN Zeitverfall — das ist die bestehende Regel, nicht eine neue für
    // ALEMBIC: Schon vorher wurde der Auswahlmodus ausschliesslich hier
    // gelöscht, nie durch Ablauf einer Frist. Wer eine zweite Ladung nicht
    // nutzt, verliert sie also mit dem Kampf und nicht vorher.
    //
    // Das steht hier ausdrücklich, weil der nächste Leser sonst einen Timer
    // sucht, den es nie gab.
    this.resetArmedForValue = -1;
    this.resetChargesLeftValue = 0;

    this.shieldValue = 0;
    this.shieldExpiresAtMs = -1;
    this.shieldStacksValue = 0;
    this.shieldAbsorbedValue = 0;
    this.healedValue = 0;
    // **Auch die laufende Heilung endet mit der Runde** (POULTICE, Issue #481).
    // Ohne diese vier Zeilen liefe ein POULTICE aus der vorigen Runde in die
    // nächste hinein und heilte dort weiter — sichtbar erst an einer Anzeige,
    // die ohne Zauber steigt.
    this.healOverTimeAmount = 0;
    this.healOverTimeSteps = 0;
    this.healOverTimeIntervalMs = 0;
    this.healOverTimeDone = 0;
    this.healOverTimeCredited = 0;
    this.healOverTimeNextAtMs = 0;
    this.conversionUntilMs = -1;
    this.conversionPercentValue = 0;
    // PUTRID BLOOM (#479) — eine Anhebung, die einen Kampf überlebte, gäbe im
    // nächsten Plätze, die niemand bezahlt hat.
    this.decayCapBonusValue = 0;
    this.decayCapUntilMs = -1;
    // DECAY (#479) — eine Schwächung, die einen Kampf überlebte, schützte im
    // nächsten gratis.
    this.decayWardUntilMs = -1;
    this.decayWardPermilleValue = 0;
    this.convertedValue = 0;
    this.reflectUntilMs = -1;
    this.reflectPercentValue = 0;
    this.reflectedValue = 0;
    this.reflectSpellIndex = alsSpellIndex(0);

    this.controlValue = 0;
    this.pendingConvertedDamage = 0;
    this.controlSpentValue = 0;
    this.controlFailedValue = 0;
    this.phaseHoldPercentValue = 0;
    this.phaseHoldUntilMs = -1;

    // **Über `setDamageFactor`, nicht über eine Zuweisung** (Issue #196): Die
    // Krit-Fassung zieht mit. `resistance` steht nicht dabei — der Widerstand
    // hängt seit #328 am Zauber und wird vor jeder Wirkung neu gesetzt.
    this.setDamageFactor('vulnerability', 1);

    // Die Schul-Mechaniken führen eigene Zähler. Sie hängen am Kampf und
    // müssen mit ihm zurück — sonst zählt der zweite Lauf weiter, wo der erste
    // aufgehört hat.
    this.stacks.reset();
    this.sublimation.reset();
  }
}
