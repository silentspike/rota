/**
 * Vorab angelegter Objekt-Pool (GAME-DESIGN 18.2).
 *
 * Alle Objekte entstehen **einmal** im Konstruktor. `acquire` und `release` reichen
 * danach nur noch Indizes durch eine `Int32Array`-Freiliste weiter und fordern keinen
 * Speicher an. Damit bleibt der Bild-Pfad frei von Aufräum-Last, die sonst als
 * Ruckler sichtbar würde.
 *
 * Ist der Pool erschöpft, wird das **älteste noch in Gebrauch befindliche** Objekt
 * wiederverwendet, statt still nichts zu liefern (Vorgabe aus Issue E1-6): Eine
 * fehlende Schadenszahl fällt niemandem auf, eine stehengebliebene sehr wohl.
 */
export class ObjectPool<T> {
  private readonly items: readonly T[];
  /** Freie Plätze als Stapel: `free[0 .. freeCount-1]` sind verfügbar. */
  private readonly free: Int32Array;
  private freeCount: number;
  /** Belegte Plätze in der Reihenfolge ihrer Vergabe (Warteschlange). */
  private readonly used: Int32Array;
  private usedHead = 0;
  private usedCount = 0;
  /** Für jeden Platz: liegt er gerade in `used`? */
  private readonly inUse: Uint8Array;
  private recycled = 0;

  /**
   * @param capacity Zahl der vorab angelegten Objekte.
   * @param create Erzeugt ein Objekt. Wird ausschließlich hier aufgerufen.
   * @param reset Setzt ein Objekt vor der Wiedervergabe zurück.
   */
  constructor(
    capacity: number,
    create: (index: number) => T,
    private readonly reset: (item: T) => void,
  ) {
    if (!Number.isInteger(capacity) || capacity <= 0) {
      throw new RangeError(`Pool braucht eine positive ganzzahlige Größe, war: ${capacity}`);
    }
    const items: T[] = new Array<T>(capacity);
    this.free = new Int32Array(capacity);
    this.used = new Int32Array(capacity);
    this.inUse = new Uint8Array(capacity);
    for (let i = 0; i < capacity; i++) {
      items[i] = create(i);
      // Freiliste rückwärts füllen, damit zuerst Index 0 vergeben wird.
      this.free[i] = capacity - 1 - i;
    }
    this.items = items;
    this.freeCount = capacity;
  }

  get capacity(): number {
    return this.items.length;
  }

  get activeCount(): number {
    return this.usedCount;
  }

  /** Wie oft ein noch belegtes Objekt wiederverwendet werden musste. */
  get recycleCount(): number {
    return this.recycled;
  }

  /**
   * Liefert ein zurückgesetztes Objekt. Fordert keinen Speicher an und liefert
   * immer ein Objekt — notfalls das älteste noch belegte.
   */
  acquire(): T {
    let index: number;
    if (this.freeCount > 0) {
      this.freeCount--;
      index = this.free[this.freeCount] as number;
    } else {
      // Pool erschöpft: ältestes belegtes Objekt zurückholen.
      index = this.used[this.usedHead] as number;
      this.usedHead = this.usedHead + 1 === this.used.length ? 0 : this.usedHead + 1;
      this.usedCount--;
      this.inUse[index] = 0;
      this.recycled++;
    }
    const item = this.items[index] as T;
    this.reset(item);
    this.inUse[index] = 1;
    const tail = (this.usedHead + this.usedCount) % this.used.length;
    this.used[tail] = index;
    this.usedCount++;
    return item;
  }

  /**
   * Gibt ein Objekt über seinen Index frei. Ein bereits freier Index wird
   * stillschweigend übergangen, damit doppeltes Freigeben nichts kaputtmacht.
   */
  release(index: number): void {
    if (index < 0 || index >= this.items.length) return;
    if (this.inUse[index] === 0) return;
    this.inUse[index] = 0;
    // Aus der Warteschlange herausnehmen: verbleibende Einträge nachrücken lassen.
    let read = this.usedHead;
    let write = this.usedHead;
    const size = this.used.length;
    for (let i = 0; i < this.usedCount; i++) {
      const value = this.used[read] as number;
      if (value !== index) {
        this.used[write] = value;
        write = write + 1 === size ? 0 : write + 1;
      }
      read = read + 1 === size ? 0 : read + 1;
    }
    this.usedCount--;
    this.free[this.freeCount] = index;
    this.freeCount++;
  }

  /** Objekt an einem Platz — für Verbraucher, die alle aktiven durchlaufen. */
  itemAt(index: number): T {
    return this.items[index] as T;
  }

  /** Ist der Platz gerade vergeben? */
  isActive(index: number): boolean {
    return this.inUse[index] === 1;
  }

  releaseAll(): void {
    for (let i = 0; i < this.items.length; i++) {
      this.inUse[i] = 0;
      this.free[i] = this.items.length - 1 - i;
    }
    this.freeCount = this.items.length;
    this.usedHead = 0;
    this.usedCount = 0;
  }
}
