import { afterEach, describe, expect, it } from 'vitest';
import { FALLBACK_LABELS, KeyLabels } from '../../src/input/layout.js';
import { SLOT_CODES, SPACE_POSITION } from '../../src/input/keys.js';

const originalNavigator = Object.getOwnPropertyDescriptor(globalThis, 'navigator');

/** Setzt einen Ersatz-`navigator` mit oder ohne Keyboard-Map ein. */
function useLayout(map: Record<string, string> | null): void {
  const value =
    map === null
      ? {}
      : {
          keyboard: {
            getLayoutMap: () =>
              Promise.resolve({
                get: (code: string): string | undefined => map[code],
              }),
          },
        };
  Object.defineProperty(globalThis, 'navigator', { value, configurable: true, writable: true });
}

/** Beschriftung einer deutschen Tastatur für die dreizehn Stellen. */
const GERMAN: Record<string, string> = {
  KeyQ: 'q',
  KeyW: 'w',
  KeyE: 'e',
  KeyR: 'r',
  KeyA: 'a',
  KeyS: 's',
  KeyD: 'd',
  KeyF: 'f',
  KeyZ: 'y',
  KeyX: 'x',
  KeyC: 'c',
  KeyV: 'v',
  Space: ' ',
};

/** Dieselben Stellen auf einer englischen Tastatur. */
const ENGLISH: Record<string, string> = { ...GERMAN, KeyZ: 'z' };

afterEach(() => {
  if (originalNavigator === undefined) {
    Reflect.deleteProperty(globalThis, 'navigator');
  } else {
    Object.defineProperty(globalThis, 'navigator', originalNavigator);
  }
});

describe('KeyLabels', () => {
  it('beginnt mit der US-Beschriftung', () => {
    const labels = new KeyLabels();
    expect(labels.source).toBe('fallback');
    for (let i = 0; i < SLOT_CODES.length; i++) {
      expect(labels.label(i)).toBe(FALLBACK_LABELS[i]);
    }
  });

  it('zeigt auf einer deutschen Tastatur Y, wo die Kennung KeyZ heißt (AC2)', async () => {
    useLayout(GERMAN);
    const labels = new KeyLabels();
    await labels.refresh();

    expect(labels.source).toBe('keyboard-map');
    expect(labels.label(8)).toBe('Y');
    expect(labels.label(9)).toBe('X');
  });

  it('zeigt auf einer englischen Tastatur Z an derselben Stelle (AC1/AC2)', async () => {
    useLayout(ENGLISH);
    const labels = new KeyLabels();
    await labels.refresh();

    // Dieselbe Stelle, dieselbe Belegung, andere Beschriftung — genau das ist
    // der Sinn der Trennung von Kennung und Zeichen.
    expect(SLOT_CODES[8]).toBe('KeyZ');
    expect(labels.label(8)).toBe('Z');
  });

  it('schreibt die Leertaste aus, statt ein Leerzeichen zu zeigen', async () => {
    useLayout(GERMAN);
    const labels = new KeyLabels();
    await labels.refresh();

    expect(labels.label(SPACE_POSITION)).toBe('SPACE');
  });

  it('bleibt bei der US-Beschriftung, wenn der Browser keine Auskunft gibt', async () => {
    useLayout(null);
    const labels = new KeyLabels();
    const source = await labels.refresh();

    expect(source).toBe('fallback');
    expect(labels.label(8)).toBe('Z');
  });

  it('behält die bisherige Beschriftung, wenn der Abruf fehlschlägt', async () => {
    Object.defineProperty(globalThis, 'navigator', {
      value: { keyboard: { getLayoutMap: () => Promise.reject(new Error('kaputt')) } },
      configurable: true,
      writable: true,
    });
    const labels = new KeyLabels();
    await labels.refresh();

    expect(labels.source).toBe('fallback');
    expect(labels.label(0)).toBe('Q');
  });

  it('kann das Layout erneut abfragen, wenn der Spieler umschaltet', async () => {
    useLayout(ENGLISH);
    const labels = new KeyLabels();
    await labels.refresh();
    expect(labels.label(8)).toBe('Z');

    useLayout(GERMAN);
    await labels.refresh();
    expect(labels.label(8)).toBe('Y');
  });
});
