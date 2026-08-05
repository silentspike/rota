/**
 * Der Fortschritt wird an EINER Stelle gerechnet (Issue #256, AC1).
 *
 * WARUM ES DIESEN WÄCHTER GIBT: 1079 PRÜFFÄLLE HABEN DEN FEHLER NICHT GESEHEN
 * ---------------------------------------------------------------------------
 * `progressOf` kannte die Umkehrung schon, die Kurzfassung meldete 85 % — und
 * die **Anzeige** zeigte 100 %. Sie rechnete selbst:
 *
 *     const progress = view.bossHp > 0 ? (100 * view.damage) / view.bossHp : 0;
 *
 * Gefunden hat es kein Prüffall, sondern ein Screenshot. Der Grund ist
 * strukturell: `overlay.ts` braucht ein Dokument, und die Prüffälle laufen
 * ohne DOM (`environment: 'node'`). Was dort steht, ist mit Vitest nicht
 * spielbar — also bleibt der Quelltext als Messgegenstand.
 *
 * DIE ZUTAT IST WEG, NICHT NUR DER AUFRUF
 * ---------------------------------------
 * `ResultView.bossHp` wurde durch `progressPercent` ersetzt. Wer die Formel
 * hier wieder aufschreiben wollte, fände den Nenner nicht mehr — das erzwingt
 * schon `tsc`. Dieser Wächter deckt den Rest ab: eine Rechnung aus anderen
 * Zutaten (`combat.boss.hp`, `summary.bossHp`), die dasselbe täte.
 *
 * Es ist dieselbe Gestalt wie bei `applyBindings` (#264) und bei den zwei
 * Schreibstellen des Fortschritts (#218): **Die Regel steht an zwei Orten, und
 * einer läuft weg.** Der Unterschied ist nur, wo sie steht — dort in der
 * Logik, hier in der Anzeige.
 *
 * WAS ER NICHT KANN: Er prüft Text, nicht Bedeutung. Wer über eine
 * Hilfsvariable rechnet, kommt daran vorbei. Das steht hier, damit niemand die
 * Prüfung für mehr hält, als sie ist.
 */
import { describe, expect, it } from 'vitest';

const QUELLE = import.meta.glob('/src/render/{overlay,hud}.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function quelleVon(datei: string): string {
  const pfad = Object.keys(QUELLE).find((k) => k.endsWith(datei));
  const text = pfad === undefined ? undefined : QUELLE[pfad];
  if (text === undefined || text.length === 0) {
    throw new Error(`${datei} nicht geladen — die Pruefung wuerde nichts messen`);
  }
  return text;
}

/**
 * Zeilen, die aus Schaden und Lebenspunkten einen Prozentsatz rechnen.
 *
 * Der Ausdruck sucht das Muster, nicht den Namen: `100 * irgendwas` geteilt
 * durch irgendwas. Kommentare fallen weg, sonst schlüge die Beschreibung des
 * Fehlers oben selbst an — die Falle, in die ein Wächter leicht tappt, der
 * seine eigene Begründung mitliest.
 */
export function prozentRechnungen(quelle: string): string[] {
  return quelle
    .split('\n')
    .map((zeile, i) => ({ zeile: zeile.trim(), nummer: i + 1 }))
    .filter(({ zeile }) => !zeile.startsWith('*') && !zeile.startsWith('//'))
    .filter(({ zeile }) => /100\s*\*[^;]*\//.test(zeile))
    .map(({ zeile, nummer }) => `${nummer}: ${zeile}`);
}

describe('Die Anzeige rechnet den Fortschritt nicht selbst', () => {
  it('findet den Quelltext ueberhaupt', () => {
    // Ohne diese Zusicherung wäre ein leerer Treffer unten grün — und der
    // Wächter meldete Ruhe, weil er nichts gelesen hat.
    expect(quelleVon('overlay.ts').length).toBeGreaterThan(1000);
    expect(quelleVon('hud.ts').length).toBeGreaterThan(1000);
  });

  it('hat in overlay.ts keine Prozentrechnung stehen', () => {
    expect(prozentRechnungen(quelleVon('overlay.ts'))).toEqual([]);
  });

  it('SIEHT eine, wenn jemand sie einfuegt — die Gegenprobe', () => {
    // Ohne diesen Fall wüsste niemand, ob der Ausdruck oben überhaupt greift
    // oder nur nichts findet, weil er nichts finden KANN.
    const erfunden = ['function f() {', '  const p = (100 * view.damage) / view.bossHp;', '}'].join(
      '\n',
    );
    expect(prozentRechnungen(erfunden)).toHaveLength(1);
  });

  it('nimmt die fertige Zahl entgegen, statt ihre Zutaten', () => {
    // `bossHp` im ResultView war der Nenner der alten Formel. Solange er nicht
    // dasteht, kann die Anzeige sie nicht wieder aufschreiben.
    const overlay = quelleVon('overlay.ts');
    expect(overlay).toContain('progressPercent');
    expect(/readonly bossHp/.test(overlay), 'die Zutat ist weg, nicht nur der Aufruf').toBe(false);
  });

  it('holt die Zahl in hud.ts aus progressOf', () => {
    // Die eine Stelle, die rechnen darf, ist `summary.ts`. Hier steht nur, wer
    // sie fragt.
    expect(quelleVon('hud.ts')).toContain('progressOf(combat');
  });
});
