/**
 * Der Balance-Deckel auf WARD (Issue #466, GAME-DESIGN „Die Deckel").
 *
 * # Warum es ihn gibt
 *
 * Ohne ihn kann eine hinreichend gute Ausrüstung den Spieler **unverwundbar**
 * machen. Dann verliert nicht nur der Boss-Schaden seinen Sinn, sondern auch
 * die ganze COAGULATION-Schule — sie handelt vom Überleben.
 *
 * 75 % sind **gesetzt**, nicht abgeleitet: Entscheidung des Auftraggebers vom
 * 2026-08-06, als Zahl in `data/player/base.json`.
 *
 * # Er gilt für die MINDERUNG, nicht für den Ausrüstungsanteil
 *
 * Die Design-Tabelle sagt „WARD (Minderung) | 75 %", und der Kampf deckelt die
 * Summe aus `PLAYER.wardReduction` und dem getragenen Anteil. Mit einer Basis
 * von 0,2 heisst das: Ab **550 ‰** getragenem WARD ist der Deckel erreicht,
 * alles darüber wirkungslos.
 *
 * Das ist mehr als eine Feinheit. Läse man die 75 % als Grenze des
 * Ausrüstungsanteils, läge der Deckel effektiv bei 95 % Gesamtminderung — und
 * ein Prüffall mit 700 gegen 750 ‰ könnte gar nichts zeigen, weil beide Werte
 * dann schon darüber liegen.
 *
 * # Gemessen am Kampf, nicht an der Formel
 *
 * `hitPlayer` ist privat, und das ist richtig so. Was zählt, ist was beim
 * Spieler ankommt: `playerHp` nach denselben Takten. Eine Prüfung, die
 * `PLAYER.wardCapPermille` gegen sich selbst rechnet, bestätigte nur, dass eine
 * Konstante eine Konstante ist.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, PLAYER, type BossDef } from '../../src/core/data.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';
import { statOverview, type StatRow } from '../../src/progression/stat-overview.js';

/**
 * Ein Kampf, in dem **nur** WARD abweicht — alles andere neutral.
 *
 * **Gegen `test-onslaught` und nicht gegen den ersten Tiegel.** Der erste
 * Entwurf nahm FIRST_CRUCIBLE, und dort verlor der Spieler in dreissig
 * Sekunden **null** Leben: Der Boss schlägt zu selten zu. Alle Vergleiche
 * unten wären damit „0 gegen 0" gewesen — AC1 grün, ohne irgendetwas zu
 * belegen. Gefangen hat das die Zusicherung ganz unten, nicht Sorgfalt.
 *
 * `test-onslaught` trägt seit #110 genau diesen Zweck: ein Boss, der
 * verlässlich trifft. `player-damage.json` benutzt ihn aus demselben Grund.
 */
function kampf(wardPermille: number): Combat {
  const boss = bossById('test-onslaught');
  if (boss === undefined) throw new Error('Der Boss test-onslaught fehlt.');
  return new Combat({
    school: CALCINATION.value,
    boss: boss as BossDef,
    worn: { ...NOTHING_WORN, ward: wardPermille },
  });
}

/**
 * Wie viel Leben nach `ms` fehlt.
 *
 * Der Boss schlägt von selbst zu; es wird nichts gewirkt. Ein Zauber brächte
 * Schaden, Hitze und womöglich Heilung ins Spiel — hier soll ausschliesslich
 * die Minderung gemessen werden.
 */
function verlust(wardPermille: number, ms = 30_000): number {
  const combat = kampf(wardPermille);
  const voll = combat.playerHp;
  for (let t = 0; t <= ms; t += 10) combat.stepPassive(t);
  return voll - combat.playerHp;
}

/** Die Gesamtminderung in Promille, wie der Kampf sie rechnet. */
function gesamtPermille(wardPermille: number): number {
  return PLAYER.wardReduction * 1000 + wardPermille;
}

describe('Die Zusicherung vor jeder Messung', () => {
  it('der Boss trifft überhaupt — sonst misst hier nichts', () => {
    // Ohne diese Zeile wären alle Vergleiche unten „0 gegen 0" und grün.
    expect(verlust(0)).toBeGreaterThan(0);
  });

  it('und der Deckel liegt über der Basis', () => {
    // Ein Deckel unter der Basis wäre keine Grenze, sondern eine Absenkung:
    // Getragene Ausrüstung minderte dann weniger als gar keine.
    expect(PLAYER.wardCapPermille).toBeGreaterThan(PLAYER.wardReduction * 1000);
  });

  it('die geprüften Werte liegen wirklich dies- und jenseits des Deckels', () => {
    // **Der Grund, warum die AC-Zahlen als GESAMTMINDERUNG zu lesen sind.**
    // Als Ausrüstungswerte gelesen lägen 700 und 750 beide über dem Deckel,
    // und AC2 könnte nichts zeigen.
    expect(gesamtPermille(400)).toBeLessThan(PLAYER.wardCapPermille);
    expect(gesamtPermille(500)).toBeLessThan(PLAYER.wardCapPermille);
    expect(gesamtPermille(550)).toBe(PLAYER.wardCapPermille);
    expect(gesamtPermille(700)).toBeGreaterThan(PLAYER.wardCapPermille);
    expect(gesamtPermille(900)).toBeGreaterThan(PLAYER.wardCapPermille);
  });
});

describe('AC1 — über dem Deckel mindert WARD nicht mehr', () => {
  it('genau am Deckel und weit darüber ergeben denselben Verlust', () => {
    const amDeckel = verlust(550);
    expect(verlust(700)).toBe(amDeckel);
    expect(verlust(900)).toBe(amDeckel);
    // Und ein absurd hoher Wert ebenfalls: Der Deckel ist eine Grenze, keine
    // Dämpfung.
    expect(verlust(5000)).toBe(amDeckel);
  });
});

describe('AC2 — unter dem Deckel wirkt jeder Punkt', () => {
  it('mehr WARD ergibt messbar weniger Verlust', () => {
    // **Ohne diesen Fall wäre AC1 auch erfüllt, wenn WARD gar nichts täte.**
    expect(verlust(500)).toBeLessThan(verlust(400));
    expect(verlust(400)).toBeLessThan(verlust(0));
  });

  it('und noch unmittelbar unter dem Deckel', () => {
    // Die Grenze ist scharf, nicht ausgeschlichen: 540 ‰ mindert weniger als
    // 550 ‰, obwohl nur zehn Promille dazwischen liegen.
    expect(verlust(550)).toBeLessThan(verlust(540));
  });
});

describe('Der Zweck des Deckels: kein unverwundbarer Spieler', () => {
  it('auch mit absurd viel WARD kommt Schaden durch', () => {
    // Das ist die Aussage, um die es geht — nicht „ein Wert ist gedeckelt",
    // sondern „die COAGULATION-Schule behält ihren Gegenstand".
    expect(verlust(5000)).toBeGreaterThan(0);
  });

  it('GEGENPROBE: ohne den Deckel wäre er es', () => {
    // Nachgerechnet, nicht behauptet: Bei 5000 ‰ wäre die ungedeckelte
    // Minderung 5,2 — jeder Treffer ergäbe null oder Heilung.
    const ungedeckelt = PLAYER.wardReduction + 5000 / 1000;
    expect(ungedeckelt).toBeGreaterThan(1);
  });
});

describe('Die beiden Deckel sagen Verschiedenes', () => {
  it('der Balance-Deckel liegt unter der Rechenregel', () => {
    // `Math.min(1, …)` steht weiter im Kampf und ist heute unerreichbar. Er
    // bleibt, weil er etwas anderes sagt: Über 1 würde der Spieler geheilt.
    // Ginge der Balance-Deckel eines Tages darüber, fängt ihn diese Zeile.
    expect(PLAYER.wardCapPermille).toBeLessThanOrEqual(1000);
  });

  it('und der Balance-Deckel steht in den DATEN, nicht im Rumpf', () => {
    // Dieselbe Linie wie `dropChancePerMille`: Ein Wechsel ist ein Zahlenwert
    // in `data/player/base.json`, kein Umbau. Wäre er im Code, müsste der
    // Auftraggeber für eine Balance-Entscheidung eine Zeile ändern lassen.
    expect(PLAYER.wardCapPermille).toBe(750);
  });
});

/**
 * AC4 — der Deckel ist SICHTBAR.
 *
 * Ein Deckel, den man nicht sieht, ist eine Falle: Wer weiter in WARD
 * investiert, verliert Ausrüstungsplätze an einen Wert, der nichts mehr
 * bringt. Die Werte-Übersicht macht ihn deshalb sichtbar — dieselbe Mechanik
 * wie bei PURITY, die es seit #188 gibt und die für WARD nur nicht eingetragen
 * war.
 *
 * Der Browser-Nachweis steht im PR; hier steht, was die Anzeige RECHNET.
 */
describe('AC4 — die Werte-Übersicht zeigt den Deckel', () => {
  const GRUND = { essence: 1000, vessel: 5000, ward: PLAYER.wardReduction * 100 };
  const ward = (permille: number): StatRow =>
    statOverview(GRUND, { ...NOTHING_WORN, ward: permille }).find(
      (r) => r.id === 'ward',
    ) as StatRow;

  it('nennt den Deckel, auch wenn er nicht überschritten ist', () => {
    // Die Zahl steht immer da — sonst erführe der Spieler erst davon, wenn er
    // schon zu viel investiert hat.
    expect(ward(0).cap).toBe(75);
  });

  it('weist den Überschuss aus, wenn er überschritten ist', () => {
    // 900 ‰ Ausrüstung plus 20 % Basis sind 110 % — also 35 % über dem Limit.
    const zeile = ward(900);
    expect(zeile.value).toBe(110);
    expect(zeile.overCap).toBe(35);
    expect(zeile.warning).not.toBeNull();
    expect(zeile.warning).toContain('35');
  });

  it('DER GRUND, WARUM DIE ANZEIGE NICHT MEHR BEI 100 DECKELT', () => {
    // Vorher stand in `statOverview` ein `Math.min(100, …)`. Damit zeigte
    // diese Zeile „100 %" und „25 % über dem Limit" — beide Zahlen falsch,
    // und die zweite untertreibt genau das, was der Spieler verschenkt.
    expect(ward(900).value).toBeGreaterThan(100);
  });

  it('GEGENPROBE: unter dem Deckel erscheint kein Hinweis', () => {
    // Sonst wäre nicht zu sehen, ob die Warnung am Überschreiten hängt oder
    // einfach immer dasteht.
    const zeile = ward(400);
    expect(zeile.value).toBe(60);
    expect(zeile.overCap).toBeNull();
    expect(zeile.warning).toBeNull();
  });

  it('und genau AUF dem Deckel ebenfalls nicht', () => {
    // Die Grenze ist „darüber", nicht „ab" — bei exakt 75 % ist noch nichts
    // verschenkt.
    expect(ward(550).value).toBe(75);
    expect(ward(550).warning).toBeNull();
  });
});
