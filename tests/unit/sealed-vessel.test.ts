/**
 * `The Sealed Vessel` — überschüssiger Schutz wird zu Schaden (13.5, #480).
 *
 * Der vierte der sechs Rotations-Effekte (COAGULATION-Sechser).
 *
 * # Wie „überschüssig" hier gemessen wird — eine Auslegung
 *
 * Der Katalog sagt „über der Obergrenze … statt zu verfallen". Gemessen ist
 * die Obergrenze eine **Stapel**-Grenze (`shield.maxStacks` plus
 * Talent-Bonus), keine Betrags-Grenze; und ein Druck auf eine volle Schicht
 * erhöht den Betrag heute gar nicht erst, statt ihn verfallen zu lassen.
 *
 * Der Überschuss ist deshalb der Betrag, den dieser Druck **beigetragen
 * hätte**. Die gemeinte Stelle ist eindeutig, der Wortlaut ungenau — die
 * Auslegung steht hier, damit sie widersprochen werden kann.
 */

import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { activeRotationEffects, SETS, SET_COUNT } from '../../src/gear/sets.js';
import { SET_EFFECT_SEALED_VESSEL, SET_EFFECT_WHOLE_WORK } from '../../src/gear/sets.js';
import { spellIndex } from '../helpers/spell-index.js';

const SEALED_SET = SETS.find((s) => s.tiers.some((t) => t.effect === SET_EFFECT_SEALED_VESSEL))?.id;

function teile(set: number, n: number): Int32Array {
  const pieces = new Int32Array(SET_COUNT);
  pieces[set] = n;
  return pieces;
}

const BOSS = bossById('test-warden') as BossDef;

/**
 * `n` SCAB im Abstand seiner Abklingzeit — und der Stand danach.
 *
 * SCAB und nicht CRUST: Der hat 6 s Abklingzeit, und meine ersten Würfe im
 * 2-s-Takt wurden abgelehnt. Die Schicht erreichte ihre Grenze nie, und beide
 * Läufe zeigten dasselbe — was wie „der Effekt greift nicht" aussah.
 */
function nachWuerfen(effekte: readonly string[], n: number) {
  const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
  combat.setSetEffects(effekte);
  combat.beginFight(0);
  const scab = alsSpellIndex(spellIndex('scab'));
  for (let i = 0; i < n; i++) combat.cast(scab, i * 1500);
  return {
    stapel: combat.shieldStacks,
    schutz: combat.shield,
    schaden: combat.damageDealt,
  };
}

describe('AC1 — sechs Satzteile lösen ihn aus, fünf nicht', () => {
  it('die Schwelle greift, nicht die blosse Kennung', () => {
    expect(SEALED_SET).toBeDefined();
    const id = SEALED_SET as number;
    expect(activeRotationEffects(SETS, teile(id, 6))).toContain(SET_EFFECT_SEALED_VESSEL);
    expect(activeRotationEffects(SETS, teile(id, 5))).not.toContain(SET_EFFECT_SEALED_VESSEL);
  });
});

describe('AC2 — der vierte Wurf zahlt auf den Schaden ein', () => {
  it('mit Zahlen: 380 statt 180', () => {
    const mit = nachWuerfen([SET_EFFECT_SEALED_VESSEL], 4);
    const ohne = nachWuerfen([], 4);

    expect(ohne.schaden).toBe(180);
    expect(mit.schaden).toBe(380);
    // Die Differenz ist GENAU der Betrag, den die volle Schicht nicht mehr
    // aufnahm — nicht irgendein Zuschlag.
    expect(mit.schaden - ohne.schaden).toBe(200);
  });

  it('bis zur Grenze ändert sich NICHTS', () => {
    // Die ersten drei Würfe stapeln; da gibt es keinen Überschuss. Ein Effekt,
    // der schon hier zahlte, wäre ein Schadensbonus mit Schutz-Anschein.
    for (const n of [1, 2, 3]) {
      expect(nachWuerfen([SET_EFFECT_SEALED_VESSEL], n).schaden).toBe(nachWuerfen([], n).schaden);
    }
    // ZUSICHERUNG: Bei drei Würfen ist die Grenze wirklich erreicht — sonst
    // prüfte der Fall nur, dass vor der Grenze nichts passiert, und der
    // Vergleich oben hinge in der Luft.
    expect(nachWuerfen([], 3).stapel).toBe(3);
  });

  it('und jeder weitere Wurf zahlt erneut', () => {
    const vier = nachWuerfen([SET_EFFECT_SEALED_VESSEL], 4).schaden;
    const fuenf = nachWuerfen([SET_EFFECT_SEALED_VESSEL], 5).schaden;
    // 200 Überschuss plus SCABs eigene 45.
    expect(fuenf - vier).toBe(245);
  });
});

describe('AC3 — ein fremder Effekt löst ihn nicht aus', () => {
  // **Gegen die ZAHL geprüft, nicht gegen einen leeren Aufbau.** Meine erste
  // Fassung verglich mit `nachWuerfen([], 4)` — und die Mutation „Kennung
  // ignorieren, immer zahlen" liess sie GRÜN, weil dann beide Seiten zahlten
  // und wieder übereinstimmten. Ein Vergleich zweier Läufe, die die Mutation
  // gemeinsam verschiebt, misst die Mutation nicht.
  it('The Whole Work im Aufbau zahlt nichts aus', () => {
    expect(nachWuerfen([SET_EFFECT_WHOLE_WORK], 4).schaden).toBe(180);
  });

  it('und eine erfundene Kennung ebenso wenig', () => {
    expect(nachWuerfen(['gibt-es-nicht'], 4).schaden).toBe(180);
  });

  it('ein leerer Aufbau ebenso — und das ist der Bezugspunkt der 180', () => {
    expect(nachWuerfen([], 4).schaden).toBe(180);
  });
});

describe('AC4 — Kreuzprobe: der Effekt ersetzt nichts', () => {
  it('Schutz und Stapelstand bleiben unverändert', () => {
    // Er fügt nur hinzu, was vorher niemand bekam. Nähme er dafür Schutz weg,
    // wäre er ein Tausch und kein Zugewinn — und „Überleben zahlt auf Schaden
    // ein" hiesse dann das Gegenteil.
    const mit = nachWuerfen([SET_EFFECT_SEALED_VESSEL], 5);
    const ohne = nachWuerfen([], 5);
    expect(mit.schutz).toBe(ohne.schutz);
    expect(mit.stapel).toBe(ohne.stapel);
    // ZUSICHERUNG: Es liegt überhaupt Schutz.
    expect(ohne.schutz).toBeGreaterThan(0);
  });

  it('die globale Wartezeit bleibt unberührt', () => {
    const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
    combat.setSetEffects([SET_EFFECT_SEALED_VESSEL]);
    expect(combat.gcd.lengthMs).toBe(1500);
  });
});
