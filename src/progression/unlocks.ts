/**
 * Was eine Stufe freischaltet (GAME-DESIGN 12.1; Issue #248, AC3 und AC4).
 *
 * ZWEI DINGE, DIE NICHTS MITEINANDER ZU TUN HABEN
 * -----------------------------------------------
 * **Welche Zauber** verfügbar sind, sagt `unlockLevel` am Zauber. **Wie viele
 * Leisten-Plätze** offen sind, sagt die Stufe. Beides wächst mit dem Aufstieg
 * und darf trotzdem nicht verschmelzen: Ein Spieler hat regelmäßig mehr Zauber
 * als Plätze — genau das ist die Abwägung, um die es geht (4.2: „Zwölf Zauber
 * bei zwölf Leisten-Plätzen bedeutet: Man kann nicht alle mitnehmen").
 *
 * DIE PLÄTZE VERSCHIEBEN SICH NICHT
 * ---------------------------------
 * GAME-DESIGN 9.1: „Positionen müssen von Anfang an fest vergeben werden, damit
 * das Wachsen bestehende Plätze nicht verschiebt — sonst leidet das
 * Muskelgedächtnis."
 *
 * Deshalb sagt die Stufe **nur eine Zahl**: wie viele der fest nummerierten
 * Plätze offen sind. Platz 3 ist auf Stufe 10 derselbe wie auf Stufe 90. Wer
 * hier stattdessen eine Liste zurückgäbe, deren Reihenfolge sich ändern kann,
 * hätte die Regel bereits gebrochen — auch wenn heute zufällig dieselbe
 * Reihenfolge herauskäme.
 *
 * `unlockLevel` STAND SEIT LANGEM IN DEN DATEN UND WURDE VON NIEMANDEM
 * GELESEN. Der Leser trug es, kein Aufrufer fragte danach. Das ist das Muster,
 * das in diesem Projekt schon viermal aufgefallen ist; hier bekommt es seinen
 * Aufrufer.
 */
import { LEVELS, type SpellDef } from '../core/data.js';

/**
 * Wie viele Leisten-Plätze auf dieser Stufe offen sind.
 *
 * Die Zahl steigt mit der Stufe und **sinkt nie**: Ein Spielstand, der eine
 * höhere Stufe trug, verliert keinen Platz, wenn die Tabelle später wächst.
 */
/**
 * Wie viele Talentpunkte diese Stufe insgesamt eingebracht hat (Issue #379).
 *
 * **Gerechnet, nicht gespeichert** — dieselbe Linie wie die Grundwerte, die
 * Leisten-Plätze und die freigeschalteten Zauber (GAME-DESIGN 18.6). Der
 * Spielstand hält die Stufe und die AUSGABEN je Zweig; wie viele Punkte
 * verdient wurden, folgt daraus.
 *
 * Ein gespeichertes Guthaben ginge nicht kaputt, es würde nur alt: Eine
 * geänderte Verteilung erreichte bestehende Spielstände nie mehr, und niemand
 * merkte es.
 */
export function talentPointsAtLevel(level: number): number {
  const stufe = Math.max(Math.trunc(level), 1);
  let punkte = 0;
  for (const ab of LEVELS.talentPointLevels) {
    if (stufe >= ab) punkte++;
  }
  return punkte;
}

export function barSlotsAtLevel(level: number): number {
  const stufe = Math.max(Math.trunc(level), 1);
  let plaetze = LEVELS.startBarSlots;
  for (const ab of LEVELS.barSlotLevels) {
    if (stufe >= ab) plaetze++;
  }
  return plaetze;
}

/**
 * Welche Plätze auf dieser Stufe offen sind — als feste Nummern.
 *
 * **Nicht nur die Anzahl, sondern die Nummern selbst**, und das hat einen
 * Grund, der beim Prüfen aufgefallen ist: „Die Plätze verschieben sich nicht"
 * lässt sich an einer Zahl nicht messen. Ein Prüffall, der die Belegung aus
 * der Anzahl selbst herstellt, vergleicht seine eigene Erwartung mit sich —
 * er ist grün, egal was die Umsetzung tut.
 *
 * Mit dieser Funktion vergleicht er zwei echte Ausgaben. Und die Anzeige
 * braucht sie ohnehin: 9.1 verlangt „nur freigeschaltete Plätze zeigen".
 */
export function unlockedSlots(level: number): number[] {
  return Array.from({ length: barSlotsAtLevel(level) }, (_, index) => index);
}

/**
 * Ist dieser Zauber auf dieser Stufe verfügbar?
 *
 * Die Frage lautet **nicht** „liegt er auf der Leiste" — das entscheidet der
 * Spieler. Sie lautet, ob er ihn überhaupt legen dürfte.
 */
export function isSpellUnlocked(spell: SpellDef, level: number): boolean {
  return Math.trunc(level) >= spell.unlockLevel;
}

/**
 * Die auf dieser Stufe verfügbaren Zauber, in der Reihenfolge der Quelle.
 *
 * **Reihenfolge unverändert**, nicht nach Stufe sortiert: Sortieren würde die
 * Liste bei jedem Aufstieg umordnen, und eine Anzeige, die daran hängt, hüpfte
 * — dieselbe Regel wie bei den Plätzen, eine Ebene höher.
 */
export function unlockedSpells(spells: readonly SpellDef[], level: number): SpellDef[] {
  return spells.filter((spell) => isSpellUnlocked(spell, level));
}

/**
 * Die Stufe, auf der der nächste Platz aufgeht — oder `null`, wenn keiner mehr
 * kommt.
 *
 * Für die Anzeige im Player Home. `null` statt einer erfundenen Zahl: Ein
 * Spieler auf der letzten Freischalt-Stufe soll „keiner mehr" lesen, nicht eine
 * Stufe, die es nicht gibt.
 */
export function nextBarSlotLevel(level: number): number | null {
  const stufe = Math.trunc(level);
  for (const ab of LEVELS.barSlotLevels) {
    if (stufe < ab) return ab;
  }
  return null;
}
