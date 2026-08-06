/**
 * Der Wächter über den Spielstand (Issue #378, GAME-DESIGN 18.1).
 *
 * WARUM DIESE DATEI DER KERN DES ISSUES IST UND NICHT DIE PRÜFUNGEN
 * ----------------------------------------------------------------
 * Die Formprüfungen für `runHistory` und `progressMarks` sind Handarbeit, die
 * einmal getan wird. Dieser Wächter ist das, was verhindert, dass die NÄCHSTE
 * Liste dieselbe Lücke bekommt.
 *
 * Und das ist keine Theorie: `bestRuns` WAR die nächste. Ihre Zeile in
 * `ergaenzt()` wurde in #377 nachgezogen, weil sie beim Messen auffiel — nicht,
 * weil irgendetwas darauf gestossen hätte. Beim Bau dieses Issues kamen sechs
 * weitere heraus, die seit Monaten fehlten.
 *
 * ER PRÜFT VERHALTEN, NICHT QUELLTEXT
 * -----------------------------------
 * Er baut für jedes nicht primitive Feld aus `emptySave()` einen echten Stand,
 * in dem genau dieses Feld fehlt, schreibt ihn in einen Speicher und lädt ihn
 * über den normalen Weg. Das fängt auch die falsch geschriebene Zeile —
 * `geladen.items ?? leer.itmes` — die ein Quelltext-Wächter durchgelassen
 * hätte, weil dort ja etwas steht.
 *
 * DIE FELDLISTE PFLEGT NIEMAND
 * ----------------------------
 * Sie kommt aus `emptySave()` selbst. Wer ein Feld hinzufügt, hat es damit
 * automatisch in der Prüfung — dieselbe Technik wie `Object.keys` in
 * `reset-completeness.test.ts` und `#[derive(Debug)]` im Simulator
 * (WORKER-GUIDE 8.21).
 */
import { describe, expect, it } from 'vitest';
import { emptySave, RunPersistence } from '../../src/analysis/run-persistence.js';
import { checksumOf } from '../../src/analysis/save-store.js';
import { LIST_GUARDS } from '../../src/analysis/entry-shape.js';

const FASSUNG = '0.1.0';
const JETZT = 1_800_000_000_000;

/** Ein Speicher im Arbeitsspeicher — dieselbe Schnittstelle wie localStorage. */
function speicher(): Map<string, string> {
  return new Map<string, string>();
}

function alsStore(daten: Map<string, string>): {
  getItem: (k: string) => string | null;
  setItem: (k: string, v: string) => void;
  removeItem: (k: string) => void;
} {
  return {
    getItem: (k) => daten.get(k) ?? null,
    setItem: (k, v) => {
      daten.set(k, v);
    },
    removeItem: (k) => {
      daten.delete(k);
    },
  };
}

/** Einen Stand von Hand in einen Platz legen — mit gültiger Prüfsumme. */
function legeAb(daten: Map<string, string>, slot: number, stand: unknown): void {
  const payload = JSON.stringify(stand);
  daten.set(
    `rota.save.${String(slot)}`,
    JSON.stringify({ checksum: checksumOf(payload), payload }),
  );
  daten.set('rota.save.index', String(slot));
}

/** Alle Felder, die kein Primitiv sind — die Liste, die niemand pflegt. */
function zusammengesetzteFelder(): string[] {
  const leer = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
  return Object.entries(leer)
    .filter(([, wert]) => typeof wert === 'object' && wert !== null)
    .map(([name]) => name);
}

/**
 * Die Prüfung selbst, gegen eine beliebige Ergänzungs-Funktion.
 *
 * **Als Funktion und nicht als Schleife im Prüffall**, damit AC4 und AC5
 * dieselbe Logik fahren: AC4 gibt den echten Weg hinein, AC5 eine bewusst
 * lückenhafte Nachbildung. Zwei verschiedene Prüfungen zu schreiben hiesse,
 * dass die Gegenprobe etwas anderes belegt als die Prüfung.
 */
function luecken(
  ergaenze: (geladen: Record<string, unknown>) => Record<string, unknown>,
): string[] {
  const gefunden: string[] = [];
  for (const feld of zusammengesetzteFelder()) {
    const stand = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    // **`null`, nicht `delete`** — und das ist der ganze Unterschied zwischen
    // einem Wächter, der etwas findet, und einem, der immer grün ist:
    //
    //   Feld FEHLT        -> `{ ...leer, ...geladen }` nimmt den leeren Wert.
    //                        Harmlos. Die `??`-Zeile ist dafür gar nicht nötig.
    //   Feld ist `null`   -> wird durchgereicht. DAS ist der Fall, den die
    //                        Zeile fängt, und der einzige, den ein geladener
    //                        Stand erzeugen kann.
    //   Feld ist `undefined` -> wird auch durchgereicht, kommt aber nie an:
    //                        `JSON.stringify({a: undefined})` ist `{}`.
    //
    // Die erste Fassung dieses Wächters benutzte `delete` und war grün, ohne
    // etwas zu prüfen. Aufgefallen ist es der Gegenprobe unten — nicht dem
    // Lesen.
    stand[feld] = null;
    const ergebnis = ergaenze(stand)[feld];
    if (ergebnis === undefined || ergebnis === null) gefunden.push(feld);
  }
  return gefunden;
}

/** Der echte Weg: durch den Speicher, mit Prüfsumme, über `RunPersistence`. */
function echterWeg(stand: Record<string, unknown>): Record<string, unknown> {
  const daten = speicher();
  legeAb(daten, 0, stand);
  const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
  return (geladen as unknown as { data: Record<string, unknown> }).data;
}

describe('AC4 — jedes zusammengesetzte Feld steht in ergaenzt() einzeln', () => {
  const FELDER = zusammengesetzteFelder();

  it('es sind sieben, und der Wächter kennt sie ohne Pflege', () => {
    // Fünf Listen und zwei Objekte. Die Zahl steht hier, damit ein Zuwachs
    // sichtbar wird — nicht als Bedingung, sondern als Merkposten: Wer sie
    // ändert, hat gerade ein Feld hinzugefügt oder entfernt und liest die
    // Prüfung darunter.
    //
    // **Und genau das ist hier passiert.** In #435 fielen `unlockedSpells` und
    // `unlockedInstances` weg, weil sie berechenbare Werte speicherten
    // (GAME-DESIGN 18.6). Dieser Fall ist dabei rot geworden — kein Schaden,
    // sondern der Beleg, dass er tut, wofür er in #378 gebaut wurde. Wer ihn
    // ohne Nachdenken anpasst, hat die Prüfung darunter nicht gelesen.
    expect(FELDER).toEqual([
      'talentPoints',
      'items',
      'bindings',
      'bestRuns',
      'defeatedBosses',
      'runHistory',
      'progressMarks',
    ]);
  });

  it.each(FELDER)('%s überlebt einen Stand, der dort `null` trägt', (feld) => {
    const daten = speicher();
    const stand = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    stand[feld] = null;
    legeAb(daten, 0, stand);

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    const roh = geladen as unknown as { data: Record<string, unknown> };

    expect(
      roh.data[feld],
      `Das Feld "${feld}" fehlt in ergaenzt() in run-persistence.ts. ` +
        'Der Streu-Operator reicht ein `null` durch — es braucht die Zeile ' +
        `\`${feld}: geladen.${feld} ?? leer.${feld},\`.`,
    ).not.toBeNull();
  });

  it('ein FEHLENDES Feld ist dagegen harmlos — und das gehört gemessen', () => {
    // Sonst schriebe der nächste eine Zeile mehr, weil er den Kommentar in
    // `ergaenzt()` liest und ihn für vollständig hält. Der Streu-Operator
    // nimmt bei einem fehlenden Schlüssel den leeren Wert von selbst.
    const daten = speicher();
    const stand = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    delete stand['runHistory'];
    legeAb(daten, 0, stand);

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.runHistory).toEqual([]);
  });

  it('am echten Weg gemessen ist keine Lücke offen', () => {
    // Dieselbe Aussage wie die Einzelfälle darüber, in einem Satz — und die
    // Form, gegen die die Gegenprobe unten läuft.
    expect(luecken(echterWeg)).toEqual([]);
  });

  it('AC5 GEGENPROBE: eine Zeile weniger, und die Lücke wird GENANNT', () => {
    // Die Fehlerart nachgebaut, nicht irgendein Fehler: `ergaenzt()` ohne die
    // Zeile für `items` — genau der Zustand, den die nächste Erweiterung
    // erzeugt, wenn jemand sie vergisst.
    //
    // Ohne diese Zeile wäre nicht zu sehen, ob die Prüfung oben überhaupt
    // etwas finden KANN. Eine Prüfung, die immer grün ist, prüft nichts.
    const leer = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    const lueckenhaft = (geladen: Record<string, unknown>): Record<string, unknown> => {
      const ergebnis: Record<string, unknown> = { ...leer, ...geladen };
      for (const feld of zusammengesetzteFelder()) {
        // `items` bleibt bewusst aus — der Streu-Operator reicht dort das
        // `undefined` durch, genau wie in der echten Fassung vor #378.
        if (feld === 'items') continue;
        ergebnis[feld] = geladen[feld] ?? leer[feld];
      }
      return ergebnis;
    };

    expect(luecken(lueckenhaft)).toEqual(['items']);
  });

  it('GEGENPROBE ZUR GEGENPROBE: mit der Zeile ist sie wieder zu', () => {
    // Sonst bliebe offen, ob `luecken` bei DIESER Nachbildung immer etwas
    // meldet — dann hätte die Gegenprobe oben nichts belegt.
    const leer = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    const vollstaendig = (geladen: Record<string, unknown>): Record<string, unknown> => {
      const ergebnis: Record<string, unknown> = { ...leer, ...geladen };
      for (const feld of zusammengesetzteFelder()) ergebnis[feld] = geladen[feld] ?? leer[feld];
      return ergebnis;
    };

    expect(luecken(vollstaendig)).toEqual([]);
  });

  it('jede Liste hat ausserdem eine Formprüfung', () => {
    // Zwei Tabellen, die zusammengehören: Wer eine Liste anlegt, braucht ihre
    // Zeile in `ergaenzt()` UND ihren Eintrag in `LIST_GUARDS`. Die Objekte
    // (talentPoints, bindings) sind keine Listen und haben deshalb keine.
    const leer = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    const listen = Object.entries(leer)
      .filter(([, wert]) => Array.isArray(wert))
      .map(([name]) => name);

    for (const liste of listen) {
      expect(
        Object.keys(LIST_GUARDS),
        `Die Liste "${liste}" hat keine Formprüfung in LIST_GUARDS ` +
          '(src/analysis/entry-shape.ts). Ihre Einträge kämen ungeprüft durch.',
      ).toContain(liste);
    }
  });
});

describe('AC1/AC2 — ein kaputter Eintrag, nicht die ganze Liste', () => {
  function standMit(over: Record<string, unknown>): Record<string, unknown> {
    return { ...(emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>), ...over };
  }

  const GUTER_LAUF = {
    bossId: 'first-crucible',
    difficulty: 1,
    at: JETZT,
    gameVersion: FASSUNG,
    victory: true,
    elapsedMs: 40_510,
    remainingPercent: 0,
    damage: 4240,
    idleMs: 0,
    missedMs: 0,
  };

  it('verwirft den kaputten Eintrag und behält die übrigen', () => {
    const daten = speicher();
    legeAb(
      daten,
      0,
      standMit({
        runHistory: [
          GUTER_LAUF,
          { ...GUTER_LAUF, damage: 'viel' },
          { ...GUTER_LAUF, at: JETZT + 1000 },
        ],
      }),
    );

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.runHistory).toHaveLength(2);
    expect(geladen.status.status).toBe('ok');
    // Der Stand ist heil — nur ärmer.
    expect(geladen.status.skipped).toBe(0);
  });

  it('GEGENPROBE: ein heiler Stand verliert nichts', () => {
    // Ohne diese Zeile wäre nicht zu sehen, ob die Prüfung überhaupt je einen
    // Eintrag durchlässt. Eine Prüfung, die alles verwirft, verwirft auch das
    // Richtige — und der Spielstand sähe aus wie neu.
    const daten = speicher();
    legeAb(daten, 0, standMit({ runHistory: [GUTER_LAUF, { ...GUTER_LAUF, at: JETZT + 1 }] }));

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.runHistory).toHaveLength(2);
  });

  it('ein Stand, dessen Einträge ALLE kaputt sind, gilt weiter als heil', () => {
    // Eine leere Liste ist ein gültiger Zustand — jeder neue Spieler hat sie.
    // „Alle weg heisst kaputt" würde bei einem Fassungswechsel jeden Stand
    // verwerfen und damit genau den Schaden anrichten, vor dem die Rotation
    // schützen soll.
    const daten = speicher();
    legeAb(daten, 0, standMit({ runHistory: [{ kaputt: true }, { auch: 'kaputt' }] }));

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.runHistory).toEqual([]);
    expect(geladen.status.status).toBe('ok');
  });

  it('prüft die Fortschrittsmarken genauso', () => {
    const gut = {
      bossId: 'first-crucible',
      difficulty: 1,
      bestRemainingPercent: 42,
      attempts: 3,
      defeated: false,
      gameVersion: FASSUNG,
      at: JETZT,
    };
    const daten = speicher();
    legeAb(daten, 0, standMit({ progressMarks: [gut, { ...gut, defeated: 'nein' }] }));

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.progressMarks).toHaveLength(1);
  });
});

describe('AC3 — eine kaputte STRUKTUR führt zum letzten heilen Stand', () => {
  it('nimmt den vorigen Platz, wenn eine Liste gar keine Liste ist', () => {
    // Die Rotation gibt es seit #179. Was fehlte, war der Auslöser: Bis zu
    // diesem Issue war ein Stand nur bei kaputtem JSON, falscher Prüfsumme
    // oder zu hoher Schema-Nummer unbrauchbar — nie wegen seines Inhalts.
    const daten = speicher();
    const heil = {
      ...(emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>),
      level: 7,
    };
    legeAb(daten, 0, heil);
    legeAb(daten, 1, { ...heil, level: 9, runHistory: 'keine Liste' });

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.level).toBe(7);
    expect(geladen.status.status).toBe('recovered');
    expect(geladen.status.skipped).toBe(1);
  });

  it('GEGENPROBE: sind beide Plätze heil, wird der jüngere genommen', () => {
    const daten = speicher();
    const heil = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    legeAb(daten, 0, { ...heil, level: 7 });
    legeAb(daten, 1, { ...heil, level: 9 });

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.level).toBe(9);
    expect(geladen.status.status).toBe('ok');
    expect(geladen.status.skipped).toBe(0);
  });

  it('ein einzelner kaputter Eintrag löst die Rotation NICHT aus', () => {
    // Die Grenze zwischen AC2 und AC3, von der anderen Seite gemessen: Wäre
    // sie falsch gezogen, verlöre der Spieler bei einem einzigen beschädigten
    // Verlaufseintrag seinen ganzen jüngeren Stand.
    const daten = speicher();
    const heil = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    legeAb(daten, 0, { ...heil, level: 7 });
    legeAb(daten, 1, { ...heil, level: 9, runHistory: [{ kaputt: true }] });

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    expect(geladen.state.level).toBe(9);
    expect(geladen.status.status).toBe('ok');
  });
});

describe('AC2/AC3 (#435) — ein alter Stand mit den entfernten Feldern', () => {
  /**
   * `unlockedSpells` und `unlockedInstances` sind seit #435 aus dem Format
   * weg, weil sie berechenbare Werte speicherten (GAME-DESIGN 18.6).
   *
   * **Ein bestehender Spielstand trägt sie trotzdem weiter**, und das ist
   * bewusst so: `ergaenzt()` bildet den Stand als `{ ...leer, ...geladen }`
   * und reicht unbekannte Felder durch. Sie aktiv zu streichen wäre eine
   * Änderung am Lesepfad — für zwei leere Arrays, die niemand liest.
   *
   * Diese Datei hält beides fest: dass er lädt, und dass der Rest bleibt. Ein
   * vermerkter Rest ist etwas anderes als ein stiller.
   */
  function altenStandLaden(): { level: number; roh: Record<string, unknown> } {
    const daten = speicher();
    const stand = {
      ...(emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>),
      level: 12,
      unlockedSpells: ['kindle', 'pyre'],
      unlockedInstances: ['first-crucible'],
    };
    legeAb(daten, 0, stand);

    const geladen = new RunPersistence(FASSUNG, () => JETZT, alsStore(daten));
    return {
      level: geladen.state.level,
      roh: (geladen as unknown as { data: Record<string, unknown> }).data,
    };
  }

  it('lädt fehlerfrei, und der Fortschritt kommt an', () => {
    // Der eigentliche Nachweis: Ein Stand aus der Zeit vor #435 darf nicht
    // verlorengehen. Die Stufe ist der sichtbare Beleg dafür, dass er nicht
    // still verworfen und durch einen leeren ersetzt wurde.
    expect(altenStandLaden().level).toBe(12);
  });

  it('AC3 — und die entfernten Felder bleiben darin stehen', () => {
    // **Kein Versehen, sondern die Entscheidung.** Wer das ändern will, ändert
    // den Lesepfad — und sollte vorher wissen, dass es hier gemessen wurde.
    const roh = altenStandLaden().roh;
    expect(roh['unlockedSpells']).toEqual(['kindle', 'pyre']);
    expect(roh['unlockedInstances']).toEqual(['first-crucible']);
  });

  it('sie stehen aber nicht mehr im FORMAT', () => {
    // Die andere Hälfte: Ein neuer Stand legt sie nicht mehr an. Damit
    // verschwinden sie mit dem letzten alten Spielstand von selbst.
    const leer = emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>;
    expect(Object.keys(leer)).not.toContain('unlockedSpells');
    expect(Object.keys(leer)).not.toContain('unlockedInstances');
  });
});
