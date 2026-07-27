/**
 * Der Credits-Bereich traegt eine Lizenzauflage: Ohne Namensnennung ist die
 * Nutzung der CC-BY-Symbole unzulaessig (GAME-DESIGN 17.2, ASSETS.md).
 *
 * Geprueft wird deshalb nicht nur, dass die Datei laedt, sondern dass sie
 * genau die Nennungen enthaelt, die die Lizenz verlangt.
 */
import { describe, expect, it } from 'vitest';
import { CREDITS, type CreditsData } from '../../src/credits/credits-data.js';
import { sortEntries, validateCredits } from '../../src/credits/credits.js';

describe('credits.json', () => {
  it('ist in sich schluessig', () => {
    expect(validateCredits(CREDITS)).toEqual([]);
  });

  it('nennt jeden der drei game-icons-Urheber mit dem geforderten Wortlaut', () => {
    // game-icons.net verlangt woertlich: "Icons made by {author}".
    const required = CREDITS.entries.filter((entry) => entry.required);
    const texts = required.map((entry) => entry.attribution);
    expect(texts).toContain('Icons made by Lorc');
    expect(texts).toContain('Icons made by Delapouite');
    expect(texts).toContain('Icons made by Cathelineau');
  });

  it('kennzeichnet jede CC-BY-Quelle als nennungspflichtig', () => {
    for (const entry of CREDITS.entries) {
      if (entry.license.startsWith('CC BY')) {
        expect(entry.required, `${entry.id} ist CC BY, aber nicht als pflichtig markiert`).toBe(
          true,
        );
      }
    }
  });

  it('verweist bei jedem Eintrag auf Quelle und Lizenztext', () => {
    for (const entry of CREDITS.entries) {
      expect(entry.source, entry.id).toMatch(/^https:\/\//);
      expect(entry.licenseUrl, entry.id).toMatch(/^https:\/\//);
    }
  });

  it('stellt die nennungspflichtigen Eintraege nach vorn', () => {
    const order = sortEntries(CREDITS.entries).map((entry) => entry.required);
    const firstOptional = order.indexOf(false);
    if (firstOptional >= 0) {
      expect(order.slice(firstOptional)).not.toContain(true);
    }
  });
});

describe('validateCredits', () => {
  const good: CreditsData = {
    note: 'Test',
    entries: [
      {
        id: 'a',
        work: 'Icons',
        author: 'Lorc',
        attribution: 'Icons made by Lorc',
        license: 'CC BY 3.0',
        licenseUrl: 'https://creativecommons.org/licenses/by/3.0/',
        source: 'https://game-icons.net',
        required: true,
        covers: ['assets/img/icons/lorc/*.svg'],
      },
    ],
  };

  it('laesst einen vollstaendigen Eintrag durch', () => {
    expect(validateCredits(good)).toEqual([]);
  });

  it('meldet einen fehlenden Nennungstext', () => {
    const broken: CreditsData = {
      ...good,
      entries: [{ ...good.entries[0]!, attribution: '' }],
    };
    const problems = validateCredits(broken);
    expect(problems.join(' ')).toContain('attribution');
  });

  it('meldet einen Eintrag, der keine Datei abdeckt', () => {
    const broken: CreditsData = {
      ...good,
      entries: [{ ...good.entries[0]!, covers: [] }],
    };
    expect(validateCredits(broken).join(' ')).toContain('covers');
  });

  it('meldet doppelte Kennungen', () => {
    const broken: CreditsData = {
      ...good,
      entries: [good.entries[0]!, good.entries[0]!],
    };
    expect(validateCredits(broken).join(' ')).toContain('doppelt');
  });

  it('meldet eine leere Liste', () => {
    expect(validateCredits({ note: '', entries: [] }).join(' ')).toContain('keinen Eintrag');
  });
});

describe('Bearbeitungshinweis', () => {
  it('kennzeichnet die zugeschnittenen game-icons-Saetze', () => {
    // Die Symbole wurden zentriert und im Gewicht angeglichen (Issue #20).
    // CC BY verlangt, dass eine Bearbeitung kenntlich gemacht wird.
    const bearbeitet = CREDITS.entries.filter((entry) => entry.id.startsWith('game-icons-'));
    expect(bearbeitet.length).toBe(3);
    for (const entry of bearbeitet) {
      expect(entry.modified, entry.id).toBeTruthy();
    }
  });
});
