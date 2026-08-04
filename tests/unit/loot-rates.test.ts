import { describe, expect, it } from 'vitest';
import {
  QUALITY_COMMON,
  QUALITY_COUNT,
  QUALITY_EPIC,
  QUALITY_LEGENDARY,
  QUALITY_RARE,
} from '../../src/gear/budget.js';
import type { RandomStream } from '../../src/core/rng.js';
import { lootStreamFor } from '../../src/loot/stream.js';
import {
  DIFFICULTY_HEROIC,
  DIFFICULTY_MYTHIC,
  DIFFICULTY_NORMAL,
  EPIC_FROM_INSTANCE,
  FIRST_ENDGAME_INSTANCE,
  INSTANCE_COUNT,
  WEIGHT_TOTAL,
  itemLevelOf,
  mayDropAt,
  qualityCapOf,
  qualityOfRoll,
  weightsFor,
} from '../../src/loot/rates.js';

/** Ein Beute-Wurf, wie ihn das Spiel führt: genau eine Ziehung. */
function wurf(strom: RandomStream, gewichte: Int32Array): number {
  return qualityOfRoll(gewichte, strom.next());
}

function gewichteFuer(instanz: number, schwierigkeit: number, endlos = 0): Int32Array {
  const out = new Int32Array(QUALITY_COUNT);
  weightsFor(instanz, schwierigkeit, endlos, out);
  return out;
}

/** Zählt, was über viele Würfe fällt. */
function verteilung(instanz: number, schwierigkeit: number, endlos: number, würfe: number) {
  const gewichte = gewichteFuer(instanz, schwierigkeit, endlos);
  const strom = lootStreamFor(0x5eed);
  const zähler = new Int32Array(QUALITY_COUNT);
  for (let i = 0; i < würfe; i++) {
    const q = wurf(strom, gewichte);
    zähler[q] = (zähler[q] ?? 0) + 1;
  }
  return zähler;
}

describe('Obergrenze durch Inhalt (AC1)', () => {
  it('lässt in einer frühen Instanz über 10.000 Durchläufe kein Epic und kein Legendary fallen', () => {
    const zähler = verteilung(1, DIFFICULTY_NORMAL, 0, 10_000);
    expect(zähler[QUALITY_EPIC]).toBe(0);
    expect(zähler[QUALITY_LEGENDARY]).toBe(0);
    // Und es fällt trotzdem etwas — sonst wäre die Null oben wertlos.
    expect(zähler[QUALITY_COMMON]! + zähler[QUALITY_RARE]!).toBe(10_000);
  });

  it('hält die Grenze auf JEDER frühen Instanz und Schwierigkeit', () => {
    // Nicht nur Instanz 1 auf Normal — sonst belegt der Fall nur einen Punkt.
    for (let instanz = 1; instanz < EPIC_FROM_INSTANCE; instanz++) {
      for (let stufe = 0; stufe <= DIFFICULTY_MYTHIC; stufe++) {
        const zähler = verteilung(instanz, stufe, 0, 2000);
        expect(zähler[QUALITY_EPIC], `Instanz ${instanz}, Stufe ${stufe}`).toBe(0);
        expect(zähler[QUALITY_LEGENDARY], `Instanz ${instanz}, Stufe ${stufe}`).toBe(0);
      }
    }
  });

  it('lässt Legendary erst im Endgame zu', () => {
    for (let instanz = 1; instanz < FIRST_ENDGAME_INSTANCE; instanz++) {
      expect(qualityCapOf(instanz)).toBeLessThan(QUALITY_LEGENDARY);
    }
    for (let instanz = FIRST_ENDGAME_INSTANCE; instanz <= INSTANCE_COUNT; instanz++) {
      expect(qualityCapOf(instanz)).toBe(QUALITY_LEGENDARY);
    }
  });

  it('verliert kein Gewicht, wenn die Obergrenze greift', () => {
    // Das gesperrte Gewicht fällt der höchsten erlaubten Qualität zu. Ginge
    // es verloren, summierten die Gewichte nicht mehr auf WEIGHT_TOTAL, und
    // ein Wurf am oberen Rand fiele ins Leere.
    for (let instanz = 1; instanz <= INSTANCE_COUNT; instanz++) {
      for (let stufe = 0; stufe <= DIFFICULTY_MYTHIC; stufe++) {
        const gewichte = gewichteFuer(instanz, stufe);
        const summe = gewichte.reduce((a, b) => a + b, 0);
        expect(summe, `Instanz ${instanz}, Stufe ${stufe}`).toBe(WEIGHT_TOTAL);
      }
    }
  });
});

describe('Richtwerte im Endgame (AC2)', () => {
  it('trifft 60 / 30 / 9 / 1 innerhalb der Schwankung', () => {
    const würfe = 200_000;
    const zähler = verteilung(INSTANCE_COUNT, DIFFICULTY_MYTHIC, 0, würfe);
    const anteil = (q: number) => (100 * zähler[q]!) / würfe;

    expect(anteil(QUALITY_COMMON)).toBeCloseTo(60, 0);
    expect(anteil(QUALITY_RARE)).toBeCloseTo(30, 0);
    expect(anteil(QUALITY_EPIC)).toBeCloseTo(9, 0);
    // Legendary ist der einzige Wert, bei dem die Schwankung ins Gewicht
    // fällt: 1 % von 200.000 sind 2000 Stück, Streuung rund 45 — die
    // Schranken unten liegen bei 1800 und 2200 und damit weit außerhalb.
    expect(anteil(QUALITY_LEGENDARY)).toBeGreaterThan(0.9);
    expect(anteil(QUALITY_LEGENDARY)).toBeLessThan(1.1);
  });
});

describe('Zwei getrennte Achsen (AC3)', () => {
  it('macht ein episches Stück aus früher Instanz schwächer als ein seltenes aus dem Endgame', () => {
    // 13.4 wörtlich. Die Gegenstandsstufe hängt am Boss, nicht an der
    // gefallenen Qualität — sonst entwertete sich alter Inhalt nicht von
    // selbst, und man würde frühe Instanzen auf Legendary farmen.
    const frühEpic = itemLevelOf(EPIC_FROM_INSTANCE, DIFFICULTY_NORMAL, 0);
    const endgameRare = itemLevelOf(INSTANCE_COUNT, DIFFICULTY_MYTHIC, 0);
    expect(frühEpic).toBeLessThan(endgameRare);
  });

  it('lässt die Gegenstandsstufe NICHT von der Qualität abhängen', () => {
    // Der eigentliche Nachweis der Trennung: Dieselbe Stufe für jeden Fund
    // desselben Bosses, unabhängig davon, was gefallen ist.
    const stufe = itemLevelOf(7, DIFFICULTY_HEROIC, 0);
    expect(itemLevelOf(7, DIFFICULTY_HEROIC, 0)).toBe(stufe);
    // Und sie steigt monoton mit dem Inhalt, über alle drei Achsen.
    expect(itemLevelOf(8, DIFFICULTY_NORMAL, 0)).toBeGreaterThan(
      itemLevelOf(7, DIFFICULTY_NORMAL, 0),
    );
    expect(itemLevelOf(7, DIFFICULTY_MYTHIC, 0)).toBeGreaterThan(
      itemLevelOf(7, DIFFICULTY_NORMAL, 0),
    );
    expect(itemLevelOf(14, DIFFICULTY_MYTHIC, 5)).toBeGreaterThan(
      itemLevelOf(14, DIFFICULTY_MYTHIC, 0),
    );
  });
});

describe('Boss-Bindung (AC4)', () => {
  it('lässt ein gebundenes Stück bei keinem anderen Boss fallen', () => {
    expect(mayDropAt('first-crucible', 'first-crucible')).toBe(true);
    expect(mayDropAt('first-crucible', 'test-onslaught')).toBe(false);
    expect(mayDropAt('first-crucible', '')).toBe(false);
  });

  it('lässt ungebundene Stücke überall fallen', () => {
    for (const boss of ['first-crucible', 'test-onslaught', 'test-vessel']) {
      expect(mayDropAt('', boss)).toBe(true);
    }
  });
});

describe('Kein Pech-Schutz (AC5)', () => {
  it('hat gar keinen Zustand, in dem ein Zähler stecken könnte', () => {
    // Die Regel in der Bauform statt in einer Prüfung: Derselbe Wurf ergibt
    // dasselbe Ergebnis, egal was vorher geschah.
    const gewichte = gewichteFuer(INSTANCE_COUNT, DIFFICULTY_MYTHIC);
    const zuerst = qualityOfRoll(gewichte, 0.995);
    for (let i = 0; i < 1000; i++) qualityOfRoll(gewichte, 0.5);
    expect(qualityOfRoll(gewichte, 0.995)).toBe(zuerst);
  });

  it('hat nach 50 Fehlschlägen dieselbe Wahrscheinlichkeit wie am Anfang', () => {
    // **Die Auflage zu diesem Issue.** „Kein Pech-Schutz" ist trivial
    // erfüllt, wenn niemand nachsieht — ein Prüffall, der nur zeigt, dass
    // überhaupt etwas fällt, belegt das Gegenteil nicht.
    //
    // Gemessen wird deshalb der Wurf UNMITTELBAR NACH einer Pechsträhne von
    // 50 und verglichen mit der Rate über alles. Bei einem Pech-Schutz wäre
    // der Wert nach der Strähne höher.
    const gewichte = gewichteFuer(INSTANCE_COUNT, DIFFICULTY_MYTHIC);
    const strom = lootStreamFor(0xbeef);

    let strähne = 0;
    let nachSträhne = 0;
    let legendärNachSträhne = 0;
    let gesamt = 0;
    let legendärGesamt = 0;

    for (let i = 0; i < 500_000; i++) {
      const messenJetzt = strähne >= 50;
      const q = wurf(strom, gewichte);
      gesamt++;
      if (q === QUALITY_LEGENDARY) legendärGesamt++;
      if (messenJetzt) {
        nachSträhne++;
        if (q === QUALITY_LEGENDARY) legendärNachSträhne++;
      }
      strähne = q === QUALITY_LEGENDARY ? 0 : strähne + 1;
    }

    expect(nachSträhne).toBeGreaterThan(10_000);
    const rateGesamt = legendärGesamt / gesamt;
    const rateNachSträhne = legendärNachSträhne / nachSträhne;

    // Beide müssen bei 1 % liegen. Die Schranke ist großzügig gewählt und
    // trotzdem eng genug: Ein Pech-Schutz, der nach 50 Fehlschlägen auch nur
    // die doppelte Chance gäbe, läge weit außerhalb.
    expect(rateGesamt).toBeGreaterThan(0.008);
    expect(rateGesamt).toBeLessThan(0.012);
    expect(rateNachSträhne).toBeGreaterThan(0.008);
    expect(rateNachSträhne).toBeLessThan(0.012);
  });

  it('würde einen Pech-Schutz erkennen — die Gegenprobe', () => {
    // Ohne diesen Fall wäre der vorige nur eine Behauptung: Er verglich zwei
    // Zahlen, die beide bei 1 % liegen, ohne je gezeigt zu haben, dass er
    // eine Abweichung überhaupt bemerkt.
    //
    // Hier läuft dieselbe Messung gegen eine ABSICHTLICH mit Pech-Schutz
    // gebaute Ziehung. Sie muss auffliegen.
    const gewichte = gewichteFuer(INSTANCE_COUNT, DIFFICULTY_MYTHIC);
    const strom = lootStreamFor(0xbeef);

    let strähne = 0;
    let nachSträhne = 0;
    let legendärNachSträhne = 0;

    for (let i = 0; i < 500_000; i++) {
      const messenJetzt = strähne >= 50;
      const roll = strom.next();
      // **Der Pech-Schutz**: Nach 50 Fehlschlägen wird der Wurf geschönt.
      const q = strähne >= 50 ? QUALITY_LEGENDARY : qualityOfRoll(gewichte, roll);
      if (messenJetzt) {
        nachSträhne++;
        if (q === QUALITY_LEGENDARY) legendärNachSträhne++;
      }
      strähne = q === QUALITY_LEGENDARY ? 0 : strähne + 1;
    }

    const rateNachSträhne = legendärNachSträhne / nachSträhne;
    // Die Schranke aus dem Prüffall darüber. Der Pech-Schutz reißt sie weit.
    expect(rateNachSträhne).toBeGreaterThan(0.012);
  });
});
