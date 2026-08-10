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

type Talent = {
  id: string;
  path: string;
  maxRanks: number;
  requires: string | null;
  keystone?: string;
};
type Baum = { id: string; school: string; paths: string[]; talents: Talent[] };

const BAEUME = (rawTalents as { trees: Baum[] }).trees;

describe('AC1 — alle sechs Bäume, je Stamm und drei Wege', () => {
  it('führt genau die sechs Bäume aus dem Design', () => {
    expect(BAEUME.map((b) => b.school)).toEqual([...ERWARTETE_BAEUME]);
  });

  it('gibt jedem Baum einen Stamm und drei Wege', () => {
    // # Warum hier seit #419 keine NAMEN mehr stehen
    //
    // Bis zur zweiten Runde verlangte dieser Fall `['trunk','build','release',
    // 'hold']` wörtlich, von jedem Baum. Das war richtig, solange alle Bäume
    // dieselben Weg-Namen trugen — und es wurde falsch, als COAGULATION
    // `thicker/longer/stacks` bekam: Der Fall wäre rot geworden, obwohl der
    // Baum vollständig ist. Ein Wächter, der bei vollständiger Struktur rot
    // meldet, erzeugt Falschalarme, und Falschalarme werden weggeklickt.
    //
    // **Die Aussage bleibt dieselbe**, nur ihr Mittel wandert: „ein Stamm und
    // drei Wege" ist wahr oder falsch, unabhängig davon, wie die Wege heißen.
    //
    // # Und er ist dabei SCHÄRFER geworden, nicht laxer — gemessen
    //
    // | Datenlage | Namensprüfung | diese Prüfung |
    // | --- | --- | --- |
    // | ein Weg fehlt (`hold` weg) | rot | **rot** |
    // | ein Weg VERTIPPT (`hold` → `hodl`) | rot | **rot** |
    // | ein Talent mehr auf einem Weg | grün | grün — es ist das Keystone-Muster |
    //
    // **Alle drei Zeilen sind gemessen, nicht angenommen** — und zwei davon
    // haben mich korrigiert:
    //
    // Die dritte hielt ich für eine Schwäche der alten Prüfung. Sie ist die
    // Form, die CALCINATION heute hat: Das Schlüsseltalent liegt auf
    // `release`. Beinahe hätte ich eine Prüfung gebaut, die den Bestand
    // verbietet.
    //
    // **Die zweite ist ein echter VERLUST, und er steht hier statt zu
    // fehlen.** Ich hatte in diesen Kommentar geschrieben, ein Tippfehler
    // ergäbe „vier statt drei" — falsch: Ein vertippter Weg ERSETZT einen
    // richtigen, es bleiben drei verschiedene. Die Namensprüfung hätte ihn
    // gemeldet, diese meldet ihn nicht.
    //
    // Der Verlust ist in Kauf genommen, weil die Namensprüfung dafür bei
    // jeder legitimen Umbenennung rot geworden wäre — und ein Tippfehler
    // fällt beim ersten Blick in `talents.json` auf, ein Falschalarm nicht.
    //
    // **Keine Prüfung über die Bäume hinweg.** `trunk` kommt sechsmal vor;
    // eine Regel „keine Kennung doppelt" wäre aus Symmetrie falsch.
    for (const baum of BAEUME) {
      const pfade = baum.talents.map((t) => t.path);
      // Genau ein Stamm — zwei wären zwei Bäume in einem.
      expect(
        pfade.filter((p) => p === 'trunk'),
        `${baum.school} hat mehr als einen Stamm`,
      ).toEqual(['trunk']);
      // **Die DEKLARATION gegen die VERWENDUNG**, in beide Richtungen.
      //
      // Der Baum sagt in `paths`, welche Wege er hat; die Talente sagen mit
      // `path`, welchen sie belegen. Zwei Stellen, die übereinstimmen müssen —
      // ein Tippfehler steht nur an einer davon.
      //
      // Ohne `paths` gab es diese zweite Stelle nicht: Die Wege entstanden als
      // Nebenwirkung daraus, welche Zeichenketten in den Talenten vorkommen.
      // Deshalb war ein vertippter Weg von einem neuen nicht zu unterscheiden.
      expect(baum.paths[0], `${baum.school}: der erste Weg ist nicht der Stamm`).toBe('trunk');
      expect(
        baum.paths.length,
        `${baum.school} deklariert ${baum.paths.length} Wege statt vier (Stamm + drei)`,
      ).toBe(4);
      expect(
        new Set(baum.paths).size,
        `${baum.school} deklariert einen Weg doppelt: ${baum.paths.join(', ')}`,
      ).toBe(4);
      // Richtung 1 — jeder DEKLARIERTE Weg hat mindestens ein Talent.
      for (const weg of baum.paths) {
        expect(pfade, `${baum.school}: der Weg "${weg}" hat kein Talent`).toContain(weg);
      }
      // Richtung 2 — jedes Talent liegt auf einem deklarierten Weg.
      //
      // **Das ist der Gewinn gegenüber der alten Namensprüfung.** Ein
      // vertippter `path` steht nicht in `paths` und fällt hier auf; vorher
      // war er ein gültiger dritter Weg.
      for (const t of baum.talents) {
        expect(
          baum.paths,
          `${baum.school}/${t.id} liegt auf "${t.path}" — nicht deklariert`,
        ).toContain(t.path);
      }
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
