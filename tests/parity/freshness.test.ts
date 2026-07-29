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
import eingecheckteEreignisse from './recordings/rule-events.json';
import { primitivesPayload, recordingPayload, ruleEventsPayload } from './payloads.js';

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

  it('die Ereignisfolge entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteEreignisse, HINWEIS).toEqual(ruleEventsPayload());
  });
});

describe('Die Ereignisfolge taugt als Prüffall (E3-1b)', () => {
  const payload = ruleEventsPayload();
  const steps = payload['steps'] as { event: number; value: number }[];
  const after = payload['checksumAfterEach'] as number[];

  it('berührt jedes der sechs Ereignisse', () => {
    // Eine Folge, die ein Ereignis auslässt, ließe eine Abweichung genau dort
    // durch.
    expect(new Set(steps.map((s) => s.event)).size).toBe(6);
  });

  it('führt für jeden Schritt eine Prüfsumme mit', () => {
    // Nur die Endsumme zu vergleichen sagt DASS, nicht WO es auseinanderläuft.
    expect(after.length).toBe(steps.length);
    expect(new Set(after).size).toBe(after.length);
  });

  it('enthält einen Unterschied im letzten Bit', () => {
    // Wer über den gerundeten Wert prüfsummiert statt über die Bits, besteht
    // alles andere und fällt hier durch.
    const werte = steps.map((s) => s.value);
    expect(werte).toContain(1);
    expect(werte.some((v) => v !== 1 && Math.abs(v - 1) < 1e-15)).toBe(true);
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
