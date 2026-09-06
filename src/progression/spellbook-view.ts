import type { SpellDef } from '../core/data.js';
import { withPlainText } from '../analysis/help.js';
import type { SpellbookState } from './spellbook.js';

/** Dieselbe ausstehende Auswahl im Ergebnis und im Player Home. */
export function buildSpellbookView(
  book: SpellbookState,
  spells: readonly SpellDef[],
  onChoose: (spellId: string) => boolean,
): HTMLElement {
  const root = document.createElement('div');
  root.className = 'spellbook';
  const explanation = document.createElement('p');
  explanation.textContent =
    book.pendingSpellIds.length === 0
      ? 'Defeat targets to discover new spells. Quench and Purge are always available.'
      : 'Choose one spell before starting another run. Your offer is saved until you choose.';
  root.appendChild(explanation);
  for (const id of book.pendingSpellIds) {
    const spell = spells.find((entry) => entry.id === id);
    if (spell === undefined) continue;
    const choice = document.createElement('button');
    choice.type = 'button';
    choice.className = 'spellbook-choice';
    choice.dataset.spellId = id;
    const name = document.createElement('strong');
    name.textContent = spell.name;
    const detail = document.createElement('span');
    detail.textContent = `${withPlainText(spell.school ?? 'School-independent')} · ${(spell.cooldownMs / 1000).toFixed(1)} s cooldown`;
    choice.append(name, detail);
    choice.addEventListener('click', () => {
      onChoose(id);
    });
    root.appendChild(choice);
  }
  return root;
}
