/**
 * Ist `test-runaway` gewinnbar? (Issue #394, GAME-DESIGN 11.2 Baustein 6)
 *
 * WARUM ES DIESE DATEI GIBT
 * -------------------------
 * Für diesen Boss wurden `heldPercent` (#256), die Punktequelle (#276), der
 * Bestwert (#377) und die Anzeige (#387) gebaut — vier Issues. Ob man ihn
 * gewinnen kann, wusste niemand. GAME-DESIGN 20.2 verlangt einen Prüflauf, der
 * für JEDEN Boss die Besiegbarkeit belegt; den gibt es nicht, und dies ist der
 * erste Fall, in dem seine Abwesenheit gemessen weh tat.
 *
 * ER WIRD WIRKLICH GESPIELT
 * -------------------------
 * Sitzung, Countdown, Tastendrücke, Kampfende — kein nachgebautes Rechenmodell.
 * Ein Prüffall mit gesetzten Zahlen belegte die Formel ein zweites Mal; was
 * hier gebraucht wird, ist die Verbindung zum Spiel.
 *
 * DIE ZWEITE DATEI-HÄLFTE IST DIE WICHTIGERE
 * ------------------------------------------
 * Sie hält eine AUSSAGE fest statt eines Ergebnisses: Bei diesem Boss geht
 * Timing über Frequenz, weil `undoProgress` am angesammelten Verlust gedeckelt
 * ist. Ohne sie wäre diese Deckelung eine Zeile, die beim nächsten Umbau
 * jemand für eine Sicherheitsprüfung hält und wegnimmt.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { COUNTDOWN_MS, PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';

/**
 * Die Zauber werden über ihre KENNUNG gesucht, nicht über einen Index.
 *
 * **Das ist die Lehre aus dem Bau dieses Issues.** Der erste Läufer schrieb
 * die Reihenfolge aus einer Browser-Ausgabe ab und benutzte 4 für `crust` —
 * das ist `searing`, ein Schadenszauber, und Schaden zählt bei einem
 * `runaway`-Boss gegen den Spieler. Gemessen wurde also, dass ein
 * Schadenszauber nicht stabilisiert; berichtet wurde „crust stabilisiert
 * nicht".
 *
 * Zwei Zauber waren am selben Tag auf Position 3 und 4 dazugekommen (#268).
 * Eine abgeschriebene Reihenfolge hält so etwas nicht aus; eine Suche schon.
 */
function spellIndex(combat: Combat, id: string): number {
  const index = combat.spells.findIndex((spell) => spell.id === id);
  if (index < 0)
    throw new Error(`Zauber "${id}" gibt es nicht — die Pruefung wuerde nichts messen`);
  return index;
}

interface Lauf {
  readonly victory: boolean;
  readonly heldPercent: number | null;
  readonly stabilised: number;
}

/**
 * Einen Durchlauf gegen `test-runaway` spielen.
 *
 * `everyMs` ist der einzige Hebel — dieselbe Zusammenstellung, dieselbe Frist,
 * nur der Abstand zwischen den Drücken.
 */
function spiele(ids: readonly string[], everyMs: number): Lauf {
  const boss = bossById('test-runaway') as BossDef;
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  ids.forEach((id, slot) => {
    session.bind(slot, spellIndex(combat, id));
  });
  session.beginCountdown(0);

  let tick = 0;
  let presses = 0;
  let letzter = 0;
  // Über die Frist hinaus laufen lassen: Der Sieg IST hier das Ablaufen der
  // Zeit, und wer bei der Frist abbricht, misst ihn nie.
  const ende = COUNTDOWN_MS + 25_000;
  for (let t = 0; t <= ende; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      queue.push(presses % ids.length, tick, t, tick);
      presses++;
    }
    session.tick(t);
    letzter = t;
    tick++;
    if (session.phase === PHASE_ENDED) break;
  }
  const summary = session.summary(letzter);
  return {
    victory: summary.victory,
    heldPercent: summary.heldPercent,
    stabilised: summary.stabilised,
  };
}

describe('AC1/AC2 — test-runaway ist gewinnbar, und zwar so', () => {
  it('crust allein, alle 9,5 s — 94 % Ring-Rest', () => {
    const lauf = spiele(['crust'], 9500);
    expect(lauf.victory).toBe(true);
    expect(lauf.heldPercent).toBe(94);
    expect(lauf.stabilised).toBe(1140);
  });

  it('GEGENPROBE: OHNE crust geht es NICHT', () => {
    // Ohne diese Zeile wäre der Prüffall oben grün, weil der Boss zu leicht
    // ist — nicht, weil `crust` wirkt. Vier Zusammenstellungen, keine schafft
    // es, und alle vier stabilisieren exakt null.
    for (const id of ['salve', 'quench', 'temper', 'kindle']) {
      const lauf = spiele([id], 6000);
      expect(lauf.victory, `${id} hat gewonnen — die Gegenprobe misst nichts mehr`).toBe(false);
      expect(lauf.stabilised, `${id} hat stabilisiert`).toBe(0);
    }
  });

  it('GEGENPROBE ZUR GEGENPROBE: salve stabilisiert nie, bei KEINEM Intervall', () => {
    // Der strukturelle Grund, nicht ein ungünstiger Rhythmus: `heal`
    // stabilisiert um `min(Betrag, fehlende Anzeige)`, und `test-runaway` hat
    // eine leere `timeline` — der Spieler nimmt nie Schaden, also fehlt nie
    // etwas. Der halbe Stabilisierungs-Weg ist dort tot.
    for (const abstand of [1500, 3000, 6000, 9500, 12_000]) {
      expect(spiele(['salve'], abstand).stabilised).toBe(0);
    }
  });

  it('ein einziger Druck ganz am Anfang reicht nicht — und das ist kein Balance-Problem', () => {
    // Bei t=0 ist noch nichts angesammelt, das zurückgedrängt werden könnte.
    // Der Druck verpufft und verbraucht trotzdem einen der drei Stapel.
    const lauf = spiele(['crust'], 99_000);
    expect(lauf.victory).toBe(false);
    expect(lauf.stabilised).toBe(0);
  });
});

describe('Timing geht über Frequenz — die Aussage, nicht das Ergebnis', () => {
  /**
   * `undoProgress` deckelt die Rückdrängung am ANGESAMMELTEN Verlust:
   *
   *     const wirklich = Math.min(amount, this.damageDealtValue);
   *
   * Wer früh drückt, verschenkt den Rest der 900. Deshalb ist häufiger
   * Drücken hier schlechter — die einzige Stelle im Spiel, an der das gilt.
   *
   * **Dieser Prüffall bewacht die Deckelung.** Nimmt jemand sie weg, weil sie
   * wie eine Sicherheitsprüfung aussieht, wird die Reihenfolge unten monoton
   * und er fällt.
   */
  it('alle 9,5 s schlägt alle 2 s — deutlich', () => {
    const selten = spiele(['crust'], 9500);
    const oft = spiele(['crust'], 2000);

    expect(selten.victory).toBe(true);
    expect(oft.victory).toBe(true);
    // Beide gewinnen — der Unterschied ist die Güte, nicht der Ausgang.
    expect(selten.heldPercent).toBeGreaterThan(oft.heldPercent ?? 0);
    expect(selten.stabilised).toBeGreaterThan(oft.stabilised);
  });

  it('die Reihe steigt bis 9,5 s und ist danach nicht monoton', () => {
    // Die gemessene Reihe. Sie steht hier als Reihe und nicht als einzelner
    // Bestwert, weil die AUSSAGE in ihrem Verlauf liegt: Es gibt ein Optimum
    // in der Mitte, nicht am Rand.
    const gemessen = [2000, 6000, 7000, 8000, 9000, 9500].map(
      (ms) => spiele(['crust'], ms).heldPercent ?? 0,
    );
    expect(gemessen).toEqual([52, 52, 64, 76, 88, 94]);

    // Und jenseits des Optimums fällt sie wieder — ein Druck weniger passt in
    // die Frist.
    expect(spiele(['crust'], 10_000).heldPercent).toBeLessThan(94);
  });

  it('GEGENPROBE: der Ertrag waechst STETIG mit der Wartezeit, nicht in Stufen', () => {
    // **Die erste Fassung dieser Gegenprobe war unwirksam**, und das ist der
    // Grund, warum sie jetzt so aussieht: Sie prüfte, ob der Höchstwert der
    // Reihe INNEN liegt. Gemessen mit entfernter Deckelung liegt er das auch —
    // die Reihe wird dann `70, 70, 70, 100, 100, 100, 70, 70`. Die Prüfung
    // trennte die beiden Zustände nicht und wäre grün geblieben.
    //
    // Was sie trennt, ist die STETIGKEIT: Mit der Deckelung bringt jede
    // zusätzliche Sekunde Wartezeit echten Ertrag, weil mehr angesammelt ist.
    // Ohne sie bringt jeder Druck dieselben 900, und die Reihe zerfällt in
    // Plateaus — drei gleiche Werte nebeneinander.
    const reihe = [6000, 7000, 8000, 9000, 9500].map(
      (ms) => spiele(['crust'], ms).heldPercent ?? 0,
    );
    for (let i = 1; i < reihe.length; i++) {
      expect(
        reihe[i],
        `Bei ${String(i)}: ${String(reihe[i - 1])} -> ${String(reihe[i])} ist kein Anstieg. ` +
          'Ist die Deckelung in undoProgress entfernt worden? Dann bringt jeder Druck ' +
          'denselben Betrag, und aus der Kurve werden Plateaus.',
      ).toBeGreaterThan(reihe[i - 1] ?? 0);
    }
  });
});
