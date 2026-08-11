/**
 * Die fünf Ja/Nein-Schalter am Zauber (Issue #480, #498).
 *
 * # Fünf von sechs Schaltern waren ungeschützt
 *
 * Gemessen über die **oberste** Ebene der Zauber-Daten — die
 * `tools/groessen-mutation.py` nicht anfasst, weil sie nur in verschachtelte
 * Gruppen hinabsteigt:
 *
 * ```text
 *   clearsDoom             purge      true -> false   NUR FRISCHE
 *   consumesDecayStacks    necrosis   true -> false   NUR FRISCHE
 *   doublesNextCast        condense   true -> false   NUR FRISCHE
 *   freesNextGcd           sublime    true -> false   NUR FRISCHE
 *   refundsInterruptedGcd  retort     true -> false   NUR FRISCHE
 * ```
 *
 * Nur `consumesDots` bei PYROCLASM hatte einen Fall. `NUR FRISCHE` heißt: Die
 * Vergleichsdatei färbt, ein `npm run parity:export` macht sie wieder grün —
 * und die Änderung ist durch, ohne dass jemand sie bemerkt.
 *
 * Jeder Schalter gehört genau **einem** Zauber. Das ist kein Zufall: Es sind
 * die Sonderwirkungen, die einen Zauber von allen anderen unterscheiden — und
 * genau deshalb sucht sie niemand, der nicht weiß, dass es sie gibt.
 *
 * # RETORT lässt sich heute nur mit einem PRÜF-Zauber auslösen
 *
 * `refundsInterruptedGcd` erstattet die Wartezeit, wenn ein **eigenes Wirken**
 * abgebrochen wird. Gezählt über alle 66 Zauber-Datensätze:
 *
 * ```text
 *   Zauber mit castTimeMs > 0:  1   (test-cast, Stufe 101)
 * ```
 *
 * Kein einziger **spielbarer** Zauber hat eine Wirkzeit. RETORTs Wirkung ist
 * im ausgelieferten Spiel damit nicht auslösbar — der Fall unten sichert die
 * Leitung ab, bevor der erste Zauber mit Wirkzeit kommt. Der Befund ist
 * gemeldet; hier steht er, damit niemand den Prüf-Zauber für eine Bequemlichkeit
 * hält und ihn durch einen „echten" ersetzen will.
 */
import { describe, expect, it } from 'vitest';

import { CAST_OK, CAST_STARTED, Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

function neu(bossId = 'first-crucible'): Combat {
  return new Combat({ school: CALCINATION.value, boss: bossById(bossId) ?? FIRST_CRUCIBLE.value });
}

describe('PURGE räumt Verhängnis ab (clearsDoom)', () => {
  /** Lässt bei `test-doomsayer` Verhängnis auflaufen und wirft dann PURGE. */
  function raeumen(): { vor: number; nach: number; ladungenVor: number; ladungenNach: number } {
    const combat = neu('test-doomsayer');
    const purge = alsSpellIndex(combat.indexOf('purge'));
    for (let timeMs = 0; timeMs <= 9000; timeMs += TICK_MS) combat.stepPassive(timeMs);

    const vor = combat.doom;
    const ladungenVor = combat.doomCharges;
    expect(combat.cast(purge, 9000).result, 'PURGE kam nicht zustande').toBe(CAST_OK);
    return { vor, nach: combat.doom, ladungenVor, ladungenNach: combat.doomCharges };
  }

  const R = raeumen();

  it('die Zusicherung: es liegt überhaupt Verhängnis an', () => {
    // Ohne diese Zeile wäre „nachher weniger" auch bei 0 → 0 erfüllt.
    expect(R.vor, 'es ist gar kein Verhängnis aufgelaufen').toBe(45);
  });

  it('vierzig werden abgeräumt — aus 45 wird 5', () => {
    expect(R.nach, 'PURGE räumt nicht die 40 aus den Bossdaten ab').toBe(5);
  });

  it('und es kostet eine Ladung', () => {
    expect(R.ladungenVor).toBe(2);
    expect(R.ladungenNach, 'PURGE verbraucht keine Ladung').toBe(1);
  });
});

describe('NECROSIS verbraucht die Fäulnis-Stapel (consumesDecayStacks)', () => {
  function verbrauchen(): { vor: number; nach: number; schaden: number } {
    const combat = neu();
    const canker = alsSpellIndex(combat.indexOf('canker'));
    const blight = alsSpellIndex(combat.indexOf('blight'));
    const necrosis = alsSpellIndex(combat.indexOf('necrosis'));

    let zeit = 0;
    for (let i = 0; i < 6; i++) {
      combat.stepPassive(zeit);
      if (combat.isReady(canker, zeit)) combat.cast(canker, zeit);
      else if (combat.isReady(blight, zeit)) combat.cast(blight, zeit);
      zeit += 1500;
    }
    combat.stepPassive(zeit);
    const vor = combat.decayStacks;
    const schadenVor = combat.damageDealt;
    expect(combat.cast(necrosis, zeit).result, 'NECROSIS kam nicht zustande').toBe(CAST_OK);
    return { vor, nach: combat.decayStacks, schaden: combat.damageDealt - schadenVor };
  }

  const N = verbrauchen();

  it('die Zusicherung: es liegen fünf Stapel', () => {
    expect(N.vor, 'der Aufbau hat nicht fünf Stapel erreicht').toBe(5);
  });

  it('danach liegt keiner mehr', () => {
    expect(N.nach, 'NECROSIS lässt die Stapel liegen').toBe(0);
  });

  it('und er hat sie vorher noch gezählt — der Schaden steigt mit ihnen', () => {
    // **Die Reihenfolge ist die eigentliche Aussage.** Würde erst geräumt und
    // dann gerechnet, käme der Grundschaden heraus. 60 Grundschaden plus
    // 5 × 40 % ergeben 198 — die Zahl belegt, dass die Stapel VOR dem
    // Verbrauch in die Rechnung eingehen.
    expect(N.schaden, 'NECROSIS rechnet nicht mit den Stapeln, die er verbraucht').toBe(198);
  });
});

describe('CONDENSE verdoppelt den nächsten Wurf (doublesNextCast)', () => {
  /** Wirft optional `vorher` und danach KINDLE. */
  function kindleNach(vorher: string | null): number {
    const combat = neu();
    let zeit = 0;
    if (vorher !== null) {
      combat.stepPassive(0);
      expect(combat.cast(alsSpellIndex(combat.indexOf(vorher)), 0).result).toBe(CAST_OK);
      zeit = 1500;
    }
    combat.stepPassive(zeit);
    const vor = combat.damageDealt;
    combat.cast(alsSpellIndex(combat.indexOf('kindle')), zeit);
    return combat.damageDealt - vor;
  }

  it('die Zusicherung: KINDLE richtet für sich 100 an', () => {
    expect(kindleNach(null), 'KINDLE richtet nicht 100 an').toBe(100);
  });

  it('nach CONDENSE sind es 200', () => {
    // **Die Wirkung, nicht der Merker.** `nextCastDoubles` wäre auch dann
    // `true`, wenn ihn niemand läse.
    expect(kindleNach('condense'), 'CONDENSE verdoppelt nicht').toBe(200);
  });

  it('und SUBLIME verdoppelt gerade NICHT — die beiden Merker sind verschieden', () => {
    // Beide setzen einen Merker auf den nächsten Wurf. Ohne diese Zeile wäre
    // ein vertauschtes Ziel unsichtbar.
    expect(kindleNach('sublime'), 'SUBLIME verdoppelt den Schaden').toBe(100);
  });
});

describe('SUBLIME schenkt die nächste Wartezeit (freesNextGcd)', () => {
  /** Wirft optional `vorher`, dann KINDLE — geht danach sofort ein dritter? */
  function drittersofort(vorher: string | null): boolean {
    const combat = neu();
    let zeit = 0;
    if (vorher !== null) {
      combat.stepPassive(0);
      expect(combat.cast(alsSpellIndex(combat.indexOf(vorher)), 0).result).toBe(CAST_OK);
      zeit = 1500;
    }
    combat.stepPassive(zeit);
    combat.cast(alsSpellIndex(combat.indexOf('kindle')), zeit);
    combat.stepPassive(zeit + 10);
    return combat.isReady(alsSpellIndex(combat.indexOf('sear')), zeit + 10);
  }

  it('die Zusicherung: sonst blockt die Wartezeit', () => {
    expect(drittersofort(null), 'die Wartezeit blockt gar nicht').toBe(false);
  });

  it('nach SUBLIME löst KINDLE keine aus — der dritte Zauber geht sofort', () => {
    expect(drittersofort('sublime'), 'SUBLIME schenkt die Wartezeit nicht').toBe(true);
  });

  it('und CONDENSE schenkt sie gerade NICHT', () => {
    expect(drittersofort('condense'), 'CONDENSE schenkt die Wartezeit').toBe(false);
  });
});

describe('RETORT erstattet die Wartezeit bei einem Abbruch (refundsInterruptedGcd)', () => {
  /**
   * `test-caster` bricht bei 9000 ms ab (`interruptsPlayer`). Der einzige
   * Zauber mit Wirkzeit ist `test-cast` — siehe Dateikopf.
   */
  function nachAbbruch(mitRetort: boolean): { abbruchBei: number; sofortNach: boolean } {
    const combat = neu('test-caster');
    const testCast = alsSpellIndex(combat.indexOf('test-cast'));
    const kindle = alsSpellIndex(combat.indexOf('kindle'));
    const retort = alsSpellIndex(combat.indexOf('retort'));

    let gestartet = false;
    let abbruchBei = -1;
    let sofortNach = false;

    for (let timeMs = 0; timeMs <= 11000; timeMs += TICK_MS) {
      combat.stepPassive(timeMs);
      if (mitRetort && timeMs === 6000 && combat.isReady(retort, timeMs)) {
        combat.cast(retort, timeMs);
      }
      if (!gestartet && timeMs >= 8000 && combat.isReady(testCast, timeMs)) {
        const ergebnis = combat.cast(testCast, timeMs).result;
        // **`CAST_STARTED`, nicht `CAST_OK`.** Ein Zauber mit Wirkzeit landet
        // nicht sofort; ein Fall, der auf `CAST_OK` prüft, hält den Wurf für
        // gescheitert und misst danach einen Lauf ohne Wirken.
        if (ergebnis === CAST_STARTED || ergebnis === CAST_OK) gestartet = true;
      }
      if (gestartet && abbruchBei < 0 && combat.castsInterrupted > 0) {
        abbruchBei = timeMs;
        sofortNach = combat.isReady(kindle, timeMs);
      }
    }
    return { abbruchBei, sofortNach };
  }

  const OHNE = nachAbbruch(false);
  const MIT = nachAbbruch(true);

  it('die Zusicherung: das Wirken wird in beiden Läufen abgebrochen', () => {
    // Ohne Abbruch gäbe es nichts zu erstatten, und der Vergleich unten wäre
    // eine Aussage über zwei ereignislose Läufe.
    expect(OHNE.abbruchBei, 'ohne RETORT wurde nichts abgebrochen').toBe(9000);
    expect(MIT.abbruchBei, 'mit RETORT wurde nichts abgebrochen').toBe(9000);
  });

  it('ohne RETORT läuft die Wartezeit weiter', () => {
    expect(OHNE.sofortNach, 'die Wartezeit ist auch ohne RETORT weg').toBe(false);
  });

  it('mit RETORT ist sie erstattet — der nächste Zauber geht sofort', () => {
    expect(MIT.sofortNach, 'RETORT erstattet die Wartezeit nicht').toBe(true);
  });
});
