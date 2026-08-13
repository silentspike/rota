/**
 * Wie viele Boss-Casts die Aufzeichnungen wirklich prüfen (Issue #822).
 *
 * **Woher die Frage kommt.** Eine Mutation am Cast-Zustand (`casting = None`
 * statt `Some`) blieb in `parity_instance2_boss4` grün, während dieselbe Datei
 * bei sieben Schadenspunkten an einem Schlag rot wurde. Acht Erklärungen dafür
 * waren plausibel und alle falsch. Die neunte stimmte: In dieser Aufzeichnung
 * fällt der Unterbrecher-Druck in denselben Takt, in dem der Cast beginnt —
 * `interrupt_boss_cast()` leert das Feld mit `Option::take()`, und die
 * Prüfsumme sieht zu Recht `None`.
 *
 * **Warum daraus ein Prüffall wurde und kein Dokument.** Die Rechnung unten
 * ergab an dem Tag 11 offene, 10 unterbrochene und 17 ungedeckte Cast-Einträge.
 * Als Satz in einem Dokument wäre sie ein Jahr später falsch, ohne dass es
 * jemand merkt — genau die Klasse, die an diesem Tag sechsmal zugeschlagen hat
 * (zuletzt ein `_comment` in `talents.json`, der sechs Tage lang eine gebaute
 * Mechanik für unmöglich erklärte). Als Prüffall altert sie nicht: Sinkt die
 * Deckung, färbt sie.
 *
 * **Was hier NICHT geprüft wird:** ob der Gleichlauf stimmt. Das tun die
 * Paritätsdateien. Hier steht nur, wie viele von ihnen einen laufenden
 * Boss-Zauber überhaupt zu sehen bekommen.
 */

import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

/** Zauber, die einen laufenden Boss-Cast beenden. */
const UNTERBRECHER = new Set(['quench']);

const AUFZEICHNUNGEN = join(process.cwd(), 'tests/parity/recordings');
const BOSSE = join(process.cwd(), 'data/bosses');

type Lage = 'offen' | 'unterbrochen' | 'ungedeckt';

interface Eintrag {
  readonly datei: string;
  readonly beginn: number;
  readonly lage: Lage;
}

function lies(pfad: string): Record<string, unknown> {
  return JSON.parse(readFileSync(pfad, 'utf8')) as Record<string, unknown>;
}

/**
 * Jeder Cast-Eintrag jeder Aufzeichnung mit seiner Lage.
 *
 * `ungedeckt` heisst: Der Cast liegt jenseits der verglichenen Takte.
 * `first_divergence` vergleicht `min(erwartet, erzeugt)` — was darüber
 * hinausgeht, wird **still** nicht verglichen.
 */
function alleEintraege(): Eintrag[] {
  const raus: Eintrag[] = [];
  for (const name of readdirSync(AUFZEICHNUNGEN).filter((n) => n.endsWith('.json'))) {
    const auf = lies(join(AUFZEICHNUNGEN, name));
    const bossId = auf['bossId'] as string | undefined;
    if (bossId === undefined) continue;
    let boss: Record<string, unknown>;
    try {
      boss = lies(join(BOSSE, `${bossId}.json`));
    } catch {
      continue;
    }
    const zeitleiste = (boss['timeline'] ?? []) as Array<Record<string, number>>;
    const casts = zeitleiste.filter((e) => (e['castTimeMs'] ?? 0) > 0);
    if (casts.length === 0) continue;

    const tickMs = (auf['tickMs'] as number | undefined) ?? 10;
    const deckung = ((auf['checksums'] ?? []) as unknown[]).length * tickMs;
    const plaetze = new Set(
      ((auf['bindings'] ?? []) as Array<Record<string, unknown>>)
        .filter((b) => UNTERBRECHER.has(b['spellId'] as string))
        .map((b) => b['slot'] as number),
    );
    const drucke = ((auf['presses'] ?? []) as Array<Record<string, number>>)
      .filter((p) => plaetze.has(p['slot']))
      .map((p) => p['tick'] * tickMs);

    for (const cast of casts) {
      const beginn = cast['atMs'];
      const ende = beginn + (cast['castTimeMs'] ?? 0);
      const lage: Lage =
        beginn >= deckung
          ? 'ungedeckt'
          : drucke.some((t) => t >= beginn && t <= ende)
            ? 'unterbrochen'
            : 'offen';
      raus.push({ datei: name, beginn, lage });
    }
  }
  return raus;
}

describe('#822 — wie viele Boss-Casts die Aufzeichnungen prüfen', () => {
  /**
   * **Träger-Zusicherung.** Alles Weitere zählt Cast-Einträge. Gibt es keine,
   * wäre jede Zahl unten richtig und nichtssagend.
   */
  it('es gibt überhaupt Aufzeichnungen gegen Bosse mit Cast', () => {
    expect(alleEintraege().length).toBeGreaterThan(0);
  });

  /**
   * Der Kern: Mindestens eine Aufzeichnung muss einen Cast OFFEN halten, sonst
   * ist der Cast-Zustand nirgends geprüft.
   *
   * **Das war die eigentliche Frage von #822.** Die Antwort lautete 11 — die
   * Prüfsumme schützt `casting` also, nur nicht in jeder Datei. Fällt diese
   * Zahl je auf null, ist eine Mutation am Cast-Zustand nirgends mehr
   * sichtbar, und niemand merkt es.
   */
  it('mindestens eine Aufzeichnung hält einen Cast offen', () => {
    const offen = alleEintraege().filter((e) => e.lage === 'offen');
    expect(offen.length).toBeGreaterThan(0);
  });

  /**
   * `aether-fenster` ist die Datei, an der die Mutation nachweislich färbt
   * (`cargo test --test parity_aether` mit `casting = None`: ein Fall rot).
   * Sie namentlich festzuhalten, ist mehr wert als eine Gesamtzahl: Fällt
   * ausgerechnet sie weg, bricht dieser Fall statt still weiterzuzählen.
   */
  it('aether-fenster hält seinen Cast offen — dort färbt die Mutation', () => {
    const eintrag = alleEintraege().find((e) => e.datei === 'aether-fenster.json');
    expect(eintrag?.lage).toBe('offen');
  });

  /**
   * Die Gegenrichtung, damit die Rechnung nicht bloss alles „offen" nennt:
   * `instance2-boss-4` unterbricht nachweislich im selben Takt (Druck bei
   * 13000, Cast ab 13000). Wäre auch das „offen", zählte die Rechnung falsch.
   */
  it('instance2-boss-4 wird unterbrochen — die Rechnung unterscheidet', () => {
    const eintrag = alleEintraege().find((e) => e.datei === 'instance2-boss-4.json');
    expect(eintrag?.lage).toBe('unterbrochen');
  });
});
