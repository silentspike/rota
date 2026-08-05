/**
 * Die Widerstände der Bosse (GAME-DESIGN 11.4, Issue #328).
 *
 * WAS ES VORHER GAB: EINE MECHANIK OHNE EINE EINZIGE DATENZEILE
 * -------------------------------------------------------------
 * Der Faktor ist im Spiel seit langem verdrahtet — `combat.ts` liest
 * `boss.resistances[schule]` und legt ihn als Topf in die Schadensformel
 * (`damage.ts`). Nur trug **keine der elf Boss-Dateien** einen Eintrag, und
 * `?? 1` machte daraus überall die Eins.
 *
 * **Der Simulator kannte das Feld gar nicht.** `sim/build.rs` enthielt die
 * Zeichenfolge „resist" null mal; `factors.resistance` blieb dort auf 1,0.
 * Spiel und Simulator rechneten also verschieden — unsichtbar, weil beide
 * Seiten bei leeren Daten auf dieselbe Eins kamen. Eine Abweichung, die auf
 * ihre erste Datenzeile gewartet hat.
 *
 * ZWEI BÖSSE, EIN FELD UNTERSCHIED
 * --------------------------------
 * `test-resistant` (0,75) und `test-vulnerable` (1,25) sind Zeichen für
 * Zeichen gleich bis auf `resistances`. Alles, was unten verschieden
 * herauskommt, kommt aus diesem einen Feld.
 *
 * WAS HIER NICHT GEPRÜFT WIRD
 * ---------------------------
 * Die Anzeige (AC5) braucht ein Dokument und ist im Browser belegt. Die
 * Bitgleichheit mit dem Simulator (AC7) steht in
 * `sim/tests/parity_resistances.rs`. Ob die Widerstände die beste Rotation
 * ändern (AC6), misst der Balance-Läufer.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { resistanceLines } from '../../src/progression/schools.js';

function boss(id: string): BossDef {
  const gefunden = bossById(id);
  if (gefunden === undefined) throw new Error(`data/bosses/${id}.json fehlt.`);
  return gefunden;
}

const RESISTANT = boss('test-resistant');
const VULNERABLE = boss('test-vulnerable');
/**
 * Der Anker ohne Widerstand.
 *
 * **Seine übrigen Felder sind gleichgültig** — sie berühren den Schaden des
 * Spielers nicht. Was ihn zum gültigen Vergleichspunkt macht, ist allein die
 * leere Widerstands-Spalte, und genau das sichert der erste Fall unten zu.
 */
const NEUTRAL = boss('test-metronome');

const MESSDAUER_MS = 10_000;

interface Messung {
  readonly schaden: number;
  readonly vorrat: number;
  readonly wirkungen: number;
}

/**
 * Derselbe Zauber, dieselbe Druckfolge, ein anderer Boss.
 *
 * KINDLE auf Abklingzeit, sonst nichts: Die Folge hängt damit an keiner
 * Entscheidung, die sich zwischen den Bössen unterscheiden könnte. Wäre sie
 * es, verglichen die Fälle unten zwei verschiedene Kämpfe statt zweier Bösse.
 */
function messen(ziel: BossDef): Messung {
  const combat = new Combat({ school: CALCINATION.value, boss: ziel });
  const kindle = combat.indexOf('kindle');
  combat.beginFight(0);
  for (let t = 0; t <= MESSDAUER_MS; t += TICK_MS) {
    combat.stepPassive(t);
    if (combat.isReady(kindle, t)) combat.cast(kindle, t);
  }
  return {
    schaden: combat.damageDealt,
    vorrat: combat.resource,
    wirkungen: combat.castCount(kindle),
  };
}

/** Derselbe Lauf wie `messen`, aber mit einem wählbaren Zauber. */
function messenMit(ziel: BossDef, spellId: string): number {
  const combat = new Combat({ school: CALCINATION.value, boss: ziel });
  const index = combat.indexOf(spellId);
  combat.beginFight(0);
  for (let t = 0; t <= MESSDAUER_MS; t += TICK_MS) {
    combat.stepPassive(t);
    if (combat.isReady(index, t)) combat.cast(index, t);
  }
  return combat.damageDealt;
}

describe('AC1/AC2 — der Widerstand wirkt, und nur wo er steht', () => {
  it('hat überhaupt zwei Bösse, die sich in genau einem Feld unterscheiden', () => {
    // **Ohne diese Zusicherung misst alles Weitere zwei beliebige Bösse.** Der
    // Vergleich trägt nur, solange der Unterschied genau dieses Feld ist.
    expect(RESISTANT.resistances).toEqual({ CALCINATION: 0.75 });
    expect(VULNERABLE.resistances).toEqual({ CALCINATION: 1.25 });
    expect(NEUTRAL.resistances, 'der Anker darf keinen tragen').toEqual({});
    expect(RESISTANT.hp).toBe(VULNERABLE.hp);
    expect(RESISTANT.overpressureMs).toBe(VULNERABLE.overpressureMs);
    expect(RESISTANT.timeline).toEqual(VULNERABLE.timeline);
  });

  it('mindert den Schaden am widerstehenden Boss', () => {
    const neutral = messen(NEUTRAL);
    const hart = messen(RESISTANT);

    expect(neutral.wirkungen, 'ohne Wirkungen misst der Fall nichts').toBeGreaterThan(0);
    expect(hart.wirkungen, 'gleich viele Wirkungen, sonst wäre es ein anderer Kampf').toBe(
      neutral.wirkungen,
    );
    expect(hart.schaden).toBeLessThan(neutral.schaden);
    // Gemessen: 700 gegen 525.
    expect(hart.schaden / neutral.schaden).toBeCloseTo(0.75, 6);
  });

  it('ERHÖHT ihn am schwachen Boss — die andere Richtung', () => {
    // Das Design nennt beide („20–30 % weniger von der widerstandenen Schule,
    // 20–30 % **mehr** von der Schwachstelle"). Eine Umsetzung, die nur
    // mindert, erfüllte die Hälfte und bestünde jede Prüfung, die nur mindert.
    const neutral = messen(NEUTRAL);
    const weich = messen(VULNERABLE);

    expect(weich.schaden).toBeGreaterThan(neutral.schaden);
    expect(weich.schaden / neutral.schaden).toBeCloseTo(1.25, 6);
  });

  it('lässt einen Boss OHNE Eintrag unverändert — der Gegenfall', () => {
    // Ohne ihn wäre nicht gezeigt, dass der Faktor an den DATEN hängt und
    // nicht an irgendeiner anderen Eigenschaft der beiden Prüf-Bösse.
    const neutral = messen(NEUTRAL);
    // KINDLE macht 100 Grundschaden; sieben Wirkungen in zehn Sekunden.
    expect(neutral.schaden).toBe(neutral.wirkungen * 100);
  });
});

describe('AC3 — der Widerstand mindert den SCHADEN, nicht die MECHANIK', () => {
  /**
   * **Die Stelle, an der es leise falsch wird** (GAME-DESIGN 4.2).
   *
   * Der naheliegende Bau zieht den Widerstand ab, **bevor** die Schul-Mechanik
   * rechnet. Dann sammelt ein Kalzinations-Zauber gegen einen Feuer-festen Boss
   * auch weniger Hitze — und die Schule fühlt sich kaputt an statt schwächer.
   *
   * Der Grund, aus dem es hier hält: Der Vorrat kommt aus `spell.resourceGain`
   * und damit aus den DATEN des Zaubers. Er berührt die Schadensformel nie.
   * Diese Fälle halten den Zustand fest, statt sich darauf zu verlassen.
   *
   * # DIE ERSTE GEGENPROBE WAR FALSCH, und das gehört hierher
   *
   * Mein erster Versuch senkte den Vorrat in `stacks.ts` pauschal um ein
   * Viertel — an ALLEN Bössen. Der Fall blieb grün, zu Recht: Er vergleicht
   * die drei Bösse miteinander, und wenn überall dasselbe fehlt, ist der
   * Vergleich unverändert.
   *
   * Erst die zweite Fassung traf den Fehler, den dieser Fall verhindern soll:
   * `addResource` mit `this.resistanceFactor` multipliziert — also der Vorrat
   * gemindert **genau dort, wo ein Widerstand steht**. Da wurde er rot, und
   * zwar als einziger von elf.
   *
   * **Eine Gegenprobe muss die Fehlerart nachbauen, nicht irgendeinen
   * Schaden.** Sonst belegt sie nur, dass der Prüffall auf irgendetwas
   * reagiert — und das ist eine andere Aussage.
   */
  it('sammelt an allen drei Bössen gleich viel Vorrat', () => {
    const neutral = messen(NEUTRAL);
    const hart = messen(RESISTANT);
    const weich = messen(VULNERABLE);

    expect(neutral.vorrat, 'ohne Vorrat misst der Fall nichts').toBeGreaterThan(0);
    expect(hart.vorrat, 'der widerstehende Boss darf die Hitze nicht drücken').toBe(neutral.vorrat);
    expect(weich.vorrat, 'und der schwache sie nicht heben').toBe(neutral.vorrat);
  });

  it('wirkt an allen drei Bössen gleich oft — die Abklingzeit bleibt unberührt', () => {
    // Dieselbe Regel eine Ebene weiter: Ein Widerstand, der die Abklingzeit
    // streckte, machte aus „schwächer" ebenfalls „kaputt".
    const neutral = messen(NEUTRAL);
    expect(messen(RESISTANT).wirkungen).toBe(neutral.wirkungen);
    expect(messen(VULNERABLE).wirkungen).toBe(neutral.wirkungen);
  });
});

describe('Der Widerstand hängt am ZAUBER, nicht am Kampf', () => {
  /**
   * **Entschieden am 2026-08-05, nachdem die Messung es erzwungen hat.**
   *
   * Bis dahin wurde der Faktor mit der Schule des KAMPFES nachgeschlagen. Er
   * multiplizierte damit jeden Spielerschaden gleich — und etwas, das alles
   * gleich skaliert, kann keine Reihenfolge ändern. Der Rotations-Läufer fand
   * an einem Boss mit 0,75 und einem mit 1,25 dieselbe beste Liste.
   *
   * Die Auflösung steht im SSOT: „Wer **trotz Widerstand** Calcination spielt,
   * sammelt weiter normal Hitze — macht damit aber weniger Schaden." Das setzt
   * voraus, dass jemand mit einer anderen Schule nicht weniger macht.
   */
  it('mindert NUR die widerstandene Schule', () => {
    // KINDLE ist CALCINATION, CANKER ist PUTREFACTION. Der Boss hält nur
    // CALCINATION ab — ohne diesen Fall wäre nicht gezeigt, dass die anderen
    // Schulen unberührt bleiben, und genau daran hängt die ganze Abwägung.
    const kindleNeutral = messenMit(NEUTRAL, 'kindle');
    const kindleFest = messenMit(RESISTANT, 'kindle');
    const cankerNeutral = messenMit(NEUTRAL, 'canker');
    const cankerFest = messenMit(RESISTANT, 'canker');

    expect(cankerNeutral, 'ohne Schaden misst der Fall nichts').toBeGreaterThan(0);
    expect(kindleFest / kindleNeutral).toBeCloseTo(0.75, 6);
    expect(cankerFest, 'die fremde Schule bleibt unberührt').toBe(cankerNeutral);
  });

  it('LÄSST DEN SCHULUNABHÄNGIGEN ZAUBER IN RUHE', () => {
    // QUENCH gehört keiner Schule an (GAME-DESIGN 7.4) — also kann keine
    // Schule ihn abhalten. Dieselbe Trennung, die ihn schon von der globalen
    // Wartezeit ausnimmt.
    //
    // Er teilt keinen Schaden aus; geprüft wird deshalb der Faktor selbst, und
    // zwar an der Stelle, an der er entsteht: `spell.school` ist `null`, und
    // die Nachschlagestelle muss daraus 1 machen — nicht `undefined`, nicht
    // den Wert einer zufällig gleichnamigen Schule.
    const quench = CALCINATION.value.spells.find((s) => s.id === 'quench');
    expect(quench?.school, 'sonst prüft der Fall den falschen Zauber').toBeNull();

    // Gemessen über die Anzeige-Zeilen: Ein Boss, der ALLE fünf Schulen
    // abhielte, ließe QUENCH trotzdem unberührt — er steht in keiner.
    const alleSchulen = RESISTANT.resistances;
    expect(Object.keys(alleSchulen)).not.toContain('null');
  });
});

describe('AC4 — die Stärke liegt im Korridor des Designs', () => {
  it('mindert um 20 bis 30 Prozent — GEMESSEN, nicht abgeschrieben', () => {
    // **Die Zahl kommt aus zwei Kämpfen, nicht aus dem Datenfeld.** Ein Fall,
    // der `resistances.CALCINATION` liest und gegen 0,75 prüft, vergleicht die
    // Datei mit sich selbst — er wäre grün, auch wenn der Faktor nirgends
    // ankommt.
    const neutral = messen(NEUTRAL).schaden;
    const hart = messen(RESISTANT).schaden;
    const minderung = 1 - hart / neutral;

    expect(minderung).toBeGreaterThanOrEqual(0.2);
    expect(minderung).toBeLessThanOrEqual(0.3);
  });

  it('erhöht um 20 bis 30 Prozent', () => {
    const neutral = messen(NEUTRAL).schaden;
    const weich = messen(VULNERABLE).schaden;
    const zuschlag = weich / neutral - 1;

    expect(zuschlag).toBeGreaterThanOrEqual(0.2);
    expect(zuschlag).toBeLessThanOrEqual(0.3);
  });

  it('bleibt spürbar, ohne zwingend zu sein', () => {
    // GAME-DESIGN: „Wer nicht umbaut, kommt trotzdem durch, nur langsamer."
    // Ein Faktor, der den Schaden halbiert, verletzte das — und ein Faktor
    // dicht an 1,0 wäre nicht spürbar. Der Korridor hat deshalb zwei Ränder.
    const neutral = messen(NEUTRAL).schaden;
    const hart = messen(RESISTANT).schaden;
    expect(hart, 'noch mehr als die Hälfte').toBeGreaterThan(neutral * 0.5);
    expect(hart, 'aber deutlich weniger').toBeLessThan(neutral * 0.95);
  });
});

describe('AC8 — der Erste Schmelztiegel bleibt unberührt', () => {
  it('trägt keine Widerstände', () => {
    // An ihm hängt der Referenzlauf 4240 @ 40510. Ein Eintrag hier bewegte ihn,
    // und die Zahl ist seit Monaten der Anker jeder Balance-Aussage.
    expect(boss('first-crucible').resistances).toEqual({});
  });

  it('und kein anderer Boss des SPIELS auch nicht', () => {
    // Welcher ausgelieferte Boss Widerstände bekommt, entscheidet E10 —
    // dieselbe Sorte Entscheidung wie „welcher Zauber eine Wirkzeit bekommt"
    // (#304). Dieses Issue baut die Mechanik, nicht den Inhalt.
    //
    // Geprüft über die Instanz und nicht über eine Namensliste: Ein neuer
    // Prüf-Boss soll hier nichts brechen, ein neuer SPIEL-Boss mit
    // Widerständen dagegen schon.
    const spielBosse = ['first-crucible'];
    for (const id of spielBosse) {
      expect(boss(id).resistances, `${id} trägt Widerstände`).toEqual({});
    }
  });
});

describe('AC5 — die Zeilen, die der Spieler VOR dem Kampf liest', () => {
  it('nennt Richtung und Stärke in Worten, nicht als Faktor', () => {
    // `0.75` ist ein Rechenwert. Der Spieler soll wissen, was ihn erwartet,
    // nicht die Formel nachrechnen — und „less" gegen „more" trägt die
    // Richtung als WORT, nicht als Farbe (GAME-DESIGN 21.2).
    expect(resistanceLines({ CALCINATION: 0.75 })).toEqual(['CALCINATION · 25 % less damage']);
    expect(resistanceLines({ CALCINATION: 1.25 })).toEqual(['CALCINATION · 25 % more damage']);
  });

  it('schweigt, wo es nichts zu sagen gibt', () => {
    // Kein Eintrag und ein Faktor von genau 1 sind dasselbe: kein Unterschied.
    // Eine Zeile „0 % less damage" kostete den Blick, den die echten Zeilen
    // brauchen — und ein leerer Kasten stünde bei zehn von elf Bössen da.
    expect(resistanceLines({})).toEqual([]);
    expect(resistanceLines({ CALCINATION: 1 })).toEqual([]);
  });

  it('erzeugt die Zeilen der beiden Prüf-Bösse aus deren echten Daten', () => {
    // Die Brücke zwischen Datei und Anzeige. Ohne sie prüfte der Fall oben
    // eine Zahl, die ich hier hingeschrieben habe.
    expect(resistanceLines(RESISTANT.resistances)).toEqual(['CALCINATION · 25 % less damage']);
    expect(resistanceLines(VULNERABLE.resistances)).toEqual(['CALCINATION · 25 % more damage']);
    expect(resistanceLines(NEUTRAL.resistances)).toEqual([]);
  });
});
