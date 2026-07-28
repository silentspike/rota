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
import { TICK_MS } from '../core/loop.js';
import { PerfCounters, summarize, type SeriesSummary } from '../core/metrics.js';
import { KEYS_PER_LAYER, SLOT_CODES, SLOT_COUNT } from '../input/keys.js';
import { NO_SPELL } from '../core/session.js';
import { HIT_DIRECT, HIT_DOT } from '../core/hits.js';
import { combatChecksum } from '../core/checksum.js';
import { EVENT_NAMES, type LogEntry } from '../core/log.js';
import { recordingNames, recordings, replayRecording, type ReplayResult } from './recording.js';
import type { RunSummary } from '../core/summary.js';
import { OUTCOME_NAMES, PHASE_FIGHTING, PHASE_NAMES } from '../core/phases.js';
import { slotRect, sweepHandAngle, type Rect } from '../render/spellbar.js';
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
  /** Wie oft Zeit für eine Messung verworfen wurde. Nur Werkzeug-Bedingt. */
  readonly discardedForMeasurement: number;
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
  /** Phase des Kampfablaufs (GAME-DESIGN 10). */
  readonly phase: string;
  readonly outcome: string;
  /** Restzeit des Countdowns in ms. */
  readonly countdownRemainingMs: number;
  /** Verstrichene KAMPFzeit in ms — die Uhr, die der Spieler sieht. */
  readonly fightElapsedMs: number;
  readonly overpressureRemainingMs: number;
  /** Zeitpunkt auf der durchlaufenden Uhr, zu dem der Kampf begann. */
  readonly fightStartedAtMs: number;
  readonly endedAtMs: number;
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

/**
 * Ein Platz der Zauberleiste, wie er gerade gezeichnet wird.
 *
 * Die Maße stehen in **Geräte-Pixeln** auf der Zeichenfläche, nicht in
 * Entwurfs-Koordinaten: So lässt sich mit `getImageData` nachmessen, was
 * tatsächlich im Bild steht, statt dem Spiel zu glauben.
 */
export interface HudSlotSnapshot {
  readonly slot: number;
  readonly spellId: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly centerX: number;
  readonly centerY: number;
  readonly ready: boolean;
  readonly cooldownMs: number;
  readonly cooldownRemainingMs: number;
  /** Restanteil der Abklingzeit, 0 bis 1 — der Winkel der Wischblende folgt ihm. */
  readonly sweepShare: number;
  /** Winkel der Blenden-Kante in Radiant, 0 = drei Uhr, wachsend im Uhrzeigersinn. */
  readonly sweepHandAngle: number;
  readonly gcdRemainingMs: number;
  readonly gcdShare: number;
  readonly dotRemainingMs: number;
  readonly dotDurationMs: number;
}

export interface HudSnapshot {
  readonly canvasWidth: number;
  readonly canvasHeight: number;
  readonly scale: number;
  readonly offsetX: number;
  readonly offsetY: number;
  /** Zeichenvorgänge insgesamt. Muss der Zahl der Bilder entsprechen. */
  readonly drawPasses: number;
  readonly frames: number;
  readonly slots: HudSlotSnapshot[];
}

/** Zustand der Trefferwucht (GAME-DESIGN 9). */
export interface ImpactSnapshot {
  /** Plätze im vorab angelegten Pool. */
  readonly poolCapacity: number;
  /** Zahlen, die gerade unterwegs sind. */
  readonly activeCount: number;
  /** Wie oft eine noch sichtbare Zahl wiederverwendet werden musste. */
  readonly recycleCount: number;
  /** Wie viele Zahlen insgesamt gestartet sind. */
  readonly spawnCount: number;
  /** Aktuelle Auslenkung des Bild-Ruckelns in Entwurfs-Pixeln. */
  readonly shakeOffsetPx: number;
  /** Aktuelle Stärke des Aufblitzens, 0 bis 1. */
  readonly flashStrength: number;
}

/** Woraus sich der Schaden eines Treffers zusammensetzt (GAME-DESIGN 6.3, 19.1). */
export interface DamageBreakdown {
  readonly spellId: string;
  readonly baseDamage: number;
  /** Entladeschaden aus dem Vorrat, falls der Zauber einer ist. */
  readonly spenderDamage: number;
  readonly resource: number;
  readonly bonusApplied: boolean;
  /** Die Faktoren in der Reihenfolge, in der sie angewandt werden. */
  readonly factors: { readonly name: string; readonly value: number }[];
  readonly total: number;
}

/** Kennzahlen des Kampfprotokolls. */
export interface LogStats {
  readonly entries: number;
  readonly capacity: number;
  readonly totalWritten: number;
  /** Muss 0 sein. Darüber ist der Puffer zu klein ausgelegt. */
  readonly dropped: number;
  readonly overflowed: boolean;
  readonly byteLength: number;
  readonly counts: Readonly<Record<string, number>>;
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
  /** Lage und Zeitanzeigen der Zauberleiste, in Geräte-Pixeln. */
  hud(): HudSnapshot;
  /** Zustand der Trefferwucht: Pool, Zahlen, Ruckeln. */
  impact(): ImpactSnapshot;
  /**
   * Das Kampfprotokoll als lesbare Liste (GAME-DESIGN 19.1, VERIFICATION § 3.4).
   *
   * **Erzeugt Speicher** — nur für Werkzeuge. Im Spiel bleibt das Protokoll
   * intern; einen sichtbaren Textstrom gibt es nicht (GAME-DESIGN 21.2).
   */
  log(): LogEntry[];
  /** Kennzahlen des Protokolls, ohne die ganze Liste zu erzeugen. */
  logStats(): LogStats;
  /** Die Kurzfassung nach dem Kampf (GAME-DESIGN 15.1). */
  summary(): RunSummary;
  /** Das Protokoll als Klartext ausgeben (GAME-DESIGN 19.1). */
  logText(limit?: number): string;

  // ---------------------------------------------------- Zeitsteuerung (19.1)
  /** Anhalten. Bilder werden weiter gezeichnet und gemessen. */
  pause(): void;
  resume(): void;
  /** Einzelschritt: `count` Logik-Takte sofort rechnen, ohne auf ein Bild zu warten. */
  step(count?: number): number;
  /** Zeitlupe (< 1) oder Zeitraffer (> 1). Der Takt bleibt 10 ms. */
  setTimeScale(scale: number): void;
  timeScale(): number;

  // ------------------------------------------- Aufzeichnen und Abspielen (19.1)
  /** Namen der eingebauten Aufzeichnungen. */
  recordings(): string[];
  /**
   * Eine Aufzeichnung abspielen — ohne Uhr, ohne Bild, sofort.
   *
   * Zwei Aufrufe liefern byteweise dasselbe. Mit `checksumPerTick` kommt die
   * volle Prüfsummenfolge mit; ohne nur die beiden Gesamtwerte, weil 6000
   * Zahlen selten gebraucht werden.
   */
  replay(name: string, options?: { checksumPerTick?: boolean }): ReplayResult;
  /** Dieselbe Aufzeichnung im laufenden Spiel abspielen — für die Bildprüfung. */
  replayLive(name: string): boolean;
  /** Läuft gerade eine Wiedergabe im Spiel? */
  isReplaying(): boolean;

  // --------------------------------------------- Prüfsumme und Herleitung (19.1)
  /** Prüfsumme des jetzigen Zustands. */
  checksum(): number;
  /** Prüfsumme je Takt mitschreiben. Kostet Rechenzeit, daher abschaltbar. */
  setChecksumTracking(on: boolean): void;
  checksumTrail(limit?: number): number[];
  /** Schadens-Herleitung eines Zaubers beim jetzigen Zustand. */
  explainDamage(spellId: string): DamageBreakdown;
  /** Eingabe-Zeitmessung: ms nach Ablauf der globalen Wartezeit, je Druck. */
  inputTiming(): number[];
  /** Alle Abklingzeiten sofort bereit machen (GAME-DESIGN 19.1: Zustand setzen). */
  setCooldownsReady(): void;
  /**
   * Schadenszahlen einspeisen, ohne den Kampf anzufassen.
   *
   * Nur ein Messmittel: Zwanzig Treffer je Sekunde entstehen in Schritt 1 nicht
   * von selbst (ein DoT tickt alle zwei Sekunden), im späteren Spiel mit vielen
   * gleichzeitigen Effekten sehr wohl. Der Kampf bleibt unberührt — es wird
   * nichts gewirkt und kein Schaden gezählt.
   */
  feedHits(count: number, damage?: number, big?: boolean): void;
  /** Einen Zauber auf einen Platz legen — für die Prüfung wachsender Leisten. */
  bindSlot(slot: number, spellId: string): boolean;
  /** Hitze setzen — erspart Wartezeit beim Prüfen (GAME-DESIGN 19.1). */
  setResource(value: number): void;
  /** Verbleibende Lebenspunkte des Bosses setzen (GAME-DESIGN 19.1). */
  setBossHp(hp: number): void;
  /** Kampf von vorn beginnen. */
  resetFight(): void;
  /** „Ready" auslösen — der Countdown beginnt. */
  ready(): boolean;
  /** „Wipe Run" auslösen. */
  wipe(): boolean;
  /** Esc-Menü öffnen oder schließen. */
  togglePause(): void;
  /** Ist das Esc-Menü offen? */
  isPaused(): boolean;
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
  /** Name der laufenden Wiedergabe im Bild, sonst `null`. */
  let liveReplay: string | null = null;
  // Die erzwungene Aufräumung hält die Seite an. Ohne diesen Anschluss zählte
  // die Schleife die dabei verlorene Zeit als verworfene Takte des Spiels.
  probe.onCollected = () => {
    app.loop.discardPendingTime();
  };
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
      discardedForMeasurement: app.loop.discardCount,
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
      phase: PHASE_NAMES[app.session.phase] ?? '?',
      outcome: OUTCOME_NAMES[app.session.outcome] ?? '?',
      countdownRemainingMs: app.session.countdownRemainingMs(app.loop.timeMs),
      fightElapsedMs: app.session.fightElapsedMs(app.loop.timeMs),
      overpressureRemainingMs: app.session.overpressureRemainingMs(app.loop.timeMs),
      fightStartedAtMs: app.combat.fightStartedAt,
      endedAtMs: app.session.endedAtMs,
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
    hud: () => {
      const combat = app.combat;
      const surface = app.surface;
      // Dieselbe Zeit, die auch der Renderer benutzt: die interpolierte. Sonst
      // meldete die Abfrage einen anderen Stand als der, der im Bild steht, und
      // ein Vergleich mit den Bildpunkten wäre wertlos.
      const timeMs = app.loop.timeMs + app.loop.alpha * TICK_MS;
      const scale = surface.designScale;
      const rect: Rect = { x: 0, y: 0, width: 0, height: 0 };
      const slots: HudSlotSnapshot[] = [];
      for (let slot = 0; slot < SLOT_COUNT; slot++) {
        const spellIndex = app.session.spellOnSlot(slot);
        if (spellIndex === NO_SPELL) continue;
        const spell = combat.spellAt(spellIndex);
        if (spell === undefined) continue;
        slotRect(slot, rect);
        const cooldownRemainingMs = combat.cooldownRemainingMs(spellIndex, timeMs);
        const sweepShare = spell.cooldownMs > 0 ? cooldownRemainingMs / spell.cooldownMs : 0;
        const gcdRemainingMs = combat.gcd.remainingMs(timeMs);
        const x = surface.designOffsetX + rect.x * scale;
        const y = surface.designOffsetY + rect.y * scale;
        const width = rect.width * scale;
        const height = rect.height * scale;
        slots.push({
          slot,
          spellId: spell.id,
          x,
          y,
          width,
          height,
          centerX: x + width * 0.5,
          centerY: y + height * 0.5,
          ready: cooldownRemainingMs === 0 && gcdRemainingMs === 0,
          cooldownMs: spell.cooldownMs,
          cooldownRemainingMs,
          sweepShare,
          sweepHandAngle: sweepHandAngle(sweepShare),
          gcdRemainingMs,
          gcdShare: gcdRemainingMs / combat.gcd.lengthMs,
          dotRemainingMs: combat.dotRemainingMs(spellIndex, timeMs),
          dotDurationMs: spell.dot?.durationMs ?? 0,
        });
      }
      return {
        canvasWidth: surface.canvas.width,
        canvasHeight: surface.canvas.height,
        scale,
        offsetX: surface.designOffsetX,
        offsetY: surface.designOffsetY,
        drawPasses: surface.drawPasses,
        frames: app.loop.frameCount,
        slots,
      };
    },
    impact: () => {
      const shownMs = app.loop.timeMs + app.loop.alpha * TICK_MS;
      return {
        poolCapacity: app.impact.poolCapacity,
        activeCount: app.impact.activeCount,
        recycleCount: app.impact.recycleCount,
        spawnCount: app.impact.spawnCount,
        shakeOffsetPx: app.impact.shakeOffset(shownMs),
        flashStrength: app.impact.flashStrength(shownMs),
      };
    },
    log: () => app.session.log.toArray(),
    logStats: () => {
      const log = app.session.log;
      const counts: Record<string, number> = {};
      for (let type = 0; type < EVENT_NAMES.length; type++) {
        counts[EVENT_NAMES[type] as string] = log.countOf(type);
      }
      return {
        entries: log.length,
        capacity: log.capacity,
        totalWritten: log.totalWritten,
        dropped: log.droppedCount,
        overflowed: log.hasOverflowed,
        byteLength: log.byteLength,
        counts,
      };
    },
    summary: () => app.session.summary(app.loop.timeMs),
    logText: (limit = 200) => {
      const entries = app.session.log.toArray();
      const from = Math.max(0, entries.length - limit);
      let out = '';
      for (let i = from; i < entries.length; i++) {
        const e = entries[i];
        if (e === undefined) continue;
        const spell = e.spell >= 0 ? (app.combat.spellAt(e.spell)?.id ?? String(e.spell)) : '-';
        out += `${(e.timeMs / 1000).toFixed(2)}  ${e.type.padEnd(10)} ${spell.padEnd(9)} ${e.value}\n`;
      }
      return out;
    },
    pause: () => {
      app.loop.setPaused(true);
    },
    resume: () => {
      app.loop.setPaused(false);
    },
    step: (count = 1) => {
      app.loop.advanceTicks(count);
      return app.loop.timeMs;
    },
    setTimeScale: (scale: number) => {
      app.loop.setTimeScale(scale);
    },
    timeScale: () => app.loop.timeScale,

    recordings: () => recordingNames(),
    replay: (name: string, options = {}) => {
      const recording = recordings().get(name);
      if (recording === undefined) {
        throw new Error(
          `Unbekannte Aufzeichnung: ${name}. Bekannt: ${recordingNames().join(', ')}`,
        );
      }
      return replayRecording(recording, options);
    },
    replayLive: (name: string) => {
      const recording = recordings().get(name);
      if (recording === undefined) return false;
      // Die Wiedergabe legt ihre Drücke in denselben Puffer, den auch die
      // Tastatur benutzt — sonst prüfte sie einen anderen Weg als den echten.
      // Der Haken sitzt hier und nicht in `App`, damit weder die Aufzeichnungen
      // noch die Prioritätenliste, aus der sie entstehen, im Auslieferungs-
      // Bundle landen (GAME-DESIGN 18.5 und 19.3).
      app.backToMenu();
      let index = 0;
      liveReplay = name;
      app.loop.onBeforeTick = (): void => {
        if (app.session.phase !== PHASE_FIGHTING) return;
        const tick = Math.round(app.combat.fightElapsedMs(app.loop.timeMs) / TICK_MS);
        while (index < recording.presses.length) {
          const press = recording.presses[index];
          if (press === undefined || press.tick > tick) break;
          index++;
          app.pressQueue.push(
            press.slot,
            app.loop.tickCount,
            performance.now(),
            app.loop.frameCount,
          );
        }
        if (index >= recording.presses.length) {
          liveReplay = null;
          app.loop.onBeforeTick = null;
        }
      };
      app.ready();
      return true;
    },
    isReplaying: () => liveReplay !== null,

    checksum: () => combatChecksum(app.combat, app.loop.timeMs),
    setChecksumTracking: (on: boolean) => {
      app.session.setChecksumTracking(on);
    },
    checksumTrail: (limit = 600) => app.session.checksumTrail(limit),
    explainDamage: (spellId: string) => {
      const index = app.combat.indexOf(spellId);
      const spell = app.combat.spellAt(index);
      if (spell === undefined) throw new Error(`Unbekannter Zauber: ${spellId}`);
      const spender = spell.spender;
      const resource = app.combat.resource;
      const bonusApplied = spender !== undefined && resource >= spender.bonusThreshold;
      const spenderRaw =
        spender === undefined
          ? 0
          : spender.damagePerPoint * resource * (bonusApplied ? spender.bonusMultiplier : 1);
      const factors = [
        { name: 'essence', value: 1 },
        { name: 'talents', value: 1 },
        { name: 'buffs', value: 1 },
        { name: 'synergy', value: 1 },
        { name: 'resistance', value: app.combat.boss.resistances[app.combat.school.school] ?? 1 },
        { name: 'crit', value: 1 },
      ];
      let total = spell.baseDamage + spenderRaw;
      for (const factor of factors) total *= factor.value;
      return {
        spellId,
        baseDamage: spell.baseDamage,
        spenderDamage: spenderRaw,
        resource,
        bonusApplied,
        factors,
        total: Math.round(total),
      };
    },
    inputTiming: () => app.session.inputTiming.toArray(),
    setCooldownsReady: () => {
      app.combat.setCooldownsReady();
    },

    feedHits: (count: number, damage = 100, big = false) => {
      const now = app.loop.timeMs;
      for (let i = 0; i < count; i++) {
        app.impact.onHit(damage, i % 3 === 0 ? HIT_DOT : HIT_DIRECT, false, big, now);
      }
    },
    bindSlot: (slot: number, spellId: string) => {
      const index = app.combat.indexOf(spellId);
      if (index < 0) return false;
      app.session.bind(slot, index);
      return true;
    },
    setBossHp: (hp: number) => {
      app.combat.setBossHpRemaining(hp);
    },
    setResource: (value: number) => {
      app.combat.setResource(value);
    },
    resetFight: () => {
      app.backToMenu();
    },
    ready: () => app.ready(),
    wipe: () => app.wipe(),
    togglePause: () => {
      app.togglePause();
    },
    isPaused: () => app.overlay.isPaused,
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
