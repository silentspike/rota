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
import { TICK_MS } from './loop.js';

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
  /**
   * Setzt dieser Zauber den **Auswahlmodus** scharf (SUBLIMATION, Issue E3-3c)?
   *
   * Der Rücksetzer wirkt nicht auf den nächsten gewirkten Zauber, sondern
   * fragt: Welchen? Zwei Schritte, drei Aktionen — die Entscheidung des
   * Auftraggebers vom 2026-07-29.
   */
  readonly armsCooldownReset?: boolean;
}

export interface SchoolDef {
  readonly schema: number;
  readonly school: string;
  readonly resource: ResourceDef;
  readonly spells: readonly SpellDef[];
}

/**
 * Eine Phase: ab welchem Anteil der Lebenspunkte der Boss sein Verhalten
 * wechselt (GAME-DESIGN 11.1).
 *
 * **Eskalierend, nicht tauschend:** Jede Phase legt eine Mechanik obendrauf,
 * statt die vorige zu ersetzen. Die Mechaniken selbst kommen mit E4-2; hier
 * steht nur, WANN gewechselt wird.
 */
export interface PhaseDef {
  /** Anteil der verbleibenden Lebenspunkte in Prozent, ab dem sie gilt. */
  readonly atHpPercent: number;
  readonly name: string;
}

/**
 * Die Energieleiste — die zweite Uhr eines Bosses (GAME-DESIGN 11.2,
 * Baustein 1).
 *
 * > Boss-Energie steigt **unabhängig von den Lebenspunkten** und löst bei 100
 * > eine Mechanik aus.
 *
 * Das „unabhängig" ist die eigentliche Aussage: Wer schneller Schaden macht,
 * verschiebt den Auslöser **nicht**.
 */
export interface EnergyDef {
  readonly perStepMs: number;
  readonly amount: number;
  readonly threshold: number;
  readonly resetTo: number;
}

/**
 * Ein Eintrag der Boss-Zeitleiste (GAME-DESIGN 11.1: „Schadens-Ereignisse als
 * **feste Zeitleiste**").
 *
 * Fest heißt wirklich fest: Der Boss reagiert nicht auf den Spieler, und es
 * gibt keine zufällige Streuung. Nur so sind Bestzeiten vergleichbar.
 *
 * **Was hier absichtlich fehlt, ist die Wirkung.** Welcher Baustein was tut,
 * ist Gegenstand von E4-2 bis E4-6; welcher Boss welche Einträge bekommt,
 * gehört nach E10 (GAME-DESIGN 22). Hier steht das Gerüst, das beides trägt.
 */
export interface TimelineEntryDef {
  /** Zeitpunkt seit Kampfbeginn, auf dem Takt-Raster des Spiels. */
  readonly atMs: number;
  /** Kennung des Mechanik-Bausteins, der hier greift. */
  readonly mechanic: string;
}

export interface BossDef {
  readonly schema: number;
  readonly id: string;
  readonly name: string;
  readonly instance: string;
  readonly hp: number;
  readonly overpressureMs: number;
  readonly resistances: Readonly<Record<string, number>>;
  /** Phasen-Schwellen, absteigend. Leer heißt: keine Phasen. */
  readonly phases: readonly PhaseDef[];
  /** Die zweite Uhr, oder `null`. */
  readonly energy: EnergyDef | null;
  /** Die feste Abfolge der Boss-Handlungen, aufsteigend nach Zeitpunkt. */
  readonly timeline: readonly TimelineEntryDef[];
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
    ...(raw['armsCooldownReset'] === true ? { armsCooldownReset: true } : {}),
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

  // Phasen müssen absteigend stehen. Eine unsortierte Liste wäre eine stille
  // Falle: Der Kampf würde Phasen überspringen, ohne dass irgendwo etwas
  // bricht. `sim/build.rs` bricht dafür den Bau ab; hier wird es gemeldet,
  // weil die Spiel-Seite zur Laufzeit liest.
  const phases: PhaseDef[] = [];
  if (Array.isArray(source['phases'])) {
    let vorige = 101;
    for (const entry of source['phases']) {
      if (!isRecord(entry)) continue;
      const atHpPercent = number(entry['atHpPercent'], 0);
      if (atHpPercent <= 0 || atHpPercent >= 100) {
        problems.push(`Phasen-Schwelle bei ${atHpPercent} % ergibt keine Phase.`);
        continue;
      }
      if (atHpPercent >= vorige) {
        problems.push(
          `Phasen-Schwellen müssen absteigend stehen: ${atHpPercent} % nach ${vorige} %.`,
        );
        continue;
      }
      vorige = atHpPercent;
      phases.push({
        atHpPercent,
        name: typeof entry['name'] === 'string' ? entry['name'] : 'Unnamed',
      });
    }
  }

  let energy: EnergyDef | null = null;
  if (isRecord(source['energy'])) {
    const perStepMs = number(source['energy']['perStepMs'], 0);
    const threshold = number(source['energy']['threshold'], 0);
    const resetTo = number(source['energy']['resetTo'], 0);
    if (perStepMs <= 0) problems.push('Die Energieleiste hat keinen Takt (perStepMs).');
    // Das Spiel kann nur zu Takt-Grenzen handeln, der Simulator springt frei.
    // Bei 333 ms stiege die Leiste dort bei 333/666/999, hier bei 340/670/1000
    // — der Abstand wüchse mit jedem Schritt. `sim/build.rs` bricht dafür den
    // Bau ab; hier wird es gemeldet, weil diese Seite zur Laufzeit liest.
    if (perStepMs > 0 && perStepMs % TICK_MS !== 0) {
      problems.push(
        `perStepMs ${perStepMs} liegt nicht auf dem ${TICK_MS}-ms-Raster — ` +
          'Spiel und Simulator liefen mit jedem Anstieg weiter auseinander.',
      );
    }
    if (threshold <= 0) problems.push('Die Energieleiste hat keine Schwelle (threshold).');
    if (resetTo >= threshold) {
      problems.push(
        `resetTo ${resetTo} liegt nicht unter threshold ${threshold} — die Leiste käme nie herunter.`,
      );
    }
    energy = {
      perStepMs,
      amount: number(source['energy']['amount'], 0),
      threshold,
      resetTo,
    };
  }

  // Die Zeitleiste: aufsteigend und auf dem Takt-Raster. Beide Verstöße wären
  // still — ein Eintrag hinter einem späteren würde nie ausgelöst, und einer
  // neben dem Raster griffe hier später als im Simulator.
  const timeline: TimelineEntryDef[] = [];
  if (Array.isArray(source['timeline'])) {
    let voriger = -1;
    for (const entry of source['timeline']) {
      if (!isRecord(entry)) continue;
      const atMs = number(entry['atMs'], -1);
      if (atMs < 0) {
        problems.push('Ein Zeitleisten-Eintrag hat keinen Zeitpunkt (atMs).');
        continue;
      }
      if (atMs <= voriger) {
        problems.push(`Die Zeitleiste muss aufsteigend stehen: ${atMs} ms nach ${voriger} ms.`);
        continue;
      }
      if (atMs % TICK_MS !== 0) {
        problems.push(
          `Zeitleisten-Eintrag bei ${atMs} ms liegt nicht auf dem ${TICK_MS}-ms-Raster.`,
        );
        continue;
      }
      voriger = atMs;
      timeline.push({
        atMs,
        mechanic: typeof entry['mechanic'] === 'string' ? entry['mechanic'] : 'unknown',
      });
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
      phases,
      energy,
      timeline,
    },
    problems,
  };
}

/** Die Schule aus Schritt 1, fertig gelesen. */
export const CALCINATION: ParseResult<SchoolDef> = parseSchool(rawSchool);

/** Der Testboss aus Schritt 1, fertig gelesen. */
export const FIRST_CRUCIBLE: ParseResult<BossDef> = parseBoss(rawBoss);

/**
 * **Alle** Bosse, nach Bezeichner — aus dem Verzeichnis gelesen, nicht einzeln
 * aufgezählt (Issue #96, AC1: „ein neuer Boss entsteht als Datendatei, ohne
 * dass eine Zeile Programmcode geändert wird").
 *
 * Der feste `import` oben reicht dafür nicht: Er nennt genau eine Datei, und
 * ein zweiter Boss bräuchte eine zweite Zeile. `import.meta.glob` löst das zur
 * Bauzeit auf — die JSON-Dateien landen also weiterhin gebündelt im Ergebnis,
 * es wird nichts zur Laufzeit nachgeladen.
 *
 * Der Simulator macht dasselbe an seiner Stelle: `sim/build.rs` liest das
 * Verzeichnis sortiert. **Beide Seiten müssen dieselbe Menge sehen**, sonst
 * kennt einer einen Boss, den der andere nicht rechnen kann.
 */
export const BOSSES: ReadonlyMap<string, ParseResult<BossDef>> = (() => {
  const dateien = import.meta.glob<Record<string, unknown>>('../../data/bosses/*.json', {
    eager: true,
    import: 'default',
  });
  // Sortiert, damit die Reihenfolge nicht vom Dateisystem abhängt — dieselbe
  // Entscheidung wie `dateien.sort()` in `sim/build.rs` (GAME-DESIGN 18.4:
  // feste Reihenfolge, keine Hash-Container).
  const pfade = Object.keys(dateien).sort();
  const out = new Map<string, ParseResult<BossDef>>();
  for (const pfad of pfade) {
    const gelesen = parseBoss(dateien[pfad]);
    out.set(gelesen.value.id, gelesen);
  }
  return out;
})();

/**
 * Ein Boss nach Bezeichner, oder `undefined`.
 *
 * Das Gegenstück zu `boss_by_id` im Simulator (`sim/src/data.rs`).
 */
export function bossById(id: string): BossDef | undefined {
  return BOSSES.get(id)?.value;
}
