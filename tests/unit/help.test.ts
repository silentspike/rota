/**
 * Hilfe-System (Issue #85, AC2 bis AC4).
 *
 * Der Wortfilter aus #81 ist hier mitgenommen: Das Hilfe-System ist die
 * Stelle, an der eine Rotations-Empfehlung am harmlosesten aussieht - "nur ein
 * kleiner Hinweis, als Lernhilfe". Genau das schliesst GAME-DESIGN 21.2 aus.
 */
import { describe, expect, it } from 'vitest';
import {
  helpFor,
  markSeen,
  missingSubjects,
  nextIntroduction,
  SCHOOL_PLAIN,
  STAT_PLAIN,
  withPlainText,
  type HelpEntry,
  type Introduction,
} from '../../src/analysis/help.js';

const HELP: HelpEntry[] = [
  {
    id: 'kindle',
    subject: 'spell',
    title: 'Kindle',
    effect: 'Deals fire damage and builds heat.',
    values: '100 damage, 1.5 s cooldown, +3 heat',
  },
  { id: 'ESSENCE', subject: 'stat', title: 'ESSENCE (Strength of the substance)', effect: 'Raises the damage of every process.' },
  { id: 'crucible-flare', subject: 'bossAbility', title: 'Flare', effect: 'The vessel vents heat. Interruptible.' },
];

describe('AC3 — Klartext neben jedem Fachbegriff', () => {
  it('setzt ihn hinter den Begriff, wie im Design vorgerechnet', () => {
    expect(withPlainText('COAGULATION')).toBe('COAGULATION (Survival)');
  });

  it('kennt alle fuenf Schulen', () => {
    for (const schule of ['CALCINATION', 'FIXATION', 'SUBLIMATION', 'COAGULATION', 'PUTREFACTION']) {
      expect(SCHOOL_PLAIN[schule], schule).toBeTruthy();
      expect(withPlainText(schule)).toContain('(');
    }
  });

  it('kennt alle sechs Werte', () => {
    for (const wert of ['ESSENCE', 'VESSEL', 'WARD', 'CATALYSIS', 'PURITY', 'YIELD']) {
      expect(STAT_PLAIN[wert], wert).toBeTruthy();
    }
  });

  it('erfindet nichts, wo es nichts weiss', () => {
    expect(withPlainText('UNBEKANNT')).toBe('UNBEKANNT');
  });
});

describe('AC2 — Erklaerfenster ueber Symbolen, Werten UND Boss-Faehigkeiten', () => {
  it('deckt alle drei Gruppen ab', () => {
    expect(missingSubjects(HELP)).toEqual([]);
  });

  it('meldet Boss-Faehigkeiten, wenn sie fehlen', () => {
    // Die Gruppe, die man vergisst: Sie steht nicht auf der Zauberleiste.
    const ohne = HELP.filter((e) => e.subject !== 'bossAbility');
    expect(missingSubjects(ohne)).toEqual(['bossAbility']);
  });

  it('zeigt kein Fenster, wo es nichts zu sagen gibt', () => {
    expect(helpFor(HELP, 'gibt-es-nicht')).toBeNull();
    expect(helpFor(HELP, 'kindle')?.effect).toContain('heat');
  });
});

describe('AC4 — die Einfuehrung ist ueberspringbar', () => {
  const INTROS: Introduction[] = [
    { mechanicId: 'heat', title: 'Heat', body: 'Every process adds heat.' },
  ];

  it('erscheint beim ersten Auftreten', () => {
    expect(nextIntroduction(INTROS, { seenMechanicIds: [] }, 'heat')?.mechanicId).toBe('heat');
  });

  it('erscheint danach nicht mehr, auch wenn sie uebersprungen wurde', () => {
    // Ueberspringen zaehlt als gesehen - sonst kaeme sie beim naechsten Kampf
    // wieder, und der Spieler haette sie nicht wegbekommen.
    const nachher = markSeen({ seenMechanicIds: [] }, 'heat');
    expect(nextIntroduction(INTROS, nachher, 'heat')).toBeNull();
  });

  it('verwechselt Mechaniken nicht', () => {
    expect(nextIntroduction(INTROS, { seenMechanicIds: [] }, 'andere')).toBeNull();
  });
});

describe('Keine Rotations-Empfehlung, nirgends', () => {
  it('sagt in keiner Erklaerung, wann man etwas benutzen soll', () => {
    const texte = HELP.map((e) => `${e.title} ${e.effect} ${e.values ?? ''} ${e.synergies ?? ''}`)
      .join(' ')
      .toLowerCase();
    for (const wort of ['should', 'optimal', 'better', 'instead', 'rotation', 'before', 'after you', 'always cast']) {
      expect(texte, `"${wort}" darf nicht vorkommen`).not.toContain(wort);
    }
  });
});
