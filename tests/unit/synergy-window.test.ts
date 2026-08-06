import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { hastedMs } from '../../src/core/haste.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';

const GCD_MS = 1500;

/**
 * Das Synergie-Fenster steht in WARTEZEITEN, nicht in Sekunden (Issue #428).
 *
 * # Warum bei 100 und 300 Promille gemessen wird
 *
 * Bei **0, 376 und 500** ‰ ergeben beide Klammerungen dasselbe — dort trifft
 * sich die Rundung zufällig. Ein Prüffall an einem dieser Werte wäre grün,
 * ohne etwas zu belegen: Er liefe auch mit der alten Rechnung durch.
 *
 * Deshalb stehen hier nur Werte, für die die Differenz **einzeln nachgemessen**
 * ist. Wer einen weiteren aufnimmt, muss dasselbe tun — die runden Werte sind
 * nicht offensichtlich, und 376 ist erst bei einer Nachmessung aufgefallen.
 */
describe('Synergie-Fenster in Wartezeiten (Issue #428)', () => {
  function fensterLaenge(catalysis: number): number {
    const combat = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      worn: { ...NOTHING_WORN, catalysis },
    });
    // Zum Zeitpunkt 0 gewirkt: Das Fenster endet dann bei seiner eigenen
    // Länge, und `synergyWindowSum` gibt sie unmittelbar her.
    combat.cast(combat.indexOf('canker'), 0);
    return combat.synergyWindowSum;
  }

  // `canker` ist der einzige Zauber mit Synergie-Fenster, und es sind ZWEI
  // Wartezeiten (data/spells/putrefaction.json). Steht hier als Zahl, damit
  // der Prüffall bricht, wenn jemand den Wert ändert, statt sich stillschweigend
  // anzupassen.
  const WINDOW_GCDS = 2;

  for (const catalysis of [100, 300]) {
    it(`rechnet bei ${catalysis} Promille zwei echte Wartezeiten`, () => {
      const erwartet = WINDOW_GCDS * hastedMs(GCD_MS, catalysis);
      const alteRechnung = hastedMs(GCD_MS * WINDOW_GCDS, catalysis);

      // **Die Gegenprobe steht im Prüffall selbst**: Wären beide Rechnungen
      // hier gleich, prüfte er nichts — dann wäre dieses `catalysis` einer der
      // runden Werte, und der Fall müsste ausgetauscht werden.
      expect(
        erwartet,
        `${catalysis} Promille ist ein RUNDER Wert — beide Klammerungen ergeben ` +
          `${erwartet}. Dieser Prüffall kann hier nichts zeigen.`,
      ).not.toBe(alteRechnung);

      expect(fensterLaenge(catalysis)).toBe(erwartet);
    });
  }

  it('ist ohne Tempo unverändert — keine bestehende Aufzeichnung bewegt sich', () => {
    expect(fensterLaenge(0)).toBe(WINDOW_GCDS * GCD_MS);
  });
});
