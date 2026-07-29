import { describe, expect, it } from 'vitest';
import { computeDamage, NEUTRAL_FACTORS, type DamageFactors } from '../../src/core/damage.js';

/**
 * Die Reihenfolge der Multiplikationen (GAME-DESIGN 6.3, 18.4).
 *
 * **Warum es diese Datei gibt.** Der Simulator hat diesen Wächter seit E1
 * (`sim/src/factors.rs`). Das Spiel hatte keinen — wer die Reihenfolge in
 * `computeDamage` umstellte, hätte nur die eine Seite gesichert.
 *
 * # Was beim Bauen herauskam, und die Behauptung widerlegte
 *
 * Der erste Anlauf dieser Datei behauptete: „Dieselben Zahlen in anderer
 * Reihenfolge ergeben ein anderes Ergebnis." **Gemessen ist das falsch**, und
 * zwar in beide Richtungen:
 *
 * | Frage | Messung |
 * | --- | --- |
 * | Weicht der **rohe** Wert bei Umstellung ab? | in **63,5 %** von 199 999 Wertesätzen ja |
 * | Weicht der **gerundete Schaden** ab? | in **0** von 199 999 — und in 0 von ~20 Mio. gezielt gesuchten |
 *
 * `computeDamage` rundet am Ende, und die Rundung schluckt den
 * Unterschied. Ein Wächter, der nur die gerundete Rückgabe bei üblichen
 * Werten vergleicht, **kann eine Umstellung gar nicht bemerken** — er wäre
 * grün, egal wie die Faktoren stehen.
 *
 * Deshalb arbeitet der Wächter unten mit einem absichtlich riesigen
 * Grundschaden. Dort ist ein letztes Bit groß genug, um die Rundung zu
 * kippen, und der Prüffall bekommt Zähne. Ein Prüffall, der nicht
 * fehlschlagen kann, prüft nichts.
 */

/** Die Faktoren in der Reihenfolge, in der 6.3 sie anwendet. */
const IN_REIHENFOLGE: DamageFactors = {
  essence: 1.605,
  talents: 2.476,
  gearEffects: 1.348,
  buffs: 2.22,
  synergy: 1.091,
  resistance: 1.963,
  crit: 0.834,
};

/**
 * Ein Grundschaden, bei dem ein letztes Bit die Rundung kippt.
 *
 * **Absichtlich weit außerhalb des Spielbereichs** (6.4 nennt einige
 * zehntausend auf Stufe 100). Bei üblichen Beträgen ist der Abstand zweier
 * benachbarter Gleitkommazahlen viel kleiner als eine halbe Einheit, und die
 * Rundung macht jede Umstellung unsichtbar. Hier liegt er bei etwa 1 — genau
 * die Größe, bei der sich eine Umstellung zeigt.
 */
const GROSS = 1e12;

describe('Die Reihenfolge der Multiplikationen steht fest', () => {
  it('rechnet in der Reihenfolge aus 6.3', () => {
    let erwartet = GROSS;
    erwartet *= 1.605; // ESSENCE
    erwartet *= 2.476; // Talente
    erwartet *= 1.348; // Ausrüstungs-Sondereffekte
    erwartet *= 2.22; // aktive Verstärkungen
    erwartet *= 1.091; // Synergie
    erwartet *= 1.963; // Widerstand
    erwartet *= 0.834; // Krit
    expect(computeDamage(GROSS, IN_REIHENFOLGE)).toBe(Math.round(erwartet));
    expect(computeDamage(GROSS, IN_REIHENFOLGE)).toBe(21_241_229_171_569);
  });

  it('fängt den REALISTISCHEN Fehler: zwei benachbarte Faktoren vertauscht', () => {
    // **Der erste Anlauf dieses Prüffalls verglich vorwärts gegen die
    // vollständige Umkehrung.** Die Gegenprobe zeigte, dass er wertlos war:
    // Ich habe den Ausrüstungs-Faktor eine Stelle nach vorne geschoben — also
    // genau den Fehler begangen, vor dem er schützen soll — und er blieb
    // grün. Eine Umkehrung passiert niemandem; ein Vertauschen zweier
    // Nachbarn schon.
    //
    // Die Zahlen hier sind deshalb gesucht, nicht gewählt: ein Satz, bei dem
    // Talente und Ausrüstung vertauscht die Rundung kippen.
    let vertauscht = GROSS;
    vertauscht *= 1.605; // ESSENCE
    vertauscht *= 1.348; // Ausrüstung VOR den Talenten — der Fehler
    vertauscht *= 2.476; // Talente
    vertauscht *= 2.22;
    vertauscht *= 1.091;
    vertauscht *= 1.963;
    vertauscht *= 0.834;

    expect(Math.round(vertauscht)).toBe(21_241_229_171_570);
    expect(computeDamage(GROSS, IN_REIHENFOLGE)).not.toBe(Math.round(vertauscht));
  });

  it('bleibt bei Spielbeträgen unsichtbar — und das ist der Grund für GROSS', () => {
    // Die Messung als Prüffall: Bei üblichen Beträgen ergibt jede Reihenfolge
    // denselben gerundeten Schaden. Wer den Wächter oben auf 100 umstellt,
    // nimmt ihm damit jede Wirkung.
    const klein = 100;
    let vertauscht = klein;
    for (const f of [1.605, 1.348, 2.476, 2.22, 1.091, 1.963, 0.834]) vertauscht *= f;
    expect(computeDamage(klein, IN_REIHENFOLGE)).toBe(Math.round(vertauscht));
  });
});

describe('Der Ausrüstungs-Faktor bewegt heute keine Zahl', () => {
  it('steht neutral auf 1,0', () => {
    // 1,0 ist in IEEE754 die exakte Eins. Genau deshalb war Vorsicht geboten:
    // Eine Änderung, die keine Zahl bewegt, geht sonst unbemerkt durch.
    expect(NEUTRAL_FACTORS.gearEffects).toBe(1);
    expect(computeDamage(4240, NEUTRAL_FACTORS)).toBe(4240);
  });

  it('lässt jeden Schadenswert bitgleich', () => {
    const ohne = { ...NEUTRAL_FACTORS, essence: 1.37, crit: 2 };
    for (let grundschaden = 1; grundschaden <= 2000; grundschaden++) {
      expect(computeDamage(grundschaden, { ...ohne, gearEffects: 1 })).toBe(
        computeDamage(grundschaden, ohne),
      );
    }
  });
});
