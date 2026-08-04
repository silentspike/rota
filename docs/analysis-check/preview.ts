/**
 * Prüfblatt für Issue #81.
 *
 * AC4 wird hier im Bild belegt: die Spaltenfolge der Effizienz-Tabelle. Die
 * übrigen Nachweise stehen daneben, damit man nicht die Tabelle sieht und
 * glaubt, die Zahlen darin seien geprüft.
 *
 * Nach WORKER-GUIDE § 5 gebaut: Jeder Nachweis zeigt eine WIRKUNG als
 * Ungleichung und bringt den Fall mit, in dem sich nichts ändern darf. Zwei
 * Durchläufe laufen nebeneinander — ändern sich die Zahlen zwischen ihnen
 * nicht, misst die Auswertung nicht das Spiel.
 */
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { EFFICIENCY_COLUMNS } from '../../src/analysis/efficiency.js';
import { analyzeRun, runDataFor, type RunAnalysis } from '../../src/analysis/run-analysis.js';
import { buildEfficiencyView, buildLossView } from '../../src/analysis/analysis-view.js';
import '../../src/analysis/analysis.css';

const KINDLE = 0;
const SMOULDER = 1;
const FIGHT_MS = 30_000;

function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  document.querySelector('#out')?.appendChild(el);
}

/**
 * Einen Durchlauf wirklich spielen.
 *
 * Ohne `beginCountdown` bliebe die Sitzung in der Vorbereitung; es würde nichts
 * gewirkt, und jede Zahl wäre null — dann geht auch eine falsche Rechnung durch.
 */
function play(loadout: readonly number[], everyMs: number): RunAnalysis {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const session = new Session(queue, combat);
  loadout.forEach((spell, slot) => {
    session.bind(slot, spell);
  });
  session.beginCountdown(0);

  const ende = COUNTDOWN_MS + FIGHT_MS;
  let tick = 0;
  let presses = 0;
  for (let t = 0; t <= ende; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      // Reihum nach der Zahl der Drücke, nicht nach dem Takt: über den Takt
      // gerechnet träfen alle Drücke denselben Platz.
      queue.push(presses % loadout.length, tick, t, tick);
      presses++;
    }
    session.tick(t);
    tick++;
  }

  const summary = session.summary(ende);
  return analyzeRun({
    summary,
    gcdMs: GCD_MS,
    spells: runDataFor(combat, summary, loadout),
    synergyBonusDamage: 0,
    missedInterrupts: null,
    damagePerLandedBossCast: 0,
  });
}

const sauber = play([KINDLE, SMOULDER], 1500);
const schlampig = play([KINDLE, SMOULDER], 3000);

// --- AC4: die Spaltenfolge --------------------------------------------------

line('AC4 — Effizienz führt, Schadensanteil steht am Ende');
line(`Spalten laut Modul: ${EFFICIENCY_COLUMNS.join(' · ')}`);

const view = buildEfficiencyView(schlampig.table);
document.querySelector('#view')?.appendChild(view);
document.querySelector('#view')?.appendChild(buildLossView(schlampig.losses));

const kopf = [...view.querySelectorAll('th')].map((th) => th.textContent ?? '');
// Die erste Zelle ist der Zaubername; die Kennzahlen folgen dahinter.
const kennzahlen = kopf.slice(1);
line(`Im Bild gerendert: ${kopf.join(' · ')}`);
line(`Erste Kennzahl ist EFFICIENCY: ${kennzahlen[0]}`, kennzahlen[0] === 'EFFICIENCY');
line(
  `Letzte Kennzahl ist DAMAGE SHARE: ${kennzahlen.at(-1)}`,
  kennzahlen.at(-1) === 'DAMAGE SHARE',
);
line(
  'Die Ansicht schreibt die Reihenfolge nicht selbst hin, sondern nimmt sie aus dem Modul',
  kennzahlen.join('|') === [...EFFICIENCY_COLUMNS].join('|'),
);

// Der Fall, in dem sich nichts ändern darf: eine andere Zusammenstellung
// ändert die Zahlen, aber niemals die Spaltenfolge.
const andereKopf = [...buildEfficiencyView(sauber.table).querySelectorAll('th')].map(
  (th) => th.textContent ?? '',
);
line(
  'Ein anderer Durchlauf ändert die Spaltenfolge nicht',
  andereKopf.join('|') === kopf.join('|'),
);

// --- AC1: verpasste Wirkungen als größte Quelle -----------------------------

line('');
line('AC1 — absichtlich verpasste Wirkungen stehen oben');
line(
  `Größte Quelle im schlampigen Durchlauf: ${schlampig.losses.top[0]?.kind ?? '—'} ` +
    `(${((schlampig.losses.top[0]?.share ?? 0) * 100).toFixed(1)} %)`,
  schlampig.losses.top[0]?.kind === 'missedCasts',
);
line(
  'Im sauberen Durchlauf erscheinen sie nicht',
  !sauber.losses.sources.some((s) => s.kind === 'missedCasts'),
);

// --- AC7: die beiden Hälften überschneiden sich nicht -----------------------

line('');
line('AC7 — Leerlauf und verpasste Wirkungen schließen einander aus');
for (const [name, lauf] of [
  ['sauber', sauber],
  ['schlampig', schlampig],
] as const) {
  line(
    `${name}: ${lauf.missedCastWindows} Fenster mit bereitem Zauber, ` +
      `${lauf.idleWindows} ohne — zusammen ${lauf.missedCastWindows + lauf.idleWindows}`,
  );
}
line(
  `Kein Verlust übersteigt das Potenzial: ${Math.round(schlampig.losses.totalLost)} von ` +
    `${Math.round(schlampig.losses.potential)}`,
  schlampig.losses.totalLost <= schlampig.losses.potential,
);

// --- Die Lücke bleibt sichtbar ----------------------------------------------

line('');
line('Was heute nicht messbar ist (Issue #151)');
line(
  `Als Lücke ausgewiesen: ${schlampig.losses.unavailable.join(', ')}`,
  schlampig.losses.unavailable.includes('missedInterrupts'),
);
line(
  'Und nicht als Quelle mit Verlust 0',
  !schlampig.losses.sources.some((s) => s.kind === 'missedInterrupts'),
);
