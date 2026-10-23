import { describe, expect, it } from 'vitest';
// Direkt statt per `import.meta.glob`: Das Glob-Muster überspringt
// Punkt-Verzeichnisse, `.github/` bliebe also unsichtbar — und eine Prüfung,
// die ihre Quelle nicht findet, belegt nichts.
import ciYaml from '../../.github/workflows/ci.yml?raw';

/**
 * Der Pfad-Filter der `sim`-Stufe kennt alle Eingaben der Rust-Tests (#173).
 *
 * # Warum diese Datei existiert
 *
 * Das Muster in `.github/workflows/ci.yml` ist eine **von Hand geführte
 * Abschrift** dessen, was die Rust-Tests lesen. Sie war unvollständig: Die
 * dreizehn Vergleichsdateien unter `tests/parity/recordings/` werden per
 * `include_str!` eingebunden, fielen aber durch das Muster. Ein Zweig, der nur
 * eine davon neu erzeugt — ein alltäglicher Vorgang —, übersprang genau die
 * Stufe, die diese Datei prüft.
 *
 * Gemessen: Eine einzelne verfälschte Prüfsumme in `fixation.json` lässt
 * `cargo test --test parity_fixation` mit Exit 101 scheitern. Die Stufe hätte
 * also etwas zu prüfen gehabt.
 *
 * # Warum die Prüfung aus den `include_str!`-Zeilen folgt
 *
 * Eine zweite Liste hier wäre dieselbe Abschrift noch einmal — und die nächste
 * Datei, die jemand einbindet, fehlte in beiden. **Eine Abschrift kann nicht
 * bemerken, dass ihr ein Eintrag fehlt.** Dieselbe Klasse wie #134
 * (Protokoll-Nummernraum), #138 (Spielseite) und #159 (`FACTOR_NAMES`).
 *
 * Deshalb wird hier nichts aufgezählt: Das Muster kommt aus dem Workflow, die
 * Pfade kommen aus den Rust-Quellen, und geprüft wird nur, dass das eine das
 * andere abdeckt.
 *
 * # Warum `import.meta.glob` und nicht `node:fs`
 *
 * Das Projekt führt `@types/node` nicht. Ein Prüffall, der dafür eine
 * Abhängigkeit nachzieht oder mit `@ts-expect-error` am Übersetzer vorbeigeht,
 * wäre teuer bezahlt — Letzteres ist genau die Abschaltung, die in #167 den
 * Schaden angerichtet hat. Vite liest die Dateien roh ein, ist bereits
 * typisiert (`types: ['vite/client']`) und liefert die Pfade als Schlüssel.
 *
 * # Warum ein TypeScript-Prüffall und kein eigener CI-Schritt
 *
 * Der `unit`-Job läuft bei jeder Änderung außer reiner Dokumentation, also
 * auch bei jeder `.rs`- und jeder Workflow-Änderung. Ein eigener Schritt
 * kostete Gate-Zeit und liefe genau dann, wenn dieser hier ohnehin läuft.
 */

/** Die Rust-Quellen, roh. Schlüssel sind Pfade ab der Projektwurzel. */
const RUST_QUELLEN = import.meta.glob('/sim/**/*.rs', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

/**
 * Das Muster der `sim`-Stufe, gelesen aus dem Workflow.
 *
 * **Nicht abgeschrieben.** Wird es dort geändert, ändert sich diese Prüfung
 * mit — sonst prüfte sie eine Fassung, die nicht mehr läuft.
 */
function simMuster(): RegExp {
  expect(ciYaml, 'ci.yml ist nicht lesbar — dann belegt diese Datei nichts').toBeTypeOf('string');
  const treffer = [...ciYaml.matchAll(/echo "sim=\$\(match '([^']+)'\)"/g)];
  // **Ohne diese Zusicherung prüft die Datei nichts.** Findet die Suche das
  // Muster nicht mehr — weil jemand den Workflow umformatiert hat —, wäre
  // `treffer` leer, und ein Prüffall, der über eine leere Liste läuft, ist
  // grün, ohne etwas gesehen zu haben.
  expect(
    treffer,
    `im Workflow steht keine eindeutige sim-Filterzeile mehr (gefunden: ${treffer.length}) — ` +
      'dann kann diese Prüfung das Muster nicht lesen und belegt nichts',
  ).toHaveLength(1);
  return new RegExp(treffer[0]![1]!);
}

/**
 * Löst einen Pfad mit `../`-Anteilen auf. Ohne `node:path`, siehe oben.
 *
 * Die Pfade im Makro sind **relativ zur einbindenden Datei** — `sim/src/apl.rs`
 * mit `../apl/x` meint `sim/apl/x`, `sim/tests/y.rs` mit `../../tests/z` meint
 * `tests/z`. Wer das nicht auflöst, vergleicht Zeichenketten und irrt sich
 * genau dort, wo es darauf ankommt.
 */
function aufloesen(verzeichnis: string, relativ: string): string {
  const teile = verzeichnis.split('/').filter(Boolean);
  for (const stueck of relativ.split('/')) {
    if (stueck === '..') teile.pop();
    else if (stueck !== '.' && stueck !== '') teile.push(stueck);
  }
  return teile.join('/');
}

/** Was die Rust-Quellen per `include_str!` einbinden, als Pfade ab der Wurzel. */
function eingebundeneDateien(): string[] {
  const gefunden = new Set<string>();
  for (const [datei, quelle] of Object.entries(RUST_QUELLEN)) {
    const verzeichnis = datei.slice(0, datei.lastIndexOf('/'));
    for (const treffer of quelle.matchAll(/include_str!\("([^"]+)"\)/g)) {
      gefunden.add(aufloesen(verzeichnis, treffer[1]!));
    }
  }
  return [...gefunden].sort();
}

describe('Der Pfad-Filter der sim-Stufe (#173)', () => {
  it('deckt jede Datei ab, die die Rust-Tests einlesen', () => {
    const muster = simMuster();
    const dateien = eingebundeneDateien();

    // **Die Ursache zuerst.** Fände die Suche keine einzige `include_str!`-
    // Zeile — weil sich das Makro oder die Verzeichnisstruktur geändert hat —,
    // liefe die Schleife unten nicht und der Fall bliebe grün.
    expect(
      dateien.length,
      'in sim/ steht kein einziges include_str! mehr — dann sucht diese ' +
        'Prüfung an der falschen Stelle und belegt nichts',
    ).toBeGreaterThan(5);

    const fehlend = dateien.filter((d) => !muster.test(d));
    expect(
      fehlend,
      'diese Dateien werden von den Rust-Tests gelesen, fallen aber durch den ' +
        'sim-Filter — eine Änderung an ihnen ließe die Rust-Tests nicht laufen:\n  ' +
        fehlend.join('\n  '),
    ).toEqual([]);
  });

  it('lässt reine Dokumentation weiterhin durchfallen', () => {
    // **Der Gegenfall, und er ist der Grund für den Filter überhaupt.** Eine
    // Lösung, die einfach immer läuft, deckt alles ab und zerstört den Zweck:
    // Eine Korrektur an einer Markdown-Datei zahlte dann Rust-Bauzeit.
    const muster = simMuster();
    for (const doku of ['docs/GAME-DESIGN.md', 'docs/WORKER-GUIDE.md', 'README.md', 'CLAUDE.md']) {
      expect(muster.test(doku), `${doku} löst die sim-Stufe aus`).toBe(false);
    }
  });

  it('fasst eine neu erzeugte Vergleichsdatei als Rust-Eingabe auf', () => {
    // Der Fall aus dem Befund: Vergleichsdateien werden nach jeder
    // Regeländerung neu erzeugt, an einem einzigen Tag mehrfach.
    const muster = simMuster();
    expect(muster.test('tests/parity/recordings/fixation.json')).toBe(true);
    expect(muster.test('tests/parity/recordings/log-parity.json')).toBe(true);
  });

  it('fasst den Compiler-Pin als Rust-Eingabe auf', () => {
    // Seit cargo remote v2 (2026-10-23) bestimmt rust-toolchain.toml den
    // Compiler der sim-Stufe. Ein Versionswechsel, der die Stufe nicht
    // auslöst, wäre ein grüner PR für einen Compiler, auf dem nie geprüft
    // wurde. Der Anker zählt: Eine gleichnamige Datei tiefer im Baum ist nicht
    // der Pin des Workspace.
    const muster = simMuster();
    expect(muster.test('rust-toolchain.toml')).toBe(true);
    expect(muster.test('docs/rust-toolchain.toml.md')).toBe(false);
  });
});
