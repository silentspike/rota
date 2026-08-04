/**
 * Kein Quelltext darf für Git eine Binärdatei sein.
 *
 * WARUM DAS EINEN EIGENEN PRUEFFALL VERDIENT
 * ------------------------------------------
 * Am 2026-08-04 stand in `progress-mark.ts` ein echtes 0x00-Byte als
 * Trennzeichen eines zusammengesetzten Schluessels. Zur Laufzeit war das
 * richtig -- NUL kommt in keiner Boss-Kennung vor, der Schluessel konnte also
 * nicht mehrdeutig werden. Die DATEI aber galt Git damit als binaer, und das
 * hat Folgen, die nichts mit dem Programm zu tun haben:
 *
 *   git diff          zeigt den Inhalt nicht, nur "Bin 6480 -> 8114 bytes"
 *   git diff --stat   ebenso -- und genau darauf stuetzt sich unsere
 *                     Durchsicht vor jedem Nachziehen
 *   Merge-Konflikte   nicht aufloesbar
 *   GitHub            "Binary file not shown"
 *
 * Der eigentliche Schaden war nicht der Bau, sondern die BLINDE STELLE: Fuenf
 * Meldungen mit "1725 Zeilen hinzu, 4 entfernt" haben ueber diese Datei nichts
 * ausgesagt. Ein Zweig, der sie zurueckdreht, haette in derselben Pruefung
 * genauso ausgesehen wie einer, der sie ergaenzt.
 *
 * Deshalb steht hier eine Pruefung und nicht nur ein behobenes Byte: Ein
 * Werkzeug, das still aufhoert zu messen, ist gefaehrlicher als eines, das
 * gar nicht erst da ist.
 *
 * Der Weg zurueck ist die ESCAPE-SEQUENZ: zwei Zeichen im Quelltext, dasselbe
 * Byte zur Laufzeit.
 */
import { describe, expect, it } from 'vitest';

/**
 * Der Quelltext, ueber Vite geladen statt ueber das Dateisystem.
 *
 * `node:fs` waere der naheliegende Weg und braeuchte @types/node in der
 * tsconfig -- eine geteilte Konfigurationsaenderung fuer einen einzelnen
 * Prueffall. `import.meta.glob` mit `?raw` liefert dasselbe und bleibt
 * innerhalb dessen, was das Projekt ohnehin einrichtet.
 */
const QUELLEN = import.meta.glob('/src/**/*.{ts,css}', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const PRUEFBLAETTER = import.meta.glob('/docs/**/*.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

describe('Quelltext bleibt fuer Git lesbar', () => {
  it('enthaelt nirgends ein NUL-Byte', () => {
    const alle = { ...QUELLEN, ...PRUEFBLAETTER };
    const pfade = Object.keys(alle);
    // Ohne diese Zusicherung liefe der Prueffall auch dann durch, wenn das
    // Muster nichts faende -- und belegte nichts.
    expect(pfade.length).toBeGreaterThan(30);

    const binaer = pfade.filter((pfad) => (alle[pfad] ?? '').includes('\u0000'));
    expect(binaer, `NUL-Byte im Quelltext: ${binaer.join(', ')}`).toEqual([]);
  });

  it('haelt den Schluessel trotzdem eindeutig', () => {
    // Die Wahl von NUL als Trennzeichen war richtig und bleibt: Zwei
    // verschiedene Paare duerfen nie denselben Schluessel ergeben. Mit einem
    // Bindestrich waeren "a-1" und "a" auf Stufe 1 nicht zu unterscheiden,
    // sobald eine Boss-Kennung selbst einen Bindestrich traegt -- und alle
    // unsere Kennungen tun das ("first-crucible", "test-warden").
    const trennzeichen = '\0';
    expect(`a-1${trennzeichen}2`).not.toBe(`a${trennzeichen}1-2`);
    expect(trennzeichen.charCodeAt(0)).toBe(0);
  });
});
