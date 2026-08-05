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

/**
 * Der Rumpf von `setDamageFactor`, aus dem Quelltext geschnitten (Issue #196).
 *
 * WARUM NICHT `indexOf` UND WARUM KEINE ZEICHENZAHL
 * -------------------------------------------------
 * Hier stand `quelle.indexOf('setDamageFactor(')` — das **erste** Vorkommen.
 * Das ist die Definition nur so lange, wie die Methode **niemand aufruft**.
 * Der erste Aufrufer steht in #97 (`updateVulnerability`), und zwar 266 Zeilen
 * vor der Definition; der Wächter meldete daraufhin null Schreibstellen und
 * damit rot — an einer Datei, die weiterhin genau zwei hat.
 *
 * Ein Wächter, der beim ersten echten Nutzer umkippt, ist schlimmer als
 * keiner: Er meldet einen Fehler, den es nicht gibt, und lehrt damit, ihm
 * nicht zu glauben.
 *
 * Deshalb zwei Änderungen:
 *
 * **Der Anker liegt am Zeilenanfang.** Eine Definition beginnt eine Zeile,
 * ein Aufruf steht hinter `this.` — das trennt beide, ohne die Datei zu
 * kennen.
 *
 * **Das Ende ist die schließende Klammer, keine Zeichenzahl.** `slice(i, i +
 * 600)` traf bisher zufällig; wächst der Rumpf um einen Kommentar, schneidet
 * es mitten hinein, und der Fall wird still zu klein. Dieselbe Falle wie eine
 * `sed`-Zeilenspanne über das Funktionsende hinaus.
 */
export function definitionsRumpf(quelle: string): string {
  const anker = /^[ \t]*setDamageFactor\(/m.exec(quelle);
  if (anker === null) {
    throw new Error('setDamageFactor ist nicht mehr als Definition zu finden');
  }
  const rest = quelle.slice(anker.index);
  // Die erste Zeile, die auf Methoden-Ebene schliesst — zwei Leerzeichen und
  // eine Klammer, nichts sonst.
  const ende = /\n {2}\}/.exec(rest);
  return ende === null ? rest : rest.slice(0, ende.index);
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

    // Und sie stehen wirklich dort — im Rumpf der DEFINITION, nicht in den
    // ersten 600 Zeichen ab dem ersten Vorkommen des Namens.
    expect(schreibstellen(definitionsRumpf(quelle))).toHaveLength(2);
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

describe('Der Wächter übersteht seinen ersten Aufrufer (Issue #196)', () => {
  /** Ein Aufruf VOR der Definition — so, wie #97 ihn anlegt. */
  function mitAufrufDavor(quelle: string): string {
    const stelle = quelle.indexOf('  get resourceOverflow(): number {');
    expect(stelle, 'Ankerstelle für die Gegenprobe nicht gefunden').toBeGreaterThan(0);
    return (
      quelle.slice(0, stelle) +
      "  probe(): void {\n    this.setDamageFactor('buffs', 1);\n  }\n\n" +
      quelle.slice(stelle)
    );
  }

  it('findet die Definition auch dann, wenn ein Aufruf davorsteht', () => {
    // **Die Gegenprobe, ohne die nur belegt wäre, dass er heute grün ist.**
    // Gemessen an derselben Funktion, die der echte Fall benutzt — eine
    // nachgebaute Suche bliebe grün, während die echte kippt.
    const verändert = mitAufrufDavor(combatQuelle());
    expect(schreibstellen(definitionsRumpf(verändert))).toHaveLength(2);
  });

  it('und die alte Suche wäre daran zerbrochen — der Beleg für den Umbau', () => {
    // Ohne diesen Fall stünde die Begründung des Umbaus nur im Kommentar.
    const verändert = mitAufrufDavor(combatQuelle());
    const alterAnfang = verändert.indexOf('setDamageFactor(');
    const altesFenster = verändert.slice(alterAnfang, alterAnfang + 600);
    expect(schreibstellen(altesFenster)).toHaveLength(0);
  });

  it('meldet ehrlich, wenn die Definition wirklich verschwindet', () => {
    // Der Fall, in dem der Wächter anschlagen MUSS: Wer die Methode entfernt,
    // bekommt einen Fehler und keine leere Liste.
    const ohne = combatQuelle().replace(/^[ \t]*setDamageFactor\(/m, '  entfernt(');
    expect(() => definitionsRumpf(ohne)).toThrow('nicht mehr als Definition');
  });
});
