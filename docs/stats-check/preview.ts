/**
 * Prüfblatt für die Werte-Übersicht (Issue #188).
 *
 * WAS ES NICHT IST: der Nachweis für AC1 bis AC5. Die verlangen den
 * Auslieferungs-Bau an dem Ort, an dem die Übersicht später hängt — und der
 * ist offen (Meldung 1027). Hier wird belegt, dass die Ansicht aus den
 * gerechneten Zeilen das Richtige baut.
 *
 * Im Bestand gibt es keinen einzigen DOM-Prüffall; alle Ansichten werden über
 * ein Blatt wie dieses belegt. Die Prüfung läuft dem Muster nach statt eine
 * geteilte Konfiguration für einen einzelnen Fall zu ändern.
 */
import { NOTHING_WORN, type WornValues } from '../../src/gear/worn.js';
import { statOverview, STAT_CAPS, type StatRow } from '../../src/progression/stat-overview.js';
import { buildStatOverview } from '../../src/progression/stat-overview-view.js';
import '../../src/progression/stats.css';

const GRUND = { essence: 1000, vessel: 5000, ward: 20 };

function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  document.querySelector('#out')?.appendChild(el);
}

function zeilen(over: Partial<WornValues> = {}): StatRow[] {
  return statOverview(GRUND, { ...NOTHING_WORN, ...over });
}

function bild(over: Partial<WornValues> = {}): HTMLElement {
  return buildStatOverview(zeilen(over));
}

function feld(root: HTMLElement, id: string, klasse: string): string {
  return root.querySelector(`[data-stat="${id}"] .${klasse}`)?.textContent ?? '(fehlt)';
}

// --- Alle sechs Werte -------------------------------------------------------

line('Alle sechs Werte stehen im Bild');

const ohne = bild();
const kennungen = [...ohne.querySelectorAll('[data-stat]')].map(
  (n) => (n as HTMLElement).dataset.stat,
);
line(
  `Zeilen: ${kennungen.join(' · ')}`,
  kennungen.join(',') === 'essence,vessel,ward,catalysis,purity,yield',
);

// AC5-Gedanke: ohne Ausrüstung stehen die Grundwerte da, nicht leere Felder.
line(
  `Ohne Ausrüstung: ESSENCE ${feld(ohne, 'essence', 'stats-value')}, ` +
    `VESSEL ${feld(ohne, 'vessel', 'stats-value')}, YIELD ${feld(ohne, 'yield', 'stats-value')}`,
  feld(ohne, 'essence', 'stats-value') === '1000' && feld(ohne, 'yield', 'stats-value') === '150 %',
);
line(
  `…und keine einzige Warnung: ${ohne.querySelectorAll('[data-over-cap="yes"]').length}`,
  ohne.querySelectorAll('[data-over-cap="yes"]').length === 0,
);

// --- Die Warnung und ihr Gegenfall ------------------------------------------

line('');
line('Die Warnung erscheint genau dann, wenn sie soll');

const drueber = bild({ purity: 1020 });
line(
  `PURITY 102 %: ${feld(drueber, 'purity', 'stats-value')} ` +
    `${feld(drueber, 'purity', 'stats-cap')} ${feld(drueber, 'purity', 'stats-warning')}`,
  feld(drueber, 'purity', 'stats-warning') === '⚠ 2 % over the limit, no effect',
);

const drunter = bild({ purity: 800 });
line(
  `PURITY 80 %: Warnung ist ${feld(drunter, 'purity', 'stats-warning') === '' ? 'leer' : 'da'}`,
  feld(drunter, 'purity', 'stats-warning') === '',
);

// Der Fall, der nicht im Issue stand und dazwischen gehört.
const genau = bild({ purity: 1000 });
line(
  `PURITY GENAU 100 %: Warnung ist ${feld(genau, 'purity', 'stats-warning') === '' ? 'leer' : 'da'} — erreicht ist nicht überschritten`,
  feld(genau, 'purity', 'stats-warning') === '',
);

// --- CATALYSIS ohne Deckel --------------------------------------------------

line('');
line('CATALYSIS hat noch keinen Deckel (Issue #118)');

const cat = bild({ catalysis: 1020 });
line(
  `Wert ${feld(cat, 'catalysis', 'stats-value')}, Deckel-Feld ` +
    `${feld(cat, 'catalysis', 'stats-cap') === '' ? 'leer' : feld(cat, 'catalysis', 'stats-cap')}, ` +
    `Warnung ${feld(cat, 'catalysis', 'stats-warning') === '' ? 'leer' : 'da'}`,
  feld(cat, 'catalysis', 'stats-cap') === '' && feld(cat, 'catalysis', 'stats-warning') === '',
);
// Ein Strich sähe aus wie ein fehlender Wert. Hier fehlt nichts.
line(
  'Das Deckel-Feld bleibt leer statt einen Strich zu setzen',
  feld(cat, 'catalysis', 'stats-cap') === '',
);

// --- Der Deckel ist ein Datenwert -------------------------------------------

line('');
line('Was passiert, wenn #118 einen Deckel misst');

// DERSELBE Wert, einmal mit und einmal ohne Deckel in der Tabelle. Der
// Unterschied kommt ausschliesslich daher -- nicht daher, wie der Wert heisst.
line(
  `In der Tabelle steht heute: purity ${STAT_CAPS['purity']}, catalysis ${STAT_CAPS['catalysis']}`,
);
line(
  `Gleicher Wert 102 %: PURITY warnt (${feld(drueber, 'purity', 'stats-warning') !== ''}), ` +
    `CATALYSIS nicht (${feld(cat, 'catalysis', 'stats-warning') === ''})`,
  feld(drueber, 'purity', 'stats-warning') !== '' && feld(cat, 'catalysis', 'stats-warning') === '',
);
line(
  'Der Unterschied kommt AUSSCHLIESSLICH aus der Deckel-Tabelle, nicht aus dem Namen',
  zeilen({ purity: 1020 })[4]?.value === zeilen({ catalysis: 1020 })[3]?.value,
);

// --- Und wie es aussieht ----------------------------------------------------

const buehne = document.querySelector('#view');
for (const [titel, werte] of [
  ['Ohne Ausrüstung', {}],
  ['Mit Ausrüstung, PURITY über dem Deckel', { catalysis: 520, purity: 1020, yieldBonus: 300 }],
] as [string, Partial<WornValues>][]) {
  const kopf = document.createElement('p');
  kopf.className = 'sub';
  kopf.textContent = titel;
  buehne?.appendChild(kopf);
  buehne?.appendChild(bild(werte));
}
