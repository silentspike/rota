import { describe, expect, it } from 'vitest';
import { buildDummy, DEFAULT_DUMMY } from '../../src/analysis/training.js';
import { CALCINATION } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { replayRecording } from '../../src/devtools/recording.js';
import {
  DURATION_MS,
  measureStackScenario,
  STACK_SCENARIOS,
} from '../measurements/stack-capacity.js';

const runs = new Map(
  STACK_SCENARIOS.map((scenario) => [scenario.id, measureStackScenario(scenario)]),
);

function run(id: string) {
  const result = runs.get(id);
  if (result === undefined) throw new Error(`Missing measurement ${id}`);
  return result;
}

describe('#1102 production stack measurements', () => {
  it('all runs survive the full horizon and pay a real GCD for every successful spell', () => {
    for (const measurement of runs.values()) {
      expect(measurement.durationMs).toBe(120_000);
      expect(measurement.damageTaken).toBe(0);
      expect(measurement.casts.length).toBeGreaterThan(0);
      for (let i = 1; i < measurement.casts.length; i++) {
        const previous = measurement.casts[i - 1];
        const current = measurement.casts[i];
        expect(current!.timeMs - previous!.timeMs, measurement.id).toBeGreaterThanOrEqual(GCD_MS);
      }
    }
  });

  it('counts actual ROT ticks and direct cast events, including ROT cast GCDs', () => {
    expect(run('decay-direct').sourceEvents).toEqual({ canker: 30, blight: 30 });
    expect(run('decay-direct').sourceStacksPerSecond).toBeCloseTo(2 / 3, 12);
    expect(run('decay-rot-only').sourceEvents).toEqual({ rot: 90 });
    expect(run('decay-rot-only').sourceStacksPerSecond).toBe(1);
    expect(run('decay-rot').sourceEvents).toEqual({ rot: 80, canker: 20, blight: 20 });
    expect(run('decay-rot').sourceStacksPerSecond).toBeCloseTo(4 / 3, 12);
    expect(run('decay-rot').peak).toBe(5);
    expect(run('decay-rot').admittedStacks).toBe(32);
    expect(run('decay-rot').admittedStacksPerSecond).toBeLessThan(
      run('decay-rot').sourceStacksPerSecond,
    );
  });

  it('includes BLOOM maintenance in measured rates and reaches the production cap', () => {
    expect(run('decay-bloom-direct').peak).toBe(7);
    expect(run('decay-bloom-direct').sourceStacksPerSecond).toBe(0.6);
    expect(run('decay-bloom-rot').peak).toBe(7);
    expect(run('decay-bloom-rot').sourceEvents).toEqual({ rot: 74, canker: 18, blight: 18 });
    expect(run('decay-bloom-rot').sourceStacksPerSecond).toBeCloseTo(110 / 90, 12);
  });

  it('the same real eight-stack cap distinguishes ROT removal from SEPSIS substitution', () => {
    const direct = run('decay-eight-direct');
    const rot = run('decay-eight-rot');
    const sepsis = run('decay-eight-sepsis');
    for (const measured of [direct, rot, sepsis]) expect(measured.highestDecayCap).toBe(8);
    expect(direct.peak).toBe(7);
    expect(rot.peak).toBe(8);
    expect(rot.peakAtMs).toBe(6500);
    expect(sepsis.peak).toBe(8);
    expect(sepsis.peakAtMs).toBe(13500);
    expect(sepsis.casts.some(({ spellId }) => spellId === 'rot')).toBe(false);
    // Retained stacks are not new arrivals when SEPSIS moves their clocks.
    expect(sepsis.sourceStacksPerSecond).toBeGreaterThan(0);
    expect(sepsis.admittedStacks).toBe(0);
  });

  it('separates a narrow heat kit from a source that reaches the same SLAG cap', () => {
    expect(run('heat-kindle').peak).toBe(20);
    expect(run('heat-slag-kindle').peak).toBe(26);
    expect(run('heat-slag-bellows').peak).toBe(30);
  });

  it('measures shared shield refresh and checks an expiring shield against HARDEN', () => {
    expect(run('shield-crust').peak).toBe(3);
    expect(run('shield-crust-layers').peak).toBe(8);
    expect(run('shield-crust-layers').peakAtMs).toBe(42000);
    expect(run('shield-scab-layers').peak).toBe(8);
    expect(run('shield-scab-layers').peakAtMs).toBe(10500);
    expect(run('shield-plate-layers').peak).toBe(1);
    expect(run('shield-crucible-layers').peak).toBe(1);
    expect(run('shield-plate-harden-layers').peak).toBe(6);
    expect(run('shield-plate-harden-layers').peakAtMs).toBe(100000);
    expect(run('control-bind').peak).toBe(160);
  });

  it('recorded ROT inputs reproduce the checksum through the existing recording reader', () => {
    const measured = run('decay-bloom-rot');
    const boss = buildDummy(DEFAULT_DUMMY, CALCINATION.value.school);
    const bindings = measured.priority.map((spellId, slot) => ({ slot, spellId }));
    const replay = replayRecording(
      {
        name: measured.id,
        seed: measured.seed,
        ticks: DURATION_MS / measured.tickMs,
        bindings,
        presses: measured.casts.map(({ timeMs, spellId }) => ({
          tick: timeMs / measured.tickMs,
          slot: measured.priority.indexOf(spellId),
        })),
      },
      { boss, level: measured.level, worn: measured.worn },
    );
    expect(replay.rejected).toBe(0);
    expect(replay.accepted).toBe(measured.casts.length);
    expect(replay.damage).toBe(measured.damage);
    expect(replay.finalChecksum).toBe(measured.finalChecksum);
  });

  it('the measurement observer does not change the resulting fight state', () => {
    const scenario = STACK_SCENARIOS.find(({ id }) => id === 'decay-eight-sepsis');
    if (scenario === undefined) throw new Error('Missing SEPSIS control');
    const unobserved = measureStackScenario(scenario, false);
    expect(unobserved.finalChecksum).toBe(run(scenario.id).finalChecksum);
    expect(unobserved.peak).toBe(run(scenario.id).peak);
    expect(unobserved.casts).toEqual(run(scenario.id).casts);
  });
});
