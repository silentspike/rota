import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { PressQueue } from '../../src/input/queue.js';
import { NO_SPELL, Session } from '../../src/core/session.js';

/**
 * Platz und Zauber-Index lassen sich bei `bind` nicht mehr vertauschen
 * (Issue #442).
 *
 * # Warum das ein eigener Prüffall ist und nicht „der Typ ist doch da"
 *
 * `bind(slot: number, spellIndex: SpellIndex)` trägt die Marke — das allein
 * belegt nichts. Es wäre auch dann erfüllt, wenn jeder Aufrufer sofort
 * `alsSpellIndex(...)` schriebe: Dann nähme die Signatur zwar einen markierten
 * Wert, aber jede Vertauschung ginge weiter durch, nur mit einer Zeile mehr.
 *
 * Was zählt, ist die **Gegenprobe**: Vertauschte Argumente müssen den
 * Übersetzer stören. Genau das hält `@ts-expect-error` unten fest.
 *
 * # Warum `@ts-expect-error` und nicht `expect(...).toThrow()`
 *
 * Der Fehler entsteht beim Übersetzen, nicht beim Laufen — zur Laufzeit sind
 * beide Werte Zahlen, und `bind(index, slot)` liefe klaglos durch und bände den
 * falschen Platz. Ein Laufzeit-Prüffall könnte das gar nicht sehen.
 *
 * `@ts-expect-error` dreht die Richtung um: Die Zeile ist **rot, wenn der
 * Fehler ausbleibt**. Verschwindet die Marke wieder, meldet sich dieser Fall —
 * „unused '@ts-expect-error' directive".
 */

function bau(): { session: Session; combat: Combat } {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  return { session: new Session(new PressQueue(), combat), combat };
}

describe('AC2 — vertauschte Argumente stört der Übersetzer', () => {
  it('richtig herum geht durch', () => {
    const { session, combat } = bau();
    const index = combat.indexOf('kindle');

    session.bind(0, index);

    expect(session.spellOnSlot(0)).toBe(index);
  });

  it('vertauscht wird beim Übersetzen abgewiesen', () => {
    const { session, combat } = bau();
    const index = combat.indexOf('kindle');
    const slot = 0;

    // @ts-expect-error Platz und Zauber-Index vertauscht — genau das soll der
    // Übersetzer melden. Verschwindet die Marke von `bind`, wird diese Zeile
    // rot: Ein `@ts-expect-error` ohne Fehler ist selbst ein Fehler.
    session.bind(index, slot);

    // **Und zur Laufzeit passiert hier gar nichts Auffälliges** — das ist der
    // eigentliche Grund für die Marke. KINDLE hat den Index 0 und der Platz
    // ist 0, also bindet `bind(index, slot)` versehentlich genau dasselbe wie
    // `bind(slot, index)`. Die Vertauschung ist folgenlos, solange die Zahlen
    // zufällig übereinstimmen — und fällt erst auf, wenn sie es nicht mehr
    // tun. Kein Laufzeit-Prüffall der Welt fängt das zuverlässig.
    expect(index, 'KINDLE steht nicht mehr an Position 0 — dann trägt der Fall unten nicht').toBe(
      0,
    );
    expect(session.spellOnSlot(0)).toBe(0);
  });
});

describe('Und die Marke reicht bis in die leeren Plätze', () => {
  it('NO_SPELL ist selbst ein Zauber-Index', () => {
    // **Sonst müsste jeder Aufrufer von `bind(slot, NO_SPELL)` eine Umgehung
    // schreiben** — und eine Umgehung, die man überall wiederholt, hebt den
    // Sinn des Typs auf. NO_SPELL lebt im selben Wertebereich und trägt
    // deshalb dieselbe Marke.
    const { session, combat } = bau();
    session.bind(0, combat.indexOf('kindle'));
    session.bind(0, NO_SPELL);

    expect(session.spellOnSlot(0)).toBe(NO_SPELL);
    expect(session.boundSlotCount()).toBe(0);
  });
});
