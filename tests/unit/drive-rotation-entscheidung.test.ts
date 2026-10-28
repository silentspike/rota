/**
 * Wann die Schleife hinter `runReference` entscheidet (Issue #1027).
 *
 * # Das Entscheidungsmodell, festgelegt am 2026-10-28
 *
 * Das Spiel selbst (`Session.tick`) arbeitet alle Drücke eines Takts ab. Ein
 * Spieler kann deshalb nach QUENCH im **selben** Takt einen Schadenszauber
 * wirken, und er kann QUENCH **während** einer laufenden globalen Wartezeit
 * drücken. Bis hierher konnte die Schleife beides nicht: eine Entscheidung je
 * Takt, und nur bei bereiter Wartezeit. Der Orchestrator hat an #1027
 * festgelegt, dass Schleife und Simulator beides lernen:
 *
 * | | |
 * | --- | --- |
 * | (a) | nach einem Wurf ohne Wartezeit eine weitere Entscheidung beim selben Zeitstempel |
 * | (b) | Zauber ohne Wartezeit während einer laufenden |
 * | (c) | ein Nullzeit-Budget gegen Nullzeit-Zyklen |
 *
 * Die Sollwerte hier sind **von Hand hergeleitet**, aus den Zauberdaten
 * (KINDLE und GCD je 1500 ms, QUENCH ohne Wartezeit mit 5000 ms Abklingzeit,
 * SUBLIME befreit den nächsten Wurf). Gegen den Simulator steht dieselbe Lage in
 * `tests/parity/recordings/gcd-freier-wurf.json`.
 */
import { describe, expect, it } from 'vitest';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { driveRotation } from '../../src/devtools/reference-run.js';

interface Zeile {
  readonly spellId: string;
  readonly gilt: (combat: Combat, timeMs: number) => boolean;
}

const IMMER = (): boolean => true;

/** Fährt die Liste wie `Apl::decide`: die erste Zeile, deren Zauber bereit ist und deren Bedingung gilt. */
function fahre(zeilen: readonly Zeile[], dauerMs = 30_000) {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const regeln = zeilen.map((z) => ({
    index: alsSpellIndex(combat.indexOf(z.spellId)),
    gilt: z.gilt,
  }));
  const wuerfe: [number, string][] = [];
  const ergebnis = driveRotation(
    combat,
    (timeMs) => {
      for (const regel of regeln) {
        if (combat.isReady(regel.index, timeMs) && regel.gilt(combat, timeMs)) return regel.index;
      }
      return -1;
    },
    dauerMs,
    false,
    (spell, timeMs) => {
      if (combat.cast(spell, timeMs).result === 0) {
        wuerfe.push([timeMs, combat.spells[spell]?.id ?? '?']);
      }
    },
  );
  return { wuerfe, idleMs: ergebnis.idleMs };
}

function von(wuerfe: readonly [number, string][], id: string): number[] {
  return wuerfe.filter(([, s]) => s === id).map(([t]) => t);
}

describe('(a) Nach einem Wurf ohne Wartezeit entscheidet die Liste beim selben Zeitstempel', () => {
  it('nach QUENCH folgt der Füller im selben Takt, nicht einen später', () => {
    const { wuerfe } = fahre([
      { spellId: 'quench', gilt: IMMER },
      { spellId: 'kindle', gilt: IMMER },
    ]);
    expect(wuerfe.slice(0, 2)).toEqual([
      [0, 'quench'],
      [0, 'kindle'],
    ]);
  });

  it('nach dem Wurf, den SUBLIME befreit, entscheidet die Liste beim selben Zeitstempel', () => {
    // SUBLIME bei 0 (Wartezeit bis 1500) befreit KINDLE bei 1500; danach ist die
    // Wartezeit bereit, und SMOULDER — nie gelegt — ist noch bei 1500 dran.
    const { wuerfe } = fahre([
      { spellId: 'sublime', gilt: IMMER },
      { spellId: 'kindle', gilt: IMMER },
      {
        spellId: 'smoulder',
        gilt: (c, t) => c.dotRemainingMs(alsSpellIndex(c.indexOf('smoulder')), t) <= 1500,
      },
    ]);
    expect(von(wuerfe, 'kindle')[0]).toBe(1500);
    expect(von(wuerfe, 'smoulder')[0]).toBe(1500);
  });
});

describe('(b) Ein Zauber ohne Wartezeit während einer laufenden', () => {
  it('QUENCH unter KINDLE kommt im selben Takt und am Ende seiner Abklingzeit mitten im GCD', () => {
    // KINDLE steht oben und ist an jedem GCD-Ende bereit; QUENCH kam bisher NIE
    // zum Zug. Jetzt: nach KINDLE bei 0 im selben Takt, dann bei 5000 — mitten
    // im GCD des KINDLE von 4500.
    const { wuerfe } = fahre([
      { spellId: 'kindle', gilt: IMMER },
      { spellId: 'quench', gilt: IMMER },
    ]);
    expect(von(wuerfe, 'quench')).toEqual([0, 5000, 10_000, 15_000, 20_000, 25_000]);
    expect(von(wuerfe, 'kindle')).toHaveLength(20);
  });

  it('eine Bedingung, die mitten im GCD wahr wird, greift an ihrem Takt', () => {
    const { wuerfe } = fahre([
      { spellId: 'kindle', gilt: IMMER },
      { spellId: 'quench', gilt: (_c, t) => t >= 700 },
    ]);
    expect(von(wuerfe, 'quench')[0]).toBe(700);
  });

  it('ohne Zauber ohne Wartezeit ändert sich nichts', () => {
    const { wuerfe, idleMs } = fahre([{ spellId: 'kindle', gilt: IMMER }]);
    expect(von(wuerfe, 'kindle')).toEqual(Array.from({ length: 20 }, (_, i) => i * 1500));
    expect(idleMs).toBe(0);
  });
});

describe('Leerlauf: je Takt mit bereiter Wartezeit und ohne Wurf 10 ms', () => {
  it('ein Takt mit QUENCH ist kein leerer Takt, die danach schon', () => {
    // QUENCH bei 0 und 5000, KINDLE ab 5000: leer sind die Takte 10 bis 4990.
    const { wuerfe, idleMs } = fahre([
      { spellId: 'quench', gilt: IMMER },
      { spellId: 'kindle', gilt: (_c, t) => t >= 5000 },
    ]);
    expect(wuerfe.slice(0, 3)).toEqual([
      [0, 'quench'],
      [5000, 'quench'],
      [5000, 'kindle'],
    ]);
    expect(idleMs).toBe(4990);
  });

  it('ein Takt im laufenden GCD ist nie leer, auch wenn die Liste nichts wählt', () => {
    const { idleMs } = fahre([
      { spellId: 'kindle', gilt: IMMER },
      { spellId: 'quench', gilt: (_c, t) => t >= 700 },
    ]);
    expect(idleMs).toBe(0);
  });
});

describe('(c) Das Nullzeit-Budget', () => {
  it('ein Wurf, der den Zustand nie ändert, wird als Datenfehler abgelehnt statt endlos gerechnet', () => {
    // Ein Zauber, der nach dem Wurf bereit bleibt, ist ein Nullzeit-Zyklus: Die
    // Liste wählt ihn wieder und wieder, ohne dass die Zeit fortschreitet. Hier
    // ist er gebaut, indem der Wurf nichts tut.
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    const quench = alsSpellIndex(combat.indexOf('quench'));
    let wuerfe = 0;
    expect(() =>
      driveRotation(
        combat,
        () => quench,
        1000,
        false,
        () => {
          wuerfe++;
          // **Die Probe begrenzt sich selbst**, damit eine Schleife ohne Budget
          // hier rot wird statt den Prüflauf für immer anzuhalten — ein
          // synchroner Endloslauf ist von keinem Zeitlimit zu unterbrechen.
          if (wuerfe > 10_000) throw new Error('Probe abgebrochen: kein Budget gegriffen');
        },
      ),
    ).toThrow(/Nullzeit-Zyklus/);
    // Das Budget ist die Zahl der Zauber im Datensatz: Ohne Rücksetzer kann
    // jeder höchstens einmal je Zeitstempel wirken.
    expect(wuerfe).toBe(combat.spells.length);
  });
});
