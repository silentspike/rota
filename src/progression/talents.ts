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

/**
 * SUBLIMATIONs Schlüsseltalent (#419): Der Rücksetzer **wählt nicht mehr**.
 *
 * Ohne das Talent schaltet ein Rücksetzer einen Auswahlmodus scharf, und der
 * nächste Tastendruck bestimmt, welche Abklingzeit fällt — er kostet also eine
 * zusätzliche Handlung und eine Entscheidung unter Zeitdruck.
 *
 * Mit dem Talent trifft er von selbst den Zauber mit der **längsten
 * Restzeit**. Das ist ein Verhalten und keine Zahl: Die Rotation verliert
 * einen Tastendruck und gewinnt dafür die schlechteste aller Wartezeiten
 * zurück — wer vorher den Auswahlmodus einplante, plant jetzt anders.
 */
export const KEYSTONE_AUTO_RESET = 'auto-reset';

/**
 * FIXATIONs Schlüsseltalent (#419): Ein Kontroll-Zauber **scheitert nicht mehr**.
 *
 * Ohne das Talent bricht er ab, wenn nicht genug Kontrolle da ist — der Druck
 * ist verloren, und `controlFailed` zählt hoch. Wer FIXATION spielt, wartet
 * deshalb, bis der Vorrat reicht.
 *
 * Mit dem Talent wirkt er mit dem, was da ist: Der Einsatz ist der halbe
 * Vorrat statt des vollen Preises, und die **Wirkung sinkt im selben
 * Verhältnis**. Das ist der Unterschied zu einem Rabatt — billiger wäre ein
 * Faktor, anteilig ist eine andere Rechnung.
 */
export const KEYSTONE_PARTIAL_CONTROL = 'partial-control';

/**
 * COAGULATIONs Schlüsseltalent (#419): Ein brechender Schild **legt nach**.
 *
 * Ohne das Talent ist die Schicht weg, sobald sie aufgebraucht ist — der
 * nächste Schlag trifft voll. Mit ihm entsteht im selben Moment eine halbe
 * neue Schicht, **einmal je gelegtem Schild**.
 *
 * Die Begrenzung ist der Kern: Ohne sie legte jeder Bruch nach, der nächste
 * wieder, und die Schicht wäre nicht mehr aufzubrauchen. Ein Schlüsseltalent
 * baut die Rotation um, es hebelt sie nicht aus.
 */
export const KEYSTONE_SHIELD_REFUND = 'shield-refund';

/**
 * PUTREFACTIONs Schlüsseltalent (#419): Ein erloschener Brand **legt weiter
 * Stapel**, bis die Lage vergangen ist, die bei seinem Ende lag.
 *
 * **Der erste Entwurf war wirkungslos, und das war messbar.** Er ließ beim
 * Verfall eines Stapels jeden laufenden Brand einmal ticken — nur überlappen
 * Brand-Laufzeit und Stapel-Laufzeit im Normalfall gar nicht:
 *
 * ```text
 *   t=12000   4 Stapel   Brand AKTIV
 *   t=16000   2 Stapel   Brand NICHT MEHR aktiv   <- hier verfällt der erste
 * ```
 *
 * Genau diese Messung trägt jetzt das Talent: Bei t=16000 liegen Stapel, und
 * der Brand ist aus. Die Lücke, in der der alte Entwurf nichts fand, ist die
 * Zeit, die der neue füllt.
 *
 * **Der Nachlauf legt nur Stapel, keinen Schaden.** Ein toter Brand, der
 * weiter zuschlägt, wäre eine verlängerte Laufzeit — das ist bereits „Lasting
 * Rot". Hier wächst allein die Zersetzung.
 *
 * **Und er endet.** Maßgeblich ist die Frist des jüngsten Stapels, der beim
 * Erlöschen lag; was der Nachlauf selbst legt, verlängert ihn nicht. Ohne
 * diese Grenze hielte ein einziger Brand seine Stapel für immer am Leben,
 * weil jeder neue Stapel den nächsten trüge.
 */
export const KEYSTONE_DECAY_ECHO = 'decay-echo';

/** Ob die Verteilung ein bestimmtes Verhalten freigeschaltet hat (#418). */
export function hasKeystone(allocation: TalentAllocation, keystone: string): boolean {
  return TALENTS.some(
    (talent) => talent.keystone === keystone && effectiveRanks(allocation, talent) > 0,
  );
}

export function talentFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.damagePerRankPerMille);
}

/**
 * Der Faktor auf den **Ressourcen-Gewinn** (Issue #419, Weg „build").
 *
 * Eine eigene Funktion und kein zweiter Parameter an `talentFactor`: Die drei
 * Größen landen an drei verschiedenen Stellen im Kampf, und eine gemeinsame
 * Funktion mit einem Schalter läse sich an jeder Aufrufstelle wie ein Zufall.
 */
export function talentResourceFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.resourceGainPerRankPerMille);
}

/** Der Faktor auf die **Laufzeit** der Effekte über Zeit (Weg „hold"). */
export function talentDotDurationFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.dotDurationPerRankPerMille);
}

/**
 * Der Faktor auf den **Preis** der Kontroll-Wirkungen (FIXATION, Issue #419).
 *
 * **Er wird GETEILT, nicht multipliziert** — als einziger der sieben. Ein Weg
 * „billiger ausgeben" senkt den Preis; ein Faktor 1,2 darauf multipliziert
 * hieße teurer. Die Aufrufstelle im Kampf teilt deshalb und klemmt auf
 * mindestens 1 Punkt: Ein Ausgeber, der nichts kostet, wäre keiner.
 */
export function talentControlCostFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.controlCostPerRankPerMille);
}

/**
 * Der Faktor auf den **Aufbau** der Kontroll-Punkte (FIXATION, Issue #419).
 *
 * Die Wirkstelle steht seit GRIPLOCK (#489); dies ist die Speisung aus einem
 * Talent.
 */
export function talentControlGainFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.controlGainPerRankPerMille);
}

/** Der Faktor auf die **Wertigkeit** eines Fäulnis-Stapels (PUTREFACTION). */
export function talentDecayEffectFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.decayEffectPerRankPerMille);
}

/** Der Faktor auf die **Stärke** der Kontroll-Wirkungen (FIXATION). */
export function talentControlEffectFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.controlEffectPerRankPerMille);
}

/** Der Faktor auf die Zahl gelegter **Fäulnis-Stapel** (PUTREFACTION). */
export function talentDecayStackFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.decayStackPerRankPerMille);
}

/** Der Faktor auf die **Haltbarkeit** der Fäulnis-Stapel (PUTREFACTION). */
export function talentDecayDurationFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.decayDurationPerRankPerMille);
}

/** Der Faktor auf die **Dicke** der Schutzschicht (COAGULATION). */
export function talentShieldAmountFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.shieldAmountPerRankPerMille);
}

/** Der Faktor auf die **Dauer** der Schutzschicht (COAGULATION). */
export function talentShieldDurationFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.shieldDurationPerRankPerMille);
}

/**
 * Wie viele **zusätzliche Stapel** die Schutzschicht verträgt (COAGULATION).
 *
 * **Kein Faktor, sondern eine ganze Zahl** — als einzige der sieben. Ein
 * Faktor 1,5 auf `maxStacks: 3` ergäbe 4,5, und eine halbe Stapel-Erlaubnis
 * gibt es nicht. Deshalb summiert diese Funktion Ränge statt Promille und
 * gibt einen Zuschlag zurück, keinen Multiplikator. Dieselbe Wahl wie bei
 * PUTRID BLOOMs `decayCap.bonus`.
 */
export function talentShieldStacksBonus(allocation: TalentAllocation): number {
  let zuschlag = 0;
  for (const talent of TALENTS) {
    zuschlag += effectiveRanks(allocation, talent) * talent.shieldStacksPerRank;
  }
  return zuschlag;
}

/** Der Faktor auf den **Abzug** an Abklingzeit (SUBLIMATION). */
export function talentCooldownReductionFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.cooldownReductionPerRankPerMille);
}

/**
 * Wie viele **zusätzliche Rücksetz-Ladungen** dazukommen (SUBLIMATION).
 *
 * **Kein Faktor, sondern eine ganze Zahl** — zweite dieser Art nach
 * `talentShieldStacksBonus`, aus demselben Grund: `cooldownResetCharges`
 * zählt Ladungen, und ein Faktor 1,5 auf `reflux: 1` ergäbe 1,5.
 */
export function talentCooldownChargesBonus(allocation: TalentAllocation): number {
  let zuschlag = 0;
  for (const talent of TALENTS) {
    zuschlag += effectiveRanks(allocation, talent) * talent.cooldownResetChargesPerRank;
  }
  return zuschlag;
}

/** Der Faktor auf die **Schwelle**, ab der der Abzug greift (SUBLIMATION). */
export function talentCooldownThresholdFactor(allocation: TalentAllocation): number {
  return faktorAus(allocation, (t) => t.cooldownThresholdPerRankPerMille);
}

/**
 * Summiert die Promille EINER Größe über alle verteilten Ränge.
 *
 * **Der Faktor entsteht als `1 + Summe/1000` aus ganzen Zahlen** — die
 * Determinismus-Regel aus GAME-DESIGN 18.4, dieselbe wie bei
 * `factorPerMille` in `levels.json`. Eine Fließkomma-Angabe je Rang wäre eine
 * Zahl, bei der Spiel und Simulator auseinanderlaufen könnten, bevor die
 * Multiplikation überhaupt beginnt.
 */
function faktorAus(
  allocation: TalentAllocation,
  promilleVon: (talent: (typeof TALENTS)[number]) => number,
): number {
  let promille = 0;
  for (const talent of TALENTS) {
    promille += effectiveRanks(allocation, talent) * promilleVon(talent);
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
