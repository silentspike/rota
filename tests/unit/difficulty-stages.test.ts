/**
 * Drei Schwierigkeitsstufen je Boss (GAME-DESIGN 11.7, Issue #1177, Abschnitt 2).
 *
 * > Normal / Heroisch / Mythisch pro Boss — mehr Leben, härteres Zeitlimit,
 * > zusätzliche Mechaniken, mehr Punkte. (Masterplan A.4)
 *
 * # Was hier bricht
 *
 * Eine Stufe, die von der darunter **nicht abweicht** — im Leben, im Zeitlimit
 * und in der Mechanik, jedes einzeln. „Heroisch“ als bloß dickerer Lebensbalken
 * wäre keine der drei Zusagen aus dem Plan, sondern eine davon; und eine Stufe
 * ohne neue Mechanik fiele in keinem Kampf auf, nur in diesem Fall.
 *
 * **Die Richtung zählt mit.** Beim umgekehrten Ziel (`runaway`) ist das
 * Überstehen der Frist der Sieg — dort heißt „härteres Zeitlimit“ eine LÄNGERE
 * Frist. Wer die Faktoren aus `data/difficulty.json` ungesehen übernähme, machte
 * diese Bosse auf Heroisch leichter, und eine Prüfung „weicht ab“ allein sähe
 * es nicht.
 */
import { describe, expect, it } from 'vitest';
import {
  BOSSES,
  BOSS_STAGES,
  INSTANCES,
  bossAtDifficulty,
  type BossDef,
} from '../../src/core/data.js';
import {
  DIFFICULTY_STAGES,
  bossTitle,
  deriveStage,
  pointsPermilleOf,
  scaledOverpressure,
} from '../../src/core/difficulty.js';
import { pointsBreakdown } from '../../src/progression/points.js';
import { DIFFICULTY_COUNT } from '../../src/loot/rates.js';

/** Die ausgelieferten Bosse: jene, deren Instanz in `instances.json` steht. */
function ausgelieferte(): BossDef[] {
  const instanzen = new Set(INSTANCES.map((i) => i.id));
  return [...BOSSES.values()].map((e) => e.value).filter((b) => instanzen.has(b.instance));
}

/**
 * Die Mechanik eines Datensatzes als vergleichbare Liste.
 *
 * Je Baustein ein Eintrag, die Zeitleiste je Eintrag mit Zeitpunkt und Art.
 * **Nicht die Zahlen darin**: Eine Stufe, die nur den Schaden eines vorhandenen
 * Schlags hebt, bringt keine Mechanik dazu — und genau das soll hier auffallen.
 */
function mechanik(boss: BossDef): string[] {
  const out: string[] = [];
  for (const [name, wert] of [
    ['energy', boss.energy],
    ['doom', boss.doom],
    ['stacks', boss.stacks],
    ['race', boss.race],
    ['affliction', boss.affliction],
    ['escalation', boss.escalation],
    ['bossHeal', boss.bossHeal],
    ['runaway', boss.runaway],
  ] as const) {
    if (wert !== null) out.push(name);
  }
  for (const eintrag of boss.timeline) out.push(`${eintrag.atMs}:${eintrag.mechanic}`);
  return out.sort();
}

/**
 * Was einer Stufe gegenüber der darunter fehlt — leer heißt: sie weicht in
 * allen drei Zusagen ab, in der richtigen Richtung.
 *
 * Als eigene Funktion, damit die Gegenprobe sie mit einer erfundenen, gleichen
 * Stufe füttern kann: Ein Wächter, der nie rot war, ist keiner.
 */
export function fehlendeAbweichung(unten: BossDef, oben: BossDef): string[] {
  const fehlt: string[] = [];
  const umgekehrt = unten.runaway !== null;
  if (umgekehrt) {
    // Ring auf 0 ist die Niederlage: mehr Leben hülfe, eine längere Frist nicht.
    if (oben.overpressureMs <= unten.overpressureMs) {
      fehlt.push(`Frist nicht länger (${unten.overpressureMs} -> ${oben.overpressureMs})`);
    }
    if (oben.hp > unten.hp) fehlt.push(`mehr Leben macht runaway leichter (${oben.hp})`);
  } else {
    if (oben.hp <= unten.hp) fehlt.push(`nicht mehr Leben (${unten.hp} -> ${oben.hp})`);
    if (oben.overpressureMs >= unten.overpressureMs) {
      fehlt.push(`Zeitlimit nicht kürzer (${unten.overpressureMs} -> ${oben.overpressureMs})`);
    }
  }
  const vorher = new Set(mechanik(unten));
  const neu = mechanik(oben).filter((m) => !vorher.has(m));
  if (neu.length === 0) fehlt.push('keine zusätzliche Mechanik');
  // **Kumulativ**: Was unten da war, bleibt oben da. Sonst wäre Mythisch ein
  // anderer Boss als Heroisch statt ein schwererer.
  const oberes = new Set(mechanik(oben));
  const verloren = [...vorher].filter((m) => !oberes.has(m));
  if (verloren.length > 0) fehlt.push(`verliert ${verloren.join(', ')}`);
  return fehlt;
}

describe('ZUSICHERUNG — es gibt etwas zu prüfen', () => {
  it('21 ausgelieferte Bosse, darunter beide mit umgekehrtem Ziel', () => {
    const bosse = ausgelieferte();
    expect(bosse.length).toBe(21);
    expect(bosse.filter((b) => b.runaway !== null).map((b) => b.id)).toEqual([
      'boss-2-3',
      'boss-5-3',
    ]);
  });

  it('die Stufen-Datei kennt genau drei Stufen in der Reihenfolge der Beute', () => {
    expect(DIFFICULTY_STAGES.map((s) => s.id)).toEqual(['normal', 'heroic', 'mythic']);
    expect(DIFFICULTY_STAGES.length).toBe(DIFFICULTY_COUNT);
  });
});

describe('AC1 — jeder ausgelieferte Boss hat drei Stufen', () => {
  it('keiner fehlt in der Stufen-Tabelle', () => {
    const ohne = ausgelieferte()
      .map((b) => b.id)
      .filter((id) => BOSS_STAGES.get(id)?.length !== DIFFICULTY_COUNT);
    expect(ohne, 'diese Bosse haben keine drei Stufen').toEqual([]);
  });

  it('jede Stufe liest sich ohne Problem', () => {
    const probleme: string[] = [];
    for (const [id, stufen] of BOSS_STAGES) {
      stufen.forEach((stufe, i) => {
        for (const p of stufe.problems) probleme.push(`${id}/${String(i)}: ${p}`);
      });
    }
    expect(probleme).toEqual([]);
  });

  it('Normal ist der Boss aus der Datei, unverändert', () => {
    for (const boss of ausgelieferte()) {
      const normal = bossAtDifficulty(boss.id, 0);
      expect(normal, boss.id).toBe(boss);
      expect(BOSS_STAGES.get(boss.id)?.[0]?.value, boss.id).toEqual(boss);
    }
  });

  it('jede Stufe weicht von der darunter ab — Leben, Zeitlimit, Mechanik', () => {
    const befunde: string[] = [];
    for (const boss of ausgelieferte()) {
      for (let stufe = 1; stufe < DIFFICULTY_COUNT; stufe++) {
        const unten = bossAtDifficulty(boss.id, stufe - 1);
        const oben = bossAtDifficulty(boss.id, stufe);
        if (unten === undefined || oben === undefined) {
          befunde.push(`${boss.id}: Stufe ${String(stufe)} fehlt`);
          continue;
        }
        for (const f of fehlendeAbweichung(unten, oben)) {
          befunde.push(`${boss.id} ${DIFFICULTY_STAGES[stufe]?.id ?? '?'}: ${f}`);
        }
      }
    }
    expect(befunde).toEqual([]);
  });

  it('die Stufe steht am Datensatz und in der Überschrift', () => {
    const heroisch = bossAtDifficulty('boss-1-1', 1);
    expect(heroisch?.difficulty).toBe(1);
    expect(heroisch?.id, 'die Kennung bleibt — Bestwert und Marke hängen an ihr').toBe('boss-1-1');
    expect(heroisch === undefined ? '' : bossTitle(heroisch)).toBe('Instance One, First · Heroic');
    const normal = bossAtDifficulty('boss-1-1', 0);
    expect(normal === undefined ? '' : bossTitle(normal)).toBe('Instance One, First');
  });

  it('ein Prüf-Boss hat keine heroische Fassung — und bekommt keine untergeschoben', () => {
    expect(bossAtDifficulty('test-doomsayer', 1)).toBeUndefined();
    expect(bossAtDifficulty('test-doomsayer', 0)?.id).toBe('test-doomsayer');
  });
});

describe('Die Ableitung — Regeln, die die Daten nicht zeigen können', () => {
  const grund = {
    schema: 1,
    id: 'probe',
    name: 'Probe',
    instance: 'test',
    hp: 1000,
    overpressureMs: 60000,
    timeline: [{ atMs: 5000, mechanic: 'strike', name: 'A', damage: 10 }],
    doom: {
      name: 'D',
      perStepMs: 1000,
      amount: 1,
      threshold: 100,
      clearAmount: 1,
      clearCharges: 1,
    },
    difficulties: {
      heroic: {
        add: { timeline: [{ atMs: 2000, mechanic: 'strike', name: 'B', damage: 10 }] },
      },
      mythic: {
        add: { stacks: { name: 'S', perStepMs: 4000, baseDamage: 5, factor: 2, maxStacks: 3 } },
      },
    },
  };

  it('Leben und Zeitlimit folgen den Faktoren der Datei', () => {
    const heroisch = deriveStage(grund, 1).raw;
    const faktoren = DIFFICULTY_STAGES[1];
    expect(heroisch['hp']).toBe(Math.floor((1000 * (faktoren?.hpPermille ?? 0)) / 1000));
    expect(heroisch['overpressureMs']).toBe(
      scaledOverpressure(60000, faktoren?.overpressurePermille ?? 0),
    );
  });

  it('das Zeitlimit landet auf dem 10-ms-Raster, abgerundet', () => {
    // 60001 * 0,9 = 54000,9 und 60010 * 0,905 = 54309,05: beide Male abwärts
    // auf das Raster. Mit festen Faktoren, damit der Fall nicht mit den
    // vorläufigen Zahlen der Datei wandert.
    expect(scaledOverpressure(60001, 900)).toBe(54000);
    expect(scaledOverpressure(60010, 905)).toBe(54300);
    expect(scaledOverpressure(60000, 1100)).toBe(66000);
  });

  it('kumulativ: Mythisch trägt die heroische Zutat, die Zeitleiste ist sortiert', () => {
    const mythisch = deriveStage(grund, 2);
    expect(mythisch.problems).toEqual([]);
    const zeiten = (mythisch.raw['timeline'] as { atMs: number }[]).map((e) => e.atMs);
    expect(zeiten).toEqual([2000, 5000]);
    expect(mythisch.raw['stacks']).not.toBeUndefined();
    expect(mythisch.raw['difficulties']).toBeUndefined();
  });

  it('ersetzt nie einen vorhandenen Baustein', () => {
    const ersetzt = deriveStage(
      { ...grund, difficulties: { ...grund.difficulties, heroic: { add: { doom: grund.doom } } } },
      1,
    );
    expect(ersetzt.problems.join(' ')).toContain('trägt der Boss schon');
  });

  it('setzt weder das umgekehrte Ziel noch die Heilung durch Fehler', () => {
    for (const verboten of ['runaway', 'bossHeal', 'phases']) {
      const versuch = deriveStage(
        {
          ...grund,
          difficulties: { ...grund.difficulties, heroic: { add: { [verboten]: {} } } },
        },
        1,
      );
      expect(versuch.problems.join(' '), verboten).toContain('kann keine Stufe hinzufügen');
    }
  });

  it('ein überschriebener Faktor gilt nur für seine Stufe', () => {
    const roh = {
      ...grund,
      difficulties: {
        heroic: { overpressurePermille: 1100, add: grund.difficulties.heroic.add },
        mythic: grund.difficulties.mythic,
      },
    };
    expect(deriveStage(roh, 1).raw['overpressureMs']).toBe(66000);
    expect(deriveStage(roh, 2).raw['overpressureMs']).toBe(
      scaledOverpressure(60000, DIFFICULTY_STAGES[2]?.overpressurePermille ?? 0),
    );
  });
});

describe('Die Gegenprobe — der Abweichungs-Wächter kann rot werden', () => {
  const normal = bossAtDifficulty('boss-1-1', 0);
  const heroisch = bossAtDifficulty('boss-1-1', 1);

  it('eine Stufe, die der darunter gleicht, fällt in allen drei Zusagen', () => {
    if (normal === undefined) throw new Error('boss-1-1 fehlt');
    expect(fehlendeAbweichung(normal, normal)).toEqual([
      'nicht mehr Leben (5200 -> 5200)',
      'Zeitlimit nicht kürzer (60000 -> 60000)',
      'keine zusätzliche Mechanik',
    ]);
  });

  it('mehr Leben allein reicht nicht', () => {
    if (normal === undefined || heroisch === undefined) throw new Error('boss-1-1 fehlt');
    const nurLeben = { ...normal, hp: heroisch.hp, overpressureMs: heroisch.overpressureMs };
    expect(fehlendeAbweichung(normal, nurLeben)).toEqual(['keine zusätzliche Mechanik']);
  });

  it('beim umgekehrten Ziel ist die kürzere Frist die falsche Richtung', () => {
    const lauf = bossAtDifficulty('boss-2-3', 0);
    if (lauf === undefined) throw new Error('boss-2-3 fehlt');
    const falsch = {
      ...lauf,
      hp: lauf.hp * 2,
      overpressureMs: lauf.overpressureMs - 6000,
      doom: {
        name: 'x',
        perStepMs: 1000,
        amount: 1,
        threshold: 100,
        clearAmount: 1,
        clearCharges: 1,
      },
    };
    expect(fehlendeAbweichung(lauf, falsch)).toEqual([
      'Frist nicht länger (60000 -> 54000)',
      'mehr Leben macht runaway leichter (10800)',
    ]);
  });
});

describe('„mehr Punkte“ — bei gleichem relativem Vorsprung zahlt die höhere Stufe mehr', () => {
  /**
   * Die Eigenschaft, nicht die Zahl. Der Punkte-Faktor muss den Verlust an
   * Restzeit überwiegen, den das kürzere Zeitlimit mit sich bringt — und die
   * Kurve rechnet im Quadrat (`pointsExponent` 2,0). Geprüft wird über die
   * echte Formel (`pointsBreakdown`), nicht über einen Nachbau.
   */
  it('für jeden Vorsprung von 5 % bis 90 % des Zeitlimits', () => {
    const boss = bossAtDifficulty('boss-1-1', 0);
    if (boss === undefined) throw new Error('boss-1-1 fehlt');
    const kontext = { bossLevel: 1, playerLevel: 1 };
    const befunde: string[] = [];
    for (let prozent = 5; prozent <= 90; prozent += 5) {
      let vorige = -1;
      for (let stufe = 0; stufe < DIFFICULTY_COUNT; stufe++) {
        const zeit = bossAtDifficulty('boss-1-1', stufe)?.overpressureMs ?? 0;
        const punkte = pointsBreakdown(
          { victory: true, remainingMs: Math.floor((zeit * prozent) / 100), heldPercent: null },
          { ...kontext, difficulty: stufe },
        ).awarded;
        if (punkte <= vorige) {
          befunde.push(`${String(prozent)} %: Stufe ${String(stufe)} zahlt ${punkte} <= ${vorige}`);
        }
        vorige = punkte;
      }
    }
    expect(befunde).toEqual([]);
  });

  it('der Faktor steht in der Zerlegung, Normal bleibt bei 1000', () => {
    const quelle = { victory: true, remainingMs: 20000, heldPercent: null };
    const normal = pointsBreakdown(quelle, { bossLevel: 1, playerLevel: 1 });
    expect(normal.difficultyPerMille).toBe(1000);
    const mythisch = pointsBreakdown(quelle, { bossLevel: 1, playerLevel: 1, difficulty: 2 });
    expect(mythisch.difficultyPerMille).toBe(pointsPermilleOf(2));
    expect(mythisch.awarded).toBe(Math.trunc((normal.awarded * pointsPermilleOf(2)) / 1000));
  });
});
