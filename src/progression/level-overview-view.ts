/**
 * Die Aufstiegs-Übersicht als Bild (Issue #248, AC6).
 *
 * Setzt, was `level-overview.ts` gerechnet hat. Dieselbe Trennung wie bei der
 * Werte-Übersicht (#188): Hier steht keine Rechnung, dort kein DOM.
 *
 * Jede Zeile trägt ihre Kennung als Merkmal — so ist im Browser prüfbar, was
 * dasteht, ohne die Gestaltung zu befragen. Das ist nicht Bequemlichkeit: Die
 * Prüffälle laufen ohne Browser, und die einzige Prüfung, die diese Datei
 * erreicht, ist ein Blick auf die Seite.
 */
import type { LevelRow } from './level-overview.js';

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Die Übersicht aufbauen.
 *
 * **Keine eigene Überschrift.** Wer sie einsetzt, benennt den Bereich — im
 * Player Home tut das die Bereichs-Überschrift. Beide zusammen ergäben
 * dasselbe Wort zweimal untereinander; bei der Werte-Übersicht ist genau das
 * passiert und fiel erst im Bild auf.
 *
 * @param rows Die gerechneten Zeilen
 * @param fill Anteil bis zur nächsten Stufe, 0 bis 1 — für den Balken
 */
export function buildLevelOverview(rows: readonly LevelRow[], fill: number): HTMLElement {
  const root = element('section', 'levels');

  // Der Balken zuerst: Er ist das, was der Spieler sucht, wenn er hier
  // hereinschaut — „wie weit noch".
  const bar = element('div', 'levels-bar');
  bar.dataset.fill = String(Math.round(Math.min(Math.max(fill, 0), 1) * 100));
  const inner = element('div', 'levels-bar-fill');
  inner.style.width = `${Math.round(Math.min(Math.max(fill, 0), 1) * 100)}%`;
  bar.appendChild(inner);
  root.appendChild(bar);

  const grid = document.createElement('table');
  grid.className = 'levels-table';

  for (const row of rows) {
    const line = document.createElement('tr');
    line.className = 'levels-row';
    line.dataset.level = row.id;

    line.appendChild(element('td', 'levels-label', row.label));
    line.appendChild(element('td', 'levels-value', row.value));
    // Der Zusatz steht nur da, wo es einen gibt. Ein leeres Feld ist kein
    // fehlender Wert — es gibt schlicht nichts hinzuzufügen.
    line.appendChild(element('td', 'levels-note', row.note ?? ''));

    grid.appendChild(line);
  }

  root.appendChild(grid);
  return root;
}
