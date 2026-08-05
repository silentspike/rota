/**
 * Die Farbe je Schule (GAME-DESIGN 9.3, Issue #270).
 *
 * DAS DESIGN NENNT WORTE, KEINE WERTE
 * -----------------------------------
 * | Schule | Design | hier |
 * | --- | --- | --- |
 * | CALCINATION | rot-orange | `#c9702f` |
 * | FIXATION | weiß-blau | `#6f9fcc` |
 * | SUBLIMATION | gelb-gold | `#bd9130` |
 * | COAGULATION | grün-türkis | `#35997e` |
 * | PUTREFACTION | schwarz-violett | `#9b6fd0` |
 *
 * **CALCINATION ist nicht gewählt, sondern vorgefunden.** Der Wert stand
 * bereits als `SCHOOL_EDGE_READY` in `spellbar.ts` und bleibt dort, was er
 * ist. Er legt damit nicht nur eine Farbe fest, sondern das GEWICHT aller
 * fünf: Gegen den Grund des Player Home (`#1a1613`) hat er 5,00:1, und die
 * vier anderen sind auf denselben Korridor gelegt statt auf maximalen
 * Kontrast.
 *
 *   #c9702f 5,00 · #6f9fcc 6,43 · #bd9130 6,21 · #35997e 5,14 · #9b6fd0 4,78
 *
 * Läge eine bei 10:1 und CALCINATION bei 5:1, wäre eine Schule optisch
 * dominant, ohne dass es jemand entschieden hätte.
 *
 * SCHWARZ-VIOLETT IST EIN ZIELKONFLIKT, KEIN ABSTIMMUNGSPROBLEM
 * -------------------------------------------------------------
 * Das Design warnt: „Schwarz-violett braucht auf dunklem Grund sorgfältige
 * Abstimmung." Gemessen gegen `#1a1613`:
 *
 *   #5b3a8e 2,09 · #6d46a5 2,63 · #7a52b5 3,17 · #8d5fc4 3,91 · #9b6fd0 4,78
 *
 * Alles unter etwa `#9060c8` ist als Text unlesbar. `#9b6fd0` ist hell-violett
 * und trifft die Design-Farbe damit NICHT — die Abweichung ist dem
 * Auftraggeber vorgelegt. Wäre ihm die Farbe wichtiger, wäre die Folge ein
 * hellerer Grund an dieser Stelle, nicht eine dunklere Schrift.
 *
 * Ein zweiter Wert nur für den Rand (3:1 genügt dort) wäre näher am Design
 * gewesen und ist verworfen: Vier Schulen hätten einen Wert, eine hätte zwei —
 * eine Sonderregel, die die nächste Farbänderung mitdenken müsste.
 */

/** Die Farben, nach Kennung der Schule. */
export const SCHOOL_COLOURS: Readonly<Record<string, string>> = {
  CALCINATION: '#c9702f',
  FIXATION: '#6f9fcc',
  SUBLIMATION: '#bd9130',
  COAGULATION: '#35997e',
  PUTREFACTION: '#9b6fd0',
};

/**
 * Die Überschrift der Gruppe für Zauber ohne Schule — englisch (Design-Gesetz 5).
 *
 * **Der Begriff stammt aus dem Design, nicht von mir**: GAME-DESIGN 7.4 heißt
 * „Die drei schulunabhängigen Sonderzauber". „Universal" oder „Special" wären
 * Erfindungen gewesen, und „Special" sagt nur, dass er anders ist — nicht,
 * dass er zu keiner Schule gehört.
 */
export const NO_SCHOOL_LABEL = 'School-independent';

/**
 * Die Farbe einer Schule, oder `null`.
 *
 * `null` heißt: keine Schule und deshalb keine Farbe. Eine zu vergeben
 * behauptete eine Zugehörigkeit, die GAME-DESIGN 7.4 ausdrücklich verneint —
 * die Gruppe steht in der neutralen Textfarbe.
 */
export function colourOfSchool(school: string | null): string | null {
  if (school === null) return null;
  return SCHOOL_COLOURS[school] ?? null;
}
