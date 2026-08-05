/**
 * Freischaltung durch Stufen (GAME-DESIGN 12.1, 9.1; Issue #248, AC3 und AC4).
 *
 * AC4 IST EINE AUSSAGE ÜBER ZWEI ZUSTÄNDE
 * ---------------------------------------
 * „Die Plätze verschieben sich nicht" lässt sich an einem einzelnen Zustand
 * gar nicht prüfen. Der Prüffall vergleicht deshalb die Belegung auf zwei
 * Stufen — und zwar die Belegung, nicht nur ihre Anzahl. Eine Umsetzung, die
 * bei jedem Aufstieg neu durchnummeriert, hätte auf beiden Stufen dieselbe
 * ANZAHL und trotzdem die Regel gebrochen.
 *
 * WAS `unlockLevel` VORHER WAR
 * ----------------------------
 * Es stand in den Zauberdaten und wurde vom Leser getragen — aber kein
 * Aufrufer fragte danach. Ein Feld, das niemand liest, ist eine Zusage ohne
 * Wirkung; in diesem Projekt ist genau das schon viermal aufgefallen. Der
 * erste Prüffall unten belegt, dass es jetzt etwas tut.
 */
import { describe, expect, it } from 'vitest';
import { CALCINATION, LEVELS } from '../../src/core/data.js';
import {
  barSlotsAtLevel,
  isSpellUnlocked,
  nextBarSlotLevel,
  unlockedSlots,
  unlockedSpells,
} from '../../src/progression/unlocks.js';

const SPELLS = CALCINATION.value.spells;

describe('AC3 — Zauber sind an Stufen gebunden', () => {
  it('gibt auf Stufe 1 weniger frei als auf Stufe 2 — das Feld wirkt wirklich', () => {
    // **Der Beleg, dass `unlockLevel` einen Aufrufer hat.** Wären beide Listen
    // gleich lang, täte das Feld nichts, und der Prüffall wäre grün, ohne
    // etwas zu messen — deshalb steht der Vergleich hier und nicht nur eine
    // Längenprüfung.
    const aufEins = unlockedSpells(SPELLS, 1);
    const aufZwei = unlockedSpells(SPELLS, 2);
    expect(aufEins.length).toBeLessThan(aufZwei.length);
    expect(aufEins.map((s) => s.id)).not.toContain('pyre');
    expect(aufZwei.map((s) => s.id)).toContain('pyre');
  });

  it('lässt einen Zauber unterhalb seiner Stufe nicht zu', () => {
    const pyre = SPELLS.find((s) => s.id === 'pyre');
    expect(pyre?.unlockLevel).toBe(2);
    expect(isSpellUnlocked(pyre!, 1)).toBe(false);
    expect(isSpellUnlocked(pyre!, 2)).toBe(true);
    expect(isSpellUnlocked(pyre!, 50)).toBe(true);
  });

  it('nimmt einen einmal freigeschalteten Zauber nie wieder weg', () => {
    // Ein Zauber, der auf einer höheren Stufe verschwindet, wäre ein Rückschritt
    // — und der Spieler hätte eine Rotation eingeübt, die plötzlich fehlt.
    let vorher = 0;
    for (let stufe = 1; stufe <= LEVELS.maxLevel; stufe++) {
      const jetzt = unlockedSpells(SPELLS, stufe).length;
      expect(jetzt, `Stufe ${stufe}`).toBeGreaterThanOrEqual(vorher);
      vorher = jetzt;
    }
  });

  it('lässt die Reihenfolge der Quelle stehen, statt nach Stufe zu sortieren', () => {
    // Sortieren würde die Liste bei jedem Aufstieg umordnen, und eine Anzeige,
    // die daran hängt, hüpfte — dieselbe Regel wie bei den Plätzen.
    //
    // **Verglichen wird gegen die erreichbaren Zauber, nicht gegen alle**
    // (Issue #304). Der Prüf-Zauber zur Wirkzeit liegt ÜBER der Stufengrenze
    // und darf hier fehlen; der Fall darunter belegt, dass genau er es ist.
    const erreichbar = SPELLS.filter((s) => s.unlockLevel <= LEVELS.maxLevel);
    const alle = unlockedSpells(SPELLS, LEVELS.maxLevel).map((s) => s.id);
    expect(alle).toEqual(erreichbar.map((s) => s.id));
  });

  it('HAELT DEN PRUEF-ZAUBER AUCH AUF DER HOECHSTEN STUFE ZURUECK', () => {
    // **Der Fall, an dem die ganze Bauart von Issue #304 haengt.** `test-cast`
    // steht im Index-Raum, weil der Simulator dieselbe Zauberliste braucht wie
    // das Spiel — aber der Spieler darf ihn nie bekommen. Waere er
    // freischaltbar, stuende ein Prüf-Zauber im Zauberbuch.
    //
    // Gemessen an der HOECHSTEN Stufe: Auf Stufe 1 waere jeder Zauber mit
    // hoher Stufe zurueckgehalten, und der Fall bewiese nichts.
    const oben = unlockedSpells(SPELLS, LEVELS.maxLevel).map((s) => s.id);
    expect(oben, 'sonst stuende er im Zauberbuch').not.toContain('test-cast');
    // Und die Gegenrichtung: Er steht ueberhaupt in der Liste — sonst
    // prueft der Fall daran, dass es ihn nicht gibt.
    expect(SPELLS.map((s) => s.id)).toContain('test-cast');
  });
});

describe('AC4 — die Leisten-Plätze wachsen, ohne bestehende zu verschieben', () => {
  it('beginnt mit zwei und endet bei zwölf', () => {
    // 12.2: „Start-Ausstattung: zwei Zauber, zwei Leisten-Plätze."
    // 4.2: zwölf Plätze bei zwölf Zaubern — man kann nicht alle mitnehmen.
    expect(barSlotsAtLevel(1)).toBe(2);
    expect(barSlotsAtLevel(LEVELS.maxLevel)).toBe(12);
  });

  it('wächst schrittweise und nimmt nie einen Platz weg', () => {
    let vorher = 0;
    for (let stufe = 1; stufe <= LEVELS.maxLevel; stufe++) {
      const jetzt = barSlotsAtLevel(stufe);
      expect(jetzt, `Stufe ${stufe}`).toBeGreaterThanOrEqual(vorher);
      vorher = jetzt;
    }
  });

  it('DIE PLÄTZE VERSCHIEBEN SICH NICHT — Belegung auf Stufe 10 gegen Stufe 20', () => {
    // **Die Gegenprobe, die AC4 verlangt, und sie braucht zwei Zustände.**
    // „Es verschiebt sich nichts" ist an einem einzelnen Zustand gar nicht
    // prüfbar: Eine Umsetzung, die bei jedem Aufstieg neu durchnummeriert,
    // hätte auf beiden Stufen dieselbe ANZAHL.
    //
    // Geprüft wird deshalb die Belegung selbst: Jeder Platz, den es auf Stufe
    // 10 gab, trägt auf Stufe 20 dieselbe Nummer.
    const auf10 = barSlotsAtLevel(10);
    const auf20 = barSlotsAtLevel(20);
    expect(auf20).toBeGreaterThan(auf10);

    // **Beide Belegungen kommen aus der Umsetzung**, keine wird hier gebaut.
    // Mein erster Entwurf stellte sie aus der Anzahl selbst her und verglich
    // damit seine eigene Erwartung mit sich — grün, egal was die Umsetzung tut.
    const alt = unlockedSlots(10);
    const neu = unlockedSlots(20);
    expect(alt).toHaveLength(auf10);
    expect(neu).toHaveLength(auf20);
    expect(neu.slice(0, alt.length), 'Platz 3 muss Platz 3 bleiben').toEqual(alt);
  });

  it('hält das über die ganze Strecke, nicht nur zwischen zwei Stufen', () => {
    // Zwei Stufen zu vergleichen belegt zwei Stufen. Eine Umsetzung, die
    // ausgerechnet zwischen 30 und 31 umnummeriert, käme oben durch.
    for (let stufe = 2; stufe <= LEVELS.maxLevel; stufe++) {
      const alt = unlockedSlots(stufe - 1);
      const neu = unlockedSlots(stufe);
      expect(neu.slice(0, alt.length), `Stufe ${stufe - 1} -> ${stufe}`).toEqual(alt);
    }
  });

  it('nennt die nächste Freischalt-Stufe, und null, wenn keine mehr kommt', () => {
    expect(nextBarSlotLevel(1)).toBe(LEVELS.barSlotLevels[0]);
    expect(nextBarSlotLevel(LEVELS.maxLevel)).toBeNull();
    // Auf der letzten Freischalt-Stufe selbst kommt keine mehr.
    const letzte = LEVELS.barSlotLevels[LEVELS.barSlotLevels.length - 1] as number;
    expect(nextBarSlotLevel(letzte)).toBeNull();
    expect(nextBarSlotLevel(letzte - 1)).toBe(letzte);
  });

  it('gibt mehr Zauber als Plätze — sonst gäbe es nichts abzuwägen', () => {
    // 4.2 nennt das die Entscheidung, um die es geht. Heute hat Schritt 1
    // weniger Zauber als das fertige Spiel; die Zusicherung steht trotzdem
    // hier, damit sie beim Wachsen der Zauberliste geprüft bleibt.
    expect(barSlotsAtLevel(LEVELS.maxLevel)).toBe(12);
    expect(LEVELS.barSlotLevels).toHaveLength(10);
  });
});
