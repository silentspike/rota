import { describe, expect, it } from 'vitest';
import {
  KEYS_PER_LAYER,
  LAYER_DIRECT,
  LAYER_SHIFT,
  NO_SLOT,
  SLOT_CODES,
  SLOT_COUNT,
  SPACE_POSITION,
  codeOfSlot,
  columnOfPosition,
  isSlot,
  layerOfSlot,
  positionForCode,
  positionOfSlot,
  rowOfPosition,
  slotFor,
  slotOf,
} from '../../src/input/keys.js';

describe('Tastenplätze', () => {
  it('hat dreizehn Stellen und zwei Ebenen — 26 Belegungen', () => {
    expect(SLOT_CODES.length).toBe(KEYS_PER_LAYER);
    expect(KEYS_PER_LAYER).toBe(13);
    expect(SLOT_COUNT).toBe(26);
  });

  it('spricht die untere Reihe über KeyZ an, nicht über KeyY', () => {
    // GAME-DESIGN 9.1 nennt die Reihe „Y X C V" — das ist die Beschriftung auf
    // einer deutschen Tastatur. Die Kennung derselben Stelle ist KeyZ.
    expect(SLOT_CODES[8]).toBe('KeyZ');
    expect(positionForCode('KeyY')).toBe(NO_SLOT);
    expect(positionForCode('KeyZ')).toBe(8);
  });

  it('bildet die Tastatur in der richtigen Reihenfolge ab', () => {
    expect(SLOT_CODES.slice(0, 4)).toEqual(['KeyQ', 'KeyW', 'KeyE', 'KeyR']);
    expect(SLOT_CODES.slice(4, 8)).toEqual(['KeyA', 'KeyS', 'KeyD', 'KeyF']);
    expect(SLOT_CODES.slice(8, 12)).toEqual(['KeyZ', 'KeyX', 'KeyC', 'KeyV']);
    expect(SLOT_CODES[SPACE_POSITION]).toBe('Space');
  });

  it('trennt die beiden Ebenen', () => {
    expect(slotFor('KeyQ', false)).toBe(0);
    expect(slotFor('KeyQ', true)).toBe(13);
    expect(slotFor('Space', false)).toBe(12);
    expect(slotFor('Space', true)).toBe(25);
  });

  it('meldet nicht belegte Tasten', () => {
    for (const code of ['KeyT', 'KeyG', 'KeyB', 'Digit1', 'Escape', 'ShiftLeft', 'Enter']) {
      expect(slotFor(code, false)).toBe(NO_SLOT);
      expect(slotFor(code, true)).toBe(NO_SLOT);
    }
  });

  it('rechnet zwischen Platz, Ebene und Stelle widerspruchsfrei um', () => {
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      const layer = layerOfSlot(slot);
      const position = positionOfSlot(slot);
      expect(slotOf(layer, position)).toBe(slot);
      expect(codeOfSlot(slot)).toBe(SLOT_CODES[position]);
      expect(layer === LAYER_DIRECT || layer === LAYER_SHIFT).toBe(true);
    }
  });

  it('vergibt jede Belegung genau einmal', () => {
    const seen = new Set<number>();
    for (const code of SLOT_CODES) {
      for (const shift of [false, true]) {
        const slot = slotFor(code, shift);
        expect(seen.has(slot)).toBe(false);
        seen.add(slot);
      }
    }
    expect(seen.size).toBe(SLOT_COUNT);
  });

  it('ordnet die Stellen in drei Reihen plus Leertaste', () => {
    expect([0, 1, 2, 3].map(rowOfPosition)).toEqual([0, 0, 0, 0]);
    expect([4, 5, 6, 7].map(rowOfPosition)).toEqual([1, 1, 1, 1]);
    expect([8, 9, 10, 11].map(rowOfPosition)).toEqual([2, 2, 2, 2]);
    expect(rowOfPosition(SPACE_POSITION)).toBe(3);
    expect([0, 1, 2, 3].map(columnOfPosition)).toEqual([0, 1, 2, 3]);
    expect([4, 5, 6, 7].map(columnOfPosition)).toEqual([0, 1, 2, 3]);
    expect(columnOfPosition(SPACE_POSITION)).toBe(0);
  });

  it('erkennt gültige Belegungen', () => {
    expect(isSlot(0)).toBe(true);
    expect(isSlot(25)).toBe(true);
    expect(isSlot(26)).toBe(false);
    expect(isSlot(-1)).toBe(false);
    expect(isSlot(1.5)).toBe(false);
  });

  it('belegt keine Taste, die Strg oder Alt braucht', () => {
    // Design-Gesetz 2: nur einfache Tasten und Umschalt.
    for (const code of SLOT_CODES) {
      expect(code.startsWith('Control')).toBe(false);
      expect(code.startsWith('Alt')).toBe(false);
      expect(code.startsWith('Meta')).toBe(false);
    }
  });
});
