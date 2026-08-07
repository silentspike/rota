import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * SLAG hebt die Hitze-Grenze befristet an (Issue #490).
 *
 * # Warum das eine Entscheidung erzeugt
 *
 * Die Obergrenze war fest: Wer sie erreicht, verschwendet jeden weiteren
 * Aufbau. SLAG macht daraus eine Wahl — vor einem Entlader die Grenze heben
 * und mehr sammeln, statt am Deckel zu verlieren.
 *
 * # Die eine Rechenstelle
 *
 * `resource.cap` wurde an **zwei** Orten gelesen: beim Auffüllen und beim
 * Setzen (Entwickler-Werkzeug). Ein Faktor an nur einem hätte eine Grenze
 * ergeben, die beim Auffüllen anders ist als beim Setzen — und der
 * Unterschied fiele erst auf, wenn jemand das Werkzeug benutzt.
 */

const BOSS = FIRST_CRUCIBLE.value;
const CAP = CALCINATION.value.resource.cap;

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: BOSS });
}

/**
 * Füllt den Vorrat bis an die Grenze und liefert den HÖCHSTSTAND.
 *
 * **BELLOWS, nicht KINDLE.** KINDLE gibt 3 Hitze je Wurf, der Verfall nimmt
 * 1 je 2000 ms — netto etwa 2 pro Wartezeit. Bis 20 wären das über zehn
 * Würfe, und ein erster Entwurf mit acht kam nur auf 19.
 *
 * Der Fall hätte damit zwei Zahlen UNTER der Grenze verglichen und wäre
 * grün gewesen mit der Aussage "SLAG wirkt nicht" — ein erfundener Fehler.
 * BELLOWS gibt 8 und erreicht die Grenze in drei Würfen.
 *
 * Der Höchststand, nicht der Endstand: Der Vorrat verfällt zwischen den
 * Würfen, und die Frage ist, wie hoch er KOMMT.
 */
function hoechsterVorrat(c: Combat, abMs: number): number {
  const bellows = c.indexOf('bellows');
  let hoch = c.resource;
  let t = abMs;
  for (let n = 0; n < 6; n++) {
    c.cast(bellows, t);
    hoch = Math.max(hoch, c.resource);
    t += 1500;
    for (let s = t - 1500; s <= t; s += 10) {
      c.stepPassive(s);
      hoch = Math.max(hoch, c.resource);
    }
  }
  return hoch;
}

describe('AC1 — die Grenze steigt, solange das Fenster offen ist', () => {
  it('ohne SLAG bleibt der Vorrat bei der Grenze stehen', () => {
    const c = neu();
    expect(hoechsterVorrat(c, 0), 'ohne SLAG über die Grenze — dann misst der Fall nichts').toBe(
      CAP,
    );
  });

  it('mit SLAG steigt er darüber', () => {
    const c = neu();
    c.cast(c.indexOf('slag'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
    expect(hoechsterVorrat(c, 1600), 'SLAG hebt die Grenze nicht').toBeGreaterThan(CAP);
  });
});

describe('AC3 — die Gegenprobe: ein anderer Zauber hebt sie nicht', () => {
  it('KINDLE allein lässt die Grenze, wo sie ist', () => {
    // **Ohne diese Zeile belegte AC1 nur, dass irgendein Druck etwas tut.**
    const c = neu();
    c.cast(c.indexOf('kindle'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
    expect(
      hoechsterVorrat(c, 1600),
      'die Grenze steigt ohne SLAG — dann kommt es nicht von ihm',
    ).toBe(CAP);
  });
});

describe('Die anderen Zielgrößen bleiben unberührt', () => {
  it('SLAG bewegt den Schadens-Faktor nicht', () => {
    const c = neu();
    c.cast(c.indexOf('slag'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
    expect(c.damageFactors.buffs, 'SLAG hat den Schaden bewegt').toBe(1);
  });
});
