/**
 * Vorbereitung, Esc-Menü und Auswertung (GAME-DESIGN 10, 18.1).
 *
 * **HTML, nicht Canvas.** GAME-DESIGN 18.1 teilt bewusst auf: Canvas für den
 * Kampf, HTML für Menüs und Auswertung. Der Grund steht in 18.2 — jede Änderung
 * am HTML löst eine Neuberechnung des Layouts aus. Deshalb wird hier **nur bei
 * einem Phasenwechsel** angefasst, nie je Bild.
 *
 * Die Maus ist hier erlaubt: Design-Gesetz 2 verbietet das Klicken von **Zaubern**,
 * nicht das Bedienen von Menüs (GAME-DESIGN 8.1: „Die Maus dient nur Menüs,
 * Talentbaum und Zusammenstellung").
 *
 * Alle Texte englisch (Design-Gesetz 5).
 */
import {
  OUTCOME_DEATH,
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
  PHASE_ENDED,
  PHASE_PREPARE,
} from '../core/phases.js';

export interface OverlayActions {
  onReady(): void;
  onResume(): void;
  onWipe(): void;
  onRestart(): void;
  onBackToMenu(): void;
}

/** Was die Auswertung anzeigt. Die drei Kennzahlen aus 15.1 folgen mit E1-7. */
export interface ResultView {
  readonly outcome: number;
  /** Verstrichene Kampfzeit in ms. */
  readonly elapsedMs: number;
  /** Restzeit bis zum Überdruck in ms. Bei Sieg die Reserve. */
  readonly remainingMs: number;
  readonly damage: number;
  readonly bossHp: number;
  readonly casts: number;
}

const OUTCOME_HEADLINES: Readonly<Record<number, string>> = {
  [OUTCOME_VICTORY]: 'REACTION CONTAINED',
  [OUTCOME_OVERPRESSURE]: 'OVERPRESSURE',
  [OUTCOME_WIPE]: 'RUN ABANDONED',
  [OUTCOME_DEATH]: 'VESSEL LOST',
};

function element(tag: string, className: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function button(label: string, onClick: () => void): HTMLButtonElement {
  const node = document.createElement('button');
  node.className = 'rota-button';
  node.type = 'button';
  node.textContent = label;
  node.addEventListener('click', onClick);
  return node;
}

function row(label: string): { root: HTMLElement; value: HTMLElement } {
  const root = element('div', 'rota-row');
  root.appendChild(element('span', 'rota-row-label', label));
  const value = element('span', 'rota-row-value', '—');
  root.appendChild(value);
  return { root, value };
}

export class Overlay {
  private readonly root: HTMLElement;
  private readonly prepare: HTMLElement;
  private readonly pause: HTMLElement;
  private readonly result: HTMLElement;
  private readonly resultHeadline: HTMLElement;
  private readonly resultRows: Record<string, HTMLElement> = {};

  private shownPhase = -1;
  private shownOutcome = -1;
  private paused = false;

  constructor(parent: HTMLElement, actions: OverlayActions) {
    this.root = element('div', 'rota-overlay');

    // --- Vorbereitung: keine Uhr, kein Zeitdruck (GAME-DESIGN 10) -----------
    this.prepare = element('div', 'rota-panel');
    this.prepare.appendChild(element('h1', 'rota-title', 'THE FIRST CRUCIBLE'));
    this.prepare.appendChild(
      element('p', 'rota-note', 'Prologue · no target until the run begins'),
    );
    this.prepare.appendChild(
      element(
        'p',
        'rota-note rota-note-dim',
        'Cooldowns run during preparation. The clock does not.',
      ),
    );
    this.prepare.appendChild(button('Ready', actions.onReady));
    this.prepare.appendChild(element('p', 'rota-hint', 'Enter'));
    this.root.appendChild(this.prepare);

    // --- Esc-Menü ----------------------------------------------------------
    this.pause = element('div', 'rota-panel');
    this.pause.appendChild(element('h2', 'rota-title', 'PAUSED'));
    this.pause.appendChild(button('Resume', actions.onResume));
    this.pause.appendChild(button('Wipe Run', actions.onWipe));
    this.pause.appendChild(
      element('p', 'rota-hint', 'A wipe counts as an ending. No penalty either way.'),
    );
    this.root.appendChild(this.pause);

    // --- Auswertung: dieselbe für alle vier Ausgänge ------------------------
    this.result = element('div', 'rota-panel');
    this.resultHeadline = element('h2', 'rota-title', '');
    this.result.appendChild(this.resultHeadline);
    for (const [key, label] of [
      ['time', 'Time'],
      ['remaining', 'Reserve'],
      ['damage', 'Damage'],
      ['progress', 'Progress'],
      ['casts', 'Casts'],
    ] as const) {
      const made = row(label);
      this.result.appendChild(made.root);
      this.resultRows[key] = made.value;
    }
    this.result.appendChild(button('Restart Run', actions.onRestart));
    this.result.appendChild(button('Back to Menu', actions.onBackToMenu));
    this.root.appendChild(this.result);

    parent.appendChild(this.root);
    this.hideAll();
  }

  dispose(): void {
    this.root.remove();
  }

  /** Ist das Esc-Menü offen? */
  get isPaused(): boolean {
    return this.paused;
  }

  setPaused(paused: boolean): void {
    if (this.paused === paused) return;
    this.paused = paused;
    this.shownPhase = -1; // erzwingt ein Neuzeichnen der Sichtbarkeit
  }

  /**
   * Sichtbarkeit an die Phase anpassen.
   *
   * Wird je Bild aufgerufen, fasst das HTML aber nur an, wenn sich etwas geändert
   * hat — sonst zahlte jedes Bild eine Layout-Neuberechnung (GAME-DESIGN 18.2).
   */
  update(phase: number, outcome: number, view: ResultView): void {
    if (phase === this.shownPhase && outcome === this.shownOutcome) return;
    this.shownPhase = phase;
    this.shownOutcome = outcome;

    this.hideAll();
    if (phase === PHASE_PREPARE) {
      this.show(this.prepare);
      return;
    }
    if (phase === PHASE_ENDED) {
      this.fillResult(view);
      this.show(this.result);
      return;
    }
    if (this.paused) this.show(this.pause);
  }

  private fillResult(view: ResultView): void {
    this.resultHeadline.textContent = OUTCOME_HEADLINES[view.outcome] ?? 'RUN ENDED';
    this.resultHeadline.classList.toggle('rota-title-win', view.outcome === OUTCOME_VICTORY);
    const set = (key: string, text: string): void => {
      const node = this.resultRows[key];
      if (node !== undefined) node.textContent = text;
    };
    set('time', `${(view.elapsedMs / 1000).toFixed(2)} s`);
    set('remaining', `${(view.remainingMs / 1000).toFixed(2)} s`);
    set('damage', String(view.damage));
    const progress = view.bossHp > 0 ? (100 * view.damage) / view.bossHp : 0;
    set('progress', `${Math.min(100, progress).toFixed(1)} %`);
    set('casts', String(view.casts));
  }

  private show(panel: HTMLElement): void {
    panel.classList.add('rota-visible');
    this.root.classList.add('rota-overlay-active');
  }

  private hideAll(): void {
    for (const panel of [this.prepare, this.pause, this.result]) {
      panel.classList.remove('rota-visible');
    }
    this.root.classList.remove('rota-overlay-active');
  }
}
