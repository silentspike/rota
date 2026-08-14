/**
 * Wie viele Talent-Genotypen fallen auf denselben Phänotyp?
 *
 * ```text
 * npx vite-node tools/phaenotypen-talente-analytisch.mjs
 * ```
 *
 * **Nachgerechnet, nicht unabhängig gemessen.** Der Orchestrator hatte Zahl
 * und Rechenweg vorher genannt; dieser Läufer prüft beides an den Daten nach.
 * Eine Nachrechnung fängt Rechenfehler — eine unabhängige Messung fängt auch
 * Denkfehler. Der Unterschied steht hier, damit niemand die Übereinstimmung
 * für zwei Belege hält.
 *
 * # Zwei Irrwege, die vorher gescheitert sind
 *
 * 1. **Zufallsstichprobe**, 200 000 Ziehungen: null Kollisionen, Faktor 1,00.
 *    Ein Artefakt — zwei Verteilungen, die sich nur im Stamm unterscheiden,
 *    entstehen beim Ziehen praktisch nie, obwohl sie nachweislich denselben
 *    Phänotyp haben. **Der Aufbau konnte den gesuchten Fall kaum erzeugen.**
 * 2. **Vollständige Aufzählung**: zweimal core dump, zwischenzeitlich 20 GB
 *    Swap. Die Menge der Teilzustände über sechs Bäume wächst schneller als
 *    das RAM.
 *
 * Beide ignorierten die Struktur — und genau die löst das Problem in
 * Millisekunden.
 *
 * # Ein widerlegter Gedanke, der plausibel aussah
 *
 * Alle fünf Schul-Bäume haben **1189** verschiedene Beiträge über 20
 * Punktzahlen — Zeichen für Zeichen dieselbe Zahl. Daraus folgt **nicht**,
 * dass sie austauschbar sind:
 *
 * ```text
 *   calcination     damage, dotDuration, resourceGain
 *   fixation        damage, controlCost, controlEffect, controlGain
 *   sublimation     damage, convert, cooldownResetCharges, cooldownThreshold
 *   coagulation     damage, shieldAmount, shieldDuration, shieldStacks
 *   putrefaction    damage, decayCap, decayEffect, decayWard
 * ```
 *
 * Sie teilen genau ein Feld. Die 1189 sagen etwas über die **Form** der Bäume,
 * nicht über ihren **Inhalt**. Wer sie als Multimenge kombiniert, teilt den
 * Raum fälschlich durch 5! = 120.
 *
 * Austauschbar sind nur die **Stämme**: alle sechs tragen ausschließlich
 * `damage` 20 bei `maxRanks` 3.
 *
 * # Die Struktur, gemessen
 *
 * Von 18 Wirkgrößen haben nur **drei** mehr als einen Träger. `decayDuration`
 * hat als einzigen `global-hold` — daraus ist der eindeutig ablesbar, womit
 * sich `dotDuration` und `shieldDuration` nacheinander auflösen.
 *
 * **Es bleibt genau eine echte Faltung: `damage`.**
 *
 * `damage = 20 · (Summe der sechs Stämme) + 25 · calcination-release`
 *
 * # Zwei Zählweisen, beide richtig
 *
 * Ein damage-Wert kann mehrdeutig sein auf zwei Ebenen — die Zahlen
 * unterscheiden sich, weil die Fragen es tun:
 *
 *   * **Vektor-Ebene**: aus mehr als einem Stammvektor (bloße Anordnung)
 *   * **Paar-Ebene**: aus mehr als einem `(Summe, release)`-Paar — zwei
 *     *strukturell* verschiedene Wege, etwa 100 ‰ aus `(0,4)` oder `(5,0)`
 *
 * # Warum die Zahl eine Obergrenze bleibt
 *
 * `calcination-release` setzt `calcination-trunk` voraus. Kombinationen mit
 * `release > 0` und `trunk = 0` sind unerreichbar und hier ausgeschlossen.
 *
 * **Das Punktebudget bindet dagegen nicht:** Der teuerste damage-relevante
 * Aufbau kostet 23 von 50 Punkten. Die Zweige kosten mehr, tragen aber nicht
 * zu `damage` bei.
 *
 * Ungeprüft bleibt, ob die übrigen 22 `requires`-Ketten das Verhältnis
 * berühren. Sie wirken auf Größen, die nicht falten — das ist **geschlossen,
 * nicht gemessen**.
 */

import { readFileSync } from 'node:fs';

const daten = JSON.parse(readFileSync('data/player/talents.json', 'utf8'));
const alle = daten.trees.flatMap((b) => b.talents);

const traeger = alle.filter((t) => t.damagePerRankPerMille);
const staemme = traeger.filter((t) => t.id.endsWith('-trunk'));
const release = traeger.find((t) => t.id === 'calcination-release');

console.log(`damage-Traeger: ${traeger.length} (${staemme.length} Staemme + release)`);
console.log(
  `Staemme je ${staemme[0].damagePerRankPerMille} Promille, maxRanks ${staemme[0].maxRanks}; ` +
    `release ${release.damagePerRankPerMille} Promille, maxRanks ${release.maxRanks}`,
);

// Alle Stamm-Vektoren aufzählen. **Der erste Stamm wird mitgeführt**, weil
// die requires-Kette ihn braucht — eine reine Summe würde ihn wegwerfen.
const istCalcination = staemme.findIndex((t) => t.id === 'calcination-trunk');
const stammSummen = new Map();
const grenzen = staemme.map((t) => t.maxRanks);
const v = new Array(staemme.length).fill(0);
let stammVektoren = 0;
for (;;) {
  stammVektoren += 1;
  const s = v.reduce((a, b) => a + b, 0);
  const k = `${s}|${v[istCalcination] > 0 ? 1 : 0}`;
  stammSummen.set(k, (stammSummen.get(k) ?? 0) + 1);
  let i = 0;
  for (; i < v.length; i++) {
    v[i] += 1;
    if (v[i] <= grenzen[i]) break;
    v[i] = 0;
  }
  if (i === v.length) break;
}
const summenOhneFlagge = new Set([...stammSummen.keys()].map((k) => k.split('|')[0]));
console.log(`Stamm-Vektoren: ${stammVektoren} -> ${summenOhneFlagge.size} verschiedene Summen`);

// Und nun (Stammvektor, release) auf damage-Werte abbilden — nur GÜLTIGE:
// `release > 0` verlangt `calcination-trunk > 0` (requires-Kette).
const werte = new Map();
const paare = new Map();
let kombinationen = 0;
for (const [k, anzahl] of stammSummen) {
  const [summeText, flagge] = k.split('|');
  const summe = Number(summeText);
  for (let r = 0; r <= release.maxRanks; r++) {
    if (r > 0 && flagge === '0') continue;
    kombinationen += anzahl;
    const damage = summe * staemme[0].damagePerRankPerMille + r * release.damagePerRankPerMille;
    werte.set(damage, (werte.get(damage) ?? 0) + anzahl);
    if (!paare.has(damage)) paare.set(damage, new Set());
    paare.get(damage).add(`${summe},${r}`);
  }
}
const aufVektorEbene = [...werte.values()].filter((n) => n > 1).length;
const aufPaarEbene = [...paare.values()].filter((s) => s.size > 1).length;

console.log(`gueltige Kombinationen: ${kombinationen} -> ${werte.size} verschiedene damage-Werte`);
console.log(`  mehrdeutig auf VEKTOR-Ebene: ${aufVektorEbene} von ${werte.size}`);
console.log(`  mehrdeutig auf PAAR-Ebene:   ${aufPaarEbene} von ${werte.size}`);
console.log(
  `FAKTOR (Obergrenze): aus ${kombinationen} Genotypen werden ${werte.size} Phaenotypen, Faktor ${(kombinationen / werte.size).toFixed(2)}`,
);
