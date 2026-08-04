/**
 * Der Spielstand im Browser (Issue #179).
 *
 * DIE FEHLERPFADE SIND DER GRUND FUER DIESE DATEI.
 * Ein Pfad, der gebaut ist und nie gelaufen ist, ist eine Vermutung. „Vertraegt
 * einen Browser ohne Speicher" stand als Satz im Kommentar, ohne dass es je
 * jemand ausgeloest haette -- und genau solche Saetze stimmen erfahrungsgemaess
 * nicht.
 */
import { describe, expect, it } from 'vitest';
import type { KeyValueStore } from '../../src/analysis/save-store.js';
import { emptySave, RunPersistence } from '../../src/analysis/run-persistence.js';
import { checksumOf } from '../../src/analysis/save-store.js';
import { historySegments } from '../../src/analysis/run-history.js';
import type { RunHistoryEntry } from '../../src/analysis/save-format.js';
import { findForbiddenFields } from '../../src/analysis/save-format.js';

const FASSUNG = '0.1.0';
const JETZT = 1_800_000_000_000;

/** Ein Speicher im Arbeitsspeicher — dieselbe Schnittstelle wie localStorage. */
class MemoryStore implements KeyValueStore {
  readonly data = new Map<string, string>();
  getItem(key: string): string | null {
    return this.data.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.data.set(key, value);
  }
  removeItem(key: string): void {
    this.data.delete(key);
  }
}

/** Einer, der beim Schreiben wirft — wie ein volles Kontingent. */
class FullStore extends MemoryStore {
  override setItem(): void {
    throw new DOMException('QuotaExceededError');
  }
}

function eintrag(over: Partial<RunHistoryEntry> = {}): RunHistoryEntry {
  return {
    bossId: 'first-crucible',
    difficulty: 1,
    at: JETZT,
    gameVersion: FASSUNG,
    victory: false,
    elapsedMs: 60_000,
    remainingPercent: 40,
    damage: 2500,
    idleMs: 0,
    missedMs: 1500,
    ...over,
  };
}

describe('Ohne Speicher laeuft das Spiel weiter', () => {
  it('meldet ihn als nicht verfuegbar, statt zu werfen', () => {
    // Ein Spiel, das nicht startet, weil die Fortschrittsmarke nicht abgelegt
    // werden konnte, haette sein Verhaeltnis verkehrt.
    const p = new RunPersistence(FASSUNG, () => JETZT, null);
    expect(p.status.status).toBe('unavailable');
    expect(p.state.runHistory).toEqual([]);
  });

  it('meldet das Misslingen zurueck, ohne den Ablauf anzuhalten', () => {
    const p = new RunPersistence(FASSUNG, () => JETZT, null);
    const gelungen = p.persist({ runHistory: [eintrag()], progressMarks: [] });
    expect(gelungen).toBe(false);
    expect(p.status.lastError).not.toBeNull();
  });
});

describe('Ein voller Speicher', () => {
  it('faengt den Fehler und gibt false zurueck, statt ihn durchzulassen', () => {
    // Der Pfad, der gebaut war und nie gelaufen ist. Wirft setItem durch, bliebe
    // das Spiel am Kampfende stehen -- ausgerechnet in dem Moment, in dem der
    // Spieler seine Auswertung sehen will.
    const p = new RunPersistence(FASSUNG, () => JETZT, new FullStore());
    let geworfen = false;
    let gelungen = true;
    try {
      gelungen = p.persist({ runHistory: [eintrag()], progressMarks: [] });
    } catch {
      geworfen = true;
    }
    expect(geworfen).toBe(false);
    expect(gelungen).toBe(false);
    expect(p.status.lastError).toContain('Quota');
  });

  it('behaelt den Stand im Arbeitsspeicher, damit der Spieler ihn noch sieht', () => {
    // Abgelegt ist er nicht -- verloren waere er aber erst beim Neuladen.
    const p = new RunPersistence(FASSUNG, () => JETZT, new FullStore());
    p.persist({ runHistory: [eintrag()], progressMarks: [] });
    expect(p.state.runHistory).toHaveLength(1);
  });
});

describe('Der gewoehnliche Weg', () => {
  it('legt ab und liest zurueck', () => {
    const store = new MemoryStore();
    const erste = new RunPersistence(FASSUNG, () => JETZT, store);
    expect(erste.status.status).toBe('empty');
    expect(erste.persist({ runHistory: [eintrag()], progressMarks: [] })).toBe(true);

    // Ein FRISCHER Leser auf demselben Speicher -- so wie nach einem Neuladen.
    const zweite = new RunPersistence(FASSUNG, () => JETZT, store);
    expect(zweite.status.status).toBe('ok');
    expect(zweite.state.runHistory).toHaveLength(1);
  });

  it('kommt an dem Waechter vorbei, den ich selbst gebaut habe', () => {
    // save() wirft, wenn ein Feld nach einem berechneten Wert klingt. Der
    // Verlaufs-Eintrag traegt `damage` -- erlaubt nur durch die benannte
    // Ausnahme in save-format.ts. Ohne sie waere der ganze Anschluss unmoeglich.
    const store = new MemoryStore();
    const p = new RunPersistence(FASSUNG, () => JETZT, store);
    expect(p.persist({ runHistory: [eintrag()], progressMarks: [] })).toBe(true);
    expect(findForbiddenFields(p.save)).toEqual([]);
  });

  it('liefert einen vollstaendigen leeren Stand, keinen halben', () => {
    // Die Liste der erlaubten Felder ist abschliessend. Ein Stand, dem eines
    // fehlt, fiele erst beim Schreiben auf.
    const leer = emptySave(FASSUNG, JETZT);
    expect(Object.keys(leer)).toContain('runHistory');
    expect(Object.keys(leer)).toContain('progressMarks');
    expect(findForbiddenFields(leer)).toEqual([]);
  });
});

describe('Ein Fassungswechsel zwischen zwei Laeufen', () => {
  it('laesst die Fassung an den ALTEN Eintraegen stehen', () => {
    // Der Punkt, an dem der Schnitt im Verlauf entsteht. Zoege die Fassung an
    // allen Eintraegen mit, gaebe es nie einen Schnitt -- und der Spieler saehe
    // eine durchgehende Linie ueber eine Balance-Aenderung hinweg.
    const store = new MemoryStore();

    const alt = new RunPersistence('0.1.0', () => JETZT, store);
    alt.persist({ runHistory: [eintrag({ gameVersion: '0.1.0' })], progressMarks: [] });

    const neu = new RunPersistence('0.2.0', () => JETZT + 1000, store);
    neu.persist({
      runHistory: [...neu.state.runHistory, eintrag({ gameVersion: '0.2.0', at: JETZT + 1000 })],
      progressMarks: [],
    });

    const verlauf = new RunPersistence('0.2.0', () => JETZT, store).state.runHistory;
    expect(verlauf.map((e) => e.gameVersion)).toEqual(['0.1.0', '0.2.0']);
    expect(historySegments(verlauf)).toHaveLength(2);
  });

  it('zieht die Fassung des STANDES dagegen mit', () => {
    // Sie sagt, unter welcher Fassung zuletzt geschrieben wurde -- etwas
    // anderes als die Fassung je Eintrag, und beide werden gebraucht.
    const store = new MemoryStore();
    new RunPersistence('0.1.0', () => JETZT, store).persist({
      runHistory: [],
      progressMarks: [],
    });
    const neu = new RunPersistence('0.2.0', () => JETZT, store);
    neu.persist({ runHistory: [], progressMarks: [] });
    expect(neu.save.gameVersion).toBe('0.2.0');
  });
});

describe('Ein Stand aus einer aelteren Fassung', () => {
  /** Einen Stand ablegen, wie ihn eine aeltere Fassung geschrieben haette. */
  function legeAltenStandAb(store: MemoryStore, ohne: readonly string[]): void {
    const alt: Record<string, unknown> = { ...emptySave(FASSUNG, JETZT) };
    for (const feld of ohne) delete alt[feld];
    const payload = JSON.stringify(alt);
    store.setItem('rota.save.0', JSON.stringify({ checksum: checksumOf(payload), payload }));
    store.setItem('rota.save.index', '0');
  }

  it('wirft nicht, wenn runHistory und progressMarks fehlen', () => {
    // Ein Stand vor Issue #82 kennt beide nicht. Er ist deswegen NICHT
    // beschaedigt: Pruefsumme und Schema stimmen, SaveStore gibt ihn als 'ok'
    // zurueck, und das ist richtig. Ohne Ergaenzung stuende dort undefined --
    // und der erste Zugriff wuerfe BEIM KAMPFENDE, also genau dann, wenn der
    // Spieler seine Auswertung sehen will.
    const store = new MemoryStore();
    legeAltenStandAb(store, ['runHistory', 'progressMarks']);

    const p = new RunPersistence(FASSUNG, () => JETZT, store);
    expect(p.status.status).toBe('ok');
    expect(p.state.runHistory).toEqual([]);
    expect(p.state.progressMarks).toEqual([]);
    // Und der naechste Kampf laesst sich ablegen.
    expect(p.persist({ runHistory: [eintrag()], progressMarks: [] })).toBe(true);
  });

  it('behaelt, was der alte Stand SEHR WOHL hatte', () => {
    // Der Fall, in dem sich nichts aendern darf. Eine Ergaenzung, die den
    // ganzen Stand ersetzt, waere schlimmer als der Absturz: Sie loeschte
    // Fortschritt, und niemand merkte es.
    const store = new MemoryStore();
    const alt = { ...emptySave(FASSUNG, JETZT), level: 7, experience: 4321 } as Record<
      string,
      unknown
    >;
    delete alt['runHistory'];
    const payload = JSON.stringify(alt);
    store.setItem('rota.save.0', JSON.stringify({ checksum: checksumOf(payload), payload }));
    store.setItem('rota.save.index', '0');

    const p = new RunPersistence(FASSUNG, () => JETZT, store);
    expect(p.save.level).toBe(7);
    expect(p.save.experience).toBe(4321);
  });
});

describe('Ein voller Verlauf', () => {
  it('bleibt bei der Obergrenze und laeuft nicht ueber', () => {
    // 500 Eintraege sind die Grenze aus run-history.ts. Geprueft wird, dass ein
    // Stand dieser Groesse ueberhaupt abgelegt und wieder gelesen werden kann --
    // der Browser-Speicher ist knapp, und ein Stand, der nicht mehr hineinpasst,
    // kostet ALLES und nicht nur sich selbst.
    const store = new MemoryStore();
    const p = new RunPersistence(FASSUNG, () => JETZT, store);
    const voll = Array.from({ length: 500 }, (_, i) => eintrag({ at: JETZT + i }));
    expect(p.persist({ runHistory: voll, progressMarks: [] })).toBe(true);

    const zurueck = new RunPersistence(FASSUNG, () => JETZT, store);
    expect(zurueck.state.runHistory).toHaveLength(500);
    expect(zurueck.status.status).toBe('ok');
  });
});
