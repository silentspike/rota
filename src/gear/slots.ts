/**
 * Die zehn Ausrüstungsplätze (GAME-DESIGN 13.2, Issue E6-1).
 *
 * > Bewusst **keine Körperteile**, weil es keine Spielfigur gibt — arkane
 * > Artefakte. (13.2)
 *
 * **Ein Platz ist ein Platz.** Er nimmt einen Gegenstand auf, mehr nicht —
 * wie in WoW oder Diablo. Er hat keine Aufgabe, keinen Charakter und keinen
 * Wert, der ihm gehört.
 *
 * # Warum das hier so ausdrücklich steht
 *
 * Bis zum 2026-07-29 trug diese Datei eine **platz-spezifische
 * Sonderwert-Zuordnung**: Focus und Catalyst immer CATALYSIS, Sigil und
 * Talisman immer PURITY/YIELD, und so fort. Sie stand im Plan, sie stand im
 * Design-Dokument, und sie hatte eine Begründung, die überzeugend klang —
 * „gezieltes Grinden statt blinden Sammelns".
 *
 * **Angeordnet worden war sie nie.** Nachgesehen statt angenommen:
 *
 * ```text
 *   mainrag search "Ausruestungsplaetze Fokus Katalysator Siegel
 *                   platz-spezifische Sonderwerte" --source claude-conversations
 *     -> No results
 *   mainrag search "Fokus Katalysator Siegel Reliquie Grimoire"
 *     -> No results
 * ```
 *
 * Keine einzige Fundstelle. Die Regel war beim Schreiben entstanden und wurde
 * nie hinterfragt, weil ihre Begründung plausibel war. Der Auftraggeber hat
 * sie gestrichen: *„wann haben wir besprochen dass ausruestungsplaetze eine
 * aufgabe haben? das sind genauso wie bei wow oder diablo ausruestungsplaetze
 * in welche die items reinkommen"*.
 *
 * Aufgefallen ist es, weil zwei der zehn Plätze — `Relic` und `Shard` — im
 * ganzen Dokument keine Zuordnung hatten. Statt sie zu erraten, wurde
 * gefragt. Ein geratener Sonderwert hätte die falsche Regel bestätigt, statt
 * sie auffliegen zu lassen.
 *
 * **Die Lehre gehört hierher und nicht in einen Commit-Text:** Eine Regel mit
 * guter Begründung ist damit noch nicht angeordnet. Wer die Lücke in einer
 * Regel findet, hat womöglich die Regel gefunden.
 */

/** Die zehn Plätze in der Reihenfolge aus 13.2. */
export const SLOT_FOCUS = 0;
export const SLOT_CATALYST = 1;
export const SLOT_SIGIL = 2;
export const SLOT_RELIC = 3;
export const SLOT_GRIMOIRE = 4;
export const SLOT_SHARD = 5;
export const SLOT_TALISMAN = 6;
export const SLOT_PHYLACTERY = 7;
export const SLOT_ANCHOR = 8;
export const SLOT_PRISM = 9;

export const SLOT_COUNT = 10;

export const SLOT_NAMES: readonly string[] = [
  'focus',
  'catalyst',
  'sigil',
  'relic',
  'grimoire',
  'shard',
  'talisman',
  'phylactery',
  'anchor',
  'prism',
];

/**
 * Die Grundwerte, die **jeder** Gegenstand trägt (13.2, 6.1).
 *
 * > „Grundwerte (ESSENCE, VESSEL, WARD) auf jedem Platz — jeder Fund
 * > verbessert also spürbar etwas, auch wenn die Sonderwerte nicht passen."
 *
 * **Sie kommen zusätzlich zu den Affixen**, nicht aus deren Budget — wie ein
 * Diablo-Gegenstand immer Rüstung trägt, unabhängig davon, welche Affixe
 * darauf gewürfelt wurden. Ein Common-Stück hat also ESSENCE, VESSEL und WARD
 * **plus** einen Affix.
 *
 * Das war die vierte offene Frage aus #114 und ist seit dem 2026-07-29
 * entschieden. Anlass war eine Rechnung, die nicht aufging: 13.3 gibt Common
 * genau **einen** Werte-Platz, und ein Platz kann keine drei Grundwerte
 * tragen. Die Spalte meint **Affixe**, nicht Werte überhaupt.
 */
export const BASE_STAT_IDS: readonly string[] = ['essence', 'vessel', 'ward'];
