// @vitest-environment jsdom

/**
 * Die Einträge, die nicht vor einer Instanz und nicht nach einem Sieg kommen
 * (Issue #1177, Abschnitt 5 „Archiv").
 *
 * Masterplan: „… dazu Einträge für Zauber, Gegenstands-Sätze und versteckte
 * Funde." Drei Auslöser, zwei Wege:
 *
 * ```text
 *   spellLearned / schoolLearned / setFound   STILL ins Archiv, ohne Fenster
 *   firstOutcome                              als Fenster auf der Auswertung,
 *                                             wie der erste Sieg
 * ```
 *
 * **Warum still:** 58 Zauber hiessen 58 Fenster. Der Plan begründet die
 * Erzählweise ausdrücklich damit, dass die Geschichte bei hunderten
 * Wiederholungen nie im Weg steht; das Archiv ist das Sammelziel.
 *
 * **Alte Spielstände bleiben gültig:** Alle Auslöser schreiben in dieselbe
 * Liste `unlockedEntryIds` aus Kennungen. Es kommt kein Feld hinzu.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';
import { recordRun, type RecordedState } from '../../src/analysis/run-recorder.js';
import { discoveredEntries, nextAutomaticEntry } from '../../src/analysis/archive.js';
import { PLAYABLE_ARCHIVE_ENTRIES } from '../../src/analysis/archive-data.js';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { OUTCOME_DEATH, OUTCOME_OVERPRESSURE, OUTCOME_VICTORY } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';

function stubbeCanvas(): void {
  const ctx: unknown = new Proxy(
    {},
    {
      get: (_ziel, name) => (name === 'canvas' ? undefined : () => ctx),
      set: () => true,
    },
  );
  HTMLCanvasElement.prototype.getContext = ((): CanvasRenderingContext2D =>
    ctx as CanvasRenderingContext2D) as unknown as typeof HTMLCanvasElement.prototype.getContext;
}

function baueApp(): App {
  const parent = document.createElement('div');
  document.body.appendChild(parent);
  return new App(parent);
}

beforeEach(() => {
  stubbeCanvas();
  localStorage.clear();
  document.body.innerHTML = '';
});

describe('Zauber und Schulen kommen still ins Archiv', () => {
  it('ein frischer Stand kennt die Start-Zauber und ihre Schule', () => {
    const app = baueApp();
    const frei = app.persistence.state.unlockedEntryIds;
    // Kindle und Smoulder sind die Start-Zauber (spellbook.ts), Quench und
    // Purge sind schulunabhängig und immer dabei.
    for (const id of ['spell-kindle', 'spell-smoulder', 'spell-quench', 'spell-purge']) {
      expect(frei, id).toContain(id);
    }
    expect(frei).toContain('process-calcination');
    // Und KEINE Schule, deren Zauber der Spieler nicht hat.
    expect(frei).not.toContain('process-fixation');
  });

  it('ohne Fenster — das Archiv zeigt sie, das Dokument sonst nicht', () => {
    baueApp();
    expect(document.querySelector('.archive-popup')).toBeNull();
    const archiv = document.querySelector('.archive');
    expect(archiv?.textContent).toContain('Kindle');
  });

  it('ein gelernter Zauber kommt dazu, sobald er in der Zusammenstellung steht', () => {
    const neu = discoveredEntries(
      PLAYABLE_ARCHIVE_ENTRIES,
      { unlockedEntryIds: [] },
      { spellIds: ['bind'], schools: ['FIXATION'], setIds: [] },
    ).map((e) => e.id);
    expect(neu).toContain('spell-bind');
    expect(neu).toContain('process-fixation');
    // GEGENPROBE: nichts, was nicht gelernt ist.
    expect(neu).not.toContain('spell-kindle');
  });

  it('was schon im Archiv steht, kommt nicht noch einmal', () => {
    const neu = discoveredEntries(
      PLAYABLE_ARCHIVE_ENTRIES,
      { unlockedEntryIds: ['spell-bind'] },
      { spellIds: ['bind'], schools: [], setIds: [] },
    );
    expect(neu).toEqual([]);
  });

  it('ein Satz kommt mit seinem ersten Stück', () => {
    const neu = discoveredEntries(
      PLAYABLE_ARCHIVE_ENTRIES,
      { unlockedEntryIds: [] },
      { spellIds: [], schools: [], setIds: [3] },
    ).map((e) => e.id);
    expect(neu).toEqual(['set-3']);
  });
});

describe('Versteckte Funde: der erste Tod und der erste Überdruck', () => {
  const LEER: RecordedState = {
    runHistory: [],
    progressMarks: [],
    bestRuns: [],
    talentPoints: {},
    items: [],
    defeatedBosses: [],
    unlockedEntryIds: [],
    seenMechanicIds: [],
    level: 1,
    experience: 0,
  };
  const KONTEXT = {
    bossId: 'boss-1-1',
    difficulty: 1,
    bossLevel: 1,
    gameVersion: '0.1.0',
    at: 1_800_000_000_000,
    seed: 0,
    instance: 1,
    endlessLevel: 0,
  };
  function kurzfassung() {
    const boss = bossById('boss-1-1') as BossDef;
    const combat = new Combat({ school: CALCINATION.value, boss });
    const session = new Session(new PressQueue(), combat);
    session.startFight(0);
    return session.summary(1000);
  }

  it('der erste Tod schaltet seinen Fund frei und zeigt ihn', () => {
    const ergebnis = recordRun(LEER, { ...kurzfassung(), victory: false }, OUTCOME_DEATH, KONTEXT);
    expect(ergebnis.unlockedEntry?.id).toBe('find-first-death');
    expect(ergebnis.unlockedEntryIds).toContain('find-first-death');
  });

  it('der erste Überdruck seinen eigenen', () => {
    const ergebnis = recordRun(
      LEER,
      { ...kurzfassung(), victory: false },
      OUTCOME_OVERPRESSURE,
      KONTEXT,
    );
    expect(ergebnis.unlockedEntry?.id).toBe('find-first-overpressure');
  });

  it('der zweite Tod nichts mehr', () => {
    const erster = recordRun(LEER, { ...kurzfassung(), victory: false }, OUTCOME_DEATH, KONTEXT);
    const zweiter = recordRun(
      { ...LEER, unlockedEntryIds: erster.unlockedEntryIds },
      { ...kurzfassung(), victory: false },
      OUTCOME_DEATH,
      KONTEXT,
    );
    expect(zweiter.unlockedEntry).toBeNull();
  });

  it('ein Sieg gibt den Eintrag des Bosses, keinen Fund', () => {
    const ergebnis = recordRun(LEER, { ...kurzfassung(), victory: true }, OUTCOME_VICTORY, KONTEXT);
    expect(ergebnis.unlockedEntry?.id).toBe('boss-1-1-first-kill');
  });

  it('nextAutomaticEntry unterscheidet die beiden Ausgänge', () => {
    const tod = nextAutomaticEntry(
      PLAYABLE_ARCHIVE_ENTRIES,
      { unlockedEntryIds: [] },
      { trigger: 'firstOutcome', outcome: 'death' },
    );
    const druck = nextAutomaticEntry(
      PLAYABLE_ARCHIVE_ENTRIES,
      { unlockedEntryIds: [] },
      { trigger: 'firstOutcome', outcome: 'overpressure' },
    );
    expect(tod?.id).toBe('find-first-death');
    expect(druck?.id).toBe('find-first-overpressure');
  });
});

describe('Das Archiv gliedert nach Instanz und Kapitel, mit Namen', () => {
  it('die Überschriften tragen den Namen der Instanz, nicht die Kennung', () => {
    const app = baueApp();
    app.persistence.persistUnlockedEntries([
      ...app.persistence.state.unlockedEntryIds,
      'instance-1-before',
    ]);
    app.openHome();
    const ueberschriften = [...document.querySelectorAll('.archive-instance')].map(
      (h) => h.textContent,
    );
    expect(ueberschriften).toContain('The Cold Athanor');
    expect(ueberschriften).not.toContain('instance-1');
    expect(ueberschriften).toContain('Spells');
  });
});
