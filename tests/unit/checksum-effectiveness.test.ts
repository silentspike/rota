/**
 * Wirkt jedes Feld der Prüfsumme wirklich? (Issue #240)
 *
 * DER BLINDE FLECK UNSERES SCHÄRFSTEN WERKZEUGS
 * ---------------------------------------------
 * Die Parität vergleicht **zwei Rechnungen**, nicht eine Rechnung mit der
 * Wirklichkeit. Fällt ein Feld auf **beiden** Seiten weg, bleibt sie grün —
 * sie kann nicht merken, wenn beide dasselbe vergessen.
 *
 * Genau das drohte beim Nachziehen von #218 auf #97: Zwei Erweiterungen hingen
 * ihre Felder an dieselbe Stelle. Wer den Konflikt mit „eine Seite nehmen"
 * auflöst, verliert die andere Gruppe — und **kein Prüffall wird rot**. Die
 * Prüfsumme rechnet weiter, nur ohne die Felder.
 *
 * NICHT ZU VERWECHSELN MIT DEM REIHENFOLGE-WÄCHTER (#236)
 * -------------------------------------------------------
 * Die zwei beantworten verschiedene Fragen und ersetzen einander nicht:
 *
 * | | |
 * | --- | --- |
 * | #236 | Laufen die beiden **Seiten** auseinander? |
 * | dieser | Rechnet überhaupt **jemand** mit dem Feld? |
 *
 * Wer sie für Dubletten hält und eine entfernt, nimmt die Hälfte der Abdeckung
 * mit.
 *
 * WIE GEMESSEN WIRD — UND WARUM NICHT ÜBER ZWEI KAMPFLÄUFE
 * --------------------------------------------------------
 * Mein erster Entwurf ließ je Feld zwei Kämpfe laufen, einen mit und einen ohne
 * die auslösende Handlung, und verlangte verschiedene Prüfsummen. Das sah nach
 * einer Messung aus und war keine: In einem Kampf ändert sich **alles**. Der
 * Fall „spürt `doom`" wäre auch dann grün geblieben, wenn `doom` gar nicht
 * eingerührt wird — der Schaden allein hätte die Summe verschoben. Er hätte die
 * Frage nie gestellt, die er im Namen trägt.
 *
 * Gemessen wird deshalb an **einem** Zustand, mit einer gezielten Störung: Das
 * Feld wird über einen Stellvertreter um eins verschoben, sonst nichts. Ändert
 * sich die Prüfsumme nicht, dann geht das Feld nicht in sie ein — anders kann
 * es nicht sein.
 *
 * KEINE AUFZÄHLUNG DER FELDER
 * ---------------------------
 * Die Liste wird aus `checksum.ts` gelesen. Ein neues Feld ist damit **sofort**
 * geprüft, ohne dass jemand hier etwas nachträgt.
 *
 * Das hat eine Kehrseite, die benannt gehört: Wer ein Feld **herausnimmt**,
 * nimmt es auch aus der Liste — der Fall verschwindet, statt rot zu werden. Ein
 * Wächter, der beim Löschen schweigt, wäre nur anders blind. Deshalb steht
 * unten eine **Untergrenze**: Die Zahl darf steigen, aber nicht fallen. Wer sie
 * senkt, tut das ausdrücklich und begründet es.
 *
 * WAS ER NICHT KANN
 * -----------------
 * Er prüft, was **drinsteht** — nicht, was fehlen könnte. Eine Größe, die nie
 * eingerührt wurde, kennt er nicht. Zum Zeitpunkt seiner Entstehung war das
 * keine Theorie: Die drei Bausteine aus #240 (`reaction`, `afflicted` und ihre
 * Zähler) standen noch in keiner Zeile von `checksum.ts`, und dieser Wächter
 * hätte davon nie berichtet. Sie kommen im nächsten Schritt dazu — und sind
 * dann automatisch mitgeprüft.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { combatChecksum } from '../../src/core/checksum.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const QUELLE = import.meta.glob('/src/core/checksum.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function checksumQuelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length < 500) {
    throw new Error('checksum.ts nicht geladen — die Pruefung wuerde nichts messen');
  }
  return text;
}

/** Alles, was `combatChecksum` am Kampf abgreift — aus dem Quelltext gelesen. */
function abgegriffeneFelder(quelle: string): string[] {
  const rumpf = quelle.slice(quelle.indexOf('export function combatChecksum'));
  return [...new Set([...rumpf.matchAll(/\bcombat\.([A-Za-z_]\w*)/g)].map((t) => t[1] as string))];
}

const MESSZEIT_MS = 12000;

/**
 * Ein Kampf mit Zustand — damit gestört wird, was auch wirklich Werte trägt.
 *
 * **`cast` liefert einen Rückgabewert, und der wird gelesen.** Mein erster
 * Entwurf reihte die Zauber mit `else if` und fragte nur `isReady` — damit kam
 * KINDLE nie an die Reihe, weil SMOULDER bei 1500 ms Abklingzeit nach jedem GCD
 * wieder bereit ist. Ergebnis: 0 Schaden, 0 Wirkungen, ein Zustand, an dem sich
 * nichts messen ließ. Die Zusicherungen unten stehen genau deshalb da.
 *
 * SMOULDER und CRUST nur einmal, damit auch der Brand-Tick und die Schutzschicht
 * Werte tragen; den Rest füllt KINDLE.
 */
function gelaufen(): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const kindle = combat.indexOf('kindle');
  const smoulder = combat.indexOf('smoulder');
  const crust = combat.indexOf('crust');
  combat.beginFight(0);
  for (let t = 0; t <= MESSZEIT_MS; t += TICK_MS) {
    combat.stepPassive(t);
    if (!combat.gcd.isReady(t)) continue;
    if (combat.castCount(smoulder) === 0 && combat.isReady(smoulder, t)) {
      if (combat.cast(smoulder, t)) continue;
    }
    if (combat.castCount(crust) === 0 && combat.isReady(crust, t)) {
      if (combat.cast(crust, t)) continue;
    }
    if (combat.isReady(kindle, t)) combat.cast(kindle, t);
  }
  return combat;
}

/** Der um eins verschobene Wert — je nachdem, was das Feld überhaupt ist. */
function verschoben(wert: unknown): unknown {
  if (typeof wert === 'number') return wert + 1;
  if (typeof wert === 'boolean') return !wert;
  if (Array.isArray(wert)) return wert.map((e) => (typeof e === 'number' ? e + 1 : e));
  return wert;
}

/** Ob ein Wert sich überhaupt verschieben lässt — Zahlen, Wahrheitswerte, Zahlenreihen. */
function stoerbar(wert: unknown): boolean {
  if (typeof wert === 'number' || typeof wert === 'boolean') return true;
  return Array.isArray(wert) && wert.every((e) => typeof e === 'number');
}

/**
 * Ein Stellvertreter, bei dem genau ein Feld verschoben ist.
 *
 * Drei Fälle, nach dem **Laufzeit-Typ** unterschieden und nicht nach dem Namen —
 * sonst wäre es doch wieder eine Aufzählung:
 *
 * - Wert (`doom`, `shield`): direkt verschoben.
 * - Methode (`castCount`, `decayExpiryAt`): der Rückgabewert verschoben.
 * - Behälter (`gcd`, `random`): rekursiv, damit `gcd.readyAt` und
 *   `random.drawCounts()` mitkommen, ohne hier zu stehen.
 */
function mitStoerung(combat: Combat, feld: string): Combat {
  return new Proxy(combat, {
    get(ziel, name, empfaenger) {
      const wert = Reflect.get(ziel, name, empfaenger) as unknown;
      if (name !== feld) {
        return typeof wert === 'function' ? (wert as () => unknown).bind(ziel) : wert;
      }
      if (typeof wert === 'function') {
        const methode = wert as (...a: unknown[]) => unknown;
        return (...args: unknown[]) => verschoben(methode.apply(ziel, args));
      }
      if (stoerbar(wert)) return verschoben(wert);
      // Behälter: jedes störbare Glied darin verschieben.
      if (typeof wert === 'object' && wert !== null) {
        return new Proxy(wert as object, {
          get(innen, innenName, innenEmpf) {
            const drin = Reflect.get(innen, innenName, innenEmpf) as unknown;
            if (typeof drin === 'function') {
              const methode = drin as (...a: unknown[]) => unknown;
              return (...args: unknown[]) => verschoben(methode.apply(innen, args));
            }
            return stoerbar(drin) ? verschoben(drin) : drin;
          },
        });
      }
      return wert;
    },
  }) as Combat;
}

/**
 * DIE Prüfung — echter Fall und Gegenprobe benutzen sie beide.
 *
 * Eine nachgebaute zweite Fassung für die Gegenprobe wäre wertlos: Sie könnte
 * anschlagen, während die echte schweigt. (Die Lehre aus `crit-factors-source`.)
 */
export function ungespuerteFelder(
  combat: Combat,
  felder: readonly string[],
  summe: (c: Combat, t: number) => number,
): string[] {
  const echt = summe(combat, MESSZEIT_MS);
  return felder.filter((feld) => summe(mitStoerung(combat, feld), MESSZEIT_MS) === echt);
}

describe('Jedes Feld der Prüfsumme wird auch gespürt', () => {
  const felder = abgegriffeneFelder(checksumQuelle());
  const combat = gelaufen();

  it('liest ueberhaupt Felder aus dem Quelltext', () => {
    // Ohne diese Zusicherung liefe alles Weitere über eine leere Liste — und
    // eine leere Liste hat immer keine ungespürten Felder.
    expect(felder).toContain('damageDealt');
    expect(felder).toContain('bossHealed');
  });

  it('verliert kein Feld, ohne dass es jemand merkt', () => {
    // **Die Untergrenze.** Gemessen am 2026-08-05: 31 Größen. Sie darf steigen —
    // jedes neue Feld ist eine gute Nachricht. Fällt sie, hat jemand eine Zeile
    // aus `checksum.ts` verloren, und die aus dem Quelltext gelesene Liste
    // hätte das allein nicht gemeldet: Das Feld wäre einfach nicht mehr dabei.
    expect(felder.length, felder.join(', ')).toBeGreaterThanOrEqual(31);
  });

  it('hat einen Zustand, an dem sich ueberhaupt etwas messen laesst', () => {
    // **Der Fall, der den ersten Messlauf entlarvt hat.** Ohne ihn waeren alle
    // Feld-Faelle unten gruen geblieben — an einem Kampf, in dem nichts
    // passiert ist. Gruen aus dem falschen Grund ist schlimmer als rot.
    expect(combat.damageDealt).toBeGreaterThan(0);
    expect(combat.castCount(combat.indexOf('kindle'))).toBeGreaterThan(0);
    expect(combat.dotTickCount(combat.indexOf('smoulder'))).toBeGreaterThan(0);
    expect(combat.shield).toBeGreaterThan(0);
    expect(combat.resource).toBeGreaterThan(0);
  });

  // **Jedes Feld einzeln.** „Die Prüfsumme ändert sich" wäre schon erfüllt,
  // wenn EIN Feld wirkt — und genau das Feld, das beim nächsten Konflikt
  // verschwindet, wäre dann nicht das geprüfte.
  for (const feld of felder) {
    it(`spuert combat.${feld}`, () => {
      const ungespuert = ungespuerteFelder(combat, [feld], combatChecksum);
      expect(
        ungespuert,
        `combat.${feld} wird in checksum.ts gelesen, aendert die Pruefsumme aber ` +
          `nicht. Wahrscheinlich fehlt das mixInt/mixFloat dazu — dann ist die ` +
          `Pruefsumme fuer dieses Feld blind, und ein Unterschied darin faellt ` +
          `weder hier noch in der Paritaet auf.`,
      ).toEqual([]);
    });
  }
});

describe('Die Gegenprobe: ein Feld aus der Prüfsumme genommen', () => {
  /**
   * Dieselbe Prüfsumme, aber für `bossHealed` blind.
   *
   * Nicht `combatChecksum` umgeschrieben — die Funktion bleibt unangetastet.
   * Der Stellvertreter hält das Feld auf einem festen Wert, und damit verhält
   * sie sich **genau so**, wie sie sich verhielte, wenn jemand die Zeile
   * `h = mixInt(h, combat.bossHealed)` bei einem Zusammenführen verlöre.
   */
  function ohneBossHealed(combat: Combat, timeMs: number): number {
    const blind = new Proxy(combat, {
      get(ziel, name, empfaenger) {
        if (name === 'bossHealed') return 0;
        const wert = Reflect.get(ziel, name, empfaenger) as unknown;
        return typeof wert === 'function' ? (wert as () => unknown).bind(ziel) : wert;
      },
    }) as Combat;
    return combatChecksum(blind, timeMs);
  }

  const combat = gelaufen();
  const felder = abgegriffeneFelder(checksumQuelle());

  it('schlaegt an — und NENNT das Feld', () => {
    const ungespuert = ungespuerteFelder(combat, felder, ohneBossHealed);
    expect(ungespuert).toContain('bossHealed');
    // Und **nur** dieses: Ein Wächter, der bei einer entfernten Zeile die halbe
    // Liste meldet, schickt den Suchenden in die falsche Datei.
    expect(ungespuert).toEqual(['bossHealed']);
  });

  it('schweigt bei derselben Pruefsumme unveraendert', () => {
    // Die Gegenrichtung. Ohne sie belegte der Fall oben nur, dass der Wächter
    // überhaupt etwas meldet — nicht, dass er das Richtige meldet.
    expect(ungespuerteFelder(combat, felder, combatChecksum)).toEqual([]);
  });
});
