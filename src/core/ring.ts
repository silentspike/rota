/**
 * Ringpuffer über einem vorab angelegten `Float64Array` (GAME-DESIGN 18.2).
 *
 * `push` fordert keinen Speicher an — das ist der ganze Zweck. Ist der Puffer voll,
 * überschreibt der neue Wert den ältesten. Das Auslesen als gewöhnliches Array
 * (`toArray`) erzeugt sehr wohl Speicher und gehört deshalb ausschließlich in die
 * Entwicklungs-Werkzeuge, nie in den Bild-Pfad.
 */
export class RingBufferF64 {
  private readonly data: Float64Array;
  /** Nächste Schreibstelle. */
  private writeIndex = 0;
  /** Wie viele Werte gültig sind — höchstens `capacity`. */
  private filled = 0;
  /** Wie viele Werte insgesamt geschrieben wurden, auch überschriebene. */
  private written = 0;

  constructor(capacity: number) {
    if (!Number.isInteger(capacity) || capacity <= 0) {
      throw new RangeError(`Ringpuffer braucht eine positive ganzzahlige Größe, war: ${capacity}`);
    }
    this.data = new Float64Array(capacity);
  }

  get capacity(): number {
    return this.data.length;
  }

  /** Zahl der gültigen Werte. */
  get length(): number {
    return this.filled;
  }

  /** Zahl aller je geschriebenen Werte, einschließlich der überschriebenen. */
  get totalWritten(): number {
    return this.written;
  }

  /** Wie viele Werte überschrieben wurden, ohne gelesen worden zu sein. */
  get overwritten(): number {
    return this.written - this.filled;
  }

  push(value: number): void {
    this.data[this.writeIndex] = value;
    this.writeIndex = this.writeIndex + 1 === this.data.length ? 0 : this.writeIndex + 1;
    if (this.filled < this.data.length) this.filled++;
    this.written++;
  }

  /**
   * Wert an Position `index` in chronologischer Reihenfolge (0 = ältester noch
   * vorhandener Wert). Außerhalb des gültigen Bereichs: `NaN`.
   */
  at(index: number): number {
    if (index < 0 || index >= this.filled) return Number.NaN;
    const start = this.filled === this.data.length ? this.writeIndex : 0;
    const pos = start + index;
    return this.data[pos < this.data.length ? pos : pos - this.data.length] as number;
  }

  clear(): void {
    this.writeIndex = 0;
    this.filled = 0;
    this.written = 0;
  }

  /**
   * Chronologische Kopie als gewöhnliches Array.
   *
   * **Erzeugt Speicher.** Nur für Entwicklungs-Werkzeuge und Tests — niemals im
   * laufenden Bild aufrufen.
   */
  toArray(): number[] {
    const out: number[] = new Array<number>(this.filled);
    for (let i = 0; i < this.filled; i++) out[i] = this.at(i);
    return out;
  }
}
