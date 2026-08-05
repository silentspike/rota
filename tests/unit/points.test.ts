/**
 * Die Punkte für einen Sieg (Issue #276, GAME-DESIGN 12.6).
 *
 * WAS DIESE DATEI TRÄGT UND WAS NICHT
 * -----------------------------------
 * Sie prüft die **Form** der Kurve, nicht ihre Höhe. Welcher Exponent gilt,
 * entscheidet der Auftraggeber an gerechneten Vorlagen; die Prüffälle hier
 * müssen jede zulässige Wahl überleben. Deshalb steht nirgends eine erwartete
 * Punktzahl, sondern überall ein Verhältnis.
 *
 * DER EINZIGE PRÜFFALL, DER „ÜBERPROPORTIONAL" TRÄGT, IST AC2
 * ----------------------------------------------------------
 * Eine lineare Formel erfüllt AC1, AC3, AC4, AC6 und AC7 vollständig und ist
 * trotzdem falsch. Der Test zu AC2 baut deshalb die Fehlerart nach: Er rechnet
 * dieselbe Prüfung gegen eine lineare Kurve und verlangt, dass sie DURCHFÄLLT.
 * Ohne diese Gegenprobe wäre nicht zu sehen, ob die Prüfung überhaupt etwas
 * ausschließt — die Lehre aus Issue #328, wo eine Gegenprobe grün blieb, weil
 * sie irgendeinen Schaden veränderte statt die Fehlerart nachzubauen.
 */
import { describe, expect, it } from 'vitest';
import { REWARD } from '../../src/core/data.js';
import { pointsBreakdown, pointsForRun } from '../../src/progression/points.js';

/**
 * Der Quelltext von `points.ts`, über Vite geladen.
 *
 * `node:fs` bräuchte @types/node in der tsconfig — eine geteilte
 * Konfigurationsänderung für einen einzelnen Prüffall. Dasselbe Muster wie in
 * `crit-factors-source.test.ts`.
 */
const QUELLE = import.meta.glob('/src/progression/points.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function punkteQuelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length === 0) {
    throw new Error('points.ts nicht geladen — die Pruefung wuerde nichts messen');
  }
  return text;
}

/** Ein Sieg bei einem gewöhnlichen Boss: die Restzeit trennt. */
function sieg(remainingMs: number): { victory: boolean; remainingMs: number; heldPercent: null } {
  return { victory: true, remainingMs, heldPercent: null };
}

/** Ein Sieg bei einem `runaway`-Boss: die Frist ist voll ausgeschöpft. */
function runawaySieg(heldPercent: number): {
  victory: boolean;
  remainingMs: number;
  heldPercent: number;
} {
  // Restzeit NULL, wie sie bei einem runaway-Boss immer ist. Ein Wert daneben
  // liesse offen, ob die Formel den Ring oder doch die Zeit gelesen hat.
  return { victory: true, remainingMs: 0, heldPercent };
}

/** Boss und Spieler auf derselben Stufe — voller Ertrag. */
const PASSEND = { bossLevel: 10, playerLevel: 10 };

describe('AC1 — ein Sieg zahlt aus der Restlaufzeit', () => {
  it('zahlt mehr, je mehr Zeit übrig war', () => {
    const wenig = pointsForRun(sieg(5_000), PASSEND);
    const viel = pointsForRun(sieg(20_000), PASSEND);
    expect(wenig).toBeGreaterThan(0);
    expect(viel).toBeGreaterThan(wenig);
  });

  it('zahlt für eine Niederlage nichts — auch mit viel Restzeit', () => {
    // 12.6 sagt „Restlaufzeit beim Boss-SIEG". Ein Abbruch nach zwei Sekunden
    // hat die meiste Restzeit von allen.
    expect(pointsForRun({ victory: false, remainingMs: 118_000, heldPercent: null }, PASSEND)).toBe(
      0,
    );
  });

  it('rechnet mit dem Exponenten aus den Daten, nicht mit einer Zahl im Rumpf', () => {
    // Unabhängig nachgerechnet: Wer den Parameter im Code fest verdrahtet,
    // fällt hier, sobald die Datei einen anderen Wert trägt.
    for (const sekunden of [3, 7, 12, 30]) {
      const erwartet = Math.round(
        REWARD.pointsPerSecond * Math.pow(sekunden, REWARD.pointsExponent),
      );
      expect(pointsBreakdown(sieg(sekunden * 1000), PASSEND).base).toBe(erwartet);
    }
  });

  it('trägt den Exponenten nirgends als Zahl im Quelltext', () => {
    // Die Nachrechnung oben bliebe grün, wenn Code und Datei zufällig
    // denselben Wert trügen. Deshalb zusätzlich der Blick in den Rumpf: Die
    // Potenz muss den benannten Wert lesen.
    const quelle = punkteQuelle();
    const potenzen = [...quelle.matchAll(/Math\.pow\(([^)]*)\)/g)].map((t) => t[1] ?? '');
    expect(potenzen.length).toBeGreaterThan(0);
    for (const argumente of potenzen) {
      expect(argumente).toContain('REWARD.pointsExponent');
    }
  });
});

describe('AC2 — überproportional, und zwar messbar', () => {
  /**
   * Die Prüfung selbst, gegen eine beliebige Kurve.
   *
   * Zwei Siege mit 10 s Unterschied müssen **mehr als doppelt** so verschieden
   * auszahlen wie zwei mit 5 s. Das ist genau die strenge Konvexität, und sie
   * gilt bei einer solchen Kurve für JEDE Ausgangszeit — deshalb prüft der
   * Test mehrere und nicht eine günstige.
   */
  function verhaeltnisBei(kurve: (sekunden: number) => number, ab: number): number {
    const zehn = kurve(ab + 10) - kurve(ab);
    const fuenf = kurve(ab + 5) - kurve(ab);
    return zehn / fuenf;
  }

  const echt = (sekunden: number): number => pointsForRun(sieg(sekunden * 1000), PASSEND);
  const linear = (sekunden: number): number => Math.round(REWARD.pointsPerSecond * sekunden);

  it('ist bei jeder Ausgangszeit mehr als doppelt so verschieden', () => {
    for (const ab of [0, 5, 10, 20, 40, 60]) {
      expect(verhaeltnisBei(echt, ab)).toBeGreaterThan(2);
    }
  });

  it('GEGENPROBE: eine lineare Kurve fällt durch — sie trifft exakt das Doppelte', () => {
    // Der Grund, warum diese Zeile hier steht: Ohne sie wäre nicht zu sehen,
    // ob die Prüfung oben überhaupt etwas ausschließt. Eine Kurve, die alles
    // besteht, prüft nichts.
    for (const ab of [0, 5, 10, 20, 40, 60]) {
      expect(verhaeltnisBei(linear, ab)).toBeCloseTo(2, 6);
      expect(verhaeltnisBei(linear, ab)).not.toBeGreaterThan(2);
    }
  });

  it('die letzten Sekunden sind mehr wert als die ersten', () => {
    // Dieselbe Aussage von der anderen Seite, in den Worten des Designs:
    // „die letzten Sekunden sind unverhältnismäßig viel wert".
    const frueh = echt(5) - echt(0);
    const spaet = echt(45) - echt(40);
    expect(spaet).toBeGreaterThan(frueh);
  });
});

describe('AC3 — beim runaway-Boss zählt der Ring, nicht die Zeit', () => {
  it('zahlt aus heldPercent, obwohl die Restzeit null ist', () => {
    // Bei einem runaway-Boss ist die Frist immer voll ausgeschöpft. Eine
    // Formel, die nur die Restzeit liest, zahlte hier für jeden Sieg dasselbe:
    // nichts.
    const knapp = pointsForRun(runawaySieg(10), PASSEND);
    const gut = pointsForRun(runawaySieg(80), PASSEND);
    expect(knapp).toBeGreaterThan(0);
    expect(gut).toBeGreaterThan(knapp);
  });

  it('GEGENPROBE: derselbe Lauf ohne Ring zahlt nichts', () => {
    // Die Fehlerart nachgebaut: Wenn die Formel doch die Restzeit läse, wäre
    // dieser Wert derselbe wie oben. Er ist null, weil die Restzeit null ist.
    expect(pointsForRun({ victory: true, remainingMs: 0, heldPercent: null }, PASSEND)).toBe(0);
  });

  it('überproportional auch aus dem Ring', () => {
    // Dieselbe Kurve, dieselbe Aussage — ein Ring-Rest darf nicht linear
    // zahlen, während die Zeit es nicht tut.
    const kurve = (prozent: number): number => pointsForRun(runawaySieg(prozent), PASSEND);
    for (const ab of [0, 10, 30, 60]) {
      const zehn = kurve(ab + 10) - kurve(ab);
      const fuenf = kurve(ab + 5) - kurve(ab);
      expect(zehn / fuenf).toBeGreaterThan(2);
    }
  });
});

describe('AC4 — der abnehmende Ertrag über den Stufenabstand', () => {
  const LAUF = sieg(30_000);
  const voll = pointsForRun(LAUF, { bossLevel: 10, playerLevel: 10 });

  it('zahlt bei 10 Stufen Abstand messbar weniger', () => {
    const zehn = pointsForRun(LAUF, { bossLevel: 10, playerLevel: 20 });
    expect(zehn).toBeGreaterThan(0);
    expect(zehn).toBeLessThan(voll / 2);
  });

  it('zahlt bei 20 Stufen Abstand praktisch nichts', () => {
    const zwanzig = pointsForRun(LAUF, { bossLevel: 10, playerLevel: 30 });
    expect(zwanzig).toBeLessThan(voll / 20);
  });

  it('GEGENPROBE: bei passendem Boss ändert sich NICHTS', () => {
    // Die Auflage aus dem Issue, und sie ist der eigentliche Nachweis: Eine
    // Kurve, die überall mindert, bestünde beide Prüfungen oben. Hier muss der
    // Wert identisch sein — nicht „ungefähr", sondern derselbe.
    for (const abstand of [0, 1, 2]) {
      expect(pointsForRun(LAUF, { bossLevel: 10, playerLevel: 10 + abstand })).toBe(voll);
    }
  });

  it('GEGENPROBE: ein Boss ÜBER der eigenen Stufe zahlt voll', () => {
    // Der abnehmende Ertrag zielt auf Inhalt, dem man entwachsen ist. Ein
    // schwererer Boss ist nicht weniger wert — sonst bestrafte die Regel
    // genau den Spieler, den sie lenken soll.
    expect(pointsForRun(LAUF, { bossLevel: 30, playerLevel: 10 })).toBe(voll);
  });

  it('sinkt monoton, nicht sprunghaft', () => {
    let vorher = Number.POSITIVE_INFINITY;
    for (let abstand = 0; abstand < 40; abstand++) {
      const wert = pointsForRun(LAUF, { bossLevel: 10, playerLevel: 10 + abstand });
      expect(wert).toBeLessThanOrEqual(vorher);
      vorher = wert;
    }
  });
});

describe('Die Reihenfolge: erst die Kurve, dann der Abstand', () => {
  it('mindert das Ergebnis der Kurve, nicht ihre Eingabe', () => {
    // Wer zuerst mindert und dann potenziert, verstärkt den Abstand mit dem
    // Exponenten: Aus 20,5 % würden bei Exponent 2 gut 4 %. Die Aussage von
    // 12.6 („ein Bruchteil bei ~10 Stufen") wäre dann eine andere.
    const teile = pointsBreakdown(sieg(30_000), { bossLevel: 10, playerLevel: 20 });
    expect(teile.awarded).toBe(Math.trunc((teile.base * teile.falloffPerMille) / 1000));
  });
});

describe('AC5 (#377) — Differenz-Auszahlung statt voller Ertrag bei jedem Sieg', () => {
  const ERSTER = 30_000;

  it('der erste Sieg zahlt voll — ohne Bestwert ist die Differenz der ganze Wert', () => {
    const ohne = pointsForRun(sieg(ERSTER), PASSEND, null);
    expect(ohne).toBe(pointsBreakdown(sieg(ERSTER), PASSEND).base);
  });

  it('GEGENPROBE aus dem Issue: derselbe Lauf zweimal zahlt beim zweiten Mal nur den Rest-Ertrag', () => {
    const erster = pointsForRun(sieg(ERSTER), PASSEND, null);
    const zweiter = pointsForRun(sieg(ERSTER), PASSEND, ERSTER);

    expect(zweiter).toBeGreaterThan(0);
    expect(zweiter).toBeLessThan(erster);
    // Und zwar genau der benannte Anteil, nicht irgendetwas Kleineres.
    const teile = pointsBreakdown(sieg(ERSTER), PASSEND, ERSTER);
    expect(teile.repeatOnly).toBe(true);
    expect(zweiter).toBe(Math.trunc((teile.base * REWARD.repeatPerMille) / 1000));
  });

  it('eine Verbesserung zahlt die DIFFERENZ, nicht den vollen Wert', () => {
    const teile = pointsBreakdown(sieg(40_000), PASSEND, ERSTER);
    expect(teile.repeatOnly).toBe(false);
    expect(teile.awarded).toBe(teile.base - teile.previousBase);
    // Weniger als ein erster Sieg mit derselben Zeit — sonst wäre die
    // Differenz-Auszahlung wirkungslos.
    expect(teile.awarded).toBeLessThan(pointsForRun(sieg(40_000), PASSEND, null));
  });

  it('ein SCHLECHTERER Lauf zahlt auch nur den Rest-Ertrag', () => {
    const teile = pointsBreakdown(sieg(10_000), PASSEND, ERSTER);
    expect(teile.repeatOnly).toBe(true);
    expect(teile.awarded).toBeGreaterThan(0);
  });

  it('der Rest-Ertrag ist anteilig — ein guter Wiederholungslauf bringt mehr als ein schwacher', () => {
    // Der Grund, warum es ein Anteil ist und kein fester Betrag: Sonst wäre
    // jeder Sieg gleich viel wert, sobald die Marke einmal steht.
    const knapp = pointsForRun(sieg(29_000), PASSEND, ERSTER);
    const schwach = pointsForRun(sieg(5_000), PASSEND, ERSTER);
    expect(knapp).toBeGreaterThan(schwach);
  });

  it('die Differenz rechnet auf den UNGEMINDERTEN Werten', () => {
    // Der Abstandsfaktor kann sich zwischen zwei Läufen ändern, weil der
    // Spieler aufsteigt. Auf geminderten Werten zu subtrahieren hiesse, einen
    // Aufstieg als Verschlechterung zu verbuchen — der Spieler verlöre Punkte
    // dafür, dass er besser geworden ist.
    const fern = { bossLevel: 10, playerLevel: 20 };
    const teile = pointsBreakdown(sieg(40_000), fern, ERSTER);
    const roh = teile.base - teile.previousBase;
    expect(teile.awarded).toBe(Math.trunc((roh * teile.falloffPerMille) / 1000));
  });

  it('beim runaway-Boss rechnet die Differenz aus dem Ring, nicht aus der Zeit', () => {
    const erster = pointsForRun(runawaySieg(50), PASSEND, null);
    const gleich = pointsForRun(runawaySieg(50), PASSEND, 50);
    const besser = pointsForRun(runawaySieg(70), PASSEND, 50);

    expect(gleich).toBeLessThan(erster);
    expect(besser).toBeGreaterThan(gleich);
    // Die Differenz gegen den gespeicherten PROZENTWERT — hielte die Rechnung
    // ihn für Millisekunden, wäre der Abzug praktisch null und `besser` läge
    // beim vollen Ertrag.
    expect(besser).toBeLessThan(pointsForRun(runawaySieg(70), PASSEND, null));
  });

  it('eine Niederlage zahlt weiterhin nichts — auch mit Bestwert', () => {
    expect(
      pointsForRun({ victory: false, remainingMs: 118_000, heldPercent: null }, PASSEND, 5_000),
    ).toBe(0);
  });
});
