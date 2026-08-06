import { describe, expect, it } from 'vitest';
import { handvergebenePlaetze } from './slot-from-binding.js';

/**
 * Keine Platzangabe entsteht aus einem Vergleich mit einem Zauber
 * (Issue #405), und der Wächter unterscheidet dabei **Vergeben** von
 * **Lesen** (Issue #488).
 *
 * # Was #488 an dieser Prüfung berichtigt hat
 *
 * Die erste Fassung suchte mit drei regulären Ausdrücken. Einer davon,
 * `/const platz\s*=/`, traf die Zuweisung statt die Herkunft des Werts — und
 * beanstandete damit auch eine Zeile, die einen bestehenden Platz **liest**.
 * Gefunden hat ihn der Autor des Wächters am eigenen Wächter, beim Bau der
 * Fixations-Aufzeichnung.
 *
 * Er hat die Stelle damals umgeschrieben, damit der Wächter schweigt. Das ist
 * die Reihenfolge, gegen die #488 sich richtet: **Der Wächter hatte sich
 * durchgesetzt, statt berichtigt zu werden.** AC3 unten hält fest, dass die
 * ursprüngliche Form wieder durchgeht.
 *
 * # Warum die Gegenprobe hier mehr wiegt als sonst
 *
 * Die naheliegende Behebung — mehr Bedingungen ans Muster — kann den Wächter
 * blind machen, ohne dass es auffällt: Er ist dann grün, und grün sieht aus
 * wie in Ordnung. Deshalb steht unter AC2 jede Form, die er finden **muss**,
 * neben jeder, bei der er schweigen muss.
 */

/** Was der Wächter melden MUSS. */
const MUSS_MELDEN: ReadonlyArray<readonly [string, string]> = [
  ['Zahl beim Aufzeichnen', 'presses.push({ tick, slot: 3 });'],
  ['Vergleich beim Aufzeichnen', 'presses.push({ tick, slot: gewaehlt === quench ? 1 : 0 });'],
  ['Vergleich in Variable', 'const platz = gewaehlt === kindle ? 0 : 3;'],
  ['Vergleich in Variable', 'const platz = gewaehlt === kindle ? 0 : gewaehlt === pyre ? 1 : 3;'],
];

/** Wobei er SCHWEIGEN muss — jede Zeile mit dem Grund. */
const MUSS_SCHWEIGEN: ReadonlyArray<readonly [string, string]> = [
  ['liest einen bestehenden Platz', 'const platz = presses[naechster]!.slot;'],
  [
    'eine Tastenbelegung DEFINIERT Plätze',
    "const b = [{ slot: 0, key: 'q' }, { slot: 1, key: 'w' }];",
  ],
  ['leitet den Platz korrekt ab', 'presses.push({ tick, slot: slotFor(spells, idx) });'],
  [
    'sucht ÜBER den Platz, vergibt keinen',
    'const id = bindings.find((b) => b.slot === eintrag.slot);',
  ],
];

describe('AC1 — der Wächter trifft Vergeben und nicht Lesen', () => {
  it.each(MUSS_MELDEN)('meldet (%s): %s', (_art, zeile) => {
    expect(handvergebenePlaetze(zeile), 'nicht gemeldet').not.toHaveLength(0);
  });

  it.each(MUSS_SCHWEIGEN)('schweigt, weil es %s: %s', (_grund, zeile) => {
    expect(handvergebenePlaetze(zeile), 'Fehlalarm').toHaveLength(0);
  });
});

describe('AC2 — die Gegenprobe: er hat das Problem nicht abgeschafft', () => {
  it('findet nach der Verschärfung noch JEDE der drei ursprünglichen Formen', () => {
    // **Der Kern der Auflage.** Ein Wächter, der nach der Verschärfung nichts
    // mehr meldet, wäre grün — und grün sieht aus wie in Ordnung. Diese drei
    // Formen sind die aus #405, wörtlich.
    const urspruenglich = [
      'presses.push({ tick, slot: 3 });',
      'presses.push({ tick, slot: gewaehlt === quench ? 1 : 0 });',
      'const platz = gewaehlt === kindle ? 0 : 3;',
    ];
    for (const zeile of urspruenglich) {
      expect(handvergebenePlaetze(zeile), `verloren gegangen: ${zeile}`).toHaveLength(1);
    }
  });

  it('und meldet alle drei auch, wenn sie zusammen in einer Datei stehen', () => {
    // Einzeln geprüft könnte jede Form für sich funktionieren und im
    // Zusammenspiel doch nur eine gemeldet werden — etwa wenn der Durchlauf
    // beim ersten Fund abbräche.
    const zusammen = [
      'export function bauen(presses, gewaehlt, kindle, quench) {',
      '  presses.push({ tick, slot: 3 });',
      '  presses.push({ tick, slot: gewaehlt === quench ? 1 : 0 });',
      '  const platz = gewaehlt === kindle ? 0 : 3;',
      '  return platz;',
      '}',
    ].join('\n');
    expect(handvergebenePlaetze(zusammen)).toHaveLength(3);
  });
});

describe('AC3 — die umgestellte Stelle geht in ihrer ursprünglichen Form durch', () => {
  it('const platz = presses[naechster]!.slot — Lesen, kein Vergeben', () => {
    // **Sonst hätte sich der Wächter durchgesetzt, statt berichtigt zu
    // werden.** Genau diese Zeile wurde wegen des Fehlalarms umgeschrieben.
    expect(handvergebenePlaetze('const platz = presses[naechster]!.slot;')).toEqual([]);
  });

  it('auch mit anderem Variablennamen und in einer Schleife', () => {
    const echt = [
      'for (let i = 0; i < presses.length; i++) {',
      '  const platz = presses[i]!.slot;',
      '  const naechsterPlatz = presses[i + 1]?.slot ?? platz;',
      '  benutze(platz, naechsterPlatz);',
      '}',
    ].join('\n');
    expect(handvergebenePlaetze(echt)).toEqual([]);
  });
});

describe('AC4 — die Meldung nennt, welche Form gefunden wurde', () => {
  it.each(MUSS_MELDEN)('nennt die Art %s', (art, zeile) => {
    const [fund] = handvergebenePlaetze(zeile);
    expect(fund?.art, 'die Meldung sagt nicht, welche Form es ist').toBe(art);
  });

  it('und nennt die Zeilennummer, nicht nur die Datei', () => {
    // Ohne Zeilennummer muss der Nächste die Stelle selbst suchen — bei über
    // 3000 Zeilen ist das der Unterschied zwischen Hinweis und Rätsel.
    const text = ['// eins', '// zwei', 'presses.push({ tick, slot: 3 });'].join('\n');
    expect(handvergebenePlaetze(text)[0]?.zeile).toBe(3);
  });
});

describe('Die Quelle selbst ist frei davon (die eigentliche Aussage)', () => {
  it('payloads.ts enthält keine handvergebene Platzangabe', async () => {
    const roh = (await import('./payloads.ts?raw')).default as string;
    expect(
      roh.length,
      'payloads.ts nicht geladen — die Prüfung würde nichts messen',
    ).toBeGreaterThan(1000);

    const funde = handvergebenePlaetze(roh, 'payloads.ts');
    expect(
      funde.map((f) => `${f.zeile}: [${f.art}] ${f.text}`),
      'diese Stellen setzen den Platz von Hand, statt ihn aus der Belegung abzuleiten',
    ).toEqual([]);
  });
});
