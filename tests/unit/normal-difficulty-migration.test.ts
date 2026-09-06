import { describe, expect, it } from 'vitest';
import { emptySave, RunPersistence } from '../../src/analysis/run-persistence.js';
import { SaveStore, type KeyValueStore } from '../../src/analysis/save-store.js';
import { SAVE_SCHEMA } from '../../src/analysis/save-format.js';
import { markKey } from '../../src/analysis/progress-mark.js';
import { isRunHistoryEntry } from '../../src/analysis/entry-shape.js';

function memoryStore(): KeyValueStore {
  const map = new Map<string, string>();
  return {
    getItem: (key) => map.get(key) ?? null,
    setItem: (key, value) => {
      map.set(key, value);
    },
    removeItem: (key) => {
      map.delete(key);
    },
  };
}

const run = {
  bossId: 'first-crucible',
  difficulty: 1,
  at: 123,
  gameVersion: '0.1.0',
  victory: true,
  elapsedMs: 40000,
  remainingPercent: 0,
  damage: 3000,
  idleMs: 0,
  missedMs: 0,
};

describe('Migration der früheren Normal-Läufe', () => {
  it('erhält Besitz und Ergebnisse und verschiebt alle Schlüssel gemeinsam', () => {
    const storage = memoryStore();
    const old = {
      ...emptySave('0.1.0', 123),
      schema: 3,
      level: 20,
      experience: 321,
      items: [{ itemId: 'focus-mix', upgradeSteps: 2, equippedSlot: 0, itemLevel: 4, quality: 1 }],
      runHistory: [run],
      bestRuns: [
        {
          bossId: run.bossId,
          difficulty: 1,
          metric: 'reserve' as const,
          value: 20000,
          gameVersion: run.gameVersion,
          achievedAt: run.at,
        },
      ],
      progressMarks: [
        {
          bossId: run.bossId,
          difficulty: 1,
          bestRemainingPercent: 0,
          attempts: 3,
          defeated: true,
          gameVersion: run.gameVersion,
          at: run.at,
        },
      ],
      defeatedBosses: [markKey(run.bossId, 1)],
    };
    new SaveStore(storage).save(old);
    const migrated = new RunPersistence('next', () => 456, storage);
    expect(migrated.save.schema).toBe(SAVE_SCHEMA);
    expect(migrated.state.items).toEqual(old.items);
    expect(migrated.state.level).toBe(20);
    expect(migrated.state.experience).toBe(321);
    expect(migrated.state.runHistory).toEqual([{ ...run, difficulty: 0 }]);
    expect(migrated.state.bestRuns).toEqual([{ ...old.bestRuns[0], difficulty: 0 }]);
    expect(migrated.state.progressMarks).toEqual([{ ...old.progressMarks[0], difficulty: 0 }]);
    expect(migrated.state.defeatedBosses).toEqual([markKey(run.bossId, 0)]);
    expect(migrated.persist(migrated.state)).toBe(true);
    expect(new RunPersistence('next', () => 789, storage).save).toEqual(migrated.save);
  });

  it('verschiebt Heroisch in einem aktuellen Spielstand nicht erneut', () => {
    const storage = memoryStore();
    new SaveStore(storage).save({ ...emptySave('next', 123), runHistory: [run] });
    expect(new RunPersistence('next', () => 456, storage).state.runHistory).toEqual([run]);
  });

  it('liest alte Läufe ohne Beutestartwert, verwirft aber beschädigte Startwerte', () => {
    expect(isRunHistoryEntry(run)).toBe(true);
    expect(isRunHistoryEntry({ ...run, lootSeed: 0xffffffff })).toBe(true);
    for (const lootSeed of [-1, 1.5, 0x100000000, null, '42']) {
      expect(isRunHistoryEntry({ ...run, lootSeed })).toBe(false);
    }
  });
});
