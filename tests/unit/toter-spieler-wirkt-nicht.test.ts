/**
 * Ein toter Spieler wirkt nicht mehr (GAME-DESIGN A.3, Issue #567).
 *
 * # Was hier schiefgehen kann, ohne dass es jemand merkt
 *
 * Der Spielertod war schon vor diesem Fall an mehreren Stellen abgefragt —
 * `hitPlayer` nahm keinen Schaden mehr, `doom` zählte nicht weiter, die
 * Lebenspunkte standen. Wer das sah, hielt die Regel für umgesetzt. Sie war es
 * zur Hälfte: Das **Wirken** war nicht darunter. Gemessen im `doom-stacks`-Lauf
 * — 2120 Schaden bei 20000 ms, 2780 am Laufende, 660 davon von einem toten
 * Spieler.
 *
 * Deshalb prüft dieser Fall nicht, ob der Tod eintritt, sondern was **danach**
 * noch passiert — und zwar an zwei Größen, nicht an einer:
 *
 *   ausgeteilter Schaden   wächst, wenn `cast` weiter durchgeht
 *   Zeitleisten-Index      rückt vor, wenn `stepPassive` weiterläuft
 *
 * Eine der beiden allein genügt nicht: Der Fix besteht aus zwei Abfragen an
 * zwei Stellen, und ein Fall über nur eine Größe bliebe grün, wenn die andere
 * fehlte. Genau diese Halbheit war der Befund.
 *
 * # Warum `boss-1-2` — und warum NICHT `test-onslaught`
 *
 * Die Bosswahl entscheidet, ob dieser Fall etwas prüft. Gemessen, je Boss, mit
 * und ohne die beiden Abfragen (Zeitleisten-Index vom Tod bis zum Laufende):
 *
 *   boss-1-2         1 -> 1   mit beiden      1 -> 4   ohne `stepPassive`
 *   test-onslaught   4 -> 4   mit beiden      4 -> 4   ohne `stepPassive`
 *   test-doomsayer   0 -> 0   mit beiden      0 -> 0   (führt gar keine)
 *
 * Der erste Entwurf stand auf `test-onslaught`, und **alle vier Fälle blieben
 * grün, als die Abfrage in `stepPassive` entfernt wurde**: Seine vier
 * Zeitleisten-Einträge sind beim Tod nach 24 s längst durch, der Index steht
 * ohnehin. Der Fall prüfte die Größe, ohne sie je bewegen zu können.
 *
 * `boss-1-2` tötet nach 17 s über `doom` und hat dann erst einen von vier
 * Einträgen abgearbeitet — dort ist noch etwas übrig, das vorrücken KANN.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const BOSS_ID = 'boss-1-2';
const TAKTE = 6000; // 60 s — deutlich länger als der Tod nach 17 s

type Ablesung = {
  timeMs: number;
  schaden: number;
  zeitleiste: number;
  hp: number;
};

/**
 * Fährt den Kampf über die volle Zeit und liest den Zustand beim Tod und am
 * Ende ab.
 *
 * **Der Lauf bricht bewusst NICHT beim Tod ab**, obwohl der Export das tut.
 * Genau darum geht es: Ein Aufrufer, der weiterläuft, darf keinen anderen
 * Zustand mehr sehen. Wer hier abbräche, prüfte den Abbruch des Prüffalls
 * statt die Regel im Kampf-Kern.
 */
function laufeUeberDenTodHinaus(): { tod: Ablesung; ende: Ablesung; drueckeNachTod: number } {
  const boss = bossById(BOSS_ID);
  if (boss === undefined) throw new Error(`Der Boss ${BOSS_ID} fehlt.`);
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const kindle = combat.indexOf('kindle');
  expect(kindle, 'KINDLE fehlt in der Kalzination').toBeGreaterThanOrEqual(0);

  let tod: Ablesung | null = null;
  let drueckeNachTod = 0;

  const lies = (timeMs: number): Ablesung => ({
    timeMs,
    schaden: combat.damageDealt,
    zeitleiste: combat.timelineIndex,
    hp: combat.playerHp,
  });

  for (let tick = 0; tick < TAKTE; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (tod === null && combat.playerDead) tod = lies(timeMs);
    // Weiterdrücken, als wüsste der Spieler nichts von seinem Tod — das ist
    // der Fall, den A.3 abdeckt.
    if (combat.isReady(alsSpellIndex(kindle), timeMs)) {
      combat.cast(alsSpellIndex(kindle), timeMs);
      if (tod !== null) drueckeNachTod++;
    }
  }

  if (tod === null) throw new Error('Der Spieler ist nicht gestorben — der Fall misst nichts.');
  return { tod, ende: lies((TAKTE - 1) * TICK_MS), drueckeNachTod };
}

describe('Ein toter Spieler wirkt nicht mehr (A.3)', () => {
  it('der Lauf enthält überhaupt einen Tod und eine Zeitleiste', () => {
    // **Die Zusicherung vor der Messung.** Stirbt niemand oder führt der Boss
    // keine Zeitleiste, sind die Vergleiche darunter grün und prüfen nichts.
    const { tod, ende, drueckeNachTod } = laufeUeberDenTodHinaus();
    expect(tod.hp, 'der Spieler lebt noch').toBe(0);
    expect(tod.timeMs, 'der Tod liegt nicht im Lauf').toBeGreaterThan(0);
    expect(ende.timeMs, 'der Lauf endet nicht nach dem Tod').toBeGreaterThan(tod.timeMs);
    expect(drueckeNachTod, 'nach dem Tod wurde gar nicht mehr gedrückt').toBeGreaterThan(0);
  });

  it('ausgeteilter Schaden steht ab dem Tod — harte Zahlen', () => {
    const { tod, ende } = laufeUeberDenTodHinaus();
    expect(ende.schaden, `Schaden bei ${tod.timeMs} ms gegen Laufende ${ende.timeMs} ms`).toBe(
      tod.schaden,
    );
  });

  it('der Zeitleisten-Index steht ab dem Tod', () => {
    // Die zweite der beiden Größen, und die, die an `stepPassive` hängt statt
    // an `cast`. Ohne sie bliebe der Fall grün, wenn nur die Abfrage im Wirken
    // stünde — die Halbheit, aus der #567 entstand.
    const { tod, ende } = laufeUeberDenTodHinaus();
    expect(ende.zeitleiste, 'die Zeitleiste ist nach dem Tod weitergerückt').toBe(tod.zeitleiste);
  });

  it('und das ist keine leere Aussage: vor dem Tod bewegen sich beide', () => {
    // **Die Gegenprobe zur Gegenprobe.** Stünden Schaden und Zeitleiste im
    // ganzen Lauf still — falscher Boss, keine Zauber, kaputte Leiste —, wären
    // die beiden Fälle oben grün und hätten nichts gezeigt.
    const boss = bossById(BOSS_ID);
    if (boss === undefined) throw new Error(`Der Boss ${BOSS_ID} fehlt.`);
    const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
    const kindle = alsSpellIndex(combat.indexOf('kindle'));

    for (let tick = 0; tick < 200; tick++) {
      const timeMs = tick * TICK_MS;
      combat.stepPassive(timeMs);
      if (combat.isReady(kindle, timeMs)) combat.cast(kindle, timeMs);
    }
    expect(combat.playerDead, 'nach 2 s ist der Spieler schon tot — zu spät gemessen').toBe(false);
    expect(combat.damageDealt, 'im lebenden Lauf entsteht kein Schaden').toBeGreaterThan(0);
  });
});
