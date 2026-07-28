/**
 * Sind die gemeinsamen Vergleichsdateien noch aktuell? (Issue #54)
 *
 * **Das ist der Test, der den ganzen Abgleich zusammenhält.** Der Rust-Simulator
 * prüft gegen die Dateien unter `recordings/`. Ändert jemand eine Regel im Spiel
 * und schreibt sie nicht neu, prüft der Simulator weiter gegen die alten Werte —
 * und bleibt grün, obwohl beide Seiten auseinanderlaufen. Der Prüffall sagte
 * dann genau in dem Moment nichts mehr aus, in dem er gebraucht wird.
 *
 * Schlägt dieser Test fehl, ist das **keine kaputte Prüfung**, sondern der
 * Hinweis, dass die Dateien neu geschrieben gehören:
 *
 * ```text
 * npx vite-node tests/parity/export-parity.mjs
 * ```
 *
 * Danach zeigt der Diff, was sich geändert hat — und ob das gewollt war.
 */
import { describe, expect, it } from 'vitest';

// Als Modul eingebunden statt über das Dateisystem gelesen: Das Projekt führt
// bewusst keine Node-Typen (`tsconfig.json`), und für den Vergleich ist der
// Inhalt maßgeblich, nicht die Formatierung der Datei.
import eingecheckteAufzeichnung from './recordings/cooldown-precision.json';
import eingecheckteBausteine from './recordings/primitives.json';
import { primitivesPayload, recordingPayload } from './payloads.js';

const HINWEIS =
  'Die Datei ist nicht mehr aktuell. Neu schreiben mit:\n' +
  '  npx vite-node tests/parity/export-parity.mjs\n' +
  'Der Diff zeigt dann, welche Regel sich geändert hat.';

describe('Gemeinsame Vergleichsdateien', () => {
  it('die aufgezeichnete Runde entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteAufzeichnung, HINWEIS).toEqual(recordingPayload());
  });

  it('die Bausteine entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteBausteine, HINWEIS).toEqual(primitivesPayload());
  });
});

describe('Die Aufzeichnung taugt als Prüffall', () => {
  const payload = recordingPayload();
  const expected = payload['expected'] as Record<string, number>;
  const presses = payload['presses'] as { tick: number; slot: number }[];
  const checksums = payload['checksums'] as number[];

  it('enthält überhaupt Tastendrücke', () => {
    // Eine leere Aufzeichnung würde jeden Simulator bestehen lassen.
    expect(presses.length).toBeGreaterThan(0);
    expect(expected['accepted']).toBe(presses.length);
    expect(expected['rejected']).toBe(0);
  });

  it('ist nach Takt geordnet', () => {
    // Beide Seiten arbeiten die Drücke in einem Durchgang ab; eine unsortierte
    // Liste würde im Simulator stillschweigend Drücke verschlucken.
    for (let i = 1; i < presses.length; i++) {
      const aktuell = presses[i]?.tick ?? 0;
      const davor = presses[i - 1]?.tick ?? 0;
      expect(aktuell).toBeGreaterThanOrEqual(davor);
    }
  });

  it('trifft die Werte der Referenz aus STEP1-SPEC § 4', () => {
    // Die Aufzeichnung ist die gute Rotation, einmal mitgeschrieben — sie muss
    // dieselben Zahlen ergeben wie `tools/step1-check.py`. Weicht sie ab, hat
    // das Aufzeichnen selbst etwas verändert.
    expect(expected['damage']).toBe(4240);
    expect(expected['killedAtMs']).toBe(40_510);
    expect(expected['randomDraws']).toBe(0);
  });

  it('hat für jeden gerechneten Takt eine Prüfsumme', () => {
    // 40510 ms Kampf bei 10 ms Takt: Takt 0 bis 4051, also 4052 Werte.
    const killedAtMs = expected['killedAtMs'] ?? -1;
    expect(checksums.length).toBe(killedAtMs / 10 + 1);
  });
});
