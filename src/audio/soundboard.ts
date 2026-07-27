/**
 * Abhoer-Soundboard — Entwicklungs-Werkzeug (GAME-DESIGN 17.1, 19.1).
 *
 * Zwei Aufgaben:
 *
 * 1. **Abnahme durch den Auftraggeber.** Claude kann nicht hoeren; hier hoert
 *    der Auftraggeber jeden Klang einzeln ab und entscheidet behalten oder neu.
 * 2. **Messplatz.** Alles Messbare wird hier gemessen statt behauptet:
 *    Verzoegerung nach dem Tastendruck, Knoten-Erzeugung im Abspielpfad,
 *    Reglerwirkung, Naht der Schleifen.
 *
 * Die Seite steht nur in der Entwicklungs-Fassung im Bau (siehe vite.config.ts)
 * und landet nie beim Spieler.
 */
import { AudioEngine } from './engine.js';
import { resolveAssetUrl } from './asset-urls.js';
import { makeClick, makeNoiseHit, makeSeamlessTone } from './dev-tones.js';
import {
  instrumentContext,
  measureBusIsolation,
  measureFramePressure,
  measureLoopSeam,
  measurePitchSpread,
} from './measure.js';
import { parseManifest } from './manifest.js';
import { SOUND_MANIFEST } from './manifest-data.js';
import { BUSES, type Bus, type SoundDef, type VoiceHandle } from './types.js';

/**
 * Zusaetzlich zu den Klaengen des Spiels (`assets/audio/`, aufgeloest ueber
 * `asset-urls.ts`) macht das Soundboard die fertig nachbearbeiteten Ergebnisse
 * der Werkzeugkette abhoerbar.
 *
 * Dieser Glob steht bewusst hier und nicht in `asset-urls.ts`: Das Soundboard
 * wird nur in der Entwicklungs-Fassung gebaut, damit landen diese Dateien
 * niemals in der Auslieferung.
 */
const toolchainUrls = import.meta.glob<string>('/assets/sfx/final/*.{ogg,mp3}', {
  eager: true,
  query: '?url',
  import: 'default',
});

/**
 * Tastenbelegung des Soundboards, nach physischen Reihen.
 *
 * Bewusst als Liste und nicht als eine lange Zeichenkette: Eine Kette aus
 * einunddreissig verschiedenen Zeichen hat genug Entropie, dass die
 * Geheimnis-Suche der CI sie fuer einen Schluessel haelt.
 */
const KEYS = [
  ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
  ['q', 'w', 'e', 'r', 't', 'z', 'u', 'i', 'o', 'p'],
  ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'],
].flat();

interface Entry {
  readonly def: SoundDef;
  readonly key: string | null;
  /** Woher der Klang kommt: aus dem Verzeichnis, aus der Werkzeugkette, synthetisch. */
  readonly origin: string;
  readonly purpose: string;
  handle: VoiceHandle;
}

const ctx = new AudioContext({ latencyHint: 'interactive' });
const counters = instrumentContext(ctx);
const engine = new AudioEngine(ctx, {
  resolveUrl: (src) => toolchainUrls[src] ?? resolveAssetUrl(src),
});

// ----------------------------------------------------------- Klang-Verzeichnis

/** Das gepflegte Verzeichnis — die Klaenge des Spiels. */
const manifest = parseManifest(SOUND_MANIFEST);
if (manifest.problems.length > 0) {
  console.warn('[rota] Verzeichnis beanstandet:', manifest.problems);
}
const purposes = new Map(SOUND_MANIFEST.sounds.map((sound) => [sound.id, sound.purpose]));

const devDefs: SoundDef[] = [
  // Synthetische Klaenge: Sie erlauben Messungen mit bekanntem Sollwert,
  // unabhaengig davon, welche Dateien gerade vorliegen.
  { kind: 'oneshot', id: 'dev-click', bus: 'ui', src: 'dev://click', pitchJitterCents: 0 },
  { kind: 'oneshot', id: 'dev-hit', bus: 'sfx', src: 'dev://noise', pitchJitterCents: 30 },
  {
    kind: 'sustained',
    id: 'dev-loop',
    bus: 'music',
    bodySrc: 'dev://tone',
    pitchJitterCents: 0,
    fadeInSeconds: 0.05,
    fadeOutSeconds: 0.1,
  },
];

/** Ergebnisse der Werkzeugketten-Messreihen, nur zum Vergleich abhoerbar. */
const toolchainDefs: SoundDef[] = Object.keys(toolchainUrls).map((src) => ({
  kind: 'oneshot',
  id: src.replace(/^.*\//, '').replace(/\.[a-z0-9]+$/i, ''),
  bus: 'sfx',
  src,
  pitchJitterCents: 0,
}));

const defs: SoundDef[] = [...manifest.defs, ...devDefs, ...toolchainDefs];
engine.library.register(defs);
engine.library.put('dev://click', makeClick(ctx));
engine.library.put('dev://noise', makeNoiseHit(ctx));
engine.library.put('dev://tone', makeSeamlessTone(ctx));

const entries: Entry[] = defs.map((def) => ({
  def,
  key: null,
  origin: manifest.defs.includes(def)
    ? 'Verzeichnis'
    : devDefs.includes(def)
      ? 'synthetisch'
      : 'Werkzeugkette',
  purpose: purposes.get(def.id) ?? '',
  handle: -1,
}));
const byKey = new Map<string, Entry>();

/**
 * Tasten nur an Klaenge vergeben, deren Datei vorliegt.
 *
 * Wird nach dem Laden erneut aufgerufen: Vorher steht noch nicht fest, was
 * tatsaechlich da ist, und eine Taste, die ins Leere greift, waere beim
 * Abhoeren nur verwirrend.
 */
function assignKeys(): void {
  byKey.clear();
  let next = 0;
  for (const entry of entries) {
    const key = isLoaded(entry.def) ? (KEYS[next] ?? null) : null;
    (entry as { key: string | null }).key = key;
    if (key) {
      byKey.set(key, entry);
      next++;
    }
  }
}

// ------------------------------------------------------- Verzoegerungs-Messung

const HISTORY = 256;
const totalMs = new Float64Array(HISTORY);
const queueMs = new Float64Array(HISTORY);
const workMs = new Float64Array(HISTORY);
let samples = 0;

function outputLatencyMs(): number {
  const base = (ctx.baseLatency || 0) * 1000;
  const output = (ctx.outputLatency || 0) * 1000;
  return Math.max(base, output);
}

function record(queue: number, work: number): void {
  const index = samples % HISTORY;
  queueMs[index] = queue;
  workMs[index] = work;
  totalMs[index] = queue + work + outputLatencyMs();
  samples++;
  renderLatency();
}

function stats(values: Float64Array): { count: number; mean: number; max: number; p95: number } {
  const count = Math.min(samples, HISTORY);
  if (count === 0) return { count: 0, mean: 0, max: 0, p95: 0 };
  const slice = Array.from(values.slice(0, count)).sort((a, b) => a - b);
  const sum = slice.reduce((acc, v) => acc + v, 0);
  return {
    count,
    mean: sum / count,
    max: slice[count - 1] ?? 0,
    p95: slice[Math.min(count - 1, Math.floor(count * 0.95))] ?? 0,
  };
}

// ------------------------------------------------------------------ Oberflaeche

const statusEl = document.getElementById('status') as HTMLPreElement;
const latencyEl = document.getElementById('latency') as HTMLPreElement;
const soundsEl = document.getElementById('sounds') as HTMLTableSectionElement;
const fadersEl = document.getElementById('faders') as HTMLDivElement;

function renderStatus(extra = ''): void {
  statusEl.textContent = [
    `Kontext        ${ctx.state}   ${ctx.sampleRate} Hz`,
    `Puffer-Latenz  base ${(ctx.baseLatency * 1000).toFixed(2)} ms   output ${((ctx.outputLatency || 0) * 1000).toFixed(2)} ms`,
    `Stimmen        ${engine.metrics().activeVoices} aktiv von ${engine.pool.size()}   geraeumt ${engine.metrics().stolenVoices}`,
    `Knoten erzeugt Lautstaerke ${counters.gains}   Quellen ${counters.sources}`,
    extra || loadReport,
  ]
    .filter(Boolean)
    .join('\n');
}

function renderLatency(): void {
  const total = stats(totalMs);
  const queue = stats(queueMs);
  const work = stats(workMs);
  latencyEl.textContent = [
    `Messungen      ${total.count}`,
    `Gesamt         Mittel ${total.mean.toFixed(2)} ms   p95 ${total.p95.toFixed(2)} ms   Max ${total.max.toFixed(2)} ms`,
    `davon Wartezeit bis zum Handler  ${queue.mean.toFixed(2)} ms (max ${queue.max.toFixed(2)})`,
    `davon Arbeit der Ausgabe         ${work.mean.toFixed(3)} ms (max ${work.max.toFixed(3)})`,
    `davon Ausgabepuffer              ${outputLatencyMs().toFixed(2)} ms`,
  ].join('\n');
}

function renderFaders(): void {
  fadersEl.replaceChildren();
  const rows: Array<{ label: string; get: () => number; set: (v: number) => void }> = [
    ...BUSES.map((bus) => ({
      label: bus,
      get: () => engine.volume(bus),
      set: (v: number) => engine.setVolume(bus, v),
    })),
    {
      label: 'gesamt',
      get: () => engine.mixer.masterVolume(),
      set: (v: number) => engine.setMasterVolume(v),
    },
  ];
  for (const row of rows) {
    const label = document.createElement('label');
    label.textContent = row.label;
    const input = document.createElement('input');
    input.type = 'range';
    input.min = '0';
    input.max = '1';
    input.step = '0.01';
    input.value = String(row.get());
    const value = document.createElement('span');
    value.className = 'dim';
    value.textContent = row.get().toFixed(2);
    input.addEventListener('input', () => {
      const v = Number(input.value);
      row.set(v);
      value.textContent = v.toFixed(2);
    });
    fadersEl.append(label, input, value);
  }
}

/** Liegt die Datei zu diesem Klang vor? */
function isLoaded(def: SoundDef): boolean {
  const src = def.kind === 'oneshot' ? def.src : def.bodySrc;
  return engine.library.buffer(src) !== undefined;
}

function renderSounds(): void {
  soundsEl.replaceChildren();
  for (const entry of entries) {
    const loaded = isLoaded(entry.def);
    const tr = document.createElement('tr');

    const key = document.createElement('td');
    if (entry.key && loaded) {
      const kbd = document.createElement('kbd');
      kbd.textContent = entry.key;
      key.append(kbd);
    } else {
      key.textContent = '—';
    }

    const id = document.createElement('td');
    id.textContent = entry.def.id;
    const purpose = document.createElement('td');
    purpose.className = 'dim';
    purpose.textContent = entry.purpose || entry.origin;
    const bus = document.createElement('td');
    bus.textContent = entry.def.bus;
    const kind = document.createElement('td');
    kind.textContent = entry.def.kind === 'oneshot' ? 'One-Shot' : 'dreiteilig';

    const action = document.createElement('td');
    if (loaded) {
      const button = document.createElement('button');
      button.textContent = entry.def.kind === 'oneshot' ? 'abspielen' : 'start / stop';
      button.addEventListener('click', () => {
        void trigger(entry);
      });
      action.append(button);
    } else {
      action.className = 'dim';
      action.textContent = 'noch nicht erzeugt';
    }

    tr.append(key, id, purpose, bus, kind, action);
    soundsEl.append(tr);
  }
}

async function trigger(entry: Entry): Promise<void> {
  await engine.unlock();
  if (entry.def.kind === 'oneshot') {
    entry.handle = engine.play(entry.def.id);
  } else if (engine.isPlaying(entry.handle)) {
    engine.stopSustained(entry.handle);
    entry.handle = -1;
  } else {
    entry.handle = engine.startSustained(entry.def.id);
  }
  renderStatus();
}

window.addEventListener('keydown', (event) => {
  if (event.repeat || event.ctrlKey || event.altKey || event.metaKey) return;
  const entry = byKey.get(event.key.toLowerCase());
  if (!entry) return;
  event.preventDefault();

  // Ab hier wird gemessen: vom Zeitstempel des Ereignisses bis zu dem Moment,
  // in dem die Quelle gestartet ist.
  const handlerStart = performance.now();
  if (entry.def.kind === 'oneshot') {
    entry.handle = engine.play(entry.def.id);
  } else if (engine.isPlaying(entry.handle)) {
    engine.stopSustained(entry.handle);
    entry.handle = -1;
  } else {
    entry.handle = engine.startSustained(entry.def.id);
  }
  const done = performance.now();
  record(handlerStart - event.timeStamp, done - handlerStart);
  renderStatus();
});

// Der Browser weckt den Ton erst nach einer Nutzergeste.
for (const type of ['keydown', 'pointerdown'] as const) {
  window.addEventListener(type, () => void engine.unlock().then(() => renderStatus()), {
    once: true,
  });
}

// Vor den ersten Zeichnen-Aufrufen deklariert: renderStatus() liest ihn.
let loadReport = '';

assignKeys();
renderFaders();
renderSounds();
renderStatus();
renderLatency();

void engine.load().then((report) => {
  assignKeys();
  renderSounds();
  const manifestFiles = new Set(
    manifest.defs.map((def) => (def.kind === 'oneshot' ? def.src : def.bodySrc)),
  );
  const missing = report.failures.filter((failure) => manifestFiles.has(failure.src));
  loadReport =
    `Dateien        ${report.loaded} geladen, ${report.seconds.toFixed(1)} s, ${(report.bytes / 1024).toFixed(0)} KiB\n` +
    `Verzeichnis    ${manifestFiles.size - missing.length} von ${manifestFiles.size} Klaengen erzeugt` +
    (manifest.problems.length ? `\nBeanstandet    ${manifest.problems.length} Eintraege` : '');
  renderStatus(loadReport);
});

// --------------------------------------------------- Zugriff fuer die Messungen

interface AudioDevTools {
  engine: AudioEngine;
  ids(): string[];
  play(id: string): VoiceHandle;
  startSustained(id: string): VoiceHandle;
  stopSustained(handle: VoiceHandle): void;
  setVolume(bus: Bus, value: number): void;
  metrics(): ReturnType<AudioEngine['metrics']>;
  counters(): { gains: number; sources: number };
  latency(): {
    count: number;
    meanMs: number;
    p95Ms: number;
    maxMs: number;
    /**
     * Was das Spiel selbst verursacht: Wartezeit bis zum Handler plus eigene
     * Arbeit plus Web-Audio-Puffer — ohne den Ausgabepuffer des Geraets, der
     * von der Umgebung abhaengt und im Kopflos-Betrieb kuenstlich gross ist.
     */
    appMeanMs: number;
    appMaxMs: number;
    queueMeanMs: number;
    queueMaxMs: number;
    workMeanMs: number;
    workMaxMs: number;
    outputLatencyMs: number;
    baseLatencyMs: number;
  };
  resetLatency(): void;
  pitch(id: string, count?: number): ReturnType<typeof measurePitchSpread>;
  seam(rounds?: number): Promise<Awaited<ReturnType<typeof measureLoopSeam>>>;
  busMix(): Promise<Awaited<ReturnType<typeof measureBusIsolation>>>;
  frames(
    id?: string,
    seconds?: number,
    playsPerSecond?: number,
  ): Promise<Awaited<ReturnType<typeof measureFramePressure>>>;
  heapProbe(
    count?: number,
    id?: string,
  ): {
    count: number;
    deltaBytes: number;
    perPlay: number;
    available: boolean;
    /**
     * Ob die Messung ueberhaupt anspricht: Vorher werden absichtlich Objekte
     * erzeugt. Bleibt auch dabei alles bei null, misst der Browser nicht fein
     * genug und der Wert oben ist wertlos.
     */
    sensitive: boolean;
    controlDeltaBytes: number;
  };
}

const tools: AudioDevTools = {
  engine,
  ids: () => engine.library.ids(),
  play: (id) => engine.play(id),
  startSustained: (id) => engine.startSustained(id),
  stopSustained: (handle) => engine.stopSustained(handle),
  setVolume: (bus, value) => engine.setVolume(bus, value),
  metrics: () => engine.metrics(),
  counters: () => ({ ...counters }),
  latency: () => {
    const total = stats(totalMs);
    const queue = stats(queueMs);
    const work = stats(workMs);
    const base = ctx.baseLatency * 1000;
    return {
      count: total.count,
      meanMs: total.mean,
      p95Ms: total.p95,
      maxMs: total.max,
      appMeanMs: queue.mean + work.mean + base,
      appMaxMs: queue.max + work.max + base,
      queueMeanMs: queue.mean,
      queueMaxMs: queue.max,
      workMeanMs: work.mean,
      workMaxMs: work.max,
      outputLatencyMs: (ctx.outputLatency || 0) * 1000,
      baseLatencyMs: base,
    };
  },
  resetLatency: () => {
    samples = 0;
    totalMs.fill(0);
    queueMs.fill(0);
    workMs.fill(0);
    renderLatency();
  },
  pitch: (id, count = 20) => measurePitchSpread(engine, id, count),
  seam: (rounds = 20) => measureLoopSeam(rounds),
  busMix: () => measureBusIsolation(),
  frames: (id = 'dev-click', seconds = 6, playsPerSecond = 20) =>
    measureFramePressure(() => engine.play(id), seconds, playsPerSecond),
  heapProbe: (count = 2000, id = 'dev-click') => {
    const memory = (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory;

    // Gegenprobe zuerst: absichtlich Objekte erzeugen und schauen, ob der
    // Browser das ueberhaupt anzeigt.
    const controlBefore = memory?.usedJSHeapSize ?? 0;
    const control: Array<{ index: number; label: string }> = [];
    for (let i = 0; i < count; i++) control.push({ index: i, label: `ton-${i}` });
    const controlDelta = (memory?.usedJSHeapSize ?? 0) - controlBefore;
    if (control.length !== count) throw new Error('Gegenprobe fehlgeschlagen');

    const before = memory?.usedJSHeapSize ?? 0;
    for (let i = 0; i < count; i++) engine.play(id);
    engine.stopAll();
    const after = memory?.usedJSHeapSize ?? 0;
    return {
      count,
      deltaBytes: after - before,
      perPlay: count === 0 ? 0 : (after - before) / count,
      available: memory !== undefined,
      sensitive: controlDelta > 0,
      controlDeltaBytes: controlDelta,
    };
  },
};

(window as unknown as { __rotaAudio: AudioDevTools }).__rotaAudio = tools;
console.info('[rota] Soundboard bereit: window.__rotaAudio');
