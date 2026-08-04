/**
 * Fallraten (GAME-DESIGN 13.4, Issue E6-2).
 *
 * Fünf Regeln, alle aus dem Design, alle hier und nur hier:
 *
 * | Regel | Festlegung |
 * | --- | --- |
 * | **Obergrenze durch Inhalt** | Frühe Instanzen lassen nur Common und Rare fallen; Epic erst ab einer bestimmten Instanz, Legendary erst im Endgame |
 * | **Steigende Raten darin** | Innerhalb der erlaubten Qualitäten steigen die Raten mit Instanz, Schwierigkeitsstufe und Endlos-Stufe |
 * | **Richtwerte im Endgame** | Common 60 %, Rare 30 %, Epic 9 %, **Legendary 1 %** |
 * | **Zwei getrennte Achsen** | Qualität und Gegenstandsstufe laufen unabhängig |
 * | **Kein Pech-Schutz** | Eine ehrliche Prozentzahl, kein versteckter Zähler |
 *
 * # Warum die Obergrenze und die Raten zwei getrennte Dinge sind
 *
 * Sie ließen sich zusammenlegen — eine Tabelle, in der frühe Stufen bei Epic
 * einfach eine Null stehen haben. Das wäre kürzer und wäre falsch:
 *
 * - Die **Obergrenze** ist eine Design-Entscheidung mit einem Zweck.
 *   „Verhindert, dass ein früher Glückstreffer die restlichen zwanzig
 *   Aufstiegsstufen entwertet" (13.4). Sie ist nicht verhandelbar.
 * - Die **Raten** sind Balance-Zahlen, die der Balance-Läufer misst (#118).
 *
 * Stünden sie an einer Stelle, würde eine Balance-Anpassung eine
 * Design-Regel mitverschieben, ohne dass es jemand bemerkt. Getrennt kann die
 * Tabelle beliebig wandern, und die Obergrenze bleibt.
 *
 * # Was hier vorläufig ist und was nicht
 *
 * **Nicht vorläufig:** die Struktur (acht Aufstiegs-, sechs Endgame-Instanzen
 * aus 11.1), die drei Schwierigkeitsstufen (11.7), die Endlos-Leiter auf
 * Endgame-Bossen (12.4) und die Endgame-Richtwerte 60/30/9/1 (13.4).
 *
 * **Vorläufig:** ab welcher Instanz Epic fällt, die Zwischenzeilen der
 * Gewichtstabelle und die Umrechnung von Instanz/Schwierigkeit/Endlos-Stufe
 * in eine Fortschrittsstufe. Der Balance-Läufer entscheidet sie.
 */

import {
  QUALITY_COMMON,
  QUALITY_COUNT,
  QUALITY_EPIC,
  QUALITY_LEGENDARY,
  QUALITY_RARE,
} from '../gear/budget.js';

/** Wie viele Instanzen es gibt (11.1: 8 Aufstieg + 6 Endgame). */
export const INSTANCE_COUNT = 14;
/** Die erste Endgame-Instanz. Instanz 1–8 tragen den Aufstieg (12.4). */
export const FIRST_ENDGAME_INSTANCE = 9;

/** Die drei Schwierigkeitsstufen je Boss (11.7). */
export const DIFFICULTY_NORMAL = 0;
export const DIFFICULTY_HEROIC = 1;
export const DIFFICULTY_MYTHIC = 2;
export const DIFFICULTY_COUNT = 3;

export const DIFFICULTY_NAMES: readonly string[] = ['normal', 'heroic', 'mythic'];

/**
 * Ab welcher Instanz Epic überhaupt fallen kann. **Vorläufig (#118).**
 *
 * Das Design sagt „Epic erst ab einer bestimmten Instanz" und nennt keine.
 * Vier liegt in der Mitte des Aufstiegs — nah genug, dass die Obergrenze
 * spürbar wird, weit genug, dass die ersten Instanzen ihren Zweck erfüllen.
 */
export const EPIC_FROM_INSTANCE = 4;

/** Die Summe aller Gewichte. Zehntausend, damit ein Promille darstellbar ist. */
export const WEIGHT_TOTAL = 10_000;

/**
 * Wie viele Fortschrittsstufen die Gewichtstabelle führt.
 *
 * Die **letzte** Zeile ist die einzige, die nicht vorläufig ist — sie trägt
 * die Richtwerte aus 13.4.
 */
export const PROGRESS_STEPS = 6;

/**
 * Die Gewichte je Fortschrittsstufe, in Zehntausendsteln. **Vorläufig bis auf
 * die letzte Zeile.**
 *
 * ```text
 *   Stufe        Common   Rare   Epic   Legendary
 *     0            8500   1500      0           0
 *     …
 *     5 (Endgame)  6000   3000    900         100   <- 13.4, nicht vorläufig
 * ```
 *
 * Ganzzahlig, weil Spiel und Simulator bitgleich rechnen müssen (18.4) und
 * eine Verteilung aus Brüchen zwei Rundungsstellen hätte statt keiner.
 */
export const PROGRESS_WEIGHTS: readonly (readonly number[])[] = [
  [8500, 1500, 0, 0],
  [8000, 2000, 0, 0],
  [7500, 2200, 300, 0],
  [7000, 2500, 500, 0],
  [6500, 2800, 700, 0],
  // Die Richtwerte aus 13.4 — 60 %, 30 %, 9 %, 1 %.
  [6000, 3000, 900, 100],
];

/**
 * Die höchste Qualität, die in dieser Instanz überhaupt fallen kann.
 *
 * **Das ist die Design-Regel, nicht die Balance.** Sie steht vor den
 * Gewichten und überschreibt sie.
 */
export function qualityCapOf(instance: number): number {
  if (!Number.isInteger(instance) || instance < 1 || instance > INSTANCE_COUNT) {
    return QUALITY_COMMON;
  }
  if (instance >= FIRST_ENDGAME_INSTANCE) return QUALITY_LEGENDARY;
  if (instance >= EPIC_FROM_INSTANCE) return QUALITY_EPIC;
  return QUALITY_RARE;
}

/**
 * Die Fortschrittsstufe eines Kampfes. **Die Umrechnung ist vorläufig.**
 *
 * Alle drei Achsen zahlen ein, wie 13.4 es verlangt: Instanz,
 * Schwierigkeitsstufe und Endlos-Stufe.
 */
export function progressOf(instance: number, difficulty: number, endlessLevel: number): number {
  const i = Number.isInteger(instance) ? Math.min(Math.max(instance, 1), INSTANCE_COUNT) : 1;
  const d = Number.isInteger(difficulty)
    ? Math.min(Math.max(difficulty, 0), DIFFICULTY_COUNT - 1)
    : 0;
  const e = Number.isInteger(endlessLevel) && endlessLevel > 0 ? endlessLevel : 0;
  const raw = i - 1 + 4 * d + e;
  return Math.min(Math.floor(raw / 4), PROGRESS_STEPS - 1);
}

/**
 * Die Gewichte für einen Kampf, in ein übergebenes Feld geschrieben (18.2).
 *
 * **Die Obergrenze wird NACH der Tabelle angewandt**, und das Gewicht einer
 * gesperrten Qualität fällt der höchsten erlaubten zu — es verschwindet
 * nicht. Sonst summierten sich die Gewichte nicht mehr auf
 * [`WEIGHT_TOTAL`], und ein Wurf am oberen Rand fiele ins Leere.
 */
export function weightsFor(
  instance: number,
  difficulty: number,
  endlessLevel: number,
  out: Int32Array | number[],
): void {
  const row = PROGRESS_WEIGHTS[progressOf(instance, difficulty, endlessLevel)] ?? [];
  const cap = qualityCapOf(instance);
  let gesperrt = 0;
  for (let quality = 0; quality < QUALITY_COUNT; quality++) {
    const gewicht = row[quality] ?? 0;
    if (quality > cap) {
      gesperrt += gewicht;
      out[quality] = 0;
    } else {
      out[quality] = gewicht;
    }
  }
  out[cap] = (out[cap] ?? 0) + gesperrt;
}

/**
 * Welche Qualität ein Wurf ergibt.
 *
 * `roll` ist eine Zahl aus **[0, 1)** — eine einzige Ziehung aus dem
 * Beute-Strom. Genau eine, damit ein zusätzlicher Wurf nicht die Folge aller
 * späteren verschiebt.
 *
 * **Kein Pech-Schutz:** Die Funktion hat keinen Zustand. Sie kann gar nicht
 * wissen, wie oft zuvor nichts fiel — das ist die Regel aus 13.4 in der
 * Bauform ausgedrückt statt in einer Prüfung.
 */
export function qualityOfRoll(weights: Int32Array | number[], roll: number): number {
  const gezogen = Math.floor(roll * WEIGHT_TOTAL);
  let grenze = 0;
  for (let quality = 0; quality < QUALITY_COUNT; quality++) {
    grenze += weights[quality] ?? 0;
    if (gezogen < grenze) return quality;
  }
  // Erreichbar nur, wenn die Gewichte nicht auf WEIGHT_TOTAL summieren —
  // dann ist die höchste erlaubte Qualität die ehrlichste Antwort.
  return QUALITY_COMMON;
}

/**
 * Die Gegenstandsstufe eines Bosses. **Die Zahlen sind vorläufig (#118).**
 *
 * > „Beute aus altem Inhalt: ja, **aber auf der Gegenstandsstufe des
 * > Bosses**. Wer ihm entwachsen ist, bekommt Übertroffenes." (13.4)
 *
 * Deshalb hängt sie **nicht** an der Spielerstufe und **nicht** an der
 * gefallenen Qualität — das ist die zweite der beiden getrennten Achsen.
 */
export function itemLevelOf(instance: number, difficulty: number, endlessLevel: number): number {
  const i = Number.isInteger(instance) ? Math.min(Math.max(instance, 1), INSTANCE_COUNT) : 1;
  const d = Number.isInteger(difficulty)
    ? Math.min(Math.max(difficulty, 0), DIFFICULTY_COUNT - 1)
    : 0;
  const e = Number.isInteger(endlessLevel) && endlessLevel > 0 ? endlessLevel : 0;
  return (i - 1) * 6 + d * 3 + e + 1;
}

/**
 * Darf dieses Stück bei diesem Boss fallen?
 *
 * > „Bestimmte Gegenstände fallen **nur bei einem bestimmten Boss**. Jeder
 * > Boss bekommt einen eigenen Grund, wiederholt zu werden." (13.4)
 *
 * `boundToBoss` leer heißt: ungebunden, fällt überall.
 */
export function mayDropAt(boundToBoss: string, bossId: string): boolean {
  return boundToBoss === '' || boundToBoss === bossId;
}
