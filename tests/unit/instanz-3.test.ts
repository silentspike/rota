/**
 * Die dritte Instanz — vier Bosse und das Instanz-Objekt (Issue #421c).
 *
 * # Was hier geprüft wird
 *
 * | Frage                                    | Warum sie gestellt wird                     |
 * | ---------------------------------------- | ------------------------------------------- |
 * | Sind alle vier **bezwingbar**?           | #672: `test-doomsayer` ist es nicht         |
 * | **Unterscheiden** sie sich?              | Vier gleiche Bosse sind ein Boss viermal    |
 * | Ist die **Freischaltung** dicht?         | 12.7 — Hauptpfad ist „Instanz abgeschlossen" |
 *
 * # Die Bezwingbarkeit wird gespielt, nicht behauptet
 *
 * `#672` hat gezeigt, wie eine unbezwingbare Begegnung aussieht: `test-doomsayer`
 * kommt auf **5168 von 6400** Schaden und kann nicht gewinnen — das fiel erst
 * auf, als jemand einen Lauf bis zum Ende fuhr. Ein Prüffall, der nur die Daten
 * liest, hätte ihn nie gefunden.
 *
 * Deshalb steht hier je Boss eine **gemessene Zahl**: der Schaden aus einem
 * vollen Lauf, gegen die Lebenspunkte des Bosses.
 */
import { describe, expect, it } from 'vitest';

import { BOSSES, bossById, INSTANCES, instanceById, type BossDef } from '../../src/core/data.js';
import { runReference } from '../../src/devtools/reference-run.js';
import { freigeschalteteInstanzen } from '../../src/progression/boss-select.js';

/** Die vier der dritten Instanz, in der Reihenfolge des Kampfes. */
const VIER = ['boss-3-1', 'boss-3-2', 'boss-3-3', 'boss-3-4'] as const;

function boss(id: string): BossDef {
  const b = bossById(id);
  expect(b, `Boss ${id} fehlt`).toBeDefined();
  return b as BossDef;
}

/**
 * Einen vollen Lauf nach der Prioritätenliste `good` rechnen.
 *
 * # Die erste Fassung maß mit EINEM Zauber, und das war der Fehler
 *
 * Sie drückte alle 1500 ms KINDLE — und ließ alle vier neuen Bosse als
 * „unbezwingbar" durchfallen. Die Träger-Zusicherung hat es aufgedeckt:
 *
 * ```text
 *   first-crucible  4200 HP   4000 Schaden   kein Sieg
 *   boss-1-1        5200 HP   3000 Schaden   kein Sieg
 *   boss-2-4        5000 HP    100 Schaden   kein Sieg
 * ```
 *
 * **Kein einziger der neun bestehenden Bosse fiel.** Eine Anordnung, in der
 * auch der ausgelieferte Bestand scheitert, misst nicht die Begegnung,
 * sondern sich selbst — dieselbe Klasse wie ein Vergleich zweier Nullen.
 *
 * `runReference('good', …)` fährt stattdessen die Prioritätenliste, die auch
 * den Referenzlauf `4240 @ 40510` erzeugt. Damit fällt `first-crucible`, und
 * die Zahl sagt etwas über den Boss.
 */
function lauf(bossId: string): { damage: number; killedAtMs: number; victory: boolean } {
  const ergebnis = runReference('good', { bossId }) as {
    damage: number;
    killedAtMs: number;
  };
  return {
    damage: ergebnis.damage,
    killedAtMs: ergebnis.killedAtMs,
    // `killedAtMs` ist -1, solange der Boss steht.
    victory: ergebnis.killedAtMs >= 0,
  };
}

describe('Die vier sind bezwingbar — gespielt, nicht behauptet', () => {
  it.each(VIER)('%s fällt in einem vollen Lauf', (id) => {
    // **Die Zahl steht in der Meldung, nicht in der Bedingung.** Ein fester
    // Erwartungswert wäre bei jeder Balance-Änderung fällig, ohne je etwas
    // anderes zu sagen; die Bedingung ist „er fällt", die Zahl erklärt warum.
    const b = boss(id);
    const ergebnis = lauf(id);
    expect(
      ergebnis.victory,
      `${id} überlebt: ${ergebnis.damage} Schaden von ${b.hp} Lebenspunkten — ` +
        `das ist der Zustand aus #672 (test-doomsayer: 5168 von 6400).`,
    ).toBe(true);
  });

  it('GEGENPROBE: die Anordnung KANN scheitern', () => {
    // Ohne diese Zeile wäre nicht zu sehen, ob der Lauf überhaupt verlieren
    // kann — vier Siege gegen eine Anordnung, die immer gewinnt, sagen nichts.
    //
    // **Der Träger ist ein bestehender Boss, kein Prüf-Boss.** `boss-1-2`
    // fällt mit derselben Rotation nicht (1580 von 6400) — er ist damit der
    // ausgelieferte Beleg dafür, dass die Latte echt ist.
    expect(lauf('boss-1-2').victory).toBe(false);
  });
});

describe('Die vier unterscheiden sich — nicht nur in Lebenspunkten', () => {
  it('trägt jeder einen anderen Baustein', () => {
    // **Als Liste festgenagelt, nicht als Zahl.** „Vier verschiedene" bliebe
    // grün, wenn zwei denselben Baustein trügen und ein dritter zwei.
    const profil = (b: BossDef): string =>
      [
        Object.values(b.resistances).some((r) => r !== 1) ? 'widerstand' : '',
        b.energy !== null ? 'energie' : '',
        b.stacks !== null ? 'stapel' : '',
        b.doom !== null ? 'verhaengnis' : '',
        b.phases.length > 0 ? 'phasen' : '',
      ]
        .filter((x) => x !== '')
        .join('+');

    expect(profil(boss('boss-3-1'))).toBe('widerstand');
    expect(profil(boss('boss-3-2'))).toBe('energie');
    expect(profil(boss('boss-3-3'))).toBe('stapel');
    expect(profil(boss('boss-3-4'))).toBe('verhaengnis+phasen');
  });

  it('und der Unterschied wirkt im Kampf, nicht nur in den Daten', () => {
    // Ein Datenfeld, das niemand liest, wäre ein Unterschied auf dem Papier.
    // Vier verschiedene Schadenszahlen belegen, dass die Bausteine den Verlauf
    // tatsächlich verändern.
    // Gemessen wird die ZEIT bis zum Fall, nicht der Schaden: Der Schaden
    // liegt bei allen vier knapp über den Lebenspunkten (der Boss fällt am
    // Ende), die Zeit dagegen zeigt, wie unterschiedlich der Weg dorthin ist.
    const zeiten = VIER.map((id) => lauf(id).killedAtMs);
    expect(new Set(zeiten).size, `vier gleiche Läufe: ${zeiten.join(', ')}`).toBe(4);
  });

  it('holt drei Bausteine ins Spiel, die es bisher nur im Prüflauf gab', () => {
    // Vor #421c trug KEIN ausgelieferter Boss eine Energieleiste, Stapel oder
    // einen Widerstand ungleich 1 — gemessen am 2026-08-10. Die Mechaniken
    // standen gebaut und geprüft da und kamen im Spiel nirgends vor.
    const echte = [...VIER, 'boss-1-1', 'boss-2-1'].map(boss);
    expect(
      echte.some((b) => b.energy !== null),
      'kein Boss mit Energieleiste',
    ).toBe(true);
    expect(
      echte.some((b) => b.stacks !== null),
      'kein Boss mit Stapeln',
    ).toBe(true);
    expect(
      echte.some((b) => Object.values(b.resistances).some((r) => r !== 1)),
      'kein Boss mit Widerstand',
    ).toBe(true);
  });
});

describe('Das Instanz-Objekt und die Freischaltung (12.7)', () => {
  it('führt alle vier Instanzen, nach Reihenfolge sortiert', () => {
    expect(INSTANCES.map((i) => i.id)).toEqual([
      'prologue',
      'instance-1',
      'instance-2',
      'instance-3',
    ]);
  });

  it('verlangt für instance-3 den Abschluss von instance-2', () => {
    expect(instanceById('instance-3')?.requires).toBe('instance-2');
  });

  it('GEGENPROBE: die erste Instanz verlangt nichts', () => {
    // Ohne sie wäre eine Kette, in der JEDE Instanz etwas verlangt, ebenfalls
    // grün — und niemand käme je hinein.
    expect(instanceById('instance-1')?.requires).toBeNull();
    expect(instanceById('prologue')?.requires).toBeNull();
  });

  it('gibt jeder Instanz eine Beute-Nummer ab 1', () => {
    // `itemLevelOf` und `qualityCapOf` rechnen mit dieser Zahl (13.4). Eine 0
    // fiele dort auf 1 zurück, ohne dass es jemand merkte.
    for (const instanz of INSTANCES) {
      expect(instanz.number, `${instanz.id} hat keine Nummer ab 1`).toBeGreaterThanOrEqual(1);
    }
  });

  it('öffnet instance-3 NICHT, solange instance-2 offen ist', () => {
    // **Die Gegenprobe zuerst**, weil sie die eigentliche Aussage trägt: Eine
    // Freischaltung, die immer offen ist, wäre keine.
    const ohne = freigeschalteteInstanzen([], BOSSES);
    expect(ohne).not.toContain('instance-3');
    // Und die davor liegenden sind trotzdem offen — sonst käme niemand hinein.
    expect(ohne).toContain('prologue');
    expect(ohne).toContain('instance-1');
  });

  it('öffnet sie, sobald ALLE vier Bosse von instance-2 liegen', () => {
    const alleVier = ['boss-2-1', 'boss-2-2', 'boss-2-3', 'boss-2-4'];
    expect(freigeschalteteInstanzen(alleVier, BOSSES)).toContain('instance-3');
  });

  it('und NICHT, wenn drei von vier liegen', () => {
    // **Vollständig heißt jeder.** Ohne diese Zeile wäre „mindestens einer"
    // ebenso grün — und die Freischaltung hinge daran, welchen Boss jemand
    // zuerst wählt.
    const dreiVonVier = ['boss-2-1', 'boss-2-2', 'boss-2-3'];
    expect(freigeschalteteInstanzen(dreiVonVier, BOSSES)).not.toContain('instance-3');
  });

  it('lässt sich nicht mit den FALSCHEN vier öffnen', () => {
    // Vier Siege, aber aus instance-1: Wer nur zählt statt zuzuordnen, wäre
    // hier grün.
    const falsche = ['boss-1-1', 'boss-1-2', 'boss-1-3', 'boss-1-4'];
    expect(freigeschalteteInstanzen(falsche, BOSSES)).not.toContain('instance-3');
  });

  it('ordnet jeden ausgelieferten Boss einer bekannten Instanz zu', () => {
    // **Das ist die Verbindung, die es vorher nicht gab.** Ein Boss mit einer
    // Instanz-Kennung, die kein Objekt hat, erschiene in keiner Auswahl — und
    // zwar lautlos.
    const kennungen = new Set(INSTANCES.map((i) => i.id));
    for (const id of [...VIER, 'boss-1-1', 'boss-2-4', 'first-crucible']) {
      const b = boss(id);
      expect(kennungen.has(b.instance), `${id} liegt in "${b.instance}", das es nicht gibt`).toBe(
        true,
      );
    }
  });
});
