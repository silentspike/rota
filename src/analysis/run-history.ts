/**
 * Der Verlauf über alle Durchläufe (GAME-DESIGN 15.4, Issue #82).
 *
 * DER SCHNITT BEIM FASSUNGSWECHSEL
 * --------------------------------
 * Der Verlauf reicht über Balance-Änderungen hinweg. Zeichnete man ihn als
 * eine durchgehende Linie, sähe der Spieler beim Wechsel einen Sprung und
 * schriebe ihn sich selbst zu — dabei hat sich das Spiel geändert, nicht sein
 * Spielen. Deshalb zerfällt der Verlauf in **Abschnitte** je Fassung, und die
 * Ansicht zeichnet sie getrennt.
 *
 * Das ist die Bedingung, unter der der Auftraggeber am 2026-08-04 den
 * Schadenswert im Verlauf freigegeben hat. Ohne den Schnitt wäre die Freigabe
 * nicht erteilt worden, und ohne ihn ist der Verlauf irreführend.
 *
 * WARUM DER VERLAUF EINE OBERGRENZE HAT
 * -------------------------------------
 * „Alle Durchläufe" heißt nicht „unbegrenzt viele". Der Spielstand liegt im
 * Browser-Speicher, und der ist knapp; ein Verlauf, der ihn sprengt, kostet
 * den ganzen Spielstand und nicht nur sich selbst. Bei Überlauf fallen die
 * ÄLTESTEN Einträge — der Fortschritt der letzten Wochen ist das, was der
 * Spieler ansieht.
 *
 * Die Fortschrittsmarke bleibt davon unberührt: Sie steht getrennt
 * (`progressMarks`) und überlebt jedes Abschneiden. Ein Bestwert, der aus dem
 * Verlauf fällt, wäre sonst verloren.
 */
import type { RunHistoryEntry } from './save-format.js';

/**
 * Wie viele Durchläufe der Verlauf hält.
 *
 * 500 Einträge à etwa 120 Byte sind rund 60 KB — neben dem übrigen Spielstand
 * unbedenklich, und weit über den 50 Durchläufen, die die Ansicht zeigen
 * können muss (Issue #82, AC4).
 */
export const HISTORY_LIMIT = 500;

/**
 * Einen Durchlauf anhängen.
 *
 * Gibt eine **neue** Liste zurück, aus demselben Grund wie bei der
 * Fortschrittsmarke: Der Aufrufer entscheidet, wann er ablegt.
 */
export function appendRun(
  history: readonly RunHistoryEntry[],
  entry: RunHistoryEntry,
  limit = HISTORY_LIMIT,
): RunHistoryEntry[] {
  const next = [...history, entry];
  // Die ältesten fallen, nicht die neuesten.
  return next.length > limit ? next.slice(next.length - limit) : next;
}

/** Ein zusammenhängender Abschnitt unter einer Fassung. */
export interface HistorySegment {
  readonly gameVersion: string;
  readonly entries: readonly RunHistoryEntry[];
  /** Der Index im Gesamtverlauf, an dem der Abschnitt beginnt. */
  readonly startIndex: number;
}

/**
 * Den Verlauf in Abschnitte je Fassung zerlegen.
 *
 * Zerlegt wird an jedem WECHSEL, nicht nach Fassung gruppiert: Wer zwischen
 * zwei Fassungen hin und her wechselt — etwa beim Prüfen einer älteren
 * Fassung —, hat drei Abschnitte, nicht zwei. Gruppieren würde die Reihenfolge
 * zerreißen, und die Reihenfolge ist der ganze Punkt eines Verlaufs.
 */
export function historySegments(history: readonly RunHistoryEntry[]): HistorySegment[] {
  const segments: HistorySegment[] = [];
  let current: RunHistoryEntry[] = [];
  let version: string | null = null;
  let startIndex = 0;

  history.forEach((entry, index) => {
    if (version !== null && entry.gameVersion !== version) {
      segments.push({ gameVersion: version, entries: current, startIndex });
      current = [];
      startIndex = index;
    }
    version = entry.gameVersion;
    current.push(entry);
  });

  if (version !== null) segments.push({ gameVersion: version, entries: current, startIndex });
  return segments;
}

/** Nur die Durchläufe auf einem Boss und einer Stufe. */
export function historyFor(
  history: readonly RunHistoryEntry[],
  bossId: string,
  difficulty: number,
): RunHistoryEntry[] {
  return history.filter((entry) => entry.bossId === bossId && entry.difficulty === difficulty);
}
