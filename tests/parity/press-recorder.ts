/**
 * Einen Tastendruck wirken UND aufzeichnen (Issue #351).
 *
 * WARUM DAS EINE STELLE SEIN MUSS
 * -------------------------------
 * Eine Aufzeichnung hält den Druck als **Platz** fest, das Spiel wirkt ihn als
 * **Zauber-Index**. Zwei Größen, die im Bestand zufällig übereinstimmen — die
 * ersten vier Zauber liegen auf den Plätzen 0 bis 3 — und dort auseinander
 * fallen, wo ein Zauber nicht auf der Leiste liegt.
 *
 * Die alte Fassung beantwortete das mit einer Zahl:
 *
 *     presses.push({ tick, slot: slotOf.get(spell) ?? 0 });
 *
 * `?? 0` ist eine Rückfallebene ohne Fall. Sie sieht aus wie eine gültige
 * Antwort, und beim ersten Greifen steht auf der Datei CANKER und drin ist,
 * was auf Platz 0 liegt — bei `STEP1_BINDINGS` also KINDLE.
 *
 * WAS DANN PASSIERT WÄRE
 * ----------------------
 * Die Prüfsummen liefen auseinander, der Paritäts-Fall würde rot — aber die
 * Meldung zeigte auf die Prüfsumme, nicht auf die Ursache. Wer sie sucht, sucht
 * im Kampfcode, nicht in einer Aufzeichnungs-Hilfsfunktion.
 *
 * Und im ungünstigen Fall fällt es gar nicht auf: Wirken zwei Zauber zufällig
 * dasselbe, stimmen die Prüfsummen, und die Aufzeichnung belegt seither etwas
 * anderes, als ihr Name sagt.
 *
 * WARUM DIE BINDUNGEN EIN PARAMETER SIND
 * --------------------------------------
 * 19 der 30 Aufzeichnungen bringen seit #318 ihre eigene Leiste mit, 11 nicht.
 * Ein Helfer, der `STEP1_BINDINGS` voraussetzte, wäre derselbe stille Rückfall
 * eine Ebene höher — genau der, den `recording.ts` für die Gegenrichtung schon
 * beschreibt: „vergaß er sie, fiel diese Seite still auf `STEP1_BINDINGS`
 * zurück und las eine andere Leiste als der Simulator — aus derselben Datei."
 *
 * WARUM ALS EIGENE DATEI
 * ----------------------
 * Damit es eine Stelle gibt, an der ein Prüffall den Fehlerfall auslösen kann.
 * Der Fall ist im Bestand nicht erreichbar: `misplays.json` drückt die Plätze
 * 0, 1 und 2, alle gebunden. Eine Gegenprobe, die nur den Bestand fährt, bliebe
 * grün und belegte nichts.
 */
import { alsSpellIndex, type Combat, type SpellIndex } from '../../src/core/combat.js';
import type { Binding } from '../../src/core/loadout.js';
import type { RecordedPress } from '../../src/devtools/recording.js';

/**
 * Die Zuordnung Zauber-Index zu Platz, aus den Bindungen dieser Aufzeichnung.
 *
 * Getrennt vom Drücken, weil ein Erzeuger sie einmal baut und oft benutzt.
 */
export function slotMap(combat: Combat, bindings: readonly Binding[]): Map<number, number> {
  const map = new Map<number, number>();
  for (const binding of bindings) map.set(combat.indexOf(binding.spellId), binding.slot);
  return map;
}

/**
 * Den Platz eines Zaubers — oder ein Abbruch.
 *
 * **Kein Rückfall.** Wo es keine sinnvolle Antwort gibt, gehört ein Abbruch hin
 * und nicht die erstbeste Zahl. Die Meldung nennt den Zauber und die Leiste,
 * damit sie auf die Ursache zeigt statt auf ihre Folge.
 */
export function slotFor(
  slots: ReadonlyMap<number, number>,
  spell: SpellIndex,
  combat: Combat,
): number {
  const slot = slots.get(spell);
  if (slot === undefined) {
    const name = combat.spells[spell]?.id ?? `Index ${String(spell)}`;
    const belegt = [...slots.entries()]
      .map(([index, platz]) => `${String(platz)}:${combat.spells[index]?.id ?? '?'}`)
      .sort()
      .join(' ');
    throw new Error(
      `Der Zauber "${name}" liegt auf keinem Platz — die Aufzeichnung wuerde ihn still ` +
        `auf Platz 0 schreiben und der Simulator laese einen anderen. Leiste: ${belegt}`,
    );
  }
  return slot;
}

/**
 * Einen Druck wirken und mitschreiben.
 *
 * Gibt die Funktion zurück, die beide Seiten zusammenhält: Was gewirkt wird und
 * was in der Datei landet, entsteht aus **einem** Aufruf. Zwei getrennte Zeilen
 * waren die Stelle, an der sie auseinanderlaufen konnten.
 */
export function pressRecorder(
  combat: Combat,
  bindings: readonly Binding[],
  presses: RecordedPress[],
): (spell: SpellIndex, tick: number, timeMs: number) => void {
  const slots = slotMap(combat, bindings);
  return (spell, tick, timeMs) => {
    // **Der Platz ZUERST.** Wirkt man vorher und bricht dann ab, hat der Kampf
    // einen Zauber gesehen, den die Aufzeichnung nicht kennt — und ein
    // Aufrufer, der den Fehler fängt, schriebe ab da eine Datei, die still um
    // einen Druck versetzt ist.
    const slot = slotFor(slots, spell, combat);
    combat.cast(alsSpellIndex(spell), timeMs);
    presses.push({ tick, slot });
  };
}
