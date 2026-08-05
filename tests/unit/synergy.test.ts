import { describe, expect, it } from 'vitest';
import { Combat, SYNERGY_BONUS_PERCENT } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { hastedMs } from '../../src/core/haste.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';

/**
 * Synergie-Fenster im Kampf (GAME-DESIGN 5, Issue #318).
 *
 * # Der Bonus liegt NEBEN dem Stapel-Bonus, nicht darin
 *
 * FESTER wird ohnehin stärker, je mehr Zersetzungs-Stapel liegen
 * (`perDecayStackPercent`) — und CANKER, der das Fenster öffnet, legt genau
 * solche Stapel an. Beide Mechaniken feuern also bei derselben Wirkung.
 *
 * **Eine Gegenprobe, die den Stapelstand mitvariiert, misst die Stapel und
 * nicht die Synergie.** Die Fälle hier halten ihn deshalb konstant und
 * verschieben nur den Zeitpunkt: Ein Stapel liegt 30 s, das Fenster steht
 * 3 s offen. Dazwischen ist Platz für beide Zustände.
 */

function bau(): Combat {
  const boss = bossById('test-warden');
  if (boss === undefined) throw new Error('test-warden fehlt');
  return new Combat({ school: CALCINATION.value, boss });
}

/** Wirkt einen Zauber, sobald er bereit ist, und gibt die Kampfzeit zurück. */
function wirke(combat: Combat, id: string, abMs: number): number {
  const index = combat.indexOf(id);
  for (let t = abMs; t < abMs + 30_000; t += 10) {
    combat.stepPassive(t);
    if (combat.cast(index, t).result === 0) return t;
  }
  throw new Error(`${id} kam nicht zum Zug`);
}

describe('AC2 — eine Gelegenheit wird gezählt', () => {
  it('CANKER öffnet FESTER ein Fenster', () => {
    const combat = bau();
    expect(combat.synergyAvailable, 'vor dem ersten Auslöser gibt es keine Gelegenheit').toBe(0);

    wirke(combat, 'canker', 0);

    expect(
      combat.synergyAvailable,
      'CANKER hat gewirkt und FESTER ist bereit — das ist eine Gelegenheit',
    ).toBe(1);
    expect(combat.synergyUsed, 'genutzt wurde noch nichts').toBe(0);
  });

  it('ein Zauber ohne Beziehung öffnet keins', () => {
    // **Der Gegenfall.** Ohne ihn bliebe offen, ob der Zähler bei JEDER
    // Wirkung hochgeht.
    const combat = bau();
    wirke(combat, 'kindle', 0);
    expect(combat.synergyAvailable).toBe(0);
  });
});

describe('AC3 — genutzt zählt nur im Fenster', () => {
  it('FESTER im Fenster zählt, FESTER danach nicht — ein Zeitpunkt Unterschied', () => {
    // **Zwei Läufe, gleicher Stapelstand.** CANKER legt in beiden genau einen
    // Stapel an, und der liegt 30 s. Der einzige Unterschied ist, WANN FESTER
    // wirkt — im Fenster oder danach.
    const fenster = 2 * GCD_MS;

    const drin = bau();
    const canker1 = wirke(drin, 'canker', 0);
    expect(drin.decayStacks, 'ohne Stapel wäre der Vergleich unten sinnlos').toBe(1);
    const festerDrin = wirke(drin, 'fester', canker1 + GCD_MS);
    expect(festerDrin - canker1, 'FESTER wirkte außerhalb des Fensters').toBeLessThan(fenster);

    const draussen = bau();
    const canker2 = wirke(draussen, 'canker', 0);
    expect(draussen.decayStacks, 'die beiden Läufe haben verschiedene Stapelstände').toBe(1);
    const festerDraussen = wirke(draussen, 'fester', canker2 + fenster + 10);
    expect(festerDraussen - canker2, 'FESTER wirkte doch noch im Fenster').toBeGreaterThan(fenster);

    expect(drin.synergyUsed, 'die Wirkung im Fenster zählt nicht als genutzt').toBe(1);
    expect(draussen.synergyUsed, 'eine Wirkung nach Fensterende zählt mit').toBe(0);
  });

  it('und der Schaden folgt dem Zähler', () => {
    // **Die Zahl allein belegt nur die Buchführung.** Erst der Schaden zeigt,
    // dass der Faktor auch in der Rechnung ankommt.
    const fenster = 2 * GCD_MS;

    const drin = bau();
    const c1 = wirke(drin, 'canker', 0);
    const a = drin.cast(drin.indexOf('fester'), c1 + GCD_MS);

    const draussen = bau();
    const c2 = wirke(draussen, 'canker', 0);
    const b = draussen.cast(draussen.indexOf('fester'), c2 + fenster + 10);

    expect(a.result).toBe(0);
    expect(b.result).toBe(0);
    expect(
      a.damage,
      `im Fenster ${a.damage}, danach ${b.damage} — der Faktor kommt in der ` +
        'Schadensrechnung nicht an',
    ).toBeGreaterThan(b.damage);
    // **Um den gesetzten Anteil, nicht um irgendetwas** — aber als Verhältnis
    // mit Rundungsspielraum, nicht als nachgebaute Zahl.
    //
    // Hier stand `toBe(Math.round(b.damage * 1.25))` und meldete „expected 77
    // to be 78": Der Faktor geht in die GESAMTE Formel ein und wird einmal am
    // Ende gerundet, nicht auf den fertigen Schaden aufgeschlagen. Wer die
    // Zahl nachbaut, dupliziert die Formel — und prüft am Ende seine eigene
    // Kopie gegen das Original.
    const verhaeltnis = a.damage / b.damage;
    const erwartet = (100 + SYNERGY_BONUS_PERCENT) / 100;
    expect(verhaeltnis, `${a.damage} zu ${b.damage} ist ${verhaeltnis}`).toBeCloseTo(erwartet, 1);
  });

  it('das Fenster ist nach einer Nutzung verbraucht', () => {
    // Sonst zählte jede weitere Wirkung darin erneut, und die Ausbeute stiege
    // über die Zahl der Gelegenheiten.
    const combat = bau();
    const c = wirke(combat, 'canker', 0);
    wirke(combat, 'fester', c + GCD_MS);
    expect(combat.synergyUsed).toBe(1);
    expect(combat.synergyUsed).toBeLessThanOrEqual(combat.synergyAvailable);
  });
});

describe('AC4 — das Fenster steht in Wartezeiten, nicht in Millisekunden', () => {
  it('bei hohem CATALYSIS schrumpft es in Millisekunden mit', () => {
    // **Die Gegenprobe zur Einheit.** Wäre das Fenster als feste
    // Millisekundenzahl abgelegt, bliebe es bei hohem Tempo gleich lang — und
    // der Spieler hätte plötzlich vier Wartezeiten Zeit statt zwei
    // (GAME-DESIGN 327).
    //
    // Gemessen wird an der Grenze: FESTER genau am Fensterende bei erhöhtem
    // Tempo. Ist das Fenster mitgeschrumpft, liegt dieser Zeitpunkt draußen.
    const boss = bossById('test-warden');
    if (boss === undefined) throw new Error('test-warden fehlt');
    const catalysis = 5000;
    const schnell = new Combat({
      school: CALCINATION.value,
      boss,
      worn: { ...NOTHING_WORN, catalysis },
    });
    const gehastet = hastedMs(2 * GCD_MS, catalysis);
    expect(gehastet, 'CATALYSIS wirkt nicht — die Gegenprobe misst nichts').toBeLessThan(
      2 * GCD_MS,
    );

    const c = wirke(schnell, 'canker', 0);
    // Ein Zeitpunkt, der im UNGEHASTETEN Fenster läge, im gehasteten aber nicht.
    const dazwischen = c + gehastet + 10;
    expect(dazwischen - c).toBeLessThan(2 * GCD_MS);
    wirke(schnell, 'fester', dazwischen);

    expect(
      schnell.synergyUsed,
      'FESTER wirkte nach dem gehasteten Fensterende und zählte trotzdem — ' +
        'dann steht das Fenster in Millisekunden statt in Wartezeiten',
    ).toBe(0);
  });
});
