/**
 * SUBLIMATION — Verkürzung aller Abklingzeiten (GAME-DESIGN 4.1, Issue E3-3).
 *
 * > Sublimations-Zauber verkürzen laufend **alle** Abklingzeiten um einen
 * > kleinen Betrag. Die Schule für Könner.
 *
 * Sie wirkt nicht auf den Boss, sondern auf das **Rotations-System selbst** —
 * deshalb ist sie ohne Boss-Ablauf baubar, anders als FIXATION (Issue #100).
 *
 * **Der Betrag steht in den Daten** (`cooldownReductionMs`), nicht im Code.
 * Ein Zauber ohne das Feld verkürzt nichts; das gilt für alle Zauber der
 * Kalzination und ist der Grund, warum die Referenz-Rechnung unberührt bleibt.
 *
 * # Zwei Festlegungen, die das Design offenlässt
 *
 * Beide sind hier als Umsetzung getroffen, nicht als Design entschieden, und
 * beide sind in einer Zeile umzudrehen — sie stehen ausdrücklich in der Meldung
 * zum Issue.
 *
 * **1. Der wirkende Zauber verkürzt seine EIGENE Abklingzeit nicht.**
 * Täte er es, wäre der Zauber schlicht einer mit kürzerer Abklingzeit — die
 * Mechanik wäre an dieser Stelle eine umständliche Schreibweise für
 * `cooldownMs − cooldownReductionMs`. Und es widerspräche der Reihenfolge (B)
 * aus E3-1b: Ein Empfänger beeinflusst den nächsten Zauber, nicht den
 * laufenden.
 *
 * **2. Die globale Wartezeit ist keine Abklingzeit.** Das Design spricht von
 * „Abklingzeiten"; die Wartezeit ist eine eigene Größe mit eigener Anzeige
 * (GAME-DESIGN 9.4) und eigenem Talent-Pfad (6.4). Sie bleibt unberührt.
 */
import { RULE_SPELL_CAST, type Registration, type RuleReceiver } from '../events.js';
import type { Combat } from '../../core/combat.js';

/** Wohin die Verkürzung greift. Bewusst nur diese beiden Zugriffe. */
export interface CooldownSink {
  /** Was ein Zauber laut seinen Daten verkürzt. 0, wenn er nichts verkürzt. */
  reductionOfSpell(spellIndex: number): number;
  /**
   * Alle laufenden Abklingzeiten um `amount` ms verkürzen.
   *
   * @param exceptIndex Dieser Zauber bleibt außen vor.
   * @returns wie viele Abklingzeiten tatsächlich verkürzt wurden
   */
  shortenCooldowns(amount: number, timeMs: number, exceptIndex: number): number;
}

/**
 * Der Empfänger, der die Abklingzeiten verkürzt.
 *
 * **Er verändert den Zustand** — wie die Stapel-Mechanik, aber an einer
 * empfindlicheren Stelle: Abklingzeiten bestimmen, was als Nächstes gewirkt
 * werden darf. Ein Fehler hier verschiebt die ganze Rotation.
 */
export class SublimationMechanic implements RuleReceiver {
  /** Wie oft verkürzt wurde, und um wie viel insgesamt. Für die Prüfung. */
  private applications = 0;
  private shortenedTotal = 0;

  constructor(private readonly sink: CooldownSink) {}

  get applicationCount(): number {
    return this.applications;
  }

  /** Summe der verkürzten Abklingzeiten in ms. */
  get shortenedCount(): number {
    return this.shortenedTotal;
  }

  /** Die Anmeldung dieser Mechanik, als Datensatz. */
  registrations(id: string): Registration[] {
    // Rang 10: nach der Stapel-Mechanik (Rang 0). Wer eine Reihenfolge
    // braucht, bekommt sie aus der Zahl und nicht aus dem Zufall der
    // Anmeldung — auch wenn sich die beiden heute nicht ins Gehege kommen.
    return [{ id, on: RULE_SPELL_CAST, order: 10 }];
  }

  onRuleEvent(_event: number, subject: number, _value: number, timeMs: number): void {
    if (subject < 0) return;
    const amount = this.sink.reductionOfSpell(subject);
    if (amount <= 0) return;
    this.applications++;
    this.shortenedTotal += this.sink.shortenCooldowns(amount, timeMs, subject);
  }
}

/**
 * Die Mechanik eines Kampfes anmelden.
 *
 * Läuft beim Aufbau, nicht im Takt. Sie meldet sich auch dann an, wenn kein
 * Zauber etwas verkürzt — der Empfänger kostet dann einen Vergleich je Wirkung
 * und tut nichts. Ihn von der Zusammenstellung abhängig zu machen hieße, die
 * Empfänger-Liste vom Spielstand abhängig zu machen, und damit die Reihenfolge.
 */
export function registerSublimation(combat: Combat): SublimationMechanic {
  const mechanic = new SublimationMechanic(combat);
  for (const registration of mechanic.registrations('sublimation')) {
    combat.events.register(registration, mechanic);
  }
  return mechanic;
}
