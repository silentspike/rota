/**
 * Effizienz-Tabelle und Fehleranalyse an ECHTEN Durchlaeufen (Issue #81).
 *
 * Es wird wirklich gespielt: Sitzung, Countdown, Tastendruecke, Kampfende. Ein
 * Prueflauf, der `RunSummary` von Hand zusammensetzt, belegt nur, dass die
 * Rechnung mit erfundenen Zahlen umgehen kann — nicht, dass die Zahlen im Spiel
 * so entstehen. Und ohne `beginCountdown` bliebe die Sitzung in der
 * Vorbereitung; der Schaden waere ueberall null, und jede Gleichheit ginge
 * durch (WORKER-GUIDE § 5).
 *
 * DIE ERWARTUNGEN SIND UNABHAENGIG HERGELEITET
 * --------------------------------------------
 * Die Leerlauf-Sekunden werden nicht gegen sich selbst geprueft, sondern gegen
 * eine Rechnung aus der Zahl der Wirkungen im PROTOKOLL und der Abklingzeit aus
 * den DATEN. Beides kennt die Analyse nicht.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';
import { PressQueue } from '../../src/input/queue.js';
import { analyzeRun, runDataFor, type RunAnalysis } from '../../src/analysis/run-analysis.js';

const KINDLE = 0;
const SMOULDER = 1;
const PYRE = 2;
const CRUST = 4;

const FIGHT_MS = 30_000;

interface Run {
  readonly summary: RunSummary;
  readonly analysis: RunAnalysis;
  readonly combat: Combat;
}

/**
 * Einen Durchlauf spielen.
 *
 * `everyMs` bestimmt, wie oft gedrueckt wird — das ist der einzige Hebel, mit
 * dem ein Fehler absichtlich erzeugt wird. Wer seltener drueckt als der
 * Grundtakt, laesst Fenster leer.
 */
function play(loadout: readonly number[], everyMs: number): Run {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const session = new Session(queue, combat);
  loadout.forEach((spell, slot) => {
    session.bind(slot, spell);
  });
  session.beginCountdown(0);

  const ende = COUNTDOWN_MS + FIGHT_MS;
  let tick = 0;
  // Reihum ueber die Plaetze — nach der Zahl der DRUECKE, nicht nach dem Takt.
  // Ueber den Takt gerechnet trafen alle Druecke denselben Platz, und der
  // zweite Zauber blieb ungenutzt, ohne dass es auffiel.
  let presses = 0;
  for (let t = 0; t <= ende; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      queue.push(presses % loadout.length, tick, t, tick);
      presses++;
    }
    session.tick(t);
    tick++;
  }

  const summary = session.summary(ende);
  const analysis = analyzeRun({
    summary,
    gcdMs: GCD_MS,
    spells: runDataFor(combat, summary, loadout),
    synergyBonusDamage: 0,
    // Heute nicht messbar (Issue #151). Bewusst durchgereicht statt hier
    // erfunden — die Luecke steht danach in `losses.unavailable`.
    missedInterrupts: null,
    damagePerLandedBossCast: 0,
  });
  return { summary, analysis, combat };
}

// --------------------------------------------------------------------------
// AC7 — die beiden Haelften ueberschneiden sich nicht
// --------------------------------------------------------------------------

describe('AC7 — Leerlauf und verpasste Wirkungen schliessen einander aus', () => {
  const FAELLE: readonly [string, readonly number[], number][] = [
    ['sauber gespielt', [KINDLE, SMOULDER], 1500],
    ['zu selten gedrueckt', [KINDLE, SMOULDER], 3000],
    ['nur ein langsamer Zauber', [PYRE], 5000],
    ['langsamer Zauber, zu frueh gedrueckt', [PYRE], 1500],
    ['sehr lange Abklingzeit', [CRUST], 6000],
  ];

  for (const [name, loadout, everyMs] of FAELLE) {
    it(`ergibt zusammen genau den Leerlauf — ${name}`, () => {
      const { summary } = play(loadout, everyMs);
      expect(summary.lostTimeWithReadyMs + summary.lostTimeNothingReadyMs).toBe(summary.lostTimeMs);
    });

    it(`weist nie mehr aus, als verloren ging — ${name}`, () => {
      const { summary, analysis } = play(loadout, everyMs);
      // Die Summe der Quellen darf das Gesamtpotenzial nicht uebersteigen.
      expect(analysis.losses.totalLost).toBeLessThanOrEqual(analysis.losses.potential);
      expect(analysis.losses.totalShare).toBeLessThanOrEqual(1);
      // Und kein Fenster steckt in beiden Zaehlern.
      expect(analysis.missedCastWindows + analysis.idleWindows).toBe(
        Math.round(summary.lostTimeMs / GCD_MS),
      );
    });
  }

  it('haelt auch den Fall, in dem sich nichts aendern darf', () => {
    // Ein Durchlauf ohne Fehler. Beide Haelften muessen 0 sein — eine Messlatte,
    // die nur bei Fehlern stimmt, wuerde hier durchfallen.
    const { summary, analysis } = play([KINDLE, SMOULDER], 1500);
    expect(summary.lostTimeMs).toBe(0);
    expect(summary.lostTimeWithReadyMs).toBe(0);
    expect(summary.lostTimeNothingReadyMs).toBe(0);
    expect(analysis.missedCastWindows).toBe(0);
    expect(analysis.idleWindows).toBe(0);
    expect(summary.damage).toBeGreaterThan(0);
  });

  it('trennt tatsaechlich — beide Haelften kommen einzeln vor', () => {
    // Waeren beide immer gleich belegt, waere die Trennung nicht nachgewiesen.
    const verpasst = play([KINDLE, SMOULDER], 3000).summary;
    const leerlauf = play([PYRE], 5000).summary;
    expect(verpasst.lostTimeWithReadyMs).toBeGreaterThan(0);
    expect(verpasst.lostTimeNothingReadyMs).toBe(0);
    expect(leerlauf.lostTimeNothingReadyMs).toBeGreaterThan(0);
    expect(leerlauf.lostTimeWithReadyMs).toBe(0);
  });
});

// --------------------------------------------------------------------------
// AC1 — verpasste Wirkungen als groesste Quelle, mit Prozentzahl
// --------------------------------------------------------------------------

describe('AC1 — absichtlich verpasste Wirkungen', () => {
  it('stehen als groesste Verlustquelle oben', () => {
    const { analysis } = play([KINDLE, SMOULDER], 3000);
    expect(analysis.losses.top[0]?.kind).toBe('missedCasts');
  });

  it('tragen die Prozentzahl, die aus dem Durchlauf folgt', () => {
    const { summary, analysis } = play([KINDLE, SMOULDER], 3000);

    // Unabhaengig nachgerechnet, allein aus der Kurzfassung:
    const fenster = summary.lostTimeWithReadyMs / GCD_MS;
    const schnitt = summary.damage / summary.casts;
    const erwartet = fenster * schnitt;

    const quelle = analysis.losses.sources.find((s) => s.kind === 'missedCasts');
    expect(quelle?.damage).toBeCloseTo(erwartet, 6);
    expect(quelle?.share).toBeCloseTo(erwartet / (summary.damage + analysis.losses.totalLost), 6);
  });

  it('verschwinden, wenn kein Fenster leer blieb', () => {
    const { analysis } = play([KINDLE, SMOULDER], 1500);
    expect(analysis.losses.sources.map((s) => s.kind)).not.toContain('missedCasts');
  });
});

// --------------------------------------------------------------------------
// AC2 — Leerlauf, gegen die Abklingzeit gegengerechnet
// --------------------------------------------------------------------------

describe('AC2 — Leerlauf und seine Sekundenzahl', () => {
  it('stimmt mit Wirkungen aus dem Protokoll und Abklingzeit aus den Daten ueberein', () => {
    const { summary, analysis, combat } = play([PYRE], 5000);

    // Aus dem PROTOKOLL, nicht aus der Messung: so viele Wirkungen gab es.
    const wirkungen = summary.eventCounts['cast'] ?? 0;
    expect(wirkungen).toBeGreaterThan(1);
    // Aus den DATEN: nach jeder Wirkung liegt der Zauber (Abklingzeit − GCD)
    // lang auf Abklingzeit, und in dieser Zeit ist nichts bereit. Die letzte
    // Wirkung faellt ans Kampfende, ihre Wartezeit liegt nicht mehr im Lauf.
    const abklingzeit = combat.spells[PYRE]?.cooldownMs ?? 0;
    expect(abklingzeit).toBeGreaterThan(GCD_MS);
    const erwartetMs = (wirkungen - 1) * (abklingzeit - GCD_MS);

    expect(summary.lostTimeNothingReadyMs).toBe(erwartetMs);
    expect(analysis.table.idleMs).toBe(erwartetMs);
    expect(analysis.table.idleShare).toBeCloseTo(erwartetMs / summary.elapsedMs, 6);
  });

  it('bleibt 0, wenn immer etwas bereit war', () => {
    // Der Gegenfall. Ohne ihn bestuende die Pruefung auch fuer eine Rechnung,
    // die jeden leeren Takt als Leerlauf ausgibt.
    const { summary } = play([KINDLE, SMOULDER], 3000);
    expect(summary.lostTimeNothingReadyMs).toBe(0);
  });
});

// --------------------------------------------------------------------------
// AC3 — die Analyse folgt der Zusammenstellung
// --------------------------------------------------------------------------

describe('AC3 — gerechnet wird ueber die mitgefuehrten Zauber', () => {
  it('nennt genau die Zauber der Zusammenstellung, nicht die der Schule', () => {
    const zwei = play([KINDLE, SMOULDER], 1500).analysis;
    const einer = play([PYRE], 5000).analysis;
    expect(zwei.table.rows.map((r) => r.spellId).sort()).toEqual(['kindle', 'smoulder']);
    expect(einer.table.rows.map((r) => r.spellId)).toEqual(['pyre']);
  });

  it('ergibt andere Zahlen — ohne Codeaenderung', () => {
    const a = play([KINDLE, SMOULDER], 1500);
    const b = play([PYRE], 5000);
    expect(a.summary.damage).not.toBe(b.summary.damage);
    expect(a.analysis.table.overallEfficiency).not.toBe(b.analysis.table.overallEfficiency);
  });

  it('ergibt DIESELBEN Zahlen bei derselben Zusammenstellung', () => {
    // Der Fall, in dem sich nichts aendern darf: Ohne ihn koennte die
    // Auswertung schwanken und der Unterschied oben waere Rauschen.
    const a = play([KINDLE, SMOULDER], 3000).analysis;
    const b = play([KINDLE, SMOULDER], 3000).analysis;
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
  });
});

// --------------------------------------------------------------------------
// Effizienz-Tabelle
// --------------------------------------------------------------------------

describe('Die Effizienz-Tabelle', () => {
  it('fuehrt mit Effizienz und schliesst mit dem Schadensanteil (AC4)', () => {
    const { analysis } = play([KINDLE, SMOULDER], 3000);
    expect(analysis.table.columns[0]).toBe('EFFICIENCY');
    expect(analysis.table.columns.at(-1)).toBe('DAMAGE SHARE');
  });

  it('misst gegen die Fenster, die es gab — nicht gegen Zeit durch Abklingzeit', () => {
    const { summary, analysis } = play([KINDLE, SMOULDER], 3000);
    // 30 s bei 1,5 s Grundtakt sind 21 Fenster: bei 0 und dann alle 1,5 s. Die
    // verworfene Rechnung haette fuer zwei Zauber mit je 1,5 s Abklingzeit 42
    // ergeben — doppelt so viele Wirkungen, wie der Kampf hergibt.
    const fenster = FIGHT_MS / GCD_MS + 1;
    expect(analysis.table.castsMax).toBe(fenster);
    expect(analysis.table.overallEfficiency).toBeCloseTo(summary.casts / fenster, 6);
    // Nie ueber 100 %: Jede Wirkung verbraucht genau ein Fenster.
    expect(analysis.table.overallEfficiency).toBeLessThanOrEqual(1);
  });

  it('erreicht bei fehlerfreiem Spiel volle Effizienz', () => {
    const { analysis } = play([KINDLE, SMOULDER], 1500);
    expect(analysis.table.overallEfficiency).toBe(1);
    for (const row of analysis.table.rows) expect(row.efficiency).toBeCloseTo(1, 6);
  });

  it('setzt oben, was am meisten liegen blieb', () => {
    const { analysis } = play([KINDLE, SMOULDER], 3000);
    const werte = analysis.table.rows.map((r) => r.efficiency ?? 1);
    for (let i = 1; i < werte.length; i++) {
      expect(werte[i - 1]!).toBeLessThanOrEqual(werte[i]!);
    }
  });

  it('laesst die Synergie-Quote unbestimmt, statt eine zu erfinden', () => {
    // In Schritt 1 gibt es nur eine Schule und damit keine Fenster.
    const { analysis } = play([KINDLE, SMOULDER], 3000);
    expect(analysis.table.synergyRate).toBeNull();
  });

  it('summiert die Zeilen nicht zum Gesamtverlust auf', () => {
    // Ein leeres Fenster mit zwei bereiten Zaubern zaehlt in BEIDEN Zeilen —
    // je Zeile richtig, in der Summe doppelt. Der Gesamtverlust muss deshalb
    // aus der Zahl der Fenster kommen, nicht aus den Zeilen.
    const { analysis } = play([KINDLE, SMOULDER], 3000);
    const zeilenSumme = analysis.table.rows.reduce(
      (summe, row) => summe + (row.opportunities - row.casts),
      0,
    );
    expect(zeilenSumme).toBeGreaterThan(analysis.missedCastWindows);
  });
});

// --------------------------------------------------------------------------
// Die Luecke bleibt sichtbar
// --------------------------------------------------------------------------

describe('Was heute fehlt, wird ausgewiesen', () => {
  it('meldet die verpassten Unterbrechungen als nicht messbar (Issue #151)', () => {
    const { analysis } = play([KINDLE, SMOULDER], 3000);
    expect(analysis.losses.unavailable).toContain('missedInterrupts');
    // Und nicht als Quelle mit Verlust 0 — das saehe aus wie "kein Verlust".
    expect(analysis.losses.sources.map((s) => s.kind)).not.toContain('missedInterrupts');
  });
});
