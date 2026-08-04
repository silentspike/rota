/**
 * Prüfblatt für Issue #82, AC1 und AC4.
 *
 * Nach WORKER-GUIDE § 5 gebaut: Jeder Nachweis zeigt eine WIRKUNG und bringt
 * den Fall mit, in dem sich nichts ändern darf.
 *
 * AC4 verlangt „ohne spürbare Verzögerung". Das ist keine Aussage, die man
 * behaupten kann — die Zeit wird gemessen und steht als Zahl da.
 */
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';
import { PressQueue } from '../../src/input/queue.js';
import { compareRuns, sampleDamageCurve, type RunCurve } from '../../src/analysis/run-compare.js';
import { historySegments } from '../../src/analysis/run-history.js';
import type { RunHistoryEntry } from '../../src/analysis/save-format.js';
import {
  buildComparisonView,
  buildCurveView,
  buildHistoryView,
} from '../../src/analysis/history-view.js';
import '../../src/analysis/history.css';

const KINDLE = 0;
const SMOULDER = 1;

function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  document.querySelector('#out')?.appendChild(el);
}

interface Lauf {
  readonly summary: RunSummary;
  readonly curve: RunCurve;
}

/** Ein wirklich gespielter Durchlauf. */
function play(everyMs: number, fightMs: number): Lauf {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const session = new Session(queue, combat);
  session.bind(0, KINDLE);
  session.bind(1, SMOULDER);
  session.beginCountdown(0);

  const ende = COUNTDOWN_MS + fightMs;
  let tick = 0;
  let presses = 0;
  for (let t = 0; t <= ende; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      queue.push(presses % 2, tick, t, tick);
      presses++;
    }
    session.tick(t);
    tick++;
  }
  const summary = session.summary(ende);
  return { summary, curve: sampleDamageCurve(session.log, summary.elapsedMs) };
}

const bestlauf = play(1500, 30_000);
const dieserLauf = play(3000, 22_000);

// --- AC1: zwei Kurven und die Differenz je Kennzahl -------------------------

line('AC1 — zwei Kurven übereinander und die Differenz je Kennzahl');

const kennzahlen = compareRuns(dieserLauf.summary, bestlauf.summary);
const vergleich = buildComparisonView(kennzahlen);
const kurven = buildCurveView(dieserLauf.curve, bestlauf.curve);
document.querySelector('#view')?.appendChild(vergleich);
document.querySelector('#view')?.appendChild(kurven);

const zeilen = [...vergleich.querySelectorAll('tr[data-metric]')];
line(`Kennzahlen im Bild: ${zeilen.length}`, zeilen.length === kennzahlen.length);
line(
  `Jede Zeile trägt eine Differenz: ${zeilen.every((z) => (z.lastElementChild?.textContent ?? '') !== '')}`,
  zeilen.every((z) => (z.lastElementChild?.textContent ?? '').trim().length > 0),
);

const gezeichnet = kurven.querySelectorAll('polyline');
line(`Kurven gezeichnet: ${gezeichnet.length}`, gezeichnet.length === 2);

// Die absolute Zeitachse: Beide Kurven beziehen sich auf DIESELBE Länge, und
// das ist die des längeren Laufs. Waeren sie gestreckt, stuende hier die
// jeweils eigene.
const achse = Number(kurven.querySelector('svg')?.getAttribute('data-max-ms') ?? 0);
line(
  `Gemeinsame Zeitachse: ${achse} ms (dieser Lauf ${dieserLauf.summary.elapsedMs}, Bestlauf ${bestlauf.summary.elapsedMs})`,
  achse === Math.max(dieserLauf.summary.elapsedMs, bestlauf.summary.elapsedMs),
);

// Und die kuerzere Kurve endet WIRKLICH frueher: ihr letzter x-Wert liegt
// unter dem der laengeren.
function letztesX(node: Element | null): number {
  const punkte = (node?.getAttribute('points') ?? '').trim().split(/\s+/);
  return Number((punkte.at(-1) ?? '0,0').split(',')[0]);
}
const xKurz = letztesX(kurven.querySelector('.curve-current'));
const xLang = letztesX(kurven.querySelector('.curve-best'));
line(
  `Der kürzere Lauf endet früher im Bild: ${xKurz.toFixed(0)} gegen ${xLang.toFixed(0)}`,
  xKurz < xLang,
);

// Der Fall, in dem sich nichts aendern darf: derselbe Lauf gegen sich selbst.
const selbst = buildComparisonView(compareRuns(bestlauf.summary, bestlauf.summary));
line(
  'Derselbe Lauf gegen sich selbst meldet keine Verbesserung',
  [...selbst.querySelectorAll('tr[data-metric]')].every(
    (z) => (z as HTMLElement).dataset.improved === 'false',
  ),
);

// --- AC4: 50 Durchläufe ------------------------------------------------------

line('');
line('AC4 — Verlauf über 50 Durchläufe');

/**
 * Ein Verlauf mit 50 Einträgen und einem Fassungswechsel in der Mitte.
 *
 * Die Zahlen entstehen hier und nicht aus 50 gespielten Durchläufen: Geprüft
 * wird die ANSICHT, nicht der Kampf, und 50 echte Läufe kosteten eine halbe
 * Minute Rechenzeit, ohne einen einzigen zusätzlichen Nachweis zu liefern.
 */
const verlauf: RunHistoryEntry[] = Array.from({ length: 50 }, (_, i) => ({
  bossId: 'first-crucible',
  difficulty: 1,
  at: 1_800_000_000_000 + i * 600_000,
  gameVersion: i < 30 ? '0.1.0' : '0.2.0',
  victory: false,
  elapsedMs: 180_000,
  remainingPercent: 60 - i,
  damage: 40_000 + i * 500,
  idleMs: 8000 - i * 100,
  missedMs: 5000 - i * 80,
}));

const vorher = performance.now();
const verlaufsBild = buildHistoryView(verlauf);
const dauerMs = performance.now() - vorher;
document.querySelector('#view')?.appendChild(verlaufsBild);

const flaeche = verlaufsBild.querySelector('svg');
line(
  `Durchläufe im Bild: ${flaeche?.getAttribute('data-runs')}`,
  flaeche?.getAttribute('data-runs') === '50',
);
line(`Aufbau der Ansicht: ${dauerMs.toFixed(1)} ms`, dauerMs < 50);

// Der Schnitt beim Fassungswechsel — die Bedingung aus GAME-DESIGN 18.6.
const zuege = verlaufsBild.querySelectorAll('.history-line');
const schnitte = verlaufsBild.querySelectorAll('.history-cut');
line(
  `Zwei Fassungen, zwei Züge, ein Schnitt: ${zuege.length} Züge, ${schnitte.length} Schnitt`,
  zuege.length === 2 && schnitte.length === 1,
);
line(
  `Die Züge tragen ihre Fassung: ${[...zuege].map((z) => z.getAttribute('data-game-version')).join(', ')}`,
  [...zuege].map((z) => z.getAttribute('data-game-version')).join(',') === '0.1.0,0.2.0',
);

// Der Fall, in dem sich nichts ändern darf: eine einzige Fassung, KEIN Schnitt.
const eineFassung = verlauf.map((eintrag) => ({ ...eintrag, gameVersion: '0.1.0' }));
const ohneSchnitt = buildHistoryView(eineFassung);
line(
  `Eine Fassung, kein Schnitt: ${ohneSchnitt.querySelectorAll('.history-cut').length}`,
  ohneSchnitt.querySelectorAll('.history-cut').length === 0,
);
line(
  `Und die Zerlegung selbst sagt dasselbe: ${historySegments(eineFassung).length} Abschnitt`,
  historySegments(eineFassung).length === 1,
);
