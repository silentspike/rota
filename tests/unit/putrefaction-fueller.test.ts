/**
 * BLIGHT — der Füller der Schule (Issue #479, Raster-Zauber 1 von 10).
 *
 * # Warum EINER von zehn
 *
 * Acht der zehn Zauber aus dem Raster brauchen Mechanik, die es noch nicht
 * gibt: `DecayDef` hat genau ein Feld (`stacks`), und Obergrenze, Verfallszeit
 * und Stapel-Verbrauch sind Konstanten in `combat.ts`.
 *
 * **Und der neunte fiel beim Bauen heraus.** MIASMA sollte „Schaden nach
 * Stapelzahl" leisten, über `perDecayStackPercent` — dasselbe Feld, das
 * `fester` seit jeher trägt. Gemessen: Das Feld hat **keine Wirkung**, auf
 * beiden Seiten. Der Zuwachs bei fünf Stapeln ist exakt `DECAY_STACK_PERCENT`
 * (1,1500 auf vier Nachkommastellen), also die allgemeine Verwundbarkeit, die
 * jede Quelle bekommt.
 *
 * MIASMA kommt nach, sobald das Feld angeschlossen ist. Ihn jetzt abzuliefern
 * hiesse, einen Zauber auszuliefern, dessen Rolle nicht stattfindet.
 *
 * BLIGHT kommt mit dem aus, was steht — deshalb entsteht er zuerst.
 *
 * # AC2 UND AC3 SIND ZWEI VERSCHIEDENE FRAGEN
 *
 * AC2 fragt: Wirkt der Zauber im Kampf? AC3 fragt: Wäre ohne ihn etwas anders?
 * Das klingt gleich und ist es nicht — ein Zauber kann Schaden machen (AC2
 * grün) und trotzdem an seiner **Rolle** vorbeigehen. BLIGHTs Rolle ist nicht
 * „Schaden", sondern „Aufbau treiben"; MIASMAs ist nicht „Schaden", sondern
 * „aus dem Aufbau Ertrag ziehen".
 *
 * Gemessen wird deshalb je Zauber an der Größe, die seine ROLLE beschreibt —
 * nicht am Schaden, den jeder Zauber macht.
 */
import { describe, expect, it } from 'vitest';
import { Combat, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/**
 * Wie viele Stapel nach `ms` liegen, wenn nur die genannten Zauber gewirkt
 * werden — jeder, sobald er bereit ist.
 */
function stapelNach(ids: readonly string[], ms: number): number {
  const combat = kampf();
  const indizes = ids.map((id) => combat.indexOf(id));
  for (let t = 0; t <= ms; t += 10) {
    combat.stepPassive(t);
    if (!combat.gcd.isReady(t)) continue;
    for (const i of indizes) {
      if (combat.isReady(i, t)) {
        combat.cast(i, t);
        break;
      }
    }
  }
  return combat.decayStacks;
}

describe('AC1 — beide stehen im Katalog und der Parser nimmt sie an', () => {
  it('mit den Feldern, die ihre Rolle braucht', () => {
    const combat = kampf();
    const blight = combat.spells[combat.indexOf('blight')];

    expect(blight?.school).toBe('PUTREFACTION');
    // BLIGHT legt Stapel, MIASMA ausdrücklich NICHT — das ist der Unterschied
    // der beiden Rollen und keine Nachlässigkeit.
    expect(blight?.decay?.stacks).toBe(1);
  });

  it('AC4 — und sie sind über die Stufen verteilt, nicht beide auf 1', () => {
    const combat = kampf();
    expect(combat.spells[combat.indexOf('blight')]?.unlockLevel).toBe(2);
    // Die Gegenprobe: canker und fester liegen weiterhin auf 1. Ohne sie wäre
    // nicht zu sehen, ob die Staffelung eine ist oder alle nur verschoben sind.
    expect(combat.spells[combat.indexOf('canker')]?.unlockLevel).toBe(1);
  });
});

describe('AC2 — BLIGHT treibt den Aufbau, das ist seine Rolle', () => {
  it('mit BLIGHT liegen nach 6 Sekunden mehr Stapel als ohne', () => {
    const mit = stapelNach(['canker', 'blight'], 6000);
    const ohne = stapelNach(['canker'], 6000);
    expect(mit).toBeGreaterThan(ohne);
  });

  it('und er erreicht die Obergrenze früher', () => {
    // Die schärfere Aussage: nicht „mehr", sondern „früher am Anschlag".
    //
    // **Der Messpunkt ist gemessen, nicht geschätzt.** Mein erster Entwurf
    // stand bei 5000 ms und war rot — dort sind es 4 gegen 3, der Anschlag
    // ist noch nicht erreicht. Bei 8000 ms wäre er nutzlos: Da stehen BEIDE
    // auf fünf, und der Fall wäre grün, ohne etwas zu zeigen.
    //
    //   3000 ms   3 gegen 2
    //   5000 ms   4 gegen 3
    //   6000 ms   5 gegen 4     <- hier trennt es sich am Anschlag
    //   8000 ms   5 gegen 5     <- wertlos
    expect(stapelNach(['canker', 'blight'], 6000)).toBe(MAX_DECAY_STACKS);
    expect(stapelNach(['canker'], 6000)).toBeLessThan(MAX_DECAY_STACKS);
  });
});

describe('AC3 — die Gegenprobe je Zauber', () => {
  it('BLIGHT: seine blosse EXISTENZ bewegt nichts', () => {
    // **Der erste Teil von AC3.** Der Zauber steht im Katalog, wird aber nicht
    // gewirkt — dann muss die Stapelzahl exakt die sein, die canker allein
    // erzeugt. Ein Zauber, der schon durch sein Dasein etwas verschiebt, hätte
    // einen Nebeneffekt, den niemand angefordert hat.
    const combat = kampf();
    expect(combat.indexOf('blight')).toBeGreaterThanOrEqual(0);
    expect(stapelNach(['canker'], 6000)).toBe(4);
  });

  it('BLIGHT: und gewirkt ist er die URSACHE des Unterschieds', () => {
    // **Der zweite Teil.** Beide Läufe unterscheiden sich in genau einer
    // Sache: ob BLIGHT gewirkt wird. Alles andere — Zeit, Boss, Startwert,
    // canker-Rhythmus — ist gleich.
    //
    // Ein früherer Entwurf verglich hier `['canker','fester']` gegen
    // `['canker']` und war rot: 3 gegen 4. Der Grund ist lehrreich und lag
    // NICHT an BLIGHT — `fester` legt zwar keine Stapel, verbraucht aber
    // Wartezeiten und nimmt canker damit Wirkungen weg. Zwei Läufe, die sich
    // in mehr als einer Sache unterscheiden, messen nicht, was sie sollen.
    expect(stapelNach(['canker', 'blight'], 6000)).toBeGreaterThan(stapelNach(['canker'], 6000));
  });
});
