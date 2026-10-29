/**
 * Die Hilfe-Einträge aus den Daten (Issue #811, Katalog seit #1177).
 *
 * # Zwei Dateien, zwei Wege
 *
 * ```text
 *   data/help/help.json        der Katalog — geht an den Spieler
 *   data/help/test-help.json   Prüfeinträge — nur im Entwicklungs-Bau
 * ```
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
 * nichts. Deshalb trennt die Herkunft: Die Prüfdatei wird nur im
 * Entwicklungs-Bau gelesen, hinter `__ROTA_DEVTOOLS__` wie die
 * Entwicklungs-Werkzeuge (GAME-DESIGN 19.3). Ob sie wirklich aus dem
 * Auslieferungs-Bau fällt, prüft `tools/auslieferung-pruefdaten.py` am
 * gebauten Stand.
 *
 * # Die Werte eines Zaubers kommen aus seinen Daten
 *
 * Der Katalog trägt die Wirkung als Satz. Die Zahlen setzt `spellValues` aus
 * `data/spells/` dazu — eine zweite Quelle wäre nach der nächsten
 * Balance-Messung falsch.
 */
import rawCatalog from '../../data/help/help.json';
import rawHelp from '../../data/help/test-help.json';
import { CALCINATION, TALENTS } from '../core/data.js';
import type { HelpEntry, HelpSubject, Introduction } from './help.js';
import { resourceNameOf, spellValues, talentValues } from './help-values.js';

const SUBJECTS: readonly HelpSubject[] = [
  'spell',
  'stat',
  'school',
  'bossAbility',
  'mechanic',
  'talent',
  'slot',
];

/**
 * Die Einträge aus dem Katalog — sie gehen an den Spieler.
 *
 * Ein Zauber-Eintrag bekommt seine Werte-Zeile aus den Zauberdaten. Eine
 * Werte-Zeile in der Datei selbst bleibt unangetastet: Dann hat jemand sie
 * bewusst gesetzt.
 */
export const PLAYABLE_HELP_ENTRIES: readonly HelpEntry[] = parseHelpEntries(
  (rawCatalog as { entries?: unknown }).entries,
  'data/help/help.json',
).map((entry) => {
  if (entry.values !== undefined) return entry;
  if (entry.subject === 'spell') {
    const spell = CALCINATION.value.spells.find((s) => s.id === entry.id);
    if (spell !== undefined) {
      return { ...entry, values: spellValues(spell, resourceNameOf(CALCINATION.value)) };
    }
  }
  if (entry.subject === 'talent') {
    // Dieselbe Begründung wie beim Zauber: Die Ränge und ihre Wirkung stehen
    // in `data/player/talents.json`, und dort setzt sie das Talent-Paket.
    const talent = TALENTS.find((t) => t.id === entry.id);
    if (talent !== undefined) {
      return { ...entry, values: talentValues(talent, (id) => TALENTS.find((t) => t.id === id)) };
    }
  }
  return entry;
});

/** Die Einführungen aus dem Katalog. */
export const PLAYABLE_INTRODUCTIONS: readonly Introduction[] = parseIntroductions(
  (rawCatalog as { introductions?: unknown }).introductions,
  'data/help/help.json',
);

/**
 * Die Prüfeinträge — im Entwicklungs-Bau und in den Prüffällen, sonst leer.
 *
 * Sie tragen dieselben Kennungen wie der Katalog („essence", „kindle") und
 * gehören deshalb nie in dieselbe Liste: `helpFor` nähme den ersten Treffer.
 */
export const ALL_HELP_ENTRIES: readonly HelpEntry[] = __ROTA_DEVTOOLS__
  ? parseHelpEntries((rawHelp as { entries?: unknown }).entries, 'data/help/test-help.json')
  : [];

/** Die Prüf-Einführungen, ebenfalls nur im Entwicklungs-Bau. */
export const ALL_INTRODUCTIONS: readonly Introduction[] = __ROTA_DEVTOOLS__
  ? parseIntroductions(
      (rawHelp as { introductions?: unknown }).introductions,
      'data/help/test-help.json',
    )
  : [];

/**
 * Prüft und übersetzt die rohen Einträge.
 *
 * **Als eigene Funktion, damit die Prüfungen prüfbar sind** — und diesmal mit
 * Fällen, die sie auch aufrufen. Bei `parseEntries` (#85) stand derselbe Satz
 * im Kommentar, und die Prüfungen darin waren bis #809 selbst ungeprüft: Der
 * Kommentar beschrieb eine Absicht, nicht einen Zustand.
 */
export function parseHelpEntries(roh: unknown, datei = 'data/help'): HelpEntry[] {
  if (!Array.isArray(roh)) {
    throw new Error(`${datei}: "entries" fehlt oder ist keine Liste`);
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
export function parseIntroductions(roh: unknown, datei = 'data/help'): Introduction[] {
  if (!Array.isArray(roh)) {
    throw new Error(`${datei}: "introductions" fehlt oder ist keine Liste`);
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
