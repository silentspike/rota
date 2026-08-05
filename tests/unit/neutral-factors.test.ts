import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { NEUTRAL_FACTORS, type DamageFactors } from '../../src/core/damage.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { PressQueue } from '../../src/input/queue.js';
import { Session } from '../../src/core/session.js';

/**
 * Welcher Schadensfaktor bleibt im **Produktionspfad** auf seinem neutralen
 * Wert? (Issue #424)
 *
 * # Warum es diesen Wächter gibt
 *
 * Ein Faktor auf 1 verhält sich in jedem Test genau wie ein korrekt gefüllter.
 * Deshalb ist fünf Issues lang niemandem aufgefallen, dass `buffs` auf beiden
 * Seiten fertig gebaut, mit einem gemeinsamen Prüffall abgeglichen — und in
 * keinem Kampf angeschlossen war. Dasselbe gilt für `talents` (#417) und, über
 * die fehlende Ausrüstung, für `gearEffects` (#421).
 *
 * Drei Befunde, dieselbe Form. Behebt man sie einzeln, ist die Klasse nicht
 * weg: Sie trifft als Nächstes den Topf, den heute noch niemand gebaut hat.
 *
 * # Warum die Prüffälle es verdeckt haben
 *
 * **Kein Prüffall baut einen Kampf so, wie das Spiel ihn baut.** Jeder reicht
 * den Zustand selbst herein — `worn` als Parameter, Faktoren über
 * `setDamageFactor`, Zauber frei gewählt statt über die Leiste. Damit prüft
 * jeder von ihnen die Rechnung und keiner den Eingang.
 *
 * Dieser Fall geht deshalb den Weg von `src/core/app.ts:171`: derselbe
 * Konstruktor-Aufruf, dieselben Vorgaben, keine Abkürzung.
 *
 * # Warum er ROT eingecheckt wird
 *
 * Ein Wächter, der bei seiner Einführung grün ist, hat nichts bewiesen — er
 * könnte ebensogut nichts messen. Drei bekannte Befunde stehen als offene
 * Issues dagegen, also muss er sie nennen. Grün wird er, wenn #417, #420 und
 * #421 durch sind, und nicht dadurch, dass er weniger prüft.
 */

/** Die acht Töpfe, aus dem Typ abgeleitet statt von Hand aufgezählt. */
const ALLE_TOEPFE = Object.keys(NEUTRAL_FACTORS) as (keyof DamageFactors)[];

/**
 * Die drei, die heute bekanntermaßen leer bleiben — mit dem Issue, das sie
 * schließt. Wird einer davon gefüllt, fällt er hier heraus und der Fall meldet
 * es: Ein Wächter, der einen behobenen Befund weiter erwartet, ist genauso
 * falsch wie einer, der einen offenen übersieht.
 */
const BEKANNT_LEER: Readonly<Record<string, string>> = {
  talents: '#417 — kein Talent setzt einen Faktor',
  buffs: '#420 — kein Zauber des Spiels löst eine Verstärkung aus',
  gearEffects: '#421 — app.ts übergibt kein `worn`, der Kampf fällt auf NOTHING_WORN zurück',
};

/**
 * Ein Kampf, gebaut wie in `src/core/app.ts:171` — und dann gespielt.
 *
 * Absichtlich ohne `worn`, ohne `setDamageFactor`, ohne Talente: Genau das ist
 * der Punkt. Was hier nicht gefüllt wird, wird im Spiel auch nicht gefüllt.
 */
function faktorenNachEinemLauf(): DamageFactors {
  const boss = FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const queue = new PressQueue();
  const session = new Session(queue, combat);

  // Eine Leiste, wie der Spieler sie hätte. Ohne gebundene Zauber liefe der
  // Kampf leer, und ein leerer Lauf füllte auch einen angeschlossenen Topf
  // nicht — der Fall wäre rot, ohne etwas zu zeigen.
  const leiste = ['kindle', 'smoulder', 'pyre'];
  leiste.forEach((id, slot) => session.bind(slot, combat.indexOf(id)));

  session.beginCountdown(0);
  const ende = COUNTDOWN_MS + boss.overpressureMs;
  let tick = 0;
  for (let t = 0; t <= ende; t += 10) {
    // Rotation: reihum drücken, sobald die Wartezeit frei ist. Keine gute
    // Rotation — eine, die jeden Zauber der Leiste wirklich wirkt.
    if (t >= COUNTDOWN_MS && combat.gcd.isReady(t)) {
      queue.push(tick % leiste.length, tick, t, tick);
    }
    session.tick(t);
    tick++;
  }

  return combat.damageFactors;
}

describe('Kein Faktor bleibt unbemerkt neutral (Issue #424)', () => {
  const faktoren = faktorenNachEinemLauf();

  it('AC1 — der Zugriff liefert überhaupt etwas', () => {
    // **Zuerst diese Zusicherung, dann die eigentliche Messung.** Ohne sie
    // hätte dieser Wächter denselben Fehler wie die Fälle, gegen die er
    // gebaut ist: Eine Prüfung über nichts ist grün. Würde `damageFactors`
    // umbenannt oder gäbe es `undefined` zurück, liefe die Schleife unten
    // über null Felder und meldete triumphierend „kein Topf ist neutral".
    expect(faktoren, 'combat.damageFactors liefert nichts').toBeDefined();
    expect(ALLE_TOEPFE.length, 'die Liste der Töpfe ist leer').toBe(8);
    for (const topf of ALLE_TOEPFE) {
      expect(typeof faktoren[topf], `${topf} ist keine Zahl, sondern ${typeof faktoren[topf]}`).toBe(
        'number',
      );
      expect(Number.isFinite(faktoren[topf]), `${topf} ist keine endliche Zahl`).toBe(true);
    }
  });

  it('AC2 — mindestens ein Topf wurde gefüllt, der Lauf hat also stattgefunden', () => {
    // **Der Gegenfall zur Messung selbst.** Bliebe jeder der acht Töpfe
    // neutral, wäre die Ursache vermutlich nicht, dass acht Systeme fehlen,
    // sondern dass der Lauf gar nicht gelaufen ist. Dann meldete der Fall
    // unten acht Namen und keiner davon wäre der Befund.
    const gefuellt = ALLE_TOEPFE.filter((t) => faktoren[t] !== NEUTRAL_FACTORS[t]);
    expect(
      gefuellt.length,
      `KEIN Topf wurde gefüllt — dann hat der Lauf nicht stattgefunden, und ` +
        `die Meldung unten sagt nichts über die Systeme aus`,
    ).toBeGreaterThan(0);
  });

  it('AC3 — genau die bekannten drei bleiben neutral, keiner mehr', () => {
    const neutral = ALLE_TOEPFE.filter((t) => faktoren[t] === NEUTRAL_FACTORS[t]);
    const unerwartet = neutral.filter((t) => BEKANNT_LEER[t] === undefined);

    expect(
      unerwartet,
      `Diese Töpfe bleiben im Produktionspfad auf ihrem neutralen Wert, und ` +
        `kein offenes Issue erklärt das:\n` +
        unerwartet.map((t) => `  ${t} = ${faktoren[t]}`).join('\n') +
        `\n\nEin Faktor auf seinem neutralen Wert verhält sich in jedem Test ` +
        `wie ein korrekt gefüllter. Entweder ist das System nicht ` +
        `angeschlossen — dann gehört ein Issue dazu — oder es wird in diesem ` +
        `Lauf nicht ausgelöst, dann gehört der Lauf erweitert.`,
    ).toEqual([]);
  });

  it('AC4 — und die bekannten drei sind noch offen, nicht längst behoben', () => {
    // **Die andere Richtung, und sie ist genauso wichtig.** Ein Wächter, der
    // einen behobenen Befund weiter erwartet, hält eine Unwahrheit fest und
    // wird beim nächsten Lesen für bare Münze genommen.
    const behoben = Object.keys(BEKANNT_LEER).filter(
      (t) => faktoren[t as keyof DamageFactors] !== NEUTRAL_FACTORS[t as keyof DamageFactors],
    );
    expect(
      behoben,
      `Diese Töpfe werden inzwischen gefüllt und stehen trotzdem noch in der ` +
        `Ausnahmeliste:\n` +
        behoben.map((t) => `  ${t} — ${BEKANNT_LEER[t]}`).join('\n') +
        `\n\nDen Eintrag entfernen, dann bewacht dieser Fall den Topf wieder.`,
    ).toEqual([]);
  });
});
