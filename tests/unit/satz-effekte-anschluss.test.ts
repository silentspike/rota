// @vitest-environment jsdom

/**
 * Kommt ein Satz-Effekt beim Ausrüstungswechsel im Kampf an? (13.5, #480)
 *
 * # Woher dieser Fall kommt
 *
 * Aus einem Schaden, nicht aus einer Überlegung. Gemessen am 2026-08-13 auf
 * `origin/main`:
 *
 * ```text
 *   setWorn         2 Aufrufer   src/core/app.ts:209 und :635
 *   setSetEffects   0 Aufrufer   nur die Definition in combat.ts
 * ```
 *
 * Sechs Rotations-Effekte — Ashen Regimen, The Whole Work, Slow Rot, The
 * Sealed Vessel, The Long Grip, Perpetual Draft — waren gebaut, einzeln
 * geprüft, mutationsbelegt und paritätsgesichert. `hasSetEffect` gab für
 * jeden Spieler trotzdem immer `false`, weil `setEffects` nie befüllt wurde.
 * Die Leitung war bis `rotationEffectsFrom` gebaut und endete dort an der Wand.
 *
 * **Warum kein einziger der sechs Prüffälle das sah:** Jeder rief
 * `setSetEffects` selbst auf. Ein Fall, der den Setter bedient, IST die
 * fehlende Speisung — er kann sie nicht bemerken.
 *
 * # Was dieser Fall prüft, und was ausdrücklich nicht
 *
 * Er fährt den Weg des SPIELERS: Ausrüstung anlegen über `App.setItems`, und
 * dann messen, dass der Effekt im Kampf ankommt. Nicht „gibt es einen
 * Aufrufer" — das wäre ein Zähler, der auch dann grün bliebe, wenn die Zeile
 * dasteht und trotzdem nichts durchkommt.
 *
 * **Er fängt nur die Satz-Effekte, nicht die Klasse.** Ein Wächter über alle
 * `Combat`-Setter wäre die naheliegende Verallgemeinerung und ist nicht
 * baubar: Von acht Settern würden vier zu Recht anschlagen —
 * `setDamageFactor` wird intern von `setWorn` gespeist, und drei Setter sollen
 * nur aus den Devtools kommen. Ein Wächter, der zur Hälfte aus Ausnahmen
 * besteht, misst die Ausnahmeliste und nicht die Sache.
 *
 * # Warum sechs Prüfstücke
 *
 * Die Rotations-Effekte sitzen auf der SECHSTEN Satzstufe (`SET_TIERS` =
 * `[2, 4, 6]`). Der Prüfsatz hatte vier Teile; mit vieren bleibt
 * `rotationEffectsFrom` leer, und eine leere Liste kommt auch ohne die Zeilen
 * in `app.ts` an — der Fall wäre trivial grün gewesen. `test-set-5` und
 * `test-set-6` schliessen die Lücke.
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';
import { rotationEffectsFrom } from '../../src/gear/catalog.js';
import { SET_EFFECT_ASHEN_REGIMEN } from '../../src/gear/sets.js';

/** Die sechs Teile des Prüfsatzes, alle angelegt. */
function sechsTeile() {
  return [1, 2, 3, 4, 5, 6].map((n) => ({
    itemId: `test-set-${String(n)}`,
    upgradeSteps: 0,
    equippedSlot: n,
  }));
}

/** Dieselben Stücke, aber nur fünf davon angelegt. */
function fuenfTeile() {
  return sechsTeile().map((item, i) => (i === 5 ? { ...item, equippedSlot: null } : item));
}

/**
 * jsdom bringt keinen Canvas-2D-Kontext mit, und `Surface` wirft ohne ihn.
 *
 * **Als Proxy und nicht als Liste von Attrappen-Methoden**: Der Aufbau ruft
 * heute `setTransform`, `fillRect` und `fillStyle`. Eine ausgeschriebene Liste
 * bräche bei der nächsten Zeichen-Änderung, und dieser Fall prüft nicht das
 * Zeichnen — er prüft, ob ein Satz-Effekt ankommt. Der Proxy schluckt alles
 * und gibt sich für jeden Zugriff selbst zurück.
 */
function stubbeCanvas(): void {
  const ctx: unknown = new Proxy(
    {},
    {
      get: (_ziel, name) => (name === 'canvas' ? undefined : () => ctx),
      set: () => true,
    },
  );
  // Die Überladungen von `getContext` decken 2d, webgl und bitmaprenderer ab;
  // ein einzelner Rückgabetyp passt auf keine davon. Der Fall braucht nur die
  // 2d-Fassung, deshalb der Umweg über `unknown`.
  HTMLCanvasElement.prototype.getContext = ((): CanvasRenderingContext2D =>
    ctx as CanvasRenderingContext2D) as unknown as typeof HTMLCanvasElement.prototype.getContext;
}

function baueApp(): App {
  const parent = document.createElement('div');
  document.body.appendChild(parent);
  return new App(parent);
}

beforeEach(() => {
  stubbeCanvas();
  localStorage.clear();
  document.body.innerHTML = '';
});

describe('AC1 — der Wechsel im laufenden Spiel bringt den Effekt an', () => {
  it('sechs angelegte Satzteile lösen ihn im Kampf aus', () => {
    const app = baueApp();
    // Vorher nicht — sonst zeigte der Fall unten nur, dass der Effekt
    // irgendwann irgendwie da ist.
    expect(app.combat.hasSetEffect(SET_EFFECT_ASHEN_REGIMEN)).toBe(false);

    app.setItems(sechsTeile());

    expect(app.combat.hasSetEffect(SET_EFFECT_ASHEN_REGIMEN)).toBe(true);
  });

  it('fünf lösen ihn nicht aus — die Schwelle wandert mit', () => {
    // Die Gegenprobe zur Zeile oben. Käme der Effekt auch bei fünf Teilen an,
    // wäre die Leitung zwar da, aber sie trüge etwas Falsches.
    const app = baueApp();
    app.setItems(fuenfTeile());
    expect(app.combat.hasSetEffect(SET_EFFECT_ASHEN_REGIMEN)).toBe(false);
  });

  it('und das Ablegen nimmt ihn wieder weg', () => {
    // Ein Anschluss, der nur in eine Richtung trägt, liesse den Effekt nach
    // dem ersten Anlegen für immer stehen.
    const app = baueApp();
    app.setItems(sechsTeile());
    expect(app.combat.hasSetEffect(SET_EFFECT_ASHEN_REGIMEN)).toBe(true);
    app.setItems(fuenfTeile());
    expect(app.combat.hasSetEffect(SET_EFFECT_ASHEN_REGIMEN)).toBe(false);
  });
});

describe('AC2 — auch der Aufbau beim Start speist ihn, nicht nur der Wechsel', () => {
  it('ein Spielstand mit sechs Teilen wirkt sofort', () => {
    // Die ZWEITE der beiden Stellen in `app.ts`. Ohne sie käme der Effekt
    // erst beim nächsten Ausrüstungswechsel an — ein Spieler, der das Spiel
    // fertig ausgerüstet neu lädt, stünde ohne da.
    const erste = baueApp();
    erste.setItems(sechsTeile());

    // Neu laden heisst: neue App auf demselben Spielstand.
    const zweite = baueApp();
    expect(zweite.combat.hasSetEffect(SET_EFFECT_ASHEN_REGIMEN)).toBe(true);
  });
});

describe('AC3 — ZUSICHERUNG: die Prüfdaten tragen die Aussage', () => {
  it('sechs Teile ergeben wirklich einen Effekt, fünf wirklich keinen', () => {
    // Ohne diesen Fall prüfte AC1 möglicherweise zwei leere Listen
    // gegeneinander — und wäre grün, ohne je einen Effekt gesehen zu haben.
    expect(rotationEffectsFrom(sechsTeile())).toContain(SET_EFFECT_ASHEN_REGIMEN);
    expect(rotationEffectsFrom(fuenfTeile())).toEqual([]);
  });
});
