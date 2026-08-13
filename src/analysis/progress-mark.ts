/**
 * Die Fortschrittsmarke (GAME-DESIGN 15.1 und 15.3, Issue #82).
 *
 * WARUM SIE DAS GEGENMITTEL GEGEN FRUST IST
 * -----------------------------------------
 * Ein Raid-Trupp scheitert hundertmal und bleibt dabei, weil **jeder Versuch
 * etwas einbringt**. Frustrierend wird es, wenn zwanzig Versuche gleich
 * aussehen. Die Marke macht den Unterschied zwischen „wieder verloren" und
 * „drei Prozent weiter als je zuvor" sichtbar — und sie tut das gerade bei
 * einer **Niederlage**, denn dort wird sie gebraucht.
 *
 * KLEINER IST BESSER
 * ------------------
 * Gemessen wird die **verbliebene** Gesundheit des Bosses in Prozent, nicht der
 * angerichtete Schaden. „Reaction down to 23 %" ist die Formulierung des
 * Designs, und sie ist die richtige: Sie sagt, wie nah der Spieler am Sieg war,
 * und sie ist bei 0 automatisch der Sieg. Ein Schadenswert dagegen hinge an der
 * Balance des Tages und wäre zwischen zwei Fassungen nicht vergleichbar.
 *
 * GETRENNT JE BOSS **UND** SCHWIERIGKEITSSTUFE
 * --------------------------------------------
 * Ein Versuch auf Normal darf die Marke auf Mythisch nicht anfassen — sonst
 * stünde dort eine Bestleistung, die auf dieser Stufe nie erbracht wurde, und
 * der Spieler hielte sich für weiter, als er ist. Der Schlüssel trägt deshalb
 * beides; es gibt keinen Weg, ihn aus einem der beiden Teile allein zu bilden.
 */

/**
 * Ein abgeschlossener Versuch, so wie er in die Marke eingeht.
 *
 * **Nicht von Hand zusammensetzen — `attemptFrom` benutzen.** Der Grund steht
 * dort.
 */
export interface Attempt {
  readonly bossId: string;
  /** Schwierigkeitsstufe als Zahl — dieselbe Zählung wie in `BestRun`. */
  readonly difficulty: number;
  /**
   * Verbliebene Gesundheit des Bosses in Prozent, 0 bis 100.
   *
   * 0 heißt besiegt. Kleiner ist besser.
   */
  readonly remainingPercent: number;
  /**
   * Ob dieser Boss das umgekehrte Ziel hat (Issue #256).
   *
   * **Die ZAHL bleibt davon unberührt** — `remainingPercent` heißt weiter
   * „kleiner ist besser", und die Marke wählt bei beiden Boss-Arten richtig.
   * Nur der SATZ wechselt: „down to 15 %" sagt bei einem Boss, dessen Reaktion
   * davonläuft, das Gegenteil dessen, was geschehen ist. Der Spieler hat den
   * Ring nicht heruntergedrückt, er hat ihn gehalten.
   */
  readonly reversed: boolean;
  readonly victory: boolean;
  /**
   * Fassung des Spiels.
   *
   * Steht dabei aus demselben Grund wie in `BestRun`: Eine Marke aus einer
   * anderen Balance ist keine vergleichbare Marke.
   */
  readonly gameVersion: string;
  /** Zeitpunkt in ms seit 1970 — der Verlauf braucht eine Reihenfolge. */
  readonly at: number;
}

/** Die beste Leistung auf einem Boss und einer Stufe. */
export interface ProgressMark {
  readonly bossId: string;
  readonly difficulty: number;
  readonly bestRemainingPercent: number;
  /** Wie viele Versuche auf dieser Stufe gezählt wurden. */
  readonly attempts: number;
  /** Ob der Boss auf dieser Stufe schon einmal fiel. */
  readonly defeated: boolean;
  readonly gameVersion: string;
  readonly at: number;
}

/** Woher der Versuch kommt, soweit es nicht in der Kurzfassung steht. */
export interface AttemptContext {
  readonly bossId: string;
  readonly difficulty: number;
  readonly gameVersion: string;
  /** Zeitpunkt in ms seit 1970. */
  readonly at: number;
}

/**
 * Einen Versuch aus der Kurzfassung des Kampfes bilden.
 *
 * DIESE FUNKTION SCHLIESST EINE FALLE.
 * `RunSummary.progressPercent` sagt, WIE WEIT der Spieler kam — größer ist
 * besser. `Attempt.remainingPercent` sagt, WIE VIEL ÜBRIG BLIEB — kleiner ist
 * besser. Beides sind Zahlen zwischen 0 und 100 mit „Percent" im Namen, und
 * beides passt widerspruchslos ineinander.
 *
 * Wer `remainingPercent: summary.progressPercent` schreibt, dreht damit die
 * **ganze** Fortschrittsmarke um: Jeder schlechtere Versuch gälte als
 * Verbesserung, und die Marke belohnte genau das Gegenteil dessen, wofür sie
 * da ist. Der Übersetzer merkt nichts, und die Zahlen sähen plausibel aus.
 *
 * Deshalb steht die Umrechnung an **einer** Stelle und nicht an jeder
 * Aufrufstelle neu.
 */
export function attemptFrom(
  summary: {
    readonly progressPercent: number;
    readonly victory: boolean;
    readonly reversed: boolean;
  },
  context: AttemptContext,
): Attempt {
  return {
    bossId: context.bossId,
    difficulty: context.difficulty,
    // Das Minus ist die ganze Umrechnung — und der Grund, warum es diese
    // Funktion gibt.
    remainingPercent: Math.max(0, 100 - summary.progressPercent),
    reversed: summary.reversed,
    victory: summary.victory,
    gameVersion: context.gameVersion,
    at: context.at,
  };
}

/**
 * Der Schlüssel aus Boss **und** Stufe.
 *
 * Als Funktion und nicht als zusammengesetzter String an der Aufrufstelle:
 * Wer ihn an zwei Stellen bildet, bildet ihn irgendwann verschieden, und dann
 * fließen zwei Stufen still in dieselbe Marke.
 */
export function markKey(bossId: string, difficulty: number): string {
  return `${bossId}\0${difficulty}`;
}

/**
 * Die Boss-Kennung aus einem Schlüssel zurück — die Umkehrung von `markKey`.
 *
 * # Warum sie hier steht und nicht beim Aufrufer
 *
 * Dieselbe Begründung wie bei `markKey` selbst, nur eine Ebene weiter: Wer den
 * Schlüssel woanders zerlegt, zerlegt ihn irgendwann anders als er gebildet
 * wurde. Genau das ist am 2026-08-11 passiert.
 *
 * `freigeschalteteInstanzen` (Issue #421c) las `defeatedBosses` und fragte
 * darin nach der **reinen** Kennung — die Liste führt aber Schlüssel aus
 * Kennung UND Stufe. Die beiden Mengen berührten sich nie:
 *
 * ```text
 *   geschrieben   markKey('boss-1-1', 1)   ->  "boss-1-1\0 1"
 *   gesucht       gefallen.has('boss-1-1')
 * ```
 *
 * Ergebnis: Keine Instanz mit `requires` wurde je freigeschaltet. Im Browser
 * gemessen — vier Bosse besiegt, Instance Two blieb unsichtbar.
 *
 * # Die Stufe fällt bewusst weg
 *
 * Für „Instanz abgeschlossen" (GAME-DESIGN 12.7) zählt, **dass** der Boss
 * gefallen ist, nicht auf welcher Stufe. Wer ihn auf mythisch bezwingt, hat
 * ihn erst recht bezwungen; eine Freischaltung, die ausgerechnet den
 * schwereren Sieg nicht zählt, wäre die Umkehrung dessen, was sie belohnen
 * soll.
 */
export function bossIdOfMarkKey(key: string): string {
  const trenner = key.indexOf('\0');
  return trenner < 0 ? key : key.slice(0, trenner);
}

/** Was ein Versuch an der Marke bewirkt hat. */
export interface MarkUpdate {
  readonly marks: readonly ProgressMark[];
  /** Ob dieser Versuch die Marke verbessert hat. */
  readonly improved: boolean;
  /** Die vorige Marke, oder `null` beim ersten Versuch auf dieser Stufe. */
  readonly previousPercent: number | null;
  /** Um wie viele Prozentpunkte es weiter ging. 0, wenn nicht verbessert. */
  readonly gainedPercent: number;
}

/**
 * Einen Versuch verbuchen.
 *
 * Gibt eine **neue** Liste zurück statt die übergebene zu ändern: Der Aufrufer
 * entscheidet, ob und wann er sie ablegt, und ein halb geschriebener
 * Spielstand kann keine Marke verfälschen.
 *
 * **Eine Niederlage zählt genauso.** Das ist der ganze Zweck (AC3): Wer den
 * Boss auf 19 % bringt und stirbt, hat die Marke von 23 % verbessert, und
 * genau das soll er sehen.
 */
export function recordAttempt(marks: readonly ProgressMark[], attempt: Attempt): MarkUpdate {
  const key = markKey(attempt.bossId, attempt.difficulty);
  const index = marks.findIndex((mark) => markKey(mark.bossId, mark.difficulty) === key);
  const previous = index >= 0 ? marks[index] : undefined;

  // Kleiner ist besser. Beim ersten Versuch gibt es nichts zu unterbieten —
  // dann IST er die Marke, ohne dass „verbessert" behauptet wird.
  const improved =
    previous !== undefined && attempt.remainingPercent < previous.bestRemainingPercent;
  const bestRemainingPercent =
    previous === undefined
      ? attempt.remainingPercent
      : Math.min(previous.bestRemainingPercent, attempt.remainingPercent);

  const updated: ProgressMark = {
    bossId: attempt.bossId,
    difficulty: attempt.difficulty,
    bestRemainingPercent,
    attempts: (previous?.attempts ?? 0) + 1,
    // Einmal besiegt bleibt besiegt. Ein späterer gescheiterter Versuch nimmt
    // den Sieg nicht zurück.
    defeated: (previous?.defeated ?? false) || attempt.victory,
    // Fassung und Zeitpunkt gehören zur MARKE, nicht zum letzten Versuch:
    // Sie sagen, wann und worunter die Bestleistung entstand.
    gameVersion: improved || previous === undefined ? attempt.gameVersion : previous.gameVersion,
    at: improved || previous === undefined ? attempt.at : previous.at,
  };

  const next = index >= 0 ? [...marks] : [...marks, updated];
  if (index >= 0) next[index] = updated;

  return {
    marks: next,
    improved,
    previousPercent: previous?.bestRemainingPercent ?? null,
    gainedPercent: improved ? (previous?.bestRemainingPercent ?? 0) - attempt.remainingPercent : 0,
  };
}

/** Die Marke für Boss und Stufe, oder `null`. */
export function markFor(
  marks: readonly ProgressMark[],
  bossId: string,
  difficulty: number,
): ProgressMark | null {
  const key = markKey(bossId, difficulty);
  return marks.find((mark) => markKey(mark.bossId, mark.difficulty) === key) ?? null;
}

/**
 * Der Satz, den der Spieler nach dem Versuch liest — englisch (Design-Gesetz 5).
 *
 * Er sagt nie, was der Spieler hätte tun sollen (GAME-DESIGN 21.2). Er sagt,
 * wo er steht.
 */
export function markSentence(update: MarkUpdate, attempt: Attempt): string {
  // **Zwei Satzbäume, EINE Zahl.** Der Wortlaut „down to 23 %" steht in
  // GAME-DESIGN 1111 — als Beispiel in einer Tabellenzelle, und die eigentliche
  // Vorgabe daneben lautet „sichtbar ein Schritt weiter, auch ohne Sieg".
  //
  // Für einen Boss mit umgekehrtem Ziel gibt es dort keinen Wortlaut. Das ist
  // eine Lücke, keine Falschumsetzung — und „down to" wäre dort wörtlich
  // verkehrt: Die Reaktion läuft davon, statt herunterzugehen. Der Spieler hat
  // gehalten, nicht gedrückt.
  //
  // **Was er GESCHAFFT hat, nicht was fehlt.** Die Marke ist das Werkzeug
  // gegen Frust (GAME-DESIGN 1105) — sie soll Erreichtes zeigen. Deshalb
  // „held 85 %" statt „15 % short", obwohl beides dieselbe Zahl meint.
  //
  // ZWEI SATZBÄUME SIND ZWEI ORTE FÜR EINE REGEL, und einer läuft weg. Ein
  // Prüffall misst deshalb BEIDE Zweige an denselben Daten
  // (`progress-mark.test.ts`) — dann kann keiner still auseinanderlaufen.
  const gehalten = (remaining: number): string => (100 - remaining).toFixed(0);
  const uebrig = (remaining: number): string => remaining.toFixed(0);

  if (attempt.reversed) {
    // Ein Sieg zeigt heute keine Marke (#187, siehe unten); der Satz steht
    // trotzdem richtig da, falls das je aufgeweicht wird.
    if (attempt.victory) return 'Contained.';
    const jetzt = gehalten(attempt.remainingPercent);
    if (update.previousPercent === null) return `Best attempt: held ${jetzt} % of the timer.`;
    if (update.improved) {
      return (
        `New best attempt: held ${jetzt} % of the timer — ` +
        `${update.gainedPercent.toFixed(0)} points further than before.`
      );
    }
    return (
      `Held ${jetzt} % of the timer. ` +
      `Best attempt stands at ${gehalten(update.previousPercent)} %.`
    );
  }

  const now = `${uebrig(attempt.remainingPercent)} %`;
  // **Dieser Satz wird heute nie angezeigt**: `markDisplayFor` gibt nach einem
  // Sieg `null` zurück, weil dort das Ergebnis steht und nicht die Marke
  // (Issue #187, AC3 und AC4). Er bleibt trotzdem stehen und richtig — eine
  // tote Zeile, die niemand pflegt, erscheint sonst irgendwann falsch.
  if (attempt.victory) return 'Defeated.';
  if (update.previousPercent === null) return `Best attempt: down to ${now}.`;
  if (update.improved) {
    return (
      `New best attempt: down to ${now} — ` +
      `${update.gainedPercent.toFixed(0)} points further than before.`
    );
  }
  return `Down to ${now}. Best attempt stands at ${uebrig(update.previousPercent)} %.`;
}
