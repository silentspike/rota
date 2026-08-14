/**
 * Die vierte Instanz — vier Bosse, die je einen VORHANDENEN Baustein ein
 * zweites Mal tragen (Auftrag 6494).
 *
 * # Warum ein zweiter Träger, statt neue Mechaniken zu bauen
 *
 * Fünf Bausteine kamen im ausgelieferten Spiel je genau einmal vor: `energy`,
 * `stacks`, `resistances`, `escalation` und `runaway`. Der Auftraggeber hat
 * entschieden, vier davon zu vertiefen statt neue hinzuzufügen:
 *
 * > Eine Mechanik, die einmal vorkommt, überrascht. Eine, die zweimal
 * > vorkommt, muss man KÖNNEN — beim zweiten Mal ist sie kein Ereignis mehr,
 * > sondern eine Anforderung.
 *
 * # Warum `runaway` als einziger draußen bleibt
 *
 * Sein einziger Träger (`boss-2-3`) kehrt das **Ziel des Kampfes** um: Der Ring
 * sinkt von selbst, und die Frist zu überstehen IST der Sieg. Ein zweiter
 * Träger wäre deshalb keine Variation derselben Mechanik, sondern eine zweite
 * Entscheidung darüber, **wie oft das Spiel sein Gewinnkriterium umdreht** —
 * etwas anderes als ein zweiter Energie- oder Stapel-Boss.
 *
 * Damit bleiben genau vier für vier Bosse, ohne dass eine Mechanik doppelt
 * vorkommt oder eine Stelle leer bleibt.
 *
 * # Was dieser Fall zusätzlich zu `instanz-3.test.ts` prüft
 *
 * Dort ging es darum, DASS sich die vier unterscheiden. Hier kommt die Frage
 * dazu, ob jeder sich auch von **seinem Vorgänger** unterscheidet — und zwar
 * in der Bauform, nicht nur in einer Zahl. Ein zweiter Träger, der dieselbe
 * Kurve mit anderen Ziffern fährt, wäre keine Vertiefung, sondern eine Kopie.
 */
import { describe, expect, it } from 'vitest';

import { BOSSES, bossById, CALCINATION, INSTANCES, instanceById } from '../../src/core/data.js';
import type { BossDef } from '../../src/core/data.js';
import { Combat } from '../../src/core/combat.js';
import { runReference } from '../../src/devtools/reference-run.js';
import { freigeschalteteInstanzen } from '../../src/progression/boss-select.js';

const VIER = ['boss-4-1', 'boss-4-2', 'boss-4-3', 'boss-4-4'] as const;

function boss(id: string): BossDef {
  const b = bossById(id);
  expect(b, `Boss ${id} fehlt`).toBeDefined();
  return b as BossDef;
}

/** Derselbe Referenzlauf wie in `instanz-3.test.ts` — dort steht seine Grenze. */
function lauf(bossId: string): { damage: number; killedAtMs: number; victory: boolean } {
  const ergebnis = runReference('good', { bossId }) as { damage: number; killedAtMs: number };
  return {
    damage: ergebnis.damage,
    killedAtMs: ergebnis.killedAtMs,
    victory: ergebnis.killedAtMs >= 0,
  };
}

describe('Die vier sind bezwingbar — gespielt, nicht behauptet', () => {
  it.each(VIER)('%s fällt in einem vollen Lauf', (id) => {
    const b = boss(id);
    const ergebnis = lauf(id);
    expect(
      ergebnis.victory,
      `${id} überlebt: ${ergebnis.damage} Schaden von ${b.hp} Lebenspunkten — ` +
        // Die alte Fassung nannte hier „test-doomsayer: 5168 von 6400". Beide
        // Zahlen sind überholt und die zweite war nie richtig: Der Boss hat
        // 999999 HP, die 6400 gehören boss-1-2 (gemessen 2026-08-14, #672).
        // Was gemeint war, ist der Zustand SELBST — ein Boss, der stehenbleibt.
        `das ist der Zustand aus #672: ein Lauf, der nicht durchkommt.`,
    ).toBe(true);
  });

  it('GEGENPROBE: die Anordnung KANN scheitern', () => {
    // Vier Siege gegen eine Anordnung, die immer gewinnt, sagen nichts.
    expect(lauf('boss-1-2').victory).toBe(false);
  });

  it('und das Kriterium ist für DIESE vier zuständig — keiner trägt ein Wettrennen', () => {
    // **Die Auflage aus 6494**, und der Grund steht im Warnkasten von
    // `instanz-3.test.ts`: Bei einem Boss mit `race` versagt `killedAtMs >= 0`
    // lautlos, weil er sich bei verlorenem Rennen zurückheilt und
    // `damageDealt` netto zählt.
    //
    // Bekäme einer der vier später ein Wettrennen, wird DIESER Fall rot und
    // nennt den Namen — dann braucht es einen anderen Maßstab (`raceWon`), der
    // heute ungemessen ist und deshalb nicht geraten wird.
    const mitRennen = VIER.filter((id) => boss(id).race !== null);
    expect(
      mitRennen,
      `diese tragen ein Wettrennen, der Bezwingbarkeits-Maßstab passt nicht mehr: ${mitRennen.join(', ')}`,
    ).toEqual([]);
  });
});

describe('Jeder trägt seinen Baustein ANDERS als der bestehende Träger', () => {
  it('boss-4-1 kehrt den Widerstand um — Verwundbarkeit statt Härte', () => {
    // boss-3-1 senkt den Schaden (0.8), dieser hebt ihn (1.25). Nicht dieselbe
    // Mechanik mit anderer Zahl, sondern ihre Gegenrichtung: Bis heute gab es
    // Verwundbarkeit im ausgelieferten Spiel überhaupt nicht.
    // **Über die Werte, nicht über einen wiederholten Schlüssel.** Der Name
    // der Schule steht in den Boss-Daten (`"CALCINATION"`) und wird vom Parser
    // unverändert übernommen; ihn hier noch einmal hinzuschreiben wäre eine
    // zweite Wahrheit, die bei einer Umbenennung stillschweigend danebenläge.
    const alt = Object.values(boss('boss-3-1').resistances);
    const neu = Object.values(boss('boss-4-1').resistances);
    expect(alt, 'boss-3-1 trägt keinen Widerstand mehr').not.toHaveLength(0);
    expect(neu, 'boss-4-1 trägt keine Verwundbarkeit mehr').not.toHaveLength(0);
    expect(Math.max(...alt), 'boss-3-1 senkt den Schaden nicht mehr').toBeLessThan(1);
    expect(Math.min(...neu), 'boss-4-1 hebt den Schaden nicht mehr').toBeGreaterThan(1);
    // Und sie betreffen DIESELBE Schule — sonst wäre es keine Umkehrung,
    // sondern ein anderer Gegenstand.
    expect(Object.keys(boss('boss-4-1').resistances)).toEqual(
      Object.keys(boss('boss-3-1').resistances),
    );
  });

  it('boss-4-2 entlädt die Leiste nur TEILWEISE', () => {
    // Der einzige Unterschied ist `resetTo` — und er verändert den Rhythmus:
    // Bei boss-3-2 liegen zwischen zwei Auslösern immer dieselbe Zeit, hier
    // rücken sie nach dem ersten zusammen.
    expect(boss('boss-3-2').energy?.resetTo, 'boss-3-2 startet nicht mehr bei 0').toBe(0);
    expect(boss('boss-4-2').energy?.resetTo ?? 0).toBeGreaterThan(0);
  });

  it('boss-4-3 baut die Stapel später auf, dafür steiler und höher', () => {
    const alt = boss('boss-3-3').stacks;
    const neu = boss('boss-4-3').stacks;
    expect(alt, 'boss-3-3 trägt keine Stapel mehr').not.toBeNull();
    expect(neu, 'boss-4-3 trägt keine Stapel mehr').not.toBeNull();
    // Alle drei Größen unterscheiden sich — eine allein wäre Zahlenkosmetik.
    expect(neu!.perStepMs).toBeGreaterThan(alt!.perStepMs);
    expect(neu!.factor).toBeGreaterThan(alt!.factor);
    expect(neu!.maxStacks).toBeGreaterThan(alt!.maxStacks);
  });

  it('boss-4-4 verkürzt in kleineren Schritten, dafür viel weiter', () => {
    const alt = boss('boss-1-1').escalation;
    const neu = boss('boss-4-4').escalation;
    expect(alt, 'boss-1-1 trägt keine Eskalation mehr').not.toBeNull();
    expect(neu, 'boss-4-4 trägt keine Eskalation mehr').not.toBeNull();
    expect(neu!.castTimeShrinkMs).toBeLessThan(alt!.castTimeShrinkMs);
    expect(neu!.minCastTimeMs).toBeLessThan(alt!.minCastTimeMs);
  });
});

describe('Und der Unterschied WIRKT — im Kampf, nicht in den Daten', () => {
  it('die Verwundbarkeit lässt boss-4-1 früher fallen als alle drei anderen', () => {
    // **Der Wirkungsnachweis für den Baustein.** Er trägt die MEISTEN
    // Lebenspunkte der Instanz und fällt trotzdem als Erster — das kann nur an
    // den Widerständen liegen, denn die Rotation ist bei allen vier dieselbe.
    const zeiten = new Map(VIER.map((id) => [id, lauf(id).killedAtMs]));
    const meins = zeiten.get('boss-4-1') as number;
    const andere = VIER.filter((id) => id !== 'boss-4-1').map((id) => zeiten.get(id) as number);

    expect(
      boss('boss-4-1').hp,
      'boss-4-1 hat nicht mehr die meisten Lebenspunkte — die Aussage unten trägt dann nicht',
    ).toBe(Math.max(...VIER.map((id) => boss(id).hp)));
    expect(
      meins,
      `boss-4-1 fällt bei ${meins} ms, die anderen bei ${andere.join(', ')} — ` +
        `die Verwundbarkeit wirkt nicht mehr`,
    ).toBeLessThan(Math.min(...andere));
  });

  it('die Teilentladung von boss-4-2 löst öfter aus als ein Rücksprung auf 0', () => {
    // Gemessen an der Sache selbst: derselbe Kampf über 60 s, einmal gegen
    // boss-4-2 (resetTo 45) und einmal gegen boss-3-2 (resetTo 0). Die Zahl
    // der Auslöser ist der Unterschied — nicht die Zahl im Datenfeld.
    const ausloeser = (id: string): number => {
      const c = new Combat({ school: CALCINATION.value, boss: boss(id) });
      for (let t = 0; t <= 60000; t += 100) c.stepPassive(t);
      return c.bossEnergyTriggers;
    };
    const neu = ausloeser('boss-4-2');
    const alt = ausloeser('boss-3-2');
    expect(
      neu,
      `boss-4-2 löst ${neu}-mal aus, boss-3-2 ${alt}-mal — die Teilentladung wirkt nicht`,
    ).toBeGreaterThan(alt);
  });

  it('und die vier Läufe sind vier verschiedene Läufe', () => {
    const zeiten = VIER.map((id) => lauf(id).killedAtMs);
    expect(new Set(zeiten).size, `gleiche Läufe: ${zeiten.join(', ')}`).toBe(4);
  });
});

describe('Die Instanz und ihre Freischaltung (12.7)', () => {
  it('steht in den Daten und verlangt instance-3', () => {
    expect(instanceById('instance-4')?.requires).toBe('instance-3');
    expect(INSTANCES.map((i) => i.id)).toContain('instance-4');
  });

  it('trägt eine Beute-Nummer ab 1', () => {
    // `itemLevelOf` und `qualityCapOf` rechnen mit dieser Zahl (13.4).
    expect(instanceById('instance-4')?.number).toBeGreaterThanOrEqual(1);
  });

  it('bleibt zu, solange instance-3 offen ist', () => {
    expect(freigeschalteteInstanzen([], BOSSES)).not.toContain('instance-4');
  });

  it('öffnet sich, sobald ALLE vier Bosse von instance-3 liegen', () => {
    const vorher = ['boss-1-1', 'boss-1-2', 'boss-1-3', 'boss-1-4'];
    const instanz2 = ['boss-2-1', 'boss-2-2', 'boss-2-3', 'boss-2-4'];
    const instanz3 = ['boss-3-1', 'boss-3-2', 'boss-3-3', 'boss-3-4'];
    expect(freigeschalteteInstanzen([...vorher, ...instanz2, ...instanz3], BOSSES)).toContain(
      'instance-4',
    );
  });

  it('und NICHT, wenn drei von vier liegen', () => {
    const fast = [
      'boss-1-1',
      'boss-1-2',
      'boss-1-3',
      'boss-1-4',
      'boss-2-1',
      'boss-2-2',
      'boss-2-3',
      'boss-2-4',
      'boss-3-1',
      'boss-3-2',
      'boss-3-3',
    ];
    expect(freigeschalteteInstanzen(fast, BOSSES)).not.toContain('instance-4');
  });
});
