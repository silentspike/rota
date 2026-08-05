/**
 * Die Stapel-Mechanik der Schulen (GAME-DESIGN 4.1, 4.2).
 *
 * Zwei der fünf Schulen sammeln Stapel, und sie tun es nach derselben Regel —
 * deshalb steht sie einmal hier und nicht zweimal im Kampfcode:
 *
 * | Schule           | Stapel               | Wozu                              |
 * | ---------------- | -------------------- | --------------------------------- |
 * | **CALCINATION**  | Hitze                | wird vom Entlade-Zauber verbraucht |
 * | **PUTREFACTION** | Zersetzungs-Stapel   | erhöhen den Schaden **aller** Quellen |
 *
 * **Der Gewinn kommt aus den Daten, nicht aus dem Code.** Ein Zauber bringt in
 * seinem Datensatz mit, wie viel er beisteuert (`resourceGain`,
 * `resourceGainPerTick`); dieser Empfänger trägt es ein. Ein neuer Zauber ist
 * damit weiterhin ein Datensatz (Leitplanke 6) — und die Schul-Mechanik ist
 * kein Sonderfall im Kampf mehr, sondern hört auf Ereignisse wie jeder andere
 * Effekt auch (Issue E3-2).
 *
 * **Was hier NICHT steht:** der Verfall. Er hängt an der Uhr, nicht an einem
 * Ereignis — im Spiel ein Schritt je Takt, im Simulator ein eingeplantes
 * Ereignis (`EventKind::ResourceDecay`). Ihn hierher zu ziehen hieße, eine
 * Zeitregel als Reaktion zu verkleiden.
 */
import { RULE_DOT_TICK, RULE_SPELL_CAST, type Registration, type RuleReceiver } from '../events.js';
import type { Combat } from '../../core/combat.js';

/**
 * Wohin der Empfänger den Gewinn einträgt.
 *
 * Bewusst nur dieser eine Zugriff statt der ganzen `Combat`: Die Mechanik soll
 * Stapel füllen können und sonst nichts.
 */
export interface StackSink {
  /** Kennung des Vorrats, den diese Schule führt. */
  readonly resourceId: string;
  /** Was ein Zauber bei einer Wirkung beisteuert. */
  gainOfSpell(spellIndex: number, resourceId: string): number;
  /** Was ein Zauber je Tick seines Effekts über Zeit beisteuert. */
  gainOfTick(spellIndex: number, resourceId: string): number;
  /** Stapel hinzufügen. Deckelt selbst und zählt das Verpuffte mit. */
  gainResource(amount: number): void;
}

/**
 * Der Empfänger, der Stapel sammelt.
 *
 * **Er verändert den Zustand** — anders als alles, was bis E3-1b am
 * Ereignis-System hing. Deshalb steht die Reihenfolge-Frage hier zum ersten
 * Mal scharf: Er läuft auf `spell.cast`, und das Ereignis wird ausgetragen,
 * **nachdem** die Wirkung vollständig abgerechnet ist. Ein Entlade-Zauber
 * liest den Vorrat also, bevor sein eigener Gewinn ihn erhöhen kann — dieselbe
 * Regel, die `Fight::cast` im Simulator seit E2 festhält.
 */
export class StackMechanic implements RuleReceiver {
  /** Wie oft Stapel dazugekommen sind. Für die Prüfung. */
  private gains = 0;

  /**
   * Die Zähler eines neuen Versuchs (Issue #349).
   *
   * Sie hängen am Kampf und müssen mit ihm zurück. Gefunden hat es der
   * Wächter in `tests/unit/reset-completeness.test.ts`, nicht ein Gedanke:
   * Er vergleicht den zurückgesetzten Kampf gegen einen frisch gebauten und
   * geht dabei auch eine Ebene tiefer.
   */
  reset(): void {
    this.gains = 0;
  }

  constructor(private readonly sink: StackSink) {}

  get gainCount(): number {
    return this.gains;
  }

  /** Die beiden Anmeldungen dieser Mechanik, als Daten. */
  registrations(id: string): Registration[] {
    return [
      { id, on: RULE_SPELL_CAST },
      { id, on: RULE_DOT_TICK },
    ];
  }

  onRuleEvent(event: number, subject: number, _value: number, _timeMs: number): void {
    if (subject < 0) return;
    const amount =
      event === RULE_SPELL_CAST
        ? this.sink.gainOfSpell(subject, this.sink.resourceId)
        : this.sink.gainOfTick(subject, this.sink.resourceId);
    if (amount === 0) return;
    this.gains++;
    this.sink.gainResource(amount);
  }
}

/**
 * Die Schul-Mechanik eines Kampfes anmelden.
 *
 * Eine Zeile für alle Schulen: Welcher Vorrat gefüllt wird und wie viel,
 * steht in den Daten. Läuft beim Aufbau, nicht im Takt.
 */
export function registerStackMechanic(combat: Combat): StackMechanic {
  const mechanic = new StackMechanic(combat);
  for (const registration of mechanic.registrations(`stacks:${combat.school.resource.id}`)) {
    combat.events.register(registration, mechanic);
  }
  return mechanic;
}
