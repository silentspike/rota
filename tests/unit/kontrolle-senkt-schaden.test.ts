/**
 * Die Fixierung greift messbar in den Kampf ein (Issue #576, AC3).
 *
 * # Warum dieser Fall neben der Aufzeichnung steht
 *
 * `kontrolle-wirkt.json` und ihr Rust-Gegenstück prüfen, dass **beide Seiten
 * dasselbe rechnen**. Sie prüfen nicht, dass die Zahlen etwas aussagen: Wären
 * alle drei Läufe bei 560, wäre die Parität genauso grün.
 *
 * Der Frische-Fall daneben prüft `toEqual(payload())` — auch er bliebe grün,
 * wenn die Wirkung verschwände, denn er vergleicht die Datei mit dem Spiel und
 * beide bewegten sich gemeinsam.
 *
 * Hier steht die Aussage selbst: **STALL senkt den erlittenen Schaden, STAY
 * nicht.**
 *
 * # Der Befund, aus dem der Fall entstand
 *
 * Zehn von zehn Fixations-Zaubern wurden in den Aufzeichnungen gewirkt und
 * bewegten **keine** der fünf Messgrößen. Das sah nach einer Mechanik aus, die
 * nicht ankommt — sie kommt an, es fehlte die Lage.
 *
 * `delayStrike` (STAY, LATCH) bewegt `damageTaken` bauartbedingt nie: Es
 * verschiebt den nächsten Zeitleisten-Eintrag und setzt danach zurück, die
 * folgenden holen auf. Der Treffer kommt später, aber er kommt. Was wirkt, ist
 * `slowCast` plus Unterbrechen — der Zauber trifft dann gar nicht.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/**
 * Die beiden Lagen — je Boss der QUENCH-Zeitpunkt und die erwarteten Werte.
 *
 * **Die Zeitpunkte liegen so spät, dass die HALBE STALL-Wirkung nicht reicht.**
 * Der erste Entwurf legte sie auf den Einschlag ohne STALL (32500 bei
 * boss-1-4); der Fall war grün und hielt zu wenig fest: Eine Mutation, die
 * `amountMs` von 2000 auf 1000 halbiert, brach ihn nicht — der verlängerte
 * Cast landete bei 33500, und ein Druck bei 32500 war immer noch rechtzeitig.
 * Geprüft war damit nur, DASS die Verlängerung wirkt, nicht wie weit.
 *
 * ```text
 *   boss-1-4     Cast 30000+2500   ohne 32500 · halb 33500 · voll 34500
 *   test-vessel  Cast 20000+3000   ohne 23000 · halb 24000 · voll 25000
 * ```
 *
 * Zwei Bosse, weil einer den Nachweis allein trägt: `boss-1-4` endet auf 0
 * (ein einziger Zeitleisten-Eintrag), `test-vessel` liefert dieselbe Aussage
 * mit 1600 gegen 640 — keine Randwert-Zahl.
 *
 * # Unter den Zahlen von `test-vessel` liegt ein SOCKEL
 *
 * Seit Issue #568 löst dessen Energieleiste einen Boss-Schlag außerhalb der
 * Zeitleiste aus — im Fenster dieses Falls zwei Auslöser zu zusammen **320
 * Schaden**, die jeden Lauf gleich treffen. Die Werte sind deshalb von
 * 1280/320 auf 1600/640 gewandert; die **gemessene Wirkung ist unverändert:
 * 960, vor wie nach**.
 *
 * Wer sie wieder wandern sieht, prüft ZUERST die Differenz. Bewegt sie sich
 * nicht, hat jemand einen Sockel eingezogen und nicht die Kontrolle
 * verändert. `boss-1-4` hat keine Energieleiste und ist davon unberührt —
 * genau dafür stehen hier zwei Bosse.
 */
/**
 * **`wirkung` ist die Zusicherung, die Absolutwerte sind Kulisse** (#678).
 *
 * Gemessen wird `ohne − mitStall`. Legt eine unbeteiligte Mechanik am selben
 * Boss künftig einen Sockel unter beide Werte, wandern sie gemeinsam und die
 * Differenz bleibt — der Fall meldet sich dann nicht mehr grundlos.
 *
 * Genau das ist bei #568 passiert: Die Energieleiste von `test-vessel` legte
 * 320 unter beide Zahlen, fünf Fälle fielen, und gemessen hatte sich nichts.
 *
 * **Die Wirkung steht je Lage, nicht als eine Zahl über allem.** `boss-1-4`
 * trägt 560, `test-vessel` 960 — eine gemeinsame Zahl wäre schlicht falsch.
 * Der Unterschied hat einen Grund: `boss-1-4` hat einen einzigen
 * Zeitleisten-Eintrag und endet deshalb auf 0, `test-vessel` trägt mehrere.
 * Beide zusammen zeigen, dass der Nachweis nicht an einem Randwert hängt.
 */
const LAGEN = [
  { bossId: 'boss-1-4', quenchAtMs: 34_000, ohne: 560, mitStall: 0, wirkung: 560 },
  { bossId: 'test-vessel', quenchAtMs: 24_500, ohne: 1600, mitStall: 640, wirkung: 960 },
] as const;
const TAKTE = 4000;

type Ergebnis = {
  damageTaken: number;
  unterbrochen: number;
  verpasst: number;
  bindWuerfe: number;
  quenchWuerfe: number;
};

/**
 * Ein Durchgang mit oder ohne Kontroll-Zauber.
 *
 * **QUENCH wird in jedem Lauf zur selben Kampfzeit gedrückt.** Der erste
 * Entwurf drückte ihn relativ zum laufenden Cast — dann wurde er ohne STALL
 * gar nicht geworfen, weil das Fenster schon zu war, und die Läufe
 * unterschieden sich in zwei Dingen statt in einem.
 */
function lauf(bossId: string, kontrollZauber: string | null, quenchAtMs: number): Ergebnis {
  const boss = bossById(bossId);
  if (boss === undefined) throw new Error(`Der Boss ${bossId} fehlt.`);
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const quench = alsSpellIndex(combat.indexOf('quench'));
  const kontrolle =
    kontrollZauber === null ? alsSpellIndex(-1) : alsSpellIndex(combat.indexOf(kontrollZauber));

  let castBegann = -1;
  let kontrolleGeworfen = false;
  let quenchGeworfen = false;

  for (let tick = 0; tick < TAKTE; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.playerDead) break;
    if (combat.casting !== -1 && castBegann < 0) castBegann = timeMs;
    if (combat.casting === -1 && castBegann >= 0) castBegann = -1;

    if (
      kontrolle >= 0 &&
      castBegann >= 0 &&
      !kontrolleGeworfen &&
      combat.control >= 4 &&
      combat.isReady(kontrolle, timeMs) &&
      combat.cast(kontrolle, timeMs).result === CAST_OK
    ) {
      kontrolleGeworfen = true;
    } else if (
      !quenchGeworfen &&
      timeMs >= quenchAtMs &&
      combat.isReady(quench, timeMs) &&
      combat.cast(quench, timeMs).result === CAST_OK
    ) {
      quenchGeworfen = true;
    } else if (combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    }
  }

  return {
    damageTaken: combat.damageTaken,
    unterbrochen: combat.interruptsLanded,
    verpasst: combat.interruptsMissed,
    bindWuerfe: combat.castCount(bind),
    quenchWuerfe: combat.castCount(quench),
  };
}

describe('Die Kontrolle senkt den erlittenen Schaden', () => {
  for (const lage of LAGEN) {
    // `ohne` und `mitStall` werden bewusst NICHT ausgelesen: Sie stehen in
    // der Tabelle als Kulisse — als Anhalt, welche Größenordnung zu erwarten
    // ist — und sind seit #678 keine Zusicherung mehr. Zugesichert ist
    // `wirkung`, die Differenz.
    const { bossId, quenchAtMs, wirkung } = lage;

    describe(bossId, () => {
      it('alle drei Läufe drücken QUENCH genau einmal', () => {
        // **Die Zusicherung vor der Messung.** Würde QUENCH nur im STALL-Lauf
        // geworfen, wäre die Differenz echt und der Nachweis trotzdem
        // angreifbar: Die Läufe unterschieden sich in zwei Dingen.
        for (const k of [null, 'stay', 'stall']) {
          expect(lauf(bossId, k, quenchAtMs).quenchWuerfe, `Lauf ${k ?? 'ohne'}`).toBe(1);
        }
      });

      it('ohne Kontrolle kommt der Zauber an', () => {
        const r = lauf(bossId, null, quenchAtMs);
        // Der Schadenswert ist hier KULISSE und keine Zusicherung — er darf
        // wandern, sobald eine unbeteiligte Mechanik am Boss Schaden macht.
        // Zugesichert sind die beiden Zähler: Der Zauber wurde nicht
        // unterbrochen und ist angekommen.
        expect(r.unterbrochen).toBe(0);
        expect(r.verpasst).toBe(1);
      });

      it('mit STALL trifft er nicht', () => {
        const r = lauf(bossId, 'stall', quenchAtMs);
        expect(r.unterbrochen).toBe(1);
        expect(r.verpasst).toBe(0);
      });

      it(`STALL verhindert ${wirkung} Schaden — die Differenz, nicht die Absolutwerte`, () => {
        // **Der Kern von #678.** Gemessen wird `ohne − mit`, weil nur diese
        // Zahl die Wirkung von STALL ist. Beide Absolutwerte wandern
        // gemeinsam, sobald jemand dem Boss eine weitere Mechanik gibt; die
        // Differenz überlebt das.
        const ohneKontrolle = lauf(bossId, null, quenchAtMs).damageTaken;
        const mitKontrolle = lauf(bossId, 'stall', quenchAtMs).damageTaken;

        expect(ohneKontrolle - mitKontrolle, 'die Wirkung von STALL hat sich verschoben').toBe(
          wirkung,
        );

        // **Zusicherung gegen den stillen Nullfall:** Verhinderte STALL gar
        // nichts, wäre die Differenz 0 — und eine erwartete 0 sähe genauso
        // aus wie eine gemessene. Deshalb steht hier, dass überhaupt etwas
        // verhindert wird.
        expect(wirkung, 'eine Wirkung von 0 wäre keine Wirkung').toBeGreaterThan(0);
      });

      it('STAY trägt dieselbe Last und ändert trotzdem nichts', () => {
        // **Die Kontrolle.** STALL verdrängt einen BIND-Wurf; ein Vergleich
        // gegen „gar kein Kontroll-Zauber" unterschiede sich in der Wirkung
        // UND einem GCD. STAY kostet Punkte, löst denselben GCD aus, wirkt
        // aber auf die Zeitleiste statt auf den laufenden Cast.
        const ohneK = lauf(bossId, null, quenchAtMs);
        const kontrolle = lauf(bossId, 'stay', quenchAtMs);
        const wirkung = lauf(bossId, 'stall', quenchAtMs);

        expect(kontrolle.damageTaken, 'STAY senkt den Schaden — dann ist er keine Kontrolle').toBe(
          ohneK.damageTaken,
        );
        expect(kontrolle.unterbrochen).toBe(0);

        // Gleiche Last, gemessen statt behauptet.
        expect(kontrolle.bindWuerfe, 'STAY und STALL verdrängen verschieden viele BINDs').toBe(
          wirkung.bindWuerfe,
        );
        // Und beide verdrängen wirklich etwas — sonst wäre die Zeile darüber leer.
        expect(ohneK.bindWuerfe).toBeGreaterThan(kontrolle.bindWuerfe);
      });
    });
  }

  it('boss-1-4s Null ist die Struktur des Bosses, kein Randwert', () => {
    // `Sealing Chant` ist der EINZIGE Zeitleisten-Eintrag. Es gibt kein „ein
    // Teil kommt durch" — der Zauber trifft ganz oder gar nicht. Käme ein
    // zweiter Eintrag dazu, wäre die 0 ein Zufall statt einer Aussage.
    const boss = bossById('boss-1-4');
    if (boss === undefined) throw new Error('Der Boss boss-1-4 fehlt.');
    expect(boss.timeline).toHaveLength(1);
    expect(boss.timeline[0]?.mechanic).toBe('interruptibleCast');
    expect(boss.timeline[0]?.castTimeMs).toBe(2500);
  });

  it('und der zweite Fall endet NICHT auf einer Null', () => {
    // Der Grund, warum test-vessel danebensteht: Ein Nachweis, der auf einer
    // Null endet, ist schwerer zu widerlegen, als er sein sollte.
    const r = lauf('test-vessel', 'stall', 24_500);
    expect(r.damageTaken).toBeGreaterThan(0);
    expect(r.damageTaken).toBeLessThan(1600);
  });
});
