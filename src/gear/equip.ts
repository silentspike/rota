/**
 * Anlegen und Ablegen (Issue #453, GAME-DESIGN 13.4).
 *
 * WARUM DIESE DATEI ÜBERHAUPT ENTSTEHT
 * ------------------------------------
 * Bis hierher setzte `equippedSlot` ausserhalb der Datenschicht **genau eine**
 * Stelle: `__rota.setItems` in `src/devtools/index.ts`. Das ist ein
 * Dev-Werkzeug, und im Auslieferungs-Bau wird es herausgeschnitten
 * (GAME-DESIGN 19, D.3). Ein Fund landete also im Beutel und blieb dort —
 * `wornFrom` fand überall `null`, und der Spieler kämpfte dauerhaft ohne
 * Ausrüstung.
 *
 * Die Mechanik war vollständig. Was fehlte, war der Weg hinein.
 *
 * DER PLATZ GEHÖRT DEM GEGENSTAND, NICHT DER WAHL
 * -----------------------------------------------
 * Das Issue sagt „ein Stück aus dem Beutel auf **seinen** Platz". Ein Fokus
 * geht auf den Fokus-Platz, wie bei Diablo und WoW — deshalb nimmt [`equip`]
 * **kein** Platz-Argument. Der Platz steht im Katalog (`def.slot`), und ihn
 * wählbar zu machen hiesse, eine Entscheidung anzubieten, die es nicht gibt.
 *
 * Nebenbefund beim Messen: `wornFrom` vergleicht `equippedSlot` **nicht** mit
 * `def.slot`. Ein Stück für Platz 0 auf Platz 7 würde dort mitgerechnet. Weil
 * diese Datei die einzige Schreibstelle ist, kann das nur noch aus einem von
 * Hand bearbeiteten Spielstand kommen — dafür ist die Bereinigung in
 * `entry-shape.ts` zuständig, nicht diese Datei.
 *
 * WORAN EIN STÜCK ERKANNT WIRD — UND WARUM DAS FRAGIL IST
 * ------------------------------------------------------
 * Am **Index in der Liste**. Nicht schön, aber es ist das, was es gibt: Zwei
 * Stücke derselben `itemId` mit verschiedenen `upgradeSteps` sind über die
 * Kennung nicht zu unterscheiden, und der Spielstand führt keine Stück-ID.
 *
 * Eine einzuführen wäre ein Format-Wechsel im Spielstand — dieselbe Klasse,
 * die in #435 ausdrücklich abgegrenzt wurde. Sie gehört in ein eigenes Issue,
 * nicht als stiller Nebeneffekt in eine Oberflächen-Aufgabe.
 *
 * **Was daraus folgt für den Aufrufer:** Der Index gilt nur für die Liste, aus
 * der er stammt. Wer sortiert oder filtert, darf die Indizes der Ursprungs-
 * liste nicht weiterreichen.
 */
import { itemById, type OwnedItemLike } from './catalog.js';
import { SLOT_COUNT } from './slots.js';

/**
 * Ein Besitz-Eintrag, wie ihn der Spielstand hält.
 *
 * Deckungsgleich mit [`OwnedItemLike`] — der eigene Name steht hier, weil
 * diese Datei die Einträge **schreibt** und `catalog.ts` sie nur liest.
 */
export type OwnedItem = OwnedItemLike;

/**
 * Ein Stück anlegen.
 *
 * Gibt eine **neue** Liste zurück; die übergebene bleibt unberührt. Der
 * Spielstand wird an einer Stelle ersetzt, nicht an vielen verändert — sonst
 * hätte jeder Aufrufer, der die alte Liste noch hält, plötzlich die neue.
 *
 * Was auf dem Zielplatz lag, geht **zurück in den Beutel** (Issue #453). Das
 * ist zugleich die Durchsetzung von AC4: Nach diesem Aufruf trägt der Platz
 * genau ein Stück, weil alles andere von ihm heruntergenommen wird — nicht
 * nur das eine, das man dort vermutet.
 *
 * @returns die neue Liste, oder `null`, wenn der Aufruf nicht ausführbar war
 *   (Index daneben, unbekannte Kennung, Platz ausserhalb der zehn). `null`
 *   statt einer stillschweigend unveränderten Liste: Der Aufrufer soll den
 *   Unterschied zwischen „getan" und „ging nicht" sehen können.
 */
export function equip(items: readonly OwnedItem[], index: number): OwnedItem[] | null {
  const stueck = items[index];
  if (stueck === undefined) return null;

  const def = itemById(stueck.itemId);
  // Ein Stück, das der Katalog nicht kennt, hat keinen Platz — es anzulegen
  // hiesse, eine Zahl zu erfinden. `wornFrom` übergeht solche Einträge
  // ohnehin; sie hier abzuweisen ist dieselbe Haltung an der Schreibseite.
  if (def === undefined) return null;
  if (!Number.isInteger(def.slot) || def.slot < 0 || def.slot >= SLOT_COUNT) return null;

  return items.map((eintrag, i) => {
    if (i === index) return { ...eintrag, equippedSlot: def.slot };
    // **Alles vom Zielplatz nehmen, nicht das erste.** Läge dort — aus einem
    // beschädigten Stand — mehr als ein Stück, bliebe bei „das erste" der Rest
    // liegen, und die Regel wäre nach dem Anlegen weiterhin verletzt.
    if (eintrag.equippedSlot === def.slot) return { ...eintrag, equippedSlot: null };
    return eintrag;
  });
}

/**
 * Ein Stück ablegen. Der Platz wird leer, das Stück liegt im Beutel.
 *
 * @returns die neue Liste, oder `null` bei einem Index daneben. Ein Stück, das
 *   bereits im Beutel liegt, ist **kein** Fehler — der Aufruf ist dann nur
 *   wirkungslos, und die zurückgegebene Liste ist gleichwertig.
 */
export function unequip(items: readonly OwnedItem[], index: number): OwnedItem[] | null {
  if (items[index] === undefined) return null;
  return items.map((eintrag, i) => (i === index ? { ...eintrag, equippedSlot: null } : eintrag));
}

/**
 * Was auf welchem Platz liegt — als Index in die übergebene Liste.
 *
 * Für die Ansicht: Sie zeichnet zehn Plätze und muss zu jedem wissen, ob dort
 * etwas liegt. Ohne diese Funktion suchte jeder Platz die Liste selbst durch,
 * zehnmal für dieselbe Antwort.
 *
 * Liegt — entgegen AC4 — mehr als ein Stück auf einem Platz, gewinnt das
 * **letzte**. Das ist willkürlich und soll es sein: Der Fall gehört bereinigt
 * (`entry-shape.ts`) und nicht hier still geregelt. Eine Anzeige, die dabei
 * abbricht, wäre für den Spieler schlimmer als eine, die eines von beiden
 * zeigt.
 */
export function equippedBySlot(items: readonly OwnedItem[]): readonly (number | null)[] {
  const plaetze: (number | null)[] = new Array<number | null>(SLOT_COUNT).fill(null);
  items.forEach((eintrag, i) => {
    const slot = eintrag.equippedSlot;
    if (slot === null || !Number.isInteger(slot) || slot < 0 || slot >= SLOT_COUNT) return;
    plaetze[slot] = i;
  });
  return plaetze;
}
