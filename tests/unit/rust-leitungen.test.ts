/**
 * Die sechs Grössen, die Rust seit jeher führt und das Spiel nicht (Issue #619).
 *
 * # Der Befund ist Vollständigkeit, nicht Abwesenheit
 *
 * Das Issue lautete: *„Wo Rust eine Grösse führt und das Spiel nicht, kann kein
 * Paritäts-Fall sie vergleichen."* Nachgemessen stimmt das nur für **eine**
 * der sechs:
 *
 * ```text
 *   interruptsLanded   in 5 Aufzeichnungen   5 Rust-Tests
 *   interruptsMissed   in 5                  3
 *   castsInterrupted   in 1  (cast-time)     1
 *   castsCancelled     in 1  (cast-time)     1
 *   cycle              in 1  (trial-blocks)  1
 *   stabilised         IN KEINER             1
 * ```
 *
 * Fünf werden also verglichen — aber nur dort, wo jemand sie **einzeln** in ein
 * `expected` eingetragen hat. Eine neue Aufzeichnung führt sie nicht mit; wer
 * eine baut und die Felder nicht von Hand nachträgt, bekommt einen Lauf, in
 * dem der Simulator sechs Grössen rechnet, denen niemand widerspricht.
 *
 * `stabilised` ist die eine echte Lücke: Beide Seiten führen es, kein
 * Paritäts-Fall vergleicht die Werte.
 *
 * # Warum dieser Fall über `replayRecording` geht
 *
 * WORKER-GUIDE 8.70: Ein Fall, der `Combat` direkt befragt, prüft die Mechanik
 * und nicht die Leitung. Genau das ist mir bei #605 passiert — drei von vier
 * Prüffällen blieben grün, als ich die Leitung zurückstellte, weil sie die
 * Quelle lasen statt das Ziel.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { bossById } from '../../src/core/data.js';
import { replayRecording } from '../../src/devtools/recording.js';

const DATEI = new URL('../parity/recordings/instance1-boss-1.json', import.meta.url).pathname;

type Aufzeichnung = {
  bossId: string;
  seed?: number;
  ticks: number;
  presses: { tick: number; slot: number }[];
  bindings: { slot: number; spellId: string }[];
};

function spiele(): ReturnType<typeof replayRecording> {
  const d = JSON.parse(readFileSync(DATEI, 'utf8')) as Aufzeichnung;
  const boss = bossById(d.bossId);
  if (boss === undefined) throw new Error(`Der Boss ${d.bossId} fehlt.`);
  return replayRecording(
    {
      name: 'e619-instance1-boss-1',
      seed: d.seed ?? 0,
      ticks: d.ticks,
      presses: d.presses,
      bindings: d.bindings,
    },
    { boss },
  );
}

describe('Die sechs Rust-Grössen kommen jetzt auch im Spiel-Ergebnis an', () => {
  it('AC4 — interruptsMissed und interruptsLanded stehen im ReplayResult', () => {
    // **Die Grösse, an der `bossHeal` hängt** (#609), und die einzige
    // Anforderung, die heute jeder Boss stellt. Solange sie in keinem
    // Lauf-Ergebnis ankommt, kann niemand messen, ob ein Boss seine
    // Unterbrechungen tatsächlich verlangt.
    const r = spiele();

    // **Harte Zahlen aus einem echten Boss der ersten Instanz.** Sie stehen
    // ausgeschrieben und nicht als Variable, damit eine Verschiebung auffällt
    // statt sich mitzubewegen.
    //
    // **Die 1 stammt aus `replayRecording`, nicht aus einem Nachbau.** Meine
    // erste Messung fuhr die Aufzeichnung von Hand nach und ergab 2 — weil sie
    // über den Boss-Tod hinauslief. `replayRecording` bricht bei `killedAtMs`
    // (51070) ab, und der zweite verpasste Zauber liegt danach. Ein Nachbau ist
    // eine zweite Quelle mit eigenen Zahlen, kein Blick auf die erste.
    expect(r.interruptsLanded).toBe(6);
    expect(r.interruptsMissed).toBe(1);

    // **Die zweite Zahl, die für die Aussage niemand braucht** (Guide 8.67):
    // Der Lauf muss überhaupt unterbrechbare Zauber gesehen haben. Wären beide
    // 0, wäre der Vergleich oben trivial und der Fall grün ohne Aussage.
    expect(
      r.interruptsLanded + r.interruptsMissed,
      'der Lauf enthält gar keine unterbrechbaren Boss-Zauber',
    ).toBeGreaterThan(0);
  });

  it('die übrigen vier stehen ebenfalls im Ergebnis', () => {
    const r = spiele();
    // Struktur, nicht Werte: Ein Feld mit `0` ist eine Aussage über den Lauf,
    // kein Mangel der Leitung (AC5 von #612).
    for (const feld of ['castsInterrupted', 'castsCancelled', 'cycle', 'stabilised'] as const) {
      expect(Object.keys(r), `${feld} fehlt im ReplayResult`).toContain(feld);
      expect(typeof r[feld], `${feld} ist keine Zahl`).toBe('number');
    }
  });

  it('stabilised war die einzige, die in KEINER Aufzeichnung stand', () => {
    // Der Grund, aus dem #619 mehr ist als Aufräumen: Die anderen fünf werden
    // punktuell verglichen, diese gar nicht. Beide Seiten rechnen sie, und
    // niemand widerspricht.
    const r = spiele();
    expect(typeof r.stabilised).toBe('number');
    expect(r.cycle, 'die Zeitleiste läuft in diesem Lauf gar nicht um').toBe(3);
  });
});
