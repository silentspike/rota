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
import {
  CAST_ON_COOLDOWN,
  CAST_ON_GCD,
  CAST_OK,
  CAST_STARTED,
  Combat,
  isCastAccepted,
} from '../../src/core/combat.js';
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
    resource: {
      id: 'heat',
      name: 'Heat',
      cap: 20,
      decayPerStepMs: 2000,
      decayAmount: 1,
      highlightAt: 0,
    },
    spells: [
      {
        id: 'sofort',
        name: 'Sofort',
        unlockLevel: 1,
        cooldownMs: 4000,
        baseDamage: 100,
        castTimeMs: 0,
        triggersGcd: true,
      },
      {
        id: 'lang',
        name: 'Lang',
        unlockLevel: 1,
        cooldownMs: 4000,
        baseDamage: 100,
        castTimeMs: 2000,
        triggersGcd: true,
      },
      // Der Unterbrecher, wortgleich zu QUENCH: keine Wartezeit, eigene
      // Abklingzeit. Er trägt den Fall, der als einziger auslegungsbedürftig
      // war — siehe unten.
      {
        id: 'quench',
        name: 'Quench',
        unlockLevel: 1,
        cooldownMs: 5000,
        baseDamage: 0,
        castTimeMs: 0,
        triggersGcd: false,
      },
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

  it('AC3 — ein Sofort-Zauber hat nichts abzubrechen', () => {
    // Der Gegenfall zu AC1, und er misst seit der Umkehrung etwas anderes als
    // vorher: nicht mehr „er blockiert nicht", sondern „es gibt nichts, was er
    // zerstören könnte". Ohne ihn wäre nicht gezeigt, dass der Abbruch an der
    // laufenden WIRKZEIT hängt und nicht am Nachdrücken an sich.
    const combat = aufbauen();
    const sofort = combat.indexOf('sofort');
    combat.cast(sofort, 0);
    laufen(combat, 0, 1500);
    // Ein zweiter Druck, dieselbe Lage wie in AC1 — nur ohne laufende Wirkzeit.
    combat.cast(sofort, 4000);
    expect(combat.castCount(sofort)).toBe(2);
    expect(combat.castsCancelled, 'es gab nichts abzubrechen').toBe(0);
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

describe('AC1 — die Wirkzeit sperrt nichts, und jede Wirkung bricht sie ab', () => {
  /**
   * **Diese Gruppe stand am 2026-08-05 einmal umgekehrt da.** Sie hieß „die
   * Wirkzeit blockiert" und prüfte, dass ein zweiter Druck abgewiesen wird —
   * gebaut nach GAME-DESIGN 7.1 in seiner damaligen Fassung („Blockiert.
   * Während der Balken läuft, kann nichts anderes gewirkt werden").
   *
   * Der Auftraggeber hat die Regel umgedreht, und 7.1 trägt sie jetzt:
   *
   * > Jede Wirkung des Spielers bricht einen laufenden Cast oder eine
   * > Kanalisierung ab. Jede, ohne Ausnahme — auch das Unterbrechen.
   *
   * Der Zustand blieb derselbe, nur die Antwort auf einen zweiten Druck hat
   * sich gedreht: nicht „ich kann nicht", sondern „es kostet mich".
   */
  it('lässt den zweiten Druck DURCH und bricht den laufenden Zauber ab', () => {
    const combat = aufbauen();
    const lang = combat.indexOf('lang');
    const sofort = combat.indexOf('sofort');
    combat.cast(lang, 0);

    // Bei 1600 ms ist die globale Wartezeit (1500 ms) um — der zweite Druck
    // scheitert also an nichts anderem. Ohne diesen Zeitpunkt wäre der Fall
    // auch grün, wenn die Wartezeit ihn abgewiesen hätte.
    expect(combat.gcd.isReady(1600), 'sonst prüft der Fall die falsche Regel').toBe(true);
    expect(combat.cast(sofort, 1600).result).toBe(CAST_OK);
    expect(combat.castCount(sofort), 'er ist gewirkt worden').toBe(1);

    expect(combat.playerCasting, 'und der laufende ist weg').toBe(-1);
    expect(combat.castsCancelled).toBe(1);
    // Der abgebrochene wirkt auch später nicht nach.
    laufen(combat, 1600, 4000);
    expect(combat.castCount(lang), 'seine Wirkung ist vollständig verloren').toBe(0);
  });

  it('meldet über `isReady` KEINE Sperre — daran hängt die Zauberleiste', () => {
    // Die Gegenrichtung zur alten Fassung: Zeigte die Leiste die anderen Zauber
    // als gesperrt, wüsste der Spieler nicht, dass er sie drücken KANN — und
    // die Entscheidung, um die es geht, stünde ihm gar nicht offen.
    const combat = aufbauen();
    combat.cast(combat.indexOf('lang'), 0);
    expect(combat.isReady(combat.indexOf('sofort'), 1600)).toBe(true);
  });

  it('BRICHT AUCH DURCH DEN UNTERBRECHER AB — „egal welcher Spell"', () => {
    // **Die Stelle, an der die Regel am 2026-08-05 gekippt ist.** Sie stand
    // hier vorher als „SPERRT AUCH DEN UNTERBRECHER"; die Frage war offen und
    // ist entschieden:
    //
    //   GAME-DESIGN 7.1  „Das gilt auch für Unterbrechen. Egal welcher Spell."
    //   GAME-DESIGN 6.6  „Genau eine Ausnahme: Unterbrechen löst keine globale
    //                     Wartezeit aus."
    //
    // Die zwei vertragen sich: Der Unterbrecher löst keine Wartezeit aus UND
    // bricht trotzdem ab. Er ist damit der einzige Zauber, der eine laufende
    // Wirkzeit auch innerhalb der ersten 1500 ms abbrechen kann — alle anderen
    // hängen so lange an der Wartezeit.
    const combat = aufbauen();
    const quench = combat.indexOf('quench');
    combat.cast(combat.indexOf('lang'), 0);

    // Bei 500 ms läuft die Wartezeit noch. Genau deshalb dieser Zeitpunkt: Er
    // zeigt beides auf einmal — der Unterbrecher kommt durch, und er bricht ab.
    expect(combat.gcd.isReady(500), 'sonst zeigt der Fall die Ausnahme nicht').toBe(false);
    expect(combat.cast(quench, 500).result).toBe(CAST_OK);
    expect(combat.playerCasting).toBe(-1);
    expect(combat.castsCancelled).toBe(1);
  });

  it('bricht NICHT ab, wenn der Druck an der Wartezeit scheitert', () => {
    // **Die Grenze der Regel, und sie ist wichtiger als sie aussieht.** „Jede
    // WIRKUNG bricht ab" — ein Druck, den die Wartezeit abweist, ist keine.
    // Bräche er trotzdem ab, kostete ein Fehlgriff den laufenden Zauber, und
    // aus einer Entscheidung würde eine Falle.
    const combat = aufbauen();
    const lang = combat.indexOf('lang');
    const sofort = combat.indexOf('sofort');
    combat.cast(lang, 0);

    expect(combat.cast(sofort, 500).result).toBe(CAST_ON_GCD);
    expect(combat.playerCasting, 'der Zauber läuft weiter').toBe(lang);
    expect(combat.castsCancelled).toBe(0);
  });

  it('bricht NICHT ab, wenn der Druck an der Abklingzeit scheitert', () => {
    // Dieselbe Grenze, der zweite Grund. Zwei Prüffälle und nicht einer, weil
    // die zwei Bedingungen in `cast` getrennt stehen — und eine davon könnte
    // vor den Abbruch rutschen, ohne dass die andere es meldet.
    const combat = aufbauen();
    const sofort = combat.indexOf('sofort');
    const lang = combat.indexOf('lang');
    // SOFORT wirken, damit er auf Abklingzeit liegt (4000 ms) …
    combat.cast(sofort, 0);
    laufen(combat, 0, 1500);
    // … dann den langen anfangen und SOFORT nachdrücken, während er noch liegt.
    expect(combat.cast(lang, 1500).result).toBe(CAST_STARTED);
    laufen(combat, 1500, 3000);
    expect(combat.cast(sofort, 3000).result).toBe(CAST_ON_COOLDOWN);
    expect(combat.playerCasting, 'der Zauber läuft weiter').toBe(lang);
    expect(combat.castsCancelled).toBe(0);
  });

  it('zählt den eigenen Abbruch GETRENNT vom Abbruch durch den Boss', () => {
    // Für den Spieler sind das zwei verschiedene Dinge: Das eine hat der Boss
    // getan, das andere er selbst. Eine Analyse, die beides zusammenzählte,
    // könnte ihm nicht sagen, welches davon sein Fehler war.
    const combat = aufbauen();
    combat.cast(combat.indexOf('lang'), 0);
    combat.cast(combat.indexOf('sofort'), 1600);
    expect(combat.castsCancelled).toBe(1);
    expect(combat.castsInterrupted, 'der Boss war nicht beteiligt').toBe(0);
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

  it('ZWEITER AUSLÖSER: der eigene Druck — Wirkung verloren', () => {
    // Seit dem 2026-08-05 hat der Abbruch zwei Auslöser. Die drei Aussagen
    // oben gelten für beide, und sie sind für beide einzeln zu belegen: Der
    // Boss räumt `playerCasting` über `stepPassive`, der eigene Druck über
    // `cast` — zwei Aufrufer derselben Stelle, und einer kann wegfallen, ohne
    // dass der andere es meldet.
    const combat = aufbauen();
    const lang = combat.indexOf('lang');
    combat.cast(lang, 0);
    combat.cast(combat.indexOf('sofort'), 1600);

    expect(combat.playerCasting).toBe(-1);
    laufen(combat, 1600, 5000);
    expect(combat.castCount(lang), 'er darf nicht doch noch wirken').toBe(0);
  });

  it('ZWEITER AUSLÖSER: der eigene Druck — die Abklingzeit läuft NICHT an', () => {
    // **Dieselbe Auflage wie beim Boss-Abbruch**, und sie ist hier sogar
    // wichtiger: Ein Spieler, der seinen Zauber selbst abbricht, hat eine
    // Entscheidung getroffen. Sie soll ihn den laufenden Zauber kosten — nicht
    // zusätzlich die Abklingzeit.
    const combat = aufbauen();
    const lang = combat.indexOf('lang');
    combat.cast(lang, 0);
    combat.cast(combat.indexOf('sofort'), 1600);

    expect(combat.cooldownRemainingMs(lang, 1600)).toBe(0);
    expect(combat.isOffCooldown(lang, 1600)).toBe(true);
    // Und in der Handlung: Sobald die Wartezeit des Nachdrucks frei ist, geht
    // er wieder an. 1600 + 1500 = 3100.
    laufen(combat, 1600, 3100);
    expect(combat.cast(lang, 3100).result).toBe(CAST_STARTED);
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
    expect(isCastAccepted(CAST_ON_GCD)).toBe(false);
    expect(isCastAccepted(CAST_ON_COOLDOWN)).toBe(false);
  });
});
