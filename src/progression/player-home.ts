/**
 * Das Player Home — der Bildschirm zwischen den Kämpfen (GAME-DESIGN 16.1,
 * Issue #188).
 *
 * ER IST EIN SPIELER-BILDSCHIRM
 * -----------------------------
 * Und bleibt im Auslieferungs-Bau vollständig vorhanden. Die vier
 * Test-Einstellungen aus 19.2 sind ein **Aufsatz** darin, den 19.3
 * herausschneidet — nicht der Bildschirm.
 *
 * Bis zum 2026-08-04 stand „Player Home" im ganzen Design genau einmal, in der
 * Überschrift des Werkzeug-Kapitels. Wer nur das Dokument las, musste
 * schließen, dass der Bildschirm beim Bauen verschwindet. 16.1 trägt die
 * Definition nach; diese Datei ist ihre erste Umsetzung.
 *
 * ER IST EIN GERÜST, KEIN EINZELINHALT
 * ------------------------------------
 * Heute steht die Werte-Übersicht darin. Später kommen Talentbaum (12.5),
 * Boss-Wahl (11.6), Zusammenstellung (16) und Ausrüstung (13) dazu — nicht in
 * diesem Issue, aber die Aufteilung darf sie nicht ausschließen.
 *
 * Deshalb nimmt er **Bereiche** entgegen und kennt keinen von ihnen. Ein
 * Bildschirm, der die Werte-Übersicht selbst baut, müsste für jeden weiteren
 * Bereich angefasst werden; dieser nicht.
 *
 * ER FASST DEN KAMPF NICHT AN
 * ---------------------------
 * Kein Zugriff auf `Session` oder `Combat`, kein Anhalten, kein Zurücksetzen.
 * Er zeigt und verbirgt sich, mehr nicht — ein Bildschirm, der einen laufenden
 * Kampf beschädigen kann, wäre an dieser Stelle das größere Übel als gar
 * keiner (AC1).
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */

/** Ein Bereich des Bildschirms. */
export interface HomeSection {
  /** Kennung, kleingeschrieben — als Merkmal am Knoten, für Prüfungen. */
  readonly id: string;
  /** Überschrift im Spiel, englisch. */
  readonly title: string;
  /** Der Inhalt. Wird unverändert übernommen. */
  readonly content: HTMLElement;
  /**
   * Ob der Bereich nur in der Entwicklungs-Fassung erscheint.
   *
   * Für die vier Test-Einstellungen aus 19.2. Der Aufrufer entscheidet das —
   * hier steht kein `__ROTA_DEVTOOLS__`, weil dieser Bildschirm nichts davon
   * wissen muss und der Schalter sonst an zwei Stellen stünde.
   */
  readonly devOnly?: boolean;
}

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Der Bildschirm.
 *
 * Er baut sich einmal und wird danach nur noch gezeigt und verborgen. Ein
 * Neuaufbau bei jedem Öffnen würfe die Bildlaufstellung weg und erzeugte
 * Speicher in einem Moment, in dem der Spieler etwas ansieht.
 */
export class PlayerHome {
  readonly root: HTMLElement;
  private readonly sectionRoot: HTMLElement;
  private visible = false;

  /**
   * @param onClose Wird gerufen, wenn der Spieler zurück will. Der Bildschirm
   *   entscheidet **nicht** selbst, wohin — das weiß nur der Aufrufer.
   */
  constructor(private readonly onClose: () => void) {
    this.root = element('div', 'home');
    this.root.appendChild(element('h2', 'home-title', 'PLAYER HOME'));

    // Die Bereiche stehen in einem eigenen Behälter, damit der Zurück-Knopf
    // nicht zwischen ihnen landet, wenn später welche dazukommen.
    this.sectionRoot = element('div', 'home-sections');
    this.root.appendChild(this.sectionRoot);

    const back = document.createElement('button');
    back.type = 'button';
    back.className = 'home-back';
    back.textContent = 'Back';
    back.addEventListener('click', () => {
      this.hide();
      this.onClose();
    });
    this.root.appendChild(back);
  }

  /**
   * Einen Bereich aufnehmen.
   *
   * Nur-Entwicklungs-Bereiche werden gar nicht erst angelegt, nicht bloß
   * verborgen: Was im Baum steht, steht auch im ausgelieferten Programm.
   */
  addSection(section: HomeSection): boolean {
    if (section.devOnly === true && !__ROTA_DEVTOOLS__) return false;
    const box = element('section', 'home-section');
    box.dataset.section = section.id;
    if (section.devOnly === true) box.dataset.devOnly = 'yes';
    box.appendChild(element('h3', 'home-section-title', section.title));
    box.appendChild(section.content);
    this.sectionRoot.appendChild(box);
    return true;
  }

  /** Die Kennungen der aufgenommenen Bereiche, in ihrer Reihenfolge. */
  get sectionIds(): string[] {
    return [...this.sectionRoot.querySelectorAll<HTMLElement>('[data-section]')].map(
      (node) => node.dataset.section ?? '',
    );
  }

  get isVisible(): boolean {
    return this.visible;
  }

  show(): void {
    this.visible = true;
    this.root.classList.add('home-visible');
  }

  hide(): void {
    this.visible = false;
    this.root.classList.remove('home-visible');
  }
}
