/**
 * Verstärkungen und Synergie-Fenster (GAME-DESIGN 5, Issue E3-5).
 *
 * Zwei Regeln, beide aus dem Design, beide hier und nur hier:
 *
 * | Regel                  | Festlegung                                          |
 * | ---------------------- | --------------------------------------------------- |
 * | **Stapel-Regel**       | Verschiedene Bonus-Arten stapeln **multiplikativ**, gleiche Arten **ersetzen** einander |
 * | **Synergie-Fenster**   | **In globalen Wartezeiten gerechnet, nicht in Sekunden** |
 *
 * # Warum das Fenster in Wartezeiten zählt und nicht in Sekunden
 *
 * > „Sonst verschiebt sich das Verhältnis bei hohem Tempo" (GAME-DESIGN 5).
 *
 * Ein Fenster von 4,5 Sekunden ist bei 1,5 s Grundtakt Platz für drei Zauber.
 * Sinkt die Wartezeit per Talent auf 1,0 s, wären es plötzlich viereinhalb —
 * dasselbe Fenster würde eine andere Rotation erlauben, ohne dass jemand es
 * entschieden hätte. In Wartezeiten gerechnet bleibt „drei Zauber" drei Zauber,
 * und hohes Tempo ändert die Rotation **strukturell** statt sie nur zu
 * beschleunigen.
 *
 * # Warum gleiche Arten ersetzen statt zu stapeln
 *
 * Sonst ließe sich derselbe Bonus stumpf mehrfach anlegen. Das Zusammenlegen
 * verschiedener großer Fähigkeiten zu einem Fenster soll die Kernfertigkeit
 * sein („Cooldown-Alignment") — nicht das Wiederholen einer einzigen.
 *
 * # Was hier NICHT steht
 *
 * **Welche Bonus-Arten es gibt.** Das Design verlangt, dass die Kategorien im
 * Hilfe-System erklärt werden (GAME-DESIGN 5, 17.3), nennt sie aber nicht. Eine
 * Art ist deshalb schlicht eine Nummer; was sie bedeutet, entscheiden die Daten,
 * wenn die Schulen stehen. Die Regel ist ohne den Katalog vollständig.
 */

/**
 * Wie viele Bonus-Arten gleichzeitig geführt werden können.
 *
 * Vorab angelegt, damit im Takt nichts entsteht (GAME-DESIGN 18.2). Acht deckt
 * die fünf Schulen samt Sonderzaubern ab; wird es enger, meldet
 * [`Buffs.apply`] es, statt still zu verwerfen.
 */
export const BUFF_KIND_CAPACITY = 8;

/** Keine Verstärkung dieser Art. */
const NOT_ACTIVE = -1;

/**
 * Die laufenden Verstärkungen.
 *
 * Je Art höchstens eine — das ist die Stapel-Regel, in der Datenstruktur
 * ausgedrückt statt in einer Prüfung: Zwei gleiche Arten können gar nicht
 * nebeneinander stehen.
 */
export class Buffs {
  /** Faktor je Art. 1 heißt: keine Verstärkung. */
  private readonly factors = new Float64Array(BUFF_KIND_CAPACITY).fill(1);
  /** Ablauf je Art, oder `NOT_ACTIVE`. */
  private readonly expiresAt = new Float64Array(BUFF_KIND_CAPACITY).fill(NOT_ACTIVE);
  /** Wie oft eine Art ersetzt wurde, statt danebenzustehen. Für die Prüfung. */
  private replacements = 0;
  /** Abgelaufene Verstärkungen seit dem letzten Zurücksetzen. */
  private expirations = 0;

  get replacementCount(): number {
    return this.replacements;
  }

  get expirationCount(): number {
    return this.expirations;
  }

  /**
   * Eine Verstärkung anlegen, deren Fenster in **globalen Wartezeiten** zählt.
   *
   * @param gcds Länge des Fensters in Wartezeiten. Das Design nennt zwei bis
   *   drei (GAME-DESIGN 5); die Zahl selbst kommt aus den Daten.
   * @param gcdLengthMs Die **aktuelle** Wartezeit. Sinkt sie per Talent, wird
   *   das Fenster in Sekunden kürzer — genau das ist der Zweck.
   * @returns false, wenn die Art außerhalb des Vorrats liegt.
   */
  applyForGcds(
    kind: number,
    factor: number,
    gcds: number,
    nowMs: number,
    gcdLengthMs: number,
  ): boolean {
    return this.apply(kind, factor, nowMs + gcds * gcdLengthMs, nowMs);
  }

  /**
   * Eine Verstärkung mit ausdrücklichem Ablauf anlegen.
   *
   * **Gleiche Art ersetzt.** Auch dann, wenn die neue schwächer ist oder früher
   * abläuft: „ersetzen einander" steht ohne Einschränkung im Design, und eine
   * Regel „die stärkere gewinnt" wäre eine zweite, die dort nicht steht.
   */
  apply(kind: number, factor: number, expiresAtMs: number, nowMs: number): boolean {
    if (!Number.isInteger(kind) || kind < 0 || kind >= BUFF_KIND_CAPACITY) return false;
    if (this.isActive(kind, nowMs)) this.replacements++;
    this.factors[kind] = factor;
    this.expiresAt[kind] = expiresAtMs;
    return true;
  }

  /** Läuft diese Art gerade? */
  isActive(kind: number, nowMs: number): boolean {
    const until = this.expiresAt[kind] ?? NOT_ACTIVE;
    return until !== NOT_ACTIVE && until > nowMs;
  }

  /** Restzeit dieser Art in ms, 0 wenn sie nicht läuft. */
  remainingMs(kind: number, nowMs: number): number {
    if (!this.isActive(kind, nowMs)) return 0;
    return (this.expiresAt[kind] ?? nowMs) - nowMs;
  }

  /**
   * Der Faktor aller laufenden Verstärkungen — **multiplikativ**.
   *
   * Die Reihenfolge ist die der Arten und damit fest: Fließkomma-Multiplikation
   * ist nicht assoziativ, und beide Umsetzungen müssen auf denselben Wert
   * kommen (GAME-DESIGN 18.4).
   */
  factor(nowMs: number): number {
    let product = 1;
    for (let kind = 0; kind < BUFF_KIND_CAPACITY; kind++) {
      if (!this.isActive(kind, nowMs)) continue;
      product *= this.factors[kind] as number;
    }
    return product;
  }

  /** Wie viele Arten gerade laufen. */
  activeCount(nowMs: number): number {
    let count = 0;
    for (let kind = 0; kind < BUFF_KIND_CAPACITY; kind++) {
      if (this.isActive(kind, nowMs)) count++;
    }
    return count;
  }

  /**
   * Abgelaufene Verstärkungen einsammeln.
   *
   * Läuft je Takt und darf deshalb nichts erzeugen. Wer wissen will, **welche**
   * abgelaufen ist, fragt danach — der Rückgabewert ist eine Zahl, kein Objekt.
   *
   * @returns wie viele Arten in diesem Aufruf abgelaufen sind
   */
  expire(nowMs: number): number {
    let ended = 0;
    for (let kind = 0; kind < BUFF_KIND_CAPACITY; kind++) {
      const until = this.expiresAt[kind] ?? NOT_ACTIVE;
      if (until === NOT_ACTIVE || until > nowMs) continue;
      this.expiresAt[kind] = NOT_ACTIVE;
      this.factors[kind] = 1;
      ended++;
      this.expirations++;
    }
    return ended;
  }

  /** Die zuletzt abgelaufene Art, oder -1 — für den Auslöser `buff.expired`. */
  lastExpiredKind(nowMs: number): number {
    for (let kind = 0; kind < BUFF_KIND_CAPACITY; kind++) {
      const until = this.expiresAt[kind] ?? NOT_ACTIVE;
      if (until !== NOT_ACTIVE && until <= nowMs) return kind;
    }
    return -1;
  }

  reset(): void {
    this.factors.fill(1);
    this.expiresAt.fill(NOT_ACTIVE);
    this.replacements = 0;
    this.expirations = 0;
  }
}
