/**
 * Die Effizienz-Tabelle (GAME-DESIGN 15.3, Issue #81).
 *
 * EFFIZIENZ FÜHRT, SCHADENSANTEIL FOLGT
 * -------------------------------------
 * Der Schadensanteil ist bei jedem Durchlauf nahezu gleich — er misst die
 * Zauber, nicht das Können. Er bleibt als **eine** Spalte, weil er für
 * Talent-Entscheidungen nützlich ist, aber er steht nicht vorn. Die Reihenfolge
 * der Spalten ist deshalb kein Gestaltungsdetail, sondern die Aussage der
 * Tabelle; `EFFICIENCY_COLUMNS` hält sie fest, damit sie prüfbar ist.
 *
 * WAS „GEGEN DAS MAXIMUM" HIER HEISST — UND WAS NICHT
 * --------------------------------------------------
 * NICHT „Kampfzeit ÷ Abklingzeit". Diese Rechnung ist verworfen (GAME-DESIGN
 * 15.2): Die Summe der Maxima übersteigt die Zahl der Aktionen, die es im Kampf
 * gibt — gemessen 193 gegen 160 —, weil die Zauber sich den GCD teilen.
 *
 * Stattdessen zählt je Zauber nur, was **tatsächlich frei war**: Fenster, in
 * denen dieser Zauber bereit lag und der Spieler nichts gewirkt hat. Diese
 * Fenster waren wirklich leer; niemandem wird eine Wirkung angerechnet, die ein
 * anderer Zauber besetzt hat. Wer statt A den Zauber B gewirkt hat, hat nichts
 * verloren — er hat gewählt, und Wählen ist das Spiel.
 *
 * WARUM DIE SPALTE NICHT AUFSUMMIERT WERDEN DARF
 * ----------------------------------------------
 * Ein leeres Fenster, in dem zwei Zauber bereit lagen, zählt bei **beiden**.
 * Das ist je Zeile richtig und in der Summe falsch. Der Gesamtverlust wird
 * deshalb aus der Zahl der leeren FENSTER gerechnet (loss-sources.ts), nie aus
 * der Summe dieser Zeilen. Ein Prüffall wacht darüber.
 */

/** Eine Zeile, wie sie hineingeht. */
export interface SpellEfficiencyInput {
  readonly spellId: string;
  readonly casts: number;
  /**
   * Millisekunden, in denen dieser Zauber bereit lag und nichts gewirkt wurde.
   *
   * Kommt aus `Session.idlePerSpellMs` — gezählt wird nur in Takten, in denen
   * **gar nichts** geschah.
   */
  readonly idleMs: number;
  readonly damage: number;
  /**
   * Ob dieser Zauber ein Handlungsfenster verbraucht.
   *
   * QUENCH tut es nicht (GAME-DESIGN 6.6). Wer ihn mitzaehlte, kaeme auf mehr
   * Wirkungen, als es Fenster gab — die Effizienz stuende ueber 100 %, und die
   * Spalte waere als Massstab wertlos.
   */
  readonly triggersGcd: boolean;
}

export interface EfficiencyInput {
  readonly elapsedMs: number;
  readonly gcdMs: number;
  /** Angenommene Wirkungen im ganzen Kampf, auch die ausserhalb des Grundtakts. */
  readonly casts: number;
  readonly damage: number;
  /** Leerlauf, in dem **kein** Zauber bereit war (RunSummary). */
  readonly lostTimeNothingReadyMs: number;
  readonly synergyUsed: number;
  readonly synergyAvailable: number;
  readonly spells: readonly SpellEfficiencyInput[];
}

export interface SpellEfficiencyRow {
  readonly spellId: string;
  readonly casts: number;
  /** Wirkungen plus die leeren Fenster, in denen er bereit lag. */
  readonly opportunities: number;
  /** `casts / opportunities`, 0 bis 1. `null`, wenn es keine Gelegenheit gab. */
  readonly efficiency: number | null;
  readonly idleMs: number;
  readonly damage: number;
  readonly damageShare: number;
}

export interface EfficiencyTable {
  /** Die Spaltenköpfe in ihrer Reihenfolge — englisch (Design-Gesetz 5). */
  readonly columns: readonly string[];
  readonly rows: readonly SpellEfficiencyRow[];
  /** Wirkungen gegen die Zahl der Fenster, die es im Kampf gab. */
  readonly castsMax: number;
  readonly overallEfficiency: number | null;
  readonly idleMs: number;
  readonly idleShare: number;
  /** `null`, solange es keine möglichen Synergien gibt (Schritt 1). */
  readonly synergyRate: number | null;
}

/**
 * Die Spalten in der Reihenfolge, in der sie stehen müssen.
 *
 * EFFICIENCY zuerst, DAMAGE SHARE zuletzt (GAME-DESIGN 15.3, AC4). Als Liste
 * und nicht als Reihenfolge im Auszeichnungscode, damit die Anordnung ohne
 * Browser prüfbar ist und nicht beim nächsten Umbau still kippt.
 */
export const EFFICIENCY_COLUMNS = [
  'EFFICIENCY',
  'CASTS',
  'IDLE',
  'SYNERGY',
  'DAMAGE SHARE',
] as const;

/**
 * Wie viele Handlungsfenster der Kampf überhaupt hatte.
 *
 * Das ist die einzige Obergrenze, die es ohne Referenz-Rotation gibt: Der GCD
 * begrenzt jede Rotation gleichermaßen, unabhängig davon, welche Zauber
 * mitgeführt werden.
 *
 * Das **plus eins** ist kein Aufschlag, sondern der erste Zug: Gehandelt wird
 * bei 0, dann bei 1×GCD, 2×GCD … Ein Kampf von 30 s bei 1,5 s Grundtakt hat
 * damit 21 Fenster, nicht 20 — und ein fehlerfrei gespielter Durchlauf kommt
 * auf 100 %, statt auf 105 % zu laufen.
 */
export function availableWindows(elapsedMs: number, gcdMs: number): number {
  if (gcdMs <= 0 || elapsedMs < 0) return 0;
  return Math.floor(elapsedMs / gcdMs) + 1;
}

export function buildEfficiencyTable(input: EfficiencyInput): EfficiencyTable {
  const { elapsedMs, gcdMs, damage } = input;
  const castsMax = availableWindows(elapsedMs, gcdMs);

  // Nur Zauber, die ein Fenster verbrauchen, werden gegen die Fenster gerechnet.
  const gcdCasts = input.spells.reduce(
    (summe, spell) => summe + (spell.triggersGcd ? spell.casts : 0),
    0,
  );

  const rows: SpellEfficiencyRow[] = input.spells.map((spell): SpellEfficiencyRow => {
    // Ein Fenster, kein Takt: Der Leerlauf wird in Takten gezählt, gehandelt
    // wird in GCD-Fenstern. Ohne die Umrechnung stünde eine Zahl da, die
    // niemand mit „Wirkungen" vergleichen kann.
    const idleWindows = gcdMs > 0 ? Math.round(spell.idleMs / gcdMs) : 0;
    const opportunities = spell.casts + idleWindows;
    return {
      spellId: spell.spellId,
      casts: spell.casts,
      opportunities,
      // Kein Nenner, keine Quote. Eine 0 % hieße „alles verpasst" und wäre das
      // Gegenteil der Wahrheit.
      efficiency: opportunities > 0 ? spell.casts / opportunities : null,
      idleMs: spell.idleMs,
      damage: spell.damage,
      damageShare: damage > 0 ? spell.damage / damage : 0,
    };
  });

  // Nach Effizienz aufsteigend: Was am meisten liegen blieb, steht oben. Bei
  // Gleichstand nach Kennung, damit die Reihenfolge zwischen zwei Läufen nicht
  // springt.
  rows.sort(
    (a, b) => (a.efficiency ?? 1) - (b.efficiency ?? 1) || a.spellId.localeCompare(b.spellId),
  );

  return {
    columns: [...EFFICIENCY_COLUMNS],
    rows,
    castsMax,
    overallEfficiency: castsMax > 0 ? gcdCasts / castsMax : null,
    idleMs: input.lostTimeNothingReadyMs,
    idleShare: elapsedMs > 0 ? input.lostTimeNothingReadyMs / elapsedMs : 0,
    // In Schritt 1 gibt es nur eine Schule und damit keine schulübergreifenden
    // Fenster. Eine Quote zu zeigen hieße, über etwas zu urteilen, das es nicht
    // gibt (GAME-DESIGN 5).
    synergyRate: input.synergyAvailable > 0 ? input.synergyUsed / input.synergyAvailable : null,
  };
}
