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
import {
  measureCooldown,
  runReference,
  type CooldownMeasurement,
  type ReferenceOptions,
  type ReferenceResult,
  type RotationName,
} from './reference-run.js';

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
/** Namen der Kampf-Zustände in der Reihenfolge aus `combat.ts`. */
const FIGHT_STATE_NAMES: readonly string[] = ['running', 'won', 'overpressure'];

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

/** Ein Zauber, wie er gerade dasteht. */
export interface SpellState {
  readonly id: string;
  readonly name: string;
  /** Platz auf der Leiste, oder -1. */
  readonly slot: number;
  readonly cooldownMs: number;
  readonly cooldownRemainingMs: number;
  readonly ready: boolean;
  readonly casts: number;
  readonly damage: number;
  /** Restlaufzeit des Effekts über Zeit in ms, 0 wenn keiner läuft. */
  readonly dotRemainingMs: number;
  readonly dotTicks: number;
  readonly lostDotTicks: number;
}

/** Zustand des Kampfes (GAME-DESIGN 19.1: Zustand abfragen und setzen). */
export interface CombatSnapshot {
  readonly timeMs: number;
  readonly state: string;
  readonly damage: number;
  readonly bossName: string;
  readonly bossHp: number;
  readonly bossHpRemaining: number;
  readonly overpressureMs: number;
  readonly endedAtMs: number;
  /** Der Vorrat der Schul-Mechanik — in Schritt 1 die Hitze. */
  readonly resource: number;
  readonly resourceCap: number;
  readonly resourceHighlightAt: number;
  readonly resourceOverflow: number;
  readonly idleMs: number;
  readonly spells: SpellState[];
  /** Ziehungen je Zufallsstrom. In Schritt 1 müssen alle 0 sein. */
  readonly randomDraws: number[];
  /** Was beim Lesen der Spieldaten nicht stimmte. Leer heißt: alles in Ordnung. */
  readonly dataProblems: readonly string[];
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
  /** Zustand des Kampfes. */
  combat(): CombatSnapshot;
  /** Hitze setzen — erspart Wartezeit beim Prüfen (GAME-DESIGN 19.1). */
  setResource(value: number): void;
  /** Kampf von vorn beginnen. */
  resetFight(): void;
  /**
   * Eine Runde nach fester Prioritätenliste durchrechnen und das Ergebnis melden.
   *
   * Das ist der Abgleich gegen `tools/step1-check.py` (GAME-DESIGN 18.4). Reines
   * Werkzeug — im Spiel erscheint keine Referenz-Rotation (GAME-DESIGN 18.5).
   */
  simulate(rotation: RotationName, options?: ReferenceOptions): ReferenceResult;
  /** Abklingzeit über viele Zyklen messen (VERIFICATION § 3.3). */
  measureCooldown(spellId: string, cycles: number): CooldownMeasurement;
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
    combat: () => {
      const combat = app.combat;
      const timeMs = app.loop.timeMs;
      const spells: SpellState[] = [];
      for (let i = 0; i < combat.spells.length; i++) {
        const spell = combat.spells[i];
        if (spell === undefined) continue;
        let slot = -1;
        for (let s = 0; s < SLOT_CODES.length * 2; s++) {
          if (app.session.spellOnSlot(s) === i) {
            slot = s;
            break;
          }
        }
        spells.push({
          id: spell.id,
          name: spell.name,
          slot,
          cooldownMs: spell.cooldownMs,
          cooldownRemainingMs: combat.cooldownRemainingMs(i, timeMs),
          ready: combat.isReady(i, timeMs),
          casts: combat.castCount(i),
          damage: combat.damageOf(i),
          dotRemainingMs: combat.dotRemainingMs(i, timeMs),
          dotTicks: combat.dotTickCount(i),
          lostDotTicks: combat.lostDotTicks(i),
        });
      }
      return {
        timeMs,
        state: FIGHT_STATE_NAMES[combat.fightState] ?? '?',
        damage: combat.damageDealt,
        bossName: combat.boss.name,
        bossHp: combat.boss.hp,
        bossHpRemaining: combat.bossHpRemaining,
        overpressureMs: combat.boss.overpressureMs,
        endedAtMs: combat.endedAt,
        resource: combat.resource,
        resourceCap: combat.school.resource.cap,
        resourceHighlightAt: combat.school.resource.highlightAt,
        resourceOverflow: combat.resourceOverflow,
        idleMs: app.session.idleMs,
        spells,
        randomDraws: combat.random.drawCounts(),
        dataProblems: app.dataProblems,
      };
    },
    setResource: (value: number) => {
      app.combat.setResource(value);
    },
    resetFight: () => {
      app.session.reset();
      app.loop.reset();
    },
    simulate: (rotation, options) => runReference(rotation, options),
    measureCooldown: (spellId, cycles) => measureCooldown(spellId, cycles),
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
