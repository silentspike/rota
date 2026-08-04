/**
 * VAPOUR und REFLUX — die zwei Zauber der SUBLIMATION (Issue #201).
 *
 * Die Mechanik stand seit E3-3 auf beiden Seiten fertig da, und **kein Zauber
 * nutzte sie**. Der Rust-Prüffall `kein_ausgelieferter_zauber_verkuerzt_etwas`
 * sagte das wörtlich und nannte sich selbst als vorläufig.
 *
 * Hier stehen die Fälle, die er verlangt hat.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { TICK_MS } from '../../src/core/loop.js';
import { PressQueue } from '../../src/input/queue.js';
import { Session } from '../../src/core/session.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';

function neuerKampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Eine Sitzung im Kampf, mit den genannten Zaubern auf den Plätzen 0..n. */
function imKampf(...zauber: string[]): { session: Session; combat: Combat; queue: PressQueue } {
  const combat = neuerKampf();
  const queue = new PressQueue();
  const session = new Session(queue, combat);
  zauber.forEach((id, slot) => session.bind(slot, combat.indexOf(id)));
  session.beginCountdown(0);
  for (let t = 0; t <= COUNTDOWN_MS; t += TICK_MS) session.tick(t);
  return { session, combat, queue };
}

/** Einen Druck auf `slot` zum Zeitpunkt `t` durchreichen. */
function druecken(
  h: { session: Session; queue: PressQueue },
  slot: number,
  t: number,
  tick: number,
): void {
  h.queue.push(slot, tick, t, tick);
  h.session.tick(t);
}

describe('AC1 — VAPOUR verkürzt die Abklingzeiten messbar', () => {
  it('unterscheidet zwei Läufe, die sich NUR in seiner Anwesenheit unterscheiden', () => {
    const vapourReduktion = neuerKampf().spells.find((s) => s.id === 'vapour')?.cooldownReductionMs;
    expect(vapourReduktion).toBeGreaterThan(0);

    // Lauf A: PYRE, dann warten. Lauf B: PYRE, dann VAPOUR.
    // Alles andere ist gleich — derselbe Boss, dieselben Zeitpunkte.
    const ohne = neuerKampf();
    const mit = neuerKampf();
    const pyreA = ohne.indexOf('pyre');
    const pyreB = mit.indexOf('pyre');

    ohne.cast(pyreA, 0);
    mit.cast(pyreB, 0);
    // Der zweite Druck liegt hinter der globalen Wartezeit — sonst wird er
    // abgelehnt, und der Lauf misst nur, dass nichts geschah.
    expect(mit.cast(mit.indexOf('vapour'), GCD_MS).result).toBe(0);

    const restOhne = ohne.cooldownRemainingMs(pyreA, GCD_MS);
    const restMit = mit.cooldownRemainingMs(pyreB, GCD_MS);

    expect(restOhne - restMit).toBe(vapourReduktion);
    // Und die Gegenrichtung: Ohne VAPOUR bewegt sich nichts.
    expect(restOhne).toBe(ohne.spells[pyreA]!.cooldownMs - GCD_MS);
  });

  it('verkürzt ALLE laufenden Abklingzeiten, nicht nur eine', () => {
    const combat = neuerKampf();
    const reduktion = combat.spells.find((s) => s.id === 'vapour')!.cooldownReductionMs!;
    // **Zwei LANGE Abklingzeiten.** Mein erster Entwurf nahm SMOULDER mit 1500
    // ms — bei 2 x GCD war die schon abgelaufen, und die Verkuerzung hatte
    // nichts mehr zu verkuerzen. Der Fall meldete 0 statt 1000, und die
    // Rechnung war die ganze Zeit richtig.
    const temper = combat.indexOf('temper'); // 15 000 ms
    const pyre = combat.indexOf('pyre'); //    5 000 ms

    combat.cast(temper, 0);
    combat.cast(pyre, GCD_MS);
    const vorher = [
      combat.cooldownRemainingMs(temper, 2 * GCD_MS),
      combat.cooldownRemainingMs(pyre, 2 * GCD_MS),
    ];
    expect(vorher.every((rest) => rest > 0)).toBe(true);
    combat.cast(combat.indexOf('vapour'), 2 * GCD_MS);
    const nachher = [
      combat.cooldownRemainingMs(temper, 2 * GCD_MS),
      combat.cooldownRemainingMs(pyre, 2 * GCD_MS),
    ];

    expect(vorher[0]! - nachher[0]!).toBe(reduktion);
    expect(vorher[1]! - nachher[1]!).toBe(reduktion);
    expect(combat.sublimation.shortenedCount).toBe(2);
  });

  it('verkürzt seine EIGENE Abklingzeit nicht', () => {
    // Die Festlegung aus `sublimation.ts`: Täte er es, wäre der Zauber
    // schlicht einer mit kürzerer Abklingzeit.
    const combat = neuerKampf();
    const vapour = combat.indexOf('vapour');
    combat.cast(vapour, 0);
    expect(combat.cooldownRemainingMs(vapour, 0)).toBe(combat.spells[vapour]!.cooldownMs);
  });
});

describe('AC2 — REFLUX setzt den NÄCHSTEN gewirkten Zauber zurück', () => {
  it('trifft, was danach gedrückt wird — und zwar irgendeinen, nicht einen bestimmten', () => {
    // **Zweimal derselbe Ablauf mit verschiedenem Nachdruck.** Ein Prüffall,
    // der REFLUX vor einem festen Zauber zündet, belegt nicht, dass er
    // *irgendeinen* trifft — er könnte auf genau diesen verdrahtet sein.
    for (const ziel of ['pyre', 'smoulder']) {
      const h = imKampf('kindle', ziel, 'reflux');
      const zielIndex = h.combat.indexOf(ziel);
      let t = COUNTDOWN_MS;
      let tick = t / TICK_MS;

      // Das Ziel wirken, damit es überhaupt auf Abklingzeit liegt.
      druecken(h, 1, t, tick);
      expect(h.combat.cooldownRemainingMs(zielIndex, t)).toBeGreaterThan(0);

      // REFLUX wirken, dann das Ziel wählen.
      t += GCD_MS;
      tick = t / TICK_MS;
      druecken(h, 2, t, tick);
      expect(h.session.isResetArmed, `${ziel}: nach REFLUX scharf`).toBe(true);

      t += GCD_MS;
      tick = t / TICK_MS;
      druecken(h, 1, t, tick);
      expect(h.session.isResetArmed, `${ziel}: Auswahl verbraucht`).toBe(false);
      expect(h.combat.cooldownRemainingMs(zielIndex, t), `${ziel}: zurückgesetzt`).toBe(0);
    }
  });

  it('tut NICHTS, wenn danach nichts mehr gewirkt wird — der Gegenfall', () => {
    const h = imKampf('kindle', 'pyre', 'reflux');
    const pyre = h.combat.indexOf('pyre');
    let t = COUNTDOWN_MS;

    druecken(h, 1, t, t / TICK_MS);
    const restNachWirken = h.combat.cooldownRemainingMs(pyre, t);
    expect(restNachWirken).toBeGreaterThan(0);

    t += GCD_MS;
    druecken(h, 2, t, t / TICK_MS);
    expect(h.session.isResetArmed).toBe(true);

    // Und jetzt: nichts mehr drücken, nur die Uhr laufen lassen.
    for (let u = t; u <= t + 5000; u += TICK_MS) h.session.tick(u);

    // Der Modus steht weiter scharf, und KEINE Abklingzeit ist gesprungen —
    // sie ist nur normal weitergelaufen.
    expect(h.session.isResetArmed).toBe(true);
    expect(h.combat.cooldownRemainingMs(pyre, t + 5000)).toBe(
      Math.max(0, restNachWirken - 5000 - GCD_MS),
    );
  });
});

describe('Die Reihenfolge im Auswahlmodus (Worker-1s Hinweis)', () => {
  it('stellt erst NACH der eigenen Wirkung scharf — sonst wählte REFLUX sich selbst', () => {
    // **Diese Regel hielt bis heute kein Prüffall**, und sie konnte es nicht:
    // Sie braucht einen Rücksetzer-Zauber, und den gab es nicht.
    //
    // `session.ts` setzt `resetArmedFor` NACH `combat.cast(...)`. Zöge jemand
    // es davor, wäre `chose` schon beim eigenen Druck wahr — REFLUX würde als
    // Auswahl verbraucht, statt gewirkt zu werden.
    //
    // Beides ist am Ergebnis unterscheidbar, und zwar ohne in den Code zu
    // sehen: Wird REFLUX gewirkt, liegt er danach auf seiner Abklingzeit und
    // zählt als angenommene Wirkung. Wird er als Auswahl verbraucht, tut er
    // beides nicht.
    const h = imKampf('kindle', 'pyre', 'reflux');
    const reflux = h.combat.indexOf('reflux');
    const t = COUNTDOWN_MS;

    // `castCount` und die Abklingzeit statt des Zaehlers `accepted`: der ist
    // privat, und ein Prueffall, der daran vorbeigreift, faellt erst in der
    // Typpruefung auf — vitest prueft keine Typen. Genau so ist es mir hier
    // passiert.
    druecken(h, 2, t, t / TICK_MS);

    expect(h.combat.castCount(reflux), 'REFLUX wurde gewirkt').toBe(1);
    expect(h.combat.cooldownRemainingMs(reflux, t), 'und liegt auf Abklingzeit').toBeGreaterThan(0);
    // Erst danach steht der Modus scharf.
    expect(h.session.isResetArmed).toBe(true);
  });
});
