/**
 * Nur EINE Stelle darf die Schadensfaktoren schreiben (Issue #196, AC4).
 *
 * WARUM DAS EIN PRUEFFALL SEIN MUSS UND KEIN VORSATZ
 * -------------------------------------------------
 * Die Krit-Fassung wird beim Setzen eines Faktors nachgezogen. Wer kuenftig
 * `this.factors[...] = ...` woanders hinschreibt, umgeht das Nachziehen -- und
 * der Fehler ist STILL: Der gewoehnliche Treffer bleibt richtig, nur der
 * kritische rechnet mit dem alten Wert. Ein Prueffall faengt das erst, wenn er
 * Krit UND einen veraenderlichen Faktor gemeinsam abdeckt.
 *
 * "Daran denken" ist keine Absicherung. Diese Pruefung liest den Quelltext und
 * schlaegt bei jeder weiteren Schreibstelle an -- dasselbe Muster wie
 * tests/unit/log-kinds-source.test.ts, das den gemeinsamen Nummernraum
 * bewacht.
 *
 * WAS SIE NICHT KANN: Sie prueft Text, nicht Bedeutung. Wer die Faktoren
 * ueber eine Hilfsvariable schreibt (`const f = this.factors; f.buffs = 2`),
 * kommt daran vorbei. Das ist die Grenze, und sie steht hier, damit niemand
 * die Pruefung fuer mehr haelt, als sie ist.
 *
 * Beide SCHREIBWEISEN sind dagegen erfasst -- mit Klammer und mit Punkt. Mein
 * erster Entwurf erfasste nur die Klammer-Form, und die Gegenprobe zeigte, dass
 * `this.factors.talents = 1` glatt durchging.
 */
import { describe, expect, it } from 'vitest';

/**
 * Der Quelltext von `combat.ts`, ueber Vite geladen.
 *
 * `node:fs` braeuchte @types/node in der tsconfig -- eine geteilte
 * Konfigurationsaenderung fuer einen einzelnen Prueffall.
 */
const QUELLE = import.meta.glob('/src/core/combat.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function combatQuelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length === 0) {
    throw new Error('combat.ts nicht geladen — die Pruefung wuerde nichts messen');
  }
  return text;
}

/** Zeilen, die in `factors` oder `factorsOnCrit` schreiben. */
function schreibstellen(quelle: string): string[] {
  return (
    quelle
      .split('\n')
      .map((zeile, i) => ({ zeile: zeile.trim(), nummer: i + 1 }))
      // BEIDE Schreibweisen. Mein erster Entwurf erfasste nur die Klammer-Form
      // -- die Gegenprobe zeigte, dass `this.factors.talents = 1` glatt
      // durchging. Ein Waechter, der die Haelfte sieht, ist gefaehrlicher als
      // keiner: Er meldet gruen und man glaubt ihm.
      .filter(({ zeile }) =>
        /this\.(factors|factorsOnCrit)(\[[^\]]+\]|\.[A-Za-z]\w*)\s*=[^=]/.test(zeile),
      )
      .map(({ zeile, nummer }) => `${nummer}: ${zeile}`)
  );
}

describe('Die Schadensfaktoren haben genau einen Schreiber', () => {
  it('findet den Quelltext ueberhaupt', () => {
    // Ohne diese Zusicherung liefe die Pruefung auch dann durch, wenn das
    // Muster nichts faende -- und belegte nichts.
    const quelle = combatQuelle();
    expect(quelle.length).toBeGreaterThan(1000);
    expect(quelle).toContain('setDamageFactor');
  });

  it('schreibt an genau ZWEI Stellen, und beide stehen in setDamageFactor', () => {
    const quelle = combatQuelle();
    const stellen = schreibstellen(quelle);

    // Zwei: die Basis und die Krit-Fassung, beide in derselben Methode.
    expect(stellen, stellen.join('\n')).toHaveLength(2);

    // Und sie stehen wirklich dort. Gemessen am Abstand zur Methode, nicht
    // vermutet: Der Rumpf ist kurz, beide Zeilen liegen dicht dahinter.
    const beginn = quelle.indexOf('setDamageFactor(');
    expect(beginn).toBeGreaterThan(0);
    const rumpf = quelle.slice(beginn, beginn + 600);
    expect(schreibstellen(rumpf)).toHaveLength(2);
  });

  it('haelt die Anzahl fest, damit eine dritte Stelle auffaellt', () => {
    // Der eigentliche Zweck. Wer eine weitere Schreibstelle anlegt, laesst
    // diese Zahl steigen -- und muss sich fragen, ob er die Krit-Fassung
    // nachgezogen hat.
    expect(schreibstellen(combatQuelle())).toEqual([
      expect.stringContaining('this.factors[kind] = value;'),
      expect.stringContaining('this.factorsOnCrit[kind] = value;'),
    ]);
  });
});
