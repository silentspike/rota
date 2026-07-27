/**
 * Spur für die Messung der Eingabe-Verzögerung (VERIFICATION § 3.2).
 *
 * Gemessen wird von `keydown` bis zum **Zeichnen des Bildes, in dem die Wirkung
 * sichtbar ist** — nicht bis zur Verarbeitung im Logik-Takt. Deshalb legt der Takt
 * hier ab, was er sichtbar gemacht hat, und die Entwicklungs-Werkzeuge holen es
 * nach dem Zeichnen wieder ab.
 *
 * Der Kampf schreibt immer, auch im Auslieferungsstand: Das kostet zwei Zahlen in
 * vorab angelegten Feldern und hält die Messung frei von Sonderpfaden. Holt sie
 * niemand ab, überschreiben neue Einträge die ältesten.
 */

export const LATENCY_TRACE_CAPACITY = 64;

export class LatencyTrace {
  private readonly stamps: Float64Array;
  private readonly frames: Int32Array;
  private head = 0;
  private count = 0;

  constructor(capacity: number = LATENCY_TRACE_CAPACITY) {
    this.stamps = new Float64Array(capacity);
    this.frames = new Int32Array(capacity);
  }

  get size(): number {
    return this.count;
  }

  /**
   * @param stampMs Zeitstempel des `keydown`.
   * @param frameIndex Zahl der gezeichneten Bilder zum Zeitpunkt des `keydown`.
   */
  push(stampMs: number, frameIndex: number): void {
    const at = (this.head + this.count) % this.stamps.length;
    this.stamps[at] = stampMs;
    this.frames[at] = frameIndex;
    if (this.count === this.stamps.length) {
      // Voll: Der älteste Eintrag fällt heraus.
      this.head = this.head + 1 === this.stamps.length ? 0 : this.head + 1;
    } else {
      this.count++;
    }
  }

  frontStamp(): number {
    return this.stamps[this.head] as number;
  }

  frontFrame(): number {
    return this.frames[this.head] as number;
  }

  shift(): void {
    if (this.count === 0) return;
    this.head = this.head + 1 === this.stamps.length ? 0 : this.head + 1;
    this.count--;
  }

  clear(): void {
    this.head = 0;
    this.count = 0;
  }
}
