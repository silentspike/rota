/**
 * Trefferwucht: Schadenszahlen, Aufblitzen, Bild-Ruckeln (GAME-DESIGN 9, 9.4).
 *
 * **Alle Bewegungen sind über vier Größen beschrieben** — Dauer, Kurve,
 * Überschwingen, Verzögerung (GAME-DESIGN 9.4). Sie stehen als Konstanten
 * beieinander, damit sich eine Bewegung ändern lässt, ohne sie zu suchen.
 *
 * | Bewegung        | Dauer  | Kurve           | Überschwingen | Verzögerung |
 * | --------------- | ------ | --------------- | ------------- | ----------- |
 * | Schadenszahl    | 700 ms | Aufstieg gedämpft, Ausblendung quadratisch | keins | keine |
 * | Aufblitzen Ring | 180 ms | quadratisch abklingend | keins  | keine       |
 * | Bild-Ruckeln    | 120 ms | Sinus, abklingend | keins       | keine       |
 *
 * **Nichts wird im Bild erzeugt** (GAME-DESIGN 18.2): Die Zahlen liegen in einem
 * vorab angelegten Pool, gezeichnet werden sie ziffernweise. Läuft der Pool voll,
 * wird die älteste Zahl wiederverwendet — eine fehlende Zahl fällt niemandem auf,
 * eine stehengebliebene sehr wohl.
 */
import { ObjectPool } from '../core/pool.js';
import { HIT_DOT } from '../core/hits.js';
import { drawIntegerRight, measureDigitAdvance } from './numerals.js';
import { VESSEL_CENTER_X, VESSEL_CENTER_Y, VESSEL_RADIUS } from './vessel.js';

// ---------------------------------------------------- Die vier Größen je Bewegung

/** GAME-DESIGN 9.4: „Schadenszahl: Translation + Ausblendung über 700 ms". */
export const NUMBER_LIFETIME_MS = 700;
/** Wie weit die Zahl in dieser Zeit steigt, in Entwurfs-Pixeln. */
const NUMBER_RISE_PX = 96;
/** Seitlicher Versatz, damit aufeinanderfolgende Zahlen nicht übereinanderliegen. */
const NUMBER_SPREAD_PX = 34;

/** GAME-DESIGN 9.4: „Bild-Ruckeln: 6 px über 120 ms". */
export const SHAKE_DURATION_MS = 120;
export const SHAKE_AMPLITUDE_PX = 6;
/** Zwei volle Schwingungen in 120 ms — sichtbar, aber nicht als Zittern. */
const SHAKE_CYCLES = 2;

/** Aufblitzen des Reaktionsgefäßes bei jedem Treffer. */
export const FLASH_DURATION_MS = 180;

/** So viele Zahlen können gleichzeitig unterwegs sein. */
export const NUMBER_POOL_SIZE = 64;

const COLOR_DIRECT = '#f4e6d2';
/** Ticks eines Effekts über Zeit: kleiner und ruhiger als ein direkter Treffer. */
const COLOR_DOT = '#d4762e';
/** Krit ruht in Schritt 1 (PURITY = 0), die Anzeige ist trotzdem vorbereitet. */
const COLOR_CRIT = '#ffd27a';

const FONT_DIRECT = '38px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_DOT = '26px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_CRIT = '54px ui-monospace, "DejaVu Sans Mono", monospace';

interface FloatingNumber {
  readonly index: number;
  value: number;
  startedAtMs: number;
  x: number;
  y: number;
  kind: number;
  isCrit: boolean;
  active: boolean;
}

/** Kurve der Ausblendung: lange gut lesbar, dann schnell weg. */
function fadeOut(t: number): number {
  const rest = 1 - t;
  return rest * rest;
}

/** Kurve des Aufstiegs: schnell los, dann auslaufend. Kein Überschwingen. */
function rise(t: number): number {
  return 1 - (1 - t) * (1 - t);
}

/**
 * Auslenkung des Bild-Ruckelns.
 *
 * Abklingende Sinusschwingung — deterministisch, damit zwei Durchläufe dasselbe
 * Bild ergeben (Design-Gesetz 6).
 */
export function shakeAt(ageMs: number): number {
  if (ageMs < 0 || ageMs >= SHAKE_DURATION_MS) return 0;
  const t = ageMs / SHAKE_DURATION_MS;
  return Math.sin(t * SHAKE_CYCLES * Math.PI * 2) * SHAKE_AMPLITUDE_PX * (1 - t);
}

/** Stärke des Aufblitzens, 0 bis 1. */
export function flashAt(ageMs: number): number {
  if (ageMs < 0 || ageMs >= FLASH_DURATION_MS) return 0;
  return fadeOut(ageMs / FLASH_DURATION_MS);
}

export class Impact {
  private readonly pool: ObjectPool<FloatingNumber>;
  private spawned = 0;
  private lastHitAtMs = -1;
  private lastBigHitAtMs = -1;
  private digitDirect = 20;
  private digitDot = 14;
  private digitCrit = 28;

  constructor(capacity: number = NUMBER_POOL_SIZE) {
    this.pool = new ObjectPool<FloatingNumber>(
      capacity,
      (index) => ({
        index,
        value: 0,
        startedAtMs: 0,
        x: 0,
        y: 0,
        kind: 0,
        isCrit: false,
        active: false,
      }),
      (item) => {
        item.value = 0;
        item.startedAtMs = 0;
        item.active = false;
      },
    );
  }

  /** Einmalig aufrufen — `measureText` erzeugt Speicher. */
  measure(ctx: CanvasRenderingContext2D): void {
    ctx.font = FONT_DIRECT;
    this.digitDirect = measureDigitAdvance(ctx);
    ctx.font = FONT_DOT;
    this.digitDot = measureDigitAdvance(ctx);
    ctx.font = FONT_CRIT;
    this.digitCrit = measureDigitAdvance(ctx);
  }

  get poolCapacity(): number {
    return this.pool.capacity;
  }

  get activeCount(): number {
    return this.pool.activeCount;
  }

  /** Wie oft eine noch sichtbare Zahl wiederverwendet werden musste. */
  get recycleCount(): number {
    return this.pool.recycleCount;
  }

  get spawnCount(): number {
    return this.spawned;
  }

  /** Zeitpunkt des letzten Treffers — treibt das Aufblitzen des Rings. */
  lastHitAt(): number {
    return this.lastHitAtMs;
  }

  /** Auslenkung des Bild-Ruckelns zur Anzeigezeit. */
  shakeOffset(shownMs: number): number {
    if (this.lastBigHitAtMs < 0) return 0;
    return shakeAt(shownMs - this.lastBigHitAtMs);
  }

  /** Stärke des Aufblitzens zur Anzeigezeit. */
  flashStrength(shownMs: number): number {
    if (this.lastHitAtMs < 0) return 0;
    return flashAt(shownMs - this.lastHitAtMs);
  }

  /**
   * Einen Treffer aufnehmen. Erzeugt nichts — der Platz kommt aus dem Pool.
   *
   * Wird aus dem Logik-Takt gerufen, nicht aus dem Bild.
   */
  onHit(damage: number, kind: number, isCrit: boolean, big: boolean, timeMs: number): void {
    const item = this.pool.acquire();
    item.value = damage;
    item.startedAtMs = timeMs;
    item.kind = kind;
    item.isCrit = isCrit;
    item.active = true;
    // **Neben dem Ring, nicht darin.** In der Mitte des Reaktionsgefäßes stehen
    // die Prozentzahl und ihre Beschriftung; Schadenszahlen darüber machen beide
    // unlesbar, und zwar genau dann, wenn am meisten passiert.
    //
    // Gefächert, damit aufeinanderfolgende Zahlen nicht übereinanderliegen — über
    // einen Zähler, nicht über Zufall (Design-Gesetz 6).
    const step = this.spawned % 5;
    item.x = VESSEL_CENTER_X - VESSEL_RADIUS - 24 + (step - 2) * NUMBER_SPREAD_PX;
    item.y = VESSEL_CENTER_Y + 60 + (step % 2) * 20;
    this.spawned++;

    this.lastHitAtMs = timeMs;
    if (big) this.lastBigHitAtMs = timeMs;
  }

  draw(ctx: CanvasRenderingContext2D, shownMs: number): void {
    ctx.textAlign = 'left';
    for (let i = 0; i < this.pool.capacity; i++) {
      if (!this.pool.isActive(i)) continue;
      const item = this.pool.itemAt(i);
      const age = shownMs - item.startedAtMs;
      if (age < 0) continue;
      if (age >= NUMBER_LIFETIME_MS) {
        // Abgelaufen: Platz freigeben, damit der Pool nicht unnötig eng wird.
        this.pool.release(i);
        item.active = false;
        continue;
      }
      const t = age / NUMBER_LIFETIME_MS;
      const advance = item.isCrit
        ? this.digitCrit
        : item.kind === HIT_DOT
          ? this.digitDot
          : this.digitDirect;
      ctx.font = item.isCrit ? FONT_CRIT : item.kind === HIT_DOT ? FONT_DOT : FONT_DIRECT;
      ctx.fillStyle = item.isCrit ? COLOR_CRIT : item.kind === HIT_DOT ? COLOR_DOT : COLOR_DIRECT;
      ctx.globalAlpha = fadeOut(t);
      drawIntegerRight(ctx, item.value, item.x, item.y - rise(t) * NUMBER_RISE_PX, advance);
      ctx.globalAlpha = 1;
    }
  }

  /** Alles zurücksetzen — bei einem neuen Lauf. */
  reset(): void {
    this.pool.releaseAll();
    for (let i = 0; i < this.pool.capacity; i++) this.pool.itemAt(i).active = false;
    this.spawned = 0;
    this.lastHitAtMs = -1;
    this.lastBigHitAtMs = -1;
  }
}
