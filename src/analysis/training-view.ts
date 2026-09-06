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

const IDLE_LABEL = 'IDLE';
const MISSED_LABEL = 'MISSED CASTS';
const SYNERGY_LABEL = 'SYNERGY';

function idleText(metrics: LiveMetrics): string {
  return `${seconds(metrics.lostTimeMs)} · ${percent(metrics.lostTimeShare)}`;
}

function synergyText(metrics: LiveMetrics): string {
  return metrics.synergyAvailable > 0
    ? `${metrics.synergyUsed} / ${metrics.synergyAvailable}`
    : 'n/a';
}

/** The production HUD draws these in its existing Canvas pass (§18.2). */
export function drawLiveMetrics(
  ctx: CanvasRenderingContext2D,
  metrics: LiveMetrics,
  left: number,
  right: number,
  top: number,
): void {
  ctx.font = '24px ui-monospace, "DejaVu Sans Mono", monospace';
  ctx.textAlign = 'left';
  ctx.fillStyle = '#61616e';
  ctx.fillText(IDLE_LABEL, left, top);
  ctx.fillText(MISSED_LABEL, left, top + 44);
  ctx.fillText(SYNERGY_LABEL, left, top + 88);
  ctx.textAlign = 'right';
  ctx.fillStyle = '#d5d5de';
  ctx.fillText(idleText(metrics), right, top);
  ctx.fillText(String(metrics.missedCasts), right, top + 44);
  ctx.fillText(synergyText(metrics), right, top + 88);
  ctx.textAlign = 'left';
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
      idle: make('idle', IDLE_LABEL),
      missed: make('missed', MISSED_LABEL),
      synergy: make('synergy', SYNERGY_LABEL),
    };
  }

  update(metrics: LiveMetrics): void {
    const idle = this.readouts['idle'];
    if (idle !== undefined) {
      idle.value.textContent = idleText(metrics);
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
      synergy.value.textContent = synergyText(metrics);
    }
  }
}
