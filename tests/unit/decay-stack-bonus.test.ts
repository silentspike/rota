/**
 * `perDecayStackPercent` kommt am Schaden an (Issue #484).
 *
 * # WARUM „MEHR STAPEL, MEHR SCHADEN" HIER NICHTS BELEGT
 *
 * Das stimmt auch **ohne** diese Mechanik: Zersetzungs-Stapel heben über
 * `vulnerability` den Schaden **aller** Quellen. Wer so prüft, bekommt grün aus
 * der falschen Mechanik — genau darauf bin ich beim Bau von MIASMA zweimal
 * hereingefallen, und genau deshalb blieb das Feld so lange unangeschlossen.
 *
 * Der Prüffall muss den **Betrag** treffen. Bei fünf Stapeln:
 *
 * | | Faktor |
 * | --- | --- |
 * | ohne den Zuschlag (nur Verwundbarkeit) | `1 + 5 × 3/100` = **1,15** |
 * | mit FESTERs Zuschlag von 20 | `1 + 5 × (3+20)/100` = **2,15** |
 *
 * Zwischen 1,15 und 2,15 liegt keine Rundung, sondern eine Mechanik.
 *
 * # Die Entscheidung dahinter (GAME-DESIGN 1405)
 *
 * # AC7 IST AUF DIESEM STAND NICHT PRUEFBAR — und das ist kein Versaeumnis
 *
 * Das Kriterium verlangt, dass `parity:export` **jede** Aufzeichnung mit FESTER
 * und liegenden Stapeln bewegt — und **keine andere**. Die zweite Haelfte laesst
 * sich hier nicht belegen:
 *
 * #481 (SCAB) liegt im selben Zweig und bewegt ueber die gestiegene Zauberzahl
 * bereits **jede** Pruefsumme. Die Zielgenauigkeit dieser Aenderung ist damit
 * nicht von der Breitenwirkung des anderen Vorhabens zu trennen.
 *
 * Ein eigener Zweig von main wuerde es nur unter einer Bedingung loesen, die
 * niemand garantieren kann: dass #484 **vor** #481 gemergt wird. Kommt SCAB
 * zuerst, tritt beim naechsten Export dasselbe auf.
 *
 * Deshalb steht das hier und nicht als Haken: Ein Kriterium, das man nicht
 * erfuellen kann, wird sonst entweder abgehakt („die Tests sind gruen“) oder
 * blockiert ein fertiges Vorhaben. Die dritte Moeglichkeit ist, die
 * **Messbarkeit selbst zu messen** und das Ergebnis aufzuschreiben.
 *
 * Der Zuschlag wird **additiv in denselben Topf** gerechnet, in dem die Stapel
 * schon liegen — nicht als eigener Multiplikator. *„Was multiplikativ ist, wird
 * gejagt."* Ein neunter Faktor nur für einen Zauber wäre der Diablo-IV-Fehler
 * mit der Verwundbarkeit.
 */
import { describe, expect, it } from 'vitest';
import { Combat, DECAY_STACK_PERCENT, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Der Schaden eines Zaubers, einmal ohne und einmal mit vollen Stapeln. */
function verhaeltnis(id: string): number {
  const ohne = kampf();
  const i = ohne.indexOf(id);
  ohne.cast(i, 0);
  const basis = ohne.damageDealt;

  const mit = kampf();
  const canker = mit.indexOf('canker');
  // Stapel legen, bis die Leiste voll ist — über CANKER, nicht über den
  // Prüfling selbst: Sonst trüge der erste Wurf schon seinen eigenen Zuschlag.
  for (let t = 0; mit.decayStacks < MAX_DECAY_STACKS && t < 60_000; t += 10) {
    mit.stepPassive(t);
    if (mit.gcd.isReady(t) && mit.isOffCooldown(canker, t)) mit.cast(canker, t);
  }
  const vorher = mit.damageDealt;
  mit.cast(mit.indexOf(id), 60_000);
  return (mit.damageDealt - vorher) / basis;
}

describe('AC2 — FESTER schlägt härter zu als die Verwundbarkeit allein', () => {
  it('FESTER trägt den Zuschlag', () => {
    // Die Schranke ist der Punkt: Ohne den Anschluss läge das Verhältnis bei
    // 1,15 (nur Verwundbarkeit). Alles darüber kann NUR aus dem Feld kommen.
    const nurVerwundbarkeit = 1 + (MAX_DECAY_STACKS * DECAY_STACK_PERCENT) / 100;
    const gemessen = verhaeltnis('fester');
    expect(
      gemessen,
      `FESTER bei ${MAX_DECAY_STACKS} Stapeln: ${gemessen} — die allgemeine ` +
        `Verwundbarkeit allein ergäbe ${nurVerwundbarkeit}`,
    ).toBeGreaterThan(nurVerwundbarkeit * 1.4);
  });

  it('und der gemessene Betrag entspricht der additiven Rechnung', () => {
    // **Nicht nur die Richtung, sondern die Zahl.** Ein multiplikativer Aufbau
    // ergäbe hier 1,15 × 2,0 = 2,30 statt 2,15 — der Fall unterscheidet die
    // beiden Bauarten, statt beide durchzulassen.
    const c = kampf();
    const proStapel = c.spellAt(c.indexOf('fester'))?.perDecayStackPercent ?? 0;
    expect(
      proStapel,
      'FESTER trägt das Feld nicht mehr — dann prüft der Fall nichts',
    ).toBeGreaterThan(0);

    const additiv = 1 + (MAX_DECAY_STACKS * (DECAY_STACK_PERCENT + proStapel)) / 100;
    expect(verhaeltnis('fester')).toBeCloseTo(additiv, 1);
  });
});

describe('AC3 — Gegenprobe: ein Zauber OHNE das Feld bleibt bei der Verwundbarkeit', () => {
  it('CANKER steigt nur um die allgemeine Verwundbarkeit', () => {
    // Der Beleg, dass die Schranke oben nicht einfach „Stapel wirken" misst:
    // Derselbe Aufbau, derselbe Stapelstand, ein Zauber ohne den Zuschlag.
    const c = kampf();
    expect(c.spellAt(c.indexOf('canker'))?.perDecayStackPercent ?? 0).toBe(0);
    const nurVerwundbarkeit = 1 + (MAX_DECAY_STACKS * DECAY_STACK_PERCENT) / 100;
    expect(verhaeltnis('canker')).toBeCloseTo(nurVerwundbarkeit, 1);
  });
});
