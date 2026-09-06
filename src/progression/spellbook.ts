import type { SpellDef } from '../core/data.js';
import { RandomStream, seedForStream } from '../core/rng.js';

/** Besitz und ein bereits gezogenes Angebot sind Entscheidungen, keine Kampfwerte. */
export interface SpellbookState {
  readonly knownSpellIds: readonly string[];
  readonly pendingSpellIds: readonly string[];
}

export const STARTER_SPELL_IDS = ['kindle', 'smoulder'] as const;
export const SPELL_DRAFT_SIZE = 3;
/** Benannte Implementierungsentscheidung: 2:1, solange Survival noch fehlt. */
export const FIRST_SURVIVAL_WEIGHT = 2;
const DRAFT_STREAM_INDEX = 4;

function playable(spell: SpellDef): boolean {
  return spell.unlockLevel <= 100;
}

export function newSpellbook(spells: readonly SpellDef[]): SpellbookState {
  return {
    knownSpellIds: spells
      .filter(
        (spell) =>
          playable(spell) &&
          (spell.school === null || STARTER_SPELL_IDS.some((id) => id === spell.id)),
      )
      .map((spell) => spell.id),
    pendingSpellIds: [],
  };
}

/** Alte Spieler behalten jeden bisher zugänglichen Zauber, auch ihre Belegung. */
export function legacySpellbook(
  level: number,
  spells: readonly SpellDef[],
  bindings: Readonly<Record<string, string>>,
): SpellbookState {
  const equipped = new Set(Object.values(bindings));
  return {
    knownSpellIds: spells
      .filter(
        (spell) =>
          playable(spell) &&
          (spell.unlockLevel <= level || spell.school === null || equipped.has(spell.id)),
      )
      .map((spell) => spell.id),
    pendingSpellIds: [],
  };
}

/** Common ist immer verfügbar; Prüfzauber gehören nie zum Besitz. */
export function spellbookSpells(book: SpellbookState, spells: readonly SpellDef[]): SpellDef[] {
  const known = new Set(book.knownSpellIds);
  return spells.filter(
    (spell) => playable(spell) && (spell.school === null || known.has(spell.id)),
  );
}

/** Entfernte Angebots-Einträge dürfen den nächsten Kampf nicht dauerhaft sperren. */
export function reconcileSpellbook(
  book: SpellbookState,
  spells: readonly SpellDef[],
): SpellbookState {
  const available = new Set(
    spells.filter((spell) => playable(spell) && spell.school !== null).map((spell) => spell.id),
  );
  return {
    ...book,
    pendingSpellIds: book.pendingSpellIds.filter(
      (id) => available.has(id) && !book.knownSpellIds.includes(id),
    ),
  };
}

export function isSpellbookState(value: unknown): value is SpellbookState {
  if (value === null || typeof value !== 'object') return false;
  const raw = value as Record<string, unknown>;
  for (const field of ['knownSpellIds', 'pendingSpellIds']) {
    const ids = raw[field];
    if (!Array.isArray(ids) || ids.some((id) => typeof id !== 'string' || id.length === 0))
      return false;
    if (new Set(ids).size !== ids.length) return false;
  }
  const pending = raw['pendingSpellIds'] as string[];
  const known = raw['knownSpellIds'] as string[];
  return pending.length <= SPELL_DRAFT_SIZE && pending.every((id) => !known.includes(id));
}

/** Keine erneute Ziehung eines offenen Angebots, auch nach weiteren Aufrufen. */
export function offerSpellDraft(
  book: SpellbookState,
  level: number,
  spells: readonly SpellDef[],
  seed: number,
): SpellbookState {
  if (book.pendingSpellIds.length > 0) return book;
  const known = new Set(book.knownSpellIds);
  const hasSurvival = spells.some((spell) => known.has(spell.id) && spell.school === 'COAGULATION');
  const candidates = spells.filter(
    (spell) =>
      playable(spell) &&
      spell.school !== null &&
      spell.unlockLevel <= level &&
      !known.has(spell.id),
  );
  const random = new RandomStream(seedForStream(seed, DRAFT_STREAM_INDEX));
  const pendingSpellIds: string[] = [];
  const weight = (spell: SpellDef): number =>
    !hasSurvival && spell.school === 'COAGULATION' ? FIRST_SURVIVAL_WEIGHT : 1;
  while (pendingSpellIds.length < SPELL_DRAFT_SIZE && candidates.length > 0) {
    let draw = random.next() * candidates.reduce((sum, spell) => sum + weight(spell), 0);
    let index = 0;
    for (; index < candidates.length - 1; index++) {
      draw -= weight(candidates[index]!);
      if (draw < 0) break;
    }
    const selected = candidates.splice(index, 1)[0];
    if (selected !== undefined) pendingSpellIds.push(selected.id);
  }
  return pendingSpellIds.length === 0 ? book : { ...book, pendingSpellIds };
}

/** Genau ein angebotenes Stück erwerben; doppelte oder fremde Auswahl wirkt nicht. */
export function chooseDraftSpell(book: SpellbookState, spellId: string): SpellbookState | null {
  if (!book.pendingSpellIds.includes(spellId) || book.knownSpellIds.includes(spellId)) return null;
  return { knownSpellIds: [...book.knownSpellIds, spellId], pendingSpellIds: [] };
}
