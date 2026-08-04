/**
 * Die Regel aus GAME-DESIGN 17.4: einmal automatisch, danach nie wieder von
 * selbst, aber jederzeit nachlesbar (Issue #85, AC1).
 *
 * Bei hunderten Wiederholungen desselben Bosses ist genau das der Unterschied
 * zwischen einer Geschichte, die traegt, und einer, die im Weg steht.
 */
import { describe, expect, it } from 'vitest';
import {
  buildArchive,
  nextAutomaticEntry,
  unlock,
  unlockedCount,
  type ArchiveEntry,
  type ArchiveState,
} from '../../src/analysis/archive.js';

const ENTRIES: ArchiveEntry[] = [
  {
    id: 'crucible-intro',
    instanceId: 'first-crucible',
    trigger: 'beforeInstance',
    voice: 'ownNotes',
    title: 'Before the first crucible',
    body: '...',
  },
  {
    id: 'crucible-kill',
    instanceId: 'first-crucible',
    trigger: 'afterFirstKill',
    voice: 'deviceLog',
    title: 'Reaction contained',
    body: '...',
    bossId: 'first-crucible-boss',
  },
  {
    id: 'ashfall-intro',
    instanceId: 'ashfall',
    trigger: 'beforeInstance',
    voice: 'fragment',
    title: 'Ashfall',
    body: '...',
  },
];

const LEER: ArchiveState = { unlockedEntryIds: [] };

describe('AC1 — einmal automatisch, dann nie wieder von selbst', () => {
  it('zeigt den Eintrag beim ersten Sieg', () => {
    const entry = nextAutomaticEntry(ENTRIES, LEER, {
      trigger: 'afterFirstKill',
      instanceId: 'first-crucible',
      bossId: 'first-crucible-boss',
    });
    expect(entry?.id).toBe('crucible-kill');
  });

  it('zeigt ihn beim zweiten Sieg NICHT mehr', () => {
    const nachher = unlock(LEER, 'crucible-kill');
    const entry = nextAutomaticEntry(ENTRIES, nachher, {
      trigger: 'afterFirstKill',
      instanceId: 'first-crucible',
      bossId: 'first-crucible-boss',
    });
    expect(entry).toBeNull();
  });

  it('haelt ihn danach im Archiv auffindbar', () => {
    const nachher = unlock(LEER, 'crucible-kill');
    const archiv = buildArchive(ENTRIES, nachher);
    expect(archiv).toHaveLength(1);
    expect(archiv[0]?.entries.map((e) => e.id)).toEqual(['crucible-kill']);
  });

  it('verwechselt Instanzen nicht', () => {
    const entry = nextAutomaticEntry(ENTRIES, LEER, {
      trigger: 'beforeInstance',
      instanceId: 'ashfall',
    });
    expect(entry?.id).toBe('ashfall-intro');
  });

  it('zeigt bei einem anderen Boss derselben Instanz nichts', () => {
    const entry = nextAutomaticEntry(ENTRIES, LEER, {
      trigger: 'afterFirstKill',
      instanceId: 'first-crucible',
      bossId: 'ein-anderer',
    });
    expect(entry).toBeNull();
  });
});

describe('Archiv zeigt nur Freigeschaltetes', () => {
  it('deutet gesperrte Eintraege nicht einmal an', () => {
    // Eine graue Zeile waere eine Ankuendigung - und damit ein Hinweis auf
    // Inhalte, die der Spieler noch nicht erreicht hat.
    expect(buildArchive(ENTRIES, LEER)).toEqual([]);
    expect(unlockedCount(ENTRIES, LEER)).toBe(0);
  });

  it('gliedert nach Instanz, Text davor vor Text danach', () => {
    let state = LEER;
    for (const id of ['crucible-kill', 'crucible-intro', 'ashfall-intro'])
      state = unlock(state, id);
    const archiv = buildArchive(ENTRIES, state);
    const crucible = archiv.find((s) => s.instanceId === 'first-crucible');
    expect(crucible?.entries.map((e) => e.id)).toEqual(['crucible-intro', 'crucible-kill']);
    expect(unlockedCount(ENTRIES, state)).toBe(3);
  });
});

describe('unlock', () => {
  it('vermerkt einen Eintrag genau einmal', () => {
    const einmal = unlock(LEER, 'a');
    const zweimal = unlock(einmal, 'a');
    expect(zweimal.unlockedEntryIds).toEqual(['a']);
    expect(zweimal).toBe(einmal);
  });

  it('laesst den bisherigen Zustand unveraendert', () => {
    const neu = unlock(LEER, 'a');
    expect(LEER.unlockedEntryIds).toEqual([]);
    expect(neu.unlockedEntryIds).toEqual(['a']);
  });
});
