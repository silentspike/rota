import { describe, expect, it } from 'vitest';
import {
  BUDGET_PER_PERMILLE,
  SET_VALUE_TIERS,
  SOURCE_AFFIXES,
  SOURCE_COUNT,
  SOURCE_LEGENDARY,
  SOURCE_NAMES,
  SOURCE_SETS,
  SOURCE_SPELLS,
  SOURCE_TALENTS,
  STAT_CATALYSIS,
  STAT_COUNT,
  STAT_NAMES,
  STAT_PURITY,
  STAT_YIELD,
  affixPermille,
  buildTotal,
  isPermanent,
  maxFromAffixes,
  maxOf,
  mixedBuild,
  permanentMax,
  pureBuild,
  totalMax,
} from '../../src/gear/sources.js';
import { MAX_ITEM_LEVEL, QUALITY_COMMON, QUALITY_EPIC, budgetOf } from '../../src/gear/budget.js';
import { SLOT_COUNT } from '../../src/gear/slots.js';
import { SET_TIERS } from '../../src/gear/sets.js';

/**
 * Die fünf Quellen der Sonderwerte (Issue #178).
 *
 * **Die Zahlen sind vorläufig, ihre Struktur nicht.** Geprüft wird deshalb
 * durchweg gegen Ungleichungen und Ableitungen — eine Prüfung gegen feste Werte
 * wäre beim nächsten Balance-Lauf rot, ohne dass etwas kaputt wäre.
 *
 * Die einzigen ausgeschriebenen Zahlen stehen dort, wo das Design sie nennt:
 * die 3 % aus 6.7 und die 100 % bei PURITY.
 */

describe('Jede der fünf Quellen trägt ihren Anteil (AC1)', () => {
  it('kennt genau fünf, jede mit Namen', () => {
    expect(SOURCE_NAMES).toHaveLength(SOURCE_COUNT);
    expect(SOURCE_COUNT).toBe(5);
    // Die Kennnummern sind Teil der Schnittstelle — sie bestimmen, in welcher
    // Reihenfolge der Simulator dieselben Werte liest.
    expect(SOURCE_NAMES[SOURCE_AFFIXES]).toBe('affixes');
    expect(SOURCE_NAMES[SOURCE_TALENTS]).toBe('talents');
    expect(SOURCE_NAMES[SOURCE_SPELLS]).toBe('spells');
    expect(SOURCE_NAMES[SOURCE_LEGENDARY]).toBe('legendary');
    expect(SOURCE_NAMES[SOURCE_SETS]).toBe('sets');
    expect(STAT_NAMES).toHaveLength(STAT_COUNT);
  });

  it('lässt KEINE Quelle bei allen drei Werten leer — einzeln geprüft', () => {
    // **Der Kern von AC1.** Eine gemeinsame Prüfung über die Summe bliebe grün,
    // wenn eine Quelle gar nichts beitrüge — genau der Fall aus #100, wo STALL
    // durch die Prüfsumme lief, ohne eigenen Nachweis.
    for (let source = 0; source < SOURCE_COUNT; source++) {
      let summe = 0;
      for (let stat = 0; stat < STAT_COUNT; stat++) summe += maxOf(source, stat);
      expect(summe, `Quelle ${SOURCE_NAMES[source]} trägt nirgends etwas bei`).toBeGreaterThan(0);
    }
  });

  it('nennt für jede Quelle einzeln, worauf sie wirkt', () => {
    // Nicht jede Quelle wirkt auf jeden Wert, und das ist Absicht. Der
    // Overdrive ist ein **Tempo**-Schub (7.4) — auf den Krit wirkt er nicht.
    expect(maxOf(SOURCE_SPELLS, STAT_CATALYSIS)).toBeGreaterThan(0);
    expect(maxOf(SOURCE_SPELLS, STAT_PURITY)).toBe(0);
    expect(maxOf(SOURCE_SPELLS, STAT_YIELD)).toBe(0);

    // Die vier anderen wirken auf alle drei.
    for (const source of [SOURCE_AFFIXES, SOURCE_TALENTS, SOURCE_LEGENDARY, SOURCE_SETS]) {
      for (let stat = 0; stat < STAT_COUNT; stat++) {
        expect(maxOf(source, stat), `${SOURCE_NAMES[source]} auf ${STAT_NAMES[stat]}`).toBeGreaterThan(0);
      }
    }
  });

  it('gibt für unbekannte Quellen und Werte nichts, statt etwas zu erfinden', () => {
    expect(maxOf(SOURCE_COUNT, STAT_CATALYSIS)).toBe(0);
    expect(maxOf(-1, STAT_CATALYSIS)).toBe(0);
    expect(maxOf(SOURCE_TALENTS, STAT_COUNT)).toBe(0);
    expect(maxOf(SOURCE_TALENTS, -1)).toBe(0);
  });
});

describe('Der Affix-Beitrag FOLGT aus dem Budget', () => {
  it('trifft die Größenordnung, die 6.7 selbst nennt: +3 %', () => {
    // > Ein Fund trägt **„+3 % Tempo"**, nicht „+120 Tempo".
    //
    // Das ist die einzige Zahl, die das Design für einen Affix nennt — und der
    // Umrechner ist so gewählt, dass ein Epic-Affix auf der höchsten Stufe sie
    // genau trifft.
    expect(affixPermille(MAX_ITEM_LEVEL, QUALITY_EPIC)).toBe(30);
  });

  it('wandert mit dem Budget mit, statt daneben gepflegt zu werden', () => {
    // **Die eigentliche Zusicherung.** Ändert jemand das Budget-Wachstum, muss
    // der Affix-Wert mitwandern — sonst ist es eine Handabschrift (#134, #138,
    // #159, #173).
    for (const quality of [QUALITY_COMMON, QUALITY_EPIC]) {
      for (const level of [1, 25, 50, 75, MAX_ITEM_LEVEL]) {
        const budget = budgetOf(level, quality);
        const promille = affixPermille(level, quality);
        // Kein festes Verhältnis geprüft, sondern die Herkunft: Der Wert liegt
        // zwischen dem, was das Budget hergibt, und einer Stufe weniger.
        expect(promille * BUDGET_PER_PERMILLE).toBeLessThanOrEqual(budget);
      }
    }
  });

  it('steigt mit der Gegenstandsstufe — als Ungleichung', () => {
    let vorher = affixPermille(1, QUALITY_EPIC);
    for (let level = 2; level <= MAX_ITEM_LEVEL; level++) {
      const jetzt = affixPermille(level, QUALITY_EPIC);
      expect(jetzt, `Stufe ${level}`).toBeGreaterThanOrEqual(vorher);
      vorher = jetzt;
    }
    // Und über den ganzen Bereich wächst er wirklich, statt nur nie zu fallen.
    expect(affixPermille(MAX_ITEM_LEVEL, QUALITY_EPIC)).toBeGreaterThan(
      affixPermille(1, QUALITY_EPIC),
    );
  });

  it('rechnet ein Epic besser als ein Common — trotz kleinerem Einzel-Affix', () => {
    // Ein Common trägt EINEN Affix, der das ganze Budget bekommt; ein Epic
    // teilt seins auf drei. Der einzelne Common-Affix ist deshalb größer — die
    // drei des Epic sind zusammen mehr. Das folgt aus 13.3 und ist kein Zufall.
    const common = affixPermille(MAX_ITEM_LEVEL, QUALITY_COMMON);
    const epicEinzeln = affixPermille(MAX_ITEM_LEVEL, QUALITY_EPIC);
    expect(common).toBeGreaterThan(epicEinzeln);
    expect(epicEinzeln * 3).toBeGreaterThan(common);
  });

  it('rechnet alle zehn Plätze zusammen, nicht einen', () => {
    expect(maxFromAffixes()).toBe(affixPermille(MAX_ITEM_LEVEL, QUALITY_EPIC) * 3 * SLOT_COUNT);
    expect(SLOT_COUNT).toBe(10);
  });
});

describe('Die Summe überschreitet, was ein Deckel sein könnte (AC2)', () => {
  it('beziffert sie — mit Zahl, nicht als Aussage', () => {
    // **Wofür die Zahl da ist:** Sie ist die Obergrenze, UNTER der ein
    // gemessener Deckel liegen muss. Liegt er darüber, ist er unerreichbar und
    // wirkungslos; liegt er genau darauf, ist „alles auf Tempo" wieder die
    // einzig richtige Antwort statt einer Entscheidung.
    const dauerhaft = permanentMax(STAT_CATALYSIS);
    const gesamt = totalMax(STAT_CATALYSIS);

    // Der Bezugspunkt, den 6.7 bis zum 2026-08-04 als Deckel führte. Er ist
    // gestrichen — als Größenordnung, die überschritten werden muss, taugt er
    // weiterhin, und der Auftraggeber hat ihn ausdrücklich genannt.
    const FRUEHERER_DECKEL = 500;
    expect(dauerhaft).toBeGreaterThan(FRUEHERER_DECKEL);
    expect(gesamt).toBeGreaterThan(dauerhaft);
  });

  it('trennt dauerhaft von kurzzeitig — sonst wäre die Zahl nicht deutbar', () => {
    // 6.2: CATALYSIS „wächst **dauerhaft** über Talente und Ausrüstung,
    // **kurzzeitig** über Overdrive". Ein Deckel greift im Spitzenmoment anders
    // als im Dauerzustand.
    expect(isPermanent(SOURCE_SPELLS)).toBe(false);
    for (const source of [SOURCE_AFFIXES, SOURCE_TALENTS, SOURCE_LEGENDARY, SOURCE_SETS]) {
      expect(isPermanent(source), SOURCE_NAMES[source]).toBe(true);
    }
    // Und der Unterschied ist wirklich einer: Bei Tempo liegt die Gesamtsumme
    // über der dauerhaften, bei den Kritwerten fallen beide zusammen.
    expect(totalMax(STAT_CATALYSIS)).toBeGreaterThan(permanentMax(STAT_CATALYSIS));
    expect(totalMax(STAT_PURITY)).toBe(permanentMax(STAT_PURITY));
    expect(totalMax(STAT_YIELD)).toBe(permanentMax(STAT_YIELD));
  });

  it('lässt den Deckel NICHT von einer einzigen Quelle erreichen', () => {
    // „Es darf das Cap nur nicht zu leicht erreichbar sein." Wer nur Affixe
    // wählt, kommt nicht über den früheren Deckel — es braucht mehrere Quellen.
    // Ohne diese Zeile wäre die Prüfung darüber auch mit einer einzigen,
    // maßlosen Quelle erfüllt.
    for (let source = 0; source < SOURCE_COUNT; source++) {
      expect(
        maxOf(source, STAT_CATALYSIS),
        `${SOURCE_NAMES[source]} allein reicht bis zum Deckel`,
      ).toBeLessThan(1000);
    }
  });

  it('hält den PURITY-Deckel erreichbar — die Zusicherung aus 6.7', () => {
    // > Am Deckel ist Krit kein Zufall mehr. … **Das ist ein erreichbares
    // > Ziel**, auf das sich ein ganzer Aufbau ausrichten lässt.
    //
    // **Diese Prüfung hat die Wahl des Umrechners bestimmt.** Mit 500 Punkten
    // je Promille käme ein voller Aufbau auf 680 Promille und bliebe unter dem
    // Deckel — das Ziel wäre unerreichbar und der Satz im Design falsch.
    // Gemessen am 2026-08-04, deshalb steht sie hier: Wer an den Talent- oder
    // Satz-Zahlen dreht, soll es merken.
    expect(permanentMax(STAT_PURITY)).toBeGreaterThanOrEqual(1000);

    // Und er ist nicht mit einer Hand voll Gegenstände erreichbar: Die Affixe
    // allein reichen nicht.
    expect(maxOf(SOURCE_AFFIXES, STAT_PURITY)).toBeLessThan(1000);
  });
});

describe('Rein gegen gemischt — der Vergleich läuft (AC4)', () => {
  it('setzt beide Aufbauten auf dasselbe Budget', () => {
    // **Sonst verglichen man nicht zwei Aufbauten, sondern zwei Budgets.**
    // Dieselbe Überlegung wie beim Satz-gegen-Affix-Vergleich aus #166.
    const rein = pureBuild(STAT_CATALYSIS);
    const gemischt = mixedBuild(STAT_CATALYSIS);
    // Auf drei Werte verteilt bleibt ein Rest, wenn die Summe nicht durch drei
    // teilbar ist — er verfällt, statt einen Aufbau zu bevorzugen.
    expect(buildTotal(rein) - buildTotal(gemischt)).toBeLessThan(STAT_COUNT);
    expect(buildTotal(gemischt)).toBeLessThanOrEqual(buildTotal(rein));

    // **Und für JEDEN Bezugswert, nicht nur für Tempo.** Die erste Fassung von
    // `mixedBuild` nahm das größte der drei Maxima und gab dem gemischten
    // Aufbau damit 1359 Promille gegen 1130 — er hätte gewonnen, ohne dass die
    // Verteilung etwas damit zu tun gehabt hätte.
    for (const stat of [STAT_CATALYSIS, STAT_PURITY, STAT_YIELD]) {
      const r = buildTotal(pureBuild(stat));
      const m = buildTotal(mixedBuild(stat));
      expect(m, `Bezugswert ${STAT_NAMES[stat]}`).toBeLessThanOrEqual(r);
      expect(r - m, `Bezugswert ${STAT_NAMES[stat]}`).toBeLessThan(STAT_COUNT);
    }
  });

  it('setzt beim reinen Aufbau wirklich alles auf einen Wert', () => {
    for (const stat of [STAT_CATALYSIS, STAT_PURITY, STAT_YIELD]) {
      const b = pureBuild(stat);
      const werte = [b.catalysis, b.purity, b.yieldBonus];
      expect(werte[stat]).toBeGreaterThan(0);
      for (let i = 0; i < STAT_COUNT; i++) {
        if (i !== stat) expect(werte[i], `${STAT_NAMES[i]} bei reinem ${STAT_NAMES[stat]}`).toBe(0);
      }
    }
  });

  it('rechnet den reinen Aufbau OHNE Overdrive', () => {
    // Ein Dauerwert, der einen befristeten Spitzenwert enthält, wäre eine
    // falsche Zahl — der Vergleich liefe dann über einen Zustand, den es die
    // meiste Zeit gar nicht gibt.
    expect(pureBuild(STAT_CATALYSIS).catalysis).toBe(permanentMax(STAT_CATALYSIS));
    expect(pureBuild(STAT_CATALYSIS).catalysis).toBeLessThan(totalMax(STAT_CATALYSIS));
  });

  it('verteilt beim gemischten Aufbau gleichmäßig', () => {
    const g = mixedBuild(STAT_CATALYSIS);
    expect(g.purity).toBe(g.catalysis);
    expect(g.yieldBonus).toBe(g.catalysis);
    expect(g.catalysis).toBeGreaterThan(0);
  });
});

describe('Was aus dem Design kommt, wird nicht abgetippt', () => {
  it('leitet die Zahl der Werte-Stufen aus den Satz-Stufen ab', () => {
    // 13.5: drei Stufen (zwei, vier, sechs Teile), Werte auf den unteren,
    // Rotations-Effekte auf der obersten.
    expect(SET_VALUE_TIERS).toBe(SET_TIERS.length - 1);
    expect(SET_TIERS).toEqual([2, 4, 6]);
  });

  it('rechnet mit zehn Plätzen, weil 13.2 zehn nennt', () => {
    expect(maxFromAffixes()).toBe(maxFromAffixes(MAX_ITEM_LEVEL));
    expect(SLOT_COUNT).toBe(10);
  });
});
