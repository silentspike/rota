/**
 * EMBER, ROT und EFFLUX — die letzten drei Größen der Vollmessung
 * (Issue #498, #540).
 *
 * # Warum drei Schulen in einer Datei
 *
 * Sie haben fachlich nichts miteinander zu tun — sie sind das, was von der
 * Vollmessung aller 116 Feld-Zauber-Paare übrig blieb:
 *
 * ```text
 *   resourceGain.heat   ember    2 -> 1     NUR FRISCHE
 *   dot.tickDamage      rot     25 -> 12    NUR FRISCHE
 *   convert.perSecond   efflux  12 -> 6     NUR FRISCHE
 * ```
 *
 * Jede ist ein Einzelfall in ihrer Schule; drei eigene Dateien mit je zwei
 * Fällen wären mehr Ablage als Inhalt. Wer eine davon sucht, findet sie über
 * den Zaubernamen.
 *
 * # Bei ROT ist der ERSTE Tick der Messpunkt, nicht die Summe
 *
 * Gemessen über sechs Sekunden:
 *
 * ```text
 *   nach   1000   2000   3000   4000   5000 ms
 *   Summe    25     52     80    110    110
 *   Zuwachs  25     27     28     30      0
 * ```
 *
 * Die Ticks werden **größer**, obwohl `tickDamage` fest ist: `decayPerTick`
 * legt mit jedem Tick einen Stapel auf den Boss, und der hebt die
 * Verwundbarkeit. Nur der erste Tick fällt auf einen unbelasteten Boss und
 * zeigt die Zahl aus der Datei unverfälscht.
 *
 * Die Summe steht trotzdem als zweiter Fall da — sie fängt eine Änderung, die
 * nur die späteren Ticks beträfe.
 *
 * # Alle drei Zeilen „halbiert" sind gemessen
 *
 * ```text
 *                       Bestand   halbiert
 *   EMBER, resource         2         1
 *   ROT, erster Tick       25        12
 *   ROT, Summe bei 4000   110        52
 *   EFFLUX, 18 s Restzeit 216       108
 * ```
 *
 * Aus Läufen mit mutierter Datei, nicht gerechnet. Bei EFFLUX ist das nicht
 * bloß Sorgfalt: `Math.floor(restMs / 1000)` schneidet ab, und eine Zahl, die
 * man sich herleitet, geht dort leicht um einen Schritt daneben.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('EMBER bringt zwei Hitze (resourceGain.heat)', () => {
  it('aus null werden zwei', () => {
    const combat = neu();
    combat.stepPassive(0);
    expect(combat.resource, 'der Vorrat startet nicht bei null').toBe(0);
    expect(combat.cast(alsSpellIndex(combat.indexOf('ember')), 0).result).toBe(CAST_OK);
    expect(combat.resource, 'EMBER bringt nicht zwei Hitze').toBe(2);
  });
});

describe('ROT tickt mit 25 (dot.tickDamage)', () => {
  /** Schaden aus ROT allein, an mehreren Zeitpunkten. */
  function schadenNach(): Record<number, number> {
    const combat = neu();
    const rot = alsSpellIndex(combat.indexOf('rot'));
    combat.stepPassive(0);
    expect(combat.cast(rot, 0).result, 'ROT kam nicht zustande').toBe(CAST_OK);

    const werte: Record<number, number> = {};
    for (let timeMs = 0; timeMs <= 6000; timeMs += TICK_MS) {
      combat.stepPassive(timeMs);
      if (timeMs % 1000 === 0) werte[timeMs] = combat.damageDealt;
    }
    return werte;
  }

  const WERTE = schadenNach();

  it('der erste Tick ist genau 25 — er fällt auf einen unbelasteten Boss', () => {
    // **Der saubere Messpunkt.** Ab dem zweiten Tick liegt schon ein Stapel
    // aus `decayPerTick` auf dem Boss, und die Verwundbarkeit hebt den
    // Schaden: 25, 27, 28, 30 statt viermal 25.
    expect(WERTE[1000], 'ROTs erster Tick ist nicht 25').toBe(25);
  });

  it('ROT richtet beim Wirken selbst keinen Schaden an', () => {
    // Die Zusicherung zur Zeile darüber: Wäre `baseDamage` nicht 0, steckte in
    // den 25 mehr als ein Tick.
    expect(WERTE[0], 'ROT richtet schon beim Wirken Schaden an').toBe(0);
  });

  it('die Summe über alle vier Ticks steht bei 110', () => {
    // Fängt eine Änderung, die nur die späteren Ticks beträfe — der Fall oben
    // sähe davon nichts.
    expect(WERTE[4000], 'die vier Ticks ergeben nicht 110').toBe(110);
  });

  it('und danach tickt nichts mehr nach', () => {
    expect(WERTE[5000]).toBe(110);
    expect(WERTE[6000]).toBe(110);
  });
});

describe('EFFLUX wandelt Restzeit mit 12 je Sekunde (convert.perSecond)', () => {
  /**
   * Legt `vorlauf` ab, misst die aufgelaufene Restzeit und wirft EFFLUX.
   *
   * `remainingCooldownSumMs` ist der Bezugspunkt: Ohne ihn stünde nur eine
   * Schadenszahl da, und ob die zum Satz passt, wäre nicht nachprüfbar.
   */
  function wandeln(vorlauf: readonly string[]): { restMs: number; schaden: number } {
    const combat = neu();
    let zeit = 0;
    for (const id of vorlauf) {
      combat.stepPassive(zeit);
      combat.cast(alsSpellIndex(combat.indexOf(id)), zeit);
      zeit += 1500;
    }
    combat.stepPassive(zeit);
    const restMs = combat.remainingCooldownSumMs(zeit);
    const vorher = combat.damageDealt;
    expect(combat.cast(alsSpellIndex(combat.indexOf('efflux')), zeit).result).toBe(CAST_OK);
    return { restMs, schaden: combat.damageDealt - vorher };
  }

  it('18 Sekunden Restzeit ergeben 216 Schaden', () => {
    const { restMs, schaden } = wandeln(['plate']);
    expect(restMs, 'die Restzeit stimmt nicht mehr — der Vorlauf hat sich geändert').toBe(18500);
    expect(schaden, '18 volle Sekunden à 12 sind 216').toBe(216);
  });

  it('17 Sekunden ergeben 204 — der Satz gilt, nicht die eine Zahl', () => {
    // **Zwei verschiedene Restzeiten, nicht zweimal dieselbe.** Ein einzelner
    // Fall nagelte ein Ergebnis fest; erst der zweite zeigt, dass dahinter
    // ein Satz je Sekunde steht.
    const { restMs, schaden } = wandeln(['plate', 'scab']);
    expect(restMs).toBe(17000);
    expect(schaden).toBe(204);
  });

  it('ohne Restzeit wandelt er nichts — angebrochene Sekunden zählen nicht', () => {
    const { restMs, schaden } = wandeln([]);
    expect(restMs, 'es läuft doch etwas').toBe(0);
    expect(schaden, 'aus null Restzeit entsteht Schaden').toBe(0);
  });
});
