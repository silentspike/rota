/**
 * Die fünf Quellen der Sonderwerte (GAME-DESIGN 6.2, 12.5, 13.3, 13.5;
 * Issue #178).
 *
 * CATALYSIS, PURITY und YIELD kommen nicht aus einem Topf, sondern aus fünf
 * verschiedenen Systemen. Diese Datei sagt, **wie viel jedes davon höchstens
 * beiträgt** — und was in der Summe erreichbar ist.
 *
 * # Die Reihenfolge ist umgekehrt zur naheliegenden
 *
 * **Nicht** „Deckel ÷ zehn Plätze = Wert je Platz". Diese Rechnung sähe nur die
 * Affixe und unterstellte, dass sie die einzige Quelle sind. Richtig ist die
 * andere Richtung: Erst steht fest, was Talente, Zauber, legendäre Effekte und
 * Sätze geben — die Affixe kommen dazu, nicht als Rest.
 *
 * # Wofür die Summe da ist
 *
 * Der Tempo-Deckel wird **gemessen, nicht gesetzt** (6.7, Fassung vom
 * 2026-08-04). Die Summe aus dieser Datei ist die Obergrenze, **unter** der ein
 * gemessener Deckel liegen muss:
 *
 * - Liegt er **darüber**, ist er unerreichbar und damit wirkungslos.
 * - Liegt er **genau darauf**, ist „alles auf Tempo" wieder die einzig richtige
 *   Antwort statt einer Entscheidung.
 * - Nur **darunter** entsteht die Frage „was lasse ich dafür weg".
 *
 * Damit ist die Vorgabe des Auftraggebers vom 2026-08-04 prüfbar, ohne dass
 * hier ein Deckel erfunden wird:
 *
 * > „Was aber nicht heißt dass alles in summe 50% ergeben muss. der spieler
 * > kann ja auch overcapped sein, das ist ja dann das problem von dem spieler.
 * > es darf das cap nur nicht zu leicht erreichbar sein."
 *
 * # Dauerhaft und kurzzeitig sind zwei verschiedene Summen
 *
 * 6.2 trennt sie ausdrücklich: CATALYSIS *„wächst **dauerhaft** über Talente und
 * Ausrüstung, **kurzzeitig** über Overdrive"*. Ein Deckel greift im
 * Spitzenmoment anders als im Dauerzustand — wer beides in eine Zahl wirft,
 * kann die Frage „ist der Deckel zu leicht erreichbar" nicht mehr beantworten.
 *
 * # Die Zahlen sind vorläufig, ihre Struktur nicht
 *
 * Sie gehen in die Balance-Runde (GAME-DESIGN F.6, vor E10). Was **nicht**
 * vorläufig ist: dass es fünf Quellen sind, dass der Affix-Beitrag aus dem
 * Budget folgt statt danebenzustehen, und dass die Summe den Deckel
 * überschreiten können muss.
 */

import { MAX_ITEM_LEVEL, QUALITY_EPIC, budgetOf, distributeBudget } from './budget.js';
import { LEVELS } from '../core/data.js';

/**
 * Was ein Gegenstand auf Stufe 1 an je einem Grundwert trägt, in Promille.
 *
 * **UNKALIBRIERT.** Diese Zahl ist nicht gemessen, sie ist gesetzt, damit die
 * Leitung überhaupt etwas transportiert. Das Design nennt keine Zahl dafür, und
 * sie zu erfinden wäre eine Balance-Entscheidung unter dem Deckmantel einer
 * Fehlerbehebung.
 *
 * Sie gehört kalibriert, sobald der Balance-Läufer den Skalierungsfaktor je
 * Wert misst — dafür ist er da. Bis dahin steht sie hier an **einer** Stelle,
 * damit die Kalibrierung eine Zeile ist und keine Suche.
 *
 * **Kein Prüffall prüft ihren Betrag.** Er prüft das Verhältnis: Ein
 * Gegenstand doppelter Stufe trägt doppelt so viel. Ein Prüffall auf den
 * Betrag würde rot melden, sobald jemand kalibriert — der Wächter arbeitete
 * dann gegen die Balance-Arbeit statt für sie.
 */
export const BASE_STAT_AT_LEVEL_1 = 10;
import { SLOT_COUNT } from './slots.js';
import { SET_TIERS } from './sets.js';

/** Affixe auf getragenen Gegenständen. */
export const SOURCE_AFFIXES = 0;
/** Der globale Talentbaum (12.5). */
export const SOURCE_TALENTS = 1;
/** Selbst-Verstärkungen aus Zaubern — vor allem Overdrive (7.4). */
export const SOURCE_SPELLS = 2;
/** Legendäre Sondereffekte (13.3). */
export const SOURCE_LEGENDARY = 3;
/** Satz-Boni (13.5). */
export const SOURCE_SETS = 4;
/** Wie viele Quellen es gibt. */
export const SOURCE_COUNT = 5;

/** Die Namen in der Reihenfolge der Kennnummern — Teil der Schnittstelle. */
export const SOURCE_NAMES: readonly string[] = [
  'affixes',
  'talents',
  'spells',
  'legendary',
  'sets',
];

/** CATALYSIS — Tempo. */
export const STAT_CATALYSIS = 0;
/** PURITY — Kritchance. */
export const STAT_PURITY = 1;
/** YIELD — Kritschaden. */
export const STAT_YIELD = 2;
/** Wie viele Sonderwerte es gibt (6.2). */
export const STAT_COUNT = 3;

/** Die Namen in der Reihenfolge der Kennnummern. */
export const STAT_NAMES: readonly string[] = ['catalysis', 'purity', 'yield'];

/**
 * Wie viele **Budget-Punkte** ein Promille eines Sonderwerts kostet.
 * **Vorläufig.**
 *
 * # Warum es diesen Umrechner überhaupt gibt
 *
 * Das Budget-System aus 13.3 rechnet in abstrakten Punkten: Ein Epic-Affix auf
 * Gegenstandsstufe 100 trägt 7527 davon. 6.7 verlangt aber **Prozent direkt auf
 * dem Gegenstand** („+3 % Tempo", nicht „+120 Tempo").
 *
 * Diese eine Zahl verbindet beides. Sie ist der Grund, warum der Affix-Beitrag
 * unten nicht als zweite Zahl gepflegt wird, sondern aus `budgetOf` **folgt** —
 * dieselbe Überlegung wie bei den Handabschriften aus #134, #138, #159 und
 * #173. Ändert jemand das Budget-Wachstum, wandert der Affix-Beitrag mit.
 *
 * # Warum ausgerechnet 250 — zwei unabhängige Gründe
 *
 * **Erstens nennt 6.7 die Größenordnung selbst.** Als Beispiel für einen Fund
 * steht dort *„+3 % Tempo"*. Ein Epic-Affix auf der höchsten Gegenstandsstufe
 * trägt 7527 Budget-Punkte; geteilt durch 250 sind das genau 30 Promille, also
 * **3 %**.
 *
 * **Zweitens macht erst dieser Wert den einzigen festen Deckel des Spiels
 * erreichbar.** 6.7 sagt über die 100 % bei PURITY: *„Das ist ein erreichbares
 * Ziel, auf das sich ein ganzer Aufbau ausrichten lässt."* Mit einem Umrechner
 * von 500 käme ein voller Aufbau auf 680 Promille und bliebe **unter** dem
 * Deckel — das Ziel wäre unerreichbar, und der Satz im Design falsch.
 *
 * Beide Gründe treffen dieselbe Zahl, und das ist mehr wert als jeder für sich:
 * Die eine Begründung stützt sich auf ein Beispiel, die andere auf eine
 * Zusicherung. `der_purity_deckel_bleibt_erreichbar` hält die zweite fest, damit
 * sie nicht still verlorengeht, wenn jemand an den Talent- oder Satz-Zahlen
 * dreht.
 */
export const BUDGET_PER_PERMILLE = 250;

/**
 * Was ein **einzelner Affix** einer Qualität auf einer Gegenstandsstufe trägt,
 * in Promille.
 *
 * Abgeleitet aus dem Budget, nicht daneben gesetzt.
 */
export function affixPermille(itemLevel: number, quality: number): number {
  const slots = [0, 0, 0, 0];
  const count = distributeBudget(itemLevel, quality, slots);
  if (count === 0) return 0;
  return Math.floor((slots[0] ?? 0) / BUDGET_PER_PERMILLE);
}

/**
 * Was ein Gegenstand dieser Stufe an **je einem Grundwert** trägt, in Promille
 * (Issue #282).
 *
 * # Warum das neben `affixPermille` steht und nicht darin
 *
 * Grundwerte und Affixe sind zwei verschiedene Quellen (GAME-DESIGN 13.5):
 *
 * | Quelle | hängt ab von | Qualität |
 * | --- | --- | --- |
 * | Affixe | Budget, Stufe **und** Qualität | mehr Qualität, mehr Affixe |
 * | Grundwerte | **nur** der Gegenstandsstufe | keine Wirkung |
 *
 * > „Ein Gegenstand trägt IMMER die drei Grundwerte … so wie ein
 * > Diablo-Gegenstand immer Rüstung trägt." (GAME-DESIGN 1346, 1359)
 *
 * In Diablo skaliert die Rüstung mit der Gegenstandsstufe; die Qualität gibt
 * Affixe, nicht mehr Rüstung. Deshalb nimmt diese Funktion **keine** Qualität
 * entgegen — nicht aus Vergesslichkeit, sondern weil sie keine hat.
 *
 * # Die Kurve ist nicht erfunden
 *
 * Es ist dieselbe, mit der schon die Stufenwerte wachsen: `factorPerMille` aus
 * `data/player/levels.json`. Eingang ist aber die **Gegenstandsstufe**, nicht
 * die Spielerstufe — sonst bekäme ein Spieler auf Höchststufe in einer frühen
 * Instanz plötzlich brauchbare Funde, und genau das soll der abnehmende Ertrag
 * verhindern.
 */
export function basePermille(itemLevel: number): number {
  if (!Number.isInteger(itemLevel) || itemLevel < 1 || itemLevel > MAX_ITEM_LEVEL) return 0;
  const promille = LEVELS.factorPerMille[itemLevel - 1] ?? 1000;
  return Math.trunc((BASE_STAT_AT_LEVEL_1 * promille) / 1000);
}

/**
 * Das Maximum aus **Affixen**: jeder Platz mit der besten Qualität, jeder Affix
 * auf denselben Wert.
 *
 * **Legendary trägt genauso viele Affixe wie Epic** (13.3: „3+", das Plus ist
 * der Sondereffekt). Gerechnet wird deshalb mit Epic — das Ergebnis ist
 * dasselbe, und die Absicht ist klarer.
 */
export function maxFromAffixes(itemLevel: number = MAX_ITEM_LEVEL): number {
  const slots = [0, 0, 0, 0];
  const perItem = distributeBudget(itemLevel, QUALITY_EPIC, slots);
  let sum = 0;
  for (let i = 0; i < perItem; i++) {
    sum += Math.floor((slots[i] ?? 0) / BUDGET_PER_PERMILLE);
  }
  return sum * SLOT_COUNT;
}

/**
 * Die Maxima der vier übrigen Quellen, je Sonderwert, in Promille.
 * **Alle Zahlen vorläufig.**
 *
 * | Quelle     | Begründung der Größenordnung |
 * | ---------- | ---------------------------- |
 * | Talente    | 12.5: ~50 Punkte auf sieben Bäume, „niemals alles". Der globale Baum wirkt auf fünf Werte (CATALYSIS/PURITY/YIELD/VESSEL/GCD) — wer alles davon in einen steckt, bekommt 100 Promille. |
 * | Zauber     | 7.4: Overdrive ist ein „kurzer, **starker** Tempo-Schub", und jede Zusammenstellung hat ihn. Stark heißt hier: der größte Einzelbeitrag überhaupt. Auf Kritwerte wirkt er nicht. |
 * | Legendär   | 13.3: der Sondereffekt „verändert die Rotation" — ein Wert-Bonus ist die Ausnahme, nicht die Regel. Deshalb klein. |
 * | Sätze      | 13.5: Werte-Boni auf den **unteren** Stufen (zwei und vier Teile), Rotations-Effekte auf der sechsten. Gerechnet werden die beiden unteren. |
 */
const MAX_PERMILLE: readonly (readonly number[])[] = [
  // [CATALYSIS, PURITY, YIELD]
  [0, 0, 0], // Affixe — kommen aus dem Budget, siehe `maxFromAffixes`
  [100, 100, 200], // Talente
  [300, 0, 0], // Zauber (Overdrive) — nur Tempo, und nur kurzzeitig
  [50, 50, 100], // legendäre Effekte
  [80, 80, 160], // Satz-Boni, zwei Stufen zusammen
];

/**
 * Welche Quellen **dauerhaft** wirken.
 *
 * Der Overdrive ist der einzige befristete Beitrag (6.2, 7.4). Er steht in
 * einer eigenen Summe, weil ein Deckel im Spitzenmoment anders greift als im
 * Dauerzustand.
 */
export function isPermanent(source: number): boolean {
  return source !== SOURCE_SPELLS;
}

/** Das Maximum einer Quelle für einen Sonderwert, in Promille. */
export function maxOf(source: number, stat: number, itemLevel: number = MAX_ITEM_LEVEL): number {
  if (source < 0 || source >= SOURCE_COUNT || stat < 0 || stat >= STAT_COUNT) return 0;
  if (source === SOURCE_AFFIXES) return maxFromAffixes(itemLevel);
  return MAX_PERMILLE[source]?.[stat] ?? 0;
}

/**
 * Die Summe aller **dauerhaften** Quellen für einen Sonderwert.
 *
 * Das ist die Zahl, unter der ein gemessener Deckel liegen muss.
 */
export function permanentMax(stat: number, itemLevel: number = MAX_ITEM_LEVEL): number {
  let sum = 0;
  for (let source = 0; source < SOURCE_COUNT; source++) {
    if (isPermanent(source)) sum += maxOf(source, stat, itemLevel);
  }
  return sum;
}

/** Die Summe **aller** Quellen, einschließlich der befristeten. */
export function totalMax(stat: number, itemLevel: number = MAX_ITEM_LEVEL): number {
  let sum = 0;
  for (let source = 0; source < SOURCE_COUNT; source++) {
    sum += maxOf(source, stat, itemLevel);
  }
  return sum;
}

/**
 * Wie viele Satz-Stufen Werte-Boni tragen (13.5).
 *
 * Die oberste trägt Rotations-Effekte statt Werten — sie zählt in `MAX_PERMILLE`
 * nicht mit. Die Zahl steht hier, damit die Prüfung sie nicht abtippen muss.
 */
export const SET_VALUE_TIERS = SET_TIERS.length - 1;

/**
 * Was ein **einzelner legendärer Effekt** zu Topf 4 beiträgt, in Promille.
 *
 * **Vorläufig — der Wert wird gemessen (#118).** Hier steht er, damit Topf 4
 * überhaupt eine Quelle hat; eine Begründung, warum ausgerechnet diese Zahl,
 * gibt es nicht und soll es nicht geben. Eine Zahl mit Begründungs-Anschein
 * wäre schlimmer als eine offen gesetzte (6.7).
 */
export const LEGENDARY_EFFECT_PERMILLE = 200;

/**
 * Was ein **Satz-Bonus** je Werte-Stufe zu Topf 4 beiträgt, in Promille.
 *
 * **Vorläufig — der Wert wird gemessen (#118).** 13.5 gibt nur die Struktur
 * vor: Werte-Boni auf den unteren Stufen (zwei und vier Teile),
 * Rotations-Effekte auf der sechsten.
 */
export const SET_BONUS_PERMILLE = 150;

/**
 * Topf 4 aus getragenen legendären Effekten und Satz-Stufen.
 *
 * **Beide Quellen, eine Zahl** — genau das ist der Zweck des gemeinsamen
 * Topfes (13.1b): Innerhalb wird addiert, also konkurrieren sie, statt sich zu
 * multiplizieren. Lägen sie getrennt, schlüge die Kombination aus beidem jede
 * andere Ausrichtung, und alles ohne Satz-Bonus würde liegen gelassen — die
 * Lehre aus Diablo, die 13.1b zitiert.
 */
export function gearEffectsPermille(legendaries: number, setTiers: number): number {
  const l = legendaries > 0 ? legendaries : 0;
  const t = setTiers > 0 ? Math.min(setTiers, SET_VALUE_TIERS) : 0;
  return l * LEGENDARY_EFFECT_PERMILLE + t * SET_BONUS_PERMILLE;
}

/**
 * Ein Aufbau: was der Spieler aus jeder Quelle tatsächlich zieht, in Promille.
 *
 * Gebraucht für den Vergleich aus AC4 — ein reiner Tempo-Aufbau gegen einen
 * gemischten.
 */
export interface Build {
  readonly catalysis: number;
  readonly purity: number;
  readonly yieldBonus: number;
}

/**
 * Der Aufbau, der **alles** auf einen Wert setzt: jede Quelle, die ihn liefern
 * kann, auf Anschlag.
 *
 * Ohne Overdrive gerechnet — er läuft nicht den ganzen Kampf, und ein
 * Dauerwert, der einen Spitzenwert enthält, wäre eine falsche Zahl.
 */
export function pureBuild(stat: number, itemLevel: number = MAX_ITEM_LEVEL): Build {
  const alles = permanentMax(stat, itemLevel);
  return {
    catalysis: stat === STAT_CATALYSIS ? alles : 0,
    purity: stat === STAT_PURITY ? alles : 0,
    yieldBonus: stat === STAT_YIELD ? alles : 0,
  };
}

/**
 * Der gleichmäßig gemischte Aufbau: dasselbe Budget, auf drei Werte verteilt.
 *
 * **Dieselbe Gesamtmenge wie `pureBuild`**, sonst verglichen man nicht zwei
 * Aufbauten, sondern zwei Budgets. Das ist dieselbe Überlegung, die schon dem
 * Satz-gegen-Affix-Vergleich aus #166 zugrunde liegt.
 */
export function mixedBuild(
  referenceStat: number = STAT_CATALYSIS,
  itemLevel: number = MAX_ITEM_LEVEL,
): Build {
  // **Das Budget kommt vom Aufbau, gegen den verglichen wird** — nicht vom
  // größten der drei Maxima.
  //
  // Die erste Fassung nahm das größte, und der Prüffall hat sie sofort
  // umgeworfen: YIELD hat die höchsten Beiträge aus Talenten und Sätzen, also
  // bekam der gemischte Aufbau 1359 Promille gegen 1130 des reinen. Er hätte
  // gewonnen, ohne dass die Verteilung etwas damit zu tun gehabt hätte.
  //
  // Verglichen werden zwei **Verteilungen desselben Budgets**, sonst vergleicht
  // man zwei Budgets. Dieselbe Überlegung wie beim Satz-gegen-Affix-Vergleich
  // aus #166.
  const budget = permanentMax(referenceStat, itemLevel);
  const drittel = Math.floor(budget / STAT_COUNT);
  return { catalysis: drittel, purity: drittel, yieldBonus: drittel };
}

/** Was ein Aufbau insgesamt an Promille einsetzt — für den fairen Vergleich. */
export function buildTotal(build: Build): number {
  return build.catalysis + build.purity + build.yieldBonus;
}

/** Das Budget eines Gegenstands, für die Herleitung des Affix-Werts. */
export { budgetOf };
