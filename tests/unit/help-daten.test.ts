/**
 * Die Prüfungen im Hilfe-Parser, geprüft (Issue #811).
 *
 * # Warum es diese Datei gibt
 *
 * `parseHelpEntries` trägt im Kommentar den Satz „als eigene Funktion, damit
 * die Prüfungen prüfbar sind". Bei `parseEntries` (#85) stand derselbe Satz —
 * und die Prüfungen darin waren bis #809 selbst ungeprüft: Der Kommentar
 * beschrieb eine Absicht, keinen Zustand. Diesmal steht die Datei im selben
 * Commit wie der Parser.
 *
 * # Ein Fall je Prüfung, und jeder nennt die Folge
 *
 * Nicht „wirft bei falschen Daten", sondern: was der Spieler bekäme, wenn die
 * Prüfung fehlte. Eine doppelte Kennung ist kein Formfehler — sie macht einen
 * Eintrag unerreichbar, ohne dass irgendwo etwas rot wird.
 */
import { describe, expect, it } from 'vitest';
import {
  ALL_HELP_ENTRIES,
  ALL_INTRODUCTIONS,
  parseHelpEntries,
  parseIntroductions,
  PLAYABLE_HELP_ENTRIES,
  PLAYABLE_INTRODUCTIONS,
} from '../../src/analysis/help-data.js';
import { helpFor, missingSubjects } from '../../src/analysis/help.js';

/** Ein gültiger Roh-Eintrag, den jeder Fall gezielt kaputt macht. */
const roh = (aenderung: Record<string, unknown> = {}): Record<string, unknown> => ({
  id: 'essence',
  subject: 'stat',
  title: 'ESSENCE',
  effect: 'Raises the damage of every spell.',
  ...aenderung,
});

const rohIntro = (aenderung: Record<string, unknown> = {}): Record<string, unknown> => ({
  mechanicId: 'strike',
  title: 'Strike',
  body: 'A test record.',
  ...aenderung,
});

describe('Der Hilfe-Parser lässt nichts durch, was still wäre', () => {
  it('gültige Einträge kommen unverändert an', () => {
    // **Die Zusicherung vor den Gegenproben.** Käme hier nichts an, wären alle
    // Fälle darunter grün, ohne etwas zu prüfen: Ein Parser, der immer wirft,
    // besteht jeden `toThrow`.
    const eintraege = parseHelpEntries([roh()]);
    expect(eintraege).toHaveLength(1);
    expect(eintraege[0]?.id).toBe('essence');
    expect(eintraege[0]?.values, 'ein fehlendes Feld darf nicht als leer ankommen').toBeUndefined();
  });

  it('„entries" ist keine Liste — nichts käme an', () => {
    expect(() => parseHelpEntries({ essence: {} })).toThrow(/keine Liste/);
  });

  it('ein Eintrag ohne Kennung — er wäre nie auffindbar', () => {
    // `helpFor` sucht über die Kennung. Ohne sie gibt es keinen Weg zu ihm.
    expect(() => parseHelpEntries([roh({ id: '' })])).toThrow(/ohne Kennung/);
  });

  it('doppelte Kennung — der zweite Eintrag erschiene nie', () => {
    // `helpFor` nimmt den ERSTEN Treffer. Welcher gewinnt, hinge an der
    // Reihenfolge in der Datei.
    expect(() => parseHelpEntries([roh(), roh()])).toThrow(/kommt zweimal vor/);
  });

  it('unbekannter Bezug — die Deckungsprüfung liefe ins Leere', () => {
    // `missingSubjects` fragt nach 'spell', 'stat' und 'bossAbility'. Ein
    // Tippfehler im Bezug machte den Eintrag für sie unsichtbar.
    expect(() => parseHelpEntries([roh({ subject: 'stats' })])).toThrow(/unbekannten Bezug/);
  });

  it('ohne Wirkung — das Fenster wäre eine leere Überschrift', () => {
    expect(() => parseHelpEntries([roh({ effect: '' })])).toThrow(/keine Wirkung/);
    expect(() => parseHelpEntries([roh({ title: '' })])).toThrow(/keine Überschrift/);
  });

  it('eine Einführung ohne Mechanik — sie käme nie an die Reihe', () => {
    expect(() => parseIntroductions([rohIntro({ mechanicId: '' })])).toThrow(/ohne Mechanik/);
  });

  it('zwei Einführungen zu derselben Mechanik — die zweite gälte als gesehen', () => {
    // `markSeen` vermerkt die MECHANIK, nicht die Einführung. Nach der ersten
    // wäre auch die zweite abgehakt, ohne dass sie jemand gesehen hat.
    expect(() => parseIntroductions([rohIntro(), rohIntro()])).toThrow(/zwei Einführungen/);
  });

  it('eine Einführung ohne Text — ein leeres Fenster hielte den Spieler auf', () => {
    expect(() => parseIntroductions([rohIntro({ body: '' })])).toThrow(/keinen Text/);
  });
});

describe('Die ausgelieferten Daten tragen, was AC2 verlangt', () => {
  it('alle drei Gruppen sind gedeckt — Symbole, Werte, Boss-Fähigkeiten', () => {
    // **Die Gruppe, die man vergisst, ist die dritte.** Sie steht nicht auf
    // der Zauberleiste, wo man beim Bauen hinsieht — deshalb prüft
    // `missingSubjects` sie ausdrücklich mit. Seit #1177 am Katalog, der an
    // den Spieler geht, und weiter an den Prüfdaten.
    expect(missingSubjects(PLAYABLE_HELP_ENTRIES)).toEqual([]);
    expect(missingSubjects(ALL_HELP_ENTRIES)).toEqual([]);
  });

  it('die Kennungen treffen das, woran sie hängen', () => {
    // Ohne diesen Fall wäre die Kupplung eine Behauptung: Die Ansicht setzt
    // `data-help` auf die Kennung des Wertes, der Schule und des Zaubers, und
    // `helpFor` muss dazu etwas finden. Ein Tippfehler bliebe sonst unsichtbar
    // — es erschiene schlicht kein Fenster. Die volle Abdeckung über ALLE
    // gezeigten Kennungen hält `hilfe-abdeckung.test.ts`.
    for (const id of ['essence', 'CALCINATION', 'kindle', 'Kindling Roar']) {
      expect(helpFor(PLAYABLE_HELP_ENTRIES, id), `keine Erklärung zu "${id}"`).not.toBeNull();
      expect(helpFor(ALL_HELP_ENTRIES, id), `keine Prüf-Erklärung zu "${id}"`).not.toBeNull();
    }
  });

  it('es gibt Einführungen, und sie nennen ihre Mechanik', () => {
    expect(PLAYABLE_INTRODUCTIONS.length).toBeGreaterThan(0);
    expect(ALL_INTRODUCTIONS.length).toBeGreaterThan(0);
    for (const intro of [...PLAYABLE_INTRODUCTIONS, ...ALL_INTRODUCTIONS])
      expect(intro.mechanicId, 'eine Einführung ohne Mechanik').not.toBe('');
  });

  it('der Katalog ist gefüllt, und die Prüfdaten stehen getrennt', () => {
    // **Bis #1177 stand hier die Zusage „der Katalog ist heute leer"** — mit
    // dem Vermerk, dieser Fall werde rot, sobald E10 liefert, und dann gehöre
    // `app.ts` auf die gefilterte Liste umgestellt. Beides ist geschehen:
    // `data/help/help.json` liefert, und `app.ts` hängt `PLAYABLE_HELP_ENTRIES`
    // ein (`hilfe-anschluss.test.ts`, „die Fenster tragen die Erklärung aus den
    // Daten").
    expect(PLAYABLE_HELP_ENTRIES.length).toBeGreaterThan(0);
    // Die Prüfdaten gibt es weiter — im Entwicklungs-Bau, in dem auch diese
    // Fälle laufen. Im Auslieferungs-Bau sind sie leer; das misst
    // `tools/auslieferung-pruefdaten.py` am gebauten Stand.
    expect(
      ALL_HELP_ENTRIES.length,
      'die Prüfdaten sind leer — dann zeigt die Prüfseite nichts',
    ).toBeGreaterThan(0);
    // **Nie dieselbe Liste:** Beide tragen „essence"; `helpFor` nähme den
    // ersten Treffer, und welcher es wäre, hinge an der Reihenfolge.
    const pruef = helpFor(ALL_HELP_ENTRIES, 'essence');
    const katalog = helpFor(PLAYABLE_HELP_ENTRIES, 'essence');
    expect(katalog?.effect).not.toBe(pruef?.effect);
  });
});
