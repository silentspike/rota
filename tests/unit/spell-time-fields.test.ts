import { describe, expect, it } from 'vitest';
import { parseSchool } from '../../src/core/data.js';

const files = import.meta.glob<Record<string, unknown>>('/data/spells/*.json', {
  eager: true,
  import: 'default',
});

function durations(value: unknown, path: (string | number)[] = []): (string | number)[][] {
  if (Array.isArray(value))
    return value.flatMap((child, index) => durations(child, [...path, index]));
  if (value === null || typeof value !== 'object') return [];
  return Object.entries(value).flatMap(([key, child]) => [
    ...(key.endsWith('Ms') ? [[...path, key]] : []),
    ...durations(child, [...path, key]),
  ]);
}

function setAt(value: unknown, path: (string | number)[], replacement: unknown): void {
  let node = value as Record<string | number, unknown>;
  for (const key of path.slice(0, -1)) node = node[key] as Record<string | number, unknown>;
  node[path[path.length - 1]!] = replacement;
}

describe('Every spell-data time has an executable validation contract (#1104)', () => {
  for (const [name, raw] of Object.entries(files)) {
    // The production loader combines all spell files with the one shared
    // resource from CALCINATION; auxiliary files deliberately omit it — and
    // common.json also omits the school name, which only CALCINATION carries
    // (sim/build.rs reads both from calcination.json alone).
    const calcination = files['/data/spells/calcination.json']!;
    const source = { school: calcination['school'], ...raw, resource: calcination['resource'] };
    it(`${name}: accepts original data and detects every off-grid duration`, () => {
      expect(parseSchool(source).problems).toEqual([]);
      const paths = durations(source);
      expect(paths.length).toBeGreaterThan(0);
      for (const path of paths) {
        if (path.at(-1) === 'cooldownReductionBelowMs') continue;
        const changed = structuredClone(source);
        setAt(changed, path, 12001);
        const result = parseSchool(changed);
        expect(result.problems.some((problem) => problem.includes(String(path.at(-1))))).toBe(true);
      }
    });
  }

  const original = files['/data/spells/calcination.json']!;
  it('a newly added duration cannot escape by being absent from a maintained field list', () => {
    expect(parseSchool({ ...original, futureEffect: { settlingMs: 12001 } }).problems).toContain(
      'school.futureEffect.settlingMs: settlingMs 12001 liegt nicht auf dem 10-ms-Raster.',
    );
    expect(parseSchool({ ...original, futureEffect: { settlingMs: 12000 } }).problems).toEqual([]);
  });

  it('rejects invalid types instead of silently treating durations as absent', () => {
    for (const value of [null, '12000', -10, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) {
      const changed = structuredClone(original);
      setAt(changed, ['spells', 0, 'extendShieldMs'], value);
      expect(
        parseSchool(changed).problems.some((problem) => problem.includes('extendShieldMs')),
      ).toBe(true);
    }
  });

  it('preserves the explicit off-grid threshold exception', () => {
    const changed = structuredClone(original);
    setAt(changed, ['spells', 0, 'cooldownReductionBelowMs'], 10005);
    expect(parseSchool(changed).problems).toEqual([]);
  });
});
