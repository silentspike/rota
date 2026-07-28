/**
 * Messung der Speicheranforderungen je Bild (GAME-DESIGN 18.2, VERIFICATION § 3.5).
 *
 * Die härteste der fünf Standard-Messungen: Nach dem Aufwärmen darf der laufende
 * Bild-Pfad **nichts** mehr erzeugen. JavaScript kennt keinen Zähler dafür, deshalb
 * wird der Halden-Verbrauch gemessen — jeweils direkt nach einer erzwungenen
 * Aufräumung, damit nicht die Aufräum-Laune des Browsers das Ergebnis bestimmt.
 *
 * Zwei Chrome-Schalter machen die Messung erst aussagekräftig; sie stehen in
 * `.playwright/cli.config.json`:
 *
 *   --js-flags=--expose-gc        stellt `window.gc()` bereit
 *   --enable-precise-memory-info  liefert `performance.memory` auf das Byte genau
 *                                 statt auf 100 KB gerundet
 *
 * Ohne die Schalter meldet `precise: false` bzw. `gcAvailable: false` — dann ist das
 * Ergebnis ein Hinweis, kein Nachweis.
 */

interface MemoryInfo {
  readonly usedJSHeapSize: number;
  readonly totalJSHeapSize: number;
  readonly jsHeapSizeLimit: number;
}

export interface AllocationResult {
  /** Über wie viele Bilder gemessen wurde. */
  readonly frames: number;
  /** Zuwachs der Halde über den Messzeitraum, in Bytes. */
  readonly bytesTotal: number;
  /** Zuwachs je Bild, in Bytes. Erwartet: 0. */
  readonly bytesPerFrame: number;
  /** War `window.gc()` verfügbar? Ohne sie ist das Ergebnis unzuverlässig. */
  readonly gcAvailable: boolean;
  /** Liefert `performance.memory` byte-genaue Werte? */
  readonly precise: boolean;
}

/** So viele Anfragen warten höchstens. Mehr wäre ein Bedienfehler. */
const WAITING_LIMIT = 8;

const NO_RESULT: AllocationResult = {
  frames: 0,
  bytesTotal: 0,
  bytesPerFrame: 0,
  gcAvailable: false,
  precise: false,
};

function readHeap(): number {
  const memory = (performance as Performance & { memory?: MemoryInfo }).memory;
  return memory === undefined ? Number.NaN : memory.usedJSHeapSize;
}

function collectGarbage(): boolean {
  const gc = (globalThis as typeof globalThis & { gc?: () => void }).gc;
  if (typeof gc !== 'function') return false;
  // Zweimal: der erste Durchgang gibt frei, der zweite räumt auf, was dabei
  // erreichbar wurde. Ohne das schwankt der Messwert um einige Kilobyte.
  gc();
  gc();
  return true;
}

/**
 * Ob die Halden-Werte byte-genau sind.
 *
 * Ohne `--enable-precise-memory-info` rundet Chrome auf 100 KB; das erkennt man
 * daran, dass der Wert glatt durch 100000 teilbar ist. Zwei Messungen mit
 * unterschiedlichem Rest belegen dagegen die genaue Auflösung.
 */
function looksPrecise(sample: number): boolean {
  return Number.isFinite(sample) && sample % 100000 !== 0;
}

/**
 * Zählt Bilder und bestimmt den Halden-Zuwachs.
 *
 * Die Messung selbst läuft ausschließlich an den beiden Rändern; dazwischen tut
 * `onFrame` nichts als eine Zahl hochzuzählen.
 */
export class AllocationProbe {
  /**
   * Wird nach jeder erzwungenen Aufräumung gerufen, damit die Schleife die
   * dabei verlorene Zeit nicht als Rückstand des Spiels zählt.
   */
  onCollected: (() => void) | null = null;

  private measuring = false;
  private framesLeft = 0;
  private framesRequested = 0;
  private heapAtStart = 0;
  private gcAvailable = false;
  private precise = false;
  private result: AllocationResult = NO_RESULT;
  private onDone: ((result: AllocationResult) => void) | null = null;
  /**
   * Anfragen, die eintrafen, während schon gemessen wurde.
   *
   * Ohne diese Liste bliebe der Aufrufer hängen: `begin` würde stillschweigend
   * nichts tun, und das Versprechen aus `measureAllocations` würde nie eingelöst.
   * Genau das ist beim Messen zu Issue E1-2 passiert — die automatische Messung
   * lief gerade, und der Abruf lief in eine Zeitüberschreitung.
   */
  private readonly waiting: { frames: number; onDone: ((r: AllocationResult) => void) | null }[] =
    [];

  get lastResult(): AllocationResult {
    return this.result;
  }

  get isMeasuring(): boolean {
    return this.measuring;
  }

  /**
   * Startet eine Messung über `frames` Bilder.
   *
   * Läuft bereits eine, wird die Anfrage hinten angestellt und startet, sobald
   * die laufende fertig ist. Wer nicht warten will, prüft vorher `isMeasuring`.
   */
  begin(frames: number, onDone?: (result: AllocationResult) => void): void {
    if (this.measuring) {
      if (this.waiting.length < WAITING_LIMIT) {
        this.waiting.push({ frames, onDone: onDone ?? null });
      } else if (onDone !== undefined) {
        // Reißleiste gegen unbegrenztes Wachsen: Der Aufrufer bekommt das
        // letzte Ergebnis statt eines Versprechens, das niemand einlöst.
        onDone(this.result);
      }
      return;
    }
    this.startNow(frames, onDone ?? null);
  }

  private startNow(frames: number, onDone: ((result: AllocationResult) => void) | null): void {
    this.gcAvailable = collectGarbage();
    if (this.onCollected !== null) this.onCollected();
    this.heapAtStart = readHeap();
    this.precise = looksPrecise(this.heapAtStart);
    this.framesRequested = frames;
    this.framesLeft = frames;
    this.measuring = true;
    this.onDone = onDone ?? null;
  }

  /** Je Bild aufzurufen. Erzeugt während der Messung nichts. */
  onFrame(): void {
    if (!this.measuring) return;
    this.framesLeft--;
    if (this.framesLeft > 0) return;
    this.finish();
  }

  private finish(): void {
    this.measuring = false;
    const gcAgain = collectGarbage();
    const heapAtEnd = readHeap();
    if (this.onCollected !== null) this.onCollected();
    const bytesTotal = heapAtEnd - this.heapAtStart;
    this.result = {
      frames: this.framesRequested,
      bytesTotal,
      bytesPerFrame: bytesTotal / this.framesRequested,
      gcAvailable: this.gcAvailable && gcAgain,
      precise: this.precise || looksPrecise(heapAtEnd),
    };
    const done = this.onDone;
    this.onDone = null;
    if (done !== null) done(this.result);

    const next = this.waiting.shift();
    if (next !== undefined) this.startNow(next.frames, next.onDone);
  }
}
