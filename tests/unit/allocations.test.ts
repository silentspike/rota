import { describe, expect, it } from 'vitest';
import { AllocationProbe, type AllocationResult } from '../../src/devtools/allocations.js';

/** Zählt Bilder ab, bis die Messung fertig ist. */
function runFrames(probe: AllocationProbe, count: number): void {
  for (let i = 0; i < count; i++) probe.onFrame();
}

describe('AllocationProbe', () => {
  it('meldet das Ergebnis nach der gewünschten Zahl von Bildern', () => {
    const probe = new AllocationProbe();
    let result: AllocationResult | null = null;
    probe.begin(10, (r) => {
      result = r;
    });

    runFrames(probe, 9);
    expect(result).toBeNull();
    expect(probe.isMeasuring).toBe(true);

    probe.onFrame();
    expect(result).not.toBeNull();
    expect(probe.isMeasuring).toBe(false);
    expect(probe.lastResult.frames).toBe(10);
  });

  it('stellt eine zweite Anfrage hinten an, statt sie fallen zu lassen', () => {
    // Der Fehler, der das aufgedeckt hat: Eine Anfrage während einer laufenden
    // Messung wurde stillschweigend verworfen. Das Versprechen des Aufrufers
    // wurde damit nie eingelöst, und die Messung lief in eine Zeitüberschreitung.
    const probe = new AllocationProbe();
    const done: string[] = [];
    probe.begin(10, () => done.push('erste'));
    probe.begin(5, () => done.push('zweite'));

    runFrames(probe, 10);
    expect(done).toEqual(['erste']);
    expect(probe.isMeasuring).toBe(true);

    runFrames(probe, 5);
    expect(done).toEqual(['erste', 'zweite']);
    expect(probe.isMeasuring).toBe(false);
    expect(probe.lastResult.frames).toBe(5);
  });

  it('arbeitet mehrere wartende Anfragen der Reihe nach ab', () => {
    const probe = new AllocationProbe();
    const done: number[] = [];
    probe.begin(2, (r) => done.push(r.frames));
    probe.begin(3, (r) => done.push(r.frames));
    probe.begin(4, (r) => done.push(r.frames));

    runFrames(probe, 2 + 3 + 4);
    expect(done).toEqual([2, 3, 4]);
    expect(probe.isMeasuring).toBe(false);
  });

  it('lässt niemanden hängen, wenn die Warteliste voll ist', () => {
    const probe = new AllocationProbe();
    const done: number[] = [];
    probe.begin(1000, () => done.push(-1));
    for (let i = 0; i < 20; i++) probe.begin(1, () => done.push(i));

    // Alles über der Grenze bekommt sofort das letzte Ergebnis — irgendeine
    // Antwort ist besser als ein Versprechen, das nie eingelöst wird.
    expect(done.length).toBeGreaterThan(0);
  });

  it('bleibt ohne Aufräum-Funktion auskunftsfähig', () => {
    const probe = new AllocationProbe();
    let result: AllocationResult | null = null;
    probe.begin(1, (r) => {
      result = r;
    });
    probe.onFrame();

    // In Node gibt es weder `gc()` noch `performance.memory`. Die Messung meldet
    // das ehrlich, statt eine Zahl vorzutäuschen.
    expect(result).not.toBeNull();
    const value = result as unknown as AllocationResult;
    expect(value.gcAvailable).toBe(false);
    expect(Number.isNaN(value.bytesTotal)).toBe(true);
  });

  it('übergeht Bilder, solange nicht gemessen wird', () => {
    const probe = new AllocationProbe();
    runFrames(probe, 100);
    expect(probe.isMeasuring).toBe(false);
    expect(probe.lastResult.frames).toBe(0);
  });
});
