import { describe, expect, it } from 'vitest';
import {
  CAST_FIGHT_OVER,
  CAST_OK,
  CAST_ON_COOLDOWN,
  CAST_ON_GCD,
  CAST_UNKNOWN,
  Combat,
  FIGHT_OVERPRESSURE,
  FIGHT_RUNNING,
  FIGHT_WON,
  alsSpellIndex,
} from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { STREAM_COUNT } from '../../src/core/rng.js';

function makeCombat(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Rechnet die laufenden Wirkungen von `fromMs` bis `toMs` durch. */
function stepUntil(combat: Combat, fromMs: number, toMs: number): void {
  for (let t = fromMs; t < toMs; t += TICK_MS) combat.stepPassive(t);
}

describe('Combat — Grundwerte aus den Datensätzen', () => {
  it('liest die drei Zauber und den Testboss fehlerfrei', () => {
    expect(CALCINATION.problems).toEqual([]);
    expect(FIRST_CRUCIBLE.problems).toEqual([]);
    const combat = makeCombat();
    // Drei Kampfzauber, dazu seit #110 der Unterbrecher QUENCH.
    expect(combat.spells.map((s) => s.id)).toEqual([
      'kindle',
      'smoulder',
      'pyre',
      // Issue #268 — bei ihrer Schule statt hinten. Die Begruendung steht in
      // spell-school.test.ts: Die Hinten-anfuegen-Regel schuetzte davor, dass
      // eine Umstellung STILL geschieht, und tools/ac6.sh macht sie sichtbar.
      'bellows',
      'searing',
      'quench',
      // Die drei Wege der Koagulation (Issue #79). Sie stehen HINTEN, damit
      // die Plätze 0 bis 3 weiterhin dasselbe bedeuten.
      'crust',
      'salve',
      'temper',
      // Die vier der Fixierung (Issue #100), ebenfalls hinten.
      'bind',
      'stall',
      'stay',
      'anchor',
      // Die zwei der Sublimation (Issue #201), ebenfalls hinten.
      //
      // **Diese Liste bleibt eine Liste.** Anders als die Zahlen, die ich in
      // diesem Issue abgeleitet habe, IST hier die Reihenfolge die Aussage:
      // Sie bestimmt die Indizes, und die stehen in jeder aufgezeichneten
      // Runde. Wer einen Zauber dazwischenschiebt, muss das hier sehen.
      'vapour',
      'reflux',
      // Die zwei der Zersetzung (Issue #97), ebenfalls hinten. Aus demselben
      // Grund wie oben: Die Reihenfolge IST hier die Aussage.
      'canker',
      'fester',
      // Der Prüf-Zauber zur Wirkzeit (Issue #304), ganz hinten. Er gehört
      // nicht zum Spiel — seine Stufe liegt über der Grenze, und beide
      // Stellen, die dem Spieler Zauber anbieten, filtern darüber. In DIESER
      // Liste steht er trotzdem: Sie ist der Index-Raum, und der ist für
      // Aufzeichnungen und Simulator derselbe.
      'test-cast',
      'test-buff',
    ]);
    expect(combat.boss.hp).toBe(4200);
    expect(combat.boss.overpressureMs).toBe(60_000);
    expect(combat.school.resource.cap).toBe(20);
    expect(combat.school.resource.highlightAt).toBe(15);
  });
});

describe('Combat — KINDLE', () => {
  it('macht 100 Schaden und gibt 3 Hitze', () => {
    const combat = makeCombat();
    const kindle = combat.indexOf('kindle');
    const outcome = combat.cast(kindle, 0);

    expect(outcome.result).toBe(CAST_OK);
    expect(outcome.damage).toBe(100);
    expect(combat.damageDealt).toBe(100);
    expect(combat.resource).toBe(3);
  });

  it('hat eine Abklingzeit von genau der globalen Wartezeit', () => {
    const combat = makeCombat();
    const kindle = combat.indexOf('kindle');
    combat.cast(kindle, 0);

    expect(combat.cooldownRemainingMs(kindle, 1490)).toBe(10);
    expect(combat.isOffCooldown(kindle, 1500)).toBe(true);
    expect(combat.isReady(kindle, 1500)).toBe(true);
  });
});

describe('Combat — Hitze', () => {
  it('verfällt um einen Punkt alle zwei Sekunden', () => {
    const combat = makeCombat();
    combat.setResource(10);
    stepUntil(combat, 0, 2000);
    expect(combat.resource).toBe(10);
    combat.stepPassive(2000);
    expect(combat.resource).toBe(9);
    stepUntil(combat, 2010, 4000);
    expect(combat.resource).toBe(9);
    combat.stepPassive(4000);
    expect(combat.resource).toBe(8);
  });

  it('fällt nicht unter null', () => {
    const combat = makeCombat();
    for (let t = 0; t <= 20_000; t += TICK_MS) combat.stepPassive(t);
    expect(combat.resource).toBe(0);
  });

  it('läuft an der Obergrenze über und zählt das mit', () => {
    const combat = makeCombat();
    combat.setResource(19);
    combat.cast(combat.indexOf('kindle'), 0); // +3 auf 19

    expect(combat.resource).toBe(20);
    expect(combat.resourceOverflow).toBe(2);
  });

  it('baut sich mit durchgehendem KINDLE und laufendem SMOULDER auf', () => {
    // STEP1-SPEC 1.4: 3/1,5 s + 1/2 s − 1/2 s = 2,0 Hitze je Sekunde.
    const combat = makeCombat();
    const kindle = combat.indexOf('kindle');
    const smoulder = combat.indexOf('smoulder');
    combat.cast(smoulder, 0);
    for (let t = TICK_MS; t <= 7500; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.isReady(kindle, t)) combat.cast(kindle, t);
    }
    // Nach 7,5 s ist die Bonus-Schwelle erreicht — der Kern der Entscheidung.
    expect(combat.resource).toBeGreaterThanOrEqual(15);
  });
});

describe('Combat — PYRE (AC2)', () => {
  it('verdoppelt ab genau 15 Hitze, nicht ab 14', () => {
    const at14 = makeCombat();
    at14.setResource(14);
    const outcome14 = at14.cast(at14.indexOf('pyre'), 0);

    const at15 = makeCombat();
    at15.setResource(15);
    const outcome15 = at15.cast(at15.indexOf('pyre'), 0);

    expect(outcome14.damage).toBe(168); // 12 × 14, kein Bonus
    expect(outcome14.bonusApplied).toBe(false);
    expect(outcome15.damage).toBe(360); // 12 × 15 × 2
    expect(outcome15.bonusApplied).toBe(true);
  });

  it('rechnet die Beispiele aus STEP1-SPEC 1.3 nach', () => {
    const cases: readonly [number, number][] = [
      [10, 120],
      [14, 168],
      [15, 360],
      [20, 480],
    ];
    for (const [heat, expected] of cases) {
      const combat = makeCombat();
      combat.setResource(heat);
      expect(combat.cast(combat.indexOf('pyre'), 0).damage).toBe(expected);
    }
  });

  it('setzt die Hitze auf null', () => {
    const combat = makeCombat();
    combat.setResource(17);
    combat.cast(combat.indexOf('pyre'), 0);
    expect(combat.resource).toBe(0);
  });

  it('macht ohne Hitze keinen Schaden, verbraucht aber die Wartezeit', () => {
    const combat = makeCombat();
    const pyre = combat.indexOf('pyre');
    expect(combat.cast(pyre, 0).damage).toBe(0);
    expect(combat.gcd.isReady(0)).toBe(false);
    expect(combat.cooldownRemainingMs(pyre, 0)).toBe(5000);
  });
});

describe('Combat — SMOULDER (AC3)', () => {
  it('tickt alle zwei Sekunden für 40 Schaden und einen Punkt Hitze', () => {
    // Bewusst bei 500 ms gewirkt: Bei einem Start auf 0 fielen Tick und Verfall
    // auf denselben Takt, und der gewonnene Punkt wäre sofort wieder weg.
    const combat = makeCombat();
    combat.cast(combat.indexOf('smoulder'), 500);
    expect(combat.damageDealt).toBe(0);

    stepUntil(combat, 510, 2500);
    expect(combat.damageDealt).toBe(0);
    combat.stepPassive(2500);
    expect(combat.damageDealt).toBe(40);
    expect(combat.resource).toBe(1);
  });

  it('lässt Tick und Verfall im selben Takt nacheinander wirken', () => {
    // Reihenfolge wie in `tools/step1-check.py`: erst der Tick, dann der Verfall.
    const combat = makeCombat();
    combat.cast(combat.indexOf('smoulder'), 0);
    stepUntil(combat, TICK_MS, 2000);
    combat.stepPassive(2000);

    expect(combat.damageDealt).toBe(40);
    expect(combat.resource).toBe(0); // +1 durch den Tick, −1 durch den Verfall
  });

  it('verwirft beim Erneuern die Restzeit — kein Überlagern', () => {
    const combat = makeCombat();
    const smoulder = combat.indexOf('smoulder');
    combat.cast(smoulder, 0);
    // Bei 6 s: 6 s Restzeit, drei Ticks gelaufen.
    stepUntil(combat, TICK_MS, 6000);
    combat.stepPassive(6000);
    expect(combat.dotTickCount(smoulder)).toBe(3);
    expect(combat.dotRemainingMs(smoulder, 6000)).toBe(6000);

    combat.cast(smoulder, 6000);
    expect(combat.dotRemainingMs(smoulder, 6000)).toBe(12_000);
    // Die drei Ticks, die in der Restzeit noch gekommen wären, sind verloren.
    expect(combat.lostDotTicks(smoulder)).toBe(3);

    // Der nächste Tick kommt zwei Sekunden nach dem Erneuern, nicht früher.
    stepUntil(combat, 6010, 8000);
    expect(combat.dotTickCount(smoulder)).toBe(3);
    combat.stepPassive(8000);
    expect(combat.dotTickCount(smoulder)).toBe(4);
  });

  it('läuft nach der vollen Dauer aus', () => {
    const combat = makeCombat();
    const smoulder = combat.indexOf('smoulder');
    combat.cast(smoulder, 0);
    stepUntil(combat, TICK_MS, 20_000);

    // Zum Ablaufzeitpunkt selbst gilt der Effekt noch als aktiv — genau dort
    // fällt der sechste Tick. Erst danach ist er weg.
    expect(combat.isDotActive(smoulder, 12_000)).toBe(true);
    expect(combat.isDotActive(smoulder, 12_010)).toBe(false);
    expect(combat.dotRemainingMs(smoulder, 12_000)).toBe(0);
  });

  it('tickt sechsmal je Anwendung — der letzte Tick fällt mit dem Ablauf zusammen', () => {
    // Entschieden am 2026-07-28 (STEP1-SPEC § 1.2): Ticks bei 2, 4, 6, 8, 10 und
    // 12 s. Bei fünf Ticks wären die letzten zwei Sekunden wirkungslos, obwohl
    // die Restlaufzeit-Anzeige 12,0 s zeigt.
    const combat = makeCombat();
    const smoulder = combat.indexOf('smoulder');
    combat.cast(smoulder, 0);
    stepUntil(combat, TICK_MS, 20_000);

    expect(combat.dotTickCount(smoulder)).toBe(6);
    expect(combat.damageOf(smoulder)).toBe(240);
  });
});

describe('Combat — Wirken abweisen', () => {
  it('weist einen unbekannten Zauber ab', () => {
    const combat = makeCombat();
    // **Die 99 bleibt eine Zahl, und das ist Absicht** (Issue #369): Sie soll
    // gerade KEINEN Zauber treffen. Ein Bezeichner wäre hier falsch — es gibt
    // keinen für „gibt es nicht".
    // **Die sichtbare Umgehung** (Issue #372): `alsSpellIndex` sagt, dass hier
    // bewusst eine Zahl steht, die KEINEN Zauber trifft. Ein Bezeichner wäre
    // falsch — es gibt keinen für „gibt es nicht".
    expect(combat.cast(alsSpellIndex(99), 0).result).toBe(CAST_UNKNOWN);
    expect(combat.spells.length, 'die 99 ist keine unmögliche Zahl mehr').toBeLessThan(99);
  });

  it('weist einen Zauber auf Abklingzeit ab', () => {
    const combat = makeCombat();
    const pyre = combat.indexOf('pyre');
    combat.cast(pyre, 0);
    expect(combat.cast(pyre, 1500).result).toBe(CAST_ON_COOLDOWN);
    expect(combat.cast(pyre, 5000).result).toBe(CAST_OK);
  });

  it('weist jeden Zauber während der globalen Wartezeit ab', () => {
    const combat = makeCombat();
    combat.cast(combat.indexOf('kindle'), 0);
    expect(combat.cast(combat.indexOf('smoulder'), 500).result).toBe(CAST_ON_GCD);
    expect(combat.cast(combat.indexOf('pyre'), 1490).result).toBe(CAST_ON_GCD);
  });

  it('weist jeden Zauber nach dem Kampfende ab', () => {
    const combat = makeCombat();
    combat.stepPassive(60_000);
    combat.checkDefeat(60_000);
    expect(combat.fightState).toBe(FIGHT_OVERPRESSURE);
    expect(combat.cast(combat.indexOf('kindle'), 60_000).result).toBe(CAST_FIGHT_OVER);
  });
});

describe('Combat — Kampfende', () => {
  it('meldet den Sieg, sobald die Lebenspunkte aufgebraucht sind', () => {
    // Kleinerer Boss: KINDLE allein schafft in 60 s nur 4000 Schaden und käme am
    // echten Testboss (4200) nie an — das ist genau die Trennschärfe aus
    // STEP1-SPEC § 3 und hier nicht der Prüfgegenstand.
    const combat = new Combat({
      school: CALCINATION.value,
      boss: { ...FIRST_CRUCIBLE.value, hp: 400 },
    });
    const kindle = combat.indexOf('kindle');
    let timeMs = 0;
    while (combat.fightState === FIGHT_RUNNING && timeMs < 120_000) {
      combat.stepPassive(timeMs);
      if (combat.checkDefeat(timeMs)) break;
      if (combat.isReady(kindle, timeMs)) combat.cast(kindle, timeMs);
      timeMs += TICK_MS;
    }
    expect(combat.fightState).toBe(FIGHT_WON);
    expect(combat.damageDealt).toBeGreaterThanOrEqual(400);
    expect(combat.bossHpRemaining).toBe(0);
    expect(combat.bossHpFraction).toBe(0);
  });

  it('meldet den Überdruck nach 60 Sekunden', () => {
    const combat = makeCombat();
    combat.checkDefeat(59_990);
    expect(combat.fightState).toBe(FIGHT_RUNNING);
    combat.checkDefeat(60_000);
    expect(combat.fightState).toBe(FIGHT_OVERPRESSURE);
    expect(combat.endedAt).toBe(60_000);
  });
});

describe('Combat — Zufallsströme', () => {
  it('zieht in Schritt 1 aus keinem Strom', () => {
    // STEP1-SPEC § 2: PURITY ist auf Stufe 1 gleich 0, also gibt es keinen Krit.
    // Die Ströme müssen trotzdem angelegt sein (GAME-DESIGN 18.4).
    const combat = makeCombat();
    const kindle = combat.indexOf('kindle');
    let timeMs = 0;
    while (timeMs < 30_000) {
      combat.stepPassive(timeMs);
      if (combat.isReady(kindle, timeMs)) combat.cast(kindle, timeMs);
      timeMs += TICK_MS;
    }
    expect(combat.random.totalDraws).toBe(0);
    // An STREAM_COUNT gebunden statt hart eingetragen: Die Aussage ist
    // „aus KEINEM Strom gezogen", nicht „es gibt genau drei".
    //
    // Der Anlass ist wieder weggefallen — beim ersten Entwurf von #115 bekam
    // die Beute einen vierten Strom, und diese Zeile fiel als [0, 0, 0] um,
    // obwohl die geprüfte Eigenschaft unverändert galt. Der Strom liegt
    // inzwischen außerhalb (Entscheidung vom 2026-08-04), STREAM_COUNT steht
    // wieder bei 3. Die Bindung bleibt trotzdem: Sie kostet nichts und
    // beschreibt, was gemeint ist.
    expect(combat.random.drawCounts()).toEqual(new Array<number>(STREAM_COUNT).fill(0));
  });
});

describe('Combat — zurücksetzen', () => {
  it('stellt den Ausgangszustand vollständig her', () => {
    const combat = makeCombat();
    combat.cast(combat.indexOf('kindle'), 0);
    combat.cast(combat.indexOf('smoulder'), 1500);
    stepUntil(combat, 1510, 5000);
    combat.reset();

    expect(combat.damageDealt).toBe(0);
    expect(combat.resource).toBe(0);
    expect(combat.resourceOverflow).toBe(0);
    expect(combat.fightState).toBe(FIGHT_RUNNING);
    // **Über den Bezeichner, nicht über die Zahl** (Issue #369). Eine 0 hier
    // zeigt auf den Zauber, der heute vorn steht.
    expect(combat.castCount(combat.indexOf('kindle'))).toBe(0);
    expect(combat.isReady(combat.indexOf('kindle'), 0)).toBe(true);
    expect(combat.dotRemainingMs(combat.indexOf('smoulder'), 0)).toBe(0);
  });
});
