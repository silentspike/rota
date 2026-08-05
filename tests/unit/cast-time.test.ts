/**
 * Die Wirkzeit der Spieler-Zauber (Issue #304).
 *
 * WAS ES VORHER GAB: EIN FELD, DAS GELESEN UND FALLEN GELASSEN WURDE
 * ------------------------------------------------------------------
 * ```
 * src/core/data.ts:60    readonly castTimeMs: number;              im Schema
 * src/core/data.ts:874   castTimeMs: number(raw['castTimeMs'], 0)  wird geladen
 * ```
 *
 * `Combat.cast` fragte es an keiner Stelle, und alle fünfzehn Zauber standen
 * auf 0 — deshalb ist nie aufgefallen, dass es nichts bewirkt. Die einzigen
 * Fundstellen von `castTimeMs` im Kampf gehörten dem **Boss**.
 *
 * DAS IST KEIN „FELD ANSCHLIESSEN", SONDERN EIN NEUER ZUSTAND
 * -----------------------------------------------------------
 * Für den Spieler gab es kein „wirkt gerade". `cast()` prüfte Abklingzeiten und
 * die globale Wartezeit, sonst nichts. Alles, was hier steht, hängt daran:
 * Sperren, Abbrechen, Prüfsumme, Parität.
 *
 * WAS HIER GEPRÜFT WIRD UND WAS NICHT
 * -----------------------------------
 * Diese Datei misst die MECHANIK an einer Prüf-Schule. Der Balken selbst
 * braucht ein Dokument und ist mit Vitest (`environment: 'node'`) nicht
 * spielbar — er ist im Browser belegt (AC1, AC3). Die Bitgleichheit mit dem
 * Simulator steht in `sim/tests/parity_cast_time.rs`.
 */
import { describe, expect, it } from 'vitest';
import { CAST_BUSY, CAST_OK, CAST_STARTED, Combat, isCastAccepted } from '../../src/core/combat.js';
import { bossById, FIRST_CRUCIBLE, parseSchool, type BossDef } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { TICK_MS } from '../../src/core/loop.js';

/**
 * Eine Prüf-Schule mit einem Sofort-Zauber und einem mit Wirkzeit.
 *
 * **Zwei Zauber, ein Unterschied.** Sie tragen denselben Schaden und dieselbe
 * Abklingzeit; nur `castTimeMs` trennt sie. Alles, was unten verschieden
 * herauskommt, kommt damit aus diesem einen Feld und aus nichts sonst — ein
 * Vergleich zweier Zauber, die sich in drei Dingen unterscheiden, könnte das
 * nicht behaupten.
 */
function pruefSchule(): ReturnType<typeof parseSchool> {
  return parseSchool({
    schema: 1,
    school: 'CALCINATION',
    resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
    spells: [
      { id: 'sofort', name: 'Sofort', cooldownMs: 4000, baseDamage: 100, castTimeMs: 0 },
      { id: 'lang', name: 'Lang', cooldownMs: 4000, baseDamage: 100, castTimeMs: 2000 },
      // Der Unterbrecher, wortgleich zu QUENCH: keine Wartezeit, eigene
      // Abklingzeit. Er trägt den Fall, der als einziger auslegungsbedürftig
      // war — siehe unten.
      { id: 'quench', name: 'Quench', cooldownMs: 5000, baseDamage: 0, triggersGcd: false },
    ],
  });
}

function aufbauen(boss: BossDef = FIRST_CRUCIBLE.value): Combat {
  const schule = pruefSchule();
  expect(schule.problems, 'die Prüf-Schule muss fehlerfrei lesbar sein').toEqual([]);
  const combat = new Combat({ school: schule.value, boss });
  combat.beginFight(0);
  return combat;
}

/** Rechnet die Takte von `vonMs` bis einschließlich `bisMs` durch. */
function laufen(combat: Combat, vonMs: number, bisMs: number): void {
  for (let t = vonMs; t <= bisMs; t += TICK_MS) combat.stepPassive(t);
}

describe('AC2 — der Schaden fällt am ENDE der Wirkzeit', () => {
  it('bucht beim Drücken noch nichts', () => {
    const combat = aufbauen();
    const lang = combat.indexOf('lang');

    const ausgang = combat.cast(lang, 0);
    expect(ausgang.result, 'ein eigener Ausgang, nicht CAST_OK').toBe(CAST_STARTED);
    expect(ausgang.damage).toBe(0);
    expect(combat.damageDealt, 'beim Drücken darf nichts gebucht sein').toBe(0);
    expect(combat.castCount(lang), 'und gewirkt ist er auch noch nicht').toBe(0);
    expect(combat.playerCasting).toBe(lang);
  });

  it('bucht ihn, wenn die Zeit um ist', () => {
    const combat = aufbauen();
    const lang = combat.indexOf('lang');
    combat.cast(lang, 0);

    // Ein Takt VOR dem Ende: noch nichts. Ohne diese Zusicherung bliebe offen,
    // ob der Schaden am Ende fällt oder irgendwann dazwischen.
    laufen(combat, 0, 1990);
    expect(combat.damageDealt, 'bei 1990 ms ist die Zeit noch nicht um').toBe(0);

    combat.stepPassive(2000);
    expect(combat.damageDealt, 'bei 2000 ms fällt er').toBe(100);
    expect(combat.castCount(lang)).toBe(1);
    expect(combat.playerCasting, 'und danach läuft nichts mehr').toBe(-1);
  });

  it('lässt den Sofort-Zauber unverändert sofort wirken — der Gegenfall', () => {
    // Ohne ihn wäre nicht gezeigt, dass die Umstellung NUR Zauber mit Wirkzeit
    // betrifft. Ein Sofort-Zauber läuft seit #304 einen Aufruf tiefer durch
    // dieselbe Stelle; käme er dabei einen Takt später an, merkte es niemand.
    const combat = aufbauen();
    const sofort = combat.indexOf('sofort');

    const ausgang = combat.cast(sofort, 0);
    expect(ausgang.result).toBe(CAST_OK);
    expect(ausgang.damage).toBe(100);
    expect(combat.damageDealt).toBe(100);
    expect(combat.playerCasting, 'er läuft gar nicht erst').toBe(-1);
  });

  it('setzt die Abklingzeit erst am Ende, nicht am Anfang', () => {
    // Die Voraussetzung für AC4: „läuft nicht an" ist nur prüfbar, wenn sie am
    // Anfang noch nicht lief.
    const combat = aufbauen();
    const lang = combat.indexOf('lang');
    combat.cast(lang, 0);

    expect(combat.cooldownRemainingMs(lang, 0), 'am Anfang keine').toBe(0);
    laufen(combat, 0, 2000);
    // 4000 ms Abklingzeit ab dem Ende bei 2000 ms.
    expect(combat.cooldownRemainingMs(lang, 2000)).toBe(4000);
  });
});

describe('AC1 — die Wirkzeit blockiert', () => {
  it('lehnt jeden weiteren Zauber ab, solange sie läuft', () => {
    const combat = aufbauen();
    const lang = combat.indexOf('lang');
    const sofort = combat.indexOf('sofort');
    combat.cast(lang, 0);

    // Bei 1600 ms ist die globale Wartezeit (1500 ms) längst um — der Zauber
    // wird also von der WIRKZEIT gesperrt und nicht von ihr. Ohne diesen
    // Zeitpunkt wäre der Fall grün, auch wenn die Wirkzeit gar nichts sperrte.
    expect(combat.gcd.isReady(1600), 'sonst prüft der Fall die falsche Sperre').toBe(true);
    expect(combat.cast(sofort, 1600).result).toBe(CAST_BUSY);
    expect(combat.castCount(sofort)).toBe(0);
  });

  it('meldet auch über `isReady` gesperrt — daran hängt die Zauberleiste', () => {
    // Zwei Stellen, dieselbe Regel: `cast` entscheidet, `isReady` zeigt an.
    // Fehlte die zweite, zeigte die Leiste während der Wirkzeit lauter bereite
    // Zauber, und jeder Druck darauf ginge ins rote Blinken.
    const combat = aufbauen();
    combat.cast(combat.indexOf('lang'), 0);
    expect(combat.isReady(combat.indexOf('sofort'), 1600)).toBe(false);
  });

  it('SPERRT AUCH DEN UNTERBRECHER, der von der Wartezeit ausgenommen ist', () => {
    // **Die einzige Stelle, an der das Design ausgelegt werden musste**, und
    // sie ist deshalb ausdrücklich festgehalten:
    //
    //   GAME-DESIGN 6.6  „Genau eine Ausnahme: Unterbrechen löst keine globale
    //                     Wartezeit aus."
    //   GAME-DESIGN 7.1  „Blockiert. Während der Balken läuft, kann nichts
    //                     anderes gewirkt werden."
    //
    // Der erste Satz sagt, was der Unterbrecher nicht AUSLÖST — nicht, wovon er
    // ausgenommen ist. Gebaut ist 7.1 wörtlich: Er wird mitgesperrt. Das ist
    // auch der Grund, aus dem 7.1 von einer „echten Verpflichtung" spricht;
    // bliebe der Unterbrecher frei, wäre ein Zauber mit Wirkzeit risikolos.
    //
    // Die Frage liegt beim Auftraggeber. Entscheidet er anders, ist dieser Fall
    // die Stelle, an der es steht — und nicht eine Zeile im Kampfcode, die
    // niemand findet.
    const combat = aufbauen();
    const quench = combat.indexOf('quench');
    combat.cast(combat.indexOf('lang'), 0);

    expect(combat.cast(quench, 1600).result).toBe(CAST_BUSY);
    expect(combat.isReady(quench, 1600)).toBe(false);
  });

  it('gibt danach wieder frei', () => {
    // Sonst bliebe offen, ob die Sperre je endet — ein Kampf, in dem nach dem
    // ersten Zauber mit Wirkzeit nichts mehr geht, bestünde jede Prüfung oben.
    const combat = aufbauen();
    const sofort = combat.indexOf('sofort');
    combat.cast(combat.indexOf('lang'), 0);
    laufen(combat, 0, 2000);
    expect(combat.cast(sofort, 2000).result).toBe(CAST_OK);
  });

  it('startet die globale Wartezeit beim BEGINN, nicht am Ende', () => {
    // GAME-DESIGN 7.1 misst die Wirkzeit in Wartezeiten („1,5–2,5 s, also ein
    // bis knapp zwei globale Wartezeiten"). Dieser Vergleich ergibt nur einen
    // Sinn, wenn beide zugleich laufen; startete sie am Ende, kostete ein
    // 2000-ms-Zauber 3500 ms und die Angabe im Design wäre irreführend.
    const combat = aufbauen();
    combat.cast(combat.indexOf('lang'), 0);
    expect(combat.gcd.readyAt).toBe(GCD_MS);

    // Und sie wird am Ende nicht ein zweites Mal gestartet.
    laufen(combat, 0, 2000);
    expect(combat.gcd.readyAt, 'sonst wären es drei Wartezeiten statt zwei').toBe(GCD_MS);
  });
});

describe('AC4 — der Abbruch durch einen Boss-Angriff, beide Hälften', () => {
  const CASTER = bossById('test-caster');
  if (CASTER === undefined) throw new Error('data/bosses/test-caster.json fehlt.');

  /**
   * Wirkt `lang` so, dass die Wirkzeit über den Schlag bei `schlagMs` läuft.
   *
   * Der Boss schlägt bei 8500 (harmlos) und 9000 ms (abbrechend). Gewirkt wird
   * bei 8000 ms, die Wirkzeit liefe also bis 10000 ms.
   */
  function bisSchlag(bisMs: number): Combat {
    const combat = aufbauen(CASTER);
    laufen(combat, 0, 7990);
    expect(combat.cast(combat.indexOf('lang'), 8000).result).toBe(CAST_STARTED);
    laufen(combat, 8000, bisMs);
    return combat;
  }

  it('ERSTE HÄLFTE: der Zauber geht verloren', () => {
    const combat = bisSchlag(9000);
    const lang = combat.indexOf('lang');

    expect(combat.playerCasting, 'die Wirkzeit ist weg').toBe(-1);
    expect(combat.castsInterrupted).toBe(1);
    // Und der Schaden fällt auch später nicht nach: Der Zauber ist verloren,
    // nicht verschoben.
    laufen(combat, 9000, 12_000);
    expect(combat.castCount(lang), 'er darf nicht doch noch wirken').toBe(0);
    expect(combat.damageDealt, 'und keinen Schaden buchen').toBe(0);
  });

  it('ZWEITE HÄLFTE: die Abklingzeit läuft NICHT an', () => {
    // **Die Regel, die gern vergessen wird** (GAME-DESIGN 7.1). Wer nur die
    // erste Hälfte baut, hat einen Zauber, der bei Abbruch doppelt kostet — und
    // ein Spieler, der ihn deshalb nie wieder anfängt, hat recht.
    const combat = bisSchlag(9000);
    const lang = combat.indexOf('lang');

    expect(combat.cooldownRemainingMs(lang, 9000), 'keine Abklingzeit').toBe(0);
    expect(combat.isOffCooldown(lang, 9000)).toBe(true);
    // Und der Beweis in der Handlung: Sobald die Wartezeit frei ist, geht er
    // wieder. Die Wartezeit stammt vom Beginn bei 8000 ms und endet bei 9500.
    laufen(combat, 9000, 9500);
    expect(combat.cast(lang, 9500).result).toBe(CAST_STARTED);
  });

  it('EIN GEWÖHNLICHER TREFFER STÖRT NICHT — der Gegenfall', () => {
    // GAME-DESIGN 7.1: „nur durch **bestimmte**, vorher erkennbare
    // Boss-Angriffe. Normale Treffer stören nicht."
    //
    // Ohne diesen Fall wäre eine Umsetzung, in der JEDER Treffer abbricht,
    // ebenso grün — und aus einer lernbaren Mechanik würde Zufallsärger.
    const combat = bisSchlag(8500);
    const lang = combat.indexOf('lang');

    expect(combat.damageTaken, 'der Schlag muss angekommen sein').toBeGreaterThan(0);
    expect(combat.playerCasting, 'die Wirkzeit läuft weiter').toBe(lang);
    expect(combat.castsInterrupted).toBe(0);
  });

  it('trägt den Unterschied in den DATEN, nicht im Kampfcode', () => {
    // Sonst wäre „bestimmte Angriffe" eine Regel des Spiels statt eine
    // Eigenschaft dieses Bosses — und ein neuer Boss brauchte eine
    // Code-Änderung statt eines Datensatzes (Leitplanke 6).
    const abbrechend = CASTER.timeline.filter((e) => e.interruptsPlayer);
    const harmlos = CASTER.timeline.filter((e) => !e.interruptsPlayer);
    expect(abbrechend.map((e) => e.atMs)).toEqual([9000, 18_000]);
    expect(harmlos.map((e) => e.atMs)).toEqual([8500]);
    // Und der Erste Schmelztiegel bricht gar nichts ab: Ein bestehender Boss
    // ändert sein Verhalten durch dieses Issue nicht.
    expect(FIRST_CRUCIBLE.value.timeline.some((e) => e.interruptsPlayer)).toBe(false);
  });
});

describe('Der Ausgang eines Tastendrucks', () => {
  it('zählt einen begonnenen Zauber als angenommen', () => {
    // Sonst bekäme der Spieler für einen richtigen Druck ein rotes Blinken.
    // Die Regel steht an einer Stelle, weil zwei Aufrufer sie brauchen:
    // `Session.apply` im Spiel und `replayRecording` in den Werkzeugen.
    expect(isCastAccepted(CAST_OK)).toBe(true);
    expect(isCastAccepted(CAST_STARTED)).toBe(true);
    expect(isCastAccepted(CAST_BUSY)).toBe(false);
  });
});
