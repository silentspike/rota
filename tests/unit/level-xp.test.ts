/**
 * Erfahrung und Stufenaufstieg (GAME-DESIGN 12.1; Issue #248, AC1).
 *
 * DIE KURVE IST GERECHNET, NICHT GERATEN — und dieser Prüffall rechnet sie
 * nach, statt die Zahlen abzuschreiben. Eine Tabelle, die niemand nachrechnet,
 * ist eine Behauptung: Sie stimmt genau so lange, wie sich niemand vertippt.
 *
 * Die Vorgabe aus dem Plan sind ~24 Stunden für Stufe 1 bis 100. Der Fall
 * unten prüft die ganze Strecke — nicht drei Stichproben, denn eine Kurve mit
 * einem Knick in der Mitte hätte dieselben Endpunkte.
 *
 * ERREICHBARKEIT, wie es die Auflage verlangt: Stufe 100 wird hier wirklich
 * erreicht. Eine Kurve, bei der der Prüflauf auf Stufe 60 hängen bleibt,
 * machte AC2 zu einem Kriterium, das nie etwas berührt.
 */
import { describe, expect, it } from 'vitest';
import { LEVELS } from '../../src/core/data.js';
import {
  grantXp,
  levelProgress,
  newLevelState,
  START_LEVEL,
  xpForVictory,
  xpToNext,
} from '../../src/progression/level.js';

describe('AC1 — ein Sieg gibt Erfahrung, genug davon hebt die Stufe', () => {
  it('beginnt auf Stufe 1 ohne Erfahrung', () => {
    expect(newLevelState()).toEqual({ level: START_LEVEL, xp: 0 });
    expect(START_LEVEL).toBe(1);
  });

  it('rechnet die GRUNDMENGE an der BOSS-Stufe, nicht an der eigenen', () => {
    // GAME-DESIGN 12.6: Einen Stufenabstand gibt es nur, wenn beide Seiten
    // eine Stufe haben — hinge die Erfahrung am Spieler, wäre die Regel gar
    // nicht baubar. Hier steht die Grundmenge, ohne Abstand.
    expect(xpForVictory(1, 1)).toBe(100);
    expect(xpForVictory(10, 10)).toBe(1000);
    // Ein Boss ÜBER der eigenen Stufe zahlt ebenfalls voll — der abnehmende
    // Ertrag zielt auf Inhalt, dem man entwachsen ist, nicht auf schweren.
    expect(xpForVictory(10, 1)).toBe(1000);
  });

  it('mindert sie um den Stufenabstand (Issue #276)', () => {
    // **Bis Issue #276 stand hier das Gegenteil**: „die Spieler-Stufe wirkt
    // heute nicht" war richtig, solange `xpForVictory` sie mit `void`
    // wegwarf. Der abnehmende Ertrag ist jetzt gebaut, und zwar als EINE
    // Kurve für Erfahrung und Punkte zusammen — die Zusicherung dazu steht in
    // `reward-falloff.test.ts`, weil sie beide Währungen braucht.
    expect(xpForVictory(10, 99)).toBeLessThan(1000);
    expect(xpForVictory(10, 20)).toBeLessThan(xpForVictory(10, 12));
    expect(xpForVictory(10, 12)).toBe(1000);
  });

  it('steigt auf, sobald der Bedarf erreicht ist', () => {
    // Stufe 1 verlangt 200; ein Sieg über einen Boss der Stufe 1 gibt 100.
    // Zwei Siege heben also die Stufe — die Zahl aus der Variantentabelle.
    expect(xpToNext(1)).toBe(200);
    let stand = newLevelState();
    stand = grantXp(stand, xpForVictory(1, stand.level));
    expect(stand).toEqual({ level: 1, xp: 100 });
    stand = grantXp(stand, xpForVictory(1, stand.level));
    expect(stand).toEqual({ level: 2, xp: 0 });
  });

  it('nimmt überschüssige Erfahrung mit, statt sie verfallen zu lassen', () => {
    // **Kostet 42 Kämpfe über den ganzen Aufstieg** (1323 statt 1365) und
    // bestraft den Spieler dafür, dass er einen starken Boss gewählt hat.
    const stand = grantXp({ level: 1, xp: 190 }, 1000);
    expect(stand.level).toBeGreaterThan(1);
    expect(stand.xp, 'der Rest zählt weiter').toBeGreaterThan(0);
  });

  it('steigt MEHRERE Stufen, wenn ein Sieg dafür reicht', () => {
    // Ein Sieg über einen weit höheren Boss. Wer nur einmal aufsteigt, lässt
    // den Rest liegen — und die Anzeige zeigt mehr Erfahrung, als die Stufe
    // zulässt.
    const stand = grantXp(newLevelState(), 10000);
    expect(stand.level).toBeGreaterThan(3);
    expect(stand.xp).toBeLessThan(xpToNext(stand.level));
  });

  it('lässt denselben Zustand unberührt — zweimal gerechnet, gleiches Ergebnis', () => {
    // 18.4: Ohne Seiteneffekt lässt sich derselbe Sieg zweimal rechnen.
    const anfang = newLevelState();
    const a = grantXp(anfang, 500);
    const b = grantXp(anfang, 500);
    expect(anfang).toEqual({ level: 1, xp: 0 });
    expect(a).toEqual(b);
  });
});

describe('Die Kurve trifft die Vorgabe aus dem Plan', () => {
  it('erreicht Stufe 100 und braucht dafür rund 1323 Kämpfe', () => {
    // **Die ganze Strecke, nicht drei Stichproben.** Gespielt wird gegen einen
    // Boss der eigenen Stufe — so ist die Instanz-Struktur aus 12.4 gedacht
    // (acht Instanzen à vier Bosse tragen Stufe 1 bis 100).
    let stand = newLevelState();
    let kaempfe = 0;
    while (stand.level < LEVELS.maxLevel && kaempfe < 100_000) {
      stand = grantXp(stand, xpForVictory(stand.level, stand.level));
      kaempfe++;
    }

    expect(stand.level, 'Stufe 100 muss wirklich erreichbar sein').toBe(LEVELS.maxLevel);
    // Gerechnet: Summe(200*n^1.5) / (100*n) über n = 1..99 ergibt 1323.
    expect(kaempfe).toBeGreaterThan(1300);
    expect(kaempfe).toBeLessThan(1345);

    // Bei ~60 s je Kampfzyklus sind das rund 22 Stunden — die Vorgabe lautet
    // ~24. Die Spanne der einzigen Schätzung (50 bis 75 s) ergibt 18,4 bis
    // 27,6 h und liegt damit ganz im Zielbereich.
    const stunden = (kaempfe * 60) / 3600;
    expect(stunden).toBeGreaterThan(18);
    expect(stunden).toBeLessThan(28);
  });

  it('wird von Stufe zu Stufe langsamer, ohne zur Wand zu werden', () => {
    // Der Grund, C zu wählen: A war gleichförmig (Stufe 99 wie Stufe 2), B und
    // D ließen die Kampfzahl explodieren (198 bzw. 99 für die letzte Stufe).
    const kaempfeAuf = (n: number): number => xpToNext(n) / xpForVictory(n, n);
    expect(kaempfeAuf(1)).toBeCloseTo(2, 0);
    expect(kaempfeAuf(25)).toBeCloseTo(10, 0);
    expect(kaempfeAuf(50)).toBeCloseTo(14, 0);
    expect(kaempfeAuf(99)).toBeCloseTo(20, 0);

    // Monoton steigend über die ganze Strecke — kein Abschnitt, in dem der
    // Aufstieg wieder schneller geht.
    for (let n = 2; n < LEVELS.maxLevel; n++) {
      expect(kaempfeAuf(n), `Stufe ${n}`).toBeGreaterThan(kaempfeAuf(n - 1));
    }
  });

  it('kennt über der Höchststufe kein Weiter', () => {
    expect(xpToNext(LEVELS.maxLevel)).toBe(Number.POSITIVE_INFINITY);
    const oben = grantXp({ level: LEVELS.maxLevel, xp: 0 }, 1_000_000);
    expect(oben).toEqual({ level: LEVELS.maxLevel, xp: 0 });
    expect(levelProgress(oben), 'der Balken steht voll, statt bei null zu hängen').toBe(1);
  });
});

describe('AC5 — der Zustand trägt nur, was erreicht wurde', () => {
  it('besteht aus genau zwei Zahlen: Stufe und Erfahrung', () => {
    // **Der Prüfstein für AC5.** Käme ein Grundwert, eine Zauberliste oder die
    // Zahl der Leisten-Plätze hier hinein, wäre er gespeichert statt gerechnet
    // — und jede künftige Balance-Änderung ginge an bestehenden Spielständen
    // vorbei, ohne dass etwas kaputtgeht.
    const stand = grantXp(newLevelState(), 5000);
    expect(Object.keys(stand).sort()).toEqual(['level', 'xp']);
  });

  it('rechnet den Anzeige-Anteil, statt ihn zu führen', () => {
    const stand = { level: 1, xp: 100 };
    expect(levelProgress(stand)).toBeCloseTo(0.5, 5);
    expect(Object.keys(stand)).not.toContain('progress');
  });
});
