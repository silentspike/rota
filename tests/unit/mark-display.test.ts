/**
 * Die Fortschrittsmarke, fertig zum Anzeigen (Issue #187).
 *
 * Der Kern ist nicht, DASS ein Satz herauskommt, sondern WANN keiner
 * herauskommt. Eine Anzeige, die beim ersten Kampf „0 %" zeigt, behauptet
 * einen Fortschritt, den es nicht gibt -- das ist schlechter als gar keine.
 */
import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../../src/core/data.js';
import { spellIndex } from '../helpers/spell-index.js';
import { COUNTDOWN_MS, OUTCOME_WIPE, PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';
import { PressQueue } from '../../src/input/queue.js';
import { markDisplayFor } from '../../src/analysis/mark-display.js';
import { recordRun, type RecordedState } from '../../src/analysis/run-recorder.js';

const KINDLE = spellIndex('kindle');
const SMOULDER = spellIndex('smoulder');
const LEER: RecordedState = {
  runHistory: [],
  progressMarks: [],
  bestRuns: [],
  talentPoints: {},
  items: [],
  defeatedBosses: [],
  level: 1,
  experience: 0,
};
const KONTEXT = {
  bossId: 'first-crucible',
  difficulty: 1,
  bossLevel: 1,
  gameVersion: '0.1.0',
  at: 1_800_000_000_000,
};

function play(
  everyMs: number,
  boss: BossDef = FIRST_CRUCIBLE.value,
  loadout: readonly number[] = [KINDLE, SMOULDER],
): { summary: RunSummary; outcome: number } {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  loadout.forEach((spell, slot) => {
    session.bind(slot, alsSpellIndex(spell));
  });
  session.beginCountdown(0);
  let tick = 0;
  let presses = 0;
  let letzter = 0;
  for (let t = 0; t <= COUNTDOWN_MS + 120_000; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      queue.push(presses % loadout.length, tick, t, tick);
      presses++;
    }
    session.tick(t);
    letzter = t;
    tick++;
    if (session.phase === PHASE_ENDED) break;
  }
  return { summary: session.summary(letzter), outcome: session.outcome };
}

describe('AC1 und AC2 — was nach einer Niederlage dasteht', () => {
  it('nennt beim ersten Versuch die Marke, ohne Fortschritt zu behaupten', () => {
    const lauf = play(3000);
    const anzeige = markDisplayFor(
      recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT),
      lauf.summary,
      'first-crucible',
    );
    expect(anzeige?.sentence).toMatch(/^Best attempt: down to \d+ %\.$/);
    expect(anzeige?.sentence).not.toContain('further');
    expect(anzeige?.attempts).toBe(1);
  });

  it('meldet beim besseren Versuch, um wie viel es weiterging', () => {
    const schwach = play(3000);
    const stark = play(1500);
    const erster = recordRun(LEER, schwach.summary, schwach.outcome, KONTEXT);
    const zweiter = recordRun(erster, stark.summary, stark.outcome, {
      ...KONTEXT,
      at: KONTEXT.at + 1,
    });

    const anzeige = markDisplayFor(zweiter, stark.summary, 'first-crucible');
    expect(anzeige?.sentence).toMatch(
      /^New best attempt: down to \d+ % — \d+ points further than before\.$/,
    );
    expect(anzeige?.attempts).toBe(2);
  });

  it('zeigt beim SCHLECHTEREN Versuch die bestehende Marke, statt zu schweigen', () => {
    // Der Kern von AC2. Eine Anzeige, die nur bei Verbesserung erscheint,
    // laesst den Spieler genau in dem Moment im Unklaren, fuer den die Marke
    // gebaut wurde.
    const stark = play(1500);
    const schwach = play(3000);
    const erster = recordRun(LEER, stark.summary, stark.outcome, KONTEXT);
    const zweiter = recordRun(erster, schwach.summary, schwach.outcome, {
      ...KONTEXT,
      at: KONTEXT.at + 1,
    });

    const anzeige = markDisplayFor(zweiter, schwach.summary, 'first-crucible');
    expect(anzeige).not.toBeNull();
    expect(anzeige?.sentence).toMatch(/^Down to \d+ %\. Best attempt stands at \d+ %\.$/);
    // Kein Fortschritt behauptet.
    expect(anzeige?.sentence).not.toContain('New best');
    expect(anzeige?.attempts).toBe(2);
  });
});

describe('AC3 — nach einem Sieg steht das Ergebnis da', () => {
  it('zeigt keine Marke', () => {
    const boss = bossById('test-warden') as BossDef;
    const lauf = play(1500, boss, [KINDLE]);
    expect(lauf.summary.victory).toBe(true);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    expect(markDisplayFor(ergebnis, lauf.summary, 'first-crucible')).toBeNull();
  });
});

describe('AC4 — ohne Marke wird nichts gezeigt', () => {
  it('gibt null zurueck, wenn der Durchlauf gar nicht zaehlte', () => {
    // Ein Abbruch. Die Anzeige darf dann NICHTS zeigen -- nicht '0 %', nicht
    // einen leeren Satz.
    const lauf = play(1500);
    const ergebnis = recordRun(LEER, lauf.summary, OUTCOME_WIPE, KONTEXT);
    expect(ergebnis.counted).toBe(false);
    expect(markDisplayFor(ergebnis, lauf.summary, 'first-crucible')).toBeNull();
  });

  it('gibt null zurueck, wenn die Marke zu einem anderen Boss gehoert', () => {
    // Der erste Kampf auf einem NEUEN Boss: Es gibt Marken, aber keine fuer
    // diesen. Ohne diese Pruefung stuende dort die Marke eines fremden Bosses.
    const lauf = play(3000);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    expect(markDisplayFor(ergebnis, lauf.summary, 'ein-anderer-boss')).toBeNull();
  });

  it('gibt null zurueck, wenn die Marke zu einer anderen Stufe gehoert', () => {
    const lauf = play(3000);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    expect(markDisplayFor(ergebnis, lauf.summary, 'first-crucible', 3)).toBeNull();
  });
});

describe('Die Umrechnung laeuft auch hier ueber attemptFrom', () => {
  it('nennt die VERBLIEBENE Gesundheit, nicht den Fortschritt', () => {
    // Dieselbe Falle wie an jeder anderen Aufrufstelle. Der Satz enthaelt eine
    // Prozentzahl -- sie muss die verbliebene sein.
    //
    // NICHT alle 3000 ms: Dieser Durchlauf endet bei genau 50 % Fortschritt,
    // und dort ist die verbliebene Gesundheit ebenfalls 50. Der Prueffall
    // koennte die beiden dann gar nicht unterscheiden -- er waere gruen oder
    // rot aus dem falschen Grund. Bei 1500 ms liegen sie bei 66 gegen 34.
    const lauf = play(1500);
    expect(Math.round(lauf.summary.progressPercent)).not.toBe(50);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    const anzeige = markDisplayFor(ergebnis, lauf.summary, 'first-crucible');

    const genannt = Number(/(\d+) %/.exec(anzeige?.sentence ?? '')?.[1] ?? -1);
    expect(genannt).toBe(Math.round(100 - lauf.summary.progressPercent));
    expect(genannt).not.toBe(Math.round(lauf.summary.progressPercent));
  });
});
