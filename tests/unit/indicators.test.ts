import { describe, expect, it } from 'vitest';
import {
  MAX_INDICATORS,
  hiddenIndicatorCount,
  visibleIndicators,
  type MechanicInfo,
} from '../../src/combat/schools/indicators.js';

/**
 * Fünf erfundene Mechaniken.
 *
 * **Der Grund, warum dieser Prüffall im Prüflauf steht und nicht im Browser:**
 * Es gibt heute zwei Schul-Mechaniken. Ein Nachweis über die Oberfläche könnte
 * nur zeigen, dass zwei weniger als vier ist — und das ist keine Prüfung der
 * Regel, sondern eine Prüfung des Bestands. Die Grenze lässt sich nur zeigen,
 * indem man sie überschreitet.
 */
const FUENF: MechanicInfo[] = [
  { id: 'heat', contributors: 3 },
  { id: 'decay', contributors: 2 },
  { id: 'control', contributors: 4 },
  { id: 'shield', contributors: 1 },
  { id: 'flux', contributors: 5 },
];

describe('Nur die aktiv genutzten Mechaniken (AC3)', () => {
  it('lässt eine Mechanik ohne einzahlenden Zauber weg', () => {
    const sichtbar = visibleIndicators([
      { id: 'heat', contributors: 3 },
      { id: 'decay', contributors: 0 },
    ]);
    expect(sichtbar).toEqual(['heat']);
  });

  it('zeigt gar nichts, wenn keine Mechanik genutzt wird', () => {
    expect(visibleIndicators([{ id: 'heat', contributors: 0 }])).toEqual([]);
    expect(visibleIndicators([])).toEqual([]);
  });

  it('zeigt keine Hitze-Anzeige ohne Kalzination auf der Leiste', () => {
    // AC3 wörtlich. Der Fall ist heute erreichbar: Eine Leiste ohne
    // Kalzinations-Zauber lässt `contributors` auf 0 fallen.
    const ohneKalzination = visibleIndicators([
      { id: 'heat', contributors: 0 },
      { id: 'decay', contributors: 2 },
    ]);
    expect(ohneKalzination).not.toContain('heat');
    expect(ohneKalzination).toEqual(['decay']);
  });
});

describe('Höchstens vier Anzeigen (AC4)', () => {
  it('schneidet bei fünf Mechaniken auf vier ab', () => {
    const sichtbar = visibleIndicators(FUENF);
    expect(sichtbar.length).toBe(MAX_INDICATORS);
    expect(sichtbar.length).toBeLessThanOrEqual(4);
  });

  it('behält die Mechaniken mit den meisten einzahlenden Zaubern', () => {
    // flux 5, control 4, heat 3, decay 2 — shield mit 1 fällt weg.
    expect(visibleIndicators(FUENF)).toEqual(['flux', 'control', 'heat', 'decay']);
  });

  it('meldet, wie viele weggelassen wurden', () => {
    // Stillschweigend abzuschneiden wäre der Fehler.
    expect(hiddenIndicatorCount(FUENF)).toBe(1);
    expect(hiddenIndicatorCount(FUENF.slice(0, 4))).toBe(0);
  });

  it('hält die Grenze auch bei zwölf Mechaniken', () => {
    const viele = Array.from({ length: 12 }, (_, i) => ({
      id: `m${String(i).padStart(2, '0')}`,
      contributors: 1,
    }));
    expect(visibleIndicators(viele).length).toBe(MAX_INDICATORS);
    expect(hiddenIndicatorCount(viele)).toBe(8);
  });

  it('gibt bei Gleichstand immer dieselbe Reihenfolge', () => {
    // Sonst springen die Anzeigen zwischen zwei Bildern — und der Blick,
    // der ohnehin dort ist, findet die Zahl nicht wieder.
    const gleich: MechanicInfo[] = [
      { id: 'delta', contributors: 2 },
      { id: 'alpha', contributors: 2 },
      { id: 'charlie', contributors: 2 },
      { id: 'bravo', contributors: 2 },
      { id: 'echo', contributors: 2 },
    ];
    const erste = visibleIndicators(gleich);
    for (let i = 0; i < 50; i++) {
      expect(visibleIndicators([...gleich].reverse())).toEqual(erste);
    }
    expect(erste).toEqual(['alpha', 'bravo', 'charlie', 'delta']);
  });

  it('lässt die Eingabe unverändert', () => {
    // `sort` arbeitet an Ort und Stelle — eine Liste, die der Aufrufer
    // weiterverwendet, darf davon nichts merken.
    const eingabe: MechanicInfo[] = [
      { id: 'heat', contributors: 1 },
      { id: 'decay', contributors: 9 },
    ];
    const vorher = eingabe.map((m) => m.id);
    visibleIndicators(eingabe);
    expect(eingabe.map((m) => m.id)).toEqual(vorher);
  });
});
