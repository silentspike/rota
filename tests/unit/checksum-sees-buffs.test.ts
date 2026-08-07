import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { combatChecksum } from '../../src/core/checksum.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Die Prüfsumme sieht ein laufendes Verstärkungs-Fenster (Issue #493).
 *
 * # Der blinde Fleck
 *
 * Bis hierher ging die Verstärkung nur **indirekt** in die Prüfsumme ein —
 * über den Schaden, sobald sie wirkt. Ein offenes Fenster, in dem noch nichts
 * gewirkt wurde, war unsichtbar: Zwei Läufe, einer mit laufender Verstärkung
 * und einer ohne, ergaben dieselbe Summe.
 *
 * Für den Abgleich heißt das: Beide Seiten könnten verschieden weit im
 * Fenster stehen, ohne dass es auffällt — bis der nächste Zauber fällt und
 * der Unterschied als Schadensabweichung erscheint, an einer Stelle, die
 * nicht die Ursache ist.
 *
 * Mit den Verstärkungen auf andere Größen (Punktegewinn, Hitze-Aufbau,
 * Tick-Abstand, Stapel-Wertigkeit) wird der Fleck größer statt kleiner: Ein
 * Fehler im Punktegewinn schlägt sich **nie** direkt im Schaden nieder.
 *
 * # Warum beide Läufe denselben Zauber wirken
 *
 * **Ein erster Messversuch verglich `test-buff` gegen `smoulder`.** Die beiden
 * Läufe unterschieden sich damit in zwei Größen, und die Prüfsumme hätte
 * ebensogut den anderen Zauber gesehen haben können. Das Ergebnis sah richtig
 * aus — verschiedene Summen, wie erwartet — und belegte trotzdem nichts.
 *
 * Aufgefallen ist es an den Schadenszahlen: 100 gegen 0.
 */

const BOSS = FIRST_CRUCIBLE.value;

/**
 * Ein Lauf, der `test-buff` wirkt und dann `bisMs` lang den Takt fahren
 * lässt. Das Fenster ist drei Wartezeiten lang (4500 ms).
 */
function nachWirkung(bisMs: number): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
  combat.cast(combat.indexOf('test-buff'), 0);
  for (let t = 0; t <= bisMs; t += 10) combat.stepPassive(t);
  return combat;
}

describe('AC1 — ein offenes Fenster bewegt die Prüfsumme', () => {
  it('zwei Läufe mit demselben Zauber, einer im Fenster, einer danach', () => {
    const imFenster = nachWirkung(1000);
    const danach = nachWirkung(6000);

    // **Die Ursachen zuerst.** Ohne sie bliebe offen, ob die beiden Läufe
    // überhaupt vergleichbar sind — und der Fall unten belegte nur, dass
    // zwei Zahlen verschieden sind.
    expect(imFenster.damageDealt, 'der Schaden unterscheidet sich').toBe(danach.damageDealt);
    expect(imFenster.damageFactors.buffs, 'das Fenster ist nicht offen').toBeGreaterThan(1);
    expect(danach.damageFactors.buffs, 'das Fenster ist noch offen').toBe(1);

    // Beide bei DERSELBEN Zeit gemessen, damit die Uhr nicht der Unterschied
    // ist.
    expect(
      combatChecksum(imFenster, 6000),
      'die Prüfsumme sieht das laufende Fenster nicht — dann können beide ' +
        'Seiten verschieden weit darin stehen, ohne dass der Abgleich es meldet',
    ).not.toBe(combatChecksum(danach, 6000));
  });

  it('Gegenprobe: zwei gleiche Läufe ergeben dieselbe Summe', () => {
    // Ohne diese Zeile hieße die Ungleichheit oben nur „zwei Läufe sind
    // verschieden" — nicht, dass das Fenster die Ursache ist.
    const a = nachWirkung(1000);
    const b = nachWirkung(1000);
    expect(combatChecksum(a, 6000)).toBe(combatChecksum(b, 6000));
  });
});

describe('AC2 — die Summe unterscheidet auch die Zielgrößen', () => {
  it('ein Lauf ohne jede Verstärkung trägt 0', () => {
    // **Der Nullpunkt.** Ohne ihn wäre nicht belegt, dass die Summe
    // überhaupt vom Zustand abhängt und nicht eine Konstante ist.
    const leer = new Combat({ school: CALCINATION.value, boss: BOSS });
    expect(leer.buffWindowSum).toBe(0);
  });

  it('und mit offenem Fenster mehr als 0', () => {
    const offen = nachWirkung(1000);
    expect(offen.buffWindowSum).toBeGreaterThan(0);
  });

  it('nach dem Ablauf wieder 0 — das Fenster wird geräumt', () => {
    // **Sonst wüchse die Summe über den Kampf hinweg und trüge Fenster mit,
    // die längst zu sind.** Der Abgleich meldete dann Unterschiede, die
    // keine sind.
    const abgelaufen = nachWirkung(6000);
    expect(abgelaufen.buffWindowSum).toBe(0);
  });
});
