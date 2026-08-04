/**
 * Die laufende Anzeige im Trainingsraum (GAME-DESIGN 15.5, Issue #84).
 *
 * DAS IST DER KERN DES ISSUES, NICHT DIE PUPPE
 * --------------------------------------------
 * Ohne sie ist der Trainingsraum ein Boss ohne Zeitlimit. Mit ihr sieht der
 * Spieler im selben Moment, in dem er einen Fehler macht, dass er ihn gemacht
 * hat.
 *
 * **Und es ist der einzige Ort im Spiel, an dem das erlaubt ist.** Im Kampf
 * waere dieselbe Anzeige eine Hilfe, die dem Spieler die Aufgabe abnimmt
 * (GAME-DESIGN 21.2). Hier ist sie ein Uebungswerkzeug.
 *
 * WAS SIE NICHT TUT
 * -----------------
 * Sie RECHNET NICHT. Jede Zahl kommt aus einer Quelle, die sie uebergeben
 * bekommt. Der Grund ist die offene Definition von "verpasste Wirkungen" in
 * Issue #81: Wuerde sie hier nachgebaut, gaebe es zwei Wahrheiten ueber
 * dieselbe Zahl — und die zweite wird beim Nachziehen vergessen.
 *
 * Und sie sagt nie, WAS der Spieler stattdessen tun soll. Sie zeigt Zahlen,
 * keine Ratschlaege.
 */
import type { LiveMetrics } from './training.js';

/**
 * Woher die Zahlen kommen.
 *
 * Ein Aufruf je Bild. Was dahintersteckt — heute `Session`, spaeter die
 * Verlustrechnung aus #81 — ist dieser Datei gleichgueltig, und das ist der
 * Zweck.
 */
export type MetricsSource = () => LiveMetrics;

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function seconds(ms: number): string {
  return `${(ms / 1000).toFixed(1)} s`;
}

function percent(share: number): string {
  return `${(share * 100).toFixed(0)} %`;
}

/** Eine Kennzahl mit Beschriftung — englisch, steht im Spiel. */
interface Readout {
  readonly label: string;
  readonly value: HTMLElement;
}

/**
 * Die drei Kennzahlen als Anzeige.
 *
 * `update` bei jedem Bild aufrufen. Sie erzeugt dabei nichts: Die Knoten
 * stehen, nur ihr Text wechselt (GAME-DESIGN 18.2).
 */
export class LiveMetricsView {
  readonly root: HTMLElement;
  private readonly readouts: Readonly<Record<string, Readout>>;

  constructor() {
    this.root = element('div', 'training-metrics');
    const make = (key: string, label: string): Readout => {
      const box = element('div', 'training-metric');
      box.dataset.metric = key;
      box.appendChild(element('span', 'training-metric-label', label));
      const value = element('span', 'training-metric-value', '—');
      box.appendChild(value);
      this.root.appendChild(box);
      return { label, value };
    };
    this.readouts = {
      // Die drei aus GAME-DESIGN 15.1, in derselben Reihenfolge.
      idle: make('idle', 'IDLE'),
      missed: make('missed', 'MISSED CASTS'),
      synergy: make('synergy', 'SYNERGY'),
    };
  }

  update(metrics: LiveMetrics): void {
    const idle = this.readouts['idle'];
    if (idle !== undefined) {
      idle.value.textContent = `${seconds(metrics.lostTimeMs)} · ${percent(metrics.lostTimeShare)}`;
    }
    const missed = this.readouts['missed'];
    if (missed !== undefined) {
      missed.value.textContent = String(metrics.missedCasts);
    }
    const synergy = this.readouts['synergy'];
    if (synergy !== undefined) {
      // Ohne moegliche Synergien bleibt die Ausbeute unbestimmt. Hier eine
      // Zahl zu erfinden hiesse, dem Spieler etwas vorzurechnen — in Schritt 1
      // gibt es nur eine Schule und damit keine schulübergreifenden Fenster.
      synergy.value.textContent =
        metrics.synergyAvailable > 0
          ? `${metrics.synergyUsed} / ${metrics.synergyAvailable}`
          : 'n/a';
    }
  }
}

/**
 * Die Anzeige an eine Quelle haengen.
 *
 * Gibt eine Funktion zum Loesen zurueck — ohne sie liefe die Schleife nach dem
 * Abbrechen weiter und liesse eine tote Anzeige mitlaufen.
 */
export function runLiveMetrics(
  view: LiveMetricsView,
  source: MetricsSource,
  schedule: (callback: () => void) => number = (cb) => window.requestAnimationFrame(cb),
  cancel: (handle: number) => void = (h) => {
    window.cancelAnimationFrame(h);
  },
): () => void {
  let handle = 0;
  let stopped = false;
  const tick = (): void => {
    if (stopped) return;
    view.update(source());
    handle = schedule(tick);
  };
  handle = schedule(tick);
  return () => {
    stopped = true;
    cancel(handle);
  };
}
