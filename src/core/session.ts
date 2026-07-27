/**
 * Der Kampf-Zustand: Eingabe, globale Wartezeit, Zauber.
 *
 * Hier treffen die Tastendrücke aus `src/input/` auf die Regeln aus `combat.ts`.
 * Zwei Regeln des Designs werden an dieser Stelle durchgesetzt:
 *
 * - Ein Druck, der nicht gewirkt werden kann, wird **verworfen**, nicht gemerkt
 *   (GAME-DESIGN 8.2 und 21.1). Der Spieler lernt sofort.
 * - Die Rückmeldung darauf ist rotes Aufblinken und ein dumpfer Ton.
 *
 * Der Ablauf je Takt folgt `tools/step1-check.py`: erst die laufenden Wirkungen,
 * dann die Prüfung auf das Kampfende, dann die Tastendrücke dieses Takts.
 */
import { CAST_OK, Combat, FIGHT_RUNNING } from './combat.js';
import { LatencyTrace } from './latency.js';
import type { Simulation } from './loop.js';
import { TICK_MS } from './loop.js';
import { SLOT_COUNT } from '../input/keys.js';
import type { PressQueue } from '../input/queue.js';

/** Kein Zauber auf diesem Platz. */
export const NO_SPELL = -1;

/** Nie gedrückt. */
export const NEVER = -1;

/**
 * Wohin die Rückmeldung geht, die nicht im Bild stattfindet — derzeit der Klang.
 * Der Kampf kennt weder Puffer noch Knoten; er sagt nur, was geschehen ist.
 */
export interface FeedbackSink {
  onPressAccepted(slot: number, spellIndex: number, timeMs: number): void;
  onPressRejected(slot: number, reason: number, timeMs: number): void;
}

export class Session implements Simulation {
  readonly latency = new LatencyTrace();

  /** Kampfzeit des letzten angenommenen Drucks je Belegung, sonst `NEVER`. */
  readonly acceptedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);
  /** Kampfzeit des letzten verworfenen Drucks je Belegung, sonst `NEVER`. */
  readonly rejectedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);

  /** Belegung → Zauber. `NO_SPELL`, solange der Platz leer ist. */
  private readonly bindings = new Int32Array(SLOT_COUNT).fill(NO_SPELL);

  private accepted = 0;
  private rejected = 0;
  private lastSlot = -1;
  private timeMs = 0;
  private idleTicks = 0;

  constructor(
    private readonly queue: PressQueue,
    readonly combat: Combat,
    private readonly feedback: FeedbackSink | null = null,
  ) {}

  /** Abkürzung: Die globale Wartezeit gehört dem Kampf. */
  get gcd(): Combat['gcd'] {
    return this.combat.gcd;
  }

  get acceptedCount(): number {
    return this.accepted;
  }

  get rejectedCount(): number {
    return this.rejected;
  }

  /** Zuletzt gedrückte Belegung, oder -1. */
  get lastPressedSlot(): number {
    return this.lastSlot;
  }

  /** Kampfzeit, bis zu der gerechnet wurde. */
  get nowMs(): number {
    return this.timeMs;
  }

  /**
   * Takte, in denen gewirkt werden konnte, aber nichts gewirkt wurde.
   *
   * Grundlage der Kennzahl „verlorene Zeit" aus GAME-DESIGN 15.1; ausgewertet
   * wird sie in E1-7.
   */
  get idleMs(): number {
    return this.idleTicks * TICK_MS;
  }

  bind(slot: number, spellIndex: number): void {
    if (slot < 0 || slot >= SLOT_COUNT) return;
    this.bindings[slot] = spellIndex;
  }

  spellOnSlot(slot: number): number {
    return this.bindings[slot] ?? NO_SPELL;
  }

  /** Plätze mit einem Zauber darauf — die einzigen, die die Leiste zeigt. */
  boundSlotCount(): number {
    let count = 0;
    for (let i = 0; i < SLOT_COUNT; i++) {
      if ((this.bindings[i] ?? NO_SPELL) !== NO_SPELL) count++;
    }
    return count;
  }

  tick(timeMs: number): void {
    this.timeMs = timeMs;

    // 1. laufende Wirkungen
    this.combat.stepPassive(timeMs);

    // 2. Kampfende
    const over = this.combat.checkDefeat(timeMs);

    // 3. Tastendrücke dieses Takts. Alles abarbeiten, was diesem Takt oder einem
    //    früheren zugewiesen war — dadurch kann der Puffer nie zur Warteschlange
    //    werden.
    const tickIndex = timeMs / TICK_MS;
    let casted = false;
    while (this.queue.size > 0 && this.queue.frontTick() <= tickIndex) {
      const slot = this.queue.frontSlot();
      const stampMs = this.queue.frontStamp();
      const frameIndex = this.queue.frontFrame();
      this.queue.shift();
      if (this.apply(slot, timeMs, stampMs, frameIndex)) casted = true;
    }

    if (!over && !casted && this.combat.gcd.isReady(timeMs)) this.idleTicks++;
  }

  /** @returns true, wenn tatsächlich gewirkt wurde. */
  private apply(slot: number, timeMs: number, stampMs: number, frameIndex: number): boolean {
    this.lastSlot = slot;
    const spellIndex = this.bindings[slot] ?? NO_SPELL;

    let accepted = false;
    let reason = CAST_OK;
    if (spellIndex === NO_SPELL || this.combat.fightState !== FIGHT_RUNNING) {
      // Ein leerer Platz ist eine Fehleingabe wie jede andere.
      reason = spellIndex === NO_SPELL ? -1 : -2;
    } else {
      const outcome = this.combat.cast(spellIndex, timeMs);
      reason = outcome.result;
      accepted = outcome.result === CAST_OK;
    }

    if (accepted) {
      this.acceptedAtMs[slot] = timeMs;
      this.accepted++;
      if (this.feedback !== null) this.feedback.onPressAccepted(slot, spellIndex, timeMs);
    } else {
      this.rejectedAtMs[slot] = timeMs;
      this.rejected++;
      if (this.feedback !== null) this.feedback.onPressRejected(slot, reason, timeMs);
    }
    // Beide Ausgänge sind im nächsten Bild sichtbar: der eine als Aufleuchten,
    // der andere als rotes Blinken. Beide zählen daher für die Verzögerung.
    this.latency.push(stampMs, frameIndex);
    return accepted;
  }

  reset(): void {
    this.combat.reset();
    this.acceptedAtMs.fill(NEVER);
    this.rejectedAtMs.fill(NEVER);
    this.latency.clear();
    this.queue.clear();
    this.accepted = 0;
    this.rejected = 0;
    this.lastSlot = -1;
    this.timeMs = 0;
    this.idleTicks = 0;
  }
}
