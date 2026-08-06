import type { SpellIndex } from '../../core/combat.js';
/**
 * Welche Schul-Anzeigen über der Leiste erscheinen (GAME-DESIGN 5, Issue E3-5).
 *
 * > **Nur die aktiv genutzten**, direkt über der Zauberleiste. Wer keine
 * > CALCINATION mitführt, sieht keine Hitze-Anzeige. Nie mehr als drei bis vier
 * > gleichzeitig, und sie sitzen dort, wo der Blick ohnehin ist.
 *
 * Zwei Regeln, und beide sind Auswahl, nicht Zeichnung — deshalb stehen sie
 * hier und nicht in `src/render/`:
 *
 * 1. **Aktiv genutzt heißt: ein Zauber auf der Leiste zahlt darauf ein.** Nicht
 *    „die Schule existiert", nicht „der Spieler hat sie freigeschaltet". Eine
 *    Anzeige ohne zugehörigen Zauber wäre eine Zahl, die sich nie bewegt.
 * 2. **Höchstens vier.** Darüber hinaus wird abgeschnitten, nicht verkleinert:
 *    Fünf Anzeigen nebeneinander sind bei jeder Größe fünf Dinge, auf die der
 *    Blick fallen kann, und das Design nennt die Grenze in Anzeigen, nicht in
 *    Pixeln.
 *
 * # Welche vier, wenn es mehr sind
 *
 * Nach der **Zahl der einzahlenden Zauber**, bei Gleichstand nach der Kennung.
 * Begründung: Die Mechanik, auf die vier Zauber einzahlen, bewegt sich öfter
 * als die mit einem — sie ist die, die der Spieler im Blick behalten muss. Bei
 * Gleichstand entscheidet die Kennung, damit die Reihenfolge zwischen zwei
 * Bildern nicht springt.
 *
 * Das Design nennt keine Auswahlregel für den Fall; es nennt nur die Grenze.
 * Diese hier ist Umsetzung und in einer Zeile zu ändern.
 */

/** Nie mehr als so viele Anzeigen gleichzeitig (GAME-DESIGN 5). */
export const MAX_INDICATORS = 4;

/**
 * Die Schul-Mechaniken, die der Kampf führt (Issue #270).
 *
 * **Diese Tabelle ist die erste Stelle, an der die drei zusammenstehen** —
 * und das war der Befund, aus dem sie entstand. Der Kampf führt sie längst:
 *
 * | Zähler | in `combat.ts` | Schule |
 * | --- | --- | --- |
 * | `heat` | `resource()` | CALCINATION |
 * | `control` | `control()` | FIXATION |
 * | `decay` | `decayStacks()` | PUTREFACTION |
 *
 * Bis zum 2026-08-05 kannte diese Auswahl nur die Hitze, weil sie über
 * `SchoolDef.resource` ging — und dort steht nur eine. Die anderen zwei
 * rechneten mit, ohne dass es irgendwo ankam: `MAX_INDICATORS` steht auf 4,
 * geliefert wurde EINE. Die Konstante existiert nur, weil jemand mit mehreren
 * gerechnet hat.
 *
 * GAME-DESIGN 5 verlangt „nur die aktiv genutzten" — das heißt, dass eine
 * genutzte Mechanik ANGEZEIGT wird. Wer Kontroll-Punkte sammelt und sie nicht
 * sieht, ist kein Zustand, den jemand entschieden hat.
 *
 * **Woher die Zuordnung kommt:** Aus den Zauberdaten, nicht aus dieser
 * Tabelle. `feedsMechanic` liest die Felder, die den Beitrag tragen; hier
 * steht nur, welche Mechaniken es gibt und wie sie heißen. Eine Liste von
 * Zauber-Kennungen wäre die Handabschrift, die beim nächsten Zauber vergessen
 * wird — und #268 baut neun neue.
 */
export interface MechanicDef {
  /** Kennung, wie sie in der Anzeige und in `MechanicInfo` steht. */
  readonly id: string;
  /** Name für den Spieler, englisch (Design-Gesetz 5). */
  readonly label: string;
  /** Zu welcher Schule sie gehört. */
  readonly school: string;
}

/** Die drei, in der Reihenfolge der Schulen in den Daten. */
export const MECHANICS: readonly MechanicDef[] = [
  { id: 'heat', label: 'Heat', school: 'CALCINATION' },
  { id: 'control', label: 'Control', school: 'FIXATION' },
  { id: 'decay', label: 'Decay', school: 'PUTREFACTION' },
];

/** Was ein Zauber mitbringen muss, damit er auf eine Mechanik einzahlt. */
export interface MechanicContributor {
  readonly resourceGain?: Readonly<Record<string, number>>;
  readonly dot?: { readonly resourceGainPerTick: Readonly<Record<string, number>> };
  readonly controlGain?: number;
  readonly decay?: unknown;
}

/**
 * Ob dieser Zauber auf diese Mechanik einzahlt.
 *
 * **Die eine Stelle, die das entscheidet.** Sie liest die Datenfelder, nicht
 * eine Liste von Kennungen: Ein neuer Zauber ist damit weiterhin ein
 * Datensatz (Leitplanke 6).
 *
 * Der Vorrat der Schule läuft über `resourceGain` und `dot.resourceGainPerTick`
 * — beides, weil ein Effekt über Zeit genauso einzahlt wie eine Wirkung. Das
 * ist keine Feinheit: SMOULDER speist die Hitze ausschließlich über Ticks.
 */
export function feedsMechanic(spell: MechanicContributor, mechanicId: string): boolean {
  if (mechanicId === 'control') return (spell.controlGain ?? 0) > 0;
  if (mechanicId === 'decay') return spell.decay !== undefined;
  return (
    (spell.resourceGain?.[mechanicId] ?? 0) > 0 ||
    (spell.dot?.resourceGainPerTick[mechanicId] ?? 0) > 0
  );
}

/** Was ein Zauber mitbringen muss, damit er von einer Mechanik ZEHRT. */
export interface MechanicSpender {
  readonly spender?: { readonly resource: string };
  readonly control?: unknown;
  readonly perDecayStackPercent?: number;
}

/**
 * Ob dieser Zauber von dieser Mechanik zehrt.
 *
 * **Über die Felder, nicht über eine Liste von Kennungen.** Eine Liste wäre
 * die Handabschrift, die beim nächsten Entlade-Zauber vergessen wird — und
 * #268 baut neun neue.
 *
 * Sie steht neben `feedsMechanic` und nicht darin: Einzahlen und Zehren sind
 * zwei Größen. Ein Zauber könnte beides tun, und eine Funktion, die „speist
 * oder zehrt" beantwortet, könnte die beiden nie wieder trennen.
 */
export function spendsMechanic(spell: MechanicSpender, mechanicId: string): boolean {
  if (mechanicId === 'control') return spell.control !== undefined;
  if (mechanicId === 'decay') return (spell.perDecayStackPercent ?? 0) > 0;
  return spell.spender?.resource === mechanicId;
}

/** Eine Mechanik, die eine Anzeige haben könnte. */
export interface MechanicInfo {
  /** Kennung des Vorrats, z. B. `heat`. */
  readonly id: string;
  /** Wie viele Zauber der Leiste darauf einzahlen. */
  readonly contributors: number;
}

/**
 * Welche Anzeigen erscheinen, in ihrer Reihenfolge.
 *
 * Läuft beim Zusammenstellen der Leiste, nicht im Bild — hier darf eine Liste
 * entstehen. Das Bild liest nur das Ergebnis.
 */
export function visibleIndicators(mechanics: readonly MechanicInfo[]): string[] {
  const genutzt = mechanics.filter((mechanic) => mechanic.contributors > 0);
  genutzt.sort((a, b) => b.contributors - a.contributors || a.id.localeCompare(b.id));
  return genutzt.slice(0, MAX_INDICATORS).map((mechanic) => mechanic.id);
}

/**
 * Die Mechaniken, die eine Zusammenstellung tatsächlich nutzt.
 *
 * **Gezählt wird über die Leiste, nicht über die Schule.** Ein Zauber zahlt
 * ein, wenn er beim Wirken oder bei einem Tick etwas beisteuert — steht kein
 * solcher Zauber auf der Leiste, gibt es nichts anzuzeigen.
 */
export function mechanicsOfLoadout(source: LoadoutSource): MechanicInfo[] {
  // **Alle drei, nicht nur die der Schule** (Issue #270). Vorher stand hier
  // `source.resourceId` und damit genau eine — die anderen zwei rechneten im
  // Kampf mit, ohne dass es irgendwo ankam.
  // **GEZÄHLT WERDEN QUELLEN, NICHT PLÄTZE** (Issue #287). Wer denselben
  // Zauber auf zwei Plätze legt, hat eine Quelle auf zwei Knöpfen — keine
  // zweite. Vorher zählte diese Schleife linear hoch, und damit waren zwei
  // Zusammenstellungen nicht mehr zu unterscheiden:
  //
  //   KINDLE zweimal        Heat · 2 feeding
  //   KINDLE + SMOULDER     Heat · 2 feeding
  //
  // Genau ihr Unterschied ist die Frage aus GAME-DESIGN 16 — „ob die
  // Hitze-Mechanik mit nur einem Zauber trägt". Eine Anzeige, die beide gleich
  // darstellt, ist an dieser Stelle nicht ungenau, sondern blind.
  //
  // **Entdoppelt wird über die KENNUNG, nicht über die Mechanik.** Wer die
  // Mechanik nimmt, macht aus KINDLE + SMOULDER eine Eins — dann ist die
  // Aussage wieder falsch, nur andersherum. Ein Prüffall misst beide
  // Richtungen.
  const belegt = new Set<SpellIndex>();
  for (let slot = 0; slot < source.slotCount; slot++) {
    const spell = source.spellOnSlot(slot);
    if (spell >= 0) belegt.add(spell);
  }
  return MECHANICS.map((mechanic) => {
    let contributors = 0;
    for (const spell of belegt) {
      if (source.feedsMechanic(spell, mechanic.id)) contributors++;
    }
    return { id: mechanic.id, contributors };
  });
}

/**
 * Ein Fingerabdruck der Zusammenstellung — **ohne Speicheranforderung**.
 *
 * Die Auswahl selbst erzeugt eine Liste und darf deshalb nicht je Bild laufen
 * (GAME-DESIGN 18.2). Diese Zahl ändert sich genau dann, wenn sich die Auswahl
 * ändern könnte; wer sie mitführt, rechnet die Liste nur bei Bedarf neu.
 */
export function loadoutFingerprint(source: LoadoutSource): number {
  let fingerprint = 0;
  for (let slot = 0; slot < source.slotCount; slot++) {
    const spell = source.spellOnSlot(slot);
    if (spell < 0) continue;
    // **Jede Mechanik geht ein, nicht nur die der Schule.** Sonst bliebe der
    // Abdruck gleich, wenn jemand einen FIXATION-Zauber austauscht — und die
    // Auswahl liefe mit einer veralteten Liste weiter.
    if (MECHANICS.some((mechanic) => source.feedsMechanic(spell, mechanic.id))) {
      // Der Platz geht mit ein, nicht nur die Anzahl: Ein Umbelegen, das die
      // Zahl gleich lässt, soll trotzdem auffallen.
      fingerprint = fingerprint * 31 + slot + 1;
    }
  }
  return fingerprint;
}

/** Woraus die Auswahl liest. Nur Abfragen — sie ändert nichts. */
export interface LoadoutSource {
  readonly slotCount: number;
  spellOnSlot(slot: number): SpellIndex;
  /**
   * Ob der Zauber auf diesem Platz die Mechanik speist.
   *
   * Ersetzt seit #270 die zwei Abfragen `gainOfSpell`/`gainOfTick`: Die
   * beantworteten die Frage nur für einen Vorrat, und mit `control` und
   * `decay` kamen zwei Mechaniken dazu, die keinen führen. Die Entscheidung
   * steht jetzt in `feedsMechanic`, an einer Stelle.
   */
  feedsMechanic(spellIndex: SpellIndex, mechanicId: string): boolean;
}

/**
 * Wie viele Anzeigen weggelassen wurden.
 *
 * **Stillschweigend abzuschneiden wäre der Fehler.** Wer eine Mechanik
 * mitführt, deren Anzeige nicht erscheint, soll das nachlesen können — im
 * Werkzeug, nicht im Bild.
 */
export function hiddenIndicatorCount(mechanics: readonly MechanicInfo[]): number {
  const genutzt = mechanics.filter((mechanic) => mechanic.contributors > 0).length;
  return genutzt > MAX_INDICATORS ? genutzt - MAX_INDICATORS : 0;
}
