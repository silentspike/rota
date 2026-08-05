/**
 * Welcher Zauber auf welchem Platz liegt (GAME-DESIGN 14.4; Issue #264).
 *
 * > **Leiste frei anordenbar.** Der Spieler bestimmt, welcher Zauber auf
 * > welchem Platz liegt. **Muskelgedächtnis entsteht nur, wenn ein Zauber
 * > immer an derselben Stelle liegt** — Grundvoraussetzung für ein
 * > Rotations-Spiel.
 *
 * KENNUNGEN, NICHT INDIZES
 * ------------------------
 * Der Spielstand führte `bindings` als `Record<string, number>` — Platz auf
 * Zauber-INDEX. Das Feld war bisher immer leer, niemand schrieb es, und
 * deshalb ließ es sich noch ändern.
 *
 * Ein Index ist eine **berechnete** Größe: die Position in einer Liste. Kommt
 * ein Zauber dazu — und es kommen noch 45 —, zeigt jede gespeicherte Zahl auf
 * einen anderen Zauber. Der Spieler fände seine Leiste vertauscht, ohne dass
 * etwas kaputtgegangen wäre. Das ist genau die Regel aus AC5 von #248, nur an
 * einer anderen Stelle: **gespeichert wird, was der Spieler entschieden hat**,
 * nicht wo es gerade steht.
 *
 * Die Vergleichsdateien der Parität führen die Belegung längst über `spellId`
 * — die Form war also schon da, nur nicht im Spielstand.
 *
 * ZWEI BEDINGUNGEN, ZWEI GRÜNDE
 * -----------------------------
 * Ein Zauber muss freigeschaltet sein (seine Stufe erreicht), und ein Platz
 * muss offen sein (genug Plätze auf dieser Stufe). Beides prüft `canBind` —
 * getrennt, weil die Meldungen an den Spieler verschieden sind: „noch nicht
 * gelernt" und „noch kein Platz dafür" sind zwei Sachen.
 */
import type { SpellDef } from '../core/data.js';
import { alsSpellIndex } from '../core/combat.js';
import { isSpellUnlocked, barSlotsAtLevel } from './unlocks.js';

/** Die Belegung: Platz als Text auf Zauber-Kennung. */
export type Bindings = Readonly<Record<string, string>>;

/** Warum eine Belegung nicht geht, oder `null`, wenn sie geht. */
export type BindProblem = 'spell-locked' | 'slot-locked' | 'unknown-spell' | 'already-bound' | null;

/**
 * Ob dieser Zauber auf diesen Platz darf.
 *
 * **Getrennte Gründe statt eines `boolean`.** Der Spieler soll lesen können,
 * WARUM es nicht geht; ein `false` ließe die Anzeige raten, und sie riete beim
 * nächsten Umbau anders.
 */
export function bindProblem(
  spellId: string,
  slot: number,
  level: number,
  spells: readonly SpellDef[],
  /**
   * Die bestehende Belegung — **Pflicht, nicht optional** (Issue #287).
   *
   * Optional wäre dasselbe Muster, das bei #306 die Aufschlüsselung kaputt
   * hielt: Die Sperre schwiege genau dann, wenn ein künftiger Aufrufer sie zu
   * reichen vergisst. Ein leeres Objekt ist ein bewusster Aufrufer, ein
   * fehlender Parameter ein vergesslicher — der Unterschied ist der ganze
   * Zweck.
   */
  bindings: Bindings,
): BindProblem {
  const spell = spells.find((s) => s.id === spellId);
  if (spell === undefined) return 'unknown-spell';
  if (!isSpellUnlocked(spell, level)) return 'spell-locked';
  if (!Number.isInteger(slot) || slot < 0 || slot >= barSlotsAtLevel(level)) return 'slot-locked';
  // **Ein Zauber liegt auf höchstens einem Platz** (GAME-DESIGN 715, Issue
  // #287 AC3). Zwei Plätze mit demselben Zauber sind keine zwei Einzahler —
  // der zweite gibt nur einen zweiten Knopf für dieselbe Quelle, und die
  // Vorschau behauptete eine Vielfalt, die es nicht gibt.
  //
  // **Derselbe Platz ist kein Problem.** Wer einen Zauber dorthin legt, wo er
  // schon liegt, ändert nichts — das als Fehler zu melden hieße, dem Spieler
  // eine Sperre zu zeigen, die keine ist.
  for (const [platz, gelegt] of Object.entries(bindings)) {
    if (gelegt === spellId && Number.parseInt(platz, 10) !== slot) return 'already-bound';
  }
  return null;
}

/** Kurzform von `bindProblem` für den Fall, dass nur das Ob zählt. */
export function canBind(
  spellId: string,
  slot: number,
  level: number,
  spells: readonly SpellDef[],
  bindings: Bindings,
): boolean {
  return bindProblem(spellId, slot, level, spells, bindings) === null;
}

/**
 * Einen Zauber auf einen Platz legen.
 *
 * **Ohne Seiteneffekt**: Die alte Belegung bleibt, die neue wird
 * zurückgegeben. Wer sie ablegt, entscheidet der Aufrufer — dieselbe Bauart
 * wie bei `grantXp` (#248).
 *
 * Geht es nicht, kommt die **unveränderte** Belegung zurück. Eine halb
 * geänderte wäre schlimmer als gar keine: Der Spieler sähe einen Platz leer,
 * den er nie geräumt hat.
 */
export function bindSpell(
  bindings: Bindings,
  spellId: string,
  slot: number,
  level: number,
  spells: readonly SpellDef[],
): Bindings {
  // **Gegen die bestehende Belegung** (Issue #287 AC3): Legt der Spieler einen
  // Zauber auf einen zweiten Platz, kommt die unveraenderte Belegung zurueck.
  if (!canBind(spellId, slot, level, spells, bindings)) return bindings;
  return { ...bindings, [String(slot)]: spellId };
}

/**
 * Einen Platz räumen.
 *
 * Der Platz verschwindet aus der Zuordnung, statt auf einen leeren Text zu
 * zeigen: Ein Eintrag mit leerem Wert sähe aus wie eine Belegung, die etwas
 * meint.
 */
export function clearSlot(bindings: Bindings, slot: number): Bindings {
  const kopie: Record<string, string> = { ...bindings };
  delete kopie[String(slot)];
  return kopie;
}

/**
 * Die Belegung auf das bringen, was diese Stufe zulässt.
 *
 * **Nur beim LESEN, nicht beim Spielen.** Ein Spielstand kann Einträge
 * tragen, die heute nicht mehr gelten — aus einer Fassung mit anderen
 * Freischaltstufen etwa. Sie stillschweigend zu behalten hieße, dass der Kampf
 * einen Zauber wirkt, den der Spieler gar nicht haben dürfte.
 *
 * **Was gültig bleibt, behält seinen Platz.** Das ist die Regel aus 14.4: Ein
 * Zauber liegt immer an derselben Stelle. Ungültige Einträge fallen weg, die
 * übrigen rücken NICHT nach.
 */
export function sanitize(bindings: Bindings, level: number, spells: readonly SpellDef[]): Bindings {
  const heil: Record<string, string> = {};
  // **Aufsteigend nach Platz**, damit die Bereinigung wiederholbar ist: Bei
  // einer Doppelbelegung behält der KLEINERE Platz den Zauber. Object.entries
  // gibt Zahlen-Schlüssel zwar aufsteigend zurück, aber das ist eine
  // Eigenschaft der Laufzeit und keine Zusage dieser Funktion.
  const plaetze = Object.keys(bindings)
    .map((platz) => Number.parseInt(platz, 10))
    .filter((slot) => !Number.isNaN(slot))
    .sort((a, b) => a - b);
  for (const slot of plaetze) {
    const spellId = bindings[String(slot)];
    if (spellId === undefined) continue;
    // **Gegen das schon Bereinigte prüfen, nicht gegen die Eingabe** (Issue
    // #287 AC3). Es gibt Spielstände, die unter der alten Regel entstanden
    // sind — ein Nachweis, der nur die Auswahl prüft, lässt sie durch.
    if (canBind(spellId, slot, level, spells, heil)) heil[String(slot)] = spellId;
  }
  return heil;
}

/**
 * Die Startbelegung für eine Stufe.
 *
 * **Nur so viele Zauber, wie Plätze offen sind**, und in der Reihenfolge der
 * Zauberdaten. Sie ist ein Vorschlag für einen leeren Spielstand, keine
 * Vorgabe: Sobald der Spieler etwas ändert, gilt seine Belegung.
 *
 * Keine Sortierung nach „bester Zauber" — GAME-DESIGN sagt zweimal, dass das
 * Zusammenstellen eine der beiden Kernaufgaben ist. Eine Empfehlung nähme sie
 * weg.
 */
export function defaultBindings(level: number, spells: readonly SpellDef[]): Bindings {
  const plaetze = barSlotsAtLevel(level);
  const heil: Record<string, string> = {};
  let slot = 0;
  for (const spell of spells) {
    if (slot >= plaetze) break;
    if (!isSpellUnlocked(spell, level)) continue;
    heil[String(slot)] = spell.id;
    slot++;
  }
  return heil;
}

/**
 * Eine Belegung auf die Plätze einer Sitzung übertragen (Issue #264).
 *
 * **Die einzige Stelle, die das tut** — und sie steht hier, weil sie bis zum
 * 2026-08-05 an ZWEI Orten stand: in `App.applyBindings` und noch einmal im
 * Prüfstand, der sie nachbaute. Beide Fassungen waren gleich, und nur eine
 * wurde gemessen.
 *
 * Aufgefallen ist es an einer Gegenprobe, die NICHT anschlug: Wer `app.ts`
 * umstellt, trifft keinen Prüffall, denn keiner ruft `App` auf (sie braucht
 * ein Dokument). Meine eigene Gegenprobe hatte den PRÜFSTAND umgestellt und
 * war rot — sie belegte damit, dass der Prüfstand die Regel misst, nicht dass
 * die Umsetzung sie einhält.
 *
 * Das ist dasselbe Muster wie zwei Schreibstellen für denselben Wert: Die
 * Regel steht an zwei Orten, und einer läuft weg.
 *
 * **Jeder Platz wird gesetzt, auch die leeren.** Wer nur die Einträge der
 * Belegung überträgt, lässt einen geräumten Platz auf seinem alten Zauber
 * stehen — nichts stürzt ab, die Taste tut nur etwas anderes, als dasteht,
 * und der Spieler hält es für einen Fehler in seiner Rotation.
 *
 * @param slotCount Wie viele Plätze es überhaupt gibt
 * @param indexOf Zauber-Kennung auf Index im Kampf, oder negativ
 * @param bind Setzt den Platz; ein negativer Index heißt „leer"
 */
export function applyBindings(
  bindings: Bindings,
  slotCount: number,
  indexOf: (spellId: string) => number,
  bind: (slot: number, spellIndex: number) => void,
): void {
  for (let slot = 0; slot < slotCount; slot++) {
    const spellId = spellAtSlot(bindings, slot);
    bind(slot, spellId === null ? alsSpellIndex(-1) : indexOf(spellId));
  }
}

/**
 * Der Zauber auf einem Platz, oder `null`.
 *
 * Für die Anzeige und für den Kampf. `null` statt eines leeren Textes: Ein
 * leerer Platz ist etwas anderes als ein Zauber ohne Namen.
 */
export function spellAtSlot(bindings: Bindings, slot: number): string | null {
  return bindings[String(slot)] ?? null;
}
