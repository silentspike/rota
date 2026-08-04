/**
 * Bestlauf-Vergleich und Verlauf als Ansicht (GAME-DESIGN 15.3, Issue #82).
 *
 * SVG UND NICHT CANVAS
 * --------------------
 * Der Kampf läuft auf der Zeichenfläche, alles danach in HTML (GAME-DESIGN
 * 18.1). Ein Diagramm im Menü ist „alles danach": Es muss nicht sechzigmal je
 * Sekunde neu entstehen, es soll skalieren und im Text durchsuchbar sein. SVG
 * ist dafür das richtige Werkzeug, und es kostet keine Bildrate, weil es
 * nicht im Bild liegt.
 *
 * ZWEI KURVEN ÜBEREINANDER, AUF DERSELBEN ZEITACHSE
 * -------------------------------------------------
 * Beide teilen sich eine x-Achse in **absoluten** Sekunden. Die kürzere endet
 * früher, sichtbar — das ist die halbe Aussage des Vergleichs und darf nicht
 * weggestreckt werden (siehe run-compare.ts).
 *
 * DER SCHNITT BEIM FASSUNGSWECHSEL IST PFLICHT
 * --------------------------------------------
 * Der Verlauf wird je Fassung als eigener Zug gezeichnet, mit sichtbarer
 * Trennlinie. Er ist die Bedingung, unter der der Schadenswert überhaupt
 * gespeichert werden darf: Ohne ihn sieht der Spieler beim Wechsel einen
 * Sprung und schreibt ihn sich selbst zu.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import type { ComparedMetric, RunCurve } from './run-compare.js';
import { historySegments } from './run-history.js';
import type { RunHistoryEntry } from './save-format.js';

const SVG_NS = 'http://www.w3.org/2000/svg';

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function svg(tag: string, attrs: Readonly<Record<string, string | number>>): SVGElement {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, String(value));
  return node;
}

/** Millisekunden als Sekunden mit einer Nachkommastelle. */
function seconds(ms: number): string {
  return `${(ms / 1000).toFixed(1)} s`;
}

/**
 * Wie eine Kennzahl geschrieben wird.
 *
 * EINE Stelle fuer Wert UND Differenz. Zwei getrennte Formatierer sind
 * auseinandergelaufen: Die Differenz eines Prozentwerts stand als blanke Zahl
 * da, waehrend die Werte daneben ihr Prozentzeichen trugen — „34,8 % gegen
 * 18,1 %, Differenz 17". Gemessen im Pruefblatt, bevor es jemand anders sah.
 */
function format(key: string, value: number): string {
  if (key === 'time' || key === 'idle' || key === 'missed') return seconds(value);
  if (key === 'progress') return `${value.toFixed(1)} %`;
  return Math.round(value).toLocaleString('en-US');
}

/**
 * Eine Differenz mit Vorzeichen — und mit dem Wissen, wohin „besser" zeigt.
 *
 * Das Vorzeichen allein sagt nichts: Bei der Kampfzeit ist weniger besser,
 * beim Schaden mehr. Deshalb steht die Richtung am Merkmal der Zeile, und die
 * Farbe liest sie dort ab, statt sie zu erraten.
 */
function deltaText(metric: ComparedMetric): string {
  if (metric.delta === null) return '—';
  const zahl = format(metric.key, Math.abs(metric.delta));
  if (metric.delta === 0) return `= ${zahl}`;
  return `${metric.delta > 0 ? '+' : '−'} ${zahl}`;
}

function valueText(metric: ComparedMetric, value: number): string {
  return format(metric.key, value);
}

/** Die Kennzahlen mit Differenz (AC1). */
export function buildComparisonView(metrics: readonly ComparedMetric[]): HTMLElement {
  const root = element('section', 'compare');
  root.appendChild(element('h3', 'compare-heading', 'AGAINST YOUR BEST'));

  const grid = document.createElement('table');
  grid.className = 'compare-table';

  const head = document.createElement('tr');
  for (const label of ['', 'THIS RUN', 'BEST', 'DIFFERENCE']) {
    head.appendChild(element('th', undefined, label));
  }
  grid.appendChild(head);

  for (const metric of metrics) {
    const row = document.createElement('tr');
    row.dataset.metric = metric.key;
    // Der Zustand steht am Merkmal und nicht in einer Farbklasse allein:
    // So ist er prüfbar, ohne die Gestaltung zu befragen.
    row.dataset.improved = metric.improved === null ? 'unknown' : String(metric.improved);
    row.appendChild(element('td', 'compare-label', metric.label));
    row.appendChild(element('td', undefined, valueText(metric, metric.current)));
    row.appendChild(
      element('td', undefined, metric.best === null ? '—' : valueText(metric, metric.best)),
    );
    const delta = element('td', 'compare-delta', deltaText(metric));
    row.appendChild(delta);
    grid.appendChild(row);
  }

  root.appendChild(grid);
  if (metrics.every((metric) => metric.best === null)) {
    // Kein Bestlauf: Das gehört gesagt, statt Striche zu zeigen und den
    // Spieler raten zu lassen, ob etwas kaputt ist.
    root.appendChild(
      element('p', 'compare-empty', 'First run on this boss — nothing to compare yet.'),
    );
  }
  return root;
}

/** Ein Punktzug aus Werten, auf die Fläche gerechnet. */
function polyline(
  points: readonly number[],
  stepMs: number,
  maxValue: number,
  maxMs: number,
  width: number,
  height: number,
  className: string,
): SVGElement {
  const koordinaten = points.map((value, index) => {
    const x = maxMs > 0 ? (index * stepMs * width) / maxMs : 0;
    const y = maxValue > 0 ? height - (value * height) / maxValue : height;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  return svg('polyline', { class: className, points: koordinaten.join(' '), fill: 'none' });
}

/**
 * Zwei Verläufe übereinander (AC1).
 *
 * `best` darf `null` sein — beim ersten Durchlauf gibt es keinen Bestlauf, und
 * dann steht dort eine Kurve statt zweier.
 */
export function buildCurveView(current: RunCurve, best: RunCurve | null): HTMLElement {
  const WIDTH = 640;
  const HEIGHT = 180;

  const root = element('section', 'curves');
  root.appendChild(element('h3', 'curves-heading', 'DAMAGE OVER TIME'));

  // Beide Kurven teilen sich EINE Achse. Jede auf ihr eigenes Maximum zu
  // beziehen hiesse, zwei Bilder nebeneinander zu zeigen und sie übereinander
  // zu nennen.
  const maxMs = Math.max(current.durationMs, best?.durationMs ?? 0);
  const maxValue = Math.max(current.points.at(-1) ?? 0, best?.points.at(-1) ?? 0);

  const flaeche = svg('svg', {
    class: 'curve-plot',
    viewBox: `0 0 ${WIDTH} ${HEIGHT}`,
    preserveAspectRatio: 'none',
    role: 'img',
  });
  flaeche.setAttribute('data-max-ms', String(maxMs));

  if (best !== null) {
    flaeche.appendChild(
      polyline(best.points, best.stepMs, maxValue, maxMs, WIDTH, HEIGHT, 'curve curve-best'),
    );
  }
  flaeche.appendChild(
    polyline(current.points, current.stepMs, maxValue, maxMs, WIDTH, HEIGHT, 'curve curve-current'),
  );

  root.appendChild(flaeche);

  const legende = element('p', 'curve-legend');
  legende.textContent =
    best === null
      ? `This run — ${seconds(current.durationMs)}`
      : `This run ${seconds(current.durationMs)} · Best ${seconds(best.durationMs)}`;
  root.appendChild(legende);
  return root;
}

/**
 * Der Verlauf über alle Durchläufe (AC4).
 *
 * Gezeichnet wird je Fassung ein eigener Zug, mit Trennlinie am Wechsel. Eine
 * durchgehende Linie über einen Balance-Wechsel hinweg wäre eine Aussage über
 * eine Entwicklung, die es so nicht gab.
 */
export function buildHistoryView(history: readonly RunHistoryEntry[]): HTMLElement {
  const WIDTH = 640;
  const HEIGHT = 160;

  const root = element('section', 'history');
  root.appendChild(element('h3', 'history-heading', 'OVER TIME'));

  if (history.length === 0) {
    root.appendChild(element('p', 'history-empty', 'No runs recorded yet.'));
    return root;
  }

  const segmente = historySegments(history);
  const maxDamage = Math.max(...history.map((entry) => entry.damage), 1);
  const schritt = history.length > 1 ? WIDTH / (history.length - 1) : WIDTH;

  const flaeche = svg('svg', {
    class: 'history-plot',
    viewBox: `0 0 ${WIDTH} ${HEIGHT}`,
    preserveAspectRatio: 'none',
    role: 'img',
  });
  flaeche.setAttribute('data-runs', String(history.length));
  flaeche.setAttribute('data-segments', String(segmente.length));

  for (const segment of segmente) {
    const punkte = segment.entries.map((entry, index) => {
      const x = (segment.startIndex + index) * schritt;
      const y = HEIGHT - (entry.damage * HEIGHT) / maxDamage;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    const zug = svg('polyline', {
      class: 'history-line',
      points: punkte.join(' '),
      fill: 'none',
    });
    zug.setAttribute('data-game-version', segment.gameVersion);
    flaeche.appendChild(zug);

    // Die Trennlinie steht VOR dem Abschnitt, nicht am Ende des vorigen: So
    // gibt es sie genau einmal je Wechsel und nie am Rand.
    if (segment.startIndex > 0) {
      const marke = svg('line', {
        class: 'history-cut',
        x1: (segment.startIndex * schritt - schritt / 2).toFixed(1),
        x2: (segment.startIndex * schritt - schritt / 2).toFixed(1),
        y1: 0,
        y2: HEIGHT,
      });
      marke.setAttribute('data-game-version', segment.gameVersion);
      flaeche.appendChild(marke);
    }
  }

  root.appendChild(flaeche);

  const fuss = element('p', 'history-legend');
  fuss.textContent =
    segmente.length > 1
      ? `${history.length} runs · ${segmente.length} game versions — the line breaks where the balance changed`
      : `${history.length} runs`;
  root.appendChild(fuss);
  return root;
}
