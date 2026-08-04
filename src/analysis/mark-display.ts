/**
 * Die Fortschrittsmarke, fertig zum Anzeigen (Issue #187).
 *
 * WARUM ES DIESE ZWISCHENSTUFE GIBT
 * ---------------------------------
 * Die Marke wird gespeichert, seit Issue #179 — und der Spieler bekam sie nie
 * zu sehen. Sie existiert für genau einen Zweck (GAME-DESIGN 10.3): dem
 * Spieler nach einer **Niederlage** zu sagen, dass er weiterkommt. Ohne
 * Anzeige ist sie eine Zahl im Speicher.
 *
 * Der Satz entsteht hier und nicht in der Ansicht. Zwei Stellen, die aus
 * denselben Zahlen einen Satz machen, laufen beim nächsten Nachziehen
 * auseinander — dieselbe Regel wie bei der laufenden Anzeige im Trainingsraum.
 *
 * WANN NICHTS GEZEIGT WIRD, UND WARUM DAS EIN ERGEBNIS IST
 * -------------------------------------------------------
 * `null` heißt: **nichts anzeigen**, nicht „0 %" und nicht „—".
 *
 *   Erster Kampf eines Spielstands   es gibt keine Marke zu zeigen
 *   Sieg                             dort steht das Ergebnis, nicht die Marke
 *   Abbruch                          zählt nicht als Versuch (GAME-DESIGN 12.8)
 *
 * Eine Anzeige, die im ersten Fall eine Null zeigt, behauptet einen
 * Fortschritt, den es nicht gibt — das ist schlechter als gar keine Anzeige.
 */
import type { MarkDisplay } from '../render/overlay.js';
import type { RunSummary } from '../core/summary.js';
import { attemptFrom, markFor, markSentence } from './progress-mark.js';
import type { RecordResult } from './run-recorder.js';

/**
 * Aus dem Ergebnis eines Kampfes den Satz für den Spieler machen.
 *
 * Die Umrechnung läuft über `attemptFrom` — hier wie überall. `progressPercent`
 * und `remainingPercent` sind beides Zahlen zwischen 0 und 100 mit „Percent"
 * im Namen und entgegengesetzter Richtung, und die eine Zeile, an der man sie
 * verwechselt, dreht die ganze Marke um.
 */
export function markDisplayFor(
  record: RecordResult,
  summary: RunSummary,
  bossId: string,
  difficulty = 1,
): MarkDisplay | null {
  // Nicht gezählt (Abbruch) oder kein Ergebnis: nichts zu zeigen.
  if (!record.counted || record.mark === null) return null;
  // Nach einem Sieg steht das Ergebnis da, nicht die Marke.
  if (summary.victory) return null;

  const eintrag = markFor(record.progressMarks, bossId, difficulty);
  if (eintrag === null) return null;

  const attempt = attemptFrom(summary, {
    bossId,
    difficulty,
    gameVersion: eintrag.gameVersion,
    at: eintrag.at,
  });

  return {
    sentence: markSentence(record.mark, attempt),
    attempts: eintrag.attempts,
  };
}
