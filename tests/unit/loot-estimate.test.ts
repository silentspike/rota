/**
 * Die Eingabe für den Gegenstands-Vergleich (Issue #455).
 *
 * # AC2 IST DER GANZE PUNKT UND WIRD LEICHT ZU WEICH GEBAUT
 *
 * „Mehr PURITY wird als besser angezeigt" ist trivial und wäre auch mit einer
 * festen Formel wahr. Was belegt, dass wirklich **gerechnet** wird, ist die
 * Gegenprobe: **dasselbe Stück, andere Zusammenstellung, anderes Urteil.**
 *
 * Diese Datei nimmt dafür zwei Fälle, in denen ein Wert nicht bloß weniger
 * bringt, sondern **exakt nichts** — beide sind Design-Regeln und keine
 * Erfindung:
 *
 * | Fall | Regel | Quelle |
 * | --- | --- | --- |
 * | PURITY bei 100 % Krit | `critChance` deckelt bei 1 | 6.7, `worn.ts` |
 * | YIELD bei 0 % Krit | ein Krit, der nie eintritt, wiegt nichts | Erwartungswert |
 *
 * Der zweite ist der aussagekräftigere: **derselbe** Wert, **dieselbe** Zahl,
 * und die Antwort hängt allein daran, was sonst getragen wird.
 */
import { describe, expect, it } from 'vitest';
import { compareBagItem, estimateDamagePerHit, estimatedGain } from '../../src/loot/estimate.js';
import {
  compareItems,
  COMPARE_BETTER,
  COMPARE_EQUAL,
  COMPARE_WORSE,
} from '../../src/loot/compare.js';
import { NOTHING_WORN, type WornValues } from '../../src/gear/worn.js';

function worn(werte: Partial<WornValues>): WornValues {
  return { ...NOTHING_WORN, ...werte };
}

describe('AC1 — die Schätzung kommt aus der echten Wertformel', () => {
  it('ohne Ausrüstung ist sie exakt 1,0 — keine Zahl bewegt sich', () => {
    // Dieselbe Zusicherung wie bei `NOTHING_WORN` im Kampf. Nicht „ungefähr":
    // 1,0 ist in IEEE754 die exakte Eins.
    expect(estimateDamagePerHit(NOTHING_WORN)).toBe(1);
  });

  it('OHNE KRIT ist sie das Produkt der beiden Topf-Faktoren — auf das Bit', () => {
    // **Der Nachweis, dass hier keine zweite Quelle rechnet.** Bei `purity: 0`
    // tritt nie ein Krit ein, also bleibt genau das übrig, was `Combat` als
    // `essence`- und `gearEffects`-Faktor führt. Wäre die Schätzung eine
    // eigene Näherung, träfe sie diesen Wert nicht exakt.
    const w = worn({ essence: 500, gearEffects: 300, purity: 0 });
    expect(estimateDamagePerHit(w)).toBe(1.5 * 1.3);
  });

  it('BEI SICHEREM KRIT kommt genau der Kritfaktor dazu', () => {
    // Die andere Kante: `purity >= 1000` heisst, jeder Treffer kritet. Dann
    // ist der Erwartungswert der Kritfall selbst — Basis 150 % plus YIELD.
    const w = worn({ essence: 500, gearEffects: 300, purity: 1000, yieldBonus: 500 });
    expect(estimateDamagePerHit(w)).toBe(1.5 * 1.3 * 2.0);
  });

  it('und eine Änderung an den Werten verschiebt sie messbar', () => {
    const ohne = estimateDamagePerHit(worn({ essence: 0 }));
    const mit = estimateDamagePerHit(worn({ essence: 200 }));
    expect(mit).toBeGreaterThan(ohne);
    expect(compareItems(mit, ohne).verdict).toBe(COMPARE_BETTER);
  });
});

describe('AC2 — dasselbe Stück, andere Zusammenstellung, anderes Urteil', () => {
  /** Ein Fund, der 200 Promille PURITY mitbringt. */
  const FUND_PURITY = 200;

  it('mehr PURITY ist besser, solange Krit nicht gedeckelt ist', () => {
    const getragen = worn({ purity: 100, yieldBonus: 500 });
    const mitFund = worn({ purity: 100 + FUND_PURITY, yieldBonus: 500 });
    expect(
      compareItems(estimateDamagePerHit(mitFund), estimateDamagePerHit(getragen)).verdict,
    ).toBe(COMPARE_BETTER);
  });

  it('GEGENPROBE: derselbe Fund bringt bei 100 % Krit EXAKT nichts', () => {
    // `critChance` deckelt bei 1 (GAME-DESIGN 6.7). Wer schon immer kritet,
    // gewinnt durch mehr Kritchance nichts — und zwar nicht „wenig", sondern
    // null. Eine feste Formel „PURITY ist gut" käme hier zum falschen Urteil.
    const getragen = worn({ purity: 1000, yieldBonus: 500 });
    const mitFund = worn({ purity: 1000 + FUND_PURITY, yieldBonus: 500 });
    expect(estimateDamagePerHit(mitFund)).toBe(estimateDamagePerHit(getragen));
    expect(
      compareItems(estimateDamagePerHit(mitFund), estimateDamagePerHit(getragen)).verdict,
    ).toBe(COMPARE_EQUAL);
  });

  it('und dieselbe Kreuzprobe für YIELD — er wiegt nur, wenn überhaupt kritet wird', () => {
    // Der aussagekräftigere Fall: **derselbe Wert, dieselbe Zahl**, und das
    // Urteil hängt allein an dem, was sonst getragen wird.
    const fund = 400;
    const mitKrit = estimatedGain(
      worn({ purity: 500, yieldBonus: fund }),
      worn({ purity: 500, yieldBonus: 0 }),
    );
    const ohneKrit = estimatedGain(
      worn({ purity: 0, yieldBonus: fund }),
      worn({ purity: 0, yieldBonus: 0 }),
    );
    expect(mitKrit, 'bei 50 % Krit muss YIELD etwas bringen').toBeGreaterThan(0);
    expect(ohneKrit, 'ohne Krit wiegt Kritschaden nichts').toBe(0);
  });
});

describe('AC3 — die Näherung bleibt als Näherung erkennbar', () => {
  it('auch wenn die Schätzung aus der echten Formel kommt', () => {
    // Die Kennzeichnung hängt NICHT daran, wie genau gerechnet wurde. Sie
    // steht, weil CATALYSIS fehlt — siehe die Tabelle in `estimate.ts`.
    const c = compareItems(
      estimateDamagePerHit(worn({ essence: 300 })),
      estimateDamagePerHit(NOTHING_WORN),
    );
    expect(c.text.startsWith('~')).toBe(true);
    expect(c.caveat.length).toBeGreaterThan(0);
  });
});

describe('Die Grenze der Näherung — gemessen, nicht behauptet', () => {
  it('CATALYSIS bewegt die Schätzung NICHT, obwohl es im Kampf wirkt', () => {
    // **Das ist kein Fehler, sondern die dokumentierte Grenze** — und sie
    // gehört gemessen, damit sie nicht unbemerkt verschwindet oder wächst.
    // Bekommt die Schätzung eines Tages eine Rotations-Wirkung, wird dieser
    // Fall rot und verlangt, dass die Tabelle in `estimate.ts` nachgezogen
    // wird.
    const ohne = estimateDamagePerHit(worn({ catalysis: 0 }));
    const mit = estimateDamagePerHit(worn({ catalysis: 900 }));
    expect(mit).toBe(ohne);
  });
});

describe('Der Tausch auf der echten Datenstruktur', () => {
  // **Diese Einschränkung ist seit Issue #480 weggefallen.** Hier stand, ein
  // Fall „besserer Fund verdrängt schlechteres Stück" lasse sich nicht bauen,
  // weil der Katalog nur einen Eintrag trägt UND `upgradeSteps` nicht in
  // `wornFrom` einfliesst — zwei Stücke derselben Kennung seien damit
  // zwangsläufig gleichwertig.
  //
  // Beide Halbsätze sind inzwischen überholt, jeder auf eigene Weise. Der
  // Katalog trägt seit #741 einundzwanzig Einträge statt einem — allerdings
  // alle auf Stufe 40 in Qualität 2, die Vorlagen unterscheiden sich also nach
  // wie vor nicht in ihrer STÄRKE. Und `upgradeSteps` fliesst jetzt sehr wohl
  // in `wornFrom` ein, zusammen mit der gewürfelten Fundstufe.
  //
  // Zwei Stücke derselben Kennung sind damit verschieden stark, und die drei
  // Fälle unten waren vorher nicht baubar.
  //
  // Der Rest der Begründung steht: Die Rechnung selbst ist oben auf
  // `WornValues` geprüft, mitsamt Mutations-Gegenprobe. Hier geht es um den
  // TAUSCH.
  const RELIC = 'test-relic';

  it('ein Fund auf einen LEEREN Platz ist besser', () => {
    const c = compareBagItem([{ itemId: RELIC, upgradeSteps: 0, equippedSlot: null }], 0);
    expect(c?.verdict).toBe(COMPARE_BETTER);
  });

  it('und verdrängt ein gleichwertiges Stück zu einem Gleichstand', () => {
    // Der Beleg, dass wirklich getauscht wird: Läge der Fund einfach obendrauf,
    // käme hier BETTER heraus — zwei Stücke tragen mehr als eines.
    const c = compareBagItem(
      [
        { itemId: RELIC, upgradeSteps: 0, equippedSlot: 0 },
        { itemId: RELIC, upgradeSteps: 0, equippedSlot: null },
      ],
      1,
    );
    expect(c?.verdict).toBe(COMPARE_EQUAL);
  });

  it('ein angelegtes Stück ist kein Fund — es gibt kein Urteil', () => {
    const c = compareBagItem([{ itemId: RELIC, upgradeSteps: 0, equippedSlot: 0 }], 0);
    expect(c).toBeUndefined();
  });

  // Die drei Fälle, die der Kommentar oben bis #480 für unbaubar erklärt hat.
  it('ein höher aufgewerteter Fund ist BESSER als das getragene Stück', () => {
    const c = compareBagItem(
      [
        { itemId: RELIC, upgradeSteps: 0, equippedSlot: 0 },
        { itemId: RELIC, upgradeSteps: 20, equippedSlot: null },
      ],
      1,
    );
    expect(c?.verdict).toBe(COMPARE_BETTER);
  });

  it('und ein schwächerer Fund ist SCHLECHTER — nicht bloss „nicht besser"', () => {
    // Der Gegenpol. Ohne ihn wäre auch eine Rechnung grün, die jeden Fund mit
    // abweichender Stufe für besser hält.
    const c = compareBagItem(
      [
        { itemId: RELIC, upgradeSteps: 20, equippedSlot: 0 },
        { itemId: RELIC, upgradeSteps: 0, equippedSlot: null },
      ],
      1,
    );
    expect(c?.verdict).toBe(COMPARE_WORSE);
  });

  it('dasselbe über die FUNDSTUFE statt über die Aufwertung', () => {
    // Beide Achsen laufen in `effectiveLevelOf` zusammen; dass der Tausch auch
    // die zweite sieht, ist damit nicht selbstverständlich, sondern gemessen.
    const c = compareBagItem(
      [
        { itemId: RELIC, upgradeSteps: 0, equippedSlot: 0, itemLevel: 20 },
        { itemId: RELIC, upgradeSteps: 0, equippedSlot: null, itemLevel: 85 },
      ],
      1,
    );
    expect(c?.verdict).toBe(COMPARE_BETTER);
  });
});
