import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, TALENTS, bossById } from '../../src/core/data.js';

/**
 * Der globale Talentbaum trägt drei VERSCHIEDENE Größen (Issue #419).
 *
 * # Was vorher war
 *
 * Alle drei Wege führten `damagePerRankPerMille: 15` — dieselbe Größe, derselbe
 * Betrag. Drei Wege, die dasselbe um dasselbe bewegen, können nicht „jeder
 * irgendwo die beste Wahl" sein; AC3 war für sie aus den Daten heraus
 * unerfüllbar, nicht aus dem Code.
 *
 * # Was jeder Weg jetzt tut
 *
 * ```text
 *   build    "Greater Catalysis"  Aufbau      resourceGain, decayStack, controlGain
 *   release  "Greater Purity"     Entlader    spenderDamage
 *   hold     "Greater Vessel"     Haltbarkeit dotDuration, decayDuration, shieldDuration
 * ```
 *
 * # Was diese Datei NICHT belegt, und warum das hier steht
 *
 * Zwei der sechs mitgenommenen Bestandsgrößen sind im Balance-Läufer heute
 * nicht messbar: `controlGain` und `shieldDuration` (gemessen, 6771). Sie
 * stehen in den Daten, weil sie fachlich in ihren Weg gehören — aber kein Fall
 * unten behauptet ihre Wirkung. Ein Kommentar ersetzt keine Prüfung, und eine
 * Prüfung, die nichts erreicht, ist schlimmer als keine.
 */

const talentById = (id: string) => TALENTS.find((t) => t.id === id);

describe('Die Zusicherung vor der Messung', () => {
  it('die drei Wege tragen NICHT mehr dieselbe Größe', () => {
    // Der Fall, der den alten Zustand rot färben würde. Ohne ihn könnten die
    // Daten zurückfallen, ohne dass ein Wirkungs-Fall unten es merkt: Ein
    // Schadensfaktor auf allen dreien wäre ja messbar — nur eben dreimal
    // derselbe.
    const wege = ['global-build', 'global-release', 'global-hold'].map((id) => {
      const t = talentById(id);
      expect(t, `${id} fehlt in den Talentdaten`).toBeDefined();
      return t;
    });
    for (const weg of wege) {
      expect(weg?.damagePerRankPerMille, `${weg?.id} trägt weiter den Schadensfaktor`).toBe(0);
    }
    expect(
      wege[0]?.buildDamagePerRankPerMille,
      'build hebt den Schaden der Aufbau-Zauber nicht',
    ).toBeGreaterThan(0);
    expect(wege[1]?.spenderDamagePerRankPerMille, 'release hebt keine Entlader').toBeGreaterThan(0);
    expect(wege[2]?.dotDurationPerRankPerMille, 'hold hebt keine Haltbarkeit').toBeGreaterThan(0);
  });
});

/** Wirkt einen Zauber nach acht Füllern und gibt seinen Schaden zurück. */
function schadenVon(zauber: string, talents?: Record<string, number>): number {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
  // Acht KINDLE bauen Hitze auf — ohne sie hätte PYRE nichts zu entladen.
  for (let i = 0; i < 8; i++) combat.cast(alsSpellIndex(combat.indexOf('kindle')), i * 1500);
  const vorher = combat.damageDealt;
  combat.cast(alsSpellIndex(combat.indexOf(zauber)), 12000);
  return combat.damageDealt - vorher;
}

describe('release — der Entlader-Weg', () => {
  it('PYRE schlägt härter zu', () => {
    // Fünf Ränge à 100 Promille = Faktor 1,5.
    const ohne = schadenVon('pyre');
    const mit = schadenVon('pyre', { 'global-release': 5 });
    expect(ohne, 'PYRE teilt gar keinen Schaden aus — der Vergleich misst nichts').toBeGreaterThan(
      0,
    );
    expect(mit, 'der Entlader-Zuschlag kommt nicht an').toBe(Math.round(ohne * 1.5));
  });

  it('KINDLE aber NICHT — und das ist der ganze Punkt', () => {
    // **Der Fall, der einen Zuschlag auf alles von einem auf Entlader trennt.**
    // Ohne ihn wäre ein Faktor, der jeden Zauber trifft, genauso grün.
    const ohne = schadenVon('kindle');
    const mit = schadenVon('kindle', { 'global-release': 5 });
    expect(ohne, 'KINDLE teilt keinen Schaden aus — der Vergleich misst nichts').toBeGreaterThan(0);
    expect(mit, 'der Zuschlag trifft auch Nicht-Entlader').toBe(ohne);
  });

  it('und er bleibt nicht am nächsten Zauber hängen', () => {
    // **Der Fehler, den die Bauart nahelegt.** Der Talent-Topf ist EIN Feld;
    // setzte ihn nur der Entlader, trüge der nächste Treffer den Zuschlag
    // weiter. Deshalb schreibt `applySpenderBonus` auch den gewöhnlichen Fall
    // zurück — und dieser Fall prüft genau das, in der Reihenfolge PYRE dann
    // KINDLE.
    const combat = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: { 'global-release': 5 },
    });
    for (let i = 0; i < 8; i++) combat.cast(alsSpellIndex(combat.indexOf('kindle')), i * 1500);
    combat.cast(alsSpellIndex(combat.indexOf('pyre')), 12000);
    const vorKindle = combat.damageDealt;
    combat.cast(alsSpellIndex(combat.indexOf('kindle')), 13500);
    const kindleDanach = combat.damageDealt - vorKindle;

    expect(kindleDanach, 'KINDLE nach PYRE trägt dessen Zuschlag weiter').toBe(
      schadenVon('kindle'),
    );
  });
});

describe('build — der Aufbau-Weg', () => {
  it('KINDLE schlägt härter zu — seit dem Umbau der Aufbau-SCHADEN', () => {
    // **Der Weg hob bis #419 die Aufbau-MENGE** (Hitze, Kontrollpunkte,
    // Stapel) und verlor damit gegen `release` in jeder Rotation: Mehr Hitze
    // wirkt über den Entlader, der Zuschlag wirkt direkt auf ihn — der direkte
    // Weg schlägt den indirekten.
    //
    // Jetzt hebt er den Schaden der Zauber, die ANSAMMELN. Gemessen tragen die
    // 81 % des Schadens einer Rotation mit Entlader und 100 % einer ohne;
    // `build` und `release` heben damit verschiedene Zauber statt derselben
    // Summe.
    const schaden = (talents?: Record<string, number>) => {
      const combat = new Combat({
        school: CALCINATION.value,
        boss: FIRST_CRUCIBLE.value,
        ...(talents === undefined ? {} : { talents }),
      });
      combat.cast(alsSpellIndex(combat.indexOf('kindle')), 0);
      return combat.damageDealt;
    };
    const ohne = schaden();
    expect(ohne, 'KINDLE teilt keinen Schaden aus — der Vergleich misst nichts').toBeGreaterThan(0);
    expect(schaden({ 'global-build': 5 }), 'der Aufbau-Weg hebt den Schaden nicht').toBeGreaterThan(
      ohne,
    );
  });

  it('und PYRE nicht — er sammelt nichts an', () => {
    // **Die Trennschärfe.** Ohne diesen Fall wäre ein Zuschlag auf ALLE Zauber
    // genauso grün — und `build` und `release` hätten wieder dieselbe Wirkung.
    const schadenVon = (zauber: string, talents?: Record<string, number>) => {
      const combat = new Combat({
        school: CALCINATION.value,
        boss: FIRST_CRUCIBLE.value,
        ...(talents === undefined ? {} : { talents }),
      });
      for (let i = 0; i < 8; i++) combat.cast(alsSpellIndex(combat.indexOf('kindle')), i * 1500);
      const vorher = combat.damageDealt;
      combat.cast(alsSpellIndex(combat.indexOf(zauber)), 12000);
      return combat.damageDealt - vorher;
    };
    const ohne = schadenVon('pyre');
    expect(ohne, 'PYRE teilt keinen Schaden aus — der Vergleich misst nichts').toBeGreaterThan(0);
    expect(schadenVon('pyre', { 'global-build': 5 }), 'der Aufbau-Weg trifft auch PYRE').toBe(ohne);
  });

  it('die Zersetzung hebt er bei den heutigen Zauberdaten NICHT — gemessen', () => {
    // **Kein Versehen, sondern ein festgehaltener Befund.** `addDecayStacks`
    // rundet ab (`floor`), und alle drei Quellen legen genau EINEN Stapel:
    //
    //   canker  stacks 1   ·   blight  stacks 1   ·   rot  decayPerTick 1
    //
    // Bei Basis 1 wirkt ein Faktor erst ab 2,0:
    //
    //   floor(1 × 1,3) = 1     floor(1 × 1,5) = 1     floor(1 × 2,0) = 2
    //
    // Der globale Weg trägt 5 × 60 Promille, also 1,3. Dass `decayStack` in
    // seinen Daten steht, ist fachlich richtig und heute folgenlos — dieser
    // Fall behauptet deshalb NICHT das Gegenteil, sondern nagelt den Zustand
    // fest. Ändert jemand die Zahlen, wird er rot und verlangt eine
    // Entscheidung, statt eine stille Wirkung entstehen zu lassen.
    //
    // Dasselbe trifft den SCHUL-Weg `putrefaction-build` (5 × 100 = 1,5). Der
    // Befund ist gemeldet; die Zahl zu ändern ist Balance und nicht meine
    // Entscheidung.
    const stapel = (talents?: Record<string, number>) => {
      const combat = new Combat({
        school: CALCINATION.value,
        boss: bossById('training-dummy') ?? FIRST_CRUCIBLE.value,
        ...(talents === undefined ? {} : { talents }),
      });
      combat.cast(alsSpellIndex(combat.indexOf('canker')), 0);
      return combat.decayStacks;
    };
    const ohne = stapel();
    expect(ohne, 'CANKER legt keine Stapel — der Vergleich misst nichts').toBe(1);
    expect(
      stapel({ 'global-build': 5 }),
      'die Stapelzahl hat sich geändert — dann ist der Befund oben veraltet',
    ).toBe(ohne);
    expect(
      stapel({ 'putrefaction-build': 5 }),
      'auch der Schul-Weg bewegt sie nicht (5 × 100 Promille = Faktor 1,5)',
    ).toBe(ohne);
  });
});

describe('hold — der Haltbarkeits-Weg', () => {
  it('SMOULDER brennt länger', () => {
    const laufzeit = (talents?: Record<string, number>) => {
      const combat = new Combat({
        school: CALCINATION.value,
        boss: FIRST_CRUCIBLE.value,
        ...(talents === undefined ? {} : { talents }),
      });
      const smoulder = combat.indexOf('smoulder');
      combat.cast(alsSpellIndex(smoulder), 0);
      return combat.dotRemainingMs(smoulder, 0);
    };
    const ohne = laufzeit();
    expect(ohne, 'SMOULDER brennt gar nicht — der Vergleich misst nichts').toBeGreaterThan(0);
    expect(laufzeit({ 'global-hold': 5 }), 'der Haltbarkeits-Weg verlängert nicht').toBeGreaterThan(
      ohne,
    );
  });
});
