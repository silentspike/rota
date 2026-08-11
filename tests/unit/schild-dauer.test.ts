/**
 * PLATE und SCAB — wie lange die Schicht hält (Issue #79, #481).
 *
 * # Die Dauer war bei BEIDEN Schild-Zaubern offen
 *
 * `tools/groessen-mutation.py`, Vollmessung aller 116 Paare:
 *
 * ```text
 *   shield.durationMs  plate  15000 -> 7500  NUR FRISCHE
 *   shield.durationMs  scab    8000 -> 4000  SCHWEIGT
 * ```
 *
 * Die beiden **anderen** Größen derselben Zauber sind längst gedeckt —
 * `amount` und `maxStacks` je dreifach. Das ist das Muster hinter fast allen
 * Lücken dieser Messreihe:
 *
 * > Die Größe, die man **sieht**, ist geprüft. Die Dauer, die man erst merkt,
 * > wenn sie abläuft, nicht.
 *
 * # Warum SCAB schwieg
 *
 * `SCHWEIGT` heißt: Nicht einmal die Vergleichsdatei färbte. SCAB hat 1500 ms
 * Abklingzeit — in jedem Lauf, der ihn regelmäßig wirft, wird die Frist
 * **ständig erneuert** und läuft nie ab. Eine halbierte Dauer fällt dort
 * niemandem auf.
 *
 * Der Fall unten wirft ihn deshalb **genau einmal** und sieht dann zu.
 *
 * # Gemessen, nicht gerechnet
 *
 * ```text
 *              1000   6000   8000   10000  15000  16000
 *   PLATE      2400   2400   2400   2400   2400      0
 *   halbiert   2400      —      0       0      0      0
 *
 *   SCAB        200    200      0       0      —      —
 *   halbiert     200      0      0       0      —      —
 * ```
 *
 * Beide Zeilen „halbiert" stammen aus Läufen mit mutierter Datei. Ohne sie
 * bliebe offen, ob die Messpunkte die Mutation überhaupt trennen — bei 1000 ms
 * etwa tun sie es nicht.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/**
 * Wirft `id` **genau einmal** bei 0 und liest den Schild bei `beiMs` ab.
 *
 * Einmal, nicht wiederholt: Wer nachlegt, erneuert die Frist und misst
 * dann die Abklingzeit statt der Dauer.
 */
function schildBei(id: string, beiMs: number): number {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const index = alsSpellIndex(combat.indexOf(id));
  combat.stepPassive(0);
  if (combat.cast(index, 0).result !== CAST_OK) return -1;

  for (let timeMs = 0; timeMs <= beiMs; timeMs += TICK_MS) combat.stepPassive(timeMs);
  return combat.shield;
}

describe('PLATEs Schicht hält fünfzehn Sekunden (shield.durationMs)', () => {
  it('die Zusicherung: sie entsteht überhaupt', () => {
    // Ohne diese Zeile wäre jedes „0 nach Ablauf" auch dann grün, wenn PLATE
    // nie eine Schicht gelegt hätte.
    expect(schildBei('plate', 1000), 'PLATE legt keine Schicht von 2400').toBe(2400);
  });

  it('nach 8000 ms steht sie noch — mit halber Dauer wäre sie weg', () => {
    // **Der Fänger.** Halbiert man auf 7500, steht hier 0. Gemessen mit
    // mutierter Datei; bei 1000 ms trennt der Messpunkt die beiden Fälle
    // dagegen nicht.
    expect(schildBei('plate', 8000), 'die Schicht endet vor 8000 ms').toBe(2400);
  });

  it('bei 15000 ms gerade noch, bei 16000 nicht mehr', () => {
    // Die Grenze selbst — ohne sie wäre auch eine Dauer von einer Minute grün.
    expect(schildBei('plate', 15000), 'die Schicht endet vor ihrer Frist').toBe(2400);
    expect(schildBei('plate', 16000), 'die Schicht überlebt ihre Frist').toBe(0);
  });
});

describe('SCABs Schicht hält acht Sekunden (shield.durationMs)', () => {
  it('die Zusicherung: sie entsteht überhaupt', () => {
    expect(schildBei('scab', 1000), 'SCAB legt keine Schicht von 200').toBe(200);
  });

  it('nach 6000 ms steht sie noch — mit halber Dauer wäre sie weg', () => {
    // Derselbe Fänger wie oben, an SCABs kürzerer Frist ausgerichtet:
    // halbiert endet sie bei 4000.
    expect(schildBei('scab', 6000), 'die Schicht endet vor 6000 ms').toBe(200);
  });

  it('bei 8000 ms gerade noch, bei 9000 nicht mehr', () => {
    expect(schildBei('scab', 8000), 'die Schicht endet vor ihrer Frist').toBe(200);
    expect(schildBei('scab', 9000), 'die Schicht überlebt ihre Frist').toBe(0);
  });

  it('und die beiden Fristen sind wirklich verschieden', () => {
    // **Die Zusicherung gegen den stillsten Fehler dieser Sorte**: Läse der
    // Code für beide Zauber dieselbe Zahl, wären alle Fälle oben trotzdem
    // grün, solange sie zufällig zusammenpasst. Bei 10000 ms trennt sich das:
    // PLATE steht noch, SCAB nicht mehr.
    expect(schildBei('plate', 10000)).toBe(2400);
    expect(schildBei('scab', 10000)).toBe(0);
  });
});
