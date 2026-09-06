// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from '../../src/core/app.js';
import { buildDummy, DEFAULT_DUMMY } from '../../src/analysis/training.js';
import { PHASE_COUNTDOWN, PHASE_FIGHTING, PHASE_PREPARE } from '../../src/core/phases.js';

beforeEach(() => {
  const context: unknown = new Proxy({}, { get: () => () => context, set: () => true });
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(
    context as CanvasRenderingContext2D,
  );
  localStorage.clear();
  document.body.innerHTML = '';
});

afterEach(() => vi.restoreAllMocks());

function appWithForm() {
  const root = document.createElement('div');
  document.body.appendChild(root);
  const app = new App(root);
  app.openHome();
  const form = app.home.root.querySelector<HTMLFormElement>('.training-settings');
  if (form === null) throw new Error('Missing training form');
  const input = (name: string): HTMLInputElement => {
    const field = form.querySelector<HTMLInputElement>(`[name="${name}"]`);
    if (field === null) throw new Error(`Missing ${name}`);
    return field;
  };
  const enter = (): void => {
    const button = form.querySelector<HTMLButtonElement>('.training-start');
    if (button === null) throw new Error('Missing training button');
    button.click();
  };
  return { app, form, input, enter };
}

function practice(resistance: string, damage: boolean, casts: boolean) {
  const controls = appWithForm();
  controls.input('resistance').value = resistance;
  controls.input('dealsDamage').checked = damage;
  controls.input('castsSpells').checked = casts;
  controls.enter();
  expect(controls.app.ready()).toBe(true);
  controls.app.session.startFight(0);
  controls.app.pressQueue.push(0, 0, 0, 0);
  for (let timeMs = 0; timeMs <= 15_000; timeMs += 10) controls.app.session.tick(timeMs);
  return controls.app;
}

describe('training settings through Player Home', () => {
  it('keeps the default dummy and both independent options off', () => {
    const { app, input, enter } = appWithForm();
    expect(input('resistance').value).toBe('0');
    expect(input('dealsDamage').checked).toBe(false);
    expect(input('castsSpells').checked).toBe(false);
    enter();
    expect(app.combat.boss).toEqual(buildDummy(DEFAULT_DUMMY, app.combat.school.school));
    expect(app.home.isVisible).toBe(false);
  });

  it('turns the entered resistance into actual damage reduction', () => {
    const neutral = practice('0', false, false);
    const resistant = practice('50', false, false);
    expect(neutral.combat.damageDealt).toBeGreaterThan(0);
    expect(resistant.combat.damageDealt).toBe(neutral.combat.damageDealt / 2);
    expect(resistant.combat.boss.resistances['CALCINATION']).toBe(0.5);
    expect(resistant.combat.boss.resistances['PUTREFACTION']).toBeUndefined();
  });

  it.each([
    [false, false],
    [false, true],
    [true, false],
    [true, true],
  ])('damage=%s and casts=%s change the actual fight independently', (damage, casts) => {
    const app = practice('0', damage, casts);
    expect(app.combat.damageTaken > 0).toBe(damage);
    expect(app.combat.interruptsMissed > 0).toBe(casts);
    expect(app.session.phase).toBe(PHASE_FIGHTING);
  });

  it.each(['', '-1', '100.1', 'Infinity', 'not-a-number'])(
    'rejects invalid resistance %j without changing the selected fight',
    (value) => {
      const { app, form, input, enter } = appWithForm();
      const original = app.combat.boss;
      input('resistance').value = value;
      enter();
      expect(app.combat.boss).toBe(original);
      expect(app.home.isVisible).toBe(true);
      expect(form.querySelector('[role="alert"]')?.textContent).toBe(
        'Enter resistance from 0 to 100%.',
      );
      expect(input('resistance').getAttribute('aria-invalid')).toBe('true');
      // Correcting the field makes the same form usable again.
      input('resistance').value = '25';
      input('resistance').dispatchEvent(new Event('input', { bubbles: true }));
      enter();
      expect(app.combat.boss.resistances['CALCINATION']).toBe(0.75);
    },
  );

  it.each(['0', '100', '12.5'])('accepts boundary and fractional resistance %s', (value) => {
    const { app, input, enter } = appWithForm();
    input('resistance').value = value;
    enter();
    expect(app.combat.boss.resistances['CALCINATION']).toBe(1 - Number(value) / 100);
  });

  it.each([PHASE_COUNTDOWN, PHASE_FIGHTING])(
    'does not replace an active run in phase %s',
    (phase) => {
      const { app, form, input, enter } = appWithForm();
      enter();
      expect(app.ready()).toBe(true);
      if (phase === PHASE_FIGHTING) app.session.startFight(0);
      const original = app.combat.boss;
      input('resistance').value = '50';
      enter();
      expect(app.combat.boss).toBe(original);
      expect(app.session.phase).toBe(phase);
      expect(form.querySelector('[role="alert"]')?.textContent).toContain('Leave the current run');
    },
  );

  it('keeps the chosen conditions when returning for another training run', () => {
    const { app, input, enter } = appWithForm();
    input('resistance').value = '25';
    input('castsSpells').checked = true;
    enter();
    app.backToMenu();
    app.openHome();
    const currentForm = app.home.root.querySelector<HTMLFormElement>('.training-settings');
    const currentResistance = currentForm?.querySelector<HTMLInputElement>('[name="resistance"]');
    const currentCasts = currentForm?.querySelector<HTMLInputElement>('[name="castsSpells"]');
    const currentEnter = currentForm?.querySelector<HTMLButtonElement>('.training-start');
    if (!currentResistance || !currentCasts || !currentEnter) {
      throw new Error('Missing current training controls after reopening Home');
    }
    expect(currentResistance.value).toBe('25');
    expect(currentCasts.checked).toBe(true);
    // Start through the current DOM and prove that its newly edited values apply.
    currentResistance.value = '50';
    currentCasts.checked = false;
    currentEnter.click();
    expect(app.combat.boss.resistances['CALCINATION']).toBe(0.5);
    expect(app.combat.boss.timeline).toEqual([]);
  });

  it('keeps keyboard editing and form submission separate from combat shortcuts', () => {
    const { app, form, input } = appWithForm();
    vi.spyOn(app.loop, 'start').mockImplementation(() => {});
    vi.spyOn(app.audio, 'init').mockResolvedValue();
    app.start();
    try {
      input('resistance').value = '50';
      for (const code of ['KeyE', 'Enter', 'Space']) {
        const key = new KeyboardEvent('keydown', { code, bubbles: true, cancelable: true });
        input('resistance').dispatchEvent(key);
        expect(key.defaultPrevented).toBe(false);
        expect(app.input.pressed).toBe(0);
        expect(app.session.phase).toBe(PHASE_PREPARE);
      }
      // jsdom does not synthesize the native submit default action for Enter.
      form.requestSubmit();
      expect(app.combat.boss.resistances['CALCINATION']).toBe(0.5);
      expect(app.session.phase).toBe(PHASE_PREPARE);
      window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Enter' }));
      expect(app.session.phase).toBe(PHASE_COUNTDOWN);
    } finally {
      app.stop();
    }
  });
});
