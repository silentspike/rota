/**
 * Eine Zeitleiste muss in ihren Wiederholungs-Rahmen passen (Issue #565).
 *
 * # Der Fall, aus dem diese Prüfung entstand
 *
 * `boss-1-1` trug `timelineLoopMs: 15000` bei Einträgen von 12000 bis 48000.
 * Der vierte Eintrag wurde damit bei 27000 fällig — **rückwärts**. Spiel und
 * Simulator beantworteten diese Lage verschieden, und der Fehler zeigte sich
 * erst als Paritätsbruch bei Takt 4800.
 *
 * Keine der beiden Seiten rechnete falsch. Die Daten waren widersprüchlich,
 * und **nichts hat sie daran gehindert**: kein Schema, kein Ladepfad, keine
 * Prüfung.
 *
 * # Warum die SPANNWEITE und nicht der Höchstwert
 *
 * Der erste Entwurf der Regel lautete `loop > max`. Er hätte `test-metronome`
 * verurteilt — Einträge 2000, 4000, 6000 bei `loop 6000`, ein tadelloser Takt
 * alle zwei Sekunden. Ein Fehlalarm ist die teurere Richtung: Eine Prüfung,
 * der man nicht traut, wird abgeschaltet.
 *
 * Richtig ist `loop > max − min`: Der letzte Eintrag eines Zyklus liegt bei
 * `0·loop + max`, der erste des nächsten bei `1·loop + min`.
 */
import { describe, expect, it } from 'vitest';
import { BOSSES, parseBoss } from '../../src/core/data.js';

/** Ein Boss-Rohdatensatz mit genau den Feldern, die die Prüfung braucht. */
function roh(loopMs: number | null, zeiten: number[]): Record<string, unknown> {
  return {
    schema: 1,
    id: 'pruef-boss',
    name: 'Prüf-Boss',
    instance: 'testing',
    hp: 1000,
    overpressureMs: 60_000,
    phases: [],
    energy: null,
    timelineLoopMs: loopMs,
    timeline: zeiten.map((atMs) => ({ atMs, mechanic: 'strike', name: 'x', damage: 1 })),
    resistances: {},
    signature: null,
  };
}

/** Die Meldungen, die auf die Zeitleisten-Regel zeigen. */
function rahmenProbleme(loopMs: number | null, zeiten: number[]): string[] {
  return parseBoss(roh(loopMs, zeiten)).problems.filter((p) => p.includes('timelineLoopMs'));
}

describe('AC1 und AC3 — widersprüchliche Rahmen werden abgelehnt', () => {
  it('die alte boss-1-1-Belegung fällt durch, mit BEIDEN Zahlen in der Meldung', () => {
    const [meldung] = rahmenProbleme(15_000, [12_000, 32_000, 48_000]);

    expect(meldung, 'die Belegung wird nicht abgelehnt').toBeDefined();
    // Beide Zahlen, nicht nur „passt nicht": Wer die Meldung liest, muss
    // wissen, was zu klein ist und wogegen.
    expect(meldung).toContain('15000');
    expect(meldung).toContain('36000');
    // Und der Ausweg, damit niemand rechnen muss.
    expect(meldung).toContain('36001');
  });
});

describe('AC4 — der Grenzfall wird abgelehnt, nicht durchgelassen', () => {
  it('loop GLEICH der Spannweite fällt durch', () => {
    // Bei Gleichheit fällt der erste Eintrag des neuen Zyklus exakt auf den
    // letzten des alten — kein Rückwärtslauf, aber zwei Ereignisse zur selben
    // Zeit. Deshalb ist die Grenze strikt.
    expect(rahmenProbleme(4000, [2000, 6000])).not.toHaveLength(0);
  });

  it('und einen Takt darüber geht durch', () => {
    // **Die Gegenprobe zum Grenzfall.** Ohne sie bliebe offen, ob die Regel
    // an der richtigen Stelle schneidet oder einfach alles ablehnt.
    expect(rahmenProbleme(4001, [2000, 6000])).toHaveLength(0);
  });
});

describe('AC2 — die Regel schwärzt keinen bestehenden Boss an', () => {
  it('test-metronome passiert sie — genau der Fall, den der erste Entwurf verurteilt hätte', () => {
    // Einträge 2000, 4000, 6000 bei loop 6000: Höchstwert 6000 ist NICHT
    // kleiner als der Rahmen, die Spannweite 4000 aber schon.
    expect(rahmenProbleme(6000, [2000, 4000, 6000])).toHaveLength(0);
  });

  it('KEIN Boss im Bestand wird abgelehnt', () => {
    // Die eigentliche Zusicherung: Die Regel läuft gegen alles, was heute
    // ausgeliefert wird. Ein Fehlalarm hier wäre teurer als eine Lücke.
    expect(BOSSES.size, 'keine Bosse geladen — dann prüft der Fall nichts').toBeGreaterThan(10);
    for (const [id, eintrag] of BOSSES) {
      const auffaellig = eintrag.problems.filter((p) => p.includes('timelineLoopMs'));
      expect(auffaellig, `${id} wird von der Zeitleisten-Regel abgelehnt`).toHaveLength(0);
    }
  });
});

describe('Was die Regel NICHT anfasst', () => {
  it('ohne Wiederholung gibt es keinen Rahmen', () => {
    expect(rahmenProbleme(null, [12_000, 48_000])).toHaveLength(0);
  });

  it('ein einzelner Eintrag hat keine Spannweite', () => {
    expect(rahmenProbleme(1000, [48_000])).toHaveLength(0);
  });
});
