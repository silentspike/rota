import { describe, expect, it } from 'vitest';
import {
  EVENT_BOSS_ENERGY_TRIGGERED,
  EVENT_CAST,
  EVENT_CONVERTED_TO_SHIELD,
  EVENT_CRIT,
  EVENT_DAMAGE,
  EVENT_DOT_TICK,
  EVENT_KINDS_WRITTEN_BY_GAME,
  EVENT_NAMES,
  EVENT_PHASE,
} from '../../src/core/log.js';

/**
 * Der Nummernraum der Spielseite hat **eine** Quelle (Issue #138).
 *
 * # Was diese Datei NICHT prüfen muss
 *
 * **Dass eine Konstante ohne Namen entsteht.** Das ist seit #138 ein
 * Übersetzungsfehler, kein Laufzeit-Befund:
 *
 * ```text
 * $ export const EVENT_NEU = EVENT_KINDS.neueArt;
 * src/core/log.ts(120,38): error TS2339: Property 'neueArt' does not exist
 *   on type '{ readonly cast: 0; readonly damage: 1; ... }'
 * ```
 *
 * Die Auflage zum Issue verlangte „unmöglich, nicht gemeldet". Ein Prüffall,
 * der die Längen zweier Listen vergleicht, wäre wieder nur ein Wächter
 * gewesen — dieselbe Klasse, die #134 auf der Rust-Seite ersetzt hat.
 *
 * # Was übrig bleibt
 *
 * Eine **übersprungene Nummer**. Wer `17` statt `16` vergibt, lässt eine
 * Lücke, und `EVENT_NAMES` hätte an der Stelle `undefined`. Das kann der
 * Übersetzer nicht sehen — dafür ist der Prüffall unten da.
 */
describe('Der Nummernraum der Spielseite', () => {
  it('hat keine Lücke — jeder Index trägt einen Namen', () => {
    for (let kind = 0; kind < EVENT_NAMES.length; kind++) {
      expect(EVENT_NAMES[kind], `Kennnummer ${kind} hat keinen Namen`).toBeTypeOf('string');
      expect(EVENT_NAMES[kind]).not.toBe('');
    }
  });

  it('vergibt keinen Namen zweimal', () => {
    expect(new Set(EVENT_NAMES).size).toBe(EVENT_NAMES.length);
  });

  it('gibt jeder Konstante den Namen an ihrer Stelle', () => {
    // Stichproben über den ganzen Bereich — Anfang, Mitte, Ende. Ein
    // Verrutschen um eins fiele hier auf.
    expect(EVENT_NAMES[EVENT_CAST]).toBe('cast');
    expect(EVENT_NAMES[EVENT_DAMAGE]).toBe('damage');
    expect(EVENT_NAMES[EVENT_DOT_TICK]).toBe('dotTick');
    expect(EVENT_NAMES[EVENT_CRIT]).toBe('crit');
    expect(EVENT_NAMES[EVENT_PHASE]).toBe('phase');
    expect(EVENT_NAMES[EVENT_BOSS_ENERGY_TRIGGERED]).toBe('bossEnergyTriggered');
    expect(EVENT_NAMES[EVENT_CONVERTED_TO_SHIELD]).toBe('convertedToShield');
  });

  it('hält die Nummern fest, die im Format zwischen beiden Seiten stehen', () => {
    // **Kein Zierrat.** Der Nummernraum ist ein Format zwischen zwei
    // Programmen: Wer einen Eintrag in der Mitte einfügt, verschiebt sonst
    // alle folgenden Nummern, und ein aufgezeichnetes Protokoll bedeutete
    // danach etwas anderes. Diese Zeilen sind die Sperre dagegen.
    expect(EVENT_CAST).toBe(0);
    expect(EVENT_DAMAGE).toBe(1);
    expect(EVENT_DOT_TICK).toBe(2);
    expect(EVENT_CRIT).toBe(3);
    expect(EVENT_PHASE).toBe(6);
    expect(EVENT_BOSS_ENERGY_TRIGGERED).toBe(8);
    expect(EVENT_CONVERTED_TO_SHIELD).toBe(15);
  });

  it('führt nur Arten, die es auch gibt, als vom Spiel geschrieben', () => {
    for (const kind of EVENT_KINDS_WRITTEN_BY_GAME) {
      expect(EVENT_NAMES[kind], `Kennnummer ${kind} hat keinen Namen`).toBeTypeOf('string');
    }
    expect(new Set(EVENT_KINDS_WRITTEN_BY_GAME).size).toBe(EVENT_KINDS_WRITTEN_BY_GAME.length);
  });
});
