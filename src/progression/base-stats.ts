/**
 * Die Grundwerte des Spielers (Issue #188).
 *
 * SIE STEHEN IN DEN DATEN, NICHT IM CODE
 * --------------------------------------
 * `data/player/base.json` fuehrt sie, und die Datei sagt selbst, dass sie
 * vorlaeufig sind: „NICHT BALANCIERT … Der Balance-Laeufer setzt sie spaeter."
 * Genau deshalb duerfen sie nicht in der Anzeige abgeschrieben stehen — eine
 * Zahl an zwei Orten wird beim Nachziehen an einem vergessen.
 *
 * ESSENCE HAT HEUTE KEINEN GRUNDWERT
 * ----------------------------------
 * `base.json` fuehrt `vessel` und `wardReduction`, sonst nichts. ESSENCE ist
 * der Vorrat der Schule und steht in den Schuldaten, nicht am Spieler; ohne
 * Ausruestung ist der Zuschlag 0. Hier eine Zahl zu erfinden hiesse, dem
 * Spieler etwas vorzurechnen, das nirgends herkommt.
 */
import { PLAYER, statAtLevel } from '../core/data.js';
import type { BaseStats } from './stat-overview.js';

export function baseStats(level = 1): BaseStats {
  return {
    // Kein Grundwert in den Daten -- ohne Ausruestung ist er 0.
    essence: 0,
    // **Der Wert DIESER Stufe** (Issue #289). Vorher stand hier der Grundwert
    // aus base.json, waehrend die Anzeige daneben "Stat growth x4.116"
    // zeigte -- eine Zahl, die den Fortschritt behauptete, und eine, die ihn
    // nicht abbildete.
    vessel: statAtLevel(PLAYER.vessel, level),
    // Als Prozentsatz, wie ihn der Spieler liest: 0,2 heisst 20 % weniger
    // eingehender Schaden (combat.ts liest denselben Wert als Faktor).
    ward: PLAYER.wardReduction * 100,
  };
}
