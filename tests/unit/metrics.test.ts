import { describe, expect, it } from 'vitest';
import { PerfCounters, summarize } from '../../src/core/metrics.js';
import { RingBufferF64 } from '../../src/core/ring.js';

describe('PerfCounters', () => {
  it('zählt Bilder, Takte und die schlechteste Bildzeit', () => {
    const perf = new PerfCounters();
    perf.recordFrame(2.5, 16.7, 2);
    perf.recordFrame(9.1, 16.6, 1);
    perf.recordFrame(1.2, 16.7, 2);

    expect(perf.frameCount).toBe(3);
    expect(perf.tickCount).toBe(5);
    expect(perf.worstFrameMs).toBeCloseTo(9.1);
  });

  it('setzt sich vollständig zurück', () => {
    const perf = new PerfCounters();
    for (let i = 0; i < 100; i++) perf.recordFrame(i, i, 1);
    perf.reset();

    expect(perf.frameCount).toBe(0);
    expect(perf.tickCount).toBe(0);
    expect(perf.worstFrameMs).toBe(0);
    expect(perf.frameWork.length).toBe(0);
  });

  it('hält die Eingabe-Verzögerung getrennt', () => {
    const perf = new PerfCounters();
    perf.recordInputLatency(8.2, 1);
    perf.recordInputLatency(12.4, 1);

    expect(perf.inputLatency.toArray()).toEqual([8.2, 12.4]);
    expect(perf.inputFrameDelay.toArray()).toEqual([1, 1]);
    expect(perf.frameCount).toBe(0);
  });
});

describe('summarize', () => {
  it('rechnet Kennzahlen einer Messreihe aus', () => {
    const ring = new RingBufferF64(16);
    for (const value of [1, 2, 3, 4, 5, 20]) ring.push(value);
    const summary = summarize(ring, 16.7);

    expect(summary.count).toBe(6);
    expect(summary.min).toBe(1);
    expect(summary.max).toBe(20);
    expect(summary.mean).toBeCloseTo(35 / 6);
    expect(summary.over).toBe(1);
    expect(summary.threshold).toBe(16.7);
  });

  it('betrachtet nur die letzten Werte, wenn eine Zahl vorgegeben ist', () => {
    const ring = new RingBufferF64(1024);
    for (let i = 0; i < 1000; i++) ring.push(i);
    const summary = summarize(ring, 500, 100);

    expect(summary.count).toBe(100);
    expect(summary.min).toBe(900);
    expect(summary.max).toBe(999);
    expect(summary.over).toBe(100);
  });

  it('liefert bei leerer Reihe Nullen statt NaN', () => {
    const summary = summarize(new RingBufferF64(8), 16.7);

    expect(summary.count).toBe(0);
    expect(summary.max).toBe(0);
    expect(summary.mean).toBe(0);
    expect(Number.isNaN(summary.p99)).toBe(false);
  });

  it('bestimmt das 99. Perzentil', () => {
    const ring = new RingBufferF64(200);
    for (let i = 1; i <= 100; i++) ring.push(i);
    const summary = summarize(ring, 1000);

    expect(summary.p99).toBe(100);
  });
});
