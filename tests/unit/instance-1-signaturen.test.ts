/**
 * Jede Signatur der ersten Instanz ändert den Verlauf (Issue #544, AC6).
 *
 * # Warum diese Fälle als Test stehen und nicht als Messung in einer Meldung
 *
 * Jeder der vier Bosse nennt in seinem `signature`-Feld einen Baustein, der
 * ihn besonders macht. Ob dieser Baustein den Kampf **tatsächlich** ändert,
 * lässt sich nur durch Wegnehmen prüfen — und genau das ist beim Bau
 * schiefgegangen:
 *
 * `boss-1-2` sollte zuerst seine **Phasen** als Signatur tragen. Der Boss war
 * gebaut, die Beschreibung klang stimmig, und die Messung ergab **1720
 * angekommenen Schaden mit Phasen und 1720 ohne**. Ein Phasenwechsel setzt
 * einen Zähler und schreibt ein Ereignis; er ändert nichts. Die Signatur
 * beschrieb etwas, das im Verlauf nicht vorkommt.
 *
 * Gefunden hat das keine Regel, sondern eine Zahl. Deshalb stehen die
 * Wegnahmen hier als Prüffälle: Beim nächsten Mal meldet es sich von selbst,
 * ohne dass jemand daran denken muss.
 *
 * # Was ein Fall hier NICHT behauptet
 *
 * Er sagt **nicht**, dass die Signatur gut gewählt oder ausgewogen ist. Er
 * sagt nur: Nimmt man den Baustein weg, den sie nennt, verläuft der Kampf
 * messbar anders. Eine Signatur, die das nicht schafft, ist eine
 * Beschreibung — und die soll auffallen.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { BOSSES, CALCINATION } from '../../src/core/data.js';
import type { BossDef } from '../../src/core/data.js';

function boss(id: string): BossDef {
  const eintrag = BOSSES.get(id);
  // **Die Zusicherung vor der Messung.** Fehlt der Boss, liefe jeder Fall
  // darunter auf `undefined` und meldete etwas anderes als das Fehlen.
  expect(eintrag, `${id} steht nicht in BOSSES`).toBeDefined();
  return eintrag!.value;
}

/** Fährt einen Kampf ohne Tastendrücke und gibt zurück, was der Boss anrichtet. */
function nurBoss(b: BossDef, bisMs = 60_000): Combat {
  const c = new Combat({ school: CALCINATION.value, boss: b });
  for (let t = 0; t <= bisMs; t += 10) c.stepPassive(t);
  return c;
}

/**
 * Wie `nurBoss`, aber der Spieler drückt KINDLE, sooft er darf.
 *
 * **Nötig für boss-1-4.** Ein Boss, dem niemand Schaden zufügt, kann sich
 * auch nicht heilen: Der erste Entwurf maß dort `bossHealed === 0` in beiden
 * Läufen und wäre grün gewesen, hätte die Zusicherung „der Boss heilt sich
 * nicht — dann misst der Fall nichts" nicht davorgestanden.
 */
function mitSpieler(b: BossDef, bisMs = 60_000): Combat {
  const c = new Combat({ school: CALCINATION.value, boss: b });
  const kindle = c.indexOf('kindle');
  for (let t = 0; t <= bisMs; t += 10) {
    c.stepPassive(t);
    if (c.damageDealt >= c.boss.hp) break;
    if (c.gcd.isReady(t) && c.isOffCooldown(kindle, t)) c.cast(kindle, t);
  }
  return c;
}

describe('boss-1-1 — die Signatur trägt die unterbrechbaren Zauber', () => {
  it('ohne ihren Schaden kommt deutlich weniger an', () => {
    const mit = nurBoss(boss('boss-1-1'));
    const ohne = nurBoss({
      ...boss('boss-1-1'),
      timeline: boss('boss-1-1').timeline.map((e) =>
        e.mechanic === 'interruptibleCast' ? { ...e, damage: 0 } : e,
      ),
    });

    expect(mit.damageTaken, 'ohne eingesteckten Schaden misst der Fall nichts').toBeGreaterThan(0);
    expect(ohne.damageTaken).toBeLessThan(mit.damageTaken);
  });
});

describe('boss-1-2 — die Signatur trägt das Verhängnis', () => {
  it('ohne doom füllt sich kein Zähler', () => {
    const mit = nurBoss(boss('boss-1-2'));
    const ohne = nurBoss({ ...boss('boss-1-2'), doom: null });

    expect(mit.doom, 'der Zähler steigt nicht — dann prüft der Fall nichts').toBeGreaterThan(0);
    expect(ohne.doom).toBe(0);
  });

  it('und die PHASEN ändern nichts — sie sind Kulisse, kein Baustein', () => {
    // **Der Fall, aus dem dieser Prüfstand entstanden ist.** Er hält fest,
    // was gemessen wurde, damit niemand die Phasen erneut für eine Mechanik
    // hält: Sie stehen in `boss-1-2`, und ihr Wegnehmen ändert nichts.
    //
    // Fällt dieser Fall eines Tages, ist das eine gute Nachricht — dann hat
    // #557 aus der Kulisse eine Mechanik gemacht, und die Signatur des
    // Bosses darf neu bewertet werden.
    const mit = nurBoss(boss('boss-1-2'));
    const ohne = nurBoss({ ...boss('boss-1-2'), phases: [] });

    expect(boss('boss-1-2').phases.length, 'der Boss führt keine Phasen mehr').toBeGreaterThan(0);
    expect(ohne.damageTaken).toBe(mit.damageTaken);
  });
});

describe('boss-1-3 — die Signatur trägt das Gift', () => {
  it('ohne affliction fehlt der laufende Verlust', () => {
    const mit = nurBoss(boss('boss-1-3'));
    const ohne = nurBoss({ ...boss('boss-1-3'), affliction: null });

    expect(mit.damageTaken).toBeGreaterThan(ohne.damageTaken);
    // Der Unterschied muss die Größenordnung des Gifts haben, nicht ein
    // Rundungsrest: 35 je Sekunde ab der neunten sind über 1000.
    expect(mit.damageTaken - ohne.damageTaken).toBeGreaterThan(1000);
  });
});

describe('boss-1-4 — die Signatur trägt ZWEI Bausteine, getrennt geprüft', () => {
  /**
   * `race` entscheidet, ob die Schwelle verfehlt wird; `bossHeal` führt die
   * Heilung aus. Beide einzeln, weil sie aus verschiedenen Gründen wirken —
   * ein gemeinsamer Fall wäre schon grün, wenn nur einer greift.
   */
  it('die Heilung haengt an der verpassten Unterbrechung, NICHT am Rennen', () => {
    // **Dieser Fall hat die erste Fassung der Signatur widerlegt.** Sie nannte
    // `race` als Träger, weil am Balance-Läufer beide Wegnahmen den
    // ausgeteilten Schaden bewegten. Der Läufer gibt aber Schaden aus, nicht
    // Heilung — die Wirkung war gesehen, die Ursache erfunden.
    //
    // Gemessen wird deshalb `bossHealed` selbst: Ohne `race` heilt der Boss
    // unverändert weiter. Ausgelöst wird sie in combat.ts:2885, nachdem ein
    // Wirkzauber durchgekommen ist.
    const mit = mitSpieler(boss('boss-1-4'));
    const ohneRennen = mitSpieler({ ...boss('boss-1-4'), race: null });

    expect(
      mit.bossHealed,
      'der Boss heilt sich nicht — dann misst der Fall nichts',
    ).toBeGreaterThan(0);
    expect(ohneRennen.bossHealed).toBe(mit.bossHealed);
  });

  it('ohne bossHeal bleibt die verfehlte Schwelle folgenlos', () => {
    const mit = mitSpieler(boss('boss-1-4'));
    const ohne = mitSpieler({ ...boss('boss-1-4'), bossHeal: null });

    expect(mit.bossHealed).toBeGreaterThan(0);
    expect(ohne.bossHealed).toBe(0);
  });
});

describe('boss-1-1 — die Verschärfung ist mit dem Balance-Läufer NICHT messbar', () => {
  /**
   * `escalation` verkürzt die Wirkzeit unterbrechbarer Zauber je Zyklus. Der
   * Balance-Läufer zeigt davon **nichts**: Er fährt eine Rotation ohne
   * Unterbrecher, der Zauber kommt so oder so durch, und ob seine Wirkzeit
   * 2500 oder 1200 ms beträgt, ändert das Ergebnis nicht (1920 gegen 1920).
   *
   * Das ist der Unterschied zu `phases`, und er entscheidet, ob ein Baustein
   * bleiben darf:
   *
   * | | wird gelesen? | wirkt? |
   * | --- | --- | --- |
   * | `phases` | nein | nie |
   * | `escalation` | ja | nur für einen Spieler, der unterbricht |
   *
   * Beim ersten ist der Baustein eine Beschriftung. Beim zweiten war meine
   * Messung zu grob. Deshalb wird hier die Größe gelesen, um die es geht —
   * die Wirkzeit selbst — statt einer, die sich mitbewegt.
   */
  it('die Wirkzeit schrumpft mit jedem Zyklus', () => {
    const b = boss('boss-1-1');
    expect(b.escalation, 'ohne escalation misst der Fall nichts').not.toBeNull();
    expect(b.timelineLoopMs, 'escalation wirkt nur mit timelineLoopMs').not.toBeNull();

    // Die Wirkzeiten aller Zauber sammeln, die im Lauf angefangen werden.
    const c = new Combat({ school: CALCINATION.value, boss: b });
    const dauern: number[] = [];
    let laufend = -1;
    for (let t = 0; t <= 120_000; t += 10) {
      c.stepPassive(t);
      if (c.casting !== -1 && c.casting !== laufend) {
        dauern.push(c.castLandsAtMs - c.castingStartedAtMs);
        laufend = c.casting;
      } else if (c.casting === -1) {
        laufend = -1;
      }
    }

    expect(dauern.length, 'kein Wirkzauber begonnen — dann prüft der Fall nichts').toBeGreaterThan(
      2,
    );
    // **Die Aussage:** Später ist kürzer. Nicht jeder Schritt einzeln — die
    // Untergrenze (`minCastTimeMs`) sorgt dafür, dass es irgendwann aufhört.
    expect(dauern[dauern.length - 1]).toBeLessThan(dauern[0] as number);
  });
});

describe('boss-1-4 — bedingtes Unterbrechen lohnt sich, unbedingtes nicht', () => {
  /**
   * Der Kern von AC2 für diesen Boss — und der Fall, der einen umgekehrten
   * Befund festhält.
   *
   * Am Balance-Läufer gemessen sah es so aus, als koste der Unterbrecher mehr
   * als er bringt (4380 gegen 4204 ausgeteilt). Dort steht QUENCH aber ganz
   * vorn und wird gewirkt, **sobald er bereit ist** — er verdrängt dann
   * laufend Schadenszauber. Der Läufer kann ihn nicht bedingen (#553), misst
   * also zwangsläufig die naive Nutzung.
   *
   * Wer nur drückt, **während der Boss wirkt**, drückt einmal statt zwanzigmal:
   *
   * ```text
   * roh            netto 2900   geheilt 800   verpasst 1
   * unterbrechen   netto 3600   geheilt   0   verpasst 0
   * ```
   *
   * 700 mehr Schaden. Die Signatur verlangt also etwas, das sich lohnt — nur
   * belegen lässt es sich mit dem Läufer nicht.
   */
  function lauf(mitQuench: boolean): Combat {
    const c = new Combat({ school: CALCINATION.value, boss: boss('boss-1-4') });
    const kindle = c.indexOf('kindle');
    const quench = c.indexOf('quench');
    for (let t = 0; t <= 60_000; t += 10) {
      c.stepPassive(t);
      if (c.damageDealt >= c.boss.hp) break;
      if (!c.gcd.isReady(t)) continue;
      // **Die Bedingung IST die Aussage.** Ohne sie misst der Fall die naive
      // Nutzung und käme zum gegenteiligen Ergebnis.
      if (mitQuench && c.casting !== -1 && c.isOffCooldown(quench, t)) c.cast(quench, t);
      else if (c.isOffCooldown(kindle, t)) c.cast(kindle, t);
    }
    return c;
  }

  it('wer im richtigen Moment unterbricht, teilt mehr aus', () => {
    const roh = lauf(false);
    const mit = lauf(true);

    // Die Zusicherungen zuerst: Ohne verpasste Unterbrechung und ohne Heilung
    // vergliche der Fall zwei gleiche Läufe.
    expect(roh.interruptsMissed, 'nichts verpasst — dann prüft der Fall nichts').toBeGreaterThan(0);
    expect(roh.bossHealed, 'der Boss heilt nicht — dann fehlt der Anreiz').toBeGreaterThan(0);
    expect(mit.interruptsMissed, 'auch mit QUENCH wurde etwas verpasst').toBe(0);
    expect(mit.bossHealed).toBe(0);

    expect(mit.damageDealt).toBeGreaterThan(roh.damageDealt);
  });

  it('und das Brutto zeigt, was das Unterbrechen KOSTET', () => {
    // `undoProgress` senkt `damageDealt` selbst — die ausgeteilte Zahl ist
    // bereits netto. Das Brutto ergibt sich erst durch Addition der Heilung,
    // und ohne diese Unterscheidung zog eine frühere Rechnung die Heilung
    // zweimal ab.
    const roh = lauf(false);
    const mit = lauf(true);

    const bruttoRoh = roh.damageDealt + roh.bossHealed;
    const bruttoMit = mit.damageDealt + mit.bossHealed;

    // Der Unterbrecher kostet einen Leistenplatz — brutto steht er schlechter
    // da. Erst die gesparte Heilung dreht es um.
    expect(bruttoMit).toBeLessThan(bruttoRoh);
    expect(mit.damageDealt).toBeGreaterThan(roh.damageDealt);
  });
});

describe('boss-1-4 — AC2: bei 4900 gewinnt NUR, wer unterbricht', () => {
  /**
   * Beide Ränder, nicht nur der Sieg.
   *
   * Ein Fall, der allein prüft „mit QUENCH gewinnt man", hält auch dann noch,
   * wenn jemand die Heilung entfernt — dann gewinnen beide, und die Signatur
   * ist still verschwunden. **Der Verlierer-Fall ist der wichtigere.**
   *
   * Die 4900 sind gemessen, nicht gewählt. Das Fenster, in dem der Boss seine
   * Signatur verlangt, ist 4569 bis 5195:
   *
   * ```text
   * 4568 = 5368 (roh brutto) − 800 (Heilung)   darunter gewinnt auch die rohe
   * 5196 = netto mit Unterbrechen              darüber gewinnt niemand
   * ```
   *
   * Beide Ränder sind Kippgrenzen ohne Abstand — 4900 hat den größten Abstand
   * zu beiden Fehlerarten.
   */
  function siegBei(mitQuench: boolean): number {
    const b = boss('boss-1-4');
    const c = new Combat({ school: CALCINATION.value, boss: b });
    const i = (id: string): ReturnType<typeof c.indexOf> => c.indexOf(id);
    for (let t = 0; t <= 60_000; t += 10) {
      c.stepPassive(t);
      if (c.damageDealt >= b.hp) return t;
      if (!c.gcd.isReady(t)) continue;
      if (mitQuench && c.casting !== -1 && c.isOffCooldown(i('quench'), t)) c.cast(i('quench'), t);
      else if (!c.isDotActive(i('smoulder'), t) && c.isOffCooldown(i('smoulder'), t))
        c.cast(i('smoulder'), t);
      else if (c.resource >= 15 && c.isOffCooldown(i('pyre'), t)) c.cast(i('pyre'), t);
      else if (c.isOffCooldown(i('kindle'), t)) c.cast(i('kindle'), t);
    }
    return -1;
  }

  it('WER NICHT UNTERBRICHT, VERLIERT — der wichtigere der beiden Fälle', () => {
    expect(boss('boss-1-4').hp, 'die Lebenspunkte sind nicht mehr 4900').toBe(4900);
    expect(siegBei(false)).toBe(-1);
  });

  it('wer unterbricht, gewinnt — und nicht trivial (Restzeit unter 40 %)', () => {
    const t = siegBei(true);
    expect(t, 'auch mit Unterbrechen kein Sieg').toBeGreaterThan(0);
    // AC2 verlangt Restzeit unter 40 %. Gemessen sind es 7 %.
    expect((60_000 - t) / 60_000).toBeLessThan(0.4);
  });
});

describe('AC2 — der Sieger LEBT', () => {
  /**
   * Der Fall, der eine falsche Zahl in einem gemergten PR aufgedeckt hätte.
   *
   * `boss-1-1` meldete „Sieg nach 51010 ms, Restzeit 15 %". Der Spieler war
   * zu diesem Zeitpunkt **seit 6,3 Sekunden tot** — `checkDefeat()` prüft den
   * Tod nicht, und meine Messschleife brach bei `damageDealt >= boss.hp` ab,
   * ohne nach dem Leben zu fragen. Ein toter Spieler wirkt weiter und legt
   * den Boss irgendwann um.
   *
   * Die Zahl war trotzdem richtig — aber aus dem falschen Grund: Meine
   * Rotation ließ QUENCH weg, obwohl `boss-1-1` die Unterbrechung **als
   * Signatur** verlangt. Mit ihr überlebt der Spieler mit 3560 von 5000
   * Lebenspunkten, und der Siegzeitpunkt verschiebt sich um 60 ms.
   *
   * **Ein AC2-Beleg, der gegen eine Rotation ohne die Signatur des Bosses
   * gemessen wurde, ist kein Beleg.** Hier war das Ergebnis zufällig richtig.
   */
  function ausgang(bossId: string): { sieg: number; tot: number; hp: number } {
    const b = boss(bossId);
    const c = new Combat({ school: CALCINATION.value, boss: b });
    const i = (id: string): ReturnType<typeof c.indexOf> => c.indexOf(id);
    for (let t = 0; t <= 60_000; t += 10) {
      c.stepPassive(t);
      if (c.damageDealt >= b.hp) return { sieg: t, tot: c.diedAtMs, hp: c.playerHp };
      if (!c.gcd.isReady(t)) continue;
      // **QUENCH gehört in JEDE dieser Rotationen.** Drei der vier Bosse
      // verlangen die Unterbrechung; ihn wegzulassen misst einen Spieler,
      // der die Signatur ignoriert.
      if (c.casting !== -1 && c.isOffCooldown(i('quench'), t)) c.cast(i('quench'), t);
      else if (!c.isDotActive(i('smoulder'), t) && c.isOffCooldown(i('smoulder'), t))
        c.cast(i('smoulder'), t);
      else if (c.resource >= 15 && c.isOffCooldown(i('pyre'), t)) c.cast(i('pyre'), t);
      else if (c.isOffCooldown(i('kindle'), t)) c.cast(i('kindle'), t);
    }
    return { sieg: -1, tot: c.diedAtMs, hp: c.playerHp };
  }

  for (const id of ['boss-1-1', 'boss-1-3', 'boss-1-4']) {
    it(`${id}: wer gewinnt, lebt auch`, () => {
      const r = ausgang(id);
      expect(r.sieg, `${id} ist mit dieser Rotation nicht besiegbar`).toBeGreaterThan(0);
      expect(r.tot, 'der Sieger ist tot — die AC2-Zahl misst eine Leiche').toBe(-1);
      expect(r.hp, 'der Sieger hat keine Lebenspunkte mehr').toBeGreaterThan(0);
    });
  }

  it('boss-1-2 dagegen NICHT — und das ist der bekannte Blocker', () => {
    // Er stirbt an `doom`, und dagegen hilft keine Rotation: `clearDoom()`
    // hängt an keinem Zauber (#561). Der Fall hält den Zustand fest, statt
    // ihn auszulassen — verschwindet er, ist der Blocker behoben.
    const r = ausgang('boss-1-2');
    expect(r.tot, 'boss-1-2 ist plötzlich überlebbar — dann ist #561 behoben').toBeGreaterThan(-1);
  });
});
