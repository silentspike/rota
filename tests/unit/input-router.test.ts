import { beforeEach, describe, expect, it } from 'vitest';
import { InputRouter } from '../../src/input/router.js';
import { PressQueue } from '../../src/input/queue.js';
import { NO_SLOT, slotFor } from '../../src/input/keys.js';

interface FakeKeyEvent {
  code: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  repeat: boolean;
  timeStamp: number;
  preventDefault(): void;
}

let defaultPrevented = 0;

function keyEvent(code: string, options: Partial<FakeKeyEvent> = {}): FakeKeyEvent {
  return {
    code,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    repeat: false,
    timeStamp: 0,
    preventDefault: () => {
      defaultPrevented++;
    },
    ...options,
  };
}

/** Ersatz für `window`: hält Zuhörer und stellt Ereignisse zu. */
class FakeWindow {
  private readonly listeners = new Map<string, ((event: FakeKeyEvent) => void)[]>();

  addEventListener(type: string, fn: (event: FakeKeyEvent) => void): void {
    const list = this.listeners.get(type) ?? [];
    list.push(fn);
    this.listeners.set(type, list);
  }

  removeEventListener(type: string, fn: (event: FakeKeyEvent) => void): void {
    const list = this.listeners.get(type);
    if (list === undefined) return;
    const index = list.indexOf(fn);
    if (index >= 0) list.splice(index, 1);
  }

  dispatch(type: string, event: FakeKeyEvent): void {
    for (const fn of this.listeners.get(type) ?? []) fn(event);
  }

  get listenerCount(): number {
    let total = 0;
    for (const list of this.listeners.values()) total += list.length;
    return total;
  }
}

interface Harness {
  readonly router: InputRouter;
  readonly queue: PressQueue;
  readonly target: FakeWindow;
  /** Wie oft `onFirstInput` gerufen wurde. */
  readonly firstInputs: () => number;
  /** Takt und Bildzähler weiterstellen, als wäre Zeit vergangen. */
  readonly bump: (tick: number, frame: number) => void;
}

function setup(options: { tick?: number; frame?: number } = {}): Harness {
  const queue = new PressQueue();
  const target = new FakeWindow();
  const state = { tick: options.tick ?? 0, frame: options.frame ?? 0, firstInputs: 0 };
  const router = new InputRouter({
    queue,
    nextTickIndex: () => state.tick,
    frameIndex: () => state.frame,
    onFirstInput: () => {
      state.firstInputs++;
    },
  });
  router.attach(target as unknown as Window);
  return {
    router,
    queue,
    target,
    firstInputs: () => state.firstInputs,
    bump: (tick: number, frame: number): void => {
      state.tick = tick;
      state.frame = frame;
    },
  };
}

beforeEach(() => {
  defaultPrevented = 0;
});

describe('InputRouter — physische Tastenpositionen', () => {
  it('reagiert auf die Kennung, nicht auf das Zeichen (AC1)', () => {
    const { queue, target } = setup();
    // Dieselbe Stelle, zwei Layouts: deutsch liefert "y", englisch "z".
    // Der `code` ist in beiden Fällen KeyZ — und nur der zählt.
    target.dispatch('keydown', keyEvent('KeyZ', { timeStamp: 100 }));
    expect(queue.size).toBe(1);
    expect(queue.frontSlot()).toBe(slotFor('KeyZ', false));
  });

  it('nimmt die Umschalt-Ebene als eigene Belegung', () => {
    const { queue, target } = setup();
    target.dispatch('keydown', keyEvent('KeyQ', { timeStamp: 1 }));
    target.dispatch('keyup', keyEvent('KeyQ'));
    target.dispatch('keydown', keyEvent('KeyQ', { shiftKey: true, timeStamp: 2 }));

    expect(queue.size).toBe(2);
    expect(queue.frontSlot()).toBe(0);
    queue.shift();
    expect(queue.frontSlot()).toBe(13);
  });

  it('übergeht Tastenfolgen mit Strg, Alt oder Meta (Design-Gesetz 2)', () => {
    const { router, queue, target } = setup();
    target.dispatch('keydown', keyEvent('KeyQ', { ctrlKey: true }));
    target.dispatch('keydown', keyEvent('KeyW', { altKey: true }));
    target.dispatch('keydown', keyEvent('KeyE', { metaKey: true }));

    expect(queue.size).toBe(0);
    expect(router.modifierIgnores).toBe(3);
  });

  it('zählt nicht belegte Tasten, ohne sie weiterzureichen', () => {
    const { router, queue, target } = setup();
    target.dispatch('keydown', keyEvent('KeyT'));
    target.dispatch('keydown', keyEvent('Escape'));

    expect(queue.size).toBe(0);
    expect(router.unbound).toBe(2);
  });

  it('unterdrückt die Scroll-Wirkung der Leertaste', () => {
    const { target } = setup();
    target.dispatch('keydown', keyEvent('Space'));
    expect(defaultPrevented).toBe(1);
  });
});

describe('InputRouter — Keypress = Trigger (AC3)', () => {
  it('löst beim Gedrückthalten genau eine Aktion aus', () => {
    const { router, queue, target } = setup();
    target.dispatch('keydown', keyEvent('KeyQ', { timeStamp: 10 }));
    // So verhält sich die Wiederholung des Betriebssystems.
    for (let i = 0; i < 30; i++) {
      target.dispatch('keydown', keyEvent('KeyQ', { repeat: true, timeStamp: 40 + i * 30 }));
    }

    expect(queue.size).toBe(1);
    expect(router.pressed).toBe(1);
    expect(router.suppressedRepeats).toBe(30);
  });

  it('unterdrückt Wiederholungen auch ohne die Kennzeichnung des Browsers', () => {
    const { router, queue, target } = setup();
    target.dispatch('keydown', keyEvent('KeyA', { timeStamp: 1 }));
    target.dispatch('keydown', keyEvent('KeyA', { timeStamp: 2 }));
    target.dispatch('keydown', keyEvent('KeyA', { timeStamp: 3 }));

    expect(queue.size).toBe(1);
    expect(router.suppressedRepeats).toBe(2);
  });

  it('lässt nach dem Loslassen wieder einen Druck zu', () => {
    const { queue, target } = setup();
    target.dispatch('keydown', keyEvent('KeyA', { timeStamp: 1 }));
    target.dispatch('keyup', keyEvent('KeyA'));
    target.dispatch('keydown', keyEvent('KeyA', { timeStamp: 2 }));

    expect(queue.size).toBe(2);
  });

  it('sperrt eine Stelle in beiden Ebenen, solange sie unten ist', () => {
    const { router, queue, target } = setup();
    target.dispatch('keydown', keyEvent('KeyQ', { timeStamp: 1 }));
    // Umschalt dazunehmen, ohne Q loszulassen, ergibt keinen zweiten Druck:
    // die Taste ist physisch nicht neu betätigt worden.
    target.dispatch('keydown', keyEvent('KeyQ', { shiftKey: true, timeStamp: 2 }));

    expect(queue.size).toBe(1);
    expect(router.suppressedRepeats).toBe(1);
  });

  it('setzt den Gedrückt-Zustand beim Fensterwechsel zurück', () => {
    const { router, target } = setup();
    target.dispatch('keydown', keyEvent('KeyQ', { timeStamp: 1 }));
    expect(router.isHeld(0)).toBe(true);
    target.dispatch('blur', keyEvent('KeyQ'));
    expect(router.isHeld(0)).toBe(false);
  });
});

describe('InputRouter — Zuordnung zu Takt und Bild', () => {
  it('weist jedem Druck den nächsten zu rechnenden Takt zu', () => {
    const { queue, target, bump } = setup({ tick: 7, frame: 3 });
    target.dispatch('keydown', keyEvent('KeyQ', { timeStamp: 70.5 }));
    bump(12, 5);
    target.dispatch('keydown', keyEvent('KeyW', { timeStamp: 120.5 }));

    expect(queue.frontTick()).toBe(7);
    expect(queue.frontStamp()).toBe(70.5);
    expect(queue.frontFrame()).toBe(3);
    queue.shift();
    expect(queue.frontTick()).toBe(12);
    expect(queue.frontFrame()).toBe(5);
  });

  it('meldet den ersten Tastendruck genau einmal', () => {
    const setupResult = setup();
    setupResult.target.dispatch('keydown', keyEvent('KeyQ', { timeStamp: 1 }));
    setupResult.target.dispatch('keyup', keyEvent('KeyQ'));
    setupResult.target.dispatch('keydown', keyEvent('KeyW', { timeStamp: 2 }));

    expect(setupResult.firstInputs()).toBe(1);
  });
});

describe('InputRouter — An- und Abmelden', () => {
  it('meldet alle Zuhörer wieder ab', () => {
    const { router, target } = setup();
    expect(target.listenerCount).toBe(3);
    router.detach(target as unknown as Window);
    expect(target.listenerCount).toBe(0);
  });

  it('reagiert nach dem Abmelden nicht mehr', () => {
    const { router, queue, target } = setup();
    router.detach(target as unknown as Window);
    target.dispatch('keydown', keyEvent('KeyQ', { timeStamp: 1 }));
    expect(queue.size).toBe(0);
  });

  it('meldet sich nicht doppelt an', () => {
    const { router, target } = setup();
    router.attach(target as unknown as Window);
    expect(target.listenerCount).toBe(3);
  });
});

describe('InputRouter — Belegungstabelle', () => {
  it('erreicht alle 26 Belegungen über echte Tastendrücke', () => {
    const { queue, target } = setup();
    const seen = new Set<number>();
    for (const shift of [false, true]) {
      for (const code of [
        'KeyQ',
        'KeyW',
        'KeyE',
        'KeyR',
        'KeyA',
        'KeyS',
        'KeyD',
        'KeyF',
        'KeyZ',
        'KeyX',
        'KeyC',
        'KeyV',
        'Space',
      ]) {
        target.dispatch('keydown', keyEvent(code, { shiftKey: shift, timeStamp: 1 }));
        target.dispatch('keyup', keyEvent(code));
      }
    }
    while (queue.size > 0) {
      seen.add(queue.frontSlot());
      queue.shift();
    }

    expect(seen.size).toBe(26);
    expect(Math.min(...seen)).toBe(0);
    expect(Math.max(...seen)).toBe(25);
    expect(seen.has(NO_SLOT)).toBe(false);
  });
});
