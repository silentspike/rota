import { describe, expect, it } from 'vitest';

/**
 * **Kein Boss trägt ein Feld, das nach Mechanik aussieht und keine ist**
 * (Issue #570).
 *
 * # Der Befund, aus dem dieser Fall entstand
 *
 * `signature` stand in zwanzig Boss-Dateien, vier davon mit ausformuliertem
 * Text. Geladen wurde es auf **keiner** Seite:
 *
 * ```text
 * src/core/data.ts   0 Treffer
 * sim/src/data.rs    0
 * sim/build.rs       0
 * ```
 *
 * Vier Bosse trugen ihre Signatur-Regel als Satz in einem Feld, das nie
 * jemand liest.
 *
 * # Warum das mehr ist als ein ungenutztes Feld
 *
 * Ein Feld, das `_comment` heisst, sagt selbst, dass es Dokumentation ist.
 * Ein Feld, das `signature` heisst — in einem Projekt, dessen GAME-DESIGN die
 * Signatur-Regel als **tragende Boss-Mechanik** führt (11.1) — sagt das
 * Gegenteil und liegt trotzdem tot in den Daten.
 *
 * Der Nächste, der einen Boss baut, nimmt an, dass es wirkt.
 *
 * # Warum dieser Fall und nicht bloss die Umbenennung
 *
 * Die Umbenennung räumt den heutigen Bestand auf. Sie verhindert nicht, dass
 * jemand das Feld morgen wieder schreibt — der Name liegt nahe, das
 * GAME-DESIGN nennt ihn, und nichts würde widersprechen.
 *
 * Bricht dieser Fall, ist die Frage nicht „wie nenne ich es um", sondern:
 * **Soll `signature` geladen werden?** Dann ist Weg (a) aus #570 dran, und
 * dieser Wächter gehört durch eine Ladeprüfung ersetzt.
 */

/** Alle Boss-Datensätze, roh — nicht über den Leser, der das Feld ja ignoriert. */
const BOSSES = import.meta.glob('../../data/bosses/*.json', {
  eager: true,
  import: 'default',
}) as Record<string, Record<string, unknown>>;

describe('AC4 (#570) — kein Boss trägt ein ungeladenes Mechanik-Feld', () => {
  it('die Prüfung liest überhaupt Bosse — sonst belegt sie nichts', () => {
    // **Die Zusicherung in den Datensatz.** Ein leerer Glob wäre grün und
    // hätte nichts geprüft; genau so ist ein Wächter schon einmal blind
    // geworden, ohne dass es auffiel.
    expect(Object.keys(BOSSES).length, 'kein Boss-Datensatz gefunden').toBeGreaterThan(10);
  });

  it('keiner trägt das Feld `signature`', () => {
    const mit = Object.entries(BOSSES)
      .filter(([, b]) => 'signature' in b)
      .map(([pfad]) => pfad.split('/').pop());

    expect(
      mit,
      'Diese Bosse tragen `signature` — ein Feld, das auf KEINER Seite geladen ' +
        'wird (data.ts, data.rs, build.rs: je 0 Treffer). Es sieht aus wie ' +
        'Mechanik und ist Dokumentation.\n\n' +
        'Die Signatur-Regel gehört in `_comment`, solange sie nicht geladen ' +
        'wird. Soll sie geladen werden, ist das Weg (a) aus #570 — dann ' +
        'gehört dieser Fall durch eine Ladeprüfung ersetzt, nicht das Feld ' +
        'wieder entfernt.',
    ).toEqual([]);
  });
});
