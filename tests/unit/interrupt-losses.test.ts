/**
 * Verpasste Unterbrechungen (Issue #81, AC5 und AC6).
 *
 * Erst seit Issue #151 belegbar: Bis dahin schrieb nur der Simulator die
 * Boss-Ereignisse, und die Fehleranalyse laeuft im Browser.
 *
 * DIE BEIDEN AC SIND EIN PAAR UND MUESSEN ES BLEIBEN.
 * AC5 allein bestuende auch fuer eine Rechnung, die Unterbrecher schlicht
 * ignoriert. AC6 allein bestuende auch fuer die alte Bereitschafts-Regel. Erst
 * zusammen zeigen sie, dass ueber die GELEGENHEIT gezaehlt wird.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import {
  analyzeRun,
  interruptDataFor,
  runDataFor,
  type RunAnalysis,
} from '../../src/analysis/run-analysis.js';

const KINDLE = 0;
const QUENCH = 3;

/** Ein Boss mit drei unterbrechbaren Zaubern zu je 2000 Schaden. */
const ONSLAUGHT = bossById('test-onslaught') as BossDef;
/** Einer mit genau einem, zu 600 — und zwei Schlaegen, die nicht zu unterbrechen sind. */
const WARDEN = bossById('test-warden') as BossDef;

interface Lauf {
  readonly analysis: RunAnalysis;
  readonly combat: Combat;
}

/**
 * Ein wirklich gespielter Durchlauf.
 *
 * `interruptAtMs` sagt, wann QUENCH gedrueckt wird — leer heisst: nie. Das ist
 * der einzige Hebel, mit dem eine Gelegenheit genutzt oder verpasst wird.
 */
function play(boss: BossDef, loadout: readonly number[], interruptAtMs: readonly number[]): Lauf {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  loadout.forEach((spell, slot) => {
    session.bind(slot, spell);
  });
  session.beginCountdown(0);

  const quenchSlot = loadout.indexOf(QUENCH);
  const ende = COUNTDOWN_MS + 30_000;
  let tick = 0;
  for (let t = 0; t <= ende; t += 10) {
    const seitBeginn = t - COUNTDOWN_MS;
    if (t >= COUNTDOWN_MS) {
      // Der Grundtakt laeuft auf KINDLE, damit ueberhaupt Schaden entsteht --
      // ohne ihn waere der Durchschnittsschaden 0 und jede Verlustrechnung 0.
      if (seitBeginn % 1500 === 0) queue.push(loadout.indexOf(KINDLE), tick, t, tick);
      if (quenchSlot >= 0 && interruptAtMs.includes(seitBeginn)) {
        queue.push(quenchSlot, tick, t, tick);
      }
    }
    session.tick(t);
    tick++;
  }

  const summary = session.summary(ende);
  const interrupts = interruptDataFor(combat);
  return {
    combat,
    analysis: analyzeRun({
      summary,
      gcdMs: GCD_MS,
      spells: runDataFor(combat, summary, loadout),
      synergyBonusDamage: 0,
      missedInterrupts: interrupts.missed,
      damagePerLandedBossCast: interrupts.damagePerLandedCast,
      bossHealed: 0,
    }),
  };
}

describe('AC5 — nichts zu unterbrechen, kein Verlust', () => {
  it('nennt den Unterbrecher NICHT, wenn der Boss gar nichts wirkt', () => {
    // FIRST_CRUCIBLE hat eine leere Zeitleiste. QUENCH liegt die ganze Zeit
    // bereit und wird nie gewirkt -- nach der alten Bereitschafts-Regel stuende
    // er als GROESSTE Verlustquelle ganz oben.
    const { analysis, combat } = play(FIRST_CRUCIBLE.value, [KINDLE, QUENCH], []);

    expect(combat.boss.timeline).toHaveLength(0);
    expect(analysis.losses.interrupts).toBeNull();
    expect(analysis.losses.sources.map((s) => s.kind)).not.toContain('missedInterrupts');
    // Und NICHT als fehlende Datengrundlage: Es wurde gemessen, dass es nichts
    // zu verpassen gab. Das ist eine Aussage, kein Loch.
    expect(analysis.losses.unavailable).not.toContain('missedInterrupts');
  });

  it('nennt ihn auch dann nicht, wenn der Boss NUR nicht-unterbrechbare Schlaege hat', () => {
    // Der schaerfere Fall: Der Boss handelt, aber nichts davon ist zu
    // unterbrechen. Eine Rechnung, die schlicht 'der Boss hat eine Zeitleiste'
    // prueft, faellt hier durch.
    const nurSchlaege: BossDef = {
      ...WARDEN,
      timeline: WARDEN.timeline.filter((e) => e.mechanic !== 'interruptibleCast'),
    };
    const { analysis } = play(nurSchlaege, [KINDLE, QUENCH], []);

    expect(nurSchlaege.timeline.length).toBeGreaterThan(0);
    expect(analysis.losses.interrupts).toBeNull();
  });
});

describe('AC6 — eine echte Gelegenheit ungenutzt', () => {
  it('weist den Unterbrecher aus, wenn ein Zauber durchkam', () => {
    const { analysis, combat } = play(ONSLAUGHT, [KINDLE, QUENCH], []);

    // Drei unterbrechbare Zauber, keiner unterbrochen.
    expect(combat.interruptsMissed).toBeGreaterThan(0);
    expect(analysis.losses.interrupts?.missed).toBe(combat.interruptsMissed);
    // In der eigenen Zeile, NICHT in der Prozentrechnung der Schadensquellen.
    expect(analysis.losses.sources.map((s) => s.kind)).not.toContain('missedInterrupts');
  });

  it('rechnet Anzahl mal Schaden des durchgekommenen Zaubers', () => {
    const { analysis, combat } = play(ONSLAUGHT, [KINDLE, QUENCH], []);

    // Unabhaengig hergeleitet: aus dem Zaehler des Kampfes und dem Schaden,
    // der in den BOSS-DATEN steht. Beides kennt die Verlustrechnung nicht.
    const jeZauber = ONSLAUGHT.timeline.find((e) => e.mechanic === 'interruptibleCast')?.damage;
    expect(jeZauber).toBe(2000);
    const erwartet = combat.interruptsMissed * (jeZauber ?? 0);

    expect(analysis.losses.interrupts?.damageTaken).toBe(erwartet);
  });

  it('bleibt aus dem Prozentsatz der Schadensquellen heraus', () => {
    // Der Kern der Entscheidung vom 2026-08-04. Gemessen: 6000 genommener
    // Schaden gegen 2100 ausgeteilten -- in derselben Rechnung stuenden 74 %
    // 'liegen gelassen' da, obwohl der Spieler nichts liegen gelassen, sondern
    // etwas EINGESTECKT hat.
    const { analysis } = play(ONSLAUGHT, [KINDLE, QUENCH], []);
    expect(analysis.losses.interrupts?.damageTaken).toBe(6000);
    expect(analysis.losses.totalLost).toBe(0);
    expect(analysis.losses.totalShare).toBe(0);
  });

  it('zaehlt ihn NICHT doppelt — weder als Leerlauf noch als verpasste Wirkung', () => {
    // Die zweite Haelfte von AC6, und die schwerere. QUENCH loest keinen
    // Grundtakt aus (GAME-DESIGN 6.6) und kann deshalb kein Fenster fuellen --
    // ein Fenster, in dem er bereitlag, ist entweder leer oder von einem
    // anderen Zauber besetzt. Die Fenster-Zahlen duerfen sich dadurch nicht
    // aendern.
    const ohne = play(ONSLAUGHT, [KINDLE], []).analysis;
    const mit = play(ONSLAUGHT, [KINDLE, QUENCH], []).analysis;

    expect(mit.missedCastWindows).toBe(ohne.missedCastWindows);
    expect(mit.idleWindows).toBe(ohne.idleWindows);

    // Und der Unterbrecher steht genau EINMAL -- in seiner eigenen Zeile.
    expect(mit.losses.interrupts?.missed).toBe(3);
    expect(mit.losses.sources.map((s) => s.kind)).not.toContain('missedInterrupts');
  });

  it('verschwindet wieder, sobald der Spieler unterbricht', () => {
    // Der Fall, in dem sich etwas aendern MUSS. Ohne ihn bestuende die
    // Pruefung auch fuer eine Rechnung, die jeden unterbrechbaren Zauber als
    // verpasst zaehlt, egal was der Spieler tut.
    const verpasst = play(ONSLAUGHT, [KINDLE, QUENCH], []);
    // Waehrend der Wirkzeit druecken: Cascade bei 3000, 10000, 17000 ms mit
    // je 2000 ms Wirkzeit.
    const unterbrochen = play(ONSLAUGHT, [KINDLE, QUENCH], [4000, 11_000, 18_000]);

    expect(unterbrochen.combat.interruptsLanded).toBeGreaterThan(0);
    expect(unterbrochen.combat.interruptsMissed).toBeLessThan(verpasst.combat.interruptsMissed);

    expect(unterbrochen.analysis.losses.interrupts?.damageTaken ?? 0).toBeLessThan(
      verpasst.analysis.losses.interrupts?.damageTaken ?? 0,
    );
  });
});

describe('Die Gelegenheits-Daten selbst', () => {
  it('meldet einen Mittelwert als Mittelwert', () => {
    // Bei verschieden starken unterbrechbaren Zaubern ist "Anzahl mal Schaden"
    // eine Naeherung. Das gehoert gesagt, nicht verschwiegen.
    const gemischt: BossDef = {
      ...ONSLAUGHT,
      timeline: [
        {
          atMs: 3000,
          mechanic: 'interruptibleCast',
          name: 'A',
          damage: 1000,
          castTimeMs: 2000,
          interruptsPlayer: false,
        },
        {
          atMs: 10_000,
          mechanic: 'interruptibleCast',
          name: 'B',
          damage: 3000,
          castTimeMs: 2000,
          interruptsPlayer: false,
        },
      ],
    };
    const daten = interruptDataFor(new Combat({ school: CALCINATION.value, boss: gemischt }));
    expect(daten.damagePerLandedCast).toBe(2000);
    expect(daten.averaged).toBe(true);
  });

  it('meldet gleiche Werte NICHT als Mittelwert', () => {
    const daten = interruptDataFor(new Combat({ school: CALCINATION.value, boss: ONSLAUGHT }));
    expect(daten.damagePerLandedCast).toBe(2000);
    expect(daten.averaged).toBe(false);
  });

  it('gibt 0 zurueck, wo es nichts zu unterbrechen gibt — nicht null', () => {
    const daten = interruptDataFor(
      new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value }),
    );
    expect(daten.missed).toBe(0);
    expect(daten.averaged).toBe(false);
  });
});
