/**
 * Der Prüf-Boss erzwingt die Kollision — und zwar in den Daten (Issue #331).
 *
 * # Warum die Zahlen in der Datei stehen und nicht hier
 *
 * Die Kollision ist ein **Zeitpunkt**: Ein Eintrag beginnt in genau dem Takt,
 * in dem der vorige landet. Ein Prüffall, der sich diese Lage selbst
 * herstellt, belegt nur, dass er sie herstellen kann — er sagt nichts darüber,
 * ob ein Boss sie je erzeugt.
 *
 * Deshalb steht `test-collision.json` daneben, und dieser Fall **misst** die
 * Bedingung, statt sie vorauszusetzen. Wer die Datei ändert und die Kollision
 * dabei zerstört, bekommt es hier gesagt — und nicht erst dadurch, dass ein
 * anderer Prüffall stumm grün wird.
 */
import { describe, expect, it } from 'vitest';
import { BOSSES } from '../../src/core/data.js';

describe('Der Kollisions-Boss (AC1)', () => {
  it('existiert und trägt eine Zeitleiste', () => {
    const boss = BOSSES.get('test-collision');
    expect(boss, 'data/bosses/test-collision.json wird nicht geladen').toBeDefined();
    expect(boss?.value.timeline.length).toBeGreaterThan(1);
  });

  it('lässt einen Eintrag genau dort beginnen, wo der vorige landet', () => {
    const timeline = BOSSES.get('test-collision')?.value.timeline ?? [];
    const kollisionen: string[] = [];

    for (let i = 1; i < timeline.length; i++) {
      const vorher = timeline[i - 1];
      const jetzt = timeline[i];
      if (vorher === undefined || jetzt === undefined) continue;
      const landet = vorher.atMs + (vorher.castTimeMs ?? 0);
      // **Beide müssen unterbrechbar sein**, und das ist die Bedingung, auf
      // die es ankommt. Meine erste Fassung prüfte nur, dass der VORIGE eine
      // Wirkzeit hat — der Fall war grün, und der Boss erzeugte trotzdem
      // keine Kollision: `casting` wird nur bei einem `interruptibleCast`
      // gesetzt, ein `strike` an derselben Stelle berührt es nicht.
      //
      // Ein Prüffall, der die halbe Bedingung prüft, ist grün und stumm.
      if (
        landet === jetzt.atMs &&
        (vorher.castTimeMs ?? 0) > 0 &&
        vorher.mechanic === 'interruptibleCast' &&
        jetzt.mechanic === 'interruptibleCast'
      ) {
        kollisionen.push(`${vorher.name} landet bei ${landet}, ${jetzt.name} beginnt dort`);
      }
    }

    expect(
      kollisionen.length,
      'Kein Eintrag beginnt in dem Takt, in dem der vorige landet — dann ' +
        'erzeugt dieser Boss die Kollision nicht, und jeder Prüffall, der ' +
        'ihn benutzt, ist stumm grün.',
    ).toBeGreaterThanOrEqual(2);
  });

  it('ist der einzige Boss, der das tut — sonst wäre der Befund kein Befund', () => {
    // Der Gegenfall: Träfe irgendein bestehender Boss die Bedingung schon,
    // wäre der Unterschied zwischen den Seiten nicht latent, sondern längst
    // aufgefallen. Diese Prüfung hält fest, warum es niemandem auffiel.
    const andere: string[] = [];
    for (const [id, eintrag] of BOSSES) {
      if (id === 'test-collision') continue;
      const timeline = eintrag.value.timeline;
      for (let i = 1; i < timeline.length; i++) {
        const vorher = timeline[i - 1];
        const jetzt = timeline[i];
        if (vorher === undefined || jetzt === undefined) continue;
        if (
          (vorher.castTimeMs ?? 0) > 0 &&
          vorher.atMs + vorher.castTimeMs === jetzt.atMs &&
          vorher.mechanic === 'interruptibleCast' &&
          jetzt.mechanic === 'interruptibleCast'
        ) {
          andere.push(`${id}: ${vorher.name} -> ${jetzt.name}`);
        }
      }
    }

    expect(
      andere,
      'Ein anderer Boss trifft die Kollision ebenfalls. Dann ist der Befund ' +
        'aus #331 nicht latent, und dieser Prüf-Boss ist überflüssig — oder ' +
        'die Bedingung ist falsch formuliert.',
    ).toEqual([]);
  });
});
