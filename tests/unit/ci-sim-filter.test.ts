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

/**
 * Der Abschnitt des `sim`-Jobs, roh — von seiner Kopfzeile bis zum nächsten Job.
 *
 * Text statt YAML-Baum: Das Projekt führt keinen YAML-Leser. Die Jobs stehen in
 * `ci.yml` als Schlüssel mit genau zwei Leerzeichen Einzug, ihre Schritte
 * tiefer; die Trennkommentare zwischen den Jobs beginnen mit `#`.
 */
function simJob(): string {
  const zeilen = ciYaml.split('\n');
  const anfang = zeilen.findIndex((z) => z === '  sim:');
  expect(
    anfang,
    'im Workflow steht kein Job "sim" mehr — dann belegt diese Prüfung nichts',
  ).toBeGreaterThan(-1);
  const rest = zeilen.slice(anfang + 1);
  const ende = rest.findIndex((z) => /^ {2}[a-z][\w-]*:\s*$/.test(z));
  const abschnitt = (ende === -1 ? rest : rest.slice(0, ende)).join('\n');
  // **Zusicherung in den Datensatz.** Träfe die Suche den falschen Abschnitt,
  // fände sie dort keine Werkzeuge, und der Fall darunter wäre grün.
  expect(abschnitt, 'der gelesene Abschnitt ist nicht der Rust-Job').toContain('cargo test');
  return abschnitt;
}

/**
 * Die Zeilen des `sim`-Jobs, die GitHub ausführt: ohne Kommentarzeilen.
 *
 * **Eine Regel für alle Leser dieses Abschnitts.** Ein auskommentierter Schritt
 * läuft nicht; wer ihn als Text findet, hält ihn für angeschlossen. Bis #892
 * filterte nur die Werkzeugsuche Kommentare heraus, die Prüfung des Schritts
 * nicht — und blieb grün, als der Schritt auskommentiert war.
 */
function simZeilen(): string[] {
  return simJob()
    .split('\n')
    .filter((zeile) => !/^\s*#/.test(zeile));
}

/** Die Werkzeuge unter `tools/`, die der `sim`-Job selbst aufruft — Kommentare zählen nicht. */
function simWerkzeuge(): string[] {
  const gefunden = new Set<string>();
  for (const zeile of simZeilen()) {
    for (const treffer of zeile.matchAll(/\btools\/[\w./-]+\.(?:py|sh|mjs)\b/g)) {
      gefunden.add(treffer[0]);
    }
  }
  return [...gefunden].sort();
}

/**
 * Die Schritte des `sim`-Jobs, je als Block ihrer Zeilen: von `- ` bis vor den
 * nächsten Schritt.
 *
 * Text statt YAML-Baum, siehe `simJob()`. Die Schritte stehen mit sechs
 * Leerzeichen Einzug und `- `, ihre Schlüssel mit acht.
 */
function simSchritte(): string[][] {
  const schritte: string[][] = [];
  for (const zeile of simZeilen()) {
    if (zeile.trim() === '') continue;
    if (/^ {6}- /.test(zeile)) schritte.push([zeile]);
    else if (schritte.length > 0) schritte[schritte.length - 1]!.push(zeile);
  }
  return schritte;
}

/** Schlüssel und Wert je Schlüsselzeile eines Schritts — tiefer Eingerücktes ist Wert, kein Schlüssel. */
function schluessel(schritt: string[]): { name: string; wert: string }[] {
  return schritt.flatMap((zeile) => {
    const treffer = /^(?: {6}- | {8})([\w-]+):(.*)$/.exec(zeile);
    return treffer ? [{ name: treffer[1]!, wert: treffer[2]!.trim() }] : [];
  });
}

/**
 * Schlüssel, die einen Schritt weder überspringen noch seinen Ausgang
 * verschlucken können. **Eine Zulassung, keine Sperrliste:** `if` und
 * `continue-on-error` schalten ab, eine eigene `shell` kann den Ausgang
 * verschlucken — und eine Sperrliste fängt nur, woran ihr Verfasser gedacht
 * hat. Ein neuer Schlüssel, der nichts davon kann, gehört hier dazu.
 */
const HARMLOSE_SCHLUESSEL = new Set(['name', 'id', 'run', 'timeout-minutes']);

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

  it('fasst jedes Werkzeug, das die sim-Stufe selbst ausführt, als Eingabe auf (#892)', () => {
    // Dieselbe Klasse wie `ci.yml` im Muster, eine Ebene tiefer: Ein Werkzeug,
    // das der sim-Job ausführt, IST Teil der Prüfung. Fällt es durch den Filter,
    // ändert jemand die Prüfung, die Stufe läuft nicht, und ci-pass meldet grün.
    // Die Liste kommt aus dem Job selbst, nicht aus einer Abschrift.
    const muster = simMuster();
    const werkzeuge = simWerkzeuge();
    expect(
      werkzeuge.length,
      'der sim-Job ruft kein einziges Werkzeug unter tools/ auf — dann sucht diese Prüfung an der falschen Stelle',
    ).toBeGreaterThan(0);
    const fehlend = werkzeuge.filter((w) => !muster.test(w));
    expect(
      fehlend,
      'diese Werkzeuge fährt der sim-Job selbst, sie fallen aber durch den sim-Filter — ' +
        'eine Änderung an ihnen ließe genau die Stufe nicht laufen, die sie benutzt:\n  ' +
        fehlend.join('\n  '),
    ).toEqual([]);
  });

  it('führt die Baubruch-Gegenprobe der Wirkungsbeschreibung im sim-Job aus (#892)', () => {
    // Die Abnahme aus Epic #892: Ein neuer Effekt-Baustein bricht den Bau an
    // jeder registrierten Analyse — nachgewiesen bei JEDER sim-Änderung mit dem
    // echten Übersetzer, nicht einmal im PR-Text. Ohne diesen Schritt wäre das
    // Werkzeug eine Leitung ohne Anschluss.
    //
    // **Angeschlossen heißt: Er läuft, und sein Scheitern bricht den Job.** Ein
    // Text im Abschnitt belegt das nicht. Die erste Fassung dieses Falls suchte
    // nur die Befehlszeile und blieb grün, als der Schritt auskommentiert, mit
    // `if: false` übersprungen, mit `continue-on-error: true` entschärft oder
    // sein Ausgang mit `|| true` verschluckt war (gemessen bei der Prüfung von
    // PR #1136). Ein Abschalten des GANZEN Jobs fängt dieser Fall nicht: Es
    // träfe jeden Schritt der Stufe gleich.
    const NAME = 'Wirkungs-Baubruch-Gegenprobe';
    const BEFEHL = 'python3 -B tools/wirkung-baubruch-gegenprobe.py --befehl cargo';
    const schritte = simSchritte();
    // **Zusicherung in den Datensatz.** Zerlegte die Suche den Job falsch, fände
    // sie den Schritt nicht und meldete „fehlt“ statt „falsch gelesen“.
    expect(
      schritte.filter((s) =>
        schluessel(s).some((k) => k.name === 'run' && k.wert === 'cargo test'),
      ),
      'die Zerlegung in Schritte findet nicht einmal `run: cargo test` — dann liest sie den Job falsch',
    ).toHaveLength(1);

    const treffer = schritte.filter((s) =>
      schluessel(s).some((k) => k.name === 'name' && k.wert === NAME),
    );
    expect(
      treffer.length,
      `im sim-Job steht der Schritt „${NAME}“ ${treffer.length}-mal (Kommentarzeilen nicht ` +
        'mitgezählt), erwartet genau einmal — auskommentiert läuft er nicht, und bei zweien ' +
        'belegt dieser Fall nicht, welcher läuft',
    ).toBe(1);
    const schritt = schluessel(treffer[0]!);

    const unzulaessig = schritt.map((k) => k.name).filter((k) => !HARMLOSE_SCHLUESSEL.has(k));
    expect(
      unzulaessig,
      `„${NAME}“ trägt Schlüssel außerhalb der Zulassung — \`if\` und \`continue-on-error\` ` +
        'schalten ihn ab, `shell` kann sein Scheitern verschlucken; zugelassen sind nur ' +
        `${[...HARMLOSE_SCHLUESSEL].join(', ')}. Ein Schlüssel, der nichts davon kann, gehört ` +
        'in HARMLOSE_SCHLUESSEL',
    ).toEqual([]);

    // Der Wert verankert bis zum Zeilenende: `… --befehl cargo || true` ist ein
    // anderer Befehl, und ein Block-Skalar (`run: |`) könnte einen solchen Zusatz tragen.
    const befehle = schritt.filter((k) => k.name === 'run').map((k) => k.wert);
    expect(
      befehle,
      `„${NAME}“ muss genau \`${BEFEHL}\` ausführen, als eine Zeile ohne Zusatz — ` +
        `gefunden: ${JSON.stringify(befehle)}`,
    ).toEqual([BEFEHL]);
  });
});
