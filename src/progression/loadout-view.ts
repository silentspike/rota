/**
 * Die Leiste belegen — ein Bereich im Player Home (Issue #264).
 *
 * DIES IST NICHT DIE ZUSAMMENSTELLUNGS-ANSICHT AUS GAME-DESIGN 16
 * ---------------------------------------------------------------
 * Der Satz steht hier, damit ihn in drei Wochen niemand für ihre Umsetzung
 * hält und sie abhakt. Das ist derselbe Fall wie beim Player Home, das lange
 * nur in einer Überschrift stand: Ein Begriff, der benannt und nicht definiert
 * ist, wird zur Vermutung.
 *
 * | | |
 * | --- | --- |
 * | was er kann | Zauber auf Plätze legen, Plätze räumen |
 * | was fehlt | Gruppierung nach Schule, Live-Vorschau der Synergien, Schul-Mechanik-Zähler |
 * | wo das hingehört | **Issue #270** — die Ansicht aus 16, mit Live-Vorschau |
 *
 * **#270 ersetzt diesen Bereich, es gibt danach EINE Stelle zum Belegen.**
 * Zwei Stellen für dieselbe Sache wären dieselbe Dublette wie zwei Werte
 * namens REACTION (#247) — nur an der Oberfläche statt in den Daten.
 *
 * **Warum kein eigener Bildschirm:** Eine halbe Zusammenstellungs-Ansicht
 * müsste später ERSETZT werden, nicht erweitert — wer sie dann baut, fände
 * einen Bildschirm vor, der aussieht wie das Ziel und es nicht ist. Ein
 * Bereich im Player Home ist die kleinere Zusage: dieselbe Bauart wie
 * Progression und Stats, kein neuer Navigationsweg, und der Spieler lernt
 * keine Stelle, die drei Wochen später woanders liegt.
 *
 * EINE AUSWAHLLISTE JE PLATZ, KEIN ZIEHEN
 * ---------------------------------------
 * Ziehen bräuchte eine Zustandsmaschine (was ist gegriffen, wo schwebt es,
 * was passiert beim Loslassen daneben) und wäre ohne Maus nicht bedienbar.
 * Eine Liste je Platz ist in einem Zug lesbar, mit der Tastatur bedienbar und
 * im Browser prüfbar.
 *
 * Design-Gesetz 2 („nur Keybinds") gilt dem AUSLÖSEN im Kampf, nicht dem
 * Einrichten im Menü — das Player Home hat schon Knöpfe.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import type { SpellDef } from '../core/data.js';
import { spellAtSlot, type Bindings } from './bindings.js';
import { barSlotsAtLevel } from './unlocks.js';
import { isSpellUnlocked } from './unlocks.js';

/** Der Text für „kein Zauber auf diesem Platz". */
const LEER_LABEL = '— empty —';

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** Was die Ansicht braucht, um eine Leiste zu zeigen. */
export interface LoadoutViewOptions {
  readonly bindings: Bindings;
  readonly level: number;
  readonly spells: readonly SpellDef[];
  /**
   * Wird gerufen, wenn der Spieler einen Platz ändert.
   *
   * `spellId` ist `null`, wenn der Platz geräumt wird. Die Ansicht entscheidet
   * **nicht**, ob es erlaubt ist — das prüft `bindSpell`, und sie erfährt das
   * Ergebnis über den Rückgabewert.
   *
   * @returns ob die Änderung angenommen wurde
   */
  readonly onBind: (spellId: string | null, slot: number) => boolean;
}

/**
 * Die Leiste aufbauen.
 *
 * Jede Zeile trägt ihre Platznummer als Merkmal — so ist im Browser prüfbar,
 * was wo liegt, ohne die Gestaltung zu befragen.
 */
export function buildLoadout(options: LoadoutViewOptions): HTMLElement {
  const root = element('section', 'loadout');
  const plaetze = barSlotsAtLevel(options.level);

  // **Nur freigeschaltete Zauber zur Auswahl.** Einen gesperrten anzubieten und
  // dann abzulehnen wäre eine Einladung, die zurückgenommen wird; der Spieler
  // suchte den Grund bei sich.
  const waehlbar = options.spells.filter((spell) => isSpellUnlocked(spell, options.level));

  const grid = document.createElement('table');
  grid.className = 'loadout-table';

  for (let slot = 0; slot < plaetze; slot++) {
    const line = document.createElement('tr');
    line.className = 'loadout-row';
    line.dataset.slot = String(slot);

    line.appendChild(element('td', 'loadout-slot', `Slot ${slot + 1}`));

    const zelle = document.createElement('td');
    zelle.className = 'loadout-choice';
    const auswahl = document.createElement('select');
    auswahl.className = 'loadout-select';
    auswahl.dataset.slot = String(slot);

    const leer = document.createElement('option');
    leer.value = '';
    leer.textContent = LEER_LABEL;
    auswahl.appendChild(leer);

    for (const spell of waehlbar) {
      const eintrag = document.createElement('option');
      eintrag.value = spell.id;
      eintrag.textContent = spell.name;
      auswahl.appendChild(eintrag);
    }

    auswahl.value = spellAtSlot(options.bindings, slot) ?? '';
    auswahl.addEventListener('change', () => {
      const gewaehlt = auswahl.value === '' ? null : auswahl.value;
      // **Der Rückgabewert entscheidet, was dasteht.** Wird die Änderung
      // abgelehnt, springt die Liste auf ihren alten Wert zurück — sonst
      // zeigte sie etwas an, das im Kampf nicht gilt, und das ist genau der
      // Fall, der still bricht.
      if (!options.onBind(gewaehlt, slot)) {
        auswahl.value = spellAtSlot(options.bindings, slot) ?? '';
      }
    });

    zelle.appendChild(auswahl);
    line.appendChild(zelle);
    grid.appendChild(line);
  }

  root.appendChild(grid);

  // Was noch nicht offen ist, steht als Zeile da — sonst wirkt die Leiste
  // fertig, und der Spieler weiß nicht, dass sie wächst.
  const gesperrt = 12 - plaetze;
  if (gesperrt > 0) {
    root.appendChild(element('p', 'loadout-locked', `${gesperrt} more slots unlock as you level.`));
  }

  return root;
}
