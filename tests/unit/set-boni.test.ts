/**
 * Satz-Boni und legendäre Effekte kommen im Kampf an (Topf 4, Issue #480).
 *
 * Die Leitung nach `worn.gearEffects` lag seit Langem und war über einen Affix
 * erreichbar — gespeist hat sie niemand. `countPieces` und `valueShareOf` in
 * `sets.ts` hatten zusammen **null** Aufrufer, es gab keine `data/sets.json`,
 * und kein Katalog-Eintrag trug ein `setId`. Ein legendäres Stück war damit in
 * jeder messbaren Grösse ein Epic: `QUALITY_HAS_EFFECT` ist das Einzige, was
 * die beiden unterscheidet (gleicher Topf, gleiche Affix-Plätze), und niemand
 * las es.
 *
 * # Die Zahlen hier stammen aus einem PRÜFSATZ
 *
 * `test-set` ist in derselben Bauart gebaut wie `test-relic` und `test-warden`
 * und wie sie an einen Boss gebunden, damit er nicht als reguläre Beute fällt.
 * Seine Anteile sind rund (0,1 / 0,2), damit niemand sie für balanciert hält —
 * die sechs echten Sätze aus 13.5 sind eine Design-Entscheidung und stehen aus.
 */

import { describe, expect, it } from 'vitest';
import { wornFrom, type OwnedItemLike } from '../../src/gear/catalog.js';
import { gearEffectFactor } from '../../src/gear/worn.js';
import { SET_TIERS } from '../../src/gear/sets.js';
import { type WornValues } from '../../src/gear/worn.js';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { COUNTDOWN_MS, PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { spellIndex } from '../helpers/spell-index.js';
import { rollLoot } from '../../src/loot/roll.js';
import { lootStreamFor } from '../../src/loot/stream.js';

/** `n` Satz-Teile, jedes auf einem eigenen Platz — der Satz zählt nur die Anzahl. */
function satzTeile(n: number): OwnedItemLike[] {
  return Array.from({ length: n }, (_, i) => ({
    itemId: `test-set-${i + 1}`,
    upgradeSteps: 0,
    equippedSlot: i + 1,
  }));
}

/** Ein gewöhnliches Stück ohne Satz — der Bezugspunkt. */
const OHNE_SATZ: OwnedItemLike[] = [{ itemId: 'test-relic', upgradeSteps: 0, equippedSlot: 0 }];

describe('AC1/AC2 — mehr Satz-Teile, mehr Topf 4', () => {
  it('die gemessene Reihe über alle Stufen', () => {
    const reihe = [0, 1, 2, 3, 4].map((n) => wornFrom(satzTeile(n)).gearEffects);
    // Gemessen, nicht gerundet geschätzt: 0,1 bei zwei Teilen, 0,1+0,2 bei vier.
    expect(reihe).toEqual([0, 0, 100, 100, 300]);
  });

  it('vier Teile schlagen zwei — und DREI tun es nicht', () => {
    const zwei = wornFrom(satzTeile(2)).gearEffects;
    const drei = wornFrom(satzTeile(3)).gearEffects;
    const vier = wornFrom(satzTeile(4)).gearEffects;

    expect(vier).toBeGreaterThan(zwei);
    // **Der Auftrag verlangte „drei mehr als zwei". Das ist nicht erfüllbar**,
    // und zwar nicht wegen dieses Baus: `SET_TIERS` steht auf [2, 4, 6] und ist
    // in `sets.ts` ausdrücklich als „nicht vorläufig" markiert — die drei
    // Schwellen stehen im Design (13.5). Zwischen zwei und vier Teilen ändert
    // sich nichts, und ein Prüffall, der es behauptete, wäre rot geworden.
    expect(drei).toBe(zwei);
    expect(SET_TIERS).toEqual([2, 4, 6]);
  });

  it('zwei Teile schlagen eines — der Bonus greift erst ab zwei', () => {
    expect(wornFrom(satzTeile(1)).gearEffects).toBe(0);
    expect(wornFrom(satzTeile(2)).gearEffects).toBeGreaterThan(0);
  });
});

describe('AC3 — ohne Satz-Teile bleibt der Topf leer', () => {
  it('gearEffects ist 0 und der Faktor genau 1', () => {
    const w = wornFrom(OHNE_SATZ);
    expect(w.gearEffects).toBe(0);
    expect(gearEffectFactor(w)).toBe(1);
  });

  it('auch ganz ohne getragene Stücke', () => {
    expect(wornFrom([]).gearEffects).toBe(0);
    expect(gearEffectFactor(wornFrom([]))).toBe(1);
  });

  it('ein Satz-Teil im BEUTEL zählt nicht', () => {
    // Sonst wäre Sammeln stärker als Ausrüsten — dieselbe Regel wie bei den
    // übrigen Werten.
    const imBeutel = satzTeile(4).map((s) => ({ ...s, equippedSlot: null }));
    expect(wornFrom(imBeutel).gearEffects).toBe(0);
  });
});

describe('AC4 — es GIBT einen Unterschied zu sehen', () => {
  it('der Faktor unterscheidet sich wirklich, mit Zahlen', () => {
    // Die zweite Hälfte der Gegenprobe: Ein Fall, der nur „ohne Satz ist es 1"
    // prüft, wäre auch grün, wenn der Satz gar nicht ankommt. Er muss zeigen,
    // dass es überhaupt etwas zu sehen gibt.
    expect(gearEffectFactor(wornFrom(OHNE_SATZ))).toBe(1);
    expect(gearEffectFactor(wornFrom(satzTeile(2)))).toBeCloseTo(1.1, 10);
    expect(gearEffectFactor(wornFrom(satzTeile(4)))).toBeCloseTo(1.3, 10);
  });

  it('und die Satz-Teile sind wirklich verschiedene Stücke auf verschiedenen Plätzen', () => {
    // ZUSICHERUNG gegen den Aufbau selbst: Lägen alle vier auf demselben Platz,
    // zählte `countPieces` sie trotzdem — der Fall wäre grün und beschriebe
    // eine Ausrüstung, die es nicht geben kann.
    const vier = satzTeile(4);
    expect(new Set(vier.map((s) => s.itemId)).size).toBe(4);
    expect(new Set(vier.map((s) => s.equippedSlot)).size).toBe(4);
  });
});

describe('AC-Legendär — der Effekt ist das Einzige, was Legendary von Epic trennt', () => {
  /** Dasselbe Stück, nur in legendärer Qualität — die Instanz schlägt die Vorlage. */
  const legendaer = (n: number): OwnedItemLike[] =>
    Array.from({ length: n }, (_, i) => ({
      itemId: 'test-relic',
      upgradeSteps: 0,
      equippedSlot: i,
      quality: 3,
    }));

  it('ein legendäres Stück füllt Topf 4, ein episches nicht', () => {
    expect(wornFrom(OHNE_SATZ).gearEffects).toBe(0);
    expect(wornFrom(legendaer(1)).gearEffects).toBe(200);
  });

  it('zwei legendäre addieren sich', () => {
    expect(wornFrom(legendaer(2)).gearEffects).toBe(400);
  });

  it('sie sind bei allem ANDEREN gleich — der Effekt ist der ganze Unterschied', () => {
    // `QUALITY_BASE_BUDGET` und `QUALITY_AFFIX_SLOTS` führen für Epic und
    // Legendary denselben Wert ([…, 190, 190] und […, 3, 3]). Das sah wie ein
    // Fehler aus und ist Absicht: `QUALITY_HAS_EFFECT` trennt sie, und bis
    // hierher las es niemand. Wäre das nicht so, käme der Unterschied unten
    // aus dem Budget statt aus dem Effekt.
    const epic = wornFrom(OHNE_SATZ);
    const leg = wornFrom(legendaer(1));
    expect(leg.catalysis).toBe(epic.catalysis);
    expect(leg.essence).toBe(epic.essence);
    expect(leg.gearEffects).not.toBe(epic.gearEffects);
  });

  it('Satz-Bonus und legendärer Effekt ADDIEREN sich, sie multiplizieren nicht', () => {
    // 13.1b: ein gemeinsamer Topf. Getrennt gerechnet schlüge die Kombination
    // jede andere Ausrichtung, und alles ohne Satz-Bonus bliebe liegen.
    const nurSatz = wornFrom(satzTeile(4)).gearEffects; // 300
    const gemischt = wornFrom([
      ...satzTeile(4),
      { itemId: 'test-relic', upgradeSteps: 0, equippedSlot: 0, quality: 3 },
    ]).gearEffects;

    expect(nurSatz).toBe(300);
    expect(gemischt).toBe(nurSatz + 200);
    // Und NICHT multipliziert — das wäre 1,3 × 1,2 = 1,56, also 560.
    expect(gemischt).not.toBe(560);
  });
});

describe('AC5 — der Topf-4-Faktor kommt als SCHADEN an', () => {
  /** Derselbe Kampf, dieselben Tastendrücke — nur die Ausrüstung unterscheidet sich. */
  function schadenMit(worn: WornValues): number {
    const queue = new PressQueue();
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value, worn });
    const session = new Session(queue, combat);
    session.bind(0, alsSpellIndex(spellIndex('kindle')));
    session.beginCountdown(0);
    let tick = 0;
    let letzter = 0;
    for (let t = 0; t <= COUNTDOWN_MS + 30_000; t += 10) {
      if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % 1500 === 0) queue.push(0, tick, t, tick);
      session.tick(t);
      letzter = t;
      tick++;
      if (session.phase === PHASE_ENDED) break;
    }
    return session.summary(letzter).damage;
  }

  /**
   * **Warum hier `gearEffects` einzeln weggenommen wird und nicht Satz-Teile.**
   *
   * Mein erster Aufbau verglich vier Satz-Teile gegen zwei und gegen ein
   * `test-relic` — und maß 3465 gegen 2625 gegen 3015. Zwei Satz-Teile machten
   * weniger Schaden als gar keines, was nach einem Fehler im Bau aussah.
   *
   * Es war der Messweg: Die drei Läufe unterschieden sich nicht nur im
   * Satz-Bonus. `test-relic` trägt drei Affixe, ein Satz-Teil einen; vier
   * Stücke tragen mehr Werte als zwei. Verglichen wurden also Ausrüstungen,
   * nicht Boni.
   *
   * Hier bleibt alles gleich AUSSER Topf 4 — dieselbe Zahl in `catalysis`,
   * dieselben Grundwerte, nur `gearEffects` einmal mit und einmal ohne. Dass
   * die Satz-Teile diesen Topf überhaupt füllen, messen die Fälle darüber.
   */
  it('derselbe Aufbau mit und ohne Topf 4 — mit Zahlen', () => {
    const mitSatz = wornFrom(satzTeile(4));
    const ohneTopf4: WornValues = { ...mitSatz, gearEffects: 0 };

    const mit = schadenMit(mitSatz);
    const ohne = schadenMit(ohneTopf4);

    expect(mit).toBeGreaterThan(ohne);
    // ZUSICHERUNG: Es wurde überhaupt gekämpft. Ohne sie wäre der Fall auch
    // grün, wenn beide Läufe 0 Schaden machten — zwei übereinstimmende Nullen.
    expect(ohne).toBeGreaterThan(0);
    // Und der Unterschied ist GENAU der Faktor, nicht irgendeiner: 1,3.
    expect(mit / ohne).toBeCloseTo(1.3, 2);
  });

  it('und die beiden Aufbauten unterscheiden sich WIRKLICH nur in Topf 4', () => {
    // Die Gegenprobe zum Aufbau selbst. Wäre noch etwas anderes verschieden,
    // liesse sich der Unterschied oben nicht dem Satz-Bonus zuschreiben — genau
    // der Fehler, den mein erster Versuch gemacht hat.
    const mitSatz = wornFrom(satzTeile(4));
    const ohneTopf4: WornValues = { ...mitSatz, gearEffects: 0 };
    const abweichend = (Object.keys(mitSatz) as (keyof WornValues)[]).filter(
      (k) => mitSatz[k] !== ohneTopf4[k],
    );
    expect(abweichend).toEqual(['gearEffects']);
  });
});

describe('AC6 — kein Prüf-Eintrag fällt als reguläre Beute', () => {
  it('200 Läufe, kein einziger test-Eintrag', () => {
    // `test-set-*` ist wie `test-relic` an `test-warden` gebunden. Ohne die
    // Bindung fiele der Prüfsatz im Spiel — ein Spieler bekäme Gegenstände mit
    // runden, ausdrücklich unbalancierten Werten.
    const treffer = new Set<string>();
    let funde = 0;
    for (let seed = 0; seed < 200; seed++) {
      const d = rollLoot(lootStreamFor(seed), {
        instance: 5,
        difficulty: 1,
        endlessLevel: 0,
        bossId: 'first-crucible',
      });
      if (d !== null) {
        funde++;
        treffer.add(d.itemId);
      }
    }
    expect([...treffer].filter((id) => id.startsWith('test'))).toEqual([]);
    // ZUSICHERUNG: Es ist überhaupt Beute gefallen, und zwar vielfältige.
    // Ohne sie wäre der Fall auch grün, wenn `rollLoot` gar nichts liefert.
    expect(funde).toBe(200);
    expect(treffer.size).toBeGreaterThan(10);
  });

  it('bei test-warden fällt der Prüfsatz sehr wohl — die Bindung sperrt, sie löscht nicht', () => {
    // Die andere Hälfte: Wäre der Satz überall gesperrt, gäbe es ihn effektiv
    // nicht, und der Fall darüber wäre aus dem falschen Grund grün.
    const treffer = new Set<string>();
    for (let seed = 0; seed < 200; seed++) {
      const d = rollLoot(lootStreamFor(seed), {
        instance: 5,
        difficulty: 1,
        endlessLevel: 0,
        bossId: 'test-warden',
      });
      if (d !== null) treffer.add(d.itemId);
    }
    expect([...treffer].some((id) => id.startsWith('test-set'))).toBe(true);
  });
});
