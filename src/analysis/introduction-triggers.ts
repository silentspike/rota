/**
 * Wann welche geführte Einführung fällig ist (Issue #1177; GAME-DESIGN 17.3).
 *
 * # Drei Quellen, eine Reihenfolge
 *
 * ```text
 *   der Spielstand   'reaction' — sobald es einen ersten Lauf gab
 *   der Boss         seine Bausteine, in der Reihenfolge seiner Zeitleiste
 *   die Leiste       Abklingzeit, die Vorräte der Schulen, Synergie, Rücksetzer
 * ```
 *
 * Zuerst der Ring, weil ihn jeder Kampf hat; dann der Boss, weil er gleich
 * kommt; zuletzt die eigene Leiste. Gezeigt wird die ERSTE ungesehene —
 * mehrere Fenster übereinander wären keine Führung, sondern ein Stapel.
 *
 * # Der allererste Start bekommt keine
 *
 * Masterplan, Einstieg: „Sofort drücken, Erklärung nebenbei. Kein Menü, kein
 * Vorspann." Dort steht nur die Zeile „Press Q" auf der Zeichenfläche
 * ([`firstPressHint`]). Die Einführungen beginnen, wenn der Spieler den Ring
 * einmal gesehen hat.
 *
 * # Die Kennungen sind die Kupplung
 *
 * Jede Kennung, die hier entsteht, braucht ihre Einführung in
 * `data/help/help.json` — und jede Einführung dort eine Kennung, die hier
 * entstehen kann. `tests/unit/einfuehrung-anschluss.test.ts` hält beide
 * Richtungen über den ganzen ausgelieferten Inhalt.
 */
import type { BossDef, SpellDef } from '../core/data.js';
import { GCD_MS } from '../core/gcd.js';
import { feedsMechanic, MECHANICS, spendsMechanic } from '../combat/schools/indicators.js';

/** Die Einführung, die nach dem ersten Lauf kommt: der Ring und die Uhr. */
export const FIRST_RUN_MECHANIC = 'reaction';

/**
 * Die Bausteine eines Bosses, jeder einmal (GAME-DESIGN 11.2).
 *
 * Zuerst die Mechaniken seiner Zeitleiste in deren Reihenfolge — die begegnen
 * dem Spieler zuerst —, dann die Bausteine, die als eigenes Feld am Boss
 * stehen.
 */
export function bossMechanics(boss: BossDef): readonly string[] {
  const out: string[] = [];
  const dazu = (id: string): void => {
    if (!out.includes(id)) out.push(id);
  };
  for (const eintrag of boss.timeline) {
    if (eintrag.mechanic !== '') dazu(eintrag.mechanic);
  }
  if (boss.phases.length > 0) dazu('phases');
  if (boss.doom !== null) dazu('doom');
  if (boss.affliction !== null) dazu('affliction');
  if (boss.race !== null) dazu('race');
  if (boss.bossHeal !== null) dazu('bossHeal');
  // **Nur mit Wiederholung.** `escalation` wirkt erst, wenn die Zeitleiste
  // wiederkehrt (boss-1-1: „escalation wirkt NUR, wenn timelineLoopMs gesetzt
  // ist") — ohne sie gäbe es nichts, was schneller würde.
  if (boss.escalation !== null && boss.timelineLoopMs !== null) dazu('escalation');
  if (boss.runaway !== null) dazu('runaway');
  if (Object.values(boss.resistances).some((faktor) => faktor !== 1)) dazu('resistance');
  if (boss.energy !== null) dazu('energy');
  if (boss.stacks !== null) dazu('stacks');
  return out;
}

/**
 * Was die Leiste an Mechanik mitbringt (GAME-DESIGN 17.3: „erste Abklingzeit,
 * erste Synergie").
 *
 * - `cooldown`: ein Zauber, dessen Abklingzeit länger ist als die globale
 *   Wartezeit — erst dann ist sie etwas Eigenes.
 * - `heat`, `control`, `decay`: ein Zauber, der den Vorrat seiner Schule füllt.
 * - `synergy`: ein Füller und ein Verbraucher desselben Vorrats, oder ein Paar
 *   aus `synergy.partner`.
 * - `reset`: ein Zauber, der den Auswahlmodus scharf stellt.
 */
export function loadoutMechanics(spells: readonly SpellDef[]): readonly string[] {
  const out: string[] = [];
  if (spells.some((s) => s.cooldownMs > GCD_MS)) out.push('cooldown');
  for (const mechanic of MECHANICS) {
    if (spells.some((s) => feedsMechanic(s, mechanic.id))) out.push(mechanic.id);
  }
  const ids = new Set(spells.map((s) => s.id));
  const vorratsPaar = MECHANICS.some(
    (m) =>
      spells.some((s) => feedsMechanic(s, m.id)) && spells.some((s) => spendsMechanic(s, m.id)),
  );
  const partnerPaar = spells.some((s) => s.synergy !== undefined && ids.has(s.synergy.partner));
  if (vorratsPaar || partnerPaar) out.push('synergy');
  if (spells.some((s) => (s.cooldownResetCharges ?? 0) > 0)) out.push('reset');
  return out;
}

/**
 * Alle fälligen Kennungen in der Reihenfolge, in der sie erklärt werden.
 *
 * Leer, solange es keinen Lauf gab — der allererste Start bekommt kein
 * Fenster (Masterplan, Einstieg).
 */
export function dueMechanics(
  hasPlayed: boolean,
  boss: BossDef,
  barSpells: readonly SpellDef[],
): readonly string[] {
  if (!hasPlayed) return [];
  return [FIRST_RUN_MECHANIC, ...bossMechanics(boss), ...loadoutMechanics(barSpells)];
}

/**
 * Die Zeile auf der Zeichenfläche beim allerersten Kampf (Masterplan,
 * Einstieg: „ein Symbol leuchtet, eine Zeile sagt ‚Press Q'").
 *
 * Sie steht, bis der erste Druck angenommen ist, und nur, solange es keinen
 * gezählten Lauf gibt. **Die Beschriftung kommt aus dem Layout** des Spielers
 * (`KeyLabels`), nicht aus dem Text: Wer eine andere Tastatur hat, liest die
 * Taste, die unter seinem Finger liegt.
 */
export function firstPressHint(
  hasPlayed: boolean,
  pressesThisFight: number,
  keyLabel: string,
): string | null {
  if (hasPlayed || pressesThisFight > 0) return null;
  return `Press ${keyLabel}`;
}
