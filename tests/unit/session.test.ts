import { describe, expect, it } from 'vitest';
import { GCD_MS, GlobalCooldown } from '../../src/core/gcd.js';
import { NEVER, Session, type FeedbackSink } from '../../src/core/session.js';
import { TICK_MS } from '../../src/core/loop.js';
import { PressQueue } from '../../src/input/queue.js';

class RecordingFeedback implements FeedbackSink {
  readonly accepted: number[] = [];
  readonly rejected: number[] = [];

  onPressAccepted(slot: number, timeMs: number): void {
    this.accepted.push(timeMs);
    void slot;
  }

  onPressRejected(slot: number, timeMs: number): void {
    this.rejected.push(timeMs);
    void slot;
  }
}

/** Rechnet `ticks` Takte ab `fromMs` — so, wie es die Schleife täte. */
function run(session: Session, ticks: number, fromMs = 0): void {
  for (let i = 0; i < ticks; i++) session.tick(fromMs + i * TICK_MS);
}

describe('GlobalCooldown', () => {
  it('ist zu Beginn bereit', () => {
    const gcd = new GlobalCooldown();
    expect(gcd.isReady(0)).toBe(true);
    expect(gcd.remainingMs(0)).toBe(0);
    expect(gcd.lengthMs).toBe(GCD_MS);
  });

  it('sperrt exakt 1,5 Sekunden', () => {
    const gcd = new GlobalCooldown();
    gcd.trigger(1000);

    expect(gcd.isReady(1000)).toBe(false);
    expect(gcd.isReady(2490)).toBe(false);
    expect(gcd.remainingMs(2490)).toBe(10);
    // Auf die Millisekunde: bei 2500 ist wieder frei.
    expect(gcd.isReady(2500)).toBe(true);
    expect(gcd.remainingMs(2500)).toBe(0);
  });

  it('rechnet den Fortschritt für die Anzeige aus', () => {
    const gcd = new GlobalCooldown();
    gcd.trigger(0);

    expect(gcd.progress(0)).toBeCloseTo(0);
    expect(gcd.progress(750)).toBeCloseTo(0.5);
    expect(gcd.progress(1500)).toBe(1);
  });

  it('driftet über 100 Auslösungen nicht', () => {
    const gcd = new GlobalCooldown();
    let now = 0;
    for (let i = 0; i < 100; i++) {
      gcd.trigger(now);
      now = gcd.readyAt;
    }
    expect(now).toBe(100 * GCD_MS);
    expect(Number.isInteger(now)).toBe(true);
  });

  it('weist krumme Längen zurück', () => {
    const gcd = new GlobalCooldown();
    expect(() => gcd.setLength(1000.5)).toThrow(RangeError);
    expect(() => gcd.setLength(-1)).toThrow(RangeError);
  });
});

describe('Session — Eingabe während der Wartezeit (AC4)', () => {
  it('nimmt den ersten Druck an und löst die Wartezeit aus', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(0, 0, 0, 0);
    run(session, 1);

    expect(session.acceptedCount).toBe(1);
    expect(session.rejectedCount).toBe(0);
    expect(session.gcd.isReady(0)).toBe(false);
  });

  it('verwirft einen Druck während der Wartezeit — er wird nicht nachgeholt', () => {
    const queue = new PressQueue();
    const feedback = new RecordingFeedback();
    const session = new Session(queue, feedback);

    queue.push(0, 0, 0, 0); // Takt 0: angenommen
    queue.push(1, 50, 0, 0); // Takt 50 (500 ms): mitten in der Wartezeit
    run(session, 400); // 4 Sekunden — weit über die Wartezeit hinaus

    expect(session.acceptedCount).toBe(1);
    expect(session.rejectedCount).toBe(1);
    // Entscheidend: Nach Ablauf der Wartezeit passiert nichts mehr. Wäre der
    // Druck vorgemerkt worden, stünde hier eine zweite Annahme.
    expect(feedback.accepted).toEqual([0]);
    expect(feedback.rejected).toEqual([500]);
    expect(queue.size).toBe(0);
  });

  it('nimmt den nächsten Druck nach Ablauf der Wartezeit an', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(0, 0, 0, 0);
    queue.push(1, 150, 0, 0); // genau bei 1500 ms
    run(session, 200);

    expect(session.acceptedCount).toBe(2);
    expect(session.rejectedCount).toBe(0);
  });

  it('weist einen Druck eine Millisekunde zu früh ab', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(0, 0, 0, 0);
    queue.push(1, 149, 0, 0); // 1490 ms — ein Takt zu früh
    run(session, 200);

    expect(session.acceptedCount).toBe(1);
    expect(session.rejectedCount).toBe(1);
  });

  it('verwirft auch eine ganze Serie, ohne einen davon nachzuholen', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(0, 0, 0, 0);
    for (let tick = 1; tick < 60; tick++) queue.push(1, tick, 0, 0);
    run(session, 600);

    expect(session.acceptedCount).toBe(1);
    expect(session.rejectedCount).toBe(59);
  });
});

describe('Session — Zustand für die Anzeige', () => {
  it('merkt sich, wann eine Belegung zuletzt angenommen oder verworfen wurde', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(3, 0, 0, 0);
    queue.push(7, 20, 0, 0);
    run(session, 100);

    expect(session.acceptedAtMs[3]).toBe(0);
    expect(session.rejectedAtMs[7]).toBe(200);
    expect(session.acceptedAtMs[7]).toBe(NEVER);
    expect(session.lastPressedSlot).toBe(7);
  });

  it('legt für jeden sichtbaren Ausgang eine Messspur ab', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(0, 0, 111.5, 4);
    queue.push(1, 10, 222.5, 9);
    run(session, 100);

    expect(session.latency.size).toBe(2);
    expect(session.latency.frontStamp()).toBe(111.5);
    expect(session.latency.frontFrame()).toBe(4);
    session.latency.shift();
    expect(session.latency.frontStamp()).toBe(222.5);
  });

  it('setzt sich vollständig zurück', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(0, 0, 0, 0);
    run(session, 10);
    session.reset();

    expect(session.acceptedCount).toBe(0);
    expect(session.rejectedCount).toBe(0);
    expect(session.lastPressedSlot).toBe(-1);
    expect(session.acceptedAtMs[0]).toBe(NEVER);
    expect(session.gcd.isReady(0)).toBe(true);
    expect(session.latency.size).toBe(0);
  });
});

describe('Session — Puffer wird nie zur Warteschlange', () => {
  it('leert alles, was dem laufenden Takt oder früher zugewiesen war', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    for (let i = 0; i < 10; i++) queue.push(i, 0, 0, 0);
    session.tick(0);

    expect(queue.size).toBe(0);
    expect(session.acceptedCount + session.rejectedCount).toBe(10);
  });

  it('lässt einen Druck aus einem späteren Takt liegen, bis dieser Takt kommt', () => {
    const queue = new PressQueue();
    const session = new Session(queue);
    queue.push(0, 5, 0, 0);
    run(session, 5);
    expect(queue.size).toBe(1);
    expect(session.acceptedCount).toBe(0);

    session.tick(50);
    expect(queue.size).toBe(0);
    expect(session.acceptedCount).toBe(1);
  });
});
