// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from '../../src/core/app.js';
import { analyzeRun } from '../../src/analysis/run-analysis.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { PHASE_ENDED, PHASE_FIGHTING, PHASE_PREPARE } from '../../src/core/phases.js';

const frames: FrameRequestCallback[] = [];
const drawn: { text: string; x: number; y: number }[] = [];
const apps: App[] = [];

beforeEach(() => {
  frames.length = 0;
  drawn.length = 0;
  vi.spyOn(window, 'requestAnimationFrame').mockImplementation((callback) => {
    frames.push(callback);
    return frames.length;
  });
  vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {});
  const ctx: unknown = new Proxy(
    {},
    {
      get: (_target, key) => {
        if (key === 'fillText')
          return (text: string, x: number, y: number) => drawn.push({ text, x, y });
        if (key === 'measureText') return () => ({ width: 16 });
        return () => ctx;
      },
      set: () => true,
    },
  );
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(
    ctx as CanvasRenderingContext2D,
  );
  localStorage.clear();
  document.body.innerHTML = '';
});

afterEach(() => {
  for (const app of apps) app.stop();
  apps.length = 0;
  vi.restoreAllMocks();
});

/** Draw through the actual App → GameLoop → HudRenderer connection. */
function frame(): void {
  drawn.length = 0;
  const callback = frames.shift();
  if (callback === undefined) throw new Error('The app did not request a frame');
  callback(0);
}

function practice(): App {
  const parent = document.createElement('div');
  document.body.appendChild(parent);
  const app = new App(parent);
  apps.push(app);
  vi.spyOn(app.audio, 'init').mockResolvedValue();
  app.start();
  app.openHome();
  const enter = app.home.root.querySelector<HTMLButtonElement>('.training-start');
  if (enter === null) throw new Error('Missing training controls');
  enter.click();
  frame();
  return app;
}

function start(app: App): void {
  expect(app.ready()).toBe(true);
  app.session.startFight(app.loop.timeMs);
  frame();
}

function valueAt(y: number): string | undefined {
  return drawn.find((entry) => entry.x === 1830 && entry.y === y)?.text;
}

function resultValue(app: App, label: string): string | null {
  const parent = app.surface.canvas.parentElement;
  if (parent === null) throw new Error('Missing app container');
  const row = [...parent.querySelectorAll('.rota-result .rota-row')].find(
    (node) => node.querySelector('.rota-row-label')?.textContent === label,
  );
  return row?.querySelector('.rota-row-value')?.textContent ?? null;
}

describe('live training in the actual app', () => {
  it('shows no deadline and renders the three metrics only during practice', () => {
    const app = practice();
    expect(app.session.phase).toBe(PHASE_PREPARE);
    expect(drawn.map((entry) => entry.text)).toContain('No time limit');
    expect(drawn.map((entry) => entry.text)).not.toContain('OVERPRESSURE IN');
    expect(drawn.map((entry) => entry.text)).not.toContain('IDLE');
    start(app);
    expect(drawn.map((entry) => entry.text)).toEqual(
      expect.arrayContaining(['IDLE', 'MISSED CASTS', 'SYNERGY']),
    );
    expect(valueAt(660)).toBe('0.0 s · 0 %');
    expect(valueAt(704)).toBe('0');
    expect(valueAt(748)).toBe('n/a');
    expect(frames).toHaveLength(1);
    const passes = app.surface.drawPasses;
    frame();
    expect(app.surface.drawPasses).toBe(passes + 1);
    expect(frames).toHaveLength(1);
  });

  it('displays measured idle, missed windows and a real used synergy from the current session', () => {
    const app = practice();
    // A fixed, real pair isolates the readout from spellbook acquisition.
    app.session.bind(0, app.combat.indexOf('canker'));
    app.session.bind(1, app.combat.indexOf('fester'));
    start(app);
    app.pressQueue.push(0, 0, 0, 0);
    app.loop.advanceTicks(150);
    app.pressQueue.push(1, 150, 1500, 150);
    app.loop.advanceTicks(350);
    frame();
    expect(app.session.phase).toBe(PHASE_FIGHTING);
    const summary = app.session.summary(app.loop.timeMs);
    expect(summary.synergyUsed).toBe(1);
    expect(summary.synergyAvailable).toBe(1);
    expect(summary.lostTimeMs).toBeGreaterThan(0);
    const analysis = analyzeRun({
      summary,
      gcdMs: GCD_MS,
      spells: [],
      synergyBonusDamage: 0,
      missedInterrupts: 0,
      damagePerLandedBossCast: 0,
      bossHealed: 0,
    });
    expect(analysis.missedCastWindows).toBeGreaterThan(0);
    expect(valueAt(660)).toBe(
      `${(summary.lostTimeMs / 1000).toFixed(1)} s · ${(summary.lostTimeShare * 100).toFixed(0)} %`,
    );
    expect(valueAt(704)).toBe(String(analysis.missedCastWindows));
    expect(valueAt(748)).toBe('1 / 1');
  });

  it('ends without a false reserve and resets all live values for the next run', () => {
    const app = practice();
    start(app);
    app.loop.advanceTicks(300);
    frame();
    expect(valueAt(660)).not.toBe('0.0 s · 0 %');
    expect(app.wipe()).toBe(true);
    frame();
    expect(app.session.phase).toBe(PHASE_ENDED);
    expect(resultValue(app, 'Reserve')).toBe('—');
    expect(drawn.map((entry) => entry.text)).not.toContain('IDLE');
    app.backToMenu();
    frame();
    expect(drawn.map((entry) => entry.text)).not.toContain('IDLE');
    start(app);
    expect(valueAt(660)).toBe('0.0 s · 0 %');
    expect(valueAt(704)).toBe('0');
    expect(valueAt(748)).toBe('n/a');
  });

  it('restores the ordinary timer and result reserve after leaving training', () => {
    const app = practice();
    start(app);
    app.loop.advanceTicks(100);
    app.wipe();
    frame();
    app.selectBoss('first-crucible');
    frame();
    expect(drawn.map((entry) => entry.text)).toContain('OVERPRESSURE IN');
    expect(drawn.map((entry) => entry.text)).not.toContain('No time limit');
    start(app);
    app.loop.advanceTicks(100);
    frame();
    expect(drawn.map((entry) => entry.text)).not.toContain('IDLE');
    app.wipe();
    frame();
    expect(resultValue(app, 'Reserve')).toBe(
      `${(app.session.overpressureRemainingMs(app.loop.timeMs) / 1000).toFixed(2)} s`,
    );
  });
});
