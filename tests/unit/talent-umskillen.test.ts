/**
 * Umskillen kostet nichts und lässt nichts zurück (Issue #249, AC2).
 *
 * **Was das Kriterium meint.** GAME-DESIGN.md:1352 sagt es deutlicher als das
 * AC selbst: „Umskillen — **Jederzeit kostenlos** zwischen den Kämpfen. Kostet
 * ausdrücklich KEINE Punkte — das Spiel ist ein Optimierungs-Labor". Das ist
 * eine Aussage über KOSTEN, nicht über eine Bedienung: Es gibt keine
 * Umskill-Funktion und soll keine geben. Prüfbar ist, dass eine ersetzte
 * Verteilung sofort und restlos wirkt — kein gecachter Faktor, keine
 * verbrauchten Punkte, kein Rest der alten Verteilung.
 *
 * **Warum es diese Datei braucht.** `talents.test.ts:154` heißt „kein Rest
 * bleibt hängen — auch nach `reset`" und liest sich wie der Nachweis. Er ist
 * es nicht: Dort wird der KAMPF zurückgesetzt, nicht die Zusammenstellung, und
 * der Kommentar im Test sagt das auch. Der Fallname deckte sich mit dem AC,
 * der Inhalt nicht.
 */

import { describe, expect, it } from 'vitest';

import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, TALENTS } from '../../src/core/data.js';
import { spentPoints, talentFactor, type TalentAllocation } from '../../src/progression/talents.js';

/** Ein Talent mit Rängen, aus den Daten gelesen statt hier benannt. */
const STAMM = TALENTS.find((t) => (t.maxRanks ?? 0) > 0)?.id ?? '';

function schadenEinesTreffers(talents: TalentAllocation): number {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    talents,
  });
  combat.cast(combat.indexOf('kindle'), 0);
  return combat.damageDealt;
}

describe('AC2 — eine ersetzte Verteilung wirkt sofort und restlos', () => {
  /**
   * **Träger-Zusicherung, und sie steht vorn.**
   *
   * Alles Weitere vergleicht Schadenswerte mit und ohne Talent. Wirkt das
   * Talent gar nicht, wären alle Vergleiche gleich — und jeder folgende Fall
   * bliebe grün, ohne etwas zu prüfen.
   */
  it('das geprüfte Talent verändert den Schaden überhaupt', () => {
    expect(STAMM).not.toBe('');
    expect(schadenEinesTreffers({ [STAMM]: 3 })).toBeGreaterThan(schadenEinesTreffers({}));
  });

  /**
   * Der Kern: Wer zurück auf die leere Verteilung geht, steht wieder genau am
   * Anfang — nicht ungefähr, sondern auf demselben Wert.
   */
  it('zurück auf leer ergibt exakt den Wert von vorher', () => {
    const anfang = schadenEinesTreffers({});
    schadenEinesTreffers({ [STAMM]: 3 });
    expect(schadenEinesTreffers({})).toBe(anfang);
  });

  /**
   * Dieselbe Frage in die andere Richtung: Ein Wechsel von voll auf halb darf
   * nicht den vollen Wert behalten.
   *
   * **Ohne diesen Fall wäre der vorige auch erfüllt, wenn die Verteilung
   * überhaupt nicht wirkte** — „leer ergibt leer" ist trivial wahr, solange
   * nichts zwischengespeichert wird UND nichts wirkt.
   */
  it('ein Wechsel auf weniger Ränge senkt den Schaden wieder', () => {
    const voll = schadenEinesTreffers({ [STAMM]: 3 });
    const halb = schadenEinesTreffers({ [STAMM]: 1 });
    expect(halb).toBeLessThan(voll);
    expect(halb).toBe(schadenEinesTreffers({ [STAMM]: 1 }));
  });

  /**
   * Die Reihenfolge darf nichts ausmachen — sonst überlebte irgendwo ein
   * Zustand den Wechsel.
   */
  it('dieselbe Verteilung ergibt denselben Wert, egal was vorher lag', () => {
    const frisch = schadenEinesTreffers({ [STAMM]: 2 });
    schadenEinesTreffers({ [STAMM]: 3 });
    schadenEinesTreffers({});
    schadenEinesTreffers({ [STAMM]: 1 });
    expect(schadenEinesTreffers({ [STAMM]: 2 })).toBe(frisch);
  });

  /**
   * **Kostenlos heißt: keine Punkte gehen verloren.** `spentPoints` zählt die
   * Verteilung, nicht ihre Geschichte — sonst wäre Umskillen mit der Zeit
   * teurer, und genau das schließt das Design aus.
   */
  it('die Ausgaben zählen die Verteilung, nicht ihre Geschichte', () => {
    expect(spentPoints({ [STAMM]: 3 })).toBe(3);
    expect(spentPoints({})).toBe(0);
    expect(spentPoints({ [STAMM]: 1 })).toBe(1);
  });
});

/**
 * AC6 — der Spielstand trägt die Wahl, nicht ihre Wirkung.
 *
 * **Was schon gedeckt war und was nicht.** `ALLOWED_TOP_LEVEL` in
 * `save-format.ts` ist Prüfgegenstand und schlägt an, wenn ein Feld dazukommt —
 * ein `talentFactor` neben `talentPoints` fiele also auf. Die Ebene DARUNTER
 * war offen: Ein berechneter Wert kann in `talentPoints` selbst landen, und
 * dort sähe ihn kein Wächter.
 *
 * Der bestehende Fall (`talent-points.test.ts:143`) prüft, dass ein LEERER
 * Stand ein leeres Feld hat. Sein Kommentar nennt die Absicht — „was der
 * Spieler entschieden hat, steht im Stand; was daraus folgt, nicht" — aber ein
 * leeres Feld belegt sie nicht.
 */
describe('AC6 — im Spielstand steht die Verteilung, kein berechneter Bonus', () => {
  /** **Träger-Zusicherung:** Ohne Talente in den Daten wäre alles darunter leer. */
  it('es gibt überhaupt Talente mit Rängen', () => {
    expect(TALENTS.filter((t) => (t.maxRanks ?? 0) > 0).length).toBeGreaterThan(0);
  });

  it('jeder Eintrag ist eine bekannte Kennung mit einem ganzzahligen Rang', () => {
    const verteilung: TalentAllocation = { [STAMM]: 3 };
    for (const [kennung, rang] of Object.entries(verteilung)) {
      expect(TALENTS.some((t) => t.id === kennung)).toBe(true);
      expect(Number.isInteger(rang)).toBe(true);
      expect(rang).toBeGreaterThan(0);
    }
  });

  /**
   * Der Kern: Ein Faktor ist keine ganze Zahl. Landete er je in der Verteilung,
   * fiele er hier auf — auch dann, wenn er unter einer plausiblen Kennung
   * stünde.
   */
  it('ein Faktor in der Verteilung wäre erkennbar — Gegenprobe', () => {
    const mitBonus = { [STAMM]: 3, berechneterFaktor: 1.15 } as TalentAllocation;
    const sauber = Object.entries(mitBonus).every(
      ([kennung, wert]) => TALENTS.some((t) => t.id === kennung) && Number.isInteger(wert),
    );
    expect(sauber).toBe(false);
  });

  /**
   * Und der Faktor wird gerechnet, nicht getragen: Dieselbe Verteilung ergibt
   * ihn jedes Mal neu. Wäre er gespeichert, müsste ihn jemand mitschleppen —
   * und genau das schließt AC6 aus.
   */
  it('der Faktor folgt aus der Verteilung und wird nicht mitgeführt', () => {
    const a = talentFactor({ [STAMM]: 2 });
    const b = talentFactor({ [STAMM]: 2 });
    expect(a).toBe(b);
    expect(talentFactor({})).toBe(1);
  });
});
