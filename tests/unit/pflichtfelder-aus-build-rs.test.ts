/**
 * Die Pflichtfelder der TypeScript-Seite sind aus `sim/build.rs` abgeleitet,
 * nicht gepflegt (Issue #1055).
 *
 * Dieser Prüffall liest `build.rs` als Text und sucht jede Stelle, an der ein
 * Feld ZWINGEND gelesen wird. Jede muss in `src/core/pflichtfelder.ts` unter
 * `rust` stehen, so oft, wie sie in `build.rs` vorkommt — und umgekehrt darf
 * dort keine Stelle stehen, die es in `build.rs` nicht gibt. Bekommt `build.rs`
 * ein neues Pflichtfeld, wird dieser Fall rot, bevor das Spiel einen Datensatz
 * weiterliest, den der Simulator ablehnt.
 *
 * # Was als „zwingend" erkannt wird
 *
 * 1. Die Lesehelfer `uint`, `boolean`, `text`, `resource_amount` — jeder
 *    Aufruf, auch über mehrere Zeilen umbrochen. `opt_uint` nicht: Es hat eine
 *    Vorgabe.
 * 2. `.get("feld")`, gefolgt von `.and_then(…)`-Gliedern und
 *    `.unwrap_or_else(|| panic!…)` oder `.expect(…)`.
 * 3. Ein örtlicher Helfer `let name = |feld: &str| { … .get(feld) … panic! … };`
 *    und jeder seiner Aufrufe `name("feld")` — so liest `emit_laufbahn` die
 *    Zahlen von `reward.json` (`gleitkomma`, #922). Die erste Fassung dieses
 *    Prüffalls kannte nur 1 und 2 und sah diese vier Felder nicht.
 *
 * Bedingte Regeln (`overMs` verlangt `tickIntervalMs`) sind keine
 * Pflichtfelder in diesem Sinn: `build.rs` prüft sie mit `assert!`, und die
 * TypeScript-Seite prüft sie bereits mit eigener Meldung.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { ALLE_PFLICHTFELDER } from '../../src/core/pflichtfelder.js';

const BUILD_RS = readFileSync(new URL('../../sim/build.rs', import.meta.url), 'utf8');

/** Die Lesehelfer selbst — ihre Definitionen sind keine Lesestellen. */
const HELFER = new Set(['uint', 'boolean', 'text', 'opt_uint', 'resource_amount']);

/**
 * `read_json_at` liest `schema` aus JEDER Datei, die `build.rs` öffnet. Diese
 * eine Stelle steht deshalb in mehreren Tafeln; für sie genügt „mindestens
 * einmal".
 */
const JEDE_DATEI = new Set(['read_json_at:schema']);

/** Zeilenkommentare entfernen, Zeichenketten unangetastet lassen. */
function ohneKommentare(quelle: string): string {
  let aus = '';
  let inText = false;
  for (let i = 0; i < quelle.length; i++) {
    const zeichen = quelle[i] as string;
    if (inText) {
      aus += zeichen;
      if (zeichen === '\\') {
        aus += quelle[i + 1] ?? '';
        i++;
      } else if (zeichen === '"') inText = false;
      continue;
    }
    if (zeichen === '/' && quelle[i + 1] === '/') {
      while (i < quelle.length && quelle[i] !== '\n') {
        aus += ' ';
        i++;
      }
      aus += '\n';
      continue;
    }
    if (zeichen === '"') inText = true;
    aus += zeichen;
  }
  return aus;
}

/** Jede zwingende Lesestelle als `Funktion:Feld`, mit Zeilennummer. */
function lesestellen(quelle: string): { schluessel: string; zeile: number }[] {
  const text = ohneKommentare(quelle);
  const funktionen = [...text.matchAll(/^(?:pub )?fn (\w+)/gm)].map((m) => ({
    ab: m.index,
    name: m[1] as string,
  }));
  const funktionBei = (stelle: number): string =>
    funktionen.filter((f) => f.ab <= stelle).at(-1)?.name ?? '?';
  const zeileBei = (stelle: number): number => text.slice(0, stelle).split('\n').length;

  const stellen: { schluessel: string; zeile: number }[] = [];
  for (const m of text.matchAll(
    /\b(uint|boolean|text|resource_amount)\(\s*([^,]+?)\s*,\s*(?:"([^"]+)"|([a-z_]+))\s*,/gs,
  )) {
    const funktion = funktionBei(m.index);
    if (HELFER.has(funktion)) continue;
    const feld = m[3] ?? `$${m[4] as string}`;
    stellen.push({ schluessel: `${funktion}:${feld}`, zeile: zeileBei(m.index) });
  }
  // 3. Örtliche Helfer: Kopf `let name = |feld: &str| {`, Rumpf bis zum ersten
  // `};` — darin `.get(feld)` und ein `panic!`. Dann jeder Aufruf `name("…")`
  // in derselben Funktion.
  for (const kopf of text.matchAll(/let (\w+) = \|(\w+): &str\|\s*\{/g)) {
    const [, name, param] = kopf as unknown as [string, string, string];
    const ende = text.indexOf('};', kopf.index);
    const rumpf = text.slice(kopf.index, ende);
    if (!rumpf.includes(`.get(${param})`) || !rumpf.includes('panic!')) continue;
    const funktion = funktionBei(kopf.index);
    const naechste = funktionen.find((f) => f.ab > kopf.index)?.ab ?? text.length;
    for (const aufruf of text
      .slice(ende, naechste)
      .matchAll(new RegExp(`\\b${name}\\("([^"]+)"\\)`, 'g'))) {
      stellen.push({
        schluessel: `${funktion}:${aufruf[1] as string}`,
        zeile: zeileBei(ende + aufruf.index),
      });
    }
  }
  for (const m of text.matchAll(
    /\.get\("([^"]+)"\)(?:\s*\.\s*and_then\([^()]*\))*\s*\.\s*(?:unwrap_or_else\(\s*\|\|\s*\{?\s*panic!|expect\()/g,
  )) {
    stellen.push({
      schluessel: `${funktionBei(m.index)}:${m[1] as string}`,
      zeile: zeileBei(m.index),
    });
  }
  return stellen;
}

function zaehle(schluessel: readonly string[]): Map<string, number> {
  const anzahl = new Map<string, number>();
  for (const s of schluessel) anzahl.set(s, (anzahl.get(s) ?? 0) + 1);
  return anzahl;
}

describe('Pflichtfelder: build.rs gegen src/core/pflichtfelder.ts (#1055)', () => {
  const inBuild = zaehle(lesestellen(BUILD_RS).map((s) => s.schluessel));
  const inTafel = zaehle(ALLE_PFLICHTFELDER.flatMap((tafel) => tafel.flatMap((f) => f.rust)));

  it('die Ableitung findet die Stellen, die es sicher gibt (Positivkontrolle)', () => {
    // Ohne diese Zeilen bestünde der Abgleich auch mit einer Suche, die nichts
    // findet: Eine leere Menge deckt jede Tafel.
    for (const bekannt of [
      'emit_dot:tickIntervalMs', // uint(), eine Zeile
      'emit_spell:baseDamage', // uint(), über drei Zeilen umbrochen
      'emit_buff:factor', // .get().and_then().unwrap_or_else(|| panic!)
      'emit_energy:onTrigger', // .get().unwrap_or_else(|| { panic! … })
      'emit_fixation:$feld', // Feldname als Variable
      'emit_laufbahn:pointsPerSecond', // örtlicher Helfer `gleitkomma("…")`
    ]) {
      expect(inBuild.has(bekannt), bekannt).toBe(true);
    }
    expect([...inBuild.values()].reduce((a, b) => a + b, 0)).toBeGreaterThanOrEqual(90);
  });

  it('jede zwingende Lesestelle aus build.rs steht in der Tafel, so oft wie dort', () => {
    const fehlt: string[] = [];
    for (const [schluessel, anzahl] of inBuild) {
      const getafelt = inTafel.get(schluessel) ?? 0;
      const genug =
        JEDE_DATEI.has(schluessel) || schluessel.includes(':$')
          ? getafelt >= 1
          : getafelt === anzahl;
      if (!genug) fehlt.push(`${schluessel}: build.rs ${anzahl}, Tafel ${getafelt}`);
    }
    expect(fehlt).toEqual([]);
  });

  it('die Tafel nennt keine Stelle, die es in build.rs nicht gibt', () => {
    expect([...inTafel.keys()].filter((s) => !inBuild.has(s))).toEqual([]);
  });
});
