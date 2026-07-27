/**
 * Übergabe der Tastendrücke vom Browser an den Logik-Takt.
 *
 * **Das ist ausdrücklich keine Eingabe-Warteschlange im Sinne von GAME-DESIGN 21.1.**
 * Vorgemerkt wird nichts: Jeder Eintrag wird in dem Takt abgearbeitet, dem er beim
 * Eintreffen zugeordnet wurde, und dort entweder angenommen oder verworfen. Was der
 * Puffer überbrückt, ist allein die Strecke zwischen dem Ereignis des Browsers und
 * dem nächsten Logik-Takt — höchstens ein Takt, nie mehr.
 *
 * Vorab angelegte Zahlenfelder statt Objekten (GAME-DESIGN 18.2): Ein Tastendruck
 * darf keine Speicheranforderung auslösen.
 */

/** Ein Mensch schafft keine 64 Anschläge zwischen zwei Takten. */
export const PRESS_QUEUE_CAPACITY = 64;

export class PressQueue {
  private readonly slots: Int32Array;
  private readonly tickIndices: Int32Array;
  private readonly stamps: Float64Array;
  private readonly frames: Int32Array;
  private head = 0;
  private count = 0;
  private lost = 0;

  constructor(capacity: number = PRESS_QUEUE_CAPACITY) {
    if (!Number.isInteger(capacity) || capacity <= 0) {
      throw new RangeError(`Puffergröße muss positiv und ganzzahlig sein, war: ${capacity}`);
    }
    this.slots = new Int32Array(capacity);
    this.tickIndices = new Int32Array(capacity);
    this.stamps = new Float64Array(capacity);
    this.frames = new Int32Array(capacity);
  }

  get capacity(): number {
    return this.slots.length;
  }

  get size(): number {
    return this.count;
  }

  /**
   * Wie viele Tastendrücke verloren gingen, weil der Puffer voll war.
   *
   * Muss 0 bleiben. Ein Wert darüber wird gemeldet statt still hingenommen — ein
   * verschluckter Anschlag ist der ärgerlichste denkbare Fehler in einem Spiel,
   * dessen ganzer Inhalt das Drücken ist.
   */
  get lostCount(): number {
    return this.lost;
  }

  /**
   * @param tickIndex Der Takt, in dem der Druck wirken soll — der nächste, der
   *   gerechnet wird.
   * @param stampMs Zeitstempel des `keydown` in der Uhr von `performance.now()`.
   * @param frameIndex Zahl der bis dahin gezeichneten Bilder.
   * @returns false, wenn der Puffer voll war.
   */
  push(slot: number, tickIndex: number, stampMs: number, frameIndex: number): boolean {
    if (this.count === this.slots.length) {
      this.lost++;
      return false;
    }
    const at = (this.head + this.count) % this.slots.length;
    this.slots[at] = slot;
    this.tickIndices[at] = tickIndex;
    this.stamps[at] = stampMs;
    this.frames[at] = frameIndex;
    this.count++;
    return true;
  }

  frontSlot(): number {
    return this.slots[this.head] as number;
  }

  frontTick(): number {
    return this.tickIndices[this.head] as number;
  }

  frontStamp(): number {
    return this.stamps[this.head] as number;
  }

  frontFrame(): number {
    return this.frames[this.head] as number;
  }

  shift(): void {
    if (this.count === 0) return;
    this.head = this.head + 1 === this.slots.length ? 0 : this.head + 1;
    this.count--;
  }

  clear(): void {
    this.head = 0;
    this.count = 0;
  }
}
