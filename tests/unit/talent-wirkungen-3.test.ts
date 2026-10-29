/**
 * SUBLIMATIONs drei Talent-Wege bewegen drei verschiedene Größen (Issue #419).
 *
 * # Diese Schule galt als nicht baubar — zu Unrecht
 *
 * Sie stand in meiner Übergabe als „elf Zauber, aber null hebbare Größen:
 * `sublime` trägt `freesNextGcd`, `condense` `doublesNextCast` — Schalter, keine
 * Zahlen". Das war an **zwei** der elf gemessen und auf alle elf verallgemeinert.
 * Die anderen sechs tragen Zahlen:
 *
 * | Größe | Zauber |
 * | --- | --- |
 * | `cooldownReductionMs` | vapour 1000 · wisp 500 · decant 2000 · ascend 10000 |
 * | `cooldownResetCharges` | reflux 1 · alembic 2 |
 * | `cooldownReductionBelowMs` | ascend 10000 |
 *
 * # Warum die Schwelle eine eigene Größe ist und keine Spielart des Abzugs
 *
 * `reduction` sagt, **wie viel** abgezogen wird; `threshold` sagt, **auf welche
 * Zauber** der Abzug überhaupt greift. Beide sitzen an derselben Mechanik und
 * sind trotzdem unabhängig — deshalb misst die Kreuzprobe hier nicht nur „die
 * fremde Zahl bleibt", sondern dass ein Rang auf `threshold` den Abzug
 * unverändert lässt, obwohl er dieselbe Verkürzung betrifft.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { FIRST_CRUCIBLE, parseSchool } from '../../src/core/data.js';

const VOLL = 5;

/**
 * Eine Prüfschule mit den drei Größen, wie es `sublimation.test.ts` und
 * `cooldown-reset.test.ts` auch tun.
 *
 * **Nicht die echten SUBLIMATION-Daten**, und das ist Absicht: `vapour: 1000`
 * ist ein Balance-Wert, der sich ändern darf. Eine Probe, die ihn fest erwartet,
 * ginge beim nächsten Ausbalancieren rot, ohne dass am Talent etwas kaputt ist.
 * Gemessen wird die Rechnung, nicht der Katalog.
 */
function pruefSchule(): ReturnType<typeof parseSchool> {
  return parseSchool({
    schema: 1,
    school: 'CALCINATION',
    resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
    spells: [
      {
        id: 'gross',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Gross',
        cooldownMs: 30_000,
        baseDamage: 10,
        cooldownReductionMs: 1000,
      },
      {
        id: 'klein',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Klein',
        cooldownMs: 20_000,
        baseDamage: 10,
        cooldownReductionMs: 500,
      },
      {
        id: 'eng',
        name: 'Eng',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        cooldownMs: 40_000,
        baseDamage: 10,
        cooldownReductionMs: 1000,
        cooldownReductionBelowMs: 10_000,
      },
      {
        id: 'reset',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Reset',
        cooldownMs: 60_000,
        baseDamage: 0,
        cooldownResetCharges: 1,
      },
      {
        id: 'stumpf',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Stumpf',
        cooldownMs: 5000,
        baseDamage: 10,
      },
    ],
  });
}

function kampf(talente: Record<string, number>): Combat {
  const schule = pruefSchule();
  expect(schule.problems, 'die Prüfschule hat Mängel').toEqual([]);
  const combat = new Combat({ school: schule.value, boss: FIRST_CRUCIBLE.value, seed: 0 });
  // **VOR `beginFight`** — ab Zeit 0 ist die Verteilung gesperrt (#425).
  combat.setTalents(talente);
  combat.beginFight(0);
  return combat;
}

/** Weg „reduction": Wie viel Abklingzeit ein Zauber abzieht. */
function abzug(talente: Record<string, number>, id = 'gross'): number {
  const combat = kampf(talente);
  return combat.reductionOfSpell(combat.indexOf(id));
}

/** Weg „threshold": Ab welcher Restzeit der Abzug greift. */
function schwelle(talente: Record<string, number>, id = 'eng'): number {
  const combat = kampf(talente);
  return combat.reductionScopeOfSpell(combat.indexOf(id)).belowMs;
}

/**
 * Weg „charges": Wie viele Rücksetz-Ladungen scharf stehen.
 *
 * **`armResetIfNeeded` gehört dazu und wird hier von Hand gerufen.** Es steht
 * bewusst nicht in `Combat.cast`, sondern in `Session.apply` (session.ts:849) —
 * eine Probe, die nur wirkt, misst deshalb immer null, ganz gleich wie viele
 * Ränge verteilt sind. Genau das ist mir hier passiert: vier rote Fälle, die
 * wie ein kaputtes Talent aussahen und ein fehlender Aufruf waren.
 */
function ladungen(talente: Record<string, number>, id = 'reset'): number {
  const combat = kampf(talente);
  const index = alsSpellIndex(combat.indexOf(id));
  const ergebnis = combat.cast(index, 0);
  expect(ergebnis.result, `${id} wurde abgewiesen`).toBe(CAST_OK);
  combat.armResetIfNeeded(index, 0);
  return combat.resetChargesLeft;
}

describe('SUBLIMATION — drei Wege, drei Größen', () => {
  it('die Messlatte ohne Talente', () => {
    expect(abzug({}), 'GROSS zieht nicht 1000 ms ab').toBe(1000);
    expect(schwelle({}), 'ENGs Schwelle liegt nicht bei 10000 ms').toBe(10_000);
    expect(ladungen({}), 'RESET stellt nicht 1 Ladung scharf').toBe(1);
  });

  describe('reduction — seit dem Umbau die UMWANDLUNG', () => {
    const TALENT = { 'sublimation-reduction': VOLL };

    it('lässt die FREMDEN Größen unberührt — auch den Abzug', () => {
      // **Der Abzug gehört seit #419 nicht mehr dazu.** Der Weg hob ihn und
      // verlor damit in jeder Rotation gegen `threshold`: Beide bewegten
      // dieselbe Größe, einer breiter.
      //
      // Jetzt zieht er in die Gegenrichtung — EFFLUX macht lange Abklingzeiten
      // WERTVOLL, während `threshold` sie verkürzt. Zwei Wege, die dieselbe
      // Größe gegenläufig behandeln, können nicht denselben Sieger haben.
      expect(abzug(TALENT), 'reduction wirkt noch auf den Abzug').toBe(1000);
      expect(schwelle(TALENT), 'reduction wirkt auf die Schwelle').toBe(10_000);
      expect(ladungen(TALENT), 'reduction wirkt auf die Ladungen').toBe(1);
    });
  });

  describe('charges — die Ladungen', () => {
    const TALENT = { 'sublimation-charges': VOLL };
    it('stellt mehr Ladungen scharf — ein ganzzahliger ZUSCHLAG', () => {
      // 1 je Rang, fünf Ränge: 1 + 5 = 6.
      expect(ladungen(TALENT)).toBe(6);
    });
    it('macht aus einem Nicht-Rücksetzer KEINEN Rücksetzer', () => {
      // **Der Fall, der den Zuschlag an die richtige Stelle zwingt.** Läge er
      // in der Prüfung `ladungen <= 0` statt dahinter, wäre jeder Zauber ein
      // Rücksetzer, sobald ein einziger Rang verteilt ist. STUMPF trägt keine
      // Ladungen und muss auch mit vollem Weg bei null bleiben.
      const combat = kampf(TALENT);
      const index = alsSpellIndex(combat.indexOf('stumpf'));
      expect(combat.cast(index, 0).result).toBe(CAST_OK);
      combat.armResetIfNeeded(index, 0);
      expect(combat.resetChargesLeft, 'STUMPF wurde zum Rücksetzer').toBe(0);
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(abzug(TALENT), 'charges wirkt auf den Abzug').toBe(1000);
      expect(schwelle(TALENT), 'charges wirkt auf die Schwelle').toBe(10_000);
    });
  });

  describe('threshold — die Schwelle', () => {
    const TALENT = { 'sublimation-threshold': VOLL };
    it('lässt den Abzug auf mehr Zauber greifen', () => {
      // 60 Promille je Rang, fünf Ränge: Faktor 1,3 auf 10000 ergibt 13000.
      expect(schwelle(TALENT)).toBe(13_000);
    });
    it('lässt die FREMDEN Größen unberührt — auch den Abzug an DERSELBEN Mechanik', () => {
      // **Das ist die schärfste der neun Kreuzproben.** Schwelle und Abzug
      // sitzen beide an der Verkürzung; wer sie in einer Rechnung zusammenzöge,
      // sähe hier 1300 statt 1000. Die anderen acht Kreuzproben trennen Größen
      // aus verschiedenen Mechaniken und sind damit leichter zu erfüllen.
      expect(abzug(TALENT), 'threshold wirkt auf den Abzug').toBe(1000);
      expect(ladungen(TALENT), 'threshold wirkt auf die Ladungen').toBe(1);
    });
  });
});
