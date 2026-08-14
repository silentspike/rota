// @vitest-environment jsdom

/**
 * Kommt ein Talentpunkt vom KLICK bis in den Kampf? (Issue #802)
 *
 * # Woher dieser Fall kommt
 *
 * Aus einem Befund, nicht aus einer Überlegung. Gemessen am 2026-08-13:
 *
 * ```text
 *   App.setTalents   1 Aufrufer ausserhalb der Tests — in devtools
 * ```
 *
 * Sechs Bäume, achtzehn Wege, vier Schlüsseltalente, ~50 Punkte auf 100
 * Stufen: alles gebaut, alles geprüft, alles mit Rust-Parität — und kein
 * Spieler konnte einen einzigen Punkt setzen. Die Kette war bis
 * `persistTalents` fertig und endete davor an der Wand.
 *
 * # Warum der Fall KLICKT statt `setTalents` zu rufen
 *
 * Genau daran hat es in #480 gelegen: „Ein Fall, der den Setter bedient, IST
 * die fehlende Speisung — er kann sie nicht bemerken." Alle bestehenden
 * Talent-Prüffälle übergeben die Verteilung im Konstruktor oder rufen
 * `setTalents` selbst. Jeder davon war grün, während die Oberfläche fehlte.
 *
 * Dieser Fall sucht deshalb den Knopf im Baum und drückt ihn.
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';

/**
 * jsdom bringt keinen Canvas-2D-Kontext mit, und `Surface` wirft ohne ihn.
 * Derselbe Proxy wie in `satz-effekte-anschluss.test.ts`, aus demselben Grund:
 * Dieser Fall prüft nicht das Zeichnen.
 */
function stubbeCanvas(): void {
  const ctx: unknown = new Proxy(
    {},
    {
      get: (_ziel, name) => (name === 'canvas' ? undefined : () => ctx),
      set: () => true,
    },
  );
  HTMLCanvasElement.prototype.getContext = ((): CanvasRenderingContext2D =>
    ctx as CanvasRenderingContext2D) as unknown as typeof HTMLCanvasElement.prototype.getContext;
}

function baueApp(): App {
  const parent = document.createElement('div');
  document.body.appendChild(parent);
  return new App(parent);
}

/**
 * Eine App auf einer Stufe, die Punkte hergibt.
 *
 * **Gemessen, nicht angenommen:** `talentPointLevels` beginnt bei 2 — auf
 * Stufe 1 hat der Spieler NULL Punkte. Mein erster Entwurf klickte auf Stufe 1
 * und meldete „der Klick kommt im Kampf nicht an", wo in Wahrheit gar nichts
 * zu verteilen war. Die Ansicht war richtig, der Fall falsch.
 *
 *     talentPointLevels   [2, 4, 6, 8, 10, ...]
 *     Stufe 1  ->  0 Punkte      Stufe 2  ->  1      Stufe 5  ->  2
 */
function appMitPunkten(stufe = 4): App {
  const app = baueApp();
  app.setPlayerLevel(stufe);
  return app;
}

/** Der Knopf, der einen Rang in diesem Talent hinzufügt. */
function knopf(talentId: string, aktion: 'add' | 'remove'): HTMLButtonElement | null {
  return document.querySelector<HTMLButtonElement>(
    `button[data-talent="${talentId}"][data-talent-action="${aktion}"]`,
  );
}

beforeEach(() => {
  stubbeCanvas();
  localStorage.clear();
  document.body.innerHTML = '';
});

describe('AC1 — die Bäume stehen im Player Home', () => {
  it('alle sechs sind da, mit ihren Wegen', () => {
    appMitPunkten();
    const knoepfe = document.querySelectorAll('button[data-talent]');
    expect(knoepfe.length, 'kein einziger Talent-Knopf im Player Home').toBeGreaterThan(0);

    // Je Baum mindestens der Stamm — sonst fehlte ein ganzer Baum, ohne dass
    // die Zahl oben es verriete.
    for (const stamm of [
      'calcination-trunk',
      'fixation-trunk',
      'sublimation-trunk',
      'coagulation-trunk',
      'putrefaction-trunk',
      'global-trunk',
    ]) {
      expect(knopf(stamm, 'add'), `${stamm} fehlt in der Ansicht`).not.toBeNull();
    }
  });
});

describe('AC4 — der Weg vom Klick bis in den Kampf', () => {
  it('ein Klick setzt den Rang, und der Kampf rechnet damit', () => {
    const app = appMitPunkten();
    // **Vorher nicht** — sonst zeigte der Fall unten nur, dass irgendwann
    // irgendwas verteilt ist.
    expect(app.combat.talentAllocation['calcination-trunk'] ?? 0).toBe(0);

    knopf('calcination-trunk', 'add')?.click();

    expect(
      app.combat.talentAllocation['calcination-trunk'],
      'der Klick kommt im Kampf nicht an',
    ).toBe(1);
  });

  it('und die WIRKUNG ist messbar, nicht nur die Zahl', () => {
    // Der eigentliche Nachweis: Ein Rang im Stamm hebt den Schadensfaktor.
    // Ohne diesen Fall bliebe offen, ob die Verteilung nur abgelegt wird.
    const app = appMitPunkten();
    const vorher = app.combat.damageFactors.talents;

    knopf('calcination-trunk', 'add')?.click();

    expect(
      app.combat.damageFactors.talents,
      'der Rang steht im Kampf, wirkt aber nicht auf den Schaden',
    ).toBeGreaterThan(vorher);
  });

  it('das Zurücknehmen nimmt ihn wieder weg', () => {
    // Ein Anschluss, der nur in eine Richtung trägt, liesse den ersten Punkt
    // für immer stehen — und ein Spieler könnte sich nie umentscheiden.
    const app = appMitPunkten();
    knopf('calcination-trunk', 'add')?.click();
    expect(app.combat.talentAllocation['calcination-trunk']).toBe(1);

    knopf('calcination-trunk', 'remove')?.click();
    expect(
      app.combat.talentAllocation['calcination-trunk'] ?? 0,
      'der Punkt lässt sich nicht lösen',
    ).toBe(0);
  });
});

describe('AC2 — der Spielstand nimmt es auf', () => {
  it('nach dem Neuladen steht der Punkt noch', () => {
    const erste = appMitPunkten();
    knopf('calcination-trunk', 'add')?.click();
    expect(erste.combat.talentAllocation['calcination-trunk']).toBe(1);

    // Neu aufbauen wie nach einem Neuladen — derselbe localStorage.
    document.body.innerHTML = '';
    const zweite = baueApp();
    expect(
      zweite.combat.talentAllocation['calcination-trunk'],
      'der Punkt hat das Neuladen nicht überlebt',
    ).toBe(1);
  });
});

describe('AC3 — die Grenze aus talentPointsAtLevel hält', () => {
  it('auf Stufe 2 lässt sich kein zweiter Punkt setzen', () => {
    // `talentPointLevels` beginnt bei 2: Stufe 2 gibt GENAU einen Punkt. Der
    // zweite Klick darf nichts bewirken — sonst verteilte ein Spieler mehr,
    // als er verdient hat.
    const app = appMitPunkten(2);
    knopf('calcination-trunk', 'add')?.click();
    expect(app.combat.talentAllocation['calcination-trunk']).toBe(1);

    knopf('fixation-trunk', 'add')?.click();
    expect(
      app.combat.talentAllocation['fixation-trunk'] ?? 0,
      'ein zweiter Punkt ging durch, obwohl Stufe 2 nur einen gibt',
    ).toBe(0);
  });
});

describe('AC3 von #85 — jeder Prozess-Name trägt seinen Klartext', () => {
  /**
   * Der Befund, aus dem dieser Block entstand (2026-08-14, im Browser
   * gemessen): Von zehn Vorkommen der fünf Schulnamen im Dokument trugen
   * **fünf** den Klartext und fünf nicht — und die fünf ohne waren allesamt
   * die Überschriften der Talentbäume.
   *
   * ```text
   *   composition-group-title   "CALCINATION (Damage over time)"   richtig
   *   talents-tree-name         "CALCINATION"                      Lücke
   * ```
   *
   * **Warum dieser Fall über ALLE Bäume läuft und nicht über einen.** Genau
   * daran wäre die Lücke vorbeigegangen: Wer ein Beispiel prüft, prüft die
   * Stelle, die schon stimmt. Das Kriterium sagt „JEDER Prozess-Name", und
   * eine Prüfung, die das Wort ernst nimmt, muss zählen statt zu stichproben.
   */
  const PROZESSE: readonly (readonly [string, string])[] = [
    ['CALCINATION', 'Damage over time'],
    ['FIXATION', 'Control'],
    ['SUBLIMATION', 'Cooldown control'],
    ['COAGULATION', 'Survival'],
    ['PUTREFACTION', 'Weakening'],
  ];

  it('alle fünf Baum-Überschriften nennen ihren Klartext', () => {
    appMitPunkten();
    const ueberschriften = [...document.querySelectorAll('.talents-tree-name')].map(
      (e) => e.textContent ?? '',
    );
    // ZUSICHERUNG: Es gibt die Überschriften überhaupt. Ohne sie liefe die
    // Schleife darunter über eine leere Liste und bestätigte alles.
    expect(ueberschriften.length, 'keine Baum-Überschrift im Dokument').toBeGreaterThanOrEqual(6);

    for (const [name, klartext] of PROZESSE) {
      const zeile = ueberschriften.find((t) => t.includes(name));
      expect(zeile, `der Baum ${name} fehlt in der Ansicht`).toBeDefined();
      expect(zeile, `${name} steht ohne Klartext da`).toContain(klartext);
    }
  });

  it('`global` bleibt ohne Klartext — er ist kein Prozess', () => {
    // Die Gegenseite, die gleich bleiben muss: `withPlainText` erfindet
    // nichts. Ohne diesen Fall wäre eine Umsetzung grün, die jedem Baum
    // irgendeinen Klammerzusatz anhängt.
    appMitPunkten();
    const global = [...document.querySelectorAll('.talents-tree-name')]
      .map((e) => e.textContent ?? '')
      .find((t) => t.includes('GLOBAL'));
    expect(global, 'der Baum GLOBAL fehlt').toBeDefined();
    expect(global?.trim()).toBe('GLOBAL');
  });

  it('und die Zusammenstellung tut es weiterhin', () => {
    // Die Stelle, die schon vor #85 stimmte. Sie steht hier, damit eine
    // Änderung an `withPlainText` nicht die eine Ansicht repariert und die
    // andere bricht.
    appMitPunkten();
    const titel = [...document.querySelectorAll('.composition-group-title')].map(
      (e) => e.textContent ?? '',
    );
    expect(titel.length, 'keine Schul-Überschrift in der Zusammenstellung').toBeGreaterThan(0);
    for (const [name, klartext] of PROZESSE) {
      const zeile = titel.find((t) => t.includes(name));
      expect(zeile, `${name} fehlt in der Zusammenstellung`).toBeDefined();
      expect(zeile, `${name} steht dort ohne Klartext`).toContain(klartext);
    }
  });
});
