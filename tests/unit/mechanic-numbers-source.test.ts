/**
 * Die Zahlen neben dem Hitze-Balken (Issue #283).
 *
 * WAS DIESER PRUEFFALL KANN UND WAS NICHT
 * ---------------------------------------
 * Die Anzeige selbst ist Canvas und braucht ein Dokument — mit Vitest ist sie
 * nicht spielbar. Der Nachweis dafuer ist ein Screenshot, und das steht so im
 * PR-Text. Ein Prueffall, der behauptet, die Zeichnung zu pruefen, waere
 * schlimmer als keiner.
 *
 * Was hier geprueft wird, ist die eine Sache, die im Quelltext entscheidbar
 * ist: **dass der Nenner der Zersetzung nicht abgeschrieben ist.**
 *
 * `MAX_DECAY_STACKS` traegt den Kommentar „Wird gemessen" — die Fuenf ist ein
 * Platzhalter des Balance-Laeufers und wandert, sobald #268 gebaut ist. Steht
 * im HUD eine 5, zeigt die Anzeige nach der Balance „3 / 5", waehrend der
 * Kampf mit sieben rechnet. Das ist genau die Klasse Fehler, die niemand
 * sieht: Die Zahl SIEHT richtig aus.
 */
import { describe, expect, it } from 'vitest';
import { DECAY_STACK_CAPACITY, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { CALCINATION } from '../../src/core/data.js';

const QUELLE = import.meta.glob('/src/render/hud.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function hudQuelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length === 0) {
    throw new Error('hud.ts nicht geladen — die Pruefung wuerde nichts messen');
  }
  return text;
}

describe('Der Nenner wandert mit', () => {
  it('findet den Quelltext ueberhaupt', () => {
    expect(hudQuelle().length).toBeGreaterThan(1000);
  });

  it('nimmt den Nenner AUS DEM KAMPF, statt die Zahl abzuschreiben', () => {
    // # Warum hier seit #479 nicht mehr `MAX_DECAY_STACKS` steht
    //
    // Die Prüfung hieß bis PUTRID BLOOM „nimmt MAX_DECAY_STACKS". Das war
    // richtig, solange die Grenze fest war — sie ist es nicht mehr: Der Zauber
    // hebt sie für eine Weile, und ein HUD, das die Konstante zeichnet, zeigte
    // dann „6 / 5".
    //
    // **Die Absicht der Prüfung bleibt dieselbe**, nur ihre Quelle wandert:
    // Der Nenner darf nicht abgeschrieben sein. Statt der Konstante steht dort
    // jetzt der Wert, den der Kampf führt — das ist strenger, nicht lockerer,
    // denn er folgt auch der Anhebung.
    const quelle = hudQuelle();
    const nennerZeile = quelle
      .split('\n')
      .find((z) => z.includes('decayStacks') && z.includes('/'));
    expect(nennerZeile, 'die Nenner-Zeile fehlt').toBeDefined();
    expect(nennerZeile, 'der Nenner ist abgeschrieben').toContain('decayStackCap');
  });

  it('SIEHT eine abgeschriebene Zahl — die Gegenprobe', () => {
    // Ohne sie waere nicht gezeigt, dass die Pruefung oben etwas sieht.
    const erfunden = 'zeichne(LABEL_DECAY, `${combat.decayStacks} / 5`);';
    expect(erfunden).not.toContain('MAX_DECAY_STACKS');
    expect(erfunden).toContain('/');
  });

  it('die Konstante ist heute fuenf — und das ist ein Platzhalter', () => {
    // Steht hier, damit die Zahl EINMAL im Pruefstand auftaucht: Wer sie
    // aendert, sieht diesen Fall und den Kommentar dazu.
    expect(MAX_DECAY_STACKS).toBe(5);
  });

  it('die KAPAZITAET traegt jeden Zuschlag, den ein Zauber geben darf', () => {
    // **Das ist der Wächter, den `data.ts` nicht stellen kann** (Issue #479):
    // Die Konstanten stehen in `combat.ts`, und das importiert aus `data.ts` —
    // ein Import in die Gegenrichtung wäre ein Zirkelbezug. Die Prüfung steht
    // deshalb hier, wo beide Seiten erreichbar sind.
    //
    // Ein Zuschlag über die Kapazität hinaus hätte keinen Platz im Feld, und
    // die Prüfsumme läse Werte, die es nicht gibt. `sim/build.rs` bricht dafür
    // den Bau ab; diese Zeile ist das Gegenstück auf der Spiel-Seite.
    const zuschlaege = CALCINATION.value.spells
      .filter((s) => s.decayCap !== undefined)
      .map((s) => ({ id: s.id, bonus: s.decayCap?.bonus ?? 0 }));
    expect(
      zuschlaege.length,
      'kein Zauber hebt die Grenze — dann prüft dieser Fall nichts',
    ).toBeGreaterThan(0);
    for (const { id, bonus } of zuschlaege) {
      expect(
        MAX_DECAY_STACKS + bonus,
        `${id} hebt die Grenze auf ${MAX_DECAY_STACKS + bonus}, die Struktur fasst ${DECAY_STACK_CAPACITY}`,
      ).toBeLessThanOrEqual(DECAY_STACK_CAPACITY);
    }
  });
});
