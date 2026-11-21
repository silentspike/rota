/**
 * Die Prüfungen im Archiv-Parser (Issue #85, Befund aus #809).
 *
 * # Warum diese Datei nachträglich entstand
 *
 * `parseEntries` wurde in #808 mit dieser Begründung exportiert:
 *
 *     „Als eigene Funktion, damit die Prüfungen prüfbar sind."
 *
 * Gemessen beim Aufräumen für #809: **kein einziger Fall rief sie auf.** Der
 * Export war begründet, die Begründung unerfüllt — eine eigene Spielart der
 * Klasse „gebaut, grün, unerreichbar": nicht Code ohne Aufrufer, sondern ein
 * Versprechen im Kommentar, das nie eingelöst wurde.
 *
 * Diese Datei löst es ein. Jede Prüfung im Parser bekommt einen Fall, der sie
 * auslöst — sonst stünde dort eine Wand aus `throw`, von der niemand weiss,
 * ob sie greift.
 */

import { describe, expect, it } from 'vitest';
import { parseEntries } from '../../src/analysis/archive-data.js';

/** Ein gültiger Rohling; jeder Fall verdirbt genau ein Feld. */
function roh(over: Record<string, unknown> = {}) {
  return {
    id: 'test-x',
    instanceId: 'instance-1',
    trigger: 'beforeInstance',
    voice: 'ownNotes',
    title: 'Titel',
    body: 'Text',
    ...over,
  };
}

describe('AC1 — was durchgeht', () => {
  it('ein gültiger Eintrag kommt vollständig an', () => {
    const [eintrag] = parseEntries([roh()]);
    expect(eintrag?.id).toBe('test-x');
    expect(eintrag?.voice).toBe('ownNotes');
    expect(eintrag?.bossId).toBeUndefined();
  });

  it('und einer mit Boss trägt ihn mit', () => {
    const [eintrag] = parseEntries([roh({ trigger: 'afterFirstKill', bossId: 'boss-1-1' })]);
    expect(eintrag?.bossId).toBe('boss-1-1');
  });
});

describe('AC2 — jede Prüfung greift, und sie nennt den Eintrag', () => {
  it('keine Liste', () => {
    expect(() => parseEntries('keine Liste')).toThrow(/keine Liste/);
  });

  it('Eintrag ohne Kennung — er wäre nie freischaltbar', () => {
    expect(() => parseEntries([roh({ id: '' })])).toThrow(/ohne Kennung/);
  });

  it('doppelte Kennung — beide gälten als gesehen', () => {
    // Dieselbe Klasse wie die Satz-Kennungen in #780: `unlock` merkt sich die
    // Kennung, nicht den Eintrag. Der zweite erschiene nie.
    expect(() => parseEntries([roh(), roh()])).toThrow(/kommt zweimal vor/);
  });

  it('unbekannter Auslöser', () => {
    expect(() => parseEntries([roh({ trigger: 'irgendwann' })])).toThrow(/Auslöser/);
  });

  it('unbekannte Stimme', () => {
    expect(() => parseEntries([roh({ voice: 'gesang' })])).toThrow(/Stimme/);
  });

  it('keine Instanz — die Ansicht gliedert danach', () => {
    expect(() => parseEntries([roh({ instanceId: '' })])).toThrow(/keine Instanz/);
  });

  it('`afterFirstKill` ohne Boss — er erschiene nie', () => {
    // Der stillste der Fälle: `nextAutomaticEntry` verglicht mit einer leeren
    // Kennung, der Vergleich träfe nie zu, und der Eintrag bliebe für immer
    // verborgen — ohne Fehler, ohne Hinweis.
    expect(() => parseEntries([roh({ trigger: 'afterFirstKill' })])).toThrow(/keinen Boss/);
  });

  it('ohne Titel oder ohne Text', () => {
    expect(() => parseEntries([roh({ title: '' })])).toThrow(/Titel oder/);
    expect(() => parseEntries([roh({ body: '' })])).toThrow(/Titel oder/);
  });
});

describe('AC3 — ZUSICHERUNG: der gültige Rohling geht wirklich durch', () => {
  it('sonst prüften alle Fälle oben nur, dass irgendetwas wirft', () => {
    expect(() => parseEntries([roh()])).not.toThrow();
    expect(parseEntries([roh()])).toHaveLength(1);
  });
});

describe('#1177 — die neuen Auslöser verlangen ihr Feld, und nur sie', () => {
  it('Zauber, Schule, Satz und Fund kommen ohne Instanz durch', () => {
    // Sie hängen an keiner Instanz; eine Pflicht-Instanz wäre eine erfundene
    // Zuordnung. Gegliedert wird nach Kapitel (`ARCHIVE_CHAPTERS`).
    const [zauber, schule, satz, fund] = parseEntries([
      roh({ id: 'a', trigger: 'spellLearned', instanceId: undefined, spellId: 'kindle' }),
      roh({ id: 'b', trigger: 'schoolLearned', instanceId: undefined, school: 'CALCINATION' }),
      roh({ id: 'c', trigger: 'setFound', instanceId: undefined, setId: 2 }),
      roh({ id: 'd', trigger: 'firstOutcome', instanceId: undefined, outcome: 'death' }),
    ]);
    expect(zauber?.spellId).toBe('kindle');
    expect(zauber?.instanceId).toBeUndefined();
    expect(schule?.school).toBe('CALCINATION');
    expect(satz?.setId).toBe(2);
    expect(fund?.outcome).toBe('death');
  });

  it('ohne ihr Feld erschienen sie nie — und werfen deshalb', () => {
    expect(() => parseEntries([roh({ trigger: 'spellLearned' })])).toThrow(/spellId/);
    expect(() => parseEntries([roh({ trigger: 'schoolLearned' })])).toThrow(/school/);
    expect(() => parseEntries([roh({ trigger: 'setFound' })])).toThrow(/Satz/);
    expect(() => parseEntries([roh({ trigger: 'firstOutcome', outcome: 'sieg' })])).toThrow(
      /Ausgang/,
    );
  });

  it('die zwei Auslöser an einer Instanz verlangen sie weiter', () => {
    expect(() => parseEntries([roh({ instanceId: undefined })])).toThrow(/keine Instanz/);
    expect(() =>
      parseEntries([roh({ trigger: 'afterFirstKill', bossId: 'boss-1-1', instanceId: '' })]),
    ).toThrow(/keine Instanz/);
  });
});
