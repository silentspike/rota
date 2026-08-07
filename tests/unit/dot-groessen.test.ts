import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Die GRÖSSEN der Brände — Issue #592, die vier teuersten Lücken aus #547.
 *
 * # Warum es diese Datei braucht
 *
 * `dot.tickIntervalMs` bei SEARING bewegt **72,58 %** des Schadens einer
 * Aufzeichnung (gemessen, `docs/groessen-hebel-messung.txt`). Bis heute ließ
 * sich der Wert halbieren, ohne dass eine Prüfung anschlug — außer der
 * Frischeprüfung, und die verstummt nach einem `npm run parity:export`.
 *
 * Das ist der Kern des Befunds aus #547: Eine Aufzeichnung sichert die
 * **Bitgleichheit** — beide Seiten rechnen dasselbe. Ob sie das *Richtige*
 * rechnen, prüft sie nicht.
 *
 * # Die Zahlen stehen HART und werden nicht aus den Daten gelesen
 *
 * Ein Sollwert aus derselben Quelle wie der Istwert ist keiner: Läse dieser
 * Fall `searing.dot.tickDamage`, läse er bei der Gegenprobe die mutierte Zahl
 * mit und bliebe grün. Wer die Design-Werte absichtlich ändert, ändert diese
 * Zeilen mit — und genau dann fällt auf, dass eine Entscheidung getroffen
 * wurde.
 *
 * # Was die beiden Fälle je Zauber trennen
 *
 * Der Zwischenstand nagelt das **Intervall** fest, die Endsumme den
 * **Schaden je Tick**. Beide fangen auch die jeweils andere Mutation — das ist
 * kein Mangel, sondern Diagnose: Fällt nur der Zwischenstand, hat sich die
 * Taktung verschoben; fallen beide, die Höhe.
 */

const BOSS = FIRST_CRUCIBLE.value;

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: BOSS });
}

/** Wirkt den Brand bei 0 und lässt die Zeit bis `bisMs` laufen. */
function schadenNach(zauber: string, bisMs: number): number {
  const c = neu();
  c.cast(c.indexOf(zauber), 0);
  for (let t = 0; t <= bisMs; t += 10) c.stepPassive(t);
  return c.damageDealt;
}

describe('SEARING — 3 Ticks à 90 über 6 s', () => {
  /**
   * **Der Zwischenstand nagelt das Intervall fest.**
   *
   * Bei 2000 ms Abstand ist nach 2000 ms genau **ein** Tick gefallen. Wird das
   * Intervall auf 1000 ms halbiert, sind es zwei — der Fall wird rot, ohne
   * dass sich die Gesamtdauer ändert.
   */
  it('nach 2000 ms ist genau ein Tick gefallen', () => {
    expect(
      schadenNach('searing', 2000),
      'die Taktung hat sich verschoben — bei 2000 ms Abstand fällt bis dahin genau ein Tick',
    ).toBe(90);
  });

  /**
   * **Die Endsumme nagelt den Schaden je Tick fest.**
   *
   * 6000 ms Laufzeit, 2000 ms Abstand: drei Ticks à 90 = 270. Auf `tickDamage
   * 45` gesenkt wären es 135.
   */
  it('am Ende stehen 270 — drei Ticks, nicht mehr und nicht weniger', () => {
    expect(
      schadenNach('searing', 7000),
      'SEARING richtet über seine Laufzeit nicht 270 an (3 × 90)',
    ).toBe(270);
  });
});

describe('CINDER — 4 Ticks à 30 über 4 s', () => {
  /**
   * CINDER tickt **dichter** als SEARING: 1000 ms statt 2000, dafür nur 4 s
   * Laufzeit. Nach 1000 ms ist deshalb schon ein Tick gefallen — bei einem
   * Intervall von 2000 ms wäre es keiner.
   */
  it('nach 1000 ms ist bereits ein Tick gefallen', () => {
    expect(
      schadenNach('cinder', 1000),
      'CINDER tickt nicht mehr im Sekundentakt — das ist sein Unterschied zu SEARING',
    ).toBe(30);
  });

  it('am Ende stehen 120 — vier Ticks à 30', () => {
    expect(schadenNach('cinder', 5000), 'CINDER richtet über seine Laufzeit nicht 120 an').toBe(
      120,
    );
  });
});
