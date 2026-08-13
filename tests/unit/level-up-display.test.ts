/**
 * Der Aufstieg zeigt nur, was wirklich eintritt (Issue #371).
 *
 * Die Prüfung an einer **leeren Stufe** ist die wichtigere: Dort gibt es weder
 * Zauber noch Platz, nur die Grundwerte. Eine Einblendung, die dort vier Zeilen
 * zeigt, von denen drei leer sind, wäre schlechter als keine.
 *
 * # Die leere Stufe wird GESUCHT, nicht genannt (Issue #323)
 *
 * Bis hierher stand dort die Zahl **7**. Sie war die einzige leere Stufe unter
 * 20 — und als `decoction` sie besetzte, wurde der Fall rot, ohne dass an der
 * geprüften Sache etwas falsch war.
 *
 * Eine Zahl in der Bedingung altert mit den Daten. Die EIGENSCHAFT „eine Stufe,
 * auf der nichts dazukommt" gilt weiter, solange es eine solche Stufe gibt —
 * und wenn es keine mehr gibt, sagt das die Zusicherung unten, statt still
 * durchzulaufen.
 */
import { describe, expect, it } from 'vitest';
import { levelUpDisplayFor } from '../../src/analysis/level-up-display.js';
import { CALCINATION, LEVELS } from '../../src/core/data.js';

const SPELLS = CALCINATION.value.spells;

/** Die erste Stufe, auf der weder ein Zauber noch ein Platz dazukommt. */
function leereStufe(): number {
  for (let stufe = 2; stufe <= 100; stufe++) {
    const zauber = SPELLS.some((s) => s.unlockLevel === stufe);
    const platz = LEVELS.barSlotLevels.includes(stufe);
    if (!zauber && !platz) return stufe;
  }
  return -1;
}

describe('Was eine Stufe bringt', () => {
  it('zeigt nichts ohne Aufstieg', () => {
    expect(levelUpDisplayFor(false, 7, SPELLS)).toBeNull();
  });

  it('nennt auf Stufe 3 den Zauber UND den Platz', () => {
    // Stufe 3 steht in barSlotLevels und trägt einen Zauber — der Fall, in
    // dem alles Sichtbare zusammenkommt.
    const d = levelUpDisplayFor(true, 3, SPELLS);
    expect(d?.level).toBe(3);
    expect(d?.newBarSlots).toBe(1);
    expect(d?.newSpells.length).toBeGreaterThan(0);
  });

  it('nennt auf einer leeren Stufe NUR die Grundwerte', () => {
    // **Der eigentliche Fall.** Ohne die Bedingung stünden hier zwei leere
    // Zeilen.
    const stufe = leereStufe();
    // Die Zusicherung vor der Messung: Gäbe es keine leere Stufe mehr, liefe
    // der Fall unten mit `levelUpDisplayFor(-1, …)` durch und prüfte nichts.
    expect(stufe, 'keine Stufe ohne neuen Zauber und ohne neuen Platz').toBeGreaterThan(0);

    const d = levelUpDisplayFor(true, stufe, SPELLS);
    expect(d?.newSpells, `Stufe ${stufe} trägt doch einen Zauber`).toEqual([]);
    expect(d?.newBarSlots).toBe(0);
    expect(d?.statPercent).toBe(LEVELS.growthPerLevelPercent);
  });

  it('nennt nur die Zauber DIESER Stufe, nicht alle bisherigen', () => {
    // Der Gegenfall zur einfachsten Fehlbauart: `unlockLevel <= stufe` statt
    // `=== stufe`. Auf Stufe 3 wären das alle Zauber ab Stufe 1.
    const d = levelUpDisplayFor(true, 3, SPELLS);
    const alleBis3 = SPELLS.filter((s) => s.unlockLevel <= 3).length;
    expect(d?.newSpells.length).toBeLessThan(alleBis3);
  });

  it('gibt nie negative Plaetze', () => {
    // barSlotsAtLevel sinkt nie — aber die Differenz wird gerechnet, und eine
    // Rechnung kann sich irren.
    for (const stufe of [1, 2, 3, 7, 22, 90, 100]) {
      expect(levelUpDisplayFor(true, stufe, SPELLS)?.newBarSlots).toBeGreaterThanOrEqual(0);
    }
  });

  it('liefert die Zauber als LISTE, damit die Anzeige zaehlen kann', () => {
    // **Der Anlass war ein echter Fehler in der Anzeige** (#371): Dort stand
    // „New spell:" im Singular, auch bei vier Zaubern — auf Stufe 2 werden
    // vier frei, und es ist der zweite Text, den das Spiel dem Spieler
    // überhaupt zeigt.
    //
    // Dieser Fall prüft die Voraussetzung dafür, dass die Anzeige es richtig
    // machen KANN: eine Liste, keine fertige Zeichenkette. Wer hier einen
    // vorformatierten Satz zurückgäbe, nähme der Anzeige die Entscheidung ab
    // — und mit ihr die Möglichkeit, sie richtig zu treffen.
    const zwei = levelUpDisplayFor(true, 2, SPELLS);
    expect(Array.isArray(zwei?.newSpells)).toBe(true);
    expect(zwei?.newSpells.length).toBeGreaterThan(1);
    for (const name of zwei?.newSpells ?? []) {
      expect(name).not.toContain(',');
    }
  });
});
