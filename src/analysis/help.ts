/**
 * Hilfe-System (GAME-DESIGN 17.3, Issue #85).
 *
 * Drei Bausteine, ein Grundsatz: **Kein separates Nachschlagewerk im Menue**
 * (GAME-DESIGN 21.2). Die Erklaerung steht dort, wo die Sache steht — sonst
 * muesste der Spieler mitten im Lernen den Ort wechseln.
 *
 *   Klartext        ueberall "COAGULATION (Survival)"
 *   Erklaerfenster  Maus ueber Symbol, Wert oder Begriff
 *   Einfuehrung     beim ERSTEN Auftreten einer Mechanik, ueberspringbar
 *
 * WAS HIER NICHT PASSIERT
 * -----------------------
 * Das Hilfe-System erklaert, WAS etwas tut — nie, WANN man es benutzen
 * sollte. "Kindle builds heat" ist eine Erklaerung; "cast Kindle before Pyre"
 * waere eine Rotations-Empfehlung und damit genau das, was GAME-DESIGN 21.2
 * ausschliesst. Ein Prueffall wacht darueber (tests/unit/help.test.ts).
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */

/** Die fuenf Schulen mit ihrem Klartext (GAME-DESIGN 4). */
export const SCHOOL_PLAIN: Readonly<Record<string, string>> = {
  CALCINATION: 'Damage over time',
  FIXATION: 'Control',
  SUBLIMATION: 'Cooldown control',
  COAGULATION: 'Survival',
  PUTREFACTION: 'Weakening',
};

/** Die sechs Werte mit ihrem Klartext (GAME-DESIGN 6.1 und 6.2). */
export const STAT_PLAIN: Readonly<Record<string, string>> = {
  ESSENCE: 'Strength of the substance',
  VESSEL: 'How much the vessel endures',
  WARD: 'Protection against backlash',
  CATALYSIS: 'Speeds up reactions',
  PURITY: 'Chance of a critical result',
  YIELD: 'Size of a critical result',
};

/**
 * Fachbegriff mit Klartext daneben — "COAGULATION (Survival)".
 *
 * Gibt den Begriff unveraendert zurueck, wenn kein Klartext bekannt ist. Eine
 * erfundene Erklaerung waere schlimmer als keine.
 */
export function withPlainText(term: string): string {
  const plain = SCHOOL_PLAIN[term] ?? STAT_PLAIN[term];
  return plain === undefined ? term : `${term} (${plain})`;
}

/** Worauf sich eine Erklaerung bezieht. */
export type HelpSubject = 'spell' | 'stat' | 'school' | 'bossAbility' | 'mechanic';

export interface HelpEntry {
  readonly id: string;
  readonly subject: HelpSubject;
  /** Ueberschrift im Fenster, mit Klartext falls vorhanden. */
  readonly title: string;
  /** Was es tut. Nie, wann man es benutzen sollte. */
  readonly effect: string;
  /** Zahlen, soweit welche dazugehoeren. */
  readonly values?: string;
  /** Mit welchen Schulen es zusammenwirkt — Tatsache, keine Empfehlung. */
  readonly synergies?: string;
}

/**
 * Die Erklaerung zu einem Gegenstand der Oberflaeche.
 *
 * `null` heisst: dafuer gibt es keine: Dann erscheint kein Fenster, statt eines
 * mit leerem Inhalt.
 */
export function helpFor(entries: readonly HelpEntry[], id: string): HelpEntry | null {
  return entries.find((entry) => entry.id === id) ?? null;
}

/**
 * Deckt die Hilfe alles ab, was sie abdecken muss?
 *
 * AC2 verlangt Erklaerfenster ueber Zauber-Symbolen, Werten UND
 * Boss-Faehigkeiten. Die letzte Gruppe ist die, die man vergisst — sie steht
 * nicht auf der Zauberleiste, wo man beim Bauen hinsieht.
 */
export function missingSubjects(entries: readonly HelpEntry[]): HelpSubject[] {
  const vorhanden = new Set(entries.map((entry) => entry.subject));
  const noetig: HelpSubject[] = ['spell', 'stat', 'bossAbility'];
  return noetig.filter((subject) => !vorhanden.has(subject));
}

/** Eine Mechanik, die beim ersten Auftreten erklaert wird. */
export interface Introduction {
  readonly mechanicId: string;
  readonly title: string;
  readonly body: string;
}

export interface IntroductionState {
  readonly seenMechanicIds: readonly string[];
}

/**
 * Welche Einfuehrung jetzt faellig ist — hoechstens eine.
 *
 * Wie im Archiv haengt es daran, ob sie schon gesehen wurde, nicht daran, wie
 * oft die Mechanik auftrat.
 */
export function nextIntroduction(
  introductions: readonly Introduction[],
  state: IntroductionState,
  mechanicId: string
): Introduction | null {
  if (state.seenMechanicIds.includes(mechanicId)) return null;
  return introductions.find((intro) => intro.mechanicId === mechanicId) ?? null;
}

/** Eine Einfuehrung als gesehen vermerken — auch beim Ueberspringen. */
export function markSeen(state: IntroductionState, mechanicId: string): IntroductionState {
  if (state.seenMechanicIds.includes(mechanicId)) return state;
  return { seenMechanicIds: [...state.seenMechanicIds, mechanicId] };
}
