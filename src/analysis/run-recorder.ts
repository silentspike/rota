/**
 * Was am Kampfende festgehalten wird (Issue #179).
 *
 * DIE STELLE, AN DER DIE AUSWERTUNG AUFHÖRT, EINE RECHNUNG ZU SEIN
 * ----------------------------------------------------------------
 * Fortschrittsmarke und Verlauf waren gebaut, geprüft und im Browser gemessen —
 * und blieben leer, weil niemand sie schrieb. Diese Datei ist die Verbindung.
 *
 * SIE RECHNET NICHTS SELBST
 * -------------------------
 * Die Umrechnung macht `attemptFrom`, das Fortschreiben `recordAttempt`, das
 * Anhängen `appendRun`. Hier wird nur zusammengesetzt und abgelegt. Der Grund
 * ist eine Falle, die genau an dieser Aufrufstelle zuschlägt:
 *
 *   RunSummary.progressPercent   wie WEIT der Spieler kam   größer  = besser
 *   Attempt.remainingPercent     wie viel ÜBRIG blieb       kleiner = besser
 *
 * Beides Zahlen zwischen 0 und 100 mit „Percent" im Namen, beides passt
 * widerspruchslos ineinander — und wer sie hier verwechselt, dreht die ganze
 * Marke um, ohne dass der Übersetzer etwas merkt.
 *
 * WELCHE AUSGÄNGE ZÄHLEN, STEHT IN EINER LISTE
 * --------------------------------------------
 * Und nicht in einer Bedingung im Rumpf: Die Frage, ob ein Abbruch ein Versuch
 * ist, gehört dem Design und nicht dieser Datei. Als Liste ist die Antwort ein
 * Eintrag; als Bedingung wäre sie ein Umbau.
 */
import { grantXp, xpForVictory } from '../progression/level.js';
import {
  OUTCOME_DEATH,
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
} from '../core/phases.js';
import type { RunSummary } from '../core/summary.js';
import { attemptFrom, recordAttempt, type MarkUpdate } from './progress-mark.js';
import { appendRun } from './run-history.js';
import type { ProgressMarkEntry, RunHistoryEntry } from './save-format.js';

/**
 * Die Ausgänge, die als Versuch zählen.
 *
 * **Der Abbruch ist bewusst nicht dabei.** Er ist „ausdrücklich keine
 * Niederlage" und trägt „keine Strafe" (GAME-DESIGN 10) — und zugleich der
 * „häufigste Übergang" (GAME-DESIGN 12.8). Schriebe der häufigste Übergang
 * jedes Mal einen Verlaufs-Eintrag, zeigte „Entwicklung über die Zeit" vor
 * allem, wie oft jemand neu angefangen hat: Wer sich fünfmal vertippt und neu
 * startet, hätte fünf Einträge bei 2 % Fortschritt. Der Versuchszähler an der
 * Marke stiege mit, ohne dass ein Versuch stattgefunden hätte.
 *
 * Ein Abbruch verschlechtert die Marke nie — sie sinkt nur. Er verwässert aber
 * beide Zahlen daneben.
 *
 * Die Entscheidung liegt beim Auftraggeber; sie zu ändern ist ein Eintrag in
 * dieser Liste.
 */
export const COUNTED_OUTCOMES: readonly number[] = [
  OUTCOME_VICTORY,
  OUTCOME_OVERPRESSURE,
  OUTCOME_DEATH,
];

/** Nur zur Lesbarkeit an der Aufrufstelle — der Abbruch steht bewusst draußen. */
export const IGNORED_OUTCOMES: readonly number[] = [OUTCOME_WIPE];

export function countsAsAttempt(outcome: number): boolean {
  return COUNTED_OUTCOMES.includes(outcome);
}

/** Was der Aufrufer beisteuert, weil es nicht im Kampf steht. */
export interface RecordContext {
  readonly bossId: string;
  readonly difficulty: number;
  /**
   * Stufe des Bosses (Issue #248).
   *
   * Getrennt von `difficulty`: Die Schwierigkeit ist die gewählte Stufe des
   * Kampfes (normal, heroisch, mythisch), die Boss-Stufe seine Stellung im
   * Aufstieg. Beides zusammenzulegen hieße, dass ein heroischer Anfangsboss
   * so viel Erfahrung gäbe wie ein normaler Endboss.
   */
  readonly bossLevel: number;
  /** Fassung des Spiels. Bedingung, nicht Beiwerk (GAME-DESIGN 18.6). */
  readonly gameVersion: string;
  /** Zeitpunkt in ms seit 1970. */
  readonly at: number;
}

/** Der Stand, der fortgeschrieben wird. */
export interface RecordedState {
  readonly runHistory: readonly RunHistoryEntry[];
  readonly progressMarks: readonly ProgressMarkEntry[];
  /** Erreichte Stufe (Issue #248). */
  readonly level: number;
  /** Erfahrung auf dieser Stufe, noch nicht verbraucht (Issue #248). */
  readonly experience: number;
}

export interface RecordResult extends RecordedState {
  /** Ob dieser Durchlauf überhaupt gezählt hat. */
  readonly counted: boolean;
  /**
   * Erfahrung aus diesem Durchlauf, 0 ohne Sieg (Issue #248).
   *
   * **Nur der Sieg zählt.** Ein Versuch, der scheitert, geht in die
   * Fehleranalyse ein und in den Verlauf — aber er bringt keinen Fortschritt,
   * sonst wäre Verlieren eine Strategie.
   */
  readonly xpGained: number;
  /** Ob dieser Durchlauf die Stufe gehoben hat. */
  readonly leveledUp: boolean;
  /**
   * Was der Versuch an der Marke bewirkt hat, oder `null` bei einem Abbruch.
   *
   * Der Aufrufer zeigt daraus den Satz an den Spieler — die Marke ist die
   * Rückmeldung, für die es sie gibt.
   */
  readonly mark: MarkUpdate | null;
}

/**
 * Einen abgeschlossenen Kampf festhalten.
 *
 * Gibt einen **neuen** Stand zurück und ändert den übergebenen nicht: Der
 * Aufrufer entscheidet, wann er ablegt, und ein halb geschriebener Spielstand
 * kann keine Marke verfälschen.
 */
export function recordRun(
  state: RecordedState,
  summary: RunSummary,
  outcome: number,
  context: RecordContext,
): RecordResult {
  if (!countsAsAttempt(outcome)) {
    return { ...state, counted: false, mark: null, xpGained: 0, leveledUp: false };
  }

  // Die Umrechnung läuft über attemptFrom. Sie hier nachzubauen wäre die eine
  // Zeile, an der die ganze Marke kippt.
  const attempt = attemptFrom(summary, context);
  const mark = recordAttempt(state.progressMarks, attempt);

  const entry: RunHistoryEntry = {
    bossId: context.bossId,
    difficulty: context.difficulty,
    at: context.at,
    gameVersion: context.gameVersion,
    victory: summary.victory,
    elapsedMs: summary.elapsedMs,
    // Dieselbe Zahl wie in der Marke, aus derselben Umrechnung. Zwei Wege zu
    // ihr wären zwei Gelegenheiten, sie zu verdrehen.
    remainingPercent: attempt.remainingPercent,
    damage: summary.damage,
    idleMs: summary.lostTimeNothingReadyMs,
    missedMs: summary.lostTimeWithReadyMs,
  };

  // **Erfahrung nur für den Sieg** (Issue #248). Sie hängt an der Stufe des
  // BOSSES: GAME-DESIGN 12.6 sieht einen abnehmenden Ertrag über den
  // Stufenabstand vor, und einen Abstand gibt es nur, wenn beide Seiten eine
  // Stufe haben. Der Ertrag selbst ist noch nicht gebaut — die Unterschrift
  // nimmt die Spieler-Stufe trotzdem schon auf, damit später nicht jede
  // Aufrufstelle geändert werden muss.
  const xpGained = summary.victory ? xpForVictory(context.bossLevel, state.level) : 0;
  const nachher = grantXp({ level: state.level, xp: state.experience }, xpGained);

  return {
    runHistory: appendRun(state.runHistory, entry),
    progressMarks: mark.marks,
    level: nachher.level,
    experience: nachher.xp,
    counted: true,
    mark,
    xpGained,
    leveledUp: nachher.level > state.level,
  };
}
