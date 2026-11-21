/**
 * `Perpetual Draft` — der Rücksetzer bekommt eine Ladung mehr (13.5, #480).
 *
 * Der fünfte der sechs Rotations-Effekte (SUBLIMATION-Sechser).
 *
 * # Warum nicht das, was im Katalog stand
 *
 * Dort stand: „Der Rücksetzer wirkt auf die zwei längsten Abklingzeiten statt
 * auf eine. Macht lange Zauber planbar statt zufällig."
 *
 * Gemessen ist der Rücksetzer ein **Auswahlmodus**: Er wird scharf gestellt,
 * und der Spieler wählt den Zauber, dessen Abklingzeit fällt. Es gibt nichts
 * Zufälliges, das planbar werden könnte, und keine Auswahl „nach Länge" —
 * die Wahl nimmt entgegen, was gedrückt wurde. Eine automatische Wahl der zwei
 * längsten wäre kein Zusatz, sondern ein Umbau, und sie nähme dem Spieler die
 * Entscheidung weg, die 12.x als Kern von SUBLIMATION nennt.
 *
 * Entschieden (Auftraggeber, 2026-08-13): **+1 Ladung**, über denselben Weg
 * wie Zauber-Eigenschaft und Talent. Der Spieler wählt zweimal statt einmal —
 * „auf zwei statt auf eine" ist damit erfüllt, ohne die Wahl zu automatisieren.
 *
 * # Über `Session`, nicht über `Combat.cast`
 *
 * `armResetIfNeeded` wird von `Session.apply` gerufen, ausdrücklich NICHT von
 * `Combat.cast` (`session.ts`, Zeile 108). Ein Fall, der nur `cast` bemüht,
 * stellt nie scharf — und wird trotzdem grün, wenn er auf „keine Ladung offen"
 * prüft. Genau das ist mir hier zuerst passiert: ein Fall, der aus dem
 * falschen Grund bestand.
 *
 * # Eine eigene Prüfschule, kein ausgelieferter Inhalt
 *
 * Wie in `cooldown-reset.test.ts`: Geprüft wird die MECHANIK. Ein Fall, der an
 * `reflux` und `alembic` hängt, misst ab der nächsten Katalog-Änderung etwas
 * anderes als heute — und meldet das nicht.
 */

import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { FIRST_CRUCIBLE, parseSchool } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import {
  activeRotationEffects,
  PERPETUAL_DRAFT_EXTRA_CHARGE,
  SETS,
  SET_COUNT,
  SET_EFFECT_PERPETUAL_DRAFT,
  SET_EFFECT_WHOLE_WORK,
} from '../../src/gear/sets.js';

/** Der Satz mit `perpetual-draft` — abgelesen, nicht angenommen. */
const DRAFT_SET = SETS.find((s) =>
  s.tiers.some((t) => t.effect === SET_EFFECT_PERPETUAL_DRAFT),
)?.id;

/**
 * Drei Zauber: einer ohne Ladungen, einer mit einer, einer mit zweien.
 *
 * `zwei` trägt seine zwei aus der ZAUBER-Eigenschaft — daran zeigt sich, ob
 * der Satz-Effekt sich addiert oder die Eigenschaft ersetzt.
 */
function pruefSchule(): ReturnType<typeof parseSchool> {
  return parseSchool({
    schema: 1,
    school: 'CALCINATION',
    resource: {
      id: 'heat',
      name: 'Heat',
      cap: 20,
      decayPerStepMs: 2000,
      decayAmount: 1,
      highlightAt: 0,
    },
    spells: [
      {
        id: 'lang',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Lang',
        cooldownMs: 30_000,
        baseDamage: 50,
      },
      {
        id: 'eine',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Eine',
        cooldownMs: 60_000,
        baseDamage: 0,
        cooldownResetCharges: 1,
      },
      {
        id: 'zwei',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Zwei',
        cooldownMs: 60_000,
        baseDamage: 0,
        cooldownResetCharges: 2,
      },
    ],
  });
}

function teile(set: number, n: number): Int32Array {
  const pieces = new Int32Array(SET_COUNT);
  pieces[set] = n;
  return pieces;
}

interface Harness {
  readonly session: Session;
  readonly combat: Combat;
  /** Slot 0 `lang`, Slot 1 `eine`, Slot 2 `zwei`. */
  readonly druecke: (slot: number, timeMs: number) => void;
}

function setup(effekte: readonly string[]): Harness {
  const school = pruefSchule();
  expect(school.problems).toEqual([]);
  const queue = new PressQueue();
  const combat = new Combat({ school: school.value, boss: FIRST_CRUCIBLE.value });
  combat.setSetEffects(effekte);
  const session = new Session(queue, combat);
  session.bind(0, combat.indexOf('lang'));
  session.bind(1, combat.indexOf('eine'));
  session.bind(2, combat.indexOf('zwei'));
  session.startFight(0);
  return {
    session,
    combat,
    druecke: (slot, timeMs) => {
      queue.push(slot, timeMs / TICK_MS, 0, 0);
      session.tick(timeMs);
    },
  };
}

/** Wie viele Auswahlen nach dem Druck auf `slot` offen sind. */
function ladungenNach(effekte: readonly string[], slot: number): number {
  const h = setup(effekte);
  h.druecke(slot, 0);
  return h.combat.resetChargesLeft;
}

describe('AC1 — sechs Satzteile lösen ihn aus, fünf nicht', () => {
  it('die Schwelle greift, nicht die blosse Kennung', () => {
    expect(DRAFT_SET).toBeDefined();
    const id = DRAFT_SET as number;
    expect(activeRotationEffects(SETS, teile(id, 6))).toContain(SET_EFFECT_PERPETUAL_DRAFT);
    expect(activeRotationEffects(SETS, teile(id, 5))).not.toContain(SET_EFFECT_PERPETUAL_DRAFT);
  });
});

describe('AC2 — die Ladung kommt an', () => {
  it('mit Zahlen: 2 statt 1', () => {
    // Gegen die festen Zahlen, nicht gegen die Differenz zweier Läufe: Eine
    // Mutation, die beide Seiten gemeinsam verschiebt, bliebe sonst grün.
    expect(ladungenNach([], 1)).toBe(1);
    expect(ladungenNach([SET_EFFECT_PERPETUAL_DRAFT], 1)).toBe(2);
    expect(PERPETUAL_DRAFT_EXTRA_CHARGE).toBe(1);
  });

  it('und die zweite Wahl wird wirklich angenommen', () => {
    // Die Zahl allein wäre Buchhaltung. Hier zählt, dass der Auswahlmodus
    // nach der ersten Wahl noch steht — sonst wäre die Ladung eine Anzeige.
    const h = setup([SET_EFFECT_PERPETUAL_DRAFT]);
    h.druecke(1, 0);
    expect(h.session.isResetArmed).toBe(true);

    h.druecke(0, 100);
    expect(h.combat.resetChargesLeft).toBe(1);
    // NOCH scharf — das ist der Unterschied zum Zustand ohne den Effekt.
    expect(h.session.isResetArmed).toBe(true);

    h.druecke(2, 200);
    expect(h.combat.resetChargesLeft).toBe(0);
    expect(h.session.isResetArmed).toBe(false);
  });

  it('ohne den Effekt ist nach der ERSTEN Wahl Schluss', () => {
    // Der Bezugspunkt der Aussage oben.
    const h = setup([]);
    h.druecke(1, 0);
    expect(h.session.isResetArmed).toBe(true);
    h.druecke(0, 100);
    expect(h.combat.resetChargesLeft).toBe(0);
    expect(h.session.isResetArmed).toBe(false);
  });
});

describe('AC3 — eine Summe mit drei Speisungen, keine drei Rechnungen', () => {
  it('die Zauber-Eigenschaft wird ergänzt, nicht ersetzt', () => {
    // `zwei` trägt zwei Ladungen aus der Zauber-Eigenschaft. Kommt der
    // Satz-Effekt dazu, müssen es drei sein. Wären es zwei, liefe der Effekt
    // an der Eigenschaft vorbei; wären es vier, zählte er doppelt.
    expect(ladungenNach([SET_EFFECT_PERPETUAL_DRAFT], 2)).toBe(3);
  });

  it('ZUSICHERUNG: derselbe Zauber trägt ohne den Effekt genau zwei', () => {
    expect(ladungenNach([], 2)).toBe(2);
  });
});

describe('AC4 — er macht keinen Zauber zum Rücksetzer', () => {
  it('ein Zauber ohne Ladungen bleibt ohne Auswahlmodus', () => {
    // Die Fehlerklasse, vor der der Kommentar in `armResetIfNeeded` warnt:
    // Zöge man den Zuschlag in die Prüfung `ladungen <= 0`, wäre sie nie wahr
    // und JEDER Wurf stellte den Auswahlmodus scharf.
    const h = setup([SET_EFFECT_PERPETUAL_DRAFT]);
    h.druecke(0, 0);
    expect(h.session.isResetArmed).toBe(false);
    expect(h.combat.resetChargesLeft).toBe(0);
  });
});

describe('AC5 — ein fremder Effekt löst ihn nicht aus', () => {
  it('The Whole Work im Aufbau legt keine Ladung zu', () => {
    expect(ladungenNach([SET_EFFECT_WHOLE_WORK], 1)).toBe(1);
  });

  it('und eine erfundene Kennung ebenso wenig', () => {
    expect(ladungenNach(['gibt-es-nicht'], 1)).toBe(1);
  });
});
