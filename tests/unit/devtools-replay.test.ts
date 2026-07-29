import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { combatChecksum, mixFloat, mixInt } from '../../src/core/checksum.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GameLoop, TICK_MS, type Renderer, type Simulation } from '../../src/core/loop.js';
import { recordingNames, recordings, replayRecording } from '../../src/devtools/recording.js';

function makeCombat(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('Zustands-Prüfsumme', () => {
  it('ändert sich, sobald sich der Zustand ändert', () => {
    const combat = makeCombat();
    const vorher = combatChecksum(combat, 0);
    combat.cast(combat.indexOf('kindle'), 0);
    expect(combatChecksum(combat, 0)).not.toBe(vorher);
  });

  it('ist bei gleichem Zustand gleich', () => {
    const a = makeCombat();
    const b = makeCombat();
    a.cast(a.indexOf('kindle'), 0);
    b.cast(b.indexOf('kindle'), 0);
    expect(combatChecksum(a, 500)).toBe(combatChecksum(b, 500));
  });

  it('unterscheidet einen einzigen Schadenspunkt', () => {
    const a = makeCombat();
    const b = makeCombat();
    a.setResource(14);
    b.setResource(15); // 168 gegen 360 Schaden
    a.cast(a.indexOf('pyre'), 0);
    b.cast(b.indexOf('pyre'), 0);
    expect(combatChecksum(a, 0)).not.toBe(combatChecksum(b, 0));
  });

  it('unterscheidet Zahlen, die sich im letzten Bit unterscheiden', () => {
    // Über die Bit-Darstellung, nicht über den gerundeten Wert — genau solche
    // Unterschiede sucht der Abgleich mit dem Simulator.
    expect(mixFloat(0, 1)).not.toBe(mixFloat(0, 1 + Number.EPSILON));
    expect(mixInt(0, 1)).not.toBe(mixInt(0, 2));
  });

  it('bleibt im Bereich vorzeichenloser 32 Bit', () => {
    const combat = makeCombat();
    for (let t = 0; t < 500; t += TICK_MS) {
      const hash = combatChecksum(combat, t);
      expect(hash).toBeGreaterThanOrEqual(0);
      expect(hash).toBeLessThan(2 ** 32);
      expect(Number.isInteger(hash)).toBe(true);
    }
  });
});

describe('Aufzeichnungen', () => {
  it('kennt die beiden aus VERIFICATION genannten Namen', () => {
    // `vessel-phases` ist mit Issue #96 dazugekommen — der gemeinsame Prüffall
    // gegen einen Boss mit Phasen und Energieleiste. Die beiden aus VERIFICATION
    // stehen weiterhin vorn und unverändert; deshalb wird auf sie geprüft und
    // nicht auf die Gesamtliste, die mit jedem weiteren Prüffall wächst.
    expect(recordingNames().slice(0, 2)).toEqual(['cooldown-precision', 'cooldown-sweep']);
    expect(recordingNames()).toContain('vessel-phases');
  });

  it('ist eine echte Aufzeichnung, keine abgetippte Liste', () => {
    // GAME-DESIGN 18.4: aufgezeichnete echte Durchläufe. Die Drücke müssen
    // aufsteigend liegen und auf belegten Plätzen.
    const recording = recordings().get('cooldown-precision');
    expect(recording).toBeDefined();
    const presses = recording?.presses ?? [];
    expect(presses.length).toBe(28);
    for (let i = 1; i < presses.length; i++) {
      expect((presses[i] as { tick: number }).tick).toBeGreaterThan(
        (presses[i - 1] as { tick: number }).tick,
      );
    }
    for (const press of presses) expect([0, 1, 2]).toContain(press.slot);
  });

  it('trifft beim Abspielen die Referenz-Rechnung', () => {
    // Dieselben Zahlen wie `python3 tools/step1-check.py` und wie der Prüffall.
    const result = replayRecording(recordings().get('cooldown-precision') as never);
    expect(result.damage).toBe(4240);
    expect(result.killedAtMs).toBe(40_510);
    expect(result.casts).toEqual({ kindle: 20, smoulder: 4, pyre: 4 });
    expect(result.accepted).toBe(28);
    expect(result.rejected).toBe(0);
    expect(result.randomDraws).toBe(0);
  });

  it('liefert bei zwei Wiedergaben dieselbe Prüfsummenfolge (AC1)', () => {
    const recording = recordings().get('cooldown-precision') as never;
    const a = replayRecording(recording, { checksumPerTick: true });
    const b = replayRecording(recording, { checksumPerTick: true });

    expect(a.checksums.length).toBeGreaterThan(4000);
    expect(b.checksums.length).toBe(a.checksums.length);
    // Jede einzelne Prüfsumme, nicht nur die Summe darüber.
    for (let i = 0; i < a.checksums.length; i++) {
      expect(b.checksums[i]).toBe(a.checksums[i]);
    }
    expect(b.sequenceChecksum).toBe(a.sequenceChecksum);
    expect(b.finalChecksum).toBe(a.finalChecksum);
  });

  it('bemerkt eine geänderte Aufzeichnung an der Prüfsumme', () => {
    const original = recordings().get('cooldown-precision') as {
      name: string;
      seed: number;
      ticks: number;
      presses: readonly { tick: number; slot: number }[];
    };
    // Ein einziger Druck einen Takt später — mehr nicht.
    const presses = original.presses.map((p, i) =>
      i === 5 ? { tick: p.tick + 1, slot: p.slot } : p,
    );
    const geaendert = { ...original, presses };
    const a = replayRecording(original as never);
    const b = replayRecording(geaendert as never);

    expect(b.sequenceChecksum).not.toBe(a.sequenceChecksum);
  });

  it('spielt die Blenden-Aufzeichnung ohne Sieg durch', () => {
    const result = replayRecording(recordings().get('cooldown-sweep') as never);
    expect(result.presses).toBeGreaterThan(10);
    expect(result.accepted).toBe(result.presses);
    expect(result.rejected).toBe(0);
  });
});

/** Treiber wie in `loop.test.ts`: Bilder ohne Browser. */
class FrameDriver {
  clockMs = 0;
  private pending: ((timestampMs: number) => void) | null = null;
  readonly requestFrame = (cb: (t: number) => void): number => {
    this.pending = cb;
    return 1;
  };
  readonly cancelFrame = (): void => {
    this.pending = null;
  };
  readonly now = (): number => this.clockMs;
  runFrames(count: number, intervalMs: number): void {
    for (let i = 0; i < count; i++) {
      const cb = this.pending;
      if (cb === null) return;
      this.pending = null;
      cb(this.clockMs);
      this.clockMs += intervalMs;
    }
  }
}

class CountingSimulation implements Simulation {
  ticks = 0;
  tick(): void {
    this.ticks++;
  }
}

const nullRenderer: Renderer = { render: () => undefined };

describe('Zeitsteuerung (AC4)', () => {
  function setup(): { loop: GameLoop; driver: FrameDriver; sim: CountingSimulation } {
    const driver = new FrameDriver();
    const sim = new CountingSimulation();
    const loop = new GameLoop({
      simulation: sim,
      renderer: nullRenderer,
      requestFrame: driver.requestFrame,
      cancelFrame: driver.cancelFrame,
      now: driver.now,
    });
    return { loop, driver, sim };
  }

  it('hält die Simulation an, zeichnet aber weiter', () => {
    const h = setup();
    let frames = 0;
    h.loop.onFrameMeasured = (): void => {
      frames++;
    };
    h.loop.start();
    h.driver.runFrames(60, 1000 / 60);
    const vorher = h.sim.ticks;
    expect(vorher).toBeGreaterThan(50);

    h.loop.setPaused(true);
    h.driver.runFrames(60, 1000 / 60);
    // Kein Takt mehr — aber die Bilder laufen und werden gemessen.
    expect(h.sim.ticks).toBe(vorher);
    expect(frames).toBe(120);
  });

  it('rechnet nach dem Fortsetzen normal weiter, ohne die Pause nachzuholen', () => {
    const h = setup();
    h.loop.start();
    h.driver.runFrames(60, 1000 / 60);
    h.loop.setPaused(true);
    h.driver.runFrames(300, 1000 / 60); // fünf Sekunden Pause
    const stand = h.sim.ticks;
    h.loop.setPaused(false);
    h.driver.runFrames(60, 1000 / 60);

    // Rund eine Sekunde, nicht sechs: Die Pause wird nicht nachgeholt.
    expect(h.sim.ticks - stand).toBeGreaterThan(90);
    expect(h.sim.ticks - stand).toBeLessThan(110);
    expect(h.loop.droppedTickCount).toBe(0);
  });

  it('halbiert und verdoppelt die Taktzahl, ohne die Bildmessung anzufassen', () => {
    const messen = (scale: number): { ticks: number; intervalle: number[] } => {
      const h = setup();
      const intervalle: number[] = [];
      h.loop.onFrameMeasured = (_work, interval): void => {
        intervalle.push(interval);
      };
      h.loop.setTimeScale(scale);
      h.loop.start();
      h.driver.runFrames(120, 1000 / 60); // zwei Sekunden Realzeit
      return { ticks: h.sim.ticks, intervalle };
    };

    const normal = messen(1);
    const zeitlupe = messen(0.5);
    const zeitraffer = messen(2);

    expect(zeitlupe.ticks).toBeCloseTo(normal.ticks / 2, -1);
    expect(zeitraffer.ticks).toBeCloseTo(normal.ticks * 2, -1);

    // Die gemessenen Bildabstände sind in allen drei Fällen dieselben — die
    // Zeitsteuerung verfälscht die Leistungs-Messwerte nicht.
    const mittel = (xs: number[]): number => xs.reduce((a, b) => a + b, 0) / xs.length;
    expect(mittel(zeitlupe.intervalle)).toBeCloseTo(mittel(normal.intervalle), 6);
    expect(mittel(zeitraffer.intervalle)).toBeCloseTo(mittel(normal.intervalle), 6);
    expect(zeitlupe.intervalle.length).toBe(normal.intervalle.length);
  });

  it('lässt den Takt bei 10 ms, egal wie die Zeit skaliert ist', () => {
    const zeiten: number[] = [];
    const driver = new FrameDriver();
    const loop = new GameLoop({
      simulation: { tick: (t: number) => zeiten.push(t) },
      renderer: nullRenderer,
      requestFrame: driver.requestFrame,
      cancelFrame: driver.cancelFrame,
      now: driver.now,
    });
    loop.setTimeScale(0.25);
    loop.start();
    driver.runFrames(240, 1000 / 60);

    for (let i = 0; i < zeiten.length; i++) expect(zeiten[i]).toBe(i * TICK_MS);
  });

  it('weist unsinnige Faktoren zurück', () => {
    const h = setup();
    h.loop.setTimeScale(-1);
    expect(h.loop.timeScale).toBe(1);
    h.loop.setTimeScale(Number.NaN);
    expect(h.loop.timeScale).toBe(1);
    h.loop.setTimeScale(0);
    expect(h.loop.timeScale).toBe(0);
  });

  it('rechnet im Einzelschritt genau die verlangten Takte', () => {
    const h = setup();
    h.loop.setPaused(true);
    h.loop.start();
    h.driver.runFrames(30, 1000 / 60);
    expect(h.sim.ticks).toBe(0);

    h.loop.advanceTicks(7);
    expect(h.sim.ticks).toBe(7);
    expect(h.loop.timeMs).toBe(70);
  });
});
