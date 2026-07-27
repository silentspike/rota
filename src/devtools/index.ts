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
import { KEYS_PER_LAYER, SLOT_CODES } from '../input/keys.js';
import { AllocationProbe, type AllocationResult } from './allocations.js';

/** Bilder, die vor der ersten Speicher-Messung verstreichen (Aufwärmen). */
const WARMUP_FRAMES = 240;
/** Länge einer Speicher-Messung in Bildern. */
const ALLOCATION_WINDOW_FRAMES = 600;
/** Abstand zwischen zwei Speicher-Messungen in Bildern. */
const ALLOCATION_INTERVAL_FRAMES = 1800;

/** Die Bildzeit-Schwelle aus GAME-DESIGN 20.2 / VERIFICATION § 3.1. */
const FRAME_BUDGET_MS = 16.7;
/** Die Schwelle der Eingabe-Verzögerung aus GAME-DESIGN 20.2 / VERIFICATION § 3.2. */
const INPUT_BUDGET_MS = 16;

export interface PerfSnapshot {
  /** Arbeitszeit je Bild in ms: Logik-Takte plus Zeichnen. Das Maß für Ruckelfreiheit. */
  readonly frameTimes: number[];
  /** Abstand zwischen zwei Bildern in ms. Vom Bildschirm bestimmt, nicht vom Spiel. */
  readonly frameIntervals: number[];
  /** Logik-Takte je Bild. */
  readonly ticksPerFrame: number[];
  /** Eingabe-Verzögerung je Tastendruck in ms, von `keydown` bis zum Zeichnen. */
  readonly inputLatency: number[];
  /** Bilder zwischen `keydown` und dem Bild mit der Wirkung. Muss 1 sein. */
  readonly inputFrameDelay: number[];
  readonly inputLatencySummary: SeriesSummary;
  readonly inputFrameDelaySummary: SeriesSummary;
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
  /** Restzeit der globalen Wartezeit in ms. */
  readonly gcdRemainingMs: number;
  readonly gcdLengthMs: number;
}

/** Zustand der Eingabe (GAME-DESIGN 8). */
export interface InputSnapshot {
  /** Angenommene Tastendrücke — sie haben die globale Wartezeit ausgelöst. */
  readonly accepted: number;
  /** Verworfene Tastendrücke — sie fielen in die laufende Wartezeit. */
  readonly rejected: number;
  /** Tastendrücke, die in den Puffer gelangt sind. */
  readonly pressed: number;
  /** Unterdrückte Wiederholungen durch Gedrückthalten. Muss dem Halten entsprechen. */
  readonly suppressedRepeats: number;
  /** Tastendrücke auf nicht belegte Tasten. */
  readonly unbound: number;
  /** Übergangene Tastenfolgen mit Strg, Alt oder Meta. */
  readonly modifierIgnores: number;
  /** Tastendrücke, die der Puffer nicht mehr fassen konnte. Muss 0 sein. */
  readonly lost: number;
  /** Noch nicht abgearbeitete Drücke. Zwischen zwei Takten höchstens wenige. */
  readonly queued: number;
  readonly lastPressedSlot: number;
  /** Woher die Beschriftung stammt. */
  readonly labelSource: string;
  /** Beschriftung der dreizehn Stellen in der Reihenfolge der Leiste. */
  readonly labels: string[];
  /** Tastenkennungen derselben dreizehn Stellen. */
  readonly codes: string[];
  readonly audioReady: boolean;
  readonly audioRequests: number;
  readonly audioPlays: number;
  readonly audioMissing: number;
}

export interface DevTools {
  /** Kennung, an der der CI-Job "build" prüft, dass nichts durchrutscht. */
  readonly marker: '__ROTA_DEVTOOLS__';
  version(): string;
  /** Leistungs-Messwerte (VERIFICATION § 3). */
  perf(): PerfSnapshot;
  /** Zustand des Zeitgerüsts. */
  state(): StateSnapshot;
  /** Zustand der Eingabe (GAME-DESIGN 8). */
  input(): InputSnapshot;
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

    // Direkt nach dem Zeichnen: Was der Takt sichtbar gemacht hat, ist jetzt im
    // Bild. Genau bis hierhin misst VERIFICATION § 3.2.
    const trace = app.session.latency;
    if (trace.size > 0) {
      const shownAtMs = performance.now();
      const frameIndex = app.loop.frameCount;
      while (trace.size > 0) {
        perf.recordInputLatency(shownAtMs - trace.frontStamp(), frameIndex - trace.frontFrame());
        trace.shift();
      }
    }

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
      inputFrameDelay: perf.inputFrameDelay.toArray(),
      inputLatencySummary: summarize(perf.inputLatency, INPUT_BUDGET_MS),
      inputFrameDelaySummary: summarize(perf.inputFrameDelay, 1),
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
      gcdRemainingMs: app.session.gcd.remainingMs(app.loop.timeMs),
      gcdLengthMs: app.session.gcd.lengthMs,
    }),
    input: () => {
      const metrics = app.audio.audio?.metrics();
      const labels: string[] = [];
      const codes: string[] = [];
      for (let position = 0; position < KEYS_PER_LAYER; position++) {
        labels.push(app.labels.label(position));
        codes.push(SLOT_CODES[position] as string);
      }
      return {
        accepted: app.session.acceptedCount,
        rejected: app.session.rejectedCount,
        pressed: app.input.pressed,
        suppressedRepeats: app.input.suppressedRepeats,
        unbound: app.input.unbound,
        modifierIgnores: app.input.modifierIgnores,
        lost: app.pressQueue.lostCount,
        queued: app.pressQueue.size,
        lastPressedSlot: app.session.lastPressedSlot,
        labelSource: app.labels.source,
        labels,
        codes,
        audioReady: app.audio.isReady,
        audioRequests: app.audio.requestCount,
        audioPlays: metrics?.plays ?? 0,
        audioMissing: metrics?.missing ?? 0,
      };
    },
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
