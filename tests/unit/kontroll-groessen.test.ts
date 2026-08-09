import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { replayRecording } from '../../src/devtools/recording.js';
import { bossById } from '../../src/core/data.js';

/**
 * Die GRÖSSEN der Kontroll-Zauber — Fortsetzung von #592.
 *
 * ```text
 *   control.amountMs  stall    300,00 %   damageTaken  320 → 1280
 * ```
 *
 * # Unter diesen Zahlen liegt ein SOCKEL, der mit STALL nichts zu tun hat
 *
 * Seit Issue #568 löst die Energieleiste von `test-vessel` einen Boss-Schlag
 * außerhalb der Zeitleiste aus. Im Fenster dieser Fälle sind das zwei
 * Auslöser, zusammen **320 Schaden** — sie treffen jeden Lauf gleich, ob mit
 * STALL, mit STAY oder ohne Kontrolle.
 *
 * Deshalb sind die Absolutwerte hier um 320 gewandert:
 *
 * ```text
 *                    vorher   jetzt   Differenz
 *   ohne Kontrolle    1280     1600      +320
 *   mit STALL          320      640      +320
 *   ------------------------------------------
 *   Wirkung von STALL  960      960         0
 * ```
 *
 * **Die 960 sind der Zweck dieser Datei; die Absolutwerte sind Kulisse.**
 *
 * Wer sie das nächste Mal wandern sieht, prüft deshalb ZUERST die Differenz.
 * Bewegt sie sich nicht, hat jemand einen Sockel eingezogen und nicht die
 * Kontrolle verändert — dann sind die Zahlen nachzuziehen und dieser Absatz
 * zu ergänzen, nicht STALL zu verdächtigen.
 *
 * # ANCHOR steht hier NICHT, und das ist der lehrreichere Teil
 *
 * `control.cost/anchor` stand mit 6,67 % in der Rangfolge. Ein erster Entwurf
 * baute einen Prüffall darauf — und der prüfte die Größe **gar nicht**:
 *
 * ```text
 *   anchor.cost = 0 … 999   der Fall bleibt grün, über den ganzen Bereich
 *   fixation#0   bind 20                            damage 1200
 *   fixation#1   bind 15, stall 2, stay 1, anchor 2  damage  900
 *   1200 / 20 = 60 je bind      15 × 60 = 900
 * ```
 *
 * Der Unterschied ist nicht die Ressource, sondern dass fünf Drücke keinen
 * Schadenszauber tragen. Die 6,67 % stammten aus einem Export-Lauf, der bei
 * halbierten Kosten eine andere Rotation wählte.
 *
 * **Ich hatte dieses Artefakt eine Stunde vorher selbst gemeldet — und im
 * selben PR einen Prüffall daraus gebaut, als wäre die Zahl echt.** Ein Befund
 * über ein Werkzeug wirkt nicht rückwirkend auf die Zahlen, die es schon
 * geliefert hat; er muss auf jede einzelne angewandt werden.
 *
 * # Warum diese zwei erst jetzt messbar sind
 *
 * Bis zum 2026-08-07 standen **alle elf** Kontroll-Größen auf 0,00 % — nicht
 * weil sie belanglos wären, sondern weil keine Aufzeichnung eine Lage
 * herstellte, in der ihr Betrag den Ausgang ändert. Das war Sorte „keine
 * Lage" aus #547.
 *
 * `kontrolle-wirkt.json` (Worker-2, #593) baut sie: STALL vergrößert das
 * Fenster, QUENCH unterbricht. Erst damit wurde aus einer Null ein Hebel —
 * und zwar dem größten im ganzen Bestand.
 *
 * # Die 300 % sind kein Ausreißer, sondern eine andere Frage
 *
 * Bei den Brand-Größen ist die Basis der **Gesamtschaden**: 72,58 % bei
 * SEARING heißen „ein Viertel mehr". Bei STALL ist die Basis der **verhinderte
 * Teil**: 640 von 1600. Halbiert man seine Verzögerung, steigt der erlittene
 * Schaden auf 1600 — also **exakt auf den Wert ohne STALL**.
 *
 * Das ist keine Verschiebung um einen Betrag, sondern der vollständige
 * Wirkungsverlust. Deshalb steht in `groessen-hebel-messung.txt` seit heute
 * die Basis neben dem Prozentsatz: Ohne sie liest sich „150 vor 72" wie
 * „doppelt so wichtig", und das stimmt nicht.
 *
 * **Dieser Absatz hat seitdem sein eigenes Beispiel bekommen.** Als er
 * geschrieben wurde, stand dort 300 % — der Energie-Auslöser aus #568 legte
 * einen Sockel unter beide Zahlen, der Nenner wuchs mit, und aus 300 wurden
 * 150. Die absolute Wirkung von STALL hat sich dabei nicht bewegt.
 *
 * Genau davor warnt der Absatz: **Eine Verhältniszahl ohne ihre Basis sagt
 * nichts.** Die 300 war nicht falsch, sie galt für einen anderen Stand.
 *
 * # Die Aufzeichnung liefert die Gegenprobe gleich mit
 *
 * Derselbe Fall enthält drei Läufe — `none`, `stay`, `stall`. Die ersten
 * beiden kommen auf 1600, der dritte auf 640. Ein Prüffall, der nur `stall`
 * misst, könnte einen Boss erwischen, der ohnehin wenig austeilt; mit `none`
 * daneben ist das ausgeschlossen.
 */

const DATEI = 'tests/parity/recordings/kontrolle-wirkt.json';

interface Lauf {
  readonly way: string;
  readonly ticks: number;
  readonly presses: { readonly tick: number; readonly slot: number }[];
  readonly bindings: { readonly slot: number; readonly spellId: string }[];
  readonly expected: Record<string, number>;
}

function faelle(): { bossId: string; runs: Lauf[] }[] {
  return JSON.parse(readFileSync(DATEI, 'utf8')).cases;
}

function erlittenerSchaden(bossId: string, lauf: Lauf): number {
  const boss = bossById(bossId);
  if (!boss) throw new Error(`Boss ${bossId} fehlt — ohne ihn fällt kein Schaden`);
  return replayRecording(
    {
      name: 'kontroll-groesse',
      seed: 0,
      ticks: lauf.ticks,
      presses: lauf.presses,
      bindings: lauf.bindings,
    },
    { boss },
  ).damageTaken;
}

describe('STALL senkt den erlittenen Schaden von 1600 auf 640', () => {
  const fall = faelle()[1]!;
  const weg = (name: string): Lauf => {
    const l = fall.runs.find((r) => r.way === name);
    if (!l) throw new Error(`Lauf "${name}" fehlt in ${DATEI}`);
    return l;
  };

  /**
   * **Die Zusicherung vor der Messung.** Teilt der Boss ohnehin wenig aus,
   * misst der Fall darunter nichts und wäre grün mit der Aussage „STALL
   * wirkt" — auch wenn er gar nichts täte.
   */
  /**
   * **Seit #678 misst dieser Fall die DIFFERENZ, nicht zwei Absolutwerte.**
   *
   * Der Grund steht im Kopf: Ein Sockel aus einer unbeteiligten Mechanik
   * verschiebt beide Zahlen gemeinsam und ließ hier fünf Fälle fallen,
   * obwohl sich das Gemessene nicht bewegt hatte. Die Differenz überlebt
   * jeden künftigen Sockel.
   *
   * Die 1600 und 640 stehen weiter in den Meldungen — als **Kulisse**, damit
   * die Größenordnung lesbar bleibt. Zugesichert ist allein die 960.
   */
  it('STALL verhindert 960 Schaden — gegen `none` gemessen', () => {
    const ohne = erlittenerSchaden(fall.bossId, weg('none'));
    const mit = erlittenerSchaden(fall.bossId, weg('stall'));

    expect(
      ohne - mit,
      `die Wirkung von STALL hat sich verschoben (heute ${ohne} gegen ${mit}, ` +
        'zur Zeit von #678 waren es 1600 gegen 640)',
    ).toBe(960);
  });

  /**
   * STAY ist der Gegenbeleg: derselbe Kampf, ein anderer Kontroll-Zauber,
   * **keine** Wirkung auf den erlittenen Schaden. Ohne ihn bliebe offen, ob
   * die 640 von STALL kommen oder davon, dass überhaupt jemand etwas wirkt.
   */
  it('mit STAY verhindert er NICHTS — es liegt an STALL', () => {
    const ohne = erlittenerSchaden(fall.bossId, weg('none'));
    const mitStay = erlittenerSchaden(fall.bossId, weg('stay'));

    expect(
      ohne - mitStay,
      `STAY senkt den Schaden ebenfalls (${ohne} gegen ${mitStay}) — dann sagt ` +
        'der Fall oben nichts über STALL',
    ).toBe(0);
  });
});
