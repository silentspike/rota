/**
 * Zeichenfläche für den Kampf (GAME-DESIGN 9.2, 18.1).
 *
 * Das HUD ist ein festes Bild mit festem Seitenverhältnis, das als Ganzes auf die
 * Fenstergröße skaliert wird — wie bei einer Spielkonsole. Damit ist die Anordnung
 * auf jedem Gerät identisch und die Lesbarkeit muss nur einmal geprüft werden. Auf
 * abweichenden Seitenverhältnissen bleiben Ränder frei.
 *
 * Die Größenrechnung läuft **nur bei Größenänderungen**, nie je Bild: Der laufende
 * Bild-Pfad setzt lediglich eine fertig berechnete Transformation.
 */

/** Entwurfs-Auflösung. Alle Koordinaten im Spiel beziehen sich hierauf. */
export const DESIGN_WIDTH = 1920;
export const DESIGN_HEIGHT = 1080;

export class Surface {
  readonly canvas: HTMLCanvasElement;
  readonly ctx: CanvasRenderingContext2D;

  /** Skalierung von Entwurfs- auf Gerätepixel. */
  private scale = 1;
  private offsetX = 0;
  private offsetY = 0;
  private passes = 0;
  private readonly boundResize: () => void;

  constructor(parent: HTMLElement) {
    const canvas = document.createElement('canvas');
    canvas.style.display = 'block';
    canvas.style.width = '100%';
    canvas.style.height = '100%';
    parent.appendChild(canvas);

    const ctx = canvas.getContext('2d', { alpha: false });
    if (ctx === null) {
      throw new Error('Canvas 2D context nicht verfügbar');
    }
    this.canvas = canvas;
    this.ctx = ctx;
    this.boundResize = (): void => {
      this.resize();
    };
    window.addEventListener('resize', this.boundResize);
    this.resize();
  }

  get designScale(): number {
    return this.scale;
  }

  get designOffsetX(): number {
    return this.offsetX;
  }

  get designOffsetY(): number {
    return this.offsetY;
  }

  /**
   * Zahl der Zeichenvorgänge.
   *
   * GAME-DESIGN 18.2 verlangt **einen Zeichenvorgang je Bild**. Der Zähler macht
   * das nachprüfbar: Er muss mit der Zahl der gezeichneten Bilder übereinstimmen.
   */
  get drawPasses(): number {
    return this.passes;
  }

  dispose(): void {
    window.removeEventListener('resize', this.boundResize);
    this.canvas.remove();
  }

  /** Passt die Puffergröße an das Fenster an. Läuft nur bei Größenänderungen. */
  resize(): void {
    const dpr = window.devicePixelRatio || 1;
    const cssWidth = this.canvas.clientWidth || DESIGN_WIDTH;
    const cssHeight = this.canvas.clientHeight || DESIGN_HEIGHT;
    const pixelWidth = Math.max(1, Math.round(cssWidth * dpr));
    const pixelHeight = Math.max(1, Math.round(cssHeight * dpr));
    if (this.canvas.width !== pixelWidth) this.canvas.width = pixelWidth;
    if (this.canvas.height !== pixelHeight) this.canvas.height = pixelHeight;

    this.scale = Math.min(pixelWidth / DESIGN_WIDTH, pixelHeight / DESIGN_HEIGHT);
    this.offsetX = (pixelWidth - DESIGN_WIDTH * this.scale) * 0.5;
    this.offsetY = (pixelHeight - DESIGN_HEIGHT * this.scale) * 0.5;
  }

  /**
   * Bereitet das Bild vor: Ränder füllen, auf Entwurfs-Koordinaten umstellen,
   * Spielfläche freiräumen. Fordert keinen Speicher an.
   */
  beginFrame(background: string, letterbox: string): void {
    this.passes++;
    const ctx = this.ctx;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = letterbox;
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.setTransform(this.scale, 0, 0, this.scale, this.offsetX, this.offsetY);
    ctx.fillStyle = background;
    ctx.fillRect(0, 0, DESIGN_WIDTH, DESIGN_HEIGHT);
  }
}
