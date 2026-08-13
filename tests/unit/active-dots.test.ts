/**
 * Die Schleife über laufende Brände (Issue #517).
 *
 * # Was hier geprüft wird — und was noch nicht
 *
 * Der **Baustein**, den EMBER und PYROCLASM sich teilen: eine Iteration über
 * die Brände, die gerade brennen. Die beiden Zauber selbst sind Datensätze und
 * kommen danach; ohne den Baustein wären sie nicht baubar.
 *
 * Gemessen war: `dotExpiresAtMs` hatte **sechs** Zugriffe, alle je Index oder
 * `fill` ohne Bedingung. Es gab keine Stelle, an der man etwas mit *allen
 * laufenden* Bränden tun konnte.
 */
import { describe, expect, it } from 'vitest';
import { Combat, type SpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Einen Brand legen und den Kampf bis `bisMs` laufen lassen. */
function mitBrand(bisMs: number): { combat: Combat; smoulder: SpellIndex } {
  const combat = kampf();
  const smoulder = combat.indexOf('smoulder');
  combat.cast(smoulder, 0);
  for (let t = 0; t <= bisMs; t += 10) combat.stepPassive(t);
  return { combat, smoulder };
}

describe('AC1 — die Iteration findet, was gerade brennt', () => {
  it('EMBER verlängert einen laufenden Brand', () => {
    const { combat, smoulder } = mitBrand(1000);
    const vorher = combat.dotRemainingMs(smoulder, 1000);
    combat.extendActiveDots(1000, 1);
    expect(combat.dotRemainingMs(smoulder, 1000)).toBeGreaterThan(vorher);
  });

  it('und zwar um GENAU ein Tick-Intervall, nicht um irgendetwas', () => {
    // Die Richtung allein wäre zu weich: Jede Verlängerung wäre grün. Der
    // Betrag ist die Aussage.
    const { combat, smoulder } = mitBrand(1000);
    const vorher = combat.dotRemainingMs(smoulder, 1000);
    combat.extendActiveDots(1000, 1);
    const dazu = combat.dotRemainingMs(smoulder, 1000) - vorher;
    const abstand = combat.spellAt(smoulder)?.dot?.tickIntervalMs ?? 0;
    expect(
      abstand,
      'SMOULDER hat keinen Tick-Abstand — dann prüft der Fall nichts',
    ).toBeGreaterThan(0);
    expect(dazu).toBe(abstand);
  });
});

describe('AC2 — der Takt bleibt stehen, wenn die Laufzeit wächst', () => {
  it('ein verlängerter Brand tickt WEITER, nicht seltener', () => {
    // **Der Kern von AC2.** `dotNextTickAtMs` läuft unabhängig von der
    // Laufzeit. Wer beim Verlängern auch den Takt verschöbe, liesse den Brand
    // länger brennen und dabei seltener ticken — der Zuwachs käme nie an.
    const { combat, smoulder } = mitBrand(1000);
    const abstand = combat.spellAt(smoulder)?.dot?.tickIntervalMs ?? 0;
    const dauer = combat.spellAt(smoulder)?.dot?.durationMs ?? 0;

    combat.extendActiveDots(1000, 1);
    for (let t = 1000; t <= dauer + abstand + 200; t += 10) combat.stepPassive(t);

    // Ein Tick mehr als ohne Verlängerung: dieselbe Rechnung, ein Intervall
    // länger.
    const ohne = kampf();
    ohne.cast(ohne.indexOf('smoulder'), 0);
    for (let t = 0; t <= dauer + abstand + 200; t += 10) ohne.stepPassive(t);

    expect(combat.dotTickCount(smoulder)).toBe(ohne.dotTickCount(ohne.indexOf('smoulder')) + 1);
  });
});

describe('AC3 — PYROCLASM rechnet den Rest und räumt ab', () => {
  it('er gibt Schaden und löscht den Brand', () => {
    const { combat, smoulder } = mitBrand(1000);
    expect(combat.dotRemainingMs(smoulder, 1000)).toBeGreaterThan(0);

    const schaden = combat.consumeActiveDots(1000);
    expect(schaden, 'ein laufender Brand muss etwas beitragen').toBeGreaterThan(0);
    expect(combat.dotRemainingMs(smoulder, 1000), 'der Brand muss weg sein').toBe(0);
  });

  it('ein ABGELAUFENER Brand trägt null bei', () => {
    // Die Gegenprobe zur Schleife: Sie darf nur finden, was wirklich brennt.
    const dauer = kampf().spellAt(kampf().indexOf('smoulder'))?.dot?.durationMs ?? 0;
    const { combat } = mitBrand(dauer + 1000);
    expect(combat.consumeActiveDots(dauer + 1000)).toBe(0);
  });
});

describe('AC5 — ohne laufende Brände tun beide nichts und brechen nicht', () => {
  it('EMBER auf einem leeren Kampf', () => {
    const combat = kampf();
    expect(() => combat.extendActiveDots(0, 1)).not.toThrow();
  });

  it('PYROCLASM auf einem leeren Kampf gibt null', () => {
    expect(kampf().consumeActiveDots(0)).toBe(0);
  });

  it('und EMBER mit null Ticks bewegt nichts', () => {
    const { combat, smoulder } = mitBrand(1000);
    const vorher = combat.dotRemainingMs(smoulder, 1000);
    combat.extendActiveDots(1000, 0);
    expect(combat.dotRemainingMs(smoulder, 1000)).toBe(vorher);
  });
});

describe('Die Zauber sind ANGESCHLOSSEN, nicht nur die Mechanik', () => {
  // **Der Baustein allein wäre die Klasse „gebaut, geprüft, nicht
  // angeschlossen".** Diese Fälle prüfen den Weg vom Tastendruck bis zur
  // Wirkung — nicht die Methode, sondern den Zauber.

  it('EMBER verlängert beim WIRKEN, nicht erst beim Aufruf der Methode', () => {
    // **Erst 1600 ms, nicht 1000.** Der GCD ist nach SMOULDER bis 1500 belegt;
    // ein Wurf davor wird abgelehnt, und der Fall pruefte dann den Anschluss
    // nicht, sondern die Wartezeit.
    const { combat, smoulder } = mitBrand(1600);
    const vorher = combat.dotRemainingMs(smoulder, 1600);
    expect(combat.cast(combat.indexOf('ember'), 1600).result, 'EMBER wurde abgelehnt').toBe(0);
    expect(combat.dotRemainingMs(smoulder, 1600)).toBeGreaterThan(vorher);
  });

  it('EMBER macht dabei KEINEN Schaden — das ist seine Aussage', () => {
    const combat = kampf();
    combat.cast(combat.indexOf('ember'), 0);
    expect(combat.damageDealt).toBe(0);
  });

  it('PYROCLASM verbraucht die Brände beim Wirken', () => {
    const { combat, smoulder } = mitBrand(1600);
    const vorher = combat.damageDealt;
    expect(combat.cast(combat.indexOf('pyroclasm'), 1600).result, 'PYROCLASM wurde abgelehnt').toBe(
      0,
    );
    expect(combat.damageDealt, 'er muss den Rest als Schaden geben').toBeGreaterThan(vorher);
    expect(combat.dotRemainingMs(smoulder, 1600), 'und den Brand abräumen').toBe(0);
  });

  it('GEGENPROBE: ein Zauber OHNE die Felder lässt Brände in Ruhe', () => {
    // Ohne diese Zeile wäre nicht zu sehen, ob die Felder wirken oder ob
    // jeder Zauber die Brände anfasst.
    const { combat, smoulder } = mitBrand(1600);
    const vorher = combat.dotRemainingMs(smoulder, 1600);
    expect(combat.cast(combat.indexOf('kindle'), 1600).result, 'KINDLE wurde abgelehnt').toBe(0);
    expect(combat.dotRemainingMs(smoulder, 1600)).toBe(vorher);
  });
});

describe('Verlängern und Erneuern sind ZWEI Regeln (#485, AC2)', () => {
  /**
   * Der Kern des Issues, und er stand bis hierher nirgends.
   *
   * GAME-DESIGN 7.2 hat sich ausdrücklich für die **harte** Erneuerung
   * entschieden: „Erneuern setzt zurück, die Restzeit ist verloren" — *„die
   * Präzision ist gewollt und wird als Können belohnt"*. Eine Verlängerung,
   * die daraus versehentlich eine weiche Regel machte, höbe diese
   * Entscheidung auf, ohne dass irgendein Prüffall es meldete.
   *
   * Der Unterschied in einem Satz: **Erneuern setzt auf volle Dauer zurück.
   * Verlängern addiert auf die Restzeit.**
   *
   * Beide Richtungen stehen bewusst im selben `describe`: Ein Fall, der nur
   * die Verlängerung misst, belegt nicht, dass sie eine ANDERE Regel ist als
   * das Erneuern.
   */
  it('ERNEUERN setzt auf volle Dauer — die Restzeit ist verloren', () => {
    const { combat, smoulder } = mitBrand(1000);
    const dauer = combat.spellAt(smoulder)?.dot?.durationMs ?? 0;
    expect(dauer, 'SMOULDER hat keine Dauer — dann prüft der Fall nichts').toBeGreaterThan(0);

    // Kurz vor Schluss erneuern: Was übrig war, zählt NICHT dazu.
    const spaet = dauer - 1000;
    for (let t = 1000; t <= spaet; t += 10) combat.stepPassive(t);
    const rest = combat.dotRemainingMs(smoulder, spaet);
    expect(rest, 'der Brand läuft nicht mehr — der Fall misst nichts').toBeGreaterThan(0);

    expect(combat.cast(smoulder, spaet).result, 'SMOULDER wurde abgelehnt').toBe(0);

    // GENAU die volle Dauer, nicht Dauer plus Rest. Das ist die harte Regel.
    expect(combat.dotRemainingMs(smoulder, spaet)).toBe(dauer);
    expect(
      combat.dotRemainingMs(smoulder, spaet),
      'Erneuern hat die Restzeit ADDIERT — 7.2 ist ausgehebelt',
    ).not.toBe(dauer + rest);
  });

  it('VERLÄNGERN addiert auf die Restzeit — dieselbe Lage, anderes Ergebnis', () => {
    // Wortwörtlich derselbe Aufbau wie oben, nur mit EMBER statt einer
    // Erneuerung. Der Unterschied im Ergebnis ist die ganze Aussage.
    const { combat, smoulder } = mitBrand(1000);
    const dauer = combat.spellAt(smoulder)?.dot?.durationMs ?? 0;
    const abstand = combat.spellAt(smoulder)?.dot?.tickIntervalMs ?? 0;

    const spaet = dauer - 1000;
    for (let t = 1000; t <= spaet; t += 10) combat.stepPassive(t);
    const rest = combat.dotRemainingMs(smoulder, spaet);

    combat.extendActiveDots(spaet, 1);

    // Rest PLUS ein Intervall — und ausdrücklich nicht die volle Dauer.
    expect(combat.dotRemainingMs(smoulder, spaet)).toBe(rest + abstand);
    expect(
      combat.dotRemainingMs(smoulder, spaet),
      'Verlängern hat auf volle Dauer gesetzt — dann wäre es eine Erneuerung',
    ).not.toBe(dauer);
  });

  it('und ein verlängerter Brand bleibt erneuerbar — hart wie zuvor', () => {
    // Die Reihenfolge könnte etwas verstellen: Wer verlängert und DANN
    // erneuert, muss trotzdem bei der vollen Dauer landen, nicht darüber.
    //
    // **Bei 2000 ms geworfen, nicht bei 1000.** Die globale Wartezeit läuft
    // 1500 ms; ein Wurf davor wird abgelehnt, und der Fall prüfte dann nur,
    // dass eine Ablehnung nichts verändert.
    const { combat, smoulder } = mitBrand(1000);
    const dauer = combat.spellAt(smoulder)?.dot?.durationMs ?? 0;

    combat.extendActiveDots(1000, 3);
    for (let t = 1000; t <= 2000; t += 10) combat.stepPassive(t);
    expect(combat.dotRemainingMs(smoulder, 2000)).toBeGreaterThan(dauer - 2000);

    expect(combat.cast(smoulder, 2000).result, 'SMOULDER wurde abgelehnt').toBe(0);
    expect(combat.dotRemainingMs(smoulder, 2000)).toBe(dauer);
  });
});

describe('Die Verlängerung findet ALLE laufenden Brände (#485, AC3)', () => {
  /**
   * Die zweite Falle des Issues. Jeder bisherige Fall lief mit **einem**
   * Brand; ein Zugriff, der nur den zuletzt angelegten fände, hätte sie alle
   * bestanden.
   *
   * Drei Brände, drei verschiedene Dauern und Takte (SMOULDER 12000/2000,
   * SEARING 6000/2000, CINDER 4000/1000) — das trennt zusätzlich „alle
   * gefunden" von „allen denselben Betrag gegeben".
   */
  function dreiBraende(): {
    combat: Combat;
    braende: readonly { name: string; index: SpellIndex }[];
  } {
    const combat = kampf();
    const namen = ['smoulder', 'searing', 'cinder'] as const;
    const braende = namen.map((name) => ({ name, index: combat.indexOf(name) }));
    let t = 0;
    for (const brand of braende) {
      expect(combat.cast(brand.index, t).result, `${brand.name} wurde abgelehnt`).toBe(0);
      // Die globale Wartezeit abwarten, sonst lehnt der nächste Wurf ab.
      t += 1500;
      for (let u = t - 1500; u <= t; u += 10) combat.stepPassive(u);
    }
    return { combat, braende };
  }

  it('alle drei brennen gleichzeitig — sonst prüft der Fall nichts', () => {
    // ZUSICHERUNG vor der Aussage: Liefe nur einer, wäre „alle drei
    // verlängert" trivial erfüllt.
    const { combat, braende } = dreiBraende();
    for (const brand of braende) {
      expect(
        combat.dotRemainingMs(brand.index, 3000),
        `${brand.name} brennt nicht`,
      ).toBeGreaterThan(0);
    }
  });

  it('EMBER verlängert JEDEN von ihnen, je um seinen EIGENEN Takt', () => {
    const { combat, braende } = dreiBraende();
    const vorher = braende.map((b) => combat.dotRemainingMs(b.index, 3000));

    combat.extendActiveDots(3000, 1);

    for (const [i, brand] of braende.entries()) {
      const takt = combat.spellAt(brand.index)?.dot?.tickIntervalMs ?? 0;
      const dazu = combat.dotRemainingMs(brand.index, 3000) - (vorher[i] ?? 0);
      expect(dazu, `${brand.name} wurde nicht um seinen eigenen Takt verlängert`).toBe(takt);
    }
  });

  it('CINDER hat einen ANDEREN Takt als die beiden anderen', () => {
    // Ohne das wäre „je um seinen eigenen Takt" nicht von „allen denselben
    // Betrag" zu unterscheiden — beide Aussagen sähen gleich aus.
    const combat = kampf();
    const taktVon = (name: string): number =>
      combat.spellAt(combat.indexOf(name))?.dot?.tickIntervalMs ?? 0;
    expect(taktVon('cinder')).not.toBe(taktVon('smoulder'));
  });
});
