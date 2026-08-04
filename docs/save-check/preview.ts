/**
 * Prüfblatt für Issue #83, AC1 und AC2 — im ECHTEN localStorage.
 *
 * Die Unit-Tests prüfen die Rotation gegen eine nachgebaute Abbildung. Das ist
 * für die Verdrängungsregel richtig, beweist aber nicht, dass es im Browser
 * ebenso läuft: Dort ist der Speicher geteilt, begrenzt und überlebt einen
 * Neustart. Deshalb hier derselbe Code gegen `window.localStorage`.
 */
import { SaveStore, checksumOf, SLOT_COUNT } from '../../src/analysis/save-store.js';
import { SAVE_SCHEMA, type SaveData } from '../../src/analysis/save-format.js';

function save(level: number): SaveData {
  return {
    schema: SAVE_SCHEMA,
    gameVersion: '0.1.0',
    savedAt: Date.now() + level,
    level,
    experience: level * 1000,
    talentPoints: { calcination: level },
    items: [{ itemId: 'ember-band', upgradeSteps: level, equippedSlot: 2 }],
    bindings: { q: 0, w: 1, e: 2 },
    unlockedSpells: ['kindle', 'smoulder', 'pyre'],
    unlockedInstances: ['first-crucible'],
    bestRuns: [
      {
        bossId: 'first-crucible',
        difficulty: 1,
        timeMs: 184_200,
        gameVersion: '0.1.0',
        achievedAt: Date.now(),
      },
    ],
    defeatedBosses: ['first-crucible'],
  };
}

function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  document.querySelector('#out')?.appendChild(el);
}

localStorage.clear();
const store = new SaveStore(localStorage);

// --- AC1: beschädigter Stand führt zum letzten heilen ---------------------
line('AC1 — beschädigter Spielstand');
for (let i = 1; i <= SLOT_COUNT; i++) store.save(save(i));
line(`  ${SLOT_COUNT} Stände in localStorage abgelegt, zuletzt Stufe ${store.load().data?.level}`);

const key = `rota.save.${localStorage.getItem('rota.save.index')}`;
const raw = JSON.parse(localStorage.getItem(key) as string) as { payload: string };
localStorage.setItem(
  key,
  JSON.stringify({ checksum: checksumOf(raw.payload), payload: raw.payload + ' ' }),
);
line('  jüngsten Stand von außen verändert (wie ein abgebrochener Schreibvorgang)');

const rescued = store.load();
line(
  `  geladen: Status ${rescued.status}, übersprungen ${rescued.skipped}, Stufe ${rescued.data?.level}`,
  rescued.status === 'recovered' && rescued.skipped === 1 && rescued.data?.level === SLOT_COUNT - 1,
);

// --- AC2: Ausfuhr und Wiedereinfuhr ---------------------------------------
line('');
line('AC2 — Ausfuhr und Wiedereinfuhr');
const text = store.export();
line(`  ausgeführt: ${text?.length ?? 0} Zeichen JSON`);

localStorage.clear();
line('  localStorage geleert — wie auf einem anderen Gerät');
const back = new SaveStore(localStorage).import(text as string);
const same = JSON.stringify(back.data) === JSON.stringify(JSON.parse(text as string));
line(
  `  wieder eingelesen: Status ${back.status}, Stand identisch: ${same}`,
  back.status === 'ok' && same,
);
