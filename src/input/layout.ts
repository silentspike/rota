/**
 * Beschriftung der Tasten zur Laufzeit ermitteln (GAME-DESIGN 8.1).
 *
 * Die Belegung hängt an der **Stelle**, die Beschriftung am **Layout des Spielers**.
 * Auf der Stelle `KeyZ` steht auf einer deutschen Tastatur ein `Y`, auf einer
 * englischen ein `Z`. Das Symbol auf der Leiste muss zeigen, was der Spieler vor
 * sich sieht — sonst sucht er die falsche Taste.
 *
 * Quelle ist die Keyboard-Map des Browsers (`navigator.keyboard.getLayoutMap()`).
 * Sie kennt genau diese Zuordnung. Wo es sie nicht gibt (derzeit Firefox und
 * Safari), bleibt die US-Beschriftung stehen — sie ist die Grundlage der
 * Kennungen und damit die einzig sinnvolle Annahme.
 */
import { OVERDRIVE_CODE, SLOT_CODES, SPACE_POSITION } from './keys.js';

/** Beschriftung ohne Layout-Auskunft. Entspricht der US-Tastatur. */
export const FALLBACK_LABELS: readonly string[] = [
  'Q',
  'W',
  'E',
  'R',
  'A',
  'S',
  'D',
  'F',
  'Z',
  'X',
  'C',
  'V',
  'SPACE',
];

/** Beschriftung der Overdrive-Taste ohne Layout-Auskunft (#1177). */
export const FALLBACK_OVERDRIVE_LABEL = 'G';

/** Woher die Beschriftung stammt — für die Entwicklungs-Werkzeuge. */
export type LabelSource = 'fallback' | 'keyboard-map';

interface KeyboardLayoutMapLike {
  get(code: string): string | undefined;
}

interface KeyboardLike {
  getLayoutMap(): Promise<KeyboardLayoutMapLike>;
}

function keyboardApi(): KeyboardLike | undefined {
  const nav = navigator as Navigator & { keyboard?: KeyboardLike };
  const keyboard = nav.keyboard;
  return keyboard !== undefined && typeof keyboard.getLayoutMap === 'function'
    ? keyboard
    : undefined;
}

export class KeyLabels {
  private readonly labels: string[] = [...FALLBACK_LABELS];
  private overdriveText = FALLBACK_OVERDRIVE_LABEL;
  private origin: LabelSource = 'fallback';

  /** Beschriftung einer Stelle (0 bis 12). */
  label(position: number): string {
    return this.labels[position] ?? '?';
  }

  /**
   * Beschriftung der Overdrive-Taste (#1177) — was auf der Taste rechts neben
   * F steht. Auf einer Dvorak-Tastatur ist das `I`, nicht `G`.
   */
  get overdriveLabel(): string {
    return this.overdriveText;
  }

  get source(): LabelSource {
    return this.origin;
  }

  /**
   * Beschriftung aus dem Layout des Spielers holen.
   *
   * Läuft einmal beim Start und kann jederzeit erneut aufgerufen werden — das
   * Layout lässt sich im Betriebssystem umschalten, ohne die Seite neu zu laden.
   * Schlägt der Abruf fehl, bleibt die bisherige Beschriftung stehen.
   */
  async refresh(): Promise<LabelSource> {
    const keyboard = keyboardApi();
    if (keyboard === undefined) return this.origin;
    try {
      const map = await keyboard.getLayoutMap();
      for (let position = 0; position < SLOT_CODES.length; position++) {
        // Die Leertaste liefert ein Leerzeichen; ausgeschrieben ist sie lesbarer.
        if (position === SPACE_POSITION) {
          this.labels[position] = 'SPACE';
          continue;
        }
        const raw = map.get(SLOT_CODES[position] as string);
        if (raw !== undefined && raw.length > 0) {
          this.labels[position] = raw.toUpperCase();
        }
      }
      const overdrive = map.get(OVERDRIVE_CODE);
      if (overdrive !== undefined && overdrive.length > 0) {
        this.overdriveText = overdrive.toUpperCase();
      }
      this.origin = 'keyboard-map';
    } catch {
      // Kein Grund, das Spiel anzuhalten: Die US-Beschriftung bleibt stehen.
    }
    return this.origin;
  }
}
