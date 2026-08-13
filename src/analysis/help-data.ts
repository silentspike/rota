/**
 * Die Hilfe-Einträge aus den Daten (Issue #811).
 *
 * # Warum hier heute nur Prüfeinträge stehen
 *
 * Die Erklärungen sind **Inhalt** und gehören damit zu E10 — dieselbe
 * Entscheidung wie bei den ~150 Archiv-Texten (GAME-DESIGN Zeile 2620, Issue
 * #85). Was hier entsteht, ist die Leitung: von den Daten über das Merkmal
 * `data-help` bis in das Fenster, das der Spieler sieht.
 *
 * # Das Trennmerkmal ist die DATEI, nicht die Kennung
 *
 * Bei `test-cast` (#304), `test-buff` (#420), `test-channel` (#323) und den
 * Archiv-Einträgen (#85) trennt ein Präfix beziehungsweise `unlockLevel: 101`.
 * Hier geht das nicht, und der Grund ist keine Bequemlichkeit:
 *
 * ```text
 *   <td data-help="essence">   ->   helpFor(entries, 'essence')
 * ```
 *
 * **Die Kennung IST die Kupplung.** Hiesse der Eintrag `test-essence`, fände
 * `helpFor` ihn nicht, kein Fenster erschiene, und die Prüfdaten belegten
 * nichts — sie wären genau das Gegenteil eines Nachweises.
 *
 * Getrennt wird deshalb über die Herkunft: Diese Datei ist die Prüfdatei.
 * Sobald E10 `data/help/help.json` liefert, kommen die ausgelieferten Einträge
 * von dort, und [`PLAYABLE_HELP_ENTRIES`] füllt sich, ohne dass hier eine Zeile
 * geändert werden muss.
 */
import rawHelp from '../../data/help/test-help.json';
import type { HelpEntry, HelpSubject, Introduction } from './help.js';

const SUBJECTS: readonly HelpSubject[] = ['spell', 'stat', 'school', 'bossAbility', 'mechanic'];

/**
 * Alle Einträge, auch die Prüfeinträge.
 *
 * Für Prüffälle und für die Ansicht, solange es keinen Katalog gibt. Das
 * ausgelieferte Spiel nimmt [`PLAYABLE_HELP_ENTRIES`], sobald diese Liste
 * etwas enthält.
 */
export const ALL_HELP_ENTRIES: readonly HelpEntry[] = parseHelpEntries(
  (rawHelp as { entries?: unknown }).entries,
);

/**
 * Die Einträge aus dem Katalog.
 *
 * Heute ist die Liste **leer**, und das ist kein Fehler: `data/help/help.json`
 * gibt es noch nicht. Der Fall ist derselbe wie bei
 * `PLAYABLE_ARCHIVE_ENTRIES`.
 */
export const PLAYABLE_HELP_ENTRIES: readonly HelpEntry[] = [];

/** Die Einführungen, ebenfalls als Prüfdaten. */
export const ALL_INTRODUCTIONS: readonly Introduction[] = parseIntroductions(
  (rawHelp as { introductions?: unknown }).introductions,
);

/**
 * Prüft und übersetzt die rohen Einträge.
 *
 * **Als eigene Funktion, damit die Prüfungen prüfbar sind** — und diesmal mit
 * Fällen, die sie auch aufrufen. Bei `parseEntries` (#85) stand derselbe Satz
 * im Kommentar, und die Prüfungen darin waren bis #809 selbst ungeprüft: Der
 * Kommentar beschrieb eine Absicht, nicht einen Zustand.
 */
export function parseHelpEntries(roh: unknown): HelpEntry[] {
  if (!Array.isArray(roh)) {
    throw new Error('data/help/test-help.json: "entries" fehlt oder ist keine Liste');
  }
  const vergeben = new Set<string>();
  return roh.map((eintrag) => {
    const e = eintrag as Record<string, unknown>;
    const id = typeof e['id'] === 'string' ? e['id'] : '';
    if (id === '') {
      throw new Error('data/help: ein Eintrag ohne Kennung — er wäre nie auffindbar');
    }
    // **Zwei Einträge mit derselben Kennung wären still**: `helpFor` nimmt den
    // ERSTEN Treffer. Der zweite erschiene nie, und welcher von beiden gewinnt,
    // hinge an der Reihenfolge in der Datei. Dieselbe Klasse wie die
    // Satz-Kennungen in #780 und die Archiv-Kennungen in #809.
    if (vergeben.has(id)) {
      throw new Error(`data/help: die Kennung "${id}" kommt zweimal vor`);
    }
    vergeben.add(id);

    const subject = e['subject'];
    if (!SUBJECTS.includes(subject as HelpSubject)) {
      throw new Error(`data/help: "${id}" hat den unbekannten Bezug ${String(subject)}`);
    }
    const title = typeof e['title'] === 'string' ? e['title'] : '';
    const effect = typeof e['effect'] === 'string' ? e['effect'] : '';
    if (title === '' || effect === '') {
      throw new Error(`data/help: "${id}" hat keine Überschrift oder keine Wirkung`);
    }

    return {
      id,
      subject: subject as HelpSubject,
      title,
      effect,
      ...(typeof e['values'] === 'string' ? { values: e['values'] } : {}),
      ...(typeof e['synergies'] === 'string' ? { synergies: e['synergies'] } : {}),
    };
  });
}

/** Prüft und übersetzt die rohen Einführungen. */
export function parseIntroductions(roh: unknown): Introduction[] {
  if (!Array.isArray(roh)) {
    throw new Error('data/help/test-help.json: "introductions" fehlt oder ist keine Liste');
  }
  const vergeben = new Set<string>();
  return roh.map((eintrag) => {
    const e = eintrag as Record<string, unknown>;
    const mechanicId = typeof e['mechanicId'] === 'string' ? e['mechanicId'] : '';
    if (mechanicId === '') {
      throw new Error('data/help: eine Einführung ohne Mechanik — sie käme nie an die Reihe');
    }
    // **Zwei Einführungen zu derselben Mechanik wären still**: `markSeen`
    // vermerkt die Mechanik, nicht die Einführung. Nach der ersten gälte auch
    // die zweite als gesehen — dieselbe Klasse wie bei den Archiv-Kennungen.
    if (vergeben.has(mechanicId)) {
      throw new Error(`data/help: die Mechanik "${mechanicId}" hat zwei Einführungen`);
    }
    vergeben.add(mechanicId);

    const title = typeof e['title'] === 'string' ? e['title'] : '';
    const body = typeof e['body'] === 'string' ? e['body'] : '';
    if (title === '' || body === '') {
      throw new Error(`data/help: "${mechanicId}" hat keine Überschrift oder keinen Text`);
    }
    return { mechanicId, title, body };
  });
}
