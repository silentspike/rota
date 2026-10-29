/**
 * Archiv und Hilfe als Ansicht (GAME-DESIGN 17.3 und 17.4, Issue #85).
 *
 * HTML, nicht Zeichenflaeche: Der Kampf laeuft auf dem Canvas, Menues laufen in
 * HTML (GAME-DESIGN 18.1).
 *
 * DREI DINGE, DIE DIESE ANSICHT NICHT TUT
 * ---------------------------------------
 * 1. Sie zeigt keinen gesperrten Eintrag, auch nicht ausgegraut. Eine graue
 *    Zeile ist eine Ankuendigung — ein Hinweis auf Inhalte, die der Spieler
 *    noch nicht erreicht hat.
 * 2. Sie nennt in keinem Erklaerfenster, WANN man etwas benutzen sollte. Das
 *    waere eine Rotations-Empfehlung (GAME-DESIGN 21.2).
 * 3. Sie oeffnet kein Nachschlagewerk im Menue. Die Erklaerung steht dort, wo
 *    die Sache steht.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import type { ArchiveEntry, ArchiveSection } from './archive.js';
import { helpFor, withPlainText, type HelpEntry, type Introduction } from './help.js';

const VOICE_LABEL: Readonly<Record<string, string>> = {
  ownNotes: 'Own notes',
  fragment: 'Found fragment',
  deviceLog: 'Device log',
  other: 'Unattributed',
};

/**
 * Die Überschriften der Kapitel ohne Instanz (Issue #1177). Die Kennungen
 * stehen in `ARCHIVE_CHAPTERS` (archive.ts); hier steht nur, was der Spieler
 * liest.
 */
export const ARCHIVE_CHAPTER_TITLES: Readonly<Record<string, string>> = {
  processes: 'Processes',
  spells: 'Spells',
  sets: 'Sets',
  finds: 'Finds',
};

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** Ein einzelner Archiv-Eintrag. */
export function buildEntry(entry: ArchiveEntry): HTMLElement {
  const box = element('article', 'archive-entry');
  box.dataset.entryId = entry.id;
  const head = element('header', 'archive-entry-head');
  head.appendChild(element('h3', 'archive-entry-title', entry.title));
  head.appendChild(element('span', 'archive-voice', VOICE_LABEL[entry.voice] ?? entry.voice));
  box.appendChild(head);
  box.appendChild(element('p', 'archive-body', entry.body));
  return box;
}

/**
 * Der frisch freigeschaltete Eintrag als Fenster ueber der Auswertung
 * (Issue #85, AC1; GAME-DESIGN 20.5 Punkt 6).
 *
 * # Warum dieselbe Bauart wie die gefuehrte Einfuehrung
 *
 * Der Auftraggeber hat es so entschieden: `.intro`, CONTINUE und SKIP. Der
 * Spieler kennt die Geste bereits, und der Text ist nicht zu uebersehen.
 *
 * Verworfen wurden: ein Abschnitt IN der Auswertung (dort stehen schon
 * Punkte, Bestlauf-Vergleich und Stufe — er ginge unter) und der Weg ueber
 * das Player Home (der Zusammenhang zum Sieg ginge verloren, und AC1 sagt
 * „beim ersten Sieg").
 *
 * # Der Inhalt kommt aus `buildEntry`
 *
 * Nicht neu zusammengesetzt: Titel, Stimme und Text stehen im Archiv schon
 * fertig da, und zwei Stellen, die einen Eintrag darstellen, gingen beim
 * naechsten Feld auseinander.
 *
 * # Beide Knoepfe schliessen — und das ist kein Versehen
 *
 * Genau wie bei der Einfuehrung: Wer ueberspringt, will den Text nicht lesen,
 * nicht ihn spaeter wiedersehen. Das Wiedersehen steht ohnehin frei, im
 * Archiv des Player Home — dort ist der Eintrag ab sofort dauerhaft.
 */
export function buildEntryPopup(entry: ArchiveEntry, onDismiss: () => void): HTMLElement {
  const box = element('div', 'intro archive-popup');
  box.setAttribute('role', 'dialog');
  box.dataset.entryId = entry.id;
  box.appendChild(buildEntry(entry));

  const actions = element('div', 'intro-actions');
  for (const [label, className] of [
    ['CONTINUE', 'intro-continue'],
    ['SKIP', 'intro-skip'],
  ] as const) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = className;
    button.textContent = label;
    button.addEventListener('click', () => {
      onDismiss();
      box.remove();
    });
    actions.appendChild(button);
  }
  box.appendChild(actions);
  return box;
}

/**
 * Das Archiv, nach Instanz gegliedert.
 *
 * Bekommt bereits gefilterte Abschnitte — was freigeschaltet ist, entscheidet
 * `buildArchive` in archive.ts, nicht die Ansicht. Zwei Stellen, die darueber
 * befinden, waeren eine zu viel.
 */
export function buildArchiveView(
  sections: readonly ArchiveSection[],
  /**
   * Die Überschrift eines Abschnitts (Issue #1177): der Name der Instanz oder
   * des Kapitels. Vorher stand dort die Kennung („instance-1"). Ohne Angabe
   * bleibt es bei ihr — die Prüfseite kennt keine Instanzen.
   */
  titleOf: (sectionId: string) => string = (sectionId) => sectionId,
): HTMLElement {
  const root = element('section', 'archive');
  root.appendChild(element('h2', 'archive-heading', 'ARCHIVE'));

  if (sections.length === 0) {
    root.appendChild(
      element('p', 'archive-empty', 'Nothing recorded yet. Entries appear as you go.'),
    );
    return root;
  }

  for (const section of sections) {
    const group = element('div', 'archive-section');
    group.dataset.section = section.instanceId;
    group.appendChild(element('h3', 'archive-instance', titleOf(section.instanceId)));
    for (const entry of section.entries) group.appendChild(buildEntry(entry));
    root.appendChild(group);
  }
  return root;
}

/**
 * Das Erklaerfenster.
 *
 * Es traegt Wirkung, Werte und Synergien — in dieser Reihenfolge, weil der
 * Spieler zuerst wissen will, WAS passiert. Die Ueberschrift bekommt ihren
 * Klartext (AC3).
 */
export function buildHelpPopover(entry: HelpEntry): HTMLElement {
  const box = element('div', 'help-popover');
  box.setAttribute('role', 'tooltip');
  box.appendChild(element('h4', 'help-title', withPlainText(entry.title)));
  box.appendChild(element('p', 'help-effect', entry.effect));
  if (entry.values !== undefined) box.appendChild(element('p', 'help-values', entry.values));
  if (entry.synergies !== undefined) {
    box.appendChild(element('p', 'help-synergies', entry.synergies));
  }
  return box;
}

/**
 * Haengt Erklaerfenster an alles, was ein `data-help` traegt.
 *
 * Ueber das Merkmal statt ueber eine Liste von Auswahlausdruecken: So bekommt
 * eine Boss-Faehigkeit ihr Fenster durch dasselbe Zutun wie ein Zauber-Symbol,
 * und es kann nicht passieren, dass eine Gruppe vergessen wird, weil sie
 * woanders im Baum steht (AC2).
 */
export function attachHelp(root: ParentNode, entries: readonly HelpEntry[]): () => void {
  // **Wer schon versorgt ist, wird uebersprungen.** Das Player Home ersetzt
  // einzelne Bereiche und laesst andere stehen (`refreshHome` in app.ts); ein
  // zweiter Aufruf legte sonst ein zweites Paar Zuhoerer auf jeden Knoten, der
  // stehen geblieben ist. Das Merkmal steht am Knoten, damit die Entscheidung
  // dort faellt, wo sie gilt — eine Liste in dieser Funktion ueberlebte den
  // Austausch des Bereichs nicht.
  const targets = [...root.querySelectorAll<HTMLElement>('[data-help]')].filter(
    (node) => node.dataset.helpAttached !== 'yes',
  );
  let open: HTMLElement | null = null;

  const close = (): void => {
    open?.remove();
    open = null;
  };

  const show = (target: HTMLElement): void => {
    const id = target.dataset.help;
    if (id === undefined) return;
    const entry = helpFor(entries, id);
    // Kein Fenster ist besser als ein leeres.
    if (entry === null) return;
    close();
    const popover = buildHelpPopover(entry);
    target.appendChild(popover);
    open = popover;
  };

  for (const target of targets) {
    target.dataset.helpAttached = 'yes';
    target.addEventListener('mouseenter', () => {
      show(target);
    });
    target.addEventListener('mouseleave', close);
    // Auch ueber die Tastatur erreichbar — die Zauberleiste wird ausschliesslich
    // ueber Tasten bedient (Design-Gesetz 2), und wer sie so bedient, soll die
    // Erklaerung nicht nur mit der Maus bekommen.
    target.addEventListener('focus', () => {
      show(target);
    });
    target.addEventListener('blur', close);
  }
  return close;
}

/**
 * Die gefuehrte Einfuehrung.
 *
 * Sie haelt das Spiel an und erklaert die Mechanik an Ort und Stelle. Der
 * Knopf zum Ueberspringen ist Pflicht (AC4) — und Ueberspringen zaehlt als
 * gesehen, sonst kaeme sie beim naechsten Kampf wieder.
 */
export function buildIntroduction(
  intro: Introduction,
  onDismiss: (mechanicId: string) => void,
): HTMLElement {
  const box = element('div', 'intro');
  box.setAttribute('role', 'dialog');
  box.dataset.mechanicId = intro.mechanicId;
  box.appendChild(element('h3', 'intro-title', intro.title));
  box.appendChild(element('p', 'intro-body', intro.body));

  const actions = element('div', 'intro-actions');
  for (const [label, className] of [
    ['CONTINUE', 'intro-continue'],
    ['SKIP', 'intro-skip'],
  ] as const) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = className;
    button.textContent = label;
    button.addEventListener('click', () => {
      // Beide Wege zaehlen als gesehen. Wer ueberspringt, will sie nicht
      // wiedersehen — das ist der Sinn des Knopfes.
      onDismiss(intro.mechanicId);
      box.remove();
    });
    actions.appendChild(button);
  }
  box.appendChild(actions);
  return box;
}
