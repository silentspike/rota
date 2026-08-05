/**
 * Die Belegung kommt im Kampf an (Issue #264, AC1 und AC5).
 *
 * AC5 IST DER FALL, DER STILL BRICHT
 * ----------------------------------
 * Wer die Belegung ändert und der Kampf wirkt weiter den alten Zauber, merkt es
 * erst im Kampf — und hält es dann für einen Fehler in seiner Rotation. Nichts
 * stürzt ab, nichts wird rot; die Tasten tun nur etwas anderes, als dasteht.
 *
 * Der Prüffall unten misst deshalb nicht, dass `bind` aufgerufen wird, sondern
 * **was nach dem Drücken passiert**: Welcher Zauber wurde gewirkt?
 *
 * WARUM ÜBER `Session` UND NICHT ÜBER `App`
 * -----------------------------------------
 * `App` braucht ein Dokument — die Prüffälle laufen ohne DOM. Was sich hier
 * messen lässt, ist die Kette von der Belegung zur gewirkten Wirkung; dass die
 * App sie beim Start herstellt, zeigt der Browser (AC1).
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { COUNTDOWN_MS, PHASE_FIGHTING } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { SLOT_COUNT } from '../../src/input/keys.js';
import { PressQueue } from '../../src/input/queue.js';
import { bindSpell, defaultBindings, spellAtSlot } from '../../src/progression/bindings.js';

const SPELLS = CALCINATION.value.spells;

/** Eine Belegung auf eine Sitzung übertragen — wie `App.applyBindings`. */
function uebertragen(
  session: Session,
  combat: Combat,
  bindings: Readonly<Record<string, string>>,
): void {
  for (let slot = 0; slot < SLOT_COUNT; slot++) {
    const spellId = spellAtSlot(bindings, slot);
    session.bind(slot, spellId === null ? -1 : combat.indexOf(spellId));
  }
}

/** Einen Platz drücken und sagen, welcher Zauber dabei gewirkt wurde. */
function druecken(
  session: Session,
  combat: Combat,
  queue: PressQueue,
  slot: number,
): string | null {
  const vorher = combat.spells.map((_, i) => combat.castCount(i));
  let tick = 0;
  session.beginCountdown(0);
  for (let t = 0; t <= COUNTDOWN_MS + 3000; t += 10) {
    if (t === COUNTDOWN_MS + 100) queue.push(slot, tick, t, tick);
    session.tick(t);
    tick++;
  }
  expect(session.phase, 'der Kampf muss laufen, sonst misst der Fall nichts').toBe(PHASE_FIGHTING);
  for (let i = 0; i < combat.spells.length; i++) {
    if (combat.castCount(i) > (vorher[i] ?? 0)) return combat.spells[i]?.id ?? null;
  }
  return null;
}

function aufbau(): { session: Session; combat: Combat; queue: PressQueue } {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  return { session: new Session(queue, combat), combat, queue };
}

describe('AC5 — der Kampf wirkt den Zauber, der JETZT auf dem Platz liegt', () => {
  it('wirkt nach einer Änderung den neuen Zauber, nicht den alten', () => {
    // **Der Kern von AC5.** Zwei Belegungen, derselbe Tastendruck, zwei
    // verschiedene Zauber — gemessen an dem, was gewirkt wurde, nicht daran,
    // dass eine Zuweisung stattfand.
    const alt = aufbau();
    uebertragen(alt.session, alt.combat, { '0': 'kindle' });
    expect(druecken(alt.session, alt.combat, alt.queue, 0)).toBe('kindle');

    const neu = aufbau();
    uebertragen(neu.session, neu.combat, { '0': 'smoulder' });
    expect(druecken(neu.session, neu.combat, neu.queue, 0)).toBe('smoulder');
  });

  it('wirkt nichts auf einem Platz, der geräumt wurde', () => {
    // **Der Fall, der still bricht.** Wer nur die belegten Plätze überträgt,
    // lässt einen geräumten auf seinem alten Zauber stehen — und der Spieler
    // wundert sich im Kampf.
    const { session, combat, queue } = aufbau();
    uebertragen(session, combat, { '0': 'kindle' });
    uebertragen(session, combat, {});
    expect(druecken(session, combat, queue, 0)).toBeNull();
  });

  it('lässt die anderen Plätze unberührt, wenn einer sich ändert', () => {
    const { session, combat, queue } = aufbau();
    let belegung = bindSpell({}, 'kindle', 0, 2, SPELLS);
    belegung = bindSpell(belegung, 'smoulder', 1, 2, SPELLS);
    uebertragen(session, combat, belegung);

    // Platz 0 wird ausgetauscht, Platz 1 bleibt.
    const geaendert = bindSpell(belegung, 'pyre', 0, 2, SPELLS);
    uebertragen(session, combat, geaendert);

    expect(spellAtSlot(geaendert, 1)).toBe('smoulder');
    expect(druecken(session, combat, queue, 1)).toBe('smoulder');
  });
});

describe('AC1 — die Startbelegung greift, solange der Spieler nichts geändert hat', () => {
  it('legt die vorgeschlagene Belegung auf die Leiste', () => {
    const { session, combat, queue } = aufbau();
    const vorschlag = defaultBindings(1, SPELLS);
    uebertragen(session, combat, vorschlag);

    const erster = spellAtSlot(vorschlag, 0);
    expect(erster).not.toBeNull();
    expect(druecken(session, combat, queue, 0)).toBe(erster);
  });

  it('bindet auch die Plätze, die der Vorschlag nicht füllt', () => {
    // Sonst bliebe dort stehen, was eine frühere Belegung gesetzt hat — und
    // auf Stufe 1 gibt es nur zwei Plätze, die übrigen müssen leer sein.
    const { session, combat, queue } = aufbau();
    uebertragen(session, combat, { '5': 'kindle' });
    uebertragen(session, combat, defaultBindings(1, SPELLS));
    expect(druecken(session, combat, queue, 5)).toBeNull();
  });
});
