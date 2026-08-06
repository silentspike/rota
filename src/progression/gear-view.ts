/**
 * Die Ausrüst-Ansicht (Issue #453, GAME-DESIGN 13.4 und 16.1).
 *
 * WARUM SIE WIE DIE ZUSAMMENSTELLUNG GEBAUT IST
 * ---------------------------------------------
 * Ein `<select>` je Platz, kein Ziehen und Fallenlassen. Nicht aus Bequem-
 * lichkeit: `composition-view.ts` bedient die Zauberleiste genau so, und zwei
 * Bildschirme im selben Menü, die sich verschieden bedienen lassen, sind für
 * den Spieler zwei Systeme statt einem. Ausserdem ist ein `<select>` mit der
 * Tastatur erreichbar — bei einem Spiel, dessen erstes Design-Gesetz die
 * Tastatur ist, wäre alles andere schwer zu begründen.
 *
 * Anlegen und Ablegen sind damit **dieselbe** Bedienung: einen Eintrag wählen
 * oder „empty". Zwei getrennte Knöpfe wären zwei Wege zu einem Zustand.
 *
 * SIE SCHREIBT NICHTS SELBST
 * --------------------------
 * Wie `buildComposition` gibt sie ein Element zurück und meldet Wünsche über
 * einen Rückruf nach oben. Der Rückgabewert entscheidet, was danach dasteht —
 * wird ein Wunsch abgelehnt, springt die Liste zurück, statt einen Zustand zu
 * zeigen, den der nächste Kampf nicht kennt.
 *
 * DER VERGLEICH — SEIT #455 MIT EINGABE
 * -------------------------------------
 * GAME-DESIGN 13.6 verlangt, dass jeder Fund zeigt, ob er besser ist als das
 * Getragene. `compareItems` war fertig und hatte keine Eingabe; sie steht jetzt
 * in `src/loot/estimate.ts` und rechnet mit der **echten** Wertformel über die
 * ganze getragene Ausrüstung.
 *
 * **Was hier NICHT passiert: eine Platzhalter-Zahl.** Sie wäre schlimmer als
 * keine Angabe, weil der Spieler sie als Messung liest — davor warnt die
 * Auflage in `compare.ts`. Angezeigt wird deshalb nur, was gerechnet wurde,
 * und mit beiden Merkmalen der Näherung: dem Zeichen `~` und dem Zusatz in
 * Worten.
 *
 * Die Grenze der Näherung steht an der Rechenstelle (`estimate.ts`), nicht
 * hier: Sie sieht CATALYSIS nicht, weil Tempo über die Rotation wirkt.
 *
 * Alle Spieltexte englisch (Design-Gesetz 5).
 */
import { itemById, type OwnedItemLike } from '../gear/catalog.js';
import { compareBagItem } from '../loot/estimate.js';
import { equippedBySlot } from '../gear/equip.js';
import { SLOT_COUNT, SLOT_NAMES } from '../gear/slots.js';

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** Was in der Liste steht, wenn ein Platz leer ist. */
export const EMPTY_LABEL = '— empty —';

/** Die vier Qualitätsstufen als Wort (13.3). */
const QUALITY_NAMES: readonly string[] = ['common', 'rare', 'epic', 'legendary'];

/** Der Platzname mit grossem Anfangsbuchstaben — `SLOT_NAMES` hält sie klein. */
function slotLabel(slot: number): string {
  const name = SLOT_NAMES[slot] ?? `slot ${slot + 1}`;
  return name.charAt(0).toUpperCase() + name.slice(1);
}

/**
 * Wie ein Stück in der Liste heisst.
 *
 * Die Aufwertungsstufe gehört dazu: Zwei Stücke derselben Kennung sind sonst
 * nicht zu unterscheiden — und weil die Identität am Listenindex hängt
 * (`equip.ts`), wäre eine Liste mit zwei gleichen Einträgen eine Wahl ohne
 * erkennbaren Unterschied.
 */
export function itemLabel(item: OwnedItemLike): string {
  const def = itemById(item.itemId);
  if (def === undefined) return `${item.itemId} (unknown)`;
  const stufe = item.upgradeSteps > 0 ? ` +${item.upgradeSteps}` : '';
  const qualitaet = QUALITY_NAMES[def.quality] ?? 'common';
  return `${def.name}${stufe} · ilvl ${def.itemLevel} · ${qualitaet}`;
}

export interface GearViewOptions {
  /** Der gesamte Besitz — angelegt und im Beutel. */
  readonly items: readonly OwnedItemLike[];
  /**
   * Ein Stück auf einen Platz legen, oder den Platz leeren (`null`).
   *
   * @returns ob es geschehen ist. `false` lässt die Liste zurückspringen.
   */
  readonly onEquip: (index: number | null, slot: number) => boolean;
}

/**
 * Die Ansicht aufbauen.
 *
 * Jede Zeile trägt ihre Platznummer als `data-slot` und jeder Beutel-Eintrag
 * seinen Listenindex als `data-index` — dieselbe Linie wie bei der
 * Zusammenstellung: Im Browser ist prüfbar, was wo liegt, ohne die Gestaltung
 * zu befragen.
 */
export function buildGearView(options: GearViewOptions): HTMLElement {
  const root = element('section', 'gear');
  const belegung = equippedBySlot(options.items);

  // --- Links: die zehn Plätze ---------------------------------------------
  const links = element('div', 'gear-slots');
  links.appendChild(element('h3', 'gear-heading', 'Equipment'));
  const grid = document.createElement('table');
  grid.className = 'gear-table';

  for (let slot = 0; slot < SLOT_COUNT; slot++) {
    const zeile = document.createElement('tr');
    zeile.className = 'gear-row';
    zeile.dataset.slot = String(slot);
    zeile.appendChild(element('td', 'gear-slot-name', slotLabel(slot)));

    const zelle = document.createElement('td');
    zelle.className = 'gear-choice';
    const auswahl = document.createElement('select');
    auswahl.className = 'gear-select';
    auswahl.dataset.slot = String(slot);

    const leer = document.createElement('option');
    leer.value = '';
    leer.textContent = EMPTY_LABEL;
    auswahl.appendChild(leer);

    // **Nur Stücke, die auf DIESEN Platz gehören.** Der Platz steht am
    // Gegenstand (`def.slot`), nicht zur Wahl — ein Fokus auf dem Anker-Platz
    // wäre eine Bedienung, die `equip` gleich darauf ablehnte.
    options.items.forEach((besitz, index) => {
      const def = itemById(besitz.itemId);
      if (def === undefined || def.slot !== slot) return;
      const eintrag = document.createElement('option');
      eintrag.value = String(index);
      eintrag.textContent = itemLabel(besitz);
      auswahl.appendChild(eintrag);
    });

    const liegt = belegung[slot];
    auswahl.value = liegt === null || liegt === undefined ? '' : String(liegt);
    // Ein Platz ohne einziges passendes Stück bleibt sichtbar, aber stumm:
    // Zehn Plätze zu zeigen und neun davon zu verschweigen, nähme dem Spieler
    // das Bild davon, was es überhaupt zu füllen gibt.
    auswahl.disabled = auswahl.options.length <= 1;

    auswahl.addEventListener('change', () => {
      const gewaehlt = auswahl.value === '' ? null : Number(auswahl.value);
      if (!options.onEquip(gewaehlt, slot)) {
        auswahl.value = liegt === null || liegt === undefined ? '' : String(liegt);
      }
    });

    zelle.appendChild(auswahl);
    zeile.appendChild(zelle);
    grid.appendChild(zeile);
  }
  links.appendChild(grid);

  // --- Rechts: der Beutel --------------------------------------------------
  const rechts = element('div', 'gear-bag');
  rechts.appendChild(element('h3', 'gear-heading', 'Bag'));

  const imBeutel = options.items
    .map((besitz, index) => ({ besitz, index }))
    .filter(({ besitz }) => besitz.equippedSlot === null);

  if (imBeutel.length === 0) {
    // **Der leere Beutel bekommt einen Satz, keine Leere.** Ein Bereich ohne
    // Inhalt sieht aus wie ein Fehler; er soll aussehen wie ein Zustand.
    rechts.appendChild(element('p', 'gear-bag-empty', 'Nothing here. Defeat bosses to find gear.'));
  }
  for (const { besitz, index } of imBeutel) {
    const zeile = element('p', 'gear-bag-item', itemLabel(besitz));
    zeile.dataset.index = String(index);
    const def = itemById(besitz.itemId);
    if (def !== undefined) zeile.dataset.slot = String(def.slot);

    // Der Vergleich (13.6, Issue #455). **Beide Merkmale der Näherung stehen
    // da** — das Zeichen im Text und der Zusatz in Worten als `title`; ein
    // Zeichen allein wird übersehen, ein Wort allein überlesen (#114).
    const urteil = compareBagItem(options.items, index);
    if (urteil !== undefined) {
      const marke = element('span', 'gear-bag-compare', urteil.text);
      marke.title = urteil.caveat;
      // Die Richtung steht als Merkmal daneben, nicht nur als Farbe: Wer
      // Farben nicht unterscheidet, liest sonst nur eine Zahl.
      marke.dataset.verdict =
        urteil.verdict > 0 ? 'better' : urteil.verdict < 0 ? 'worse' : 'equal';
      zeile.appendChild(document.createTextNode(' '));
      zeile.appendChild(marke);
    }
    rechts.appendChild(zeile);
  }

  root.appendChild(links);
  root.appendChild(rechts);
  return root;
}
