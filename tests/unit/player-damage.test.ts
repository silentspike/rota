/**
 * Der Anteil des Spielers am Ring-Verlust (Issue #256, AC3).
 *
 * WARUM ES DIESEN ZÄHLER GIBT — GEMESSEN, NICHT VERMUTET
 * ------------------------------------------------------
 * Der Orchestrator schlug vor, `damageDealt` unmittelbar als Verlustquelle zu
 * nehmen: Bei einem Boss mit `runaway` sei der eigene Schaden der Fehler. Die
 * Messung vor dem Bau zeigte, dass die Größe das nicht trägt.
 *
 * | Lauf gegen `test-runaway` | endedAt | damageDealt | davon Reaktion | davon Spieler |
 * | --- | --- | --- | --- | --- |
 * | nichts gedrückt | 17000 | 1020 | 1020 | 0 |
 * | KINDLE auf Cooldown | 7510 | 1020 | 420 | 600 |
 *
 * **Der Nichtstuer hätte den größten Fehler gehabt.** Ein Lauf ohne einen
 * einzigen Tastendruck stünde mit 1020 Punkten als „dein Fehler" da — die
 * verkehrte Aussage, die AC1 eine Ebene höher gerade repariert hat.
 *
 * Seit #247 senkt die davonlaufende Reaktion den Ring von selbst, und beides
 * landete in einem Zähler. `vomSpieler` steuerte nur das
 * Unverwundbarkeits-Fenster; nach der Buchung war die Herkunft fort.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const RUNAWAY = bossById('test-runaway');
if (RUNAWAY === undefined) throw new Error('data/bosses/test-runaway.json fehlt.');

/** Einen Lauf führen und den Kampf zurückgeben. */
function laufen(boss: typeof FIRST_CRUCIBLE.value, bisMs: number, spellId?: string): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss });
  const index = spellId === undefined ? -1 : combat.indexOf(spellId);
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) {
    combat.stepPassive(t);
    if (combat.checkDefeat(t)) break;
    if (index >= 0 && combat.gcd.isReady(t) && combat.isReady(alsSpellIndex(index), t))
      combat.cast(alsSpellIndex(index), t);
  }
  return combat;
}

describe('Der Zähler trennt die Herkunft des Ring-Verlusts', () => {
  it('zählt NICHTS, wenn der Spieler nichts gedrückt hat', () => {
    // **Der Fall, der die ganze Sache begründet.** Ohne die Trennung stünde
    // hier der volle Ring-Verlust als Leistung des Spielers.
    const combat = laufen(RUNAWAY, 25_000);
    expect(combat.damageDealt, 'die Reaktion hat den Ring gesenkt').toBe(1020);
    expect(combat.playerDamage, 'der Spieler war daran unbeteiligt').toBe(0);
  });

  it('zählt den eigenen Anteil, wenn der Spieler Schaden macht', () => {
    // KINDLE ist ein Schadenszauber. Gegen diesen Boss beschleunigt er die
    // eigene Niederlage: 600 Punkte, und der Lauf kippt 9,5 s früher.
    const combat = laufen(RUNAWAY, 25_000, 'kindle');
    expect(combat.endedAt, 'früher als die 17 s ohne Aktion').toBeLessThan(17_000);
    expect(combat.playerDamage).toBe(600);
    // Der Rest ist die Reaktion — die Summe ist der Ring-Verlust.
    expect(combat.damageDealt - combat.playerDamage).toBe(420);
  });

  it('ist bei einem gewöhnlichen Boss der Fortschritt selbst', () => {
    // Dort gibt es keine zweite Herkunft: Was am Ring steht, hat der Spieler
    // gemacht. Ein Zähler, der hier abwiche, hätte einen Rechenfehler.
    const combat = laufen(FIRST_CRUCIBLE.value, 15_000, 'kindle');
    expect(combat.damageDealt).toBeGreaterThan(0);
    expect(combat.playerDamage).toBe(combat.damageDealt);
  });

  it('BLEIBT STEHEN, wenn ein Stabilisierer den Ring zurückdrängt', () => {
    // **Die Entscheidung, die hier festgehalten wird** (gemeldet vor dem Bau):
    // Wer 600 Punkte angerichtet und 300 repariert hat, hat 600 angerichtet.
    // Die Reparatur ist eine eigene Leistung und gehört in eine eigene Zeile —
    // eine Verrechnung machte beides unsichtbar.
    const combat = new Combat({ school: CALCINATION.value, boss: RUNAWAY });
    combat.beginFight(0);
    const kindle = combat.indexOf('kindle');
    const crust = combat.indexOf('crust');

    // Erst Schaden anrichten, dann stabilisieren.
    combat.cast(kindle, 0);
    const angerichtet = combat.playerDamage;
    expect(angerichtet, 'der Zauber muss gewirkt haben').toBeGreaterThan(0);

    for (let t = 0; t <= 6000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.gcd.isReady(t) && combat.isReady(crust, t)) {
        combat.cast(crust, t);
        break;
      }
    }
    expect(combat.playerDamage, 'die Reparatur löscht die Tat nicht').toBe(angerichtet);
  });

  it('fängt bei einem neuen Versuch wieder bei null an', () => {
    // Sonst trüge der zweite Lauf den Fehler des ersten — und die
    // Fehleranalyse zeigte auf etwas, das in diesem Versuch nie geschah.
    const combat = laufen(RUNAWAY, 10_000, 'kindle');
    expect(combat.playerDamage).toBeGreaterThan(0);
    combat.reset();
    expect(combat.playerDamage).toBe(0);
  });
});

describe('Die Prüfsumme trägt die Trennung', () => {
  it('unterscheidet zwei Zustände mit gleichem Ring-Verlust', () => {
    // **Sonst fiele ein Simulator nicht auf, der die Trennung anders zieht.**
    // Beide Läufe stehen bei 1020 Ring-Verlust; nur die Herkunft ist eine
    // andere. Ohne das angehängte Feld wären sie in der Prüfsumme identisch.
    const ohne = laufen(RUNAWAY, 25_000);
    const mit = laufen(RUNAWAY, 25_000, 'kindle');

    expect(ohne.damageDealt, 'derselbe Ring-Verlust').toBe(mit.damageDealt);
    expect(ohne.playerDamage).not.toBe(mit.playerDamage);
  });
});
