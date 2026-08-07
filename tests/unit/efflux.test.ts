import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function frisch(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/**
 * EFFLUX wandelt Abklingzeit in Schaden (Issue #520).
 *
 * Er ist keine Cooldown-Mechanik: Nichts wird verkürzt. Es ist dieselbe
 * Umwandlung wie bei WEDGE, nur mit anderer Quelle.
 */
describe('EFFLUX — Abklingzeit als Quelle (Issue #520)', () => {
  it('AC4 — ohne laufende Abklingzeiten macht er null und bricht nicht', () => {
    const combat = frisch();
    const efflux = combat.indexOf('efflux');
    // Zum Zeitpunkt 0 liegt keine Abklingzeit an: nichts wurde gewirkt.
    expect(combat.remainingCooldownSumMs(0), 'nichts läuft').toBe(0);

    const vorher = combat.damageDealt;
    expect(() => combat.cast(efflux, 0)).not.toThrow();
    combat.stepPassive(10);
    expect(combat.damageDealt - vorher, 'null Restzeit ergibt null Schaden').toBe(0);
  });

  it('AC1 — WEDGE verhält sich unverändert: er nimmt weiterhin die Punkte', () => {
    // **Die Gegenprobe zur Erweiterung.** Ohne sie belegte AC4 nur, dass der
    // neue Zweig läuft — nicht, dass der alte unberührt blieb.
    const combat = frisch();
    const bind = combat.indexOf('bind');
    const wedge = combat.indexOf('wedge');
    combat.cast(bind, 0);
    combat.cast(bind, 1600);
    const punkte = combat.control;
    expect(punkte, 'zwei BIND geben Punkte').toBeGreaterThan(0);

    const vorher = combat.damageDealt;
    combat.cast(wedge, 3200);
    combat.stepPassive(3210);
    expect(combat.damageDealt - vorher, 'WEDGE zieht aus den Punkten').toBeGreaterThan(0);
    expect(combat.control, 'und verbraucht sie').toBe(0);
  });

  it('mehr Restzeit ergibt mehr Schaden — die Quelle wirkt wirklich', () => {
    // **Zwei Läufe, ein Unterschied.** Ein Prüffall, der nur „größer als null"
    // fordert, wäre auch grün, wenn EFFLUX eine feste Zahl liefert.
    function schaden(vorlauf: number): { schaden: number; restMs: number } {
      const combat = frisch();
      const efflux = combat.indexOf('efflux');
      // **Vorlauf mit Zaubern, die LÄNGER als die Wartezeit brauchen.** Meine
      // erste Fassung wirkte KINDLE alle 1600 ms — bei 1500 ms Abklingzeit war
      // er jedes Mal wieder bereit, die Summe blieb null, und beide Läufe
      // ergaben denselben Schaden: null. Der Prüffall war rot aus dem
      // richtigen Grund, nur nicht dem gemeinten.
      const lang = ['pyre', 'reflux', 'quench'].map((id) => combat.indexOf(id));
      let ms = 0;
      for (let i = 0; i < vorlauf; i++) {
        const index = lang[i % lang.length];
        if (index !== undefined && combat.isOffCooldown(index, ms)) combat.cast(index, ms);
        ms += 1600;
      }
      const restMs = combat.remainingCooldownSumMs(ms);
      const vorher = combat.damageDealt;
      combat.cast(efflux, ms);
      combat.stepPassive(ms + 10);
      return { schaden: combat.damageDealt - vorher, restMs };
    }
    const wenig = schaden(1);
    const viel = schaden(3);
    // Erst belegen, dass sich die QUELLE unterscheidet — sonst prüft der
    // Vergleich darunter zwei gleiche Lagen.
    expect(viel.restMs, 'drei Zauber lassen mehr Restzeit liegen als einer').toBeGreaterThan(
      wenig.restMs,
    );
    expect(
      viel.schaden,
      `${viel.restMs} ms Restzeit ergeben mehr als ${wenig.restMs} ms`,
    ).toBeGreaterThan(wenig.schaden);
  });
});
