/**
 * Der Wächter über die Herleitung in `GAME-DESIGN` 221 (Issue #496).
 *
 * # Warum es diesen Prüffall gibt
 *
 * Die Begründung für „6 % je Zersetzungs-Stapel" nannte **4,5 s Abklingzeit,
 * drei Stapel und 18 % Gesamtwirkung**. Zum Zeitpunkt der Entscheidung
 * (#224, 2026-08-05) stimmte das. Danach wanderte `canker.cooldownMs` auf 2 s
 * und `MAX_DECAY_STACKS` auf 5 — und der Text blieb stehen.
 *
 * **Eine Design-Rechnung altert still.** Kein Übersetzer und kein Test sieht
 * eine Zahl in einem Markdown-Absatz an; sie wird erst falsch, wenn jemand sie
 * nachrechnet. Bei #496 hat das über einen Monat gedauert, und in der
 * Zwischenzeit war die Entscheidung auf einer Grundlage begründet, die es nicht
 * mehr gab.
 *
 * # Was dieser Fall prüft — und was ausdrücklich nicht
 *
 * Er prüft, dass die **vier Zahlen in der Tabelle** und die daraus gerechnete
 * Gesamtwirkung zu den Konstanten und Daten passen. Er prüft **nicht**, ob 30 %
 * die richtige Zahl sind — das ist eine Balance-Frage und liegt beim
 * Auftraggeber (#496 AC3).
 *
 * # Die Gegenprobe gehört dazu
 *
 * Ein Wächter, der bei seiner Einführung grün ist, hat nichts bewiesen. Zwei
 * Mutationen gefahren am 2026-08-07, beide gefangen — hier die tatsächliche
 * Ausgabe, nicht die erwartete:
 *
 *   MAX_DECAY_STACKS 5 -> 6      2 Fälle rot
 *     Die Zahl der Stapel im Text passt nicht zu MAX_DECAY_STACKS:
 *       expected 5 to be 6
 *     Die Zeit bis zur vollen Wirkung im Text passt nicht zu den Daten (10 s)
 *
 *   DECAY_STACK_DURATION_MS 12_000 -> 20_000      1 Fall rot
 *     Die Haltbarkeit im Text passt nicht zu DECAY_STACK_DURATION_MS (20 s):
 *       expected '...' to contain '**20 s**'
 *
 * Beachtenswert am ersten Lauf: Die Gesamtwirkung selbst blieb grün, weil der
 * Fall die Stapelzahl **vor** dem Produkt prüft und dort schon abbricht. Wer
 * nur auf „36 %" gewartet hätte, hätte den Wächter für schwächer gehalten, als
 * er ist — die Meldung nennt die Ursache, nicht die Folge.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

import spells from '../../data/spells/putrefaction.json';
import {
  DECAY_STACK_DURATION_MS,
  DECAY_STACK_PERCENT,
  MAX_DECAY_STACKS,
} from '../../src/core/combat';

const DOKUMENT = readFileSync(join(process.cwd(), 'docs/GAME-DESIGN.md'), 'utf8');

/**
 * Der Absatz, um den es geht — abgegrenzt am Anfang und am nächsten Kapitel.
 *
 * Nicht das ganze Dokument durchsuchen: „30 %" steht anderswo ebenfalls, und
 * ein Treffer an falscher Stelle wäre ein grüner Fall ohne Aussage.
 */
function herleitung(): string {
  const anfang = DOKUMENT.indexOf('**Warum 6 % und nicht 3 %.**');
  expect(
    anfang,
    'Der Absatz "Warum 6 % und nicht 3 %" steht nicht mehr im Dokument',
  ).toBeGreaterThan(-1);
  const ende = DOKUMENT.indexOf('\n\n', DOKUMENT.indexOf('design-herleitung.test.ts', anfang));
  return DOKUMENT.slice(anfang, ende > anfang ? ende : anfang + 2500);
}

describe('GAME-DESIGN 221 — die Herleitung der 6 % (Issue #496)', () => {
  it('nennt die Gesamtwirkung, die aus den Konstanten folgt', () => {
    const erwartet = MAX_DECAY_STACKS * DECAY_STACK_PERCENT;
    const text = herleitung();

    const treffer = text.match(/Gesamtwirkung:\s*(\d+)\s*×\s*(\d+)\s*%\s*=\s*(\d+)\s*%/u);
    expect(treffer, 'Die Zeile "Gesamtwirkung: N × M % = X %" fehlt im Absatz').not.toBeNull();

    const [, stapel, jeStapel, gesamt] = treffer as RegExpMatchArray;
    expect(Number(stapel), 'Die Zahl der Stapel im Text passt nicht zu MAX_DECAY_STACKS').toBe(
      MAX_DECAY_STACKS,
    );
    expect(Number(jeStapel), 'Der Prozentsatz im Text passt nicht zu DECAY_STACK_PERCENT').toBe(
      DECAY_STACK_PERCENT,
    );
    expect(
      Number(gesamt),
      `GAME-DESIGN 221 nennt eine Gesamtwirkung, die nicht zu den Konstanten passt: erwartet ${erwartet} %`,
    ).toBe(erwartet);
  });

  it('nennt die Haltbarkeit und die Abklingzeit des Erzeugers, wie sie in den Daten stehen', () => {
    const text = herleitung();
    const canker = spells.spells.find((s) => s.id === 'canker');
    expect(canker, 'CANKER steht nicht mehr in putrefaction.json').toBeDefined();

    const haltbarkeitS = DECAY_STACK_DURATION_MS / 1000;
    const abklingzeitS = (canker?.cooldownMs ?? 0) / 1000;

    expect(
      text,
      `Die Haltbarkeit im Text passt nicht zu DECAY_STACK_DURATION_MS (${haltbarkeitS} s)`,
    ).toContain(`**${haltbarkeitS} s**`);
    expect(
      text,
      `Die Abklingzeit im Text passt nicht zu canker.cooldownMs (${abklingzeitS} s)`,
    ).toContain(`**${abklingzeitS} s**`);
  });

  it('nennt die Zeit bis zur vollen Wirkung, und sie stimmt rechnerisch', () => {
    // **Der Fall, der am ehesten still altert.** Die Zeit folgt aus zwei
    // Werten, die unabhängig voneinander wandern können — die Zahl der Stapel
    // und die Abklingzeit. Ohne diesen Fall bliebe „acht Sekunden" stehen,
    // während die Tabelle darüber längst korrigiert wäre.
    const canker = spells.spells.find((s) => s.id === 'canker');
    const bisVoll = ((MAX_DECAY_STACKS - 1) * (canker?.cooldownMs ?? 0)) / 1000;

    expect(
      bisVoll * 1000,
      'Die volle Zahl Stapel ist gar nicht erreichbar — der erste verfällt vorher',
    ).toBeLessThan(DECAY_STACK_DURATION_MS);

    expect(
      herleitung(),
      `Die Zeit bis zur vollen Wirkung im Text passt nicht zu den Daten (${bisVoll} s)`,
    ).toMatch(new RegExp(`\\*\\*${bisVoll === 8 ? 'acht' : String(bisVoll)} Sekunden\\*\\*`, 'u'));
  });
});
