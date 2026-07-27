/**
 * Der Credits-Bereich.
 *
 * **Das ist eine Lizenzauflage, keine Höflichkeit.** Die Zauber-Symbole stammen
 * von game-icons.net und stehen unter CC BY 3.0 (GAME-DESIGN 17.2, `ASSETS.md`).
 * CC BY erlaubt die kommerzielle Nutzung nur mit Namensnennung, und game-icons.net
 * sagt dazu ausdrücklich: „if it's a video game, the mention can be accessible
 * from a menu". Ohne diesen Bereich wäre die Nutzung unzulässig.
 *
 * Bewusst HTML, nicht Zeichenfläche: Der Kampf läuft auf dem Canvas, Menüs laufen
 * in HTML (GAME-DESIGN 18.1). Der Bereich liegt damit auch außerhalb des
 * Bild-Pfads und kostet im Kampf nichts.
 *
 * Die Inhalte kommen aus `data/credits.json` — dieselbe Datei, gegen die
 * `tools/assets-check.py` prüft, dass keine nennungspflichtige Datei ohne
 * Nennung ausgeliefert wird.
 */
import type { CreditsData, CreditsEntry } from './credits-data.js';

/** Fehler, die eine Auslieferung verhindern müssen. */
export function validateCredits(data: CreditsData): string[] {
  const problems: string[] = [];
  const seen = new Set<string>();

  if (data.entries.length === 0) {
    problems.push('credits.json enthält keinen Eintrag');
  }

  for (const entry of data.entries) {
    const where = entry.id || '(ohne id)';
    if (!entry.id) {
      problems.push('Eintrag ohne id');
    } else if (seen.has(entry.id)) {
      problems.push(`${where}: id kommt doppelt vor`);
    } else {
      seen.add(entry.id);
    }
    for (const field of ['work', 'author', 'attribution', 'license', 'source'] as const) {
      if (!entry[field] || entry[field].trim() === '') {
        problems.push(`${where}: Feld "${field}" fehlt oder ist leer`);
      }
    }
    if (entry.covers.length === 0) {
      problems.push(`${where}: nennt keine Datei ("covers" ist leer)`);
    }
    // Eine nennungspflichtige Lizenz ohne Nennungstext wäre genau der Fehler,
    // den dieser Bereich verhindern soll.
    if (entry.required && !/^\S/.test(entry.attribution ?? '')) {
      problems.push(`${where}: nennungspflichtig, aber ohne brauchbaren Nennungstext`);
    }
  }
  return problems;
}

/** Die nennungspflichtigen Einträge zuerst — sie sind der Grund für den Bereich. */
export function sortEntries(entries: readonly CreditsEntry[]): CreditsEntry[] {
  return [...entries].sort((a, b) => {
    if (a.required !== b.required) return a.required ? -1 : 1;
    return a.author.localeCompare(b.author, 'en');
  });
}

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Baut den Inhalt des Bereichs. Reine DOM-Erzeugung ohne Nebenwirkung, damit die
 * Liste auch außerhalb des laufenden Spiels erzeugt werden kann.
 *
 * Spieltexte auf Englisch (Design-Gesetz 5).
 */
export function buildCreditsPanel(data: CreditsData): HTMLElement {
  const panel = element('div', 'credits-panel');
  panel.setAttribute('role', 'dialog');
  panel.setAttribute('aria-modal', 'true');
  panel.setAttribute('aria-label', 'Credits');

  panel.appendChild(element('h1', 'credits-title', 'CREDITS'));
  panel.appendChild(
    element(
      'p',
      'credits-lede',
      'ROTA uses free assets made by other people. The entries marked "attribution required" are a licence condition.',
    ),
  );

  const list = element('ul', 'credits-list');
  let lastWork = '';
  for (const entry of sortEntries(data.entries)) {
    const item = element('li', 'credits-item');
    // Drei Urheber teilen sich dasselbe Werk („Spell and process icons"). Die
    // Nennung braucht jeden einzeln, die Überschrift nur einmal.
    if (entry.work !== lastWork) {
      item.appendChild(element('div', 'credits-work', entry.work));
      lastWork = entry.work;
    }

    const line = element('div', 'credits-line');
    line.appendChild(element('span', 'credits-author', entry.attribution));
    const link = document.createElement('a');
    link.className = 'credits-source';
    link.href = entry.source;
    link.target = '_blank';
    link.rel = 'noopener noreferrer';
    link.textContent = entry.source.replace(/^https?:\/\//, '');
    line.appendChild(link);
    item.appendChild(line);

    const licence = element('div', 'credits-licence');
    const licenceLink = document.createElement('a');
    licenceLink.href = entry.licenseUrl;
    licenceLink.target = '_blank';
    licenceLink.rel = 'noopener noreferrer';
    licenceLink.textContent = entry.license;
    licence.appendChild(licenceLink);
    if (entry.required) {
      licence.appendChild(element('span', 'credits-required', 'attribution required'));
    }
    item.appendChild(licence);

    list.appendChild(item);
  }
  panel.appendChild(list);

  panel.appendChild(
    element(
      'p',
      'credits-foot',
      'Sounds are produced for ROTA. Full origin and licence of every file: ASSETS.md in the repository.',
    ),
  );
  return panel;
}

/**
 * Der Bereich im laufenden Spiel: ein Knopf und eine Überlagerung.
 *
 * Der Knopf sitzt vorläufig unten rechts am Bildrand. Sobald es ein Hauptmenü
 * gibt, gehört er dorthin — bis dahin muss die Nennung trotzdem erreichbar sein,
 * weil sonst jede Auslieferung die Lizenz verletzt.
 */
export class CreditsView {
  private readonly button: HTMLButtonElement;
  private readonly backdrop: HTMLElement;
  private readonly onKeyDown: (event: KeyboardEvent) => void;
  private mounted = false;

  constructor(
    private readonly parent: HTMLElement,
    private readonly data: CreditsData,
  ) {
    this.button = document.createElement('button');
    this.button.type = 'button';
    this.button.className = 'credits-button';
    this.button.textContent = 'CREDITS';
    this.button.addEventListener('click', () => {
      this.open();
    });

    this.backdrop = element('div', 'credits-backdrop');
    this.backdrop.hidden = true;
    this.backdrop.appendChild(buildCreditsPanel(this.data));
    this.backdrop.addEventListener('click', (event) => {
      // Nur der Rand schließt — ein Klick auf einen Quellen-Link nicht.
      if (event.target === this.backdrop) this.close();
    });

    this.onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape' && !this.backdrop.hidden) {
        this.close();
        // Der Kampf darf den Druck nicht als Fehleingabe werten.
        event.stopPropagation();
      }
    };
  }

  get isOpen(): boolean {
    return !this.backdrop.hidden;
  }

  mount(): void {
    if (this.mounted) return;
    this.parent.appendChild(this.button);
    this.parent.appendChild(this.backdrop);
    // In der Erfassungsphase, damit Escape den Bereich schließt, bevor die
    // Eingabe-Weiche den Druck sieht.
    window.addEventListener('keydown', this.onKeyDown, true);
    this.mounted = true;
  }

  open(): void {
    this.backdrop.hidden = false;
    this.dropFocus();
  }

  close(): void {
    this.backdrop.hidden = true;
    this.dropFocus();
  }

  /**
   * Der Knopf darf den Tastatur-Fokus nicht behalten.
   *
   * Sonst löst der Browser ihn bei jedem Druck auf Leertaste oder Eingabetaste
   * erneut aus — und die Leertaste ist im Spiel eine Zaubertaste (9.1). Ein
   * Menüknopf, der mitten im Kampf ein Fenster aufreißt, wäre genau der Fehler,
   * den Design-Gesetz 1 ausschließt: ein Tastendruck, eine Wirkung.
   */
  private dropFocus(): void {
    if (document.activeElement === this.button) {
      this.button.blur();
    }
  }

  dispose(): void {
    window.removeEventListener('keydown', this.onKeyDown, true);
    this.button.remove();
    this.backdrop.remove();
    this.mounted = false;
  }
}
