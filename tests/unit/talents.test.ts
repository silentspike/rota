/**
 * Der Talent-Faktor (Issue #417, GAME-DESIGN 12.5 und 13.1b).
 *
 * AC5 IST DIE FALLE, UND SIE STEHT IM ISSUE
 * -----------------------------------------
 * „Talente wirken über die Faktoren-Töpfe, nicht daneben" ist erfüllt, indem
 * man **nichts Neues baut** — der Topf `talents` liegt seit #196 in der einen
 * bewachten Schreibstelle. Ein Kriterium, das ohne Arbeit erfüllt ist, prüft
 * nichts.
 *
 * Deshalb misst diese Datei den UNTERSCHIED: Ein Talent-Faktor und ein
 * ESSENCE-Faktor **derselben Grösse** müssen verschiedene Ergebnisse liefern.
 * Läge beides im selben Topf, wären sie gleich — und niemand hätte es gemerkt.
 *
 * AC7 PRÜFT DEN KAMPF, NICHT DIE DATEN
 * ------------------------------------
 * In dieser Woche gab es viermal denselben Fall: eine fertige Mechanik, die
 * kein Aufruf benutzt. Ein Prüffall, der `talentFactor` direkt aufruft, belegt
 * die Rechnung — nicht ihre Anwendung. Deshalb wird hier ein Kampf gespielt.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, TALENTS } from '../../src/core/data.js';
import { computeDamage, NEUTRAL_FACTORS } from '../../src/core/damage.js';
import {
  allocationProblem,
  effectiveRanks,
  spentPoints,
  talentFactor,
  type TalentAllocation,
} from '../../src/progression/talents.js';

/**
 * Der Quelltext von `combat.ts`, über Vite geladen.
 *
 * `node:fs` bräuchte @types/node in der tsconfig — dasselbe Muster wie in
 * `crit-factors-source.test.ts`.
 */
const QUELLE = import.meta.glob('/src/core/combat.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function combatQuelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length === 0) {
    throw new Error('combat.ts nicht geladen — die Pruefung wuerde nichts messen');
  }
  return text;
}

const STAMM = 'calcination-trunk';
const ENTLADEN = 'calcination-release';

function kampf(talents: TalentAllocation = {}): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value, talents });
}

/** Ein Kindle wirken und den Schaden ablesen — der Kampf, nicht die Formel. */
function schadenEinesTreffers(talents: TalentAllocation = {}): number {
  const combat = kampf(talents);
  combat.cast(combat.indexOf('kindle'), 0);
  return combat.damageDealt;
}

describe('AC1 — ein verteilter Punkt verändert den Kampf messbar', () => {
  it('mit Rängen trifft es härter als ohne', () => {
    const ohne = schadenEinesTreffers();
    const mit = schadenEinesTreffers({ [STAMM]: 3 });
    expect(ohne).toBeGreaterThan(0);
    expect(mit).toBeGreaterThan(ohne);
  });

  it('mehr Ränge wirken mehr', () => {
    const einer = schadenEinesTreffers({ [STAMM]: 1 });
    const drei = schadenEinesTreffers({ [STAMM]: 3 });
    expect(drei).toBeGreaterThan(einer);
  });

  it('die Rechnung ist 1 + Promille/1000, additiv innerhalb des Topfes', () => {
    // **Innerhalb additiv** (13.1b): Drei Ränge à 20 Promille sind 60, nicht
    // 1,02³. Wer sie einzeln multiplizierte, machte aus einem Topf drei.
    expect(talentFactor({ [STAMM]: 3 })).toBeCloseTo(1.06, 10);
    expect(talentFactor({ [STAMM]: 3, [ENTLADEN]: 2 })).toBeCloseTo(1.11, 10);
  });
});

describe('AC5 — der eigene Topf, und der Unterschied ist messbar', () => {
  it('GEGENPROBE: derselbe Betrag in zwei Töpfen ergibt etwas anderes als in einem', () => {
    // Die Aussage von 13.1b in Zahlen: Zwischen den Töpfen wird multipliziert.
    // 1,1 × 1,1 ist 1,21 — im selben Topf wären es 1,2.
    const zweiToepfe = computeDamage(1000, {
      ...NEUTRAL_FACTORS,
      essence: 1.1,
      talents: 1.1,
    });
    const einTopf = computeDamage(1000, { ...NEUTRAL_FACTORS, essence: 1.2 });

    expect(zweiToepfe).toBe(1210);
    expect(einTopf).toBe(1200);
    expect(zweiToepfe).not.toBe(einTopf);
  });

  it('und der Kampf schreibt in den Topf `talents`, nicht in `essence`', () => {
    // **Am Quelltext und nicht am Schaden**, und das ist eine Grenze, die
    // benannt gehört: Solange NUR ein Topf gefüllt ist, ergeben beide
    // dieselbe Zahl — `essence: 1.06` und `talents: 1.06` liefern denselben
    // Treffer. Der Unterschied entsteht erst, wenn beide gefüllt sind, und
    // dafür bräuchte dieser Prüffall eine Ausrüstung.
    //
    // Was die Rechnung angeht, ist es oben am Beispiel belegt. Hier bleibt die
    // Frage, WELCHE Zeile in `combat.ts` steht — und die beantwortet der
    // Quelltext eindeutig.
    const quelle = combatQuelle();
    expect(quelle).toContain('talents: talentFactor(');
    // Und ausdrücklich nicht in den Nachbartopf gemischt.
    expect(quelle).not.toContain('essence: NEUTRAL_FACTORS.essence * talentFactor');
    expect(quelle).not.toMatch(/essence:[^,]*talentFactor/);
  });
});

describe('AC7 — der KAMPF wendet sie an, nicht die Daten kennen sie', () => {
  it('ein gespielter Treffer trägt den Faktor', () => {
    // **Gespielt, nicht gerechnet.** `talentFactor` direkt aufzurufen belegt
    // die Formel; hier wirkt ein Zauber, und der Schaden muss den Faktor
    // tragen. Wer die Zeile in `combat.ts` entfernt, bleibt bei `talentFactor`
    // grün und fällt hier.
    const ohne = schadenEinesTreffers();
    const mit = schadenEinesTreffers({ [STAMM]: 3 });
    expect(mit).toBe(Math.round(ohne * talentFactor({ [STAMM]: 3 })));
  });

  it('GEGENPROBE: ohne Verteilung bewegt sich nichts', () => {
    // Die Zusicherung, an der die unveränderte Referenz hängt: Solange niemand
    // verteilt, ist der Faktor exakt 1,0 und die Multiplikation bewegt keine
    // Zahl.
    expect(talentFactor({})).toBe(1);
    expect(schadenEinesTreffers()).toBe(schadenEinesTreffers({}));
  });
});

describe('AC2 — Umskillen setzt vollständig zurück', () => {
  it('ein neuer Kampf mit anderer Verteilung rechnet anders', () => {
    // Zwischen Kämpfen kostenlos (12.4): Der Kampf liest die Verteilung beim
    // Aufbau, und ein neuer Kampf bekommt eine neue.
    const stark = schadenEinesTreffers({ [STAMM]: 3, [ENTLADEN]: 5 });
    const schwach = schadenEinesTreffers({ [STAMM]: 1 });
    const gar = schadenEinesTreffers();
    expect(stark).toBeGreaterThan(schwach);
    expect(schwach).toBeGreaterThan(gar);
  });

  it('kein Rest bleibt hängen — auch nach `reset`', () => {
    // `reset` setzt den Kampfzustand zurück, nicht die Zusammenstellung. Der
    // Faktor gehört zur Zusammenstellung: Nach dem Zurücksetzen muss derselbe
    // Treffer wieder denselben Schaden machen.
    const combat = kampf({ [STAMM]: 3 });
    combat.cast(combat.indexOf('kindle'), 0);
    const erster = combat.damageDealt;
    combat.reset();
    expect(combat.damageDealt).toBe(0);
    combat.cast(combat.indexOf('kindle'), 0);
    expect(combat.damageDealt).toBe(erster);
  });
});

describe('Eine alte Verteilung bricht den Kampf nicht ab', () => {
  it('zu viele Ränge werden geklemmt', () => {
    const talent = TALENTS.find((t) => t.id === STAMM);
    expect(talent?.maxRanks).toBe(3);
    expect(effectiveRanks({ [STAMM]: 99 }, talent!)).toBe(3);
    expect(talentFactor({ [STAMM]: 99 })).toBe(talentFactor({ [STAMM]: 3 }));
  });

  it('unbekannte Kennungen werden übergangen', () => {
    // Ein Spielstand aus einer älteren Fassung kann ein Talent nennen, das es
    // nicht mehr gibt. Ein Kampf, der deswegen gar nicht startet, wäre
    // schlimmer als einer, der die alte Verteilung zu Ende rechnet.
    expect(talentFactor({ 'gibt-es-nicht': 5 })).toBe(1);
    expect(schadenEinesTreffers({ 'gibt-es-nicht': 5 })).toBe(schadenEinesTreffers());
  });

  it('negative und krumme Ränge zählen nicht', () => {
    expect(talentFactor({ [STAMM]: -3 })).toBe(1);
    expect(talentFactor({ [STAMM]: Number.NaN })).toBe(1);
  });
});

describe('Was verteilt werden DARF — getrennt von dem, was gerechnet wird', () => {
  it('nennt zu viele Ränge', () => {
    expect(allocationProblem({ [STAMM]: 4 }, 50)).toContain('erlaubt sind 3');
  });

  it('nennt einen Weg ohne seinen Stamm', () => {
    // 12.5: „Gemeinsamer Stamm → drei Wege". Ohne die Bedingung wäre es eine
    // Liste und kein Baum.
    expect(allocationProblem({ [ENTLADEN]: 1 }, 50)).toContain(STAMM);
    expect(allocationProblem({ [STAMM]: 1, [ENTLADEN]: 1 }, 50)).toBeNull();
  });

  it('nennt mehr verteilte als verdiente Punkte', () => {
    expect(allocationProblem({ [STAMM]: 3 }, 2)).toContain('3 Punkte verteilt, 2 verdient');
    expect(allocationProblem({ [STAMM]: 3 }, 3)).toBeNull();
  });

  it('nennt ein unbekanntes Talent', () => {
    expect(allocationProblem({ 'gibt-es-nicht': 1 }, 50)).toContain('Unbekanntes Talent');
  });

  it('GEGENPROBE: die Prüfung lässt eine gültige Verteilung durch', () => {
    // Eine Prüfung, die alles ablehnt, lehnt auch das Richtige ab.
    expect(allocationProblem({ [STAMM]: 3, [ENTLADEN]: 5 }, 8)).toBeNull();
    expect(spentPoints({ [STAMM]: 3, [ENTLADEN]: 5 })).toBe(8);
  });

  it('und sie ist NICHT dieselbe Funktion wie die Rechnung', () => {
    // Der Unterschied ist der Zweck: Wer verteilt, fragt `allocationProblem`;
    // wer rechnet, klemmt. Dieselbe Verteilung wird hier abgelehnt und dort
    // klaglos gerechnet.
    expect(allocationProblem({ [STAMM]: 99 }, 50)).not.toBeNull();
    expect(talentFactor({ [STAMM]: 99 })).toBeGreaterThan(1);
  });
});
