import { describe, expect, it } from 'vitest';
import rawSchool from '../../data/spells/calcination.json';
import rawCoagulation from '../../data/spells/coagulation.json';
import { CAST_STARTED, Combat, isCastAccepted } from '../../src/core/combat.js';
import { bossById, parseSchool, type BossDef } from '../../src/core/data.js';

function channelSchool(tick: number | undefined) {
  const carrier = rawCoagulation.spells.find((spell) => spell.id === 'decoction');
  expect(carrier).toBeDefined();
  const spell: Record<string, unknown> = { ...carrier };
  if (tick === undefined) delete spell['channelTickMs'];
  else spell['channelTickMs'] = tick;
  return parseSchool({ ...rawSchool, spells: [spell] });
}

describe('invalid channel tick cannot reach the combat loop (#1055)', () => {
  it.each([undefined, 0])('rejects the real channel with channelTickMs=%s', (tick) => {
    const parsed = channelSchool(tick);
    expect(parsed.problems.some((problem) => problem.includes('channelTickMs'))).toBe(true);
    expect(parsed.value.spells.find((spell) => spell.id === 'decoction')).toBeUndefined();

    const combat = new Combat({
      school: parsed.value,
      boss: bossById('test-warden') as BossDef,
    });
    combat.beginFight(0);
    expect(isCastAccepted(combat.cast(combat.indexOf('decoction'), 0).result)).toBe(false);
    combat.stepPassive(0);
    expect(combat.damageDealt).toBe(0);
  });

  it('still resolves the real channel with a positive tick interval', () => {
    const parsed = channelSchool(1000);
    expect(parsed.problems).toEqual([]);
    const combat = new Combat({
      school: parsed.value,
      boss: bossById('test-warden') as BossDef,
    });
    combat.beginFight(0);
    expect(combat.cast(combat.indexOf('decoction'), 0).result).toBe(CAST_STARTED);
    for (let time = 0; time <= 1000; time += 10) combat.stepPassive(time);
    expect(combat.damageDealt).toBeGreaterThan(0);
  });

  it('allows a nonchannel spell to omit both channel fields', () => {
    const spell = { id: 'instant', baseDamage: 10, castTimeMs: 0, cooldownMs: 0 };
    const parsed = parseSchool({ ...rawSchool, spells: [spell] });
    expect(parsed.problems).toEqual([]);
    expect(parsed.value.spells.map((entry) => entry.id)).toEqual(['instant']);
  });
});
