/**
 * Stufe und Erfahrung (GAME-DESIGN 12.1; Issue #248).
 *
 * WAS HIER STEHT UND WAS NICHT
 * ----------------------------
 * Hier steht die Rechnung: Wie viel Erfahrung ein Sieg bringt und wann sie zur
 * nächsten Stufe reicht. Was daraus folgt — Grundwerte, freigeschaltete Zauber,
 * Leisten-Plätze — steht anderswo und wird aus der Stufe **gerechnet**, nicht
 * gespeichert (GAME-DESIGN 18.5, AC5 des Issues).
 *
 * Der Unterschied ist still und teuer: Ein gespeicherter Wert geht nicht
 * kaputt, er wird nur alt. Eine Balance-Änderung erreicht bestehende
 * Spielstände dann nie mehr, und niemand merkt es.
 *
 * DIE ZAHLEN STEHEN IN DEN DATEN
 * ------------------------------
 * `data/player/levels.json` trägt die Kurve als Tabelle, samt der erzeugenden
 * Formel und den vier Varianten, die zur Wahl standen. Der Grund für die
 * Tabelle ist derselbe wie bei den Grundwerten: `n^1.5` ist eine
 * Fließkomma-Potenz, und `Math.pow` in JavaScript ist gegenüber `f64::powf` in
 * Rust nicht garantiert bitgleich (GAME-DESIGN 18.4).
 *
 * ÜBERSCHÜSSIGE ERFAHRUNG WIRD ÜBERNOMMEN
 * ---------------------------------------
 * Wer mit 190 von 200 in einen Kampf geht und 1000 gewinnt, steht danach auf
 * der nächsten Stufe **und** hat 990 übrig. Die Alternative wäre, den Rest
 * verfallen zu lassen — das kostet über den ganzen Aufstieg 42 zusätzliche
 * Kämpfe (1365 statt 1323) und bestraft den Spieler dafür, dass er einen
 * starken Boss gewählt hat.
 */
import { LEVELS } from '../core/data.js';

/** Die Stufe, mit der jeder Spielstand beginnt. */
export const START_LEVEL = 1;

/** Stand des Aufstiegs: was erreicht wurde, sonst nichts. */
export interface LevelState {
  /** Erreichte Stufe, mindestens 1. */
  readonly level: number;
  /** Erfahrung **auf dieser Stufe**, noch nicht für den Aufstieg verbraucht. */
  readonly xp: number;
}

/** Der Anfangszustand. */
export function newLevelState(): LevelState {
  return { level: START_LEVEL, xp: 0 };
}

/**
 * Erfahrung für einen Sieg.
 *
 * **Zwei Stufen gehen ein, auch wenn heute nur eine wirkt.** GAME-DESIGN 12.6
 * sieht einen abnehmenden Ertrag über den Stufenabstand vor; er ist noch nicht
 * gebaut und gehört in ein eigenes Issue. Die Unterschrift nimmt ihn trotzdem
 * schon auf — sonst müsste jede Aufrufstelle später geändert werden, und eine
 * davon würde vergessen.
 *
 * @param bossLevel Stufe des besiegten Bosses
 * @param playerLevel Stufe des Spielers — heute ohne Wirkung, siehe oben
 */
export function xpForVictory(bossLevel: number, playerLevel: number): number {
  void playerLevel;
  const stufe = Math.max(Math.trunc(bossLevel), 1);
  return stufe * LEVELS.xpPerBossLevel;
}

/**
 * Was der Aufstieg von dieser Stufe zur nächsten kostet.
 *
 * Auf der Höchststufe gibt es kein Weiter: `Infinity` statt einer Zahl, damit
 * kein Vergleich zufällig wahr wird. Eine 0 hier hieße „sofort aufsteigen".
 */
export function xpToNext(level: number): number {
  const stufe = Math.trunc(level);
  if (stufe >= LEVELS.maxLevel) return Number.POSITIVE_INFINITY;
  return LEVELS.xpToNextLevel[Math.max(stufe, 1) - 1] ?? Number.POSITIVE_INFINITY;
}

/**
 * Erfahrung gutschreiben und so oft aufsteigen, wie sie reicht.
 *
 * **Eine Schleife, kein einzelner Aufstieg.** Ein Sieg auf einer viel höheren
 * Boss-Stufe kann mehrere Stufen auf einmal bringen; wer nur einmal aufsteigt,
 * lässt den Rest liegen und der Spieler wundert sich über eine Anzeige, die
 * mehr zeigt, als sie zulässt.
 *
 * **Ohne Seiteneffekt**: Der alte Zustand bleibt, der neue wird
 * zurückgegeben. So lässt sich derselbe Sieg zweimal rechnen, ohne dass beim
 * zweiten Mal etwas anderes herauskommt (18.4).
 */
export function grantXp(state: LevelState, amount: number): LevelState {
  if (amount <= 0) return state;
  let level = Math.max(Math.trunc(state.level), START_LEVEL);
  let xp = state.xp + Math.trunc(amount);

  while (level < LEVELS.maxLevel) {
    const bedarf = xpToNext(level);
    if (xp < bedarf) break;
    xp -= bedarf;
    level++;
  }

  // Auf der Höchststufe sammelt sich nichts mehr an: Eine wachsende Zahl ohne
  // Ziel wäre eine Anzeige, die etwas verspricht, das nicht kommt.
  if (level >= LEVELS.maxLevel) return { level: LEVELS.maxLevel, xp: 0 };
  return { level, xp };
}

/**
 * Anteil bis zur nächsten Stufe, zwischen 0 und 1.
 *
 * Für die Anzeige — und ausdrücklich **gerechnet**, nicht gespeichert. Auf der
 * Höchststufe ist er 1: Der Balken steht voll, statt bei null zu hängen.
 */
export function levelProgress(state: LevelState): number {
  const bedarf = xpToNext(state.level);
  if (!Number.isFinite(bedarf)) return 1;
  return Math.min(state.xp / bedarf, 1);
}
