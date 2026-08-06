/**
 * ROT legt Zersetzungs-Stapel über die Zeit (Issue #498).
 *
 * # Warum das eine neue Mechanik ist
 *
 * Bis hierher entstanden Stapel **nur beim Wirken**, über `spell.decay.stacks`.
 * Wer die Leiste anderweitig braucht, verlor den Aufbau. ROT lässt ihn
 * nebenherlaufen.
 *
 * Der naheliegende Verdacht war, dass es dafür kein neues Feld braucht:
 * `DotDef` trägt bereits `resourceGainPerTick`, und `cinder` nutzt es für
 * Hitze. **Gemessen und widerlegt** — Stapel sind keine Schul-Ressource:
 *
 * | | |
 * | --- | --- |
 * | Ressource | Vorrat des **Spielers**, eine Zahl, verfällt als Ganzes |
 * | Zersetzungs-Stapel | liegen auf dem **Boss**, verfallen **einzeln** nach 12 s, eigene Obergrenze |
 *
 * Deshalb `decayPerTick` neben `resourceGainPerTick` und nicht darin.
 *
 * # AC3 IST DER TEURE TEIL, UND ER MISST EINZELN
 *
 * `decayExpiryMs` ist ein Feld fester Größe, und beim Verfall rücken die
 * Einträge nach vorn. Ein DoT mit 1000 ms Tick füllt es **viermal schneller**
 * als jeder bisherige Zauber — wenn das Nachrücken dabei danebengeht, sieht
 * man es an der Summe nicht. Geprüft wird deshalb der Ablauf **je Stapel**.
 */
import { describe, expect, it } from 'vitest';
import { Combat, DECAY_STACK_DURATION_MS, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * ROT ist ein PUTREFACTION-Zauber, hier steht aber CALCINATION — **kein
 * Versehen.** `src/core/data.ts` exportiert heute nur die eine Schule, und
 * `school` wählt die **Ressource**, nicht die Zauberliste: `indexOf('rot')`
 * liefert auch hier einen gültigen Platz (gemessen: 17).
 *
 * Das ist deshalb erwähnenswert, weil `indexOf` bei einem unbekannten Zauber
 * **`-1` zurückgibt statt zu werfen** (siehe `Combat.indexOf`, Kommentar zu
 * Issue #372). Ein Tippfehler im Namen liefe also durch, und `cast(-1)` täte
 * still nichts — der Fall wäre grün, ohne etwas zu prüfen. Die Gegenproben
 * unten fangen genau das ab.
 */
function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Bis `ms` laufen lassen, ohne etwas zu wirken. */
function laufe(combat: Combat, bisMs: number, abMs = 0): void {
  for (let t = abMs; t <= bisMs; t += 10) combat.stepPassive(t);
}

describe('AC1 — ein DoT legt bei jedem Tick einen Stapel', () => {
  it('vier Ticks, vier Stapel — an der Stapelzahl, nicht am Feld', () => {
    const combat = kampf();
    combat.cast(combat.indexOf('rot'), 0);

    // ROT: 1000 ms Tick, 4000 ms Laufzeit. Der erste Stapel fällt beim ERSTEN
    // Tick, nicht beim Wirken — deshalb ist es bei 500 ms noch keiner.
    laufe(combat, 500);
    expect(combat.decayStacks, 'vor dem ersten Tick liegt nichts').toBe(0);

    for (const [ms, erwartet] of [
      [1000, 1],
      [2000, 2],
      [3000, 3],
      [4000, 4],
    ] as const) {
      laufe(combat, ms);
      expect(combat.decayStacks, `bei ${ms} ms`).toBe(erwartet);
    }
  });

  it('und danach hört es auf — die Laufzeit begrenzt, nicht der Zufall', () => {
    const combat = kampf();
    combat.cast(combat.indexOf('rot'), 0);
    laufe(combat, 8000);
    // 4000 ms Laufzeit bei 1000 ms Tick sind genau vier. Ein fünfter hiesse,
    // dass der DoT über seine Dauer hinaus tickt.
    expect(combat.decayStacks).toBe(4);
  });

  it('GEGENPROBE: ein DoT OHNE das Feld legt keine Stapel', () => {
    // **Ohne diese Zeile wäre nicht zu sehen, ob `decayPerTick` wirkt oder ob
    // jeder DoT Stapel legt.** `smoulder` ist derselbe Bautyp — Brand mit
    // Ticks — und trägt das Feld nicht.
    const combat = kampf();
    combat.cast(combat.indexOf('smoulder'), 0);
    laufe(combat, 8000);
    expect(combat.decayStacks).toBe(0);
  });
});

describe('AC2 — die Obergrenze gilt weiterhin', () => {
  /**
   * Höchstens `hoechstensDruecke` mal CANKER drücken, dabei ROT ticken lassen,
   * und den höchsten je gesehenen Stand zurückgeben.
   */
  function laufMitDruecken(mitRot: boolean, hoechstensDruecke: number): number {
    const combat = kampf();
    const canker = combat.indexOf('canker');
    if (mitRot) combat.cast(combat.indexOf('rot'), 0);

    let druecke = 0;
    let hoechster = 0;
    for (let t = 0; t <= 10_000; t += 10) {
      combat.stepPassive(t);
      if (druecke < hoechstensDruecke && combat.isOffCooldown(canker, t)) {
        if (combat.cast(canker, t).result === 0) druecke++;
      }
      expect(
        combat.decayStacks,
        `bei ${t} ms liegen mehr als ${MAX_DECAY_STACKS} Stapel`,
      ).toBeLessThanOrEqual(MAX_DECAY_STACKS);
      hoechster = Math.max(hoechster, combat.decayStacks);
    }
    return hoechster;
  }

  it('auch wenn Ticks und Tastendrücke zusammenkommen', () => {
    // **Die Zahl der Drücke ist gedeckelt, und das ist der ganze Punkt.**
    // Ohne den Deckel drückt man CANKER, sooft er bereit ist, und erreicht die
    // Obergrenze allein damit — der Fall wäre grün, ohne dass ROT darin
    // irgendetwas beiträgt. **Gemessen: genau das war er.**
    expect(laufMitDruecken(true, 2)).toBe(MAX_DECAY_STACKS);
  });

  it('GEGENPROBE: dieselben zwei Drücke ohne ROT erreichen sie nicht', () => {
    // Der Beleg, dass oben die Ticks den Ausschlag geben und nicht die
    // Tastendrücke. Zwei Drücke sind zwei Stapel; erst ROTs vier Ticks bringen
    // die Summe über die Grenze.
    expect(laufMitDruecken(false, 2)).toBeLessThan(MAX_DECAY_STACKS);
  });
});

describe('AC3 — die Verfallszeit je Stapel bleibt richtig', () => {
  it('ein durch einen TICK gelegter Stapel hält dieselben 12 s wie ein gedrückter', () => {
    // **Einzeln gemessen, nicht an der Summe.** `decayExpiryAt(i)` nennt den
    // Ablauf des i-ten Stapels; die Summe verschwiege ein falsches
    // Nachrücken.
    const combat = kampf();
    combat.cast(combat.indexOf('rot'), 0);
    laufe(combat, 4000);
    expect(combat.decayStacks).toBe(4);

    // Vier Ticks bei 1000, 2000, 3000, 4000 ms — also vier Abläufe im selben
    // Abstand, jeder 12 s nach seinem Tick.
    for (let i = 0; i < 4; i++) {
      expect(combat.decayExpiryAt(i), `Stapel ${i}`).toBe((i + 1) * 1000 + DECAY_STACK_DURATION_MS);
    }
  });

  it('und sie verfallen EINZELN, in der Reihenfolge ihres Entstehens', () => {
    // Der Kern von AC3: Beim Ablauf des ersten rücken die übrigen nach vorn.
    // Geht das daneben, stimmt die Zahl trotzdem — und die Abläufe nicht.
    const combat = kampf();
    combat.cast(combat.indexOf('rot'), 0);
    laufe(combat, 4000);

    // Kurz nach dem Ablauf des ersten Stapels (Tick bei 1000 + 12 000).
    laufe(combat, 13_100, 4010);
    expect(combat.decayStacks, 'einer ist weg').toBe(3);
    // Und die drei übrigen tragen jetzt die Abläufe der Stapel 2, 3, 4 —
    // nach vorn gerückt, nicht durcheinander.
    for (let i = 0; i < 3; i++) {
      expect(combat.decayExpiryAt(i), `nachgerückter Stapel ${i}`).toBe(
        (i + 2) * 1000 + DECAY_STACK_DURATION_MS,
      );
    }
  });

  it('und die Verwundbarkeit sinkt mit ihnen', () => {
    // **Nicht selbstverständlich.** Sie könnte beim Legen eingefroren werden;
    // gemessen wird sie laufend neu gerechnet. Ein langer DoT rechnet also
    // mit sinkendem Bonus, während er läuft — das ist Spielmechanik.
    const combat = kampf();
    combat.cast(combat.indexOf('rot'), 0);
    laufe(combat, 4000);
    const voll = combat.vulnerability;
    laufe(combat, 13_100, 4010);
    expect(combat.vulnerability).toBeLessThan(voll);
    expect(combat.vulnerability).toBeGreaterThan(1);
  });
});
