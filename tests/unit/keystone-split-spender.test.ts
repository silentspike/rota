import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { KEYSTONE_SPLIT_SPENDER } from '../../src/progression/talents.js';

/**
 * Das erste Schlüsseltalent ändert ein **Verhalten**, nicht eine Zahl
 * (Issue #418, GAME-DESIGN 13.09).
 *
 * > „Der Entlade-Zauber verbraucht nur die HÄLFTE der Hitze, wirkt dafür
 * > ZWEIMAL."
 *
 * # Warum der Gegenfall hier der Kern ist
 *
 * „Wirkt sich aus" ist bei einem Faktor trivial — jeder Faktor ändert eine
 * Schadenszahl. Ein Schlüsseltalent muss mehr: Es muss den **Verbrauch** oder
 * die **Reihenfolge** ändern. Zeigt sich der Unterschied nur im Schaden, ist
 * es kein Schlüsseltalent, sondern ein Faktor mit einem großen Namen.
 *
 * Deshalb misst dieser Fall den **Vorrat nach dem Wirken**, nicht den Schaden.
 * Der Schaden ist die Folge, der Verbrauch ist die Sache.
 */

const BOSS = FIRST_CRUCIBLE.value;
const MIT_TALENT = { 'calcination-keystone': 1 };

/** Ein Kampf mit aufgeladenem Vorrat, bereit für den Entlade-Zauber. */
function geladen(talents?: Record<string, number>): Combat {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: BOSS,
    ...(talents === undefined ? {} : { talents }),
  });
  const kindle = combat.indexOf('kindle');
  // KINDLE lädt Hitze auf. Zwölf Wirkungen bringen den Vorrat über die
  // Bonus-Schwelle — gemessen, nicht geschätzt (siehe die Zusicherung unten).
  let t = 0;
  for (let n = 0; n < 12; n++) {
    combat.cast(kindle, t);
    t += 1500;
  }
  return combat;
}

describe('AC2/AC3 — das Talent ändert den VERBRAUCH, nicht nur den Schaden', () => {
  it('ohne Talent verbraucht der Entlade-Zauber alles', () => {
    const c = geladen();
    const vorrat = c.resource;
    expect(vorrat, 'der Vorrat ist leer — dann misst der Fall unten nichts').toBeGreaterThan(1);

    c.cast(c.indexOf('pyre'), 20000);

    expect(c.resource, 'ohne Talent muss der Entlade-Zauber alles verbrauchen').toBe(0);
  });

  it('mit Talent bleibt ein Rest liegen — DAS ist der Unterschied', () => {
    // **Der eigentliche Nachweis.** Bei ungeradem Vorrat teilt die
    // ganzzahlige Rechnung nicht auf: Aus 15 werden zweimal 7, und eine
    // Hitze bleibt übrig (GAME-DESIGN 18.4 — ganzzahlig bis zur letzten
    // Teilung).
    //
    // Ein Faktor könnte das nicht: Er änderte den Schaden und ließe den
    // Vorrat, wo er ist.
    const c = geladen(MIT_TALENT);
    const vorrat = c.resource;

    c.cast(c.indexOf('pyre'), 20000);

    const rest = c.resource;
    expect(
      rest,
      `mit dem Schlüsseltalent muss bei ungeradem Vorrat (${vorrat}) ein Rest ` +
        `liegen bleiben — geblieben sind ${rest}. Ist er 0, verbraucht der ` +
        `Zauber weiterhin alles, und das Talent ändert nur eine Zahl.`,
    ).toBe(vorrat - 2 * Math.floor(vorrat / 2));
  });

  it('und es ist nicht nur ein anderer Schaden — beide Läufe starten gleich', () => {
    // **Die Ursache mitgemessen.** Ohne diese Zeile bliebe offen, ob die
    // beiden Läufe überhaupt vergleichbar sind: Ein anderer Vorrat vor dem
    // Wirken erklärte den Unterschied ebenso gut wie das Talent.
    const ohne = geladen();
    const mit = geladen(MIT_TALENT);
    expect(mit.resource, 'die beiden Läufe starten mit verschiedenem Vorrat').toBe(ohne.resource);
  });
});

describe('AC1 — die Nicht-Zahl kommt aus den Daten, nicht aus dem Code', () => {
  it('das Talent trägt ein Verhalten, keinen Promille-Wert', () => {
    const c = geladen(MIT_TALENT);
    expect(c.hasKeystone(KEYSTONE_SPLIT_SPENDER)).toBe(true);
  });

  it('ohne Verteilung ist es aus — der Gegenfall', () => {
    // Sonst zeigte der Fall oben nur, dass `hasKeystone` `true` zurückgibt.
    const c = geladen();
    expect(c.hasKeystone(KEYSTONE_SPLIT_SPENDER)).toBe(false);
  });

  it('ein unbekanntes Verhalten ist aus, nicht an', () => {
    // **Der Fall, den eine nachlässige Umsetzung falsch macht.** Ein
    // `hasKeystone` das bei unbekannter Kennung `true` liefert, schaltete
    // jedes Verhalten frei, sobald irgendein Schlüsseltalent verteilt ist.
    const c = geladen(MIT_TALENT);
    expect(c.hasKeystone('gibt-es-nicht')).toBe(false);
  });
});
