/**
 * Der Spieler trägt, was im Spielstand steht (Issue #421a).
 *
 * WAS BIS HIERHER FEHLTE
 * ----------------------
 * `worn → essenceFactor → factors` ist seit #282 verdrahtet und wurde zu Recht
 * abgenommen. Aber die Kette beginnt bei `worn`, und **niemand füllte `worn`**:
 * Im ganzen Projekt gab es genau zwei `WornValues`, beide von Hand.
 *
 * Ein leeres `NOTHING_WORN` durchläuft die Kette fehlerfrei und ergibt exakt
 * die Werte eines nackten Spielers — nichts war kaputt, nichts war
 * angeschlossen.
 *
 * DER MESSPUNKT GEHÖRT ZUM WERT
 * -----------------------------
 * Aus der Messung des Orchestrators: Bei VESSEL sieht man den Unterschied
 * schon bei Takt 0 (er hebt `playerHp`, das in der Prüfsumme steht), bei
 * CATALYSIS erst mit laufender Abklingzeit. Ein Prüffall, der pauschal „nach
 * dem Kampf" misst, verfehlt beide — deshalb wird hier je Wert an seiner
 * eigenen Stelle gemessen.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { combatChecksum } from '../../src/core/checksum.js';
import { ITEMS, itemById, wornFrom } from '../../src/gear/catalog.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';

const RELIC = 'test-relic';

/** Ein Stück auf einem Platz. */
function getragen(itemId = RELIC): { itemId: string; upgradeSteps: number; equippedSlot: number } {
  return { itemId, upgradeSteps: 0, equippedSlot: 0 };
}

/** Dasselbe Stück im Beutel. */
function imBeutel(itemId = RELIC): {
  itemId: string;
  upgradeSteps: number;
  equippedSlot: null;
} {
  return { itemId, upgradeSteps: 0, equippedSlot: null };
}

function kampf(
  items: readonly { itemId: string; upgradeSteps: number; equippedSlot: number | null }[] = [],
): Combat {
  return new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    worn: wornFrom(items),
  });
}

describe('Der Katalog trägt keine Zahlen', () => {
  it('ein Eintrag nennt Stufe, Qualität und Werte-Arten', () => {
    // GAME-DESIGN 1437: „Kein Gegenstand wird von Hand balanciert — ein neuer
    // Gegenstand ist ein Eintrag mit Stufe, Qualität und Werte-Arten."
    const relic = itemById(RELIC);
    expect(relic?.itemLevel).toBe(40);
    expect(relic?.quality).toBe(2);
    expect(relic?.affixKinds).toEqual(['catalysis', 'purity', 'yield']);
    // Und ausdrücklich keine Zahl je Wert.
    expect(Object.keys(relic ?? {})).not.toContain('catalysis');
  });

  it('der Prüf-Gegenstand ist noch da — an ihm hängen die Fälle dieser Datei', () => {
    // **Bis #421b stand hier `toEqual([RELIC])`** — „es gibt nur den
    // Prüf-Gegenstand". Das war richtig, solange der Katalog leer war, und ist
    // mit den zwanzig Einträgen aus `docs/AUSRUESTUNGS-RASTER.md` hinfällig.
    //
    // Was bleibt, ist die Aussage, die diese Datei wirklich braucht: `RELIC`
    // existiert und trägt seine Werte-Arten unverändert. Wer ihn löscht, macht
    // jeden Fall hier grün, ohne dass etwas geprüft wäre.
    expect(ITEMS.map((item) => item.id)).toContain(RELIC);
    expect(itemById(RELIC)?.affixKinds).toEqual(['catalysis', 'purity', 'yield']);
  });
});

describe('AC3 — getragene Stücke ergeben Werte, der Kampf bekommt sie', () => {
  it('ein getragenes Stück füllt die Werte', () => {
    const worn = wornFrom([getragen()]);
    // Die drei Grundwerte trägt JEDER Gegenstand (13.5) …
    expect(worn.essence).toBeGreaterThan(0);
    expect(worn.vessel).toBeGreaterThan(0);
    expect(worn.ward).toBeGreaterThan(0);
    // … die Affixe folgen den `affixKinds` des Eintrags.
    expect(worn.catalysis).toBeGreaterThan(0);
    expect(worn.purity).toBeGreaterThan(0);
    expect(worn.yieldBonus).toBeGreaterThan(0);
    // Was der Eintrag NICHT nennt, bleibt leer.
    expect(worn.gearEffects).toBe(0);
  });

  it('GEGENPROBE: ein Stück im Beutel trägt nichts bei', () => {
    // Sonst wäre Sammeln stärker als Ausrüsten — und der Prüffall darüber
    // wäre grün, egal ob `equippedSlot` überhaupt gelesen wird.
    expect(wornFrom([imBeutel()])).toEqual(NOTHING_WORN);
  });

  it('GEGENPROBE: ein unbekanntes Stück wird übergangen, nicht abgebrochen', () => {
    // Ein Spielstand kann ein Stück tragen, das es nach einem Update nicht
    // mehr gibt. Ein Kampf, der deswegen nicht startet, wäre schlimmer.
    expect(wornFrom([getragen('gibt-es-nicht')])).toEqual(NOTHING_WORN);
    expect(wornFrom([getragen('gibt-es-nicht'), getragen()]).essence).toBeGreaterThan(0);
  });

  it('ohne Stücke ist es exakt NOTHING_WORN', () => {
    // **Dieselbe Kennung, nicht nur derselbe Inhalt**: `isNothingWorn` und die
    // Zusicherung „ohne Ausrüstung bewegt sich nichts" hängen daran.
    expect(wornFrom([])).toBe(NOTHING_WORN);
  });
});

describe('AC4 — die Prüfsumme, beide Hälften', () => {
  /**
   * Die zweite Hälfte ist die wichtigere: „ohne Ausrüstung genau wie heute"
   * belegt, dass die Änderung an der Ausrüstung liegt und nichts sonst
   * verschoben wurde. „Mit Ausrüstung anders" allein wäre auch grün, wenn
   * irgendetwas anderes den Lauf verändert.
   */
  function pruefsummeNach(
    items: readonly { itemId: string; upgradeSteps: number; equippedSlot: number | null }[],
    takte: number,
  ): number {
    const combat = kampf(items);
    if (takte > 0) combat.cast(combat.indexOf('kindle'), 0);
    for (let t = 0; t <= takte * 10; t += 10) combat.stepPassive(t);
    return combatChecksum(combat, takte * 10);
  }

  it('VESSEL zeigt sich schon bei Takt 0 — über die Lebenspunkte', () => {
    // Der Messpunkt gehört zum Wert: VESSEL hebt `playerHp`, und das steht in
    // der Prüfsumme. Wer erst nach dem Kampf misst, sieht denselben
    // Unterschied — aber nicht, dass er hier schon da ist.
    expect(pruefsummeNach([getragen()], 0)).not.toBe(pruefsummeNach([], 0));
  });

  it('und der Schaden zeigt ihn nach dem ersten Treffer', () => {
    expect(pruefsummeNach([getragen()], 5)).not.toBe(pruefsummeNach([], 5));
  });

  it('ZWEITE HÄLFTE: ohne Ausrüstung genau wie ohne die Änderung', () => {
    // Ein Kampf ohne Stücke muss dieselbe Prüfsumme haben wie einer, der gar
    // kein `worn` bekommt — dieselbe Zusicherung, an der die Referenz hängt.
    const ohneFeld = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    const mitLeer = kampf([]);
    expect(combatChecksum(mitLeer, 0)).toBe(combatChecksum(ohneFeld, 0));

    ohneFeld.cast(ohneFeld.indexOf('kindle'), 0);
    mitLeer.cast(mitLeer.indexOf('kindle'), 0);
    expect(combatChecksum(mitLeer, 50)).toBe(combatChecksum(ohneFeld, 50));
  });

  it('und ein Stück im Beutel verändert nichts', () => {
    // Die Gegenprobe zur Gegenprobe: Wäre `equippedSlot` unbeachtet, fiele
    // dieser Fall — und die Prüfung darüber wäre trotzdem grün.
    expect(pruefsummeNach([imBeutel()], 5)).toBe(pruefsummeNach([], 5));
  });
});

describe('Der Kampf trägt es wirklich', () => {
  it('mit Ausrüstung trifft es härter', () => {
    const ohne = kampf([]);
    const mit = kampf([getragen()]);
    ohne.cast(ohne.indexOf('kindle'), 0);
    mit.cast(mit.indexOf('kindle'), 0);
    expect(mit.damageDealt).toBeGreaterThan(ohne.damageDealt);
  });

  it('und die Lebenspunkte sind höher', () => {
    // VESSEL wirkt auf die Obergrenze, nicht auf den Schaden — ein Prüffall,
    // der nur den Schaden misst, sähe diesen Teil nie. Gemessen an `playerHp`,
    // weil der Kampf beim Aufbau voll startet.
    expect(kampf([getragen()]).playerHp).toBeGreaterThan(kampf([]).playerHp);
  });

  it('setWorn wirkt zwischen den Kämpfen', () => {
    const combat = kampf([]);
    const vorher = combat.playerHp;
    combat.setWorn(wornFrom([getragen()]));
    expect(combat.playerHp).toBeGreaterThan(vorher);
  });

  /**
   * **Richtung ist nicht Größe — und darin lag eine echte Lücke.**
   *
   * Der Fall darüber prüft `toBeGreaterThan`. Verdoppelt man den Wert, den
   * `setWorn` einsetzt, ist das Ergebnis immer noch „größer": Die Mutation
   *
   * ```ts
   * this.vesselMax = vesselWithGear(…, worn.vessel * 2);   // in setWorn
   * ```
   *
   * ließ **2047 von 2047** Fällen grün — gemessen am 2026-08-10. `vesselMax`
   * wird an zwei Stellen gerechnet, im Aufbau und in `setWorn`; die Parität
   * deckt nur den Aufbau, weil keine Aufzeichnung `setWorn` fährt.
   *
   * Dieselbe Klasse wie Falle 2 im TALENT-RASTER: dieselbe Größe zweimal
   * gelesen, nur eine Lesestelle bewacht.
   *
   * # Warum gegen den Aufbau und nicht gegen eine Zahl
   *
   * Die Aussage, die zählt, ist **„beide Wege führen zum selben Zustand"** —
   * wer die Ausrüstung zwischen zwei Kämpfen anlegt, muss dastehen wie
   * jemand, der sie von Anfang an trug. Eine feste Zahl hier hätte einen
   * zweiten Ort geschaffen, an dem die Formel steht, und wäre bei jeder
   * Balance-Änderung fällig geworden, ohne je etwas anderes zu sagen.
   *
   * Die Kette schließt sich so: Dieser Fall bindet `setWorn` an den Aufbau,
   * und die Paritäts-Aufzeichnungen binden den Aufbau an einen festen Stand.
   *
   * # Erst gekämpft, dann verglichen — und das war ein eigener Messfehler
   *
   * Meine erste Fassung verglich die Prüfsumme bei **Takt 0** und behauptete
   * im Kommentar, sie binde auch `essence`, `gearEffects` und den
   * Krit-Faktor, deren Wirkstellen ebenfalls in `setWorn` liegen. Gemessen:
   *
   * ```text
   *   gearEffectFactor(worn) * 2   in setWorn   ->   2048 von 2048 grün
   * ```
   *
   * Bei Takt 0 ist kein Treffer gefallen, `damage` steht auf 0, und ein
   * Schadensfaktor bewegt nichts, was die Prüfsumme sähe. VESSEL fiel nur
   * auf, weil es `playerHp` hebt — und das steht schon vor dem ersten Zauber
   * fest. **Genau davor warnt der Kopf dieser Datei** („bei VESSEL sieht man
   * den Unterschied schon bei Takt 0, bei CATALYSIS erst mit laufender
   * Abklingzeit"); ich habe die Warnung gelesen und den Fehler trotzdem
   * gemacht.
   *
   * Deshalb kämpfen jetzt beide Seiten ein Stück weit, bevor verglichen wird.
   * `setWorn` läuft davor, weil es mitten im Kampf ausdrücklich sperrt.
   */
  it('setWorn führt in DENSELBEN Zustand wie der Aufbau — nicht bloß in einen besseren', () => {
    const ueberAufbau = kampf([getragen()]);
    const nachtraeglich = kampf([]);
    nachtraeglich.setWorn(wornFrom([getragen()]));

    // Zusicherung, dass hier überhaupt etwas anliegt: Wäre `worn` leer,
    // vergliche die Zeile unten zwei nackte Spieler und wäre grün, ohne etwas
    // zu zeigen.
    const werte = wornFrom([getragen()]);
    expect(werte.vessel, 'der Prüf-Gegenstand trägt kein VESSEL').toBeGreaterThan(0);
    expect(werte.essence, 'der Prüf-Gegenstand trägt kein ESSENCE').toBeGreaterThan(0);

    for (const combat of [ueberAufbau, nachtraeglich]) {
      combat.beginFight(1);
      // **Ein Treffer muss fallen**, sonst bleiben die Schadensfaktoren
      // unsichtbar — siehe Kopf dieses Falls.
      expect(combat.cast(combat.indexOf('kindle'), 10).damage).toBeGreaterThan(0);
      combat.stepPassive(2000);
    }
    expect(
      ueberAufbau.damageDealt,
      'ohne gefallenen Schaden sagt der Vergleich nichts',
    ).toBeGreaterThan(0);
    expect(combatChecksum(nachtraeglich, 2000)).toBe(combatChecksum(ueberAufbau, 2000));
  });

  it('GEGENPROBE: mitten im Kampf nicht', () => {
    // Dieselbe Linie wie beim Umskillen (12.4): Wer die Werte mitten im Kampf
    // wechselte, rechnete den Schaden rückwirkend anders als die bereits
    // gefallenen Treffer.
    //
    // **`beginFight` gehört dazu, und das war ein eigener Messfehler**: Die
    // erste Fassung wirkte nur einen Zauber und liess die Uhr auf null. Die
    // Sperre prüft `fightStartedAtMs > 0` — sie griff korrekt nicht, weil
    // dieser Prüffall den Kampf nie begonnen hatte. Nicht die Sperre war
    // falsch, sondern die Messung.
    // **`beginFight(1)`, nicht `(0)`** — und das ist ein Befund, kein Kniff:
    // Die Sperre prüft `fightStartedAtMs > 0`. Ein Kampf, der zur Zeit 0
    // beginnt, gilt ihr als nicht begonnen. Im Spiel läuft der Countdown
    // vorher, also ist der Startzeitpunkt dort immer positiv; im Prüfstand ist
    // 0 der naheliegende Wert. Dasselbe Muster steht seit #289 in
    // `setPlayerLevel` und seit #417 in `setTalents` — gemeldet, hier nicht
    // geändert.
    const combat = kampf([]);
    combat.beginFight(1);
    combat.cast(combat.indexOf('kindle'), 10);
    combat.stepPassive(100);
    const vorher = combat.playerHp;
    combat.setWorn(wornFrom([getragen()]));
    expect(combat.playerHp).toBe(vorher);
  });
});

describe('AC3 — und der ANSCHLUSS selbst ist bewacht', () => {
  /**
   * Der Quelltext von `app.ts`, über Vite geladen.
   *
   * **Der Grund für diesen Prüffall ist eine Gegenprobe, die nichts fand.**
   * Nach dem Bau habe ich die Zeile `setWorn(wornFrom(…))` aus `app.ts`
   * entfernt — und alle 1494 Fälle blieben grün. `wornFrom` ist bewacht,
   * `setWorn` ist bewacht, aber die eine Zeile, die AC3 überhaupt erfüllt,
   * war es nicht: `app.ts` braucht einen DOM, und die Prüfungen laufen ohne.
   *
   * Genau diese Klasse hat #421 aufgedeckt — eine verdrahtete Kette, die bei
   * einem Wert beginnt, den niemand füllt. Ein PR, der sie schliesst und die
   * schliessende Zeile ungeschützt lässt, hätte den Fehler nur verschoben.
   */
  const QUELLE = import.meta.glob('/src/core/app.ts', {
    query: '?raw',
    import: 'default',
    eager: true,
  }) as Record<string, string>;

  function appQuelle(): string {
    const text = Object.values(QUELLE)[0];
    if (text === undefined || text.length === 0) {
      throw new Error('app.ts nicht geladen — die Pruefung wuerde nichts messen');
    }
    return text;
  }

  it('beim Aufbau: aus dem Spielstand in den Kampf', () => {
    // **Genau diese Stelle**, nicht irgendein `setWorn(wornFrom(…))`: Meine
    // erste Fassung prüfte das lose Muster und blieb grün, als ich die
    // Aufbau-Zeile entfernte — `setItems` enthält dasselbe Muster. Ein
    // Wächter, der zwei Stellen nicht unterscheidet, bewacht keine.
    expect(appQuelle()).toMatch(/setWorn\(wornFrom\(this\.persistence\.state\.items\)\)/);
  });

  it('und beim Anlegen: derselbe Weg in beide Richtungen', () => {
    // Wer nur den Stand schriebe, rechnete bis zum nächsten Neuladen mit den
    // alten Werten weiter.
    const quelle = appQuelle();
    expect(quelle).toMatch(/persistItems\(items\)/);
    expect(quelle).toMatch(/setWorn\(wornFrom\(items\)\)/);
  });

  it('AC5 — die Werte-Übersicht zeigt keine feste Vorgabe mehr', () => {
    // **Als Zählung, nicht als Muster**: Meine erste Fassung suchte
    // `statOverview(… NOTHING_WORN` mit einem Regex, der über Zeilenumbrüche
    // nicht greift — sie blieb grün, als ich die Anzeige zurückdrehte.
    //
    // `NOTHING_WORN` darf in `app.ts` überhaupt nicht mehr als Wert stehen.
    // Der einzige erlaubte Treffer ist die Erwähnung im Kommentar, die
    // erklärt, warum es hier nicht mehr steht.
    const zeilen = appQuelle()
      .split('\n')
      .filter((zeile) => zeile.includes('NOTHING_WORN'))
      .filter((zeile) => !zeile.trimStart().startsWith('//'));
    expect(zeilen).toEqual([]);
  });
});
