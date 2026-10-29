/**
 * Die drei Schwierigkeitsstufen je Boss (GAME-DESIGN 11.7, Issue #1177).
 *
 * > Normal / Heroisch / Mythisch pro Boss — mehr Leben, härteres Zeitlimit,
 * > zusätzliche Mechaniken, mehr Punkte. (Masterplan A.4)
 *
 * # Eine Stufe ist ein abgeleiteter Datensatz, kein Schalter im Kampf
 *
 * Hier entsteht aus einer Bossdatei und ihrer Stufe ein **vollständiger
 * Datensatz** — derselbe, den eine Datei mit diesen Werten von Hand hätte.
 * Der Kampf bekommt ihn über `switchBoss` und rechnet ihn wie jeden anderen;
 * `combat.ts` kennt keine Stufe. Das ist die eine Stelle, an der „mehr Leben“
 * und „zusätzliche Mechanik“ entstehen, und sie liegt vor dem Kampf statt in
 * seinen Takten.
 *
 * **Die Ableitung arbeitet am Rohformat, nicht am gelesenen Boss.** Danach
 * läuft der abgeleitete Datensatz durch `parseBoss` — dieselben Prüfungen wie
 * jede Datei: Raster, Schleifen-Rahmen, Zwei-Wege-Regel der Schwächung. Eine
 * Stufe, die einen Eintrag außerhalb des Rahmens legt, fällt dort auf, nicht
 * erst im Kampf. `sim/build.rs` leitet mit derselben Regel ab und gibt das
 * Ergebnis an denselben Leser (`emit_boss`); dass beide dasselbe ergeben,
 * hält `tests/parity/recordings/difficulty-stages.json` für jeden Boss fest.
 *
 * # Die Regel der Ableitung
 *
 * ```text
 *   Lebenspunkte   floor(hp * hpPermille / 1000)
 *   Zeitlimit      floor(overpressureMs * overpressurePermille / 10000) * 10
 *   Mechaniken     Normal + heroic.add (+ mythic.add) — aufsteigend kumulativ
 * ```
 *
 * - **Kumulativ**: Mythisch trägt, was Heroisch dazubringt, und legt eigenes
 *   darauf. „Mythisch öffnet nach Heroisch“ (12.7) — wer dort ankommt, kennt
 *   die heroische Mechanik schon.
 * - **Nur dazu, nie ersetzt**: Ein Baustein, den der Boss schon trägt, darf
 *   eine Stufe nicht noch einmal setzen. Sonst wäre „zusätzliche Mechanik“
 *   still eine ausgetauschte, und der Spieler träfe auf Heroisch einen
 *   anderen Boss statt einen schwereren.
 * - **Kein `runaway`, kein `bossHeal`**: Das umgekehrte Ziel tauscht Sieg und
 *   Niederlage (11.2, Baustein 6) — eine Stufe, die es setzt, wäre ein anderer
 *   Kampf. Die Heilung durch Fehler ist laut 11.2 „nur für einzelne Bosse,
 *   sonst frustrierend“; als Stufen-Zutat stünde sie bei jedem Boss offen.
 *
 * Spieltexte (die Stufennamen) englisch, Design-Gesetz 5.
 */
import rawDifficulty from '../../data/difficulty.json';
import { DIFFICULTY_COUNT, DIFFICULTY_NAMES, DIFFICULTY_NORMAL } from '../loot/rates.js';

/** Eine Stufe aus `data/difficulty.json`. */
export interface DifficultyStageDef {
  /** `normal`, `heroic`, `mythic` — dieselbe Kennung wie `DIFFICULTY_NAMES`. */
  readonly id: string;
  /** Anzeigename. Englisch. */
  readonly name: string;
  /** Lebens-Faktor in Promille. **Vorläufig** (siehe die Datei). */
  readonly hpPermille: number;
  /** Zeitlimit-Faktor in Promille. **Vorläufig.** */
  readonly overpressurePermille: number;
  /** Punkte-Faktor in Promille. **Vorläufig.** */
  readonly pointsPermille: number;
}

/**
 * Die Bausteine, die eine Stufe einem Boss hinzufügen darf.
 *
 * Die Zeitleiste steht nicht darin: Sie wird nicht gesetzt, sondern ergänzt.
 * Was fehlt, fehlt mit Grund — siehe den Dateikopf (`runaway`, `bossHeal`)
 * und `phases`: Eine Phase, die erst auf Heroisch entsteht, verschöbe die
 * Phasenbindung jedes Zeitleisten-Eintrags, der `phase` trägt.
 */
export const ADDABLE_BLOCKS: readonly string[] = [
  'energy',
  'doom',
  'stacks',
  'race',
  'affliction',
  'escalation',
];

/** Das Raster der Zeitwerte, wie `RASTER_MS` in `sim/build.rs`. */
const RASTER_MS = 10;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function positiveInteger(value: unknown): number | null {
  return typeof value === 'number' && Number.isInteger(value) && value > 0 ? value : null;
}

/**
 * Die drei Stufen, in ihrer Reihenfolge.
 *
 * **Harter Abbruch statt `problems`-Kanal**, wie bei `instances.json`: Das ist
 * keine Inhaltsdatei, sondern eine Voraussetzung jedes Bosses. Eine Stufe mit
 * falscher Kennung oder ein Faktor von 0 machte jede Ableitung falsch, und ein
 * Abbruch beim Laden ist ehrlicher als 68 Bosse mit null Lebenspunkten.
 */
export const DIFFICULTY_STAGES: readonly DifficultyStageDef[] = (() => {
  const roh = rawDifficulty as { stages?: unknown[] };
  const stufen = roh.stages ?? [];
  if (stufen.length !== DIFFICULTY_COUNT) {
    throw new Error(
      `data/difficulty.json: ${String(stufen.length)} Stufen, erwartet ${String(DIFFICULTY_COUNT)} ` +
        `(src/loot/rates.ts, DIFFICULTY_COUNT).`,
    );
  }
  return stufen.map((eintrag, index) => {
    if (!isRecord(eintrag)) {
      throw new Error(`data/difficulty.json: Stufe ${String(index)} ist kein Objekt.`);
    }
    const id = eintrag['id'];
    if (typeof id !== 'string' || id !== DIFFICULTY_NAMES[index]) {
      throw new Error(
        `data/difficulty.json: Stufe ${String(index)} heißt "${String(id)}", erwartet ` +
          `"${String(DIFFICULTY_NAMES[index])}" — die Reihenfolge ist die Stufennummer.`,
      );
    }
    const zahl = (feld: string): number => {
      const wert = positiveInteger(eintrag[feld]);
      if (wert === null) {
        throw new Error(`data/difficulty.json: "${id}" braucht ${feld} als positive ganze Zahl.`);
      }
      return wert;
    };
    return {
      id,
      name: typeof eintrag['name'] === 'string' ? eintrag['name'] : id,
      hpPermille: zahl('hpPermille'),
      overpressurePermille: zahl('overpressurePermille'),
      pointsPermille: zahl('pointsPermille'),
    };
  });
})();

/** Eine Stufennummer auf den gültigen Bereich geklemmt — wie `itemLevelOf`. */
export function clampDifficulty(difficulty: number): number {
  if (!Number.isInteger(difficulty)) return DIFFICULTY_NORMAL;
  return Math.min(Math.max(difficulty, DIFFICULTY_NORMAL), DIFFICULTY_COUNT - 1);
}

/** Die Stufe eines Datensatzes. Fehlt das Feld, ist es Normal. */
export function difficultyOf(boss: { readonly difficulty?: number }): number {
  return clampDifficulty(boss.difficulty ?? DIFFICULTY_NORMAL);
}

/** Der Anzeigename einer Stufe. Englisch. */
export function difficultyName(difficulty: number): string {
  return DIFFICULTY_STAGES[clampDifficulty(difficulty)]?.name ?? 'Normal';
}

/**
 * Die Überschrift eines Bosses mit seiner Stufe.
 *
 * Normal ohne Zusatz: Das ist der Boss, wie der Spieler ihn kennt. Eine Stufe
 * darüber steht dahinter, damit HUD und Vorbereitung dasselbe sagen — und
 * damit ein Wechsel auf Heroisch nicht aussieht wie gar kein Wechsel.
 */
export function bossTitle(boss: { readonly name: string; readonly difficulty?: number }): string {
  const stufe = difficultyOf(boss);
  return stufe === DIFFICULTY_NORMAL ? boss.name : `${boss.name} · ${difficultyName(stufe)}`;
}

/** Der Punkte-Faktor einer Stufe in Promille (Masterplan A.4: „mehr Punkte“). */
export function pointsPermilleOf(difficulty: number): number {
  return DIFFICULTY_STAGES[clampDifficulty(difficulty)]?.pointsPermille ?? 1000;
}

/** Lebenspunkte nach Faktor — ganzzahlig, abgerundet (18.4). */
export function scaledHp(hp: number, permille: number): number {
  return Math.floor((hp * permille) / 1000);
}

/** Zeitlimit nach Faktor — abgerundet auf das 10-ms-Raster. */
export function scaledOverpressure(overpressureMs: number, permille: number): number {
  return Math.floor((overpressureMs * permille) / (1000 * RASTER_MS)) * RASTER_MS;
}

/** Was eine Ableitung ergibt. */
export interface DerivedStage {
  /** Der abgeleitete Datensatz im Rohformat, ohne `difficulties`. */
  readonly raw: Record<string, unknown>;
  /** Leer heißt: alles in Ordnung. */
  readonly problems: readonly string[];
}

function atMsOf(entry: unknown): number {
  return isRecord(entry) && typeof entry['atMs'] === 'number' ? entry['atMs'] : 0;
}

/**
 * Den Datensatz einer Stufe aus der Bossdatei ableiten.
 *
 * Stufe 0 ist die Datei selbst, ohne den Stufen-Block. Darüber kommen die
 * Zutaten aller Stufen bis zur verlangten dazu, und Leben und Zeitlimit folgen
 * dem Faktor der verlangten Stufe — aus `data/difficulty.json`, oder aus dem
 * Boss, wenn er ihn für diese Stufe überschreibt.
 *
 * **Die Zeitleiste wird nach `atMs` sortiert, stabil.** Bei gleichem Zeitpunkt
 * steht der ältere Eintrag vorn: Normal vor Heroisch vor Mythisch. Dieselbe
 * Reihenfolge ergibt `sort_by_key` in `sim/build.rs` — beide sind stabil.
 */
export function deriveStage(raw: unknown, difficulty: number): DerivedStage {
  const problems: string[] = [];
  const quelle = isRecord(raw) ? raw : {};
  const stufe = clampDifficulty(difficulty);
  const ergebnis: Record<string, unknown> = {};
  for (const [schluessel, wert] of Object.entries(quelle)) {
    if (schluessel !== 'difficulties') ergebnis[schluessel] = wert;
  }
  ergebnis['difficulty'] = stufe;
  if (stufe === DIFFICULTY_NORMAL) return { raw: ergebnis, problems };

  const bloecke = quelle['difficulties'];
  if (!isRecord(bloecke)) {
    problems.push(
      `Der Boss hat keine Stufen ("difficulties" fehlt), verlangt war ${difficultyName(stufe)}.`,
    );
    return { raw: ergebnis, problems };
  }

  let zeitleiste: unknown[] = Array.isArray(quelle['timeline']) ? [...quelle['timeline']] : [];
  for (let s = 1; s <= stufe; s++) {
    const id = DIFFICULTY_NAMES[s] ?? '';
    const block = bloecke[id];
    if (!isRecord(block)) {
      problems.push(`Die Stufe "${id}" fehlt in "difficulties".`);
      continue;
    }
    const zutaten = block['add'];
    if (!isRecord(zutaten)) {
      problems.push(`Die Stufe "${id}" hat kein "add" — sie brächte keine Mechanik dazu.`);
      continue;
    }
    for (const [schluessel, wert] of Object.entries(zutaten)) {
      if (schluessel.startsWith('_')) continue;
      if (schluessel === 'timeline') {
        if (!Array.isArray(wert)) {
          problems.push(`Stufe "${id}": "timeline" ist keine Liste.`);
          continue;
        }
        zeitleiste = [...zeitleiste, ...(wert as unknown[])];
        continue;
      }
      if (!ADDABLE_BLOCKS.includes(schluessel)) {
        problems.push(
          `Stufe "${id}": "${schluessel}" kann keine Stufe hinzufügen (erlaubt: timeline, ` +
            `${ADDABLE_BLOCKS.join(', ')}).`,
        );
        continue;
      }
      const vorhanden = ergebnis[schluessel];
      if (vorhanden !== undefined && vorhanden !== null) {
        problems.push(
          `Stufe "${id}": "${schluessel}" trägt der Boss schon — eine Stufe fügt hinzu, sie ersetzt nicht.`,
        );
        continue;
      }
      ergebnis[schluessel] = wert;
    }
  }
  ergebnis['timeline'] = zeitleiste
    .map((eintrag, index) => ({ eintrag, index }))
    .sort((a, b) => atMsOf(a.eintrag) - atMsOf(b.eintrag) || a.index - b.index)
    .map((x) => x.eintrag);

  const ziel = DIFFICULTY_STAGES[stufe];
  const eigen = bloecke[DIFFICULTY_NAMES[stufe] ?? ''];
  const faktor = (feld: 'hpPermille' | 'overpressurePermille'): number => {
    const ueberschrieben = isRecord(eigen) ? eigen[feld] : undefined;
    if (ueberschrieben === undefined) return ziel?.[feld] ?? 1000;
    const wert = positiveInteger(ueberschrieben);
    if (wert === null) {
      problems.push(`Stufe "${String(ziel?.id)}": ${feld} ist keine positive ganze Zahl.`);
      return ziel?.[feld] ?? 1000;
    }
    return wert;
  };
  const hp = quelle['hp'];
  const zeit = quelle['overpressureMs'];
  if (typeof hp === 'number') ergebnis['hp'] = scaledHp(hp, faktor('hpPermille'));
  if (typeof zeit === 'number') {
    ergebnis['overpressureMs'] = scaledOverpressure(zeit, faktor('overpressurePermille'));
  }
  return { raw: ergebnis, problems };
}
