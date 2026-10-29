/**
 * Die Anzeige für Overdrive (#1177, Abschnitt 2).
 *
 * **Rechts neben der Leiste, in der mittleren Reihe** — dort, wo die Taste
 * unter der Hand liegt: G steht rechts neben F (Entscheidung des Auftraggebers
 * vom 2026-10-29), und die Leiste bildet die Tastatur nach (GAME-DESIGN 9.1).
 * Sie ist **kein Platz der Leiste** und wird auch nicht so gezeichnet: kein
 * Zauber, keine Schule, keine Schulfarbe, keine Wischblende. Klickbar ist sie
 * so wenig wie die Leiste (Design-Gesetz 2).
 *
 * | Zustand    | Mitte          | Rand                         |
 * | ---------- | -------------- | ---------------------------- |
 * | bereit     | `READY`        | hell, voll                   |
 * | aktiv      | Restsekunden   | läuft im Uhrzeigersinn leer  |
 * | verbraucht | `SPENT`        | abgedunkelt                  |
 *
 * **Der Zustand steht als Wort oder Zahl da, nicht nur als Helligkeit** —
 * die spielkritische Auskunft hängt nicht an der Farbe (GAME-DESIGN 21.2).
 * Die Restzeit läuft am RAND leer, weil die drei Formen in der Leiste schon
 * vergeben sind (Wischblende, Streifen unten, Balken rechts).
 */
import type { Session } from '../core/session.js';
import {
  OVERDRIVE_ACTIVE,
  OVERDRIVE_READY,
  OVERDRIVE_SPENT,
  Overdrive,
} from '../combat/overdrive.js';
import type { KeyLabels } from '../input/layout.js';
import { drawFixedRight, measureDigitAdvance } from './numerals.js';
import { BAR_RIGHT, BLOCK_TOP, SLOT_GAP, SLOT_SIZE } from './spellbar.js';

/**
 * Rechts neben der Leiste, mit demselben Abstand wie zwischen zwei Plätzen —
 * als fünfte Taste hinter F, die sie auf der Tastatur ist.
 *
 * **Nachgemessen am Bild, einmal verschoben:** Mit doppeltem Abstand lag die
 * Kachel 20 px vor den Anzeigen rechts (`READOUT_LEFT` 1180) und las sich als
 * Teil ihrer Spalte statt der Leiste.
 */
export const OVERDRIVE_TILE_LEFT = BAR_RIGHT + SLOT_GAP;
/** In der mittleren Reihe — die Reihe von A S D F, und G liegt daneben. */
export const OVERDRIVE_TILE_TOP = BLOCK_TOP + SLOT_SIZE + SLOT_GAP;
export const OVERDRIVE_TILE_SIZE = SLOT_SIZE;

// Spieltexte englisch (Design-Gesetz 5).
const LABEL_TITLE = 'OVERDRIVE';
const LABEL_READY = 'READY';
const LABEL_SPENT = 'SPENT';
const LABEL_CATALYSIS = 'CATALYSIS';

const FILL_READY = '#232330';
const FILL_ACTIVE = '#2c2c3a';
const FILL_SPENT = '#15151c';
/** Neutral, weil Overdrive zu keiner Schule gehört (GAME-DESIGN 9.3). */
const EDGE = '#efeff6';
const EDGE_DIM = '#4a4a56';
const TRACK = 'rgba(255, 255, 255, 0.14)';
const INK = '#e2e2ea';
const INK_DIM = '#6a6a78';
const FLASH_ACCEPT = '#efe7d4';
const FLASH_REJECT = '#c03a30';

const FONT_KEY = '21px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_STATE = '19px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_TITLE = '15px ui-monospace, "DejaVu Sans Mono", monospace';
/**
 * Kleiner als die Restzeit auf der Leiste (34 px): Vier Zeichen wie „17.0"
 * standen dort im Bild bündig am linken Rand der Kachel.
 */
const FONT_SECONDS = '30px ui-monospace, "DejaVu Sans Mono", monospace';

const EDGE_WIDTH = 4;
/** Dieselben Rückmeldezeiten wie auf der Leiste (GAME-DESIGN 8.2, 9.4). */
const ACCEPT_FLASH_MS = 140;
const REJECT_FLASH_MS = 220;

export class OverdriveTile {
  private digitAdvance = 16;
  /**
   * Die Zeile unter der Anzeige, **einmal gebaut** — im Bild entsteht kein
   * Text (GAME-DESIGN 18.2). Die Zahl kommt aus den Daten; ändert sie sich,
   * ändert sich die Zeile mit.
   */
  private readonly bonusText = `+${String(Overdrive.catalysisBonus / 10)}% ${LABEL_CATALYSIS}`;

  constructor(
    private readonly session: Session,
    private readonly labels: KeyLabels,
  ) {}

  /** Einmalig nach dem Setzen der Schrift — `measureText` erzeugt Speicher. */
  measure(ctx: CanvasRenderingContext2D): void {
    ctx.font = FONT_SECONDS;
    this.digitAdvance = measureDigitAdvance(ctx);
  }

  draw(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const overdrive = this.session.combat.overdrive;
    const zustand = overdrive.state(shownMs);
    const x = OVERDRIVE_TILE_LEFT;
    const y = OVERDRIVE_TILE_TOP;
    const size = OVERDRIVE_TILE_SIZE;
    const hell = zustand !== OVERDRIVE_SPENT;

    ctx.fillStyle =
      zustand === OVERDRIVE_ACTIVE
        ? FILL_ACTIVE
        : zustand === OVERDRIVE_READY
          ? FILL_READY
          : FILL_SPENT;
    ctx.fillRect(x, y, size, size);

    // Der Rand: voll, solange er bereit ist; im Fenster läuft er leer.
    const rest =
      zustand === OVERDRIVE_ACTIVE ? overdrive.remainingMs(shownMs) / Overdrive.durationMs : 1;
    ctx.lineWidth = EDGE_WIDTH;
    if (zustand === OVERDRIVE_ACTIVE) {
      ctx.strokeStyle = TRACK;
      ctx.strokeRect(x + 2, y + 2, size - 4, size - 4);
    }
    ctx.strokeStyle = hell ? EDGE : EDGE_DIM;
    strokePerimeter(ctx, x + 2, y + 2, size - 4, size - 4, zustand === OVERDRIVE_SPENT ? 1 : rest);

    ctx.textAlign = 'left';
    ctx.font = FONT_KEY;
    ctx.fillStyle = hell ? INK : INK_DIM;
    ctx.fillText(this.labels.overdriveLabel, x + 9, y + 26);

    ctx.textAlign = 'center';
    if (zustand === OVERDRIVE_ACTIVE) {
      ctx.font = FONT_SECONDS;
      ctx.fillStyle = INK;
      // **Mittig nach der Zahl der Zeichen**, nicht fest um anderthalb
      // Ziffern verschoben: „18.2" hat vier Zeichen, und mit der festen
      // Verschiebung der Leiste ragte die erste Ziffer im Bild über den
      // linken Rand der Kachel.
      const sekunden = overdrive.remainingMs(shownMs) / 1000;
      const zeichen = sekunden >= 9.95 ? 4 : 3;
      drawFixedRight(
        ctx,
        sekunden,
        1,
        x + size * 0.5 + this.digitAdvance * zeichen * 0.5,
        y + size * 0.5 + 10,
        this.digitAdvance,
      );
    } else {
      ctx.font = FONT_STATE;
      ctx.fillStyle = hell ? INK : INK_DIM;
      ctx.fillText(zustand === OVERDRIVE_READY ? LABEL_READY : LABEL_SPENT, x + size * 0.5, y + 60);
    }
    ctx.font = FONT_TITLE;
    ctx.fillStyle = hell ? INK : INK_DIM;
    ctx.fillText(LABEL_TITLE, x + size * 0.5, y + size - 12);

    ctx.textAlign = 'left';
    // Was er tut, solange er es tut — als Zahl, damit der Tempo-Schub nicht
    // nur am Rand abzulesen ist. Unter der Anzeige, bündig mit ihrer linken
    // Kante: Mittig gesetzt ragte die Zeile in die untere Reihe der Leiste.
    if (zustand === OVERDRIVE_ACTIVE) {
      ctx.font = FONT_TITLE;
      ctx.fillStyle = INK;
      ctx.fillText(this.bonusText, x, y + size + 24);
    }

    this.drawFlash(ctx, x, y, size, shownMs);
  }

  /** Dieselbe Rückmeldung auf den Druck wie auf der Leiste (GAME-DESIGN 8.2). */
  private drawFlash(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    size: number,
    shownMs: number,
  ): void {
    const angenommen = this.session.overdriveAcceptedAtMs;
    const verworfen = this.session.overdriveRejectedAtMs;
    const alterAn = angenommen < 0 ? Number.POSITIVE_INFINITY : shownMs - angenommen;
    const alterVer = verworfen < 0 ? Number.POSITIVE_INFINITY : shownMs - verworfen;
    if (alterVer <= alterAn) {
      if (alterVer >= 0 && alterVer < REJECT_FLASH_MS) {
        ctx.globalAlpha = falloff(alterVer / REJECT_FLASH_MS);
        ctx.fillStyle = FLASH_REJECT;
        ctx.fillRect(x, y, size, size);
        ctx.globalAlpha = 1;
      }
      return;
    }
    if (alterAn >= 0 && alterAn < ACCEPT_FLASH_MS) {
      ctx.globalAlpha = falloff(alterAn / ACCEPT_FLASH_MS) * 0.8;
      ctx.fillStyle = FLASH_ACCEPT;
      ctx.fillRect(x, y, size, size);
      ctx.globalAlpha = 1;
    }
  }
}

/**
 * Den Rand eines Rechtecks zu einem Anteil zeichnen, oben in der Mitte
 * beginnend, im Uhrzeigersinn — so läuft er leer wie eine Uhr.
 *
 * Ohne Speicher im Bild (GAME-DESIGN 18.2): nur Pfad-Befehle.
 */
export function strokePerimeter(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  share: number,
): void {
  const anteil = share > 1 ? 1 : share < 0 ? 0 : share;
  if (anteil <= 0) return;
  let rest = anteil * 2 * (width + height);
  const mitte = x + width * 0.5;
  ctx.beginPath();
  ctx.moveTo(mitte, y);
  // Fünf Strecken: obere Hälfte rechts, rechts, unten, links, obere Hälfte
  // links. Über Indizes, nicht `for … of`: kein Iterator im Bild.
  let px = mitte;
  let py = y;
  for (let i = 0; i < STRECKE_DX.length; i++) {
    const art = STRECKE_LAENGE[i] ?? 2;
    const l = art === 0 ? width * 0.5 : art === 1 ? height : width;
    const schritt = rest < l ? rest : l;
    px += (STRECKE_DX[i] ?? 0) * schritt;
    py += (STRECKE_DY[i] ?? 0) * schritt;
    ctx.lineTo(px, py);
    rest -= schritt;
    if (rest <= 0) break;
  }
  ctx.stroke();
}

/** Richtung je Strecke. */
const STRECKE_DX: readonly number[] = [1, 0, -1, 0, 1];
const STRECKE_DY: readonly number[] = [0, 1, 0, -1, 0];
/** Länge je Strecke: 0 = halbe Breite, 1 = Höhe, 2 = Breite. */
const STRECKE_LAENGE: readonly number[] = [0, 1, 2, 1, 0];

/** Ausblendkurve: schnell weg, dann ausklingen. Kein Überschwingen. */
function falloff(t: number): number {
  const rest = 1 - t;
  return rest * rest;
}
