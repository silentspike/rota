/**
 * Nur EINE Stelle darf den Fortschritt buchen (Issue #218).
 *
 * WARUM DAS EIN PRÜFFALL SEIN MUSS UND KEIN VORSATZ
 * -------------------------------------------------
 * Bis #218 erhöhten **zwei** Stellen `damageDealtValue`: die Wirkung und der
 * Brand-Tick. Mit dem Unverwundbarkeits-Fenster (11.2, Baustein 4) hätten
 * beide dieselbe Sonderregel einzeln kennen müssen — und der Fehler wäre
 * STILL: Der gewöhnliche Schaden respektierte das Fenster, der Tick nicht.
 * Gesehen hätte es niemand, außer jemand baut zufällig einen Boss mit Fenster
 * **und** DoT.
 *
 * Das ist wörtlich die Bauart, aus der der Krit-Fehler in #196 entstand: zwei
 * Schreibstellen, eine kennt die Sonderregel, die andere nicht.
 *
 * WAS SIE NICHT KANN: Sie prüft Text, nicht Bedeutung. Wer den Fortschritt
 * über eine Hilfsvariable schreibt, kommt daran vorbei. Das ist die Grenze,
 * und sie steht hier, damit niemand die Prüfung für mehr hält, als sie ist.
 *
 * ZWEI FALLEN, DIE HIER BEWUSST NICHT WIEDERKEHREN
 * ------------------------------------------------
 * Der Schwester-Wächter `crit-factors-source.test.ts` hatte beide, und beide
 * wurden am 2026-08-05 repariert:
 *
 * - **kein `indexOf` als Anfang.** Es findet das erste Vorkommen, und das ist
 *   die Definition nur so lange, wie niemand die Methode aufruft.
 * - **keine Zeichenzahl als Ende.** `slice(i, i + 600)` trifft zufällig;
 *   wächst der Rumpf um einen Kommentar, schneidet es mitten hinein und der
 *   Fall wird still zu klein.
 *
 * Hier wird deshalb über die **ganze Datei** gezählt und die Zugehörigkeit
 * über den Methodennamen geprüft, nicht über eine Textspanne.
 */
import { describe, expect, it } from 'vitest';

/**
 * Der Quelltext von `combat.ts`, über Vite geladen.
 *
 * `node:fs` bräuchte @types/node in der tsconfig — eine geteilte
 * Konfigurationsänderung für einen einzelnen Prüffall.
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

/**
 * Zeilen, die den Fortschritt schreiben.
 *
 * Beide Richtungen: Das Fenster verhindert das Buchen, die Boss-Heilung dreht
 * zurück. Wer eine dritte Stelle anlegt, umgeht eine der beiden Regeln.
 */
export function fortschrittsSchreiber(quelle: string): string[] {
  return quelle
    .split('\n')
    .map((zeile, i) => ({ zeile: zeile.trim(), nummer: i + 1 }))
    .filter(({ zeile }) => /this\.damageDealtValue\s*[+-]?=[^=]/.test(zeile))
    .map(({ zeile, nummer }) => `${nummer}: ${zeile}`);
}

/**
 * In welcher Methode eine Zeile steht — über die Einrückung ermittelt.
 *
 * Eine Methode beginnt auf Klassen-Ebene (zwei Leerzeichen) und endet bei der
 * nächsten Zeile derselben Ebene. Das ist unabhängig davon, wie lang sie ist,
 * und damit nicht die Zeichenzahl-Falle von `crit-factors-source`.
 */
export function methodeVon(quelle: string, zeilennummer: number): string {
  const zeilen = quelle.split('\n');
  for (let i = zeilennummer - 1; i >= 0; i--) {
    const zeile = zeilen[i] ?? '';
    const treffer = /^ {2}(?:private |public |protected )?(?:get |set )?([A-Za-z_]\w*)\s*\(/.exec(
      zeile,
    );
    if (treffer !== null) return treffer[1] as string;
  }
  return '(keine Methode)';
}

describe('Der Fortschritt hat genau einen Schreiber', () => {
  it('findet den Quelltext ueberhaupt', () => {
    // Ohne diese Zusicherung liefe die Pruefung auch dann durch, wenn das
    // Muster nichts faende — und belegte nichts.
    const quelle = combatQuelle();
    expect(quelle.length).toBeGreaterThan(1000);
    expect(quelle).toContain('bookProgress');
  });

  it('schreibt an genau VIER Stellen, und jede steht dort, wo sie hingehoert', () => {
    const quelle = combatQuelle();
    const stellen = fortschrittsSchreiber(quelle);

    // **Vier, nicht zwei.** Mein erster Entwurf erwartete zwei — das Buchen
    // und das Zurueckdrehen — und uebersah die zwei legitimen: das
    // Zuruecksetzen und den Werkzeug-Setter. Gemessen statt vermutet; ein
    // Waechter, der an der Datei von heute schon falsch liegt, ist keiner.
    expect(stellen, stellen.join('\n')).toHaveLength(4);

    const methoden = stellen.map((eintrag) =>
      methodeVon(quelle, Number.parseInt(eintrag.split(':')[0] as string, 10)),
    );
    // Die vier und ihre Rollen:
    //   bookProgress        bucht — die EINZIGE Stelle, die das Fenster kennt
    //   undoProgress        dreht zurueck — die EINZIGE Stelle mit der
    //                       Untergrenze bei null
    //   reset               setzt auf null, kein Buchen
    //   setBossHpRemaining  Werkzeug (19.2), setzt einen Stand von aussen
    //
    // **Hier stand bis #247 `healBoss`.** Die Zahl ist dieselbe geblieben, die
    // STELLE ist umgezogen: Mit Baustein 6 gibt es einen zweiten Grund
    // zurueckzudrehen — der Spieler haelt die Reaktion —, und beide brauchen
    // dieselbe Untergrenze. `healBoss` ruft jetzt `undoProgress`, `stabilise`
    // ebenfalls.
    //
    // Der Waechter hat das gemeldet, und das ist sein Zweck: Wer eine
    // Schreibstelle verschiebt, muss es hier ausdruecklich sagen. Was er
    // verbietet, ist die ZAHL zu erhoehen, um eine neue Stelle durchzuwinken.
    expect([...methoden].sort()).toEqual([
      'bookProgress',
      'reset',
      'setBossHpRemaining',
      'undoProgress',
    ]);
  });

  it('haelt die Anzahl fest, damit eine dritte Stelle auffaellt', () => {
    // Der eigentliche Zweck. Wer eine weitere Schreibstelle anlegt, laesst
    // diese Zahl steigen — und muss sich fragen, ob er das Fenster und die
    // Heilung dort mitgedacht hat.
    const stellen = fortschrittsSchreiber(combatQuelle());
    expect(stellen.some((s) => s.includes('+='))).toBe(true);
    expect(stellen.some((s) => s.includes('-='))).toBe(true);
  });
});

describe('Der Waechter schlaegt bei einer zweiten Buchungsstelle an', () => {
  /** Eine Schreibstelle in einer Methode, die keine Fortschritts-Regel ist. */
  function mitZweiterStelle(quelle: string): string {
    const marke = '  beginFight(timeMs: number): void {';
    expect(quelle.includes(marke), 'Ankerstelle fuer die Gegenprobe').toBe(true);
    return quelle.replace(marke, `${marke}\n    this.damageDealtValue += 1;`);
  }

  it('zaehlt sie mit — sonst waere nur belegt, dass er heute gruen ist', () => {
    // **Die Gegenprobe, ohne die der Waechter nichts wert waere.** Gemessen an
    // derselben Funktion, die der echte Fall benutzt: Eine nachgebaute Suche
    // bliebe gruen, waehrend die echte kippt.
    const verändert = mitZweiterStelle(combatQuelle());
    expect(fortschrittsSchreiber(verändert)).toHaveLength(5);
  });

  it('und meldet, in WELCHER Methode sie steht', () => {
    // Die Zahl allein sagt "irgendwo eine zu viel". Der Name sagt, wo.
    const verändert = mitZweiterStelle(combatQuelle());
    const stellen = fortschrittsSchreiber(verändert);
    const methoden = stellen.map((eintrag) =>
      methodeVon(verändert, Number.parseInt(eintrag.split(':')[0] as string, 10)),
    );
    expect(methoden).toContain('beginFight');
  });

  it('meldet ehrlich, wenn die Buchung ganz verschwindet', () => {
    // Die Gegenrichtung: Ein Waechter, der bei NULL Schreibstellen schweigt,
    // waere nur anders blind.
    const quelle = combatQuelle();
    const ohne = quelle.replaceAll('this.damageDealtValue +=', 'this.woanders +=');
    const stellen = fortschrittsSchreiber(ohne);
    expect(stellen.length).toBeLessThan(fortschrittsSchreiber(quelle).length);
    const methoden = stellen.map((eintrag) =>
      methodeVon(ohne, Number.parseInt(eintrag.split(':')[0] as string, 10)),
    );
    expect(methoden).not.toContain('bookProgress');
  });
});
