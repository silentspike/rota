/**
 * Die letzten nackten Signaturen im Kern tragen die Marke (Issue #444).
 *
 * # Was hier geprüft wird — und was nicht
 *
 * Dass `CastOutcome.spellIndex` und `bookProgress` jetzt `SpellIndex` heissen,
 * belegt ein Blick in die Datei. Das ist keine Prüfung.
 *
 * **Was zählt, ist die Gegenprobe: Eine nackte Zahl muss den Übersetzer
 * stören.** Die Marke zu entfernen und danach null Fehler zu sehen beweist
 * dagegen nichts — genau dieser Fehler steht in VERIFICATION.md.
 *
 * # Warum `@ts-expect-error` und nicht `expect(...).toThrow()`
 *
 * Der Fehler entsteht beim Übersetzen, nicht beim Laufen: Zur Laufzeit ist ein
 * `SpellIndex` eine Zahl, und eine vertauschte oder erfundene Zahl liefe
 * klaglos durch. Ein Laufzeit-Prüffall könnte das gar nicht sehen.
 *
 * `@ts-expect-error` dreht die Richtung um: Die Zeile ist **rot, wenn der
 * Fehler ausbleibt**. Verschwindet die Marke wieder, meldet sich dieser Fall
 * — „unused '@ts-expect-error' directive".
 */
import { describe, expect, it } from 'vitest';
import { Combat, NO_SPELL, type CastOutcome } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('AC1/AC2 — `CastOutcome.spellIndex` nimmt keine nackte Zahl', () => {
  it('ein markierter Wert geht durch', () => {
    const combat = kampf();
    const ergebnis: CastOutcome = combat.cast(combat.indexOf('kindle'), 0);
    expect(ergebnis.spellIndex).toBe(combat.indexOf('kindle'));
  });

  it('GEGENPROBE: eine nackte Zahl stört den Übersetzer', () => {
    // **Ohne `as CastOutcome`**, und das ist der Punkt: Meine erste Fassung
    // hatte den Cast, und er schluckte den Fehler — die Direktive war unbenutzt
    // und der Prüffall belegte nichts. Genau die Falle, gegen die AC2
    // geschrieben ist.
    const roh: CastOutcome = {
      result: 0,
      // @ts-expect-error — eine nackte Zahl ist kein SpellIndex (Issue #444).
      // Diese Zeile ist ROT, wenn der Fehler ausbleibt: Verschwindet die Marke
      // wieder, meldet TypeScript die unbenutzte Direktive.
      spellIndex: 3,
      damage: 0,
      resourceBefore: 0,
      bonusApplied: false,
    };
    expect(roh.result).toBe(0);
  });

  it('und `NO_SPELL` ist der markierte Weg, „kein Zauber" zu sagen', () => {
    // Seit #444 liegt er in `combat.ts` — vorher in `session.ts`, wo
    // `combat.ts` ihn wegen des Zirkelbezugs nicht holen konnte.
    const ergebnis: CastOutcome = kampf().cast(NO_SPELL, 0);
    expect(ergebnis.spellIndex).toBe(NO_SPELL);
  });
});

describe('`NO_SPELL` bleibt über beide Wege erreichbar', () => {
  it('aus `combat.ts`, wo er jetzt steht', () => {
    expect(NO_SPELL).toBe(-1);
  });

  it('und aus `session.ts`, wo er stand', async () => {
    // **Der Re-Export ist kein Beiwerk.** Ihn wegzulassen hiesse, fünfzehn
    // Aufrufer anzufassen für eine Änderung, die sie nichts angeht — und jede
    // dieser Änderungen wäre eine Gelegenheit, etwas anderes zu treffen.
    const session = await import('../../src/core/session.js');
    expect(session.NO_SPELL).toBe(NO_SPELL);
  });
});
