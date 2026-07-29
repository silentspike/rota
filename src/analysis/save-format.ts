/**
 * Was im Spielstand stehen darf — und was niemals (Issue #83).
 *
 * DER FALLSTRICK, GEGEN DEN DIESE DATEI GEBAUT IST
 * ------------------------------------------------
 * "Nur was besessen und erreicht wurde, nie berechnete Werte" klingt
 * selbstverstaendlich und ist es nicht. Sobald irgendwo ein Schadenswert, eine
 * Gegenstandsstufe oder eine Talentwirkung mitgespeichert wird, friert der
 * Spielstand die Balance des Tages ein, an dem er entstand. Eine spaetere
 * Balance-Aenderung erreicht diesen Spieler dann nie — und niemand merkt es,
 * weil der Stand ja laedt.
 *
 * Deshalb ist die Liste der erlaubten Felder hier ABSCHLIESSEND und wird
 * mechanisch geprueft (tests/unit/save-format.test.ts). Ein neues Feld
 * aufzunehmen ist eine bewusste Entscheidung, kein Nebeneffekt.
 *
 * Die Faustregel: Gespeichert wird, WAS der Spieler hat oder erreicht hat.
 * Was es BEWIRKT, steht in den Daten unter data/ und wird bei jedem Laden neu
 * gelesen.
 *
 *   erlaubt      "Gegenstand X liegt auf Platz 3", "Boss Y auf Stufe 2 besiegt",
 *                "Talentpunkt in Zweig Z", "Bestzeit 184,2 s"
 *   verboten     "Gegenstand X gibt +40 ESSENCE", "Zauber A macht 1200 Schaden",
 *                "Talent Z erhoeht um 5 %"
 */

/** Fassung des Speicherformats. Steigt nur, wenn sich die Struktur aendert. */
export const SAVE_SCHEMA = 1;

/**
 * Ein erzielter Bestlauf.
 *
 * Die Fassung des Spiels steht dabei (Umfang von #83): Eine Bestzeit aus einer
 * anderen Balance ist keine vergleichbare Bestzeit, und ohne den Vermerk
 * verglichen man spaeter Aepfel mit Birnen.
 */
export interface BestRun {
  readonly bossId: string;
  readonly difficulty: number;
  /** Die einzige Zahl, die zaehlt — erreicht, nicht gerechnet. */
  readonly timeMs: number;
  /** Fassung des Spiels, unter der sie erzielt wurde. */
  readonly gameVersion: string;
  /** Zeitpunkt in ms seit 1970, damit der Verlauf eine Reihenfolge hat. */
  readonly achievedAt: number;
}

/** Ein Gegenstand im Besitz — WAS er ist, nicht was er bewirkt. */
export interface OwnedItem {
  readonly itemId: string;
  /** Gegenstandsstufe als ERREICHTE Aufwertung, nicht als Wirkung. */
  readonly upgradeSteps: number;
  /** Auf welchem Ausruestungsplatz er liegt, oder null im Beutel. */
  readonly equippedSlot: number | null;
}

export interface SaveData {
  readonly schema: number;
  readonly gameVersion: string;
  readonly savedAt: number;
  readonly level: number;
  readonly experience: number;
  /** Ausgegebene Talentpunkte je Zweig — die Wahl, nicht ihre Wirkung. */
  readonly talentPoints: Readonly<Record<string, number>>;
  readonly items: readonly OwnedItem[];
  /** Welcher Zauber auf welchem Platz liegt. */
  readonly bindings: Readonly<Record<string, number>>;
  readonly unlockedSpells: readonly string[];
  readonly unlockedInstances: readonly string[];
  readonly bestRuns: readonly BestRun[];
  readonly defeatedBosses: readonly string[];
}

/**
 * Die abschliessende Liste der erlaubten Felder auf oberster Ebene.
 *
 * Sie ist Pruefgegenstand, nicht Dokumentation: Der Test vergleicht sie mit
 * dem, was tatsaechlich geschrieben wird, und schlaegt bei jeder Abweichung an.
 */
export const ALLOWED_TOP_LEVEL: readonly string[] = [
  'schema',
  'gameVersion',
  'savedAt',
  'level',
  'experience',
  'talentPoints',
  'items',
  'bindings',
  'unlockedSpells',
  'unlockedInstances',
  'bestRuns',
  'defeatedBosses',
];

/**
 * Feldnamen, die auf einen berechneten Wert hindeuten.
 *
 * Bewusst grob: Lieber ein Fehlalarm, der eine Umbenennung erzwingt, als ein
 * eingefrorener Balance-Wert, den erst in einem Jahr jemand bemerkt.
 */
export const FORBIDDEN_NAME_PATTERNS: readonly RegExp[] = [
  /damage/i,
  /\bdps\b/i,
  /power/i,
  /bonus/i,
  /multiplier/i,
  /modifier/i,
  /\bstat/i,
  /essence|vessel|ward|catalysis|purity|yield/i,
  /cooldown/i,
  /score(?!board)/i,
];

/** Alle Feldnamen eines Objektbaums, rekursiv. */
export function collectFieldNames(value: unknown, into: Set<string> = new Set()): Set<string> {
  if (Array.isArray(value)) {
    for (const item of value) collectFieldNames(item, into);
    return into;
  }
  if (value !== null && typeof value === 'object') {
    for (const [key, inner] of Object.entries(value)) {
      into.add(key);
      collectFieldNames(inner, into);
    }
  }
  return into;
}

/**
 * Prueft einen Spielstand gegen die Regel "nichts Berechnetes".
 *
 * Laeuft nicht nur im Test, sondern auch beim Schreiben: Ein Stand, der die
 * Regel verletzt, wird gar nicht erst abgelegt.
 */
export function findForbiddenFields(data: unknown): string[] {
  const found: string[] = [];
  for (const name of collectFieldNames(data)) {
    for (const pattern of FORBIDDEN_NAME_PATTERNS) {
      if (pattern.test(name)) {
        found.push(name);
        break;
      }
    }
  }
  return found.sort();
}
