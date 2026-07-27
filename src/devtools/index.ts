/**
 * Entwicklungs-Werkzeuge (GAME-DESIGN 19.1).
 *
 * Nur über die Browser-Konsole ansprechbar — keine Oberfläche, keine Knöpfe,
 * nichts im Bild. Der Auftraggeber sieht beim Testen ausschließlich das Spiel.
 *
 * Der gesamte Inhalt dieser Datei steht hinter `__ROTA_DEVTOOLS__` und wird in
 * der Auslieferungs-Fassung vollständig herausgeschnitten (19.3). Geprüft im
 * CI-Job "build" per grep über `dist/`.
 *
 * Umfang zum jetzigen Stand (Issue E1-1): Leistungs-Messwerte und Zustandsabfrage.
 * Zeitsteuerung, Aufzeichnen, Prüfsumme und Kampfprotokoll folgen mit E1-8.
 */
import type { App } from '../core/app.js';
import { PerfCounters, summarize, type SeriesSummary } from '../core/metrics.js';
import { AllocationProbe, type AllocationResult } from './allocations.js';

/** Bilder, die vor der ersten Speicher-Messung verstreichen (Aufwärmen). */
const WARMUP_FRAMES = 240;
/** Länge einer Speicher-Messung in Bildern. */
const ALLOCATION_WINDOW_FRAMES = 600;
/** Abstand zwischen zwei Speicher-Messungen in Bildern. */
const ALLOCATION_INTERVAL_FRAMES = 1800;

/** Die Bildzeit-Schwelle aus GAME-DESIGN 20.2 / VERIFICATION § 3.1. */
const FRAME_BUDGET_MS = 16.7;

export interface PerfSnapshot {
  /** Arbeitszeit je Bild in ms: Logik-Takte plus Zeichnen. Das Maß für Ruckelfreiheit. */
  readonly frameTimes: number[];
  /** Abstand zwischen zwei Bildern in ms. Vom Bildschirm bestimmt, nicht vom Spiel. */
  readonly frameIntervals: number[];
  /** Logik-Takte je Bild. */
  readonly ticksPerFrame: number[];
  /** Eingabe-Verzögerung je Tastendruck in ms (wird ab E1-2 gefüllt). */
  readonly inputLatency: number[];
  /** Speicheranforderung je Bild in Bytes aus der letzten Messung. Erwartet: 0. */
  readonly allocationsPerFrame: number;
  readonly allocations: AllocationResult;
  readonly frameTimeSummary: SeriesSummary;
  readonly frameIntervalSummary: SeriesSummary;
  readonly frames: number;
  readonly ticks: number;
  readonly droppedTicks: number;
  readonly simTimeMs: number;
}

export interface StateSnapshot {
  readonly running: boolean;
  /** Ganzzahlige Kampfzeit in ms. */
  readonly timeMs: number;
  readonly ticks: number;
  readonly frames: number;
  readonly droppedTicks: number;
  /** Anteil des angebrochenen Takts, in [0, 1). */
  readonly alpha: number;
}

export interface DevTools {
  /** Kennung, an der der CI-Job "build" prüft, dass nichts durchrutscht. */
  readonly marker: '__ROTA_DEVTOOLS__';
  version(): string;
  /** Leistungs-Messwerte (VERIFICATION § 3). */
  perf(): PerfSnapshot;
  /** Zustand des Zeitgerüsts. */
  state(): StateSnapshot;
  /** Setzt alle Messreihen zurück. */
  resetPerf(): void;
  /** Misst den Halden-Zuwachs über `frames` Bilder. */
  measureAllocations(frames?: number): Promise<AllocationResult>;
}

export function installDevTools(app: App): void {
  if (!__ROTA_DEVTOOLS__) return;

  const perf = new PerfCounters();
  const probe = new AllocationProbe();
  let framesSinceMeasurement = 0;
  let warmupLeft = WARMUP_FRAMES;

  app.loop.onFrameMeasured = (frameWorkMs, frameIntervalMs, ticksThisFrame): void => {
    perf.recordFrame(frameWorkMs, frameIntervalMs, ticksThisFrame);
    probe.onFrame();
    if (probe.isMeasuring) return;
    if (warmupLeft > 0) {
      warmupLeft--;
      if (warmupLeft === 0) probe.begin(ALLOCATION_WINDOW_FRAMES);
      return;
    }
    framesSinceMeasurement++;
    if (framesSinceMeasurement >= ALLOCATION_INTERVAL_FRAMES) {
      framesSinceMeasurement = 0;
      probe.begin(ALLOCATION_WINDOW_FRAMES);
    }
  };

  const tools: DevTools = {
    marker: '__ROTA_DEVTOOLS__',
    version: () => '0.1.0',
    perf: () => ({
      frameTimes: perf.frameWork.toArray(),
      frameIntervals: perf.frameInterval.toArray(),
      ticksPerFrame: perf.tickWork.toArray(),
      inputLatency: perf.inputLatency.toArray(),
      allocationsPerFrame: probe.lastResult.bytesPerFrame,
      allocations: probe.lastResult,
      frameTimeSummary: summarize(perf.frameWork, FRAME_BUDGET_MS),
      frameIntervalSummary: summarize(perf.frameInterval, FRAME_BUDGET_MS),
      frames: perf.frameCount,
      ticks: perf.tickCount,
      droppedTicks: app.loop.droppedTickCount,
      simTimeMs: app.loop.timeMs,
    }),
    state: () => ({
      running: app.loop.isRunning,
      timeMs: app.loop.timeMs,
      ticks: app.loop.tickCount,
      frames: app.loop.frameCount,
      droppedTicks: app.loop.droppedTickCount,
      alpha: app.loop.alpha,
    }),
    resetPerf: () => {
      perf.reset();
      framesSinceMeasurement = 0;
      warmupLeft = WARMUP_FRAMES;
    },
    measureAllocations: (frames = ALLOCATION_WINDOW_FRAMES) =>
      new Promise<AllocationResult>((resolve) => {
        probe.begin(frames, resolve);
      }),
  };

  (window as unknown as { __rota: DevTools }).__rota = tools;
  console.info('[rota] Entwicklungs-Werkzeuge aktiv: window.__rota');
}
