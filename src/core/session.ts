/**
 * Der Kampf-Zustand, so weit er in Issue E1-2 reicht: Eingabe und globale
 * Wartezeit.
 *
 * Die Zauber kommen mit E1-3 dazu und setzen genau hier an — ein angenommener
 * Tastendruck wird dann zum gewirkten Zauber, statt nur die Wartezeit auszulösen.
 * Was bereits jetzt endgültig ist, sind die beiden Regeln, die das Design
 * festschreibt:
 *
 * - Ein Druck während der globalen Wartezeit wird **verworfen**, nicht gemerkt
 *   (GAME-DESIGN 8.2 und 21.1). Der Spieler lernt sofort.
 * - Die Rückmeldung darauf ist rotes Aufblinken und ein dumpfer Ton.
 */
import { GlobalCooldown } from './gcd.js';
import { LatencyTrace } from './latency.js';
import type { Simulation } from './loop.js';
import { TICK_MS } from './loop.js';
import { SLOT_COUNT } from '../input/keys.js';
import type { PressQueue } from '../input/queue.js';

/** Wie ein Tastendruck ausgegangen ist. */
export const PRESS_ACCEPTED = 0;
export const PRESS_REJECTED_GCD = 1;

/**
 * Wohin die Rückmeldung geht, die nicht im Bild stattfindet — derzeit der Klang.
 * Der Kampf kennt weder Puffer noch Knoten; er sagt nur, was geschehen ist.
 */
export interface FeedbackSink {
  onPressAccepted(slot: number, timeMs: number): void;
  onPressRejected(slot: number, timeMs: number): void;
}

/** Nie gedrückt. */
export const NEVER = -1;

export class Session implements Simulation {
  readonly gcd = new GlobalCooldown();
  readonly latency = new LatencyTrace();

  /** Kampfzeit des letzten angenommenen Drucks je Belegung, sonst `NEVER`. */
  readonly acceptedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);
  /** Kampfzeit des letzten verworfenen Drucks je Belegung, sonst `NEVER`. */
  readonly rejectedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);

  private accepted = 0;
  private rejected = 0;
  private lastSlot = -1;
  private timeMs = 0;

  constructor(
    private readonly queue: PressQueue,
    private readonly feedback: FeedbackSink | null = null,
  ) {}

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

  tick(timeMs: number): void {
    this.timeMs = timeMs;
    const tickIndex = timeMs / TICK_MS;
    // Alles abarbeiten, was diesem Takt oder einem früheren zugewiesen war.
    // Nichts bleibt liegen — dadurch kann der Puffer nie zur Warteschlange werden.
    while (this.queue.size > 0 && this.queue.frontTick() <= tickIndex) {
      const slot = this.queue.frontSlot();
      const stampMs = this.queue.frontStamp();
      const frameIndex = this.queue.frontFrame();
      this.queue.shift();
      this.apply(slot, timeMs, stampMs, frameIndex);
    }
  }

  private apply(slot: number, timeMs: number, stampMs: number, frameIndex: number): void {
    this.lastSlot = slot;
    if (this.gcd.isReady(timeMs)) {
      this.gcd.trigger(timeMs);
      this.acceptedAtMs[slot] = timeMs;
      this.accepted++;
      if (this.feedback !== null) this.feedback.onPressAccepted(slot, timeMs);
    } else {
      this.rejectedAtMs[slot] = timeMs;
      this.rejected++;
      if (this.feedback !== null) this.feedback.onPressRejected(slot, timeMs);
    }
    // Beide Ausgänge sind im nächsten Bild sichtbar: der eine als Aufleuchten,
    // der andere als rotes Blinken. Beide zählen daher für die Verzögerung.
    this.latency.push(stampMs, frameIndex);
  }

  reset(): void {
    this.gcd.reset();
    this.acceptedAtMs.fill(NEVER);
    this.rejectedAtMs.fill(NEVER);
    this.latency.clear();
    this.queue.clear();
    this.accepted = 0;
    this.rejected = 0;
    this.lastSlot = -1;
    this.timeMs = 0;
  }
}
