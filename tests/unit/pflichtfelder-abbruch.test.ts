/**
 * Fehlt der Schuldatei, dem Startboss oder den Spielerdaten ein Pflichtfeld,
 * bricht das Spiel beim Laden ab und nennt das Feld (Issue #1055).
 *
 * # Warum Abbruch und nicht Verwerfen
 *
 * Ein Zauber oder ein Boss ist ein Datensatz unter vielen; fehlt ihm etwas,
 * verwirft der Lader ihn, und das Spiel läuft ohne ihn weiter
 * (`pflichtfelder-verwerfen.test.ts`). Diese vier Dateien sind das Spiel
 * selbst — ohne Schule, Startboss oder Spieler gibt es nichts weiterzuspielen,
 * und `sim/build.rs` bricht an denselben Stellen ab. Entscheidung des
 * Orchestrators vom 2026-10-29.
 *
 * # Wie
 *
 * Der echte Lader `src/core/data.ts` wird je Fall neu geladen, mit EINER
 * veränderten Datendatei (`vi.doMock`). So läuft genau der Weg, auf dem
 * `CALCINATION`, `FIRST_CRUCIBLE`, `PLAYER`, `LEVELS`, `REWARD`, `INSTANCES`
 * und der Gegenstands-Katalog beim Spielstart entstehen — nicht eine
 * Nachbildung davon. Dieselbe Regel galt für die drei letzten schon vorher
 * („keine Inhaltsdatei, sondern eine Voraussetzung", `src/gear/catalog.ts`).
 */
import { readFileSync } from 'node:fs';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { parseSchool } from '../../src/core/data.js';
import {
  BELOHNUNG_PFLICHT,
  BOSS_PFLICHT,
  DATEI_PFLICHT,
  GEGENSTAND_PFLICHT,
  INSTANZ_PFLICHT,
  SCHUL_PFLICHT,
  SPIELER_PFLICHT,
  STUFEN_PFLICHT,
  type Pflichtfeld,
} from '../../src/core/pflichtfelder.js';

type Roh = Record<string, unknown>;

function lies(datei: string): Roh {
  return JSON.parse(readFileSync(new URL(`../../${datei}`, import.meta.url), 'utf8')) as Roh;
}

/** Das Feld am Pfad streichen; `null`, wenn die Datei es nicht trägt. */
function ohne(roh: Roh, pfad: string): Roh | null {
  const kopie = JSON.parse(JSON.stringify(roh)) as Roh;
  const teile = pfad.split('.');
  let knoten: unknown = kopie;
  for (const teil of teile.slice(0, -1)) {
    const name = teil.replace(/\[\]$/, '');
    const kind = (knoten as Roh | null)?.[name];
    knoten = teil.endsWith('[]') ? (Array.isArray(kind) ? kind[0] : undefined) : kind;
  }
  const blatt = teile.at(-1) as string;
  if (typeof knoten !== 'object' || knoten === null || !(blatt in knoten)) return null;
  delete (knoten as Roh)[blatt];
  return kopie;
}

/** `data.ts` frisch laden, mit `inhalt` an Stelle von `datei`. Der Fehler oder `null`. */
async function ladeMit(datei: string, inhalt: unknown): Promise<Error | null> {
  vi.resetModules();
  vi.doMock(`../../${datei}`, () => ({ default: inhalt }));
  try {
    await import('../../src/core/data.js');
    // `data/items.json` liest der Gegenstands-Katalog, nicht `data.ts`.
    await import('../../src/gear/catalog.js');
    return null;
  } catch (fehler) {
    return fehler as Error;
  } finally {
    vi.doUnmock(`../../${datei}`);
  }
}

afterEach(() => {
  vi.resetModules();
});

const blattVon = (eintrag: Pflichtfeld): string => eintrag.pfad.split('.').at(-1) as string;

/** Je Datei die Einträge, die sie trägt — ein Eintrag ohne Träger wäre ungeprüft. */
function faelle(datei: string, tafel: readonly Pflichtfeld[]) {
  const roh = lies(datei);
  return tafel
    .map((eintrag) => ({ eintrag, gestrichen: ohne(roh, eintrag.pfad) }))
    .filter((fall) => fall.gestrichen !== null)
    .map(
      (fall) => [`${datei}: ${fall.eintrag.pfad}`, datei, fall.eintrag, fall.gestrichen] as const,
    );
}

const ZAUBERDATEIEN = [
  'common',
  'coagulation',
  'fixation',
  'sublimation',
  'putrefaction',
  'test-cast',
  'test-buff',
  'test-channel',
  'test-channel-heal',
  'test-channel-shield',
].map((name) => `data/spells/${name}.json`);

describe('ohne Streichung lädt das Spiel (Positivkontrolle)', () => {
  it('der unveränderte Lader wirft nicht', async () => {
    expect(
      await ladeMit('data/spells/calcination.json', lies('data/spells/calcination.json')),
    ).toBe(null);
  });
});

describe('Abbruch mit Feldnamen, wenn eine der Spiel-Dateien ein Pflichtfeld verliert (#1055)', () => {
  const alle = [
    ...faelle('data/spells/calcination.json', SCHUL_PFLICHT),
    ...ZAUBERDATEIEN.flatMap((datei) => faelle(datei, DATEI_PFLICHT)),
    ...faelle('data/bosses/first-crucible.json', BOSS_PFLICHT),
    ...faelle('data/player/base.json', SPIELER_PFLICHT),
    ...faelle('data/player/levels.json', STUFEN_PFLICHT),
    ...faelle('data/player/reward.json', BELOHNUNG_PFLICHT),
    ...faelle('data/items.json', GEGENSTAND_PFLICHT),
    ...faelle('data/instances.json', INSTANZ_PFLICHT),
  ];

  it('jede Tafel hat in ihrer Datei Träger', () => {
    const geprueft = new Set(alle.map(([, , eintrag]) => eintrag));
    for (const tafel of [
      SCHUL_PFLICHT,
      SPIELER_PFLICHT,
      STUFEN_PFLICHT,
      BELOHNUNG_PFLICHT,
      GEGENSTAND_PFLICHT,
      INSTANZ_PFLICHT,
    ]) {
      expect(tafel.filter((e) => !geprueft.has(e)).map((e) => e.pfad)).toEqual([]);
    }
    // Der Startboss trägt nicht jede Boss-Mechanik; die übrigen Felder prüft
    // `pflichtfelder-verwerfen` an den Bossen, die sie tragen.
    expect(
      alle.filter(([, datei]) => datei.endsWith('first-crucible.json')).length,
    ).toBeGreaterThan(5);
  });

  it.each(alle)('%s', async (_name, datei, eintrag, gestrichen) => {
    const fehler = await ladeMit(datei, gestrichen);
    expect(fehler, `${datei} ohne ${eintrag.pfad} lädt`).toBeInstanceOf(Error);
    expect(fehler?.message).toContain(`Feld "${blattVon(eintrag)}" fehlt`);
    expect(fehler?.message).toContain(datei);
  });
});

describe('parseSchool nennt fehlende Schulfelder beim Namen (#1055)', () => {
  const schule = lies('data/spells/calcination.json');

  it.each(SCHUL_PFLICHT.map((e) => [e.pfad, e] as const))('%s', (_pfad, eintrag) => {
    const gestrichen = ohne(schule, eintrag.pfad);
    expect(gestrichen, `calcination.json trägt ${eintrag.pfad}`).not.toBe(null);
    const gelesen = parseSchool(gestrichen);
    expect(gelesen.problems.some((p) => p.includes(`Feld "${blattVon(eintrag)}" fehlt`))).toBe(
      true,
    );
  });
});
