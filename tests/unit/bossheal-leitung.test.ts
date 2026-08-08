/**
 * `bossHealed` kommt im `ReplayResult` an (Issue #609, Guide 8.70).
 *
 * # Warum dieser Fall neben `bossheal-ausloeser.test.ts` steht
 *
 * Der Fall dort misst die **Mechanik**: Er baut ein `Combat`, lässt die
 * Schwächung zehren und liest `combat.bossHealed`. Er bleibt grün, wenn die
 * Leitung ins `ReplayResult` fehlt oder wieder entfernt wird — er fragt die
 * Quelle, nicht das Ziel.
 *
 * **Gemessen, nicht behauptet:** Der Orchestrator hat
 * `bossHealed: combat.bossHealed` in `recording.ts` auf `0` gesetzt und den
 * vollen Lauf gefahren. Ergebnis: **1904 von 1904 grün.** Kein einziger Fall
 * im Bestand meldete den Verlust — auch meiner nicht, obwohl ich die Leitung
 * gerade erst gelegt hatte.
 *
 * Damit stand `bossHealed` genau da, wo `controlSpent` vor #605 stand:
 * geführt, paritätsgesichert, mit frischer Leitung — und der nächste Umbau
 * nimmt sie weg, ohne dass etwas meldet.
 *
 * # Warum der Leitungs-Wächter das nicht fängt
 *
 * `leitungs-waechter.test.ts` prüft **strukturell**, ob ein Getter einen Weg
 * hat: entweder eine Zuweisung `feld: combat.x` oder eine Deckung über die
 * Prüfsumme. `bossHealed` steht in der Prüfsumme, gilt dort also als gedeckt —
 * auch dann, wenn die Zuweisung verschwindet. Der Wächter beantwortet „gibt es
 * einen Weg", dieser Fall beantwortet „kommt der Wert an".
 */
import { describe, expect, it } from 'vitest';
import { bossById } from '../../src/core/data.js';
import { STEP1_BINDINGS } from '../../src/core/loadout.js';
import { replayRecording } from '../../src/devtools/recording.js';

/** Der Prüf-Boss aus #609: Schwächung ab 5000 ms, 50 zurück je Schritt. */
const LAPSE = 'test-lapse';

function lauf(ticks: number): ReturnType<typeof replayRecording> {
  const boss = bossById(LAPSE);
  if (boss === undefined) throw new Error(`Der Boss ${LAPSE} fehlt.`);
  // **Es MUSS gedrückt werden, und das ist der Punkt.** Der erste Entwurf
  // dieses Falls fuhr ohne Eingaben — „wer nichts drückt, reinigt nicht" — und
  // maß 0 statt eines Betrags. Der Grund ist derselbe Deckel, an dem schon der Prüf-Boss
  // scheiterte: `undoProgress` dreht nur zurück, was vorher an Fortschritt
  // gefallen ist. Ohne Rotation gibt es keinen, also auch nichts zurückzudrehen.
  //
  // Gedrückt wird nur Platz 1 (KINDLE). Er trifft und reinigt nicht — die
  // Schwächung bleibt stehen, und der zweite Auslöser zehrt.
  const presses: { tick: number; slot: number }[] = [];
  for (let tick = 0; tick < ticks; tick += 100) presses.push({ tick, slot: 0 });
  // **Die Belegung steht jetzt dabei** (Issue #663). Sie war nie anders
  // gemeint — der Kommentar oben nennt KINDLE, und `STEP1_BINDINGS` bindet
  // KINDLE auf Platz 0. Bis heute kam sie aus einem Default in
  // `spellForSlot`; dieser Fall war also richtig, weil zwei Stellen
  // zufällig dasselbe meinten.
  return replayRecording(
    { name: 'lapse-leitung', seed: 0, ticks, presses, bindings: STEP1_BINDINGS },
    { boss },
  );
}

describe('die Größe kommt im Ergebnis an, nicht nur im Kampf', () => {
  it('das Feld existiert überhaupt im ReplayResult', () => {
    // Die Zusicherung vor der Messung: Fehlte das Feld ganz, vergliche der
    // Fall darunter `undefined` mit einer Zahl und meldete etwas anderes, als
    // er zu messen vorgibt.
    expect(Object.keys(lauf(10))).toContain('bossHealed');
  });

  it('DER WERT KOMMT AN — harte Zahl aus dem Ergebnis, nicht aus `Combat`', () => {
    // Schwächung ab 5000 ms, Takt 1000 ms, 50 je Schritt. Bei 1200 Takten
    // (12 s) kommen **sechs** Schritte an, nicht sieben.
    //
    // **Die Zahl ist gemessen, nicht aus dem Mechanik-Fall übernommen.** Dort
    // steht 350, weil die Rotation dem GCD folgt; hier drückt die Aufzeichnung
    // starr alle 100 Takte, und ein Schritt findet weniger Fortschritt zum
    // Zurückdrehen vor. Wer die 350 hierher kopiert hätte, hätte einen Fall
    // gebaut, der aus dem falschen Grund rot ist.
    //
    // **Die Zahl steht hier ausgeschrieben und wird nicht gerechnet.** Ein
    // `toBeGreaterThan(0)` bliebe grün, wenn die Leitung einen anderen Wert
    // durchreichte — und eine Größe, die den Fortschritt ZURÜCKDREHT, um den
    // Faktor zehn danebenzuhaben, ist derselbe Fehler wie sie zu verlieren.
    expect(lauf(1200).bossHealed).toBe(300);
  });

  it('und sie wächst mit der Zeit — der Wert ist nicht festgenagelt', () => {
    // Ohne diesen Fall bliebe offen, ob die Leitung eine Konstante durchreicht
    // oder den tatsächlichen Stand. Beides sähe bei einer einzelnen Messung
    // gleich aus.
    expect(lauf(2000).bossHealed).toBeGreaterThan(lauf(1200).bossHealed);
  });

  it('ein Boss ohne Heilung meldet 0 — die Gegenprobe gegen eine feste Zahl', () => {
    // Käme hier ebenfalls 350, reichte die Leitung nicht den Kampf durch,
    // sondern irgendetwas anderes.
    const boss = bossById('boss-2-1');
    if (boss === undefined) throw new Error('boss-2-1 fehlt.');
    expect(boss.bossHeal, 'dieser Boss hat inzwischen eine Heilung').toBeNull();
    expect(
      replayRecording({ name: 'ohne-heilung', seed: 0, ticks: 1200, presses: [] }, { boss })
        .bossHealed,
    ).toBe(0);
  });
});
