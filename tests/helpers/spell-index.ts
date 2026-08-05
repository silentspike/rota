import type { Combat } from '../../src/core/combat.js';
import { CALCINATION } from '../../src/core/data.js';

/**
 * # Der `combat`-Parameter: eine Annahme weniger, kein messbarer Gewinn
 *
 * Eine zweite Fassung dieses Helfers las `combat.spells` statt der Schule
 * (#394). Gemessen, bevor sie übernommen wurde:
 *
 * ```
 * src/core/combat.ts:688   this.spells = options.school.spells;
 * ```
 *
 * `combat.spells` **ist** `school.spells` — direkt zugewiesen, und es gibt
 * heute genau eine Schule. Die beiden Fassungen können also nirgends
 * verschiedene Ergebnisse liefern.
 *
 * Schärfer noch: `CALCINATION` trägt die Zauber aus **allen sieben Dateien**
 * (`data.ts` setzt coagulation, fixation, putrefaction und die übrigen
 * zusammen). Die eine Schule ist heute kein Ausschnitt, sondern der ganze
 * Bestand — es gibt derzeit nichts, wovon ein zweiter Kampf sich
 * unterscheiden könnte.
 *
 * **Aber „man kann es heute nicht messen" wäre falsch.** Ein Fall, in dem die
 * beiden Fassungen Verschiedenes liefern, ist in vier Zeilen gebaut: `Combat`
 * nimmt jede `SchoolDef`, also auch eine mit gekürzter Zauberliste. Dann zählt
 * `spellIndex(id)` in der vollen Schule und `spellIndex(id, combat)` in der
 * des Kampfes — und die Zahlen gehen auseinander. `spell-index-helper.test.ts`
 * hält das fest.
 *
 * Was fehlt, ist also nicht die Wirkung, sondern **ein Aufrufer im Bestand,
 * der sie braucht.** Der Unterschied ist wichtig: „nicht messbar" klingt nach
 * Vorrat auf Verdacht; „heute nutzt es niemand" ist eine Aussage über den
 * Bestand, nicht über die Fähigkeit.
 */
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
export function spellIndex(id: string, combat?: Combat): number {
  const zauber = combat?.spells ?? CALCINATION.value.spells;
  const i = zauber.findIndex((spell) => spell.id === id);
  if (i < 0) throw new Error(`Zauber ${id} gibt es nicht`);
  return i;
}
