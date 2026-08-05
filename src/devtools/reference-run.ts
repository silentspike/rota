/**
 * Durchrechnen einer Runde nach einer festen Prioritätenliste — **Werkzeug, kein
 * Spielinhalt** (GAME-DESIGN 19.1).
 *
 * Steht hinter `__ROTA_DEVTOOLS__` und fällt aus der Auslieferung heraus. Das ist
 * nicht nur Sparsamkeit: GAME-DESIGN 18.5 verlangt ausdrücklich, dass die
 * Referenz-Rotationen **nirgends im Spiel** erscheinen, auch nicht als Lernhilfe.
 * Das Herausfinden der besten Reihenfolge ist der Kern des Spiels.
 *
 * Wozu es hier gebraucht wird: Der Abgleich mit `tools/step1-check.py`. Beide
 * Seiten müssen bei derselben Prioritätenliste denselben Schaden ergeben, sonst
 * stimmt eine Regel nicht (GAME-DESIGN 18.4).
 *
 * Die beiden Listen sind wortgleich zu `apl_good` und `apl_bad` der Referenz.
 */
import { CALCINATION, FIRST_CRUCIBLE } from '../core/data.js';
import { Combat, FIGHT_RUNNING, type SpellIndex, alsSpellIndex } from '../core/combat.js';
import { GCD_MS } from '../core/gcd.js';
import { TICK_MS } from '../core/loop.js';

/** Nichts wirken. */
const IDLE = -1;

export type RotationName = 'good' | 'bad' | 'dot-error' | 'pyre-error';

export interface ReferenceResult {
  readonly rotation: RotationName;
  readonly damage: number;
  /** Kampfzeit des Sieges in ms, oder -1, wenn der Boss überlebt hat. */
  readonly killedAtMs: number;
  readonly remainingMs: number;
  readonly casts: Readonly<Record<string, number>>;
  readonly damageBySpell: Readonly<Record<string, number>>;
  readonly lostDotTicks: number;
  readonly resourceOverflow: number;
  readonly idleMs: number;
  readonly bossHpFraction: number;
  /** Ziehungen aus den Zufallsströmen. In Schritt 1 muss das 0 sein. */
  readonly randomDraws: number;
}

interface Indices {
  readonly kindle: SpellIndex;
  readonly smoulder: SpellIndex;
  readonly pyre: SpellIndex;
}

/**
 * Gute Rotation: nutzt beide Entscheidungen, die Schritt 1 anbietet.
 *
 * 1. SMOULDER erst erneuern, wenn er ausläuft — keine Sekunde früher
 * 2. PYRE erst ab der Bonus-Schwelle zünden
 * 3. sonst KINDLE als Füller
 */
function aplGood(combat: Combat, index: Indices, timeMs: number): number {
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < GCD_MS &&
    combat.isReady(index.smoulder, timeMs)
  ) {
    return index.smoulder;
  }
  const spender = combat.spellAt(index.pyre)?.spender;
  const threshold = spender?.bonusThreshold ?? 0;
  if (combat.isReady(index.pyre, timeMs) && combat.resource >= threshold) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

/** Schlechte Rotation: plausibel falsch, nicht absichtlich dumm. */
function aplBad(combat: Combat, index: Indices, timeMs: number): number {
  const half = Math.floor((combat.spellAt(index.smoulder)?.dot?.durationMs ?? 0) / 2);
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < half &&
    combat.isReady(index.smoulder, timeMs)
  )
    return index.smoulder;
  if (combat.isReady(index.pyre, timeMs) && combat.resource > 0) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

/** Nur der DoT-Fehler: erneuert zu früh, zündet PYRE aber richtig. */
function aplDotError(combat: Combat, index: Indices, timeMs: number): number {
  const half = Math.floor((combat.spellAt(index.smoulder)?.dot?.durationMs ?? 0) / 2);
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < half &&
    combat.isReady(index.smoulder, timeMs)
  )
    return index.smoulder;
  const threshold = combat.spellAt(index.pyre)?.spender?.bonusThreshold ?? 0;
  if (combat.isReady(index.pyre, timeMs) && combat.resource >= threshold) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

/** Nur der PYRE-Fehler: DoT richtig, aber jede Zündung zu früh. */
function aplPyreError(combat: Combat, index: Indices, timeMs: number): number {
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < GCD_MS &&
    combat.isReady(index.smoulder, timeMs)
  ) {
    return index.smoulder;
  }
  if (combat.isReady(index.pyre, timeMs) && combat.resource > 0) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

const ROTATIONS: Record<RotationName, (c: Combat, i: Indices, t: number) => number> = {
  good: aplGood,
  bad: aplBad,
  'dot-error': aplDotError,
  'pyre-error': aplPyreError,
};

export interface ReferenceOptions {
  /** Kampfdauer. Ohne Angabe der Überdruck-Zeitpunkt des Bosses. */
  readonly durationMs?: number;
  /**
   * Beim Sieg abbrechen. `false` rechnet die vollen Sekunden durch — so werden
   * zwei Rotationen über dieselbe Zeit verglichen (STEP1-SPEC § 4).
   */
  readonly stopOnKill?: boolean;
}

/** Eine Runde nach fester Prioritätenliste durchrechnen. */
export function runReference(
  rotation: RotationName,
  options: ReferenceOptions = {},
): ReferenceResult {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const index: Indices = {
    kindle: combat.indexOf('kindle'),
    smoulder: combat.indexOf('smoulder'),
    pyre: combat.indexOf('pyre'),
  };
  const apl = ROTATIONS[rotation];
  const durationMs = options.durationMs ?? combat.boss.overpressureMs;
  const stopOnKill = options.stopOnKill !== false;

  let killedAtMs = -1;
  let idleMs = 0;
  let timeMs = 0;

  while (timeMs < durationMs) {
    combat.stepPassive(timeMs);
    if (killedAtMs < 0 && combat.damageDealt >= combat.boss.hp) {
      killedAtMs = timeMs;
      if (stopOnKill) break;
    }
    if (combat.gcd.isReady(timeMs)) {
      const choice = apl(combat, index, timeMs);
      if (choice === IDLE) idleMs += TICK_MS;
      else combat.cast(alsSpellIndex(choice), timeMs);
    }
    timeMs += TICK_MS;
  }

  const casts: Record<string, number> = {};
  const damageBySpell: Record<string, number> = {};
  let lostDotTicks = 0;
  for (let i = 0; i < combat.spells.length; i++) {
    const id = combat.spells[i]?.id ?? String(i);
    casts[id] = combat.castCount(alsSpellIndex(i));
    damageBySpell[id] = combat.damageOf(alsSpellIndex(i));
    lostDotTicks += combat.lostDotTicks(i);
  }

  return {
    rotation,
    damage: combat.damageDealt,
    killedAtMs,
    remainingMs: killedAtMs < 0 ? 0 : durationMs - killedAtMs,
    casts,
    damageBySpell,
    lostDotTicks,
    resourceOverflow: combat.resourceOverflow,
    idleMs,
    bossHpFraction: combat.bossHpFraction,
    randomDraws: combat.random.totalDraws,
  };
}

export interface CooldownMeasurement {
  readonly spellId: string;
  readonly cycles: number;
  /** Erwartete Gesamtzeit: Zyklen × Abklingzeit. */
  readonly expectedMs: number;
  /** Tatsächlich vergangene Zeit zwischen erster und letzter Wirkung. */
  readonly actualMs: number;
  /** Abweichung in ms. Bei ganzzahliger Zeit muss sie 0 sein. */
  readonly driftMs: number;
  /** Kleinster und größter Abstand zwischen zwei Wirkungen. */
  readonly minIntervalMs: number;
  readonly maxIntervalMs: number;
}

/**
 * Abklingzeit über viele Zyklen messen (VERIFICATION § 3.3).
 *
 * Der Zauber wird gewirkt, sobald er bereit ist, und zwar `cycles + 1` Mal. Die
 * Zeit zwischen erster und letzter Wirkung muss exakt `cycles × Abklingzeit`
 * betragen — ganzzahlige Zeit darf gar nicht driften.
 */
export function measureCooldown(spellId: string, cycles: number): CooldownMeasurement {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const index = combat.indexOf(spellId);
  const spell = combat.spellAt(index);
  if (spell === undefined) {
    return {
      spellId,
      cycles: 0,
      expectedMs: 0,
      actualMs: 0,
      driftMs: Number.NaN,
      minIntervalMs: 0,
      maxIntervalMs: 0,
    };
  }

  // Der Zauber darf den Boss nicht legen — sonst endet die Messung vorzeitig.
  const step = Math.max(spell.cooldownMs, combat.gcd.lengthMs);
  const castTimes: number[] = [];
  let timeMs = 0;
  const limitMs = (cycles + 2) * step + 1000;
  while (timeMs < limitMs && castTimes.length <= cycles) {
    combat.stepPassive(timeMs);
    if (combat.isReady(index, timeMs)) {
      combat.cast(index, timeMs);
      castTimes.push(timeMs);
    }
    timeMs += TICK_MS;
  }

  let minIntervalMs = Number.POSITIVE_INFINITY;
  let maxIntervalMs = 0;
  for (let i = 1; i < castTimes.length; i++) {
    const gap = (castTimes[i] as number) - (castTimes[i - 1] as number);
    if (gap < minIntervalMs) minIntervalMs = gap;
    if (gap > maxIntervalMs) maxIntervalMs = gap;
  }
  const measured = castTimes.length - 1;
  const actualMs = measured > 0 ? (castTimes[measured] as number) - (castTimes[0] as number) : 0;
  const expectedMs = measured * step;

  return {
    spellId,
    cycles: measured,
    expectedMs,
    actualMs,
    driftMs: actualMs - expectedMs,
    minIntervalMs: measured > 0 ? minIntervalMs : 0,
    maxIntervalMs,
  };
}

/** Nur damit der Zustand des Kampfes lesbar bleibt, wenn er nicht mehr läuft. */
export const REFERENCE_FIGHT_RUNNING = FIGHT_RUNNING;
