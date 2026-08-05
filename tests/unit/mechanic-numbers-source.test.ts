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
import { MAX_DECAY_STACKS } from '../../src/core/combat.js';

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

  it('nimmt MAX_DECAY_STACKS, statt die Zahl abzuschreiben', () => {
    const quelle = hudQuelle();
    expect(quelle, 'die Konstante muss benutzt werden').toContain('MAX_DECAY_STACKS');
    // Und die Zeile, die den Nenner baut, nennt sie beim Namen.
    const nennerZeile = quelle
      .split('\n')
      .find((z) => z.includes('decayStacks') && z.includes('/'));
    expect(nennerZeile, 'die Nenner-Zeile fehlt').toBeDefined();
    expect(nennerZeile, 'der Nenner ist abgeschrieben').toContain('MAX_DECAY_STACKS');
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
});
