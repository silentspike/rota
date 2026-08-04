/**
 * Prüfblatt für die Zähler-Anzeige (Issue #216, AC6).
 *
 * Hier läuft der **echte** `HudRenderer` gegen `test-doomsayer` — nicht eine
 * nachgebaute Zeichnung. Was auf dem Bild steht, steht auch im Spiel, sobald
 * ein Boss den Baustein trägt.
 *
 * Der Gegenfall gehört dazu: Gegen `first-crucible`, einen Boss ohne Zähler,
 * darf **nichts** erscheinen. Ein Balken, der immer dasteht, wird übersehen.
 */
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { PressQueue } from '../../src/input/queue.js';
import { Session } from '../../src/core/session.js';
import { HudRenderer } from '../../src/render/hud.js';
import { Surface } from '../../src/render/surface.js';
import { Overlay } from '../../src/render/overlay.js';
import { Impact } from '../../src/render/impact.js';
import { KeyLabels } from '../../src/input/layout.js';
import '../../src/render/page.css';

function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  document.querySelector('#out')?.appendChild(el);
}

/**
 * Ein Kampf, dessen KAMPFUHR `kampfMs` erreicht hat, mit fertigem Bild.
 *
 * Die Kampfuhr, nicht die Wanduhr: Vor dem Kampf steht der Countdown, und der
 * Zähler beginnt erst danach. Mein erster Entwurf rechnete mit der Wanduhr und
 * erwartete bei 19 s einen Stand von 95 — dagestanden hat 80, weil drei
 * Sekunden Countdown davor lagen. Die Rechnung war richtig, die Erwartung
 * falsch.
 */
function bild(bossId: string, kampfMs: number, titel: string): Combat {
  const boss = bossById(bossId);
  if (boss === undefined) throw new Error(`Boss ${bossId} fehlt.`);

  const kopf = document.createElement('p');
  kopf.className = 'sub';
  kopf.textContent = titel;
  document.querySelector('#view')?.appendChild(kopf);

  const halter = document.createElement('div');
  document.querySelector('#view')?.appendChild(halter);

  const combat = new Combat({ school: CALCINATION.value, boss });
  const queue = new PressQueue();
  const session = new Session(queue, combat);
  const surface = new Surface(halter);
  const overlay = new Overlay(halter, {
    onReady: () => undefined,
    onResume: () => undefined,
    onWipe: () => undefined,
    onRestart: () => undefined,
    onBackToMenu: () => undefined,
    onOpenHome: () => undefined,
  });
  const renderer = new HudRenderer(surface, session, new KeyLabels(), overlay, new Impact());

  session.beginCountdown(0);
  const bisMs = COUNTDOWN_MS + kampfMs;
  for (let t = 0; t <= bisMs; t += TICK_MS) session.tick(t);
  renderer.render(bisMs, 0);
  return combat;
}

// --- Der Zähler steht da, solange er läuft -----------------------------------

line('Ein Boss MIT Zähler: die Anzeige ist da und zeigt den Stand');

/** Was nach `kampfMs` Kampfzeit auf dem Zähler stehen muss — gerechnet. */
function erwartet(combat: Combat, kampfMs: number): number {
  const doom = combat.boss.doom;
  if (doom === null) return 0;
  return Math.min(doom.threshold, Math.floor(kampfMs / doom.perStepMs) * doom.amount);
}

const mit = bild('test-doomsayer', 12000, 'test-doomsayer, 12 s Kampfzeit');
line(
  `Zählerstand nach 12 s Kampf: ${mit.doom} von ${mit.boss.doom?.threshold} ` +
    `(gerechnet ${erwartet(mit, 12000)})`,
  mit.doom === erwartet(mit, 12000),
);
line(`Verbleibende Anwendungen: ${mit.doomCharges}`, mit.doomCharges === 2);

// Kurz vor der Schwelle — hier steht die Warnfarbe.
const knapp = bild(
  'test-doomsayer',
  19000,
  'derselbe Boss, 19 s Kampfzeit — eine Sekunde vor dem Tod',
);
line(
  `Zählerstand nach 19 s Kampf: ${knapp.doom} (gerechnet ${erwartet(knapp, 19000)})`,
  knapp.doom === erwartet(knapp, 19000),
);
line(`Über der Warnschwelle von 70 %: ${Math.round((knapp.doom / 100) * 100)} %`, knapp.doom >= 70);
line(`Der Spieler lebt noch: ${knapp.playerHp} Lebenspunkte`, knapp.playerHp > 0);

// --- Der Gegenfall ----------------------------------------------------------

line('');
line('Ein Boss OHNE Zähler: es darf nichts erscheinen');

const ohne = bild('first-crucible', 12000, 'first-crucible, 12 s Kampfzeit — keine Zähler-Anzeige');
line(`doom im Datensatz: ${String(ohne.boss.doom)}`, ohne.boss.doom === null);
line(`Zählerstand: ${ohne.doom}`, ohne.doom === 0);

line('');
line(
  'Was dieses Blatt NICHT belegt: dass ein Boss des Spiels den Baustein trägt. Keiner tut es — ' +
    'das entscheidet E10 mit den Signatur-Regeln.',
);
