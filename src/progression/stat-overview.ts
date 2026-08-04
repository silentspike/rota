/**
 * Die Werte-Übersicht (GAME-DESIGN 6.7, Issue #188).
 *
 * WARUM ES SIE GIBT
 * -----------------
 * > **Ein Deckel, den man nicht sieht, ist eine Falle:** Wer weiter in Tempo
 * > investiert, ohne es zu merken, verliert Ausrüstungsplätze an einen Wert,
 * > der nichts mehr bringt.
 *
 * Die Werte kommen seit Issue #177 im Kampf an — auf dem Bildschirm erschienen
 * sie nie. Der Spieler kann heute nicht sehen, wie viel Tempo er hat, und erst
 * recht nicht, ob er darüber hinausgeschossen ist.
 *
 * DER DECKEL IST EIN DATENWERT UND `null` HEISST „ES GIBT KEINEN"
 * --------------------------------------------------------------
 * PURITY ist bei 100 % gedeckelt — das ist keine Balance-Entscheidung, sondern
 * eine Wahrscheinlichkeit, die nicht über eins gehen kann. Der Deckel für
 * CATALYSIS wird dagegen erst **gemessen** (Issue #118); bis dahin gibt es
 * keinen, und „2 % über dem Limit" ohne Limit ist nicht darstellbar.
 *
 * Deshalb trägt jeder Wert seinen Deckel als `number | null` **mit sich**.
 * Kommt die Messung, wird eine Zahl eingetragen — es muss keine
 * Fallunterscheidung im Anzeigecode angefasst werden. Ein `if (name ===
 * 'catalysis')` an der Anzeige wäre genau die Stelle, die man beim Nachziehen
 * übersieht.
 *
 * SIE RECHNET, SIE ZEIGT NICHT
 * ----------------------------
 * Kein DOM, kein Text mit Auszeichnung. Was hier entsteht, ist eine Liste von
 * Zeilen mit Zahlen und einem fertigen Warnsatz — die Ansicht setzt sie nur.
 * Der Grund ist derselbe wie bei der Fortschrittsmarke: Zwei Stellen, die aus
 * denselben Zahlen einen Satz machen, laufen beim nächsten Nachziehen
 * auseinander.
 */
import { CRIT_BASE_PERMILLE, type WornValues } from '../gear/worn.js';

/** Wie ein Wert geschrieben wird. */
export type StatUnit = 'percent' | 'flat';

/** Eine Zeile der Übersicht. */
export interface StatRow {
  /** Kennung, kleingeschrieben — für Prüfungen und Merkmale. */
  readonly id: string;
  /** Beschriftung im Spiel, englisch (Design-Gesetz 5). */
  readonly label: string;
  /** Der Wert in seiner Einheit: Prozent oder Rohzahl. */
  readonly value: number;
  readonly unit: StatUnit;
  /**
   * Der Deckel in derselben Einheit, oder `null`.
   *
   * `null` heißt **es gibt keinen** — nicht „unbekannt" und nicht „unendlich".
   * Für CATALYSIS steht hier eine Zahl, sobald Issue #118 sie gemessen hat.
   */
  readonly cap: number | null;
  /**
   * Der Überschuss über dem Deckel, oder `null`.
   *
   * `null`, wenn es keinen Deckel gibt **oder** der Wert darunter liegt. Beide
   * Fälle bedeuten dasselbe für die Anzeige: keine Warnung.
   */
  readonly overCap: number | null;
  /** Der Warnsatz, oder `null`. Fertig formuliert (GAME-DESIGN 6.7). */
  readonly warning: string | null;
}

/**
 * Die Deckel, wie sie heute bekannt sind.
 *
 * **Diese Tabelle ist der Ort, an dem Issue #118 sein Ergebnis einträgt.**
 * `catalysis: null` ist kein vergessener Eintrag, sondern die Aussage „der
 * Deckel ist noch nicht gemessen".
 */
export const STAT_CAPS: Readonly<Record<string, number | null>> = {
  essence: null,
  vessel: null,
  ward: null,
  // Der Deckel wird gemessen (Issue #118). Bis dahin: kein Deckel, keine
  // Warnung, nur der Wert.
  catalysis: null,
  // Eine Wahrscheinlichkeit kann nicht über eins gehen — das ist keine
  // Balance-Entscheidung und wird sich nicht ändern.
  purity: 100,
  yield: null,
};

/** Die sechs Werte in der Reihenfolge des Designs (GAME-DESIGN 6). */
export const STAT_ORDER: readonly string[] = [
  'essence',
  'vessel',
  'ward',
  'catalysis',
  'purity',
  'yield',
];

const LABELS: Readonly<Record<string, string>> = {
  essence: 'ESSENCE',
  vessel: 'VESSEL',
  ward: 'WARD',
  catalysis: 'CATALYSIS',
  purity: 'PURITY',
  yield: 'YIELD',
};

const UNITS: Readonly<Record<string, StatUnit>> = {
  essence: 'flat',
  vessel: 'flat',
  ward: 'flat',
  catalysis: 'percent',
  purity: 'percent',
  yield: 'percent',
};

/** Die drei Grundwerte, wie sie ohne jede Ausrüstung gelten. */
export interface BaseStats {
  readonly essence: number;
  readonly vessel: number;
  readonly ward: number;
}

/**
 * Eine Zeile bauen.
 *
 * Der Warnsatz entsteht **hier** und nicht in der Anzeige, und er entsteht nur,
 * wenn es einen Deckel gibt UND der Wert darüber liegt. Ohne Deckel keine
 * Warnung — das ist der Fall CATALYSIS bis Issue #118.
 */
function row(id: string, value: number): StatRow {
  const cap = STAT_CAPS[id] ?? null;
  const unit = UNITS[id] ?? 'flat';
  const over = cap !== null && value > cap ? value - cap : null;
  return {
    id,
    label: LABELS[id] ?? id.toUpperCase(),
    value,
    unit,
    cap,
    overCap: over,
    warning:
      over === null
        ? null
        : // Der Wortlaut aus GAME-DESIGN 6.7: Zahl plus Hinweis, kein Balken.
          `${formatValue(over, unit)} over the limit, no effect`,
  };
}

/** Eine Zahl in ihrer Einheit — eine Stelle, damit Wert und Deckel gleich aussehen. */
export function formatValue(value: number, unit: StatUnit): string {
  return unit === 'percent' ? `${value.toFixed(0)} %` : String(Math.round(value));
}

/**
 * Die Übersicht aus Grundwerten und getragener Ausrüstung.
 *
 * `worn` darf `NOTHING_WORN` sein — ein neuer Spielstand hat keine Ausrüstung,
 * und die Übersicht zeigt dann die **Grundwerte**, nicht nichts (AC5). Eine
 * leere Übersicht sähe aus wie ein Fehler.
 */
export function statOverview(base: BaseStats, worn: WornValues): StatRow[] {
  const werte: Readonly<Record<string, number>> = {
    essence: base.essence,
    vessel: base.vessel,
    ward: base.ward,
    // Promille im Kampf, Prozent auf dem Bildschirm. Die Umrechnung steht an
    // einer Stelle, nicht in jeder Zeile.
    catalysis: worn.catalysis / 10,
    purity: worn.purity / 10,
    // YIELD zeigt den GESAMTEN Kritschaden, nicht nur den Zuschlag: Der
    // Spieler vergleicht ihn mit dem, was ein Treffer austeilt, und der
    // Grundwert von 150 % gehört dazu.
    yield: (CRIT_BASE_PERMILLE + worn.yieldBonus) / 10,
  };
  return STAT_ORDER.map((id) => row(id, werte[id] ?? 0));
}
