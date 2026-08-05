import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { PressQueue } from '../../src/input/queue.js';
import { NO_SPELL, Session } from '../../src/core/session.js';

/**
 * Ein Zauber, der von der Leiste genommen wird, verlässt auch die Rechnung
 * (Issue #306).
 *
 * # Der Befund
 *
 * `bind` setzte `boundSpells[neu] = 1` und löschte **nie**. Wer einen Zauber
 * von einem Platz nahm, hatte ihn aus der Leiste entfernt und trotzdem weiter
 * in der Leerlauf-Aufschlüsselung — mit voller Zeit, obwohl er nirgends mehr
 * lag.
 *
 * Aufgefallen ist das nicht an der Anzeige, sondern beim Versuch, für #300
 * eine Leiste mit **nur einem** Zauber herzustellen: Der zuvor gebundene blieb
 * markiert, beide Läufe maßen dasselbe, und die gemessene Null belegte nichts.
 *
 * # Warum die Prüffälle die Markierung direkt lesen
 *
 * `boundSpells` ist privat und hat keinen Lesezugriff. Sichtbar wird es an
 * `lostTimeNothingReadyMs`: Die Quelle erscheint nur, wenn **kein** markierter
 * Zauber bereit ist. Ein Zauber, der markiert bleibt, obwohl er weg ist,
 * unterdrückt sie also — das ist der Hebel, mit dem sich der Fehler messen
 * lässt, ohne an die Innerei zu gehen.
 */

function bau(): { session: Session; combat: Combat; queue: PressQueue } {
  const boss = FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const queue = new PressQueue();
  return { session: new Session(queue, combat), combat, queue };
}

/** Ein voller Lauf; gibt zurück, wie lange **nichts** bereit lag. */
function nothingReadyMs(session: Session, queue: PressQueue): number {
  session.beginCountdown(0);
  const ende = COUNTDOWN_MS + FIRST_CRUCIBLE.value.overpressureMs;
  let tick = 0;
  for (let t = 0; t <= ende; t += 10) {
    // Einmal wirken, damit die Abklingzeit überhaupt anläuft — ohne Druck
    // gilt jeder Zauber den ganzen Kampf als bereit (#300).
    if (t === COUNTDOWN_MS) queue.push(0, tick, t, tick);
    session.tick(t);
    tick++;
  }
  return session.summary(ende).lostTimeNothingReadyMs;
}

describe('AC1 — bind() nimmt die Markierung des vorherigen Zaubers zurück', () => {
  it('der überschriebene Zauber unterdrückt die Leerlauf-Quelle nicht mehr', () => {
    // KINDLE hat eine Abklingzeit auf GCD-Höhe: Solange er markiert ist, ist
    // immer etwas bereit, und die Quelle bleibt leer. REFLUX hat 45 s.
    const a = bau();
    a.session.bind(0, a.combat.indexOf('kindle'));
    // **Derselbe Platz, anderer Zauber.** Genau der Fall aus dem Befund.
    a.session.bind(0, a.combat.indexOf('reflux'));

    expect(
      nothingReadyMs(a.session, a.queue),
      'KINDLE ist vom Platz genommen, unterdrückt die Quelle aber weiter — ' +
        'dann führt die Rechnung ihn noch',
    ).toBeGreaterThan(0);
  });

  it('die Ursache: mit KINDLE auf einem eigenen Platz bleibt sie leer', () => {
    // **Der Gegenfall.** Ohne ihn zeigte der Fall oben nur, dass REFLUX allein
    // Leerlauf erzeugt — nicht, dass das Wegnehmen etwas bewirkt hat.
    const b = bau();
    b.session.bind(0, b.combat.indexOf('reflux'));
    b.session.bind(1, b.combat.indexOf('kindle'));
    expect(nothingReadyMs(b.session, b.queue)).toBe(0);
  });
});

describe('AC3 — bind(slot, NO_SPELL) leert den Platz', () => {
  it('der Platz ist leer und der Zauber aus der Rechnung', () => {
    const a = bau();
    a.session.bind(0, a.combat.indexOf('reflux'));
    a.session.bind(1, a.combat.indexOf('kindle'));
    expect(a.session.boundSlotCount()).toBe(2);

    a.session.bind(1, NO_SPELL);

    expect(a.session.spellOnSlot(1)).toBe(NO_SPELL);
    expect(a.session.boundSlotCount()).toBe(1);
    expect(
      nothingReadyMs(a.session, a.queue),
      'der geleerte Platz unterdrückt die Quelle weiter — die Markierung ist ' +
        'nicht zurückgenommen',
    ).toBeGreaterThan(0);
  });
});

describe('AC2 — Doppelbelegung kann die Markierung nicht verlieren', () => {
  it('derselbe Zauber auf zwei Plätzen überlebt das Leeren des einen', () => {
    // **Warum dieser Fall existiert, obwohl GAME-DESIGN 715 Doppelbelegung
    // verbietet.** Die naheliegende Reparatur wäre `boundSpells[alt] = 0` in
    // `bind`. Sie ist unter der Regel richtig und bricht still, sobald die
    // Regel woanders verletzt wird: Das Leeren des einen Platzes löschte die
    // Markierung, die der andere noch braucht.
    //
    // Die Ableitung aus den Plätzen kann diesen Fehler nicht machen. Der Fall
    // hält das fest — er prüft die REPARATUR, nicht die Regel. Dass die Lage
    // im Spiel gar nicht erst entsteht, ist #287 AC3 und ein eigenes Issue.
    const a = bau();
    const kindle = a.combat.indexOf('kindle');
    a.session.bind(0, a.combat.indexOf('reflux'));
    a.session.bind(1, kindle);
    a.session.bind(2, kindle);

    a.session.bind(1, NO_SPELL);

    // KINDLE liegt noch auf Platz 2 — er muss weiter zählen.
    expect(a.session.spellOnSlot(2)).toBe(kindle);
    expect(
      nothingReadyMs(a.session, a.queue),
      'KINDLE liegt noch auf Platz 2, wird aber nicht mehr gezählt — die ' +
        'Reparatur hat eine Markierung gelöscht, die ein anderer Platz braucht',
    ).toBe(0);
  });
});

describe('AC5 — ein liegender Zauber bleibt in der Rechnung, auch ungewirkt', () => {
  it('die Reparatur leert die Anzeige nicht', () => {
    // **Der Gegenfall zur Reparatur selbst.** Ohne ihn behöbe sie den Befund
    // auch dann, wenn sie einfach alles aus der Aufschlüsselung entfernte —
    // ein weggebundener Zauber verschwände, ein liegender aber ebenso.
    const a = bau();
    a.session.bind(0, a.combat.indexOf('reflux'));
    a.session.bind(1, a.combat.indexOf('kindle'));

    a.session.beginCountdown(0);
    const ende = COUNTDOWN_MS + FIRST_CRUCIBLE.value.overpressureMs;
    // **Kein einziger Tastendruck.** KINDLE liegt und wird nie gewirkt.
    for (let t = 0; t <= ende; t += 10) a.session.tick(t);

    const zeilen = a.session.summary(ende).idlePerSpell;
    const kindle = zeilen.find((z) => z.spellId === 'kindle');
    expect(kindle, 'KINDLE liegt auf Platz 1 und fehlt in der Aufschlüsselung').toBeDefined();
    expect(
      kindle?.idleMs,
      'KINDLE wurde nie gewirkt und müsste mit voller Leerlaufzeit dastehen',
    ).toBeGreaterThan(0);
  });
});
