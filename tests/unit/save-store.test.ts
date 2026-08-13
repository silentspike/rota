/**
 * Rotation, Beschaedigung und Ausfuhr (Issue #83, AC1 bis AC4).
 *
 * Der Browser-Speicher ist hier als einfache Abbildung nachgebaut - nicht um
 * ihn zu umgehen, sondern weil die Verdraengungsregel sonst nur im Browser
 * pruefbar waere und ein Fehler darin sich hinter der Umgebung versteckte.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { SAVE_SCHEMA, type SaveData } from '../../src/analysis/save-format.js';
import {
  checksumOf,
  SaveStore,
  SLOT_COUNT,
  type KeyValueStore,
} from '../../src/analysis/save-store.js';

class MemoryStore implements KeyValueStore {
  readonly map = new Map<string, string>();
  getItem(key: string): string | null {
    return this.map.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.map.set(key, value);
  }
  removeItem(key: string): void {
    this.map.delete(key);
  }
}

function save(level = 1): SaveData {
  return {
    schema: SAVE_SCHEMA,
    gameVersion: '0.1.0',
    savedAt: 1_800_000_000_000 + level,
    level,
    experience: level * 1000,
    talentPoints: {},
    items: [],
    bindings: { '0': 'kindle' },
    bestRuns: [],
    defeatedBosses: [],
    unlockedEntryIds: [],
    seenMechanicIds: [],
    runHistory: [],
    progressMarks: [],
  };
}

let store: MemoryStore;
let saves: SaveStore;

beforeEach(() => {
  store = new MemoryStore();
  saves = new SaveStore(store);
});

describe('AC4 — fuenf Sicherungen, die sechste verdraengt die aelteste', () => {
  it('haelt genau fuenf Plaetze', () => {
    for (let i = 1; i <= SLOT_COUNT; i++) saves.save(save(i));
    expect(saves.healthySlots()).toBe(SLOT_COUNT);
    expect(saves.load().data?.level).toBe(SLOT_COUNT);
  });

  it('ueberschreibt beim sechsten den aeltesten, nicht einen anderen', () => {
    for (let i = 1; i <= SLOT_COUNT; i++) saves.save(save(i));
    saves.save(save(6));
    expect(saves.healthySlots()).toBe(SLOT_COUNT);
    // Stand 1 ist weg, 2 bis 6 sind da.
    const stufen = new Set<number>();
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      const raw = store.getItem(`rota.save.${slot}`);
      if (raw !== null) {
        const { payload } = JSON.parse(raw) as { payload: string };
        stufen.add((JSON.parse(payload) as SaveData).level);
      }
    }
    expect([...stufen].sort((a, b) => a - b)).toEqual([2, 3, 4, 5, 6]);
  });

  it('legt auch nach vielen Durchlaeufen nie mehr als fuenf an', () => {
    for (let i = 1; i <= 40; i++) saves.save(save(i));
    expect(store.map.size).toBe(SLOT_COUNT + 1); // fuenf Staende plus Zeiger
    expect(saves.load().data?.level).toBe(40);
  });
});

describe('AC1 — ein beschaedigter Stand fuehrt zum letzten heilen', () => {
  it('ueberspringt den juengsten, wenn seine Pruefsumme nicht stimmt', () => {
    saves.save(save(1));
    saves.save(save(2));
    // Von aussen veraendert, wie es ein abgebrochener Schreibvorgang taete.
    const raw = JSON.parse(store.getItem('rota.save.1') as string) as { payload: string };
    store.setItem(
      'rota.save.1',
      JSON.stringify({ checksum: checksumOf(raw.payload), payload: raw.payload + ' ' }),
    );

    const result = saves.load();
    expect(result.status).toBe('recovered');
    expect(result.skipped).toBe(1);
    expect(result.data?.level).toBe(1);
  });

  it('meldet leer, statt Unsinn zu liefern, wenn alle beschaedigt sind', () => {
    for (let i = 1; i <= SLOT_COUNT; i++) saves.save(save(i));
    for (let slot = 0; slot < SLOT_COUNT; slot++) store.setItem(`rota.save.${slot}`, 'kaputt{');
    const result = saves.load();
    expect(result.status).toBe('empty');
    expect(result.data).toBeNull();
  });

  it('haelt einen Stand aus einer kuenftigen Fassung fuer unlesbar', () => {
    // "Neuer" heisst nicht "heil" - wir wissen nicht, was darin steht.
    saves.save(save(1));
    const zukunft = { ...save(2), schema: SAVE_SCHEMA + 1 };
    const payload = JSON.stringify(zukunft);
    store.setItem('rota.save.1', JSON.stringify({ checksum: checksumOf(payload), payload }));
    store.setItem('rota.save.index', '1');
    const result = saves.load();
    expect(result.status).toBe('recovered');
    expect(result.data?.level).toBe(1);
  });

  it('sagt ok, wenn nichts uebersprungen wurde', () => {
    saves.save(save(3));
    const result = saves.load();
    expect(result.status).toBe('ok');
    expect(result.skipped).toBe(0);
  });
});

describe('AC2 — Ausfuhr und Wiedereinfuhr stellen den Stand her', () => {
  it('bringt denselben Stand zurueck', () => {
    saves.save(save(7));
    const text = saves.export();
    expect(text).not.toBeNull();

    const andererBrowser = new SaveStore(new MemoryStore());
    const result = andererBrowser.import(text as string);
    expect(result.status).toBe('ok');
    expect(andererBrowser.load().data).toEqual(save(7));
  });

  it('verdraengt beim Einlesen keinen bestehenden Stand', () => {
    // Wer eine fremde Datei einliest, soll seinen eigenen Stand behalten,
    // falls sie nicht ist, was er dachte.
    saves.save(save(1));
    saves.import(JSON.stringify(save(2)));
    expect(saves.healthySlots()).toBe(2);
  });

  it('lehnt unlesbaren Text ab, statt ihn abzulegen', () => {
    saves.save(save(1));
    expect(saves.import('kein json').status).toBe('empty');
    expect(saves.healthySlots()).toBe(1);
  });

  it('gibt null aus, wenn es nichts auszufuehren gibt', () => {
    expect(saves.export()).toBeNull();
  });
});

describe('AC3 — kein berechneter Wert kommt in den Speicher', () => {
  it('verweigert das Schreiben, statt es spaeter auffallen zu lassen', () => {
    const verdorben = {
      ...save(1),
      items: [{ itemId: 'x', bonusDamage: 40 }],
    } as unknown as SaveData;
    expect(() => saves.save(verdorben)).toThrow(/berechnete Werte/);
    expect(saves.healthySlots()).toBe(0);
  });
});
