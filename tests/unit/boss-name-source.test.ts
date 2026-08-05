/**
 * Der Boss-Name steht nirgends fest im Quelltext (Issue #229, AC2).
 *
 * WARUM DAS EIN PRÜFFALL SEIN MUSS UND KEIN VORSATZ
 * ------------------------------------------------
 * Bis #229 stand `'THE FIRST CRUCIBLE'` an **zwei** Stellen fest — in
 * `hud.ts` als Konstante und in `overlay.ts` im Aufruf. Solange nur ein Boss
 * spielbar war, stimmte die Abschrift zufällig.
 *
 * Sichtbar wurde sie erst durch den Startparameter aus #219: Über dem
 * Testboss stand der Name des ersten Tiegels.
 *
 * **Das ist die gefährlichere Sorte Handabschrift.** Sie erzeugt keinen
 * Pflegeaufwand, sondern führt eine MESSUNG in die Irre: Wer den Boss wechselt
 * und den vertrauten Namen sieht, hält den Wechsel für gescheitert — oder,
 * schlimmer, hält einen echten Fehlschlag für gelungen, weil alles aussieht
 * wie immer.
 *
 * WAS DIESE PRÜFUNG NICHT KANN
 * ----------------------------
 * Sie liest Text, nicht Bedeutung. Wer einen Boss-Namen über eine
 * Hilfsvariable oder in Einzelteilen zusammensetzt, kommt daran vorbei. Das
 * ist die Grenze, und sie steht hier, damit niemand die Prüfung für mehr hält,
 * als sie ist.
 *
 * Was sie **kann**: jeden Namen aus `data/bosses/` als Zeichenkette im
 * Quelltext finden — in jeder Schreibweise, weil verglichen wird, nachdem
 * beide Seiten auf Großbuchstaben gebracht wurden.
 */
import { describe, expect, it } from 'vitest';
import { BOSSES } from '../../src/core/data.js';

/**
 * Die Quelltexte der Anzeige, über Vite geladen.
 *
 * `node:fs` bräuchte @types/node in der tsconfig — eine geteilte
 * Konfigurationsänderung für einen einzelnen Prüffall.
 */
const QUELLEN = import.meta.glob('/src/render/*.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

/** Alle Boss-Namen, wie sie in den Datensätzen stehen. */
function bossNamen(): string[] {
  return [...BOSSES.values()].map((eintrag) => eintrag.value.name);
}

describe('Kein Boss-Name steht fest im Quelltext (AC2)', () => {
  it('lädt die Quelltexte überhaupt — sonst prüfte der Rest nichts', () => {
    // Ohne diese Zeile wäre eine leere Liste die stillste Art, grün zu sein:
    // keine Datei, keine Fundstelle, kein Fehler.
    const dateien = Object.keys(QUELLEN);
    expect(dateien.length).toBeGreaterThan(0);
    expect(dateien.some((pfad) => pfad.endsWith('hud.ts'))).toBe(true);
    expect(dateien.some((pfad) => pfad.endsWith('overlay.ts'))).toBe(true);
  });

  it('kennt überhaupt Namen, gegen die es prüfen kann', () => {
    // Derselbe Gegenfall eine Ebene tiefer: Eine leere Namensliste ließe jede
    // Suche unten ins Leere laufen.
    const namen = bossNamen();
    expect(namen.length).toBeGreaterThan(1);
    expect(namen).toContain('The First Crucible');
  });

  it('findet keinen einzigen davon in der Anzeige', () => {
    const namen = bossNamen();
    const treffer: string[] = [];

    for (const [pfad, quelle] of Object.entries(QUELLEN)) {
      const gross = quelle.toUpperCase();
      for (const name of namen) {
        if (gross.includes(name.toUpperCase())) {
          treffer.push(`${pfad}: ${name}`);
        }
      }
    }

    expect(
      treffer,
      'Ein Boss-Name steht fest im Quelltext. Er gehört aus dem Datensatz ' +
        'gelesen — sonst zeigt die Anzeige beim nächsten Boss den falschen ' +
        'Namen, und wer misst, hält das Ergebnis für das richtige.',
    ).toEqual([]);
  });
});
