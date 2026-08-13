/**
 * Die Archiv-Einträge aus den Daten (Issue #85).
 *
 * # Warum hier heute nur Prüfeinträge stehen
 *
 * Die ~150 echten Texte sind ausdrücklich **E10**. Der Issue sagt es wörtlich
 * („Die ~150 Archiv-Texte selbst gehören zu E10 — hier entsteht nur die
 * Ansicht"), und GAME-DESIGN Zeile 2620 führt sie in der Tabelle der offenen
 * Inhalte unter E10. Was hier entsteht, ist die Leitung: von den Daten über
 * den Spielstand bis in die Ansicht.
 *
 * # Die Trennung liegt am Präfix und ist gemessen
 *
 * `test-*` fällt aus dem ausgelieferten Stand heraus, alles andere bleibt
 * drin. Dieselbe Konvention wie bei `test-cast` (#304), `test-buff` (#420) und
 * `test-channel` (#323) — dort über `unlockLevel: 101`, hier über die Kennung,
 * weil ein Archiv-Eintrag keine Stufe hat.
 *
 * **Warum überhaupt getrennt und nicht einfach weggelassen:** Ohne Einträge
 * lässt sich die Ansicht nicht prüfen — weder dass sie gliedert, noch dass ein
 * Eintrag beim ersten Sieg erscheint und beim zweiten nicht mehr. Mit ihnen im
 * ausgelieferten Stand stünde Prüftext im Spiel.
 */

import rawEntries from '../../data/archive/test-entries.json';
import { ARCHIVE_TRIGGERS, ARCHIVE_VOICES, type ArchiveEntry } from './archive.js';

/** Woran ein Prüfeintrag erkannt wird. */
const TEST_PREFIX = 'test-';

/**
 * Alle Einträge, auch die Prüfeinträge.
 *
 * Für Prüffälle und die Ansicht im Entwicklungs-Bau. Das Spiel nimmt
 * [`PLAYABLE_ARCHIVE_ENTRIES`].
 */
export const ALL_ARCHIVE_ENTRIES: readonly ArchiveEntry[] = parseEntries(
  (rawEntries as { entries?: unknown }).entries,
);

/**
 * Die Einträge, die der Spieler sehen kann.
 *
 * Heute ist die Liste **leer**, und das ist kein Fehler: Es gibt noch keinen
 * Katalog-Inhalt. Sobald E10 ihn liefert, füllt sie sich von selbst — ohne
 * dass hier eine Zeile geändert werden muss.
 */
export const PLAYABLE_ARCHIVE_ENTRIES: readonly ArchiveEntry[] = ALL_ARCHIVE_ENTRIES.filter(
  (entry) => !entry.id.startsWith(TEST_PREFIX),
);

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
export function parseEntries(roh: unknown): ArchiveEntry[] {
  if (!Array.isArray(roh)) {
    throw new Error('data/archive/test-entries.json: "entries" fehlt oder ist keine Liste');
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
    const instanceId = typeof e['instanceId'] === 'string' ? e['instanceId'] : '';
    if (instanceId === '') {
      throw new Error(`data/archive: "${id}" nennt keine Instanz — die Ansicht gliedert danach`);
    }
    // **Ein `afterFirstKill` ohne Boss erschiene nie.** `nextAutomaticEntry`
    // vergleicht `entry.bossId` mit dem gefallenen Boss; ohne Kennung trifft
    // der Vergleich nie zu, und der Eintrag bliebe für immer verborgen — ohne
    // Fehler, ohne Hinweis.
    const bossId = typeof e['bossId'] === 'string' ? e['bossId'] : '';
    if (trigger === 'afterFirstKill' && bossId === '') {
      throw new Error(`data/archive: "${id}" löst nach einem Sieg aus, nennt aber keinen Boss`);
    }
    const title = typeof e['title'] === 'string' ? e['title'] : '';
    const body = typeof e['body'] === 'string' ? e['body'] : '';
    if (title === '' || body === '') {
      throw new Error(`data/archive: "${id}" hat keinen Titel oder keinen Text`);
    }

    return {
      id,
      instanceId,
      trigger: trigger as ArchiveEntry['trigger'],
      voice: voice as ArchiveEntry['voice'],
      title,
      body,
      ...(bossId === '' ? {} : { bossId }),
    };
  });
}
