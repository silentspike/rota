/**
 * Ein Sieg bringt Fortschritt (Issue #248, AC1 im laufenden Spiel).
 *
 * WAS DIESE PRÜFFÄLLE VON DEN ANDEREN UNTERSCHEIDET
 * -------------------------------------------------
 * `level-xp.test.ts` prüft die Rechnung: Wie viel Erfahrung ein Sieg bringt und
 * wann sie zur nächsten Stufe reicht. Hier geht es darum, ob sie überhaupt
 * jemand aufruft — und ob der Spielstand hinterher trägt, was passiert ist.
 *
 * Der Unterschied ist genau das Muster, das in diesem Projekt viermal
 * aufgefallen ist: ein fertiger Mechanismus, den kein Aufrufer benutzt. Bis zu
 * diesem Commit war `grantXp` einer davon.
 *
 * NUR DER SIEG ZÄHLT
 * ------------------
 * Ein gescheiterter Versuch geht in Verlauf und Fehleranalyse ein, bringt aber
 * keinen Fortschritt — sonst wäre Verlieren eine Strategie. Der Prüffall dazu
 * steht unten und ist wichtiger, als er aussieht: Er ist die Bedingung dafür,
 * dass die 1323 Kämpfe aus der Kurvenrechnung überhaupt stimmen.
 */
import { describe, expect, it } from 'vitest';
import { recordRun, type RecordedState } from '../../src/analysis/run-recorder.js';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { COUNTDOWN_MS, PHASE_ENDED } from '../../src/core/phases.js';
import { PressQueue } from '../../src/input/queue.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';

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

/**
 * Einen echten Kampf spielen und seine Zusammenfassung nehmen.
 *
 * **Nicht eine `RunSummary` von Hand bauen.** Mein erster Entwurf tat das mit
 * einem `as RunSummary` — und `tsc` wies ihn zurück, weil zehn Felder fehlten.
 * Ein Cast, der die Prüfung wegdrückt, wäre die schlechtere Antwort gewesen:
 * Er hätte den Prüffall gegen eine Struktur laufen lassen, die es im Spiel gar
 * nicht gibt.
 *
 * `everyMs` steuert den Ausgang: dicht gedrückt gewinnt der Lauf, gar nicht
 * gedrückt läuft die Zeit ab.
 *
 * **Gegen den Türhüter, nicht gegen den Ersten Schmelztiegel.** Gemessen: Eine
 * Rundlauf-Belegung bringt gegen 4200 Lebenspunkte höchstens 4000 Schaden (bei
 * 1500, 1000, 750 und 500 ms Abstand) — der Sieg-Lauf gewann nie, und drei
 * Prüffälle waren rot, ohne dass an der Sache etwas falsch war.
 *
 * **DAS IST KEINE SCHWÄCHE DES SCHMELZTIEGELS, SONDERN SEIN ZWECK.** Er ist
 * der einzige ausgelieferte Boss und so ausgelegt, dass eine schlechte
 * Rotation ihn nicht schafft: Das Abnahme-Kriterium aus GAME-DESIGN 20.3
 * verlangt mindestens 10 % Unterschied zwischen guter und schlechter Rotation,
 * gemessen wurden +48,2 %.
 *
 * Für Prüffälle heißt das: **Er taugt nicht als Kulisse für etwas anderes.**
 * Wer einen Sieg braucht, um daran etwas zu messen, das mit der Rotation
 * nichts zu tun hat, nimmt den Türhüter (1200) oder legt einen eigenen
 * Datensatz an.
 */
function spielen(everyMs: number): { summary: RunSummary; outcome: number } {
  const boss = bossById('test-gatekeeper') ?? FIRST_CRUCIBLE.value;
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  const belegung = [combat.indexOf('kindle'), combat.indexOf('smoulder')];
  belegung.forEach((spell, slot) => {
    session.bind(slot, spell);
  });
  session.beginCountdown(0);
  let tick = 0;
  let presses = 0;
  let letzter = 0;
  for (let t = 0; t <= COUNTDOWN_MS + 120_000; t += 10) {
    if (everyMs > 0 && t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      queue.push(presses % belegung.length, tick, t, tick);
      presses++;
    }
    session.tick(t);
    letzter = t;
    tick++;
    if (session.phase === PHASE_ENDED) break;
  }
  return { summary: session.summary(letzter), outcome: session.outcome };
}

const SIEG = spielen(1500);
const NIEDERLAGE = spielen(0);

function kontext(bossLevel: number) {
  return {
    bossId: 'first-crucible',
    difficulty: 1,
    bossLevel,
    gameVersion: '0.1.0',
    at: 1_700_000_000_000,
    seed: 0,
    instance: 1,
    endlessLevel: 0,
  };
}

describe('Ein Sieg gibt Erfahrung — und jemand ruft die Rechnung wirklich auf', () => {
  it('schreibt Erfahrung gut, die zur Boss-Stufe passt', () => {
    const nachher = recordRun(LEER, SIEG.summary, SIEG.outcome, kontext(1));
    expect(nachher.xpGained, 'Stufe 1 mal 100').toBe(100);
    expect(nachher.experience).toBe(100);
    expect(nachher.level).toBe(1);
  });

  it('gibt für einen höheren Boss mehr — das ist der Sinn der Boss-Stufe', () => {
    const klein = recordRun(LEER, SIEG.summary, SIEG.outcome, kontext(1));
    const gross = recordRun(LEER, SIEG.summary, SIEG.outcome, kontext(10));
    expect(gross.xpGained).toBeGreaterThan(klein.xpGained);
    expect(gross.xpGained).toBe(1000);
  });

  it('hebt die Stufe, wenn es reicht, und meldet das dem Aufrufer', () => {
    // Stufe 1 verlangt 200. Ein Sieg über einen Boss der Stufe 10 gibt 1000 —
    // das reicht für mehrere Stufen auf einmal.
    const nachher = recordRun(LEER, SIEG.summary, SIEG.outcome, kontext(10));
    expect(nachher.leveledUp, 'der Aufrufer braucht das für die Anzeige').toBe(true);
    expect(nachher.level).toBeGreaterThan(1);
  });

  it('meldet KEINEN Aufstieg, wenn es nicht gereicht hat', () => {
    // Sonst zeigte das Spiel eine Aufstiegs-Meldung für einen Sieg, der keine
    // Stufe gebracht hat — und der Spieler suchte, was sich geändert hat.
    const nachher = recordRun(LEER, SIEG.summary, SIEG.outcome, kontext(1));
    expect(nachher.leveledUp).toBe(false);
    expect(nachher.level).toBe(1);
  });

  it('gibt für eine NIEDERLAGE nichts — sonst wäre Verlieren eine Strategie', () => {
    // **Die Bedingung dafür, dass die Kurvenrechnung stimmt.** Zählten
    // gescheiterte Versuche mit, wären die 1323 Kämpfe für Stufe 1 bis 100
    // eine Zahl ohne Bezug.
    const nachher = recordRun(LEER, NIEDERLAGE.summary, NIEDERLAGE.outcome, kontext(10));
    expect(nachher.xpGained).toBe(0);
    expect(nachher.level).toBe(1);
    expect(nachher.experience).toBe(0);
    expect(nachher.counted, 'der Versuch zählt trotzdem für Verlauf und Marke').toBe(true);
  });

  it('lässt den übergebenen Stand unberührt', () => {
    // Derselbe Grundsatz wie in `grantXp`: Der Aufrufer entscheidet, wann er
    // ablegt — ein halb geschriebener Spielstand kann sonst eine Marke
    // verfälschen.
    recordRun(LEER, SIEG.summary, SIEG.outcome, kontext(10));
    expect(LEER.level).toBe(1);
    expect(LEER.experience).toBe(0);
  });

  it('trägt die Erfahrung über mehrere Siege fort', () => {
    // Ein einzelner Aufruf könnte auch zufällig stimmen. Erst die Folge zeigt,
    // dass der Stand wirklich weitergereicht wird.
    let stand: RecordedState = LEER;
    for (let i = 0; i < 3; i++) {
      const ergebnis = recordRun(stand, SIEG.summary, SIEG.outcome, kontext(1));
      stand = {
        runHistory: ergebnis.runHistory,
        progressMarks: ergebnis.progressMarks,
        bestRuns: [],
        talentPoints: {},
        items: [],
        defeatedBosses: [],
        unlockedEntryIds: [],
        seenMechanicIds: [],
        level: ergebnis.level,
        experience: ergebnis.experience,
      };
    }
    // 3 x 100 = 300; Stufe 1 kostet 200, also Stufe 2 mit 100 übrig.
    expect(stand.level).toBe(2);
    expect(stand.experience).toBe(100);
  });
});
