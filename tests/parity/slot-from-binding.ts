import ts from 'typescript';

/**
 * Findet Platzangaben, die von Hand entstehen statt aus der Tastenbelegung
 * (Issue #405, nachgeschärft in #488).
 *
 * # Warum ein Syntaxbaum und kein Suchmuster
 *
 * Die erste Fassung suchte mit drei regulären Ausdrücken, und einer davon —
 * `/const platz\s*=/` — traf die **Zuweisung** statt die Herkunft des Werts.
 * Damit beanstandete er auch
 *
 * ```ts
 * const platz = presses[naechster]!.slot;   // liest einen bestehenden Platz
 * ```
 *
 * Ein Wächter mit Fehlalarmen wird nach dem dritten Mal umgangen statt
 * gelesen — und meldet den echten Fall dann an jemanden, der schon
 * weggeklickt hat.
 *
 * # Das Kriterium stand von Anfang an im Kommentar, nur nicht im Muster
 *
 * > **Entsteht der Platz aus einem Vergleich mit einem Zauber?**
 *
 * Das ist eine Frage an den Syntaxbaum, nicht an den Text. Gemessen: Die
 * Datei zu parsen und zu durchlaufen kostet rund 130 ms — für einen Prüffall
 * bezahlbar.
 *
 * # Zwei Verschärfungen, die aus Messungen stammen
 *
 * Ein erster Entwurf prüfte nur „enthält einen `===`-Vergleich". Der meldete
 * **41 Stellen** statt einer:
 *
 * - 39 mal `slot: 0` in **Tastenbelegungen** — dort ist eine Platzzahl genau
 *   richtig, sie *definiert* ja die Belegung. Deshalb zählt nur, was in einem
 *   `presses.push(…)` steht: beim **Aufzeichnen** eines Drucks.
 * - `bindings.find((b) => b.slot === eintrag.slot)` — dieser Vergleich
 *   **sucht** über den Platz, statt einen zu vergeben. Deshalb muss der
 *   Vergleich eine **Zahl auswählen** (`… ? 0 : 3`), nicht bloß vorkommen.
 *
 * Beide Male hätte ein schärferes Suchmuster dasselbe Problem nur verschoben.
 */

/** Welche der beiden Formen gefunden wurde — für die Meldung (AC4). */
export type FundArt =
  'Zahl beim Aufzeichnen' | 'Vergleich beim Aufzeichnen' | 'Vergleich in Variable';

export interface Fund {
  art: FundArt;
  zeile: number;
  text: string;
}

/**
 * Wählt der Ausdruck eine **Zahl** über eine Bedingung aus?
 *
 * Das unterscheidet Vergeben von Suchen:
 * - `gewaehlt === kindle ? 0 : 3` — vergibt (die Zweige sind Zahlen)
 * - `bindings.find((b) => b.slot === x)` — sucht (kein Zahlen-Zweig)
 *
 * # Warum die Bedingung KEIN `===` enthalten muss
 *
 * Der erste Entwurf verlangte einen `===`-Vergleich in der Bedingung. Der
 * Orchestrator hat beim Nachmessen die Lücke gefunden:
 *
 * ```ts
 * const istErster = true;
 * const platz = istErster ? 0 : 3;   // rutschte durch
 * ```
 *
 * Das ist dieselbe Handvergabe — nur mit **ausgelagerter Bedingung**. Wer
 * `gewaehlt === kindle` in eine Variable zieht, weil er sie zweimal braucht,
 * fiele aus dem Netz.
 *
 * **Gemessen, bevor die Bedingung fiel:** Ohne den `===`-Teil meldet der
 * Wächter in `payloads.ts` weiterhin **0** Stellen, und alle vier Fälle, bei
 * denen er schweigen muss, bleiben still. Die Verschärfung ist hier also
 * kostenlos — anders als beim ersten Entwurf, der 41 Fehlalarme brachte.
 *
 * Was zählt, ist allein: **Eine Bedingung wählt eine Zahl aus.** In einer
 * Platzangabe ist das immer verdächtig, egal wie die Bedingung aussieht.
 */
function waehltZahl(node: ts.Node): boolean {
  let ja = false;
  const gehe = (n: ts.Node): void => {
    if (ts.isConditionalExpression(n)) {
      // Verschachtelte Ketten (`a ? 0 : b ? 1 : 3`) zählen mit — der innere
      // Zweig ist selbst wieder eine Auswahl.
      const istZahl = (z: ts.Node): boolean =>
        ts.isNumericLiteral(z) ||
        ts.isConditionalExpression(z) ||
        (ts.isPrefixUnaryExpression(z) && ts.isNumericLiteral(z.operand));

      if (istZahl(n.whenTrue) || istZahl(n.whenFalse)) ja = true;
    }
    ts.forEachChild(n, gehe);
  };
  gehe(node);
  return ja;
}

/** Durchsucht einen Quelltext nach handvergebenen Plätzen. */
export function handvergebenePlaetze(quelltext: string, dateiname = 'quelle.ts'): Fund[] {
  // `createSourceFile` nimmt den **Namen** zuerst und den **Text** an zweiter
  // Stelle. Beide sind `string`, der Übersetzer fängt ein Vertauschen also
  // nicht — dieselbe Falle wie bei `parse(name, text)` im APL-Leser.
  const sf = ts.createSourceFile(dateiname, quelltext, ts.ScriptTarget.ESNext, true);

  const funde: Fund[] = [];
  const merke = (art: FundArt, node: ts.Node): void => {
    const { line } = sf.getLineAndCharacterOfPosition(node.getStart(sf));
    funde.push({ art, zeile: line + 1, text: node.getText(sf).slice(0, 60) });
  };

  const gehe = (n: ts.Node): void => {
    // (a) Beim AUFZEICHNEN eines Drucks: presses.push({ …, slot: … })
    //
    // Nur hier, nicht überall — `slot: 0` in einer Tastenbelegung ist die
    // Belegung selbst und völlig richtig.
    if (ts.isCallExpression(n) && n.expression.getText(sf).endsWith('presses.push')) {
      for (const arg of n.arguments) {
        if (!ts.isObjectLiteralExpression(arg)) continue;
        for (const eigenschaft of arg.properties) {
          if (!ts.isPropertyAssignment(eigenschaft)) continue;
          if (eigenschaft.name.getText(sf) !== 'slot') continue;
          if (ts.isNumericLiteral(eigenschaft.initializer)) {
            merke('Zahl beim Aufzeichnen', eigenschaft);
          } else if (waehltZahl(eigenschaft.initializer)) {
            merke('Vergleich beim Aufzeichnen', eigenschaft);
          }
        }
      }
    }

    // (b) In einer Zwischenvariablen: const platz = gewaehlt === … ? 0 : 3
    //
    // Ein Lesezugriff (`presses[n]!.slot`) hat keinen Vergleich, der eine Zahl
    // auswählt, und geht deshalb durch — das war der Fehlalarm aus #488.
    if (ts.isVariableDeclaration(n) && n.initializer !== undefined) {
      if (waehltZahl(n.initializer)) merke('Vergleich in Variable', n);
    }

    ts.forEachChild(n, gehe);
  };
  gehe(sf);
  return funde;
}
