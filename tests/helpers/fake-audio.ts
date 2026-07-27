/**
 * Nachbau der Web-Audio-Schnittstelle fuer Tests im Node-Lauf.
 *
 * Prueft die Buchfuehrung der Ausgabe — Platzverwaltung, Streuung, Regler,
 * Schleifen-Grenzen — ohne Browser. Was das Signal selbst betrifft (Naht der
 * Schleife, Pegel je Weg), wird im Browser gegen einen echten
 * `OfflineAudioContext` gemessen; ein Nachbau koennte das nur behaupten.
 *
 * Der Nachbau zaehlt jede Knoten-Erzeugung mit. Genau daran haengt der
 * Nachweis, dass im Abspielpfad nichts Neues entsteht (GAME-DESIGN 18.2).
 */

export class FakeAudioParam {
  value: number;
  readonly events: Array<{ type: string; value: number; time: number }> = [];

  constructor(value = 1) {
    this.value = value;
  }

  setValueAtTime(value: number, time: number): FakeAudioParam {
    this.value = value;
    this.events.push({ type: 'set', value, time });
    return this;
  }

  linearRampToValueAtTime(value: number, time: number): FakeAudioParam {
    // Der Nachbau springt sofort auf den Zielwert; geprueft wird, DASS eine
    // Rampe angefordert wurde und mit welchem Ziel.
    this.value = value;
    this.events.push({ type: 'ramp', value, time });
    return this;
  }

  cancelScheduledValues(time: number): FakeAudioParam {
    this.events.push({ type: 'cancel', value: this.value, time });
    return this;
  }
}

export class FakeAudioNode {
  readonly connections: FakeAudioNode[] = [];
  disconnected = 0;

  connect(target: FakeAudioNode): FakeAudioNode {
    this.connections.push(target);
    return target;
  }

  disconnect(): void {
    this.disconnected++;
    this.connections.length = 0;
  }
}

export class FakeGainNode extends FakeAudioNode {
  readonly gain = new FakeAudioParam(1);
}

export class FakeAudioBuffer {
  constructor(
    readonly duration: number,
    readonly numberOfChannels = 1,
    readonly sampleRate = 48000,
  ) {}

  get length(): number {
    return Math.round(this.duration * this.sampleRate);
  }
}

export class FakeBufferSource extends FakeAudioNode {
  buffer: FakeAudioBuffer | null = null;
  readonly playbackRate = new FakeAudioParam(1);
  loop = false;
  loopStart = 0;
  loopEnd = 0;
  onended: (() => void) | null = null;
  startedAt: number | null = null;
  stoppedAt: number | null = null;

  start(when = 0): void {
    if (this.startedAt !== null) throw new Error('start() zweimal aufgerufen');
    this.startedAt = when;
  }

  stop(when = 0): void {
    this.stoppedAt = when;
  }

  /** Das natuerliche Ende ausloesen — im Browser macht das der Ausgabe-Faden. */
  finish(): void {
    this.onended?.();
  }
}

export class FakeAudioContext {
  currentTime = 0;
  state: 'suspended' | 'running' | 'closed' = 'running';
  readonly sampleRate = 48000;
  readonly destination = new FakeAudioNode();

  gainsCreated = 0;
  sourcesCreated = 0;
  resumeCalls = 0;

  createGain(): FakeGainNode {
    this.gainsCreated++;
    return new FakeGainNode();
  }

  createBufferSource(): FakeBufferSource {
    this.sourcesCreated++;
    return new FakeBufferSource();
  }

  async resume(): Promise<void> {
    this.resumeCalls++;
    this.state = 'running';
  }

  advance(seconds: number): void {
    this.currentTime += seconds;
  }
}

/** Der Nachbau erfuellt genau den Teil der Schnittstelle, den die Ausgabe nutzt. */
export function asContext(ctx: FakeAudioContext): BaseAudioContext {
  return ctx as unknown as BaseAudioContext;
}

export function asBuffer(buffer: FakeAudioBuffer): AudioBuffer {
  return buffer as unknown as AudioBuffer;
}
