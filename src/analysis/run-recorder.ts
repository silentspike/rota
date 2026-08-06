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
import { pointsForRun } from '../progression/points.js';
import {
  OUTCOME_DEATH,
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
} from '../core/phases.js';
import type { RunSummary } from '../core/summary.js';
import { attemptFrom, recordAttempt, type MarkUpdate } from './progress-mark.js';
import { recordBest, type BestUpdate } from './best-run.js';
import { recordDefeat } from './defeated.js';
import { appendRun } from './run-history.js';
import type { BestRun, OwnedItem, ProgressMarkEntry, RunHistoryEntry } from './save-format.js';

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
  /**
   * Der beste Lauf je Boss (Issue #377).
   *
   * Stand bis dahin im Spielstand-Format und wurde **nie geschrieben**. Er
   * gehört hierher und nicht nur in `SaveData`, weil er hier ENTSTEHT: Die
   * Differenz-Auszahlung aus 12.6 rechnet gegen ihn, und dafür muss er den
   * Weg von der Aufzeichnung bis zum Speicher haben.
   */
  readonly bestRuns: readonly BestRun[];
  /**
   * Die verteilten Talente (Issue #417).
   *
   * **Die Wahl des Spielers, nicht ihre Wirkung** (GAME-DESIGN 18.6): Was der
   * Faktor daraus macht, wird bei jedem Kampf neu gerechnet.
   */
  readonly talentPoints: Readonly<Record<string, number>>;
  /**
   * Die besessenen Gegenstände (Issue #421a).
   *
   * **Was gefunden und angelegt wurde, nicht was es bewirkt** (18.6): Die
   * getragenen Werte rechnet `wornFrom` bei jedem Kampf neu.
   */
  readonly items: readonly OwnedItem[];
  /**
   * Welche Bosse besiegt wurden, je Boss und Stufe (Issue #434).
   *
   * **Was ERREICHT wurde** (GAME-DESIGN 18.6) — und nicht aus `bestRuns`
   * ableitbar: „beste Zeit" ist eine andere Aussage als „besiegt".
   */
  readonly defeatedBosses: readonly string[];
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
  /**
   * Punkte aus diesem Durchlauf, 0 ohne Sieg (Issue #276).
   *
   * **Eine Auszahlung, kein Konto.** Der Wert reist von hier zur Anzeige und
   * wird nirgends abgelegt — GAME-DESIGN 18.5 verlangt, dass der Spielstand
   * keine berechneten Werte hält, die auf kommende Kämpfe zurückwirken, und
   * `save-format.ts` setzt das mit einem Wächter durch. Ein Punkte-KONTO,
   * wie es 12.7 für die Freischaltungs-Schwelle braucht, ist deshalb eine
   * eigene Frage und ausdrücklich nicht hier beantwortet.
   */
  readonly pointsGained: number;
  /**
   * Was an diesem Lauf am Bestwert geschah, oder `null` bei einem Abbruch
   * (Issue #377).
   *
   * Trägt `previous` MIT, und das ist die eigentliche Vorsorge: Wer die
   * Differenz gegen den schon fortgeschriebenen Wert rechnete, bekäme immer
   * null. Dieselbe Falle wie bei der Marke (#187).
   */
  readonly best: BestUpdate | null;
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
    return {
      ...state,
      counted: false,
      mark: null,
      best: null,
      xpGained: 0,
      pointsGained: 0,
      leveledUp: false,
    };
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
  // BOSSES; der abnehmende Ertrag über den Stufenabstand steckt seit Issue
  // #276 in `xpForVictory` und stammt aus derselben Kurve wie bei den Punkten.
  const xpGained = summary.victory ? xpForVictory(context.bossLevel, state.level) : 0;
  const nachher = grantXp({ level: state.level, xp: state.experience }, xpGained);

  // **Punkte hier und nicht in der Zusammenfassung** (Issue #276): Sie brauchen
  // beide Stufen, und `RunSummary` kennt keine — sie beschreibt einen Kampf,
  // nicht einen Spieler. Dies ist die einzige Stelle, an der Lauf, Boss-Stufe
  // und Spieler-Stufe zusammenkommen.
  //
  // **Die Stufe von VOR dem Aufstieg** (`state.level`, nicht `nachher.level`),
  // dieselbe wie bei der Erfahrung: Wer mit dem Sieg aufsteigt, hat den Kampf
  // noch auf der alten Stufe bestritten, und der Abstand gehört zum Kampf.
  // **Der Bestwert VOR den Punkten** (Issue #377): Erst nachsehen, was da war,
  // dann fortschreiben, dann gegen den alten Wert rechnen. Die andere
  // Reihenfolge zahlte jedem Sieg nur den Rest-Ertrag, weil die Differenz
  // gegen sich selbst immer null ist.
  //
  // **Nur ein Sieg setzt einen Bestwert.** Ein gescheiterter Versuch hat keine
  // Reserve und keinen gehaltenen Ring — er trüge eine 0 ein und würde damit
  // eine Marke behaupten, die es nicht gibt.
  const best = summary.victory
    ? recordBest(state.bestRuns, {
        bossId: context.bossId,
        difficulty: context.difficulty,
        // Die Verzweigung von #256, geerbt und nicht neu getroffen: Bei einem
        // runaway-Boss ist die Reserve konstant null und trennt nichts.
        metric: summary.heldPercent === null ? 'reserve' : 'held',
        value: summary.heldPercent === null ? summary.remainingMs : summary.heldPercent,
        gameVersion: context.gameVersion,
        at: context.at,
      })
    : null;

  const pointsGained = pointsForRun(
    summary,
    { bossLevel: context.bossLevel, playerLevel: state.level },
    best?.previous ?? null,
  );

  return {
    runHistory: appendRun(state.runHistory, entry),
    progressMarks: mark.marks,
    bestRuns: best?.runs ?? state.bestRuns,
    // **Unverändert weitergereicht**: Ein Kampf verteilt keine Talente — das
    // tut der Spieler zwischen den Kämpfen (GAME-DESIGN 12.4).
    talentPoints: state.talentPoints,
    // Ein Kampf findet noch keine Beute — das ist #421b.
    items: state.items,
    // **Der Hauptpfad aus 12.7** (Issue #434). `recordDefeat` entscheidet
    // selbst, ob der Lauf zählt — die Bedingung gehört zum Lauf, nicht zur
    // Aufrufstelle.
    defeatedBosses: recordDefeat(
      state.defeatedBosses,
      context.bossId,
      context.difficulty,
      summary.victory,
    ),
    level: nachher.level,
    experience: nachher.xp,
    counted: true,
    mark,
    best,
    xpGained,
    pointsGained,
    leveledUp: nachher.level > state.level,
  };
}
