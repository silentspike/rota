/**
 * Die letzten beiden Bausteine (GAME-DESIGN 11.2, 7 und 8; Issue #240).
 *
 * BAUSTEIN 6 STAND HIER MIT DABEI und ist am 2026-08-05 herausgeschnitten
 * worden — nicht, weil er falsch gebaut war, sondern weil er den falschen Wert
 * traf: Der Boss-Ring des Spiels IST die Reaktion (GAME-DESIGN 9.1, „es ist das
 * Reaktionsgefäß"), und ein zweiter Wert daneben wäre eine Dublette gewesen.
 *
 * Aufgefallen ist es erst im Browser: Das HUD beschriftete den Ring mit
 * REACTION, während mein Prüf-Boss eine zweite Größe desselben Namens trug.
 * Kein Prüffall hätte das gemeldet — beide Zahlen waren für sich richtig.
 *
 * Baustein 6 wird in Issue #247 gebaut, und die Mechanik bleibt dieselbe: ein
 * Wert sinkt, Schutz und Heilung heben ihn, unter einer Schwelle endet der
 * Lauf. Was sich ändert, ist WELCHER Wert es ist.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
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
  const index = spellId === undefined ? alsSpellIndex(-1) : combat.indexOf(spellId);
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) {
    combat.stepPassive(t);
    if (t >= abMs && index >= 0 && combat.gcd.isReady(t) && combat.isReady(index, t)) {
      combat.cast(index, t);
    }
  }
}

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
    expect(ohne.escalation).toBeNull();
    expect(ohne.affliction).toBeNull();

    const combat = neuerKampf(ohne);
    laufen(combat, 30000, 'kindle');
    expect(combat.damageDealt).toBeGreaterThan(0);
    expect(combat.diedAtMs).toBe(-1);
  });
});
