/**
 * `The Whole Work` — der globale Cooldown sinkt um 100 ms (13.5, Issue #480).
 *
 * Der einzige der sechs Rotations-Effekte **ohne Schulbindung**: Er wirkt auf
 * jede Zusammenstellung gleich und zwingt zu keiner. Ein Vorentwurf belohnte
 * Schulenvielfalt und wurde verworfen — Kombinieren ist ohnehin frei.
 *
 * # Was dieser Zweig nebenbei belebt hat
 *
 * `GlobalCooldown.setLength` gibt es seit #177 und hatte **keinen Aufrufer
 * ausserhalb der Prüffälle**. Die variable Wartezeit war damit dieselbe Klasse
 * wie `activeRotationEffects` vor #774, `upgradeSteps` vor #740 und
 * `gearEffects` vor #748: gebaut, gespeichert, nie gelesen.
 *
 * Die 100 ms sind ein **Startwert** des Orchestrators, keine
 * Balance-Entscheidung — der Prüflauf aus #118 misst, ob er trägt.
 */

import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { activeRotationEffects, SETS, SET_COUNT } from '../../src/gear/sets.js';
import {
  SET_EFFECT_ASHEN_REGIMEN,
  SET_EFFECT_WHOLE_WORK,
  WHOLE_WORK_GCD_REDUCTION_MS,
} from '../../src/gear/sets.js';

/** Der Satz mit `whole-work` liegt auf Kennung 1 (`data/sets.json`). */
const WHOLE_WORK_SET = 1;

/** `n` Teile des Satzes `set` — die Verteilung auf Plätze spielt keine Rolle. */
function teile(set: number, n: number): Int32Array {
  const pieces = new Int32Array(SET_COUNT);
  pieces[set] = n;
  return pieces;
}

/** Ein Kampf mit den gegebenen Effekten, vor dem ersten Wurf. */
function kampfMit(effekte: readonly string[]): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  combat.setSetEffects(effekte);
  return combat;
}

describe('AC1 — sechs Satzteile lösen ihn aus, fünf nicht', () => {
  it('die Schwelle greift, nicht die blosse Kennung', () => {
    expect(activeRotationEffects(SETS, teile(WHOLE_WORK_SET, 6))).toContain(SET_EFFECT_WHOLE_WORK);
    expect(activeRotationEffects(SETS, teile(WHOLE_WORK_SET, 5))).not.toContain(
      SET_EFFECT_WHOLE_WORK,
    );
    // ZUSICHERUNG: Bei fünf Teilen ist der Satz sehr wohl getragen — die
    // unteren Stufen greifen. Der Fall prüft die SCHWELLE der sechsten.
    expect(teile(WHOLE_WORK_SET, 5)[WHOLE_WORK_SET]).toBe(5);
  });

  it('und die Wartezeit sinkt wirklich — mit Zahlen', () => {
    expect(kampfMit([]).gcd.lengthMs).toBe(GCD_MS);
    expect(kampfMit([SET_EFFECT_WHOLE_WORK]).gcd.lengthMs).toBe(
      GCD_MS - WHOLE_WORK_GCD_REDUCTION_MS,
    );
    // Die Zahlen selbst, nicht nur „kürzer": 1500 gegen 1400.
    expect(GCD_MS).toBe(1500);
    expect(kampfMit([SET_EFFECT_WHOLE_WORK]).gcd.lengthMs).toBe(1400);
  });

  it('sie wirkt auch auf den Zeitpunkt, ab dem wieder gewirkt werden darf', () => {
    // Die Länge allein wäre eine Zahl in einem Feld. Der Fall misst, was sie
    // BEWIRKT: wann die Wartezeit endet.
    const mit = kampfMit([SET_EFFECT_WHOLE_WORK]);
    const ohne = kampfMit([]);
    mit.gcd.trigger(0);
    ohne.gcd.trigger(0);
    expect(mit.gcd.readyAt).toBe(1400);
    expect(ohne.gcd.readyAt).toBe(1500);
    expect(mit.gcd.isReady(1400)).toBe(true);
    expect(ohne.gcd.isReady(1400)).toBe(false);
  });
});

describe('AC2 — ein fremder Effekt löst ihn nicht aus', () => {
  it('Ashen Regimen im Aufbau lässt die Wartezeit unberührt', () => {
    // Der Fall, der bei Ashen Regimen den Unterschied gemacht hat: Ohne ihn
    // wäre auch eine Rechnung grün, die auf jede beliebige Kennung reagiert.
    expect(kampfMit([SET_EFFECT_ASHEN_REGIMEN]).gcd.lengthMs).toBe(GCD_MS);
  });

  it('und eine erfundene Kennung ebenso wenig', () => {
    expect(kampfMit(['gibt-es-nicht']).gcd.lengthMs).toBe(GCD_MS);
  });

  it('ein Wechsel der Effekte zieht die Länge nach', () => {
    // **Abgeleitet, nicht mitgeführt.** Würde die Länge beim Setzen einmal
    // geschrieben und nie zurückgenommen, bliebe sie nach dem Ablegen des
    // Satzes gesenkt — ein Effekt, der weiterwirkt, obwohl er nicht mehr
    // getragen wird.
    const combat = kampfMit([SET_EFFECT_WHOLE_WORK]);
    expect(combat.gcd.lengthMs).toBe(1400);
    combat.setSetEffects([]);
    expect(combat.gcd.lengthMs).toBe(GCD_MS);
  });
});

describe('AC3 — Kreuzprobe: was der Effekt NICHT anfasst', () => {
  it('der Schaden eines Wurfs bleibt gleich', () => {
    const schaden = (effekte: readonly string[]): number => {
      const combat = kampfMit(effekte);
      combat.beginFight(0);
      combat.cast(combat.indexOf('kindle'), 0);
      return combat.damageDealt;
    };
    expect(schaden([SET_EFFECT_WHOLE_WORK])).toBe(schaden([]));
    // ZUSICHERUNG: Es wurde überhaupt Schaden gemacht.
    expect(schaden([])).toBeGreaterThan(0);
  });

  it('die Abklingzeit eines Zaubers bleibt gleich', () => {
    // Die Wartezeit und die Abklingzeit sind zwei Grössen. Sänke beides,
    // wäre der Effekt doppelt so stark wie beschrieben.
    const rest = (effekte: readonly string[]): number => {
      const combat = kampfMit(effekte);
      combat.beginFight(0);
      const kindle = combat.indexOf('kindle');
      combat.cast(kindle, 0);
      return combat.cooldownRemainingMs(kindle, 0);
    };
    expect(rest([SET_EFFECT_WHOLE_WORK])).toBe(rest([]));
    expect(rest([])).toBeGreaterThan(0);
  });

  it('der Ressourcen-Aufbau bleibt gleich', () => {
    const vorrat = (effekte: readonly string[]): number => {
      const combat = kampfMit(effekte);
      combat.beginFight(0);
      combat.cast(combat.indexOf('kindle'), 0);
      return combat.resource;
    };
    expect(vorrat([SET_EFFECT_WHOLE_WORK])).toBe(vorrat([]));
    expect(vorrat([])).toBeGreaterThan(0);
  });
});

describe('Die Kreuzwirkung, die es SEHR WOHL gibt — und die gewollt ist', () => {
  it('Verstärkungs-Dauern in GCDs werden mit kürzer', () => {
    // `buffs.ts` rechnet Fenster in Vielfachen der Wartezeit („drei GCDs
    // lang"). Sinkt sie, sinkt auch das Fenster — das ist keine Nebenwirkung,
    // sondern die Bedeutung der Einheit.
    //
    // Der Fall steht hier, weil er beim Bauen aufgefallen ist und nicht im
    // Auftrag stand: Wer „der GCD sinkt um 100 ms" liest, denkt an den Takt
    // und nicht an Buff-Fenster.
    const fenster = (effekte: readonly string[]): number => kampfMit(effekte).gcd.lengthMs * 3;
    expect(fenster([SET_EFFECT_WHOLE_WORK])).toBe(4200);
    expect(fenster([])).toBe(4500);
  });
});
