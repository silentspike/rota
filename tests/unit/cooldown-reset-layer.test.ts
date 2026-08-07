/**
 * Der Cooldown-Rücksetzer liegt in `Combat` (Issue #528).
 *
 * # Warum diese Datei nicht in `session.test.ts` steht
 *
 * Bis #528 lag die Regel in `src/core/session.ts` — im Spiel eine Schicht über
 * ihrer eigenen Wirkung, im Simulator dagegen im Kampf. Die Reihenfolge war auf
 * beiden Seiten sorgfältig gleichgehalten, die **Schicht** nicht.
 *
 * Das war kein Schönheitsfehler: **Jede Aufzeichnung fährt `Combat` direkt**
 * (`tests/parity/payloads.ts` — `new Session` kommt in keiner vor). Eine Regel
 * ausserhalb von `Combat` ist für die Parität deshalb nicht schwer prüfbar,
 * sondern **unsichtbar**. Der Bruch fiel erst auf, als jemand REFLUX in eine
 * Aufzeichnung nehmen wollte: Prüfsumme ab Takt 150 auseinander, 880 gegen 640.
 *
 * Diese Fälle fahren `Combat` ohne `Session` — genau so, wie eine Aufzeichnung
 * es tut. Liefe die Regel wieder in die Sitzung zurück, würden sie rot.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('AC4 — der Rücksetzer wirkt auf den GEWÄHLTEN Zauber, nicht auf einen festen', () => {
  it('dieselbe Ausgangslage, zwei verschiedene Wahlen, zwei verschiedene Ergebnisse', () => {
    // **Das ist der Kern.** Ein Rücksetzer, der auf „den nächsten" oder auf
    // einen fest verdrahteten Zauber wirkte, käme in beiden Läufen zum selben
    // Ergebnis. Nur weil die WAHL entscheidet, gehen sie auseinander.
    const a = kampf();
    const b = kampf();
    for (const c of [a, b]) {
      // **Zwei Zauber mit LANGER Abklingzeit** (crust 6000, pyre 5000). Mit
      // SMOULDER (1500) wäre der Fall zur Wahlzeit schon wieder frei, die Wahl
      // verpuffte, und der Fall prüfte nur, dass verpuffen verpufft.
      c.cast(c.indexOf('crust'), 0);
      c.cast(c.indexOf('pyre'), 1500);
      c.cast(c.indexOf('reflux'), 3000);
      expect(c.resetArmed, 'REFLUX muss den Auswahlmodus scharf stellen').toBe(true);
    }

    // Beide haben CRUST und PYRE auf Abklingzeit. Jeder wählt einen anderen.
    expect(a.chooseForReset(a.indexOf('crust'), 4500)).toBe(true);
    expect(b.chooseForReset(b.indexOf('pyre'), 4500)).toBe(true);

    expect(a.isOffCooldown(a.indexOf('crust'), 4500), 'a hat CRUST frei').toBe(true);
    expect(a.isOffCooldown(a.indexOf('pyre'), 4500), 'a hat PYRE NICHT frei').toBe(false);
    expect(b.isOffCooldown(b.indexOf('pyre'), 4500), 'b hat PYRE frei').toBe(true);
    expect(b.isOffCooldown(b.indexOf('crust'), 4500), 'b hat CRUST NICHT frei').toBe(false);
  });

  it('die Wahl verbraucht den Rücksetzer — auch eine, die verpufft', () => {
    // Ein Zauber, der gar nicht auf Abklingzeit liegt: Die Wahl ist vertan,
    // der Modus trotzdem weg. Der Auftraggeber hat das ausdrücklich so
    // gewollt; die einfachere Regel gewinnt über die gnädigere.
    const c = kampf();
    c.cast(c.indexOf('reflux'), 0);
    expect(c.resetArmed).toBe(true);
    expect(c.chooseForReset(c.indexOf('kindle'), 1500), 'KINDLE lag nicht auf Abklingzeit').toBe(
      false,
    );
    expect(c.resetArmed, 'verbraucht ist verbraucht').toBe(false);
  });

  it('und er wirkt NIE auf sich selbst', () => {
    // Sonst liesse er sich endlos ketten — der erste Zug, den eine
    // Optimums-Suche findet.
    const c = kampf();
    c.cast(c.indexOf('reflux'), 0);
    expect(c.chooseForReset(c.indexOf('reflux'), 1500)).toBe(false);
    expect(c.isOffCooldown(c.indexOf('reflux'), 1500), 'REFLUX bleibt auf Abklingzeit').toBe(false);
  });
});

describe('Die Regel liegt in Combat — und das ist prüfbar, ohne Session', () => {
  it('ein Kampf ohne Sitzung kennt den Auswahlmodus', () => {
    // Genau das ging vor #528 nicht: `Combat` wusste nichts davon, und eine
    // Aufzeichnung sah die Regel deshalb nie.
    const c = kampf();
    expect(c.resetArmed).toBe(false);
    c.cast(c.indexOf('reflux'), 0);
    expect(c.resetArmed).toBe(true);
    expect(c.resetArmedFor).toBe(c.indexOf('reflux'));
  });

  it('und der Auswahlmodus endet mit dem Kampf', () => {
    const c = kampf();
    c.cast(c.indexOf('reflux'), 0);
    c.reset();
    expect(c.resetArmed, 'ein neuer Kampf beginnt ohne scharfen Modus').toBe(false);
  });
});
