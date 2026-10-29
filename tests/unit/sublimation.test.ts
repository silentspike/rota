import { describe, expect, it } from 'vitest';
import { RULE_SPELL_CAST } from '../../src/combat/events.js';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, parseSchool } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { GCD_MS } from '../../src/core/gcd.js';

/**
 * Eine Prüf-Schule mit einem Sublimations-Zauber.
 *
 * **Sie wird nicht ausgeliefert.** SUBLIMATION hat im Design noch keine
 * Zauber (GAME-DESIGN 22 setzt die ersten zwei auf Schritt 3 an) — geprüft
 * wird deshalb die Mechanik, nicht ein Inhalt, den es noch nicht gibt. Die
 * Zahlen hier sind Prüfzahlen und ausdrücklich keine Balance-Aussage.
 */
function pruefSchule(reductionMs: number): ReturnType<typeof parseSchool> {
  return parseSchool({
    schema: 1,
    school: 'CALCINATION',
    resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
    spells: [
      // Zwei Zauber mit langer Abklingzeit, an denen sich eine Verkürzung
      // ablesen lässt, und einer, der verkürzt.
      {
        id: 'lang-a',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'A',
        cooldownMs: 10_000,
        baseDamage: 10,
      },
      {
        id: 'lang-b',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'B',
        cooldownMs: 20_000,
        baseDamage: 10,
      },
      {
        id: 'kurz',
        name: 'Kurz',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        cooldownMs: 1500,
        baseDamage: 10,
        cooldownReductionMs: reductionMs,
      },
    ],
  });
}

function pruefKampf(reductionMs = 2000): Combat {
  const school = pruefSchule(reductionMs);
  expect(school.problems).toEqual([]);
  return new Combat({ school: school.value, boss: FIRST_CRUCIBLE.value });
}

describe('Die Verkürzung kommt aus den Daten (E3-3)', () => {
  it('liest den Betrag aus dem Zauber, nicht aus dem Code', () => {
    const combat = pruefKampf();
    expect(combat.reductionOfSpell(combat.indexOf('kurz'))).toBe(2000);
    expect(combat.reductionOfSpell(combat.indexOf('lang-a'))).toBe(0);
  });

  it('beanstandet eine unsinnige Angabe', () => {
    const negativ = parseSchool({
      schema: 1,
      school: 'CALCINATION',
      resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
      spells: [
        {
          id: 'x',
          name: 'x',
          unlockLevel: 1,
          baseDamage: 0,
          castTimeMs: 0,
          triggersGcd: true,
          cooldownMs: 1000,
          cooldownReductionMs: -5,
        },
      ],
    });
    expect(negativ.problems.join(' ')).toContain('Verkürzung ist negativ');

    const krumm = parseSchool({
      schema: 1,
      school: 'CALCINATION',
      resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
      spells: [
        {
          id: 'x',
          name: 'x',
          unlockLevel: 1,
          baseDamage: 0,
          castTimeMs: 0,
          triggersGcd: true,
          cooldownMs: 1000,
          cooldownReductionMs: 12.5,
        },
      ],
    });
    expect(krumm.problems.join(' ')).toContain('nicht ganzzahlig');
  });

  it('ist als Empfänger angemeldet, nach der Stapel-Mechanik', () => {
    const combat = pruefKampf();
    const ids = combat.events.receiverIds(RULE_SPELL_CAST);
    expect(ids).toContain('sublimation');
    expect(ids.indexOf('sublimation')).toBeGreaterThan(ids.indexOf('stacks:heat'));
  });
});

describe('Verkürzung wirkt auf ALLE laufenden Abklingzeiten (AC1)', () => {
  it('verkürzt jede laufende Abklingzeit um den Betrag', () => {
    const combat = pruefKampf();
    const a = combat.indexOf('lang-a');
    const b = combat.indexOf('lang-b');
    const kurz = combat.indexOf('kurz');

    combat.cast(a, 0);
    combat.cast(b, 1500);
    // Bei 3000 ms: A läuft noch 7000, B noch 18500.
    expect(combat.cooldownRemainingMs(a, 3000)).toBe(7000);
    expect(combat.cooldownRemainingMs(b, 3000)).toBe(18_500);

    combat.cast(kurz, 3000);
    expect(combat.cooldownRemainingMs(a, 3000)).toBe(5000);
    expect(combat.cooldownRemainingMs(b, 3000)).toBe(16_500);
    expect(combat.sublimation.applicationCount).toBe(1);
    expect(combat.sublimation.shortenedCount).toBe(2);
  });

  it('verkürzt auch quer über Schulen — die Mechanik kennt keine Schule', () => {
    // AC1 wörtlich: „nicht nur auf die der eigenen Schule". Die Mechanik
    // fragt nie nach der Schule eines Zaubers, und dieser Prüffall hält das
    // fest: Er zählt die verkürzten Abklingzeiten, ohne eine auszunehmen.
    const combat = pruefKampf();
    combat.cast(combat.indexOf('lang-a'), 0);
    combat.cast(combat.indexOf('lang-b'), 1500);
    combat.cast(combat.indexOf('kurz'), 3000);
    expect(combat.sublimation.shortenedCount).toBe(2);
  });

  it('verkürzt die eigene Abklingzeit des wirkenden Zaubers NICHT', () => {
    // Umsetzungs-Festlegung, kein Design: Täte er es, wäre der Zauber
    // schlicht einer mit kürzerer Abklingzeit. Umzudrehen in einer Zeile.
    const combat = pruefKampf();
    const kurz = combat.indexOf('kurz');
    combat.cast(kurz, 0);
    expect(combat.cooldownRemainingMs(kurz, 0)).toBe(1500);
  });

  it('lässt die globale Wartezeit unberührt', () => {
    const combat = pruefKampf();
    combat.cast(combat.indexOf('kurz'), 0);
    // 1,5 s Wartezeit, unverkürzt — sie ist keine Abklingzeit.
    expect(combat.gcd.remainingMs(0)).toBe(1500);
  });
});

describe('Kein Zauber bekommt negative Restzeit (AC2)', () => {
  it('lässt einen bereiten Zauber unberührt', () => {
    const combat = pruefKampf();
    const a = combat.indexOf('lang-a');
    // A war nie gewirkt, ist also bereit.
    combat.cast(combat.indexOf('kurz'), 0);
    expect(combat.cooldownRemainingMs(a, 0)).toBe(0);
    expect(combat.isOffCooldown(a, 0)).toBe(true);
    expect(combat.sublimation.shortenedCount).toBe(0);
  });

  it('begrenzt auf den Jetzt-Zeitpunkt statt einfach abzuziehen', () => {
    const combat = pruefKampf();
    const a = combat.indexOf('lang-a');
    combat.cast(a, 0);
    // Bei 9000 ms läuft A noch 1000 ms — die Verkürzung ist doppelt so groß.
    combat.cast(combat.indexOf('kurz'), 9000);
    expect(combat.cooldownRemainingMs(a, 9000)).toBe(0);
    expect(combat.isOffCooldown(a, 9000)).toBe(true);
    // Und eine Millisekunde später ebenfalls nicht negativ.
    expect(combat.cooldownRemainingMs(a, 9001)).toBe(0);
  });

  it('macht aus einer knappen Restzeit keine Zeitreise', () => {
    const combat = pruefKampf();
    const a = combat.indexOf('lang-a');
    combat.cast(a, 0);
    for (let t = 1500; t <= 12_000; t += 1500) {
      if (combat.isOffCooldown(combat.indexOf('kurz'), t)) combat.cast(combat.indexOf('kurz'), t);
      expect(combat.cooldownRemainingMs(a, t)).toBeGreaterThanOrEqual(0);
    }
  });
});

describe('Die Referenz bleibt unberührt (AC3)', () => {
  it('verkürzt nichts, solange die Referenz-Rotation keinen Verkürzer wirkt', () => {
    // **Ersetzt, nicht gelöscht** (Issue #201). Vorher hieß dieser Fall
    // „verkürzt nichts, wenn kein Zauber es kann" und sicherte zu, dass KEIN
    // Zauber das Feld führt. Seit VAPOUR führt einer es — die alte Zusicherung
    // wäre jetzt schlicht falsch.
    //
    // Die Aussage, auf die es ankommt, ist eine andere und bleibt gültig: Die
    // Referenz-Rechnung bei 4240 hängt daran, dass die Rotation aus Schritt 1
    // **keinen** Verkürzer wirkt. Ob es einen gibt, ist dafür gleichgültig.
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });

    // Es GIBT welche — sonst prüfte der Fall unten eine leere Menge.
    //
    // **Nicht die vollständige Liste festnageln.** Sie wächst mit jedem
    // neuen Verkürzer (Issue #480 brachte WISP), und die Zusicherung hier
    // meint „es gibt mindestens einen", nicht „es gibt genau diese".
    // Ein festgenagelter Name macht den Fall rot, ohne dass etwas kaputt ist.
    const mitVerkuerzung = combat.spells.filter((spell) => (spell.cooldownReductionMs ?? 0) > 0);
    expect(mitVerkuerzung.length, 'kein Verkürzer — der Fall unten misst nichts').toBeGreaterThan(
      0,
    );
    expect(mitVerkuerzung.map((spell) => spell.id)).toContain('vapour');

    const kindle = combat.indexOf('kindle');
    for (let t = 0; t <= 20_000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.gcd.isReady(t) && combat.isReady(kindle, t)) combat.cast(kindle, t);
    }
    expect(combat.sublimation.applicationCount).toBe(0);
    expect(combat.sublimation.shortenedCount).toBe(0);
  });

  it('verkürzt sehr wohl, sobald der Verkürzer gewirkt wird — der Gegenfall', () => {
    // Ohne ihn bliebe der Fall darüber auch dann grün, wenn die Verkürzung
    // überhaupt nicht mehr griffe. Er misst dann nur noch, dass niemand sie
    // auslöst.
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    const vapour = combat.indexOf('vapour');
    const pyre = combat.indexOf('pyre');
    expect(vapour).toBeGreaterThanOrEqual(0);

    // **Nach der globalen Wartezeit**, sonst wird der zweite Druck abgelehnt
    // und der Fall misst nichts. Mein erster Entwurf wirkte bei 1000 ms und
    // bekam applicationCount 0 — nicht weil die Verkuerzung fehlte, sondern
    // weil der Zauber gar nicht gewirkt wurde.
    combat.cast(pyre, 0);
    const t2 = GCD_MS;
    const ohne = combat.cooldownRemainingMs(pyre, t2);
    expect(combat.cast(vapour, t2).result).toBe(0);
    const mit = combat.cooldownRemainingMs(pyre, t2);

    expect(combat.sublimation.applicationCount).toBe(1);
    expect(ohne - mit).toBe(combat.spells[vapour]?.cooldownReductionMs);
  });
});
