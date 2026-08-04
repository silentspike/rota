/**
 * Prüfblatt für Issue #84, AC1 bis AC3.
 *
 * Nach WORKER-GUIDE § 5 gebaut: Jeder Nachweis zeigt eine WIRKUNG als
 * Ungleichung, nicht als gesetzten Wert — und jeder bringt den Fall mit, in
 * dem sich nichts ändern darf. Ein Blatt, das nur zeigt, dass eine Einstellung
 * gesetzt IST, wäre grün, auch wenn sie nichts bewirkt.
 */
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, type BossDef } from '../../src/core/data.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';
import { PressQueue } from '../../src/input/queue.js';
import { buildDummy, DEFAULT_DUMMY, TrainingRoom } from '../../src/analysis/training.js';
import { LiveMetricsView } from '../../src/analysis/training-view.js';
import '../../src/analysis/training.css';

function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  document.querySelector('#out')?.appendChild(el);
}

/** Ein Durchgang, in dem WIRKLICH gewirkt wird — sonst ist alles null. */
function runFor(boss: BossDef, untilMs = COUNTDOWN_MS + 9000): RunSummary {
  const queue = new PressQueue();
  const session = new Session(queue, new Combat({ school: CALCINATION.value, boss }));
  session.bind(0, 0);
  session.beginCountdown(0);
  let tick = 0;
  for (let t = 0; t <= untilMs; t += 100) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % 1500 === 0) queue.push(0, tick, t, tick);
    session.tick(t);
    tick++;
  }
  return session.summary(untilMs);
}

// --- AC1: die drei Einstellungen wirken -----------------------------------
line('AC1 — die drei Einstellungen wirken, einzeln und kombiniert');

const neutral = runFor(buildDummy(DEFAULT_DUMMY, 'CALCINATION'));
const halb = runFor(buildDummy({ ...DEFAULT_DUMMY, resistance: 0.5 }, 'CALCINATION'));
line(`  Schaden ohne Widerstand: ${Math.round(neutral.damage)}`);
line(`  Schaden bei Widerstand 0,5: ${Math.round(halb.damage)}`);
// Ungleichung, nicht Gleichheit: Wären beide null, fiele es hier auf.
line(`  Widerstand VERÄNDERT den Schaden`, halb.damage < neutral.damage && neutral.damage > 0);

const nochmalNeutral = runFor(buildDummy({ ...DEFAULT_DUMMY, resistance: 1 }, 'CALCINATION'));
line(`  neutraler Widerstand ändert NICHTS`, nochmalNeutral.damage === neutral.damage);

const mitZaubern = buildDummy({ ...DEFAULT_DUMMY, castsSpells: true }, 'CALCINATION');
const ohneZauber = buildDummy({ ...DEFAULT_DUMMY, castsSpells: false }, 'CALCINATION');
line(
  `  Zauber-Einstellung: Zeitleiste ${ohneZauber.timeline.length} → ${mitZaubern.timeline.length}`,
  ohneZauber.timeline.length === 0 && mitZaubern.timeline.length > 0,
);

const kombi = runFor(
  buildDummy({ resistance: 0.25, dealsDamage: true, castsSpells: true }, 'CALCINATION'),
);
line(`  kombiniert: Schaden ${Math.round(kombi.damage)}`, kombi.damage < halb.damage);

// --- AC2: die Kennzahlen bewegen sich WÄHREND des Übens --------------------
line('');
line('AC2 — die Kennzahlen aktualisieren sich während des Übens');

const view = new LiveMetricsView();
document.querySelector('#view')?.appendChild(view.root);

const queue = new PressQueue();
const session = new Session(
  queue,
  new Combat({ school: CALCINATION.value, boss: buildDummy(DEFAULT_DUMMY, 'CALCINATION') }),
);
session.bind(0, 0);
session.beginCountdown(0);

const proben: string[] = [];
for (let t = 0; t <= COUNTDOWN_MS + 12_000; t += 100) {
  session.tick(t);
  view.update({
    lostTimeMs: session.idleMs,
    lostTimeShare: 0,
    missedCasts: 0,
    synergyUsed: 0,
    synergyAvailable: 0,
  });
  // Mitten im Durchgang ablesen, nicht am Ende.
  if (t === COUNTDOWN_MS + 2000 || t === COUNTDOWN_MS + 6000 || t === COUNTDOWN_MS + 11_000) {
    proben.push(
      document.querySelector('[data-metric="idle"] .training-metric-value')?.textContent ?? '',
    );
  }
}
for (const [i, p] of proben.entries()) line(`  Ablesung ${i + 1} mitten im Durchgang: ${p}`);
// Ungleichung: Die Werte müssen sich UNTERSCHEIDEN, nicht nur existieren.
line(`  die Anzeige BEWEGT sich`, new Set(proben).size === proben.length && proben[0] !== '—');

// --- AC3: Wechsel mitten im Durchgang wird abgelehnt -----------------------
line('');
line('AC3 — Wechsel der Zusammenstellung mitten im Durchgang');

const raum = new TrainingRoom();
raum.setLoadout(['kindle', 'smoulder']);
const vorher = [...raum.loadout];
raum.start();
const abgelehnt = raum.setLoadout(['pyre']);
line(
  `  Versuch während des Übens: ${abgelehnt.accepted ? 'angenommen' : 'abgelehnt'} — "${abgelehnt.reason}"`,
);
// Der Fall, in dem sich nichts ändern darf: die alte Belegung MUSS stehen.
line(
  `  alte Zusammenstellung steht: ${raum.loadout.join(', ')}`,
  !abgelehnt.accepted && raum.loadout.join(',') === vorher.join(','),
);
raum.stop();
const danach = raum.setLoadout(['pyre']);
line(`  nach dem Abbrechen wieder möglich`, danach.accepted && raum.loadout.join(',') === 'pyre');
