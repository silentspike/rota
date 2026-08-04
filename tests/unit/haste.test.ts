import { describe, expect, it } from 'vitest';
import { hastedMs, tickCount } from '../../src/core/haste.js';
import { TICK_MS, snapUp } from '../../src/core/loop.js';

describe('Tempo verkürzt Zeiten (GAME-DESIGN 6.7)', () => {
  it('lässt ohne Tempo JEDE Zeit auf ihrem Wert — die Wurzel von AC7', () => {
    // Bewusst auch Werte, die NICHT auf dem Takt-Raster liegen. Ohne den
    // Kurzschluss bei 0 hinge die Zusicherung daran, dass alle heutigen Daten
    // zufällig gerastert sind — und genau das darf sie nicht.
    for (const base of [0, 1, 7, 10, 1500, 3000, 8000, 40_501, 180_000]) {
      expect(hastedMs(base, 0), `${base} ms ohne Tempo`).toBe(base);
    }
  });

  it('verkürzt und rastert nach oben', () => {
    expect(hastedMs(8000, 200)).toBe(6670); // 6666,67 -> 6670
    expect(hastedMs(3000, 500)).toBe(2000); // glatt
    expect(hastedMs(1500, 500)).toBe(1000); // der Wert aus der Deckel-Rechnung
  });

  it('macht aus keiner Zeit keine Zeit — und aus einer Zeit nie null', () => {
    for (const permille of [1, 200, 1000, 100_000]) {
      expect(hastedMs(0, permille), `+${permille} auf 0 ms`).toBe(0);
    }
    // Der Gegenfall: Was es gibt, verschwindet auch bei absurdem Tempo nicht.
    expect(hastedMs(10, 1_000_000)).toBe(TICK_MS);
    expect(hastedMs(3000, 10_000_000)).toBe(TICK_MS);
    expect(hastedMs(3000, 10_000_000)).toBeGreaterThan(0);
  });

  it('ist über den ganzen Bereich monoton — mehr Tempo ist nie langsamer', () => {
    for (const base of [500, 1500, 3000, 8000, 180_000]) {
      let vorher = hastedMs(base, 0);
      for (let permille = 10; permille < 2000; permille += 10) {
        const jetzt = hastedMs(base, permille);
        expect(jetzt, `${base} ms bei +${permille}`).toBeLessThanOrEqual(vorher);
        vorher = jetzt;
      }
    }
  });

  it('liefert mit Tempo ausschließlich Zeiten auf dem Takt-Raster', () => {
    // **`permille` beginnt bei 1, und das ist keine Nachlässigkeit.**
    //
    // Bei 0 widersprechen sich die beiden Zusicherungen dieser Datei, und die
    // erste Fassung dieser Prüfung ist genau daran gefallen:
    //
    //     hastedMs(12345, 0)  ->  12345, nicht 12350
    //
    // „Ohne Tempo ändert sich nichts" (AC7) und „jedes Ergebnis liegt auf dem
    // Raster" können für eine ungerasterte Eingabe nicht beide gelten. AC7
    // gewinnt: Der Umbau muss bruchfrei einführbar sein, und dazu gehört, dass
    // er auch einen krummen Wert unberührt lässt statt ihn stillschweigend zu
    // verschieben. Gerastert wird, was Tempo anfasst — sonst nichts.
    for (const base of [500, 1500, 3000, 8000, 12_345, 180_000]) {
      for (let permille = 1; permille < 3000; permille += 7) {
        const t = hastedMs(base, permille);
        expect(t % TICK_MS, `${base} ms bei +${permille} ergab ${t}`).toBe(0);
      }
    }
  });

  it('bleibt bei langen Zeiten im sicheren Zahlenbereich', () => {
    // Auf der Rust-Seite läuft der Zwischenschritt in u64, weil `base × 1000`
    // sonst ab 4 294 968 ms überliefe. JavaScript hat die Grenze erst bei
    // 9,007e15 — die Prüfung steht hier trotzdem, damit beide Seiten dieselbe
    // Zahl liefern statt nur beide „irgendetwas".
    expect(hastedMs(4_300_000, 1000)).toBe(2_150_000);
    expect((4_300_000 * 1000) / 2000).toBeLessThan(Number.MAX_SAFE_INTEGER);
  });
});

describe('DoTs: die Dauer bleibt, die Ticks werden mehr (AC4)', () => {
  it('teilt die Dauer ganzzahlig und zahlt den Rest nicht aus', () => {
    expect(tickCount(12_000, 2000)).toBe(6);
    const schneller = hastedMs(2000, 200);
    expect(schneller).toBe(1670);
    expect(tickCount(12_000, schneller)).toBe(7);
    // 7 × 1670 = 11 690 < 12 000. Kein achter Tick, kein anteiliger.
    expect(7 * schneller).toBeLessThanOrEqual(12_000);
    expect(8 * schneller).toBeGreaterThan(12_000);
  });

  it('überschreitet mit der Summe der Tick-Zeiten nie die Dauer', () => {
    for (const dauer of [6000, 12_000, 18_000, 30_000]) {
      for (const basis of [1000, 1500, 2000, 3000]) {
        for (let permille = 0; permille < 2000; permille += 13) {
          const abstand = hastedMs(basis, permille);
          const ticks = tickCount(dauer, abstand);
          expect(ticks * abstand, `${dauer} ms, Abstand ${abstand} ms`).toBeLessThanOrEqual(dauer);
          // Ohne diesen Gegenfall wäre die Zeile darüber mit `ticks = 0` immer
          // erfüllt und sagte gar nichts.
          expect((ticks + 1) * abstand).toBeGreaterThan(dauer);
        }
      }
    }
  });

  it('ergibt bei einem Abstand von null keine Ticks statt einer Division durch null', () => {
    expect(tickCount(12_000, 0)).toBe(0);
    expect(Number.isFinite(tickCount(12_000, 0))).toBe(true);
  });
});

describe('snapUp — das Raster, das dem Spiel bis #177 fehlte', () => {
  it('lässt Takt-Grenzen unverändert und schiebt alles andere nach oben', () => {
    for (const t of [0, TICK_MS, 1500, 40_500, 60_000]) {
      expect(snapUp(t), `${t} liegt auf einer Grenze`).toBe(t);
    }
    expect(snapUp(1)).toBe(TICK_MS);
    expect(snapUp(9)).toBe(TICK_MS);
    expect(snapUp(4507)).toBe(4510);
    // Nie abwärts — der Fehler, der den Simulator schneller machte als das Spiel.
    for (let t = 0; t < 500; t++) {
      expect(snapUp(t)).toBeGreaterThanOrEqual(t);
    }
  });
});
