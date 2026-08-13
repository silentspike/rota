/**
 * Die Werte-Übersicht als Ansicht (GAME-DESIGN 6.7, Issue #188).
 *
 * SIE WEISS NICHT, WO SIE HÄNGT
 * -----------------------------
 * `buildStatOverview` gibt ein Element zurück und hängt es nirgends ein. Der
 * Ort ist zum Zeitpunkt dieser Datei **offen**: Das Issue verweist auf das
 * „Player Home" (GAME-DESIGN 19.2), und das ist der Werkzeug-Bereich, den 19.3
 * im Auslieferungs-Bau vollständig herausschneidet — während die Nachweise
 * genau dort verlangt werden. Die Frage liegt beim Auftraggeber.
 *
 * Eine Ansicht, die sich selbst einhängt, müsste beim Umzug angefasst werden.
 * Diese hier nicht: Wer sie zeigt, entscheidet der Aufrufer.
 *
 * SIE RECHNET NICHT UND FORMULIERT NICHT
 * --------------------------------------
 * Zahlen und Warnsatz kommen fertig aus `stat-overview.ts`. Zwei Stellen, die
 * aus denselben Werten einen Satz machen, laufen beim nächsten Nachziehen
 * auseinander — dieselbe Regel wie bei der Fortschrittsmarke und der laufenden
 * Anzeige im Trainingsraum.
 *
 * KEIN BALKEN
 * -----------
 * „Zahl plus Warnhinweis, kein eigener Balken" (GAME-DESIGN 6.7). Ein Balken
 * bräuchte ein Maximum, und für vier der sechs Werte gibt es keins.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import { withPlainText } from '../analysis/help.js';
import { formatValue, type StatRow } from './stat-overview.js';

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Die Übersicht als Element.
 *
 * Jede Zeile trägt ihre Kennung als Merkmal — so ist im Browser prüfbar, was
 * dasteht, ohne die Gestaltung zu befragen.
 */
export function buildStatOverview(rows: readonly StatRow[]): HTMLElement {
  // KEINE eigene Ueberschrift. Wer die Uebersicht einsetzt, benennt den
  // Bereich -- im Player Home tut das die Bereichs-Ueberschrift. Beide
  // zusammen ergaben zweimal 'STATS' untereinander; im Bild gesehen, nicht
  // im Code.
  const root = element('section', 'stats');

  const grid = document.createElement('table');
  grid.className = 'stats-table';

  for (const row of rows) {
    const line = document.createElement('tr');
    line.className = 'stats-row';
    line.dataset.stat = row.id;
    // Ob gewarnt wird, steht am Merkmal und nicht allein in einer Farbe: So
    // lässt es sich messen, und wer keine Farben unterscheidet, sieht es am
    // Text daneben.
    line.dataset.overCap = row.overCap === null ? 'no' : 'yes';

    // **Der Fachbegriff bekommt seinen Klartext daneben** (AC3 von #811,
    // GAME-DESIGN 17.3): „ESSENCE (Strength of the substance)". Kennt
    // `withPlainText` den Begriff nicht, bleibt er unverändert stehen — eine
    // erfundene Erklärung wäre schlimmer als keine.
    //
    // Und dieselbe Zelle trägt die Kennung für das Erklärfenster (AC2). Beides
    // gehört an den Namen, nicht an die Zahl: Wer nicht weiß, was ESSENCE ist,
    // sieht auf das Wort.
    const bezeichnung = element('td', 'stats-label', withPlainText(row.label));
    bezeichnung.dataset.help = row.id;
    line.appendChild(bezeichnung);
    line.appendChild(element('td', 'stats-value', formatValue(row.value, row.unit)));

    // Der Deckel steht nur da, wo es einen gibt. Ein leeres Feld mit Strich
    // sähe aus wie ein fehlender Wert — hier fehlt aber nichts, es gibt
    // schlicht keinen Deckel (CATALYSIS bis Issue #118).
    line.appendChild(
      element('td', 'stats-cap', row.cap === null ? '' : `/ ${formatValue(row.cap, row.unit)}`),
    );

    const warnung = element('td', 'stats-warning');
    if (row.warning !== null) {
      // Das Zeichen steht im Design (6.7). Es trägt die Aussage nicht allein —
      // der Satz daneben tut es.
      warnung.textContent = `⚠ ${row.warning}`;
    }
    line.appendChild(warnung);

    grid.appendChild(line);
  }

  root.appendChild(grid);
  return root;
}
