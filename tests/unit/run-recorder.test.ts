/**
 * Was am Kampfende festgehalten wird (Issue #179).
 *
 * Es wird WIRKLICH gespielt: Sitzung, Countdown, Tastendruecke, Kampfende. Ein
 * Prueffall mit von Hand gesetzten Zahlen wuerde die Rechnung aus #164 ein
 * zweites Mal messen -- was hier fehlte, war die VERBINDUNG zum Spiel.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../../src/core/data.js';
import { COUNTDOWN_MS, OUTCOME_VICTORY, OUTCOME_WIPE, PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { markFor } from '../../src/analysis/progress-mark.js';
import {
  countsAsAttempt,
  recordRun,
  COUNTED_OUTCOMES,
  IGNORED_OUTCOMES,
  type RecordedState,
} from '../../src/analysis/run-recorder.js';

/**
 * Zauber-Indizes, **aus den Daten abgeleitet statt hingeschrieben** (Issue #369).
 *
 * Eine feste Zahl hier zeigt auf den Zauber, der heute an dieser Stelle steht.
 * Verschiebt sich die Liste, prüft der Fall still einen anderen — er wird nicht
 * rot, er misst etwas anderes.
 */
function indexVon(id: string): number {
  const i = CALCINATION.value.spells.findIndex((spell) => spell.id === id);
  if (i < 0) throw new Error(`Zauber ${id} steht nicht in der Schule`);
  return i;
}

const KINDLE = indexVon('kindle');
const SMOULDER = indexVon('smoulder');
const LEER: RecordedState = { runHistory: [], progressMarks: [], level: 1, experience: 0 };

const KONTEXT = {
  bossId: 'first-crucible',
  difficulty: 1,
  bossLevel: 1,
  gameVersion: '0.1.0',
  at: 1_800_000_000_000,
};

interface Lauf {
  readonly summary: ReturnType<Session['summary']>;
  readonly outcome: number;
}

/**
 * Einen Durchlauf spielen.
 *
 * `everyMs` steuert, wie gut gespielt wird -- der einzige Hebel, mit dem sich
 * ein besserer und ein schlechterer Versuch unterscheiden.
 */
function play(
  everyMs: number,
  fightMs = 60_000,
  boss: BossDef = FIRST_CRUCIBLE.value,
  loadout: readonly number[] = [KINDLE, SMOULDER],
): Lauf {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  loadout.forEach((spell, slot) => {
    session.bind(slot, spell);
  });
  session.beginCountdown(0);

  // Bis zum ECHTEN Ende laufen lassen, nicht bis zu einer gewaehlten Dauer:
  // Ohne Sieg oder Ueberdruck bleibt der Ausgang 'none', und dann zaehlt der
  // Durchlauf gar nicht -- der Prueffall haette nichts gemessen.
  const ende = COUNTDOWN_MS + fightMs;
  let tick = 0;
  let presses = 0;
  let letzter = 0;
  for (let t = 0; t <= ende; t += 10) {
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

describe('AC6 — die Umrechnung laeuft ueber attemptFrom', () => {
  it('schreibt die VERBLIEBENE Gesundheit, nicht den Fortschritt', () => {
    // Der Nachweis, dass an dieser Aufrufstelle nicht verwechselt wurde. Die
    // beiden Zahlen ergaenzen sich zu 100 -- wer sie vertauscht, bekommt eine
    // Zahl, bei der niemand stutzt.
    const lauf = play(1500);
    const ergebnis = recordRun(LEER, lauf.summary, OUTCOME_VICTORY, KONTEXT);

    const mark = markFor(ergebnis.progressMarks, 'first-crucible', 1);
    expect(mark?.bestRemainingPercent).toBeCloseTo(100 - lauf.summary.progressPercent, 6);
    // Und nicht der Fortschritt selbst -- sonst waere die Pruefung blind.
    expect(lauf.summary.progressPercent).toBeGreaterThan(0);
    expect(mark?.bestRemainingPercent).not.toBeCloseTo(lauf.summary.progressPercent, 6);
  });

  it('legt dieselbe Zahl in Marke UND Verlaufs-Eintrag', () => {
    // Zwei Wege zu ihr waeren zwei Gelegenheiten, sie zu verdrehen.
    const lauf = play(1500);
    const ergebnis = recordRun(LEER, lauf.summary, OUTCOME_VICTORY, KONTEXT);
    expect(ergebnis.runHistory[0]?.remainingPercent).toBe(
      markFor(ergebnis.progressMarks, 'first-crucible', 1)?.bestRemainingPercent,
    );
  });
});

describe('AC1 — ein besserer Versuch verbessert die Marke', () => {
  it('meldet den Fortschritt beim zweiten, besseren Durchlauf', () => {
    const schwach = play(3000);
    const stark = play(1500);
    expect(stark.summary.progressPercent).toBeGreaterThan(schwach.summary.progressPercent);

    let stand: RecordedState = LEER;
    const erster = recordRun(stand, schwach.summary, schwach.outcome, KONTEXT);
    stand = erster;
    const zweiter = recordRun(stand, stark.summary, stark.outcome, {
      ...KONTEXT,
      at: KONTEXT.at + 1,
    });

    expect(erster.mark?.improved).toBe(false); // erster Versuch: nichts zu unterbieten
    expect(zweiter.mark?.improved).toBe(true);
    expect(zweiter.mark?.gainedPercent).toBeGreaterThan(0);
  });
});

describe('AC2 — ein schlechterer Versuch laesst die Marke stehen', () => {
  it('zaehlt trotzdem als Versuch mit', () => {
    // Der Gegenfall. Ohne ihn bestuende auch eine Marke, die sich bei JEDEM
    // Versuch fortschreibt -- und die belohnte das Gegenteil dessen, wofuer sie
    // da ist.
    const stark = play(1500);
    const schwach = play(3000);

    const stand: RecordedState = recordRun(LEER, stark.summary, stark.outcome, KONTEXT);
    const vorher = markFor(stand.progressMarks, 'first-crucible', 1)?.bestRemainingPercent;

    const danach = recordRun(stand, schwach.summary, schwach.outcome, {
      ...KONTEXT,
      at: KONTEXT.at + 1,
    });
    const mark = markFor(danach.progressMarks, 'first-crucible', 1);

    expect(danach.mark?.improved).toBe(false);
    expect(mark?.bestRemainingPercent).toBe(vorher);
    expect(mark?.attempts).toBe(2);
    // Und im Verlauf steht er sehr wohl -- er hat stattgefunden.
    expect(danach.runHistory).toHaveLength(2);
  });
});

describe('AC3 — ein Sieg setzt die Marke auf null', () => {
  it('faellt auf 0 und merkt sich den Sieg', () => {
    // Ein Sieg muss erst einmal moeglich sein. Gemessen: KINDLE und SMOULDER
    // abwechselnd bringen in 60 s nur 2760 Schaden -- test-warden hat 3000 und
    // ueberlebt den Ueberdruck. KINDLE ALLEIN trifft jedes Fenster und faellt
    // ihn nach 43,5 s.
    const boss = bossById('test-warden') as BossDef;
    const lauf = play(1500, 120_000, boss, [KINDLE]);
    expect(lauf.summary.victory).toBe(true);
    expect(lauf.outcome).toBe(OUTCOME_VICTORY);

    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    const mark = markFor(ergebnis.progressMarks, 'first-crucible', 1);
    expect(mark?.bestRemainingPercent).toBe(0);
    expect(mark?.defeated).toBe(true);
  });
});

describe('Der Abbruch zaehlt nicht', () => {
  it('schreibt weder Marke noch Verlaufs-Eintrag', () => {
    const lauf = play(1500);
    const ergebnis = recordRun(LEER, lauf.summary, OUTCOME_WIPE, KONTEXT);

    expect(ergebnis.counted).toBe(false);
    expect(ergebnis.mark).toBeNull();
    expect(ergebnis.progressMarks).toEqual([]);
    expect(ergebnis.runHistory).toEqual([]);
  });

  it('steht als Liste da, nicht als Bedingung im Rumpf', () => {
    // Die Frage, ob ein Abbruch ein Versuch ist, gehoert dem Design. Als Liste
    // ist die Antwort ein Eintrag, als Bedingung waere sie ein Umbau.
    expect(countsAsAttempt(OUTCOME_VICTORY)).toBe(true);
    expect(countsAsAttempt(OUTCOME_WIPE)).toBe(false);
    expect(COUNTED_OUTCOMES).toHaveLength(3);
    expect(IGNORED_OUTCOMES).toEqual([OUTCOME_WIPE]);
  });
});

describe('Der uebergebene Stand bleibt unangetastet', () => {
  it('gibt einen neuen zurueck', () => {
    // Ein halb geschriebener Spielstand darf keine Marke verfaelschen.
    const lauf = play(1500);
    const stand: RecordedState = { runHistory: [], progressMarks: [], level: 1, experience: 0 };
    const kopie = JSON.stringify(stand);
    recordRun(stand, lauf.summary, lauf.outcome, KONTEXT);
    expect(JSON.stringify(stand)).toBe(kopie);
  });
});
