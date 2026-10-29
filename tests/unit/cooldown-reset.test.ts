import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { FIRST_CRUCIBLE, parseSchool } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { TICK_MS } from '../../src/core/loop.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';

/**
 * Eine Prüf-Schule mit einem Rücksetzer.
 *
 * **Sie wird nicht ausgeliefert.** SUBLIMATION hat im Design noch keine
 * Zauber; die zwei offenen Zahlen des Rücksetzers — eigene Abklingzeit und
 * auf welche Zauber anwendbar — sind noch nicht festgelegt (Issue #102).
 * Geprüft wird deshalb die MECHANIK, nicht ein Inhalt, den es nicht gibt.
 */
function pruefSchule(): ReturnType<typeof parseSchool> {
  return parseSchool({
    schema: 1,
    school: 'CALCINATION',
    resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
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
        id: 'kurz',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Kurz',
        cooldownMs: 1500,
        baseDamage: 10,
      },
      {
        id: 'reset',
        unlockLevel: 1,
        castTimeMs: 0,
        triggersGcd: true,
        name: 'Reset',
        cooldownMs: 60_000,
        baseDamage: 0,
        cooldownResetCharges: 1,
      },
    ],
  });
}

interface Harness {
  readonly session: Session;
  readonly combat: Combat;
  readonly queue: PressQueue;
  readonly druecke: (slot: number, timeMs: number) => void;
}

function setup(): Harness {
  const school = pruefSchule();
  expect(school.problems).toEqual([]);
  const queue = new PressQueue();
  const combat = new Combat({ school: school.value, boss: FIRST_CRUCIBLE.value });
  const session = new Session(queue, combat);
  session.bind(0, combat.indexOf('lang'));
  session.bind(1, combat.indexOf('kurz'));
  session.bind(2, combat.indexOf('reset'));
  session.startFight(0);
  return {
    session,
    combat,
    queue,
    druecke: (slot, timeMs) => {
      queue.push(slot, timeMs / TICK_MS, 0, 0);
      session.tick(timeMs);
    },
  };
}

describe('Der Auswahlmodus (E3-3c)', () => {
  it('steht nach dem Rücksetzer scharf, vorher nicht', () => {
    const h = setup();
    expect(h.session.isResetArmed).toBe(false);
    h.druecke(2, 0);
    expect(h.session.isResetArmed).toBe(true);
    expect(h.session.resetArmedBy).toBe(h.combat.indexOf('reset'));
  });

  it('stellt sich erst NACH der eigenen Wirkung scharf', () => {
    // Sonst wählte der Druck des Rücksetzers bereits — und zwar sich selbst.
    const h = setup();
    h.druecke(2, 0);
    expect(h.combat.castCount(h.combat.indexOf('reset'))).toBe(1);
    expect(h.session.resetsAppliedCount).toBe(0);
    expect(h.session.resetsWastedCount).toBe(0);
  });
});

describe('Der gewählte Zauber ist bereit und ungewirkt (AC1)', () => {
  it('setzt die Abklingzeit auf null, ohne zu wirken', () => {
    const h = setup();
    const lang = h.combat.indexOf('lang');
    h.druecke(0, 0); // LANG wirken — 30 s Abklingzeit
    expect(h.combat.castCount(lang)).toBe(1);
    expect(h.combat.cooldownRemainingMs(lang, 1500)).toBe(28_500);

    h.druecke(2, 1500); // Rücksetzer
    h.druecke(0, 3000); // LANG wählen

    expect(h.combat.isOffCooldown(lang, 3000)).toBe(true);
    expect(h.combat.cooldownRemainingMs(lang, 3000)).toBe(0);
    // **Nicht gewirkt**: die Zahl der Wirkungen steht still.
    expect(h.combat.castCount(lang)).toBe(1);
    expect(h.session.resetsAppliedCount).toBe(1);
  });

  it('zählt die Auswahl nicht als Wirkung', () => {
    // Sonst stünde in der Auswertung ein Zauber, der nie gewirkt wurde.
    const h = setup();
    h.druecke(0, 0);
    h.druecke(2, 1500);
    const vorher = h.session.acceptedCount;
    h.druecke(0, 3000);
    expect(h.session.acceptedCount).toBe(vorher);
  });

  it('lässt den Modus danach los', () => {
    const h = setup();
    h.druecke(0, 0);
    h.druecke(2, 1500);
    h.druecke(0, 3000);
    expect(h.session.isResetArmed).toBe(false);

    // Der nächste Druck wirkt wieder ganz gewöhnlich.
    h.druecke(0, 4500);
    expect(h.combat.castCount(h.combat.indexOf('lang'))).toBe(2);
  });
});

describe('Er wählt nie sich selbst (AC2)', () => {
  it('verpufft bei der Selbstwahl, statt scharf zu bleiben', () => {
    // **Der erste Anlauf war anders herum** — hergeleitet aus GAME-DESIGN 8.2
    // („Fehleingabe wird verworfen, nicht gemerkt"). Die Herleitung war
    // schlüssig und trotzdem falsch: 8.2 meint eine Taste OHNE Zauber, also
    // eine Eingabe ins Leere. Eine Selbstwahl trifft einen Zauber, nur den
    // falschen — ein Spielfehler. Der Auftraggeber hat die Linie vorgegeben:
    // „pech für den user, muss er lernen besser zu spielen oder timen".
    const h = setup();
    const reset = h.combat.indexOf('reset');
    h.druecke(2, 0);
    expect(h.session.isResetArmed).toBe(true);

    h.druecke(2, 1500); // sich selbst wählen
    expect(h.session.isResetArmed).toBe(false);
    expect(h.session.resetsWastedCount).toBe(1);
    // Seine eigene Abklingzeit bleibt stehen — er wirkt nie auf sich.
    expect(h.combat.isOffCooldown(reset, 1500)).toBe(false);
    expect(h.combat.castCount(reset)).toBe(1);
  });

  it('lässt sich damit nicht endlos ketten', () => {
    // Der Grund für die Regel: Setzte er sich selbst zurück, wäre er der
    // erste Zug, den eine Optimums-Suche findet.
    const h = setup();
    const reset = h.combat.indexOf('reset');
    h.druecke(2, 0);
    for (let t = 1500; t <= 15_000; t += 1500) h.druecke(2, t);
    expect(h.combat.castCount(reset)).toBe(1);
    expect(h.combat.cooldownRemainingMs(reset, 15_000)).toBe(45_000);
  });
});

describe('Ein bereiter Zauber verbraucht ihn (AC3)', () => {
  it('verpufft und wirkt nicht', () => {
    const h = setup();
    const kurz = h.combat.indexOf('kurz');
    h.druecke(2, 0);
    // KURZ wurde nie gewirkt, liegt also nicht auf Abklingzeit.
    h.druecke(1, 1500);

    expect(h.session.resetsWastedCount).toBe(1);
    expect(h.session.resetsAppliedCount).toBe(0);
    expect(h.combat.castCount(kurz)).toBe(0);
    expect(h.session.isResetArmed).toBe(false);
  });

  it('kostet dasselbe wie eine gelungene Auswahl', () => {
    // „Zwei verlorene Aktionen" — der Fehlgriff darf nicht billiger sein als
    // der Treffer, sonst wäre Raten die bessere Strategie.
    const gelungen = setup();
    gelungen.druecke(0, 0);
    gelungen.druecke(2, 1500);
    gelungen.druecke(0, 3000);

    const verpufft = setup();
    verpufft.druecke(0, 0);
    verpufft.druecke(2, 1500);
    verpufft.druecke(1, 3000);

    expect(verpufft.combat.gcd.readyAt).toBe(gelungen.combat.gcd.readyAt);
  });
});

describe('Die globale Wartezeit (AC4)', () => {
  it('läuft für die Auswahl wie für eine Wirkung', () => {
    const h = setup();
    h.druecke(0, 0);
    h.druecke(2, 1500);
    h.druecke(0, 3000);
    // Die Auswahl bei 3000 sperrt bis 4500 — sonst fiele der Nachdruck in
    // denselben Takt, und aus drei Aktionen würden zwei.
    expect(h.combat.gcd.readyAt).toBe(3000 + GCD_MS);
    expect(h.combat.gcd.isReady(3000)).toBe(false);
  });

  it('erlaubt kein Doppel-Wirken im selben Takt', () => {
    const h = setup();
    h.druecke(0, 0);
    h.druecke(2, 1500);
    // Auswahl UND Nachdruck im selben Takt: Der zweite Druck muss abprallen.
    h.queue.push(0, 3000 / TICK_MS, 0, 0);
    h.queue.push(0, 3000 / TICK_MS, 0, 0);
    h.session.tick(3000);

    expect(h.session.resetsAppliedCount).toBe(1);
    // Nur die Wirkung von 0 ms steht — der Nachdruck kam nicht durch.
    expect(h.combat.castCount(h.combat.indexOf('lang'))).toBe(1);
  });

  it('setzt die Wartezeit nicht mit zurück', () => {
    const h = setup();
    h.druecke(0, 0);
    h.druecke(2, 1500);
    h.druecke(0, 3000);
    expect(h.combat.gcd.remainingMs(3000)).toBe(GCD_MS);
    expect(h.combat.gcd.remainingMs(3000)).toBeGreaterThan(0);
  });

  it('gibt den Zauber erst nach der Wartezeit frei', () => {
    // Drei Aktionen: Rücksetzer (0), Auswahl (1500), Nachdruck (3000).
    const h = setup();
    const lang = h.combat.indexOf('lang');
    h.druecke(0, 0);
    h.druecke(2, 1500);
    h.druecke(0, 3000);
    h.druecke(0, 4500);
    expect(h.combat.castCount(lang)).toBe(2);
  });
});

describe('Ohne Rücksetzer ändert sich nichts', () => {
  it('wirkt ganz gewöhnlich, solange der Modus nicht scharf steht', () => {
    const h = setup();
    h.druecke(0, 0);
    h.druecke(1, 1500);
    expect(h.combat.castCount(h.combat.indexOf('lang'))).toBe(1);
    expect(h.combat.castCount(h.combat.indexOf('kurz'))).toBe(1);
    expect(h.session.resetsAppliedCount).toBe(0);
    expect(h.session.resetsWastedCount).toBe(0);
  });

  it('setzt den Modus beim Zurücksetzen zurück', () => {
    const h = setup();
    h.druecke(2, 0);
    expect(h.session.isResetArmed).toBe(true);
    h.session.reset();
    expect(h.session.isResetArmed).toBe(false);
    expect(h.session.resetsAppliedCount).toBe(0);
  });
});

/**
 * ZWEI LADUNGEN — ALEMBIC (Issue #521).
 *
 * Dieselben Zusicherungen stehen im Simulator an `shorten`-Ebene
 * (`sim/src/cooldown_reset.rs`). Sie hier zu wiederholen ist kein Duplikat,
 * sondern der Sinn der Sache: Die Regel muss auf **beiden** Seiten gelten,
 * und eine Aufzeichnung allein zeigt nur, dass beide dasselbe TUN — nicht,
 * dass beide dasselbe SOLLEN.
 */
describe('ALEMBIC — zwei Ladungen (#521)', () => {
  function zweiLadungen(): ReturnType<typeof parseSchool> {
    return parseSchool({
      schema: 1,
      school: 'CALCINATION',
      resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
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
          id: 'kurz',
          unlockLevel: 1,
          castTimeMs: 0,
          triggersGcd: true,
          name: 'Kurz',
          cooldownMs: 9000,
          baseDamage: 10,
        },
        {
          id: 'reset',
          unlockLevel: 1,
          castTimeMs: 0,
          triggersGcd: true,
          name: 'Reset',
          cooldownMs: 60_000,
          baseDamage: 0,
          cooldownResetCharges: 2,
        },
      ],
    });
  }

  function aufbau(): { combat: Combat; lang: number; kurz: number; reset: number } {
    const school = zweiLadungen();
    expect(school.problems).toEqual([]);
    const combat = new Combat({ school: school.value, boss: FIRST_CRUCIBLE.value, seed: 0 });
    return {
      combat,
      lang: combat.indexOf('lang'),
      kurz: combat.indexOf('kurz'),
      reset: combat.indexOf('reset'),
    };
  }

  it('erlaubt ZWEI Auswahlen, und der Modus fällt erst nach der zweiten', () => {
    const { combat, lang, kurz, reset } = aufbau();
    combat.cast(alsSpellIndex(lang), 0);
    combat.cast(alsSpellIndex(kurz), GCD_MS);
    combat.cast(alsSpellIndex(reset), 2 * GCD_MS);
    combat.armResetIfNeeded(reset, GCD_MS);
    expect(combat.resetChargesLeft).toBe(2);

    expect(combat.chooseForReset(alsSpellIndex(lang), 3 * GCD_MS)).toBe(true);
    expect(combat.resetArmed, 'nach der ERSTEN muss er scharf bleiben').toBe(true);
    expect(combat.resetChargesLeft).toBe(1);

    expect(combat.chooseForReset(alsSpellIndex(kurz), 4 * GCD_MS)).toBe(true);
    expect(combat.resetArmed, 'nach der ZWEITEN fällt er').toBe(false);
    expect(combat.resetChargesLeft).toBe(0);
  });

  it('AC4 — auch die ZWEITE Auswahl trifft nie den Rücksetzer selbst', () => {
    // Der Index bleibt über beide Auswahlen stehen; genau dafür ist er ein
    // eigenes Feld neben dem Zähler. Wäre er nach der ersten gelöscht, träfe
    // die zweite den Rücksetzer und machte ihn sofort wieder bereit — eine
    // endlose Kette.
    const { combat, lang, reset } = aufbau();
    combat.cast(alsSpellIndex(lang), 0);
    combat.cast(alsSpellIndex(reset), GCD_MS);
    combat.armResetIfNeeded(reset, GCD_MS);

    expect(combat.chooseForReset(alsSpellIndex(lang), 2 * GCD_MS)).toBe(true);
    expect(
      combat.chooseForReset(alsSpellIndex(reset), 3 * GCD_MS),
      'die zweite Auswahl auf den Rücksetzer selbst muss verpuffen',
    ).toBe(false);
    expect(combat.cooldownRemainingMs(alsSpellIndex(reset), 3 * GCD_MS)).toBeGreaterThan(0);
  });

  it('AC3 — eine ungenutzte Ladung verfällt erst mit dem Kampf, nicht mit der Zeit', () => {
    // **Es gibt keinen Zeitverfall, und das ist die BESTEHENDE Regel** — nicht
    // eine neue für ALEMBIC. Schon vorher wurde der Auswahlmodus
    // ausschliesslich beim Kampfende gelöscht, nie durch Ablauf einer Frist.
    const { combat, lang, reset } = aufbau();
    combat.cast(alsSpellIndex(lang), 0);
    combat.cast(alsSpellIndex(reset), GCD_MS);
    combat.armResetIfNeeded(reset, GCD_MS);
    combat.chooseForReset(alsSpellIndex(lang), 2 * GCD_MS);

    // Beliebig viel später — es gibt keine Frist.
    combat.stepPassive(29_000);
    expect(combat.resetArmed, 'die zweite Ladung steht weiter scharf').toBe(true);
    expect(combat.resetChargesLeft).toBe(1);
  });

  it('AC1 — bei EINER Ladung verhält er sich exakt wie vorher', () => {
    // Die Gegenprobe zu AC1 in einem Satz: dieselbe Mechanik, Anzahl 1, und
    // der Modus fällt nach der ersten Auswahl. Die 15 Fälle darüber fahren
    // denselben Nachweis über die volle Sitzungs-Kette.
    const school = pruefSchule();
    const combat = new Combat({ school: school.value, boss: FIRST_CRUCIBLE.value, seed: 0 });
    const lang = combat.indexOf('lang');
    const reset = combat.indexOf('reset');
    combat.cast(alsSpellIndex(lang), 0);
    combat.cast(alsSpellIndex(reset), GCD_MS);
    combat.armResetIfNeeded(reset, GCD_MS);
    expect(combat.resetChargesLeft).toBe(1);
    expect(combat.chooseForReset(alsSpellIndex(lang), 2 * GCD_MS)).toBe(true);
    expect(combat.resetArmed).toBe(false);
  });
});
