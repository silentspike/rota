/**
 * Kennt der Simulator jedes Feld, das das Spiel liest? (Issue #492)
 *
 * # Warum es diesen Wächter braucht — die Bauart, nicht der Einzelfall
 *
 * Die Rust-Seite wird gegen die **heutigen Daten** geschrieben, nicht gegen
 * das Schema. Solange ein Feld nur einen Wert hat, ist seine Auslassung
 * unsichtbar: Beide Seiten tun dasselbe, aus verschiedenen Gründen. Der erste
 * Datensatz mit dem zweiten Wert bricht dann die Parität — und bis dahin ist
 * jeder Prüffall grün.
 *
 * # DREI FÄLLE, UND NUR EINER IST EIN MANGEL
 *
 * Ein Feld aus dem TypeScript-Schema kann auf drei Weisen behandelt sein:
 *
 * | | |
 * | --- | --- |
 * | in `sim/src/data.rs` | steht in der Struktur, wird zur Laufzeit gerechnet |
 * | in `sim/build.rs` | wird beim Erzeugen der Daten **geprüft**: der einzig erlaubte Wert ist erzwungen, ein anderer bricht den Bau ab |
 * | nirgends | **stille Auslassung** — der Fall, für den dieser Wächter da ist |
 *
 * Die mittlere Spalte ist der Grund, warum dieser Wächter zweimal geschrieben
 * werden musste.
 *
 * # DER PFAD-FEHLER, DER IHN FAST FALSCH GEMACHT HÄTTE
 *
 * Issue #492 entstand aus dem Befund, `consumesAll` sei dem Simulator
 * unbekannt. Gemessen wurde:
 *
 * ```
 * grep -rn "consumes_all" sim/src/    ->  kein Treffer
 * ```
 *
 * **`sim/build.rs` liegt in `sim/`, nicht in `sim/src/`.** Dort steht:
 *
 * ```rust
 * let consumes_all = boolean(spender, "consumesAll", &whose);
 * assert!(consumes_all, "consumesAll=false ist nicht umgesetzt; …");
 * ```
 *
 * Das Feld ist also nicht vergessen — es ist bewusst nur in einem Wert
 * erlaubt, und der Bau setzt das durch. Derselbe zu enge Pfad steckte im
 * Issue, in der Nachmessung und in meiner ersten Fassung dieses Wächters, die
 * drei Felder gemeldet hätte, von denen keines fehlt.
 *
 * Ein Wächter, der drei falsche Befunde meldet, ist schlimmer als keiner: Der
 * nächste echte wird dann nicht geglaubt. Deshalb liest dieser hier **beide**
 * Dateien.
 */
import { describe, expect, it } from 'vitest';

/** Die drei Quelltexte, über Vites `?raw` — das Projekt führt keine Node-Typen. */
const TS_QUELLE = import.meta.glob('../../src/core/data.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const RS_STRUKTUR = import.meta.glob('../../sim/src/data.rs', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const RS_ERZEUGER = import.meta.glob('../../sim/build.rs', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

/**
 * Welche TypeScript-Schnittstelle welcher Rust-Struktur entspricht.
 *
 * **Von Hand gepflegt, und das ist Absicht.** Eine Zuordnung über Namensregeln
 * träfe `SpellDef` → `Spell`, verschwiege aber eine neue Schnittstelle, für
 * die es gar keine Rust-Seite gibt. Die Zusicherung unten fängt genau das.
 */
const PAARE: readonly (readonly [string, string])[] = [
  ['SpellDef', 'Spell'],
  ['SpenderDef', 'Spender'],
  ['DotDef', 'Dot'],
  ['BuffDef', 'Buff'],
  ['ShieldDef', 'Shield'],
  ['HealDef', 'Heal'],
  ['ConversionDef', 'Conversion'],
  ['ControlDef', 'Control'],
];

/**
 * Felder, die heute weder in der Struktur noch im Erzeuger vorkommen dürfen —
 * ohne dass jemand entschieden hat, dass das in Ordnung ist.
 *
 * **„Noch nicht entschieden" ist ein Zustand mit Verfallsdatum, „Ausnahme"
 * wäre ein Urteil.** Wer hier einen Eintrag hinzufügt, sagt nicht „das ist
 * egal", sondern „das ist offen" — und der zweite Fall unten wird rot, sobald
 * der Eintrag überholt ist.
 *
 * Sie ist heute **leer**, und das ist die ehrliche Antwort: Alle Felder sind
 * behandelt, die meisten in der Struktur, drei im Erzeuger.
 */
const NOCH_NICHT_ENTSCHIEDEN: Readonly<Record<string, string>> = {};

/** camelCase zu snake_case — die Namensregel zwischen den beiden Seiten. */
function schlange(name: string): string {
  return name.replace(/(?<!^)([A-Z])/g, '_$1').toLowerCase();
}

function tsFelder(quelle: string, name: string): string[] {
  const block = new RegExp(`export interface ${name} \\{([\\s\\S]*?)\\n\\}`).exec(quelle);
  if (block === null) return [];
  return [...(block[1] ?? '').matchAll(/readonly ([a-zA-Z]+)\??:/g)].map((m) => m[1] ?? '');
}

function rsFelder(quelle: string, name: string): string[] {
  const block = new RegExp(`pub struct ${name} \\{([\\s\\S]*?)\\n\\}`).exec(quelle);
  if (block === null) return [];
  return [...(block[1] ?? '').matchAll(/pub ([a-z_0-9]+):/g)].map((m) => m[1] ?? '');
}

/**
 * Ob die Struktur das Feld führt — auch in abgeflachter Form.
 *
 * Rust flacht ab, wo TypeScript verschachtelt: `decay: { stacks }` wird zu
 * `decay_stacks`, `synergy: { partner, windowGcds }` zu `synergy_partner` und
 * `synergy_window_gcds`. Das ist eine andere Form, kein Mangel.
 */
function inStruktur(rust: readonly string[], feld: string): boolean {
  const gesucht = schlange(feld);
  return rust.some((r) => r === gesucht || r.startsWith(`${gesucht}_`));
}

/**
 * Ob der Erzeuger das Feld liest.
 *
 * `sim/build.rs` liest die JSON-Schlüssel in **camelCase**, so wie sie in den
 * Datendateien stehen — nicht in snake_case. Gesucht wird deshalb nach dem
 * Namen in Anführungszeichen, wie er im Aufruf steht:
 * `boolean(spender, "consumesAll", &whose)`.
 */
function imErzeuger(quelle: string, feld: string): boolean {
  return quelle.includes(`"${feld}"`);
}

const ts = Object.values(TS_QUELLE)[0] ?? '';
const struktur = Object.values(RS_STRUKTUR)[0] ?? '';
const erzeuger = Object.values(RS_ERZEUGER)[0] ?? '';

/** Was auf KEINER der beiden Rust-Seiten vorkommt. */
function stilleAuslassungen(): string[] {
  const gefunden: string[] = [];
  for (const [tsName, rsName] of PAARE) {
    const rust = rsFelder(struktur, rsName);
    for (const feld of tsFelder(ts, tsName)) {
      if (inStruktur(rust, feld)) continue;
      if (imErzeuger(erzeuger, feld)) continue;
      gefunden.push(`${tsName}.${feld}`);
    }
  }
  return gefunden;
}

describe('Die Zusicherungen — ohne sie prüft der Wächter nichts', () => {
  it('alle drei Quelltexte sind lesbar', () => {
    // Ein leerer Quelltext ergäbe leere Listen und damit einen Wächter, der
    // immer grün ist — genau die Falle, gegen die er gebaut ist.
    expect(ts.length, 'src/core/data.ts ist nicht lesbar').toBeGreaterThan(1000);
    expect(struktur.length, 'sim/src/data.rs ist nicht lesbar').toBeGreaterThan(1000);
    expect(erzeuger.length, 'sim/build.rs ist nicht lesbar').toBeGreaterThan(1000);
  });

  it('und die Schnittstellen werden wirklich gefunden', () => {
    // Ein Tippfehler im Namen ergäbe eine leere Feldliste — grün, ohne zu
    // prüfen. Und eine neue TS-Schnittstelle ohne Rust-Seite fällt hier auf.
    for (const [tsName, rsName] of PAARE) {
      expect(
        tsFelder(ts, tsName).length,
        `TS-Schnittstelle ${tsName} nicht gefunden`,
      ).toBeGreaterThan(0);
      expect(
        rsFelder(struktur, rsName).length,
        `Rust-Struktur ${rsName} nicht gefunden`,
      ).toBeGreaterThan(0);
    }
  });

  it('die Abflachungs-Regel greift — decay und synergy gelten als bekannt', () => {
    // **Die Gegenprobe zur Regel selbst.** Ohne sie meldete der Wächter zwei
    // Felder, denen nichts fehlt, und ein echter Fall ginge darin unter.
    const rust = rsFelder(struktur, 'Spell');
    expect(inStruktur(rust, 'decay'), 'decay_stacks existiert').toBe(true);
    expect(inStruktur(rust, 'synergy'), 'synergy_partner existiert').toBe(true);
    expect(inStruktur(rust, 'gibtEsNicht')).toBe(false);
  });

  it('und der Erzeuger-Blick findet, was er finden soll', () => {
    // **Die Gegenprobe zur zweiten Regel.** Sie ist der Grund, warum dieser
    // Wächter zweimal geschrieben werden musste: Ohne den Blick in build.rs
    // meldete er `consumesAll`, `resource` und `refreshMode` als Mängel — und
    // alle drei sind dort geprüft.
    expect(imErzeuger(erzeuger, 'consumesAll'), 'build.rs:564 liest es').toBe(true);
    expect(imErzeuger(erzeuger, 'refreshMode'), 'build.rs:519 liest es').toBe(true);
    expect(imErzeuger(erzeuger, 'resource'), 'build.rs:557 liest es').toBe(true);
    expect(imErzeuger(erzeuger, 'gibtEsNicht')).toBe(false);
  });
});

describe('Kein Feld fällt zwischen die beiden Rust-Seiten', () => {
  it('jedes TypeScript-Feld steht in der Struktur ODER wird beim Bauen geprüft', () => {
    const offen = stilleAuslassungen().filter((f) => !(f in NOCH_NICHT_ENTSCHIEDEN));
    expect(
      offen,
      'Diese Felder liest das Spiel, und der Simulator kennt sie auf KEINER ' +
        'Seite:\n' +
        offen.map((f) => `  ${f}`).join('\n') +
        '\n\nHeute ist das folgenlos, solange alle Datensätze denselben Wert ' +
        'tragen — und genau deshalb fällt es nicht auf. Der erste Datensatz ' +
        'mit dem zweiten Wert bricht die Parität in jeder Aufzeichnung, in ' +
        'der der Zauber vorkommt.\n\n' +
        'Drei Wege: das Feld in sim/src/data.rs nachziehen und in fight.rs ' +
        'auswerten; oder in sim/build.rs prüfen, wenn nur ein Wert erlaubt ' +
        'sein soll; oder aus dem TypeScript-Schema entfernen, wenn es nichts ' +
        'abbildet. Ein Eintrag in NOCH_NICHT_ENTSCHIEDEN ist die vierte und ' +
        'braucht eine Entscheidung, keine Notiz.',
    ).toEqual([]);
  });

  it('und die Liste der offenen Fälle ist noch nötig', () => {
    // **Die Selbstabmeldung.** Ein Eintrag, dessen Feld inzwischen behandelt
    // ist, bliebe sonst stehen — und beim nächsten offenen Fall fiele es
    // niemandem auf, weil „da stehen ja schon welche drin".
    const ueberholt = Object.keys(NOCH_NICHT_ENTSCHIEDEN).filter(
      (f) => !stilleAuslassungen().includes(f),
    );
    expect(
      ueberholt,
      'Diese Felder stehen in NOCH_NICHT_ENTSCHIEDEN, sind aber inzwischen ' +
        'behandelt:\n' +
        ueberholt.map((f) => `  ${f} — ${NOCH_NICHT_ENTSCHIEDEN[f] ?? ''}`).join('\n') +
        '\n\nEintrag entfernen.',
    ).toEqual([]);
  });
});
