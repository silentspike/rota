/**
 * Die Schadensformel (GAME-DESIGN 6.3).
 *
 * ```
 * Schaden = Grundschaden
 *         × ESSENCE-Faktor
 *         × Talent-Faktoren
 *         × aktive Verstärkungen
 *         × Synergie-Faktoren
 *         × Widerstandsfaktor des Bosses
 *         × Krit
 * ```
 *
 * **Die Reihenfolge der Multiplikationen ist festgelegt und für Spiel und Simulator
 * identisch.** Fließkomma-Multiplikation ist nicht assoziativ: `(a × b) × c` und
 * `a × (b × c)` können sich im letzten Bit unterscheiden. Bei zehntausenden Treffern
 * je Balance-Lauf summiert sich das zu einer Abweichung, die niemand mehr zuordnen
 * kann. Deshalb steht die Reihenfolge hier an genau einer Stelle.
 *
 * In Schritt 1 sind alle Faktoren 1,0 und Krit ruht (STEP1-SPEC § 2) — die Struktur
 * steht trotzdem schon, weil sie nachträglich einzuziehen jede aufgezeichnete Runde
 * ungültig machen würde.
 */

/** Die Faktoren in der Reihenfolge, in der sie angewandt werden. */
export interface DamageFactors {
  /** Aus dem Wert ESSENCE (GAME-DESIGN 6.1). */
  readonly essence: number;
  /** Aus dem Talentbaum. */
  readonly talents: number;
  /** Laufende Verstärkungen auf dem Spieler. */
  readonly buffs: number;
  /** Offene Synergie-Fenster (GAME-DESIGN 5). */
  readonly synergy: number;
  /** Widerstand des Bosses gegen die Schule (GAME-DESIGN 11.4). */
  readonly resistance: number;
  /** 1,0 ohne Krit, sonst der YIELD-Faktor. */
  readonly crit: number;
}

/** Alle Faktoren neutral — der Zustand auf Stufe 1 ohne Ausrüstung. */
export const NEUTRAL_FACTORS: DamageFactors = {
  essence: 1,
  talents: 1,
  buffs: 1,
  synergy: 1,
  resistance: 1,
  crit: 1,
};

/**
 * Schaden eines einzelnen Treffers.
 *
 * Gerundet wird **einmal am Ende**, nicht zwischendurch: Zwischenrundungen wären
 * eine zweite Stelle, an der Spiel und Simulator auseinanderlaufen könnten. Der
 * Rundungspunkt gehört damit zur gemeinsamen Schnittstelle.
 */
export function computeDamage(baseDamage: number, factors: DamageFactors): number {
  let value = baseDamage;
  value *= factors.essence;
  value *= factors.talents;
  value *= factors.buffs;
  value *= factors.synergy;
  value *= factors.resistance;
  value *= factors.crit;
  return Math.round(value);
}
