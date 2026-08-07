import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * WEDGE wandelt Kontroll-Punkte in Schaden (Issue #478).
 *
 * # Warum dieser Zauber der wichtigste der acht ist
 *
 * FIXATION ist heute gegen einen Boss **ohne Mechanik** wertlos: Es gibt
 * nichts zu verlangsamen, zu verzögern oder hinauszuzögern, und drei der vier
 * Zauber machen keinen Schaden. Wer die Schule dort spielt, hat eine Rotation
 * aus einem einzigen Zauber.
 *
 * WEDGE ist der Umweg von Kontrolle zu Schaden — er macht die Schule überall
 * spielbar, ohne ihren Charakter zu ändern. Das ist keine Balance-Zahl,
 * sondern eine Lücke in der Mechanik.
 */

const BOSS = FIRST_CRUCIBLE.value;

/** Ein Kampf mit aufgebauten Kontroll-Punkten. */
function mitPunkten(): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
  const bind = combat.indexOf('bind');
  let t = 0;
  for (let n = 0; n < 5; n++) {
    combat.cast(bind, t);
    t += 1500;
  }
  return combat;
}

describe('AC3 — ohne WEDGE ändert sich die gemessene Größe nicht', () => {
  it('WEDGE teilt Schaden aus, obwohl er keinen Grundschaden hat', () => {
    const c = mitPunkten();
    const punkte = c.control;
    expect(punkte, 'ohne Punkte misst der Fall nichts').toBeGreaterThan(0);

    const vorher = c.damageDealt;
    c.cast(c.indexOf('wedge'), 10000);

    expect(
      c.damageDealt - vorher,
      'WEDGE hat keinen Schaden ausgeteilt — dann ist der Umweg von Kontrolle ' +
        'zu Schaden nicht angeschlossen',
    ).toBeGreaterThan(0);
  });

  it('die Gegenprobe: OHNE Punkte teilt derselbe Zauber nichts aus', () => {
    // **Ohne diese Zeile belegte der Fall oben nichts.** Er zeigte nur, dass
    // irgendein Schaden fällt — nicht, dass die Punkte die Quelle sind.
    const c = new Combat({ school: CALCINATION.value, boss: BOSS });
    expect(c.control, 'der Kampf startet mit Punkten').toBe(0);

    const vorher = c.damageDealt;
    c.cast(c.indexOf('wedge'), 0);

    expect(
      c.damageDealt - vorher,
      'WEDGE teilt ohne Kontroll-Punkte Schaden aus — dann kommt er nicht ' + 'aus der Umwandlung',
    ).toBe(0);
  });

  it('mehr Punkte ergeben mehr Schaden — die Menge ist die Quelle', () => {
    // **Der schärfere Nachweis.** Zwei Läufe, gleicher Zauber, nur ein
    // anderer Vorrat: Was sich dann unterscheidet, kann nur die Umwandlung
    // sein.
    const wenig = new Combat({ school: CALCINATION.value, boss: BOSS });
    const bind = wenig.indexOf('bind');
    wenig.cast(bind, 0);
    wenig.cast(bind, 1500);

    const viel = mitPunkten();
    expect(viel.control).toBeGreaterThan(wenig.control);

    const a = wenig.damageDealt;
    wenig.cast(wenig.indexOf('wedge'), 10000);
    const b = viel.damageDealt;
    viel.cast(viel.indexOf('wedge'), 10000);

    expect(viel.damageDealt - b).toBeGreaterThan(wenig.damageDealt - a);
  });
});

describe('Alle Punkte gehen mit, nicht nur die Kosten', () => {
  it('der Vorrat ist danach leer', () => {
    // **Sonst wäre „aufsparen" sinnlos.** `cost` ist der Mindesteinsatz; was
    // darüber liegt, geht mit. Bliebe der Rest liegen, brächte WEDGE bei
    // vollem Vorrat genauso viel wie bei knappem.
    const c = mitPunkten();
    c.cast(c.indexOf('wedge'), 10000);
    expect(c.control).toBe(0);
  });

  it('und der Schaden entspricht ALLEN Punkten mal dem Satz je Punkt', () => {
    const c = mitPunkten();
    const punkte = c.control;
    const vorher = c.damageDealt;

    c.cast(c.indexOf('wedge'), 10000);

    // 8 Schaden je Punkt (Platzhalter aus dem Raster), ohne Faktoren im
    // neutralen Kampf.
    expect(c.damageDealt - vorher).toBe(punkte * 8);
  });
});

describe('TETHER — der zweite Schadenszauber der Schule (#478)', () => {
  it('teilt Schaden aus UND gibt Kontroll-Punkte', () => {
    // Bis hierher machte nur `bind` Schaden; wer FIXATION spielte, hatte eine
    // Rotation aus einem Zauber und drei Werkzeugen.
    const c = new Combat({ school: CALCINATION.value, boss: BOSS });
    const vorher = c.damageDealt;

    c.cast(c.indexOf('tether'), 0);

    expect(c.damageDealt - vorher, 'TETHER teilt keinen Schaden aus').toBeGreaterThan(0);
    expect(c.control, 'TETHER gibt keine Kontroll-Punkte').toBeGreaterThan(0);
  });

  it('die Gegenprobe: ein Ausgeber tut beides nicht', () => {
    // Ohne diese Zeile zeigte der Fall oben nur, dass irgendein Zauber
    // wirkt — nicht, dass TETHER die Lücke füllt, die die Ausgeber lassen.
    const c = new Combat({ school: CALCINATION.value, boss: BOSS });
    const bind = c.indexOf('bind');
    for (let n = 0; n < 5; n++) c.cast(bind, n * 1500);
    const vorher = c.damageDealt;

    c.cast(c.indexOf('stall'), 10000);

    expect(c.damageDealt - vorher, 'STALL teilt Schaden aus — dann ist er kein Ausgeber').toBe(0);
  });
});

describe('CAPSTONE — alle drei Wirkungen auf einmal (#478)', () => {
  it('setzt den Phasen-Halt, den sonst nur ANCHOR setzt', () => {
    const c = new Combat({ school: CALCINATION.value, boss: BOSS });
    const bind = c.indexOf('bind');
    for (let n = 0; n < 8; n++) c.cast(bind, n * 1500);

    c.cast(c.indexOf('capstone'), 20000);

    expect(c.phaseHoldPercent, 'CAPSTONE hat den Phasen-Halt nicht gesetzt').toBeGreaterThan(0);
  });

  it('und verbraucht ALLE Punkte, nicht nur die Kosten', () => {
    // **Sonst wäre der große Moment der Schule bei vollem Vorrat genauso
    // viel wert wie bei knappem.** `cost` ist der Mindesteinsatz.
    const c = new Combat({ school: CALCINATION.value, boss: BOSS });
    const bind = c.indexOf('bind');
    for (let n = 0; n < 8; n++) c.cast(bind, n * 1500);
    const punkte = c.control;
    expect(punkte, 'zu wenig Punkte für den Fall').toBeGreaterThan(8);

    c.cast(c.indexOf('capstone'), 20000);

    expect(c.control).toBe(0);
  });

  it('die Gegenprobe: ohne genug Punkte passiert nichts', () => {
    const c = new Combat({ school: CALCINATION.value, boss: BOSS });
    c.cast(c.indexOf('bind'), 0);

    c.cast(c.indexOf('capstone'), 5000);

    expect(c.phaseHoldPercent, 'CAPSTONE wirkte ohne genug Punkte').toBe(0);
  });
});
