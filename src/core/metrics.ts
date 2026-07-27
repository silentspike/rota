/**
 * Leistungs-Messwerte (GAME-DESIGN 19.1, geprüft nach docs/VERIFICATION.md § 3).
 *
 * Läuft im Auslieferungsstand nicht mit: Die Werte werden nur erhoben, wenn die
 * Entwicklungs-Werkzeuge aktiv sind. Das Erheben selbst darf keinen Speicher
 * anfordern — deshalb Ringpuffer über `Float64Array` und Zähler als Zahlen.
 *
 * **Zwei Bildzeiten, bewusst getrennt:**
 * - `frameWork` ist die Zeit, die ein Bild in Logik-Takt und Zeichnen verbringt.
 *   Das ist der Wert, den das Spiel beeinflussen kann, und damit das Maß für
 *   Ruckelfreiheit.
 * - `frameInterval` ist der Abstand zwischen zwei Bildern. Er wird vom Bildschirm
 *   und vom Fenstersystem bestimmt und liegt bei 60 Hz naturgemäß bei ~16,7 ms.
 */
import { RingBufferF64 } from './ring.js';

/** Zehn Sekunden bei 60 Hz sind 600 Bilder; 1024 lässt Luft für höhere Bildraten. */
export const FRAME_HISTORY = 1024;

export class PerfCounters {
  readonly frameWork = new RingBufferF64(FRAME_HISTORY);
  readonly frameInterval = new RingBufferF64(FRAME_HISTORY);
  readonly tickWork = new RingBufferF64(FRAME_HISTORY);
  readonly inputLatency = new RingBufferF64(FRAME_HISTORY);
  /**
   * Wie viele Bilder zwischen `keydown` und dem Bild lagen, das die Wirkung zeigt.
   *
   * Das ist das eigentliche Kriterium aus GAME-DESIGN 20.2 („unter einem Bild"):
   * Der Wert muss **1** sein. Die absolute Verzögerung enthält zusätzlich die
   * Wartezeit auf das nächste Bild des Bildschirms, die kein Spiel unterbieten kann.
   */
  readonly inputFrameDelay = new RingBufferF64(FRAME_HISTORY);

  private frames = 0;
  private ticks = 0;
  private worstFrameWorkMs = 0;

  /** Wird je Bild aufgerufen. Erzeugt nichts. */
  recordFrame(frameWorkMs: number, frameIntervalMs: number, ticksThisFrame: number): void {
    this.frameWork.push(frameWorkMs);
    this.frameInterval.push(frameIntervalMs);
    this.tickWork.push(ticksThisFrame);
    this.frames++;
    this.ticks += ticksThisFrame;
    if (frameWorkMs > this.worstFrameWorkMs) this.worstFrameWorkMs = frameWorkMs;
  }

  /** Eingabe-Verzögerung von `keydown` bis zum Zeichnen des Bildes mit der Wirkung. */
  recordInputLatency(latencyMs: number, frameDelay: number): void {
    this.inputLatency.push(latencyMs);
    this.inputFrameDelay.push(frameDelay);
  }

  get frameCount(): number {
    return this.frames;
  }

  get tickCount(): number {
    return this.ticks;
  }

  get worstFrameMs(): number {
    return this.worstFrameWorkMs;
  }

  reset(): void {
    this.frameWork.clear();
    this.frameInterval.clear();
    this.tickWork.clear();
    this.inputLatency.clear();
    this.inputFrameDelay.clear();
    this.frames = 0;
    this.ticks = 0;
    this.worstFrameWorkMs = 0;
  }
}

/** Kennzahlen einer Messreihe. Wird nur von den Werkzeugen erzeugt, nie im Bild. */
export interface SeriesSummary {
  readonly count: number;
  readonly min: number;
  readonly max: number;
  readonly mean: number;
  readonly p99: number;
  /** Wie viele Werte über der Schwelle lagen. */
  readonly over: number;
  readonly threshold: number;
}

/**
 * Fasst die letzten `sampleCount` Werte zusammen.
 *
 * **Erzeugt Speicher** — ausschließlich für die Entwicklungs-Werkzeuge gedacht.
 */
export function summarize(
  buffer: RingBufferF64,
  threshold: number,
  sampleCount = buffer.length,
): SeriesSummary {
  const available = Math.min(sampleCount, buffer.length);
  if (available === 0) {
    return { count: 0, min: 0, max: 0, mean: 0, p99: 0, over: 0, threshold };
  }
  const first = buffer.length - available;
  const values: number[] = new Array<number>(available);
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  let sum = 0;
  let over = 0;
  for (let i = 0; i < available; i++) {
    const v = buffer.at(first + i);
    values[i] = v;
    if (v < min) min = v;
    if (v > max) max = v;
    sum += v;
    if (v > threshold) over++;
  }
  values.sort((a, b) => a - b);
  const p99Index = Math.min(available - 1, Math.floor(available * 0.99));
  return {
    count: available,
    min,
    max,
    mean: sum / available,
    p99: values[p99Index] as number,
    over,
    threshold,
  };
}
