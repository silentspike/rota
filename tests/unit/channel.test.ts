/**
 * Kanalisierungen — der vierte Zauber-Typ (Issue #323, GAME-DESIGN 7.2).
 *
 * # Die Regel
 *
 * | Typ | Wirkung fällt an | bei Abbruch |
 * | --- | --- | --- |
 * | Cast (`castTimeMs`) | am Ende, vollständig | alles verloren |
 * | **Kanalisierung** | **laufend, über die Dauer** | **der erreichte Anteil bleibt** |
 *
 * Beispiel des Auftraggebers, wörtlich: *„Eine Kanalisierung über 5 s für 500
 * Heilung, abgebrochen nach 3 s, hat 300 gegeben."*
 *
 * # Design-Gesetz 1 bleibt unberührt
 *
 * Verboten ist das **Tastenhalten**, nicht die Kanalisierung (GAME-DESIGN 108).
 * Sie startet mit EINEM Druck und läuft von selbst — es gibt in dieser Datei
 * keinen Fall, der eine Taste hält, weil es im Code keinen Zustand dafür gibt.
 * Was nach dem Druck geschieht, hängt allein an der Zeit.
 *
 * # Warum die erwarteten Zahlen gerechnet und nicht abgeschrieben sind
 *
 * Der Issue verlangt es ausdrücklich: „Der Prüffall rechnet den erwarteten
 * Anteil **aus der Dauer**, nicht als Literal — sonst ist es die nächste
 * Handabschrift." Alle Erwartungen hier leiten sich aus `channelMs`,
 * `channelTickMs` und `baseDamage` des Zaubers ab. Die 300 aus dem Beispiel
 * stehen nur an einer Stelle als Zahl: in der Zusicherung, dass der Prüfzauber
 * wirklich der aus dem Design ist.
 */

import { describe, expect, it } from 'vitest';
import { alsSpellIndex, CAST_OK, CAST_STARTED, Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';

const BOSS = bossById('test-warden') as BossDef;

function kampf(): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
  combat.beginFight(0);
  return combat;
}

/** Die Kennzahlen des Prüfzaubers — abgelesen, nicht angenommen. */
function kanalDaten(combat: Combat) {
  const index = combat.indexOf('test-channel');
  const spell = combat.spellAt(index);
  const dauer = spell?.channelMs ?? 0;
  const raster = spell?.channelTickMs ?? 0;
  const gesamt = spell?.baseDamage ?? 0;
  const schritte = raster > 0 ? Math.floor(dauer / raster) : 0;
  return { index, dauer, raster, gesamt, schritte, jeSchritt: Math.floor(gesamt / schritte) };
}

/** Kanalisierung wirken und bis `bisMs` laufen lassen. */
function kanalBis(bisMs: number): { combat: Combat; schaden: number } {
  const combat = kampf();
  const { index } = kanalDaten(combat);
  expect(combat.cast(index, 0).result, 'die Kanalisierung wurde abgelehnt').toBe(CAST_STARTED);
  for (let t = 0; t <= bisMs; t += 10) combat.stepPassive(t);
  return { combat, schaden: combat.damageDealt };
}

describe('AC0 — ZUSICHERUNG: der Prüfzauber IST der aus dem Design-Beispiel', () => {
  it('5 s, 500 Gesamtwirkung, fünf Schritte à 100', () => {
    // Ohne diesen Fall prüften alle folgenden gegen irgendwelche Zahlen. Das
    // Design nennt 5 s und 500; nur dann ist „300 nach 3 s" das Beispiel und
    // nicht ein Zufall.
    const d = kanalDaten(kampf());
    expect(d.dauer).toBe(5000);
    expect(d.gesamt).toBe(500);
    expect(d.schritte).toBe(5);
    expect(d.jeSchritt).toBe(100);
  });

  it('und er trägt KEINE Wirkzeit — beides zugleich lehnt der Parser ab', () => {
    expect(kampf().spellAt(kampf().indexOf('test-channel'))?.castTimeMs).toBe(0);
    expect(CALCINATION.problems).toEqual([]);
  });
});

describe('AC1 — die Wirkung fällt LAUFEND an, nicht am Ende', () => {
  it('nach der Hälfte der Dauer ist die Hälfte da', () => {
    const d = kanalDaten(kampf());
    const { schaden } = kanalBis(d.dauer / 2);
    // Aus der Dauer gerechnet: Nach 2500 ms sind zwei Schritte gefallen (bei
    // 1000 und 2000), der dritte kommt erst bei 3000.
    const erwartet = Math.floor(d.dauer / 2 / d.raster) * d.jeSchritt;
    expect(schaden).toBe(erwartet);
  });

  it('und vor dem ersten Raster-Schritt ist es NICHTS', () => {
    // Der Gegenpol: Ein Kanal, der sofort etwas gäbe, wäre ein Sofort-Zauber
    // mit Nachlauf. Die erste Teilwirkung fällt nach dem ersten Schritt.
    const d = kanalDaten(kampf());
    expect(kanalBis(d.raster - 10).schaden).toBe(0);
  });

  it('ein Cast dagegen gibt bis zum Ende GAR nichts — der Unterschied', () => {
    // Die Gegenprobe zum Typ: `test-cast` trägt eine Wirkzeit und löst erst
    // am Ende auf. Ohne sie zeigte AC1 nur, dass irgendwann Schaden kommt.
    const combat = kampf();
    const cast = combat.indexOf('test-cast');
    const wirkzeit = combat.spellAt(cast)?.castTimeMs ?? 0;
    expect(wirkzeit, 'test-cast hat keine Wirkzeit — dann prüft der Fall nichts').toBeGreaterThan(
      0,
    );
    expect(combat.cast(cast, 0).result).toBe(CAST_STARTED);
    for (let t = 0; t <= wirkzeit - 10; t += 10) combat.stepPassive(t);
    expect(combat.damageDealt).toBe(0);
  });
});

describe('AC2 — bei Abbruch bleibt der erreichte Anteil', () => {
  it('das Beispiel des Auftraggebers: nach 3 von 5 Schritten bleiben 3/5', () => {
    const d = kanalDaten(kampf());
    const combat = kampf();
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);

    // Drei Schritte laufen lassen …
    const abbruchAt = 3 * d.raster;
    for (let t = 0; t <= abbruchAt; t += 10) combat.stepPassive(t);
    const vorAbbruch = combat.damageDealt;

    // … dann eine eigene Wirkung: Sie bricht ab (GAME-DESIGN 7.2).
    const anderer = combat.indexOf('kindle');
    expect(combat.cast(anderer, abbruchAt).result, 'KINDLE wurde abgelehnt').toBe(CAST_OK);
    const nachAbbruch = combat.damageDealt;

    // … und danach fällt nichts mehr.
    for (let t = abbruchAt; t <= d.dauer + 2000; t += 10) combat.stepPassive(t);

    // Der Anteil aus der Dauer gerechnet: drei Schritte.
    const anteil = 3 * d.jeSchritt;
    expect(vorAbbruch).toBe(anteil);
    // Was KINDLE selbst beigetragen hat, gehört nicht zur Kanalisierung —
    // deshalb wird ab `nachAbbruch` gemessen, nicht ab `vorAbbruch`.
    expect(combat.damageDealt).toBe(nachAbbruch);
  });

  it('die Zahl aus dem Design: 300 von 500', () => {
    // Dieselbe Aussage, einmal in den Zahlen des Auftraggebers. Sie steht
    // hier UND oben gerechnet: Stimmt eine der beiden nicht mehr, ist
    // entweder die Rechnung falsch oder der Prüfzauber ein anderer.
    const d = kanalDaten(kampf());
    expect(3 * d.jeSchritt).toBe(300);
    expect(d.gesamt).toBe(500);
  });
});

describe('AC3 — läuft sie durch, kommt die volle Wirkung an', () => {
  it('nach der ganzen Dauer ist alles da', () => {
    const d = kanalDaten(kampf());
    const { schaden } = kanalBis(d.dauer);
    expect(schaden).toBe(d.schritte * d.jeSchritt);
  });

  it('und danach wächst nichts mehr nach', () => {
    // Ein Kanal, der über sein Ende hinaus tickte, gäbe mehr als seine
    // Gesamtwirkung — der Fall oben allein sähe das nicht.
    const d = kanalDaten(kampf());
    expect(kanalBis(d.dauer + 3 * d.raster).schaden).toBe(d.schritte * d.jeSchritt);
  });

  it('auch bei einem SPRUNG weit über das Ende hinaus', () => {
    // **Der Fall, den ein 10-ms-Takt nie zeigt.** Die Fälle oben rufen
    // `stepPassive` in Zehner-Schritten; die Schleife holt dann höchstens
    // einen Termin je Aufruf nach. Springt die Zeit dagegen in einem Satz
    // über das Kanal-Ende, muss die Schleife von sich aus bei der Dauer
    // aufhören — sonst zahlt sie so viele Schritte aus, wie zwischen Beginn
    // und Sprungziel Raster passen.
    //
    // Gemessen: Ohne die Grenze `<= playerCastEndsAtMs` bleibt jeder andere
    // Fall dieser Datei grün. Dieser nicht.
    const combat = kampf();
    const d = kanalDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    combat.stepPassive(d.dauer + 10 * d.raster);
    expect(combat.damageDealt).toBe(d.schritte * d.jeSchritt);
  });
});

describe('AC4 — jede eigene Wirkung bricht ab, auch das Unterbrechen', () => {
  /** Wirkt `zauber` mitten in der Kanalisierung und gibt zurück, ob sie noch läuft. */
  function bricht(zauber: string): boolean {
    const combat = kampf();
    const d = kanalDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    const mitten = 2 * d.raster;
    for (let t = 0; t <= mitten; t += 10) combat.stepPassive(t);
    expect(combat.playerCasting, 'die Kanalisierung lief gar nicht').toBeGreaterThanOrEqual(0);
    combat.cast(combat.indexOf(zauber), mitten);
    return combat.playerCasting >= 0;
  }

  it('ein gewöhnlicher Zauber bricht sie ab', () => {
    expect(bricht('kindle')).toBe(false);
  });

  it('das UNTERBRECHEN ebenfalls — und das ist der Fall, den man vergisst', () => {
    // Es löst keine globale Wartezeit aus (GAME-DESIGN 473) und ist damit die
    // einzige Taste, die jederzeit drückbar ist. Sie bricht trotzdem ab.
    const combat = kampf();
    const unterbrecher = [...Array(60).keys()]
      .map((i) => combat.spellAt(alsSpellIndex(i)))
      .find((s) => s?.interrupt === true);
    expect(unterbrecher, 'kein unterbrechender Zauber im Katalog').toBeDefined();
    expect(bricht(unterbrecher?.id ?? '')).toBe(false);
  });

  it('GEGENPROBE: ohne eigene Wirkung läuft sie weiter', () => {
    // Ohne diese Zeile bewiese AC4 nur, dass sie irgendwann endet.
    const combat = kampf();
    const d = kanalDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    for (let t = 0; t <= 2 * d.raster; t += 10) combat.stepPassive(t);
    expect(combat.playerCasting).toBeGreaterThanOrEqual(0);
  });
});
