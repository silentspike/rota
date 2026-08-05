import { CALCINATION } from '../../src/core/data.js';

/**
 * Der Index eines Zaubers, **über den Bezeichner statt über die Zahl**
 * (GAME-DESIGN 18.7, Issue #369).
 *
 * # Warum diese vier Zeilen an einer Stelle stehen
 *
 * Sie standen sechsmal im Projekt, unter zwei Namen: `spellIndex` in
 * `interrupt-losses` und `run-analysis` (aus #268), `indexVon` in vier
 * weiteren Dateien (aus #369). Zwei Sitzungen lösten dasselbe Problem in
 * derselben Stunde, ohne voneinander zu wissen.
 *
 * Sechs Kopien derselben Logik sind kein Schaden, solange sie gleich bleiben —
 * und genau das kann niemand zusichern. Ändert sich die Fehlerbehandlung an
 * einer Stelle, laufen die anderen fünf still weiter (WORKER-GUIDE 8.21).
 *
 * # Der Wurf ist Absicht
 *
 * Ein unbekannter Bezeichner ergäbe `-1`, und das ist ein gültiger Index für
 * „gibt es nicht" — der Prüffall liefe weiter und misst nichts. Ein Fehler
 * hier zeigt auf den Tippfehler statt auf seine Folge.
 */
export function spellIndex(id: string): number {
  const i = CALCINATION.value.spells.findIndex((spell) => spell.id === id);
  if (i < 0) throw new Error(`Zauber ${id} gibt es nicht`);
  return i;
}
