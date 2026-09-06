// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from '../../src/core/app.js';
import { FIRST_CRUCIBLE } from '../../src/core/data.js';
import { combatChecksum } from '../../src/core/checksum.js';
import { PHASE_ENDED } from '../../src/core/phases.js';
import { rollLoot } from '../../src/loot/roll.js';
import { lootStreamFor } from '../../src/loot/stream.js';
import { DIFFICULTY_NORMAL } from '../../src/loot/rates.js';
import { RunPersistence } from '../../src/analysis/run-persistence.js';
import { markKey } from '../../src/analysis/progress-mark.js';

beforeEach(() => {
  const context: unknown = new Proxy({}, { get: () => () => context, set: () => true });
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(
    context as CanvasRenderingContext2D,
  );
  let nextSeed = 0;
  vi.spyOn(globalThis.crypto, 'getRandomValues').mockImplementation((array) => {
    if (array instanceof Uint32Array) array[0] = nextSeed++;
    return array;
  });
  localStorage.clear();
  document.body.innerHTML = '';
});

afterEach(() => vi.restoreAllMocks());

function makeApp(): App {
  const root = document.createElement('div');
  document.body.appendChild(root);
  const app = new App(root);
  // Ein echter Sieg durch den Produktions-Callback, ohne 40 Sekunden pro Fall.
  app.combat.switchBoss({ ...FIRST_CRUCIBLE.value, hp: 1 });
  return app;
}

function win(app: App): void {
  expect(app.ready()).toBe(true);
  expect(app.ready(), 'ein zweites Ready darf den Fund nicht neu würfeln').toBe(false);
  app.session.startFight(0);
  app.pressQueue.push(0, 0, 0, 0);
  for (let t = 0; t <= 1000 && app.session.phase !== PHASE_ENDED; t += 10) {
    app.session.tick(t);
  }
  expect(app.lastRecord?.counted).toBe(true);
  expect(app.lastSummary?.victory).toBe(true);
}

describe('Beute im wirklichen App-Kampfende', () => {
  it('Normal verwendet Stufe 0 für Fund, Bestwert und Freischaltung', () => {
    const app = makeApp();
    win(app);
    expect(app.lastRecord?.drop?.itemLevel).toBe(1);
    expect(app.lastRecord?.bestRuns[0]?.difficulty).toBe(DIFFICULTY_NORMAL);
    expect(app.lastRecord?.runHistory[0]?.difficulty).toBe(DIFFICULTY_NORMAL);
    expect(app.lastRecord?.defeatedBosses).toContain(markKey('first-crucible', DIFFICULTY_NORMAL));
    expect(app.comparisonForResult()?.some((metric) => metric.best !== null)).toBe(true);
  });

  it('jeder begonnene Versuch erhält einen neuen nachrechenbaren Beutewurf', () => {
    const app = makeApp();
    const seeds = new Set<number>();
    const items = new Set<string>();
    for (let i = 0; i < 12; i++) {
      if (i > 0) {
        const choice = app.persistence.spellbook.pendingSpellIds[0];
        if (choice !== undefined) expect(app.chooseDraftSpell(choice)).toBe(true);
        app.backToMenu();
      }
      win(app);
      const seed = app.lastRecord?.runHistory.at(-1)?.lootSeed;
      expect(seed).toBeDefined();
      seeds.add(seed as number);
      items.add(app.lastRecord?.drop?.itemId ?? '');
      expect(app.lastRecord?.drop).toEqual(
        rollLoot(lootStreamFor(seed as number), {
          instance: 1,
          difficulty: DIFFICULTY_NORMAL,
          endlessLevel: 0,
          bossId: 'first-crucible',
        }),
      );
    }
    expect(seeds.size).toBe(12);
    expect(items.size).toBeGreaterThan(1);
    const reloaded = new RunPersistence('test', () => 1);
    expect(reloaded.state.runHistory.at(-1)?.lootSeed).toBe([...seeds].at(-1));
  });

  it('verschiedene Beutestartwerte verändern den Kampf nicht', () => {
    const first = makeApp();
    win(first);
    localStorage.clear();
    const second = makeApp();
    win(second);
    expect(first.lastRecord?.runHistory[0]?.lootSeed).not.toBe(
      second.lastRecord?.runHistory[0]?.lootSeed,
    );
    expect(first.lastSummary).toEqual(second.lastSummary);
    expect(combatChecksum(first.combat, 1000)).toBe(combatChecksum(second.combat, 1000));
  });
});

describe('Draft im Spielerablauf', () => {
  it.each(['training', 'testing'])(
    'belohnt Ziele aus %s weder mit Beute noch mit Zaubern oder Erfahrung',
    (instance) => {
      const app = makeApp();
      app.combat.switchBoss({ ...FIRST_CRUCIBLE.value, id: 'practice', instance, hp: 1 });
      win(app);
      expect(app.lastRecord?.drop).toBeNull();
      expect(app.lastRecord?.xpGained).toBe(0);
      expect(app.lastRecord?.pointsGained).toBe(0);
      expect(app.persistence.state.items).toEqual([]);
      expect(app.persistence.state.defeatedBosses).toEqual([]);
      expect(app.persistence.spellbook.pendingSpellIds).toEqual([]);
    },
  );

  it('erlaubt nur erworbene Zauber und hält die gemeinsamen Werkzeuge offen', () => {
    const app = makeApp();
    expect(app.bindSpellToSlot('crust', 0)).toBe(false);
    expect(app.bindSpellToSlot('quench', 0)).toBe(true);
    expect(app.persistence.save.bindings['0']).toBe('quench');
    expect(app.persistence.spellbook.pendingSpellIds).toEqual([]);
  });

  it('zeigt nach Sieg und Neustart dasselbe Angebot und verbucht die echte Button-Auswahl', () => {
    const first = makeApp();
    first.persistence.persistBindings({ '0': 'kindle', '1': 'smoulder' });
    win(first);
    const pending = [...first.persistence.spellbook.pendingSpellIds];
    expect(pending).toHaveLength(3);
    expect(document.querySelector('.rota-spell-draft')?.textContent).toContain('Choose a spell');
    const reloaded = makeApp();
    expect(reloaded.home.isVisible).toBe(true);
    expect(reloaded.persistence.spellbook.pendingSpellIds).toEqual(pending);
    expect(reloaded.ready()).toBe(false);
    expect(reloaded.home.root.textContent).toContain('Your offer is saved until you choose');
    expect(reloaded.bindSpellToSlot(pending[0]!, 0)).toBe(false);
    const buttons = [
      ...reloaded.home.root.querySelectorAll<HTMLButtonElement>('.spellbook-choice'),
    ];
    expect(buttons.map((button) => button.dataset.spellId)).toEqual(pending);
    buttons[0]!.click();
    expect(reloaded.persistence.spellbook.pendingSpellIds).toEqual([]);
    expect(reloaded.persistence.spellbook.knownSpellIds).toContain(pending[0]);
    expect(reloaded.persistence.spellbook.knownSpellIds).not.toContain(pending[1]);
    expect(reloaded.chooseDraftSpell(pending[1]!)).toBe(false);
    expect(reloaded.persistence.save.bindings).toEqual({ '0': 'kindle', '1': 'smoulder' });
    expect(reloaded.bindSpellToSlot(pending[0]!, 0)).toBe(true);
    const afterChoice = makeApp();
    expect(afterChoice.persistence.spellbook.pendingSpellIds).toEqual([]);
    expect(afterChoice.persistence.save.bindings['0']).toBe(pending[0]);
    expect(afterChoice.ready()).toBe(true);
  });
});
