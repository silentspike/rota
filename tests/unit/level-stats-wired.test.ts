/**
 * Die Stufe wirkt auf die Grundwerte (Issue #289).
 *
 * AC4 STEHT ZUERST, UND ZWAR AUS EINEM GEMESSENEN GRUND
 * ----------------------------------------------------
 * Der Referenzlauf und jede eingecheckte Vergleichsdatei laufen auf Stufe 1.
 * Liegt die neue Rechnung dort um EINE Einheit daneben, wandert jede
 * Aufzeichnung — und der Nachweis, dass sich nichts bewegt hat, ist verloren.
 *
 * Deshalb prueft der erste Fall unten nicht, dass die Stufe wirkt, sondern
 * dass sie auf Stufe 1 NICHTS tut.
 *
 * WARUM ES DIESE PRUEFFAELLE UEBERHAUPT BRAUCHT
 * ---------------------------------------------
 * `statAtLevel` gab es seit #248, mit siebzehn Prueffaellen — und keinem
 * Aufrufer in `src/`. Ein Spieler auf Stufe 100 war genauso stark wie einer
 * auf Stufe 1, waehrend das Player Home ihm „Stat growth x4.116" zeigte.
 *
 * Die Funktion war richtig und nutzlos. Was fehlte, war nicht die Rechnung,
 * sondern ihre Anwendung — und genau das misst diese Datei.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, PLAYER, statAtLevel } from '../../src/core/data.js';

function kampf(level?: number): Combat {
  return new Combat(
    level === undefined
      ? { school: CALCINATION.value, boss: FIRST_CRUCIBLE.value }
      : { school: CALCINATION.value, boss: FIRST_CRUCIBLE.value, level },
  );
}

describe('AC4 — auf Stufe 1 aendert sich NICHTS', () => {
  it('nimmt dort exakt den Grundwert aus base.json', () => {
    expect(statAtLevel(PLAYER.vessel, 1)).toBe(PLAYER.vessel);
    expect(kampf(1).playerHp).toBe(PLAYER.vessel);
  });

  it('nimmt OHNE Stufenangabe denselben Wert', () => {
    // Der Referenzlauf und die Aufzeichnungen geben keine Stufe an. Waere die
    // Vorgabe eine andere als 1, wanderte jede Vergleichsdatei.
    expect(kampf().playerHp).toBe(PLAYER.vessel);
    expect(kampf().playerHp).toBe(kampf(1).playerHp);
  });
});

describe('AC1 — die Stufe wirkt auf die Lebenspunkte', () => {
  it('gibt einem Kampf auf Stufe 30 messbar mehr als auf Stufe 1', () => {
    const auf1 = kampf(1).playerHp;
    const auf30 = kampf(30).playerHp;
    expect(auf30).toBeGreaterThan(auf1);
    // **Die Zahl kommt aus statAtLevel, nicht aus diesem Pruefstand.** Eine
    // hier abgeschriebene Zahl waere eine Balance-Aussage — und base.json
    // traegt „VORLAEUFIGE WERTE, NICHT BALANCIERT".
    expect(auf30).toBe(statAtLevel(PLAYER.vessel, 30));
  });

  it('waechst mit jeder Stufe, statt zu springen', () => {
    let vorher = 0;
    for (const stufe of [1, 2, 5, 10, 30, 100]) {
      const jetzt = kampf(stufe).playerHp;
      expect(jetzt, `Stufe ${stufe}`).toBeGreaterThanOrEqual(vorher);
      vorher = jetzt;
    }
  });

  it('deckelt die Heilung am Wert DIESER Stufe', () => {
    // Sonst heilte ein Spieler auf Stufe 30 nur bis zum Grundwert — und die
    // gewonnenen Lebenspunkte waeren nach dem ersten Schaden fort.
    const combat = kampf(30);
    expect(combat.playerHp).toBe(statAtLevel(PLAYER.vessel, 30));
    expect(combat.playerHp).toBeGreaterThan(PLAYER.vessel);
  });
});

describe('AC2 — WARD bleibt ein Anteil', () => {
  it('skaliert NICHT mit der Stufe', () => {
    // **Gemessen, bevor gebaut wurde**: `statAtLevel(0.2, 30)` ist 0 — die
    // Funktion rechnet ganzzahlig, und ein Anteil von 0,2 wird dabei zu
    // nichts. Der Spieler verloere seine Reduktion vollstaendig.
    //
    // Der Simulator skaliert WARD ebenfalls nicht (`fight.rs:1596`,
    // `PLAYER.incoming(raw)`). Ihn hier zu skalieren braeche die Paritaet.
    //
    // Sachlich ist es dieselbe Antwort: Ein Prozentsatz, der mitwaechst,
    // erreicht irgendwann 100 % und macht den Spieler unverwundbar. Was
    // mitwaechst, ist der ABGEFANGENE Schaden — und der ergibt sich von
    // selbst aus dem groesseren Pool.
    expect(statAtLevel(PLAYER.wardReduction, 30), 'so ginge es NICHT').toBe(0);

    const auf1 = kampf(1);
    const auf30 = kampf(30);
    expect(auf30.wardReduction).toBe(auf1.wardReduction);
    expect(auf30.wardReduction).toBe(PLAYER.wardReduction);
  });
});
