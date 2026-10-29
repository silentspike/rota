/**
 * Die Felder, die `sim/build.rs` ZWINGEND liest — als Tafel für die
 * TypeScript-Seite (Issue #1055).
 *
 * # Warum es diese Tafel gibt — gemessen am 2026-10-29
 *
 * Fehlt einem Datensatz ein solches Feld, bricht der Rust-Bau mit
 * `Feld "…" fehlt` ab. Das Spiel las denselben Datensatz weiter, mit einer
 * stillen Null an der Stelle: `dot.tickIntervalMs` gestrichen ergab einen
 * Brand mit Tick-Abstand 0, und `drawDotBar` in `src/render/spellbar.ts`
 * zeichnete `durationMs / 0` Tick-Marken — endlos. Zwei Prüfdateien, die
 * das echte Spiel zeichnen, hingen damit über die Stufengrenze von 1800 s
 * des Streich-Durchgangs hinaus.
 *
 * Von den 25 Feldern, deren Streichung den Rust-Bau bricht, wies die
 * TypeScript-Seite genau eines ab (`channelTickMs`, #1055). 11 lud sie ganz
 * ohne Meldung, die übrigen mit einer Meldung, die das Feld meist nicht
 * nannte.
 *
 * # Die Regel
 *
 * Was `build.rs` zwingend liest, prüft das Spiel beim Lesen auch: Fehlt es
 * oder hat es die falsche Art, steht ein Problem im Wortlaut von `build.rs`
 * da (`Zauber "x"/dot: Feld "tickIntervalMs" fehlt`), und der Datensatz wird
 * VERWORFEN — nicht mit einer Vorgabe weitergeführt. Das Spiel startet
 * trotzdem (`App.start`: „Nicht abbrechen … aber sichtbar sein muss es").
 *
 * # Die Liste ist abgeleitet, nicht gepflegt
 *
 * `tests/unit/pflichtfelder-aus-build-rs.test.ts` liest `sim/build.rs`, sucht
 * jede zwingende Lesestelle (`uint`, `boolean`, `text`, `resource_amount` und
 * `.get(…)` mit `panic!` oder `expect`) und verlangt, dass jede hier unter
 * `rust` steht — so oft, wie sie in `build.rs` vorkommt. Bekommt `build.rs`
 * ein neues Pflichtfeld, wird der Prüffall rot, bevor die zwei Seiten
 * auseinanderlaufen. Die Pfade selbst stehen hier, weil eine Lesestelle in
 * `build.rs` nur den Feldnamen kennt, nicht das Objekt, an dem er hängt.
 */

/** Wie `build.rs` den Wert liest — daraus folgt, was „falsche Art" heißt. */
export type Pflichtart =
  /** `uint()`: ganze Zahl >= 0. */
  | 'ganzzahl'
  /** `.and_then(Value::as_f64)`: irgendeine endliche Zahl. */
  | 'zahl'
  /** `boolean()`. */
  | 'wahrheitswert'
  /** `text()` und `.and_then(Value::as_str)`. */
  | 'text'
  /** `.get(…)` mit `panic!`/`expect`: es muss stehen, ein Objekt sein. */
  | 'objekt'
  /** `.and_then(Value::as_array)` mit `panic!`. */
  | 'liste'
  /**
   * `resource_amount()`: darf fehlen (dann 0), steht es, ist es ein Objekt mit
   * genau einem Eintrag, dessen Wert eine ganze Zahl >= 0 ist.
   */
  | 'betrag';

export interface Pflichtfeld {
  /**
   * Der Pfad im Datensatz, Teile mit Punkt getrennt; `[]` hinter einem Teil
   * heißt „jeder Eintrag dieser Liste". Fehlt ein Teil VOR dem letzten, gilt
   * das Feld nicht: `dot.tickIntervalMs` verlangt nichts von einem Zauber
   * ohne Brand — genau wie `emit_dot`, das ohne `dot` gar nicht erst liest.
   */
  readonly pfad: string;
  readonly art: Pflichtart;
  /**
   * Die Lesestellen in `build.rs` als `Funktion:Feld`, so oft, wie sie dort
   * stehen. `Funktion:$name` steht für eine Stelle, deren Feldname eine
   * Variable ist (`emit_fixation` liest `amountMs`, `amountPercent` oder
   * `amountPerPoint` je nach Wirkung).
   */
  readonly rust: readonly string[];
  /**
   * `build.rs` liest das Feld nur, wenn es steht (`decayPerTick`,
   * `resource_amount`). Fehlen ist dann erlaubt; die Art wird nur geprüft,
   * wenn es da ist.
   */
  readonly bewacht?: true;
  /** Gilt nur, wenn das Geschwisterfeld `feld` einen der Werte trägt. */
  readonly wenn?: { readonly feld: string; readonly werte: readonly string[] };
}

/** Ein Zauber, aus jeder Zauberdatei (`emit_spell` und die `emit_*` darunter). */
export const ZAUBER_PFLICHT: readonly Pflichtfeld[] = [
  { pfad: 'id', art: 'text', rust: ['main:id', 'emit_spells:id', 'emit_spell:id'] },
  { pfad: 'name', art: 'text', rust: ['emit_spell:name'] },
  { pfad: 'unlockLevel', art: 'ganzzahl', rust: ['emit_spell:unlockLevel'] },
  { pfad: 'baseDamage', art: 'ganzzahl', rust: ['emit_spell:baseDamage'] },
  { pfad: 'cooldownMs', art: 'ganzzahl', rust: ['emit_spell:cooldownMs'] },
  { pfad: 'castTimeMs', art: 'ganzzahl', rust: ['emit_spell:castTimeMs'] },
  {
    pfad: 'triggersGcd',
    art: 'wahrheitswert',
    rust: ['emit_spell:triggersGcd', 'emit_spell:triggersGcd'],
  },
  { pfad: 'resourceGain', art: 'betrag', bewacht: true, rust: ['emit_spell:resourceGain'] },
  { pfad: 'buff.kind', art: 'ganzzahl', rust: ['emit_buff:kind'] },
  { pfad: 'buff.factor', art: 'zahl', rust: ['emit_buff:factor'] },
  { pfad: 'buff.gcds', art: 'ganzzahl', rust: ['emit_buff:gcds'] },
  { pfad: 'dot.refreshMode', art: 'text', rust: ['emit_dot:refreshMode'] },
  { pfad: 'dot.tickDamage', art: 'ganzzahl', rust: ['emit_dot:tickDamage'] },
  { pfad: 'dot.tickIntervalMs', art: 'ganzzahl', rust: ['emit_dot:tickIntervalMs'] },
  { pfad: 'dot.durationMs', art: 'ganzzahl', rust: ['emit_dot:durationMs'] },
  {
    pfad: 'dot.resourceGainPerTick',
    art: 'betrag',
    bewacht: true,
    rust: ['emit_dot:resourceGainPerTick'],
  },
  { pfad: 'dot.decayPerTick', art: 'ganzzahl', bewacht: true, rust: ['emit_dot:decayPerTick'] },
  { pfad: 'spender.resource', art: 'text', rust: ['emit_spender:resource'] },
  { pfad: 'spender.consumesAll', art: 'wahrheitswert', rust: ['emit_spender:consumesAll'] },
  { pfad: 'spender.damagePerPoint', art: 'ganzzahl', rust: ['emit_spender:damagePerPoint'] },
  { pfad: 'spender.bonusThreshold', art: 'ganzzahl', rust: ['emit_spender:bonusThreshold'] },
  { pfad: 'spender.bonusMultiplier', art: 'ganzzahl', rust: ['emit_spender:bonusMultiplier'] },
  { pfad: 'shield.amount', art: 'ganzzahl', rust: ['emit_coagulation:amount'] },
  { pfad: 'shield.durationMs', art: 'ganzzahl', rust: ['emit_coagulation:durationMs'] },
  { pfad: 'shield.maxStacks', art: 'ganzzahl', rust: ['emit_coagulation:maxStacks'] },
  { pfad: 'heal.amount', art: 'ganzzahl', rust: ['emit_coagulation:amount'] },
  {
    pfad: 'decayWard.permillePerStack',
    art: 'ganzzahl',
    rust: ['emit_coagulation:permillePerStack'],
  },
  { pfad: 'decayWard.durationMs', art: 'ganzzahl', rust: ['emit_coagulation:durationMs'] },
  { pfad: 'decayCap.bonus', art: 'ganzzahl', rust: ['emit_coagulation:bonus'] },
  { pfad: 'decayCap.durationMs', art: 'ganzzahl', rust: ['emit_coagulation:durationMs'] },
  { pfad: 'conversion.sharePercent', art: 'ganzzahl', rust: ['emit_coagulation:sharePercent'] },
  { pfad: 'conversion.durationMs', art: 'ganzzahl', rust: ['emit_coagulation:durationMs'] },
  { pfad: 'reflect.sharePercent', art: 'ganzzahl', rust: ['emit_coagulation:sharePercent'] },
  { pfad: 'reflect.durationMs', art: 'ganzzahl', rust: ['emit_coagulation:durationMs'] },
  { pfad: 'convert.from', art: 'text', rust: ['emit_convert:from'] },
  { pfad: 'convert.perSecond', art: 'ganzzahl', rust: ['emit_convert:perSecond'] },
  { pfad: 'control.cost', art: 'ganzzahl', rust: ['emit_fixation:cost'] },
  { pfad: 'control.effect', art: 'text', rust: ['emit_fixation:effect'] },
  // Eine Lesestelle in `build.rs` (`uint(node, feld, …)`), drei Felder je
  // nach Wirkung — die Zuordnung steht dort in einem `match` über `effect`.
  {
    pfad: 'control.amountMs',
    art: 'ganzzahl',
    rust: ['emit_fixation:$feld'],
    wenn: { feld: 'effect', werte: ['slowCast', 'delayStrike', 'allThree', 'makeInterruptible'] },
  },
  {
    pfad: 'control.amountPercent',
    art: 'ganzzahl',
    rust: ['emit_fixation:$feld'],
    wenn: { feld: 'effect', werte: ['holdPhase'] },
  },
  {
    pfad: 'control.amountPerPoint',
    art: 'ganzzahl',
    rust: ['emit_fixation:$feld'],
    wenn: { feld: 'effect', werte: ['convertToDamage'] },
  },
];

/** Ein Boss, aus jeder Datei unter `data/bosses/` (`emit_boss` und darunter). */
export const BOSS_PFLICHT: readonly Pflichtfeld[] = [
  { pfad: 'id', art: 'text', rust: ['emit_boss:id'] },
  { pfad: 'name', art: 'text', rust: ['emit_boss:name'] },
  { pfad: 'instance', art: 'text', rust: ['emit_boss:instance', 'pruefe_instanz:instance'] },
  { pfad: 'hp', art: 'ganzzahl', rust: ['emit_boss:hp'] },
  { pfad: 'overpressureMs', art: 'ganzzahl', rust: ['emit_boss:overpressureMs'] },
  { pfad: 'timeline[].atMs', art: 'ganzzahl', rust: ['emit_timeline:atMs'] },
  { pfad: 'timeline[].mechanic', art: 'text', rust: ['emit_timeline:mechanic'] },
  { pfad: 'timeline[].name', art: 'text', rust: ['emit_timeline:name'] },
  { pfad: 'phases[].atHpPercent', art: 'ganzzahl', rust: ['emit_phases:atHpPercent'] },
  { pfad: 'phases[].name', art: 'text', rust: ['emit_phases:name'] },
  { pfad: 'energy.perStepMs', art: 'ganzzahl', rust: ['emit_energy:perStepMs'] },
  { pfad: 'energy.threshold', art: 'ganzzahl', rust: ['emit_energy:threshold'] },
  { pfad: 'energy.resetTo', art: 'ganzzahl', rust: ['emit_energy:resetTo'] },
  { pfad: 'energy.amount', art: 'ganzzahl', rust: ['emit_energy:amount'] },
  { pfad: 'energy.onTrigger', art: 'objekt', rust: ['emit_energy:onTrigger'] },
  { pfad: 'energy.onTrigger.mechanic', art: 'text', rust: ['emit_energy:mechanic'] },
  { pfad: 'energy.onTrigger.damage', art: 'ganzzahl', rust: ['emit_energy:damage'] },
  { pfad: 'doom.perStepMs', art: 'ganzzahl', rust: ['emit_doom:perStepMs'] },
  { pfad: 'doom.threshold', art: 'ganzzahl', rust: ['emit_doom:threshold'] },
  { pfad: 'doom.amount', art: 'ganzzahl', rust: ['emit_doom:amount'] },
  { pfad: 'doom.clearAmount', art: 'ganzzahl', rust: ['emit_doom:clearAmount'] },
  { pfad: 'doom.clearCharges', art: 'ganzzahl', rust: ['emit_doom:clearCharges'] },
  { pfad: 'stacks.perStepMs', art: 'ganzzahl', rust: ['emit_stacks:perStepMs'] },
  { pfad: 'stacks.baseDamage', art: 'ganzzahl', rust: ['emit_stacks:baseDamage'] },
  { pfad: 'stacks.factor', art: 'ganzzahl', rust: ['emit_stacks:factor'] },
  { pfad: 'stacks.maxStacks', art: 'ganzzahl', rust: ['emit_stacks:maxStacks'] },
  { pfad: 'race.atMs', art: 'ganzzahl', rust: ['emit_race:atMs'] },
  { pfad: 'race.windowMs', art: 'ganzzahl', rust: ['emit_race:windowMs'] },
  { pfad: 'race.threshold', art: 'ganzzahl', rust: ['emit_race:threshold'] },
  { pfad: 'bossHeal.amount', art: 'ganzzahl', rust: ['emit_boss_heal:amount'] },
  { pfad: 'runaway.perStepMs', art: 'ganzzahl', rust: ['emit_runaway:perStepMs'] },
  { pfad: 'runaway.amount', art: 'ganzzahl', rust: ['emit_runaway:amount'] },
  {
    pfad: 'escalation.castTimeShrinkMs',
    art: 'ganzzahl',
    rust: ['emit_escalation:castTimeShrinkMs'],
  },
  { pfad: 'escalation.minCastTimeMs', art: 'ganzzahl', rust: ['emit_escalation:minCastTimeMs'] },
  { pfad: 'affliction.atMs', art: 'ganzzahl', rust: ['emit_affliction:atMs'] },
  { pfad: 'affliction.perStepMs', art: 'ganzzahl', rust: ['emit_affliction:perStepMs'] },
  { pfad: 'affliction.damage', art: 'ganzzahl', rust: ['emit_affliction:damage'] },
];

/**
 * Die Schuldatei selbst: Name, Mechanik und die Liste (`emit_school`,
 * `emit_resource`, `zauber_liste`). Dazu `schema`, das `read_json_at` von
 * JEDER Datei verlangt.
 */
export const SCHUL_PFLICHT: readonly Pflichtfeld[] = [
  { pfad: 'schema', art: 'ganzzahl', rust: ['read_json_at:schema'] },
  { pfad: 'school', art: 'text', rust: ['emit_school:school'] },
  { pfad: 'spells', art: 'liste', rust: ['zauber_liste:spells', 'zauber_liste:spells'] },
  { pfad: 'resource', art: 'objekt', rust: ['emit_resource:resource', 'emit_spells:resource'] },
  { pfad: 'resource.id', art: 'text', rust: ['emit_resource:id', 'emit_spells:id'] },
  { pfad: 'resource.name', art: 'text', rust: ['emit_resource:name'] },
  { pfad: 'resource.cap', art: 'ganzzahl', rust: ['emit_resource:cap'] },
  { pfad: 'resource.decayPerStepMs', art: 'ganzzahl', rust: ['emit_resource:decayPerStepMs'] },
  { pfad: 'resource.decayAmount', art: 'ganzzahl', rust: ['emit_resource:decayAmount'] },
  { pfad: 'resource.highlightAt', art: 'ganzzahl', rust: ['emit_resource:highlightAt'] },
];

/** `data/player/base.json` (`emit_player`). */
export const SPIELER_PFLICHT: readonly Pflichtfeld[] = [
  { pfad: 'vessel', art: 'ganzzahl', rust: ['emit_player:vessel'] },
  { pfad: 'wardReduction', art: 'zahl', rust: ['emit_player:wardReduction'] },
  { pfad: 'wardCapPermille', art: 'ganzzahl', rust: ['emit_player:wardCapPermille'] },
];

/** `data/player/levels.json` (`emit_levels`). */
export const STUFEN_PFLICHT: readonly Pflichtfeld[] = [
  { pfad: 'maxLevel', art: 'ganzzahl', rust: ['emit_levels:maxLevel'] },
  { pfad: 'factorPerMille', art: 'liste', rust: ['emit_levels:factorPerMille'] },
];

/** Alle Tafeln, für den Abgleich gegen `build.rs`. */
export const ALLE_PFLICHTFELDER: readonly (readonly Pflichtfeld[])[] = [
  ZAUBER_PFLICHT,
  BOSS_PFLICHT,
  SCHUL_PFLICHT,
  SPIELER_PFLICHT,
  STUFEN_PFLICHT,
];

function istObjekt(wert: unknown): wert is Record<string, unknown> {
  return typeof wert === 'object' && wert !== null && !Array.isArray(wert);
}

/** Was an der Art falsch ist, oder `null`. Wortlaut wie in `build.rs`. */
function artFehler(art: Pflichtart, feld: string, wert: unknown): string | null {
  switch (art) {
    case 'ganzzahl':
      return typeof wert === 'number' && Number.isSafeInteger(wert) && wert >= 0
        ? null
        : `"${feld}" muss eine ganze Zahl >= 0 sein, ist aber ${JSON.stringify(wert)}`;
    case 'zahl':
      return typeof wert === 'number' && Number.isFinite(wert)
        ? null
        : `Feld "${feld}" fehlt oder ist keine Zahl`;
    case 'wahrheitswert':
      return typeof wert === 'boolean' ? null : `Feld "${feld}" fehlt oder ist kein Wahrheitswert`;
    case 'text':
      return typeof wert === 'string' ? null : `Feld "${feld}" fehlt oder ist keine Zeichenkette`;
    case 'objekt':
      return istObjekt(wert) ? null : `"${feld}" muss ein Objekt sein`;
    case 'liste':
      return Array.isArray(wert) ? null : `"${feld}" muss eine Liste sein`;
    case 'betrag': {
      if (!istObjekt(wert)) return `"${feld}" muss ein Objekt sein`;
      const eintraege = Object.entries(wert);
      if (eintraege.length !== 1) {
        return `"${feld}" nennt ${eintraege.length} Ressourcen; Schritt 1 kennt genau eine`;
      }
      const [schluessel, betrag] = eintraege[0] as [string, unknown];
      return typeof betrag === 'number' && Number.isSafeInteger(betrag) && betrag >= 0
        ? null
        : `"${feld}.${schluessel}" muss eine ganze Zahl >= 0 sein`;
    }
  }
}

/**
 * Die Probleme eines Datensatzes gegen eine Tafel — leer heißt: Alles, was
 * `build.rs` zwingend liest, steht da und hat die richtige Art.
 *
 * `wer` beginnt jede Meldung (`Zauber "smoulder"`), der Pfad bis zum
 * Elternobjekt folgt wie bei `build.rs` mit Schrägstrichen
 * (`Zauber "smoulder"/dot: Feld "tickIntervalMs" fehlt`).
 */
export function pflichtProbleme(
  datensatz: unknown,
  tafel: readonly Pflichtfeld[],
  wer: string,
): string[] {
  const probleme: string[] = [];
  for (const eintrag of tafel) pruefe(datensatz, eintrag.pfad.split('.'), eintrag, wer, probleme);
  return probleme;
}

function pruefe(
  knoten: unknown,
  teile: readonly string[],
  eintrag: Pflichtfeld,
  wo: string,
  probleme: string[],
): void {
  if (!istObjekt(knoten)) return;
  const [kopf, ...rest] = teile;
  if (kopf === undefined) return;
  const liste = kopf.endsWith('[]');
  const name = liste ? kopf.slice(0, -2) : kopf;
  if (rest.length === 0) {
    if (eintrag.wenn !== undefined) {
      const bedingung = knoten[eintrag.wenn.feld];
      if (typeof bedingung !== 'string' || !eintrag.wenn.werte.includes(bedingung)) return;
    }
    if (!(name in knoten)) {
      if (eintrag.bewacht !== true) probleme.push(`${wo}: Feld "${name}" fehlt`);
      return;
    }
    const fehler = artFehler(eintrag.art, name, knoten[name]);
    if (fehler !== null) probleme.push(`${wo}: ${fehler}`);
    return;
  }
  // Ein fehlendes oder leeres Elternobjekt verlangt nichts — `emit_dot` liest
  // ohne `dot` gar nicht erst, `emit_energy` behandelt `null` wie „fehlt".
  const kind = knoten[name];
  if (kind === undefined || kind === null) return;
  if (liste) {
    if (!Array.isArray(kind)) return;
    kind.forEach((element, index) =>
      pruefe(element, rest, eintrag, `${wo}/${name}[${index}]`, probleme),
    );
    return;
  }
  pruefe(kind, rest, eintrag, `${wo}/${name}`, probleme);
}
