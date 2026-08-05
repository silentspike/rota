/**
 * Der Bestwert je Boss (Issue #377).
 *
 * WAS DIESE DATEI TRÄGT
 * ---------------------
 * AC1 (schreiben und ersetzen), AC2 (das Mass steht im Eintrag) und AC3 (die
 * Gegenprobe dazu). Der Kern ist AC3, und zwar seine zweite Lesart: Nicht der
 * Eintrag OHNE Angabe ist gefährlich — der fällt sofort auf — sondern der
 * Eintrag MIT einer Angabe, die nicht mehr stimmt.
 *
 * Ein Boss bekommt `runaway` oder verliert es. Derselbe `bossId` trägt danach
 * eine andere Grösse: 85 hiess gestern „85 % Ring gehalten", heute „85 ms
 * Reserve". Beide sind plausible Zahlen, keine ist kaputt, und ein Vergleich
 * zwischen ihnen ist eine Erfindung.
 */
import { describe, expect, it } from 'vitest';
import { isWellFormed, recordBest, usableBest } from '../../src/analysis/best-run.js';
import type { BestRun } from '../../src/analysis/save-format.js';

const FASSUNG = '0.1.0';
const JETZT = 1_800_000_000_000;

function versuch(
  over: Partial<Parameters<typeof recordBest>[1]> = {},
): Parameters<typeof recordBest>[1] {
  return {
    bossId: 'first-crucible',
    difficulty: 1,
    metric: 'reserve',
    value: 20_000,
    gameVersion: FASSUNG,
    at: JETZT,
    ...over,
  };
}

function eintrag(over: Partial<BestRun> = {}): BestRun {
  return {
    bossId: 'first-crucible',
    difficulty: 1,
    metric: 'reserve',
    value: 20_000,
    gameVersion: FASSUNG,
    achievedAt: JETZT,
    ...over,
  };
}

describe('AC1 — ein Sieg schreibt, ein besserer ersetzt', () => {
  it('legt beim ersten Sieg einen Eintrag an', () => {
    const nachher = recordBest([], versuch());
    expect(nachher.runs).toHaveLength(1);
    expect(nachher.runs[0]?.value).toBe(20_000);
    // **Kein „verbessert" beim ersten Mal.** Es gab nichts zu unterbieten;
    // eine Verbesserung zu behaupten wäre der Grund, warum die Differenz-
    // Auszahlung später doppelt zahlte.
    expect(nachher.improved).toBe(false);
    expect(nachher.previous).toBeNull();
  });

  it('ersetzt bei einem besseren Lauf', () => {
    const nachher = recordBest([eintrag()], versuch({ value: 35_000, at: JETZT + 1000 }));
    expect(nachher.improved).toBe(true);
    expect(nachher.previous).toBe(20_000);
    expect(nachher.runs[0]?.value).toBe(35_000);
    expect(nachher.runs[0]?.achievedAt).toBe(JETZT + 1000);
  });

  it('behält den alten Wert bei einem schlechteren Lauf', () => {
    const nachher = recordBest([eintrag()], versuch({ value: 5_000, at: JETZT + 1000 }));
    expect(nachher.improved).toBe(false);
    expect(nachher.previous).toBe(20_000);
    expect(nachher.runs[0]?.value).toBe(20_000);
    // Zeitpunkt und Fassung gehören zum BESTWERT, nicht zum letzten Versuch.
    expect(nachher.runs[0]?.achievedAt).toBe(JETZT);
  });

  it('führt Bosse und Schwierigkeitsstufen getrennt', () => {
    const eins = recordBest([], versuch());
    const zwei = recordBest(eins.runs, versuch({ bossId: 'test-warden', value: 9_000 }));
    const drei = recordBest(zwei.runs, versuch({ difficulty: 2, value: 3_000 }));
    expect(drei.runs).toHaveLength(3);
    expect(drei.previous).toBeNull();
  });

  it('lässt den übergebenen Stand unangetastet', () => {
    const vorher = [eintrag()];
    const kopie = JSON.stringify(vorher);
    recordBest(vorher, versuch({ value: 99_000 }));
    expect(JSON.stringify(vorher)).toBe(kopie);
  });
});

describe('AC2 — das Mass steht im Eintrag', () => {
  it('schreibt bei einem runaway-Boss den Ring-Rest, nicht die Reserve', () => {
    const nachher = recordBest([], versuch({ metric: 'held', value: 62 }));
    expect(nachher.runs[0]?.metric).toBe('held');
    expect(nachher.runs[0]?.value).toBe(62);
  });

  it('bei beiden Massen ist grösser besser', () => {
    // Der Grund, warum die Reserve gespeichert wird und nicht die Kampfdauer:
    // Sonst hinge die Vergleichsrichtung am Mass, und eine Bestenliste, die
    // den schlechtesten Lauf führt, sieht aus wie eine Bestenliste.
    const reserve = recordBest([eintrag()], versuch({ value: 30_000 }));
    const ring = recordBest(
      [eintrag({ metric: 'held', value: 40 })],
      versuch({ metric: 'held', value: 55 }),
    );
    expect(reserve.improved).toBe(true);
    expect(ring.improved).toBe(true);
  });
});

describe('AC3 — ein unbrauchbarer Eintrag wird erkannt, nicht gedeutet', () => {
  it('DER TEURE FALL: der Boss hat sein Mass gewechselt', () => {
    // Gestern trug der Eintrag 85 Prozent gehaltenen Ring. Heute ist derselbe
    // Boss ein gewöhnlicher, und 85 wäre eine Reserve von 85 Millisekunden.
    // Beide Zahlen sind plausibel. Ohne die Prüfung würde die Differenz-
    // Auszahlung Prozent gegen Millisekunden halten.
    const alt = [eintrag({ metric: 'held', value: 85 })];
    const geprueft = usableBest(alt, 'first-crucible', 1, 'reserve');

    expect(geprueft.entry).toBeNull();
    expect(geprueft.discarded).toBe('Mass held statt reserve');
  });

  it('GEGENPROBE: passt das Mass, wird der Eintrag benutzt', () => {
    // Ohne diese Zeile wäre nicht zu sehen, ob die Prüfung überhaupt je einen
    // Eintrag durchlässt — eine Prüfung, die alles verwirft, verwirft auch
    // das Richtige.
    const geprueft = usableBest(
      [eintrag({ metric: 'held', value: 85 })],
      'first-crucible',
      1,
      'held',
    );
    expect(geprueft.entry?.value).toBe(85);
    expect(geprueft.discarded).toBeNull();
  });

  it('ein Eintrag ohne Mass ist unbrauchbar', () => {
    // Die erste Lesart: strukturell. Sie fällt sofort auf — aber sie muss
    // trotzdem stehen, weil ein Stand aus der Fassung vor #377 genau so
    // aussähe.
    const ohne = [{ ...eintrag(), metric: undefined }] as unknown as BestRun[];
    const geprueft = usableBest(ohne, 'first-crucible', 1, 'reserve');
    expect(geprueft.entry).toBeNull();
    expect(geprueft.discarded).toBe('Form unbrauchbar');
  });

  it('ein verworfener Eintrag wird ERSETZT, nicht verdoppelt', () => {
    // Sonst stünden zwei Einträge für denselben Boss da, und welcher gilt,
    // entschiede die Reihenfolge in der Liste.
    const alt = [eintrag({ metric: 'held', value: 85 })];
    const nachher = recordBest(alt, versuch({ metric: 'reserve', value: 12_000 }));
    expect(nachher.runs).toHaveLength(1);
    expect(nachher.runs[0]?.metric).toBe('reserve');
    expect(nachher.runs[0]?.value).toBe(12_000);
    // Und der Aufrufer erfährt, dass hier nichts zu vergleichen war.
    expect(nachher.previous).toBeNull();
    expect(nachher.discarded).toBe('Mass held statt reserve');
  });

  describe('die Formprüfung im Einzelnen', () => {
    it('nimmt einen vollständigen Eintrag an', () => {
      expect(isWellFormed(eintrag())).toBe(true);
    });

    it.each([
      ['kein Objekt', null],
      ['leeres Objekt', {}],
      ['bossId leer', { ...eintrag(), bossId: '' }],
      ['metric unbekannt', { ...eintrag(), metric: 'time' }],
      ['value als Text', { ...eintrag(), value: '20000' }],
      ['value negativ', { ...eintrag(), value: -1 }],
      ['value NaN', { ...eintrag(), value: Number.NaN }],
      ['difficulty fehlt', { ...eintrag(), difficulty: undefined }],
      ['gameVersion fehlt', { ...eintrag(), gameVersion: undefined }],
      ['achievedAt als Text', { ...eintrag(), achievedAt: 'gestern' }],
    ])('verwirft: %s', (_name, wert) => {
      expect(isWellFormed(wert)).toBe(false);
    });

    it('WARUM NaN eigens dasteht', () => {
      // `NaN > irgendwas` ist immer falsch. Ein Bestwert von NaN wäre für
      // immer unschlagbar und würde nie ersetzt — der Spieler sähe eine Marke,
      // die er nicht mehr los wird.
      // Die Regel `use-isnan` warnt vor genau diesem Muster. Hier wird es
      // absichtlich vorgefuehrt: Der stumme Vergleich IST der Fehler, gegen
      // den `isWellFormed` schuetzt.
      // eslint-disable-next-line use-isnan -- das Muster ist die Aussage
      expect(Number.NaN > 999_999).toBe(false);
      const nachher = recordBest(
        [eintrag({ value: Number.NaN })] as BestRun[],
        versuch({ value: 20_000 }),
      );
      expect(nachher.runs[0]?.value).toBe(20_000);
    });
  });
});
