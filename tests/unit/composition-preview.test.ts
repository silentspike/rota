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
  /**
   * Was der Kampf fuer diese Mechanik zaehlt, nachdem der Zauber gewirkt hat.
   *
   * **ALLE DREI, nicht nur die Hitze** — und das war ein Befund am eigenen
   * Pruefstand. Bis zum 2026-08-05 fragte diese Funktion allein
   * `stacks.gainCount`, den HITZE-Zaehler, und die beiden Schleifen unten
   * nannten fuenf Zauber aus der Kalzination. `bind` und `canker` kamen nicht
   * vor.
   *
   * Damit belegte AC4 genau den Teil, der schon VORHER stimmte: Die Hitze lief
   * lange; Kontrolle und Zersetzung sind in dieser Etappe dazugekommen. Der
   * Nachweis deckte das Alte ab und liess das Neue ungeprueft.
   *
   * Gefunden hat es eine Gegenprobe des Orchestrators: Er hat den
   * Kontroll-Zweig in `feedsMechanic` abgeschaltet, und nur EIN Fall wurde rot
   * — der aus AC2. AC4 blieb gruen, obwohl die Vorschau die Kontrolle
   * falsch gezaehlt haette.
   *
   * Die drei Zaehler stehen an verschiedenen Stellen, weil sie verschieden
   * arbeiten:
   *
   * | Mechanik | gemessen an | warum nicht anders |
   * | --- | --- | --- |
   * | `heat` | `stacks.gainCount` | Tick und Verfall haben dieselbe Periode — der Vorrat ist blind, und ein Hoechstwert waere bei SMOULDER null |
   * | `control` | Hoechstwert von `combat.control` | |
   * | `decay` | Hoechstwert von `combat.decayStacks` | |
   *
   * **DER HOECHSTWERT, NICHT DER ENDWERT** — und das war der zweite Befund an
   * dieser Funktion. Mein erster Entwurf las `combat.decayStacks` nach 13
   * Sekunden und bekam null. Gemessen:
   *
   *     DECAY 1s:1 2s:1 ... 11s:1 12s:0 13s:0
   *
   * Der Stapel verfaellt nach zwoelf Sekunden. Die Frage lautet aber nicht
   * „steht der Zaehler am Ende hoch", sondern „hat der Zauber ihn bewegt" —
   * und die beantwortet nur der Hoechstwert.
   *
   * Dieselbe Lehre wie bei der Hitze, eine Stufe frueher: Ein Endwert misst,
   * was uebrig ist, nicht was geschehen ist.
   */
  function zaehlerImKampf(spellId: string, mechanicId: string): number {
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    combat.beginFight(0);
    combat.cast(combat.indexOf(spellId), 0);
    let hoechst = 0;
    for (let t = TICK_MS; t <= 13_000; t += TICK_MS) {
      combat.stepPassive(t);
      const jetzt = mechanicId === 'control' ? combat.control : combat.decayStacks;
      if (jetzt > hoechst) hoechst = jetzt;
    }
    if (mechanicId === 'control' || mechanicId === 'decay') return hoechst;
    return (combat as unknown as { stacks: { gainCount: number } }).stacks.gainCount;
  }

  /** Welche Zauber die Vorschau als Einzahler fuehrt, und welche nicht. */
  const FAELLE: readonly { spell: string; mechanic: string; speist: boolean }[] = [
    { spell: 'kindle', mechanic: 'heat', speist: true },
    { spell: 'smoulder', mechanic: 'heat', speist: true },
    { spell: 'pyre', mechanic: 'heat', speist: false },
    { spell: 'crust', mechanic: 'heat', speist: false },
    { spell: 'quench', mechanic: 'heat', speist: false },
    { spell: 'bind', mechanic: 'control', speist: true },
    { spell: 'stall', mechanic: 'control', speist: false },
    { spell: 'canker', mechanic: 'decay', speist: true },
    { spell: 'fester', mechanic: 'decay', speist: false },
  ];

  it('nennt genau die Zauber als Einzahler, die im Kampf einzahlen', () => {
    // **Zwei Wege zur selben Aussage.** Links die Vorschau aus den Daten,
    // rechts ein gespielter Lauf — fuer JEDE der drei Mechaniken.
    for (const fall of FAELLE) {
      const zeile = vorschau({ '0': fall.spell })[0] ?? '';
      const imKampf = zaehlerImKampf(fall.spell, fall.mechanic);
      if (fall.speist) {
        expect(zeile, `${fall.spell} in der Vorschau`).toContain('1 feeding');
        expect(imKampf, `${fall.spell} im Kampf (${fall.mechanic})`).toBeGreaterThan(0);
      } else {
        expect(imKampf, `${fall.spell} zahlt im Kampf nicht ein`).toBe(0);
        // Wer eine Zeile bekommt, aber nicht einzahlt, steht dort bei null.
        if (zeile.length > 0 && !zeile.startsWith('no shared meter')) {
          expect(zeile, `${fall.spell} in der Vorschau`).toContain('0 feeding');
        }
      }
    }
  });

  it('DECKT ALLE DREI MECHANIKEN AB — sonst belegt der Fall ein Drittel', () => {
    // Die Zusicherung gegen genau den Fehler, der hier stand: Ein Nachweis
    // ueber fuenf Kalzinations-Zauber sagt nichts ueber Kontrolle und
    // Zersetzung — und das sind die beiden NEUEN.
    const gemessen = [...new Set(FAELLE.map((f) => f.mechanic))].sort();
    expect(gemessen).toEqual(['control', 'decay', 'heat']);
    for (const mechanic of gemessen) {
      expect(
        FAELLE.some((f) => f.mechanic === mechanic && f.speist),
        `${mechanic} braucht einen Einzahler`,
      ).toBe(true);
      expect(
        FAELLE.some((f) => f.mechanic === mechanic && !f.speist),
        `${mechanic} braucht einen Nicht-Einzahler`,
      ).toBe(true);
    }
  });

  it('MISST UEBERHAUPT etwas — die Gegenprobe, je Mechanik', () => {
    // Ohne sie waere nicht gezeigt, dass die Zaehler Unterschiede sehen.
    expect(zaehlerImKampf('kindle', 'heat')).not.toBe(zaehlerImKampf('pyre', 'heat'));
    expect(zaehlerImKampf('bind', 'control')).not.toBe(zaehlerImKampf('stall', 'control'));
    expect(zaehlerImKampf('canker', 'decay')).not.toBe(zaehlerImKampf('fester', 'decay'));
  });
});
