import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';

/**
 * SUBLIMATIONs Schlüsseltalent (Issue #419, Bauart aus #418).
 *
 * # Was es ändert — und warum das kein Faktor ist
 *
 * Ein Rücksetzer schaltet normalerweise einen **Auswahlmodus** scharf: Der
 * nächste Tastendruck bestimmt, welche Abklingzeit fällt. Das kostet eine
 * zusätzliche Handlung und eine Entscheidung unter Zeitdruck.
 *
 * Mit dem Talent entfällt die Wahl. Der Rücksetzer trifft von selbst den
 * Zauber mit der **längsten Restzeit**.
 *
 * Der Nachweis dafür ist nicht eine andere Schadenszahl, sondern ein anderer
 * ZUSTAND: `resetArmedFor` steht ohne Talent auf dem Rücksetzer und mit
 * Talent auf −1, und die Abklingzeit ist sofort weg statt nach einem weiteren
 * Druck. Genau das verlangt #418 von einem Schlüsseltalent — ein Talent, das
 * sich als Zahl ausdrücken ließe, wäre keines.
 */

const MIT_TALENT = { 'sublimation-keystone': 1 };

function neu(talents?: Record<string, number>): Combat {
  return new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
}

/** Der Rücksetzer der Schule — sein Wirken stellt sonst den Auswahlmodus scharf. */
function ruecksetzer(combat: Combat): number {
  for (let i = 0; i < combat.spells.length; i++) {
    if ((combat.spells[i]?.cooldownResetCharges ?? 0) > 0) return i;
  }
  return -1;
}

describe('Die Zusicherung vor der Messung', () => {
  it('es GIBT einen Rücksetzer auf der Leiste', () => {
    // Ohne ihn prüfte alles darunter eine Sache, die im Bestand nicht
    // vorkommt — und wäre grün, weil nie etwas geschieht.
    const c = neu();
    expect(ruecksetzer(c), 'kein Zauber trägt cooldownResetCharges').toBeGreaterThanOrEqual(0);
  });

  it('und mindestens ein anderer Zauber steht danach auf Abklingzeit', () => {
    const c = neu();
    const reset = ruecksetzer(c);
    const pyre = c.indexOf('pyre');
    c.cast(alsSpellIndex(pyre), 0);
    expect(
      c.cooldownRemainingMs(pyre, GCD_MS),
      'PYRE steht nicht auf Abklingzeit — dann gibt es nichts zurückzusetzen',
    ).toBeGreaterThan(0);
    expect(reset).toBeGreaterThanOrEqual(0);
  });
});

describe('AC2 — das Talent ändert das VERHALTEN, nicht eine Zahl', () => {
  it('ohne Talent steht der Auswahlmodus scharf und die Abklingzeit bleibt', () => {
    const c = neu();
    const reset = ruecksetzer(c);
    const pyre = c.indexOf('pyre');

    c.cast(alsSpellIndex(pyre), 0);
    const vorher = c.cooldownRemainingMs(pyre, GCD_MS);
    c.cast(alsSpellIndex(reset), GCD_MS);
    c.armResetIfNeeded(reset, GCD_MS);

    expect(c.resetArmedFor, 'ohne Talent muss der Auswahlmodus scharf stehen').toBe(reset);
    expect(
      c.cooldownRemainingMs(pyre, GCD_MS),
      'ohne Talent darf die Abklingzeit noch NICHT weg sein — es fehlt die Wahl',
    ).toBe(vorher);
  });

  it('mit Talent entfällt die Wahl und die längste Restzeit ist sofort weg', () => {
    const c = neu(MIT_TALENT);
    const reset = ruecksetzer(c);
    const pyre = c.indexOf('pyre');

    c.cast(alsSpellIndex(pyre), 0);
    expect(c.cooldownRemainingMs(pyre, GCD_MS)).toBeGreaterThan(0);

    c.cast(alsSpellIndex(reset), GCD_MS);
    c.armResetIfNeeded(reset, GCD_MS);

    expect(c.resetArmedFor, 'mit Talent darf KEIN Auswahlmodus offen sein').toBe(-1);
    expect(
      c.cooldownRemainingMs(pyre, GCD_MS),
      'mit Talent muss die längste Restzeit sofort weg sein',
    ).toBe(0);
  });

  it('und es trifft die LÄNGSTE Restzeit, nicht irgendeine', () => {
    // **Der Unterschied zwischen „es tut etwas" und „es tut das Richtige".**
    // Zwei Zauber auf Abklingzeit, verschieden lang — das Talent muss den
    // teureren treffen, sonst wäre es eine Zufallswahl mit gutem Namen.
    // **Zwei Zauber mit derselben Abklingzeit, ZEITVERSETZT gewirkt.** Der
    // erste Entwurf nahm PYRE und SMOULDER — und SMOULDER hat gar keine
    // Abklingzeit (gemessen: 0 ms). „Die kürzere" war deshalb immer schon
    // abgelaufen, und der Fall brach am eigenen Aufbau statt am Talent.
    const c = neu(MIT_TALENT);
    const reset = ruecksetzer(c);
    const frueh = c.indexOf('pyre');
    const spaet = c.indexOf('quench');

    c.cast(alsSpellIndex(frueh), 0);
    c.cast(alsSpellIndex(spaet), GCD_MS);
    const restFrueh = c.cooldownRemainingMs(frueh, 2 * GCD_MS);
    const restSpaet = c.cooldownRemainingMs(spaet, 2 * GCD_MS);
    expect(restFrueh, 'der frühere steht nicht mehr auf Abklingzeit').toBeGreaterThan(0);
    expect(
      restFrueh === restSpaet,
      `beide Restzeiten sind gleich (${restFrueh}) — der Fall unterscheidet dann nichts`,
    ).toBe(false);

    const laenger = restFrueh > restSpaet ? frueh : spaet;
    const kuerzer = restFrueh > restSpaet ? spaet : frueh;

    c.cast(alsSpellIndex(reset), 2 * GCD_MS);
    c.armResetIfNeeded(reset, 2 * GCD_MS);

    expect(c.cooldownRemainingMs(laenger, 2 * GCD_MS), 'die längere wurde nicht getroffen').toBe(0);
    expect(
      c.cooldownRemainingMs(kuerzer, 2 * GCD_MS),
      'die kürzere wurde mitgenommen — es trifft genau EINEN',
    ).toBeGreaterThan(0);
  });
});
