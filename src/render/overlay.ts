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
import type { RunSummary } from '../core/summary.js';
import { classOfStat, formatStat, type StatLine } from '../gear/format.js';

export interface OverlayActions {
  onReady(): void;
  onResume(): void;
  onWipe(): void;
  onRestart(): void;
  onBackToMenu(): void;
}

/** Was die Auswertung anzeigt. */
export interface ResultView {
  readonly outcome: number;
  /** Verstrichene Kampfzeit in ms. */
  readonly elapsedMs: number;
  /** Restzeit bis zum Überdruck in ms. Bei Sieg die Reserve. */
  readonly remainingMs: number;
  readonly damage: number;
  readonly bossHp: number;
  readonly casts: number;
  /** Die Kurzfassung mit den drei Kennzahlen (GAME-DESIGN 15.1). */
  readonly summary: RunSummary | null;
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
  private readonly detail: HTMLElement;
  /** Die Werte-Karte des Vorbereitungs-Bildschirms (Issue E6-1, AC4). */
  private readonly gearCard: HTMLElement;
  private readonly analysisButton: HTMLButtonElement;
  private detailOpen = false;

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
    // Die Werte-Karte. **Leer, solange es keine Gegenstände gibt** — der
    // Katalog steht noch nicht (GAME-DESIGN 22, vor E6). Sie zeigt sich erst,
    // wenn etwas anzuzeigen ist, und schiebt bis dahin nichts.
    this.gearCard = element('div', 'rota-gear-card');
    this.gearCard.hidden = true;
    this.prepare.appendChild(this.gearCard);

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
      ['score', 'Score'],
    ] as const) {
      const made = row(label);
      this.result.appendChild(made.root);
      this.resultRows[key] = made.value;
    }

    // Die drei Kennzahlen aus GAME-DESIGN 15.1.
    this.result.appendChild(element('div', 'rota-divider'));
    for (const [key, label] of [
      ['idleWorst', 'Idle · worst spell'],
      ['lostTime', 'Time lost'],
      ['synergy', 'Synergy yield'],
    ] as const) {
      const made = row(label);
      this.result.appendChild(made.root);
      this.resultRows[key] = made.value;
    }

    // "View Analysis" klappt den Leerlauf je Zauber auf. Die ausführliche
    // Auswertung mit Effizienz-Tabelle und Bestlauf-Vergleich ist Epic E7.
    this.detail = element('div', 'rota-detail');
    this.result.appendChild(this.detail);

    this.result.appendChild(button('Retry', actions.onRestart));
    this.analysisButton = button('View Analysis', () => {
      this.toggleDetail();
    });
    this.result.appendChild(this.analysisButton);
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

    this.detailOpen = false;
    this.detail.classList.remove('rota-visible');
    this.analysisButton.textContent = 'View Analysis';
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
    // Die Punkteformel ist noch nicht ausgearbeitet (GAME-DESIGN 22, vor Epic
    // E5). Eine erfundene Zahl wäre schlimmer als ein ehrlicher Strich.
    set('score', '—');

    const summary = view.summary;
    if (summary === null) return;
    const worst = summary.idlePerSpell[0];
    set(
      'idleWorst',
      worst === undefined
        ? '—'
        : `${worst.spellId} · ${(worst.idleMs / 1000).toFixed(1)} s (${(worst.share * 100).toFixed(0)} %)`,
    );
    set(
      'lostTime',
      `${(summary.lostTimeMs / 1000).toFixed(1)} s (${(summary.lostTimeShare * 100).toFixed(0)} %)`,
    );
    set(
      'synergy',
      summary.synergyAvailable === 0
        ? 'none available'
        : `${summary.synergyUsed} of ${summary.synergyAvailable}`,
    );
    this.fillDetail(summary);
  }

  /** Der Leerlauf je Zauber, nach Verlust sortiert (GAME-DESIGN 15.1). */
  private fillDetail(summary: RunSummary): void {
    this.detail.replaceChildren();
    this.detail.appendChild(element('p', 'rota-hint', 'Idle time per spell, worst first'));
    for (const entry of summary.idlePerSpell) {
      const line = element('div', 'rota-row');
      line.appendChild(
        element('span', 'rota-row-label', `${entry.spellId} · ${entry.casts} casts`),
      );
      line.appendChild(
        element(
          'span',
          'rota-row-value',
          `${(entry.idleMs / 1000).toFixed(1)} s (${(entry.share * 100).toFixed(0)} %)`,
        ),
      );
      this.detail.appendChild(line);
    }
    const log = element('div', 'rota-row');
    log.appendChild(element('span', 'rota-row-label', 'Log entries'));
    log.appendChild(
      element(
        'span',
        'rota-row-value',
        `${summary.logEntries} / ${summary.logCapacity}${summary.logOverflowed ? ` · ${summary.logDropped} dropped` : ''}`,
      ),
    );
    this.detail.appendChild(log);
  }

  private toggleDetail(): void {
    this.detailOpen = !this.detailOpen;
    this.detail.classList.toggle('rota-visible', this.detailOpen);
    this.analysisButton.textContent = this.detailOpen ? 'Hide Analysis' : 'View Analysis';
  }

  /**
   * Die Werte eines Gegenstands anzeigen (Issue E6-1, AC4).
   *
   * **Additive und multiplikative Werte sind an zwei Merkmalen zu
   * unterscheiden:** an der Schreibweise (`+20%` gegen `x1.20`) und an der
   * Farbe. Zwei Merkmale, weil eine Farbe allein nicht trägt — etwa jeder
   * zwölfte Mann sieht Rot und Grün nicht auseinander.
   *
   * Wird **nur bei einer Änderung** angefasst, nie je Bild: Jede Änderung am
   * HTML löst eine Neuberechnung des Layouts aus (GAME-DESIGN 18.2).
   */
  showGear(title: string, lines: readonly StatLine[]): void {
    this.gearCard.replaceChildren();
    if (lines.length === 0) {
      this.gearCard.hidden = true;
      return;
    }
    this.gearCard.appendChild(element('h3', 'rota-gear-title', title));
    for (const line of lines) {
      const root = element('div', 'rota-row');
      root.appendChild(element('span', 'rota-row-label', line.label));
      root.appendChild(
        element('span', `rota-row-value ${classOfStat(line.kind)}`, formatStat(line)),
      );
      this.gearCard.appendChild(root);
    }
    this.gearCard.hidden = false;
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
