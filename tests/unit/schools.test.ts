/**
 * Die Schulfarben (GAME-DESIGN 9.3, Issue #270, AC1).
 *
 * WAS DIESE PRUEFFAELLE KOENNEN UND WAS NICHT
 * -------------------------------------------
 * Sie pruefen die WERTE und ihren Kontrast. Ob die Farbe im Bild richtig
 * aussieht, kann nur ein Screenshot zeigen — und ob sie die Design-Worte
 * trifft, entscheidet der Auftraggeber.
 *
 * Die Kontrastrechnung steht hier ausgeschrieben und nicht als Zahl daneben:
 * Eine abgeschriebene Kontrastzahl waere beim naechsten Farbwechsel still
 * falsch. So rechnet der Prueffall sie jedes Mal neu.
 */
import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';
import { colourOfSchool, NO_SCHOOL_LABEL, SCHOOL_COLOURS } from '../../src/progression/schools.js';

/** Der Grund des Player Home — der HELLERE der beiden und damit der strengere. */
const GRUND = '#1a1613';

function linear(kanal: number): number {
  const c = kanal / 255;
  return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

function luminanz(farbe: string): number {
  const h = farbe.replace('#', '');
  const r = Number.parseInt(h.slice(0, 2), 16);
  const g = Number.parseInt(h.slice(2, 4), 16);
  const b = Number.parseInt(h.slice(4, 6), 16);
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

/** Kontrastverhaeltnis nach WCAG 2.1. */
export function kontrast(a: string, b: string): number {
  const la = luminanz(a);
  const lb = luminanz(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

describe('AC1 — jede Schule hat ihre Farbe', () => {
  it('deckt ALLE Schulen ab, die es in den Daten gibt', () => {
    // **Nicht die Tabelle gegen sich selbst pruefen.** Die erwartete Menge
    // kommt aus den Zauberdaten, nicht aus SCHOOL_COLOURS — sonst waere der
    // Fall gruen, egal welche Schule fehlt.
    const ausDenDaten = [
      ...new Set(CALCINATION.value.spells.map((s) => s.school).filter((s) => s !== null)),
    ].sort();
    expect(Object.keys(SCHOOL_COLOURS).sort()).toEqual(ausDenDaten);
  });

  it('ist auf dem Grund des Player Home lesbar — 4,5:1 fuer Text', () => {
    for (const [schule, farbe] of Object.entries(SCHOOL_COLOURS)) {
      expect(kontrast(farbe, GRUND), `${schule} (${farbe})`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('haelt die fuenf im selben GEWICHT, statt eine hervorzuheben', () => {
    // Der Sinn des Korridors: Laege eine bei 10:1 und CALCINATION bei 5:1,
    // waere eine Schule optisch dominant, ohne dass es jemand entschieden
    // haette.
    const werte = Object.values(SCHOOL_COLOURS).map((f) => kontrast(f, GRUND));
    const spanne = Math.max(...werte) - Math.min(...werte);
    expect(spanne, `Spanne ${spanne.toFixed(2)}`).toBeLessThan(2);
  });

  it('LAESST DEN BESTEHENDEN CALCINATION-WERT UNANGETASTET', () => {
    // Er steht seit E1 als SCHOOL_EDGE_READY in spellbar.ts. Ihn hier zu
    // aendern hiesse, die Zauberleiste an einer Stelle umzufaerben, die #270
    // nicht betrifft.
    expect(SCHOOL_COLOURS['CALCINATION']).toBe('#c9702f');
  });

  it('gibt einem Zauber ohne Schule KEINE Farbe', () => {
    expect(colourOfSchool(null)).toBeNull();
    expect(colourOfSchool('GIBTSNICHT')).toBeNull();
    expect(colourOfSchool('CALCINATION')).toBe('#c9702f');
  });

  it('nennt die Gruppe ohne Schule beim Design-Begriff', () => {
    // GAME-DESIGN 7.4: "Die drei schulunabhaengigen Sonderzauber".
    expect(NO_SCHOOL_LABEL).toBe('School-independent');
  });

  it('MISST UEBERHAUPT etwas — die Gegenprobe zur Rechnung', () => {
    // Ohne sie waere nicht gezeigt, dass `kontrast` Unterschiede sieht: Eine
    // Funktion, die immer 21 zurueckgibt, bestuende jeden Fall oben.
    expect(kontrast('#000000', '#ffffff')).toBeCloseTo(21, 1);
    expect(kontrast(GRUND, GRUND)).toBeCloseTo(1, 5);
    // Und das verworfene schwarz-violett faellt durch, wie gemessen.
    expect(kontrast('#5b3a8e', GRUND)).toBeLessThan(4.5);
  });
});
