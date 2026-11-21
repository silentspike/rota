/**
 * Die Archiv-Einträge aus den Daten (Issue #85, Katalog seit #1177).
 *
 * # Zwei Dateien, zwei Wege
 *
 * ```text
 *   data/archive/archive.json        der Katalog — geht an den Spieler
 *   data/archive/test-entries.json   Prüfeinträge — nur im Entwicklungs-Bau
 * ```
 *
 * Bis #1177 gab es nur die zweite Datei, und das Spiel las sie: Nach dem
 * ersten Sieg über den Prolog-Boss stand beim Spieler ein Fenster „Test Entry
 * — Prologue Victory". Die Trennung über das Präfix `test-` filterte zwar die
 * ausgelieferte Liste, aber Aufzeichnung und Ansicht nahmen die ungefilterte.
 *
 * # Die Prüfeinträge fallen aus dem Auslieferungs-Bau heraus
 *
 * Hinter `__ROTA_DEVTOOLS__`, wie die Entwicklungs-Werkzeuge (GAME-DESIGN
 * 19.3): In der Auslieferung ersetzt Vite die Konstante durch `false`, der
 * Zweig fällt weg, und mit ihm der einzige Leser der Prüfdatei — sie landet
 * nicht im Bündel. Geprüft wird das am gebauten Stand, nicht hier:
 * `tools/auslieferung-pruefdaten.py` sucht ihre Texte in `dist/`, im CI-Job
 * „build" nach jedem Bau.
 */

import rawCatalog from '../../data/archive/archive.json';
import rawTestEntries from '../../data/archive/test-entries.json';
import {
  ARCHIVE_OUTCOMES,
  ARCHIVE_TRIGGERS,
  ARCHIVE_VOICES,
  type ArchiveEntry,
  type ArchiveOutcome,
} from './archive.js';

/** Woran ein Prüfeintrag erkannt wird. */
const TEST_PREFIX = 'test-';

/**
 * Die Einträge, die der Spieler sehen kann: der Katalog.
 *
 * Aufzeichnung (`recordRun`), Boss-Wahl und Archiv-Ansicht lesen diese Liste
 * und keine andere.
 */
export const PLAYABLE_ARCHIVE_ENTRIES: readonly ArchiveEntry[] = parseEntries(
  (rawCatalog as { entries?: unknown }).entries,
  'data/archive/archive.json',
).filter((entry) => !entry.id.startsWith(TEST_PREFIX));

/**
 * Die Prüfeinträge — im Entwicklungs-Bau und in den Prüffällen, sonst leer.
 *
 * Für Prüffälle, die alle vier Stimmen in kleinem Massstab brauchen. Das
 * Spiel liest sie nicht — auch im Entwicklungs-Bau nicht, sonst stünde dort
 * nach dem zweiten Sieg über den Prolog-Boss wieder ein Prüftext.
 */
export const TEST_ARCHIVE_ENTRIES: readonly ArchiveEntry[] = __ROTA_DEVTOOLS__
  ? parseEntries(
      (rawTestEntries as { entries?: unknown }).entries,
      'data/archive/test-entries.json',
    )
  : [];

/** Welches Feld ein Auslöser zusätzlich verlangt — sonst erschiene er nie. */
function pflichtfeld(trigger: string): string | null {
  if (trigger === 'afterFirstKill') return 'bossId';
  if (trigger === 'spellLearned') return 'spellId';
  if (trigger === 'schoolLearned') return 'school';
  return null;
}

/**
 * Prüft und übersetzt die rohen Einträge.
 *
 * **Als eigene Funktion, damit die Prüfungen prüfbar sind.** Steckte sie in
 * einer sofort ausgeführten Funktion, liefe sie genau einmal beim Laden des
 * Moduls mit genau den ausgelieferten Daten — ein Fall müsste die echte Datei
 * kaputtmachen, um zu sehen, ob ein Wächter greift, und damit jeden anderen
 * Fall gleich mit. Gemessen an `parseSets` (#780) und `comparisonForResult`
 * (#82): zweimal dieselbe Lehre.
 */
export function parseEntries(roh: unknown, datei = 'data/archive'): ArchiveEntry[] {
  if (!Array.isArray(roh)) {
    throw new Error(`${datei}: "entries" fehlt oder ist keine Liste`);
  }
  const vergeben = new Set<string>();
  return roh.map((eintrag) => {
    const e = eintrag as Record<string, unknown>;
    const id = typeof e['id'] === 'string' ? e['id'] : '';
    if (id === '') {
      throw new Error('data/archive: ein Eintrag ohne Kennung — er wäre nie freischaltbar');
    }
    // **Zwei Einträge mit derselben Kennung wären still**: `unlock` merkt sich
    // die Kennung, nicht den Eintrag. Beide gälten danach als gesehen, und der
    // zweite erschiene nie. Dieselbe Klasse wie die Satz-Kennungen in #780,
    // wo drei Zweige unabhängig dieselbe Nummer vergaben.
    if (vergeben.has(id)) {
      throw new Error(`data/archive: die Kennung "${id}" kommt zweimal vor`);
    }
    vergeben.add(id);

    const trigger = e['trigger'];
    if (!ARCHIVE_TRIGGERS.includes(trigger as (typeof ARCHIVE_TRIGGERS)[number])) {
      throw new Error(`data/archive: "${id}" hat den unbekannten Auslöser ${String(trigger)}`);
    }
    const voice = e['voice'];
    if (!ARCHIVE_VOICES.includes(voice as (typeof ARCHIVE_VOICES)[number])) {
      throw new Error(`data/archive: "${id}" hat die unbekannte Stimme ${String(voice)}`);
    }
    // **Die Instanz brauchen nur die zwei Auslöser, die an einer hängen.**
    // Die übrigen stehen in ihrem Kapitel (`ARCHIVE_CHAPTERS`); für sie wäre
    // eine Pflicht-Instanz eine erfundene Zuordnung.
    const instanceId = typeof e['instanceId'] === 'string' ? e['instanceId'] : '';
    const anInstanz = trigger === 'beforeInstance' || trigger === 'afterFirstKill';
    if (anInstanz && instanceId === '') {
      throw new Error(`data/archive: "${id}" nennt keine Instanz — die Ansicht gliedert danach`);
    }
    // **Ein Auslöser ohne sein Feld erschiene nie.** `nextAutomaticEntry`
    // vergleicht `entry.bossId` mit dem gefallenen Boss, `discoveredEntries`
    // den Zauber und die Schule; ohne Kennung trifft der Vergleich nie zu, und
    // der Eintrag bliebe für immer verborgen — ohne Fehler, ohne Hinweis.
    const feld = pflichtfeld(String(trigger));
    if (feld !== null && (typeof e[feld] !== 'string' || e[feld] === '')) {
      throw new Error(
        trigger === 'afterFirstKill'
          ? `data/archive: "${id}" löst nach einem Sieg aus, nennt aber keinen Boss`
          : `data/archive: "${id}" löst bei ${String(trigger)} aus, nennt aber kein ${feld}`,
      );
    }
    const setId = e['setId'];
    if (trigger === 'setFound' && (typeof setId !== 'number' || !Number.isInteger(setId))) {
      throw new Error(`data/archive: "${id}" löst bei einem Satz aus, nennt aber keinen`);
    }
    const outcome = e['outcome'];
    if (
      trigger === 'firstOutcome' &&
      !ARCHIVE_OUTCOMES.includes(outcome as (typeof ARCHIVE_OUTCOMES)[number])
    ) {
      throw new Error(`data/archive: "${id}" hat den unbekannten Ausgang ${String(outcome)}`);
    }
    const title = typeof e['title'] === 'string' ? e['title'] : '';
    const body = typeof e['body'] === 'string' ? e['body'] : '';
    if (title === '' || body === '') {
      throw new Error(`data/archive: "${id}" hat keinen Titel oder keinen Text`);
    }

    const text = (name: string): string => (typeof e[name] === 'string' ? (e[name] as string) : '');
    return {
      id,
      trigger: trigger as ArchiveEntry['trigger'],
      voice: voice as ArchiveEntry['voice'],
      title,
      body,
      ...(instanceId === '' ? {} : { instanceId }),
      ...(text('bossId') === '' ? {} : { bossId: text('bossId') }),
      ...(text('spellId') === '' ? {} : { spellId: text('spellId') }),
      ...(text('school') === '' ? {} : { school: text('school') }),
      ...(trigger === 'setFound' ? { setId: setId as number } : {}),
      ...(trigger === 'firstOutcome' ? { outcome: outcome as ArchiveOutcome } : {}),
    };
  });
}
