/**
 * Der Aufstieg zeigt nur, was wirklich eintritt (Issue #371).
 *
 * Die Prüfung an **Stufe 7** ist die wichtigere: Dort gibt es weder Zauber
 * noch Platz, nur die Grundwerte. Eine Einblendung, die dort vier Zeilen
 * zeigt, von denen drei leer sind, wäre schlechter als keine.
 */
import { describe, expect, it } from 'vitest';
import { levelUpDisplayFor } from '../../src/analysis/level-up-display.js';
import { CALCINATION, LEVELS } from '../../src/core/data.js';

const SPELLS = CALCINATION.value.spells;

describe('Was eine Stufe bringt', () => {
  it('zeigt nichts ohne Aufstieg', () => {
    expect(levelUpDisplayFor(false, 7, SPELLS)).toBeNull();
  });

  it('nennt auf Stufe 3 den Zauber UND den Platz', () => {
    // Stufe 3 steht in barSlotLevels und trägt einen Zauber — der Fall, in
    // dem alles Sichtbare zusammenkommt.
    const d = levelUpDisplayFor(true, 3, SPELLS);
    expect(d?.level).toBe(3);
    expect(d?.newBarSlots).toBe(1);
    expect(d?.newSpells.length).toBeGreaterThan(0);
  });

  it('nennt auf Stufe 7 NUR die Grundwerte', () => {
    // **Der eigentliche Fall.** Ohne die Bedingung stünden hier zwei leere
    // Zeilen.
    const d = levelUpDisplayFor(true, 7, SPELLS);
    expect(d?.newSpells).toEqual([]);
    expect(d?.newBarSlots).toBe(0);
    expect(d?.statPercent).toBe(LEVELS.growthPerLevelPercent);
  });

  it('nennt nur die Zauber DIESER Stufe, nicht alle bisherigen', () => {
    // Der Gegenfall zur einfachsten Fehlbauart: `unlockLevel <= stufe` statt
    // `=== stufe`. Auf Stufe 3 wären das alle Zauber ab Stufe 1.
    const d = levelUpDisplayFor(true, 3, SPELLS);
    const alleBis3 = SPELLS.filter((s) => s.unlockLevel <= 3).length;
    expect(d?.newSpells.length).toBeLessThan(alleBis3);
  });

  it('gibt nie negative Plaetze', () => {
    // barSlotsAtLevel sinkt nie — aber die Differenz wird gerechnet, und eine
    // Rechnung kann sich irren.
    for (const stufe of [1, 2, 3, 7, 22, 90, 100]) {
      expect(levelUpDisplayFor(true, stufe, SPELLS)?.newBarSlots).toBeGreaterThanOrEqual(0);
    }
  });
});
