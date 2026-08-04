/**
 * Der Zähler, der bei der Schwelle tötet, und der exponentielle Stapel
 * (GAME-DESIGN 11.2, Bausteine 2 und 3; Issue #216).
 *
 * WARUM BEIDE IN EINER DATEI
 * --------------------------
 * Sie haben dieselbe Form: Ein Wert steigt, und bei einem Punkt passiert etwas
 * Hartes. Was sie unterscheidet, ist die Stelle, an der es weh tut — beim
 * Zähler ist die SCHWELLE hart, beim Stapel die KURVE. Nebeneinander geprüft
 * bleibt dieser Unterschied sichtbar; getrennt sähen beide aus wie „ein Wert
 * steigt".
 *
 * Der Prüf-Boss `test-doomsayer` trägt beide und ist nirgends im Spiel
 * eingehängt (11.2: welcher Boss welchen Baustein bekommt, entscheidet E10).
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, PLAYER } from '../../src/core/data.js';
import { stackDamage } from '../../src/core/damage.js';
import { TICK_MS } from '../../src/core/loop.js';

const DOOMSAYER = bossById('test-doomsayer');
if (DOOMSAYER === undefined) throw new Error('data/bosses/test-doomsayer.json fehlt.');

function neuerKampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: DOOMSAYER as never });
}

/** Den Kampf bis `bisMs` laufen lassen, ohne einen einzigen Tastendruck. */
function laufen(combat: Combat, bisMs: number, beiMs?: (t: number, c: Combat) => void): void {
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) {
    beiMs?.(t, combat);
    combat.stepPassive(t);
  }
}

describe('AC1 — der Zähler tötet bei seiner Schwelle', () => {
  it('bringt den Spieler um, und zwar am ausgerechneten Zeitpunkt', () => {
    const doom = DOOMSAYER.doom;
    expect(doom).not.toBeNull();
    // 100 bei 5 je Sekunde: der zwanzigste Schritt erreicht die Schwelle.
    const erwartetMs = (doom!.threshold / doom!.amount) * doom!.perStepMs;
    expect(erwartetMs).toBe(20000);

    const combat = neuerKampf();
    laufen(combat, erwartetMs + 1000);

    expect(combat.doom).toBe(doom!.threshold);
    expect(combat.playerHp).toBe(0);
    expect(combat.diedAtMs).toBe(erwartetMs);
  });

  it('tötet durch den ZÄHLER und nicht durch den Stapel-Schaden', () => {
    // Der Unterschied ist belegbar: Zum Todeszeitpunkt hatte der Spieler noch
    // Leben übrig. Ohne diese Zusicherung bliebe der Fall auch dann grün, wenn
    // in Wahrheit der Stapel ihn erschlagen hätte — und dann prüfte er den
    // falschen Baustein.
    const combat = neuerKampf();
    laufen(combat, 19990);

    expect(combat.playerHp).toBeGreaterThan(0);
    expect(combat.damageTaken).toBeLessThan(PLAYER.vessel);
    expect(combat.diedAtMs).toBe(-1);

    combat.stepPassive(20000);
    expect(combat.playerHp).toBe(0);
  });

  it('läuft nach dem Tod nicht weiter', () => {
    const combat = neuerKampf();
    laufen(combat, 30000);
    // Der Zähler bleibt auf der Schwelle stehen statt weiterzuzählen, und der
    // Todeszeitpunkt bleibt der erste.
    expect(combat.doom).toBe(DOOMSAYER.doom!.threshold);
    expect(combat.diedAtMs).toBe(20000);
  });
});

describe('AC2 — die Rücksetz-Handlung senkt, und ihre Begrenzung greift', () => {
  it('zeigt BEIDE Hälften in EINEM Lauf', () => {
    const doom = DOOMSAYER.doom!;
    const combat = neuerKampf();
    const stand: { zeit: number; wert: number; ladungen: number }[] = [];

    // Bis 40 s: Der Tod liegt durch die zwei Senkungen bei 36 s, nicht mehr
    // bei 20. Mein erster Entwurf lief nur bis 25 s und stellte fest, dass
    // der Spieler noch lebt — was nichts belegte ausser der zu kurzen Uhr.
    laufen(combat, 40000, (t, c) => {
      // Zweimal senken — mehr Ladungen gibt es nicht.
      if (t === 5000 || t === 10000) {
        const vorher = c.doom;
        const gegriffen = c.clearDoom();
        stand.push({ zeit: t, wert: c.doom, ladungen: c.doomCharges });
        expect(gegriffen, `bei ${t} ms`).toBe(true);
        expect(c.doom).toBe(Math.max(0, vorher - doom.clearAmount));
      }
      // Und ein dritter Versuch, für den es keine Ladung mehr gibt.
      if (t === 15000) {
        const vorher = c.doom;
        expect(c.doomCharges).toBe(0);
        expect(c.clearDoom()).toBe(false);
        expect(c.doom).toBe(vorher);
      }
    });

    // Erste Hälfte: sie hat gesenkt.
    expect(stand).toHaveLength(2);
    expect(stand[0]!.ladungen).toBe(1);
    expect(stand[1]!.ladungen).toBe(0);

    // Zweite Hälfte, und das ist die eigentliche Aussage: Der Spieler stirbt
    // trotzdem.
    //
    // WIE VIEL SPÄTER, IST DIE MECHANIK SELBST. Zu früh gesenkt heißt
    // verschenkt: Bei 5 s standen erst 20 auf dem Zähler, bei 10 s erst 25 —
    // die Handlung nimmt bis zu 40, konnte aber nur wegnehmen, was da war.
    // Zusammen 45 statt der möglichen 80, also 45/5 = 9 s Aufschub.
    //
    // Mein erster Entwurf setzte hier 2 × 40 an und erwartete 36 s. Gemessen
    // wurden 29 s — und die Rechnung des Kampfes war richtig, meine
    // Erwartung falsch. Genau dieser Unterschied ist die Entscheidung, die
    // 11.2 mit „begrenzt verfügbar" meint.
    const genommen = 20 + 25;
    expect(combat.playerHp).toBe(0);
    expect(combat.diedAtMs).toBe(20000 + (genommen * doom.perStepMs) / doom.amount);
    expect(combat.diedAtMs).toBe(29000);
    // Und zum Beleg, dass hier wirklich verschenkt wurde: Wäre beide Male die
    // volle Menge abgegangen, läge das Ende 7 s später.
    expect(combat.diedAtMs).toBeLessThan(
      20000 + (2 * doom.clearAmount * doom.perStepMs) / doom.amount,
    );
  });

  it('nimmt sich nichts, was sie nicht hat — ohne Ladung geschieht NICHTS', () => {
    const combat = neuerKampf();
    combat.beginFight(0);
    combat.stepPassive(0);
    expect(combat.clearDoom()).toBe(true);
    expect(combat.clearDoom()).toBe(true);
    const wert = combat.doom;
    const ladungen = combat.doomCharges;
    expect(combat.clearDoom()).toBe(false);
    expect(combat.doom).toBe(wert);
    expect(combat.doomCharges).toBe(ladungen);
  });
});

describe('AC3 — der Stapel wächst exponentiell, nicht linear', () => {
  it('unterscheidet beide Kurven ZAHLENMÄSSIG, bevor sie verglichen werden', () => {
    const def = DOOMSAYER.stacks!;
    // Erst der Nachweis, dass die beiden Kurven bei DIESEN Werten überhaupt
    // auseinanderliegen. Ohne ihn wäre der Vergleich unten stumm grün: Bei
    // einem Stapel sind linear und exponentiell identisch, und ein Prüffall,
    // der nur dort misst, hielte jede Rechnung für richtig.
    const linear = [1, 2, 3, 4].map((n) => def.baseDamage * n);
    const exponentiell = [1, 2, 3, 4].map((n) => stackDamage(def, n));

    expect(linear).toEqual([20, 40, 60, 80]);
    expect(exponentiell).toEqual([20, 60, 180, 540]);
    // Beim ERSTEN Stapel sind sie gleich — genau deshalb wird ab dem zweiten
    // verglichen.
    expect(exponentiell[0]).toBe(linear[0]);
    for (let i = 1; i < linear.length; i++) {
      expect(exponentiell[i], `Stapel ${i + 1}`).not.toBe(linear[i]);
    }
    // Und die Reihe aus 11.2 wörtlich: 1 → 3 → 9.
    expect(exponentiell[1]! / exponentiell[0]!).toBe(def.factor);
    expect(exponentiell[2]! / exponentiell[0]!).toBe(def.factor * def.factor);
  });

  it('rechnet im Kampf mit der exponentiellen Reihe', () => {
    const def = DOOMSAYER.stacks!;
    const combat = neuerKampf();
    const genommen: number[] = [];
    let vorher = 0;

    // **Nach** dem Takt ablesen. Mein erster Entwurf las davor und bekam
    // [0, 16, 48, 144] — um eins verschoben, weil der Schlag desselben
    // Zeitpunkts noch nicht gefallen war. Die Reihe sah dadurch aus, als
    // begaenne sie bei null; die Rechnung war die ganze Zeit richtig.
    combat.beginFight(0);
    for (let t = 0; t <= 8000; t += TICK_MS) {
      combat.stepPassive(t);
      if (t > 0 && t % def.perStepMs === 0) {
        genommen.push(combat.damageTaken - vorher);
        vorher = combat.damageTaken;
      }
    }

    // Nach WARD: round(x * 0.8). Die Rundung steht hier ausgeschrieben, weil
    // sie zur Schnittstelle gehört — einmal am Ende, wie überall sonst.
    const nachWard = (roh: number): number => Math.round(roh * (1 - PLAYER.wardReduction));
    const erwartet = [1, 2, 3, 4].map((n) => nachWard(stackDamage(def, n)));
    expect(genommen.slice(0, 4)).toEqual(erwartet);

    // Der Gegenvergleich: Mit linearem Wachstum käme etwas anderes heraus.
    const linear = [1, 2, 3, 4].map((n) => nachWard(def.baseDamage * n));
    expect(genommen.slice(0, 4)).not.toEqual(linear);
  });

  it('hört bei der Obergrenze auf zu wachsen', () => {
    const def = DOOMSAYER.stacks!;
    const combat = neuerKampf();
    laufen(combat, def.perStepMs * (def.maxStacks + 3));
    expect(combat.bossStacks).toBe(def.maxStacks);
  });
});

describe('AC4 — ein Boss ohne die Felder bleibt unberührt', () => {
  it('hat weder Zähler noch Stapel, und beide Werte stehen auf null', () => {
    const ohne = bossById('first-crucible');
    expect(ohne?.doom).toBeNull();
    expect(ohne?.stacks).toBeNull();

    const combat = new Combat({ school: CALCINATION.value, boss: ohne as never });
    combat.beginFight(0);
    for (let t = 0; t <= 30000; t += TICK_MS) combat.stepPassive(t);

    expect(combat.doom).toBe(0);
    expect(combat.doomCharges).toBe(0);
    expect(combat.bossStacks).toBe(0);
    expect(combat.playerHp).toBe(PLAYER.vessel);
    expect(combat.diedAtMs).toBe(-1);
    // Und die Handlung greift ins Leere, statt einen Zähler zu erfinden.
    expect(combat.clearDoom()).toBe(false);
  });
});
