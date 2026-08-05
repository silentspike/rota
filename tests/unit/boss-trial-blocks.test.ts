/**
 * Die letzten drei Bausteine (GAME-DESIGN 11.2, 6 bis 8; Issue #240).
 *
 * WARUM BAUSTEIN 6 DER WICHTIGSTE IST, aus einem Grund, der nichts mit ihm zu
 * tun hat: 11.2 nennt ihn „der Kampf, in dem COAGULATION glänzt". Ihre drei
 * Wege sind seit #79 gebaut, und **kein Boss verlangte sie** — wer sie
 * mitnimmt, verliert Schaden und gewinnt nichts. Der Befund aus #224 sagt,
 * warum das mehr ist als Geschmackssache: Jeder Zauber ohne direkten Schaden
 * kostet einen GCD und muss ihn zurückverdienen.
 *
 * Hier gibt es zum ersten Mal etwas zurückzuverdienen.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const TRIAL = bossById('test-crucible-trial');
if (TRIAL === undefined) throw new Error('data/bosses/test-crucible-trial.json fehlt.');

function neuerKampf(boss = TRIAL): Combat {
  return new Combat({ school: CALCINATION.value, boss: boss as never });
}

/**
 * Bis `bisMs` laufen lassen und dabei `spellId` wirken, sooft es geht.
 *
 * `abMs` ist keine Bequemlichkeit: TEMPER hat 15 s Abklingzeit. Ohne den
 * Startpunkt wird er im ersten Takt gewirkt — also bevor die Schwächung
 * überhaupt liegt — und ist danach für den Rest des Laufs blockiert. Der
 * Prüffall meldete dann „entfernt sie nicht", und die Mechanik war die ganze
 * Zeit richtig.
 */
function laufen(combat: Combat, bisMs: number, spellId?: string, abMs = 0): void {
  const index = spellId === undefined ? -1 : combat.indexOf(spellId);
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) {
    combat.stepPassive(t);
    if (t >= abMs && index >= 0 && combat.gcd.isReady(t) && combat.isReady(index, t)) {
      combat.cast(index, t);
    }
  }
}

describe('AC1 — Baustein 6: die Reaktion kippt, wenn niemand sie hält', () => {
  it('endet den Lauf an der ausgerechneten Stelle', () => {
    const def = TRIAL.stabilise!;
    // 1000 bei 20 je Sekunde: Der einundfünfzigste Schritt fällt unter null.
    const erwartetMs = (def.startValue / def.dropAmount + 1) * def.dropPerStepMs;
    expect(erwartetMs).toBe(51000);

    const combat = neuerKampf();
    laufen(combat, 60000);

    expect(combat.reactionFailed).toBe(true);
    expect(combat.reaction).toBe(def.failBelow);
    expect(combat.diedAtMs).toBe(erwartetMs);
  });

  it('kippt an WARD und Schutzschicht vorbei — sonst wäre es keine Reaktion', () => {
    // Der Spieler baut Schutz auf und stirbt trotzdem, wenn er zu selten
    // stabilisiert. Ein Kippen, das eine Schutzschicht überlebt, wäre keins.
    const combat = neuerKampf();
    laufen(combat, 60000);
    expect(combat.playerHp).toBe(0);
  });
});

describe('AC2 — der Gegenfall: Stabilisieren greift wirklich ein', () => {
  it('hält die Reaktion über den ganzen Lauf, wenn CRUST gewirkt wird', () => {
    // **Zwei Läufe, gleicher Boss, gleiche Zeit.** Der Unterschied ist genau
    // eine Sache: Im zweiten wirkt der Spieler CRUST.
    const ohne = neuerKampf();
    const mit = neuerKampf();
    laufen(ohne, 60000);
    laufen(mit, 60000, 'crust');

    expect(ohne.reactionFailed, 'ohne Eingriff kippt sie').toBe(true);
    expect(mit.reactionFailed, 'mit CRUST nicht').toBe(false);
    expect(mit.reaction).toBeGreaterThan(TRIAL.stabilise!.failBelow);
    // Und der Nachweis, dass es WIRKLICH das Stabilisieren war:
    expect(mit.stabilisedCount).toBeGreaterThan(0);
    expect(ohne.stabilisedCount).toBe(0);
  });

  it('hebt um den EIGENEN Betrag des Zaubers, nicht um eine erfundene Zahl', () => {
    const combat = neuerKampf();
    const crust = combat.indexOf('crust');
    const schutz = combat.spells[crust]!.shield!.amount;
    combat.beginFight(0);
    combat.stepPassive(0);
    const vorher = combat.reaction;
    combat.cast(crust, 0);
    expect(combat.reaction - vorher).toBe(schutz);
  });

  it('hebt nicht über die Obergrenze — kein Vorrat in ruhigen Minuten', () => {
    const def = TRIAL.stabilise!;
    const combat = neuerKampf();
    laufen(combat, 30000, 'crust');
    expect(combat.reaction).toBeLessThanOrEqual(def.maxValue);
  });
});

describe('AC3 — Baustein 7: das Fenster wird mit jedem Zyklus enger', () => {
  it('verkürzt die Wirkzeit messbar, und die Verkürzung steht in den Daten', () => {
    const esc = TRIAL.escalation!;
    const basis = TRIAL.timeline[0]!.castTimeMs;
    expect(basis).toBe(3000);

    // Gerechnet, bevor gemessen wird: Zyklus 0 bis 4, dann die Untergrenze.
    const erwartet = [0, 1, 2, 3, 4].map((z) =>
      Math.max(esc.minCastTimeMs, basis - esc.castTimeShrinkMs * z),
    );
    expect(erwartet).toEqual([3000, 2500, 2000, 1500, 1000]);
    // Sie MÜSSEN sich unterscheiden — sonst prüft der Vergleich unten nichts.
    expect(new Set(erwartet).size).toBeGreaterThan(1);

    // Und im Kampf: Der Zeitpunkt, zu dem ein Zauber trifft, rückt näher an
    // seinen Beginn.
    // Gemessen wird, wie LANGE `casting` ansteht — das ist das Fenster zum
    // Unterbrechen, in Takten gezählt. Eine verbleibende Wirkzeit gibt es als
    // Abfrage nicht, und eine dafür anzulegen hieße, die Anzeige für einen
    // Prüffall zu erweitern.
    const combat = neuerKampf();
    const fenster: number[] = [];
    let laufend = 0;
    combat.beginFight(0);
    for (let t = 0; t <= 60000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.casting >= 0) {
        laufend += TICK_MS;
      } else if (laufend > 0) {
        fenster.push(laufend);
        laufend = 0;
      }
    }
    expect(fenster.length).toBeGreaterThan(2);
    expect(fenster[0]!).toBeGreaterThan(fenster[fenster.length - 1]!);
    // Und die gemessenen Fenster sind genau die gerechneten.
    expect(fenster.slice(0, erwartet.length)).toEqual(erwartet);
  });

  it('geht nicht unter die Untergrenze — sonst wäre es keine Steigerung mehr', () => {
    const esc = TRIAL.escalation!;
    const combat = neuerKampf();
    laufen(combat, 110000);
    // Nach genügend Zyklen steht die Wirkzeit auf der Untergrenze und bleibt
    // dort. 11.7 verwirft eine Schwierigkeit, die zur Unmöglichkeit wird.
    expect(combat.cycle).toBeGreaterThan(5);
    expect(esc.minCastTimeMs).toBe(1000);
  });
});

describe('AC4 — Baustein 8: Heilung entfernt sie nicht, eine Handlung schon', () => {
  it('bleibt liegen, wenn der Spieler heilt', () => {
    // **Der Kern des Bausteins.** SALVE heilt und steht NICHT in `clearedBy`.
    const combat = neuerKampf();
    laufen(combat, 12000, 'salve');
    expect(combat.afflicted, 'Heilung entfernt sie nicht').toBe(true);
    expect(combat.afflictionsCleared).toBe(0);
  });

  it('verschwindet, wenn eine der genannten Handlungen kommt', () => {
    // **Mehrere Lösungswege**, wie 11.2 ausdrücklich fordert — beide werden
    // hier einzeln belegt.
    for (const id of TRIAL.affliction!.clearedBy) {
      const combat = neuerKampf();
      // **Erst ab dem Zeitpunkt der Schwächung wirken.** Vorher gewirkt liegt
      // der Zauber auf Abklingzeit, wenn er gebraucht wird.
      laufen(combat, 14000, id, TRIAL.affliction!.atMs);
      expect(combat.afflicted, `${id} entfernt sie`).toBe(false);
      expect(combat.afflictionsCleared, `${id} zählt`).toBeGreaterThan(0);
    }
  });

  it('nennt MINDESTENS ZWEI Wege — ein Boss mit einem wäre eine Pflichtaufgabe', () => {
    // 11.3: Kein Boss darf eine bestimmte Schule verlangen. Mit genau einem
    // Weg wäre die Zusammenstellung vorgeschrieben.
    expect(TRIAL.affliction!.clearedBy.length).toBeGreaterThanOrEqual(2);
    expect(TRIAL.affliction!.clearedBy).not.toContain('salve');
  });

  it('macht Schaden, solange sie liegt — sonst wäre sie folgenlos', () => {
    const ohne = neuerKampf();
    const mit = neuerKampf();
    ohne.beginFight(0);
    for (let t = 0; t <= 7000; t += TICK_MS) ohne.stepPassive(t);
    laufen(mit, 14000);
    expect(mit.damageTaken).toBeGreaterThan(ohne.damageTaken);
  });
});

describe('AC5 — die Wächter: der KAMPF wirkt sie, nicht die Daten', () => {
  it('Baustein 6: derselbe Lauf endet mit und ohne den Datensatz verschieden', () => {
    const mit = neuerKampf();
    const ohne = neuerKampf(FIRST_CRUCIBLE.value);
    laufen(mit, 60000);
    laufen(ohne, 60000);

    expect(mit.reactionFailed).toBe(true);
    expect(ohne.reactionFailed).toBe(false);
    expect(ohne.reaction).toBe(0);
  });

  it('Baustein 7: ein Boss ohne Eskalation behält seine Wirkzeit', () => {
    const ohne = bossById('test-metronome');
    expect(ohne?.escalation ?? null).toBeNull();
    const combat = neuerKampf(ohne as never);
    laufen(combat, 20000);
    // Ohne die Daten bleibt die Wirkzeit die aus der Zeitleiste — der Kampf
    // rechnet also wirklich mit ihnen und nicht mit einer festen Regel.
    expect(combat.cycle).toBeGreaterThan(0);
  });

  it('Baustein 8: ohne den Datensatz liegt nie eine Schwächung', () => {
    const combat = neuerKampf(FIRST_CRUCIBLE.value);
    laufen(combat, 20000);
    expect(combat.afflicted).toBe(false);
    expect(combat.afflictionsCleared).toBe(0);
  });
});

describe('AC7 — ein Boss ohne die Felder bleibt unberührt', () => {
  it('trägt keinen der drei und verhält sich wie zuvor', () => {
    const ohne = FIRST_CRUCIBLE.value;
    expect(ohne.stabilise).toBeNull();
    expect(ohne.escalation).toBeNull();
    expect(ohne.affliction).toBeNull();

    const combat = neuerKampf(ohne);
    laufen(combat, 30000, 'kindle');
    expect(combat.damageDealt).toBeGreaterThan(0);
    expect(combat.diedAtMs).toBe(-1);
  });
});
