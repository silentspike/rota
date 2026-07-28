/**
 * Der Kampfablauf (GAME-DESIGN 10).
 *
 * ```
 *   VORBEREITUNG  --Ready-->  COUNTDOWN  --3 s-->  KAMPF  --Ende-->  AUSWERTUNG
 *        ^                                                               |
 *        +---------------------- Back to Menu ---------------------------+
 * ```
 *
 * **Vorbereitung:** Kein Ziel vorhanden, also sind alle Zauber auf den Boss
 * ausgegraut. Abklingzeiten laufen mit, **eine Uhr läuft nicht**. Schlechte
 * Vorbereitung ist die Schuld des Spielers, nicht die des Zeitdrucks.
 *
 * **Countdown:** Drei Sekunden. Danach beginnen Uhr und Eingabe **gleichzeitig** —
 * im selben Logik-Takt, nicht ungefähr.
 *
 * **Ende:** Sieg, Spieler-Tod, Zeitablauf oder Abbruch. Alle vier führen zur
 * selben Auswertung; der Abbruch ist ausdrücklich kein Sonderfall (GAME-DESIGN 10).
 * Keiner der Ausgänge zieht eine Strafe nach sich — der Gegner ist die Stoppuhr,
 * nicht der Verlust.
 */

export const PHASE_PREPARE = 0;
export const PHASE_COUNTDOWN = 1;
export const PHASE_FIGHTING = 2;
export const PHASE_ENDED = 3;

export const PHASE_NAMES: readonly string[] = ['prepare', 'countdown', 'fighting', 'ended'];

/** Wie der Kampf ausgegangen ist. */
export const OUTCOME_NONE = 0;
export const OUTCOME_VICTORY = 1;
/** Zeitablauf: Das Gefäß hält dem Druck nicht mehr stand. */
export const OUTCOME_OVERPRESSURE = 2;
/** Abbruch durch den Spieler über das Esc-Menü. */
export const OUTCOME_WIPE = 3;
/** Spieler-Tod. In Schritt 1 unerreichbar — der Testboss teilt keinen Schaden aus. */
export const OUTCOME_DEATH = 4;

export const OUTCOME_NAMES: readonly string[] = [
  'none',
  'victory',
  'overpressure',
  'wipe',
  'death',
];

/** Dauer des Countdowns (GAME-DESIGN 10). Ganzzahlig wie alle Zeiten. */
export const COUNTDOWN_MS = 3000;

/**
 * Ab dieser Restzeit warnt das Spiel vor dem Überdruck.
 *
 * Aus E1-5: Ab hier färbt sich der Ring und beginnt zu vibrieren. Klang und
 * Musik hängen an **derselben** Schwelle — zwei Warnungen zu verschiedenen
 * Zeitpunkten wären zwei Warnungen.
 */
export const OVERPRESSURE_WARNING_MS = 10_000;

/**
 * Endspurt: der Boss steht kurz vor dem Fall (GAME-DESIGN 10.1, „unter 20 %").
 *
 * Anteil der **verbliebenen** Lebenspunkte.
 */
export const LOW_HEALTH_SHARE = 0.2;

/** Zählt der Ausgang als Sieg? */
export function isVictory(outcome: number): boolean {
  return outcome === OUTCOME_VICTORY;
}

/** Ist der Kampf vorbei, gleich auf welche Weise? */
export function isFinished(outcome: number): boolean {
  return outcome !== OUTCOME_NONE;
}
