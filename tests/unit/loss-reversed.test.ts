/**
 * Die Fehleranalyse bei einem umgekehrten Ziel (Issue #256, AC3).
 *
 * WAS SIE VORHER SAGTE, GEMESSEN IM BROWSER
 * -----------------------------------------
 * Nach einem SIEG gegen `test-runaway` stand in der Auswertung:
 *
 *     Idle · worst spell   kindle · 15.5 s (78 %)
 *     Time lost            15.5 s (78 %)
 *
 * Beides war korrekt gemessen und trotzdem verkehrt. KINDLE ist der Zauber,
 * der bei diesem Boss 600 Ring-Punkte setzt und den Lauf 9,5 s früher kippt —
 * die Analyse riet dem Spieler zu dem, was ihn umbringt. Und bei einem Lauf
 * ohne einen Tastendruck meldete sie „100 % Zeit verloren", wo Nichtstun das
 * Bestmögliche war.
 *
 * GAME-DESIGN 15.2 sagt, was die vier Schadensquellen messen: „Sie messen alle
 * dasselbe: Schaden, den der Spieler NICHT AUSGETEILT hat." Bei diesem Boss
 * ist ausgeteilter Schaden der Weg zur Niederlage. Die Währung wechselt das
 * Vorzeichen, und damit ist die ganze Kategorie dort nicht falsch gewichtet,
 * sondern bedeutungslos.
 *
 * DIE UNTERBRECHUNGEN BLEIBEN
 * ---------------------------
 * Sie messen EINGESTECKTEN Schaden — der ist bei jedem Boss ein Fehler. Ihre
 * Zeile ist die einzige, die beide Boss-Arten unverändert teilen.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { buildSummary, type RunSummary } from '../../src/core/summary.js';
import { CombatLog } from '../../src/core/log.js';
import { analyzeRun } from '../../src/analysis/run-analysis.js';
import type { LossKind } from '../../src/analysis/loss-sources.js';

const RUNAWAY = bossById('test-runaway');
if (RUNAWAY === undefined) throw new Error('data/bosses/test-runaway.json fehlt.');

/** Ein echter Lauf, keine von Hand gesetzte Kurzfassung. */
function spielen(boss: typeof FIRST_CRUCIBLE.value, spellId?: string, nurAlleMs = 0): RunSummary {
  const combat = new Combat({ school: CALCINATION.value, boss });
  const index = spellId === undefined ? -1 : combat.indexOf(spellId);
  combat.beginFight(0);
  let letzte = 0;
  let casts = 0;
  // **Die Leerzeit wird MITGEMESSEN, nicht auf null gesetzt.** Mein erster
  // Entwurf übergab `lostTimeMs: 0` — dann meldete der Gegenfall am
  // gewöhnlichen Boss gar keine Schadensquelle, weil es nichts zu verlieren
  // gab, und er hätte den Unterschied nicht zeigen können. Die Zusicherung
  // unten hat das gefangen.
  let leerMs = 0;
  for (let t = 0; t <= 25_000; t += TICK_MS) {
    combat.stepPassive(t);
    letzte = t;
    if (combat.checkDefeat(t)) break;
    const willWirken = nurAlleMs <= 0 || t % nurAlleMs === 0;
    if (index >= 0 && willWirken && combat.gcd.isReady(t) && combat.isReady(index, t)) {
      combat.cast(index, t);
      casts++;
    } else if (combat.gcd.isReady(t)) {
      leerMs += TICK_MS;
    }
  }
  return buildSummary({
    combat,
    log: new CombatLog(),
    outcome: combat.fightState === 1 ? 'victory' : 'overpressure',
    victory: combat.fightState === 1,
    elapsedMs: combat.endedAt >= 0 ? combat.endedAt : letzte,
    remainingMs: 0,
    casts,
    lostTimeMs: leerMs,
    lostTimeWithReadyMs: leerMs,
    lostTimeNothingReadyMs: 0,
    idlePerSpellMs: new Float64Array(combat.spells.length),
    synergyUsed: 0,
    synergyAvailable: 0,
  });
}

function analysieren(summary: RunSummary): readonly LossKind[] {
  const analyse = analyzeRun({
    summary,
    gcdMs: 1500,
    spells: [],
    synergyBonusDamage: 0,
    missedInterrupts: null,
    damagePerLandedBossCast: 0,
    bossHealed: 0,
  });
  return analyse.losses.sources.map((s) => s.kind);
}

describe('Bei einem umgekehrten Ziel zählt der eigene Schaden', () => {
  it('nennt ihn als Verlustquelle, mit dem Ring als Bezug', () => {
    const summary = spielen(RUNAWAY, 'kindle');
    expect(summary.reversed, 'sonst prüft der Fall den falschen Boss').toBe(true);
    expect(summary.playerDamage, 'gemessen: KINDLE setzt 600 Ring-Punkte').toBe(600);

    const analyse = analyzeRun({
      summary,
      gcdMs: 1500,
      spells: [],
      synergyBonusDamage: 0,
      missedInterrupts: null,
      damagePerLandedBossCast: 0,
      bossHealed: 0,
    });

    const eigen = analyse.losses.sources.find((s) => s.kind === 'selfInflicted');
    expect(eigen, 'die Quelle muss dastehen').toBeDefined();
    expect(eigen?.damage).toBe(600);
    // **Der Anteil bezieht sich auf den ganzen Ring**, nicht auf den
    // ausgeteilten Schaden: 600 von 1000 sind 60 %. Ohne den ausdrücklichen
    // Bezug stünde die einzige Quelle bei 100 %, egal wie klein sie ist.
    expect(eigen?.share).toBeCloseTo(0.6, 5);
    expect(eigen?.detail).toContain('60 %');
  });

  it('LÄSST DIE VIER SCHADENSQUELLEN WEG — sie messen die andere Währung', () => {
    // **MIT LÜCKEN GESPIELT, und das ist der ganze Punkt.** Mein erster
    // Entwurf drückte KINDLE auf Cooldown: Dann gibt es keine Leerzeit, die
    // vier Quellen erscheinen ohnehin nicht, und der Fall war grün — egal was
    // die Umsetzung tat. Aufgefallen an der Gegenprobe: Ich habe die
    // Umstellung zurückgedreht, und dieser Fall blieb als einziger grün.
    //
    // Zum dritten Mal heute dieselbe Frage: Könnte er den Unterschied
    // überhaupt zeigen?
    const summary = spielen(RUNAWAY, 'kindle', 3000);
    expect(summary.lostTimeMs, 'ohne Leerzeit gäbe es die vier gar nicht').toBeGreaterThan(0);

    const arten = analysieren(summary);
    for (const weg of ['missedCasts', 'idle', 'missedSynergies', 'lostDotTicks'] as const) {
      expect(arten, `${weg} rät bei diesem Boss zu mehr Schaden`).not.toContain(weg);
    }
    expect(arten, 'die eine richtige Quelle steht dafür da').toContain('selfInflicted');
  });

  it('meldet dem, der nichts gedrückt hat, KEINEN Fehler', () => {
    // **Der Fall, an dem die ganze Etappe hängt.** Vorher stand hier der volle
    // Ring-Verlust als Leistung des Spielers — 1020 Punkte für einen Lauf ohne
    // einen einzigen Tastendruck.
    const summary = spielen(RUNAWAY);
    expect(summary.damage, 'der Ring ist gefallen').toBe(1020);
    expect(summary.playerDamage, 'aber nicht durch ihn').toBe(0);

    const arten = analysieren(summary);
    expect(arten, 'eine Quelle mit 0 wird gar nicht erst gezeigt').toHaveLength(0);
  });
});

describe('Bei einem gewöhnlichen Boss bleibt alles, wie es war', () => {
  it('nennt die Schadensquellen und NICHT den eigenen Schaden', () => {
    // Der Gegenfall. Ohne ihn wäre nicht gezeigt, dass die Umstellung nur den
    // einen Boss-Typ trifft — sie könnte die Analyse überall umgestellt haben.
    // **Mit Lücken gespielt.** Wer KINDLE auf Cooldown drückt, lässt nichts
    // liegen — dann gibt es keine Verlustquelle, und der Fall zeigte nichts.
    // Genau das war mein erster Entwurf, und die Zusicherung unten hat es
    // gefangen.
    const summary = spielen(FIRST_CRUCIBLE.value, 'kindle', 3000);
    expect(summary.reversed).toBe(false);
    expect(summary.lostTimeMs, 'ohne Leerzeit gäbe es nichts zu melden').toBeGreaterThan(0);

    const arten = analysieren(summary);
    expect(arten).not.toContain('selfInflicted');
    // Mindestens eine der vier muss vorkommen, sonst prüfte der Fall nichts:
    // Eine leere Liste enthält auch keine verbotene Art.
    expect(
      arten.some((a) => a === 'missedCasts' || a === 'idle' || a === 'lostDotTicks'),
      'sonst könnte der Fall den Unterschied nicht zeigen',
    ).toBe(true);
  });
});

describe('Die Zeile je Zauber trägt den Ring-Schaden', () => {
  it('nennt für jeden Zauber, wie viel Ring er gekostet hat', () => {
    // **Die Datengrundlage des Detailbereichs** (Issue #256). Er stand bis
    // zum 2026-08-05 auch bei einem umgekehrten Ziel als „Idle time per
    // spell" da — dieselbe verkehrte Aussage wie die Zeile darüber, nur
    // hinter einem Knopf, den ich beim Umbau nicht gedrückt hatte.
    //
    // Die Anzeige selbst braucht ein Dokument und ist hier nicht prüfbar; was
    // sie zeigt, steht in dieser Zahl.
    const summary = spielen(RUNAWAY, 'kindle', 3000);
    const kindle = summary.idlePerSpell.find((e) => e.spellId === 'kindle');
    expect(kindle?.damage, 'KINDLE senkt den Ring').toBeGreaterThan(0);
    expect(kindle?.damage).toBe(summary.playerDamage);
  });

  it('lässt einen Stabilisierer bei NULL — er bucht keinen Schaden', () => {
    // CRUST drängt den Ring über `undoProgress` zurück, statt Schaden zu
    // buchen. Stünde er hier mit einer Zahl, wiese die Liste ausgerechnet den
    // Zauber als schädlich aus, der hilft.
    const summary = spielen(RUNAWAY, 'crust');
    const crust = summary.idlePerSpell.find((e) => e.spellId === 'crust');
    expect(crust?.casts, 'er muss gewirkt worden sein, sonst prüft das nichts').toBeGreaterThan(0);
    expect(crust?.damage).toBe(0);
  });

  it('summiert sich auf den Anteil des Spielers', () => {
    // Sonst zeigte die Liste etwas anderes als die Zeile darüber, und der
    // Spieler hätte zwei Zahlen, die einander widersprechen.
    const summary = spielen(RUNAWAY, 'kindle', 3000);
    const summe = summary.idlePerSpell.reduce((s, e) => s + e.damage, 0);
    expect(summe).toBe(summary.playerDamage);
  });
});
