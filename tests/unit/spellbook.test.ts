import { describe, expect, it } from 'vitest';
import { CALCINATION, type SpellDef } from '../../src/core/data.js';
import {
  chooseDraftSpell,
  isSpellbookState,
  legacySpellbook,
  newSpellbook,
  offerSpellDraft,
  reconcileSpellbook,
  spellbookSpells,
} from '../../src/progression/spellbook.js';
import { emptySave, RunPersistence } from '../../src/analysis/run-persistence.js';
import { SaveStore, type KeyValueStore } from '../../src/analysis/save-store.js';

const spells = CALCINATION.value.spells;
const spell = (id: string): SpellDef => {
  const found = spells.find((entry) => entry.id === id);
  if (found === undefined) throw new Error(`Missing fixture spell ${id}`);
  return found;
};

function memoryStore(): KeyValueStore {
  const values = new Map<string, string>();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
  };
}

describe('Zauberbuch-Draft', () => {
  it('beginnt mit zwei festen Zaubern und den immer verfügbaren gemeinsamen Werkzeugen', () => {
    expect(newSpellbook(spells).knownSpellIds).toEqual(['kindle', 'smoulder', 'quench', 'purge']);
    expect(spellbookSpells(newSpellbook(spells), spells).map((entry) => entry.id)).toEqual([
      'kindle',
      'smoulder',
      'quench',
      'purge',
    ]);
  });

  it('zieht drei verschiedene noch nicht besessene Zauber der erreichten Stufe', () => {
    const book = newSpellbook(spells);
    for (let seed = 0; seed < 100; seed++) {
      const offered = offerSpellDraft(book, 1, spells, seed);
      expect(offered.pendingSpellIds).toHaveLength(3);
      expect(new Set(offered.pendingSpellIds).size).toBe(3);
      for (const id of offered.pendingSpellIds) {
        expect(book.knownSpellIds).not.toContain(id);
        expect(spell(id).unlockLevel).toBeLessThanOrEqual(1);
        expect(spell(id).school).not.toBeNull();
      }
    }
  });

  it('gibt Prüfzauber auch auf einer unzulässig hohen Stufe nicht aus', () => {
    const knownSpellIds = spells
      .filter((entry) => entry.unlockLevel <= 100)
      .map((entry) => entry.id);
    const book = { knownSpellIds, pendingSpellIds: [] };
    expect(offerSpellDraft(book, 999, spells, 42)).toBe(book);
  });

  it('lässt ein offenes Angebot samt Reihenfolge stehen und erwirbt genau einen Zauber', () => {
    const offered = offerSpellDraft(newSpellbook(spells), 1, spells, 42);
    expect(offerSpellDraft(offered, 100, spells, 99)).toBe(offered);
    expect(chooseDraftSpell(offered, 'retort')).toBeNull();
    const selected = offered.pendingSpellIds[1]!;
    const acquired = chooseDraftSpell(offered, selected)!;
    expect(acquired.pendingSpellIds).toEqual([]);
    expect(acquired.knownSpellIds).toEqual([...offered.knownSpellIds, selected]);
    expect(chooseDraftSpell(acquired, selected)).toBeNull();
    expect(offerSpellDraft(acquired, 1, spells, 43).pendingSpellIds).not.toContain(selected);
  });

  it('kommt auch mit einem fast vollständigen Buch ohne Duplikate zurecht', () => {
    const book = {
      knownSpellIds: spells
        .filter((entry) => entry.unlockLevel <= 100 && entry.id !== 'crust')
        .map((entry) => entry.id),
      pendingSpellIds: [],
    };
    expect(offerSpellDraft(book, 100, spells, 42).pendingSpellIds).toEqual(['crust']);
  });

  it('bevorzugt fehlendes Survival, ohne andere Schulen aus dem Topf zu nehmen', () => {
    const pool = [spell('crust'), spell('bind'), spell('salve')];
    let survivalFirst = 0;
    let controlFirst = 0;
    let afterSurvival = 0;
    for (let seed = 0; seed < 3000; seed++) {
      const first = offerSpellDraft({ knownSpellIds: [], pendingSpellIds: [] }, 1, pool, seed)
        .pendingSpellIds[0];
      if (first === 'crust') survivalFirst++;
      if (first === 'bind') controlFirst++;
      const next = offerSpellDraft({ knownSpellIds: ['salve'], pendingSpellIds: [] }, 1, pool, seed)
        .pendingSpellIds[0];
      if (next === 'crust') afterSurvival++;
    }
    expect(survivalFirst / controlFirst).toBeGreaterThan(1.7);
    expect(survivalFirst / controlFirst).toBeLessThan(2.3);
    expect(afterSurvival / 3000).toBeGreaterThan(0.46);
    expect(afterSurvival / 3000).toBeLessThan(0.54);
  });
});

describe('Migration und Wiederherstellung des Zauberbuchs', () => {
  it('ein alter Spielstand behält alle bisher zugänglichen Zauber und seine Belegung', () => {
    const storage = memoryStore();
    const { spellbook: _book, ...old } = emptySave('old', 1);
    void _book;
    new SaveStore(storage).save({
      ...old,
      schema: 3,
      level: 12,
      bindings: { '0': 'crust', '1': 'pyre' },
    });
    const loaded = new RunPersistence('new', () => 2, storage);
    expect(loaded.spellbook).toEqual(legacySpellbook(12, spells, { '0': 'crust', '1': 'pyre' }));
    for (const entry of spells.filter((value) => value.unlockLevel <= 12)) {
      expect(loaded.spellbook.knownSpellIds).toContain(entry.id);
    }
    expect(loaded.save.bindings).toEqual({ '0': 'crust', '1': 'pyre' });
    expect(loaded.spellbook.pendingSpellIds).toEqual([]);
  });

  it('bewahrt ein gezogenes Angebot und danach die Auswahl über mehrere Neustarts', () => {
    const storage = memoryStore();
    const first = new RunPersistence('new', () => 1, storage);
    const offer = offerSpellDraft(first.spellbook, 1, spells, 42);
    expect(first.persistSpellbook(offer)).toBe(true);
    const second = new RunPersistence('new', () => 2, storage);
    expect(second.spellbook).toEqual(offer);
    const choice = chooseDraftSpell(second.spellbook, offer.pendingSpellIds[0]!)!;
    expect(second.persistSpellbook(choice)).toBe(true);
    expect(new RunPersistence('new', () => 3, storage).spellbook).toEqual(choice);
  });

  it('fällt bei beschädigter Angebotsstruktur auf den letzten gesunden Stand zurück', () => {
    const storage = memoryStore();
    const saves = new SaveStore(storage);
    saves.save(emptySave('new', 1));
    saves.save({
      ...emptySave('new', 2),
      spellbook: { knownSpellIds: [], pendingSpellIds: ['crust', 'crust'] },
    });
    const loaded = new RunPersistence('new', () => 3, storage);
    expect(loaded.status.status).toBe('recovered');
    expect(loaded.spellbook).toEqual(newSpellbook(spells));
    expect(isSpellbookState({ knownSpellIds: [], pendingSpellIds: null })).toBe(false);
  });

  it('entfernte Angebotszauber sperren nicht dauerhaft, erworbene Kennungen bleiben erhalten', () => {
    const old = { knownSpellIds: ['removed-owned'], pendingSpellIds: ['removed-offer', 'crust'] };
    expect(reconcileSpellbook(old, spells)).toEqual({
      knownSpellIds: ['removed-owned'],
      pendingSpellIds: ['crust'],
    });
  });
});
