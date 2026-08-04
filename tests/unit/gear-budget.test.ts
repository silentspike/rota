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
  QUALITY_AFFIX_SLOTS,
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
    // Gesammelt statt je Platz geprüft — siehe Kopf von `log.test.ts`:
    // `expect` in einer heißen Schleife ist selbst die Messgröße (#127).
    const rueckschritte: string[] = [];
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      for (let level = 1; level < MAX_ITEM_LEVEL; level++) {
        const unten = werte(level, quality);
        const oben = werte(level + 1, quality);
        if (oben.length !== unten.length) {
          rueckschritte.push(`q${quality} Stufe ${level}: andere Zahl Plätze`);
          continue;
        }
        for (let slot = 0; slot < unten.length; slot++) {
          if (!((oben[slot] as number) > (unten[slot] as number))) {
            rueckschritte.push(
              `q${quality} Stufe ${level}->${level + 1} Platz ${slot}: ${unten[slot]} -> ${oben[slot]}`,
            );
          }
        }
      }
    }
    expect(rueckschritte).toEqual([]);
  });

  it('hängt an der Wachstumsrate, nicht an der Verteilungsformel', () => {
    // **Dieser Prüffall stand zuerst falsch da.** Er behauptete, die
    // naheliegende Verteilung — ⌊budget/plätze⌋ plus Rest auf die vorderen
    // Plätze — würde AC1 brechen und die geschlossene Form rette es. Gemessen
    // stimmt das nicht: Beide sind DIESELBE Funktion. Mit budget = q·plätze + r
    // wird ⌊(budget+plätze−1−index)/plätze⌋ zu q + (index < r ? 1 : 0).
    const naiv = (budget: number, slots: number, index: number): number =>
      Math.floor(budget / slots) + (index < budget % slots ? 1 : 0);
    const unterschiede: string[] = [];
    for (let budget = 0; budget <= 2000; budget++) {
      for (let slots = 1; slots <= 4; slots++) {
        for (let index = 0; index < slots; index++) {
          if (shareOfSlot(budget, slots, index) !== naiv(budget, slots, index)) {
            unterschiede.push(`budget ${budget}, ${slots} Plätze, Platz ${index}`);
          }
        }
      }
    }
    expect(unterschiede).toEqual([]);

    // Woran AC1 WIRKLICH hängt: Das Budget muss je Stufe um mindestens so
    // viel wachsen, wie es Plätze gibt. Bei einem Zuwachs von 1 könnte gar
    // keine Verteilung alle Plätze anheben — die Summe stiege sonst um die
    // Zahl der Plätze.
    const zuKnapp: string[] = [];
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      const slots = QUALITY_AFFIX_SLOTS[quality] as number;
      for (let level = 1; level < MAX_ITEM_LEVEL; level++) {
        const zuwachs = budgetOf(level + 1, quality) - budgetOf(level, quality);
        if (zuwachs < slots)
          zuKnapp.push(`q${quality} Stufe ${level}: Zuwachs ${zuwachs} < ${slots}`);
      }
    }
    expect(zuKnapp).toEqual([]);
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
    const verloren: string[] = [];
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      for (let level = 1; level <= MAX_ITEM_LEVEL; level++) {
        const summe = werte(level, quality).reduce((a, b) => a + b, 0);
        if (summe !== budgetOf(level, quality)) {
          verloren.push(`q${quality} Stufe ${level}: ${summe} statt ${budgetOf(level, quality)}`);
        }
      }
    }
    expect(verloren).toEqual([]);
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
  it('trägt Affixe 1, 2, 3, 3 — zusätzlich zu den drei Grundwerten', () => {
    expect(QUALITY_AFFIX_SLOTS[QUALITY_COMMON]).toBe(1);
    expect(QUALITY_AFFIX_SLOTS[QUALITY_RARE]).toBe(2);
    expect(QUALITY_AFFIX_SLOTS[QUALITY_EPIC]).toBe(3);
    // „3+" gelesen als: drei Affixe UND ein Effekt — nicht als vierter Affix.
    expect(QUALITY_AFFIX_SLOTS[QUALITY_LEGENDARY]).toBe(3);
    expect(QUALITY_HAS_EFFECT[QUALITY_LEGENDARY]).toBe(true);
    expect(QUALITY_HAS_EFFECT[QUALITY_EPIC]).toBe(false);
    expect(QUALITY_NAMES.length).toBe(QUALITY_COUNT);
  });

  it('gibt einer höheren Qualität auf gleicher Stufe mehr Budget', () => {
    const falschHerum: string[] = [];
    for (let level = 1; level <= MAX_ITEM_LEVEL; level++) {
      if (budgetOf(level, QUALITY_RARE) <= budgetOf(level, QUALITY_COMMON)) {
        falschHerum.push(`Stufe ${level}: Rare <= Common`);
      }
      if (budgetOf(level, QUALITY_EPIC) <= budgetOf(level, QUALITY_RARE)) {
        falschHerum.push(`Stufe ${level}: Epic <= Rare`);
      }
    }
    expect(falschHerum).toEqual([]);
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
