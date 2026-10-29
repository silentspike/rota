/**
 * Overdrive — der Tempo-Schub auf eigener Taste (#1177, Abschnitt 2).
 *
 * Das Gegenstück zu `sim/src/overdrive.rs`, Zeile für Zeile dieselbe Regel.
 *
 * > **Overdrive** (statt „Bloodlust"): außerhalb der Schulen, belegt keinen
 * > Leisten-Platz, eigene Taste **G**, frei zündbar, einmal pro Kampf, kurzer
 * > starker Tempo-Schub. […] der verlässliche Angelpunkt fürs Zusammenlegen
 * > der großen Fähigkeiten.
 * >
 * > — Masterplan A.4, Zeile „Overdrive + Überdruck"
 *
 * # Was „Tempo" hier heißt
 *
 * CATALYSIS. GAME-DESIGN 6.2 sagt es wörtlich: CATALYSIS „wächst dauerhaft
 * über Talente und Ausrüstung, kurzzeitig über Overdrive". Overdrive geht
 * deshalb durch **dieselbe** Stelle wie getragenes Tempo (`Combat.hasted`) und
 * wirkt dort, wo Tempo wirkt — auf Abklingzeiten, Wirkzeiten, Kanalisierungen
 * und DoT-Abstände. **Nicht auf die globale Wartezeit:** Die sinkt seit dem
 * 2026-07-29 ausschließlich über den Talentbaum (6.2, 6.7).
 *
 * # Die drei Zustände
 *
 * | Zustand    | `ignitedAtMs` | Wirkung |
 * | ---------- | ------------- | ------- |
 * | bereit     | `-1`          | keine |
 * | aktiv      | `t`, `jetzt < t + Dauer` | `+overdriveCatalysisPermille` |
 * | verbraucht | `t`, `jetzt >= t + Dauer` | keine, und kein zweites Mal |
 *
 * Einen vierten gibt es mit Absicht nicht: keine Abklingzeit, nach der
 * Overdrive zurückkäme. „Einmal pro Kampf" heißt, dass nur ein neuer Kampf ihn
 * zurückgibt — `Combat.reset()`.
 *
 * # Die Zahlen sind vorläufig
 *
 * Dauer und Stärke stehen in `data/player/base.json`, mit ihrer Herleitung.
 * Gemessen sind sie nicht; das ist Sache des Balance-Läufers.
 */
import { PLAYER } from '../core/data.js';

/** Noch nicht gezündet. */
const NOT_IGNITED = -1;

/** Die Zustände für die Anzeige. */
export const OVERDRIVE_READY = 0;
export const OVERDRIVE_ACTIVE = 1;
export const OVERDRIVE_SPENT = 2;

export class Overdrive {
  /** Wann gezündet wurde, sonst `NOT_IGNITED`. */
  private ignitedAt = NOT_IGNITED;

  /** Wie lange er wirkt, in ms — aus `data/player/base.json`. */
  static get durationMs(): number {
    return PLAYER.overdriveDurationMs;
  }

  /** Wie viel CATALYSIS er solange dazugibt, in Promille. */
  static get catalysisBonus(): number {
    return PLAYER.overdriveCatalysisPermille;
  }

  /**
   * Zünden. `false`, wenn er in diesem Kampf schon gezündet wurde.
   *
   * **Ein zweiter Druck tut nichts** — er verlängert nicht, er setzt nicht neu,
   * er verschiebt nicht. Sonst wäre „einmal pro Kampf" eine Frage des
   * Zeitpunkts, zu dem man zum zweiten Mal drückt.
   */
  ignite(timeMs: number): boolean {
    if (this.ignitedAt !== NOT_IGNITED) return false;
    this.ignitedAt = timeMs;
    return true;
  }

  /** Wann gezündet wurde, oder -1. */
  get ignitedAtMs(): number {
    return this.ignitedAt;
  }

  /** Noch nicht gezündet? */
  get isReady(): boolean {
    return this.ignitedAt === NOT_IGNITED;
  }

  /** Wann die Wirkung endet, oder -1, solange nicht gezündet. */
  get endsAtMs(): number {
    return this.ignitedAt === NOT_IGNITED ? NOT_IGNITED : this.ignitedAt + Overdrive.durationMs;
  }

  /**
   * Wirkt er gerade?
   *
   * **Halboffen, `[Zündung, Zündung + Dauer)`.** Im Takt der Zündung wirkt er
   * schon — ein Zauber, der im selben Takt nach dem Druck auf G gewirkt wird,
   * bekommt das Tempo —, im Takt des Endes nicht mehr. Genau `Dauer / Takt`
   * Takte also, keiner mehr und keiner weniger.
   */
  isActive(timeMs: number): boolean {
    if (this.ignitedAt === NOT_IGNITED) return false;
    return timeMs >= this.ignitedAt && timeMs < this.endsAtMs;
  }

  /** Restlaufzeit in ms; 0, wenn er nicht wirkt. */
  remainingMs(timeMs: number): number {
    return this.isActive(timeMs) ? this.endsAtMs - timeMs : 0;
  }

  /** Was er in diesem Moment zu CATALYSIS beiträgt, in Promille. */
  catalysisPermille(timeMs: number): number {
    return this.isActive(timeMs) ? Overdrive.catalysisBonus : 0;
  }

  /**
   * Bereit, aktiv oder verbraucht — für die Anzeige.
   *
   * Nur am Ende gemessen, nicht am Beginn: Das Bild rechnet zwischen zwei
   * Takten (`shownMs`), und ein Zeitpunkt knapp vor dem Zünd-Takt darf nicht
   * als „verbraucht" erscheinen.
   */
  state(timeMs: number): number {
    if (this.ignitedAt === NOT_IGNITED) return OVERDRIVE_READY;
    return timeMs < this.endsAtMs ? OVERDRIVE_ACTIVE : OVERDRIVE_SPENT;
  }

  /** Ein neuer Kampf gibt ihn zurück. */
  reset(): void {
    this.ignitedAt = NOT_IGNITED;
  }
}
