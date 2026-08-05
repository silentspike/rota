import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { PressQueue } from '../../src/input/queue.js';
import { Session } from '../../src/core/session.js';

/**
 * Die Verlustquelle „nichts war bereit" ist erreichbar — mit einem langen
 * Cooldown (Issue #300, AC3).
 *
 * # Warum dieser Prüffall existiert und nicht ein Browser-Nachweis
 *
 * AC3 verlangte ursprünglich `browser`: einen Lauf mit REFLUX als **einzigem**
 * gebundenen Zauber. Genau dieser Zustand ist im Spiel heute nicht
 * herstellbar (Issue #306):
 *
 * ```
 * bindSlot(slot, null)   →  indexOf(null) ist -1, der Aufruf tut nichts
 * session.bind(slot, i)  →  setzt boundSpells[i] = 1 und löscht es NIE
 * ```
 *
 * Wer KINDLE einmal gebunden hat, bekommt ihn nicht mehr von der Leiste — und
 * solange er dort liegt, kann die Quelle nicht erscheinen. Ein Prüffall baut
 * die Sitzung dagegen frisch auf und bindet nur, was er messen will.
 *
 * # Was hier gemessen wird, und warum es der Kommentar nicht behaupten sollte
 *
 * `session.ts` trug die Aussage „REFLUX allein ergibt 1 Takt bereit gegen
 * 1851 Takte nicht bereit" als **Zahl in einem Kommentar** — abgeschrieben aus
 * einer fremden Messung, ohne dass irgendeine Datei sie nachrechnete. Eine
 * Behauptung in einem Kommentar, der gegen unbelegte Behauptungen geschrieben
 * ist.
 *
 * Jetzt rechnet dieser Fall sie nach.
 */

/** Ein Lauf über die volle Kampfdauer, mit genau den genannten Zaubern. */
function laufMit(spellIds: readonly string[]): {
  nothingReady: number;
  withReady: number;
  elapsed: number;
} {
  const boss = FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const queue = new PressQueue();
  const session = new Session(queue, combat);
  // **Nur diese.** Der Prüffall baut die Sitzung frisch auf; `boundSpells`
  // trägt danach genau die genannten Zauber und keinen weiteren.
  spellIds.forEach((id, slot) => session.bind(slot, combat.indexOf(id)));

  session.beginCountdown(0);
  const ende = COUNTDOWN_MS + boss.overpressureMs;
  let tick = 0;
  for (let t = 0; t <= ende; t += 10) {
    // **Einmal wirken, ganz am Anfang.** Ohne Druck läuft die Abklingzeit nie
    // an, und der Zauber gilt den ganzen Kampf über als bereit — dann steht
    // alles unter „einer war bereit", auch bei 45 s Cooldown. Der erste
    // Entwurf dieses Falls hat genau daran 0 gemessen.
    if (t === COUNTDOWN_MS) queue.push(0, tick, t, tick);
    session.tick(t);
    tick++;
  }

  const s = session.summary(ende);
  return {
    nothingReady: s.lostTimeNothingReadyMs,
    withReady: s.lostTimeWithReadyMs,
    elapsed: s.elapsedMs,
  };
}

describe('Die Leerlauf-Quelle „nichts war bereit" (#300)', () => {
  it('erscheint mit einem Cooldown über dem GCD — AC3', () => {
    // REFLUX hat 45 s Abklingzeit. Nach seiner ersten Wirkung ist über
    // Minuten kein gebundener Zauber bereit.
    const nurReflux = laufMit(['reflux']);

    expect(nurReflux.elapsed, 'der Lauf hat gar nicht stattgefunden').toBeGreaterThan(0);
    expect(
      nurReflux.nothingReady,
      'die Quelle bleibt leer, obwohl nur ein Zauber mit 45 s Abklingzeit ' +
        'gebunden ist — dann ist sie nicht erreichbar, und der Vermerk in ' +
        'session.ts behauptet etwas Falsches',
    ).toBeGreaterThan(0);
  });

  it('bleibt leer, sobald ein Zauber mit Cooldown ≤ GCD dazukommt — AC4', () => {
    // **Der Gegenfall, und er ist der eigentliche Nachweis.** Ein Lauf mit
    // Leerlauf allein zeigt nicht, dass KINDLE ihn verhindert — dafür braucht
    // es zwei Zustände mit einem Zauber Unterschied.
    //
    // Genau diesen Unterschied konnte der Browser-Nachweis nicht herstellen
    // (#306): Dort blieb KINDLE in beiden Läufen markiert, und die gemessene
    // Null belegte nichts.
    const nurReflux = laufMit(['reflux']);
    const mitKindle = laufMit(['reflux', 'kindle']);

    // Die Ursache zuerst: Die beiden Läufe müssen sich wirklich unterscheiden.
    expect(nurReflux.nothingReady).toBeGreaterThan(0);

    expect(
      mitKindle.nothingReady,
      `mit KINDLE auf der Leiste bleiben ${mitKindle.nothingReady} ms „nichts ` +
        'war bereit" übrig — erwartet sind 0, weil sein Cooldown dem GCD ' +
        'entspricht und er damit in jedem zählbaren Takt bereit ist',
    ).toBe(0);
  });

  it('die Bedingung ist der Cooldown gegen den GCD, nicht der Zauber', () => {
    // **Aus der Struktur, nicht aus dem Namen.** Der Vermerk in session.ts
    // nennt KINDLE als Beispiel; die Regel gilt für jeden Zauber mit
    // `cooldownMs <= GCD_MS`. Hier steht sie als Rechnung.
    const boss = FIRST_CRUCIBLE.value;
    const combat = new Combat({ school: CALCINATION.value, boss });
    const kindle = combat.spells[combat.indexOf('kindle')]!;
    const reflux = combat.spells[combat.indexOf('reflux')]!;

    expect(kindle.cooldownMs).toBeLessThanOrEqual(GCD_MS);
    expect(reflux.cooldownMs).toBeGreaterThan(GCD_MS);
  });
});
