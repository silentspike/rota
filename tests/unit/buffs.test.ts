import { describe, expect, it } from 'vitest';
import { BUFF_KIND_CAPACITY, Buffs } from '../../src/combat/schools/buffs.js';
import { GCD_MS, GlobalCooldown } from '../../src/core/gcd.js';

describe('Synergie-Fenster in Wartezeiten (AC1)', () => {
  it('rechnet die Länge aus der aktuellen Wartezeit', () => {
    const buffs = new Buffs();
    const gcd = new GlobalCooldown();
    // Drei Wartezeiten bei 1,5 s Grundtakt = 4,5 s (GAME-DESIGN 5).
    buffs.applyForGcds(0, 1.2, 3, 0, gcd.lengthMs);
    expect(buffs.remainingMs(0, 0)).toBe(4500);
  });

  it('halbiert das Fenster in Sekunden, wenn die Wartezeit halbiert wird', () => {
    // Der Kern von AC1: Dasselbe Fenster — drei Wartezeiten — ist bei
    // halbiertem Grundtakt halb so lang in Sekunden. In Zaubern gerechnet
    // bleibt es dasselbe Fenster, und genau das ist der Zweck.
    const gcd = new GlobalCooldown();
    const voll = new Buffs();
    voll.applyForGcds(0, 1.2, 3, 0, gcd.lengthMs);
    const beiVollemTakt = voll.remainingMs(0, 0);

    gcd.setLength(GCD_MS / 2);
    const halb = new Buffs();
    halb.applyForGcds(0, 1.2, 3, 0, gcd.lengthMs);

    expect(beiVollemTakt).toBe(4500);
    expect(halb.remainingMs(0, 0)).toBe(2250);
    expect(halb.remainingMs(0, 0)).toBe(beiVollemTakt / 2);
  });

  it('lässt gleich viele Zauber ins Fenster, gleich wie schnell der Takt ist', () => {
    // Die eigentliche Aussage hinter AC1, in Zaubern statt in Millisekunden:
    // Ein Fenster von drei Wartezeiten fasst drei Zauber — bei jedem Tempo.
    for (const laenge of [1500, 1200, 1000]) {
      const gcd = new GlobalCooldown();
      gcd.setLength(laenge);
      const buffs = new Buffs();
      buffs.applyForGcds(0, 1.2, 3, 0, gcd.lengthMs);

      let zauber = 0;
      for (let t = 0; buffs.isActive(0, t); t += gcd.lengthMs) zauber++;
      expect(zauber, `bei ${laenge} ms Wartezeit`).toBe(3);
    }
  });

  it('läuft nach dem Fenster ab, nicht davor', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 2, 0, GCD_MS);
    expect(buffs.isActive(0, 2999)).toBe(true);
    // Genau am Ablauf ist es vorbei — dieselbe Kante wie bei den Abklingzeiten.
    expect(buffs.isActive(0, 3000)).toBe(false);
    expect(buffs.remainingMs(0, 3000)).toBe(0);
  });
});

describe('Stapel-Regel (AC2)', () => {
  it('stapelt verschiedene Arten multiplikativ', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
    buffs.applyForGcds(1, 1.5, 3, 0, GCD_MS);
    // 1,2 × 1,5 = 1,8 — nicht 1,7 (additiv) und nicht 1,5 (stärkste gewinnt).
    expect(buffs.factor(0)).toBeCloseTo(1.8, 12);
    expect(buffs.activeCount(0)).toBe(2);
  });

  it('lässt gleiche Arten einander ersetzen', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
    buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
    buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
    // Dreimal dieselbe Art: der Faktor bleibt 1,2 und wird nicht 1,728.
    expect(buffs.factor(0)).toBeCloseTo(1.2, 12);
    expect(buffs.activeCount(0)).toBe(1);
    expect(buffs.replacementCount).toBe(2);
  });

  it('ersetzt auch, wenn die neue Verstärkung schwächer ist', () => {
    // „Ersetzen einander" steht ohne Einschränkung im Design. Eine Regel
    // „die stärkere gewinnt" wäre eine zweite, die dort nicht steht.
    const buffs = new Buffs();
    buffs.applyForGcds(0, 2, 3, 0, GCD_MS);
    buffs.applyForGcds(0, 1.1, 3, 0, GCD_MS);
    expect(buffs.factor(0)).toBeCloseTo(1.1, 12);
  });

  it('setzt beim Ersetzen auch das Fenster neu', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
    // Nach 3 s erneut: das Fenster läuft wieder volle 4,5 s.
    buffs.applyForGcds(0, 1.2, 3, 3000, GCD_MS);
    expect(buffs.remainingMs(0, 3000)).toBe(4500);
    expect(buffs.isActive(0, 7000)).toBe(true);
  });

  it('rechnet ohne jede Verstärkung mit genau eins', () => {
    // Der neutrale Fall muss die exakte Eins sein, nicht etwas nahe daran:
    // Er geht als Faktor in die Schadensformel ein, und dort ist ×1 die
    // einzige Multiplikation, die nichts verändert (GAME-DESIGN 6.3).
    const buffs = new Buffs();
    expect(buffs.factor(0)).toBe(1);
  });

  it('nimmt eine abgelaufene Verstärkung aus dem Produkt', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 2, 0, GCD_MS);
    buffs.applyForGcds(1, 1.5, 3, 0, GCD_MS);
    expect(buffs.factor(0)).toBeCloseTo(1.8, 12);
    // Bei 3000 ms ist die erste vorbei, die zweite läuft bis 4500.
    expect(buffs.factor(3000)).toBeCloseTo(1.5, 12);
  });

  it('multipliziert in fester Reihenfolge', () => {
    // Fließkomma-Multiplikation ist nicht assoziativ. Die Reihenfolge ist die
    // der Arten, damit Spiel und Simulator auf dasselbe Bit kommen.
    const a = new Buffs();
    a.apply(0, 1.1, 10_000, 0);
    a.apply(1, 1.3, 10_000, 0);
    a.apply(2, 1.7, 10_000, 0);
    const b = new Buffs();
    b.apply(2, 1.7, 10_000, 0);
    b.apply(1, 1.3, 10_000, 0);
    b.apply(0, 1.1, 10_000, 0);
    expect(a.factor(0)).toBe(b.factor(0));
  });
});

describe('Ablauf und Vorrat', () => {
  it('meldet abgelaufene Verstärkungen, statt sie still fallenzulassen', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 1, 0, GCD_MS);
    buffs.applyForGcds(1, 1.5, 2, 0, GCD_MS);

    // **Der Rückgabewert ist seit #682 eine BITMASKE, keine Anzahl:**
    // Bit `k` gesetzt heisst „Art `k` ist abgelaufen". Deshalb liefert der
    // Ablauf von Art 0 eine 1 und der von Art 1 eine 2 — vorher stand hier
    // zweimal die 1, weil beide Male genau eine Art ablief.
    //
    // `0` heisst weiterhin „keine", und `popcount` ist die frühere Anzahl.
    expect(buffs.expire(1000), 'noch läuft keine ab').toBe(0);
    expect(buffs.expire(1500), 'Art 0 läuft ab — Bit 0').toBe(0b01);
    expect(buffs.expire(1500), 'zweimal derselbe Ablauf zählt nicht doppelt').toBe(0);
    expect(buffs.expire(3000), 'Art 1 läuft ab — Bit 1, nicht wieder eine 1').toBe(0b10);
    expect(buffs.expirationCount).toBe(2);
  });

  it('weist eine Art außerhalb des Vorrats zurück, statt sie zu verschlucken', () => {
    const buffs = new Buffs();
    expect(buffs.apply(BUFF_KIND_CAPACITY, 2, 1000, 0)).toBe(false);
    expect(buffs.apply(-1, 2, 1000, 0)).toBe(false);
    expect(buffs.apply(1.5, 2, 1000, 0)).toBe(false);
    expect(buffs.factor(0)).toBe(1);
  });

  it('setzt beim Zurücksetzen alles auf neutral', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
    buffs.applyForGcds(1, 1.5, 3, 0, GCD_MS);
    buffs.reset();
    expect(buffs.factor(0)).toBe(1);
    expect(buffs.activeCount(0)).toBe(0);
    expect(buffs.replacementCount).toBe(0);
  });
});
