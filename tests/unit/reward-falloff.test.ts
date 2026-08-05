/**
 * Eine Kurve für beide Währungen (Issue #276, GAME-DESIGN 12.6).
 *
 * WARUM ES DIESE DATEI GIBT
 * -------------------------
 * 12.6 sagt „der Ertrag sinkt mit dem Stufenabstand" — einmal, für Erfahrung
 * und Punkte zusammen. Im Code rufen beide `applyRewardFalloff`. Das ist heute
 * wahr und morgen eine Behauptung: Wer einer der beiden Währungen später eine
 * eigene Kurve gibt, ändert eine Datei und niemand merkt es. Genau dieses
 * Muster hat in diesem Projekt schon dreimal zugeschlagen.
 *
 * **Ein Kommentar „beide rufen dieselbe Funktion" hätte das nicht verhindert.**
 * Deshalb misst diese Datei das VERHALTEN beider Währungen gegeneinander, an
 * jedem Abstand, gegen dieselbe Quelle. Sie fällt, sobald sie auseinanderlaufen
 * — auch wenn beide für sich genommen plausibel bleiben.
 */
import { describe, expect, it } from 'vitest';
import { LEVELS, REWARD } from '../../src/core/data.js';
import { xpForVictory } from '../../src/progression/level.js';
import { pointsBreakdown } from '../../src/progression/points.js';
import {
  applyRewardFalloff,
  levelGap,
  rewardFalloffPerMille,
} from '../../src/progression/reward-falloff.js';

const BOSS_LEVEL = 30;
/** Ein Lauf mit genug Restzeit, dass die Punkte über den ganzen Abstand messbar bleiben. */
const LAUF = { victory: true, remainingMs: 60_000, heldPercent: null };

/** Die Abstände, an denen gemessen wird — inklusive der drei aus dem Design. */
const ABSTAENDE = [0, 1, 2, 3, 5, 8, 10, 12, 15, 20, 25, 30];

describe('Erfahrung und Punkte fallen mit derselben Kurve', () => {
  it.each(ABSTAENDE)('Abstand %i: beide tragen denselben Faktor', (abstand) => {
    const spieler = BOSS_LEVEL + abstand;
    const promille = rewardFalloffPerMille(BOSS_LEVEL, spieler);

    // Erfahrung: gemessen gegen den ungeminderten Wert derselben Währung.
    const xpVoll = xpForVictory(BOSS_LEVEL, BOSS_LEVEL);
    expect(xpForVictory(BOSS_LEVEL, spieler)).toBe(Math.trunc((xpVoll * promille) / 1000));

    // Punkte: dasselbe, gegen ihre eigene Grundmenge.
    const teile = pointsBreakdown(LAUF, { bossLevel: BOSS_LEVEL, playerLevel: spieler });
    expect(teile.awarded).toBe(Math.trunc((teile.base * promille) / 1000));
    expect(teile.falloffPerMille).toBe(promille);
  });

  it('GEGENPROBE: eine zweite Kurve daneben würde auffallen', () => {
    // Die Fehlerart nachgebaut — nicht irgendeine Abweichung, sondern genau
    // die, gegen die diese Datei schützt: jemand gibt einer Währung eine
    // eigene Halbwertszeit, die bei kleinen Abständen noch fast gleich aussieht.
    const fremd = (abstand: number): number =>
      Math.round(1000 * Math.pow(0.5, Math.max(0, abstand - 2) / 4.5));

    const abweichungen = ABSTAENDE.filter(
      (abstand) => fremd(abstand) !== rewardFalloffPerMille(BOSS_LEVEL, BOSS_LEVEL + abstand),
    );
    // Bei den kleinen Abständen ist die fremde Kurve identisch — dort fiele
    // ein Test nicht auf. Über die gemessene Spanne fällt sie auf, und darauf
    // kommt es an: Die Prüfung oben KANN eine zweite Kurve entdecken.
    expect(abweichungen.length).toBeGreaterThan(0);
  });
});

describe('Die Kurve hält, was 12.6 sagt', () => {
  it('voller Ertrag, solange der Boss zur eigenen Stufe passt', () => {
    for (let abstand = 0; abstand <= REWARD.fullRewardWithinLevels; abstand++) {
      expect(rewardFalloffPerMille(BOSS_LEVEL, BOSS_LEVEL + abstand)).toBe(1000);
    }
  });

  it('bei ~10 Stufen Abstand nur noch ein Bruchteil', () => {
    const zehn = rewardFalloffPerMille(BOSS_LEVEL, BOSS_LEVEL + 10);
    expect(zehn).toBeGreaterThan(0);
    expect(zehn).toBeLessThan(1000 / 3);
  });

  it('bei ~20 Stufen praktisch nichts', () => {
    expect(rewardFalloffPerMille(BOSS_LEVEL, BOSS_LEVEL + 20)).toBeLessThan(50);
  });

  it('die Tabelle stimmt mit ihrer erzeugenden Formel überein', () => {
    // Die Formel steht im `_comment` der Datei. Eine Tabelle, die von ihrer
    // dokumentierten Herkunft abweicht, ist schlimmer als eine ohne — man
    // glaubt zu wissen, wo die Zahlen herkommen.
    for (let abstand = 0; abstand < REWARD.falloffPerMille.length; abstand++) {
      const erwartet = Math.round(
        1000 *
          Math.pow(
            0.5,
            Math.max(0, abstand - REWARD.fullRewardWithinLevels) / REWARD.falloffHalfLifeLevels,
          ),
      );
      expect(REWARD.falloffPerMille[abstand]).toBe(erwartet);
    }
  });

  it('deckt jeden Abstand ab, den 100 Stufen erzeugen können', () => {
    expect(REWARD.falloffPerMille.length).toBe(LEVELS.maxLevel);
    // Und jenseits davon fällt niemand ins Leere: Der Zugriff liefert 0, nicht
    // `undefined` und nicht den vollen Ertrag.
    expect(rewardFalloffPerMille(1, 1000)).toBe(0);
  });
});

describe('Der Abstand zählt nur nach oben', () => {
  it('ein Boss über der eigenen Stufe erzeugt keinen Abstand', () => {
    expect(levelGap(50, 10)).toBe(0);
    expect(rewardFalloffPerMille(50, 10)).toBe(1000);
  });

  it('ein Boss unter der eigenen Stufe schon', () => {
    expect(levelGap(10, 50)).toBe(40);
  });

  it('Bruchteile von Stufen gibt es nicht', () => {
    // Der Index in eine Tabelle muss ganzzahlig sein. Käme hier 10,5 an, läse
    // der Zugriff `undefined` und der Ertrag fiele still auf null.
    expect(levelGap(10.9, 20.9)).toBe(10);
  });
});

describe('Mindern ist ganzzahlig', () => {
  it('schneidet ab, statt zu runden', () => {
    // Bei 205 Promille von 10 sind es 2,05 — ausgezahlt werden 2. Aufrunden
    // hiesse, dass ein Ertrag von 1 bei fast jedem Abstand erhalten bliebe.
    expect(applyRewardFalloff(10, 30, 40)).toBe(2);
  });

  it('null bleibt null', () => {
    expect(applyRewardFalloff(0, 30, 40)).toBe(0);
  });
});
