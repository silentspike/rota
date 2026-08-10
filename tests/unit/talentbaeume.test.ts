/**
 * Die sechs Talentbäume — Struktur und Punkte-Knappheit (Issue #419).
 *
 * # Was dieser Fall prüft und was nicht
 *
 * Er prüft die **Struktur**: dass alle sechs Bäume da sind, jeder einen Stamm
 * und drei Wege trägt, und dass die Wege ihren Stamm verlangen. Das ist AC1.
 *
 * Er prüft **nicht** die Wirkung. Die drei Wege eines Baums wirken heute alle
 * über denselben Topf (`damagePerRankPerMille`) — der Kopfkommentar von
 * `data/player/talents.json` nennt das ausdrücklich „eine Zwischenstufe und
 * keine Umsetzung von 12.5". Wer die Wirkung ersetzt, ersetzt nicht die
 * Struktur, die hier steht.
 *
 * # Warum die Liste abgeschrieben ist
 *
 * `ERWARTETE_BAEUME` leitet sich NICHT aus der Datei her. Täte sie das, wäre
 * dieser Fall grün, egal was drinsteht — er prüfte seine eigene Erwartung
 * gegen sich selbst. Genau davor warnt `src/progression/unlocks.ts`:
 *
 * > „Ein Wächter, der die Verteilung summiert und mit der Summe DERSELBEN
 * > Verteilung vergleicht, prüft seine eigene Erwartung gegen sich selbst —
 * > er ist grün, egal was drinsteht."
 *
 * Aus demselben Grund steht in AC4 unten `talentPointTarget` als Bezug und
 * nicht die Summe von `talentPointLevels`.
 */
import { describe, expect, it } from 'vitest';

import rawLevels from '../../data/player/levels.json';
import rawTalents from '../../data/player/talents.json';

/** Die sechs Bäume aus GAME-DESIGN 12.5 — fünf Schulen plus der globale. */
const ERWARTETE_BAEUME = [
  'CALCINATION',
  'FIXATION',
  'SUBLIMATION',
  'COAGULATION',
  'PUTREFACTION',
  'GLOBAL',
] as const;

/** Die vier Plätze je Baum: ein Stamm, drei Wege. */
const ERWARTETE_PFADE = ['trunk', 'build', 'release', 'hold'] as const;

type Talent = {
  id: string;
  path: string;
  maxRanks: number;
  requires: string | null;
  keystone?: string;
};
type Baum = { id: string; school: string; talents: Talent[] };

const BAEUME = (rawTalents as { trees: Baum[] }).trees;

describe('AC1 — alle sechs Bäume, je Stamm und drei Wege', () => {
  it('führt genau die sechs Bäume aus dem Design', () => {
    expect(BAEUME.map((b) => b.school)).toEqual([...ERWARTETE_BAEUME]);
  });

  it('gibt jedem Baum einen Stamm und drei Wege', () => {
    for (const baum of BAEUME) {
      const pfade = baum.talents.map((t) => t.path);
      for (const pfad of ERWARTETE_PFADE) {
        expect(pfade, `${baum.school} hat keinen Weg "${pfad}"`).toContain(pfad);
      }
      // Genau ein Stamm — zwei wären zwei Bäume in einem.
      expect(
        pfade.filter((p) => p === 'trunk'),
        `${baum.school} hat mehr als einen Stamm`,
      ).toEqual(['trunk']);
    }
  });

  it('hängt jedes Talent an seinem Vorgänger — sonst wäre es eine Liste', () => {
    // **Zwei Sorten, und mein erster Entwurf kannte nur eine.** Er verlangte
    // von JEDEM Talent den Stamm und wurde rot an `calcination-keystone`, das
    // `calcination-release` verlangt. Das ist kein Fehler in den Daten,
    // sondern GAME-DESIGN 12.5: „Am Ende jedes Weges ein Schlüsseltalent."
    //
    //   Stamm            requires: null
    //   die drei Wege    requires: der Stamm
    //   Schlüsseltalent  requires: der Weg, an dessen Ende es steht
    for (const baum of BAEUME) {
      const stamm = baum.talents.find((t) => t.path === 'trunk');
      expect(stamm?.requires, `${baum.school}: der Stamm hat eine Vorbedingung`).toBeNull();

      const kennungen = new Set(baum.talents.map((t) => t.id));
      for (const talent of baum.talents.filter((t) => t.path !== 'trunk')) {
        if (talent.keystone !== undefined) {
          // Ein Schlüsseltalent hängt an einem Talent DESSELBEN Baums —
          // nicht am Stamm, sondern am Weg, den es abschließt.
          expect(
            talent.requires !== null && kennungen.has(talent.requires),
            `${baum.school}/${talent.id} hängt an nichts aus seinem Baum`,
          ).toBe(true);
          continue;
        }
        expect(talent.requires, `${baum.school}/${talent.id} verlangt seinen Stamm nicht`).toBe(
          stamm?.id,
        );
      }
    }
  });

  it('vergibt jede Kennung nur einmal — sie ist der Schlüssel im Spielstand', () => {
    const ids = BAEUME.flatMap((b) => b.talents.map((t) => t.id));
    expect(new Set(ids).size, `doppelte Kennung in ${ids.join(', ')}`).toBe(ids.length);
  });
});

describe('AC4 — mit den Punkten des Spiels lässt sich NICHT alles ausbauen', () => {
  it('braucht mehr Ränge, als es Punkte gibt', () => {
    const noetig = BAEUME.flatMap((b) => b.talents).reduce((summe, t) => summe + t.maxRanks, 0);
    // **`talentPointTarget`, nicht die Summe von `talentPointLevels`.** Die
    // Zielgröße ist die entschiedene Zahl (50); die Verteilung ist laut
    // Kommentar in der Datei noch offen. Gegen die Verteilung zu rechnen
    // hieße, gegen eine Zahl zu prüfen, die sich noch bewegen darf.
    const punkte = (rawLevels as { talentPointTarget: number }).talentPointTarget;

    expect(punkte, 'die Zielgröße ist nicht mehr 50').toBe(50);
    expect(noetig, 'zu wenige Ränge — dann sagt der Vergleich unten nichts').toBeGreaterThan(
      punkte,
    );

    // Die Aussage von 12.5: „zwei Bäume tief ausbauen und ein dritter
    // antippen — niemals alles". Heute fehlen 59 Ränge.
    expect(noetig - punkte).toBeGreaterThan(0);
  });

  it('lässt auch zwei volle Bäume plus Antippen nicht zu allem werden', () => {
    // Die Gegenrichtung: Zwei Bäume müssen sich ausbauen LASSEN, sonst wäre
    // die Knappheit keine Wahl, sondern eine Sperre.
    const jeBaum = BAEUME.map((b) => b.talents.reduce((s, t) => s + t.maxRanks, 0));
    const zweiTiefste = [...jeBaum].sort((a, b) => b - a).slice(0, 2);
    const punkte = (rawLevels as { talentPointTarget: number }).talentPointTarget;

    expect(
      zweiTiefste[0]! + zweiTiefste[1]!,
      'zwei volle Bäume passen nicht in die Punkte — dann ist 12.5 nicht erfüllbar',
    ).toBeLessThanOrEqual(punkte);
  });
});
