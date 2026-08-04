import { describe, expect, it } from 'vitest';
import {
  QUALITY_COMMON,
  QUALITY_COUNT,
  QUALITY_EPIC,
  QUALITY_LEGENDARY,
  QUALITY_RARE,
} from '../../src/gear/budget.js';
import {
  MATERIAL_PER_LEVEL,
  materialFor,
  upgradeCapOf,
  upgradeItem,
} from '../../src/loot/upgrade.js';
import { DIFFICULTY_MYTHIC, DIFFICULTY_NORMAL, INSTANCE_COUNT } from '../../src/loot/rates.js';

describe('Überflüssige Beute wird Material (AC1)', () => {
  it('gibt für höhere Qualität mehr — als Ungleichung, nicht gegen Zahlen', () => {
    // Die Zahlen sind vorläufig (#118); die Ordnung ist es nicht. Eine
    // Prüfung gegen feste Werte wäre beim nächsten Balance-Lauf rot, ohne
    // dass etwas kaputt wäre — und sie fienge den Nullfall nicht.
    expect(materialFor(QUALITY_RARE)).toBeGreaterThan(materialFor(QUALITY_COMMON));
    expect(materialFor(QUALITY_EPIC)).toBeGreaterThan(materialFor(QUALITY_RARE));
    expect(materialFor(QUALITY_LEGENDARY)).toBeGreaterThan(materialFor(QUALITY_EPIC));
    // Und der Vergleichsfall: Jede Qualität bringt überhaupt etwas.
    expect(materialFor(QUALITY_COMMON)).toBeGreaterThan(0);
  });

  it('gibt für eine unmögliche Qualität nichts', () => {
    expect(materialFor(QUALITY_COUNT)).toBe(0);
    expect(materialFor(-1)).toBe(0);
    expect(materialFor(1.5)).toBe(0);
  });
});

describe('Die Aufwertung endet am Inhalt (AC2)', () => {
  it('bleibt an der Obergrenze STEHEN, auch bei beliebig viel Material', () => {
    // **Die Auflage wörtlich:** Der Prüffall muss zeigen, dass die Aufwertung
    // STEHEN BLEIBT — nicht, dass sie irgendwann teuer wird. Eine bloß
    // steigende Kostenkurve machte den Grind zäh, aber nicht endlich.
    const grenze = upgradeCapOf(3, DIFFICULTY_NORMAL, 0);
    const massenhaft = upgradeItem(1, 1_000_000_000, 3, DIFFICULTY_NORMAL, 0);

    expect(massenhaft.level).toBe(grenze);
    expect(massenhaft.cappedByContent).toBe(true);
    // Und der eigentliche Punkt: NOCH mehr Material ändert nichts.
    const nochMehr = upgradeItem(1, 9_000_000_000, 3, DIFFICULTY_NORMAL, 0);
    expect(nochMehr.level).toBe(massenhaft.level);
  });

  it('verbraucht an der Obergrenze nichts', () => {
    // Wer dort aufwertet, verliert kein Material — er kommt nur nicht weiter.
    const grenze = upgradeCapOf(3, DIFFICULTY_NORMAL, 0);
    const amDeckel = upgradeItem(grenze, 10_000, 3, DIFFICULTY_NORMAL, 0);
    expect(amDeckel.level).toBe(grenze);
    expect(amDeckel.spent).toBe(0);
    expect(amDeckel.cappedByContent).toBe(true);
  });

  it('lässt ein übertroffenes Stück in Ruhe, statt es zu senken', () => {
    // Wer weiter ist als der Inhalt, behält seine Stufe. Eine Aufwertung darf
    // nie nach unten wirken.
    const grenze = upgradeCapOf(2, DIFFICULTY_NORMAL, 0);
    const darueber = upgradeItem(grenze + 20, 10_000, 2, DIFFICULTY_NORMAL, 0);
    expect(darueber.level).toBe(grenze + 20);
    expect(darueber.spent).toBe(0);
  });

  it('hört bei knappem Material am Material auf, nicht am Inhalt', () => {
    // Der Gegenfall zur Obergrenze — ohne ihn belegte der Rest nur, dass
    // irgendetwas anhält, nicht WAS.
    const knapp = upgradeItem(1, MATERIAL_PER_LEVEL * 2, INSTANCE_COUNT, DIFFICULTY_MYTHIC, 0);
    expect(knapp.level).toBe(3);
    expect(knapp.spent).toBe(MATERIAL_PER_LEVEL * 2);
    expect(knapp.cappedByContent).toBe(false);
  });

  it('wertet ohne Material gar nicht auf', () => {
    const ohne = upgradeItem(5, 0, INSTANCE_COUNT, DIFFICULTY_MYTHIC, 0);
    expect(ohne.level).toBe(5);
    expect(ohne.spent).toBe(0);
  });

  it('hebt die Grenze mit dem Inhalt — der Vergleichsfall', () => {
    // Ohne diesen Lauf belegte AC2 nur, dass es irgendeine Grenze gibt.
    const frueh = upgradeItem(1, 1_000_000, 1, DIFFICULTY_NORMAL, 0);
    const spaet = upgradeItem(1, 1_000_000, INSTANCE_COUNT, DIFFICULTY_MYTHIC, 0);
    expect(spaet.level).toBeGreaterThan(frueh.level);
    expect(frueh.cappedByContent).toBe(true);
    expect(spaet.cappedByContent).toBe(true);
  });

  it('verbraucht nur, was es auch einsetzt', () => {
    // Kein stiller Verlust: Was nicht in eine Stufe geht, bleibt liegen.
    const rest = upgradeItem(1, MATERIAL_PER_LEVEL * 3 + 3, INSTANCE_COUNT, DIFFICULTY_MYTHIC, 0);
    expect(rest.spent).toBe(MATERIAL_PER_LEVEL * 3);
  });
});

describe('Stückgebunden, nicht platzgebunden', () => {
  it('kennt gar keinen Platz', () => {
    // Die Regel in der Bauform: `upgradeItem` bekommt eine Stufe und Material.
    // Ein Platz kommt nicht vor, also kann die Aufwertung nicht an ihm hängen.
    const a = upgradeItem(4, 100, 5, DIFFICULTY_NORMAL, 0);
    const b = upgradeItem(4, 100, 5, DIFFICULTY_NORMAL, 0);
    expect(b).toEqual(a);
  });
});
