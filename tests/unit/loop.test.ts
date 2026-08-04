import { describe, expect, it } from 'vitest';
import {
  GameLoop,
  MAX_CATCHUP_TICKS,
  TICK_MS,
  type Renderer,
  type Simulation,
} from '../../src/core/loop.js';

/**
 * Treiber, der Bilder in einem gewählten Takt liefert — damit lässt sich dieselbe
 * Schleife ohne Browser bei 60 Hz und bei 144 Hz durchrechnen.
 */
class FrameDriver {
  /** Aktuelle Uhrzeit in ms. */
  clockMs = 0;
  private pending: ((timestampMs: number) => void) | null = null;
  private nextHandle = 1;

  readonly requestFrame = (callback: (timestampMs: number) => void): number => {
    this.pending = callback;
    return this.nextHandle++;
  };

  readonly cancelFrame = (): void => {
    this.pending = null;
  };

  readonly now = (): number => this.clockMs;

  /** Lässt `count` Bilder mit dem Abstand `intervalMs` vergehen. */
  runFrames(count: number, intervalMs: number): void {
    for (let i = 0; i < count; i++) {
      const callback = this.pending;
      if (callback === null) return;
      this.pending = null;
      callback(this.clockMs);
      this.clockMs += intervalMs;
    }
  }

  /** Ein einzelnes Bild nach einer Pause von `gapMs`. */
  runFrameAfterGap(gapMs: number): void {
    this.clockMs += gapMs;
    const callback = this.pending;
    if (callback === null) return;
    this.pending = null;
    callback(this.clockMs);
  }
}

/** Zeichnet nichts, merkt sich nur, womit gezeichnet wurde. */
class RecordingRenderer implements Renderer {
  readonly simTimes: number[] = [];
  readonly alphas: number[] = [];

  render(simTimeMs: number, alpha: number): void {
    this.simTimes.push(simTimeMs);
    this.alphas.push(alpha);
  }
}

/** Hält alle Takt-Zeitpunkte fest. */
class RecordingSimulation implements Simulation {
  readonly tickTimes: number[] = [];

  tick(timeMs: number): void {
    this.tickTimes.push(timeMs);
  }
}

/**
 * Stellvertreter für den Kampf: verrechnet Tastendrücke, die zu Realzeitpunkten
 * eintreffen, in den Takt, in den sie gehören. Damit prüft AC1 genau das, was das
 * Zeitgerüst zusichert — die Zauber selbst kommen erst mit E1-3.
 */
class ScheduledInputSimulation implements Simulation {
  value = 0;
  private index = 0;

  /** @param inputTimesMs Realzeitpunkte der Tastendrücke, aufsteigend sortiert. */
  constructor(private readonly inputTimesMs: readonly number[]) {}

  tick(timeMs: number): void {
    while (
      this.index < this.inputTimesMs.length &&
      (this.inputTimesMs[this.index] as number) < timeMs + TICK_MS
    ) {
      // Der Wert hängt vom Takt ab, in dem die Eingabe landet: Verrutscht die
      // Zuordnung um auch nur einen Takt, weicht das Ergebnis ab.
      this.value += timeMs + 1;
      this.index++;
    }
  }
}

function makeLoop(
  simulation: Simulation,
  renderer: Renderer = new RecordingRenderer(),
): { loop: GameLoop; driver: FrameDriver } {
  const driver = new FrameDriver();
  const loop = new GameLoop({
    simulation,
    renderer,
    requestFrame: driver.requestFrame,
    cancelFrame: driver.cancelFrame,
    now: driver.now,
  });
  return { loop, driver };
}

describe('GameLoop — ganzzahlige Zeit', () => {
  it('rechnet Takte in exakten 10-ms-Schritten ab 0', () => {
    const sim = new RecordingSimulation();
    const { loop, driver } = makeLoop(sim);
    loop.start();
    driver.runFrames(20, 1000 / 60);

    expect(sim.tickTimes.length).toBeGreaterThan(0);
    // Gesammelt statt je Takt geprüft (#127). Gemessen: 3656 expect-Aufrufe
    // kosten 569 ms, die Rechnung dahinter 10 ms. Der Rest der Laufzeit ist
    // echte Simulationsarbeit und bleibt — hier ging es nur um den Aufruf.
    const krumm: string[] = [];
    for (let i = 0; i < sim.tickTimes.length; i++) {
      if (sim.tickTimes[i] !== i * TICK_MS) {
        krumm.push(`Takt ${i}: ${sim.tickTimes[i]} statt ${i * TICK_MS}`);
      }
    }
    expect(krumm).toEqual([]);
    expect(loop.timeMs).toBe(sim.tickTimes.length * TICK_MS);
    expect(Number.isInteger(loop.timeMs)).toBe(true);
  });

  it('driftet über fünf Minuten nicht (AC3)', () => {
    const sim = new RecordingSimulation();
    const { loop, driver } = makeLoop(sim);
    loop.start();

    // Fünf Minuten bei 60 Hz. Der Bildabstand ist bewusst krumm (16,666…),
    // damit sich ein Fließkomma-Fehler zeigen würde, falls einer entstünde.
    const frames = 60 * 60 * 5;
    driver.runFrames(frames, 1000 / 60);

    const expectedTicks = 300_000 / TICK_MS;
    // Ein angebrochener Takt am Ende ist zulässig, mehr nicht.
    expect(loop.tickCount).toBeGreaterThanOrEqual(expectedTicks - 2);
    expect(loop.tickCount).toBeLessThanOrEqual(expectedTicks);
    expect(loop.timeMs).toBe(loop.tickCount * TICK_MS);
    expect(Number.isInteger(loop.timeMs)).toBe(true);
    expect(loop.droppedTickCount).toBe(0);

    // Abweichung der Simulationszeit von der verstrichenen Realzeit.
    const elapsedRealMs = driver.clockMs - 1000 / 60;
    expect(Math.abs(elapsedRealMs - loop.timeMs)).toBeLessThan(TICK_MS);
  });

  it('hält die Takt-Zeitpunkte exakt, egal wie krumm die Bildabstände sind', () => {
    const sim = new RecordingSimulation();
    const { loop, driver } = makeLoop(sim);
    loop.start();
    // Ein Abstand, der niemals glatt in 10 ms aufgeht.
    driver.runFrames(5000, 7.3137);

    for (let i = 0; i < sim.tickTimes.length; i++) {
      expect(sim.tickTimes[i]).toBe(i * TICK_MS);
    }
  });
});

describe('GameLoop — Bildrate-Unabhängigkeit (AC1)', () => {
  it('liefert bei 60 Hz und 144 Hz dieselbe Takt-Folge', () => {
    const at60 = new RecordingSimulation();
    const at144 = new RecordingSimulation();
    const loop60 = makeLoop(at60);
    const loop144 = makeLoop(at144);

    loop60.loop.start();
    loop144.loop.start();
    // Beide bekommen dieselbe Realzeit von rund zehn Sekunden.
    loop60.driver.runFrames(600, 1000 / 60);
    loop144.driver.runFrames(1440, 1000 / 144);

    const common = Math.min(at60.tickTimes.length, at144.tickTimes.length);
    expect(common).toBeGreaterThan(900);
    for (let i = 0; i < common; i++) {
      expect(at144.tickTimes[i]).toBe(at60.tickTimes[i]);
    }
    // Nach gleicher Realzeit ist auch die Zahl der Takte gleich.
    expect(Math.abs(at60.tickTimes.length - at144.tickTimes.length)).toBeLessThanOrEqual(1);
  });

  it('führt dieselbe Eingabefolge bei 60 Hz und 144 Hz zum selben Ergebnis', () => {
    // Eingaben zu Realzeitpunkten, die absichtlich nicht auf Bildgrenzen liegen.
    const inputs = [123, 456, 789, 1500, 1507, 2345, 4001, 5555, 7777, 9999];
    const sim60 = new ScheduledInputSimulation(inputs);
    const sim144 = new ScheduledInputSimulation(inputs);
    const sim30 = new ScheduledInputSimulation(inputs);
    const a = makeLoop(sim60);
    const b = makeLoop(sim144);
    const c = makeLoop(sim30);

    a.loop.start();
    b.loop.start();
    c.loop.start();
    a.driver.runFrames(660, 1000 / 60);
    b.driver.runFrames(1584, 1000 / 144);
    c.driver.runFrames(330, 1000 / 30);

    expect(sim60.value).toBeGreaterThan(0);
    expect(sim144.value).toBe(sim60.value);
    expect(sim30.value).toBe(sim60.value);
  });
});

describe('GameLoop — Aufholen ohne Aufschaukeln', () => {
  it('holt höchstens MAX_CATCHUP_TICKS Takte je Bild nach', () => {
    const sim = new RecordingSimulation();
    const { loop, driver } = makeLoop(sim);
    loop.start();
    driver.runFrames(1, 0); // erstes Bild setzt nur den Zeitstempel

    const before = sim.tickTimes.length;
    driver.runFrameAfterGap(5000); // fünf Sekunden Rückstand
    const ticksInFrame = sim.tickTimes.length - before;

    expect(ticksInFrame).toBe(MAX_CATCHUP_TICKS);
    expect(loop.droppedTickCount).toBe(5000 / TICK_MS - MAX_CATCHUP_TICKS);
  });

  it('lässt die Takt-Zeitpunkte auch nach verworfener Zeitschuld lückenlos', () => {
    const sim = new RecordingSimulation();
    const { loop, driver } = makeLoop(sim);
    loop.start();
    driver.runFrames(1, 0);
    driver.runFrameAfterGap(5000);
    driver.runFrames(60, 1000 / 60);

    for (let i = 0; i < sim.tickTimes.length; i++) {
      expect(sim.tickTimes[i]).toBe(i * TICK_MS);
    }
    expect(loop.timeMs).toBe(sim.tickTimes.length * TICK_MS);
  });
});

describe('GameLoop — Zeichnen', () => {
  it('zeichnet genau ein Bild je Bild-Anforderung', () => {
    const renderer = new RecordingRenderer();
    const { loop, driver } = makeLoop(new RecordingSimulation(), renderer);
    loop.start();
    driver.runFrames(120, 1000 / 60);

    expect(renderer.simTimes.length).toBe(120);
    expect(loop.frameCount).toBe(120);
  });

  it('liefert alpha stets in [0, 1)', () => {
    const renderer = new RecordingRenderer();
    const { loop, driver } = makeLoop(new RecordingSimulation(), renderer);
    loop.start();
    driver.runFrames(500, 1000 / 144);
    driver.runFrameAfterGap(3000);
    driver.runFrames(200, 1000 / 60);

    for (const alpha of renderer.alphas) {
      expect(alpha).toBeGreaterThanOrEqual(0);
      expect(alpha).toBeLessThan(1);
    }
    expect(loop.alpha).toBeGreaterThanOrEqual(0);
    expect(loop.alpha).toBeLessThan(1);
  });

  it('zeichnet bei 144 Hz mehr Bilder als Takte gerechnet werden', () => {
    const sim = new RecordingSimulation();
    const renderer = new RecordingRenderer();
    const { loop, driver } = makeLoop(sim, renderer);
    loop.start();
    driver.runFrames(1440, 1000 / 144);

    expect(renderer.simTimes.length).toBe(1440);
    expect(sim.tickTimes.length).toBeLessThan(renderer.simTimes.length);
    // Genau das ist der Zweck der Interpolation: mehr Bilder als Takte.
    expect(loop.frameCount).toBeGreaterThan(loop.tickCount);
  });
});

describe('GameLoop — Steuerung', () => {
  it('rechnet nach stop() nicht weiter', () => {
    const sim = new RecordingSimulation();
    const { loop, driver } = makeLoop(sim);
    loop.start();
    driver.runFrames(60, 1000 / 60);
    const ticks = sim.tickTimes.length;
    loop.stop();
    driver.runFrames(60, 1000 / 60);

    expect(sim.tickTimes.length).toBe(ticks);
    expect(loop.isRunning).toBe(false);
  });

  it('rechnet mit advanceTicks ohne Bild weiter', () => {
    const sim = new RecordingSimulation();
    const { loop } = makeLoop(sim);
    loop.advanceTicks(100);

    expect(sim.tickTimes.length).toBe(100);
    expect(loop.timeMs).toBe(1000);
    expect(sim.tickTimes[99]).toBe(990);
  });

  it('setzt die Kampfzeit mit reset() auf 0 zurück', () => {
    const sim = new RecordingSimulation();
    const { loop } = makeLoop(sim);
    loop.advanceTicks(10);
    loop.reset();
    loop.advanceTicks(1);

    expect(loop.timeMs).toBe(TICK_MS);
    expect(sim.tickTimes[10]).toBe(0);
  });
});
