/**
 * SUTURE macht aus der liegenden Schicht Heilung (Issue #481).
 *
 * # Warum dieser Zauber zwei Zusicherungen braucht, nicht eine
 *
 * Er verbindet zwei Wege der Schule, und beide Hälften können einzeln
 * schiefgehen:
 *
 * ```text
 *   Weg 1 → weg    Die Schicht muss danach VERSCHWUNDEN sein.
 *   Weg 2 → da     Das Leben muss um ihren Betrag gestiegen sein.
 * ```
 *
 * Ein Fall, der nur das Zweite prüft, wäre auch dann grün, wenn die Schicht
 * stehen bliebe — dann gäbe der Zauber beides, und das ist kein Umwandeln,
 * sondern ein Geschenk. Ein Fall, der nur das Erste prüft, wäre grün, wenn er
 * die Schicht bloß löschte.
 *
 * # Der Kontrollfall gehört dazu
 *
 * Ohne Schicht darf nur der Grundbetrag ankommen. Sonst nähme der Zauber
 * etwas aus dem Nichts.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const SUTURE = CALCINATION.value.spells.find((s) => s.id === 'suture');
const CRUST = CALCINATION.value.spells.find((s) => s.id === 'crust');

function kampf(bossId = 'test-onslaught'): Combat {
  const boss = bossById(bossId);
  if (boss === undefined) throw new Error(`data/bosses/${bossId}.json fehlt.`);
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never, seed: 0 });
  combat.beginFight(0);
  return combat;
}

function wirf(combat: Combat, id: string, timeMs: number): void {
  const index = alsSpellIndex(combat.indexOf(id));
  expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
  expect(combat.cast(index, timeMs).result, `${id} wurde abgewiesen`).toBe(CAST_OK);
}

describe('SUTURE — die Schicht wird zu Heilung', () => {
  it('trägt die beiden Felder, aus denen die Beträge folgen', () => {
    expect(SUTURE?.heal?.amount).toBe(400);
    expect(SUTURE?.heal?.fromShieldPercent).toBe(100);
    // CRUSTs Betrag steht in den Zahlen der Fälle unten. Ändert er sich, sollen
    // sie rot werden — aber mit dieser Zeile als erstem Hinweis.
    expect(CRUST?.shield?.amount).toBe(900);
  });

  it('nimmt die Schicht WEG und schreibt ihren Betrag gut', () => {
    const combat = kampf();
    // Erst Schaden nehmen (Treffer bei 5000), sonst deckelt die Heilung.
    for (let t = 0; t <= 5200; t += TICK_MS) combat.stepPassive(t);
    wirf(combat, 'crust', 5200);
    for (let t = 5210; t <= 6800; t += TICK_MS) combat.stepPassive(t);
    expect(combat.shield, 'CRUST hat keine Schicht gelegt').toBe(900);

    wirf(combat, 'suture', 6800);

    // Weg 1: weg. Weg 2: da. Beide Hälften einzeln.
    expect(combat.shield, 'die Schicht steht noch — dann gibt der Zauber beides').toBe(0);
    expect(combat.shieldStacks).toBe(0);
    expect(combat.healed, '400 Grundbetrag + 900 aus der Schicht').toBe(1300);
  });

  it('heilt ohne Schicht NUR den Grundbetrag', () => {
    const combat = kampf();
    for (let t = 0; t <= 5200; t += TICK_MS) combat.stepPassive(t);
    expect(combat.shield, 'es liegt eine Schicht, die hier nicht liegen darf').toBe(0);

    wirf(combat, 'suture', 5200);
    expect(combat.healed).toBe(400);
  });

  it('nimmt GESTAPELTE Schichten vollständig — nicht nur eine', () => {
    // **SCAB statt CRUST, und das ist gemessen.** Zwei CRUST-Schichten
    // brauchen 6000 ms Abklingzeit plus die Wartezeit dazwischen — über 7,5 s
    // ohne Treffer. Dieses Fenster gibt es bei `test-onslaught` nicht: Der
    // erste Entwurf stapelte über den Schlag bei 12_000 hinweg, der 1600 der
    // 1800 auffraß (gemessen: 200 statt 1800). Der zweite wich auf
    // `test-collision` aus — dort teilt der Boss bis dahin nur 480 aus, und
    // die Heilung wäre am Deckel gemessen worden statt am Stapel.
    //
    // SCAB legt 200 je Anwendung bei 1500 ms Abklingzeit. Drei Stapel stehen
    // nach 3000 ms, und die passen zwischen zwei Treffer.
    const combat = kampf();
    for (let t = 0; t <= 12_200; t += TICK_MS) combat.stepPassive(t);
    for (const zeit of [12_200, 13_700, 15_200]) {
      for (let t = 12_210; t <= zeit; t += TICK_MS) combat.stepPassive(t);
      wirf(combat, 'scab', zeit);
    }
    for (let t = 15_210; t <= 16_700; t += TICK_MS) combat.stepPassive(t);

    const liegend = combat.shield;
    expect(combat.shieldStacks, 'es liegen keine drei Stapel').toBe(3);
    expect(liegend, 'die Stapel tragen nichts').toBe(600);
    const vorher = combat.healed;
    const fehlend = combat.damageTaken - combat.healed;
    expect(fehlend, 'es fehlt zu wenig, die Heilung liefe in den Deckel').toBeGreaterThan(
      400 + liegend,
    );

    wirf(combat, 'suture', 16_700);
    expect(combat.shield).toBe(0);
    expect(combat.shieldStacks).toBe(0);
    // Der ganze liegende Betrag, nicht der einer Anwendung. Ohne diesen Fall
    // wäre ein Zauber grün, der nur die oberste Schicht nimmt: Der Unterschied
    // wären 200 statt 600.
    expect(combat.healed - vorher).toBe(400 + liegend);
  });

  it('deckelt an der vollen Anzeige — auch die Umwandlung verpufft dort', () => {
    const combat = kampf();
    // Volle Anzeige, aber eine Schicht liegt.
    wirf(combat, 'crust', 0);
    for (let t = TICK_MS; t <= 1600; t += TICK_MS) combat.stepPassive(t);
    expect(combat.shield).toBe(900);

    wirf(combat, 'suture', 1600);
    // Nichts fehlt, also kommt nichts an — die Schicht ist trotzdem verbraucht.
    // Das ist die Kehrseite des Zaubers und gehört festgehalten: Wer ihn zur
    // falschen Zeit wirft, verliert seinen Schutz für nichts.
    expect(combat.healed).toBe(0);
    expect(combat.shield, 'die Schicht muss auch dann verbraucht sein').toBe(0);
  });
});
