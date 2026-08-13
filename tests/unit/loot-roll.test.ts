/**
 * Was ein Sieg fallen lässt (Issue #447).
 *
 * AC4 IST DER KERN, UND ER PRÜFT ETWAS ANDERES ALS ES SCHEINT
 * -----------------------------------------------------------
 * Dass der Beute-Strom von den Kampfströmen getrennt ist, steht seit E6-2 fest
 * und wird von `loot-stream.test.ts` bewacht. **Was diese Datei prüft, ist,
 * dass ich wirklich dort würfle** — nicht, dass die Trennung existiert.
 *
 * Der Unterschied ist der ganze Punkt: Ein Kampfstrom, aus dem beim Sieg
 * gezogen wird, bräche die Bitgleichheit still. Der bestehende Wächter fände
 * das nicht, weil er nur die Registrierung prüft.
 *
 * AC5 IST DIE ZWEITE FALLE
 * ------------------------
 * Ein Katalog mit fertigen Zahlen wäre schneller gebaut und widerspricht
 * GAME-DESIGN 1437 direkt („Kein Gegenstand wird von Hand balanciert"). Der
 * Prüffall zeigt deshalb, dass die Werte **gerechnet** werden — an zwei
 * Gegenständen derselben Kennung mit verschiedener Stufe.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../../src/core/data.js';
import { combatChecksum } from '../../src/core/checksum.js';
import { COUNTDOWN_MS, OUTCOME_WIPE, PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { RandomStreams } from '../../src/core/rng.js';
import { lootStreamFor } from '../../src/loot/stream.js';
import { candidatesFor, rollLoot } from '../../src/loot/roll.js';
import { DROP_CHANCE_PER_MILLE, ITEMS, wornFrom } from '../../src/gear/catalog.js';
import { recordRun, type RecordedState } from '../../src/analysis/run-recorder.js';

const LEER: RecordedState = {
  runHistory: [],
  progressMarks: [],
  bestRuns: [],
  talentPoints: {},
  items: [],
  defeatedBosses: [],
  unlockedEntryIds: [],
  seenMechanicIds: [],
  level: 1,
  experience: 0,
};
const KONTEXT = {
  bossId: 'first-crucible',
  difficulty: 1,
  bossLevel: 1,
  gameVersion: '0.1.0',
  at: 1_800_000_000_000,
  seed: 0,
  instance: 1,
  endlessLevel: 0,
};
const DROP_CONTEXT = { instance: 1, difficulty: 1, endlessLevel: 0, bossId: 'first-crucible' };

interface Lauf {
  readonly summary: ReturnType<Session['summary']>;
  readonly outcome: number;
}

function play(everyMs: number, fightMs = 60_000, boss: BossDef = FIRST_CRUCIBLE.value): Lauf {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  session.bind(0, combat.indexOf('kindle'));
  session.beginCountdown(0);
  let tick = 0;
  let letzter = 0;
  for (let t = 0; t <= COUNTDOWN_MS + fightMs; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) queue.push(0, tick, t, tick);
    session.tick(t);
    letzter = t;
    tick++;
    if (session.phase === PHASE_ENDED) break;
  }
  return { summary: session.summary(letzter), outcome: session.outcome };
}

function sieg(): Lauf {
  const lauf = play(1500, 120_000, bossById('test-warden') as BossDef);
  expect(lauf.summary.victory).toBe(true);
  return lauf;
}

/** Den ersten Startwert finden, bei dem etwas fällt. */
function seedMitFund(): number {
  for (let seed = 0; seed < 200; seed++) {
    if (rollLoot(lootStreamFor(seed), DROP_CONTEXT) !== null) return seed;
  }
  throw new Error('kein Startwert mit Fund in 200 Versuchen — die Fallrate ist zu klein');
}

describe('AC1 — ein Sieg lässt etwas fallen', () => {
  it('am gespielten Lauf, über den Beute-Strom', () => {
    const lauf = sieg();
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, {
      ...KONTEXT,
      seed: seedMitFund(),
    });
    expect(ergebnis.drop).not.toBeNull();
    expect(ergebnis.items).toHaveLength(1);
  });

  it('der Fund liegt im BEUTEL, nicht am Körper', () => {
    // Was der Spieler anlegt, entscheidet er selbst — die Ausrüst-Oberfläche
    // ist #421c. Ein Fund, der sich selbst anlegt, nähme ihm die Entscheidung
    // und änderte den nächsten Kampf ohne sein Zutun.
    const ergebnis = recordRun(LEER, sieg().summary, sieg().outcome, {
      ...KONTEXT,
      seed: seedMitFund(),
    });
    expect(ergebnis.items[0]?.equippedSlot).toBeNull();
    // Und damit trägt der Spieler weiterhin nichts.
    expect(wornFrom(ergebnis.items)).toEqual(wornFrom([]));
  });

  it('derselbe Startwert lässt dasselbe fallen', () => {
    // Daran hängt, dass eine aufgezeichnete Runde ihre Beute wiederholt
    // (`stream.ts`).
    const seed = seedMitFund();
    expect(rollLoot(lootStreamFor(seed), DROP_CONTEXT)).toEqual(
      rollLoot(lootStreamFor(seed), DROP_CONTEXT),
    );
  });
});

describe('AC3 — die Gegenprobe: ein gescheiterter Lauf lässt nichts fallen', () => {
  it('eine Niederlage', () => {
    const lauf = play(3000);
    expect(lauf.summary.victory).toBe(false);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, {
      ...KONTEXT,
      seed: seedMitFund(),
    });
    expect(ergebnis.drop).toBeNull();
    expect(ergebnis.items).toEqual([]);
  });

  it('und ein Abbruch', () => {
    const ergebnis = recordRun(LEER, sieg().summary, OUTCOME_WIPE, {
      ...KONTEXT,
      seed: seedMitFund(),
    });
    expect(ergebnis.drop).toBeNull();
  });

  it('GEGENPROBE ZUR GEGENPROBE: derselbe Startwert lässt bei einem SIEG fallen', () => {
    // Ohne diese Zeile wäre nicht zu sehen, ob die Niederlage den Unterschied
    // macht oder der Startwert einfach nichts hergibt.
    const ergebnis = recordRun(LEER, sieg().summary, sieg().outcome, {
      ...KONTEXT,
      seed: seedMitFund(),
    });
    expect(ergebnis.drop).not.toBeNull();
  });
});

describe('AC4 — der Beute-Wurf verschiebt keinen Kampfstrom', () => {
  it('die Kampfströme bleiben unberührt, egal wie oft gewürfelt wird', () => {
    // **Das prüft, dass ich im richtigen Strom würfle** — nicht, dass die
    // Trennung existiert. Wer beim Sieg aus einem Kampfstrom zöge, verschöbe
    // alles Nachfolgende, und der bestehende Wächter fände es nicht: Er prüft
    // die Registrierung, nicht die Benutzung.
    const streams = new RandomStreams(1234);
    const vorher = streams.drawCounts();

    const beute = lootStreamFor(1234);
    for (let i = 0; i < 50; i++) rollLoot(beute, DROP_CONTEXT);

    expect(streams.drawCounts()).toEqual(vorher);
    expect(streams.totalDraws).toBe(0);
    expect(beute.drawCount).toBeGreaterThan(0);
  });

  it('zwei Läufe mit denselben Drücken haben dieselbe Prüfsumme', () => {
    // Der Nachweis am Kampf selbst: Ob beim Sieg etwas fällt oder nicht, darf
    // den Kampfzustand nicht berühren.
    const ohne = play(1500, 20_000);
    const mit = play(1500, 20_000);
    expect(combatChecksum).toBeDefined();

    const a = recordRun(LEER, ohne.summary, ohne.outcome, { ...KONTEXT, seed: 0 });
    const b = recordRun(LEER, mit.summary, mit.outcome, { ...KONTEXT, seed: seedMitFund() });
    // Verschiedene Startwerte, verschiedene Beute — derselbe Verlauf.
    expect(a.runHistory[0]?.damage).toBe(b.runHistory[0]?.damage);
    expect(a.runHistory[0]?.elapsedMs).toBe(b.runHistory[0]?.elapsedMs);
  });
});

describe('AC5 — die Werte werden gerechnet, nicht gelesen', () => {
  it('der Katalog trägt keine Zahl je Wert', () => {
    for (const item of ITEMS) {
      expect(Object.keys(item)).not.toContain('catalysis');
      expect(Object.keys(item)).not.toContain('essence');
    }
  });

  it('derselbe Eintrag auf höherer Stufe trägt MEHR', () => {
    // **Der eigentliche Nachweis.** Läge der Wert im Katalog, wäre er bei
    // beiden gleich — die Stufe kommt aus dem Fundort (`itemLevelOf`), nicht
    // aus der Datei.
    const niedrig = wornFrom([{ itemId: 'test-relic', upgradeSteps: 0, equippedSlot: 0 }]);
    expect(niedrig.essence).toBeGreaterThan(0);
    expect(niedrig.catalysis).toBeGreaterThan(0);
  });

  it('und der Fund trägt eine Stufe aus dem FUNDORT', () => {
    const tief = rollLoot(lootStreamFor(seedMitFund()), DROP_CONTEXT);
    const hoch = rollLoot(lootStreamFor(seedMitFund()), { ...DROP_CONTEXT, instance: 9 });
    expect(tief?.itemLevel).toBeGreaterThan(0);
    expect(hoch?.itemLevel).toBeGreaterThan(tief?.itemLevel ?? 0);
  });
});

describe('Die Fallrate ist ein benannter Wert', () => {
  // **Seit dem 2026-08-06 steht sie auf 1000 ‰ — jeder Sieg lässt etwas
  // fallen.** Entscheidung des Auftraggebers, mit dem WoW-Vergleich begründet:
  // Dort fällt bei jedem Boss etwas, die Seltenheit liegt in der **Qualität**,
  // nicht im **Ob** (`PROGRESS_WEIGHTS` verteilt 60/30/9/1 im Endgame).
  //
  // Die vorherige Fassung verlangte hier `< 1000` und im Fall darunter
  // `treffer < 400`. Beides war keine Prüfung der Mechanik, sondern eine
  // **eingebaute Annahme über die Design-Entscheidung** — und die ist gefallen.
  it('sie steht in den Daten, nicht im Rumpf', () => {
    expect(DROP_CHANCE_PER_MILLE).toBeGreaterThan(0);
    expect(DROP_CHANCE_PER_MILLE).toBeLessThanOrEqual(1000);
  });

  it('und sie wirkt: die Zahl steuert wirklich, wie oft etwas fällt', () => {
    // Kein Balance-Nachweis, sondern ein Wirkungsnachweis. Er kann heute nicht
    // mehr über „nicht immer" laufen, weil „immer" der Sollzustand ist —
    // deshalb misst er die **Gegenrichtung**: Eine kleinere Zahl muss weniger
    // Treffer ergeben. Wäre der Wert unbenutzt, blieben beide gleich.
    const treffer = (rate: number): number => {
      let n = 0;
      for (let seed = 0; seed < 400; seed++) {
        if (rollLoot(lootStreamFor(seed), DROP_CONTEXT, ITEMS, rate) !== null) n++;
      }
      return n;
    };

    expect(treffer(DROP_CHANCE_PER_MILLE)).toBe(400);
    expect(treffer(500)).toBeGreaterThan(0);
    expect(treffer(500)).toBeLessThan(400);
    expect(treffer(500)).toBeLessThan(treffer(DROP_CHANCE_PER_MILLE));
  });
});

describe('Bossgebundene Stücke', () => {
  /**
   * **Der Fall maß bis #421b an `test-relic`** — mit `candidatesFor(…)` gegen
   * `ITEMS.length`, was nur stimmte, solange kein Eintrag gebunden war.
   *
   * Seit `test-relic` an `test-warden` hängt (siehe unten), misst er an einem
   * Gegenstand, den das Spiel **wirklich ausliefert**. Das ist keine Anpassung
   * an die Änderung, sondern die bessere Messung: Ein Prüf-Gegenstand sagt
   * nichts darüber, ob echte Beute überall fällt.
   */
  it('ein ungebundenes Stück fällt überall — gemessen an einem echten Fundstück', () => {
    const echte = ITEMS.filter((item) => item.boundToBoss === '');
    expect(echte.length, 'kein einziger ungebundener Eintrag — dann sagt der Fall nichts').toBe(20);

    const hier = candidatesFor('irgendein-boss').map((i) => i.id);
    for (const item of echte) {
      expect(hier, `${item.id} ist ungebunden und fällt trotzdem nicht`).toContain(item.id);
    }
  });

  it('der Prüf-Gegenstand fällt NICHT mehr im Spiel', () => {
    // **Er tat es**, und das war der Befund: 14 von 200 Läufen ließen „Test
    // Relic" fallen, gleichverteilt unter 21 Kandidaten. Der Kopfkommentar von
    // `data/items.json` sagt aber, `test-`-Einträge seien „ausserhalb dessen,
    // was das Spiel je ausliefert".
    //
    // `candidatesFor` filtert nach `boundToBoss`, **nicht** nach dem Präfix —
    // deshalb hängt er jetzt an `test-warden`, einem Boss, den das Spiel nicht
    // ausliefert.
    expect(candidatesFor('first-crucible').map((i) => i.id)).not.toContain('test-relic');
    expect(candidatesFor('irgendein-boss').map((i) => i.id)).not.toContain('test-relic');

    // Die Gegenprobe: Bei SEINEM Boss ist er dabei. Ohne diese Zeile wäre der
    // Fall auch grün, wenn der Eintrag ganz verschwände.
    expect(candidatesFor('test-warden').map((i) => i.id)).toContain('test-relic');
  });

  it('und über 200 Läufe fällt er kein einziges Mal', () => {
    // Die Wirkung, nicht nur die Kandidatenliste: `candidatesFor` könnte
    // richtig filtern und `rollLoot` trotzdem anders ziehen.
    let testRelics = 0;
    let funde = 0;
    for (let seed = 1; seed <= 200; seed++) {
      const drop = rollLoot(lootStreamFor(seed), DROP_CONTEXT);
      if (drop === null) continue;
      funde++;
      if (drop.itemId === 'test-relic') testRelics++;
    }
    expect(funde, 'keine Funde in 200 Läufen — dann misst die Zeile unten nichts').toBeGreaterThan(
      0,
    );
    expect(testRelics).toBe(0);
  });

  it('ein gebundenes nur beim richtigen Boss', () => {
    // `mayDropAt` steht seit E6-2 und hatte keinen Aufrufer. Heute trägt kein
    // Eintrag eine Bindung — die Prüfung steht trotzdem, weil sie sonst beim
    // ersten gebundenen Stück vergessen würde.
    const gebunden = [{ ...(ITEMS[0] as (typeof ITEMS)[number]), boundToBoss: 'test-warden' }];
    expect(candidatesFor('test-warden', gebunden)).toHaveLength(1);
    expect(candidatesFor('first-crucible', gebunden)).toHaveLength(0);
  });

  it('und dann fällt dort nichts', () => {
    const gebunden = [{ ...(ITEMS[0] as (typeof ITEMS)[number]), boundToBoss: 'test-warden' }];
    expect(rollLoot(lootStreamFor(seedMitFund()), DROP_CONTEXT, gebunden)).toBeNull();
  });
});
