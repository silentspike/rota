/**
 * Archiv und Erzaehlweise (GAME-DESIGN 17.4, Issue #85).
 *
 * DIE REGEL, UM DIE ES GEHT
 * -------------------------
 * Ein Text erscheint EINMAL automatisch — vor der Instanz oder nach dem
 * ERSTEN Boss-Sieg — und danach nie wieder von selbst. Nachlesbar bleibt er
 * jederzeit im Archiv.
 *
 * Das klingt nach einer Kleinigkeit und ist der ganze Unterschied: Bei
 * hunderten Wiederholungen desselben Bosses steht eine Geschichte, die sich
 * jedes Mal meldet, im Weg. Eine, die man nachlesen KANN, stoert nie.
 *
 * Diese Datei entscheidet nur, WANN etwas erscheint. Die ~150 Texte selbst
 * gehoeren zu Epic E10; hier entsteht die Ansicht und ihre Regel.
 */

/**
 * Die Ausloeser, als Liste (Issue #85).
 *
 * **Als Wert und nicht nur als Typ**: Der Parser in `archive-data.ts` muss zur
 * Laufzeit pruefen koennen, ob ein Wert aus der Datei dazugehoert. Ein Typ
 * allein verschwindet beim Uebersetzen, und die Pruefung waere eine
 * ausgeschriebene Liste an zweiter Stelle — die beim naechsten Ausloeser
 * vergessen wird.
 */
export const ARCHIVE_TRIGGERS = ['beforeInstance', 'afterFirstKill'] as const;

/** Wodurch ein Eintrag zum ersten Mal ausgeloest wird. */
export type ArchiveTrigger = (typeof ARCHIVE_TRIGGERS)[number];

/**
 * Wer spricht (GAME-DESIGN 17.4).
 *
 * Die Stimme steht am Eintrag, weil das Archiv danach gliedert — und weil ein
 * Geraete-Protokoll anders gelesen wird als eine eigene Notiz.
 */
export const ARCHIVE_VOICES = ['ownNotes', 'fragment', 'deviceLog', 'other'] as const;

/** Wer spricht — siehe [`ARCHIVE_VOICES`]. */
export type ArchiveVoice = (typeof ARCHIVE_VOICES)[number];

export interface ArchiveEntry {
  readonly id: string;
  readonly instanceId: string;
  readonly trigger: ArchiveTrigger;
  readonly voice: ArchiveVoice;
  /** Englisch — steht im Spiel (Design-Gesetz 5). */
  readonly title: string;
  readonly body: string;
  /** Bei `afterFirstKill`: welcher Boss. Sonst leer. */
  readonly bossId?: string;
}

/**
 * Was der Spieler bereits gesehen hat.
 *
 * Das ist etwas ERREICHTES und gehoert damit in den Spielstand — nicht in
 * diese Datei. Sie bekommt es uebergeben, statt es zu halten: Ein zweiter Ort,
 * an dem steht, was freigeschaltet ist, waere ein zweiter Ort, an dem es
 * falsch stehen kann.
 */
export interface ArchiveState {
  readonly unlockedEntryIds: readonly string[];
}

export interface TriggerContext {
  readonly trigger: ArchiveTrigger;
  readonly instanceId: string;
  /** Nur bei `afterFirstKill` gesetzt. */
  readonly bossId?: string;
}

/**
 * Welcher Eintrag jetzt automatisch erscheint — hoechstens einer.
 *
 * Bedingung ist NICHT "es ist der erste Sieg", sondern "der Eintrag ist noch
 * nicht freigeschaltet". Das ist robuster: Es haelt auch dann, wenn ein Sieg
 * nicht gezaehlt wurde oder ein Spielstand von einem anderen Geraet kommt.
 * Gezaehlt wird, was der Spieler gesehen hat, nicht wie oft er gewonnen hat.
 */
export function nextAutomaticEntry(
  entries: readonly ArchiveEntry[],
  state: ArchiveState,
  context: TriggerContext,
): ArchiveEntry | null {
  const seen = new Set(state.unlockedEntryIds);
  for (const entry of entries) {
    if (entry.trigger !== context.trigger) continue;
    if (entry.instanceId !== context.instanceId) continue;
    if (context.trigger === 'afterFirstKill' && entry.bossId !== context.bossId) continue;
    if (seen.has(entry.id)) continue;
    return entry;
  }
  return null;
}

/** Den Eintrag als gesehen vermerken. Erzeugt einen neuen Zustand. */
export function unlock(state: ArchiveState, entryId: string): ArchiveState {
  if (state.unlockedEntryIds.includes(entryId)) return state;
  return { unlockedEntryIds: [...state.unlockedEntryIds, entryId] };
}

/** Ein Abschnitt der Archiv-Ansicht. */
export interface ArchiveSection {
  readonly instanceId: string;
  readonly entries: readonly ArchiveEntry[];
}

/**
 * Das Archiv, nach Instanz und Stimme gegliedert.
 *
 * Es zeigt NUR Freigeschaltetes. Einen gesperrten Eintrag auch nur als graue
 * Zeile anzudeuten, waere eine Ankuendigung — und damit ein Hinweis auf
 * Inhalte, die der Spieler noch nicht erreicht hat.
 */
export function buildArchive(
  entries: readonly ArchiveEntry[],
  state: ArchiveState,
): ArchiveSection[] {
  const seen = new Set(state.unlockedEntryIds);
  const byInstance = new Map<string, ArchiveEntry[]>();
  for (const entry of entries) {
    if (!seen.has(entry.id)) continue;
    const list = byInstance.get(entry.instanceId) ?? [];
    list.push(entry);
    byInstance.set(entry.instanceId, list);
  }
  const sections: ArchiveSection[] = [];
  for (const [instanceId, list] of byInstance) {
    // Innerhalb einer Instanz zuerst der Text davor, dann die nach den Siegen —
    // das ist die Reihenfolge, in der sie erlebt wurden.
    list.sort(
      (a, b) =>
        Number(a.trigger === 'afterFirstKill') - Number(b.trigger === 'afterFirstKill') ||
        a.id.localeCompare(b.id),
    );
    sections.push({ instanceId, entries: list });
  }
  return sections;
}

/** Wie viele Eintraege freigeschaltet sind — fuer die Anzeige im Archiv. */
export function unlockedCount(entries: readonly ArchiveEntry[], state: ArchiveState): number {
  const seen = new Set(state.unlockedEntryIds);
  return entries.filter((entry) => seen.has(entry.id)).length;
}
