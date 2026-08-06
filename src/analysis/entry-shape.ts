/**
 * Die Form der Listeneinträge im Spielstand (Issue #378, GAME-DESIGN 18.1).
 *
 * WARUM DER SPIELSTAND EINE SONDERROLLE HAT
 * -----------------------------------------
 * Er ist die **einzige** Datenquelle, die das Spiel nicht selbst erzeugt hat.
 * Alles andere kommt aus `data/` — im Repository, versioniert, vom Bau geprüft
 * — oder entsteht im Lauf. Ein Spielstand kommt aus dem Browser eines
 * Spielers, ist über Fassungen gewandert und kann von Hand bearbeitet oder
 * halb geschrieben worden sein.
 *
 * Bis zu diesem Issue prüfte der Lesepfad die Prüfsumme und die Schema-Nummer,
 * sonst nichts. Ein Eintrag mit fehlendem oder falsch getipptem Feld wurde
 * gelesen, als wäre er heil, und die Auswertung rechnete damit weiter —
 * dieselbe Klasse wie überall: **nicht kaputt, sondern still fehlgedeutet.**
 *
 * GENAU PRÜFEN, NICHT STRENG
 * --------------------------
 * Geprüft wird auf die **heute vorhandenen Pflichtfelder**; zusätzliche stören
 * nicht. Der Unterschied ist teuer: Eine Prüfung, die ein elftes Feld
 * verlangte, würde bei der nächsten Fassung jeden alten Eintrag verwerfen —
 * und dem Spieler beim Update seinen Verlauf nehmen. Der Wächter soll den
 * beschädigten Stand fangen, nicht den alten.
 *
 * WO GEFILTERT WIRD UND WO NICHT
 * ------------------------------
 *   Liste ist ein Array, ein Eintrag darin unbrauchbar
 *     -> Eintrag verwerfen, Stand behalten
 *   Liste ist gar kein Array
 *     -> Stand unbrauchbar, die Rotation nimmt den nächsten Platz
 *
 * Ein Stand, dessen Einträge **alle** verworfen werden, gilt weiter als heil:
 * Eine leere Liste ist ein gültiger Zustand — jeder neue Spieler hat sie. Die
 * Regel „alle weg heisst kaputt" würde bei einem Fassungswechsel jeden Stand
 * verwerfen und damit genau den Schaden anrichten, vor dem die Rotation
 * schützen soll.
 */
import { isWellFormed as isBestRunWellFormed } from './best-run.js';
import type { ProgressMarkEntry, RunHistoryEntry } from './save-format.js';

/** Ein endlicher Zahlenwert — `NaN` und `Infinity` sind keine Messwerte. */
function istZahl(wert: unknown): boolean {
  return typeof wert === 'number' && Number.isFinite(wert);
}

/** Eine nicht leere Kennung. */
function istKennung(wert: unknown): boolean {
  return typeof wert === 'string' && wert.length > 0;
}

/**
 * Ob ein Verlaufs-Eintrag eine Form hat, mit der sich rechnen lässt.
 *
 * `gameVersion` darf leer sein, aber nicht fehlen: Ein Eintrag ohne
 * Fassungsvermerk ist nicht vergleichbar (GAME-DESIGN 18.6), und das ist eine
 * Aussage — anders als ein Eintrag, bei dem an der Stelle eine Zahl steht.
 */
export function isRunHistoryEntry(entry: unknown): entry is RunHistoryEntry {
  if (entry === null || typeof entry !== 'object') return false;
  const roh = entry as Record<string, unknown>;
  if (!istKennung(roh['bossId'])) return false;
  if (!istZahl(roh['difficulty'])) return false;
  if (!istZahl(roh['at'])) return false;
  if (typeof roh['gameVersion'] !== 'string') return false;
  if (typeof roh['victory'] !== 'boolean') return false;
  for (const feld of ['elapsedMs', 'remainingPercent', 'damage', 'idleMs', 'missedMs']) {
    if (!istZahl(roh[feld])) return false;
  }
  return true;
}

/** Ob eine Fortschrittsmarke eine Form hat, mit der sich rechnen lässt. */
export function isProgressMarkEntry(entry: unknown): entry is ProgressMarkEntry {
  if (entry === null || typeof entry !== 'object') return false;
  const roh = entry as Record<string, unknown>;
  if (!istKennung(roh['bossId'])) return false;
  if (!istZahl(roh['difficulty'])) return false;
  if (!istZahl(roh['bestRemainingPercent'])) return false;
  if (!istZahl(roh['attempts'])) return false;
  if (typeof roh['defeated'] !== 'boolean') return false;
  if (typeof roh['gameVersion'] !== 'string') return false;
  if (!istZahl(roh['at'])) return false;
  return true;
}

/** Ein Eintrag im Besitz — nur die Kennung und die Zahlen daneben. */
export function isOwnedItem(entry: unknown): boolean {
  if (entry === null || typeof entry !== 'object') return false;
  const roh = entry as Record<string, unknown>;
  if (!istKennung(roh['itemId'])) return false;
  if (!istZahl(roh['upgradeSteps'])) return false;
  // `equippedSlot` ist ausdrücklich `number | null` — nicht ausgerüstet ist
  // ein Zustand, kein fehlender Wert.
  if (roh['equippedSlot'] !== null && !istZahl(roh['equippedSlot'])) return false;
  return true;
}

/**
 * Wie jede Liste im Spielstand geprüft wird.
 *
 * **Als Tabelle und nicht als Kette von `if`.** Wer eine Liste hinzufügt und
 * sie hier vergisst, wird von `save-shape.test.ts` genannt — dieselbe Technik
 * wie beim Wächter über `ergaenzt()`: eine Feldliste, die niemand pflegen muss,
 * weil sie aus dem Gegenstand selbst kommt.
 */
export const LIST_GUARDS: Readonly<Record<string, (entry: unknown) => boolean>> = {
  runHistory: isRunHistoryEntry,
  progressMarks: isProgressMarkEntry,
  bestRuns: isBestRunWellFormed,
  items: isOwnedItem,
  // Die drei einfachen: eine Liste von Kennungen. Ein leerer String wäre ein
  // Zauber ohne Namen — im besten Fall unsichtbar, im schlechteren ein
  // Nachschlagen ins Leere.
  defeatedBosses: istKennung,
};

/**
 * Was beim Aufräumen eines Standes geschah.
 *
 * `dropped` ist nicht für den Spieler, sondern für den Prüffall und die
 * Fehlersuche: Ein stillschweigend verworfener Eintrag sieht aus wie „nie
 * gespielt".
 */
export interface CleanResult {
  /** Der Stand mit den brauchbaren Einträgen, oder `null` — siehe unten. */
  readonly data: Record<string, unknown> | null;
  /** Wie viele Einträge je Liste verworfen wurden. Leer heißt: keiner. */
  readonly dropped: Readonly<Record<string, number>>;
  /** Warum der ganze Stand unbrauchbar ist, sonst `null`. */
  readonly rejected: string | null;
}

/**
 * Einen geladenen Stand aufräumen.
 *
 * Gibt `data: null` zurück, wenn eine Liste **gar keine** Liste ist — dann ist
 * nicht ein Eintrag beschädigt, sondern die Struktur, und die Rotation soll
 * den nächsten Platz nehmen (GAME-DESIGN 18.1: „Bei beschädigtem Spielstand
 * wird automatisch der letzte heile Stand angeboten").
 */
export function cleanSave(data: unknown): CleanResult {
  if (data === null || typeof data !== 'object') {
    return { data: null, dropped: {}, rejected: 'Stand ist kein Objekt' };
  }
  const roh = { ...(data as Record<string, unknown>) };
  const dropped: Record<string, number> = {};

  for (const [name, guard] of Object.entries(LIST_GUARDS)) {
    const wert = roh[name];
    // **Fehlt ist erlaubt, falsch getippt nicht.** Ein Stand aus einer älteren
    // Fassung kennt die Liste womöglich nicht; `ergaenzt()` setzt sie danach
    // auf leer. Eine Zeichenkette an ihrer Stelle ist dagegen kein Zustand,
    // den dieses Spiel je geschrieben hat.
    if (wert === undefined || wert === null) continue;
    if (!Array.isArray(wert)) {
      return { data: null, dropped, rejected: `${name} ist keine Liste` };
    }
    const heil = wert.filter((eintrag) => guard(eintrag));
    if (heil.length !== wert.length) dropped[name] = wert.length - heil.length;
    roh[name] = heil;
  }

  return { data: roh, dropped, rejected: null };
}
