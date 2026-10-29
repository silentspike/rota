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
export const ARCHIVE_TRIGGERS = [
  'beforeInstance',
  'afterFirstKill',
  // **Seit #1177.** Der Masterplan zählt zum Archiv auch „Einträge für Zauber,
  // Gegenstands-Sätze und versteckte Funde". Die drei ersten kommen still ins
  // Archiv (`discoveredEntries`), der Fund als Fenster wie der erste Sieg.
  'firstOutcome',
  'spellLearned',
  'schoolLearned',
  'setFound',
] as const;

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

/**
 * Die Ausgänge, an denen ein versteckter Fund hängt (Issue #1177).
 *
 * Nur gezählte Ausgänge: Ein Abbruch ist „ausdrücklich keine Niederlage"
 * (GAME-DESIGN 10) und erreicht `recordRun` gar nicht erst als Versuch.
 */
export const ARCHIVE_OUTCOMES = ['death', 'overpressure'] as const;

/** Siehe [`ARCHIVE_OUTCOMES`]. */
export type ArchiveOutcome = (typeof ARCHIVE_OUTCOMES)[number];

/**
 * Die Kapitel des Archivs für Einträge, die an keiner Instanz hängen
 * (Issue #1177). Die Reihenfolge der Abschnitte folgt dem Katalog.
 */
export const ARCHIVE_CHAPTERS: Readonly<Partial<Record<ArchiveTrigger, string>>> = {
  schoolLearned: 'processes',
  spellLearned: 'spells',
  setFound: 'sets',
  firstOutcome: 'finds',
};

export interface ArchiveEntry {
  readonly id: string;
  /**
   * Bei `beforeInstance` und `afterFirstKill`: die Instanz, nach der die
   * Ansicht gliedert. Die übrigen Auslöser hängen an keiner Instanz und
   * stehen in ihrem Kapitel ([`ARCHIVE_CHAPTERS`]).
   */
  readonly instanceId?: string;
  readonly trigger: ArchiveTrigger;
  readonly voice: ArchiveVoice;
  /** Englisch — steht im Spiel (Design-Gesetz 5). */
  readonly title: string;
  readonly body: string;
  /** Bei `afterFirstKill`: welcher Boss. Sonst leer. */
  readonly bossId?: string;
  /** Bei `spellLearned`: welcher Zauber. */
  readonly spellId?: string;
  /** Bei `schoolLearned`: welche Schule. */
  readonly school?: string;
  /** Bei `setFound`: welcher Gegenstands-Satz. */
  readonly setId?: number;
  /** Bei `firstOutcome`: welcher Ausgang. */
  readonly outcome?: ArchiveOutcome;
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
  /** Bei `beforeInstance` und `afterFirstKill`. */
  readonly instanceId?: string;
  /** Nur bei `afterFirstKill` gesetzt. */
  readonly bossId?: string;
  /** Nur bei `firstOutcome` gesetzt. */
  readonly outcome?: ArchiveOutcome;
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
    if (context.trigger === 'firstOutcome') {
      // Ein Fund hängt am Ausgang, nicht an einer Instanz: Der erste Tod ist
      // der erste, gleich gegen wen.
      if (entry.outcome !== context.outcome) continue;
    } else {
      if (entry.instanceId !== context.instanceId) continue;
      if (context.trigger === 'afterFirstKill' && entry.bossId !== context.bossId) continue;
    }
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

/**
 * Was der Spieler kennt — die Grundlage der stillen Einträge (Issue #1177).
 *
 * Kein Spielstand, sondern eine Auskunft daraus: welche Zauber in seiner
 * Zusammenstellung stehen, welche Schulen er damit führt und von welchen
 * Sätzen er ein Stück besitzt. Der Aufrufer bildet sie; diese Datei kennt
 * weder Zauberbuch noch Gegenstände.
 */
export interface ArchiveKnowledge {
  readonly spellIds: readonly string[];
  readonly schools: readonly string[];
  readonly setIds: readonly number[];
}

/**
 * Die Einträge, die jetzt still dazukommen: Zauber, Schule, Satz (Issue #1177).
 *
 * # Still, und das ist entschieden
 *
 * Kein Fenster. 58 Zauber hiessen 58 Fenster — genau das, was die Erzählweise
 * aus dem Plan vermeiden will („Bei hunderten Wiederholungen steht die
 * Geschichte damit nie im Weg"). Das Archiv ist das Sammelziel; dort stehen
 * sie ab sofort.
 *
 * Wie bei `nextAutomaticEntry` zählt „noch nicht freigeschaltet", nicht
 * „gerade gelernt": Ein Spielstand von vor #1177 kennt seine Zauber längst und
 * bekommt ihre Einträge beim ersten Laden nachgereicht.
 */
export function discoveredEntries(
  entries: readonly ArchiveEntry[],
  state: ArchiveState,
  known: ArchiveKnowledge,
): ArchiveEntry[] {
  const seen = new Set(state.unlockedEntryIds);
  const spells = new Set(known.spellIds);
  const schools = new Set(known.schools);
  const sets = new Set(known.setIds);
  return entries.filter((entry) => {
    if (seen.has(entry.id)) return false;
    if (entry.trigger === 'spellLearned') return spells.has(entry.spellId ?? '');
    if (entry.trigger === 'schoolLearned') return schools.has(entry.school ?? '');
    if (entry.trigger === 'setFound') return entry.setId !== undefined && sets.has(entry.setId);
    return false;
  });
}

/** Mehrere Einträge auf einmal vermerken. Erzeugt einen neuen Zustand. */
export function unlockAll(state: ArchiveState, entryIds: readonly string[]): ArchiveState {
  return entryIds.reduce(unlock, state);
}

/**
 * Der Abschnitt, in dem ein Eintrag steht: seine Instanz oder sein Kapitel.
 *
 * Ein Eintrag ohne beides fiele in ein Kapitel ohne Namen — der Parser lässt
 * ihn deshalb gar nicht erst durch.
 */
export function archiveSectionOf(entry: ArchiveEntry): string {
  return entry.instanceId ?? ARCHIVE_CHAPTERS[entry.trigger] ?? entry.trigger;
}

/** Ein Abschnitt der Archiv-Ansicht. */
export interface ArchiveSection {
  /**
   * Die Instanz — oder, für Einträge ohne Instanz, das Kapitel aus
   * [`ARCHIVE_CHAPTERS`]. Der Name blieb, damit bestehende Leser nicht
   * umbrechen; was darin steht, entscheidet [`archiveSectionOf`].
   */
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
    const section = archiveSectionOf(entry);
    const list = byInstance.get(section) ?? [];
    list.push(entry);
    byInstance.set(section, list);
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
