/**
 * Der Fallstrick aus Issue #83 mechanisch abgesichert.
 *
 * "Nur was besessen wurde, nie berechnete Werte" ist als guter Vorsatz
 * wertlos - der Fehler faellt erst auf, wenn eine Balance-Aenderung einen alten
 * Spielstand nicht erreicht, und dann ist er ein Jahr alt. Diese Tests brechen
 * am selben Tag, an dem jemand ein Feld zu viel aufnimmt.
 */
import { describe, expect, it } from 'vitest';
import {
  ALLOWED_TOP_LEVEL,
  collectFieldNames,
  findForbiddenFields,
  SAVE_SCHEMA,
  type SaveData,
} from '../../src/analysis/save-format.js';

function save(): SaveData {
  return {
    schema: SAVE_SCHEMA,
    gameVersion: '0.1.0',
    savedAt: 1_800_000_000_000,
    level: 42,
    experience: 128_400,
    talentPoints: { calcination: 5, fixation: 2 },
    items: [{ itemId: 'ember-band', upgradeSteps: 3, equippedSlot: 2 }],
    bindings: { '0': 'kindle', '1': 'smoulder', '2': 'pyre' },
    unlockedSpells: ['kindle', 'smoulder', 'pyre'],
    unlockedInstances: ['first-crucible'],
    bestRuns: [
      {
        bossId: 'first-crucible',
        difficulty: 1,
        timeMs: 184_200,
        gameVersion: '0.1.0',
        achievedAt: 1_799_000_000_000,
      },
    ],
    defeatedBosses: ['first-crucible'],
    runHistory: [],
    progressMarks: [],
  };
}

describe('Speicherformat — nur Besessenes, nichts Berechnetes', () => {
  it('enthaelt kein Feld, das nach einem berechneten Wert klingt', () => {
    expect(findForbiddenFields(save())).toEqual([]);
  });

  it('schlaegt an, sobald ein Schadenswert mitgespeichert wird', () => {
    const verdorben = { ...save(), items: [{ itemId: 'x', upgradeSteps: 1, bonusDamage: 40 }] };
    expect(findForbiddenFields(verdorben)).toContain('bonusDamage');
  });

  it('schlaegt bei jedem der sechs Grundwerte an', () => {
    for (const name of ['essence', 'vessel', 'ward', 'catalysis', 'purity', 'yield']) {
      expect(findForbiddenFields({ [name]: 1 }), name).toContain(name);
    }
  });

  it('schlaegt auch tief im Baum an, nicht nur oben', () => {
    const tief = { bestRuns: [{ bossId: 'a', detail: { spellPower: 3 } }] };
    expect(findForbiddenFields(tief)).toContain('spellPower');
  });

  it('haelt die Liste der obersten Felder abschliessend', () => {
    // Ein neues Feld muss BEWUSST aufgenommen werden. Faellt dieser Test, hat
    // jemand die Struktur erweitert, ohne die Regel zu bedenken.
    expect(Object.keys(save()).sort()).toEqual([...ALLOWED_TOP_LEVEL].sort());
  });

  it('vermerkt bei jeder Bestzeit die Fassung, unter der sie erzielt wurde', () => {
    // Eine Bestzeit aus einer anderen Balance ist keine vergleichbare Bestzeit.
    for (const run of save().bestRuns) {
      expect(run.gameVersion).toBeTruthy();
    }
  });
});

describe('collectFieldNames', () => {
  it('sammelt aus Objekten, Listen und Verschachtelungen', () => {
    const namen = collectFieldNames({ a: 1, b: [{ c: 2 }], d: { e: { f: 3 } } });
    expect([...namen].sort()).toEqual(['a', 'b', 'c', 'd', 'e', 'f']);
  });

  it('kommt mit null und leeren Werten zurecht', () => {
    expect([...collectFieldNames(null)]).toEqual([]);
    expect([...collectFieldNames({ a: null })]).toEqual(['a']);
  });
});
