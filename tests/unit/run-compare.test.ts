/**
 * Bestlauf-Vergleich (Issue #82, AC1).
 *
 * Der Verlauf wird aus dem Protokoll ECHTER Durchlaeufe abgetastet, nicht aus
 * gesetzten Zahlen: Ein Protokoll, das ich selbst befuelle, belegt nur, dass
 * die Abtastung mit meinen Zahlen umgehen kann.
 *
 * Und jeder Nachweis bringt den Fall mit, in dem sich nichts aendern darf --
 * ein Vergleich, der IMMER eine Verbesserung meldet, waere schlimmer als
 * keiner.
 */
import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { spellIndex } from '../helpers/spell-index.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';
import { PressQueue } from '../../src/input/queue.js';
import { EVENT_CRIT, EVENT_DAMAGE, EVENT_DOT_TICK } from '../../src/core/log.js';
import { compareRuns, sampleDamageCurve } from '../../src/analysis/run-compare.js';

const KINDLE = spellIndex('kindle');
const SMOULDER = spellIndex('smoulder');
const FIGHT_MS = 30_000;

interface Lauf {
  readonly summary: RunSummary;
  readonly log: Session['log'];
}

/** Ein wirklich gespielter Durchlauf. */
function play(loadout: readonly number[], everyMs: number, fightMs = FIGHT_MS): Lauf {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const session = new Session(queue, combat);
  loadout.forEach((spell, slot) => {
    session.bind(slot, alsSpellIndex(spell));
  });
  session.beginCountdown(0);

  const ende = COUNTDOWN_MS + fightMs;
  let tick = 0;
  let presses = 0;
  for (let t = 0; t <= ende; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      queue.push(presses % loadout.length, tick, t, tick);
      presses++;
    }
    session.tick(t);
    tick++;
  }
  return { summary: session.summary(ende), log: session.log };
}

describe('AC1 — die Differenz je Kennzahl', () => {
  it('weist den besseren Lauf in beide Richtungen richtig aus', () => {
    const gut = play([KINDLE, SMOULDER], 1500).summary;
    const schlecht = play([KINDLE, SMOULDER], 3000).summary;

    const zeilen = compareRuns(schlecht, gut);
    const nach = (key: string) => zeilen.find((z) => z.key === key);

    // Weniger Schaden als der Bestlauf -- und bei DAMAGE ist mehr besser.
    expect(nach('damage')?.improved).toBe(false);
    expect(nach('damage')?.delta).toBeLessThan(0);

    // Mehr Leerlauf mit bereitem Zauber -- und dort ist WENIGER besser. Ein
    // Vergleich, der nur aufs Vorzeichen sieht, kaeme hier aufs Gegenteil.
    expect(nach('missed')?.better).toBe('lower');
    expect(nach('missed')?.delta).toBeGreaterThan(0);
    expect(nach('missed')?.improved).toBe(false);
  });

  it('meldet eine echte Verbesserung als solche', () => {
    const gut = play([KINDLE, SMOULDER], 1500).summary;
    const schlecht = play([KINDLE, SMOULDER], 3000).summary;
    const zeilen = compareRuns(gut, schlecht);
    expect(zeilen.find((z) => z.key === 'damage')?.improved).toBe(true);
    expect(zeilen.find((z) => z.key === 'missed')?.improved).toBe(true);
  });

  it('nennt Gleichstand keine Verbesserung', () => {
    // Der Fall, in dem sich nichts aendern darf: derselbe Lauf gegen sich
    // selbst. Ein Vergleich, der hier Fortschritt meldet, meldet ihn immer.
    const lauf = play([KINDLE, SMOULDER], 3000).summary;
    for (const zeile of compareRuns(lauf, lauf)) {
      expect(zeile.delta, zeile.key).toBe(0);
      // Die Kampfzeit bleibt UNBEKANNT, weil keiner der beiden Laeufe gewonnen
      // hat -- siehe den eigenen Prueffall dazu.
      expect(zeile.improved, zeile.key).toBe(zeile.key === 'time' ? null : false);
    }
  });

  it('vergleicht die Kampfzeit nur zwischen zwei SIEGEN', () => {
    // Wer nach 22 s stirbt, war nicht schneller als wer nach 30 s gewonnen
    // hat -- er war frueher fertig. Ohne diese Unterscheidung meldete der
    // Vergleich eine Niederlage als Fortschritt, und genau so stand es im
    // Bild, bevor es auffiel.
    const kurz = play([KINDLE, SMOULDER], 1500, 22_000).summary;
    const lang = play([KINDLE, SMOULDER], 1500, 30_000).summary;
    expect(kurz.victory).toBe(false);

    const zeit = compareRuns(kurz, lang).find((z) => z.key === 'time');
    expect(zeit?.delta).toBeLessThan(0);
    expect(zeit?.improved).toBeNull();

    // Der Wert selbst wird NICHT verschwiegen -- der Spieler sieht ihn im Bild,
    // und ihn wegzulassen hiesse, eine Zahl zu unterschlagen statt sie
    // einzuordnen.
    expect(zeit?.current).toBe(kurz.elapsedMs);
    expect(zeit?.best).toBe(lang.elapsedMs);

    // Und die uebrigen Kennzahlen bleiben vergleichbar: Die Ausnahme gilt
    // genau einer, nicht der ganzen Zeile.
    expect(compareRuns(kurz, lang).find((z) => z.key === 'damage')?.improved).not.toBeNull();
  });

  it('sagt beim ersten Durchlauf, dass es nichts zu vergleichen gibt', () => {
    // `null` heisst UNBEKANNT, nicht "nein". Ein false saehe aus wie ein
    // Rueckschritt gegen einen Bestlauf, den es nicht gibt.
    for (const zeile of compareRuns(play([KINDLE, SMOULDER], 1500).summary, null)) {
      expect(zeile.best, zeile.key).toBeNull();
      expect(zeile.delta, zeile.key).toBeNull();
      expect(zeile.improved, zeile.key).toBeNull();
    }
  });

  it('fuehrt den Fortschritt zuerst — er zaehlt auch ohne Sieg', () => {
    expect(compareRuns(play([KINDLE, SMOULDER], 1500).summary, null)[0]?.key).toBe('progress');
  });
});

describe('Der Verlauf als Kurve', () => {
  it('steigt und endet beim Gesamtschaden des Laufs', () => {
    const { summary, log } = play([KINDLE, SMOULDER], 1500);
    const kurve = sampleDamageCurve(log, summary.elapsedMs);

    expect(kurve.points.length).toBeGreaterThan(1);
    // Aufsummiert: nie fallend.
    for (let i = 1; i < kurve.points.length; i++) {
      expect(kurve.points[i]!).toBeGreaterThanOrEqual(kurve.points[i - 1]!);
    }
    // Und der letzte Punkt ist der Schaden aus der Kurzfassung -- kein
    // Eintrag faellt unter den Tisch.
    expect(kurve.points.at(-1)).toBe(summary.damage);
  });

  it('setzt jeden Punkt auf seinen ABSOLUTEN Zeitpunkt', () => {
    // Der eigentliche Nachweis, und er wird UNABHAENGIG hergeleitet: Punkt k
    // muss der Summe aller Schadens-Eintraege bis (k+1) x stepMs entsprechen,
    // direkt aus dem Protokoll nachgerechnet. Waere die Kurve auf die Laenge
    // des Laufs gestreckt, stimmte das an keiner Stelle ausser der letzten.
    const { summary, log } = play([KINDLE, SMOULDER], 1500);
    const kurve = sampleDamageCurve(log, summary.elapsedMs);

    const SCHADEN: readonly number[] = [EVENT_DAMAGE, EVENT_DOT_TICK, EVENT_CRIT];
    for (const k of [0, 3, 7, 12, kurve.points.length - 1]) {
      let summe = 0;
      for (let i = 0; i < log.length; i++) {
        if (SCHADEN.includes(log.typeAt(i)) && log.timeAt(i) < (k + 1) * kurve.stepMs) {
          summe += log.valueAt(i);
        }
      }
      expect(kurve.points[k], `Punkt ${k}`).toBe(summe);
    }
  });

  it('laesst den kuerzeren Lauf frueher enden, bei gleichem Raster', () => {
    // Zwei WIRKLICH verschieden lange Durchlaeufe, nicht zweimal derselbe mit
    // verschiedener Laengenangabe -- sonst zeigt der Fall nur, dass die
    // Punktzahl der uebergebenen Zahl folgt.
    const lang = play([KINDLE, SMOULDER], 1500, 30_000);
    const kurz = play([KINDLE, SMOULDER], 1500, 10_000);
    const a = sampleDamageCurve(lang.log, lang.summary.elapsedMs);
    const b = sampleDamageCurve(kurz.log, kurz.summary.elapsedMs);

    expect(b.points.length).toBeLessThan(a.points.length);
    expect(a.stepMs).toBe(b.stepMs);
    // Und bis dahin laufen sie gleich: gleiche Zusammenstellung, gleicher
    // Takt, gleiche absolute Zeit.
    for (let i = 0; i < b.points.length; i++) expect(a.points[i], `Punkt ${i}`).toBe(b.points[i]);
  });

  it('bleibt klein genug, um je Durchlauf abgelegt zu werden', () => {
    // 240 Zahlen bei vier Minuten Kampf sind etwas anderes als 10 000
    // Protokoll-Eintraege.
    const kurve = sampleDamageCurve(play([KINDLE, SMOULDER], 1500).log, 240_000);
    expect(kurve.points.length).toBe(241);
  });

  it('kommt mit einem leeren Durchlauf zurecht', () => {
    // Kein Schaden, keine Zeit -- die Kurve muss trotzdem eine Kurve sein.
    const { log } = play([KINDLE, SMOULDER], 1500);
    const leer = sampleDamageCurve(log, 0);
    expect(leer.points).toHaveLength(1);
    expect(Number.isFinite(leer.points[0]!)).toBe(true);
  });
});
