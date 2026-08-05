import { describe, expect, it } from 'vitest';

/**
 * Jede Abschnittsnummer in GAME-DESIGN kommt genau einmal vor.
 *
 * # Warum das einen Wächter braucht
 *
 * Abschnittsnummern sind Verweisziele: `vite.config.ts`, `save-format.ts` und
 * sechs weitere Stellen nennen „18.6". Vergibt jemand dieselbe Nummer ein
 * zweites Mal, zeigen alle bestehenden Verweise plötzlich auf zwei Dinge — und
 * nichts im Projekt bemerkt es.
 *
 * Genau das ist bei #369 passiert: Der neue Abschnitt bekam 18.6, und
 * „Aktualisierungen und Spielstände" hatte die Nummer bereits. Aufgefallen ist
 * es beim Abnehmen, nicht beim Schreiben.
 *
 * # Der Suffix gehört zur Nummer
 *
 * `4.2b` ist **nicht** `4.2`. Ein Muster, das den Buchstaben abschneidet,
 * meldet zwei Doppelungen, die keine sind — mein erster Entwurf tat das und
 * hätte 4.2/4.2b und 13.1/13.1b angezeigt.
 */

const DESIGN = import.meta.glob('/docs/GAME-DESIGN.md', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function nummern(): string[] {
  const text = Object.values(DESIGN)[0];
  if (text === undefined || text.length === 0) {
    throw new Error('GAME-DESIGN.md nicht geladen — die Prüfung würde nichts messen');
  }
  // **Mit Suffix**: `13.1b` ist eine eigene Nummer, nicht eine zweite `13.1`.
  return [...text.matchAll(/^#{2,4} (\d+(?:\.\d+)*[a-z]?)/gm)].map((m) => m[1] as string);
}

describe('Abschnittsnummern in GAME-DESIGN', () => {
  it('findet die Überschriften überhaupt', () => {
    // Ohne diese Zusicherung liefe der Wächter auch dann durch, wenn das
    // Muster nichts fände — und eine Prüfung über nichts ist wahr.
    expect(nummern().length).toBeGreaterThan(50);
  });

  it('vergibt keine Nummer zweimal', () => {
    const gesehen = new Map<string, number>();
    for (const n of nummern()) gesehen.set(n, (gesehen.get(n) ?? 0) + 1);
    const doppelt = [...gesehen.entries()]
      .filter(([, anzahl]) => anzahl > 1)
      .map(([n, anzahl]) => `${n} (${anzahl}x)`);

    expect(
      doppelt,
      'Abschnittsnummern sind Verweisziele — acht Stellen im Code nennen sie. ' +
        'Eine doppelt vergebene Nummer lässt jeden bestehenden Verweis auf zwei ' +
        'Dinge zeigen.',
    ).toEqual([]);
  });
});
