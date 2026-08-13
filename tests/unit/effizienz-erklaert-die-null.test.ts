// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';
import { buildEfficiencyTable, type SpellEfficiencyInput } from '../../src/analysis/efficiency.js';
import { buildEfficiencyView } from '../../src/analysis/analysis-view.js';

/**
 * Die Effizienz-Tabelle erklärt, warum eine Null eine Null ist (Issue #806).
 *
 * # Der Fall
 *
 * SMOULDER hat 1500 ms Abklingzeit, 2000 ms bis zum ersten Tick,
 * `refreshMode: reset` und keinen Grundschaden. Wer ihn drückt, sobald die
 * Taste bereit ist, wirft seinen eigenen Brand 500 ms vor dem ersten Tick weg.
 *
 * Gemessen im Kampf über 30 Sekunden, nur die Bedingung unterschiedlich:
 *
 * ```text
 *   auf Abklingzeit gedrückt   21 Wirkungen · 0 Ticks · 100 verloren · 0 Schaden
 *   erst beim Auslaufen         3 Wirkungen · 15 Ticks · 0 verloren · 2400 Schaden
 * ```
 *
 * Er ist `unlockLevel: 1` — der zweite Zauber, den jeder Spieler hat. Die
 * Tabelle zeigte bisher „0 %" und liess ihn im Dunkeln.
 *
 * # Was hier NICHT geprüft wird
 *
 * Ob die Zahlen des Zaubers gut gewählt sind. Der Auftraggeber hat entschieden,
 * nichts an ihnen zu drehen — die Präzision aus GAME-DESIGN 7.2 bleibt gewollt.
 * Geprüft wird, dass der Grund **sichtbar** wird.
 */

function zeile(overrides: Partial<SpellEfficiencyInput> = {}): SpellEfficiencyInput {
  return {
    spellId: 'smoulder',
    casts: 21,
    idleMs: 0,
    damage: 0,
    triggersGcd: true,
    dotTicks: 0,
    lostDotTicks: 100,
    damageOnlyFromDot: true,
    ...overrides,
  };
}

function tabelleMit(spells: readonly SpellEfficiencyInput[]) {
  return buildEfficiencyTable({
    elapsedMs: 30_000,
    gcdMs: 1500,
    casts: spells.reduce((s, x) => s + x.casts, 0),
    damage: spells.reduce((s, x) => s + x.damage, 0),
    lostTimeNothingReadyMs: 0,
    synergyUsed: 0,
    synergyAvailable: 0,
    spells,
  });
}

const notizVon = (id: string, spells: readonly SpellEfficiencyInput[]) =>
  tabelleMit(spells).rows.find((r) => r.spellId === id)?.note ?? null;

describe('Der Fall aus #806 — gar kein Tick', () => {
  it('die Zeile nennt den Grund und die Zahl', () => {
    expect(notizVon('smoulder', [zeile()])).toBe('renewed before its first tick — 100 ticks lost');
  });

  it('richtig gespielt steht dort NICHTS', () => {
    // **Die Gegenprobe, und sie ist der Kern.** Ein Satz, der immer dasteht,
    // erklärt nichts — er wird Rauschen und übersehen. Die Zahlen sind die
    // gemessenen des guten Laufs.
    expect(
      notizVon('smoulder', [zeile({ casts: 3, dotTicks: 15, lostDotTicks: 0, damage: 2400 })]),
    ).toBeNull();
  });
});

describe('Die zwei milderen Fälle', () => {
  it('mehr verloren als bekommen', () => {
    expect(notizVon('smoulder', [zeile({ dotTicks: 4, lostDotTicks: 9, damage: 160 })])).toBe(
      'renewed too early — 9 ticks lost, 4 landed',
    );
  });

  it('etwas verloren, aber nicht überwiegend', () => {
    expect(notizVon('smoulder', [zeile({ dotTicks: 12, lostDotTicks: 2, damage: 480 })])).toBe(
      '2 ticks lost to early renewal',
    );
  });
});

describe('Wo der Satz NICHT hingehört', () => {
  it('ein Zauber mit Grundschaden bekommt ihn nicht', () => {
    // KINDLE trägt seinen Schaden selbst. Bei ihm wäre „kein Tick" keine
    // Erklärung, sondern eine Verwirrung.
    expect(
      notizVon('kindle', [zeile({ spellId: 'kindle', damageOnlyFromDot: false, damage: 1200 })]),
    ).toBeNull();
  });

  it('und ein nie gewirkter Zauber auch nicht', () => {
    // Ohne eine einzige Wirkung ist die Null kein Rätsel: Er lag nur da.
    expect(notizVon('smoulder', [zeile({ casts: 0, lostDotTicks: 0 })])).toBeNull();
  });
});

describe('Der Satz kommt in der Ansicht an', () => {
  it('unter seiner Zeile, über die ganze Breite', () => {
    // **Ohne diesen Fall wäre der Satz gerechnet und unsichtbar** — genau die
    // Klasse, an der `buildEfficiencyView` selbst schon einmal hing: 824 Zeilen
    // fertig, und im Spiel lief nichts davon.
    const tabelle = tabelleMit([zeile()]);
    const knoten = buildEfficiencyView(tabelle);

    const hinweis = knoten.querySelector('[data-note-for="smoulder"] .efficiency-note');
    expect(hinweis, 'der Satz steht nicht in der Ansicht').not.toBeNull();
    expect(hinweis?.textContent).toBe('renewed before its first tick — 100 ticks lost');
    // Über alle Spalten plus die Kennung — sonst schöbe er die Tabelle schief.
    expect((hinweis as HTMLTableCellElement).colSpan).toBe(tabelle.columns.length + 1);
  });

  it('und bei einer sauberen Zeile steht keine Fussnote', () => {
    const knoten = buildEfficiencyView(
      tabelleMit([zeile({ casts: 3, dotTicks: 15, lostDotTicks: 0, damage: 2400 })]),
    );
    expect(knoten.querySelectorAll('.efficiency-note-row').length).toBe(0);
  });
});
