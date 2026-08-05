/**
 * Die Auswertung als Ansicht (GAME-DESIGN 15.2 und 15.3, Issue #81).
 *
 * HTML, nicht Zeichenfläche: Der Kampf läuft auf dem Canvas, alles danach in
 * HTML (GAME-DESIGN 18.1).
 *
 * DIE SPALTENFOLGE IST DIE AUSSAGE
 * --------------------------------
 * EFFICIENCY steht vorn, DAMAGE SHARE hinten (AC4). Der Schadensanteil ist bei
 * jedem Durchlauf nahezu gleich — er misst die Zauber, nicht das Können. Er
 * bleibt trotzdem, weil er für Talent-Entscheidungen nützlich ist; er darf nur
 * nicht führen.
 *
 * Die Reihenfolge kommt aus `EFFICIENCY_COLUMNS` und wird hier nicht noch
 * einmal hingeschrieben: Zwei Stellen, die sie festlegen, laufen beim nächsten
 * Umbau auseinander.
 *
 * WAS DIESE ANSICHT NIE SAGT
 * --------------------------
 * Was der Spieler stattdessen hätte tun sollen. Sie benennt, was liegen blieb —
 * keine Reihenfolge, keine Empfehlung (GAME-DESIGN 21.2).
 *
 * Und sie erfindet nichts: Wo eine Größe heute nicht messbar ist, steht das da,
 * statt einer Null. Eine Null sähe aus wie „kein Verlust".
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import type { EfficiencyTable, SpellEfficiencyRow } from './efficiency.js';
import type { LossKind, LossReport } from './loss-sources.js';

/** Klartext für die Lücken-Meldung — englisch, steht im Spiel. */
const LOSS_LABEL: Readonly<Record<LossKind, string>> = {
  missedCasts: 'Missed casts',
  idle: 'Idle time',
  missedSynergies: 'Missed synergies',
  lostDotTicks: 'Lost damage-over-time ticks',
  missedInterrupts: 'Missed interrupts',
  // Nur bei einem Boss mit umgekehrtem Ziel. Dort ist der eigene Schaden der
  // Fehler, nicht der Fortschritt (Issue #256).
  selfInflicted: 'Damage you dealt to the ring',
};

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function percent(share: number | null): string {
  return share === null ? 'n/a' : `${(share * 100).toFixed(0)} %`;
}

function seconds(ms: number): string {
  return `${(ms / 1000).toFixed(1)} s`;
}

/** Was in einer Zeile unter welcher Spalte steht. */
function cellsFor(row: SpellEfficiencyRow, table: EfficiencyTable): readonly string[] {
  return [
    percent(row.efficiency),
    `${row.casts} / ${row.opportunities}`,
    seconds(row.idleMs),
    // Die Synergie-Quote ist eine Zahl des Durchlaufs, nicht des Zaubers. Sie
    // je Zeile zu wiederholen gäbe vor, sie ließe sich einem Zauber zuordnen.
    percent(table.synergyRate),
    percent(row.damageShare),
  ];
}

/** Die Effizienz-Tabelle. */
export function buildEfficiencyView(table: EfficiencyTable): HTMLElement {
  const root = element('section', 'efficiency');
  root.appendChild(element('h3', 'efficiency-heading', 'EFFICIENCY'));

  const summary = element('p', 'efficiency-overall');
  summary.dataset.overall = String(table.overallEfficiency ?? '');
  summary.textContent =
    `${percent(table.overallEfficiency)} — ` +
    `${table.rows.reduce((sum, row) => sum + row.casts, 0)} casts in ${table.castsMax} windows · ` +
    // Ausdruecklich „with nothing ready": Die IDLE-Spalte der Zeilen meint
    // etwas anderes — dort lag DIESER Zauber bereit. Beides „idle" zu nennen
    // stellte zwei verschiedene Fehler unter denselben Namen.
    `${seconds(table.idleMs)} with nothing ready`;
  root.appendChild(summary);

  const grid = document.createElement('table');
  grid.className = 'efficiency-table';

  const head = document.createElement('tr');
  head.appendChild(element('th', 'efficiency-spell', 'SPELL'));
  for (const column of table.columns) head.appendChild(element('th', 'efficiency-column', column));
  grid.appendChild(head);

  for (const row of table.rows) {
    const line = document.createElement('tr');
    line.dataset.spellId = row.spellId;
    line.appendChild(element('td', 'efficiency-spell', row.spellId.toUpperCase()));
    for (const cell of cellsFor(row, table)) line.appendChild(element('td', undefined, cell));
    grid.appendChild(line);
  }

  root.appendChild(grid);
  return root;
}

/**
 * Die Verlustquellen — die drei größten hervorgehoben.
 *
 * Die weiteren stehen darunter, nicht unter den Tisch: Ein Spieler, der vier
 * Fehler gemacht hat, soll nicht drei sehen und den vierten wiederholen.
 */
export function buildLossView(report: LossReport): HTMLElement {
  const root = element('section', 'losses');
  root.appendChild(element('h3', 'losses-heading', 'LEFT ON THE TABLE'));

  if (report.sources.length === 0) {
    root.appendChild(element('p', 'losses-none', 'Nothing measurable was left behind.'));
  } else {
    const total = element('p', 'losses-total');
    total.textContent = `${percent(report.totalShare)} of this run's potential`;
    root.appendChild(total);

    const list = element('ul', 'losses-list');
    for (const source of report.sources) {
      const item = element('li', report.top.includes(source) ? 'loss loss-top' : 'loss');
      item.dataset.lossKind = source.kind;
      item.appendChild(element('span', 'loss-label', source.label));
      item.appendChild(element('span', 'loss-share', percent(source.share)));
      item.appendChild(element('span', 'loss-detail', source.detail));
      list.appendChild(item);
    }
    root.appendChild(list);
  }

  // Die verpassten Unterbrechungen stehen GETRENNT, mit eigener Zahl und ohne
  // Prozentsatz (GAME-DESIGN 15.2, entschieden am 2026-08-04). Sie messen
  // eingesteckten statt ausgeteilten Schaden; in derselben Prozentrechnung
  // verglichen sie Unvergleichbares — und die Zahl gewönne immer.
  if (report.interrupts !== null) {
    const box = element('div', 'losses-interrupts');
    box.dataset.missed = String(report.interrupts.missed);
    box.appendChild(element('span', 'loss-label', LOSS_LABEL.missedInterrupts));
    box.appendChild(element('span', 'loss-detail', report.interrupts.detail));
    root.appendChild(box);
  }

  // Die Lücke gehört sichtbar. Sie wegzulassen hieße, eine vollständige
  // Rechnung vorzutäuschen (Issue #151).
  if (report.unavailable.length > 0) {
    const gap = element('p', 'losses-unavailable');
    gap.dataset.unavailable = report.unavailable.join(',');
    gap.textContent = `Not measured in this run: ${report.unavailable
      .map((kind) => LOSS_LABEL[kind])
      .join(', ')}.`;
    root.appendChild(gap);
  }
  return root;
}
