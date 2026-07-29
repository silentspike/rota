/**
 * Zauber und Bosse als **Datensätze, nicht als Code** (GAME-DESIGN 7.3, 18.3,
 * Leitplanke 6).
 *
 * Das hier gelesene Format teilen sich das Spiel (TypeScript) und der Simulator
 * (Rust) — es ist damit eine Schnittstelle und keine Bequemlichkeit. Die Fassung
 * mit allen Zahlen steht in `docs/STEP1-SPEC.md` § 5.
 *
 * Fehlerhafte Einträge werden **gemeldet und übersprungen**, nicht verschluckt:
 * Ein Zauber, der still fehlt, fällt erst im Kampf auf.
 */
import rawSchool from '../../data/spells/calcination.json';
import rawBoss from '../../data/bosses/first-crucible.json';

/** Die Schul-Mechanik als Vorrat: Hitze, Kontroll-Punkte, Zersetzungs-Stapel. */
export interface ResourceDef {
  readonly id: string;
  readonly name: string;
  readonly cap: number;
  /** Abstand zwischen zwei Verfall-Schritten in ms. */
  readonly decayPerStepMs: number;
  readonly decayAmount: number;
  /** Wert, ab dem die Anzeige eine Markierung zeigt (GAME-DESIGN 9). */
  readonly highlightAt: number;
}

/** Ein Effekt über Zeit. `refreshMode` ist für V1 immer `reset` (GAME-DESIGN 7.2). */
export interface DotDef {
  readonly tickDamage: number;
  readonly tickIntervalMs: number;
  readonly durationMs: number;
  readonly refreshMode: 'reset';
  readonly resourceGainPerTick: Readonly<Record<string, number>>;
}

/** Ein Entlade-Zauber: verbraucht den Vorrat und schlägt entsprechend zu. */
export interface SpenderDef {
  readonly resource: string;
  readonly damagePerPoint: number;
  /** Ab diesem Vorrat greift der Bonus — **ab**, nicht darüber. */
  readonly bonusThreshold: number;
  readonly bonusMultiplier: number;
  readonly consumesAll: boolean;
}

export interface SpellDef {
  readonly id: string;
  readonly name: string;
  readonly unlockLevel: number;
  readonly baseDamage: number;
  readonly cooldownMs: number;
  readonly castTimeMs: number;
  readonly triggersGcd: boolean;
  readonly resourceGain?: Readonly<Record<string, number>>;
  readonly dot?: DotDef;
  readonly spender?: SpenderDef;
  /**
   * Um wie viele Millisekunden dieser Zauber **alle laufenden Abklingzeiten**
   * verkürzt (SUBLIMATION, GAME-DESIGN 4.1, Issue E3-3).
   *
   * Steht in den Daten und nicht im Code: Ein Zauber, der das kann, ist ein
   * Datensatz wie jeder andere (Leitplanke 6). Fehlt das Feld, verkürzt er
   * nichts — das ist der Fall für alle Zauber der Kalzination.
   */
  readonly cooldownReductionMs?: number;
}

export interface SchoolDef {
  readonly schema: number;
  readonly school: string;
  readonly resource: ResourceDef;
  readonly spells: readonly SpellDef[];
}

export interface BossDef {
  readonly schema: number;
  readonly id: string;
  readonly name: string;
  readonly instance: string;
  readonly hp: number;
  readonly overpressureMs: number;
  readonly resistances: Readonly<Record<string, number>>;
}

export interface ParseResult<T> {
  readonly value: T;
  /** Leer heißt: alles in Ordnung. */
  readonly problems: readonly string[];
}

const SUPPORTED_SCHEMA = 1;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function number(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function gains(value: unknown): Record<string, number> {
  const out: Record<string, number> = {};
  if (!isRecord(value)) return out;
  for (const [key, amount] of Object.entries(value)) {
    if (typeof amount === 'number' && Number.isFinite(amount)) out[key] = amount;
  }
  return out;
}

function parseResource(raw: unknown, problems: string[]): ResourceDef {
  const source = isRecord(raw) ? raw : {};
  if (!isRecord(raw)) problems.push('Die Schul-Mechanik fehlt.');
  const cap = number(source['cap'], 0);
  const highlightAt = number(source['highlightAt'], 0);
  if (cap <= 0) problems.push('Die Obergrenze der Schul-Mechanik ist nicht positiv.');
  if (highlightAt > cap) {
    problems.push(`Die Markierung (${highlightAt}) liegt über der Obergrenze (${cap}).`);
  }
  return {
    id: typeof source['id'] === 'string' ? source['id'] : 'resource',
    name: typeof source['name'] === 'string' ? source['name'] : 'Resource',
    cap,
    decayPerStepMs: number(source['decayPerStepMs'], 0),
    decayAmount: number(source['decayAmount'], 0),
    highlightAt,
  };
}

function parseDot(raw: unknown, where: string, problems: string[]): DotDef | undefined {
  if (!isRecord(raw)) return undefined;
  const mode = raw['refreshMode'];
  if (mode !== 'reset') {
    // GAME-DESIGN 7.2 und STEP1-SPEC § 5: "reset" ist für V1 der einzige zulässige
    // Wert. Ein anderer wäre eine stille Design-Änderung.
    problems.push(`${where}: refreshMode "${String(mode)}" ist in V1 nicht zulässig.`);
    return undefined;
  }
  const durationMs = number(raw['durationMs'], 0);
  const tickIntervalMs = number(raw['tickIntervalMs'], 0);
  if (tickIntervalMs <= 0) problems.push(`${where}: Tick-Abstand muss positiv sein.`);
  else if (durationMs % tickIntervalMs !== 0) {
    problems.push(`${where}: Dauer ${durationMs} ms ist kein Vielfaches von ${tickIntervalMs} ms.`);
  }
  return {
    tickDamage: number(raw['tickDamage'], 0),
    tickIntervalMs,
    durationMs,
    refreshMode: 'reset',
    resourceGainPerTick: gains(raw['resourceGainPerTick']),
  };
}

function parseSpender(raw: unknown, where: string, problems: string[]): SpenderDef | undefined {
  if (!isRecord(raw)) return undefined;
  const bonusThreshold = number(raw['bonusThreshold'], 0);
  const bonusMultiplier = number(raw['bonusMultiplier'], 1);
  if (bonusMultiplier < 1) problems.push(`${where}: Bonus-Faktor unter 1 ergibt keinen Sinn.`);
  return {
    resource: typeof raw['resource'] === 'string' ? raw['resource'] : 'resource',
    damagePerPoint: number(raw['damagePerPoint'], 0),
    bonusThreshold,
    bonusMultiplier,
    consumesAll: raw['consumesAll'] !== false,
  };
}

function parseSpell(raw: unknown, problems: string[]): SpellDef | null {
  if (!isRecord(raw)) {
    problems.push('Ein Zauber-Eintrag ist kein Objekt.');
    return null;
  }
  const id = raw['id'];
  if (typeof id !== 'string' || id.length === 0) {
    problems.push('Ein Zauber hat keine Kennung.');
    return null;
  }
  const where = `Zauber "${id}"`;
  const cooldownMs = number(raw['cooldownMs'], 0);
  if (cooldownMs < 0) problems.push(`${where}: Abklingzeit ist negativ.`);
  if (!Number.isInteger(cooldownMs)) {
    problems.push(`${where}: Abklingzeit ${cooldownMs} ist nicht ganzzahlig.`);
  }

  const dot = parseDot(raw['dot'], where, problems);
  const spender = parseSpender(raw['spender'], where, problems);
  const resourceGain = isRecord(raw['resourceGain']) ? gains(raw['resourceGain']) : undefined;

  const reduction = raw['cooldownReductionMs'];
  const cooldownReductionMs = typeof reduction === 'number' ? reduction : undefined;
  if (cooldownReductionMs !== undefined) {
    if (cooldownReductionMs < 0) problems.push(`${where}: Verkürzung ist negativ.`);
    if (!Number.isInteger(cooldownReductionMs)) {
      problems.push(`${where}: Verkürzung ${cooldownReductionMs} ist nicht ganzzahlig.`);
    }
  }

  return {
    id,
    name: typeof raw['name'] === 'string' ? raw['name'] : id,
    unlockLevel: number(raw['unlockLevel'], 1),
    baseDamage: number(raw['baseDamage'], 0),
    cooldownMs,
    castTimeMs: number(raw['castTimeMs'], 0),
    triggersGcd: raw['triggersGcd'] !== false,
    ...(resourceGain !== undefined ? { resourceGain } : {}),
    ...(dot !== undefined ? { dot } : {}),
    ...(spender !== undefined ? { spender } : {}),
    ...(cooldownReductionMs !== undefined ? { cooldownReductionMs } : {}),
  };
}

export function parseSchool(raw: unknown): ParseResult<SchoolDef> {
  const problems: string[] = [];
  const source = isRecord(raw) ? raw : {};
  const schema = number(source['schema'], 0);
  if (schema !== SUPPORTED_SCHEMA) {
    problems.push(`Unbekannte Fassung des Formats: ${schema} (erwartet ${SUPPORTED_SCHEMA}).`);
  }
  const spells: SpellDef[] = [];
  const seen = new Set<string>();
  const rawSpells = Array.isArray(source['spells']) ? source['spells'] : [];
  for (const entry of rawSpells) {
    const spell = parseSpell(entry, problems);
    if (spell === null) continue;
    if (seen.has(spell.id)) {
      problems.push(`Zauber "${spell.id}": Kennung kommt mehrfach vor.`);
      continue;
    }
    seen.add(spell.id);
    spells.push(spell);
  }
  if (spells.length === 0) problems.push('Die Schule enthält keinen einzigen Zauber.');

  return {
    value: {
      schema,
      school: typeof source['school'] === 'string' ? source['school'] : 'UNKNOWN',
      resource: parseResource(source['resource'], problems),
      spells,
    },
    problems,
  };
}

export function parseBoss(raw: unknown): ParseResult<BossDef> {
  const problems: string[] = [];
  const source = isRecord(raw) ? raw : {};
  const schema = number(source['schema'], 0);
  if (schema !== SUPPORTED_SCHEMA) {
    problems.push(`Unbekannte Fassung des Formats: ${schema} (erwartet ${SUPPORTED_SCHEMA}).`);
  }
  const hp = number(source['hp'], 0);
  if (hp <= 0) problems.push('Der Boss hat keine Lebenspunkte.');
  const overpressureMs = number(source['overpressureMs'], 0);
  if (overpressureMs <= 0) problems.push('Der Überdruck-Timer ist nicht gesetzt.');

  const resistances: Record<string, number> = {};
  if (isRecord(source['resistances'])) {
    for (const [school, factor] of Object.entries(source['resistances'])) {
      if (typeof factor === 'number' && Number.isFinite(factor)) resistances[school] = factor;
    }
  }

  return {
    value: {
      schema,
      id: typeof source['id'] === 'string' ? source['id'] : 'unknown',
      name: typeof source['name'] === 'string' ? source['name'] : 'Unknown',
      instance: typeof source['instance'] === 'string' ? source['instance'] : 'unknown',
      hp,
      overpressureMs,
      resistances,
    },
    problems,
  };
}

/** Die Schule aus Schritt 1, fertig gelesen. */
export const CALCINATION: ParseResult<SchoolDef> = parseSchool(rawSchool);

/** Der Testboss aus Schritt 1, fertig gelesen. */
export const FIRST_CRUCIBLE: ParseResult<BossDef> = parseBoss(rawBoss);
