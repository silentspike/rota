/**
 * `statAtLevel` hat einen Aufrufer — und behaelt ihn (Issue #289, AC7).
 *
 * DIE LEHRE AUS DEM BEFUND SELBST
 * -------------------------------
 * Die Funktion gab es seit #248, mit siebzehn Prueffaellen. Sie rechnete
 * richtig und wurde von niemandem gerufen: Ein Spieler auf Stufe 100 war
 * genauso stark wie einer auf Stufe 1, waehrend das Player Home ihm
 * „Stat growth x4.116" zeigte.
 *
 * Das ist der Unterschied zu den neun Fundstellen davor derselben Klasse:
 * Dort FEHLTE eine Information. Hier stand eine FALSCHE.
 *
 * WARUM DIESER WAECHTER UND KEIN BREITERER
 * ----------------------------------------
 * Ein Waechter, der IRGENDEINE Funktion ohne Aufrufer meldet, waere zu breit —
 * er schluege bei jedem absichtlich exportierten Zugang an und waere nach der
 * dritten Ausnahmeliste tot. Dieser haelt EINE Funktion an ihrem Aufrufer
 * fest, und er kostet zwoelf Zeilen.
 *
 * WAS ER NICHT KANN: Er prueft, DASS gerufen wird, nicht ob das Ergebnis
 * ankommt. Dafuer stehen die Faelle in `level-stats-wired.test.ts` daneben —
 * sie messen die Wirkung.
 */
import { describe, expect, it } from 'vitest';

const QUELLEN = import.meta.glob('/src/**/*.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

/**
 * Alle Dateien, die `statAtLevel` AUFRUFEN.
 *
 * **Kommentare zaehlen nicht, und das war ein Befund an diesem Waechter
 * selbst.** Mein erster Entwurf las die ganze Datei — und blieb bei der
 * Gegenprobe gruen, weil in `combat.ts` ein Kommentar steht, der
 * `statAtLevel(0.2, 30)` ZITIERT, um zu begruenden, warum WARD nicht
 * skaliert.
 *
 * Ein Waechter, der an der eigenen Begruendung anschlaegt, meldet Ruhe, wo
 * keine ist. Dieselbe Falle wie beim Wort-Waechter in #270 — nur andersherum:
 * Dort haette der Kommentar einen Fehlalarm ausgeloest, hier hat er einen
 * echten Alarm verschluckt.
 *
 * Die Definition zaehlt ebenfalls nicht: Sie ist kein Aufruf.
 */
function aufrufer(): string[] {
  const treffer: string[] = [];
  for (const [pfad, text] of Object.entries(QUELLEN)) {
    if (typeof text !== 'string') continue;
    const code = text
      .split('\n')
      .map((zeile) => zeile.trim())
      .filter(
        (zeile) => !zeile.startsWith('*') && !zeile.startsWith('//') && !zeile.startsWith('/*'),
      )
      .join('\n')
      .replace(/export function statAtLevel[^\n]*/g, '');
    if (/statAtLevel\s*\(/.test(code)) treffer.push(pfad);
  }
  return treffer.sort();
}

describe('AC7 — die Rechnung wird auch gerufen', () => {
  it('findet den Quelltext ueberhaupt', () => {
    // Ohne diese Zusicherung waere alles Weitere gruen, weil nichts gelesen
    // wurde — und der Waechter meldete Ruhe, wo er nichts gesehen hat.
    expect(Object.keys(QUELLEN).length).toBeGreaterThan(20);
    expect(Object.values(QUELLEN).join('')).toContain('statAtLevel');
  });

  it('hat MINDESTENS EINEN Aufrufer in src/', () => {
    const gefunden = aufrufer();
    expect(gefunden, 'niemand ruft statAtLevel — die Stufe wirkt wieder nicht').not.toEqual([]);
  });

  it('ruft sie DORT, wo die Werte entstehen', () => {
    // Ein Aufruf irgendwo genuegt nicht: Er muss an den zwei Stellen stehen,
    // die den Spieler betreffen — im Kampf und in der Anzeige.
    const gefunden = aufrufer().join(' ');
    expect(gefunden, 'der Kampf rechnet ohne die Stufe').toContain('core/combat.ts');
    expect(gefunden, 'die Anzeige zeigt den Grundwert').toContain('progression/base-stats.ts');
  });

  it('LIEST KEINE KOMMENTARE — die Gegenprobe, die den Waechter selbst traf', () => {
    // `combat.ts` zitiert `statAtLevel(0.2, 30)` in einem Kommentar, um zu
    // begruenden, warum WARD nicht skaliert. Mein erster Entwurf zaehlte das
    // als Aufruf und blieb deshalb gruen, als ich den echten entfernte.
    const nurKommentar = ['/**', ' * so ginge es nicht: statAtLevel(0.2, 30)', ' */'].join('\n');
    const code = nurKommentar
      .split('\n')
      .map((z) => z.trim())
      .filter((z) => !z.startsWith('*') && !z.startsWith('//') && !z.startsWith('/*'))
      .join('\n');
    expect(/statAtLevel\s*\(/.test(code), 'ein Zitat ist kein Aufruf').toBe(false);
  });

  it('SIEHT einen fehlenden Aufruf — die Gegenprobe', () => {
    // Ohne sie waere nicht gezeigt, dass die Suche oben etwas findet.
    const ohneAufruf = 'export function statAtLevel(base, level) { return base; }';
    const ohneDefinition = ohneAufruf.replace(/export function statAtLevel[^\n]*/g, '');
    expect(/statAtLevel\s*\(/.test(ohneDefinition)).toBe(false);

    const mitAufruf = 'const hp = statAtLevel(PLAYER.vessel, level);';
    expect(/statAtLevel\s*\(/.test(mitAufruf)).toBe(true);
  });
});
