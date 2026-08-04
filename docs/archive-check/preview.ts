/**
 * Prüfblatt für Issue #85, AC1 bis AC4.
 *
 * Es lädt `src/analysis/` unverändert und läuft über eine Vite-Eingangsseite —
 * nicht über eine nginx-Location. Eine Location in /etc/nginx/ zeigt auf einen
 * Worktree im gerade ausgecheckten Branch und ist Evidence, die nur auf einem
 * Rechner funktioniert (Lehre aus #83).
 */
import {
  buildArchive,
  nextAutomaticEntry,
  unlock,
  type ArchiveEntry,
  type ArchiveState,
} from '../../src/analysis/archive.js';
import {
  markSeen,
  nextIntroduction,
  withPlainText,
  type HelpEntry,
  type Introduction,
} from '../../src/analysis/help.js';
import {
  attachHelp,
  buildArchiveView,
  buildIntroduction,
} from '../../src/analysis/archive-view.js';
import '../../src/analysis/archive.css';

const ENTRIES: ArchiveEntry[] = [
  {
    id: 'crucible-intro',
    instanceId: 'The First Crucible',
    trigger: 'beforeInstance',
    voice: 'ownNotes',
    title: 'Before the first crucible',
    body: 'The vessel was cold when I found it. It is not cold now.',
  },
  {
    id: 'crucible-kill',
    instanceId: 'The First Crucible',
    trigger: 'afterFirstKill',
    voice: 'deviceLog',
    title: 'Reaction contained',
    body: 'Pressure nominal. Vessel integrity 71 percent. Recommend no further use of this vessel.',
    bossId: 'first-crucible-boss',
  },
];

const HELP: HelpEntry[] = [
  {
    id: 'kindle',
    subject: 'spell',
    title: 'Kindle',
    effect: 'Deals fire damage and builds heat.',
    values: '100 damage, 1.5 s cooldown, +3 heat',
  },
  {
    id: 'ESSENCE',
    subject: 'stat',
    title: 'ESSENCE',
    effect: 'Raises the damage of every process.',
  },
  {
    id: 'crucible-flare',
    subject: 'bossAbility',
    title: 'Flare',
    effect: 'The vessel vents heat. Interruptible.',
    values: '2.5 s cast',
  },
];

const INTROS: Introduction[] = [
  {
    mechanicId: 'heat',
    title: 'Heat',
    body: 'Every calcination process adds heat. One spell spends all of it at once.',
  },
];

function out(): HTMLElement {
  return document.querySelector('#out') as HTMLElement;
}
function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  out().appendChild(el);
}

// --- AC1: einmal automatisch, dann nie wieder, aber auffindbar -------------
line('AC1 — einmal automatisch, danach nachlesbar');
let state: ArchiveState = { unlockedEntryIds: [] };
const context = {
  trigger: 'afterFirstKill' as const,
  instanceId: 'The First Crucible',
  bossId: 'first-crucible-boss',
};

const ersterSieg = nextAutomaticEntry(ENTRIES, state, context);
line(`  erster Sieg: ${ersterSieg ? `"${ersterSieg.title}" erscheint` : 'nichts'}`);
if (ersterSieg) state = unlock(state, ersterSieg.id);
const zweiterSieg = nextAutomaticEntry(ENTRIES, state, context);
line(
  `  zweiter Sieg: ${zweiterSieg ? `"${zweiterSieg.title}"` : 'nichts erscheint'}`,
  ersterSieg !== null && zweiterSieg === null,
);

state = unlock(state, 'crucible-intro');
const sections = buildArchive(ENTRIES, state);
const imArchiv = sections.some((s) => s.entries.some((e) => e.id === 'crucible-kill'));
line(`  im Archiv auffindbar: ${imArchiv}`, imArchiv);

// --- AC3: Klartext neben dem Fachbegriff -----------------------------------
line('');
line('AC3 — Klartext neben jedem Fachbegriff');
const proben = ['COAGULATION', 'CALCINATION', 'CATALYSIS', 'YIELD'].map(withPlainText);
for (const p of proben) line(`  ${p}`);
line(
  '  alle mit Klartext',
  proben.every((p) => p.includes('(')),
);

// --- Ansicht aufbauen ------------------------------------------------------
const view = document.querySelector('#view') as HTMLElement;
view.appendChild(buildArchiveView(sections));

const bar = document.createElement('div');
bar.className = 'demo-bar';
for (const [id, label] of [
  ['kindle', 'Q'],
  ['ESSENCE', 'ESSENCE'],
  ['crucible-flare', 'Flare'],
] as const) {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = 'demo-target';
  el.dataset.help = id;
  el.textContent = label;
  bar.appendChild(el);
}
view.insertBefore(bar, view.firstChild);
attachHelp(view, HELP);

// --- AC2: Erklärfenster über allen drei Gruppen ----------------------------
line('');
line('AC2 — Erklärfenster über Symbol, Wert und Boss-Fähigkeit');
for (const id of ['kindle', 'ESSENCE', 'crucible-flare']) {
  const target = view.querySelector<HTMLElement>(`[data-help="${id}"]`);
  target?.dispatchEvent(new MouseEvent('mouseenter'));
  const da =
    target?.querySelector('.help-popover') !== null &&
    target?.querySelector('.help-popover') !== undefined;
  line(`  ${id}: Fenster erscheint: ${da}`, da);
  target?.dispatchEvent(new MouseEvent('mouseleave'));
}

// --- AC4: die Einführung ist überspringbar ---------------------------------
line('');
line('AC4 — geführte Einführung, überspringbar');
let introState = { seenMechanicIds: [] as string[] };
const intro = nextIntroduction(INTROS, introState, 'heat');
line(`  erstes Auftreten: ${intro ? `"${intro.title}" hält an` : 'nichts'}`);
if (intro) {
  const box = buildIntroduction(intro, (id) => {
    introState = markSeen(introState, id) as { seenMechanicIds: string[] };
  });
  view.insertBefore(box, view.firstChild);
  const skip = box.querySelector<HTMLButtonElement>('.intro-skip');
  line(`  SKIP-Knopf vorhanden: ${skip !== null}`, skip !== null);
  skip?.click();
  const wieder = nextIntroduction(INTROS, introState, 'heat');
  line(
    `  nach dem Überspringen: ${wieder ? 'kommt wieder' : 'kommt nicht wieder'}`,
    wieder === null,
  );
}
