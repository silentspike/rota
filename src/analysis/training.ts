/**
 * Der Trainingsraum (GAME-DESIGN 15.5, Issue #84).
 *
 * WARUM DIE LAUFENDE ANZEIGE DER KERN IST, NICHT DIE PUPPE
 * -------------------------------------------------------
 * Ohne sie ist der Trainingsraum ein Boss ohne Zeitlimit. Mit ihr sieht der
 * Spieler im selben Moment, in dem er einen Fehler macht, dass er ihn gemacht
 * hat — und das ist der einzige Ort im Spiel, an dem eine solche Rueckmeldung
 * erlaubt ist. Im Kampf waere sie eine Hilfe, die dem Spieler die Aufgabe
 * abnimmt (GAME-DESIGN 21.2).
 *
 * DIE PUPPE IST EIN BOSS, KEIN SONDERFALL
 * ---------------------------------------
 * Sie entsteht als `BossDef` zur Laufzeit. `Combat` bleibt unangetastet: Was
 * die Puppe von einem Boss unterscheidet, sind Werte, keine Regeln — und ein
 * zweiter Kampf-Pfad neben dem echten wuerde auseinanderlaufen, sobald einer
 * gepflegt wird.
 *
 * Die drei Kennzahlen kommen aus EINER Stelle (`RunSummary`, spaeter der
 * Verlustrechnung). Sie hier neu zu rechnen waere eine zweite Wahrheit ueber
 * dieselbe Zahl — und die zweite wird beim Nachziehen vergessen.
 */
import type { BossDef, TimelineEntryDef } from '../core/data.js';

/** Was der Spieler an der Puppe einstellen kann (GAME-DESIGN 15.5). */
export interface DummySettings {
  /**
   * Widerstand gegen die eigene Schule, 0 bis 1. 1 heisst kein Widerstand.
   *
   * Landet unveraendert in `boss.resistances` und damit im
   * Widerstandsfaktor der Schadensformel (src/core/damage.ts).
   */
  readonly resistance: number;
  /** Ob die Puppe zurueckschlaegt — zum Ueben unter Druck. */
  readonly dealsDamage: boolean;
  /** Ob sie Zauber wirkt — zum Ueben des Unterbrechens. */
  readonly castsSpells: boolean;
}

export const DEFAULT_DUMMY: DummySettings = {
  resistance: 1,
  dealsDamage: false,
  castsSpells: false,
};

/**
 * Unbegrenztes Leben.
 *
 * Nicht `Infinity`: Der Wert geht in Pruefsumme und Anzeige ein, und beides
 * verkraeftet keine Unendlichkeit. Eine Zahl, die in keinem Uebungsdurchgang
 * erreicht wird, tut dasselbe, ohne irgendwo zu brechen.
 */
export const DUMMY_HP = 1_000_000_000;

/** Kein Zeitlimit — derselbe Gedanke wie bei DUMMY_HP. */
export const DUMMY_OVERPRESSURE_MS = 86_400_000;

/**
 * Die Puppe als Boss-Definition.
 *
 * `school` bestimmt, gegen welche Schule der Widerstand wirkt: Der Faktor wird
 * in `Combat` als `boss.resistances[school]` gelesen, nicht pauschal.
 */
export function buildDummy(settings: DummySettings, school: string): BossDef {
  return {
    schema: 1,
    id: 'training-dummy',
    name: 'Training dummy',
    instance: 'training',
    hp: DUMMY_HP,
    overpressureMs: DUMMY_OVERPRESSURE_MS,
    resistances: { [school]: settings.resistance },
    phases: [],
    energy: null,
    // Die Puppe bekommt weder Zähler noch Stapel (Issue #216). Beide töten
    // beziehungsweise machen den Schaden unbezahlbar — an einem Sandsack, an
    // dem geübt wird, wäre das genau verkehrt (GAME-DESIGN 15.5).
    doom: null,
    stacks: null,
    // Auch kein Wettrennen und keine Boss-Heilung (Issue #218). Beide wirken
    // auf den Fortschritt — an einem Sandsack, der kein Zeitlimit hat und
    // nicht fallen soll, waere das ohne Bedeutung.
    race: null,
    bossHeal: null,
    // Und keiner der drei letzten: Das umgekehrte Ziel (#247) beendete den
    // Uebungsdurchgang, die Eskalation setzt eine Wiederholung mit Anspruch
    // voraus, und eine Schwaechung ohne Heil-Loesung waere an einem Sandsack
    // nur laestig.
    // Die Puppe hat keine Stufe im Sinne des Aufstiegs: Sie gibt keine
    // Erfahrung, weil an ihr nichts zu gewinnen ist (15.5).
    level: 1,
    runaway: null,
    escalation: null,
    affliction: null,
    timeline: buildDummyTimeline(settings),
    // **Die Puppe wiederholt sich** (#174, GAME-DESIGN 15.5 „kein Zeitlimit").
    // Wer eine Rotation ueber drei Minuten einschleift, uebte sonst nach der
    // ersten Minute gegen einen Sandsack, der nichts mehr tut — und es faellt
    // nicht auf, weil der Durchgang weiterlaeuft und nur die Boss-Ereignisse
    // aufhoeren.
    //
    // Die Zykluslaenge ist die Spanne selbst: Die Abstaende (12 s fuer Zauber,
    // 8 s fuer Schlaege) teilen sie beide, deshalb entsteht am Uebergang weder
    // eine Luecke noch eine Haeufung.
    timelineLoopMs: DUMMY_TIMELINE_SPAN_MS,
  };
}

/** Abstand zwischen zwei unterbrechbaren Zaubern. */
const DUMMY_CAST_INTERVAL_MS = 12_000;
/** Wirkzeit eines solchen Zaubers — das Fenster zum Unterbrechen. */
const DUMMY_CAST_TIME_MS = 2_500;
/** Abstand zwischen zwei Schlaegen. */
const DUMMY_STRIKE_INTERVAL_MS = 8_000;
/** Wie weit die Zeitleiste im Voraus gefuellt wird. */
const DUMMY_TIMELINE_SPAN_MS = 48_000;

/**
 * Schaden eines Puppen-Eintrags am Spieler.
 *
 * Klein genug, dass ein Uebungsdurchgang nicht am Spielertod endet, und gross
 * genug, dass ein Treffer sichtbar ist.
 */
const DUMMY_DAMAGE = 40;

/**
 * Die Zeitleiste der Puppe.
 *
 * **Ohne `as unknown as`** (Issue #167): Genau diese Zusicherung hat vier
 * falsche Feldnamen (`kind`, `ability`, `castMs`, fehlendes `damage`) am
 * Uebersetzer vorbeigeschleust. `Combat` las dann `eintrag.mechanic` als
 * `undefined`, fiel in den Schlag-Zweig und rief `hitPlayer(undefined)` — das
 * Spielerleben wurde `NaN`, und keine einzige Unterbrechungs-Gelegenheit
 * entstand. Wo eine Struktur nicht passt, ist die Struktur falsch.
 *
 * # Was die beiden Einstellungen steuern
 *
 * | Einstellung | Wirkung |
 * | --- | --- |
 * | `castsSpells` | ob unterbrechbare Zauber auf der Zeitleiste stehen |
 * | `dealsDamage` | ob ueberhaupt Schaden am Spieler entsteht |
 *
 * Sie sind unabhaengig. Steht `castsSpells` auf `true` und `dealsDamage` auf
 * `false`, gibt es Gelegenheiten zum Unterbrechen, die nichts kosten — der
 * Spieler uebt das Zeitfenster ohne Druck. Das folgt aus der Zusage von
 * `dealsDamage`: „macht keinen Schaden" muss auch dann gelten, wenn zugleich
 * gezaubert wird, sonst waere die Einstellung an eine zweite gekoppelt.
 *
 * Die Eintraege stehen **aufsteigend nach `atMs`** — `Combat` laeuft mit einem
 * Zeiger vorwaerts durch die Liste und bricht beim ersten nicht faelligen
 * Eintrag ab.
 */
function buildDummyTimeline(settings: DummySettings): readonly TimelineEntryDef[] {
  const schaden = settings.dealsDamage ? DUMMY_DAMAGE : 0;
  const eintraege: TimelineEntryDef[] = [];

  if (settings.castsSpells) {
    for (
      let at = DUMMY_CAST_INTERVAL_MS;
      at <= DUMMY_TIMELINE_SPAN_MS;
      at += DUMMY_CAST_INTERVAL_MS
    ) {
      eintraege.push({
        atMs: at,
        mechanic: 'interruptibleCast',
        name: 'Dummy Cast',
        damage: schaden,
        castTimeMs: DUMMY_CAST_TIME_MS,
        // Die Puppe bricht nichts ab (Issue #304): Sie ist ein Prueffeld,
        // kein Boss mit Mechanik.
        interruptsPlayer: false,
      });
    }
  }

  // Schlaege gibt es nur, wenn die Puppe zurueckschlagen soll. Ohne sie waere
  // `dealsDamage` bei ausgeschaltetem `castsSpells` wirkungslos — genau der
  // Zustand, den Issue #167 als „steht in der Schnittstelle und bewirkt
  // nichts" beschreibt.
  if (settings.dealsDamage) {
    for (
      let at = DUMMY_STRIKE_INTERVAL_MS;
      at <= DUMMY_TIMELINE_SPAN_MS;
      at += DUMMY_STRIKE_INTERVAL_MS
    ) {
      eintraege.push({
        atMs: at,
        mechanic: 'strike',
        name: 'Dummy Strike',
        damage: DUMMY_DAMAGE,
        castTimeMs: 0,
        // Die Puppe bricht nichts ab (Issue #304): Sie ist ein Prueffeld,
        // kein Boss mit Mechanik.
        interruptsPlayer: false,
      });
    }
  }

  eintraege.sort((a, b) => a.atMs - b.atMs);
  return eintraege;
}

/** Der Zustand eines Uebungsdurchgangs. */
export type TrainingPhase = 'idle' | 'running';

/** Die drei Kennzahlen, wie sie waehrend des Uebens angezeigt werden. */
export interface LiveMetrics {
  /** Verlorene Zeit in ms und ihr Anteil, 0 bis 1. */
  readonly lostTimeMs: number;
  readonly lostTimeShare: number;
  /** Verpasste Wirkungen — BEZOGEN, nicht hier gerechnet. */
  readonly missedCasts: number;
  /** Genutzte von moeglichen Synergien. */
  readonly synergyUsed: number;
  readonly synergyAvailable: number;
}

export interface ChangeResult {
  readonly accepted: boolean;
  /** Englisch — steht im Spiel. Leer, wenn angenommen. */
  readonly reason: string;
}

/**
 * Der Trainingsraum als Zustand.
 *
 * Er haelt NICHT die Kennzahlen — die kommen bei jedem Bild aus der laufenden
 * Sitzung. Er haelt nur, was der Spieler eingestellt hat und ob gerade geuebt
 * wird.
 */
export class TrainingRoom {
  private phaseValue: TrainingPhase = 'idle';
  private settingsValue: DummySettings = DEFAULT_DUMMY;
  private loadoutValue: readonly string[] = [];

  get phase(): TrainingPhase {
    return this.phaseValue;
  }

  get settings(): DummySettings {
    return this.settingsValue;
  }

  get loadout(): readonly string[] {
    return this.loadoutValue;
  }

  get isRunning(): boolean {
    return this.phaseValue === 'running';
  }

  /**
   * Die Zusammenstellung wechseln — nur zwischen Durchgaengen.
   *
   * Mitten im Ueben abgelehnt (GAME-DESIGN 15.5): Sonst waeren die Kennzahlen
   * des laufenden Durchgangs eine Mischung aus zwei Zusammenstellungen und
   * saegen richtig aus, ohne es zu sein.
   */
  setLoadout(spellIds: readonly string[]): ChangeResult {
    if (this.isRunning) {
      return {
        accepted: false,
        reason: 'Loadout can only be changed between runs.',
      };
    }
    this.loadoutValue = [...spellIds];
    return { accepted: true, reason: '' };
  }

  /**
   * Die Puppe einstellen — ebenfalls nur zwischen Durchgaengen.
   *
   * Aus demselben Grund: Ein mitten im Durchgang eingeschalteter Widerstand
   * machte den Schadenswert der ersten Haelfte unvergleichbar mit dem der
   * zweiten.
   */
  setSettings(settings: DummySettings): ChangeResult {
    if (this.isRunning) {
      return {
        accepted: false,
        reason: 'Dummy settings can only be changed between runs.',
      };
    }
    this.settingsValue = { ...settings };
    return { accepted: true, reason: '' };
  }

  start(): ChangeResult {
    if (this.isRunning) return { accepted: false, reason: 'Already running.' };
    this.phaseValue = 'running';
    return { accepted: true, reason: '' };
  }

  /** Jederzeit abbrechbar (GAME-DESIGN 15.5) — ohne Bedingung. */
  stop(): void {
    this.phaseValue = 'idle';
  }

  /** Die Puppe fuer den naechsten Durchgang. */
  dummyFor(school: string): BossDef {
    return buildDummy(this.settingsValue, school);
  }
}
