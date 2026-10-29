/**
 * Die Werte-Zeile im Erklärfenster eines Zaubers (Issue #1177).
 *
 * # Aus den Zauberdaten, nicht aus dem Hilfe-Katalog
 *
 * GAME-DESIGN 17.3 verlangt im Erklärfenster „Wirkung, Werte, Synergien". Die
 * Wirkung ist ein Satz und steht in `data/help/help.json`; die Werte stehen
 * schon in `data/spells/`. Sie dort ein zweites Mal hinzuschreiben hiesse, eine
 * zweite Quelle zu pflegen, die nach der nächsten Balance-Messung falsch ist —
 * ohne dass ein Prüffall es merkt, weil beide für sich plausibel aussehen.
 *
 * Gezeigt werden die **Grundwerte** aus den Daten. Talente, Ausrüstung und
 * CATALYSIS verändern sie im Kampf; das steht in der Zeile dazu, statt dass
 * sie eine Zahl behauptet, die so nie fällt.
 */
import type { SchoolDef, SpellDef, TalentDef } from '../core/data.js';

/** Millisekunden als Sekunden, ohne überflüssige Nachkommastellen. */
function sekunden(ms: number): string {
  return `${String(ms / 1000)} s`;
}

/**
 * Die Werte eines Zaubers als eine Zeile, Teile durch „·" getrennt.
 *
 * `resourceName` ist der Name des Schul-Vorrats in Kleinbuchstaben („heat") —
 * der Zauber selbst kennt nur die Kennung, unter der er ihn füllt.
 */
export function spellValues(spell: SpellDef, resourceName = 'heat'): string {
  const teile: string[] = [`Cooldown ${sekunden(spell.cooldownMs)}`];
  if (!spell.triggersGcd) teile.push('no global cooldown');
  if (spell.castTimeMs > 0) teile.push(`cast ${sekunden(spell.castTimeMs)}`);
  if (spell.channelMs > 0) teile.push(`channel ${sekunden(spell.channelMs)}`);
  if (spell.baseDamage > 0) teile.push(`${String(spell.baseDamage)} damage`);
  for (const [, menge] of Object.entries(spell.resourceGain ?? {})) {
    teile.push(`+${String(menge)} ${resourceName}`);
  }
  if (spell.dot !== undefined) {
    teile.push(
      `${String(spell.dot.tickDamage)} damage every ${sekunden(spell.dot.tickIntervalMs)} ` +
        `for ${sekunden(spell.dot.durationMs)}`,
    );
  }
  if (spell.spender !== undefined) {
    teile.push(`${String(spell.spender.damagePerPoint)} damage per ${resourceName}`);
    teile.push(`double from ${String(spell.spender.bonusThreshold)} ${resourceName}`);
  }
  if (spell.shield !== undefined) {
    const stapel = spell.shield.maxStacks > 1 ? `, up to ${String(spell.shield.maxStacks)}` : '';
    teile.push(
      `shield ${String(spell.shield.amount)} for ${sekunden(spell.shield.durationMs)}${stapel}`,
    );
  }
  if (spell.heal !== undefined) {
    const ueber = spell.heal.overMs === undefined ? '' : ` over ${sekunden(spell.heal.overMs)}`;
    teile.push(`heals ${String(spell.heal.amount)}${ueber}`);
  }
  if (spell.controlGain !== undefined) teile.push(`+${String(spell.controlGain)} control`);
  if (spell.control !== undefined) teile.push(`costs ${String(spell.control.cost)} control`);
  if (spell.decay !== undefined) teile.push(`+${String(spell.decay.stacks)} decay`);
  if (spell.perDecayStackPercent !== undefined) {
    teile.push(`+${String(spell.perDecayStackPercent)} % per decay stack`);
  }
  return `${teile.join(' · ')}. Base values; talents, gear and CATALYSIS change them.`;
}

/** Der Name des Vorrats einer Schule in Kleinbuchstaben, oder „heat". */
export function resourceNameOf(school: SchoolDef): string {
  return school.resource.name.toLowerCase();
}

/** Promille als Prozent, ohne überflüssige Nachkommastellen. */
function prozent(promille: number): string {
  return `${String(promille / 10)} %`;
}

/**
 * Was ein Rang eines Talents bewegt — je Feld ein Satzteil (Issue #1177).
 *
 * Nur die Felder, deren Bedeutung sich ohne die Kampfrechnung sagen lässt.
 * Zwei rechnen nicht als einfacher Zuschlag und stehen deshalb ohne Zahl da:
 * `controlCostPerRankPerMille` TEILT den Preis, `decayCapPerRankPerMille`
 * wird auf ganze Stapel abgerundet. Eine Prozentzahl daran wäre eine
 * Behauptung, die der Kampf nicht einlöst.
 */
const TALENT_FELDER: readonly [keyof TalentDef, (wert: number) => string][] = [
  ['damagePerRankPerMille', (w) => `+${prozent(w)} damage per rank`],
  ['resourceGainPerRankPerMille', (w) => `+${prozent(w)} heat gained per rank`],
  ['dotDurationPerRankPerMille', (w) => `+${prozent(w)} duration of effects over time per rank`],
  ['controlGainPerRankPerMille', (w) => `+${prozent(w)} control gained per rank`],
  ['controlCostPerRankPerMille', () => 'control costs less with every rank'],
  ['controlEffectPerRankPerMille', (w) => `+${prozent(w)} control effect per rank`],
  ['decayStackPerRankPerMille', (w) => `+${prozent(w)} decay laid per rank`],
  ['decayEffectPerRankPerMille', (w) => `+${prozent(w)} worth of each decay stack per rank`],
  ['decayDurationPerRankPerMille', (w) => `+${prozent(w)} decay duration per rank`],
  ['decayCapPerRankPerMille', () => 'the decay limit grows with the ranks, in whole stacks'],
  ['decayWardPerRankPerMille', (w) => `+${prozent(w)} protection from decay per rank`],
  ['shieldAmountPerRankPerMille', (w) => `+${prozent(w)} shield strength per rank`],
  ['shieldDurationPerRankPerMille', (w) => `+${prozent(w)} shield duration per rank`],
  ['shieldStacksPerRank', (w) => `+${String(w)} shield layer per rank`],
  ['cooldownReductionPerRankPerMille', (w) => `+${prozent(w)} cooldown shortening per rank`],
  ['cooldownResetChargesPerRank', (w) => `+${String(w)} reset per rank`],
  ['cooldownThresholdPerRankPerMille', (w) => `+${prozent(w)} reset threshold per rank`],
  ['spenderDamagePerRankPerMille', (w) => `+${prozent(w)} spender damage per rank`],
  ['buildDamagePerRankPerMille', (w) => `+${prozent(w)} builder damage per rank`],
  ['convertPerRankPerMille', (w) => `+${prozent(w)} Efflux conversion per rank`],
];

/**
 * Die Werte-Zeile eines Talents: Ränge, Wirkung je Rang, Voraussetzung.
 *
 * `talentById` löst die Voraussetzung in einen Namen auf — der Spieler liest
 * „needs Kindled Core", nicht die Kennung.
 */
export function talentValues(
  talent: TalentDef,
  talentById: (id: string) => TalentDef | undefined,
): string {
  const teile: string[] = [
    talent.maxRanks === 1 ? '1 rank' : `Up to ${String(talent.maxRanks)} ranks`,
  ];
  for (const [feld, text] of TALENT_FELDER) {
    const wert = talent[feld];
    if (typeof wert === 'number' && wert !== 0) teile.push(text(wert));
  }
  if (talent.requires !== null) {
    teile.push(`needs ${talentById(talent.requires)?.name ?? talent.requires}`);
  }
  return `${teile.join(' · ')}.`;
}
