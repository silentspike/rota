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
  EVENT_BOSS_TIMELINE_FIRED,
  EVENT_CAST,
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
  type BossDef,
  type SchoolDef,
  type SpellDef,
  type TimelineEntryDef,
} from './data.js';
import { GlobalCooldown } from './gcd.js';
import { RandomStreams, STREAM_CRIT } from './rng.js';
import { hastedMs } from './haste.js';
import {
  NOTHING_WORN,
  critChance,
  critFactor,
  gearEffectFactor,
  type WornValues,
} from '../gear/worn.js';

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
import { registerStackMechanic, type StackMechanic } from '../combat/schools/stacks.js';
import { registerSublimation, type SublimationMechanic } from '../combat/schools/sublimation.js';

/** Ausgang eines Wirk-Versuchs. */
export const CAST_OK = 0;
export const CAST_UNKNOWN = 1;
export const CAST_ON_COOLDOWN = 2;
export const CAST_ON_GCD = 3;
export const CAST_FIGHT_OVER = 4;

/** Stand des Kampfes. */
export const FIGHT_RUNNING = 0;
export const FIGHT_WON = 1;
export const FIGHT_OVERPRESSURE = 2;

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

/** Wie viele Stapel höchstens gleichzeitig liegen. Wird gemessen. */
export const MAX_DECAY_STACKS = 5;

/** Um wie viel Prozent ein Stapel den Schaden aller Quellen erhöht. Wird gemessen. */
export const DECAY_STACK_PERCENT = 3;

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
}

/**
 * Was ein Wirken bewirkt hat. **Ein einziges, wiederverwendetes Objekt** — der
 * Kampf darf im laufenden Bild nichts erzeugen (GAME-DESIGN 18.2).
 */
export interface CastOutcome {
  result: number;
  spellIndex: number;
  /** Zugefügter Schaden, Direkt- und Entladeschaden zusammen. */
  damage: number;
  /** Vorrat vor dem Wirken — für die Schadens-Herleitung (GAME-DESIGN 19.1). */
  resourceBefore: number;
  /** Hat der Entlade-Zauber seinen Bonus mitgenommen? */
  bonusApplied: boolean;
}

export class Combat {
  readonly school: SchoolDef;
  readonly boss: BossDef;
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
    spellIndex: -1,
    damage: 0,
    resourceBefore: 0,
    bonusApplied: false,
  };

  private readonly indexById = new Map<string, number>();
  private readonly cooldownReadyAtMs: Float64Array;
  private readonly castCounts: Int32Array;
  private readonly damageBySpell: Float64Array;
  private readonly dotExpiresAtMs: Float64Array;
  private readonly dotNextTickAtMs: Float64Array;
  private readonly dotTicksLost: Int32Array;
  private readonly dotTickCounts: Int32Array;

  private readonly resistanceFactor: number;
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
  private readonly decayExpiryMs = new Float64Array(MAX_DECAY_STACKS);

  /** Wie viele Stapel gerade liegen. */
  private decayCountValue = 0;

  /** Die getragenen Sonderwerte (GAME-DESIGN 6.7). */
  private readonly worn: WornValues;
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
  private readonly factorsOnCrit: MutableFactors;

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
  private castLandsAtMs = -1;

  /** Verbleibende Lebenspunkte des Spielers (Issue #110). */
  private playerHpValue: number;

  /** Stand des Zählers, der bei seiner Schwelle tötet (Issue #216). */
  private doomValue = 0;

  /** Kampfzeit des nächsten Zähler-Schritts. */
  private nextDoomAtMs = 0;

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

  /** Stand der kippenden Reaktion (Issue #240, Baustein 6). */
  private reactionValue = 0;

  /** Kampfzeit des naechsten Absinkens. */
  private nextReactionDropAtMs = 0;

  /** Ob die Reaktion gekippt ist — der Lauf endet dann. */
  private reactionFailedValue = false;

  /** Wie oft der Spieler sie gehoben hat. */
  private stabilisedCountValue = 0;

  /** Ob die Schwaechung liegt (Baustein 8). */
  private afflictedValue = false;

  /** Kampfzeit des naechsten Schwaechungs-Ticks. */
  private nextAfflictionAtMs = 0;

  /** Wie oft sie entfernt wurde. */
  private afflictionsClearedValue = 0;

  /** Kampfzeit des nächsten Stapel-Schritts. */
  private nextStackAtMs = 0;

  /** Kampfzeit des Spielertods, oder -1. */
  private diedAtMsValue = -1;

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

  /** Wie viel Schutz die Umwandlung insgesamt erzeugt hat. */
  private convertedValue = 0;

  // -- FIXATION (Issue #100) --

  /**
   * Vorrat an Kontroll-Punkten.
   *
   * **Zum Ausgeben, nicht zum Sammeln** (GAME-DESIGN 4.1): Sie verfallen nicht
   * wie die Hitze, sondern warten darauf, dass der Spieler sich entscheidet.
   */
  private controlValue = 0;

  /** Wie viele Punkte insgesamt ausgegeben wurden. */
  private controlSpentValue = 0;

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
  private damageDealtValue = 0;
  private stateValue = FIGHT_RUNNING;
  private endedAtMs = NOT_ACTIVE;
  private fightStartedAtMs = 0;

  constructor(options: CombatOptions) {
    this.school = options.school;
    this.boss = options.boss;
    this.spells = options.school.spells;
    this.random = new RandomStreams(options.seed ?? 0);

    const count = this.spells.length;
    this.cooldownReadyAtMs = new Float64Array(count);
    this.castCounts = new Int32Array(count);
    this.damageBySpell = new Float64Array(count);
    this.dotExpiresAtMs = new Float64Array(count).fill(NOT_ACTIVE);
    this.dotNextTickAtMs = new Float64Array(count).fill(NOT_ACTIVE);
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
    this.doomChargesValue = options.boss.doom?.clearCharges ?? 0;
    this.playerHpValue = PLAYER.vessel;
    this.resistanceFactor = options.boss.resistances[options.school.school] ?? 1;
    this.worn = options.worn ?? NOTHING_WORN;
    // **Topf 4 kommt beim Aufbau in die Faktoren**, nicht im Takt: Ausrüstung
    // wird zwischen den Kämpfen gewechselt (13.1). Ohne getragene Ausrüstung
    // ist der Faktor exakt 1,0 — die Multiplikation bewegt dann keine Zahl.
    this.factors = {
      ...NEUTRAL_FACTORS,
      resistance: this.resistanceFactor,
      gearEffects: gearEffectFactor(this.worn),
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

  indexOf(spellId: string): number {
    return this.indexById.get(spellId) ?? -1;
  }

  spellAt(index: number): SpellDef | undefined {
    return this.spells[index];
  }

  get damageDealt(): number {
    return this.damageDealtValue;
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

  /** Wie oft die Energieleiste ausgelöst hat. */
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
  get playerHp(): number {
    return this.playerHpValue;
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

  /** Stand der kippenden Reaktion (Issue #240). */
  get reaction(): number {
    return this.reactionValue;
  }

  /** Ob die Reaktion gekippt ist. */
  get reactionFailed(): boolean {
    return this.reactionFailedValue;
  }

  /** Wie oft der Spieler sie gehoben hat. */
  get stabilisedCount(): number {
    return this.stabilisedCountValue;
  }

  /**
   * Wie viele Zyklen die Zeitleiste gelaufen ist (Baustein 7; Issue #240).
   *
   * **Abgeleitet, nicht mitgezählt.** Ein eigenes Feld wäre eine zweite
   * Wahrheit über dieselbe Größe — und die zweite läuft beim Nachziehen weg.
   * Der Zeiger weiß es bereits.
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
    const left = this.doomValue - doom.clearAmount;
    this.doomValue = left > 0 ? left : 0;
    return true;
  }

  /** Welcher Boss-Zauber gerade läuft, oder -1. */
  get casting(): number {
    return this.castingValue;
  }

  /** Ist der Spieler tot? Der Versuch endet dann (GAME-DESIGN A.3). */
  get playerDead(): boolean {
    return this.diedAtMsValue >= 0;
  }

  /** Kampfzeit des Spielertods, oder -1. */
  get diedAtMs(): number {
    return this.diedAtMsValue;
  }

  /** Gesamter erlittener Schaden, **nach** WARD. */
  get damageTaken(): number {
    return this.damageTakenValue;
  }

  /** Wie oft der Spieler einen Boss-Zauber unterbrochen hat. */
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
  get converted(): number {
    return this.convertedValue;
  }

  /** Vorrat an Kontroll-Punkten (FIXATION, Issue #100). */
  get control(): number {
    return this.controlValue;
  }

  /** Wie viele Kontroll-Punkte insgesamt ausgegeben wurden. */
  get controlSpent(): number {
    return this.controlSpentValue;
  }

  /** Wie oft eine Kontroll-Wirkung mangels Punkten ins Leere lief. */
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
  private applyFixation(spell: SpellDef, timeMs: number): void {
    if (spell.controlGain !== undefined && spell.controlGain > 0) {
      this.controlValue += spell.controlGain;
    }

    const control = spell.control;
    if (control === undefined) return;
    if (this.controlValue < control.cost) {
      this.controlFailedValue++;
      return;
    }
    this.controlValue -= control.cost;
    this.controlSpentValue += control.cost;

    // Die drei Wirkungen aus 4.1. Hier greift FIXATION wirklich in den
    // Kampfverlauf ein — das ist, was sie von jeder anderen Schule
    // unterscheidet.
    if (control.effect === 'slowCast') {
      // Der laufende Boss-Zauber trifft später; das Unterbrechen-Fenster wird
      // größer.
      if (this.castingValue >= 0) this.castLandsAtMs += control.amount;
    } else if (control.effect === 'delayStrike') {
      // **Nur der nächste Eintrag.** Die folgenden stehen, wo sie stehen —
      // der Abstand dahinter schrumpft, und genau das ist der Preis.
      this.nextStrikeDelayMs += control.amount;
    } else if (control.effect === 'holdPhase') {
      // Die Schwelle sinkt vorübergehend. **Sie nimmt einen Wechsel nicht
      // zurück** — die Phase kann nur steigen.
      this.phaseHoldPercentValue = control.amount;
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
  private applyCoagulation(spell: SpellDef, timeMs: number): void {
    const seitBeginn = timeMs - this.fightStartedAtMs;

    // Weg 1 — Schutzschicht aufbauen.
    const shield = spell.shield;
    if (shield !== undefined) {
      // **Stapeln bis zur Grenze, dann nicht weiter.** Ein weiterer Druck auf
      // eine volle Schicht erneuert die Laufzeit, erhöht aber den Betrag
      // nicht — sonst wäre die Obergrenze wirkungslos.
      if (this.shieldStacksValue < shield.maxStacks) {
        this.shieldStacksValue++;
        this.shieldValue += shield.amount;
        // **Die Reaktion stabilisieren** (11.2, Baustein 6; Issue #240). Der
        // Schutz hebt sie um seinen EIGENEN Betrag — eine zweite Zahl
        // "stabilisiert um X" waere dieselbe Groesse an zwei Orten.
        //
        // Nur beim Stapeln, nicht beim blossen Erneuern: Sonst waere die
        // Obergrenze wirkungslos, und das gilt hier wie fuer den Schutz selbst.
        this.stabilise(shield.amount);
      }
      this.shieldExpiresAtMs = seitBeginn + shield.durationMs;
    }

    // Weg 2 — Heilung.
    const heal = spell.heal;
    if (heal !== undefined) {
      // **Nur bis zur vollen Anzeige.** Was darüber hinausginge, verpufft — es
      // als Heilung zu zählen ließe einen Lauf mit voller Anzeige nach mehr
      // Wirkung aussehen, als er gebraucht hat.
      const fehlend = Math.max(0, PLAYER.vessel - this.playerHpValue);
      const wirklich = Math.min(heal.amount, fehlend);
      if (wirklich > 0) {
        this.playerHpValue += wirklich;
        this.healedValue += wirklich;
        // Auch die Heilung stabilisiert — mit dem Betrag, der WIRKLICH
        // angekommen ist. Was ueber die volle Anzeige hinausginge, verpufft
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
    const gewonnen = Math.floor((damage * this.conversionPercentValue) / 100);
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
  private applyPutrefaction(spell: SpellDef, timeMs: number): void {
    const decay = spell.decay;
    if (decay === undefined) return;
    const seitBeginn = this.fightElapsedMs(timeMs);
    for (let i = 0; i < decay.stacks; i++) {
      if (this.decayCountValue >= MAX_DECAY_STACKS) break;
      // Ans Ende, weil der neue am spätesten verfällt — die Liste bleibt damit
      // aufsteigend, ohne je zu sortieren.
      this.decayExpiryMs[this.decayCountValue] = seitBeginn + DECAY_STACK_DURATION_MS;
      this.decayCountValue++;
    }
    this.updateVulnerability();
  }

  /**
   * Lässt abgelaufene Stapel verfallen — **den ältesten zuerst**.
   *
   * Die Liste steht aufsteigend, also stehen die abgelaufenen vorn. Was bleibt,
   * rückt auf; die jüngeren behalten ihre eigene Restzeit.
   */
  private expirePutrefaction(timeMs: number): void {
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
  private updateVulnerability(): void {
    this.setDamageFactor('vulnerability', (100 + this.decayCountValue * DECAY_STACK_PERCENT) / 100);
  }

  /** Wie viele Zersetzungs-Stapel gerade auf dem Boss liegen. */
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
  }

  private hitPlayer(raw: number, timeMs: number): void {
    if (this.diedAtMsValue >= 0) return;
    // **Einmal gerundet, am Ende** — dieselbe Regel wie bei ausgehendem
    // Schaden (GAME-DESIGN 6.3) und dieselbe Zeile wie `PlayerBase::incoming`
    // im Simulator. Zweimal zu runden ergäbe je nach Reihenfolge andere
    // Zahlen, und genau daran laufen zwei Umsetzungen auseinander.
    const nachWard = Math.max(0, Math.round(raw * (1 - PLAYER.wardReduction)));

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
        // Aufgebraucht: Die Stapel sind weg, nicht nur der Betrag.
        this.shieldStacksValue = 0;
        this.shieldExpiresAtMs = -1;
      }
    }

    const angekommen = nachWard - abgefangen;
    this.damageTakenValue += angekommen;
    this.log?.push(this.fightElapsedMs(timeMs), EVENT_PLAYER_DAMAGE, NO_SPELL_REF, angekommen);
    this.playerHpValue = Math.max(0, this.playerHpValue - angekommen);
    if (this.playerHpValue === 0) {
      this.diedAtMsValue = timeMs - this.fightStartedAtMs;
    }
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
    this.castLandsAtMs = -1;
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
    this.nextDecayAtMs = timeMs + this.school.resource.decayPerStepMs;
    this.nextEnergyAtMs = timeMs + (this.boss.energy?.perStepMs ?? 0);
    this.nextDoomAtMs = timeMs + (this.boss.doom?.perStepMs ?? 0);
    this.nextStackAtMs = timeMs + (this.boss.stacks?.perStepMs ?? 0);
    // Das Fenster haengt an der KAMPFuhr, nicht an der durchlaufenden.
    this.raceDone = false;
    this.raceEndsAtMs = -1;
    // Die Reaktion beginnt mit ihrem Startwert und sinkt ab dem ersten Takt.
    this.reactionValue = this.boss.stabilise?.startValue ?? 0;
    this.nextReactionDropAtMs = timeMs + (this.boss.stabilise?.dropPerStepMs ?? 0);
    this.reactionFailedValue = false;
    this.afflictedValue = false;
    this.nextAfflictionAtMs = 0;
  }

  castCount(index: number): number {
    return this.castCounts[index] ?? 0;
  }

  damageOf(index: number): number {
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
  dotRemainingMs(index: number, timeMs: number): number {
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
    return this.hasted(dot.tickIntervalMs);
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
  private rollCrit(): boolean {
    return this.random.stream(STREAM_CRIT).chance(critChance(this.worn));
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
  isOffCooldown(index: number, timeMs: number): boolean {
    return (this.cooldownReadyAtMs[index] ?? 0) <= timeMs;
  }

  /** Kann der Zauber jetzt gewirkt werden? */
  isReady(index: number, timeMs: number): boolean {
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
    if (this.stateValue !== FIGHT_RUNNING) return;

    for (let i = 0; i < this.spells.length; i++) {
      const spell = this.spells[i] as SpellDef;
      const dot = spell.dot;
      if (dot === undefined) continue;
      if (!this.isDotActive(i, timeMs)) continue;
      const nextTick = this.dotNextTickAtMs[i] ?? NOT_ACTIVE;
      if (nextTick === NOT_ACTIVE || timeMs < nextTick) continue;

      // **Ein eigener Wurf je Tick** (GAME-DESIGN 6.5: „pro Zauber, pro
      // DoT-Tick und pro Heilung"). Nicht der Wurf der Wirkung, die ihn gesetzt
      // hat — sonst kritete ein DoT über seine ganze Laufzeit entweder immer
      // oder nie, und der Glücksanteil, den 6.5 ausdrücklich will, verschwände
      // in einer einzigen Entscheidung.
      const kritet = this.rollCrit();
      const damage = computeDamage(dot.tickDamage, this.factorsFor(kritet));
      this.bookProgress(damage, i, timeMs);
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
      while (timeMs >= this.nextDoomAtMs) {
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

    // Die kippende Reaktion (11.2, Baustein 6; #240).
    //
    // Sie sinkt von selbst — unabhaengig vom Schaden des Spielers, wie die
    // Energieleiste. Was sie hebt, sind Schutz und Heilung; wer nur Schaden
    // macht, verliert sie.
    const stab = this.boss.stabilise;
    if (stab !== null && !this.reactionFailedValue) {
      while (timeMs >= this.nextReactionDropAtMs) {
        this.reactionValue -= stab.dropAmount;
        this.nextReactionDropAtMs += stab.dropPerStepMs;
        if (this.reactionValue < stab.failBelow) {
          this.reactionValue = stab.failBelow;
          this.reactionFailedValue = true;
          // Der Lauf endet — wie beim toedlichen Zaehler an WARD und
          // Schutzschicht vorbei. Eine Reaktion, die eine Schutzschicht
          // ueberlebt, waere keine.
          this.playerHpValue = 0;
          this.diedAtMsValue = timeMs - this.fightStartedAtMs;
          break;
        }
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

    // `atMs` zählt **ab Kampfbeginn**, `timeMs` ist die Uhr des Spiels. Beginnt
    // ein Kampf nicht bei null, ist der Unterschied der ganze Versatz — und
    // zwar ein stiller: Die Einträge kämen zu früh, aber der Kampf liefe weiter.
    const seitBeginn = timeMs - this.fightStartedAtMs;

    // Zuerst der laufende Boss-Zauber: Ist seine Wirkzeit um, trifft er.
    // **Vor** dem Vorrücken der Zeitleiste, damit ein Treffer und ein neuer
    // Eintrag im selben Takt in derselben Reihenfolge geschehen wie im
    // Simulator — dort steht `BossCastLands` vor `TimelineStep` in der
    // Rangfolge der Ereignisse.
    if (this.castingValue >= 0 && seitBeginn >= this.castLandsAtMs) {
      // Über `timelineAt`, nicht über den Platz in der Liste: Bei wiederholter
      // Zeitleiste liegt der Zähler darüber (#174).
      const laufend = this.timelineEntryAt(this.castingValue);
      this.castingValue = -1;
      this.castLandsAtMs = -1;
      this.interruptsMissedValue++;
      if (laufend !== undefined) this.hitPlayer(laufend.damage, timeMs);
      // **Ein Fehler macht Fortschritt rueckgaengig** (11.2, Baustein 5;
      // Issue #218). Der Auslöser ist der durchgekommene Boss-Zauber — kein
      // neuer Fehler-Begriff, und `interruptsMissed` zaehlt ihn ohnehin.
      //
      // Ob und wie viel er heilt, sagt der BOSS-Datensatz. Eine feste
      // Kopplung gaelte fuer alle Bosse; 11.2 sagt "nur fuer einzelne, sonst
      // frustrierend".
      this.healBoss();
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
      this.log?.push(this.fightElapsedMs(timeMs), EVENT_BOSS_TIMELINE_FIRED, NO_SPELL_REF, index);
      // Verbraucht: Der nächste Eintrag fängt wieder bei seinem eigenen
      // Zeitpunkt an (STAY, Issue #100).
      this.nextStrikeDelayMs = 0;
      this.timelineIndexValue++;
      if (eintrag.mechanic === 'interruptibleCast') {
        // Ob er trifft, entscheidet sich erst, wenn die Wirkzeit um ist — bis
        // dahin kann der Spieler ihn abbrechen.
        this.castingValue = index;
        // **Die eskalierende Wiederholung** (11.2, Baustein 7; Issue #240):
        // Mit jedem Zyklus schrumpft das Fenster zum Unterbrechen. Die
        // Verkuerzung steht in den DATEN — sonst waere sie eine Regel des
        // Spiels statt eine Eigenschaft dieses Bosses.
        this.castLandsAtMs = faelligAtMs + this.escalatedCastTime(eintrag.castTimeMs, index);
      } else {
        // Trifft sofort. Nicht zu verhindern, nur auszuhalten.
        this.hitPlayer(eintrag.damage, timeMs);
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
   * @returns ob der Schaden am Boss angekommen ist
   */
  private bookProgress(damage: number, spellIndex: number, timeMs: number): boolean {
    if (this.raceEndsAtMs >= 0) {
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
    this.damageBySpell[spellIndex] = (this.damageBySpell[spellIndex] ?? 0) + damage;
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
    if (heal === null || this.damageDealtValue <= 0) return 0;
    const wirklich = Math.min(heal.amount, this.damageDealtValue);
    this.damageDealtValue -= wirklich;
    this.bossHealedValue += wirklich;
    return wirklich;
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
   * Die kippende Reaktion heben (11.2, Baustein 6; Issue #240).
   *
   * **Die einzige Stelle, die sie hebt.** Schutz und Heilung rufen sie mit
   * ihrem eigenen Betrag; wer eine dritte Quelle anlegt, ruft hier ebenfalls
   * und muss sich nicht um die Obergrenze kümmern.
   *
   * Über `maxValue` hinaus wird nicht gehoben: Sonst ließe sich in ruhigen
   * Minuten ein Vorrat anlegen, und die Mechanik wäre in der Phase, in der sie
   * zählt, schon gewonnen.
   */
  private stabilise(amount: number): void {
    const def = this.boss.stabilise;
    if (def === null || this.reactionFailedValue || amount <= 0) return;
    const neu = this.reactionValue + amount;
    this.reactionValue = neu > def.maxValue ? def.maxValue : neu;
    this.stabilisedCountValue++;
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

  /** Prüft, ob der Boss gefallen ist. Nach `stepPassive`, vor dem Wirken. */
  checkDefeat(timeMs: number): boolean {
    if (this.stateValue !== FIGHT_RUNNING) return true;
    if (this.damageDealtValue >= this.boss.hp) {
      this.stateValue = FIGHT_WON;
      this.endedAtMs = timeMs;
      return true;
    }
    // Der Überdruck bemisst sich an der KAMPFuhr, nicht an der durchlaufenden:
    // Die Vorbereitung darf nichts von der Kampfzeit abziehen.
    if (this.fightElapsedMs(timeMs) >= this.boss.overpressureMs) {
      this.stateValue = FIGHT_OVERPRESSURE;
      this.endedAtMs = timeMs;
      return true;
    }
    return false;
  }

  // ----------------------------------------------------------------- Wirken

  /**
   * Einen Zauber wirken.
   *
   * @returns Das wiederverwendete Ergebnis-Objekt. Wer es behalten will, kopiert
   *   die Felder heraus — beim nächsten Aufruf steht etwas anderes darin.
   */
  cast(index: number, timeMs: number): CastOutcome {
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

    // Unterbrechen (GAME-DESIGN 11.2, Baustein 9).
    //
    // **Kein Sonderfall in der Wartezeit-Rechnung.** Dass dieser Zauber keine
    // globale Wartezeit auslöst, steht weiter unten als `spell.triggersGcd` und
    // kommt aus den Daten — hier wäre eine zweite Stelle, an der dieselbe Regel
    // hängt, und zwei Stellen laufen auseinander.
    if (spell.interrupt === true) this.interruptBossCast(timeMs);

    // Die drei Wege der Koagulation (Issue #79). Sie stehen VOR dem Schaden,
    // weil ein Zauber, der beides täte, erst schützen und dann zuschlagen
    // soll — heute ohne Wirkung (keiner macht beides) und deshalb jetzt
    // festzulegen und nicht dann.
    this.applyCoagulation(spell, timeMs);
    this.applyFixation(spell, timeMs);
    this.applyPutrefaction(spell, timeMs);

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
    const teiltSchadenAus = spell.baseDamage > 0 || spell.spender !== undefined;
    const kritet = teiltSchadenAus ? this.rollCrit() : false;
    const castFactors = this.factorsFor(kritet);

    let damage = 0;
    if (spell.baseDamage > 0) damage += computeDamage(spell.baseDamage, castFactors);

    const spender = spell.spender;
    if (spender !== undefined && spender.resource === this.school.resource.id) {
      const amount = this.resourceValue;
      let raw = spender.damagePerPoint * amount;
      if (amount >= spender.bonusThreshold) {
        // "ab 15 Hitze", nicht "über 14": Die Schwelle selbst zählt mit.
        raw *= spender.bonusMultiplier;
        outcome.bonusApplied = true;
      }
      damage += computeDamage(raw, castFactors);
      if (spender.consumesAll) this.resourceValue = 0;
    }

    if (damage > 0) {
      this.bookProgress(damage, index, timeMs);
      this.convertDamageToShield(damage, timeMs);
      this.reportHit(damage, HIT_DIRECT, kritet, timeMs);
      this.log?.push(this.fightElapsedMs(timeMs), EVENT_DAMAGE, index, damage);
      this.events.emit(RULE_DAMAGE, index, damage, timeMs);
      // **Hier stand bis #177: „Krit ruht in Schritt 1, ein Auslöser wäre
      // toter Code — die Kennung steht bereit, der Ort ist benannt."** Der Ort
      // war richtig benannt; jetzt hat er einen Fall. Der Auslöser steht NACH
      // `RULE_DAMAGE`, weil ein Krit eine Aussage über einen Treffer ist, den
      // es schon gegeben hat.
      if (kritet) this.events.emit(RULE_CRIT, index, damage, timeMs);
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
      this.dotExpiresAtMs[index] = timeMs + dot.durationMs;
      this.dotNextTickAtMs[index] = timeMs + this.dotInterval(dot);
    }

    // **Alle Abklingzeiten skalieren mit** (GAME-DESIGN 6.7) — anders als in
    // WoW, wo eine Ausnahmeliste je Fähigkeit entscheidet. Abklingzeiten sind
    // bei uns die alleinige Schwierigkeit (2.1); eine Liste, welcher Zauber
    // mitskaliert, wäre eine zweite Regel zum Auswendiglernen.
    this.cooldownReadyAtMs[index] = timeMs + this.hasted(spell.cooldownMs);
    if (spell.triggersGcd) this.gcd.trigger(timeMs);
    this.castCounts[index] = (this.castCounts[index] ?? 0) + 1;

    this.log?.push(this.fightElapsedMs(timeMs), EVENT_CAST, index, damage);
    // Zuletzt: Die Wirkung ist abgeschlossen — Schaden, Effekt über Zeit,
    // Vorrat, Abklingzeit und Wartezeit stehen.
    this.events.emit(RULE_SPELL_CAST, index, damage, timeMs);

    // **Die Schwaechung ohne Heil-Loesung** (11.2, Baustein 8; Issue #240).
    // Steht dieser Zauber in `clearedBy`, ist sie weg — und Heilung steht dort
    // bewusst NICHT, das ist der ganze Baustein.
    this.clearAffliction(spell.id);

    outcome.result = CAST_OK;
    outcome.damage = damage;
    return outcome;
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
    return this.spells[spellIndex]?.resourceGain?.[resourceId] ?? 0;
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
    return this.spells[spellIndex]?.cooldownReductionMs ?? 0;
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
  shortenCooldowns(amount: number, timeMs: number, exceptIndex: number): number {
    if (amount <= 0) return 0;
    let touched = 0;
    for (let i = 0; i < this.cooldownReadyAtMs.length; i++) {
      if (i === exceptIndex) continue;
      const readyAt = this.cooldownReadyAtMs[i] ?? 0;
      if (readyAt <= timeMs) continue;
      const shortened = readyAt - amount;
      this.cooldownReadyAtMs[i] = shortened > timeMs ? shortened : timeMs;
      touched++;
    }
    return touched;
  }

  private addResource(amount: number): void {
    if (amount === 0) return;
    const cap = this.school.resource.cap;
    const before = this.resourceValue;
    const raised = before + amount;
    this.resourceValue = raised > cap ? cap : raised;
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
    const cap = this.school.resource.cap;
    this.resourceValue = value < 0 ? 0 : value > cap ? cap : value;
  }

  reset(): void {
    this.cooldownReadyAtMs.fill(0);
    this.castCounts.fill(0);
    this.damageBySpell.fill(0);
    this.dotExpiresAtMs.fill(NOT_ACTIVE);
    this.dotNextTickAtMs.fill(NOT_ACTIVE);
    this.dotTicksLost.fill(0);
    this.dotTickCounts.fill(0);
    this.gcd.reset();
    this.random.reset();
    this.resourceValue = 0;
    this.resourceOverflowValue = 0;
    this.nextDecayAtMs = this.school.resource.decayPerStepMs;
    this.damageDealtValue = 0;
    this.stateValue = FIGHT_RUNNING;
    this.endedAtMs = NOT_ACTIVE;
    this.fightStartedAtMs = 0;
    // Nur die Folge, nicht die Anmeldungen: Ein neuer Versuch behält seine
    // Zauber und Talente — er beginnt lediglich wieder bei null Ereignissen.
    this.events.clearSequence();
  }
}
