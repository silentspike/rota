/**
 * Der abnehmende Ertrag über den Stufenabstand (GAME-DESIGN 12.6; Issue #276).
 *
 * WARUM DAS EIN EIGENES MODUL IST UND NICHT ZWEI FUNKTIONEN
 * --------------------------------------------------------
 * 12.6 macht **eine** Aussage: „Der Ertrag sinkt mit dem Stufenabstand: voller
 * Ertrag solange der Boss zur eigenen Stufe passt, bei ~10 Stufen Abstand nur
 * noch ein Bruchteil, bei ~20 praktisch nichts." Dort steht „der Ertrag" —
 * nicht „die Erfahrung", nicht „die Punkte". Es gibt zwei Währungen und eine
 * Regel.
 *
 * Zwei Kurven für eine Design-Aussage wären zwei Gelegenheiten, sie zu
 * verdrehen. Läge die Funktion bei der Erfahrung, müsste die Punkterechnung
 * aus `level.ts` importieren; läge sie bei den Punkten, umgekehrt. Beides
 * behauptete eine Zugehörigkeit, die es nicht gibt.
 *
 * `tests/unit/reward-falloff.test.ts` hält beide Währungen gegeneinander: Er
 * fällt, sobald eine von ihnen anders abfällt als die andere. Ein Kommentar
 * „beide rufen dieselbe Funktion" hätte das nicht getan.
 *
 * DIE RICHTUNG IST GERICHTET
 * --------------------------
 * Gezählt wird nur, wie weit der Spieler **über** dem Boss steht. Ein Boss
 * über der eigenen Stufe zahlt voll — er ist schwerer, nicht wertloser. Der
 * abnehmende Ertrag zielt auf Inhalt, dem man entwachsen ist: GAME-DESIGN
 * Zeile 1165 sagt „Instanz 1 Mythisch muss zeitnah gespielt werden, sonst
 * wirft es nichts ab".
 */
import { REWARD } from '../core/data.js';

/**
 * Der Stufenabstand, der den Ertrag mindert.
 *
 * Nur nach oben: `max(0, Spieler - Boss)`. Ein negativer Abstand — Boss über
 * dem Spieler — ist kein Abstand im Sinne von 12.6.
 */
export function levelGap(bossLevel: number, playerLevel: number): number {
  const boss = Math.max(Math.trunc(bossLevel), 1);
  const spieler = Math.max(Math.trunc(playerLevel), 1);
  return Math.max(0, spieler - boss);
}

/**
 * Der Faktor in Promille, 1000 heißt voller Ertrag.
 *
 * Jenseits der Tabelle 0: Wer 100 Stufen über einem Boss steht, bekommt
 * nichts — dieselbe Aussage wie der letzte Tabelleneintrag, nur ohne dass ein
 * Zugriff ins Leere läuft.
 */
export function rewardFalloffPerMille(bossLevel: number, playerLevel: number): number {
  const abstand = levelGap(bossLevel, playerLevel);
  return REWARD.falloffPerMille[abstand] ?? 0;
}

/**
 * Einen Ertrag um den Abstand mindern.
 *
 * **Ganzzahlig gerechnet**, in derselben Form wie `basePermille` in
 * `src/gear/sources.ts`: Betrag mal Promille, dann abgeschnitten. Der Weg über
 * eine Fließkommazahl wäre hier unnötig — beide Währungen sind ganzzahlig, und
 * `Math.trunc` am Ende einer Kette aus Fließkommazahlen ist der Ort, an dem
 * zwei Aufrufer auseinanderlaufen.
 */
export function applyRewardFalloff(amount: number, bossLevel: number, playerLevel: number): number {
  const promille = rewardFalloffPerMille(bossLevel, playerLevel);
  return Math.trunc((Math.trunc(amount) * promille) / 1000);
}
