/**
 * Die Zusammenstellungs-Ansicht (GAME-DESIGN 16, Issue #270).
 *
 * > Links die eigenen Zauber **nach Schule gruppiert**, rechts die Leiste.
 * >
 * > **Live-Vorschau** zeigt beim Bestücken die entstehenden Synergien und wie
 * > viele Zauber je Schul-Mechanik einzahlen — man erkennt sofort, ob die
 * > Hitze-Mechanik mit nur einem Zauber trägt.
 * >
 * > **Keine Empfehlungen** vom Spiel: Das Zusammenstellen ist eine der beiden
 * > Kernaufgaben.
 *
 * SIE ERSETZT DEN BEREICH AUS #264, sie steht nicht daneben. Zwei Stellen zum
 * Belegen wären dieselbe Dublette wie zwei Werte namens REACTION (#247): Der
 * Spieler wüsste nicht, welche gilt, und beide müssten gepflegt werden.
 *
 * WAS HIER NIEMALS PASSIERT
 * -------------------------
 * Eine Reihenfolge nach Stärke, eine Markierung „bester Zauber", eine Warnung
 * bei einer schwachen Zusammenstellung. Die Reihenfolge ist die der Daten —
 * innerhalb der Gruppe und zwischen den Gruppen. Ein Prüffall misst das als
 * WIRKUNG: Wird ein Zauber zehnmal stärker, ändert sich hier nichts.
 *
 * Die Synergie-Vorschau fehlt und ist im Issue als blockiert vermerkt: Es gibt
 * heute keine einzige Synergie-Beziehung in `data/**`, und `session.ts` setzt
 * beide Zahlen hart auf 0. Eine Anzeige dafür wäre eine Anzeige ohne Inhalt.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import type { SpellDef } from '../core/data.js';
import { spellAtSlot, type Bindings } from './bindings.js';
import { barSlotsAtLevel, isSpellUnlocked } from './unlocks.js';
import { colourOfSchool, NO_SCHOOL_LABEL } from './schools.js';
import { feedsMechanic, MECHANICS, spendsMechanic } from '../combat/schools/indicators.js';

/** Der Text für „kein Zauber auf diesem Platz". */
const LEER_LABEL = '— empty —';

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** Was die Ansicht braucht. */
export interface CompositionViewOptions {
  readonly bindings: Bindings;
  readonly level: number;
  readonly spells: readonly SpellDef[];
  /**
   * Wird gerufen, wenn der Spieler einen Platz ändert.
   *
   * `spellId` ist `null`, wenn der Platz geräumt wird. Die Ansicht entscheidet
   * **nicht**, ob es erlaubt ist — das prüft `bindSpell`, und sie erfährt das
   * Ergebnis über den Rückgabewert.
   */
  readonly onBind: (spellId: string | null, slot: number) => boolean;
}

/** Eine Gruppe der linken Spalte. */
export interface SpellGroup {
  /** Kennung der Schule, oder `null` für die schulunabhängigen. */
  readonly school: string | null;
  /** Überschrift, englisch. */
  readonly label: string;
  /** Farbe der Schule, oder `null` — dann neutrale Textfarbe. */
  readonly colour: string | null;
  readonly spells: readonly SpellDef[];
}

/**
 * Die Zauber nach Schule gruppieren (AC1).
 *
 * **Die Reihenfolge ist die der Daten**, in beiden Richtungen: Die Gruppen
 * stehen in der Folge ihres ersten Auftretens, die Zauber darin in ihrer
 * Listenfolge. Keine Sortierung nach Stärke, keine nach Alphabet — jede wäre
 * eine Aussage darüber, was zuerst gelesen werden soll (AC5).
 *
 * Exportiert, weil ein Prüffall sie ohne Dokument messen kann; die Ansicht
 * darunter braucht eines und ist nur im Browser prüfbar.
 */
export function groupBySchool(spells: readonly SpellDef[]): SpellGroup[] {
  const gruppen: SpellGroup[] = [];
  const index = new Map<string, number>();
  for (const spell of spells) {
    const key = spell.school ?? '';
    const gefunden = index.get(key);
    if (gefunden === undefined) {
      index.set(key, gruppen.length);
      gruppen.push({
        school: spell.school,
        label: spell.school ?? NO_SCHOOL_LABEL,
        colour: colourOfSchool(spell.school),
        spells: [spell],
      });
      continue;
    }
    const gruppe = gruppen[gefunden];
    if (gruppe !== undefined) {
      (gruppe.spells as SpellDef[]).push(spell);
    }
  }
  return gruppen;
}

/** Eine Zeile der Vorschau. */
export interface MechanicPreview {
  /** Kennung der Schule, zu der die Zeile gehört. */
  readonly school: string;
  /** Der fertige Satz, englisch (Design-Gesetz 5). */
  readonly text: string;
}

/**
 * Die Mechanik-Vorschau je Schule (AC2).
 *
 * > „wie viele Zauber je Schul-Mechanik einzahlen — man erkennt sofort, ob die
 * > Hitze-Mechanik mit nur einem Zauber trägt" (GAME-DESIGN 16)
 *
 * DREI ENTSCHEIDUNGEN STECKEN DARIN:
 *
 * 1. **„spending" steht nur da, wenn es einen gibt.** Ein „· 0 spending" bei
 *    jeder Zeile wäre Rauschen — und es wäre eine stumme Bewertung: Wer keinen
 *    Entlader mitführt, hat nicht zwangsläufig etwas falsch gemacht.
 *
 * 2. **DIE ZEILE ERSCHEINT AUCH BEI NULL.** Die Kampf-Anzeige blendet dort aus
 *    (`visibleIndicators` filtert `contributors > 0`), und das ist dort
 *    richtig: Eine Zahl, die sich nie bewegt, ist im Kampf nutzlos. Beim
 *    ZUSAMMENSTELLEN ist genau die Null die Auskunft — „Heat · 0 feeding · 1
 *    spending" bei nur PYRE ist wörtlich der Satz aus 16.
 *
 *    **Das ist keine Regel an zwei Orten, sondern zwei Fragen.** Wer die
 *    Abweichung „aufräumt", nimmt die Auskunft weg, um die es geht.
 *
 * 3. **„no shared meter" für Schulen ohne Zähler.** COAGULATION und
 *    SUBLIMATION führen keinen; eine Null wäre eine Aussage über etwas, das es
 *    nicht gibt.
 *
 * Und was NICHT dasteht: kein „only 1 — consider adding another", kein
 * Ausrufezeichen, keine Einfärbung bei 0. Die Zahl steht da, die Bewertung
 * macht der Spieler. AC5 beginnt beim Wortlaut, nicht erst bei der Sortierung.
 */
export function previewOfLoadout(
  bindings: Bindings,
  slotCount: number,
  spells: readonly SpellDef[],
): MechanicPreview[] {
  // **JEDER ZAUBER EINMAL, auch wenn er auf zwei Plätzen liegt** (Issue #287).
  // Zwei Plätze mit BIND sind eine Quelle auf zwei Knöpfen — keine zweite.
  // Vorher stand hier eine Liste über die Plätze, und damit waren zwei
  // Zusammenstellungen nicht mehr zu unterscheiden:
  //
  //   KINDLE zweimal        Heat · 2 feeding
  //   KINDLE + SMOULDER     Heat · 2 feeding
  //
  // Dieselbe Entdoppelung steht in `mechanicsOfLoadout` — die Zählung dort
  // treibt die Kampf-Anzeige, diese hier die Vorschau. Beide beantworten
  // dieselbe Frage aus GAME-DESIGN 16 und müssen deshalb dieselbe Zahl
  // liefern; ein Prüffall vergleicht sie miteinander.
  const schonGesehen = new Set<string>();
  const belegt: SpellDef[] = [];
  for (let slot = 0; slot < slotCount; slot++) {
    const id = spellAtSlot(bindings, slot);
    if (id === null || schonGesehen.has(id)) continue;
    schonGesehen.add(id);
    const spell = spells.find((s) => s.id === id);
    if (spell !== undefined) belegt.push(spell);
  }

  const zeilen: MechanicPreview[] = [];
  const schulenMitMechanik = new Set(MECHANICS.map((m) => m.school));

  for (const mechanic of MECHANICS) {
    // Nur, wenn ein Zauber DIESER SCHULE auf der Leiste liegt. Sonst stünde
    // eine Zeile für etwas da, das der Spieler gar nicht mitführt.
    if (!belegt.some((s) => s.school === mechanic.school)) continue;
    const speist = belegt.filter((s) => feedsMechanic(s, mechanic.id)).length;
    const zehrt = belegt.filter((s) => spendsMechanic(s, mechanic.id)).length;
    const text =
      zehrt > 0
        ? `${mechanic.label} · ${speist} feeding · ${zehrt} spending`
        : `${mechanic.label} · ${speist} feeding`;
    zeilen.push({ school: mechanic.school, text });
  }

  // Die Schulen ohne Zähler, in der Reihenfolge ihres Auftretens.
  const gesehen = new Set<string>();
  for (const spell of belegt) {
    const schule = spell.school;
    if (schule === null || schulenMitMechanik.has(schule) || gesehen.has(schule)) continue;
    gesehen.add(schule);
    zeilen.push({ school: schule, text: 'no shared meter' });
  }
  return zeilen;
}

/**
 * Die Ansicht aufbauen.
 *
 * Jede Zeile trägt ihre Platznummer und jede Gruppe ihre Schule als Merkmal —
 * so ist im Browser prüfbar, was wo liegt, ohne die Gestaltung zu befragen.
 */
export function buildComposition(options: CompositionViewOptions): HTMLElement {
  const root = element('section', 'composition');
  const plaetze = barSlotsAtLevel(options.level);

  // **Nur freigeschaltete Zauber.** Einen gesperrten anzubieten und dann
  // abzulehnen wäre eine Einladung, die zurückgenommen wird; der Spieler
  // suchte den Grund bei sich.
  const waehlbar = options.spells.filter((spell) => isSpellUnlocked(spell, options.level));

  // --- Links: die Zauber nach Schule (AC1) --------------------------------
  const links = element('div', 'composition-spells');
  links.appendChild(element('h3', 'composition-heading', 'Your spells'));
  for (const gruppe of groupBySchool(waehlbar)) {
    const block = element('div', 'composition-group');
    block.dataset.school = gruppe.school ?? 'none';
    const titel = element('h4', 'composition-group-title', gruppe.label);
    // Die Farbe steht am Element, nicht in der CSS-Datei: Sie kommt aus den
    // Daten (welche Schule), nicht aus der Gestaltung.
    if (gruppe.colour !== null) {
      titel.style.color = gruppe.colour;
      block.style.borderLeftColor = gruppe.colour;
    }
    block.appendChild(titel);
    for (const spell of gruppe.spells) {
      const zeile = element('div', 'composition-spell', spell.name);
      zeile.dataset.spell = spell.id;
      block.appendChild(zeile);
    }
    links.appendChild(block);
  }

  // --- Rechts: die Leiste --------------------------------------------------
  const rechts = element('div', 'composition-bar');
  rechts.appendChild(element('h3', 'composition-heading', 'Your bar'));
  const grid = document.createElement('table');
  grid.className = 'composition-table';

  for (let slot = 0; slot < plaetze; slot++) {
    const line = document.createElement('tr');
    line.className = 'composition-row';
    line.dataset.slot = String(slot);
    line.appendChild(element('td', 'composition-slot', `Slot ${slot + 1}`));

    const zelle = document.createElement('td');
    zelle.className = 'composition-choice';
    const auswahl = document.createElement('select');
    auswahl.className = 'composition-select';
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
      // zeigte sie etwas an, das im Kampf nicht gilt.
      if (!options.onBind(gewaehlt, slot)) {
        auswahl.value = spellAtSlot(options.bindings, slot) ?? '';
      }
    });

    zelle.appendChild(auswahl);
    line.appendChild(zelle);
    grid.appendChild(line);
  }
  rechts.appendChild(grid);

  // --- Die Vorschau (AC2) --------------------------------------------------
  const vorschau = element('div', 'composition-preview');
  for (const zeile of previewOfLoadout(options.bindings, plaetze, options.spells)) {
    const line = element('p', 'composition-preview-line', zeile.text);
    line.dataset.school = zeile.school;
    const farbe = colourOfSchool(zeile.school);
    if (farbe !== null) line.style.color = farbe;
    vorschau.appendChild(line);
  }
  rechts.appendChild(vorschau);

  // Was noch nicht offen ist, steht als Zeile da — sonst wirkt die Leiste
  // fertig, und der Spieler weiß nicht, dass sie wächst.
  const gesperrt = 12 - plaetze;
  if (gesperrt > 0) {
    rechts.appendChild(
      element('p', 'composition-locked', `${gesperrt} more slots unlock as you level.`),
    );
  }

  root.appendChild(links);
  root.appendChild(rechts);
  return root;
}
