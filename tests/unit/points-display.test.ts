/**
 * Warum die Punktzahl so ausfiel (Issue #387).
 *
 * WAS AC5 WIRKLICH VERLANGT
 * -------------------------
 * „Ein Prüffall belegt, dass die Anzeige die INFORMATION hat, um den Fall zu
 * unterscheiden — nicht, dass ein bestimmter Text dasteht."
 *
 * Das ist fast trivial zu erfüllen, sobald `pointsDisplayFor` eine Struktur
 * liefert: `expect(display.previous).not.toBeNull()` wäre grün und belegte
 * nichts. Deshalb prüft diese Datei die **Unterscheidung**: Drei Läufe, die
 * sich NUR im Bestwert unterscheiden, müssen drei verschiedene `kind` liefern
 * — und zwei mit gleichem Bestwert dasselbe.
 *
 * Das fällt, wenn jemand die Fallunterscheidung entfernt, und bleibt grün,
 * wenn der Text sich ändert. Genau die Trennung, die AC5 meint.
 *
 * DEN TEXT PRÜFT SIE TROTZDEM — an einer Stelle
 * ---------------------------------------------
 * `scoreText` ist eine reine Funktion und ohne DOM prüfbar. Was hier NICHT
 * steht, ist ein erwarteter Wortlaut: Geprüft wird, dass die beiden Fälle
 * **verschiedene** Texte ergeben und dass die Einheit zum Mass passt. Wer den
 * Wortlaut ändert, soll keinen Prüffall anfassen müssen.
 */
import { describe, expect, it } from 'vitest';
import { pointsDisplayFor } from '../../src/analysis/points-display.js';
import { scoreText } from '../../src/render/overlay.js';
import type { RecordResult } from '../../src/analysis/run-recorder.js';
import type { RunSummary } from '../../src/core/summary.js';

/** Eine Aufzeichnung, reduziert auf das, was die Erklärung liest. */
function aufzeichnung(
  previous: number | null,
  improved: boolean,
  over: Partial<RecordResult> = {},
): RecordResult {
  return {
    counted: true,
    pointsGained: 428,
    best: previous === null && !improved ? null : { runs: [], previous, improved, discarded: null },
    ...over,
  } as unknown as RecordResult;
}

/** Ein Lauf, reduziert auf das, was die Erklärung liest. */
function lauf(victory = true, heldPercent: number | null = null): RunSummary {
  return { victory, heldPercent } as unknown as RunSummary;
}

describe('AC5 — die Anzeige kann die Fälle unterscheiden', () => {
  it('drei Läufe, die sich NUR im Bestwert unterscheiden, ergeben drei Fälle', () => {
    // Alles andere ist identisch: derselbe Lauf, dieselbe Punktzahl, dasselbe
    // Mass. Was sie trennt, ist ausschliesslich der Bestwert davor.
    const ohne = pointsDisplayFor(aufzeichnung(null, false), lauf());
    const besser = pointsDisplayFor(aufzeichnung(55_670, true), lauf());
    const gleich = pointsDisplayFor(aufzeichnung(55_670, false), lauf());

    expect([ohne?.kind, besser?.kind, gleich?.kind]).toEqual(['full', 'difference', 'repeat']);
    // Drei verschiedene — nicht zufällig zwei gleiche.
    expect(new Set([ohne?.kind, besser?.kind, gleich?.kind]).size).toBe(3);
  });

  it('GEGENPROBE: gleicher Bestwert und gleicher Ausgang ergeben denselben Fall', () => {
    // Ohne diese Zeile bliebe offen, ob `kind` überhaupt vom Bestwert abhängt
    // — eine Funktion, die reihum drei Werte liefert, bestünde die Prüfung
    // darüber ebenfalls.
    const eins = pointsDisplayFor(aufzeichnung(55_670, false), lauf());
    const zwei = pointsDisplayFor(aufzeichnung(55_670, false), lauf());
    expect(eins?.kind).toBe(zwei?.kind);
  });

  it('trägt den Vergleichswert mit, wo es einen gibt', () => {
    expect(pointsDisplayFor(aufzeichnung(55_670, true), lauf())?.previous).toBe(55_670);
    expect(pointsDisplayFor(aufzeichnung(55_670, false), lauf())?.previous).toBe(55_670);
  });

  it('AC3 — bei vollem Ertrag gibt es nichts zu vergleichen', () => {
    const ohne = pointsDisplayFor(aufzeichnung(null, false), lauf());
    expect(ohne?.kind).toBe('full');
    // **`null` und nicht 0**: Eine Null wäre eine Marke, die es nicht gibt.
    expect(ohne?.previous).toBeNull();
  });

  it('zeigt nichts, wo es nichts zu erklären gibt', () => {
    // Ein Abbruch hat nicht stattgefunden; eine Niederlage zahlt nichts, und
    // „0 · repeat" erklärte etwas, das nicht passiert ist.
    expect(pointsDisplayFor(aufzeichnung(null, false, { counted: false }), lauf())).toBeNull();
    expect(pointsDisplayFor(aufzeichnung(55_670, false), lauf(false))).toBeNull();
    expect(pointsDisplayFor(null, lauf())).toBeNull();
    expect(pointsDisplayFor(aufzeichnung(null, false), null)).toBeNull();
  });
});

describe('AC4 — das Mass wird geerbt, nicht geraten', () => {
  it('folgt dem Lauf: Ring-Rest beim runaway-Boss, sonst Reserve', () => {
    // Dieselbe Verzweigung wie in #256, #276 und #377 — zum vierten Mal
    // geerbt und zum ersten Mal in der Anzeige gebraucht.
    expect(pointsDisplayFor(aufzeichnung(55_670, true), lauf(true, null))?.isRing).toBe(false);
    expect(pointsDisplayFor(aufzeichnung(85, true), lauf(true, 92))?.isRing).toBe(true);
  });

  it('und die Zeile schreibt die passende Einheit', () => {
    const zeit = scoreText(pointsDisplayFor(aufzeichnung(55_670, true), lauf(true, null)));
    const ring = scoreText(pointsDisplayFor(aufzeichnung(85, true), lauf(true, 92)));

    expect(zeit).toContain('55.67 s');
    expect(ring).toContain('85.0 %');
    // GEGENPROBE: die Einheiten sind nicht vertauschbar. Ohne `isRing` stünde
    // dort „best 85.00 s" oder „best 55670.0 %" — beide Male eine plausible
    // Zahl der falschen Sorte.
    expect(zeit).not.toContain('%');
    expect(ring).not.toContain(' s');
  });
});

describe('Die Zeile sagt, welche Frage sie beantwortet', () => {
  it('unterscheidet Wiederholung und Verbesserung im Text', () => {
    // Kein erwarteter Wortlaut — geprüft wird, dass die beiden Fälle
    // VERSCHIEDEN dastehen. Wer den Text ändert, soll hier nichts anfassen
    // müssen.
    const wiederholung = scoreText(pointsDisplayFor(aufzeichnung(55_670, false), lauf()));
    const verbesserung = scoreText(pointsDisplayFor(aufzeichnung(55_670, true), lauf()));
    expect(wiederholung).not.toBe(verbesserung);
    // Beide nennen die Marke — sie ist die Antwort auf beide Fragen.
    expect(wiederholung).toContain('55.67 s');
    expect(verbesserung).toContain('55.67 s');
  });

  it('bei vollem Ertrag steht die Zahl allein', () => {
    const voll = scoreText(pointsDisplayFor(aufzeichnung(null, false), lauf()));
    expect(voll).toBe('428');
  });

  it('ohne Erklärung bleibt der Strich', () => {
    expect(scoreText(null)).toBe('—');
  });

  it('die Zahl steht in jedem Fall vorn', () => {
    // Sie ist die Antwort; der Grund ist der Zusatz. Andersherum müsste der
    // Spieler lesen, um die Punktzahl zu finden.
    for (const anzeige of [
      pointsDisplayFor(aufzeichnung(null, false), lauf()),
      pointsDisplayFor(aufzeichnung(55_670, true), lauf()),
      pointsDisplayFor(aufzeichnung(55_670, false), lauf()),
    ]) {
      expect(scoreText(anzeige).startsWith('428')).toBe(true);
    }
  });
});
