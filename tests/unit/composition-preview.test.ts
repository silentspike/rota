/**
 * Die Mechanik-Vorschau (Issue #270, AC2 und AC4).
 *
 * AC4 MISST GEGEN DEN KAMPF, NICHT GEGEN SICH SELBST
 * --------------------------------------------------
 * Die Vorschau liest die DATEN (`feedsMechanic` auf `SpellDef`). Der Prueffall
 * unten misst den KAMPF: Er spielt einen Lauf und fragt den Empfaenger, wie
 * oft er einen Gewinn eingetragen hat. Zwei verschiedene Wege zur selben
 * Aussage — eine Vorschau, die ihre Zahl aus derselben Funktion holte wie ihre
 * Anzeige, waere eine Tautologie.
 *
 * WARUM NICHT DER RESSOURCENWERT
 * ------------------------------
 * Gemessen am 2026-08-05: SMOULDERs DoT-Tick gibt +1 Hitze alle 2000 ms, der
 * Verfall nimmt -1 alle 2000 ms. Dieselbe Periode — der Gewinn ist am
 * Ressourcenwert zu KEINEM Zeitpunkt und in keiner Aufloesung sichtbar. Drei
 * Messungen (Endwert, Anstiege, Verlaufsvergleich) sagten uebereinstimmend
 * "speist nicht", und alle drei waren blind.
 *
 * Deshalb `gainCount` und nicht `combat.resource`.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { previewOfLoadout } from '../../src/progression/composition-view.js';
import type { Bindings } from '../../src/progression/bindings.js';

const SPELLS = CALCINATION.value.spells;

function vorschau(bindings: Bindings): string[] {
  return previewOfLoadout(bindings, 12, SPELLS).map((z) => z.text);
}

describe('AC2 — die Vorschau nennt die Einzahler', () => {
  it('zeigt die Null, wo die Kampf-Anzeige ausblenden wuerde', () => {
    // **Die Aussage aus GAME-DESIGN 16**: "man erkennt sofort, ob die
    // Hitze-Mechanik mit nur einem Zauber traegt." Wer nur den Entlader
    // einsteckt, liest hier, dass niemand einzahlt.
    expect(vorschau({ '0': 'pyre' })).toEqual(['Heat · 0 feeding · 1 spending']);
  });

  it('zaehlt den Effekt ueber Zeit mit — SMOULDER speist ueber Ticks', () => {
    expect(vorschau({ '0': 'kindle', '1': 'smoulder', '2': 'pyre' })).toEqual([
      'Heat · 2 feeding · 1 spending',
    ]);
  });

  it('laesst "spending" weg, wenn es keinen gibt', () => {
    // Ein "· 0 spending" waere eine stumme Bewertung: Wer keinen Entlader
    // mitfuehrt, hat nicht zwangslaeufig etwas falsch gemacht.
    expect(vorschau({ '0': 'kindle' })).toEqual(['Heat · 1 feeding']);
  });

  it('kennt alle DREI Mechaniken, nicht nur die Hitze', () => {
    // Der Befund, aus dem Weg B entstand: Der Kampf fuehrt control() und
    // decayStacks(), die Auswahl kannte nur heat.
    expect(vorschau({ '0': 'bind', '1': 'stall' })).toEqual(['Control · 1 feeding · 1 spending']);
    expect(vorschau({ '0': 'canker', '1': 'fester' })).toEqual(['Decay · 1 feeding · 1 spending']);
  });

  it('sagt "no shared meter", wo es keinen Zaehler gibt', () => {
    // COAGULATION und SUBLIMATION fuehren keinen. Eine Null waere eine
    // Aussage ueber etwas, das es nicht gibt.
    expect(vorschau({ '0': 'crust' })).toEqual(['no shared meter']);
    expect(vorschau({ '0': 'vapour' })).toEqual(['no shared meter']);
  });

  it('zeigt fuer eine Schule ohne Zauber auf der Leiste GAR NICHTS', () => {
    expect(vorschau({})).toEqual([]);
    // QUENCH gehoert keiner Schule an und erzeugt deshalb keine Zeile.
    expect(vorschau({ '0': 'quench' })).toEqual([]);
  });

  it('EMPFIEHLT NICHTS — auch nicht im Wortlaut (AC5)', () => {
    const alle = [
      ...vorschau({ '0': 'pyre' }),
      ...vorschau({ '0': 'kindle' }),
      ...vorschau({ '0': 'crust' }),
    ].join(' ');
    for (const wort of ['best', 'recommend', 'optimal', 'weak', 'should', 'consider', '!']) {
      expect(alle.toLowerCase(), `"${wort}"`).not.toContain(wort);
    }
  });
});

describe('AC4 — die Zahlen stimmen mit dem Kampf ueberein', () => {
  /** Wie oft der Empfaenger im Kampf einen Gewinn eingetragen hat. */
  function gewinneImKampf(spellId: string): number {
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    combat.beginFight(0);
    combat.cast(combat.indexOf(spellId), 0);
    for (let t = TICK_MS; t <= 13_000; t += TICK_MS) combat.stepPassive(t);
    return (combat as unknown as { stacks: { gainCount: number } }).stacks.gainCount;
  }

  it('nennt genau die Zauber als Einzahler, die im Kampf einzahlen', () => {
    // **Zwei Wege zur selben Aussage.** Links die Vorschau aus den Daten,
    // rechts ein gespielter Lauf.
    for (const spell of ['kindle', 'smoulder']) {
      expect(vorschau({ '0': spell })[0], `${spell} in der Vorschau`).toContain('1 feeding');
      expect(gewinneImKampf(spell), `${spell} im Kampf`).toBeGreaterThan(0);
    }
    for (const spell of ['pyre', 'crust', 'quench']) {
      expect(gewinneImKampf(spell), `${spell} zahlt im Kampf nicht ein`).toBe(0);
    }
    expect(vorschau({ '0': 'pyre' })[0]).toContain('0 feeding');
  });

  it('MISST UEBERHAUPT etwas — die Gegenprobe', () => {
    // Ohne sie waere nicht gezeigt, dass `gewinneImKampf` Unterschiede sieht.
    expect(gewinneImKampf('kindle')).not.toBe(gewinneImKampf('pyre'));
  });
});
