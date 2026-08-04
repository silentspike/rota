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
 * Das Archiv, nach Instanz gegliedert.
 *
 * Bekommt bereits gefilterte Abschnitte — was freigeschaltet ist, entscheidet
 * `buildArchive` in archive.ts, nicht die Ansicht. Zwei Stellen, die darueber
 * befinden, waeren eine zu viel.
 */
export function buildArchiveView(sections: readonly ArchiveSection[]): HTMLElement {
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
    group.appendChild(element('h3', 'archive-instance', section.instanceId));
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
  const targets = [...root.querySelectorAll<HTMLElement>('[data-help]')];
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
