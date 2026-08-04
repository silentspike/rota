import { describe, expect, it } from 'vitest';
import {
  CLASS_ADDITIVE,
  CLASS_MULTIPLICATIVE,
  STAT_ADDITIVE,
  STAT_MULTIPLICATIVE,
  classOfStat,
  formatStat,
  isDistinguishable,
} from '../../src/gear/format.js';

const additiv = { label: 'ESSENCE', share: 0.2, kind: STAT_ADDITIVE };
const multiplikativ = { label: 'CATALYSIS', share: 0.2, kind: STAT_MULTIPLICATIVE };

describe('Additiv und multiplikativ sind unterscheidbar (AC4)', () => {
  it('schreibt sie verschieden', () => {
    expect(formatStat(additiv)).toBe('+20%');
    expect(formatStat(multiplikativ)).toBe('x1.20');
  });

  it('färbt sie verschieden', () => {
    expect(classOfStat(STAT_ADDITIVE)).toBe(CLASS_ADDITIVE);
    expect(classOfStat(STAT_MULTIPLICATIVE)).toBe(CLASS_MULTIPLICATIVE);
    expect(CLASS_ADDITIVE).not.toBe(CLASS_MULTIPLICATIVE);
  });

  it('verlangt BEIDE Merkmale, nicht nur eines', () => {
    // Eine Farbe allein trägt nicht: Etwa jeder zwölfte Mann sieht Rot und
    // Grün nicht auseinander. Das Zeichen bleibt lesbar, auch wenn die Farbe
    // nichts sagt.
    expect(isDistinguishable(additiv, multiplikativ)).toBe(true);
    // Gleiche Art ist nie unterscheidbar — auch bei verschiedenem Wert nicht.
    expect(isDistinguishable(additiv, { ...additiv, share: 0.9 })).toBe(false);
  });

  it('bleibt bei GLEICHEM Anteil unterscheidbar', () => {
    // Der eigentliche Fall: Zwei Werte mit derselben Zahl, die sich völlig
    // verschieden verhalten. Ohne Kennzeichnung sähen sie gleich aus — und
    // der Spieler müsste außerhalb des Spiels nachschlagen, warum der eine
    // Gegenstand viel besser ist.
    expect(additiv.share).toBe(multiplikativ.share);
    expect(formatStat(additiv)).not.toBe(formatStat(multiplikativ));
  });
});

describe('Die Schreibweise selbst', () => {
  it('schreibt den Multiplikator aus, statt ihn als Prozent zu tarnen', () => {
    // „×20 %" läse sich als „ein Fünftel davon" — das Gegenteil des
    // Gemeinten. Der ausgeschriebene Faktor lässt sich nicht falsch verstehen.
    expect(formatStat({ ...multiplikativ, share: 0.2 })).toBe('x1.20');
    expect(formatStat({ ...multiplikativ, share: 0.05 })).toBe('x1.05');
    expect(formatStat({ ...multiplikativ, share: 1 })).toBe('x2.00');
  });

  it('trägt beim additiven Wert immer ein Vorzeichen', () => {
    expect(formatStat({ ...additiv, share: 0.075 })).toBe('+7.5%');
    expect(formatStat({ ...additiv, share: 0 })).toBe('+0%');
    // Ein Malus ist denkbar (Gegenstände mit Nachteil) und darf kein „+—"
    // ergeben.
    expect(formatStat({ ...additiv, share: -0.1 })).toBe('-10%');
  });

  it('rundet auf eine Nachkommastelle, statt Kommastellen auszuwerfen', () => {
    // 0.1 + 0.2 = 0.30000000000000004. Ungerundet stünde das am Gegenstand.
    expect(formatStat({ ...additiv, share: 0.1 + 0.2 })).toBe('+30%');
    expect(formatStat({ ...additiv, share: 1 / 3 })).toBe('+33.3%');
  });
});
