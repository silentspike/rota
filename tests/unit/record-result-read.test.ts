/**
 * Jedes Feld von `RecordResult` wird gelesen — oder ist benannt ausgenommen
 * (Issue #371).
 *
 * # Warum es diesen Wächter gibt
 *
 * `xpGained` und `leveledUp` waren berechnet, geprüft und **tot**: Sie kamen
 * außerhalb des Erzeugers nirgends vor. Ein Wächter über die *Rechnung* sagt
 * nichts darüber, ob ihr Ergebnis irgendwo ankommt.
 *
 * Das ist die Lücke, die dieser Fall schließt — und die dritte wäre sonst nur
 * eine Frage der Zeit gewesen.
 *
 * # Wie die Feldliste entsteht
 *
 * Aus einem **echten Ergebnis**, nicht aus einer Liste daneben. Ein Interface
 * hat zur Laufzeit keine Felder; ein Objekt hat sie. `Object.keys()` wächst
 * damit mit der Struktur mit — wer ein Feld ergänzt, ergänzt die Liste, ohne
 * es zu wollen und ohne es zu können.
 *
 * Dieselbe Bauart wie `GEPRUEFTE_FAKTOREN` in `sim/tests/factors.rs` (#346),
 * nur mit `Object.keys()` statt der Debug-Ableitung.
 */
import { describe, expect, it } from 'vitest';
import {
  countsAsAttempt,
  recordRun,
  type RecordedState,
  type RecordResult,
} from '../../src/analysis/run-recorder.js';

/**
 * Felder, die **absichtlich** nicht an den Spieler gehen — namentlich und mit
 * Grund.
 *
 * **Eine Ausnahme, die man nennen muss, ist eine Entscheidung. Eine, die durch
 * Weglassen entsteht, ist ein Versehen** — und genau das war der Zustand:
 * `xpGained` und `leveledUp` fehlten nicht sichtbar, sie fehlten stumm.
 */
const NICHT_FUER_DEN_SPIELER: ReadonlyMap<string, string> = new Map([
  [
    'counted',
    'Steuert, ob der Durchlauf abgelegt wird — eine Entscheidung des Programms, ' +
      'keine Nachricht an den Spieler.',
  ],
]);

/**
 * Die Felder, die ein echtes Ergebnis führt.
 *
 * **Über den Abbruch-Pfad**, nicht über einen vollständigen Lauf: Dort setzt
 * `recordRun` jedes Feld ausdrücklich, während der Sieg-Pfad einen echten
 * Kampf bräuchte. Für die Frage „welche Felder gibt es" ist der kürzere Weg
 * der genauere — er zeigt die Struktur, nicht einen Lauf.
 */
function felderVonRecordResult(): string[] {
  const leer: RecordedState = { runHistory: [], progressMarks: [], level: 1, experience: 0 };
  const abbruch = -1;
  expect(countsAsAttempt(abbruch), 'dieser Ausgang zaehlt doch — dann ist es der falsche Pfad').toBe(
    false,
  );
  const ergebnis: RecordResult = recordRun(leer, {} as never, abbruch, {
    bossId: 'first-crucible',
    difficulty: 1,
    bossLevel: 1,
    gameVersion: '0.1.0',
    at: 1_800_000_000_000,
  });
  return Object.keys(ergebnis);
}

describe('Kein Feld von RecordResult laeuft stumm tot (AC3)', () => {
  it('liest ueberhaupt Felder — sonst prueft der Rest nichts', () => {
    // Der Gegenfall: Eine leere Liste wäre unten in null Durchläufen grün.
    const felder = felderVonRecordResult();
    expect(felder.length).toBeGreaterThan(4);
    // Und an bekannten Namen, nicht nur an der Zahl: Ein Ergebnis mit acht
    // beliebigen Schlüsseln käme sonst durch.
    for (const bekannt of ['xpGained', 'leveledUp', 'counted']) {
      expect(felder, `${bekannt} fehlt im Ergebnis — hat sich die Struktur geändert?`).toContain(
        bekannt,
      );
    }
  });

  it('nennt jedes Feld, das weder gelesen noch ausgenommen ist', () => {
    const felder = felderVonRecordResult();
    const quellen = Object.entries(
      import.meta.glob('/src/**/*.ts', { query: '?raw', import: 'default', eager: true }) as Record<
        string,
        string
      >,
    ).filter(([pfad]) => !pfad.endsWith('run-recorder.ts'));

    const ungelesen = felder.filter((feld) => {
      if (NICHT_FUER_DEN_SPIELER.has(feld)) return false;
      return !quellen.some(([, inhalt]) => inhalt.includes(`.${feld}`));
    });

    expect(
      ungelesen,
      'Diese Felder von RecordResult werden ausserhalb ihres Erzeugers nirgends ' +
        'gelesen und stehen auch nicht in NICHT_FUER_DEN_SPIELER:\n' +
        'Entweder gehoert das Feld an den Spieler — oder die Ausnahme gehoert ' +
        'benannt. Beides ist eine Entscheidung; es wegzulassen ist keine.',
    ).toEqual([]);
  });

  it('meldet eine Ausnahme fuer ein Feld, das es nicht mehr gibt', () => {
    // Die Gegenrichtung: Eine Ausnahme fuer ein verschwundenes Feld ist eine
    // Zeile, die niemand mehr prueft.
    const felder = new Set(felderVonRecordResult());
    for (const name of NICHT_FUER_DEN_SPIELER.keys()) {
      expect(felder, `NICHT_FUER_DEN_SPIELER nennt ${name}, das Ergebnis hat es nicht`).toContain(
        name,
      );
    }
  });
});
