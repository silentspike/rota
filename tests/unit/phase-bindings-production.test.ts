import { describe, expect, it } from 'vitest';
import { bossById, instanceById, type BossDef } from '../../src/core/data.js';
import { erwarteterStand } from '../../src/devtools/expected-gear.js';
import { wornFrom } from '../../src/gear/catalog.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';
import { PHASE_CASES, playPhaseBoss } from '../parity/phase-bindings-production.js';

const SEEDS = Array.from({ length: 32 }, (_, seed) => seed);

function requireBoss(id: string): BossDef {
  const boss = bossById(id);
  if (boss === undefined) throw new Error(`Missing production boss: ${id}`);
  return boss;
}

describe.each(PHASE_CASES)('$bossId: phase binding in progression (#1057)', (setup) => {
  const boss = requireBoss(setup.bossId);
  const worn = wornFrom(erwarteterStand(setup.previous, 10));

  it('the phase changes player damage without an endgame set, for every seed', () => {
    for (const seed of SEEDS) {
      const withPhases = playPhaseBoss(boss, worn, 'good', seed);
      const withoutPhases = playPhaseBoss({ ...boss, phases: [] }, worn, 'good', seed);
      expect(withPhases.setEffects).toEqual([]);
      expect(withoutPhases.setEffects).toEqual([]);
      expect(withPhases.highestPhase).toBeGreaterThan(0);
      expect(withoutPhases.highestPhase).toBe(0);
      expect(
        withPhases.damageTaken,
        `seed ${seed}: bound strike must actually land`,
      ).toBeGreaterThan(withoutPhases.damageTaken);
      expect(withPhases.playerDead).toBe(false);
      expect(withoutPhases.playerDead).toBe(false);
    }
  });

  it('removing the binding removes the measured phase effect', () => {
    const unbound = {
      ...boss,
      timeline: boss.timeline.map((event) => {
        const copy = { ...event };
        delete copy.phase;
        return copy;
      }),
    };
    const withPhases = playPhaseBoss(unbound, worn, 'good', 0);
    const withoutPhases = playPhaseBoss({ ...unbound, phases: [] }, worn, 'good', 0);
    expect(withPhases.highestPhase).toBeGreaterThan(0);
    expect(withPhases.damageTaken).toBeGreaterThan(0);
    expect(withPhases.damageTaken).toBe(withoutPhases.damageTaken);
    expect(withPhases.damage).toBe(withoutPhases.damage);
  });

  it('good rotation wins with time remaining and beats bad damage by at least 10%', () => {
    for (const seed of SEEDS) {
      const good = playPhaseBoss(boss, worn, 'good', seed);
      const bad = playPhaseBoss(boss, worn, 'bad', seed);
      expect(good.playerDead, `seed ${seed}`).toBe(false);
      expect(good.killedAtMs, `seed ${seed}: good must win`).toBeGreaterThan(0);
      expect(
        boss.overpressureMs - good.killedAtMs,
        `seed ${seed}: at least 1s remains`,
      ).toBeGreaterThanOrEqual(1000);
      // Both fights are still running at this shared time. Comparing damage
      // at each fight's own kill would cap both at boss HP and hide the gap.
      expect(good.killedAtMs).toBeGreaterThan(20000);
      expect(bad.killedAtMs === -1 || bad.killedAtMs > 20000).toBe(true);
      expect(bad.damageAt20s).toBeGreaterThan(0);
      expect(good.damageAt20s, `seed ${seed}: common 20s horizon`).toBeGreaterThanOrEqual(
        bad.damageAt20s * 1.1,
      );
    }
  });
});

it('first-instance gear comes from the actual prologue; naked entry is a separate lower bound', () => {
  expect(instanceById('prologue')?.number).toBe(1);
  const boss = requireBoss('boss-1-2');
  const naked = playPhaseBoss(boss, NOTHING_WORN, 'good', 0);
  const geared = playPhaseBoss(boss, wornFrom(erwarteterStand(1, 10)), 'good', 0);
  expect(naked.playerDead).toBe(false);
  expect(naked.killedAtMs).toBe(-1);
  expect(naked.damage).toBeLessThan(boss.hp);
  expect(geared.killedAtMs).toBeGreaterThan(0);
});
