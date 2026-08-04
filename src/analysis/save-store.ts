/**
 * Speicherung mit rotierenden Sicherungen (Issue #83).
 *
 * WARUM UEBERHAUPT SICHERUNGEN
 * ----------------------------
 * Bei 300 Stunden Spielzeit darf ein beschaedigter Spielstand nicht alles
 * kosten. Ein einzelner Platz im Browser-Speicher reicht dafuer nicht: Ein
 * abgebrochener Schreibvorgang, ein voller Speicher oder ein Eingriff von
 * aussen macht ihn unlesbar, und dann ist alles weg.
 *
 * Deshalb fuenf Staende rotierend. Beim Laden wird der juengste HEILE
 * genommen — nicht der juengste. Der Unterschied ist der ganze Zweck.
 *
 * WIE BESCHAEDIGUNG ERKANNT WIRD
 * ------------------------------
 * Ueber dieselbe FNV-1a-Pruefsumme, die der Kampf schon benutzt
 * (src/core/checksum.ts). Eine zweite einzufuehren hiesse, zwei Verfahren fuer
 * denselben Zweck zu pflegen — und das zweite altert unbemerkt.
 *
 * Die Pruefsumme steht NEBEN dem Stand, nicht darin: Sonst muesste man sie beim
 * Rechnen wieder herausschneiden, und jeder Fehler dabei sieht aus wie eine
 * Beschaedigung.
 */
import { mixInt } from '../core/checksum.js';
import { findForbiddenFields, SAVE_SCHEMA, type SaveData } from './save-format.js';

/** Wie viele Staende gehalten werden (Umfang von #83). */
export const SLOT_COUNT = 5;

const KEY_PREFIX = 'rota.save.';
const KEY_INDEX = 'rota.save.index';

/**
 * Der Ausschnitt des Browser-Speichers, den diese Datei braucht.
 *
 * Als eigenes Bild, damit die Rotation ohne Browser pruefbar ist — ein Fehler
 * in der Verdraengungsregel darf sich nicht hinter einer Umgebung verstecken.
 */
export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

/** Ein abgelegter Stand mit seiner Pruefsumme. */
interface StoredSlot {
  readonly checksum: number;
  readonly payload: string;
}

/** FNV-1a ueber einen Text, Zeichen fuer Zeichen. */
export function checksumOf(text: string): number {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash = mixInt(hash, text.charCodeAt(i));
  }
  return hash >>> 0;
}

export type LoadStatus = 'ok' | 'recovered' | 'empty';

export interface LoadResult {
  readonly status: LoadStatus;
  readonly data: SaveData | null;
  /** Wie viele juengere Staende beschaedigt waren. */
  readonly skipped: number;
  /** Welcher Platz gelesen wurde, oder -1. */
  readonly slot: number;
}

export class SaveStore {
  constructor(private readonly store: KeyValueStore) {}

  private slotKey(slot: number): string {
    return `${KEY_PREFIX}${slot}`;
  }

  /** Der Platz, auf den zuletzt geschrieben wurde. -1, wenn noch keiner. */
  private lastSlot(): number {
    const raw = this.store.getItem(KEY_INDEX);
    if (raw === null) return -1;
    const value = Number.parseInt(raw, 10);
    return Number.isInteger(value) && value >= 0 && value < SLOT_COUNT ? value : -1;
  }

  /**
   * Einen Stand ablegen.
   *
   * Prueft VORHER, ob er berechnete Werte enthaelt: Ein Stand, der die Regel
   * verletzt, wird gar nicht erst geschrieben. Spaeter faellt es niemandem
   * mehr auf (siehe save-format.ts).
   */
  save(data: SaveData): number {
    const verboten = findForbiddenFields(data);
    if (verboten.length > 0) {
      throw new Error(
        `Spielstand enthaelt berechnete Werte, das friert die Balance ein: ${verboten.join(', ')}`,
      );
    }
    const slot = (this.lastSlot() + 1) % SLOT_COUNT;
    const payload = JSON.stringify(data);
    const entry: StoredSlot = { checksum: checksumOf(payload), payload };
    this.store.setItem(this.slotKey(slot), JSON.stringify(entry));
    this.store.setItem(KEY_INDEX, String(slot));
    return slot;
  }

  /** Einen einzelnen Platz lesen, oder null bei Beschaedigung. */
  private readSlot(slot: number): SaveData | null {
    const raw = this.store.getItem(this.slotKey(slot));
    if (raw === null) return null;
    let entry: StoredSlot;
    try {
      entry = JSON.parse(raw) as StoredSlot;
    } catch {
      return null;
    }
    if (typeof entry?.payload !== 'string' || typeof entry?.checksum !== 'number') return null;
    if (checksumOf(entry.payload) !== entry.checksum) return null;
    try {
      const data = JSON.parse(entry.payload) as SaveData;
      // Ein Stand aus einer kuenftigen Fassung ist nicht "heil, nur neuer" -
      // wir wissen nicht, was darin steht.
      if (typeof data?.schema !== 'number' || data.schema > SAVE_SCHEMA) return null;
      return data;
    } catch {
      return null;
    }
  }

  /**
   * Den juengsten HEILEN Stand laden.
   *
   * Geht vom zuletzt beschriebenen Platz rueckwaerts durch alle fuenf. Der
   * Rueckgabewert sagt, ob dabei etwas uebersprungen wurde — der Spieler soll
   * erfahren, dass er einen aelteren Stand bekommt, statt es zu bemerken, wenn
   * Fortschritt fehlt.
   */
  load(): LoadResult {
    const last = this.lastSlot();
    if (last < 0) return { status: 'empty', data: null, skipped: 0, slot: -1 };

    for (let back = 0; back < SLOT_COUNT; back++) {
      const slot = (last - back + SLOT_COUNT) % SLOT_COUNT;
      const data = this.readSlot(slot);
      if (data !== null) {
        return {
          status: back === 0 ? 'ok' : 'recovered',
          data,
          skipped: back,
          slot,
        };
      }
    }
    return { status: 'empty', data: null, skipped: SLOT_COUNT, slot: -1 };
  }

  /** Wie viele Plaetze belegt und heil sind - fuer die Anzeige und die Pruefung. */
  healthySlots(): number {
    let count = 0;
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      if (this.readSlot(slot) !== null) count++;
    }
    return count;
  }

  /**
   * Ausfuhr als Text.
   *
   * Derselbe Aufbau wie im Speicher, damit eine Datei nichts anderes ist als
   * ein Stand an einem anderen Ort - Geraetewechsel und Sicherung ohne Server.
   */
  export(): string | null {
    const result = this.load();
    return result.data === null ? null : JSON.stringify(result.data, null, 2);
  }

  /**
   * Einfuhr aus Text.
   *
   * Legt den Stand als neuen Platz ab, statt einen bestehenden zu ersetzen:
   * Wer eine fremde Datei einliest, soll seinen eigenen Stand nicht verlieren,
   * falls sie nicht ist, was er dachte.
   */
  import(text: string): LoadResult {
    let data: SaveData;
    try {
      data = JSON.parse(text) as SaveData;
    } catch {
      return { status: 'empty', data: null, skipped: 0, slot: -1 };
    }
    if (typeof data?.schema !== 'number' || data.schema > SAVE_SCHEMA) {
      return { status: 'empty', data: null, skipped: 0, slot: -1 };
    }
    const slot = this.save(data);
    return { status: 'ok', data, skipped: 0, slot };
  }
}
