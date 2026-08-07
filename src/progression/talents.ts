/**
 * Der Talent-Faktor (Issue #417, GAME-DESIGN 12.5 und 13.1b).
 *
 * WAS HIER STEHT UND WAS NICHT
 * ----------------------------
 * Hier steht die Umrechnung von einer **Verteilung** in eine **Zahl**. Wo diese
 * Zahl in den Schaden eingeht, steht in `combat.ts` — in derselben einen
 * Schreibstelle wie `essence` und `gearEffects`, bewacht seit #196.
 *
 * **Der Topf ist eigen.** GAME-DESIGN 13.1b trennt die Töpfe: innerhalb
 * additiv, **zwischen** ihnen multiplikativ. Talente in `essence` einzurechnen
 * wäre eine gesparte Zeile und würde aus zwei Töpfen einen machen — genau der
 * Fehler, den `worn.ts` für ESSENCE ausdrücklich benennt. Die Gegenprobe dazu
 * steht in `tests/unit/talents.test.ts`: Zwei Faktoren derselben Grösse in
 * verschiedenen Töpfen ergeben ein anderes Ergebnis als beide im selben.
 *
 * DIE VERTEILUNG WIRD GERECHNET, NICHT GESPEICHERT — UMGEKEHRT
 * ------------------------------------------------------------
 * Anders als bei den verdienten Talentpunkten (#379) ist die Verteilung selbst
 * eine **Entscheidung des Spielers** und gehört damit in den Spielstand
 * (GAME-DESIGN 18.6). Was daraus folgt — dieser Faktor — wird bei jedem Kampf
 * neu gerechnet.
 *
 * EINE UNGÜLTIGE VERTEILUNG BRICHT NICHT AB
 * -----------------------------------------
 * Ränge über der Grenze werden geklemmt, unbekannte Kennungen übergangen. Ein
 * Spielstand aus einer älteren Fassung kann eine Verteilung tragen, die es
 * heute nicht mehr gibt; ein Kampf, der deswegen gar nicht startet, wäre
 * schlimmer als einer, der die alte Verteilung zu Ende rechnet. Die Prüfung,
 * was verteilt werden DARF, gehört an die Stelle, wo verteilt wird — nicht
 * hierher.
 */
import { TALENTS, type TalentDef } from '../core/data.js';

/** Was der Spieler verteilt hat: Talent-Kennung auf Ränge. */
export type TalentAllocation = Readonly<Record<string, number>>;

/** Nichts verteilt — der Zustand jedes neuen Spielstands. */
export const NO_TALENTS: TalentAllocation = {};

/** Ein Talent nach seiner Kennung, oder `undefined`. */
export function talentById(id: string): TalentDef | undefined {
  return TALENTS.find((talent) => talent.id === id);
}

/**
 * Wie viele Ränge in diesem Talent wirklich zählen.
 *
 * Geklemmt an `maxRanks`, nie negativ. Eine Verteilung mit sieben Rängen in
 * einem fünfstufigen Talent ist kein Grund abzubrechen — sie ist alt.
 */
export function effectiveRanks(allocation: TalentAllocation, talent: TalentDef): number {
  const roh = allocation[talent.id] ?? 0;
  if (!Number.isFinite(roh) || roh <= 0) return 0;
  return Math.min(Math.trunc(roh), talent.maxRanks);
}

/** Wie viele Punkte diese Verteilung insgesamt verbraucht. */
export function spentPoints(allocation: TalentAllocation): number {
  let summe = 0;
  for (const talent of TALENTS) summe += effectiveRanks(allocation, talent);
  return summe;
}

/**
 * Der Schadensfaktor aus einer Verteilung.
 *
 * **Innerhalb des Topfes additiv** (13.1b): Die Promille aller Ränge werden
 * summiert und einmal zu 1 addiert. Wer sie stattdessen einzeln
 * multiplizierte, machte aus einem Topf so viele Töpfe, wie es Talente gibt —
 * und die Summe wüchse schneller, als die Balance es vorsieht.
 *
 * Das Muster ist `essenceFactor` in `src/gear/worn.ts`: `1 + promille / 1000`.
 */
/**
 * Die Kennung des ersten Schlüsseltalents (Issue #418).
 *
 * **Als Konstante und nicht als Zeichenkette an der Fundstelle**, damit ein
 * Tippfehler beim Übersetzen auffällt statt im Kampf. Ein `hasKeystone('split-spener')`
 * wäre still `false` — der Zauber wirkte weiter wie vorher, und niemand sähe,
 * dass das Talent nichts tut.
 */
export const KEYSTONE_SPLIT_SPENDER = 'split-spender';

/** Ob die Verteilung ein bestimmtes Verhalten freigeschaltet hat (#418). */
export function hasKeystone(allocation: TalentAllocation, keystone: string): boolean {
  return TALENTS.some(
    (talent) => talent.keystone === keystone && effectiveRanks(allocation, talent) > 0,
  );
}

export function talentFactor(allocation: TalentAllocation): number {
  let promille = 0;
  for (const talent of TALENTS) {
    promille += effectiveRanks(allocation, talent) * talent.damagePerRankPerMille;
  }
  return 1 + promille / 1000;
}

/**
 * Ob eine Verteilung nach den heutigen Regeln zulässig wäre.
 *
 * **Getrennt von der Rechnung, und das ist der Zweck.** Wer verteilt, fragt
 * hier; wer rechnet, klemmt. So kann die Oberfläche einen Fehler zeigen, ohne
 * dass ein alter Spielstand unspielbar wird.
 *
 * @param available Wie viele Punkte die Stufe eingebracht hat (`talentPointsAtLevel`)
 */
export function allocationProblem(allocation: TalentAllocation, available: number): string | null {
  for (const [id, ranks] of Object.entries(allocation)) {
    const talent = talentById(id);
    if (talent === undefined) return `Unbekanntes Talent "${id}"`;
    if (!Number.isFinite(ranks) || ranks < 0 || !Number.isInteger(ranks)) {
      return `Talent "${id}" hat ${String(ranks)} Ränge — erlaubt sind ganze Zahlen ab 0`;
    }
    if (ranks > talent.maxRanks) {
      return `Talent "${id}" hat ${String(ranks)} Ränge, erlaubt sind ${String(talent.maxRanks)}`;
    }
    // **Der Stamm zuerst** (12.5): Ein Weg ohne seinen Stamm wäre eine Liste,
    // kein Baum.
    if (ranks > 0 && talent.requires !== null && (allocation[talent.requires] ?? 0) <= 0) {
      return `Talent "${id}" verlangt "${talent.requires}"`;
    }
  }
  const verbraucht = spentPoints(allocation);
  if (verbraucht > available) {
    return `${String(verbraucht)} Punkte verteilt, ${String(available)} verdient`;
  }
  return null;
}
