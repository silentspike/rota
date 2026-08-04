/**
 * Das Ereignis-System des Kampfes (GAME-DESIGN 18.3).
 *
 * > Jeder Zauber, jedes Talent, jeder legendäre Effekt beschreibt nur, **worauf
 * > er reagiert**. Empfänger in fest definierter Reihenfolge — keine
 * > Hash-Container. Sonst weichen Spiel und Simulator voneinander ab.
 *
 * **Zwei Dinge heißen in diesem Projekt „Ereignis", und sie sind verschieden:**
 *
 * | Wo                         | Frage                    | Beispiel                     |
 * | -------------------------- | ------------------------ | ---------------------------- |
 * | `sim/src/event.rs`         | **Wann** geschieht etwas | „In 2000 ms tickt der Brand" |
 * | **hier**                   | **Worauf** reagiert wer  | „Ein Zauber wurde gewirkt"   |
 * | `src/core/events.ts`       | Was hört der Spieler     | „Countdown, dritter Schlag"  |
 *
 * Das erste ist eine Zeitplan-Warteschlange, das dritte die Präsentation. Hier
 * stehen die **Regeln**: Was hier geschieht, verändert das Ergebnis des Kampfes
 * und muss deshalb im Simulator Zeichen für Zeichen gleich ablaufen.
 *
 * **Warum die Reihenfolge nicht verhandelbar ist:** Sie fällt erst auf, wenn
 * zwei Effekte auf dasselbe Ereignis hören — und dann nicht sofort, sondern in
 * einer Balance-Rechnung Wochen später, die niemand mehr mit diesem Modul in
 * Verbindung bringt.
 */
import { mixFloat, mixInt } from '../core/checksum.js';

/** Ein Zauber wurde gewirkt. Betreff: Zauber-Kennnummer. */
export const RULE_SPELL_CAST = 0;
/** Schaden wurde verursacht. Betreff: Zauber, Wert: Schaden. */
export const RULE_DAMAGE = 1;
/** Ein Treffer war ein Krit. Betreff: Zauber, Wert: Schaden. */
export const RULE_CRIT = 2;
/** Ein Effekt über Zeit hat getickt. Betreff: Zauber, Wert: Schaden. */
export const RULE_DOT_TICK = 3;
/** Der Boss betritt eine Phase. Wert: Nummer der Phase. */
export const RULE_BOSS_PHASE = 4;
/** Eine Verstärkung ist abgelaufen. Betreff: Kennnummer der Verstärkung. */
export const RULE_BUFF_EXPIRED = 5;

export const RULE_EVENT_COUNT = 6;

/**
 * Die Namen der sechs Ereignisse.
 *
 * **Reihenfolge und Wortlaut sind Schnittstelle** — der Simulator führt
 * dieselbe Liste, und die Prüfsumme rührt die Kennnummer ein.
 */
export const RULE_EVENT_NAMES: readonly string[] = [
  'spell.cast',
  'damage',
  'crit',
  'dot.tick',
  'boss.phase',
  'buff.expired',
];

/** Kein Betreff. */
export const NO_SUBJECT = -1;

/**
 * Wer auf ein Ereignis reagiert.
 *
 * **Nur Zahlen, kein Objekt.** Der Empfang läuft im Kampf-Pfad; dort entsteht
 * nichts (GAME-DESIGN 18.4: „Keine Speicheranforderung im Schritt").
 */
export interface RuleReceiver {
  onRuleEvent(event: number, subject: number, value: number, timeMs: number): void;
}

/**
 * Die Anmeldung — ein **Datensatz**, kein Code (Leitplanke 6).
 *
 * Ein neuer Zauber bringt seine Anmeldung in seinen Daten mit; am
 * Ereignis-System ändert sich dabei nichts.
 */
export interface Registration {
  /** Wer sich anmeldet. Erscheint in der Reihenfolge-Auskunft. */
  readonly id: string;
  /** Auf welches Ereignis. */
  readonly on: number;
  /**
   * Rang innerhalb desselben Ereignisses. Kleiner heißt früher.
   *
   * Bei Gleichstand entscheidet die Reihenfolge der Anmeldung — und die folgt
   * aus den Daten, nicht aus dem Zufall einer Sammlung.
   */
  readonly order?: number;
}

interface Entry {
  readonly id: string;
  readonly order: number;
  /** Laufende Nummer der Anmeldung. Löst Gleichstand eindeutig auf. */
  readonly seq: number;
  readonly receiver: RuleReceiver;
}

/**
 * Die Verteilung.
 *
 * Je Ereignis eine **zusammenhängende Liste**, nach `order` und Anmelde-Nummer
 * sortiert. Kein `Map`, kein `Set`, keine Zeichenketten im Verteilweg: Deren
 * Durchlauf-Reihenfolge ist eine Zusage der Laufzeitumgebung, keine des Spiels.
 */
export class RuleEvents {
  private readonly entries: Entry[][] = [];
  /** Empfänger je Ereignis, fertig sortiert — der heiße Pfad liest nur das. */
  private readonly receivers: RuleReceiver[][] = [];
  private nextSeq = 0;

  /** Prüfsumme über die ganze bisherige Ereignisfolge (GAME-DESIGN 18.4). */
  private hash = 0x811c9dc5;
  private emitted = 0;

  constructor() {
    for (let i = 0; i < RULE_EVENT_COUNT; i++) {
      this.entries.push([]);
      this.receivers.push([]);
    }
  }

  /**
   * Einen Empfänger anmelden.
   *
   * Läuft beim Aufbau des Kampfes, nicht im Schritt — hier darf Speicher
   * entstehen.
   *
   * @returns false, wenn das Ereignis nicht existiert.
   */
  register(registration: Registration, receiver: RuleReceiver): boolean {
    const list = this.entries[registration.on];
    if (list === undefined) return false;
    list.push({
      id: registration.id,
      order: registration.order ?? 0,
      seq: this.nextSeq++,
      receiver,
    });
    // Stabil nach Rang, bei Gleichstand nach Anmelde-Nummer. Damit verschiebt
    // ein neuer Empfänger die bestehenden **relativ zueinander** nie.
    list.sort((a, b) => a.order - b.order || a.seq - b.seq);
    const target = this.receivers[registration.on] as RuleReceiver[];
    target.length = 0;
    for (const entry of list) target.push(entry.receiver);
    return true;
  }

  /**
   * Ein Ereignis austragen — der heiße Pfad.
   *
   * Ein Feldzugriff und eine Schleife über eine zusammenhängende Liste. Es
   * entsteht nichts: keine Argument-Objekte, keine Abschlüsse, keine
   * Zwischenlisten.
   */
  emit(event: number, subject: number, value: number, timeMs: number): void {
    // Die Prüfsumme läuft **vor** den Empfängern: Sie beschreibt, was
    // ausgetragen wurde, nicht was daraus folgte. Sonst hinge sie davon ab, wer
    // gerade zuhört — und zwei Läufe mit verschiedenen Zuhörern wären nicht
    // mehr vergleichbar.
    this.hash = mixInt(this.hash, event);
    this.hash = mixInt(this.hash, subject);
    this.hash = mixFloat(this.hash, value);
    this.hash = mixInt(this.hash, timeMs);
    this.emitted++;

    const list = this.receivers[event];
    if (list === undefined) return;
    for (let i = 0; i < list.length; i++) {
      (list[i] as RuleReceiver).onRuleEvent(event, subject, value, timeMs);
    }
  }

  /** Prüfsumme über die Ereignisfolge. Der Vergleichswert für den Simulator. */
  get sequenceChecksum(): number {
    return this.hash >>> 0;
  }

  /** Wie viele Ereignisse ausgetragen wurden. */
  get emittedCount(): number {
    return this.emitted;
  }

  /** Wie viele Empfänger auf dieses Ereignis hören. */
  receiverCount(event: number): number {
    return this.receivers[event]?.length ?? 0;
  }

  /**
   * Die Empfänger dieses Ereignisses in ihrer Reihenfolge.
   *
   * Für Prüfung und Werkzeuge. Erzeugt eine Liste und gehört deshalb nicht in
   * den Kampf-Pfad.
   */
  receiverIds(event: number): string[] {
    return (this.entries[event] ?? []).map((entry) => entry.id);
  }

  /** Alle Anmeldungen aufheben und die Folge zurücksetzen. */
  reset(): void {
    for (let i = 0; i < RULE_EVENT_COUNT; i++) {
      (this.entries[i] as Entry[]).length = 0;
      (this.receivers[i] as RuleReceiver[]).length = 0;
    }
    this.nextSeq = 0;
    this.clearSequence();
  }

  /** Nur die Folge zurücksetzen, die Anmeldungen behalten. */
  clearSequence(): void {
    this.hash = 0x811c9dc5;
    this.emitted = 0;
  }
}
