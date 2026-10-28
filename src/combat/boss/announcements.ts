/**
 * Die Boss-Ankündigungen — eingebaut statt per Zusatzprogramm (GAME-DESIGN 11.5,
 * Issue #797).
 *
 * > Der Balken zeigt **nicht ein Warnfenster, sondern die volle Abklingzeit** der
 * > Boss-Fähigkeit. […] **Das macht aus Reagieren Planen.**
 *
 * Hier stehen **Auswahl und Zeitrechnung**, nicht die Zeichnung: Welche Fähigkeit
 * als Nächstes kommt, wann, mit welcher Stufe und Warnart, und in welcher
 * Reihenfolge sie erscheinen. Gezeichnet wird in `src/render/announcements.ts`,
 * vertont über die Ereignisse in `src/core/session.ts`. Beide fragen dieselbe
 * Klasse — eine zweite Rechnung im Bild hätte eine zweite Meinung darüber, wann
 * der Schlag kommt.
 *
 * # Die Zeitleiste wird GELESEN, nicht nachgebaut
 *
 * Fälligkeit, Schleife (#174), die Verzögerung durch STAY (#100), die
 * Phasen-Bindung (#557) und der laufende Boss-Zauber (#327) kommen aus dem Kampf
 * (`timelineDueAtMs`, `timelineEntryAtProgress`, `pendingTimelineDelayMs`,
 * `bossPhase`, `casting`). Was hier dazukommt, ist nur die eine Folgerung, die
 * der Kampf nicht braucht: Ein Eintrag kann nicht vor seinem Vorgänger feuern —
 * schiebt STAY den anstehenden über den nächsten hinaus, feuern beide im selben
 * Takt (`stepPassive`, die `for`-Schleife über die Zeitleiste).
 *
 * # Nur, was sicher kommt
 *
 * Ein an eine Phase gebundener Eintrag, dessen Phase noch nicht erreicht ist,
 * wird **nicht** angekündigt: Verfehlt der Boss die Schwelle, verfällt er
 * (GAME-DESIGN 11.1), und eine Ankündigung für etwas, das nicht kommt, wäre
 * eine falsche Auskunft. Ist die Phase erreicht, ist er sicher — Phasen laufen
 * nur vorwärts.
 *
 * # Die übrigen Bausteine (L4, Entscheidung vom 28.10.)
 *
 * Neben der Zeitleiste kündigt sich an, was der Boss sonst zu festen Zeiten tut,
 * nach der Zuordnung im Masterplan (A.5b, „Vier Warn-Arten"):
 *
 * | Baustein | Name aus | Art | Stufe | zählt auf |
 * | --- | --- | --- | --- | --- |
 * | Schwächung (`affliction`) | `affliction.name` | Schwächung auf dir | nach dem Schaden je Schritt | ihr Eintreten, danach den nächsten Schritt |
 * | Stapel (`stacks`) | `stacks.name` | Schwächung auf dir | nach dem nächsten Treffer | den nächsten Treffer |
 * | Wettrennen (`race`) | `race.name` | Gelegenheit | 1 — es trifft nicht | den Beginn, danach das späteste Ende |
 * | Zähler (`doom`) | `doom.name` | Jetzt handeln oder sterben | 3 | die Schwelle |
 * | Energieleiste (`energy`) | `onTrigger.name` | wie ein Schlag | nach dem Schaden | den Auslöser |
 *
 * Auch hier wird gelesen, nicht nachgebaut: Die Schritte stehen im Kampf
 * (`nextDoomStepAtMs` und die übrigen Zugänge); hier steht nur, wie viele
 * Schritte bis zur Schwelle fehlen.
 *
 * # Nichts entsteht im Takt
 *
 * `update` läuft je Bild und je Takt. Die Plätze werden einmal je Boss angelegt
 * (GAME-DESIGN 18.2), danach nur noch beschrieben.
 */
import { afterWard, type Combat } from '../../core/combat.js';
import { stackDamage } from '../../core/damage.js';
import type { BossDef, TimelineEntryDef } from '../../core/data.js';

// ------------------------------------------------------------- Die vier Arten

/** Schwächung auf dir — trifft nur den Spieler, verlangt persönliche Reaktion. */
export const KIND_DEBUFF = 0;
/** Allgemeiner Schaden — trifft ohne Zutun; man kann vorher heilen oder schützen. */
export const KIND_DAMAGE = 1;
/** Jetzt handeln oder sterben — unterbrechen sonst Wipe, Schutzschild jetzt oder Tod. */
export const KIND_LETHAL = 2;
/** Gelegenheit — ein Schadensfenster öffnet sich. */
export const KIND_OPPORTUNITY = 3;

/**
 * Kennungen der Arten für Werkzeuge und Prüffälle. **Keine Spieltexte** — die
 * Ankündigung nennt nur den Namen der Fähigkeit (GAME-DESIGN 11.5: „Keine
 * Erklärungen im Kampf").
 */
export const KIND_IDS: readonly string[] = ['debuff', 'damage', 'lethal', 'opportunity'];

// ------------------------------------------------------------ Die drei Stufen

/** Stufe 1 — der Balken läuft. Alles bekommt sie. */
export const TIER_BAR = 1;
/** Stufe 2 — Hervorhebung in den letzten Sekunden. Fähigkeiten, die hart treffen. */
export const TIER_HIGHLIGHT = 2;
/** Stufe 3 — Countdown, Blitz, Ruckeln. **Ausschließlich** für Tödliches. */
export const TIER_LETHAL = 3;

/**
 * Die „letzten Sekunden" aus Stufe 2 und der Countdown aus Stufe 3.
 *
 * **Aus dem Design abgeleitet, nicht gewählt:** Stufe 3 zählt „5-4-3-2-1", also
 * fünf Sekunden. Für Stufe 2 nennt 11.5 keine eigene Zahl („in den letzten
 * Sekunden"); eine zweite Länge wäre eine Erfindung, und zwei Fenster
 * verschiedener Länge zwei Regeln, die der Spieler lernen muss.
 */
export const FINAL_WINDOW_MS = 5000;

export interface Classification {
  readonly tier: number;
  readonly kind: number;
}

/** Woran sich „tödlich" bemisst: der Spieler DIESES Kampfes. */
export interface PlayerFrame {
  /** Volle Lebenspunkte mit Stufe und Ausrüstung (`Combat.playerVesselMax`). */
  readonly vesselMax: number;
  /** WARD aus der Ausrüstung in Promille (`Combat.wornValues.ward`). */
  readonly gearWardPermille: number;
}

/**
 * Stufe und Warnart eines Treffers — **aus dem Schaden abgeleitet**, ohne
 * eigenes Feld (Entscheidung des Orchestrators vom 28.10. zu L1–L3).
 *
 * | Schaden nach WARD, gegen die vollen Leben | Stufe | Art |
 * | --- | --- | --- |
 * | mindestens alles — tötet aus vollem Leben | 3 | Jetzt handeln oder sterben |
 * | mindestens die Hälfte — „trifft hart" | 2 | die Art bei überlebbarer Folge |
 * | darunter | 1 | die Art bei überlebbarer Folge |
 *
 * **Die Art folgt aus der FOLGE, die Stufe aus der SCHWERE.** „Unterbrechbar" ist
 * eine Markierung (11.6) und keine Art; ein überlebbarer Zauber ist allgemeiner
 * Schaden, ob man ihn unterbricht oder aushält.
 *
 * **Schaden, nicht Mechanik.** Der erste Entwurf gab jedem unterbrechbaren Zauber
 * Stufe 2; verworfen, weil ein Boss eine Datendatei ist und ein künftiger
 * schwacher Zauber nicht von selbst „hart" sein soll.
 *
 * Über `afterWard` — dieselbe Funktion, mit der `hitPlayer` rechnet. Die
 * Zersetzungs-Minderung (DECAY) zählt nicht: Sie ist ein Zustand des
 * Augenblicks, die Einstufung eine Eigenschaft der Fähigkeit.
 *
 * Gibt ein vorab angelegtes Objekt zurück — die Einstufung läuft auch im Takt.
 */
export function classifyDamage(
  damage: number,
  player: PlayerFrame,
  survivableKind: number,
): Classification {
  const angekommen = afterWard(damage, player.gearWardPermille, 0);
  if (angekommen >= player.vesselMax) return CLASSES[TIER_LETHAL]![KIND_LETHAL]!;
  const tier = angekommen * 2 >= player.vesselMax ? TIER_HIGHLIGHT : TIER_BAR;
  return CLASSES[tier]![survivableKind]!;
}

/** Ein Zeitleisten-Eintrag: überlebbar ist er Allgemeiner Schaden. */
export function classifyEntry(entry: TimelineEntryDef, player: PlayerFrame): Classification {
  return classifyDamage(entry.damage, player, KIND_DAMAGE);
}

/** Alle Paare aus Stufe und Art, einmal angelegt. */
const CLASSES: readonly (readonly Classification[])[] = [0, 1, 2, 3].map((tier) =>
  [KIND_DEBUFF, KIND_DAMAGE, KIND_LETHAL, KIND_OPPORTUNITY].map((kind) => ({ tier, kind })),
);

// --------------------------------------------------------- Die Ankündigungen

/** Eine Ankündigung — je Fähigkeit eine, beschrieben statt neu angelegt. */
export interface Announcement {
  /** Der Name aus den Bossdaten. „Cascade Failure in 5" statt „Danger in 5". */
  readonly name: string;
  tier: number;
  kind: number;
  /** Steht eine Wiederholung an? Nur dann erscheint sie. */
  active: boolean;
  /**
   * Läuft sie gerade — der Zauber wirkt, die Schwächung liegt, das Fenster ist
   * offen? Dann zählt sie auf seinen Treffer, ihren nächsten Schritt, sein Ende.
   */
  casting: boolean;
  /** Kampfzeit, auf die heruntergezählt wird. */
  dueAtMs: number;
  /** Kampfzeit, ab der ihr Balken läuft — die volle Abklingzeit beginnt hier. */
  sinceMs: number;
  /** `dueAtMs` minus jetzt. */
  remainingMs: number;
  /** Platz in `boss.timeline` des Eintrags, auf den gezählt wird; `-1` bei den übrigen Bausteinen. */
  entryIndex: number;
  /** Welcher Eintrag das ist — ändert er sich, beginnt der Balken neu. */
  target: number;
  /**
   * Fortschrittsstand des Eintrags — derselbe, solange er wartet UND solange er
   * wirkt. Der Klang fragt danach: Ein Zauber, der zu wirken beginnt, ist keine
   * neue Wiederholung und warnt nicht ein zweites Mal.
   */
  occurrence: number;
}

/** Noch kein Ziel in diesem Lauf: Der Balken beginnt beim Kampfbeginn. */
const NO_TARGET = Number.MIN_SAFE_INTEGER;

function slot(name: string): Announcement {
  return {
    name,
    tier: TIER_BAR,
    kind: KIND_DAMAGE,
    active: false,
    casting: false,
    dueAtMs: 0,
    sinceMs: 0,
    remainingMs: 0,
    entryIndex: -1,
    target: NO_TARGET,
    occurrence: -1,
  };
}

/** Liegt sie in ihren letzten Sekunden? */
export function inFinalWindow(a: Announcement): boolean {
  return a.remainingMs <= FINAL_WINDOW_MS;
}

/**
 * Wird sie gerade hervorgehoben? Stufe 2 und 3, in ihren letzten Sekunden, und
 * nur bis es soweit ist. Bild und Klang fragen dieselbe Regel.
 */
export function highlighted(a: Announcement): boolean {
  return a.tier >= TIER_HIGHLIGHT && a.remainingMs > 0 && inFinalWindow(a);
}

/**
 * Klingt sie, wenn sie in ihre letzten Sekunden tritt (Kanal 3)?
 *
 * | | klingt |
 * | --- | --- |
 * | Stufe 2 und 3 | ja — „Klang in den letzten Sekunden", „eigener Klang" |
 * | Schwächung auf dir, Gelegenheit, auch auf Stufe 1 | ja — „eigene Farbe und eigener Klang", „muss sich klar abheben" |
 * | Allgemeiner Schaden auf Stufe 1 | nein — „ruhigere Ankündigung" |
 * | was gerade läuft | nein — der Klang kam vor dem Eintreten; je Schritt einer liegenden Schwächung wäre er Lärm |
 */
export function warnsAudibly(a: Announcement): boolean {
  if (a.casting || a.remainingMs <= 0 || !inFinalWindow(a)) return false;
  return a.tier >= TIER_HIGHLIGHT || a.kind === KIND_DEBUFF || a.kind === KIND_OPPORTUNITY;
}

/** Die Sekunde, die der Countdown zeigt: aufgerundet wie jeder Countdown. */
export function countdownSecond(remainingMs: number): number {
  return remainingMs <= 0 ? 0 : Math.ceil(remainingMs / 1000);
}

/**
 * Dringlichkeit (GAME-DESIGN 11.5: „Nach Dringlichkeit sortiert — das Tödliche
 * oben").
 *
 * | Rang | Regel |
 * | --- | --- |
 * | 1 | Was in seinen letzten Sekunden ist, steht über allem anderen |
 * | 2 | Darunter: höhere Stufe zuerst — das Tödliche oben |
 * | 3 | Gleiche Stufe, oder außerhalb des Fensters: das Frühere zuerst |
 * | 4 | Gleichstand: Reihenfolge der Bossdaten, damit nichts zwischen zwei Bildern springt |
 *
 * **Die Stufe ordnet nur das GLEICHZEITIGE.** Die Regel steht in 11.5 unter
 * „Mehrere gleichzeitige Ereignisse". Außerhalb der letzten Sekunden ist die
 * Liste eine Zeitleiste der nächsten Ereignisse (Kanal 1) und liest sich von
 * oben nach unten in der Zeit.
 */
export function moreUrgent(a: Announcement, ai: number, b: Announcement, bi: number): boolean {
  const wa = inFinalWindow(a);
  const wb = inFinalWindow(b);
  if (wa !== wb) return wa;
  if (wa && a.tier !== b.tier) return a.tier > b.tier;
  if (a.remainingMs !== b.remainingMs) return a.remainingMs < b.remainingMs;
  return ai < bi;
}

/**
 * Plant die Ankündigungen eines Kampfes.
 *
 * Eine Instanz je Leser — das Bild hat eine, der Takt eine. Beide rechnen mit
 * denselben Regeln aus demselben Kampf, nur zu ihrer eigenen Zeit.
 */
export class BossAnnouncer {
  private boss: BossDef | null = null;
  private vesselMax = -1;
  private gearWard = -1;
  /** Woran „tödlich" und „hart" gemessen werden — beim Aufbau festgehalten. */
  private player: PlayerFrame = { vesselMax: 0, gearWardPermille: 0 };
  private slots: Announcement[] = [];
  /** Plätze der übrigen Bausteine in `slots`, `-1`, wenn der Boss ihn nicht trägt. */
  private slotEnergy = -1;
  private slotDoom = -1;
  private slotStacks = -1;
  private slotAffliction = -1;
  private slotRace = -1;
  /** `boss.timeline`-Platz → Fähigkeit (Platz in `slots`). */
  private abilityOfEntry = new Int32Array(0);
  private tierOfEntry = new Int8Array(0);
  private kindOfEntry = new Int8Array(0);
  private found = new Uint8Array(0);
  /** Plätze in `slots`, dringlichstes zuerst. Gültig sind die ersten `count`. */
  private order = new Int32Array(0);
  private countValue = 0;
  private lastNowMs = Number.POSITIVE_INFINITY;

  /** Wie viele Ankündigungen gerade anstehen. */
  get count(): number {
    return this.countValue;
  }

  /** Die Ankündigung auf Rang `rank` (0 = die dringlichste). */
  at(rank: number): Announcement {
    return this.slots[this.order[rank] as number] as Announcement;
  }

  /** Alle Fähigkeiten dieses Bosses, je eine — auch die gerade nicht anstehenden. */
  get abilities(): readonly Announcement[] {
    return this.slots;
  }

  /**
   * Die Tabelle für einen Boss anlegen — **die einzige Stelle, die erzeugt**.
   *
   * Läuft, wenn der Boss wechselt oder sich Leben/WARD des Spielers ändern
   * (Stufe, Ausrüstung zwischen zwei Kämpfen): Die Einstufung „tödlich" hängt
   * daran. Im Kampf ändert sich keins von beiden.
   */
  private rebuild(boss: BossDef, player: PlayerFrame): void {
    this.boss = boss;
    this.vesselMax = player.vesselMax;
    this.gearWard = player.gearWardPermille;
    const namen: string[] = [];
    const timeline = boss.timeline;
    this.abilityOfEntry = new Int32Array(timeline.length);
    this.tierOfEntry = new Int8Array(timeline.length);
    this.kindOfEntry = new Int8Array(timeline.length);
    for (let i = 0; i < timeline.length; i++) {
      const entry = timeline[i] as TimelineEntryDef;
      let a = namen.indexOf(entry.name);
      if (a < 0) {
        a = namen.length;
        namen.push(entry.name);
      }
      this.abilityOfEntry[i] = a;
      const c = classifyEntry(entry, player);
      this.tierOfEntry[i] = c.tier;
      this.kindOfEntry[i] = c.kind;
    }
    this.player = player;
    // Die übrigen Bausteine, je einer, hinter der Zeitleiste — eigene Plätze,
    // auch wenn ein Name einem Zeitleisten-Eintrag gliche: Sie zählen auf
    // verschiedene Dinge.
    const dazu = (name: string): number => namen.push(name) - 1;
    const energy = boss.energy;
    this.slotEnergy =
      energy !== null && energy.perStepMs > 0 && energy.amount > 0
        ? dazu(energy.onTrigger.name)
        : -1;
    const doom = boss.doom;
    this.slotDoom = doom !== null && doom.perStepMs > 0 && doom.amount > 0 ? dazu(doom.name) : -1;
    this.slotStacks =
      boss.stacks !== null && boss.stacks.perStepMs > 0 ? dazu(boss.stacks.name) : -1;
    this.slotAffliction = boss.affliction !== null ? dazu(boss.affliction.name) : -1;
    this.slotRace = boss.race !== null ? dazu(boss.race.name) : -1;
    this.slots = namen.map(slot);
    this.found = new Uint8Array(namen.length);
    this.order = new Int32Array(namen.length);
    this.countValue = 0;
    this.lastNowMs = Number.POSITIVE_INFINITY;
  }

  /**
   * Den Stand zur Kampfzeit `nowMs` nachziehen.
   *
   * @param nowMs Kampfzeit seit Beginn (`Session.fightElapsedMs`), im Bild auch
   *   zwischen zwei Takten.
   */
  update(combat: Combat, nowMs: number): void {
    const boss = combat.boss;
    const vesselMax = combat.playerVesselMax;
    const gearWard = combat.wornValues.ward;
    if (boss !== this.boss || vesselMax !== this.vesselMax || gearWard !== this.gearWard) {
      this.rebuild(boss, { vesselMax, gearWardPermille: gearWard });
    }
    // **Ein neuer Lauf** — die Uhr lief rückwärts. Die Balken beginnen wieder beim
    // Kampfbeginn, sonst trüge der erste Balken die Länge aus dem vorigen Versuch.
    if (nowMs < this.lastNowMs) {
      for (const a of this.slots) a.target = NO_TARGET;
    }
    this.lastNowMs = nowMs;
    this.found.fill(0);

    const laenge = boss.timeline.length;
    const schleife = boss.timelineLoopMs !== null;
    if (laenge > 0) {
      // 1. Der laufende Zauber zählt auf seinen Treffer. Über `casting` und
      //    nicht über die Mechanik: PIN macht auch aus einem Schlag einen
      //    laufenden Zauber (#478), und der landet ebenso.
      const laufend = combat.casting;
      if (laufend >= 0) {
        const platz = schleife ? laufend % laenge : laufend;
        const a = this.abilityOfEntry[platz] as number;
        this.place(
          a,
          platz,
          -2 - laufend,
          laufend,
          combat.castLandsAtMs,
          true,
          combat.castingStartedAtMs,
        );
      }

      // 2. Die nächste Wiederholung jeder Fähigkeit — und nur die nächste
      //    (GAME-DESIGN 11.5: kein Zähler „noch 3×"). Eine Schleife wird einen
      //    Zyklus weit gelesen: Darin kommt jeder Eintrag genau einmal vor.
      const anfang = combat.timelineIndex;
      const ende = schleife ? anfang + laenge : laenge;
      let vorgaenger = Number.NEGATIVE_INFINITY;
      for (let k = anfang; k < ende; k++) {
        let faellig = combat.timelineDueAtMs(k);
        if (faellig < 0) break;
        if (k === anfang) faellig += combat.pendingTimelineDelayMs;
        if (faellig < vorgaenger) faellig = vorgaenger;
        vorgaenger = faellig;
        const eintrag = combat.timelineEntryAtProgress(k) as TimelineEntryDef;
        if (eintrag.phase !== undefined && combat.bossPhase < eintrag.phase) continue;
        const platz = schleife ? k % laenge : k;
        const a = this.abilityOfEntry[platz] as number;
        if (this.found[a] === 1) continue;
        this.place(a, platz, k, k, faellig, false, nowMs);
      }
    }

    // 3. Die übrigen Bausteine.
    this.planMechanics(combat, boss, nowMs);

    // Rangfolge: Einfügen in eine vorab angelegte Liste — wenige Einträge,
    // kein Speicher.
    let n = 0;
    for (let a = 0; a < this.slots.length; a++) {
      const ann = this.slots[a] as Announcement;
      ann.active = this.found[a] === 1;
      if (!ann.active) continue;
      ann.remainingMs = ann.dueAtMs - nowMs;
      let j = n;
      while (j > 0) {
        const vorne = this.order[j - 1] as number;
        if (!moreUrgent(ann, a, this.slots[vorne] as Announcement, vorne)) break;
        this.order[j] = vorne;
        j--;
      }
      this.order[j] = a;
      n++;
    }
    this.countValue = n;
  }

  /**
   * Zähler, Stapel, Schwächung, Wettrennen und Energieleiste (L4).
   *
   * **Bis zur Schwelle wird gezählt, nicht geschätzt.** Zähler und Energieleiste
   * steigen je Schritt um einen festen Betrag; wie viele Schritte fehlen, folgt
   * aus dem Stand. Senkt der Spieler den Zähler, rückt die Ankündigung im selben
   * Bild nach hinten — der Stand ist ein anderer, die Rechnung dieselbe.
   */
  private planMechanics(combat: Combat, boss: BossDef, nowMs: number): void {
    const player = this.player;
    const energy = boss.energy;
    if (this.slotEnergy >= 0 && energy !== null) {
      const fehlt = energy.threshold - combat.bossEnergy;
      const schritte = fehlt <= 0 ? 1 : Math.ceil(fehlt / energy.amount);
      const faellig = combat.nextEnergyStepAtMs + (schritte - 1) * energy.perStepMs;
      const c = classifyDamage(energy.onTrigger.damage, player, KIND_DAMAGE);
      const n = combat.bossEnergyTriggers;
      this.setze(this.slotEnergy, n, n, faellig, false, nowMs, c.tier, c.kind, -1);
    }

    const doom = boss.doom;
    if (this.slotDoom >= 0 && doom !== null && !combat.playerDead) {
      const fehlt = doom.threshold - combat.doom;
      const schritte = fehlt <= 0 ? 1 : Math.ceil(fehlt / doom.amount);
      const faellig = combat.nextDoomStepAtMs + (schritte - 1) * doom.perStepMs;
      // Die Schwelle tötet — das ist die Mechanik (11.2, Baustein 2), kein Schaden,
      // den WARD mindert. Stufe 3 nach der Entscheidung zu L4.
      this.setze(
        this.slotDoom,
        faellig,
        faellig,
        faellig,
        false,
        nowMs,
        TIER_LETHAL,
        KIND_LETHAL,
        -1,
      );
    }

    const stacks = boss.stacks;
    if (this.slotStacks >= 0 && stacks !== null) {
      const faellig = combat.nextStackStepAtMs;
      const naechster =
        combat.bossStacks < stacks.maxStacks ? combat.bossStacks + 1 : stacks.maxStacks;
      const c = classifyDamage(stackDamage(stacks, naechster), player, KIND_DEBUFF);
      this.setze(this.slotStacks, faellig, faellig, faellig, false, nowMs, c.tier, c.kind, -1);
    }

    const aff = boss.affliction;
    if (this.slotAffliction >= 0 && aff !== null) {
      const c = classifyDamage(aff.damage, player, KIND_DEBUFF);
      if (combat.afflicted) {
        // Sie liegt: Gezählt wird auf den nächsten Schritt, und die Zeile zeigt,
        // dass sie läuft. Ein Klang je Schritt wäre Lärm — er kam vor dem Eintreten.
        const faellig = combat.nextAfflictionTickAtMs;
        this.setze(this.slotAffliction, faellig, faellig, faellig, true, nowMs, c.tier, c.kind, -1);
      } else if (combat.afflictionsCleared === 0 && nowMs < aff.atMs) {
        // Sie greift genau einmal; einmal entfernt, kommt sie nicht wieder.
        this.setze(
          this.slotAffliction,
          aff.atMs,
          aff.atMs,
          aff.atMs,
          false,
          nowMs,
          c.tier,
          c.kind,
          -1,
        );
      }
    }

    const race = boss.race;
    if (this.slotRace >= 0 && race !== null) {
      // Das Wettrennen trifft nicht; es ist eine Gelegenheit, und seine Stufe ist
      // die eines Schadens von null.
      const c = classifyDamage(0, player, KIND_OPPORTUNITY);
      if (combat.raceOpen) {
        const ende = combat.raceEndsAtFightMs;
        this.setze(this.slotRace, ende, ende, ende, true, nowMs, c.tier, c.kind, -1);
      } else if (!combat.raceStarted) {
        this.setze(
          this.slotRace,
          race.atMs,
          race.atMs,
          race.atMs,
          false,
          nowMs,
          c.tier,
          c.kind,
          -1,
        );
      }
    }
  }

  private place(
    a: number,
    platz: number,
    ziel: number,
    occurrence: number,
    dueAtMs: number,
    casting: boolean,
    beginntMs: number,
  ): void {
    this.setze(
      a,
      ziel,
      occurrence,
      dueAtMs,
      casting,
      beginntMs,
      this.tierOfEntry[platz] as number,
      this.kindOfEntry[platz] as number,
      platz,
    );
  }

  private setze(
    a: number,
    ziel: number,
    occurrence: number,
    dueAtMs: number,
    casting: boolean,
    beginntMs: number,
    tier: number,
    kind: number,
    platz: number,
  ): void {
    const ann = this.slots[a] as Announcement;
    this.found[a] = 1;
    ann.occurrence = occurrence;
    ann.casting = casting;
    ann.dueAtMs = dueAtMs;
    ann.entryIndex = platz;
    ann.tier = tier;
    ann.kind = kind;
    if (ann.target !== ziel) {
      // Die volle Abklingzeit beginnt dort, wo die vorige Wiederholung endete —
      // beim ersten Mal mit dem Kampf.
      ann.sinceMs = ann.target === NO_TARGET && !casting ? 0 : Math.max(0, beginntMs);
      ann.target = ziel;
    }
  }
}
