/**
 * Die Tastatur ans Spiel anschließen (GAME-DESIGN 8, Design-Gesetze 1 und 2).
 *
 * Vier Regeln, die hier durchgesetzt werden:
 *
 * 1. **Keypress = Trigger.** Ein Tastendruck löst genau eine Aktion aus. Halten
 *    wiederholt nichts — weder über die Wiederholung des Betriebssystems noch,
 *    falls ein Browser sie nicht kennzeichnet, über einen zweiten `keydown`.
 * 2. **Nur einfache Tasten und Umschalt.** Strg, Alt und die Meta-Taste sind keine
 *    Belegungen; Tastenfolgen damit gehören dem Browser und dem Betriebssystem.
 * 3. **Nichts wird vorgemerkt.** Der Druck bekommt den nächsten Takt zugewiesen und
 *    wird dort abgearbeitet — angenommen oder verworfen. Es gibt keinen Zustand,
 *    in dem ein Druck auf eine spätere Gelegenheit wartet.
 * 4. **Zeitstempel des Ereignisses, nicht der Verarbeitung.** `event.timeStamp`
 *    läuft in derselben Uhr wie `performance.now()` und ist die Grundlage der
 *    Messung der Eingabe-Verzögerung.
 */
import { KEYS_PER_LAYER, NO_SLOT, positionForCode, slotFor } from './keys.js';
import type { PressQueue } from './queue.js';

export interface InputRouterOptions {
  readonly queue: PressQueue;
  /** Der Takt, der als nächstes gerechnet wird. */
  readonly nextTickIndex: () => number;
  /** Zahl der bereits gezeichneten Bilder. */
  readonly frameIndex: () => number;
  /**
   * Wird beim allerersten Tastendruck gerufen. Browser halten den Klang-Kontext
   * bis zur ersten Nutzergeste an; das ist die Stelle, ihn aufzuwecken.
   */
  readonly onFirstInput?: () => void;
}

export class InputRouter {
  private readonly queue: PressQueue;
  private readonly nextTickIndex: () => number;
  private readonly frameIndex: () => number;
  private readonly onFirstInput: (() => void) | null;

  /** Gedrückt-Zustand je **Stelle**, nicht je Belegung. */
  private readonly held = new Uint8Array(KEYS_PER_LAYER);

  private readonly boundKeyDown: (event: KeyboardEvent) => void;
  private readonly boundKeyUp: (event: KeyboardEvent) => void;
  private readonly boundBlur: () => void;

  private attached = false;
  private sawInput = false;

  private pressCount = 0;
  private repeatSuppressed = 0;
  private unboundCount = 0;
  private modifierIgnored = 0;

  constructor(options: InputRouterOptions) {
    this.queue = options.queue;
    this.nextTickIndex = options.nextTickIndex;
    this.frameIndex = options.frameIndex;
    this.onFirstInput = options.onFirstInput ?? null;
    this.boundKeyDown = (event: KeyboardEvent): void => {
      this.handleKeyDown(event);
    };
    this.boundKeyUp = (event: KeyboardEvent): void => {
      this.handleKeyUp(event);
    };
    this.boundBlur = (): void => {
      this.held.fill(0);
    };
  }

  /** Angenommene Tastendrücke, die in den Puffer gelangt sind. */
  get pressed(): number {
    return this.pressCount;
  }

  /** Unterdrückte Wiederholungen durch Gedrückthalten. Beleg für Design-Gesetz 1. */
  get suppressedRepeats(): number {
    return this.repeatSuppressed;
  }

  /** Tastendrücke auf nicht belegte Tasten. */
  get unbound(): number {
    return this.unboundCount;
  }

  /** Übergangene Tastenfolgen mit Strg, Alt oder Meta. */
  get modifierIgnores(): number {
    return this.modifierIgnored;
  }

  attach(target: Window = window): void {
    if (this.attached) return;
    this.attached = true;
    target.addEventListener('keydown', this.boundKeyDown);
    target.addEventListener('keyup', this.boundKeyUp);
    // Wird das Fenster gewechselt, während eine Taste unten ist, kommt kein
    // `keyup` mehr. Ohne dieses Zurücksetzen bliebe die Taste für immer gedrückt.
    target.addEventListener('blur', this.boundBlur);
  }

  detach(target: Window = window): void {
    if (!this.attached) return;
    this.attached = false;
    target.removeEventListener('keydown', this.boundKeyDown);
    target.removeEventListener('keyup', this.boundKeyUp);
    target.removeEventListener('blur', this.boundBlur);
    this.held.fill(0);
  }

  /** Ist die Stelle gerade gedrückt? Nur für Tests und Werkzeuge. */
  isHeld(position: number): boolean {
    return this.held[position] === 1;
  }

  private handleKeyDown(event: KeyboardEvent): void {
    if (event.ctrlKey || event.altKey || event.metaKey) {
      if (positionForCode(event.code) !== NO_SLOT) this.modifierIgnored++;
      return;
    }

    const position = positionForCode(event.code);
    if (position === NO_SLOT) {
      this.unboundCount++;
      return;
    }

    // Die Leertaste würde sonst die Seite scrollen.
    event.preventDefault();

    // Zwei voneinander unabhängige Sperren gegen Wiederholung: die Kennzeichnung
    // des Browsers und der eigene Gedrückt-Zustand. Eine allein genügt nicht —
    // `repeat` ist nicht überall zuverlässig gesetzt.
    if (event.repeat || this.held[position] === 1) {
      this.repeatSuppressed++;
      this.held[position] = 1;
      return;
    }
    this.held[position] = 1;

    if (!this.sawInput) {
      this.sawInput = true;
      if (this.onFirstInput !== null) this.onFirstInput();
    }

    const slot = slotFor(event.code, event.shiftKey);
    if (slot === NO_SLOT) return;
    this.queue.push(slot, this.nextTickIndex(), event.timeStamp, this.frameIndex());
    this.pressCount++;
  }

  private handleKeyUp(event: KeyboardEvent): void {
    const position = positionForCode(event.code);
    if (position === NO_SLOT) return;
    this.held[position] = 0;
  }
}
