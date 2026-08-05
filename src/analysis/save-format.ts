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

/**
 * Ein Durchlauf, wie er im Verlauf steht (Issue #82).
 *
 * DER FASSUNGSVERMERK IST BEDINGUNG, NICHT BEIWERK
 * ------------------------------------------------
 * Er steht an JEDEM Eintrag, nicht einmal am Spielstand. Der Verlauf reicht
 * über Fassungswechsel hinweg, und ohne den Vermerk an der einzelnen Zeile
 * sieht der Spieler beim Wechsel einen Sprung und schreibt ihn sich selbst zu —
 * dabei hat sich die Balance geändert, nicht sein Spiel.
 *
 * Gespeichert wird, WAS geschah, nicht was es bewirkt. `damage` ist ein
 * vergangenes Ergebnis und wirkt auf keinen kommenden Kampf zurück
 * (GAME-DESIGN 18.6); die Ausnahme dazu steht in `NAMED_EXCEPTIONS`.
 */
export interface RunHistoryEntry {
  readonly bossId: string;
  readonly difficulty: number;
  /** Zeitpunkt in ms seit 1970 — der Verlauf braucht eine Reihenfolge. */
  readonly at: number;
  /** Fassung des Spiels. Ohne sie ist der Eintrag nicht vergleichbar. */
  readonly gameVersion: string;
  readonly victory: boolean;
  readonly elapsedMs: number;
  /** Verbliebene Gesundheit des Bosses in Prozent. Kleiner ist besser. */
  readonly remainingPercent: number;
  /** Erzielter Schaden — benannte Ausnahme, siehe `NAMED_EXCEPTIONS`. */
  readonly damage: number;
  /** Leerlauf ohne bereiten Zauber in ms. */
  readonly idleMs: number;
  /** Leere Fenster mit bereitem Zauber in ms. */
  readonly missedMs: number;
}

/** Die beste Leistung je Boss und Stufe (Issue #82). */
export interface ProgressMarkEntry {
  readonly bossId: string;
  readonly difficulty: number;
  readonly bestRemainingPercent: number;
  readonly attempts: number;
  readonly defeated: boolean;
  readonly gameVersion: string;
  readonly at: number;
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
  /**
   * Welcher Zauber auf welchem Platz liegt — Platz auf **Kennung** (#264).
   *
   * **Kennung, nicht Index.** Bis #264 stand hier `Record<string, number>`,
   * und das Feld war immer leer — niemand schrieb es, deshalb ließ es sich
   * noch ändern. Ein Index ist eine berechnete Größe: die Position in einer
   * Liste. Kommt ein Zauber dazu, und es kommen noch 45, zeigt jede
   * gespeicherte Zahl auf einen anderen Zauber; der Spieler fände seine Leiste
   * vertauscht, ohne dass etwas kaputtgegangen wäre.
   *
   * Dieselbe Regel wie bei Stufe und Erfahrung (#248): gespeichert wird, was
   * der Spieler entschieden hat, nicht wo es gerade steht.
   */
  readonly bindings: Readonly<Record<string, string>>;
  readonly unlockedSpells: readonly string[];
  readonly unlockedInstances: readonly string[];
  readonly bestRuns: readonly BestRun[];
  readonly defeatedBosses: readonly string[];
  /**
   * Alle Durchlaeufe (GAME-DESIGN 15.4).
   *
   * Bewusst aufgenommen, nicht nebenbei: Der Auftraggeber hat am 2026-08-04
   * entschieden, dass der Verlauf den Schaden fuehrt -- mit Fassungsvermerk je
   * Eintrag als Bedingung.
   */
  readonly runHistory: readonly RunHistoryEntry[];
  readonly progressMarks: readonly ProgressMarkEntry[];
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
  'runHistory',
  'progressMarks',
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

/**
 * Die Stellen, an denen ein sonst verbotener Name **ausnahmsweise** erlaubt ist.
 *
 * WARUM ES DIESE LISTE GIBT UND NICHT EINE UMBENENNUNG
 * ---------------------------------------------------
 * Ein Feld `dmgValue` zu nennen, um an der Prüfung vorbeizukommen, wäre
 * schlimmer als keine Prüfung: Der Nächste, der sie liest, hält sie für
 * wirksam. Eine Ausnahme gehört sichtbar hierher, mit Begründung.
 *
 * WARUM SIE AM PFAD HÄNGT UND NICHT AM NAMEN
 * ------------------------------------------
 * Eine Ausnahme für „das Feld `damage`" würde ihn ÜBERALL erlauben — auch auf
 * einem Gegenstand, und dort ist er genau das Verbotene. Eine Ausnahme, die zu
 * breit greift, hebt den Wächter auf, ohne dass es auffällt. Deshalb nennt
 * jeder Eintrag den Elternpfad, an dem er gilt, und sonst nirgends.
 *
 * DIE UNTERSCHEIDUNG DAHINTER (GAME-DESIGN 18.6)
 * ----------------------------------------------
 *   Ein gespeicherter Werte-Bonus  wirkt auf das Spiel zurück  → verboten
 *   Ein vergangenes Ergebnis       wirkt nicht zurück          → erlaubt
 *
 * Die Regel bleibt, was sie war: Kein Spielstand friert die Balance seines
 * Tages **für kommende Kämpfe** ein.
 */
export const NAMED_EXCEPTIONS: readonly {
  /** Der Elternpfad, an dem die Ausnahme gilt. Array-Glieder als Index. */
  readonly parent: RegExp;
  readonly field: string;
  readonly why: string;
}[] = [
  {
    parent: /^runHistory\.\d+$/,
    field: 'damage',
    why:
      'Der Verlauf über alle Durchläufe zeigt die Entwicklung des Schadens ' +
      '(GAME-DESIGN 15.3, Issue #82). Ein vergangenes Ergebnis wirkt nicht auf ' +
      'kommende Kämpfe zurück. Bedingung ist der Fassungsvermerk am selben ' +
      'Eintrag — ohne ihn sieht der Spieler beim Fassungswechsel einen Sprung ' +
      'und schreibt ihn sich selbst zu.',
  },
];

function isExcepted(parent: string, field: string): boolean {
  return NAMED_EXCEPTIONS.some(
    (exception) => exception.field === field && exception.parent.test(parent),
  );
}

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
  const found = new Set<string>();
  for (const { parent, name } of collectFieldPaths(data)) {
    if (isExcepted(parent, name)) continue;
    for (const pattern of FORBIDDEN_NAME_PATTERNS) {
      if (pattern.test(name)) {
        found.add(name);
        break;
      }
    }
  }
  return [...found].sort();
}

/**
 * Jedes Feld mit dem Pfad, unter dem es steht.
 *
 * Getrennt von `collectFieldNames`, weil die Ausnahmen den Pfad brauchen und
 * eine reine Namensliste ihn nicht mehr hergibt: In ihr sieht `damage` auf
 * einem Gegenstand genauso aus wie `damage` an einem Verlaufs-Eintrag.
 */
export function collectFieldPaths(
  value: unknown,
  parent = '',
  into: { parent: string; name: string }[] = [],
): { parent: string; name: string }[] {
  if (Array.isArray(value)) {
    value.forEach((item, index) => {
      collectFieldPaths(item, parent === '' ? String(index) : `${parent}.${index}`, into);
    });
    return into;
  }
  if (value !== null && typeof value === 'object') {
    for (const [key, inner] of Object.entries(value)) {
      into.push({ parent, name: key });
      collectFieldPaths(inner, parent === '' ? key : `${parent}.${key}`, into);
    }
  }
  return into;
}
