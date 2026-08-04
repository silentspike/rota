import { describe, expect, it } from 'vitest';
import { CAST_OK, Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { hastedMs } from '../../src/core/haste.js';
import { NOTHING_WORN, critChance, critFactor, type WornValues } from '../../src/gear/worn.js';

/**
 * Die Ausrüstungswerte kommen im Kampf an (GAME-DESIGN 6.7, Issue #177) —
 * die Spielseite.
 *
 * Jede Wirkung bekommt ihren **eigenen** Prüffall. Die Auflage nennt den
 * Präzedenzfall: In #100 lief STALL durch die Prüfsumme, hatte aber keinen
 * eigenen Nachweis — ein gemeinsamer Fall bleibt grün, wenn nur eine der
 * geprüften Wirkungen greift.
 */

function combatMit(worn: WornValues, seed = 7): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value, seed, worn });
}

/** Rechnet die laufenden Wirkungen von `fromMs` bis `toMs` durch. */
function stepUntil(combat: Combat, fromMs: number, toMs: number): void {
  for (let t = fromMs; t < toMs; t += TICK_MS) combat.stepPassive(t);
}

describe('Ohne getragene Ausrüstung ändert sich nichts (AC7)', () => {
  it('lässt jede Abklingzeit auf ihrem Wert', () => {
    const combat = combatMit(NOTHING_WORN);
    for (const spell of combat.spells) {
      expect(hastedMs(spell.cooldownMs, 0), `Abklingzeit von ${spell.id}`).toBe(spell.cooldownMs);
      if (spell.dot !== undefined) {
        expect(hastedMs(spell.dot.tickIntervalMs, 0), `Tick-Abstand von ${spell.id}`).toBe(
          spell.dot.tickIntervalMs,
        );
      }
    }
  });

  it('zieht keine einzige Zahl aus den Zufallsströmen', () => {
    // **Der Punkt, den man leicht übersieht.** Ein Kritwurf bei einer Chance
    // von null wäre rechnerisch folgenlos — und trotzdem sichtbar: Der
    // Ziehungszähler geht in die Zustands-Prüfsumme ein. Jeder Takt meldete
    // dann eine Abweichung gegen den Simulator.
    const combat = combatMit(NOTHING_WORN);
    combat.beginFight(0);
    expect(combat.cast(combat.indexOf('kindle'), 0).result).toBe(CAST_OK);
    stepUntil(combat, 0, 5000);
    expect(combat.random.totalDraws).toBe(0);
  });

  it('zieht MIT PURITY sehr wohl — der Gegenfall', () => {
    // Ohne ihn bliebe die Zeile darüber auch dann grün, wenn der Kampf
    // überhaupt nie würfelt.
    const combat = combatMit({ catalysis: 0, purity: 500, yieldBonus: 0 });
    combat.beginFight(0);
    combat.cast(combat.indexOf('kindle'), 0);
    expect(combat.random.totalDraws).toBeGreaterThan(0);
  });

  it('würfelt auch bei 100 % PURITY nicht — dort steht das Ergebnis fest', () => {
    // `chance` schließt bei 1 genauso kurz wie bei 0. Am Deckel ist Krit kein
    // Zufall mehr (6.7), und ein Wurf wäre Verschwendung.
    const combat = combatMit({ catalysis: 0, purity: 1000, yieldBonus: 0 });
    combat.beginFight(0);
    combat.cast(combat.indexOf('kindle'), 0);
    expect(combat.random.totalDraws).toBe(0);
    // Und er kritet trotzdem: 100 × 1,5 = 150.
    expect(combat.damageDealt).toBe(150);
  });
});

describe('Tempo verkürzt Abklingzeiten (AC2, erster Wirkort)', () => {
  it('setzt die Abklingzeit von PYRE messbar kürzer an', () => {
    const pyre = 5000;
    expect(hastedMs(pyre, 200)).toBe(4170); // 5000/1,2 = 4166,67 -> aufgerastert

    const ohne = combatMit(NOTHING_WORN);
    const mit = combatMit({ catalysis: 200, purity: 0, yieldBonus: 0 });
    for (const c of [ohne, mit]) {
      c.beginFight(0);
      // PYRE braucht Hitze; ohne sie wirkt er zwar, teilt aber keinen Schaden
      // aus. Für die Abklingzeit ist das gleichgültig — sie beginnt trotzdem.
      expect(c.cast(c.indexOf('pyre'), 0).result).toBe(CAST_OK);
    }

    const index = ohne.indexOf('pyre');
    expect(ohne.cooldownRemainingMs(index, 0)).toBe(pyre);
    expect(mit.cooldownRemainingMs(index, 0)).toBe(4170);
    expect(mit.cooldownRemainingMs(index, 0)).toBeLessThan(ohne.cooldownRemainingMs(index, 0));
  });

  it('macht ihn dadurch früher wieder bereit', () => {
    // Die Folge der Zeile darüber, und sie ist der eigentliche Zweck: Eine
    // kürzere Zahl, die niemanden früher handeln lässt, wäre wirkungslos.
    const mit = combatMit({ catalysis: 200, purity: 0, yieldBonus: 0 });
    mit.beginFight(0);
    mit.cast(mit.indexOf('pyre'), 0);
    const index = mit.indexOf('pyre');

    expect(mit.isOffCooldown(index, 4160)).toBe(false);
    expect(mit.isOffCooldown(index, 4170)).toBe(true);

    // Der Vergleichsfall: ohne Tempo ist er zu diesem Zeitpunkt noch gesperrt.
    const ohne = combatMit(NOTHING_WORN);
    ohne.beginFight(0);
    ohne.cast(ohne.indexOf('pyre'), 0);
    expect(ohne.isOffCooldown(index, 4170)).toBe(false);
    expect(ohne.isOffCooldown(index, 5000)).toBe(true);
  });
});

describe('Tempo verdichtet DoT-Ticks (AC2, zweiter Wirkort)', () => {
  it('setzt den Tick-Abstand kürzer an — eigener Nachweis, nicht der der Abklingzeit', () => {
    // **Ein gemeinsamer Prüffall mit der Abklingzeit bliebe grün, wenn nur
    // eine der beiden Stellen verdrahtet wäre.** Deshalb ein eigener Lauf.
    expect(hastedMs(2000, 200)).toBe(1670);

    const ohne = combatMit(NOTHING_WORN);
    const mit = combatMit({ catalysis: 200, purity: 0, yieldBonus: 0 });
    for (const c of [ohne, mit]) {
      c.beginFight(0);
      expect(c.cast(c.indexOf('smoulder'), 0).result).toBe(CAST_OK);
    }
    const index = ohne.indexOf('smoulder');

    // Ohne Tempo tickt es bei 2000, mit Tempo schon bei 1670.
    stepUntil(ohne, 0, 2000);
    stepUntil(mit, 0, 2000);
    expect(mit.dotTickCount(index)).toBeGreaterThan(ohne.dotTickCount(index));
    expect(ohne.dotTickCount(index)).toBe(0);
    expect(mit.dotTickCount(index)).toBe(1);
  });

  it('gibt über die volle Laufzeit mehr Ticks ab', () => {
    const ohne = combatMit(NOTHING_WORN);
    const mit = combatMit({ catalysis: 200, purity: 0, yieldBonus: 0 });
    for (const c of [ohne, mit]) {
      c.beginFight(0);
      c.cast(c.indexOf('smoulder'), 0);
      stepUntil(c, 0, 12_010);
    }
    const index = ohne.indexOf('smoulder');

    // 12 000 / 2000 = 6 gegen 12 000 / 1670 = 7.
    expect(ohne.dotTickCount(index)).toBe(6);
    expect(mit.dotTickCount(index)).toBe(7);
  });
});

describe('Die Dauer bleibt, die Ticks werden mehr (AC4)', () => {
  it('lässt das Zeitfenster unangetastet', () => {
    // **Der Kern von 6.7**: Schrumpfte die Dauer mit dem Tempo, verschöbe sich
    // der optimale Erneuerungszeitpunkt (7.2) bei jedem Ausrüstungswechsel.
    const ohne = combatMit(NOTHING_WORN);
    const mit = combatMit({ catalysis: 500, purity: 0, yieldBonus: 0 });
    for (const c of [ohne, mit]) {
      c.beginFight(0);
      c.cast(c.indexOf('smoulder'), 0);
    }
    const index = ohne.indexOf('smoulder');

    expect(mit.dotRemainingMs(index, 0)).toBe(ohne.dotRemainingMs(index, 0));
    expect(mit.dotRemainingMs(index, 0)).toBe(12_000);
    // Und am Ende der Laufzeit sind beide gleichzeitig abgelaufen.
    expect(mit.isDotActive(index, 12_000)).toBe(ohne.isDotActive(index, 12_000));
    expect(mit.isDotActive(index, 12_010)).toBe(false);
  });

  it('zahlt keinen anteiligen letzten Tick aus', () => {
    // WoW zahlt ihn aus, wenn der Rest mindestens 0,1 s beträgt; bei uns wäre
    // das eine Kommazahl im **Schaden**, also genau dort, wo Spiel und
    // Simulator bitgleich sein müssen.
    const mit = combatMit({ catalysis: 200, purity: 0, yieldBonus: 0 });
    mit.beginFight(0);
    mit.cast(mit.indexOf('smoulder'), 0);
    stepUntil(mit, 0, 12_010);
    const index = mit.indexOf('smoulder');

    const abstand = hastedMs(2000, 200);
    const ticks = mit.dotTickCount(index);
    expect(ticks * abstand).toBeLessThanOrEqual(12_000);
    // Ohne den Gegenfall wäre die Zeile darüber mit null Ticks immer erfüllt.
    expect((ticks + 1) * abstand).toBeGreaterThan(12_000);
  });
});

describe('PURITY und YIELD wirken auf den Krit (AC5)', () => {
  it('PURITY erhöht die Kritrate — eigener Nachweis', () => {
    // Gemessen an der Zahl der Krits über viele Treffer, nicht an einem
    // einzelnen: Ein einzelner Wurf sagt über eine Rate nichts.
    const treffer = 200;
    let krits = 0;
    const combat = combatMit({ catalysis: 0, purity: 500, yieldBonus: 0 });
    combat.beginFight(0);
    combat.onHit = (_damage, _kind, crit, _big, _t) => {
      if (crit) krits++;
    };
    for (let i = 0; i < treffer; i++) {
      // KINDLE hat 1500 ms Abklingzeit; wir setzen die Zeit weit genug vor.
      combat.cast(combat.indexOf('kindle'), i * 2000);
    }

    expect(krits).toBeGreaterThan(0);
    expect(krits).toBeLessThan(treffer);
    // Bei 50 % erwartet man rund die Hälfte. Die Schranken sind weit, weil es
    // echter Zufall ist (6.5) — eng gesetzt wäre die Prüfung launisch.
    expect(krits / treffer).toBeGreaterThan(0.3);
    expect(krits / treffer).toBeLessThan(0.7);
  });

  it('PURITY 0 ergibt keinen einzigen Krit — der Vergleichsfall', () => {
    let krits = 0;
    const combat = combatMit(NOTHING_WORN);
    combat.beginFight(0);
    combat.onHit = (_damage, _kind, crit, _big, _t) => {
      if (crit) krits++;
    };
    for (let i = 0; i < 50; i++) combat.cast(combat.indexOf('kindle'), i * 2000);
    expect(krits).toBe(0);
  });

  it('YIELD erhöht den Kritschaden — eigener Nachweis', () => {
    // Damit ausschließlich YIELD gemessen wird, kritet in beiden Läufen JEDER
    // Treffer (PURITY 100 %). Sonst mischte sich der Zufall der Rate hinein.
    const nurPurity = combatMit({ catalysis: 0, purity: 1000, yieldBonus: 0 });
    const mitYield = combatMit({ catalysis: 0, purity: 1000, yieldBonus: 500 });
    for (const c of [nurPurity, mitYield]) {
      c.beginFight(0);
      c.cast(c.indexOf('kindle'), 0);
    }

    // KINDLE macht 100 Grundschaden. 100 × 1,5 = 150, 100 × 2,0 = 200.
    expect(nurPurity.damageDealt).toBe(150);
    expect(mitYield.damageDealt).toBe(200);
  });

  it('YIELD ohne PURITY bewirkt nichts — die Abwägung, die 6.7 zumutet', () => {
    const ohne = combatMit(NOTHING_WORN);
    const nurYield = combatMit({ catalysis: 0, purity: 0, yieldBonus: 500 });
    for (const c of [ohne, nurYield]) {
      c.beginFight(0);
      c.cast(c.indexOf('kindle'), 0);
    }
    expect(nurYield.damageDealt).toBe(ohne.damageDealt);
    expect(ohne.damageDealt).toBe(100);
  });

  it('rechnet die beiden Werte so um, wie 6.7 es festlegt', () => {
    expect(critFactor(NOTHING_WORN)).toBe(1.5);
    expect(critFactor({ catalysis: 0, purity: 0, yieldBonus: 500 })).toBe(2);
    expect(critChance({ catalysis: 0, purity: 250, yieldBonus: 0 })).toBe(0.25);
    // Über dem Deckel bleibt die Chance eins, statt über eins zu gehen.
    expect(critChance({ catalysis: 0, purity: 5000, yieldBonus: 0 })).toBe(1);
  });
});

describe('Jeder der drei Werte bewegt etwas — der Gegenfall zum ganzen Issue', () => {
  it('lässt keinen der drei stillschweigend unverdrahtet', () => {
    // **Ohne diesen Fall könnte einer der drei Werte gar nicht angeschlossen
    // sein**, und alle Prüfungen oben liefen trotzdem durch — jede von ihnen
    // prüft nur ihren eigenen Wert.
    const lauf = (worn: WornValues): number => {
      const c = combatMit(worn);
      c.beginFight(0);
      c.cast(c.indexOf('smoulder'), 0);
      c.cast(c.indexOf('kindle'), 1500);
      stepUntil(c, 0, 12_010);
      return c.damageDealt;
    };

    const ohne = lauf(NOTHING_WORN);
    const nurTempo = lauf({ catalysis: 200, purity: 0, yieldBonus: 0 });
    const nurPurity = lauf({ catalysis: 0, purity: 1000, yieldBonus: 0 });
    const mitYield = lauf({ catalysis: 0, purity: 1000, yieldBonus: 1000 });

    expect(nurTempo, 'CATALYSIS bewegt nichts').not.toBe(ohne);
    expect(nurPurity, 'PURITY bewegt nichts').not.toBe(ohne);
    expect(mitYield, 'YIELD bewegt nichts').not.toBe(nurPurity);
  });
});
