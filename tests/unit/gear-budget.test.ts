import { describe, expect, it } from 'vitest';
import {
  MAX_ITEM_LEVEL,
  QUALITY_COMMON,
  QUALITY_COUNT,
  QUALITY_EPIC,
  QUALITY_HAS_EFFECT,
  QUALITY_LEGENDARY,
  QUALITY_NAMES,
  QUALITY_RARE,
  QUALITY_STAT_SLOTS,
  budgetOf,
  distributeBudget,
  shareOfSlot,
} from '../../src/gear/budget.js';

/** Die Werte eines Gegenstands, als gewöhnliche Liste. */
function werte(level: number, quality: number): number[] {
  const out = new Int32Array(8);
  const count = distributeBudget(level, quality, out);
  return Array.from(out.subarray(0, count));
}

describe('Höhere Stufe heißt durchweg höhere Werte (AC1)', () => {
  it('steigt auf JEDEM Werte-Platz, über alle 100 Stufen und alle Qualitäten', () => {
    // „durchweg" wörtlich: nicht die Summe, nicht der erste Platz — jeder
    // einzelne. Geprüft wird die gesamte Stufenleiter, nicht eine Stichprobe.
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      for (let level = 1; level < MAX_ITEM_LEVEL; level++) {
        const unten = werte(level, quality);
        const oben = werte(level + 1, quality);
        expect(oben.length).toBe(unten.length);
        for (let slot = 0; slot < unten.length; slot++) {
          expect(oben[slot]).toBeGreaterThan(unten[slot] as number);
        }
      }
    }
  });

  it('hängt an der Wachstumsrate, nicht an der Verteilungsformel', () => {
    // **Dieser Prüffall stand zuerst falsch da.** Er behauptete, die
    // naheliegende Verteilung — ⌊budget/plätze⌋ plus Rest auf die vorderen
    // Plätze — würde AC1 brechen und die geschlossene Form rette es. Gemessen
    // stimmt das nicht: Beide sind DIESELBE Funktion. Mit budget = q·plätze + r
    // wird ⌊(budget+plätze−1−index)/plätze⌋ zu q + (index < r ? 1 : 0).
    const naiv = (budget: number, slots: number, index: number): number =>
      Math.floor(budget / slots) + (index < budget % slots ? 1 : 0);
    for (let budget = 0; budget <= 2000; budget++) {
      for (let slots = 1; slots <= 4; slots++) {
        for (let index = 0; index < slots; index++) {
          expect(shareOfSlot(budget, slots, index)).toBe(naiv(budget, slots, index));
        }
      }
    }

    // Woran AC1 WIRKLICH hängt: Das Budget muss je Stufe um mindestens so
    // viel wachsen, wie es Plätze gibt. Bei einem Zuwachs von 1 könnte gar
    // keine Verteilung alle Plätze anheben — die Summe stiege sonst um die
    // Zahl der Plätze.
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      const slots = QUALITY_STAT_SLOTS[quality] as number;
      for (let level = 1; level < MAX_ITEM_LEVEL; level++) {
        const zuwachs = budgetOf(level + 1, quality) - budgetOf(level, quality);
        expect(zuwachs).toBeGreaterThanOrEqual(slots);
      }
    }
  });

  it('kippt, sobald das Wachstum unter die Zahl der Plätze fällt', () => {
    // Der Gegenbeweis zur Zeile darüber — und die Warnung an #118: Ein
    // kleineres Grundbudget oder flacheres Wachstum bricht AC1, ohne dass
    // jemand die Verteilung angefasst hätte.
    const knapp = 10; // Budget-Schritt von 1 bei 3 Plätzen
    const vorher = [0, 1, 2].map((i) => shareOfSlot(knapp, 3, i));
    const nachher = [0, 1, 2].map((i) => shareOfSlot(knapp + 1, 3, i));
    expect(nachher.filter((wert, i) => wert > (vorher[i] as number)).length).toBe(1);
    expect(nachher.filter((wert, i) => wert === vorher[i]).length).toBe(2);
  });

  it('verteilt das Budget restlos', () => {
    // Sonst verschwände Wert im Rundungsspalt, und niemand sähe es.
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      for (let level = 1; level <= MAX_ITEM_LEVEL; level++) {
        const summe = werte(level, quality).reduce((a, b) => a + b, 0);
        expect(summe).toBe(budgetOf(level, quality));
      }
    }
  });

  it('trägt keine Zahl von Hand ein', () => {
    // 13.3: „Kein Gegenstand wird von Hand balanciert." Der Nachweis: 400
    // verschiedene Gegenstände (100 Stufen × 4 Qualitäten) entstehen aus
    // Stufe und Qualität allein — es gibt keine Tabelle, die sie enthielte.
    const alle = new Set<string>();
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      for (let level = 1; level <= MAX_ITEM_LEVEL; level++) {
        alle.add(`${quality}:${werte(level, quality).join(',')}`);
      }
    }
    expect(alle.size).toBe(QUALITY_COUNT * MAX_ITEM_LEVEL);
  });
});

describe('Die Qualitäten (13.3)', () => {
  it('trägt Werte-Plätze 1, 2, 3, 3', () => {
    expect(QUALITY_STAT_SLOTS[QUALITY_COMMON]).toBe(1);
    expect(QUALITY_STAT_SLOTS[QUALITY_RARE]).toBe(2);
    expect(QUALITY_STAT_SLOTS[QUALITY_EPIC]).toBe(3);
    // „3+" gelesen als: drei Plätze UND ein Effekt — nicht als vierter Platz.
    expect(QUALITY_STAT_SLOTS[QUALITY_LEGENDARY]).toBe(3);
    expect(QUALITY_HAS_EFFECT[QUALITY_LEGENDARY]).toBe(true);
    expect(QUALITY_HAS_EFFECT[QUALITY_EPIC]).toBe(false);
    expect(QUALITY_NAMES.length).toBe(QUALITY_COUNT);
  });

  it('gibt einer höheren Qualität auf gleicher Stufe mehr Budget', () => {
    for (let level = 1; level <= MAX_ITEM_LEVEL; level++) {
      expect(budgetOf(level, QUALITY_RARE)).toBeGreaterThan(budgetOf(level, QUALITY_COMMON));
      expect(budgetOf(level, QUALITY_EPIC)).toBeGreaterThan(budgetOf(level, QUALITY_RARE));
    }
  });

  it('lässt Qualität und Gegenstandsstufe unabhängig laufen (13.4)', () => {
    // „Ein episches Stück aus einer frühen Instanz ist schwächer als ein
    // seltenes aus dem Endgame — alter Inhalt entwertet sich von selbst."
    expect(budgetOf(1, QUALITY_EPIC)).toBeLessThan(budgetOf(30, QUALITY_RARE));
  });
});

describe('Was außerhalb liegt, fällt auf', () => {
  it('gibt für unmögliche Stufen und Qualitäten null zurück', () => {
    expect(budgetOf(0, QUALITY_COMMON)).toBe(0);
    expect(budgetOf(MAX_ITEM_LEVEL + 1, QUALITY_COMMON)).toBe(0);
    expect(budgetOf(1.5, QUALITY_COMMON)).toBe(0);
    expect(budgetOf(1, QUALITY_COUNT)).toBe(0);
    expect(budgetOf(1, -1)).toBe(0);
  });

  it('beschreibt nie mehr Plätze als das Ziel hat', () => {
    const eng = new Int32Array(2);
    expect(distributeBudget(10, QUALITY_LEGENDARY, eng)).toBe(2);
  });
});
