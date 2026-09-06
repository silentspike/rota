/** #1102: bounded measurements of production rules, without changing spell data. */
import { RULE_DOT_TICK, RULE_SPELL_CAST } from '../../src/combat/events.js';
import { buildDummy, DEFAULT_DUMMY } from '../../src/analysis/training.js';
import { alsSpellIndex, Combat, CAST_OK } from '../../src/core/combat.js';
import { combatChecksum } from '../../src/core/checksum.js';
import { CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';
import {
  allocationProblem,
  talentDecayStackPermille,
  type TalentAllocation,
} from '../../src/progression/talents.js';
import { barSlotsAtLevel, talentPointsAtLevel } from '../../src/progression/unlocks.js';

export const DURATION_MS = 120_000;
export const RATE_FROM_MS = 30_000;
const LEVEL = 100;
const CAP_TALENT = { 'putrefaction-trunk': 1, 'putrefaction-build': 3 };
const LAYERS_TALENT = { 'coagulation-trunk': 1, 'coagulation-stacks': 5 };

export interface StackScenario {
  readonly id: string;
  readonly metric: 'decay' | 'heat' | 'shield' | 'control';
  /** First ready eligible spell wins; all commands still go through Combat.cast. */
  readonly priority: readonly string[];
  readonly talents?: TalentAllocation;
  readonly bloomAt?: number;
  readonly sepsisAt?: number;
  readonly slagAt?: number;
}

export const STACK_SCENARIOS: readonly StackScenario[] = [
  { id: 'decay-direct', metric: 'decay', priority: ['canker', 'blight'] },
  { id: 'decay-rot-only', metric: 'decay', priority: ['rot'] },
  { id: 'decay-rot', metric: 'decay', priority: ['rot', 'canker', 'blight'] },
  {
    id: 'decay-bloom-direct',
    metric: 'decay',
    priority: ['putrid-bloom', 'canker', 'blight'],
    bloomAt: 4,
  },
  {
    id: 'decay-bloom-rot',
    metric: 'decay',
    priority: ['putrid-bloom', 'rot', 'canker', 'blight'],
    bloomAt: 4,
  },
  {
    id: 'decay-eight-direct',
    metric: 'decay',
    priority: ['putrid-bloom', 'canker', 'blight'],
    bloomAt: 4,
    talents: CAP_TALENT,
  },
  {
    id: 'decay-eight-rot',
    metric: 'decay',
    priority: ['putrid-bloom', 'rot', 'canker', 'blight'],
    bloomAt: 4,
    talents: CAP_TALENT,
  },
  {
    id: 'decay-eight-sepsis',
    metric: 'decay',
    priority: ['sepsis', 'putrid-bloom', 'canker', 'blight'],
    bloomAt: 4,
    sepsisAt: 4,
    talents: CAP_TALENT,
  },
  { id: 'heat-kindle', metric: 'heat', priority: ['kindle'] },
  { id: 'heat-slag-kindle', metric: 'heat', priority: ['slag', 'kindle'], slagAt: 20 },
  { id: 'heat-slag-bellows', metric: 'heat', priority: ['slag', 'bellows'], slagAt: 20 },
  { id: 'shield-crust', metric: 'shield', priority: ['crust'] },
  { id: 'shield-crust-layers', metric: 'shield', priority: ['crust'], talents: LAYERS_TALENT },
  { id: 'shield-scab-layers', metric: 'shield', priority: ['scab'], talents: LAYERS_TALENT },
  { id: 'shield-plate-layers', metric: 'shield', priority: ['plate'], talents: LAYERS_TALENT },
  {
    id: 'shield-plate-harden-layers',
    metric: 'shield',
    priority: ['harden', 'plate'],
    talents: LAYERS_TALENT,
  },
  {
    id: 'shield-crucible-layers',
    metric: 'shield',
    priority: ['crucible-skin'],
    talents: LAYERS_TALENT,
  },
  { id: 'control-bind', metric: 'control', priority: ['bind'] },
];

function expiries(combat: Combat): number[] {
  return Array.from({ length: combat.decayStacks }, (_, i) => combat.decayExpiryAt(i));
}

/** Read actual arrivals, including a tick that adds and expires stacks together. */
function countArrivals(before: number[], combat: Combat): number {
  let arrivals = 0;
  for (const expiry of expiries(combat)) {
    const old = before.indexOf(expiry);
    if (old < 0) arrivals++;
    else before.splice(old, 1);
  }
  return arrivals;
}

/**
 * The actual Training Room dummy keeps the full measurement alive. Its spell,
 * GCD, resource and talent rules are the production Combat implementation.
 * This is not a production-boss clear or an optimal-rotation search.
 */
export function measureStackScenario(scenario: StackScenario, observeEvents = true) {
  const boss = buildDummy(DEFAULT_DUMMY, CALCINATION.value.school);
  const talents = scenario.talents ?? {};
  const problem = allocationProblem(talents, talentPointsAtLevel(LEVEL));
  if (problem !== null) throw new Error(problem);
  // Source-event counts below mean one unmodified stack per production source.
  // Echo / fractional gain would need their own observer, not a copied formula.
  if (talentDecayStackPermille(talents) !== 0 || (talents['putrefaction-keystone'] ?? 0) > 0) {
    throw new Error('This measurement does not cover fractional gains or decay echo');
  }
  if (scenario.priority.length > barSlotsAtLevel(LEVEL))
    throw new Error('Kit exceeds bar capacity');
  const combat = new Combat({
    school: CALCINATION.value,
    boss,
    seed: 0,
    level: LEVEL,
    worn: NOTHING_WORN,
    talents,
  });
  const spells = scenario.priority.map((id) => {
    const index = combat.indexOf(id);
    const spell = combat.spellAt(index);
    if (spell === undefined || spell.unlockLevel > LEVEL)
      throw new Error(`Unavailable spell ${id}`);
    if (spell.castTimeMs !== 0 || spell.channelMs !== 0 || !spell.triggersGcd) {
      throw new Error(`This measurement expects instant GCD spells: ${id}`);
    }
    return { id, index };
  });
  const sourceStacks: Record<string, number> = {};
  const sourceEvents: Record<string, number> = {};
  const casts: { timeMs: number; spellId: string }[] = [];
  let admittedStacks = 0;
  let peak = 0;
  let peakAtMs = -1;
  let highestDecayCap = 0;
  const inRateWindow = (timeMs: number) => timeMs >= RATE_FROM_MS && timeMs < DURATION_MS;
  if (observeEvents) {
    const receiver = {
      onRuleEvent(event: number, subject: number, _value: number, timeMs: number) {
        if (!inRateWindow(timeMs)) return;
        const spell = combat.spellAt(alsSpellIndex(subject));
        if (spell === undefined) return;
        const amount =
          event === RULE_SPELL_CAST ? (spell.decay?.stacks ?? 0) : (spell.dot?.decayPerTick ?? 0);
        if (amount <= 0) return;
        sourceEvents[spell.id] = (sourceEvents[spell.id] ?? 0) + 1;
        sourceStacks[spell.id] = (sourceStacks[spell.id] ?? 0) + amount;
      },
    };
    combat.events.register({ id: 'measurement:cast', on: RULE_SPELL_CAST }, receiver);
    combat.events.register({ id: 'measurement:tick', on: RULE_DOT_TICK }, receiver);
  }
  const sample = (timeMs: number) => {
    const value =
      scenario.metric === 'decay'
        ? combat.decayStacks
        : scenario.metric === 'heat'
          ? combat.resource
          : scenario.metric === 'shield'
            ? combat.shieldStacks
            : combat.control;
    if (value > peak) {
      peak = value;
      peakAtMs = timeMs;
    }
    highestDecayCap = Math.max(highestDecayCap, combat.decayStackCap);
  };
  for (let timeMs = 0; timeMs < DURATION_MS; timeMs += TICK_MS) {
    const beforeTick = expiries(combat);
    combat.stepPassive(timeMs);
    if (combat.playerDead || combat.bossHpRemaining <= 0)
      throw new Error(`Carrier ended at ${timeMs}`);
    if (inRateWindow(timeMs)) admittedStacks += countArrivals(beforeTick, combat);
    sample(timeMs);
    for (const { id, index } of spells) {
      if (id === 'putrid-bloom' && combat.decayStacks < (scenario.bloomAt ?? 4)) continue;
      if (id === 'sepsis' && combat.decayStacks < (scenario.sepsisAt ?? 4)) continue;
      if (id === 'slag' && combat.resource < (scenario.slagAt ?? 20)) continue;
      if (id === 'harden' && combat.shieldStacks === 0) continue;
      if (!combat.isReady(index, timeMs)) continue;
      const beforeCast = expiries(combat);
      if (combat.cast(index, timeMs).result !== CAST_OK)
        throw new Error(`Ready cast rejected: ${id}`);
      casts.push({ timeMs, spellId: id });
      // SEPSIS moves existing clocks; it never creates stacks.
      if (inRateWindow(timeMs) && id !== 'sepsis')
        admittedStacks += countArrivals(beforeCast, combat);
      sample(timeMs);
      break;
    }
  }
  const sourceTotal = Object.values(sourceStacks).reduce((a, b) => a + b, 0);
  const seconds = (DURATION_MS - RATE_FROM_MS) / 1000;
  return {
    id: scenario.id,
    metric: scenario.metric,
    priority: scenario.priority,
    talents,
    bossId: boss.id,
    level: LEVEL,
    worn: NOTHING_WORN,
    seed: 0,
    durationMs: DURATION_MS,
    rateFromMs: RATE_FROM_MS,
    tickMs: TICK_MS,
    peak,
    peakAtMs,
    highestDecayCap: scenario.metric === 'decay' ? highestDecayCap : null,
    sourceEvents,
    sourceStacks,
    sourceStacksPerSecond: sourceTotal / seconds,
    admittedStacks,
    admittedStacksPerSecond: admittedStacks / seconds,
    casts,
    damage: combat.damageDealt,
    damageTaken: combat.damageTaken,
    finalChecksum: combatChecksum(combat, DURATION_MS - TICK_MS),
  };
}

export function stackCapacityReport() {
  return STACK_SCENARIOS.map((scenario) => {
    const { casts, ...measurement } = measureStackScenario(scenario);
    const castCounts: Record<string, number> = {};
    for (const { spellId } of casts) castCounts[spellId] = (castCounts[spellId] ?? 0) + 1;
    return { ...measurement, castCounts };
  });
}
