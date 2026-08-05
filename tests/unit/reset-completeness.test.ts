/**
 * Der zweite Lauf: Was überlebt `Combat.reset()`? (Issue #349)
 *
 * DAS PROBLEM IST NICHT EIN FELD, SONDERN DIE ABWESENHEIT EINER PRÜFUNG
 * ---------------------------------------------------------------------
 * ```
 * Zustandsfelder in Combat            70
 * Zuweisungen in reset()              16
 * Prüffälle, die ZWEI Läufe fahren     0
 * ```
 *
 * Die dritte Zahl ist der Punkt. Jeder bestehende Prüffall baut sich einen
 * frischen `Combat`; dort ist der Zustand ohnehin leer. Ein Feld, das beim
 * Zurücksetzen vergessen wird, verhält sich im ersten Lauf richtig und in
 * jedem folgenden falsch — und niemand sieht es.
 *
 * **Es ist nicht latent.** Das Spiel baut den Kampf genau einmal
 * (`app.ts:158`), und „Retry" führt über `session.reset()` auf
 * `combat.reset()`. Gemessen vor der Reparatur:
 *
 * ```
 * LAUF 1 Ende:  HP 4552  Zeitleiste 2  Zersetzung 5  erlitten 448
 * nach reset(): HP 4552  Zeitleiste 2  Zersetzung 5  erlitten 448
 * frisch:       HP 5000  Zeitleiste 0  Zersetzung 0  erlitten 0
 * ```
 *
 * Wer den zweiten Versuch beginnt, tut das mit angeschlagenem Leben und einer
 * Zeitleiste, die schon bei Eintrag 2 steht — die ersten beiden Boss-
 * Handlungen fallen ersatzlos aus.
 *
 * WARUM DER VERGLEICH GEGEN EINEN FRISCHEN KAMPF UND NICHT GEGEN EINE LISTE
 * -------------------------------------------------------------------------
 * Eine Liste erwarteter Felder müsste jemand pflegen — und wer ein Feld
 * hinzufügt und die Liste vergisst, hat genau den Fehler gebaut, den die Liste
 * verhindern sollte. Der frische Kampf IST die Erwartung: Was in ihm steht,
 * ist der Zustand vor dem ersten Lauf.
 *
 * `private x = 0` ist in JavaScript eine gewöhnliche Eigenschaft. `Object.keys`
 * sieht deshalb **alle** Felder, auch die, die es morgen erst geben wird. Das
 * ist AC5: Der Wächter wächst mit, ohne dass jemand ihn anfasst.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/**
 * Gegen MEHRERE Bösse, und das ist keine Gründlichkeit, sondern Notwendigkeit.
 *
 * **Was der Lauf nicht berührt, kann der Wächter nicht finden.** Ein Boss ohne
 * Zähler lässt `doomValue` auf null, ein Boss ohne Wettrennen `raceDone` auf
 * false — beide Felder sähen nach einem Reset „richtig" aus, weil sie nie
 * falsch waren.
 *
 * Die sechs decken zusammen die Bausteine ab, die überhaupt Zustand
 * hinterlassen: Zeitleiste und Eskalation, Zähler und Stapel, Wettrennen und
 * Boss-Heilung, die davonlaufende Reaktion, die Schwächung, eingehenden
 * Schaden.
 */
const BOESSE = [
  'test-crucible-trial',
  'test-doomsayer',
  'test-racer',
  'test-runaway',
  'test-stacked',
  'test-warden',
] as const;

function bossOder(id: string): BossDef {
  const gefunden = bossById(id);
  if (gefunden === undefined) throw new Error(`data/bosses/${id}.json fehlt.`);
  return gefunden;
}

const BOSS = bossOder('test-crucible-trial');

/**
 * Felder, die einen Lauf überdauern DÜRFEN — mit Begründung, einzeln.
 *
 * **Heute ist die Liste leer, und das ist eine Aussage.** Wer sie füllt, sagt
 * damit: Dieses Feld gehört zur Vorgeschichte über alle Durchläufe hinweg,
 * nicht zum Lauf. Eine Ausnahme ohne Grund ist eine Reparatur, die sich als
 * Regel verkleidet.
 */
const DARF_UEBERLEBEN = new Map<string, string>([
  // Die zwei wiederverwendeten Ergebnis-Objekte. Sie sind **kein Zustand des
  // Kampfes**, sondern Schreibflächen: Der Kampf darf im Takt nichts erzeugen
  // (GAME-DESIGN 18.2), deshalb steht je eine bereit, die bei jedem Aufruf neu
  // beschrieben wird. Ihr Inhalt zwischen zwei Aufrufen bedeutet nichts —
  // `cast()` sagt das selbst: „Wer es behalten will, kopiert die Felder
  // heraus."
  //
  // Sie im Reset zu leeren wäre nicht falsch, aber es wäre eine Zeile, die
  // etwas aufräumt, das niemand liest. Der Eintrag hier ist die Entscheidung,
  // und er nennt den Grund — nicht die Reparatur, die sich als Regel verkleidet.
  ['outcome.result', 'Schreibfläche, kein Zustand'],
  ['outcome.spellIndex', 'Schreibfläche, kein Zustand'],
  ['outcome.damage', 'Schreibfläche, kein Zustand'],
  ['outcome.resourceBefore', 'Schreibfläche, kein Zustand'],
  ['outcome.bonusApplied', 'Schreibfläche, kein Zustand'],
  ['deferredOutcome.result', 'Schreibfläche, kein Zustand'],
  ['deferredOutcome.spellIndex', 'Schreibfläche, kein Zustand'],
  ['deferredOutcome.damage', 'Schreibfläche, kein Zustand'],
  ['deferredOutcome.resourceBefore', 'Schreibfläche, kein Zustand'],
  ['deferredOutcome.bonusApplied', 'Schreibfläche, kein Zustand'],
]);

/**
 * Alle Zustandswerte eines Kampfes, als flache Pfad-Wert-Abbildung.
 *
 * # Warum `Object.keys` und nicht die Getter
 *
 * Die Getter zeigen, was die Anzeige braucht — nicht, was der Kampf hält.
 * `decayCountValue` etwa hat einen Getter, `nextAfflictionAtMs` keinen; beide
 * sind Zustand, und beide gehören geprüft.
 *
 * # Warum mit Rekursion, und warum sie eine Sperre braucht
 *
 * Zustand liegt auch eine Ebene tiefer: `stacks.gains` zählt die
 * Vorrats-Gewinne und überlebte den Reset. Die Schul-Mechaniken zeigen
 * allerdings über `sink` auf den Kampf **zurück** — ein erster Messversuch
 * lief in genau diese Schleife und meldete fünfundzwanzig Zeilen Rauschen,
 * in denen dieselben Felder unter fremden Pfaden noch einmal standen.
 *
 * Deshalb: eine Sperre über bereits gesehene Objekte, und Rückverweise auf die
 * Wurzel werden übersprungen.
 */
function zustand(combat: Combat): Map<string, unknown> {
  const werte = new Map<string, unknown>();
  const gesehen = new WeakSet<object>([combat]);
  // **In die Breite, nicht in die Tiefe** — und das ist kein Geschmack: Jeder
  // Wert wird über seinen KÜRZESTEN Pfad gemeldet. In die Tiefe gegangen hieß
  // `stacks.gains` beim ersten Messversuch
  // `events.entries.0.0.receiver.gains`, weil der Ereignis-Bus früher im
  // Konstruktor steht als die Schul-Mechanik. Ein Wächter, der ein Feld unter
  // einem Pfad meldet, unter dem es niemand sucht, hat es nur halb gefunden.
  const reihe: [object, string][] = [[combat, '']];

  while (reihe.length > 0) {
    const [o, pfad] = reihe.shift() as [object, string];
    for (const schluessel of Object.keys(o)) {
      const wert = (o as Record<string, unknown>)[schluessel];
      const p = pfad === '' ? schluessel : `${pfad}.${schluessel}`;
      if (ArrayBuffer.isView(wert)) {
        // Elementweise, als Zeichenkette: Zwei Reihen sind gleich, wenn ihre
        // Inhalte gleich sind — nicht, wenn es dieselbe Reihe ist.
        werte.set(p, Array.from(wert as unknown as ArrayLike<number>).join(','));
      } else if (wert instanceof Map || wert instanceof Set) {
        werte.set(p, wert.size);
      } else if (wert !== null && typeof wert === 'object') {
        // Der Rückverweis der Schul-Mechanik auf den Kampf steht schon in
        // `gesehen`; ihm zu folgen hieße, alles doppelt zu vergleichen.
        if (gesehen.has(wert)) continue;
        gesehen.add(wert);
        reihe.push([wert, p]);
      } else if (typeof wert !== 'function') {
        werte.set(p, wert);
      }
    }
  }
  return werte;
}

function neuerKampf(boss: BossDef = BOSS): Combat {
  return new Combat({ school: CALCINATION.value, boss });
}

/**
 * Ein Lauf, der möglichst viel Zustand hinterlässt.
 *
 * **Die Zauber sind gewählt, nicht gegriffen**: Sie decken Schaden, Brand,
 * Entladung, Zersetzungs-Stapel, Schutzschicht, Kontroll-Punkte und das
 * Unterbrechen ab. Was kein Zauber berührt, kann auch kein Wächter finden.
 */
function einLauf(combat: Combat): void {
  const ids = ['kindle', 'smoulder', 'pyre', 'canker', 'fester', 'crust', 'bind', 'quench'];
  const plaetze = ids.map((id) => combat.indexOf(id)).filter((i) => i >= 0);
  combat.beginFight(0);
  for (let t = 0; t <= 20_000; t += TICK_MS) {
    combat.stepPassive(t);
    for (const i of plaetze) {
      if (combat.isReady(i, t)) {
        combat.cast(i, t);
        break;
      }
    }
  }
  // **Zum Schluss noch einmal Zersetzung**, damit Stapel LIEGEN, wenn der Lauf
  // endet. Ohne das war `decayCount` in meinem ersten Messlauf zufällig 0 —
  // die Stapel waren verfallen, das Feld wäre nicht gemeldet worden, und der
  // Wächter hätte geschwiegen, ohne dass etwas richtig gewesen wäre.
  //
  // Ein einzelner Nachdruck genügte dafür nicht: CANKER lag auf Abklingzeit.
  // Deshalb ein paar Takte weiter, bis er wieder bereit ist — und die
  // Zusicherung unten misst nach, dass es geklappt hat.
  const canker = combat.indexOf('canker');
  if (canker >= 0) {
    for (let t = 20_000; t <= 23_000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.isReady(canker, t)) {
        combat.cast(canker, t);
        break;
      }
    }
  }
}

describe('Der gespielte Kampf trägt überhaupt Zustand', () => {
  /**
   * **Die Zusicherung, die dem Wächter seinen Wert gibt.**
   *
   * Vergleicht er zwei leere Kämpfe, ist er grün — und behauptet damit, der
   * Reset sei vollständig. Dieselbe Frage wie bei der gemischten Leiste in
   * #328: Fährt mein Lauf überhaupt das, was ich prüfen will?
   */
  it('hinterlässt Schaden, Treffer, Stapel und eine vorgerückte Zeitleiste', () => {
    const combat = neuerKampf();
    einLauf(combat);

    expect(combat.damageDealt, 'ohne Schaden misst der Wächter nichts').toBeGreaterThan(0);
    expect(combat.damageTaken, 'der Boss muss getroffen haben').toBeGreaterThan(0);
    expect(combat.timelineIndex, 'die Zeitleiste muss vorgerückt sein').toBeGreaterThan(0);
    expect(combat.decayStacks, 'es müssen Stapel LIEGEN, nicht verfallen sein').toBeGreaterThan(0);
    expect(combat.playerHp, 'der Spieler muss Schaden genommen haben').toBeLessThan(5000);
  });
});

describe('Nach dem Zurücksetzen steht der Kampf wie ein frischer', () => {
  for (const id of BOESSE) {
    it(`lässt gegen ${id} KEIN Feld stehen — und nennt jedes, das es doch tut`, () => {
      const boss = bossOder(id);
      const gespielt = neuerKampf(boss);
      einLauf(gespielt);
      gespielt.reset();

      const nachher = zustand(gespielt);
      const frisch = zustand(neuerKampf(boss));

      const abweichungen: string[] = [];
      for (const [pfad, wert] of nachher) {
        if (DARF_UEBERLEBEN.has(pfad)) continue;
        const erwartet = frisch.get(pfad);
        if (erwartet !== wert) {
          abweichungen.push(`${pfad}: nach reset ${String(wert)}, frisch ${String(erwartet)}`);
        }
      }

      // **Der Wächter nennt das FELD, nicht die Tatsache.** Ein Wächter über
      // siebzig Werte, der „etwas weicht ab" meldet, ist ein Ampellicht ohne
      // Adresse.
      expect(abweichungen, `${id}:\n${abweichungen.join('\n')}`).toEqual([]);
    });
  }

  it('vergleicht überhaupt genug Werte, um etwas finden zu können', () => {
    // Ohne diese Zahl wäre der Fall oben auch dann grün, wenn `zustand()`
    // nichts einsammelt — etwa weil jemand die Rekursion abklemmt.
    const werte = zustand(neuerKampf());
    expect(werte.size, [...werte.keys()].join(', ')).toBeGreaterThan(60);
    // Und die Stichproben: eines flach, eines eine Ebene tiefer.
    expect([...werte.keys()]).toContain('playerHpValue');
    expect([...werte.keys()]).toContain('stacks.gains');
  });
});
