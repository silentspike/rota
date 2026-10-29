// @vitest-environment jsdom

/**
 * Kommen die Stufen beim Spieler an? (Issue #1177, Abschnitt 2)
 *
 * # Über die ganze App, nicht über die Funktionen
 *
 * `offeneStufen` und `bossAtDifficulty` haben eigene Prüffälle
 * (`difficulty-unlock.test.ts`, `difficulty-stages.test.ts`). Hier geht es um
 * den Weg: Siege im laufenden Spiel → Spielstand → Auswahl im Player Home →
 * Kampf gegen die abgeleitete Fassung → Beute auf ihrer Stufe. Dieselbe Lehre
 * wie bei `freigeschalteteInstanzen` (#421c) und `setSetEffects` (#774): Eine
 * Regel mit grünen Fällen, die niemand ruft, ist keine.
 *
 * Die Siege laufen wie in `archiv-anschluss.test.ts` über `session.tick` —
 * `onFinish` ist der Weg, den auch der Spieler nimmt, und dort entsteht der
 * Schlüssel, an dem die Freischaltung hängt.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';
import { bossAtDifficulty } from '../../src/core/data.js';
import { difficultyOf } from '../../src/core/difficulty.js';
import { markKey } from '../../src/analysis/progress-mark.js';
import { DIFFICULTY_HEROIC, DIFFICULTY_NORMAL, itemLevelOf } from '../../src/loot/rates.js';

const INSTANZ_1 = ['boss-1-1', 'boss-1-2', 'boss-1-3', 'boss-1-4'] as const;

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

/** Ein echter Sieg über `onFinish` — derselbe Weg wie im Archiv-Fall. */
function siegFahren(app: App): void {
  app.session.startFight(0);
  app.session.tick(5_000);
  app.combat.setBossHpRemaining(0);
  app.session.tick(6_000);
}

/** Alle vier Bosse der ersten Instanz auf Normal legen. */
function instanzEinsAufNormal(app: App): void {
  for (const id of INSTANZ_1) {
    app.selectBoss(id);
    siegFahren(app);
    // ZUSICHERUNG je Sieg: Ein nicht gezählter Lauf schriebe keinen Schlüssel,
    // und der Fall darunter prüfte dann eine Freischaltung ohne Grundlage.
    expect(app.lastRecord?.counted, `${id}: der Sieg wurde nicht gezählt`).toBe(true);
  }
}

beforeEach(() => {
  stubbeCanvas();
  localStorage.clear();
  document.body.innerHTML = '';
});

describe('AC2 im Spiel — Heroisch erst nach Normal', () => {
  it('vor dem Abschluss übergeht selectBoss die heroische Stufe', () => {
    const app = baueApp();
    const vorher = app.combat.boss;
    app.selectBoss('boss-1-1', DIFFICULTY_HEROIC);
    expect(app.combat.boss, 'eine gesperrte Stufe hat den Kampf gewechselt').toBe(vorher);
    expect(app.openStagesOf('boss-1-1')).toEqual([DIFFICULTY_NORMAL]);
  });

  it('und die Auswahl bietet sie nicht an', () => {
    const app = baueApp();
    app.openHome();
    expect(document.querySelector('.boss-select-stage')).toBeNull();
    // Die Normal-Knöpfe stehen wie bisher.
    expect(document.querySelector('.boss-select-entry[data-boss-id="boss-1-1"]')).not.toBeNull();
  });

  it('nach vier Normal-Siegen steht Heroisch in der Auswahl — Mythisch noch nicht', () => {
    const app = baueApp();
    instanzEinsAufNormal(app);
    expect(app.persistence.state.defeatedBosses).toContain(markKey('boss-1-4', DIFFICULTY_NORMAL));

    app.openHome();
    const heroisch = document.querySelectorAll('.boss-select-stage[data-difficulty="1"]');
    expect([...heroisch].map((k) => (k as HTMLElement).dataset['bossId'])).toEqual([...INSTANZ_1]);
    expect(document.querySelector('.boss-select-stage[data-difficulty="2"]')).toBeNull();
    expect(heroisch[0]?.textContent).toBe('Heroic');
  });

  it('der Knopf startet den Kampf gegen die heroische Fassung', () => {
    const app = baueApp();
    instanzEinsAufNormal(app);
    app.openHome();
    const knopf = document.querySelector<HTMLButtonElement>(
      '.boss-select-stage[data-boss-id="boss-1-1"][data-difficulty="1"]',
    );
    if (knopf === null) throw new Error('kein heroischer Knopf für boss-1-1');
    knopf.click();

    const erwartet = bossAtDifficulty('boss-1-1', DIFFICULTY_HEROIC);
    const normal = bossAtDifficulty('boss-1-1', DIFFICULTY_NORMAL);
    expect(app.combat.boss, 'der Kampf läuft nicht gegen die heroische Fassung').toBe(erwartet);
    expect(app.combat.boss.hp).toBeGreaterThan(normal?.hp ?? Infinity);
    expect(app.combat.boss.overpressureMs).toBeLessThan(normal?.overpressureMs ?? 0);
    expect(app.combat.boss.affliction, 'die heroische Zutat fehlt im Kampf').not.toBeNull();
    // Die Vorbereitung nennt die Stufe — dieselbe Überschrift wie das HUD.
    expect(document.querySelector('.rota-title')?.textContent).toBe('INSTANCE ONE, FIRST · HEROIC');
  });
});

describe('Der Klang „Freischaltung“ hängt am Sieg, der etwas öffnet', () => {
  it('der vierte Normal-Sieg meldet die heroische Instanz und die nächste — die anderen nichts', () => {
    const app = baueApp();
    const gemeldet: (readonly string[])[] = [];
    for (const id of INSTANZ_1) {
      app.selectBoss(id);
      siegFahren(app);
      gemeldet.push(app.lastUnlocks);
    }
    expect(gemeldet).toEqual([[], [], [], ['instance-1@heroic', 'instance-2']]);
  });

  it('ein Sieg danach meldet nichts mehr, und das Öffnen der Auswahl auch nicht', () => {
    const app = baueApp();
    instanzEinsAufNormal(app);
    expect(app.lastUnlocks.length).toBeGreaterThan(0);

    app.selectBoss('boss-1-1');
    siegFahren(app);
    expect(app.lastRecord?.counted).toBe(true);
    expect(app.lastUnlocks, 'ein zweiter Sieg hat die Freischaltung wiederholt').toEqual([]);

    app.openHome();
    expect(app.lastUnlocks).toEqual([]);
  });
});

describe('AC5 — die Beute-Stufe hängt an der gewählten Stufe', () => {
  it('ein Normal-Sieg wirft die Normal-Stufe ab, ein heroischer die heroische', () => {
    const app = baueApp();
    app.selectBoss('boss-1-1');
    siegFahren(app);
    expect(app.lastRecord?.drop?.itemLevel, 'Normal').toBe(itemLevelOf(1, DIFFICULTY_NORMAL, 0));

    for (const id of INSTANZ_1.slice(1)) {
      app.selectBoss(id);
      siegFahren(app);
    }
    app.selectBoss('boss-1-1', DIFFICULTY_HEROIC);
    expect(difficultyOf(app.combat.boss), 'die heroische Stufe wurde nicht übernommen').toBe(
      DIFFICULTY_HEROIC,
    );
    siegFahren(app);

    expect(app.lastRecord?.counted).toBe(true);
    // ZUSICHERUNG: Die beiden Stufen ergeben verschiedene Zahlen, sonst prüfte
    // der Vergleich darunter nichts.
    expect(itemLevelOf(1, DIFFICULTY_HEROIC, 0)).not.toBe(itemLevelOf(1, DIFFICULTY_NORMAL, 0));
    expect(app.lastRecord?.drop?.itemLevel, 'Heroisch').toBe(itemLevelOf(1, DIFFICULTY_HEROIC, 0));
    // Und der Sieg steht unter der heroischen Stufe im Spielstand.
    expect(app.persistence.state.defeatedBosses).toContain(markKey('boss-1-1', DIFFICULTY_HEROIC));
  });
});
