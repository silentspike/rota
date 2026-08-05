/**
 * Was ein Stufenaufstieg dem Spieler bringt — für die Anzeige (Issue #371).
 *
 * # Warum es diese Datei gibt
 *
 * `leveledUp` war berechnet, geprüft und **tot**: Das Spiel stieg den Spieler
 * auf und sagte es ihm nicht. Ein ungenutztes `xpGained` ist eine tote Zahl,
 * ein ungenutztes `leveledUp` ist ein **fehlendes Ereignis** — laut Design
 * (12.1) ist der Aufstieg *der* Fortschrittsmoment.
 *
 * # Nur, was wirklich eintritt
 *
 * GAME-DESIGN 12.1 sagt, eine Stufe gebe „alles vier". Gemessen an den Daten
 * ist das eine Aussage über das **System**, nicht über jede einzelne Stufe:
 *
 * | Was | wann |
 * | --- | --- |
 * | Grundwerte | jede Stufe (`growthPerLevelPercent`) |
 * | Leisten-Plätze | zehn von hundert (`barSlotLevels`) |
 * | neue Zauber | heute drei Stufen (`unlockLevel`) |
 * | Talentpunkte | **nirgends vergeben** — siehe unten |
 *
 * Auf Stufe 7 gibt es nur die Grundwerte. Eine Einblendung mit vier Zeilen,
 * von denen drei leer sind, wäre schlechter als keine.
 *
 * # Talentpunkte fehlen, und das ist kein Versehen dieser Datei
 *
 * `talentPoints` steht im Spielstand (`save-format.ts`), aber
 * `data/player/levels.json` kennt kein `talentPointsPerLevel` — sie werden
 * **nirgends vergeben**. Deshalb steht hier keine Zeile dafür, auch keine
 * mit einer Null: Eine Null behauptet, es gäbe welche und diesmal seien es
 * keine. Das Fehlen ist ein eigener Befund und hat ein eigenes Issue.
 */
import { LEVELS, type SpellDef } from '../core/data.js';
import { barSlotsAtLevel } from '../progression/unlocks.js';

/** Was eine Stufe gebracht hat — leere Listen heißen: nichts davon. */
export interface LevelUpDisplay {
  /** Die erreichte Stufe. */
  readonly level: number;
  /** Um wie viel Prozent die Grundwerte steigen. Immer gesetzt. */
  readonly statPercent: number;
  /** Zauber, die mit dieser Stufe freigeschaltet wurden. */
  readonly newSpells: readonly string[];
  /** Wie viele Leisten-Plätze dazugekommen sind — 0, wenn keiner. */
  readonly newBarSlots: number;
}

/**
 * Was der Aufstieg auf diese Stufe gebracht hat.
 *
 * `null`, wenn kein Aufstieg stattfand — der Aufrufer zeigt dann nichts. Die
 * Entscheidung fällt hier und nicht in der Anzeige: Ob es ein Ereignis gab,
 * ist eine Frage an die Daten, nicht an die Darstellung.
 */
export function levelUpDisplayFor(
  leveledUp: boolean,
  level: number,
  spells: readonly SpellDef[],
): LevelUpDisplay | null {
  if (!leveledUp) return null;
  const stufe = Math.max(Math.trunc(level), 1);
  return {
    level: stufe,
    statPercent: LEVELS.growthPerLevelPercent,
    // **Nur die dieser Stufe**, nicht alle bis hierher: Der Spieler will
    // wissen, was JETZT dazugekommen ist.
    newSpells: spells.filter((spell) => spell.unlockLevel === stufe).map((spell) => spell.name),
    // Die Differenz zur vorigen Stufe. `barSlotsAtLevel` sinkt nie, also ist
    // sie nie negativ.
    newBarSlots: barSlotsAtLevel(stufe) - barSlotsAtLevel(stufe - 1),
  };
}
