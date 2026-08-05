import { describe, expect, it } from 'vitest';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { Combat } from '../../src/core/combat.js';
import { spellIndex } from '../helpers/spell-index.js';

/**
 * Der gemeinsame Helfer hat einen eigenen Fall (Issue #369).
 *
 * Sechs Prüffälle hängen an ihm. Wäre er still falsch, wären sie es alle —
 * und keiner von ihnen prüft ihn, sie *benutzen* ihn nur.
 */
describe('spellIndex', () => {
  it('löst einen bekannten Zauber auf', () => {
    const i = spellIndex('kindle');
    expect(CALCINATION.value.spells[i]?.id).toBe('kindle');
  });

  it('wirft bei einem unbekannten — statt -1 zurückzugeben', () => {
    // **Der Grund für den Wurf.** `-1` ist ein gültiger Index für „gibt es
    // nicht": Der Prüffall liefe weiter und misst nichts. Der Fehler zeigt
    // stattdessen auf den Tippfehler selbst.
    expect(() => spellIndex('gibtsnicht')).toThrow('gibt es nicht');
  });
  it('mit combat liest er DESSEN Zauber — heute dieselben (#403)', () => {
    // **Die Messung, nicht die Behauptung.** combat.spells IST school.spells
    // (combat.ts:688), und es gibt eine Schule. Der Parameter kann heute also
    // keinen Unterschied machen — dieser Fall hält fest, dass es so ist, statt
    // „allgemeiner" zu behaupten.
    const boss = bossById('test-warden');
    if (boss === undefined) throw new Error('test-warden fehlt');
    const combat = new Combat({ school: CALCINATION.value, boss });

    expect(spellIndex('kindle', combat)).toBe(spellIndex('kindle'));
    expect(combat.spells.map((s) => s.id)).toEqual(CALCINATION.value.spells.map((s) => s.id));
  });

  it('und wirft auch mit combat bei einem unbekannten', () => {
    const boss = bossById('test-warden');
    if (boss === undefined) throw new Error('test-warden fehlt');
    const combat = new Combat({ school: CALCINATION.value, boss });
    expect(() => spellIndex('gibtsnicht', combat)).toThrow('gibt es nicht');
  });

  it('mit einer TEIL-Schule liefern die beiden Fassungen Verschiedenes (#403)', () => {
    // **Der Fall, den ich für unmöglich gehalten hatte.** Ich hatte im
    // Bestand gesucht, keinen Aufrufer gefunden und daraus geschlossen, es
    // gebe keine Lage. Der Orchestrator hat stattdessen eine GEBAUT — in vier
    // Zeilen, weil `Combat` jede `SchoolDef` nimmt, nicht nur `CALCINATION`.
    //
    // „Ich finde keine" und „es gibt keine" sind zwei Aussagen.
    const boss = bossById('test-warden');
    if (boss === undefined) throw new Error('test-warden fehlt');
    const voll = CALCINATION.value;
    const teil = { ...voll, spells: voll.spells.slice(1) } as typeof voll;
    const combat = new Combat({ school: teil, boss });

    const id = voll.spells[2]?.id;
    if (id === undefined) throw new Error('die Schule hat weniger als drei Zauber');

    expect(spellIndex(id), 'ohne Kampf zählt die Schule').toBe(2);
    expect(spellIndex(id, combat), 'mit Kampf zählt DESSEN Leiste').toBe(1);
  });
});
