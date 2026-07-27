import { describe, expect, it } from 'vitest';
import { PRESS_QUEUE_CAPACITY, PressQueue } from '../../src/input/queue.js';
import { LATENCY_TRACE_CAPACITY, LatencyTrace } from '../../src/core/latency.js';

describe('PressQueue', () => {
  it('gibt die Tastendrücke in der Reihenfolge des Eintreffens zurück', () => {
    const queue = new PressQueue();
    queue.push(3, 10, 100.5, 1);
    queue.push(7, 11, 120.5, 2);

    expect(queue.size).toBe(2);
    expect(queue.frontSlot()).toBe(3);
    expect(queue.frontTick()).toBe(10);
    expect(queue.frontStamp()).toBe(100.5);
    expect(queue.frontFrame()).toBe(1);
    queue.shift();
    expect(queue.frontSlot()).toBe(7);
    queue.shift();
    expect(queue.size).toBe(0);
  });

  it('läuft über den Rand des Puffers hinweg richtig weiter', () => {
    const queue = new PressQueue(4);
    for (let round = 0; round < 100; round++) {
      queue.push(round % 26, round, round * 1.5, round);
      expect(queue.frontSlot()).toBe(round % 26);
      queue.shift();
    }
    expect(queue.size).toBe(0);
    expect(queue.lostCount).toBe(0);
  });

  it('meldet einen Überlauf, statt still zu verwerfen', () => {
    const queue = new PressQueue(2);
    expect(queue.push(0, 0, 0, 0)).toBe(true);
    expect(queue.push(1, 0, 0, 0)).toBe(true);
    expect(queue.push(2, 0, 0, 0)).toBe(false);

    expect(queue.size).toBe(2);
    expect(queue.lostCount).toBe(1);
  });

  it('fasst mehr Anschläge, als eine Hand zwischen zwei Takten schafft', () => {
    // Zehn Millisekunden je Takt: Selbst 20 Anschläge je Sekunde ergeben
    // rechnerisch 0,2 Einträge — 64 ist reichlich Sicherheitsabstand.
    expect(PRESS_QUEUE_CAPACITY).toBeGreaterThanOrEqual(16);
  });

  it('leert sich vollständig', () => {
    const queue = new PressQueue();
    queue.push(1, 1, 1, 1);
    queue.clear();
    expect(queue.size).toBe(0);
  });

  it('weist unsinnige Größen zurück', () => {
    expect(() => new PressQueue(0)).toThrow(RangeError);
    expect(() => new PressQueue(1.5)).toThrow(RangeError);
  });

  it('übergeht shift auf einem leeren Puffer', () => {
    const queue = new PressQueue();
    queue.shift();
    expect(queue.size).toBe(0);
  });
});

describe('LatencyTrace', () => {
  it('gibt die Einträge in der Reihenfolge des Eintreffens zurück', () => {
    const trace = new LatencyTrace(4);
    trace.push(10.5, 1);
    trace.push(20.5, 2);

    expect(trace.size).toBe(2);
    expect(trace.frontStamp()).toBe(10.5);
    expect(trace.frontFrame()).toBe(1);
    trace.shift();
    expect(trace.frontStamp()).toBe(20.5);
  });

  it('überschreibt die ältesten Einträge, wenn niemand abholt', () => {
    // Im Auslieferungsstand holt niemand ab. Der Puffer darf davon nicht wachsen.
    const trace = new LatencyTrace(4);
    for (let i = 0; i < 100; i++) trace.push(i, i);

    expect(trace.size).toBe(4);
    expect(trace.frontStamp()).toBe(96);
  });

  it('leert sich vollständig', () => {
    const trace = new LatencyTrace(4);
    trace.push(1, 1);
    trace.clear();
    expect(trace.size).toBe(0);
    trace.shift();
    expect(trace.size).toBe(0);
  });

  it('ist groß genug für alles, was zwischen zwei Bildern anfällt', () => {
    expect(LATENCY_TRACE_CAPACITY).toBeGreaterThanOrEqual(16);
  });
});
