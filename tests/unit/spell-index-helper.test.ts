import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';
import { spellIndex } from '../helpers/spell-index.js';

/**
 * Der gemeinsame Helfer hat einen eigenen Fall (Issue #369).
 *
 * Sechs Prüffälle hängen an ihm. Wäre er still falsch, wären sie es alle —
 * und keiner von ihnen prüft ihn, sie *benutzen* ihn nur.
 */
describe('spellIndex', () => {
  it('löst einen bekannten Zauber auf', () => {
    const i = spellIndex('kindle');
    expect(CALCINATION.value.spells[i]?.id).toBe('kindle');
  });

  it('wirft bei einem unbekannten — statt -1 zurückzugeben', () => {
    // **Der Grund für den Wurf.** `-1` ist ein gültiger Index für „gibt es
    // nicht": Der Prüffall liefe weiter und misst nichts. Der Fehler zeigt
    // stattdessen auf den Tippfehler selbst.
    expect(() => spellIndex('gibtsnicht')).toThrow('gibt es nicht');
  });
});
