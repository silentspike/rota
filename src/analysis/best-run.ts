/**
 * Der Bestwert je Boss (Issue #377, GAME-DESIGN 12.6).
 *
 * WAS BIS HIERHER FEHLTE
 * ----------------------
 * `bestRuns` stand seit #83 im Spielstand-Format und hatte **kein einziges
 * Schreiben** — drei Vorkommen im Quelltext: Initialisierung, Typ,
 * Schlüsselliste. Die vorhandene `ProgressMark` konnte den Wert nicht
 * ersetzen, weil sie `100 - progressPercent` misst und der bei jedem Sieg 0
 * ist: Sie ist eine Marke fürs Scheitern, keine für die Qualität eines Siegs.
 *
 * WARUM DAS MASS IM EINTRAG STEHT
 * -------------------------------
 * Bei einem `runaway`-Boss ist die Frist immer voll ausgeschöpft — die Reserve
 * trennt dort keine zwei Siege, der Ring-Rest tut es (Issue #256, #276). Das
 * ist dieselbe Verzweigung wie in der Punkteformel, aber diesmal **im
 * Spielstand**, und damit wird sie zur Fassungsfrage.
 *
 * Der teure Fall ist nicht der Eintrag ohne Angabe — der fällt sofort auf. Es
 * ist der Eintrag MIT einer Angabe, die nicht mehr stimmt: Ein Boss bekommt
 * `runaway` oder verliert es, derselbe `bossId` trägt plötzlich eine andere
 * Größe, und beide sind plausible Zahlen. Deshalb prüft `usableBest` nicht
 * nur, ob ein Mass dasteht, sondern ob es das HEUTIGE ist.
 *
 * WARUM DIE PRÜFUNG HIER SITZT UND NICHT IN DER PERSISTENZ
 * --------------------------------------------------------
 * `run-persistence.ts` kennt die Bosse nicht — sie importiert `save-store`,
 * `save-format` und `run-recorder`, sonst nichts. Die heutige Bedeutung liegt
 * dagegen im Lauf selbst vor (`summary.heldPercent !== null`). Die Prüfung
 * gehört dorthin, wo beide Seiten ohne neuen Import zusammenkommen.
 */
import { BEST_METRICS, type BestMetric, type BestRun } from './save-format.js';

/** Was ein Lauf zum Bestwert beiträgt. */
export interface BestAttempt {
  readonly bossId: string;
  readonly difficulty: number;
  readonly metric: BestMetric;
  readonly value: number;
  readonly gameVersion: string;
  readonly at: number;
}

/** Was die Fortschreibung ergeben hat. */
export interface BestUpdate {
  /** Die neue Liste — der übergebene Stand bleibt unberührt. */
  readonly runs: readonly BestRun[];
  /**
   * Der Wert VOR diesem Lauf, oder `null`.
   *
   * **`null` heißt „nichts zu vergleichen"**, und das umfasst drei Fälle: Es
   * gab keinen Eintrag; es gab einen, dessen Mass nicht mehr passt; es gab
   * einen mit unbrauchbarer Form. Der Aufrufer soll sie gleich behandeln —
   * in allen dreien wäre ein Vergleich eine Erfindung.
   */
  readonly previous: number | null;
  /** Ob dieser Lauf den Bestwert verbessert hat. */
  readonly improved: boolean;
  /**
   * Warum ein vorhandener Eintrag verworfen wurde, sonst `null`.
   *
   * Nicht für den Spieler, sondern für den Prüffall und die Fehlersuche: Ein
   * stillschweigend verworfener Bestwert sieht aus wie „noch nie gespielt".
   */
  readonly discarded: string | null;
}

/** Der Schlüssel aus Boss **und** Stufe — dieselbe Zählung wie bei der Marke. */
function key(bossId: string, difficulty: number): string {
  return `${bossId}::${String(difficulty)}`;
}

/**
 * Ob ein gespeicherter Eintrag überhaupt eine Form hat, mit der sich rechnen
 * lässt.
 *
 * **Die erste Eintrags-Prüfung des Projekts.** Der Lesepfad prüft heute die
 * Prüfsumme und die Schema-Nummer (`save-store.ts`), sonst nichts — kein
 * Eintrag irgendeiner Liste wird auf Form geprüft. Das ist tragbar, solange
 * eine Liste nur beschrieben und angezeigt wird; sobald aus ihr **gerechnet**
 * wird, ist ein formloser Eintrag eine falsche Zahl statt einer leeren Zeile.
 */
export function isWellFormed(entry: unknown): entry is BestRun {
  if (entry === null || typeof entry !== 'object') return false;
  const roh = entry as Record<string, unknown>;
  if (typeof roh['bossId'] !== 'string' || roh['bossId'].length === 0) return false;
  if (typeof roh['difficulty'] !== 'number' || !Number.isFinite(roh['difficulty'])) return false;
  if (!BEST_METRICS.includes(roh['metric'] as BestMetric)) return false;
  // **Endlich und nicht negativ.** `NaN` überlebt jeden Größer-Vergleich als
  // `false` — ein Bestwert von `NaN` wäre für immer unschlagbar und würde nie
  // ersetzt.
  if (typeof roh['value'] !== 'number' || !Number.isFinite(roh['value']) || roh['value'] < 0) {
    return false;
  }
  if (typeof roh['gameVersion'] !== 'string') return false;
  if (typeof roh['achievedAt'] !== 'number' || !Number.isFinite(roh['achievedAt'])) return false;
  return true;
}

/**
 * Der brauchbare Bestwert für diesen Boss unter diesem Mass.
 *
 * Gibt `null` zurück, wenn es keinen gibt, wenn seine Form nicht trägt, oder
 * wenn er ein anderes Mass führt als das heute gültige.
 */
export function usableBest(
  runs: readonly BestRun[],
  bossId: string,
  difficulty: number,
  metric: BestMetric,
): { readonly entry: BestRun | null; readonly discarded: string | null } {
  const gesucht = key(bossId, difficulty);
  const treffer = runs.find((run) => key(run.bossId, run.difficulty) === gesucht);
  if (treffer === undefined) return { entry: null, discarded: null };
  if (!isWellFormed(treffer)) {
    return { entry: null, discarded: 'Form unbrauchbar' };
  }
  if (treffer.metric !== metric) {
    // **Der teure Fall.** Der Boss hat `runaway` bekommen oder verloren; der
    // Eintrag trägt eine plausible Zahl der falschen Sorte. Ihn zu vergleichen
    // hiesse, Sekunden gegen Prozent zu halten.
    return { entry: null, discarded: `Mass ${treffer.metric} statt ${metric}` };
  }
  return { entry: treffer, discarded: null };
}

/**
 * Einen Versuch verbuchen.
 *
 * Gibt eine **neue** Liste zurück und liefert `previous` mit — ohne das würde
 * ein Aufrufer, der die Differenz braucht, gegen den schon aktualisierten Wert
 * rechnen und immer null bekommen. Dieselbe Vorsorge wie in `recordAttempt`
 * (Issue #187), und aus demselben Grund.
 */
export function recordBest(runs: readonly BestRun[], attempt: BestAttempt): BestUpdate {
  const { entry, discarded } = usableBest(runs, attempt.bossId, attempt.difficulty, attempt.metric);
  const previous = entry === null ? null : entry.value;
  // Grösser ist besser — bei beiden Massen, siehe `BEST_METRICS`.
  const improved = previous !== null && attempt.value > previous;

  const neu: BestRun = {
    bossId: attempt.bossId,
    difficulty: attempt.difficulty,
    metric: attempt.metric,
    value: previous === null ? attempt.value : Math.max(previous, attempt.value),
    // Fassung und Zeitpunkt gehören zum BESTWERT, nicht zum letzten Versuch:
    // Sie sagen, wann und worunter er entstand.
    gameVersion: improved || previous === null ? attempt.gameVersion : (entry?.gameVersion ?? ''),
    achievedAt: improved || previous === null ? attempt.at : (entry?.achievedAt ?? 0),
  };

  const gesucht = key(attempt.bossId, attempt.difficulty);
  const index = runs.findIndex((run) => key(run.bossId, run.difficulty) === gesucht);
  const naechste = index >= 0 ? [...runs] : [...runs, neu];
  if (index >= 0) naechste[index] = neu;

  return { runs: naechste, previous, improved, discarded };
}
