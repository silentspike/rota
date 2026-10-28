/**
 * Der Auswahlmodus der Rücksetzer steht in der Prüfsumme (#419, Nachbesserung).
 *
 * # Was fehlte
 *
 * Ein Rücksetzer (REFLUX, ALEMBIC) wählt nicht selbst, er stellt einen Modus
 * scharf: Der nächste Druck bestimmt, welche Abklingzeit fällt. Der Modus ist
 * Kampfzustand, der zwischen zwei Takten lebt — wer scharf gestellt hat
 * (`resetArmedFor`, wichtig für „der Rücksetzer wählt sich nicht selbst") und
 * wie viele Auswahlen offen sind (`resetChargesLeft`).
 *
 * Beides stand in keiner Prüfsumme. Dieselbe Lückenart wie beim Zustand der
 * Schlüsseltalente: Führte eine Seite den Modus anders, sähe der Abgleich es
 * erst an der Abklingzeit, die danach fällt — oder nicht fällt.
 *
 * # Nur bei scharfem Modus
 *
 * Steht er nicht scharf, steht im Spiel -1 und 0, im Simulator dasselbe;
 * eingemischt änderten die zwei Werte trotzdem jede Prüfsumme jedes Kampfes.
 * Deshalb gehen sie nur ein, solange der Modus scharf steht — und es ändern
 * sich genau die Aufzeichnungen, in denen er das irgendwann tut.
 *
 * Gemessen wie in `schluesseltalent-pruefsumme.test.ts`: ein Zustand, eine
 * gezielte Störung, und die positive Kontrolle, dass die Störung den weiteren
 * Verlauf ändert.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, isCastAccepted } from '../../src/core/combat.js';
import { combatChecksum, recordFieldOrder } from '../../src/core/checksum.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Die privaten Felder des Modus — nur für diesen Prüffall geöffnet. */
interface Modus {
  resetArmedForValue: number;
  resetChargesLeftValue: number;
}

function innen(combat: Combat): Modus {
  return combat as unknown as Modus;
}

/** CRUST bei 0 ms (Abklingzeit 6000), ALEMBIC bei 1500 ms: zwei Auswahlen offen. */
const MESSUNG_MS = 1500;

function scharf(): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const plan = new Map([
    [0, 'crust'],
    [MESSUNG_MS, 'alembic'],
  ]);
  for (let t = 0; t <= MESSUNG_MS; t += TICK_MS) {
    combat.stepPassive(t);
    const id = plan.get(t);
    if (id === undefined) continue;
    const zauber = alsSpellIndex(combat.indexOf(id));
    const ergebnis = combat.cast(zauber, t).result;
    if (!isCastAccepted(ergebnis)) throw new Error(`${id} bei ${String(t)} ms abgewiesen`);
    // Wie die Sitzung (`Session.apply`) und `alembicPayload`: Scharf gestellt
    // wird NACH der Wirkung, nicht in `cast`.
    combat.armResetIfNeeded(zauber, t);
  }
  return combat;
}

describe('Der Auswahlmodus steht in der Prüfsumme', () => {
  it('die Messung trifft einen scharfen Modus mit zwei offenen Auswahlen', () => {
    const combat = scharf();
    expect(combat.resetArmed, 'ALEMBIC stellt nicht scharf').toBe(true);
    expect(combat.resetArmedFor).toBe(combat.indexOf('alembic'));
    expect(combat.resetChargesLeft).toBe(2);
    expect(combat.cooldownRemainingMs(combat.indexOf('crust'), MESSUNG_MS)).toBeGreaterThan(0);
  });

  it('wer scharf gestellt hat, ändert die Prüfsumme', () => {
    const echt = scharf();
    const verstellt = scharf();
    const crust = echt.indexOf('crust');
    innen(verstellt).resetArmedForValue = crust;

    expect(
      combatChecksum(verstellt, MESSUNG_MS),
      'zwei Kämpfe, scharf gestellt von verschiedenen Zaubern, ergeben dieselbe ' +
        'Prüfsumme — resetArmedFor geht nicht in combatChecksum ein',
    ).not.toBe(combatChecksum(echt, MESSUNG_MS));

    // Positive Kontrolle: Die Wahl von CRUST setzt im echten Kampf zurück, im
    // verstellten verpufft sie — der Rücksetzer wählt sich nicht selbst.
    expect(echt.chooseForReset(alsSpellIndex(crust), MESSUNG_MS)).toBe(true);
    expect(verstellt.chooseForReset(alsSpellIndex(crust), MESSUNG_MS)).toBe(false);
  });

  it('die Zahl der offenen Auswahlen ändert die Prüfsumme', () => {
    const echt = scharf();
    const verstellt = scharf();
    innen(verstellt).resetChargesLeftValue = 1;

    expect(
      combatChecksum(verstellt, MESSUNG_MS),
      'zwei Kämpfe mit verschieden vielen offenen Auswahlen ergeben dieselbe ' +
        'Prüfsumme — resetChargesLeft geht nicht in combatChecksum ein',
    ).not.toBe(combatChecksum(echt, MESSUNG_MS));

    // Positive Kontrolle: Nach einer Wahl steht nur der echte noch scharf.
    const crust = alsSpellIndex(echt.indexOf('crust'));
    echt.chooseForReset(crust, MESSUNG_MS);
    verstellt.chooseForReset(crust, MESSUNG_MS);
    expect(echt.resetArmed).toBe(true);
    expect(verstellt.resetArmed).toBe(false);
  });

  it('scharf stehen die zwei Namen am Ende, sonst fehlen sie', () => {
    const ohne = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    const mit = scharf();
    const folgeOhne = recordFieldOrder(() => combatChecksum(ohne, MESSUNG_MS));
    const folgeMit = recordFieldOrder(() => combatChecksum(mit, MESSUNG_MS));
    expect(folgeMit.slice(0, folgeOhne.length)).toEqual(folgeOhne);
    expect(folgeMit.slice(folgeOhne.length)).toEqual(['resetArmedFor', 'resetChargesLeft']);
  });

  it('nach der letzten Auswahl gehen sie nicht mehr ein', () => {
    // **Der Modus fällt, und mit ihm die zwei Felder.** Sonst trüge jeder
    // Kampf nach einem Rücksetzer bis zum Ende zwei Werte mehr als einer, der
    // nie einen gewirkt hat — bei gleichem Zustand.
    const combat = scharf();
    const crust = alsSpellIndex(combat.indexOf('crust'));
    combat.chooseForReset(crust, MESSUNG_MS);
    combat.chooseForReset(crust, MESSUNG_MS);
    expect(combat.resetArmed, 'zwei Auswahlen lassen den Modus stehen').toBe(false);
    const ohne = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    expect(recordFieldOrder(() => combatChecksum(combat, MESSUNG_MS))).toEqual(
      recordFieldOrder(() => combatChecksum(ohne, MESSUNG_MS)),
    );
  });
});
