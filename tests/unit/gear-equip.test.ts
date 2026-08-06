/**
 * Anlegen, Ablegen und die Bereinigung beim Laden (Issue #453, AC4).
 *
 * WAS HIER GEPRÜFT WIRD UND WAS NICHT
 * -----------------------------------
 * AC1 bis AC3 und AC5 sind `browser`-Evidence — sie hängen an der Ansicht und
 * am ausgelieferten Bau, und `environment: 'node'` sähe von beidem nichts.
 * Hier steht **AC4**: die Regel „ein Platz höchstens ein Stück" und die
 * Bereinigung eines Standes, der sie verletzt.
 *
 * ZWEI BEHAUPTUNGEN AUS DEN KOMMENTAREN WERDEN HIER BELEGT
 * --------------------------------------------------------
 * 1. `equip` nimmt **alles** vom Zielplatz, nicht das erste Stück. Steht so in
 *    `equip.ts`; ohne Prüffall wäre es eine Zusage an mich selbst.
 * 2. `equippedBySlot` und `fixSlotOccupancy` entscheiden sich bei einer
 *    Doppelbelegung für **dasselbe** Stück. Zwei Stellen, die verschieden
 *    wählten, zeigten dem Spieler ein anderes Stück als das, mit dem gerechnet
 *    wird — und niemandem fiele es auf, weil beide für sich plausibel sind.
 *
 * DIE WERTE-PROBE IST DER EIGENTLICHE NACHWEIS
 * --------------------------------------------
 * Dass `equippedSlot` danach `null` ist, sagt nur, dass ein Feld gesetzt wurde.
 * Dass `wornFrom` **denselben Zahlenwert** liefert wie vorher, sagt, dass es
 * angekommen ist. Das ist dieselbe Unterscheidung wie in AC3 des Issues, nur
 * eine Schicht tiefer.
 */
import { describe, expect, it } from 'vitest';
import { equip, equippedBySlot, unequip, type OwnedItem } from '../../src/gear/equip.js';
import { wornFrom } from '../../src/gear/catalog.js';
import { SLOT_COUNT } from '../../src/gear/slots.js';
import { cleanSave } from '../../src/analysis/entry-shape.js';

/** Der einzige Katalog-Eintrag heute: `test-relic`, Platz 0. */
function stueck(equippedSlot: number | null = null, upgradeSteps = 0): OwnedItem {
  return { itemId: 'test-relic', upgradeSteps, equippedSlot };
}

describe('AC4 — anlegen', () => {
  it('legt auf den Platz, der im Katalog steht — nicht auf einen gewählten', () => {
    const nachher = equip([stueck()], 0);
    expect(nachher?.[0]?.equippedSlot).toBe(0);
  });

  it('und die getragenen Werte ändern sich messbar', () => {
    // Der Wirkungsnachweis: Ein gesetztes Feld allein bewegt nichts.
    const vorher = wornFrom([stueck()]);
    const nachher = wornFrom(equip([stueck()], 0) ?? []);
    expect(nachher.essence).toBeGreaterThan(vorher.essence);
  });

  it('was auf dem Platz lag, geht zurück in den Beutel', () => {
    const nachher = equip([stueck(0), stueck()], 1);
    expect(nachher?.[0]?.equippedSlot).toBeNull();
    expect(nachher?.[1]?.equippedSlot).toBe(0);
  });

  it('BEHAUPTUNG 1: es nimmt ALLES vom Zielplatz, nicht das erste', () => {
    // Aus einem beschädigten Stand können zwei Stücke auf demselben Platz
    // liegen. Wer nur das erste herunternimmt, lässt die Regel verletzt —
    // nach einem Aufruf, der sie herstellen sollte.
    const nachher = equip([stueck(0), stueck(0), stueck()], 2);
    expect(nachher?.filter((e) => e.equippedSlot === 0)).toHaveLength(1);
    expect(nachher?.[2]?.equippedSlot).toBe(0);
  });

  it('die übergebene Liste bleibt unberührt', () => {
    const vorher: OwnedItem[] = [stueck()];
    equip(vorher, 0);
    expect(vorher[0]?.equippedSlot).toBeNull();
  });

  it('ein Index daneben gibt `null`, keine stillschweigend gleiche Liste', () => {
    expect(equip([stueck()], 7)).toBeNull();
    expect(equip([], 0)).toBeNull();
  });

  it('und eine unbekannte Kennung ebenfalls', () => {
    expect(equip([{ itemId: 'gibt-es-nicht', upgradeSteps: 0, equippedSlot: null }], 0)).toBeNull();
  });
});

describe('AC4 — ablegen', () => {
  it('der Platz wird leer', () => {
    expect(unequip([stueck(0)], 0)?.[0]?.equippedSlot).toBeNull();
  });

  it('DIE WERTE-PROBE: es fällt auf DENSELBEN Zahlenwert zurück', () => {
    // Nicht „kleiner". Derselbe Wert — sonst wäre auch ein anderer Schaden
    // wahr, und der Prüffall bemerkte ihn nicht (Auflage zu AC3).
    const leer = wornFrom([stueck()]);
    const angelegt = equip([stueck()], 0) ?? [];
    expect(wornFrom(angelegt).essence).not.toBe(leer.essence);
    expect(wornFrom(unequip(angelegt, 0) ?? [])).toEqual(leer);
  });

  it('ein Stück, das schon im Beutel liegt, abzulegen ist kein Fehler', () => {
    expect(unequip([stueck()], 0)?.[0]?.equippedSlot).toBeNull();
  });

  it('ein Index daneben gibt `null`', () => {
    expect(unequip([stueck()], 3)).toBeNull();
  });
});

describe('AC4 — was auf welchem Platz liegt', () => {
  it('zehn Plätze, leer ist `null`', () => {
    const plaetze = equippedBySlot([]);
    expect(plaetze).toHaveLength(SLOT_COUNT);
    expect(plaetze.every((p) => p === null)).toBe(true);
  });

  it('ein Platz ausserhalb der zehn wird übergangen, nicht geworfen', () => {
    expect(() => equippedBySlot([stueck(99)])).not.toThrow();
    expect(equippedBySlot([stueck(99)]).every((p) => p === null)).toBe(true);
  });
});

describe('AC4 — ein verletzter Stand wird beim Laden bereinigt', () => {
  it('zwei Stücke auf einem Platz: eines bleibt', () => {
    const ergebnis = cleanSave({ items: [stueck(0), stueck(0)] });
    const items = ergebnis.data?.['items'] as OwnedItem[];
    expect(items.filter((e) => e.equippedSlot === 0)).toHaveLength(1);
    expect(ergebnis.unequipped).toBe(1);
  });

  it('ABGELEGT, NICHT VERWORFEN — der Besitz bleibt vollständig', () => {
    // Der Unterschied ist der ganze Punkt: Ein verworfener Eintrag nähme dem
    // Spieler einen Gegenstand, den er sich erspielt hat. Falsch ist nur, WO
    // er liegt.
    const ergebnis = cleanSave({ items: [stueck(0), stueck(0), stueck(0)] });
    const items = ergebnis.data?.['items'] as OwnedItem[];
    expect(items).toHaveLength(3);
    expect(items.filter((e) => e.equippedSlot === null)).toHaveLength(2);
    // Und es zählt NICHT als verworfen.
    expect(ergebnis.dropped['items']).toBeUndefined();
  });

  it('ein Platz ausserhalb der zehn wird abgelegt', () => {
    // `isOwnedItem` lässt jede Zahl durch, und `wornFrom` prüft den Bereich
    // nicht — ein Stück auf Platz 99 würde dort mitgerechnet.
    const ergebnis = cleanSave({ items: [stueck(99)] });
    expect((ergebnis.data?.['items'] as OwnedItem[])[0]?.equippedSlot).toBeNull();
    expect(ergebnis.unequipped).toBe(1);
  });

  it('und die Bereinigung wirkt sich auf die getragenen Werte aus', () => {
    // Ohne sie trüge der Spieler zwei Stücke auf einem Platz — doppelte Werte
    // aus einer Belegung, die es nicht geben darf.
    const roh = [stueck(0), stueck(0)];
    const bereinigt = cleanSave({ items: roh }).data?.['items'] as OwnedItem[];
    expect(wornFrom(roh).essence).toBeGreaterThan(wornFrom(bereinigt).essence);
    expect(wornFrom(bereinigt)).toEqual(wornFrom([stueck(0)]));
  });

  it('GEGENPROBE: ein heiler Stand wird nicht angefasst', () => {
    // Ohne diese Zeile wäre nicht zu sehen, ob die Bereinigung den Fehlerfall
    // trifft oder einfach immer etwas ablegt.
    const heil = [stueck(0), stueck(null)];
    const ergebnis = cleanSave({ items: heil });
    expect(ergebnis.unequipped).toBe(0);
    expect(ergebnis.data?.['items']).toEqual(heil);
  });

  it('GEGENPROBE: ein Stand ganz ohne `items` bleibt heil', () => {
    const ergebnis = cleanSave({ runHistory: [] });
    expect(ergebnis.rejected).toBeNull();
    expect(ergebnis.unequipped).toBe(0);
  });

  it('erst filtern, dann Plätze ordnen', () => {
    // Der unbrauchbare Eintrag darf dem heilen den Platz nicht wegnehmen. Wäre
    // die Reihenfolge vertauscht, läge Platz 0 als besetzt vor, und das heile
    // Stück würde abgelegt — obwohl der Besetzer gleich darauf verschwindet.
    const ergebnis = cleanSave({ items: [{ itemId: '', equippedSlot: 0 }, stueck(0)] });
    const items = ergebnis.data?.['items'] as OwnedItem[];
    expect(items).toHaveLength(1);
    expect(items[0]?.equippedSlot).toBe(0);
    expect(ergebnis.unequipped).toBe(0);
  });
});

describe('BEHAUPTUNG 2: beide Stellen wählen bei Doppelbelegung dasselbe Stück', () => {
  it('`equippedBySlot` und die Bereinigung sind sich einig', () => {
    // Sonst zeigte die Ansicht ein anderes Stück als das, mit dem `wornFrom`
    // rechnet — und beide wären für sich plausibel.
    const roh = [stueck(0, 0), stueck(0, 5)];
    const gezeigt = equippedBySlot(roh)[0];
    const bereinigt = cleanSave({ items: roh }).data?.['items'] as OwnedItem[];
    const behalten = bereinigt.findIndex((e) => e.equippedSlot === 0);
    expect(behalten).toBe(gezeigt);
    // Und es ist wirklich dasselbe Stück, nicht nur derselbe Index.
    expect(bereinigt[behalten]?.upgradeSteps).toBe(roh[gezeigt as number]?.upgradeSteps);
  });
});
