/**
 * Die Grundwerte wachsen mit der Gegenstandsstufe (Issue #282).
 *
 * # Warum hier kein einziger Betrag geprüft wird
 *
 * `BASE_STAT_AT_LEVEL_1` ist **unkalibriert** — gesetzt, damit die Leitung
 * etwas transportiert, nicht gemessen. Ein Prüffall auf den Betrag würde die
 * Zahl einfrieren: Er meldete rot, sobald jemand sie kalibriert, und der
 * Wächter arbeitete gegen die Balance-Arbeit statt für sie.
 *
 * Geprüft wird deshalb das **Verhältnis**. Es folgt aus der Kurve und bleibt
 * richtig, welchen Betrag die Basis auch bekommt.
 */
import { describe, expect, it } from 'vitest';
import {
  ESSENCE_AT_LEVEL_1,
  VESSEL_AT_LEVEL_1,
  WARD_AT_LEVEL_1,
  basePermille,
} from '../../src/gear/sources.js';
import { LEVELS } from '../../src/core/data.js';
import { MAX_ITEM_LEVEL } from '../../src/gear/budget.js';

describe('Grundwerte je Gegenstandsstufe', () => {
  it('trägt auf Stufe 1 die Basis', () => {
    // Der einzige Ort, an dem Basis und Ergebnis zusammenfallen — weil der
    // Faktor auf Stufe 1 exakt 1000 Promille ist.
    expect(LEVELS.factorPerMille[0]).toBe(1000);
    // Alle drei Werte, denn sie haben je eine eigene Basis — eine gemeinsame
    // Konstante wäre die stillschweigende Behauptung, sie seien gleich stark.
    for (const basis of [ESSENCE_AT_LEVEL_1, VESSEL_AT_LEVEL_1, WARD_AT_LEVEL_1]) {
      expect(basePermille(1, basis)).toBe(basis);
    }
  });

  it('wächst mit der Stufe, im Verhältnis der Kurve', () => {
    // **Das Verhältnis, nicht der Betrag.** Wer die Basis verdoppelt, ändert
    // beide Seiten gleich — dieser Fall bleibt grün, und das ist Absicht.
    for (const stufe of [2, 10, 50, MAX_ITEM_LEVEL]) {
      const erwartet = Math.trunc(
        (ESSENCE_AT_LEVEL_1 * (LEVELS.factorPerMille[stufe - 1] ?? 0)) / 1000,
      );
      expect(basePermille(stufe, ESSENCE_AT_LEVEL_1)).toBe(erwartet);
    }
  });

  it('wächst überhaupt — sonst prüfte der Rest nichts', () => {
    // Der Gegenfall: Wäre die Kurve flach, wären alle Vergleiche oben trivial
    // erfüllt und der Prüffall stumm.
    expect(basePermille(MAX_ITEM_LEVEL, ESSENCE_AT_LEVEL_1)).toBeGreaterThan(
      basePermille(1, ESSENCE_AT_LEVEL_1) * 10,
    );
  });

  it('kennt keine Qualität', () => {
    // GAME-DESIGN 1359: Qualität gibt Affixe, nicht mehr Grundwerte. Die
    // Funktion nimmt keine entgegen — dieser Fall hält das fest, damit ein
    // späterer Zusatz auffällt statt sich einzuschleichen. Zwei Parameter:
    // Gegenstandsstufe und Basis, keine Qualität.
    expect(basePermille.length).toBe(2);
  });

  it('liefert 0 außerhalb des gültigen Bereichs', () => {
    for (const ungueltig of [0, -1, MAX_ITEM_LEVEL + 1, 1.5, Number.NaN]) {
      expect(basePermille(ungueltig, ESSENCE_AT_LEVEL_1)).toBe(0);
    }
  });
});
