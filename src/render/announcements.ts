/**
 * Die Boss-Ankündigungen im Bild (GAME-DESIGN 11.5, Issue #797).
 *
 * Auswahl, Zeitrechnung, Stufe und Art stehen in
 * `src/combat/boss/announcements.ts`; hier steht nur, wie sie aussehen. Drei der
 * vier Kanäle sind Bild:
 *
 * | Kanal (11.5) | hier | Stufe |
 * | --- | --- | --- |
 * | 1 Zeitleiste der nächsten Ereignisse — Balken mit Symbol und Restzeit | `drawList`, rechts (9.1: Anzeigen rechts) | alle |
 * | 2 Textmeldung mit Countdown in der Bildmitte | `drawCenter`, im Ring über der Prozentzahl | 2, 3 |
 * | 4 Hervorhebung am Reaktionsgefäß | `drawVesselHighlight` | 2: Bogen · 3: Bogen, Färbung, Puls |
 *
 * Kanal 3, der Klang, läuft im Takt (`Session.detectEdges`), nicht im Bild.
 *
 * **„Nicht alles ist bei allem angebracht"** (11.5): Stufe 1 bekommt nur den
 * Balken. Stufe 2 in ihren letzten Sekunden: der Balken blinkt, springt nach
 * oben in der Liste und in die Bildmitte, am Ring zieht ein Bogen auf. Stufe 3
 * zusätzlich: Ring in der Warnfarbe und pulsierend, je Sekunde ein Blitz und
 * ein Ruckeln.
 *
 * **Die Art hängt nicht allein an der Farbe** (GAME-DESIGN 21.2): Jede trägt
 * ein eigenes Symbol — Raute, Kreis, Dreieck, Kreuz.
 *
 * **Zurückhaltend** (21.2 verwirft Einstellungen zur Reizstärke, „lieber
 * zurückhaltender als zu grell"): Der Blitz deckt höchstens 12 % ab, das
 * Ruckeln ist das aus 9.4 (6 px über 120 ms).
 *
 * **Fenster und Vollbild zeichnen dasselbe** (Design-Gesetz 3): Alles hier
 * rechnet in Entwurfs-Koordinaten; was die Fläche skaliert, ist `Surface`.
 *
 * Nichts entsteht im Bild (GAME-DESIGN 18.2): Die Namen in Versalien werden je
 * Boss einmal gebildet, Zahlen ziffernweise gezeichnet.
 */
import {
  BossAnnouncer,
  FINAL_WINDOW_MS,
  KIND_DAMAGE,
  KIND_DEBUFF,
  KIND_LETHAL,
  KIND_OPPORTUNITY,
  TIER_LETHAL,
  countdownSecond,
  highlighted,
  type Announcement,
} from '../combat/boss/announcements.js';
import type { Combat } from '../core/combat.js';
import { flashAt, shakeAt } from './impact.js';
import { drawFixedRight, drawIntegerRight, measureDigitAdvance } from './numerals.js';
import { VESSEL_CENTER_X, VESSEL_CENTER_Y, VESSEL_RADIUS } from './vessel.js';
import { DESIGN_HEIGHT, DESIGN_WIDTH } from './surface.js';

// ------------------------------------------------------------------- Ort

/** Rechts, über den Instrumenten (GAME-DESIGN 9.1: „Instrumente und Anzeigen rechts"). */
export const LIST_LEFT = 1180;
export const LIST_RIGHT = DESIGN_WIDTH - 90;
/** Auf der Höhe des Überdruck-Timers oben links — die zwei Uhren des Kampfes. */
export const LIST_TOP = 96;
export const ROW_HEIGHT = 58;
/**
 * Höchstens so viele Zeilen — und zwar so viele, wie der Platz bis zu den
 * Instrumenten (660) trägt: Die neunte Zeile hat ihren Balken bei 572 bis 580.
 *
 * Gezählt über alle Bossdateien (Zeitleisten-Namen plus Bausteine, Stand #797):
 * höchstens fünf in einem Spiel-Boss (`boss-5-4`), neun im Messwerkzeug
 * `orch-maxload`. Kommt mehr, fällt das am wenigsten Dringliche weg — die Liste
 * ist nach Dringlichkeit sortiert.
 */
export const MAX_ROWS = 9;
const BAR_TOP_OFFSET = 12;
const BAR_HEIGHT = 8;
const SYMBOL_SIZE = 14;
const NAME_OFFSET = 28;

/** Kanal 2: zwei Zeilen im Ring über der Prozentzahl — die einzige freie Fläche am Ziel. */
const CENTER_NAME_Y = VESSEL_CENTER_Y - 110;
const CENTER_COUNT_Y = VESSEL_CENTER_Y - 54;

/**
 * Kanal 4: der Bogen läuft knapp außerhalb des Rings.
 *
 * **Nachgemessen und einmal korrigiert:** Mit +12 und 6 px Breite schnitt der
 * Bogen unten rechts die Zahl der Hitze-Anzeige (rechtsbündig bei 1032, y 540) —
 * im Screenshot gesehen, nicht im Code. Ihr nächster Punkt liegt etwa 196 px vom
 * Mittelpunkt; der Bogen endet jetzt bei 194.
 */
const ARC_RADIUS = VESSEL_RADIUS + 6;
const ARC_WIDTH = 4;
const RING_START = -Math.PI / 2;
const TWO_PI = Math.PI * 2;

// ----------------------------------------------------------------- Farben

/**
 * Je Art eine Farbe (11.5: „Schwächung auf dir — eigene Farbe"; „Gelegenheit
 * muss sich klar von den Warnungen abheben").
 */
export const KIND_COLORS: readonly string[] = (() => {
  const farben: string[] = [];
  farben[KIND_DEBUFF] = '#c77dff';
  farben[KIND_DAMAGE] = '#aeb6c8';
  farben[KIND_LETHAL] = '#ff4d3d';
  farben[KIND_OPPORTUNITY] = '#7ee081';
  return farben;
})();
const COLORS = KIND_COLORS;

const INK = '#d5d5de';
const INK_DIM = '#61616e';
const TRACK = 'rgba(255, 255, 255, 0.12)';
const FLASH = '#fff4e0';

/** Wie stark Nebensächliches gedämpft wird, solange Dringendes läuft. */
export const DIMMED_ALPHA = 0.45;
/** Obergrenze des Blitzes — zurückhaltend (21.2). */
export const FLASH_MAX_ALPHA = 0.12;
/** Blinken in den letzten Sekunden: Schwingungen je Sekunde. */
const BLINK_HZ = 3;

const FONT_SMALL = '19px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_TEXT = '24px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_COUNT = '46px ui-monospace, "DejaVu Sans Mono", monospace';

const LABEL_IN = 'IN ';

/**
 * Wie lange die letzte volle Countdown-Sekunde her ist — treibt Blitz und
 * Ruckeln von Stufe 3. Eine Funktion der Restzeit, kein Zufall (Gesetz 6).
 */
export function ageSinceSecondMs(remainingMs: number): number {
  return countdownSecond(remainingMs) * 1000 - remainingMs;
}

export class AnnouncementRenderer {
  readonly announcer = new BossAnnouncer();
  private shown = false;
  private upperNames: string[] = [];
  private namesFor: readonly Announcement[] | null = null;
  private digitText = 14;
  private digitCount = 28;

  /** Einmalig aufrufen — `measureText` erzeugt Speicher. */
  measure(ctx: CanvasRenderingContext2D): void {
    ctx.font = FONT_TEXT;
    this.digitText = measureDigitAdvance(ctx);
    ctx.font = FONT_COUNT;
    this.digitCount = measureDigitAdvance(ctx);
  }

  /**
   * Den Stand für dieses Bild nachziehen.
   *
   * @param fighting Nur im Kampf: In der Vorbereitung gibt es kein Ziel und keine
   *   Uhr (GAME-DESIGN 10), nach dem Ende nichts mehr anzukündigen.
   */
  update(combat: Combat, fightNowMs: number, fighting: boolean): void {
    this.shown = fighting;
    if (!fighting) return;
    this.announcer.update(combat, fightNowMs);
    const abilities = this.announcer.abilities;
    if (abilities !== this.namesFor) {
      // Einmal je Boss — wie `boss.name.toUpperCase()` in `hud.ts`, nur nicht je Bild.
      this.namesFor = abilities;
      this.upperNames = abilities.map((a) => a.name.toUpperCase());
    }
  }

  private upper(a: Announcement): string {
    const i = this.announcer.abilities.indexOf(a);
    return this.upperNames[i] ?? a.name;
  }

  /** Die dringlichste Ankündigung, wenn sie gerade hervorgehoben ist. */
  private top(): Announcement | null {
    if (!this.shown || this.announcer.count === 0) return null;
    const a = this.announcer.at(0);
    return highlighted(a) ? a : null;
  }

  /** Läuft gerade der Countdown einer tödlichen Fähigkeit? */
  private lethalTop(): Announcement | null {
    const a = this.top();
    return a !== null && a.tier >= TIER_LETHAL ? a : null;
  }

  /** Auslenkung des Ruckelns von Stufe 3 — zum Ruckeln der Treffer dazu. */
  shakeOffset(): number {
    const a = this.lethalTop();
    return a === null ? 0 : shakeAt(ageSinceSecondMs(a.remainingMs));
  }

  /** Stärke des Blitzes von Stufe 3, 0 bis `FLASH_MAX_ALPHA`. */
  flashAlpha(): number {
    const a = this.lethalTop();
    return a === null ? 0 : flashAt(ageSinceSecondMs(a.remainingMs)) * FLASH_MAX_ALPHA;
  }

  /**
   * Kanal 1: die Zeitleiste der nächsten Ereignisse.
   *
   * Je Fähigkeit eine Zeile: Symbol, Name, Restzeit, darunter der Balken über
   * die **volle Abklingzeit** — er beginnt, wo die vorige Wiederholung endete,
   * und läuft ab wie der Balken des Boss-Zaubers (#327): Was übrig ist, ist die
   * Zeit bis dahin.
   */
  drawList(ctx: CanvasRenderingContext2D, shownMs: number): void {
    if (!this.shown) return;
    const ann = this.announcer;
    const anzahl = ann.count < MAX_ROWS ? ann.count : MAX_ROWS;
    const oben = this.top();
    const dringendeStufe = oben === null ? 0 : oben.tier;
    ctx.textAlign = 'left';
    for (let rank = 0; rank < anzahl; rank++) {
      const a = ann.at(rank);
      const y = LIST_TOP + rank * ROW_HEIGHT;
      const hervor = highlighted(a);
      // „Das Nebensächliche gedämpft" — solange etwas Dringenderes in seinen
      // letzten Sekunden ist.
      let alpha = !hervor && a.tier < dringendeStufe ? DIMMED_ALPHA : 1;
      // „Balken blinkt" — eine Sinusfunktion der Zeit, kein Zufall (Gesetz 6).
      if (hervor) alpha *= 0.7 + 0.3 * Math.cos((shownMs / 1000) * BLINK_HZ * TWO_PI);
      ctx.globalAlpha = alpha;
      const farbe = COLORS[a.kind] ?? INK;

      drawKindSymbol(ctx, a.kind, LIST_LEFT + SYMBOL_SIZE * 0.5, y - 7, SYMBOL_SIZE, farbe);

      ctx.font = FONT_SMALL;
      ctx.fillStyle = hervor ? farbe : INK_DIM;
      ctx.fillText(this.upper(a), LIST_LEFT + NAME_OFFSET, y);

      ctx.font = FONT_TEXT;
      ctx.fillStyle = INK;
      const rest = a.remainingMs > 0 ? a.remainingMs : 0;
      drawFixedRight(ctx, rest / 1000, 1, LIST_RIGHT, y, this.digitText);

      const breite = LIST_RIGHT - LIST_LEFT;
      const dauer = a.dueAtMs - a.sinceMs;
      const anteil = dauer <= 0 ? 0 : rest >= dauer ? 1 : rest / dauer;
      ctx.fillStyle = TRACK;
      ctx.fillRect(LIST_LEFT, y + BAR_TOP_OFFSET, breite, BAR_HEIGHT);
      ctx.fillStyle = hervor ? farbe : INK_DIM;
      ctx.fillRect(LIST_LEFT, y + BAR_TOP_OFFSET, breite * anteil, BAR_HEIGHT);
    }
    ctx.globalAlpha = 1;
  }

  /**
   * Kanal 2: die Textmeldung in der Bildmitte — „Cascade Failure in 5".
   *
   * Nur die dringlichste, nur Stufe 2 und 3, nur in ihren letzten Sekunden. Sie
   * nennt den Namen und sonst nichts (11.5: „Keine Erklärungen im Kampf").
   */
  drawCenter(ctx: CanvasRenderingContext2D): void {
    const a = this.top();
    if (a === null) return;
    const farbe = COLORS[a.kind] ?? INK;
    ctx.textAlign = 'center';
    ctx.font = FONT_SMALL;
    ctx.fillStyle = farbe;
    ctx.fillText(this.upper(a), VESSEL_CENTER_X, CENTER_NAME_Y);

    // „IN 5" — Monospace: Breite = Zeichen mal Vorschub, ohne `measureText`.
    const sekunde = countdownSecond(a.remainingMs);
    const lethal = a.tier >= TIER_LETHAL;
    const vorschub = lethal ? this.digitCount : this.digitText;
    const ziffern = sekunde >= 10 ? 2 : 1;
    const breite = (LABEL_IN.length + ziffern) * vorschub;
    const links = VESSEL_CENTER_X - breite * 0.5;
    ctx.textAlign = 'left';
    ctx.font = lethal ? FONT_COUNT : FONT_TEXT;
    ctx.fillStyle = lethal ? farbe : INK;
    ctx.fillText(LABEL_IN, links, CENTER_COUNT_Y);
    drawIntegerRight(ctx, sekunde, links + breite, CENTER_COUNT_Y, vorschub);
  }

  /**
   * Kanal 4: die Hervorhebung am Reaktionsgefäß.
   *
   * „Ring färbt sich, pulsiert, zeigt einen aufziehenden Bogen." Der Bogen zieht
   * über die letzten Sekunden von zwölf Uhr im Uhrzeigersinn auf — voll ist er,
   * wenn es trifft. Färbung und Puls nur bei Stufe 3.
   */
  drawVesselHighlight(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const a = this.top();
    if (a === null) return;
    const farbe = COLORS[a.kind] ?? INK;
    const rest = a.remainingMs > 0 ? a.remainingMs : 0;
    const anteil = 1 - (rest >= FINAL_WINDOW_MS ? 1 : rest / FINAL_WINDOW_MS);
    ctx.strokeStyle = farbe;
    if (anteil > 0) {
      ctx.lineWidth = ARC_WIDTH;
      ctx.beginPath();
      ctx.arc(
        VESSEL_CENTER_X,
        VESSEL_CENTER_Y,
        ARC_RADIUS,
        RING_START,
        RING_START + anteil * TWO_PI,
      );
      ctx.stroke();
    }
    if (a.tier >= TIER_LETHAL) {
      ctx.globalAlpha = 0.45 + 0.4 * Math.cos((shownMs / 1000) * BLINK_HZ * TWO_PI);
      ctx.lineWidth = 4;
      ctx.beginPath();
      ctx.arc(VESSEL_CENTER_X, VESSEL_CENTER_Y, VESSEL_RADIUS + 1, 0, TWO_PI);
      ctx.stroke();
      ctx.globalAlpha = 1;
    }
  }

  /** Der Blitz von Stufe 3, über allem. */
  drawFlash(ctx: CanvasRenderingContext2D): void {
    const alpha = this.flashAlpha();
    if (alpha <= 0) return;
    ctx.globalAlpha = alpha;
    ctx.fillStyle = FLASH;
    ctx.fillRect(0, 0, DESIGN_WIDTH, DESIGN_HEIGHT);
    ctx.globalAlpha = 1;
  }
}

/**
 * Das Symbol der Art (11.5, Kanal 1: „Balken mit Symbol"). Geometrisch, kein
 * Artwork (Design-Gesetz 4), und je Art eine andere Form — die Farbe allein
 * trägt die Aussage nicht (21.2).
 */
export function drawKindSymbol(
  ctx: CanvasRenderingContext2D,
  kind: number,
  cx: number,
  cy: number,
  size: number,
  color: string,
): void {
  const h = size * 0.5;
  ctx.fillStyle = color;
  ctx.beginPath();
  if (kind === KIND_DEBUFF) {
    // Raute — auf dich gerichtet.
    ctx.moveTo(cx, cy - h);
    ctx.lineTo(cx + h, cy);
    ctx.lineTo(cx, cy + h);
    ctx.lineTo(cx - h, cy);
    ctx.closePath();
    ctx.fill();
  } else if (kind === KIND_LETHAL) {
    // Dreieck — das Warnzeichen.
    ctx.moveTo(cx, cy - h);
    ctx.lineTo(cx + h, cy + h);
    ctx.lineTo(cx - h, cy + h);
    ctx.closePath();
    ctx.fill();
  } else if (kind === KIND_OPPORTUNITY) {
    // Kreuz — etwas kommt dazu.
    const d = size * 0.18;
    ctx.rect(cx - d, cy - h, d * 2, size);
    ctx.rect(cx - h, cy - d, size, d * 2);
    ctx.fill();
  } else {
    // Kreis — der ruhige Fall.
    ctx.arc(cx, cy, h * 0.8, 0, TWO_PI);
    ctx.fill();
  }
}
