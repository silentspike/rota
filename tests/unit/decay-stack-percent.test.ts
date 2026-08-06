/**
 * Die Zersetzungs-Stapel wirken mit 6 % je Stapel (Issue #482, #224).
 *
 * # Warum es diesen Prüffall gibt, obwohl es nur eine Zahl ist
 *
 * Sie stand ein Jahr lang falsch da — nicht als Fehler, sondern als
 * **Vermerk**: „Wird gemessen." Die Messung hatte am 2026-08-05 stattgefunden
 * (Issue #224), das Design trug die 6 % samt Herleitung, und der Code blieb
 * auf 3. Niemand hat es bemerkt, weil beide Seiten dieselbe falsche Zahl
 * trugen und die Paritätsprüfung deshalb grün war.
 *
 * Ein Prüffall auf eine Konstante ist normalerweise wertlos — er schreibt ab,
 * was danebensteht. Dieser hier prüft etwas anderes: dass die Zahl im Code mit
 * der **Entscheidung** übereinstimmt und dass sie **ankommt**.
 *
 * # Die Herleitung, damit sie nicht wieder verlorengeht
 *
 * GAME-DESIGN 221: Der Break-even, ab dem sich ein Platz auf der Leiste
 * rechnet, liegt bei 5,4 % je Stapel. Mit 3 % wäre PUTREFACTION **nie** die
 * beste Wahl — und das ist etwas anderes als „allein gespielt schwach", was
 * das Design über die Schule sagt.
 */
import { describe, expect, it } from 'vitest';
import { Combat, DECAY_STACK_PERCENT, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Stapel aufbauen und dann den Schaden eines Zaubers messen. */
function schadenBei(stapelZiel: number): { stapel: number; schaden: number } {
  const combat = kampf();
  const canker = combat.indexOf('canker');
  const blight = combat.indexOf('blight');
  for (let t = 0; t <= 20_000 && combat.decayStacks < stapelZiel; t += 10) {
    combat.stepPassive(t);
    if (!combat.gcd.isReady(t)) continue;
    if (combat.isOffCooldown(canker, t)) combat.cast(canker, t);
    else if (combat.isOffCooldown(blight, t)) combat.cast(blight, t);
  }
  const stapel = combat.decayStacks;
  // Bis zur nächsten freien Wartezeit, sonst wird der Wurf abgelehnt.
  let zeit = 20_010;
  while (!combat.gcd.isReady(zeit)) {
    combat.stepPassive(zeit);
    zeit += 10;
  }
  // KINDLE misst am saubersten: fester Grundschaden, keine eigene
  // Stapel-Mechanik, andere Schule. Was sich ändert, ist die Verwundbarkeit.
  const wurf = combat.cast(combat.indexOf('kindle'), zeit);
  expect(wurf.result, 'der Wurf wurde abgelehnt — dann misst dieser Fall nichts').toBe(0);
  return { stapel, schaden: wurf.damage };
}

describe('AC1 — die Zahl steht auf dem entschiedenen Wert', () => {
  it('sechs Prozent, nicht drei', () => {
    // **Kein Abschreiben.** Die 6 stehen in GAME-DESIGN 177 als Entscheidung
    // vom 2026-08-05; hier wird geprüft, dass der Code sie trägt.
    expect(DECAY_STACK_PERCENT).toBe(6);
  });

  it('und über dem Break-even aus der Herleitung', () => {
    // GAME-DESIGN 221: unter 5,4 % rechnet sich der Leisten-Platz nie. Diese
    // Zeile fängt eine künftige Senkung, die die Herleitung übersieht.
    expect(DECAY_STACK_PERCENT).toBeGreaterThan(5.4);
  });
});

describe('AC2 — und sie kommt im Kampf an', () => {
  it('jeder Stapel erhöht den Schaden messbar', () => {
    const ohne = schadenBei(0);
    const mit = schadenBei(MAX_DECAY_STACKS);
    expect(ohne.stapel).toBe(0);
    expect(mit.stapel).toBe(MAX_DECAY_STACKS);
    expect(mit.schaden).toBeGreaterThan(ohne.schaden);
  });

  it('und zwar GENAU um sechs Prozent je Stapel', () => {
    // **Das ist der Kern.** „Mehr Schaden" wäre auch bei 3 % wahr. Geprüft
    // wird der Faktor, nicht die Richtung.
    const ohne = schadenBei(0);
    const mit = schadenBei(MAX_DECAY_STACKS);
    const erwartet = Math.round((ohne.schaden * (100 + mit.stapel * DECAY_STACK_PERCENT)) / 100);
    expect(mit.schaden).toBe(erwartet);
  });

  it('GEGENPROBE: mit dem alten Wert wäre es ein anderer Schaden', () => {
    // Ohne diese Zeile bliebe offen, ob der Fall oben die Zahl prüft oder nur
    // eine Rechnung mit sich selbst vergleicht.
    const ohne = schadenBei(0);
    const mit = schadenBei(MAX_DECAY_STACKS);
    const mitDrei = Math.round((ohne.schaden * (100 + mit.stapel * 3)) / 100);
    expect(mit.schaden).not.toBe(mitDrei);
  });
});

describe('Beide Seiten tragen dieselbe Zahl', () => {
  it('die Rust-Konstante steht im Quelltext daneben', async () => {
    // **Warum ein Quelltext-Blick und kein Aufruf:** Der Simulator läuft nicht
    // in Vitest. Was hier zählt, ist, dass die beiden Zahlen nicht
    // auseinanderlaufen — und das ist im Text sichtbar.
    //
    // Die Prüfsumme fängt es ohnehin: Vier Aufzeichnungen wirken
    // PUTREFACTION-Zauber und brechen ab dem ersten Takt mit einem Stapel.
    // Diese Zeile sagt nur früher Bescheid und nennt den Grund.
    // Über Vites `?raw` und nicht über `node:fs`: Das Projekt führt bewusst
    // keine Node-Typen (`tsconfig.json`), und `tsc` fängt den Import sofort.
    const roh = (await import('../../sim/src/fight.rs?raw')) as { default: string };
    const quelle = roh.default;
    expect(
      quelle,
      'sim/src/fight.rs trägt eine andere Zahl als combat.ts — die Prüfsumme ' +
        'bricht dann ab dem ersten Takt mit einem Stapel',
    ).toContain(`pub const DECAY_STACK_PERCENT: u32 = ${DECAY_STACK_PERCENT};`);
  });
});
