/**
 * Warum die Punktzahl so ausfiel — für die Anzeige (Issue #387).
 *
 * # Warum es diese Datei gibt
 *
 * Ein Spieler sieht `Reserve 57.56 s` und `Score 428`. Er war besser als je
 * zuvor und bekommt ein Fünfzehntel dessen, was sein erster Sieg brachte.
 * Beim ersten Mal wirkt das wie ein Fehler im Spiel — nicht wie eine Regel.
 *
 * Die Regel steht in GAME-DESIGN 12.6 und ist seit #377 vollständig gebaut:
 * Differenz-Auszahlung bei einer Verbesserung, Rest-Ertrag fürs blosse
 * Wiederholen. Was fehlte, war ihre Sichtbarkeit. `recordBest` liefert
 * `previous` und `improved` längst mit; es ist kein neuer Wert zu ermitteln,
 * nur einer anzuzeigen, der schon da ist.
 *
 * # Struktur, kein fertiger Satz
 *
 * Dieselbe Linie wie `levelUpDisplayFor` (#371): Wer hier einen vorformatierten
 * Text zurückgäbe, nähme der Anzeige die Entscheidung ab — und mit ihr die
 * Möglichkeit, sie richtig zu treffen. Ob „87 · repeat" oder „87 (best 55.67 s)"
 * dasteht, ist eine Frage an die Zeile und ihren Platz, nicht an die Rechnung.
 *
 * # Die Fälle heissen nach der AUSZAHLUNG, nicht nach der Historie
 *
 * Naheliegend wäre `first | improved | repeat` gewesen. Das wäre falsch:
 * Seit #377 gibt es **zwei** Wege zu „kein Vergleichswert" — es gab noch keinen
 * Sieg, ODER der gespeicherte Bestwert wurde verworfen, weil sein Mass nicht
 * mehr passt (der Boss hat `runaway` bekommen oder verloren). Im zweiten Fall
 * wäre „first clear" eine falsche Aussage über die Vergangenheit, obwohl die
 * volle Auszahlung richtig ist.
 *
 * `full` sagt, was passiert ist, und stimmt auf beiden Wegen.
 */
import type { RunSummary } from '../core/summary.js';
import type { RecordResult } from './run-recorder.js';

/** Wie die Punkte zustande kamen. */
export const PAYOUT_KINDS = ['full', 'difference', 'repeat'] as const;
export type PayoutKind = (typeof PAYOUT_KINDS)[number];

/** Was die Anzeige braucht, um die Zahl zu erklären. */
export interface PointsDisplay {
  /** Die ausgezahlten Punkte. */
  readonly awarded: number;
  /** Voller Ertrag, Differenz oder nur der Rest-Ertrag. */
  readonly kind: PayoutKind;
  /**
   * Der Bestwert VOR diesem Lauf als **Rohwert**, oder `null` bei `full`.
   *
   * Millisekunden oder Prozent — welches von beidem, sagt `isRing`. Roh und
   * nicht formatiert, weil das Format zur Zeile gehört: Dieselbe Zahl steht
   * bei einer Wiederholung für „das hast du nicht geschlagen" und bei einer
   * Verbesserung für „davon wurde abgezogen".
   */
  readonly previous: number | null;
  /**
   * Ob `previous` ein Ring-Rest in Prozent ist statt einer Reserve in ms.
   *
   * **Ohne diese Angabe ist `previous` nicht lesbar**: 55670 ist entweder eine
   * Reserve von 55,67 s oder ein Ring-Rest von 55670 %. Dieselbe Lücke, die im
   * Spielstand `metric` schliesst (#377) — hier eine Ebene höher.
   *
   * Sie folgt aus dem LAUF (`heldPercent !== null`), wie in `recordRun` und in
   * der Punkteformel. Die Verzweigung aus #256 wird damit ein drittes Mal
   * geerbt und kein viertes Mal getroffen.
   */
  readonly isRing: boolean;
}

/**
 * Wie die Punkte dieses Laufs zustande kamen.
 *
 * `null`, wenn es nichts zu zeigen gibt — bei einem Abbruch, der nicht zählt,
 * oder bei einem Lauf ohne Sieg. Die Entscheidung fällt hier und nicht in der
 * Anzeige: Ob es etwas zu erklären gibt, ist eine Frage an die Daten.
 */
export function pointsDisplayFor(
  record: RecordResult | null,
  summary: RunSummary | null,
): PointsDisplay | null {
  if (record === null || summary === null || !record.counted) return null;
  // Eine Niederlage zahlt nichts, und „0 · repeat" wäre eine Erklärung für
  // etwas, das gar nicht stattgefunden hat.
  if (!summary.victory) return null;

  const previous = record.best?.previous ?? null;
  const kind: PayoutKind =
    previous === null ? 'full' : record.best?.improved === true ? 'difference' : 'repeat';

  return {
    awarded: record.pointsGained,
    kind,
    // Bei `full` gibt es nichts zu vergleichen — eine Zahl dort wäre eine
    // Marke, die es nicht gibt.
    previous: kind === 'full' ? null : previous,
    isRing: summary.heldPercent !== null,
  };
}
