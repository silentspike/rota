/**
 * Die Auswertung nach dem Kampf als Ansicht (GAME-DESIGN 15.3, Issue #81).
 *
 * HTML, nicht Zeichenflaeche: Der Kampf laeuft auf dem Canvas, Menues laufen in
 * HTML (GAME-DESIGN 18.1). Die Auswertung liegt damit ausserhalb des Bild-Pfads
 * und kostet im Kampf nichts.
 *
 * WAS DIE FUEHRENDE SPALTE IST - UND WARUM
 * ----------------------------------------
 * **Effizienz**, nicht Schadensanteil. Der Schadensanteil ist bei jedem
 * Durchlauf nahezu gleich; er misst die Zauber, nicht das Koennen. Er bleibt
 * als EINE Spalte stehen, weil er fuer Talent-Entscheidungen nuetzlich ist -
 * aber er fuehrt nicht (GAME-DESIGN 15.3).
 *
 * Die Zeilen stehen nach Effizienz sortiert, die schlechteste zuerst: Wer nach
 * dem hundertsten Versuch draufschaut, soll in drei Sekunden sehen, was er
 * liegen gelassen hat - und das steht dann oben.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import type { LossReport, LossSource, SpellEfficiency } from './losses.js';

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function percent(share: number): string {
  return `${(share * 100).toFixed(1)} %`;
}

function integer(value: number): string {
  return Math.round(value).toLocaleString('en-US');
}

/**
 * Die drei groessten Verlustquellen.
 *
 * Sie stehen oben, weil sie die Antwort auf die einzige Frage sind, die der
 * Spieler nach dem Kampf hat. Jede nennt, WAS liegen blieb - keine nennt, was
 * er stattdessen haette tun sollen (GAME-DESIGN 21.2).
 */
export function buildLossPanel(report: LossReport): HTMLElement {
  const panel = element('section', 'analysis-losses');
  panel.appendChild(element('h2', 'analysis-heading', 'WHAT YOU LEFT BEHIND'));

  if (report.top.length === 0) {
    panel.appendChild(
      element('p', 'analysis-empty', 'Nothing measurable was left on the table this run.'),
    );
    return panel;
  }

  const list = element('ol', 'analysis-loss-list');
  for (const source of report.top) {
    list.appendChild(buildLossRow(source));
  }
  panel.appendChild(list);

  panel.appendChild(
    element(
      'p',
      'analysis-total',
      `${integer(report.totalLost)} damage in total — ${percent(report.totalShare)} of what this run could have done.`,
    ),
  );
  return panel;
}

function buildLossRow(source: LossSource): HTMLElement {
  const row = element('li', 'analysis-loss');
  row.appendChild(element('span', 'analysis-loss-label', source.label));
  row.appendChild(element('span', 'analysis-loss-detail', source.detail));

  // Der Balken zeigt den Anteil am Gesamtpotenzial. Er ist Beiwerk - die Zahl
  // steht daneben, damit die Aussage nicht an der Balkenlaenge haengt.
  const bar = element('span', 'analysis-loss-bar');
  const fill = element('span', 'analysis-loss-fill');
  fill.style.width = `${Math.min(100, source.share * 100).toFixed(2)}%`;
  bar.appendChild(fill);
  row.appendChild(bar);
  return row;
}

/** Die Spalten der Effizienz-Tabelle, in Anzeigereihenfolge. */
export const EFFICIENCY_COLUMNS: readonly string[] = [
  'SPELL',
  'EFFICIENCY',
  'CASTS',
  'MISSED',
  'LOST DAMAGE',
  'DAMAGE SHARE',
];

/**
 * Die Effizienz-Tabelle (GAME-DESIGN 15.3).
 *
 * Reihenfolge der Spalten ist Aussage, nicht Geschmack: `EFFICIENCY` steht
 * direkt hinter dem Namen, `DAMAGE SHARE` ganz am Ende.
 */
export function buildEfficiencyTable(report: LossReport): HTMLElement {
  const wrap = element('section', 'analysis-efficiency');
  wrap.appendChild(element('h2', 'analysis-heading', 'EFFICIENCY'));

  const table = element('table', 'analysis-table') as HTMLTableElement;
  const head = element('tr');
  for (const [index, name] of EFFICIENCY_COLUMNS.entries()) {
    const cell = element('th', index === 1 ? 'analysis-lead' : undefined, name);
    head.appendChild(cell);
  }
  const thead = element('thead');
  thead.appendChild(head);
  table.appendChild(thead);

  const body = element('tbody');
  for (const spell of report.perSpell) {
    body.appendChild(buildEfficiencyRow(spell));
  }
  table.appendChild(body);
  wrap.appendChild(table);

  wrap.appendChild(
    element(
      'p',
      'analysis-note',
      'Sorted by efficiency, weakest first. Damage share is listed last on purpose: it barely moves between runs and measures the spells, not the play.',
    ),
  );
  return wrap;
}

function buildEfficiencyRow(spell: SpellEfficiency): HTMLElement {
  const row = element('tr');
  row.appendChild(element('td', 'analysis-name', spell.name));

  const lead = element('td', 'analysis-lead');
  const value = element('span', 'analysis-efficiency-value', percent(spell.efficiency));
  // Ueber 100 % ist kein Fehler, sondern eine Leistung - etwa durch einen
  // Ruecksetzer. Sie wird als solche gekennzeichnet statt gekappt.
  if (spell.efficiency > 1) value.classList.add('analysis-over');
  lead.appendChild(value);
  const meter = element('span', 'analysis-meter');
  const fill = element('span', 'analysis-meter-fill');
  fill.style.width = `${Math.min(100, spell.efficiency * 100).toFixed(2)}%`;
  meter.appendChild(fill);
  lead.appendChild(meter);
  row.appendChild(lead);

  row.appendChild(element('td', 'analysis-num', `${spell.casts} / ${spell.possibleCasts}`));
  row.appendChild(element('td', 'analysis-num', String(spell.missedCasts)));
  row.appendChild(element('td', 'analysis-num', integer(spell.lostDamage)));
  row.appendChild(element('td', 'analysis-num analysis-dim', percent(spell.damageShare)));
  return row;
}

/** Beide Teile zusammen - was nach dem Kampf auf dem Schirm steht. */
export function buildAnalysisView(report: LossReport): HTMLElement {
  const root = element('div', 'analysis-view');
  root.appendChild(buildLossPanel(report));
  root.appendChild(buildEfficiencyTable(report));
  return root;
}
