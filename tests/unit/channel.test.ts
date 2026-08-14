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
import {
  alsSpellIndex,
  CAST_OK,
  CAST_STARTED,
  Combat,
  isCastAccepted,
} from '../../src/core/combat.js';
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

describe('DECOCTION gehört zu COAGULATION — die Umwandlung läuft MIT dem Kanal', () => {
  /**
   * # Warum dieser Block über der Schul-Zuordnung steht
   *
   * `"school": "COAGULATION"` in einer Datendatei ist eine Behauptung. Was sie
   * trägt, ist GAME-DESIGN 176 — Weg 3 der Schule: *„Schaden-in-Schutz-
   * Umwandlung, die einen Teil des ausgeteilten Schadens LAUFEND in Schutz
   * verwandelt."*
   *
   * **„Laufend" ist genau die Kanalisierung.** Und die Stelle dafür steht schon
   * im Kanal-Schritt: `resolveChannelTick` ruft `convertDamageToShield` bei
   * jedem Teilschritt. Ohne einen Fall darüber wäre die Zuordnung eine Zeile in
   * einer Datei, die niemand prüft.
   *
   * Die Umwandlung schaltet TEMPER an (25 % über 10 s, `data/spells/
   * coagulation.json`) — sie ist keine Eigenschaft von DECOCTION, sondern eine
   * der Schule. Genau deshalb ist die Gegenprobe unten nötig: Ohne sie zeigte
   * der Fall nur, dass irgendwo Schutz entsteht.
   */
  function kanalMitSchutz(mitTemper: boolean): { schutz: number; schaden: number } {
    const combat = kampf();
    const dec = combat.indexOf('decoction');
    const temper = combat.indexOf('temper');
    let t = 0;
    if (mitTemper) {
      expect(combat.cast(temper, t).result, 'TEMPER wurde abgelehnt').toBe(CAST_OK);
      // Über die globale Wartezeit hinweg, sonst weist der nächste Druck ab.
      for (; t <= 1500; t += 10) combat.stepPassive(t);
    }
    const beginn = t;
    expect(combat.cast(dec, t).result, 'DECOCTION wurde abgelehnt').toBe(CAST_STARTED);
    const dauer = combat.spellAt(dec)?.channelMs ?? 0;
    expect(dauer, 'DECOCTION kanalisiert gar nicht').toBeGreaterThan(0);
    for (; t <= beginn + dauer; t += 10) combat.stepPassive(t);
    return { schutz: combat.converted, schaden: combat.damageDealt };
  }

  it('der Schutz WÄCHST während der Kanalisierung — Schritt für Schritt', () => {
    const combat = kampf();
    const dec = combat.indexOf('decoction');
    const temper = combat.indexOf('temper');
    combat.cast(temper, 0);
    let t = 0;
    for (; t <= 1500; t += 10) combat.stepPassive(t);
    combat.cast(dec, t);
    const raster = combat.spellAt(dec)?.channelTickMs ?? 0;
    const dauer = combat.spellAt(dec)?.channelMs ?? 0;
    const beginn = t;

    // **Der Verlauf, nicht nur der Endstand.** Ein Endstand allein liesse
    // offen, ob der Schutz laufend anfällt oder in einem Sprung am Ende — und
    // „laufend" ist das ganze Argument für die Schule.
    const verlauf: number[] = [];
    for (; t <= beginn + dauer; t += 10) {
      combat.stepPassive(t);
      if ((t - beginn) % raster === 0 && t > beginn) verlauf.push(combat.converted);
    }

    expect(verlauf.length, 'kein einziger Raster-Schritt gemessen').toBeGreaterThan(1);
    expect(verlauf[0], 'nach dem ersten Schritt stand kein Schutz').toBeGreaterThan(0);
    for (let i = 1; i < verlauf.length; i++) {
      expect(
        verlauf[i],
        `Schritt ${i} hat nichts hinzugefügt: ${verlauf.join(' → ')}`,
      ).toBeGreaterThan(verlauf[i - 1] ?? 0);
    }
  });

  it('GEGENPROBE: ohne TEMPER entsteht KEIN Schutz — bei gleichem Schaden', () => {
    const mit = kanalMitSchutz(true);
    const ohne = kanalMitSchutz(false);
    expect(ohne.schutz, 'ohne die Umwandlung entstand trotzdem Schutz').toBe(0);
    expect(mit.schutz, 'mit der Umwandlung entstand kein Schutz').toBeGreaterThan(0);
    // **Und der Schaden des Kanals ist in beiden Läufen derselbe.** Ohne diese
    // Zeile könnte der Unterschied auch daher kommen, dass der eine Lauf mehr
    // ausgeteilt hat — dann wäre nicht die Umwandlung gemessen, sondern die
    // Rotation.
    expect(mit.schaden, 'die beiden Läufe teilen unterschiedlich viel aus').toBe(ohne.schaden);
  });
});

describe('Eine Kanalisierung zählt als Wurf', () => {
  /**
   * # Der Befund (2026-08-13)
   *
   * `castCounts` wird an genau einer Stelle erhöht — in `resolvePlayerCast` —,
   * und die wird für Kanalisierungen übersprungen. Eine vollständig
   * durchgelaufene Kanalisierung teilte ihre ganze Wirkung aus und stand mit
   * `castCount` **0** da.
   *
   * Dass es kein Entwurf war, zeigt der Zauber mit WIRKZEIT: dieselbe Bauart
   * — beginnt, läuft, löst auf —, und er zählt ganz normal. Die Kanalisierung
   * war der einzige Typ, der durchfiel.
   */
  it('nach vollem Durchlauf steht der Zähler auf eins', () => {
    const combat = kampf();
    const d = kanalDaten(combat);
    expect(combat.castCount(d.index), 'vor dem Wurf schon gezählt').toBe(0);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    for (let t = 0; t <= d.dauer; t += 10) combat.stepPassive(t);

    // Die Zusicherung daneben: Ohne sie könnte der Zähler auch dann auf 1
    // stehen, wenn die Kanalisierung gar nichts ausgeteilt hat.
    expect(combat.damageDealt, 'die Kanalisierung hat nichts gewirkt').toBe(
      d.schritte * d.jeSchritt,
    );
    expect(combat.castCount(d.index)).toBe(1);
  });

  it('auch ein ABGEBROCHENER Kanal zählt — er hat gewirkt', () => {
    const combat = kampf();
    const d = kanalDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    const mitten = 2 * d.raster;
    for (let t = 0; t <= mitten; t += 10) combat.stepPassive(t);
    combat.cast(combat.indexOf('kindle'), mitten);
    expect(combat.playerCasting, 'der Abbruch hat nicht gegriffen').toBe(-1);
    expect(combat.castCount(d.index)).toBe(1);
  });

  it('GEGENPROBE: der Zauber mit WIRKZEIT zählt genauso — er fiel nie durch', () => {
    // Der Vergleich, an dem sich zeigt, dass die Lücke kein Entwurf war: Beide
    // Typen beginnen, laufen und lösen auf. Nur einer wurde gezählt.
    const combat = kampf();
    const cast = combat.indexOf('test-cast');
    const wirkzeit = combat.spellAt(cast)?.castTimeMs ?? 0;
    expect(wirkzeit, 'test-cast hat keine Wirkzeit').toBeGreaterThan(0);
    expect(combat.cast(cast, 0).result).toBe(CAST_STARTED);
    for (let t = 0; t <= wirkzeit; t += 10) combat.stepPassive(t);
    expect(combat.castCount(cast)).toBe(1);
  });
});

describe('Die Abklingzeit läuft an — beim BEGINN, nicht am Ende', () => {
  /**
   * # Der Befund, aus dem diese Fälle entstanden sind (2026-08-13)
   *
   * Die Abklingzeit stand in KEINEM Pfad einer Kanalisierung. Sie wird an genau
   * einer Stelle gesetzt — in `resolvePlayerCast` —, und die wird für
   * Kanalisierungen ausdrücklich übersprungen, weil ihre Wirkung schon über die
   * Schritte gefallen ist. Was dabei mit übersprungen wurde, war die
   * Abklingzeit.
   *
   * Gemessen an DECOCTION (`cooldownMs: 12000`, `channelMs: 4000`): 8000 ms nach
   * dem Beginn stand `isReady` auf `true`. Der Zauber war nach vollem Durchlauf
   * sofort wieder werfbar.
   *
   * # Warum beim BEGINN und nicht am Ende
   *
   * GAME-DESIGN 7.1 nimmt den Cast aus: *„Bei Abbruch geht der Zauber verloren,
   * die Abklingzeit läuft aber nicht an."* Diese Ausnahme steht unter einer
   * Bedingung — der Zauber ist verloren —, und bei einer Kanalisierung ist er
   * das nicht: Der erreichte Anteil bleibt (7.2). Sie hat gewirkt, also zahlt
   * sie. Am Beginn zu setzen deckt beide Ausgänge mit einer Stelle.
   *
   * **Das ist eine Design-Entscheidung, die das Dokument nicht ausdrücklich
   * trifft.** Sie steht hier, damit ein Widerspruch dagegen eine Stelle hat.
   */
  it('nach vollem Durchlauf ist der Zauber NICHT sofort wieder bereit', () => {
    const combat = kampf();
    const d = kanalDaten(combat);
    const abkling = combat.spellAt(d.index)?.cooldownMs ?? 0;
    // Die Zusicherung vor der Messung: Ein Prüfzauber ohne Abklingzeit machte
    // jeden Fall hier grün, ohne etwas zu prüfen.
    expect(abkling, 'der Prüfzauber hat gar keine Abklingzeit').toBeGreaterThan(d.dauer);

    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    for (let t = 0; t <= d.dauer; t += 10) combat.stepPassive(t);
    expect(combat.playerCasting, 'die Kanalisierung lief noch').toBe(-1);

    // **`isOffCooldown` und NICHT `isReady`** — der Unterschied ist hier die
    // ganze Aussage. `isReady` schließt die globale Wartezeit ein; gemessen am
    // 2026-08-13 blieb der Abbruch-Fall unten damit grün, obwohl die
    // Abklingzeit gar nicht lief: Die Wartezeit des abbrechenden Zaubers
    // sperrte ihn, und das sah genauso aus.
    expect(combat.isOffCooldown(d.index, d.dauer), 'sofort nach dem Ende wieder frei').toBe(false);
    // Und die Grenze selbst — einen Takt davor gesperrt, auf der Marke frei.
    expect(combat.isOffCooldown(d.index, abkling - 10)).toBe(false);
    expect(combat.isOffCooldown(d.index, abkling)).toBe(true);
  });

  it('auch ein ABBRUCH lässt sie anlaufen — der Anteil ist gefallen, also zahlt er', () => {
    const combat = kampf();
    const d = kanalDaten(combat);
    const abkling = combat.spellAt(d.index)?.cooldownMs ?? 0;
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    const mitten = 2 * d.raster;
    for (let t = 0; t <= mitten; t += 10) combat.stepPassive(t);
    combat.cast(combat.indexOf('kindle'), mitten);
    expect(combat.playerCasting, 'der Abbruch hat nicht gegriffen').toBe(-1);

    // Wieder `isOffCooldown`: Bei `mitten` läuft die Wartezeit des Zaubers, der
    // eben abgebrochen hat — `isReady` wäre hier auch ohne Abklingzeit `false`
    // und bewiese nichts.
    expect(combat.isOffCooldown(d.index, mitten), 'nach dem Abbruch sofort wieder frei').toBe(
      false,
    );
    expect(combat.isOffCooldown(d.index, abkling)).toBe(true);
  });

  it('OFFEN AM DESIGN: auch ein BOSS-Abbruch lässt sie zahlen', () => {
    // **Dieser Fall hält eine Entscheidung fest, die GAME-DESIGN nicht trifft
    // — damit sie nicht still bleibt.**
    //
    // Das Dokument kennt zwei Abbrucharten, und sie stehen in verschiedenen
    // Absätzen:
    //
    // > 635  „Jede Wirkung des SPIELERS bricht einen laufenden Cast oder eine
    // >       Kanalisierung ab. Jede, ohne Ausnahme."
    // > 661  „Abbruch eines Zauberbalkens: nur durch bestimmte, vorher
    // >       erkennbare BOSS-Angriffe. Normale Treffer stören nicht. Bei
    // >       Abbruch geht der Zauber verloren, die Abklingzeit läuft aber
    // >       nicht an."
    //
    // Die Ausnahme steht im Boss-Absatz, und ihre Begründung darunter ist eine
    // über Boss-Angriffe: „Wer den Boss kennt, fängt den Zauber gar nicht erst
    // an." Für den SELBST-Abbruch sagt das Design nichts — dort greift sie
    // zweifach nicht, weil es kein Boss-Angriff ist und weil der Zauber nicht
    // verloren geht.
    //
    // **Beim Boss-Abbruch einer KANALISIERUNG stimmt nur noch die Hälfte
    // davon.** Es IST ein Boss-Angriff — aber der Zauber geht nicht verloren,
    // der erreichte Anteil bleibt (7.2). Die Abklingzeit läuft hier trotzdem,
    // weil sie beim Beginn gesetzt wird und diese eine Stelle beide Ausgänge
    // deckt.
    //
    // Der Fall tritt wirklich ein und ist nicht theoretisch: `test-caster`
    // unterbricht bei 9000 und 18000 ms, und die Kanal-Aufzeichnung fährt
    // gegen genau diesen Boss — sie zählt zwei unterbrochene Zauber.
    const caster = bossById('test-caster') as BossDef;
    const combat = new Combat({ school: CALCINATION.value, boss: caster });
    combat.beginFight(0);
    const d = kanalDaten(combat);
    const abkling = combat.spellAt(d.index)?.cooldownMs ?? 0;

    // Bei 7000 beginnen, damit der Schlag bei 9000 mitten hineinfällt.
    let t = 0;
    for (; t < 7000; t += 10) combat.stepPassive(t);
    expect(combat.cast(d.index, t).result).toBe(CAST_STARTED);
    const beginn = t;
    for (; t <= 9000; t += 10) combat.stepPassive(t);

    // Die Zusicherung: Ohne einen echten Boss-Abbruch prüfte der Fall nichts.
    expect(combat.castsInterrupted, 'der Boss hat gar nicht unterbrochen').toBeGreaterThan(0);
    expect(combat.playerCasting, 'die Kanalisierung lief weiter').toBe(-1);
    // Der erreichte Anteil ist geblieben — der Zauber ist nicht verloren.
    expect(combat.damageDealt, 'nichts angekommen').toBeGreaterThan(0);
    // Und sie zahlt trotzdem.
    expect(combat.isOffCooldown(d.index, 9000)).toBe(false);
    expect(combat.isOffCooldown(d.index, beginn + abkling)).toBe(true);
  });

  it('MESSUNG: Abbruch-Spam trägt nicht mehr ein als der Durchlauf', () => {
    // **Der eigentliche Schaden des Fehlers, und der Grund für die
    // Entscheidung oben.** Ohne Abklingzeit war die beste Rotation, den Kanal
    // nach dem ersten Schritt abzubrechen und sofort neu zu beginnen: Der
    // Spieler nahm jeden Schritt mit, zahlte aber nie die Sperre.
    //
    // Gemessen vor der Behebung, über 24 s mit DECOCTION: 1440 gegen 720 — der
    // Abbruch trug DOPPELT. Die Zahlen unten sind gerechnet, nicht
    // abgeschrieben; nur ihr VERHÄLTNIS ist die Aussage.
    function ertrag(neustartAlleMs: number, dauerMs: number): number {
      const combat = kampf();
      const { index } = kanalDaten(combat);
      let letzterStart = -1e9;
      for (let t = 0; t <= dauerMs; t += 10) {
        combat.stepPassive(t);
        if (t - letzterStart >= neustartAlleMs && combat.isReady(index, t)) {
          const r = combat.cast(index, t).result;
          if (r === CAST_OK || r === CAST_STARTED) letzterStart = t;
        }
      }
      return combat.damageDealt;
    }

    const combat = kampf();
    const d = kanalDaten(combat);
    const abkling = combat.spellAt(d.index)?.cooldownMs ?? 0;
    const dauer = 3 * abkling;
    // **Der Spam-Lauf startet neu, sobald EIN Schritt gefallen ist** — das ist
    // der optimale Missbrauch: den Anteil mitnehmen, sofort wieder anfangen.
    // Die globale Wartezeit bremst zusätzlich, und zwar von selbst: Ein
    // abgewiesener Druck setzt `letzterStart` nicht.
    const spam = ertrag(d.raster, dauer);
    const durchlauf = ertrag(abkling, dauer);

    expect(durchlauf, 'der Kontroll-Lauf hat gar nichts getroffen').toBeGreaterThan(0);
    expect(
      spam,
      `Abbruch-Spam ${spam} trägt mehr ein als Durchlauf ${durchlauf}`,
    ).toBeLessThanOrEqual(durchlauf);
  });
});

describe('Die HEILUNGS-Kanalisierung — das wörtliche Beispiel des Auftraggebers', () => {
  /**
   * # Der Fall, den #323 bis zuletzt nicht konnte
   *
   * GAME-DESIGN 653 nennt ihn wörtlich: *„Eine Kanalisierung über 5 s für 500
   * HEILUNG, abgebrochen nach 3 s, hat 300 Heilung gegeben. Dasselbe gilt für
   * Schadens- und Ressourcen-Kanalisierungen."*
   *
   * Der Hauptfall des Absatzes ist also die HEILUNG; DECOCTION (#832) ist die
   * Ableitung „dasselbe gilt für Schadens-Kanalisierungen". Und ausgerechnet
   * der Hauptfall tat nichts: `resolveChannelTick` stieg bei `baseDamage <= 0`
   * aus, bevor irgendetwas ausgezahlt wurde.
   *
   * **Die Gegenprobe, vor dem Bau gemessen und hier festgehalten:** Derselbe
   * Zauber heilte 0, während dem Spieler 1600 fehlten. Ohne diese Zahl wäre
   * nicht belegt, dass hier eine Lücke geschlossen und nicht bloß ein
   * Datensatz angelegt wurde.
   */
  const HEIL_BOSS = bossById('test-onslaught') as BossDef;

  /** Die Kennzahlen des Heil-Prüfzaubers — abgelesen, nicht angenommen. */
  function heilDaten(combat: Combat) {
    const index = combat.indexOf('test-channel-heal');
    const spell = combat.spellAt(index);
    const dauer = spell?.channelMs ?? 0;
    const raster = spell?.channelTickMs ?? 0;
    const gesamt = spell?.heal?.amount ?? 0;
    const schritte = raster > 0 ? Math.floor(dauer / raster) : 0;
    return { index, dauer, raster, gesamt, schritte, jeSchritt: Math.floor(gesamt / schritte) };
  }

  /**
   * Ein Kampf, in dem Gesundheit FEHLT — sonst heilt auch eine funktionierende
   * Heilung nichts, und der Fall wäre grün, ohne etwas zu prüfen.
   *
   * # Gemessen wird `healed`, NICHT die Gesundheit
   *
   * Der erste Entwurf verglich `playerHp` vorher und nachher. Der Abbruch-Fall
   * meldete daraufhin `expected -1300 to be 300`: Der Boss schlägt während der
   * Messung weiter zu, und die Gesundheit ist die Summe aus allem, was ihr
   * zustösst. `healed` führt nur, was die Heilung beigetragen hat.
   *
   * Der Durchlauf-Fall war dabei **grün** — sein Fenster lag zufällig zwischen
   * zwei Schlägen (`test-onslaught` trifft bei 5000, 12_000 und 19_000). Eine
   * falsche Messgrösse, die in einem von zwei Fällen das richtige Ergebnis
   * liefert, ist die unangenehmere Sorte.
   */
  function verwundet(): { combat: Combat; abMs: number } {
    const combat = new Combat({ school: CALCINATION.value, boss: HEIL_BOSS });
    combat.beginFight(0);
    const voll = combat.playerHp;
    let t = 0;
    for (; t <= 6000; t += 10) combat.stepPassive(t);
    expect(combat.playerHp, 'dem Spieler fehlt nichts — dann prüft der Fall nichts').toBeLessThan(
      voll,
    );
    return { combat, abMs: t };
  }

  it('ZUSICHERUNG: der Prüfzauber IST der aus dem Design-Beispiel', () => {
    // 5 s, 500 Heilung, fünf Schritte à 100. Nur dann ist „300 nach 3 s" das
    // Beispiel und nicht eine beliebige Zahl.
    const d = heilDaten(new Combat({ school: CALCINATION.value, boss: HEIL_BOSS }));
    expect(d.dauer).toBe(5000);
    expect(d.gesamt).toBe(500);
    expect(d.schritte).toBe(5);
    expect(d.jeSchritt).toBe(100);
  });

  it('läuft sie durch, sind 500 geheilt', () => {
    const { combat, abMs } = verwundet();
    const d = heilDaten(combat);
    const vorher = combat.healed;
    expect(combat.cast(d.index, abMs).result).toBe(CAST_STARTED);
    let t = abMs;
    for (; t <= abMs + d.dauer; t += 10) combat.stepPassive(t);
    // Aus der Dauer gerechnet, nicht abgeschrieben.
    expect(combat.healed - vorher).toBe(d.schritte * d.jeSchritt);
  });

  it('nach 3 von 5 Schritten abgebrochen bleiben 300 — die Zahl des Auftraggebers', () => {
    const { combat, abMs } = verwundet();
    const d = heilDaten(combat);
    const vorher = combat.healed;
    expect(combat.cast(d.index, abMs).result).toBe(CAST_STARTED);

    const abbruchAt = abMs + 3 * d.raster;
    let t = abMs;
    for (; t <= abbruchAt; t += 10) combat.stepPassive(t);
    const beiDrei = combat.healed - vorher;

    // Eine eigene Wirkung bricht ab (GAME-DESIGN 635).
    combat.cast(combat.indexOf('kindle'), abbruchAt);
    expect(combat.playerCasting, 'der Abbruch hat nicht gegriffen').toBe(-1);
    for (; t <= abMs + d.dauer + 2000; t += 10) combat.stepPassive(t);

    expect(beiDrei).toBe(3 * d.jeSchritt);
    // Und danach kommt nichts mehr nach — der Rest ist verloren, nicht
    // aufgeschoben.
    expect(combat.healed - vorher).toBe(beiDrei);
  });

  it('die Zahlen aus dem Design: 300 von 500', () => {
    // Dieselbe Aussage in den Zahlen des Auftraggebers. Sie steht hier UND
    // oben gerechnet: Stimmt eine der beiden nicht mehr, ist entweder die
    // Rechnung falsch oder der Prüfzauber ein anderer.
    const d = heilDaten(new Combat({ school: CALCINATION.value, boss: HEIL_BOSS }));
    expect(3 * d.jeSchritt).toBe(300);
    expect(d.gesamt).toBe(500);
  });

  it('TRÄGER: der Heil-Kanal heilt ÜBERHAUPT — wird bei 0 == 0 laut', () => {
    // **Der Wächter, der anschlägt, wenn der Heil-Kanal wieder durchfällt.**
    // Ohne ihn verglichen die Fälle oben zwei Nullen, sobald die Auszahlung
    // ausfällt: „nach 3 Schritten 300" wäre dann `0 === 3 * 0`, und das ist
    // wahr. Genau so war die Lage vor dem Bau — der Zauber heilte 0, und
    // niemand hätte es gemerkt.
    const { combat, abMs } = verwundet();
    const d = heilDaten(combat);
    expect(d.jeSchritt, 'der Prüfzauber heilt je Schritt 0 — dann prüft nichts').toBeGreaterThan(0);
    const vorher = combat.healed;
    expect(combat.cast(d.index, abMs).result).toBe(CAST_STARTED);
    let t = abMs;
    for (; t <= abMs + d.raster; t += 10) combat.stepPassive(t);
    expect(
      combat.healed - vorher,
      'nach dem ersten Raster-Schritt kam KEINE Heilung an',
    ).toBeGreaterThan(0);
  });

  it('und vor dem ersten Raster-Schritt ist es NICHTS', () => {
    // Der Gegenpol: Ein Heil-Kanal, der sofort alles gäbe, wäre ein
    // Sofort-Zauber mit Nachlauf.
    const { combat, abMs } = verwundet();
    const d = heilDaten(combat);
    const vorher = combat.healed;
    expect(combat.cast(d.index, abMs).result).toBe(CAST_STARTED);
    let t = abMs;
    for (; t <= abMs + d.raster - 10; t += 10) combat.stepPassive(t);
    expect(combat.healed).toBe(vorher);
  });
});

describe('Die RESSOURCEN-Kanalisierung — „dasselbe gilt für Ressourcen"', () => {
  /**
   * # Der dritte Satz des Design-Absatzes
   *
   * GAME-DESIGN 653 endet mit: *„Dasselbe gilt für Schadens- und
   * Ressourcen-Kanalisierungen."* Der Schaden lief seit #832, die Heilung seit
   * diesem Zweig — der Vorrat gar nicht.
   *
   * **Gemessen, bevor gebaut wurde:** Ein Kanalzauber mit
   * `resourceGain: {heat: 5}` stand nach einem vollen Kanal bei **0**, während
   * KINDLE 3 brachte. Der Grund war kein Rechenfehler, sondern ein fehlender
   * Weg: `resourceGain` hängt an `RULE_SPELL_CAST`, und das Ereignis sendet nur
   * `resolvePlayerCast` — die der Kanal überspringt. Der Kanal-Schritt sendete
   * `RULE_DOT_TICK`, und daran hängt `gainOfTick` mit `dot.resourceGainPerTick`:
   * ein anderes Feld, das ein Kanalzauber nicht hat.
   *
   * Deshalb gibt es `RULE_CHANNEL_TICK` — angehängt und nicht eingeschoben,
   * damit keine bestehende Kennnummer wandert.
   */
  const RES_BOSS = bossById('test-onslaught') as BossDef;

  function kanalMitVorrat() {
    const combat = new Combat({ school: CALCINATION.value, boss: RES_BOSS });
    combat.beginFight(0);
    const index = combat.indexOf('test-channel-heal');
    const spell = combat.spellAt(index);
    const schritte = Math.floor((spell?.channelMs ?? 0) / (spell?.channelTickMs ?? 1));
    const roh = spell?.resourceGain?.[combat.resourceId] ?? 0;
    return { combat, index, schritte, roh };
  }

  it('TRÄGER: der Prüfzauber trägt überhaupt einen Vorrats-Gewinn', () => {
    // **Ohne diese Zusicherung prüfte der Fall unten 0 gegen 0.** Genau so war
    // die Lage vor dem Bau, und sie sah aus wie ein bestandener Fall.
    const { roh, schritte } = kanalMitVorrat();
    expect(roh, 'der Prüfzauber trägt keinen resourceGain').toBeGreaterThan(0);
    expect(
      Math.floor(roh / schritte),
      `${roh} über ${schritte} Schritte ergibt je Schritt 0 — dann prüft der Fall nichts`,
    ).toBeGreaterThan(0);
  });

  it('der Vorrat wächst SCHRITT FÜR SCHRITT, nicht am Ende', () => {
    const { combat, index, schritte, roh } = kanalMitVorrat();
    const jeSchritt = Math.floor(roh / schritte);
    const raster = combat.spellAt(index)?.channelTickMs ?? 0;
    expect(combat.resource).toBe(0);
    expect(combat.cast(index, 0).result).toBe(CAST_STARTED);

    const verlauf: number[] = [];
    for (let t = 0; t <= schritte * raster; t += 10) {
      combat.stepPassive(t);
      if (t > 0 && t % raster === 0) verlauf.push(combat.resource);
    }

    // **Der VERFALL gehört in die Erwartung, nicht in eine Ausrede.** Der erste
    // Entwurf verglich gegen `(i + 1) * jeSchritt` und meldete
    // `1 → 1 → 2 → 2 → 3: expected 1 to be 2`. Die Umsetzung war richtig: Hitze
    // verfällt um `decayAmount` alle `decayPerStepMs` (hier 1 je 2000 ms), und
    // der Vorrat wächst und schwindet zugleich.
    //
    // Gerechnet aus den Schul-Daten, nicht abgeschrieben — sonst stünde die
    // Verfallsrate hier ein zweites Mal und liefe bei der ersten Änderung von
    // den Daten weg.
    const vorrat = CALCINATION.value.resource;
    expect(verlauf.length).toBe(schritte);
    for (let i = 0; i < verlauf.length; i++) {
      const t = (i + 1) * raster;
      const zerfallen = vorrat.decayAmount * Math.floor(t / vorrat.decayPerStepMs);
      expect(verlauf[i], `nach Schritt ${i + 1}: ${verlauf.join(' → ')}`).toBe(
        (i + 1) * jeSchritt - zerfallen,
      );
    }
    // Und die Aussage, die der Verfall nicht verwischt: Am Ende steht MEHR als
    // am Anfang. Ohne sie könnte ein Kanal, der gar nichts beisteuert, hier
    // durchlaufen, solange der Verfall die Zahlen zufällig trifft.
    expect(
      verlauf[verlauf.length - 1],
      'der Kanal hat unterm Strich nichts gebracht',
    ).toBeGreaterThan(0);
  });

  it('bei Abbruch bleibt der erreichte Anteil — auch beim Vorrat', () => {
    const { combat, index, schritte, roh } = kanalMitVorrat();
    const jeSchritt = Math.floor(roh / schritte);
    const raster = combat.spellAt(index)?.channelTickMs ?? 0;
    expect(combat.cast(index, 0).result).toBe(CAST_STARTED);

    const abbruchAt = 3 * raster;
    let t = 0;
    for (; t <= abbruchAt; t += 10) combat.stepPassive(t);
    const beiDrei = combat.resource;
    // KINDLE bricht ab — und bringt seinen EIGENEN Gewinn mit, deshalb wird
    // ab hier die Differenz gemessen und nicht der Endstand.
    combat.cast(combat.indexOf('kindle'), abbruchAt);
    expect(combat.playerCasting, 'der Abbruch hat nicht gegriffen').toBe(-1);
    const nachAbbruch = combat.resource;
    for (; t <= schritte * raster + 2000; t += 10) combat.stepPassive(t);

    // Wieder mit dem Verfall gerechnet (siehe oben): Bei 3 Schritten à 1000 ms
    // ist einmal `decayPerStepMs` vergangen.
    const vorrat = CALCINATION.value.resource;
    const zerfallen = vorrat.decayAmount * Math.floor(abbruchAt / vorrat.decayPerStepMs);
    expect(beiDrei).toBe(3 * jeSchritt - zerfallen);
    // **Der eigentliche Punkt:** Nach dem Abbruch kommt vom KANAL nichts mehr
    // nach. Gemessen wird gegen den Stand direkt nach dem Abbruch, weil KINDLE
    // seinen eigenen Gewinn mitbringt — und weil der Verfall weiterläuft, darf
    // die Zahl nur SINKEN, nicht steigen.
    expect(combat.resource, 'nach dem Abbruch kam vom Kanal noch etwas nach').toBeLessThanOrEqual(
      nachAbbruch,
    );
  });
});

describe('Die SCHUTZ-Kanalisierung — EINE Schicht, die über die Dauer wächst', () => {
  /**
   * # Die Entscheidung, und warum sie eine war
   *
   * GAME-DESIGN 20.5, Punkt 3 (entschieden am 2026-08-14):
   *
   * > Ein Schutz-Kanal legt **eine** Schicht, deren Betrag mit jedem Schritt
   * > steigt; bricht er ab, bleibt sie auf dem erreichten Wert stehen.
   *
   * Die Gegenvariante — je Schritt eine eigene Schicht — ist ausdrücklich
   * verworfen: Sie wäre bei `plate` (maxStacks 1) nach dem ERSTEN Schritt am
   * Anschlag, und vier von fünf Schritten verpufften.
   *
   * **Der Prüfzauber trägt deshalb selbst `maxStacks: 1`.** Mit 3 wäre der
   * Unterschied erst beim vierten Schritt sichtbar, und ein Fall mit drei
   * Schritten bliebe grün, obwohl die falsche Variante gebaut wäre.
   *
   * **Gemessen vor dem Bau:** Derselbe Zauber lief fünf Schritte durch und
   * legte 0 Schutz.
   */
  const SCHUTZ_BOSS = bossById('first-crucible') as BossDef;

  function schutzDaten(combat: Combat) {
    const index = combat.indexOf('test-channel-shield');
    const spell = combat.spellAt(index);
    const dauer = spell?.channelMs ?? 0;
    const raster = spell?.channelTickMs ?? 0;
    const gesamt = spell?.shield?.amount ?? 0;
    const grenze = spell?.shield?.maxStacks ?? 0;
    const schritte = raster > 0 ? Math.floor(dauer / raster) : 0;
    return {
      index,
      dauer,
      raster,
      gesamt,
      grenze,
      schritte,
      jeSchritt: schritte > 0 ? Math.floor(gesamt / schritte) : 0,
    };
  }

  function schutzKampf(): Combat {
    // **Ein Boss ohne Schläge.** Sonst verbraucht der erste Treffer die
    // Schicht, und ein Fall über ihr Wachsen misst den Boss statt den Kanal.
    const combat = new Combat({ school: CALCINATION.value, boss: SCHUTZ_BOSS });
    combat.beginFight(0);
    return combat;
  }

  it('ZUSICHERUNG: der Prüfzauber trägt die schärfste Obergrenze', () => {
    // Ohne diese Zeile prüfte alles Folgende den bequemen Fall. `maxStacks: 1`
    // ist der, an dem die verworfene Variante scheitert.
    const d = schutzDaten(schutzKampf());
    expect(d.grenze, 'der Prüfzauber hat nicht die Obergrenze 1').toBe(1);
    expect(d.schritte, 'er kanalisiert nicht über mehrere Schritte').toBeGreaterThan(1);
    expect(d.jeSchritt, 'je Schritt käme 0 an — dann prüft nichts').toBeGreaterThan(0);
  });

  it('der Betrag WÄCHST Schritt für Schritt, und es bleibt EINE Schicht', () => {
    const combat = schutzKampf();
    const d = schutzDaten(combat);
    expect(combat.shield).toBe(0);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);

    const verlauf: number[] = [];
    const stapel: number[] = [];
    for (let t = 0; t <= d.dauer; t += 10) {
      combat.stepPassive(t);
      if (t > 0 && t % d.raster === 0) {
        verlauf.push(combat.shield);
        stapel.push(combat.shieldStacks);
      }
    }

    // Aus der Dauer gerechnet, nicht abgeschrieben.
    expect(verlauf.length).toBe(d.schritte);
    for (let i = 0; i < verlauf.length; i++) {
      expect(verlauf[i], `nach Schritt ${i + 1}: ${verlauf.join(' → ')}`).toBe(
        (i + 1) * d.jeSchritt,
      );
    }

    // **Der Punkt, an dem sich die Varianten unterscheiden.** Je Schritt eine
    // eigene Schicht hiesse hier: nach dem ersten Schritt Stapel 1 und dann
    // nichts mehr — die Obergrenze ist erreicht, der Betrag bliebe stehen.
    expect(stapel, `die Stapelzahl bewegt sich: ${stapel.join(' → ')}`).toEqual(
      new Array(d.schritte).fill(1),
    );
  });

  it('ein ABBRUCH friert sie auf dem erreichten Wert ein', () => {
    const combat = schutzKampf();
    const d = schutzDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);

    const abbruchAt = 3 * d.raster;
    let t = 0;
    for (; t <= abbruchAt; t += 10) combat.stepPassive(t);
    const beiDrei = combat.shield;

    combat.cast(combat.indexOf('kindle'), abbruchAt);
    expect(combat.playerCasting, 'der Abbruch hat nicht gegriffen').toBe(-1);
    for (; t <= d.dauer + 2000; t += 10) combat.stepPassive(t);

    expect(beiDrei).toBe(3 * d.jeSchritt);
    // Eingefroren, nicht weitergewachsen und nicht verfallen.
    expect(combat.shield, 'nach dem Abbruch hat sich die Schicht bewegt').toBe(beiDrei);
    expect(combat.shieldStacks).toBe(1);
  });

  it('TRÄGER: der Schutz-Kanal legt ÜBERHAUPT etwas — wird bei 0 == 0 laut', () => {
    // **Ohne ihn prüften die Fälle oben zwei Nullen.** Genau so war die Lage
    // vor dem Bau: fünf Schritte, 0 Schutz, und „nach drei Schritten 3/5"
    // wäre als `0 === 3 * 0` wahr gewesen.
    const combat = schutzKampf();
    const d = schutzDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    for (let t = 0; t <= d.raster; t += 10) combat.stepPassive(t);
    expect(combat.shield, 'nach dem ersten Raster-Schritt kam KEIN Schutz an').toBeGreaterThan(0);
  });

  it('und vor dem ersten Raster-Schritt ist es NICHTS', () => {
    const combat = schutzKampf();
    const d = schutzDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
    for (let t = 0; t <= d.raster - 10; t += 10) combat.stepPassive(t);
    expect(combat.shield).toBe(0);
  });

  it('GEGENPROBE zur Variante: ein gewirkter Schutzzauber STAPELT, der Kanal nicht', () => {
    // Der Unterschied in einem Fall. `scab` (maxStacks 3) legt je Wurf eine
    // neue Schicht; der Kanal lässt EINE wachsen. Ohne diese Gegenprobe
    // bewiese „Stapel bleibt 1" nur, dass der Zauber einmal gewirkt hat.
    const combat = schutzKampf();
    const scab = combat.indexOf('scab');
    expect(combat.spellAt(scab)?.shield?.maxStacks ?? 0, 'scab stapelt nicht').toBeGreaterThan(1);
    let t = 0;
    for (let n = 0; n < 2; n++) {
      combat.cast(scab, t);
      for (const bis = t + 1600; t <= bis; t += 10) combat.stepPassive(t);
    }
    expect(combat.shieldStacks, 'zwei Würfe SCAB haben nicht gestapelt').toBeGreaterThan(1);
  });

  /**
   * # Die Ablaufzeit — Annahme 1, und warum sie eine eigene Messung braucht
   *
   * Die Uhr startet mit **jedem** Schritt neu. Das ist eine der drei Annahmen,
   * die GAME-DESIGN 20.5 offenlässt; sie ist am 2026-08-14 freigegeben worden.
   *
   * **Ein Fall der Form „die Schicht läuft während des Kanals nicht ab" würde
   * das NICHT belegen.** Der Prüfzauber hält 20000 ms und kanalisiert 5000 —
   * sie läuft dort auch dann nicht ab, wenn die Uhr nur einmal beim ersten
   * Schritt gesetzt würde. Der Fall wäre grün und sagte nichts.
   *
   * Was die Annahme wirklich fasst, ist die **Differenz**: Wo die Schicht
   * endet, hängt davon ab, wann der LETZTE Schritt fiel.
   *
   * | Kanal | letzter Schritt | Ende der Schicht |
   * | --- | --- | --- |
   * | läuft durch | 5000 ms | 25000 ms |
   * | Abbruch nach einem Schritt | 1000 ms | 21000 ms |
   *
   * Die 4000 ms dazwischen SIND die wandernde Uhr. Würde sie nur einmal
   * gesetzt, endeten beide bei 21000.
   */
  it('die Uhr WANDERT mit: wo die Schicht endet, hängt am LETZTEN Schritt', () => {
    // Gemessen wird der erste Takt, in dem KEINE Schicht mehr steht — nicht
    // der Ablaufzeitpunkt selbst. Der Ablauf greift bei „Zeit GRÖSSER als
    // Endzeit", die Schicht steht bei 25000 ms also noch; weg ist sie im Takt
    // danach. Die Differenz weiter unten ist von dieser Verschiebung frei,
    // weil sie in beiden Messungen gleich ist.
    const TAKT = 10;
    const ersterTaktOhneSchicht = (abbruchNachMs: number): number => {
      const combat = schutzKampf();
      const d = schutzDaten(combat);
      const dauer = combat.spellAt(d.index)?.shield?.durationMs ?? 0;
      expect(dauer, 'der Prüfzauber trägt gar keine Schicht-Dauer').toBeGreaterThan(0);
      expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);
      // Der Abbruch geschieht durch einen anderen Zauber — dieselbe Bauart wie
      // im Einfrier-Fall darüber.
      // **Der Abbruch wird versucht, bis er ANGENOMMEN ist.** Ein einzelner
      // Versuch bei `abbruchNachMs` reicht nicht: Die globale Wartezeit läuft
      // beim Start des Kanals an (1500 ms), ein Druck bei 1000 ms wird also
      // abgelehnt. Der erste Entwurf tat genau das — der Kanal lief in
      // Wahrheit durch, die Messung stand auf -1, und die Zahl hätte einen
      // Fehler in der Uhr behauptet, den es nicht gab.
      let abgebrochen = false;
      // **Erst ab dem Moment messen, in dem eine Schicht STEHT.** Vor dem
      // ersten Raster-Schritt ist sie null, und ohne dieses Merker-Flag meldete
      // die Messung 10 ms als „Ende" — gemessen, und der erste Entwurf ist
      // genau darauf hereingefallen.
      let stand = false;
      // Weit über das erwartete Ende hinaus, damit ein Ablauf auch dann
      // auffällt, wenn er später kommt als gedacht.
      const bis = (abbruchNachMs === Number.POSITIVE_INFINITY ? d.dauer : abbruchNachMs) + dauer;
      for (let t = 0; t <= bis + 2000; t += TAKT) {
        combat.stepPassive(t);
        if (!abgebrochen && t >= abbruchNachMs && abbruchNachMs < d.dauer) {
          abgebrochen = isCastAccepted(combat.cast(combat.indexOf('kindle'), t).result);
        }
        if (combat.shield > 0) stand = true;
        else if (stand) return t;
      }
      return -1;
    };

    const d = schutzDaten(schutzKampf());
    const dauer = schutzKampf().spellAt(d.index)?.shield?.durationMs ?? 0;

    // Durchgelaufen: der letzte Schritt fällt bei `d.dauer`, das Ende liegt
    // eine volle Schicht-Dauer später. **Das ist zugleich Auflage 8, erste
    // Hälfte:** nach dem letzten Schritt hält sie die VOLLE Dauer.
    const endeDurchgelaufen = ersterTaktOhneSchicht(Number.POSITIVE_INFINITY);
    expect(
      endeDurchgelaufen,
      'die Schicht lief im beobachteten Fenster gar nicht ab',
    ).toBeGreaterThan(0);
    expect(endeDurchgelaufen, `erster Takt ohne Schicht: ${endeDurchgelaufen} ms`).toBe(
      d.dauer + dauer + TAKT,
    );

    // Abbruch nach EINEM Schritt: die Uhr steht dann auf dem ersten Schritt,
    // und das Ende liegt genau `d.dauer - d.raster` früher.
    const endeAbgebrochen = ersterTaktOhneSchicht(d.raster);
    expect(
      endeAbgebrochen,
      'im abgebrochenen Lauf lief die Schicht gar nicht ab — dann vergleicht ' +
        'die Zeile darunter eine Zahl mit -1',
    ).toBeGreaterThan(0);
    expect(endeAbgebrochen, `erster Takt ohne Schicht: ${endeAbgebrochen} ms`).toBe(
      d.raster + dauer + TAKT,
    );

    // **Die eigentliche Aussage.** Ohne wandernde Uhr wären beide Zahlen
    // gleich; der Abstand ist die Zeit zwischen erstem und letztem Schritt.
    expect(
      endeDurchgelaufen - endeAbgebrochen,
      `durchgelaufen ${endeDurchgelaufen} ms, abgebrochen ${endeAbgebrochen} ms — ` +
        'gleiche Zahlen hiessen: die Uhr wird nur einmal gesetzt',
    ).toBe(d.dauer - d.raster);
  });

  it('während des Kanals läuft sie NICHT ab — auch nicht der erste Schritt', () => {
    // Auflage 8, zweite Hälfte. Bei dieser Schicht-Dauer ist der Fall für sich
    // genommen schwach (siehe der Kommentar oben), und er steht trotzdem hier:
    // Er deckt die Richtung ab, in der ein Fehler NICHT über die Endzeit
    // sichtbar würde — eine Schicht, die zwischendurch auf 0 fällt und beim
    // nächsten Schritt neu anfängt, hätte dieselbe Endzeit und wäre über den
    // Fall darüber nicht zu fassen.
    const combat = schutzKampf();
    const d = schutzDaten(combat);
    expect(combat.cast(d.index, 0).result).toBe(CAST_STARTED);

    let eingebrochen = -1;
    let hoechster = 0;
    for (let t = 0; t <= d.dauer; t += 10) {
      combat.stepPassive(t);
      if (combat.shield < hoechster && eingebrochen < 0) eingebrochen = t;
      hoechster = Math.max(hoechster, combat.shield);
    }
    expect(eingebrochen, `die Schicht fiel bei ${eingebrochen} ms zurück`).toBe(-1);
    expect(hoechster, 'am Ende des Kanals stand die volle Summe').toBe(d.gesamt);
  });
});
