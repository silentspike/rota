import { describe, expect, it } from 'vitest';
import { NEUTRAL_FACTORS, type DamageFactors } from '../../src/core/damage.js';
// **Als Text, nicht über `node:fs`.** Der Umweg über das Dateisystem bräuchte
// `@types/node`, das diese tsconfig bewusst nicht führt (`types: ["vite/client"]`)
// — und ein Pfad, den erst die Laufzeit prüft, wäre bei einer Umbenennung still
// leer statt rot. Vite löst den Import beim Bauen auf: Verschwindet die Datei,
// scheitert schon der Bau.
import COMBAT from '../../src/core/combat.ts?raw';

/**
 * Jeder Schadensfaktor hat eine Stelle, die ihn setzt (Issue #424).
 *
 * # Was hier gemessen wird — und was ausdrücklich nicht
 *
 * **Gemessen:** Gibt es im Produktionscode überhaupt eine Zeile, die diesen
 * Topf beschreibt? Das ist ein Quelltext-Scan und braucht weder einen Kampf
 * noch einen Lesezugriff auf `Combat`.
 *
 * **Nicht gemessen:** Ob der Topf in einem gegebenen Kampf auch wirklich
 * gefüllt wird. Das ist #425, und es ist die schwerere Frage — ein erster
 * Entwurf dieses Wächters ist genau daran gescheitert: In einem Kampf mit
 * FIRST_CRUCIBLE und kindle/smoulder/pyre bleiben fünf von acht Töpfen
 * neutral, ohne dass irgendetwas kaputt wäre. Der Boss hat keine Widerstände,
 * die drei Zauber öffnen kein Synergie-Fenster, und Zersetzung gehört zu
 * PUTREFACTION. „Nicht angeschlossen" und „in diesem Lauf nicht ausgelöst"
 * sahen dort gleich aus.
 *
 * Dieser Fall macht die kleinere, aber trennscharfe Aussage.
 *
 * # Warum es ihn braucht
 *
 * Ein Faktor auf 1 verhält sich in jedem Test wie ein korrekt gefüllter.
 * `buffs` war deshalb fünf Issues lang auf beiden Seiten gebaut, mit einem
 * gemeinsamen Prüffall abgeglichen — und in keinem Kampf angeschlossen. Die
 * Parität war grün, weil beide Seiten dasselbe **nicht** taten.
 *
 * # Die Falle beim Suchmuster
 *
 * Ein erster Anlauf suchte `<name>: [a-z]` und übersah
 * `essence: NEUTRAL_FACTORS.essence * …`, weil dort ein Großbuchstabe folgt.
 * Ein Suchmuster misst, was es kennt (WORKER-GUIDE 8.27). Deshalb prüft dieser
 * Fall **zwei** Formen und nennt bei einem Treffer, welche es war.
 */

/** Die acht Namen aus dem Typ, nicht von Hand aufgezählt. */
const ALLE_TOEPFE = Object.keys(NEUTRAL_FACTORS) as (keyof DamageFactors)[];

/**
 * `crit` ist ausgenommen — **mit Grund, nicht aus Bequemlichkeit.**
 *
 * Er ist per Typ von `setDamageFactor` ausgeschlossen
 * (`Exclude<keyof DamageFactors, 'crit'>`) und lebt nur in `factorsOnCrit`.
 * Das ist Absicht: Er wird aus `worn` abgeleitet und gehört nicht zu den
 * Faktoren, die eine Mechanik setzt. Wäre er hier setzbar, ließe sich die
 * Krit-Fassung still von der Basis abkoppeln.
 *
 * Eine Ausnahme ohne diese Begründung wäre die Aufweichung, gegen die der
 * ganze Wächter gebaut ist.
 */
const AUSGENOMMEN: Readonly<Record<string, string>> = {
  crit: 'per Typ von setDamageFactor ausgeschlossen, lebt nur in factorsOnCrit',
};

/** Die zwei Formen, in denen ein Topf beschrieben wird. */
function schreibstellen(topf: string): string[] {
  const gefunden: string[] = [];
  // Form 1: die eine Schreibmethode.
  if (COMBAT.includes(`setDamageFactor('${topf}'`)) gefunden.push('setDamageFactor');
  // Form 2: das Erzeugungs-Literal. Bewusst ohne Annahme darüber, was nach dem
  // Doppelpunkt steht — genau daran ist der erste Anlauf gescheitert.
  if (new RegExp(`^\\s+${topf}:\\s*\\S`, 'm').test(COMBAT)) gefunden.push('Erzeugungs-Literal');
  return gefunden;
}

describe('Jeder Schadensfaktor hat eine Schreibstelle (Issue #424)', () => {
  it('AC1 — die Messung liest überhaupt etwas', () => {
    // **Zuerst diese Zusicherung.** Ohne sie liefe der Fall unten über eine
    // leere Liste oder eine leere Datei und wäre grün, ohne etwas zu sehen —
    // genau die Klasse, gegen die er gebaut ist.
    expect(ALLE_TOEPFE.length, 'die Liste der Töpfe ist leer').toBe(8);
    expect(COMBAT.length, 'combat.ts ist leer oder nicht gefunden').toBeGreaterThan(10000);
    expect(COMBAT, 'combat.ts enthält gar kein setDamageFactor').toContain('setDamageFactor');
  });

  it('AC2 — und sie findet nicht einfach alles', () => {
    // **Der Gegenfall zum Suchmuster.** Fände `schreibstellen` für jeden
    // erfundenen Namen etwas, wäre die Messung unten wertlos.
    expect(
      schreibstellen('gibtesnicht'),
      'das Suchmuster findet auch Namen, die es nicht gibt',
    ).toEqual([]);
  });

  it('AC3 — genau die begründeten Ausnahmen haben keine, sonst keiner', () => {
    const ohne = ALLE_TOEPFE.filter((t) => schreibstellen(t).length === 0);
    const unerwartet = ohne.filter((t) => AUSGENOMMEN[t] === undefined);

    expect(
      unerwartet,
      `Diese Faktoren werden von keiner Stelle in combat.ts beschrieben:\n` +
        unerwartet.map((t) => `  ${t}`).join('\n') +
        `\n\nEin Faktor, den niemand setzt, steht dauerhaft auf ${NEUTRAL_FACTORS.essence} ` +
        `und verhält sich in jedem Test wie ein korrekt gefüllter. Entweder das ` +
        `System ist nicht angeschlossen — dann gehört ein Issue dazu — oder es ` +
        `gehört hier mit Begründung ausgenommen.`,
    ).toEqual([]);
  });

  it('AC4 — und die Ausnahmen sind noch nötig, nicht längst überholt', () => {
    // **Die andere Richtung.** Ein Wächter, der eine überholte Ausnahme
    // mitschleppt, hält eine Unwahrheit fest — und beim nächsten Lesen wird
    // sie für bare Münze genommen.
    const ueberholt = Object.keys(AUSGENOMMEN).filter((t) => schreibstellen(t).length > 0);
    expect(
      ueberholt,
      `Diese Faktoren haben inzwischen eine Schreibstelle und stehen trotzdem ` +
        `in der Ausnahmeliste:\n` +
        ueberholt.map((t) => `  ${t} — ${AUSGENOMMEN[t]}`).join('\n') +
        `\n\nDen Eintrag entfernen, dann bewacht dieser Fall den Faktor wieder.`,
    ).toEqual([]);
  });
});
