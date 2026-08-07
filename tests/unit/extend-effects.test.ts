/**
 * HARDEN und SEPSIS — einen laufenden Ablauf verlängern (Issue #523).
 *
 * Dieselbe Bewegung wie EMBER (#517), an anderen Speichern:
 *
 * | | Speicher | Struktur |
 * | --- | --- | --- |
 * | HARDEN | `shieldExpiresAtMs` | eine Zahl |
 * | SEPSIS | `decayExpiryMs` | ein Feld mit **Nachrücken** |
 *
 * # AC6 IST FÜR HARDEN NICHT ANWENDBAR — und das ist kein Versäumnis
 *
 * Das Kriterium verlangt zu messen, ob die Zauber in einer Rotation vorkommen
 * und was sie wert sind. Für SEPSIS ist das gefahren (er kostet 156 Schaden:
 * 1545 ohne ihn, 1389 mit).
 *
 * Für HARDEN geht es nicht: Der Balance-Läufer misst **ausgehenden** Schaden
 * und Siege. HARDEN wirkt auf **eingehenden** Schaden — sein Wert taucht dort
 * gar nicht auf, und eine Zahl daraus wäre keine Aussage über ihn.
 *
 * Dafür bräuchte es eine Messung über `damage_taken` oder überlebte Kämpfe.
 * Ob der Läufer das kann, ist **nicht geprüft** — das gehört zum Werkzeug und
 * nicht in dieses Vorhaben.
 *
 * Wie bei AC7 in #484 steht das hier statt als Haken: Ein Kriterium, das man
 * nicht erfüllen kann, wird sonst entweder abgehakt oder blockiert ein
 * fertiges Vorhaben.
 *
 * # Ein Werkzeug-Befund am Rande, der PUTREFACTION insgesamt betrifft
 *
 * Die Messrotation für SEPSIS musste **ohne Bedingung** auskommen: Der
 * APL-Parser kennt die Zersetzungs-Stapel nicht (möglich sind nur `heat`,
 * `resource`, `gcd`, `time`, `boss.*`, `cooldown.*`, `dot.*`). Eine
 * PUTREFACTION-Rotation kann damit nicht auf ihre eigene Kernmechanik
 * bedingen — gemessen wurde also im Blindflug: SEPSIS wird gedrückt, sobald er
 * bereit ist, statt wenn Stapel liegen.
 *
 * # Warum SEPSIS ALLE Stapel trifft, und um denselben Betrag
 *
 * `decayExpiryMs` ist aufsteigend sortiert, und das Nachrücken beim Verfall
 * setzt das voraus. Ein Zuschlag auf nur einen Eintrag stellte einen späteren
 * Ablauf vor einen früheren — dann verfallen Stapel in falscher Reihenfolge,
 * **ohne dass die Stapelzahl es verrät**.
 *
 * Genau das prüft der Ordnungs-Fall unten: nicht die Anzahl, sondern die
 * Reihenfolge der Abläufe.
 */
import { describe, expect, it } from 'vitest';
import { Combat, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Läuft bis `bisMs`, ohne zu wirken. */
function laufe(c: Combat, bisMs: number, abMs = 0): void {
  for (let t = abMs; t <= bisMs; t += 10) c.stepPassive(t);
}

describe('AC1 — HARDEN verlängert, ohne neu zu setzen', () => {
  it('der Ablauf rückt nach hinten', () => {
    const c = kampf();
    c.cast(c.indexOf('crust'), 0);
    const dauer = c.spellAt(c.indexOf('crust'))?.shield?.durationMs ?? 0;
    expect(dauer, 'CRUST hat keine Laufzeit — dann prüft der Fall nichts').toBeGreaterThan(0);

    // Kurz vor dem Ablauf verlängern, dann darüber hinaus laufen lassen.
    laufe(c, dauer - 500);
    c.extendShield(dauer - 500, dauer);
    laufe(c, dauer + 500, dauer - 490);
    expect(c.shield, 'die Schicht muss den alten Ablauf überlebt haben').toBeGreaterThan(0);
  });

  it('und der aufgebrauchte Betrag bleibt — kein zweiter CRUST', () => {
    // **Der Kern von AC1.** Wer den Schild neu setzte, füllte ihn auch wieder
    // auf. Verlängern lässt den Rest, wie er ist.
    const c = kampf();
    c.cast(c.indexOf('crust'), 0);
    const voll = c.shield;
    c.extendShield(0, 1000);
    expect(c.shield).toBe(voll);
  });
});

describe('AC2 — SEPSIS trifft ALLE Stapel und erhält ihre Ordnung', () => {
  /** Volle Stapelleiste über CANKER, mit verschieden alten Einträgen. */
  function mitStapeln(): Combat {
    const c = kampf();
    const canker = c.indexOf('canker');
    for (let t = 0; c.decayStacks < MAX_DECAY_STACKS && t < 60_000; t += 10) {
      c.stepPassive(t);
      if (c.gcd.isReady(t) && c.isOffCooldown(canker, t)) c.cast(canker, t);
    }
    return c;
  }

  it('jeder Ablauf rückt um denselben Betrag nach hinten', () => {
    const c = mitStapeln();
    const vorher = [...Array(c.decayStacks).keys()].map((i) => c.decayExpiryAt(i));
    c.extendDecayStacks(12_000);
    const nachher = [...Array(c.decayStacks).keys()].map((i) => c.decayExpiryAt(i));
    expect(nachher).toEqual(vorher.map((v) => v + 12_000));
  });

  it('DIE ORDNUNG BLEIBT — und das ist die eigentliche Aussage', () => {
    // Nicht die Anzahl, sondern die Reihenfolge. Bräche sie, verfielen die
    // Stapel in falscher Folge, und die Stapelzahl bliebe trotzdem richtig —
    // genau die Klasse, die der ROT-Prüffall aus #498 gefangen hat.
    const c = mitStapeln();
    c.extendDecayStacks(12_000);
    const ablaeufe = [...Array(c.decayStacks).keys()].map((i) => c.decayExpiryAt(i));
    for (let i = 1; i < ablaeufe.length; i++) {
      expect(ablaeufe[i], `Stapel ${i} läuft vor seinem Vorgänger ab`).toBeGreaterThanOrEqual(
        ablaeufe[i - 1] ?? 0,
      );
    }
  });
});

describe('AC4 und AC5 — ohne laufenden Effekt tut keiner etwas', () => {
  it('HARDEN ohne Schicht bricht nicht und belebt nichts', () => {
    const c = kampf();
    expect(() => c.extendShield(0, 5000)).not.toThrow();
    expect(c.shield).toBe(0);
  });

  it('ein ABGELAUFENER Schild wird nicht wiederbelebt', () => {
    // **Verlängern ist nicht Neustarten.** Ohne diese Zeile wäre HARDEN ein
    // verkappter CRUST, der aus dem Nichts eine Schicht erzeugt.
    const c = kampf();
    c.cast(c.indexOf('crust'), 0);
    const dauer = c.spellAt(c.indexOf('crust'))?.shield?.durationMs ?? 0;
    laufe(c, dauer + 1000);
    expect(c.shield, 'die Schicht muss abgelaufen sein').toBe(0);
    c.extendShield(dauer + 1000, 10_000);
    expect(c.shield).toBe(0);
  });

  it('SEPSIS ohne Stapel bricht nicht', () => {
    const c = kampf();
    expect(() => c.extendDecayStacks(12_000)).not.toThrow();
    expect(c.decayStacks).toBe(0);
  });
});
