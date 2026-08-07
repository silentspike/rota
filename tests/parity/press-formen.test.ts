/**
 * Eine Aufzeichnung nutzt genau eine Press-Form (Issue #594).
 *
 * # Warum eine Mischung gefährlich ist — und zwar nicht „unsauber"
 *
 * Ein Tastendruck steht in zwei Formen in den Aufzeichnungen:
 *
 * ```text
 *   {slot, tick}       der Normalfall
 *   {spellId, tick}    nur in buff-combat
 * ```
 *
 * **Die Rust-Seite liest je Datei genau eine davon.** `parity_coagulation.rs`
 * und die übrigen greifen `eintrag["slot"]`, `buff_combat.rs` greift
 * `eintrag["spellId"]` und bildet selbst auf Plätze ab. Beides ist umgesetzt,
 * und solange eine Datei nur eine Form führt, ist ein Leser entweder
 * vollständig richtig oder vollständig blind — und blind fällt auf, weil das
 * `expect` bricht.
 *
 * **Gemischt wäre er teilweise blind, und das fällt nicht auf.** Ein
 * `spellId`-Druck in einer `slot`-Datei liefe still ins Leere: Der Simulator
 * spielte einen Druck weniger, das Spiel einen mehr, und die Prüfsumme wiche
 * ab, ohne dass eine Regel anders gerechnet wäre. Gesucht würde der Fehler
 * dann in der Kampflogik.
 *
 * # Der Anlass, und er lag nicht in den Daten
 *
 * An einem Tag entstanden zwei Fehlmessungen aus derselben Klasse:
 *
 * ```text
 *   „anchor, stall, stay werden nie gedrückt"    gemessen wurde nur {slot}
 *   „alle zehn Fixations-Zauber nie gedrückt"    bindings als Namensliste
 * ```
 *
 * Beide sahen aus wie Befunde, die zweite ging bereits als Auftrag weiter.
 * Dieser Wächter verhindert nicht, dass jemand ein Werkzeug falsch baut — er
 * verhindert, dass die DATEN eine Lage herstellen, in der jedes Werkzeug die
 * Hälfte übersieht.
 *
 * # Warum die Prüfung eigene Funktionen sind
 *
 * `formenJe()` und `gemischte()` bekommen ihre Daten als Argument, statt sie
 * selbst zu lesen. Nur so lässt sich der Wächter mit erfundenen Daten füttern
 * und zeigen, dass er **rot werden kann** — einer, der immer grün ist, weil er
 * nichts findet, ist von einem funktionierenden nicht zu unterscheiden.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const HIER = new URL('.', import.meta.url).pathname;
const AUFZEICHNUNGEN = join(HIER, 'recordings');

/** Die beiden bekannten Formen. Eine dritte fällt über die Summenprobe auf. */
export const FORMEN = ['slot', 'spellId'] as const;
export type Form = (typeof FORMEN)[number];

export interface Zaehlung {
  /** Welche Formen in dieser Datei vorkommen. */
  readonly formen: Form[];
  /** Drücke je Form. */
  readonly jeForm: Readonly<Record<Form, number>>;
  /** Alle Drücke, auch solche ohne bekannte Form. */
  readonly gesamt: number;
}

/**
 * Sammelt alle `presses`-Listen aus einem Datensatz — **auch verschachtelte**.
 *
 * Die Bauarten sind verschieden gewachsen: Manche Datei trägt `presses` auf
 * oberster Ebene, andere unter `runs[]`, seit `kontrolle-wirkt` auch unter
 * `cases[].runs[]`. Wer nur die oberste Ebene liest, zählt je nach Datei
 * richtig oder null — und die Null sähe aus wie „keine Drücke".
 */
/**
 * **Gesucht wird gezielt der Schlüssel `presses`, nicht „Objekte mit `slot`".**
 *
 * Nachgemessen, welche Listen druckähnlich aussehen:
 *
 * ```text
 *   presses     1465 Einträge, 43 Dateien   slot · spellId · tick
 *   bindings     171 Einträge, 42 Dateien   slot · spellId
 *   factorAt       3 Einträge,  1 Datei     buffs · tick
 * ```
 *
 * `bindings` trägt **beide** Felder in DEMSELBEN Objekt — das ist dort kein
 * Widerspruch, sondern seine Aufgabe: Platz und Zauber gehören zusammen. Eine
 * Suche über „Objekte mit `slot` oder `spellId"` zählte sie mit und meldete
 * damit **jede** Datei als gemischt. Der Wächter wäre rot, ohne dass etwas
 * kaputt wäre — und würde entfernt statt verstanden.
 */
function allePressListen(wert: unknown, heraus: unknown[][] = []): unknown[][] {
  if (Array.isArray(wert)) {
    for (const eintrag of wert) allePressListen(eintrag, heraus);
    return heraus;
  }
  if (wert !== null && typeof wert === 'object') {
    for (const [schluessel, unterwert] of Object.entries(wert)) {
      if (schluessel === 'presses' && Array.isArray(unterwert)) heraus.push(unterwert);
      else allePressListen(unterwert, heraus);
    }
  }
  return heraus;
}

/**
 * Zählt für einen Datensatz, welche Press-Formen darin vorkommen.
 *
 * `gesamt` zählt **jeden** Eintrag, nicht nur die erkannten. Das ist der Kern
 * der Summenprobe: Läge eine dritte Form vor, stünde sie in `gesamt`, aber in
 * keiner der beiden Zählungen.
 */
export function formenJe(daten: unknown): Zaehlung {
  const jeForm: Record<Form, number> = { slot: 0, spellId: 0 };
  let gesamt = 0;
  for (const liste of allePressListen(daten)) {
    for (const druck of liste) {
      gesamt++;
      if (druck === null || typeof druck !== 'object') continue;
      for (const form of FORMEN) {
        if (form in druck) jeForm[form]++;
      }
    }
  }
  return { formen: FORMEN.filter((f) => jeForm[f] > 0), jeForm, gesamt };
}

/** Die Namen der Dateien, die mehr als eine Form führen. */
export function gemischte(zaehlungen: ReadonlyMap<string, Zaehlung>): string[] {
  return [...zaehlungen].filter(([, z]) => z.formen.length > 1).map(([name]) => name);
}

/**
 * Der Befund als Text — **mit den Formen, nicht nur mit dem Namen**.
 *
 * Ein blosses `['coagulation']` sagt, DASS etwas nicht stimmt. Wer es liest,
 * muss dann selbst herausfinden, welche Formen kollidieren und wie oft. Die
 * Zahlen stehen hier ohnehin schon; sie wegzulassen verschiebt Arbeit vom
 * Wächter auf den, der ihn rot vorfindet.
 */
export function befund(zaehlungen: ReadonlyMap<string, Zaehlung>): string {
  const treffer = gemischte(zaehlungen);
  if (treffer.length === 0) return 'keine Datei mischt die Press-Formen';
  return treffer
    .map((name) => {
      const z = zaehlungen.get(name);
      if (z === undefined) return name;
      const teile = FORMEN.filter((f) => z.jeForm[f] > 0).map((f) => `${f} ${z.jeForm[f]}x`);
      return `${name}: ${teile.join(' UND ')} (von ${z.gesamt} Drücken)`;
    })
    .join('; ');
}

const dateien = readdirSync(AUFZEICHNUNGEN).filter((d) => d.endsWith('.json'));
const zaehlungen = new Map<string, Zaehlung>(
  dateien.map((d) => [
    d.replace('.json', ''),
    formenJe(JSON.parse(readFileSync(join(AUFZEICHNUNGEN, d), 'utf8'))),
  ]),
);

const summe = (form: Form): number =>
  [...zaehlungen.values()].reduce((n, z) => n + z.jeForm[form], 0);
const gesamtDruecke = [...zaehlungen.values()].reduce((n, z) => n + z.gesamt, 0);

describe('Eine Aufzeichnung nutzt genau eine Press-Form', () => {
  it('AC2 — der Wächter nennt seinen Umfang', () => {
    // **Eine stumme Null ist von einem stummen Ausfall nicht zu
    // unterscheiden.** Läse das Verzeichnis leer oder fände die Suche keine
    // `presses`, wären alle Fälle darunter grün und prüften nichts.
    const umfang = `${dateien.length} Dateien, ${gesamtDruecke} Drücke`;
    expect(dateien.length, `keine Aufzeichnungen gefunden (${umfang})`).toBeGreaterThan(30);
    expect(gesamtDruecke, `keine Drücke gefunden (${umfang})`).toBeGreaterThan(1000);

    // Und mindestens eine Datei je Form — sonst prüfte der Wächter eine Form,
    // die es gar nicht mehr gibt, und die Gegenprobe unten liefe ins Leere.
    expect(summe('slot'), 'keine Datei nutzt die slot-Form').toBeGreaterThan(0);
    expect(summe('spellId'), 'keine Datei nutzt die spellId-Form').toBeGreaterThan(0);
  });

  it('AC3 — die Summenprobe: slot + spellId = gesamt', () => {
    // **Stimmt sie nicht, gibt es eine DRITTE Form.** Genau daran ist die
    // Messung gescheitert, aus der dieses Issue entstand: Wer nur eine Form
    // liest, bekommt eine Zahl, die plausibel aussieht. Wer die Summe gegen
    // die Gesamtzahl hält, sieht die Lücke sofort.
    expect(
      summe('slot') + summe('spellId'),
      `slot ${summe('slot')} + spellId ${summe('spellId')} ergibt nicht ${gesamtDruecke} — ` +
        'es gibt eine dritte Press-Form',
    ).toBe(gesamtDruecke);
  });

  it('AC1 — keine Datei mischt die Formen', () => {
    // Die Meldung nennt Datei und beide Formen mit Anzahl, damit wer sie rot
    // vorfindet nicht erst nachzählen muss.
    expect(gemischte(zaehlungen), befund(zaehlungen)).toEqual([]);
  });
});

describe('AC4 — der Wächter kann rot werden', () => {
  it('eine gemischte Datei wird gemeldet, mit Namen und beiden Formen', () => {
    // Ohne diesen Fall bliebe offen, ob `gemischte()` überhaupt je etwas
    // zurückgibt oder nur zufällig immer die leere Liste liefert.
    const erfunden = new Map<string, Zaehlung>([
      ['sauber', { formen: ['slot'], jeForm: { slot: 3, spellId: 0 }, gesamt: 3 }],
      ['gemischt', { formen: ['slot', 'spellId'], jeForm: { slot: 2, spellId: 1 }, gesamt: 3 }],
    ]);
    expect(gemischte(erfunden)).toEqual(['gemischt']);
  });

  it('und der ECHTE Bestand fiele durch, mischte man eine Datei', () => {
    // Derselbe Bestand, einer Datei die andere Form untergeschoben: Jetzt muss
    // genau sie DAZUkommen. Das belegt, dass der grüne Fall oben an den Daten
    // hängt und nicht an einer leeren Liste.
    //
    // Geprüft wird die Differenz, nicht die Gesamtliste — sonst hinge dieser
    // Fall daran, dass der Bestand sauber ist, und meldete beim ersten echten
    // Fund zwei Fehler statt einem.
    const [erster] = [...zaehlungen.keys()];
    if (erster === undefined) throw new Error('keine Aufzeichnungen — die Gegenprobe misst nichts');
    const eigene = zaehlungen.get(erster);
    if (eigene === undefined) throw new Error(`${erster} fehlt`);

    const verfaelscht = new Map(zaehlungen);
    verfaelscht.set(erster, {
      formen: ['slot', 'spellId'],
      jeForm: { slot: eigene.jeForm.slot + 1, spellId: eigene.jeForm.spellId + 1 },
      gesamt: eigene.gesamt + 2,
    });
    const neu = gemischte(verfaelscht).filter((n) => !gemischte(zaehlungen).includes(n));
    expect(neu).toEqual([erster]);

    // Und der Befundtext nennt beide Formen — nicht nur den Dateinamen.
    expect(befund(verfaelscht)).toContain('slot');
    expect(befund(verfaelscht)).toContain('spellId');
    expect(befund(verfaelscht)).toContain(erster);
  });

  it('formenJe findet presses auch verschachtelt', () => {
    // **Die Bauart, an der eine zu flache Suche scheitert.** `presses` steht je
    // nach Datei auf oberster Ebene, unter `runs[]` oder unter
    // `cases[].runs[]`. Wer nur oben liest, zählt bei zwei von drei Bauarten
    // null — und die Null sähe aus wie „keine Drücke", nicht wie ein Fehler.
    const oben = formenJe({ presses: [{ tick: 0, slot: 1 }] });
    const tief = formenJe({ cases: [{ runs: [{ presses: [{ tick: 0, spellId: 'kindle' }] }] }] });
    expect(oben.gesamt).toBe(1);
    expect(oben.formen).toEqual(['slot']);
    expect(tief.gesamt).toBe(1);
    expect(tief.formen).toEqual(['spellId']);
  });

  it('eine dritte Form bricht die Summenprobe', () => {
    // Der Fall, den AC3 im Bestand ausschliesst — hier künstlich hergestellt,
    // damit belegt ist, dass die Probe ihn überhaupt sehen kann.
    const dritte = formenJe({ presses: [{ tick: 0, keyCode: 'Q' }] });
    expect(dritte.gesamt).toBe(1);
    expect(dritte.jeForm.slot + dritte.jeForm.spellId).toBe(0);
    expect(dritte.formen).toEqual([]);
  });
});
