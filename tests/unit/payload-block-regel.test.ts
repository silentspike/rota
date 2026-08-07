/**
 * Der `race-heal`-Block liest aus `result`, nicht aus `combat` (Issue #630).
 *
 * # Die Lücke, gegen die kein Mutationstest hilft
 *
 * Bis #638 holte `payloads.ts` in diesem Block vier Werte direkt aus `combat`,
 * während die zehn Nachbarzeilen aus `result` lasen. Der Wert war **richtig** —
 * beide Quellen liefern dasselbe —, nur der Weg führte am `ReplayResult`
 * vorbei, also an genau der Stelle, die ein Werkzeug abfragt.
 *
 * **Deshalb konnte es niemand sehen.** Gemessen: Stellt man die vier Zeilen
 * wieder auf `combat` zurück, färbt das **null von 1921** Fällen. Ein Fall,
 * der es meldete, müsste prüfen, WOHER eine Zahl kommt, nicht WELCHE — und so
 * einen gab es nicht. Es war nicht „niemand hat hingesehen", sondern „es war
 * nicht sichtbar".
 *
 * # Warum ein Fall und kein Kommentar
 *
 * Der wahrscheinlichste Rückfall ist der harmloseste Handgriff: Wer den Block
 * später erweitert, schreibt `combat.x` hin, weil es hier früher so stand. Ein
 * Kommentar wird einmal gelesen; dieser Fall meldet sich wieder.
 *
 * # Die Ausnahme ist Teil der Regel
 *
 * `finalRaceProgress` liest weiter aus `combat`, und das bleibt richtig:
 * `raceProgress` ist ein **Momentanwert** — Zustand während des Kampfes, kein
 * Ergebnis danach. Das `ReplayResult` führt ihn nicht und soll ihn nicht
 * führen; es gibt kein `result.finalRaceProgress`, auf das man umstellen
 * könnte. Dasselbe gilt für alle übrigen `final*`-Felder der Datei.
 *
 * # WAS DIESE PRÜFUNG NICHT KANN
 *
 * Sie liest **Quelltext, nicht Verhalten**. Sie fängt einen vergessenen
 * Rückfall, nicht einen falschen Wert. Wer `result.raceWon` schreibt und
 * dabei das falsche Feld meint, kommt daran vorbei — dafür sind die Fälle in
 * `bossheal-leitung.test.ts` und `kumulative-leitungen.test.ts` da.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const QUELLE = readFileSync(
  new URL('../../tests/parity/payloads.ts', import.meta.url).pathname,
  'utf8',
);

/**
 * Der Block, der `race-heal.json` erzeugt.
 *
 * Abgegrenzt am Inhalt statt an Zeilennummern: Die Zahl altert mit jeder
 * Änderung an der Datei, der Name trägt. Genau diese Falle hat heute einen
 * Befund zu einem bereits erledigten Issue erzeugt.
 */
function raceHealBlock(): string {
  const start = QUELLE.indexOf('raceWon:');
  expect(start, 'der raceWon-Block steht nicht mehr in payloads.ts').toBeGreaterThan(0);
  const ende = QUELLE.indexOf('interruptsMissed:', start);
  expect(ende, 'der Block endet nicht mehr bei interruptsMissed').toBeGreaterThan(start);
  return QUELLE.slice(start, ende + 200);
}

describe('der race-heal-Block nimmt den Weg der Werkzeuge', () => {
  it('es gibt überhaupt einen Block zu prüfen', () => {
    // Läse die Datei leer oder wäre der Block umbenannt, wären alle Fälle
    // darunter Aussagen über eine leere Zeichenkette.
    expect(QUELLE.length, 'payloads.ts ist leer').toBeGreaterThan(1000);
    expect(raceHealBlock().length).toBeGreaterThan(50);
  });

  it('die vier Ergebnisgrössen lesen aus `result`', () => {
    const block = raceHealBlock();
    for (const feld of ['raceWon', 'damageLostToRace', 'bossHealed', 'interruptsMissed']) {
      expect(block, `${feld} liest wieder aus combat — das umgeht das ReplayResult`).toContain(
        `${feld}: result.${feld},`,
      );
    }
  });

  it('KEINE Zeile des Blocks liest aus `combat` — ausser den final*-Feldern', () => {
    // **Die eigentliche Regel.** Sie wächst mit: Kommt eine Zeile dazu, die
    // aus `combat` liest und nicht `final` heisst, fällt dieser Fall.
    const verstoesse = raceHealBlock()
      .split('\n')
      .map((z) => z.trim())
      .filter((z) => z.includes('combat.') && !z.startsWith('final') && !z.startsWith('//'));
    expect(verstoesse, 'diese Zeilen umgehen das ReplayResult').toEqual([]);
  });

  it('und `finalRaceProgress` liest weiterhin aus `combat` — die Ausnahme steht', () => {
    // Die Gegenrichtung: Würde jemand die Ausnahme „aufräumen" und auf
    // `result` umstellen, gäbe es das Feld dort nicht. `tsc` finge es — aber
    // erst nach dem Bau, und ohne zu sagen warum es nie dorthin gehörte.
    expect(raceHealBlock()).toContain('finalRaceProgress: combat.raceProgress,');
  });
});

describe('der Wächter kann rot werden — die Gegenprobe', () => {
  it('eine erfundene combat-Zeile im Block würde gemeldet', () => {
    // Ohne diesen Fall bliebe offen, ob der Filter oben überhaupt je etwas
    // findet oder nur zufällig immer die leere Liste liefert.
    const erfunden = ['raceWon: result.raceWon,', 'neuesFeld: combat.irgendwas,']
      .map((z) => z.trim())
      .filter((z) => z.includes('combat.') && !z.startsWith('final') && !z.startsWith('//'));
    expect(erfunden).toEqual(['neuesFeld: combat.irgendwas,']);
  });
});
