/**
 * Die Verlustrechnung traegt die Auswertung (Issue #81) - und sie darf dem
 * Spieler nie eine Reihenfolge vorrechnen (GAME-DESIGN 21.2). Geprueft wird
 * deshalb beides: dass die Zahlen stimmen, und dass die Aussagen nur benennen,
 * was liegen blieb.
 */
import { describe, expect, it } from 'vitest';
import {
  analyseLosses,
  damagePerCast,
  possibleCasts,
  type LossInput,
  type SpellRun,
} from '../../src/analysis/losses.js';

const KINDLE: SpellRun = {
  spellId: 'kindle',
  name: 'Kindle',
  cooldownMs: 12_000,
  casts: 15,
  damage: 15 * 4200,
  nominalDamagePerCast: 4200,
};

/** Ein Durchlauf ohne jeden Verlust - Grundlage der meisten Faelle. */
function perfect(): LossInput {
  return {
    elapsedMs: 240_000,
    damage: 20 * 4200,
    casts: 20,
    lostTimeMs: 0,
    gcdMs: 1500,
    spells: [{ ...KINDLE, casts: 20, damage: 20 * 4200 }],
    synergyUsed: 0,
    synergyAvailable: 0,
    synergyBonusDamage: 0,
    interruptsMissed: 0,
    interruptDamage: 0,
  };
}

describe('possibleCasts', () => {
  it('rechnet wie GAME-DESIGN 15.2: 12 s Abklingzeit in 240 s sind 20 Wirkungen', () => {
    expect(possibleCasts(240_000, 12_000)).toBe(20);
  });

  it('rundet ab statt auf', () => {
    expect(possibleCasts(239_000, 12_000)).toBe(19);
  });

  it('liefert 0 bei unbrauchbaren Werten', () => {
    expect(possibleCasts(0, 12_000)).toBe(0);
    expect(possibleCasts(240_000, 0)).toBe(0);
  });
});

describe('damagePerCast', () => {
  it('misst, solange es etwas zu messen gibt', () => {
    expect(damagePerCast(KINDLE)).toBe(4200);
  });

  it('faellt auf den deklarierten Wert zurueck, wenn nie gewirkt wurde', () => {
    // Ohne diesen Rueckfall wuerde ein nie benutzter Zauber als kostenlos
    // ausgewiesen - genau der teuerste Fall.
    expect(damagePerCast({ ...KINDLE, casts: 0, damage: 0 })).toBe(4200);
  });
});

describe('analyseLosses — AC1: verpasste Wirkungen', () => {
  it('weist sie als groesste Quelle aus, mit der Rechnung aus dem Design', () => {
    const report = analyseLosses(perfect());
    expect(report.sources).toEqual([]);

    // Fuenf von zwanzig Wirkungen ausgelassen (GAME-DESIGN 15.2, Beispielzeile).
    const withMisses = analyseLosses({
      ...perfect(),
      damage: KINDLE.damage,
      casts: 15,
      spells: [KINDLE],
    });
    const top = withMisses.top[0];
    expect(top?.kind).toBe('missedCasts');
    expect(top?.damage).toBe(5 * 4200);
    expect(top?.detail).toContain('5 casts missed');
    expect(top?.detail).toContain('21,000 damage left on the table');
    // 21.000 von 63.000 erzielt + 21.000 verloren = 25,0 %
    expect(top?.share).toBeCloseTo(21_000 / 84_000, 6);
    expect(top?.detail).toContain('25.0 %');
  });

  it('zaehlt einen nie gewirkten Zauber voll als Verlust', () => {
    const report = analyseLosses({
      ...perfect(),
      spells: [{ ...KINDLE, casts: 0, damage: 0 }],
      damage: 0,
      casts: 0,
    });
    expect(report.totalLost).toBe(20 * 4200);
    expect(report.perSpell[0]?.efficiency).toBe(0);
  });
});

describe('analyseLosses — AC2: Leerlauf', () => {
  it('rechnet verlorene Zeit in verschenkte Aktionen um', () => {
    // 6,4 s Leerlauf bei 1,5 s Takt sind vier Aktionen (GAME-DESIGN 15.2).
    const report = analyseLosses({ ...perfect(), lostTimeMs: 6400 });
    const idle = report.sources.find((s) => s.kind === 'idle');
    expect(idle?.detail).toContain('6.4 s idle');
    expect(idle?.detail).toContain('4 wasted actions');
    expect(idle?.damage).toBe(4 * 4200);
  });

  it('uebernimmt die verlorene Zeit unveraendert aus der Kurzfassung', () => {
    // Zweite Wahrheit ausgeschlossen: Was hier herauskommt, muss zu der
    // Sekundenzahl passen, die die Kurzfassung nennt.
    for (const lostTimeMs of [0, 1500, 6400, 30_000]) {
      const report = analyseLosses({ ...perfect(), lostTimeMs });
      const idle = report.sources.find((s) => s.kind === 'idle');
      const shown = idle ? Number(/([\d.]+) s idle/.exec(idle.detail)?.[1]) : 0;
      expect(shown).toBeCloseTo(lostTimeMs / 1000, 6);
    }
  });
});

describe('analyseLosses — AC3: die tatsaechlich mitgefuehrten Zauber', () => {
  it('ergibt andere Zahlen bei anderer Zusammenstellung, ohne Codeaenderung', () => {
    const einer = analyseLosses({
      ...perfect(),
      casts: 15,
      damage: KINDLE.damage,
      spells: [KINDLE],
    });
    const zwei = analyseLosses({
      ...perfect(),
      casts: 15,
      damage: KINDLE.damage,
      spells: [
        KINDLE,
        {
          spellId: 'smoulder',
          name: 'Smoulder',
          cooldownMs: 30_000,
          casts: 4,
          damage: 4 * 2400,
          nominalDamagePerCast: 2400,
        },
      ],
    });
    expect(zwei.totalLost).toBeGreaterThan(einer.totalLost);
    expect(zwei.perSpell).toHaveLength(2);
    // Der zweite Zauber passt achtmal in 240 s, gewirkt wurde viermal.
    const smoulder = zwei.perSpell.find((s) => s.spellId === 'smoulder');
    expect(smoulder?.possibleCasts).toBe(8);
    expect(smoulder?.missedCasts).toBe(4);
    expect(smoulder?.lostDamage).toBe(4 * 2400);
  });

  it('behandelt eine leere Zusammenstellung ohne zu stolpern', () => {
    const report = analyseLosses({ ...perfect(), spells: [], damage: 0, casts: 0 });
    expect(report.perSpell).toEqual([]);
    expect(report.totalLost).toBe(0);
    expect(report.totalShare).toBe(0);
  });
});

describe('analyseLosses — AC4: Effizienz fuehrt, nicht der Schadensanteil', () => {
  it('sortiert nach Effizienz, schlechteste zuerst', () => {
    const report = analyseLosses({
      ...perfect(),
      casts: 24,
      damage: 100_000,
      spells: [
        // Traegt den meisten Schaden, ist aber gut ausgenutzt.
        { ...KINDLE, spellId: 'a', name: 'A', casts: 19, damage: 80_000 },
        // Traegt wenig Schaden, wurde aber kaum genutzt - das ist der Fund.
        {
          spellId: 'b',
          name: 'B',
          cooldownMs: 12_000,
          casts: 5,
          damage: 20_000,
          nominalDamagePerCast: 4000,
        },
      ],
    });
    expect(report.perSpell.map((s) => s.spellId)).toEqual(['b', 'a']);
    // Und der Schadensanteil bleibt als eigene Groesse erhalten.
    expect(report.perSpell.find((s) => s.spellId === 'a')?.damageShare).toBeCloseTo(0.8, 6);
  });

  it('kappt eine Effizienz ueber 100 % nicht', () => {
    // Ein Ruecksetzer (GAME-DESIGN 4.1) kann mehr Wirkungen erlauben, als die
    // Abklingzeit zulaesst. Wer das kappt, verschweigt die Leistung.
    const report = analyseLosses({ ...perfect(), spells: [{ ...KINDLE, casts: 25 }] });
    expect(report.perSpell[0]?.efficiency).toBeCloseTo(1.25, 6);
    expect(report.perSpell[0]?.missedCasts).toBe(0);
  });
});

describe('analyseLosses — Bezugsgroesse und Rangfolge', () => {
  it('bezieht alle Prozente auf erzielt plus verloren', () => {
    const report = analyseLosses({
      ...perfect(),
      casts: 15,
      damage: 63_000,
      spells: [KINDLE],
      lostTimeMs: 6000,
    });
    expect(report.potential).toBe(63_000 + report.totalLost);
    const sum = report.sources.reduce((acc, s) => acc + s.share, 0);
    expect(sum).toBeCloseTo(report.totalShare, 6);
  });

  it('hebt hoechstens drei Quellen hervor, groesste zuerst', () => {
    const report = analyseLosses({
      ...perfect(),
      casts: 15,
      damage: 63_000,
      spells: [KINDLE],
      lostTimeMs: 9000,
      synergyUsed: 7,
      synergyAvailable: 10,
      synergyBonusDamage: 900,
      interruptsMissed: 2,
      interruptDamage: 7000,
    });
    expect(report.top).toHaveLength(3);
    expect(report.sources).toHaveLength(4);
    for (let i = 1; i < report.top.length; i++) {
      expect(report.top[i - 1]!.damage).toBeGreaterThanOrEqual(report.top[i]!.damage);
    }
  });

  it('zeigt keine Quelle ohne Verlust', () => {
    // In Schritt 1 gibt es weder Synergien noch Unterbrechungen. Eine Null
    // auszuweisen hiesse, eine Fehlerart vorzuhalten, die es nicht gibt.
    const report = analyseLosses({ ...perfect(), casts: 15, damage: 63_000, spells: [KINDLE] });
    expect(report.sources.map((s) => s.kind)).toEqual(['missedCasts']);
  });
});

describe('analyseLosses — keine Referenz-Rotation', () => {
  it('nennt in keiner Ausgabe eine Reihenfolge oder Empfehlung', () => {
    const report = analyseLosses({
      ...perfect(),
      casts: 15,
      damage: 63_000,
      spells: [KINDLE],
      lostTimeMs: 6000,
      synergyUsed: 7,
      synergyAvailable: 10,
      synergyBonusDamage: 900,
      interruptsMissed: 1,
      interruptDamage: 7000,
    });
    const texte = report.sources.map((s) => `${s.label} ${s.detail}`.toLowerCase()).join(' ');
    for (const wort of ['should', 'optimal', 'better', 'instead', 'rotation', 'order', 'try']) {
      expect(texte, `"${wort}" darf nicht vorkommen`).not.toContain(wort);
    }
  });
});
