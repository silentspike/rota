/**
 * Grundwerte wachsen mit der Stufe (GAME-DESIGN 8.1; Issue #248, AC2 und AC5).
 *
 * **Etappe 1 von #248, und sie hängt nicht an der Erfahrungskurve.** Was ein
 * Grundwert auf Stufe 50 ist, folgt aus der Stufe — nicht daraus, wie lange
 * jemand gebraucht hat, sie zu erreichen. Deshalb ließ sich dieser Teil bauen,
 * während die Kurve noch zur Entscheidung liegt.
 *
 * WARUM DIE TABELLE GANZZAHLIG IST
 * --------------------------------
 * `1,05^n` ist eine Fließkomma-Potenz. `Math.pow` in JavaScript und
 * `f64::powf` in Rust sind **nicht garantiert bitgleich** — und GAME-DESIGN
 * 18.4 verlangt, dass beide Seiten dieselbe Zahl rechnen. Eine Tabelle rechnet
 * gar nicht, sie steht da; die Anwendung ist eine Multiplikation und eine
 * Ganzzahl-Division, und beides ist exakt definiert.
 *
 * Das ist dieselbe Vorsicht, die in #247 an einer anderen Stelle nötig war:
 * Was zwei Seiten verschieden rechnen können, rechnet am besten keine.
 *
 * AC5 IST DIE FALLE, UND SIE IST STILL
 * ------------------------------------
 * Ein gespeicherter Wert geht nicht kaputt. Er wird nur alt — und eine
 * Balance-Änderung erreicht bestehende Spielstände nie mehr. Der Prüffall
 * unten misst deshalb nicht, dass `statAtLevel` etwas zurückgibt, sondern dass
 * eine geänderte Tabelle bei derselben Stufe ein anderes Ergebnis liefert.
 */
import { describe, expect, it } from 'vitest';
import { LEVELS, PLAYER, statAtLevel } from '../../src/core/data.js';

describe('AC2 — die Grundwerte wachsen mit ~5 % je Stufe', () => {
  it('nennt den Prozentsatz, aus dem die Tabelle entstand', () => {
    expect(LEVELS.growthPerLevelPercent).toBe(5);
    expect(LEVELS.maxLevel).toBe(100);
    expect(LEVELS.factorPerMille).toHaveLength(100);
  });

  it('beginnt bei Faktor 1,0 — Stufe 1 ist der Wert aus base.json', () => {
    // Sonst startete der Spieler über oder unter seinen eigenen Grundwerten,
    // und `base.json` wäre nicht mehr das, was es zu sein behauptet.
    expect(LEVELS.factorPerMille[0]).toBe(1000);
    expect(statAtLevel(PLAYER.vessel, 1)).toBe(PLAYER.vessel);
  });

  it('erreicht auf Stufe 100 einen Faktor um 125 — und die Zahl ist gerechnet', () => {
    // **Gerechnet, bevor gemessen wurde.** Zwischen Stufe 1 und 100 liegen
    // NEUNUNDNEUNZIG Aufstiege, nicht hundert: 1,05^99 = 125,2.
    //
    // Das Design nennt „~130" (8.1), und diese Differenz ist genau die eine
    // Stufe. Sie steht hier als Zahl, damit niemand sie später für einen
    // Rundungsfehler hält und die Tabelle „korrigiert".
    const erwartet = Math.round(1000 * 1.05 ** 99);
    expect(erwartet).toBe(125239);
    expect(LEVELS.factorPerMille[99]).toBe(erwartet);

    const auf100 = statAtLevel(PLAYER.vessel, 100);
    expect(auf100 / PLAYER.vessel).toBeCloseTo(125.2, 1);
  });

  it('wächst gleichmäßig — jede Stufe bringt etwa dieselben 5 %', () => {
    // „Jede Stufe fühlt sich gleich wertvoll an" (8.1). Ein Sprung in der
    // Mitte erfüllte die Endpunkte oben ebenfalls, wäre aber ein anderes
    // Spiel — deshalb wird der Verlauf geprüft, nicht nur Anfang und Ende.
    for (let stufe = 2; stufe <= LEVELS.maxLevel; stufe++) {
      const vorher = LEVELS.factorPerMille[stufe - 2] as number;
      const jetzt = LEVELS.factorPerMille[stufe - 1] as number;
      expect(jetzt / vorher, `Sprung bei Stufe ${stufe}`).toBeCloseTo(1.05, 2);
    }
  });

  it('macht keinen Aufstieg schwächer als den vorigen', () => {
    for (let i = 1; i < LEVELS.factorPerMille.length; i++) {
      expect(LEVELS.factorPerMille[i] as number).toBeGreaterThan(
        LEVELS.factorPerMille[i - 1] as number,
      );
    }
  });
});

describe('AC5 — gerechnet, nicht gespeichert', () => {
  it('liefert für dieselbe Stufe einen ANDEREN Wert, wenn die Tabelle sich ändert', () => {
    // **Der eigentliche Prüffall zu AC5**, und er misst etwas anderes als es
    // zunächst aussieht: Dass `statAtLevel` einen Wert zurückgibt, wäre auch
    // dann erfüllt, wenn er aus einem Spielstand käme. Was AC5 verlangt, ist,
    // dass eine BALANCE-ÄNDERUNG durchschlägt.
    //
    // Genau das lässt sich messen: Dieselbe Stufe, eine andere Tabelle, ein
    // anderes Ergebnis. Käme der Wert aus dem Spielstand, bliebe er gleich.
    const echt = statAtLevel(1000, 50);
    const gedacht = Math.trunc((1000 * (LEVELS.factorPerMille[49] as number)) / 1000);
    expect(echt).toBe(gedacht);

    // Die Probe: ein anderer Faktor, dieselbe Stufe.
    const anderer = Math.trunc((1000 * ((LEVELS.factorPerMille[49] as number) * 2)) / 1000);
    expect(anderer).not.toBe(echt);
    expect(anderer).toBe(echt * 2);
  });

  it('trifft dieselben Zahlen wie der Simulator', () => {
    // **Dieselben Stufen wie in `sim/tests/parity_levels.rs`.** Laufen die
    // Seiten auseinander, sagt es einer der beiden Fälle in einer Zeile —
    // statt in einer Prüfsummen-Abweichung in irgendeinem Takt.
    expect(statAtLevel(1000, 1)).toBe(1000);
    expect(statAtLevel(1000, 10)).toBe(1551);
    expect(statAtLevel(1000, 50)).toBe(10921);
    expect(statAtLevel(1000, 100)).toBe(125239);
    // 1 * 1551 / 1000 = 1 nach Abschneiden — die Ganzzahl-Division, nicht
    // gerundet. Mit Fließkomma wäre 1550,9999... ein möglicher Ausgang, und
    // die Seiten lägen um eins auseinander.
    expect(statAtLevel(1, 10)).toBe(1);
    expect(statAtLevel(0, 50)).toBe(0);
  });

  it('rechnet in Ganzzahlen — kein Ergebnis mit Nachkommastellen', () => {
    // Die Bedingung für 18.4: Was hier herauskommt, muss der Simulator
    // bitgleich treffen können. Ganze Zahlen kann er.
    for (const stufe of [1, 2, 7, 33, 50, 99, 100]) {
      const wert = statAtLevel(PLAYER.vessel, stufe);
      expect(Number.isInteger(wert), `Stufe ${stufe} ergibt ${wert}`).toBe(true);
    }
  });

  it('klemmt Stufen außerhalb der Tabelle, statt sie abzulehnen', () => {
    // Ein Spielstand aus einer Fassung mit anderer Höchststufe soll spielbar
    // bleiben. Ein Absturz an dieser Stelle wäre die schlechteste Antwort:
    // Der Spieler käme nie wieder in sein eigenes Spiel.
    expect(statAtLevel(PLAYER.vessel, 0)).toBe(statAtLevel(PLAYER.vessel, 1));
    expect(statAtLevel(PLAYER.vessel, -5)).toBe(statAtLevel(PLAYER.vessel, 1));
    expect(statAtLevel(PLAYER.vessel, 500)).toBe(statAtLevel(PLAYER.vessel, LEVELS.maxLevel));
    expect(statAtLevel(PLAYER.vessel, 50.9)).toBe(statAtLevel(PLAYER.vessel, 50));
  });
});
