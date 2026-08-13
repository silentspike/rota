import { describe, expect, it } from 'vitest';
import { CAST_OK, Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById } from '../../src/core/data.js';

/**
 * FIXATIONs Schlüsseltalent (Issue #419, Bauart aus #418).
 *
 * # Was es ändert
 *
 * Ohne das Talent **scheitert** ein Kontroll-Zauber, wenn der Vorrat nicht
 * reicht: Der Druck ist verloren, `controlFailed` zählt hoch. Mit dem Talent
 * wirkt er mit dem, was da ist — und die Wirkung sinkt im gleichen Verhältnis.
 *
 * # Warum das kein Rabatt ist
 *
 * Billiger wäre ein Faktor, und ein Schlüsseltalent, das sich als Faktor
 * ausdrücken lässt, ist keines (#418). Hier ändert sich, **ob** der Zauber
 * überhaupt stattfindet; die Wirkung folgt dem Einsatz, statt fest zu bleiben.
 *
 * ANCHOR ist der Träger der Messung: Er kostet 6 und senkt die Phasen-Schwelle
 * um 10 Prozentpunkte, und beides ist über `phaseHoldPercent` ablesbar.
 */

const MIT_TALENT = { 'fixation-keystone': 1 };
const ANCHOR_KOSTEN = 6;
/** Zwei BIND-Würfe. Ein ungerader Stand ist nicht erreichbar. */
const TEILVORRAT = 4;

/** Sammelt Kontrolle, bis der Stand ERREICHT ist — und wirft dann ANCHOR. */
function mitVorrat(stand: number, talents?: Record<string, number>) {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: bossById('test-phase') ?? FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const anchor = alsSpellIndex(combat.indexOf('anchor'));

  let timeMs = 0;
  while (combat.control < stand && timeMs < 60000) {
    combat.stepPassive(timeMs);
    if (combat.isReady(bind, timeMs) && combat.control < stand) combat.cast(bind, timeMs);
    timeMs += 100;
  }
  const vorrat = combat.control;
  // **Warten, bis ANCHOR bereit ist.** Der erste Entwurf warf ihn sofort nach
  // dem letzten BIND — die Wartezeit lief noch, der Druck wurde abgewiesen,
  // und der Fall sah aus, als täte das Talent nichts. Derselbe Fehler wie im
  // Rust-Fall zu diesem Issue, zwei Stunden später am selben Tag.
  while (!combat.isReady(anchor, timeMs) && timeMs < 120000) {
    combat.stepPassive(timeMs);
    timeMs += 100;
  }
  const ergebnis = combat.cast(anchor, timeMs);
  return {
    vorrat,
    angenommen: ergebnis.result === CAST_OK,
    senkung: combat.phaseHoldPercent,
    fehlgeschlagen: combat.controlFailed,
    rest: combat.control,
  };
}

describe('Die Zusicherung vor der Messung', () => {
  it('vier Punkte sind WENIGER als ANCHOR kostet', () => {
    // **Vier, nicht drei.** BIND gibt zwei Punkte je Wurf (gemessen) — ein
    // ungerader Stand ist gar nicht erreichbar, und der erste Entwurf maß
    // deshalb einen Vorrat, den es nicht gibt.
    //
    // Ohne diese Zeile könnte der Vorrat längst reichen, und beide Läufe
    // unten wären gleich — der Fall verglich dann zwei volle Wirkungen.
    const lauf = mitVorrat(TEILVORRAT);
    expect(lauf.vorrat, 'der Vorrat ist nicht bei 4').toBe(TEILVORRAT);
    expect(lauf.vorrat).toBeLessThan(ANCHOR_KOSTEN);
  });
});

describe('AC2 — ohne Talent scheitert der Zauber', () => {
  it('er wird nicht angenommen und zählt als Fehlgriff', () => {
    const lauf = mitVorrat(TEILVORRAT);
    expect(lauf.senkung, 'ohne Talent darf nichts gesenkt werden').toBe(0);
    expect(lauf.fehlgeschlagen, 'der Fehlgriff wird nicht gezählt').toBeGreaterThan(0);
    expect(lauf.rest, 'ohne Talent darf nichts verbraucht werden').toBe(TEILVORRAT);
  });
});

describe('AC2 — mit Talent wirkt er anteilig', () => {
  it('die Senkung folgt dem Einsatz, statt ganz auszufallen', () => {
    // **Der eigentliche Nachweis.** 4 von 6 Punkten, also sinkt die Schwelle
    // um round(10 * 4 / 6) = 7 statt um 10 Prozentpunkte.
    const lauf = mitVorrat(TEILVORRAT, MIT_TALENT);
    expect(lauf.vorrat).toBe(TEILVORRAT);
    expect(lauf.senkung, 'mit Talent muss anteilig gesenkt werden').toBe(7);
    expect(lauf.rest, 'der Einsatz wurde nicht verbraucht').toBe(0);
  });

  it('und es ist KEIN Rabatt — bei vollem Vorrat bleibt alles wie vorher', () => {
    // Ohne diese Gegenprobe wäre auch ein Talent grün, das die Wirkung immer
    // halbiert. Bei vollem Einsatz muss die volle Wirkung stehen.
    const ohne = mitVorrat(ANCHOR_KOSTEN);
    const mit = mitVorrat(ANCHOR_KOSTEN, MIT_TALENT);
    expect(ohne.senkung, 'ANCHOR senkt bei vollem Vorrat nicht um 10').toBe(10);
    expect(mit.senkung, 'bei vollem Vorrat darf sich nichts ändern').toBe(ohne.senkung);
  });

  it('bei LEEREM Vorrat scheitert er weiterhin', () => {
    // Ein Zauber, der aus nichts etwas macht, wäre kein Schlüsseltalent,
    // sondern ein Fehler.
    const lauf = mitVorrat(0, MIT_TALENT);
    expect(lauf.vorrat, 'der Vorrat ist nicht leer — der Fall misst nichts').toBe(0);
    expect(lauf.senkung, 'aus leerem Vorrat darf keine Wirkung entstehen').toBe(0);
    expect(lauf.fehlgeschlagen).toBeGreaterThan(0);
  });
});
