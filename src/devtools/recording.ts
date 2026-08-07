/**
 * Aufzeichnen und Abspielen (GAME-DESIGN 19.1, 18.4).
 *
 * Eine Aufzeichnung ist eine Liste aus **Takt und Belegung**, dazu der Startwert
 * der Zufallsströme — mehr braucht ein Kampf nicht, um sich exakt zu wiederholen.
 * Weder Zeitstempel des Browsers noch Bildzahlen gehören hinein: Die hängen am
 * Gerät, und genau davon soll die Aufzeichnung frei sein.
 *
 * Sie ist damit zweierlei (GAME-DESIGN 18.4):
 *
 * 1. **Regressionstest** — dieselbe Runde muss immer denselben Schaden ergeben
 * 2. **Abgleich mit dem Simulator** — beide Seiten spielen dieselbe Aufzeichnung
 *
 * Die Prüffälle sind ausdrücklich **aufgezeichnete echte Durchläufe**, nicht von
 * Hand geschriebene Listen. Die eingebaute Aufzeichnung `cooldown-precision`
 * entsteht deshalb, indem die Referenz-Prioritätenliste einmal durchgespielt und
 * dabei mitgeschrieben wird — nicht, indem jemand Takte abtippt.
 */
import { Combat, isCastAccepted, alsSpellIndex } from '../core/combat.js';
import { combatChecksum } from '../core/checksum.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../core/data.js';
import { GCD_MS } from '../core/gcd.js';
import { TICK_MS } from '../core/loop.js';
import { STEP1_BINDINGS } from '../core/loadout.js';
import { PressQueue } from '../input/queue.js';
import { Session } from '../core/session.js';
import { PHASE_FIGHTING } from '../core/phases.js';
import { NOTHING_WORN, type WornValues } from '../gear/worn.js';

/** Ein Tastendruck: in welchem Takt, auf welchen Platz. */
export interface RecordedPress {
  readonly tick: number;
  readonly slot: number;
}

export interface Recording {
  readonly name: string;
  /** Startwert der Zufallsströme. In Schritt 1 zieht niemand daraus. */
  readonly seed: number;
  /** Dauer in Takten. */
  readonly ticks: number;
  readonly presses: readonly RecordedPress[];
  /**
   * Die Leiste, mit der diese Runde gespielt wurde (Issue #318).
   *
   * **Die Aufzeichnung bringt ihre Leiste mit** — so wie der Simulator es seit
   * jeher tut (`replay.rs`, `spell_for_slot` liest `self.bindings`). Vorher
   * musste der Aufrufer sie reichen; vergaß er es, fiel diese Seite still auf
   * `STEP1_BINDINGS` zurück und las eine andere Leiste als der Simulator — aus
   * derselben Datei.
   *
   * Fehlt das Feld, gilt weiter `STEP1_BINDINGS`. Damit bleibt jede bestehende
   * Aufzeichnung unverändert.
   */
  readonly bindings?: readonly { readonly slot: number; readonly spellId: string }[];
}

export interface ReplayResult {
  readonly name: string;
  readonly ticks: number;
  readonly presses: number;
  readonly accepted: number;
  readonly rejected: number;
  readonly damage: number;
  /**
   * Die vier Größen **neben** dem Schaden (Issue #576).
   *
   * Das Spiel führt sie längst — `damageTaken`, `shieldAbsorbed`, `healed`
   * und `converted` stehen als Getter in `Combat`. Nur kamen sie hier nicht
   * an, und damit war jede Messung über einen Wiederholungslauf auf den
   * Schaden beschränkt.
   *
   * Was das kostete, ist gemessen (#547): Der Schadenshebel von 35 Größen kam
   * auf 0,00 % — bei zehn davon, weil ihr Träger gar keinen Schaden macht.
   * `salve` heilt, `plate` und `scab` schirmen ab, `temper` wandelt um. Eine
   * Heilung, die keinen Schaden bewegt, ist kein Befund; das ist die
   * Definition einer Heilung.
   *
   * Dieselbe Bauart wie `boss_healed` vor #564: geführt, paritätsgesichert,
   * und im Werkzeug nicht abrufbar.
   *
   * # Wo sie in den Aufzeichnungen stehen — und wo nicht
   *
   * In **31 von 52**. Die übrigen entstehen nicht über die erweiterten
   * Payload-Funktionen; dort fehlen die Felder, und das ist kein Fehler.
   * Der Bestand hat drei Bauarten:
   *
   * ```text
   *   presses + expected   39   ein Lauf je Datei
   *   runs                  4   mehrere Läufe je Datei
   *   anders                9   kein Kampflauf (z. B. buffs.json: `steps`)
   * ```
   *
   * Wer „über alle Aufzeichnungen" misst, misst je nach Werkzeug 39, 43 oder
   * 52. Genau daran ist meine erste Messung gescheitert — sie las die erste
   * Bauart und nannte das Ergebnis „keine Aufzeichnung".
   *
   * # Ein Risiko für später
   *
   * Die Rust-Seite nimmt die neuen Felder an, weil `deny_unknown_fields`
   * nicht gesetzt ist (gemessen: 89 Sammlungen, keine `unknown field`-Panik).
   * Wer es später setzt — etwa um Tippfehler in Aufzeichnungen zu fangen —,
   * bricht damit alle Dateien auf einmal, die diese Felder tragen.
   */
  readonly damageTaken: number;
  readonly absorbed: number;
  readonly healed: number;
  readonly converted: number;
  /**
   * Wann der Spieler gefallen ist, in ms — oder -1 (Issue #581, AC5).
   *
   * Das Gegenstück zu `killedAtMs`: Der eine sagt, wann der Boss fiel, der
   * andere, wann der Spieler es tat. Bisher stand nur die eine Richtung in
   * der Aufzeichnung, und ein Lauf, der mit dem Tod des Spielers endet, war
   * von einem, der die Zeit einfach auslief, nicht zu unterscheiden.
   */
  readonly diedAtMs: number;
  /**
   * Wie viel Fortschritt der Boss zurückgedreht hat (Baustein 5, Issue #609).
   *
   * **Sie steht in der Tabelle von Guide 8.70 als erster der drei Fälle:**
   * `boss_healed` wird seit #218 richtig gerechnet und war nicht abrufbar —
   * gefunden in #564 und dort nicht behoben, weil die Mechanik-Fälle grün
   * waren.
   *
   * Ohne dieses Feld kann eine Aufzeichnung nicht unterscheiden, ob wenig
   * Schaden gefallen ist oder viel zurückgedreht wurde: `damage` sieht in
   * beiden Fällen gleich aus. Bei einem Baustein, dessen ganzer Zweck das
   * Zurückdrehen ist, ist das die unangenehmste Sorte Lücke.
   */
  readonly bossHealed: number;
  /**
   * Ob der Spieler das Wettrennen gewonnen hat (Baustein 4).
   *
   * # Die Größe hatte eine SCHEINdeckung
   *
   * Sie steht in den `expected`-Blöcken der Aufzeichnungen und sah damit
   * gedeckt aus. Aber `tests/parity/payloads.ts` holte sie aus `combat`,
   * während die Nachbarzeilen desselben Blocks aus `result` lesen — sie ging
   * also am Weg vorbei, den ein Werkzeug nimmt.
   *
   * Und die Rust-Seite **führt** `race_won` (`fight.rs`: Feld, Init, Reset,
   * Setzen), vergleicht es aber nirgends — weder `replay.rs` noch eine
   * Testdatei liest es. Eine gepflegte Zahl, die nichts belegt.
   *
   * Das ist die Klasse aus `gegenstueck.test.ts`, eine Ebene tiefer: dort
   * ging es um Aufzeichnungen ohne Rust-Gegenstück, hier um ein **Feld** in
   * einer Aufzeichnung, die sonst geprüft wird.
   */
  readonly raceWon: boolean;
  /**
   * Acht kumulative Größen, die bis #630 keine eigene Zuweisung hatten.
   *
   * Alle standen in `combatChecksum` — und **genau deshalb schwieg der
   * Leitungs-Wächter**: Für ihn zählt „in der Prüfsumme" als Deckung, also
   * meldete er nichts, obwohl kein Werkzeug sie abrufen konnte. Er beantwortet
   * „gibt es einen Weg", nicht „kommt der Wert an".
   *
   * `playerDamage` trennt den Schaden des Spielers vom Fortschritt am Ring —
   * bei einem Boss mit `runaway` zählt `damage` beide Richtungen zusammen.
   * `damageLostToRace` und `phaseHoldPercent` beziffern, was ein verpasstes
   * Fenster gekostet hat. Die übrigen sind Zähler und Summen, die eine
   * Aufzeichnung belegen können soll.
   */
  readonly playerDamage: number;
  readonly damageLostToRace: number;
  readonly afflictionsCleared: number;
  readonly resourceOverflow: number;
  readonly conversionPercent: number;
  readonly phaseHoldPercent: number;
  readonly buffWindowSum: number;
  readonly synergyWindowSum: number;
  /**
   * Die beiden Kontroll-Größen (Issue #605).
   *
   * `controlFailed` zählt die Kontroll-Zauber, deren Buchung mangels Punkten
   * abbrach (`combat.ts`, `if (this.controlValue < control.cost)`).
   * `controlSpent` die tatsächlich ausgegebenen Punkte.
   *
   * # Warum sie gefehlt haben, und was das kostete
   *
   * **Es fehlte nicht die Größe, sondern die Leitung.** Beide werden im Kampf
   * gezählt, `controlFailed` steht sogar im `RunSummary` und wird nach jedem
   * Lauf als „Control wasted" angezeigt. Nur kam sie hier nicht an — und der
   * Balance-Läufer wie jede Hebel-Messung lesen ausschliesslich dieses
   * Ergebnis.
   *
   * Damit konnte **keine Aufzeichnung je belegen**, dass ein Kontroll-Zauber
   * mangels Punkten scheiterte. Gemessen an `fixation.json`: `anchor`s Kosten
   * von 6 auf 999 zu setzen ändert am Lauf-Ergebnis nichts — obwohl dann jede
   * Buchung abbricht. Das erklärt, warum in #592 alle elf Kontroll-Grössen auf
   * 0,00 % standen: Nicht weil die Kosten belanglos wären, sondern weil ihre
   * einzige beobachtbare Folge nicht abrufbar war.
   *
   * **`casts` zählt einen gescheiterten Zauber weiter als gewirkt.** Ein
   * Druck, der angenommen wurde und einen GCD ausgelöst hat, IST gewirkt — nur
   * seine Wirkung blieb aus. Wer ihn nicht zählte, verlöre die Unterscheidung
   * zwischen „nicht gedrückt" und „gedrückt, aber wirkungslos", und die zweite
   * ist die interessantere. `controlFailed` steht deshalb daneben, nicht
   * statt dessen.
   */
  readonly controlSpent: number;
  readonly controlFailed: number;
  /**
   * Die sechs Grössen, die Rust seit jeher im Ergebnis führt und das Spiel
   * nicht (Issue #619).
   *
   * # Der Befund ist Vollständigkeit, nicht Abwesenheit
   *
   * Fünf davon werden bereits verglichen — aber nur dort, wo jemand sie
   * EINZELN in ein `expected` eingetragen hat:
   *
   * ```text
   *   interruptsLanded   5 Aufzeichnungen
   *   interruptsMissed   5
   *   castsInterrupted   1   (cast-time)
   *   castsCancelled     1   (cast-time)
   *   cycle              1   (trial-blocks)
   *   stabilised         IN KEINER
   * ```
   *
   * Eine neue Aufzeichnung führt sie deshalb nicht mit. Wer eine baut und die
   * Felder nicht von Hand nachträgt, bekommt einen Lauf, in dem der Simulator
   * sechs Grössen rechnet, denen niemand widerspricht.
   *
   * **`stabilised` ist die eine echte Lücke:** Rust führt es, das Spiel führt
   * es, und kein Paritäts-Fall vergleicht die beiden Werte.
   *
   * # Warum sie hier stehen und nicht in der Prüfsumme
   *
   * Es sind Zähler über den ganzen Lauf, keine Zustandsgrössen. Die
   * Zustands-Prüfsumme vergleicht je Takt; ein Zähler, der nur am Ende etwas
   * aussagt, hinge dort 6000-mal als unveränderte Zahl.
   */
  readonly interruptsLanded: number;
  readonly interruptsMissed: number;
  readonly castsInterrupted: number;
  readonly castsCancelled: number;
  readonly cycle: number;
  /** Zahl, nicht Wahrheitswert — auf beiden Seiten (combat.ts:1323, replay.rs:120). */
  readonly stabilised: number;
  /** Kampfzeit des Sieges in ms, oder -1. */
  readonly killedAtMs: number;
  readonly casts: Readonly<Record<string, number>>;
  /** Prüfsumme des Endzustands. */
  readonly finalChecksum: number;
  /** Wie oft ein Synergie-Fenster genutzt wurde (Issue #318). */
  readonly synergyUsed: number;
  /** Wie oft eines offen stand, das der Partner hätte nutzen können. */
  readonly synergyAvailable: number;
  /** Prüfsumme über die ganze Folge — ein Wert für den schnellen Vergleich. */
  readonly sequenceChecksum: number;
  /**
   * Prüfsumme über die Folge der **Regel-Ereignisse** (Issue E3-1b).
   *
   * Steht neben der Zustands-Prüfsumme, nicht darin: Die Feldfolge von
   * `combatChecksum` bleibt damit unberührt, und die Parität aus #54 gilt
   * weiter. Beide Größen zusammen sagen, ob zwei Umsetzungen denselben
   * Zustand **und** denselben Ablauf haben.
   */
  readonly ruleSequenceChecksum: number;
  /** Wie viele Regel-Ereignisse dabei ausgetragen wurden. */
  readonly ruleEventCount: number;
  /** Prüfsumme je Takt. Nur gefüllt, wenn ausdrücklich verlangt. */
  readonly checksums: number[];
  readonly randomDraws: number;
}

/**
 * Belegung → Zauber-Kennung.
 *
 * Ohne eigene Angabe die aus `loadout.ts`. Eine eigene braucht seit Issue #304
 * genau eine Aufzeichnung: Der Prüf-Zauber zur Wirkzeit steht in keiner Leiste
 * des Spiels, weil er nicht zum Spiel gehört — ihm einen Platz in
 * `STEP1_BINDINGS` zu geben wäre die stille Änderung, die jede bestehende
 * Aufzeichnung umdeutet.
 */
function spellForSlot(
  combat: Combat,
  slot: number,
  bindings: readonly { readonly slot: number; readonly spellId: string }[] = STEP1_BINDINGS,
): number {
  for (const binding of bindings) {
    if (binding.slot === slot) return combat.indexOf(binding.spellId);
  }
  return -1;
}

/**
 * Die Optionen von [`replayRecording`] — **als benannter Typ** (Issue #391).
 *
 * # Warum er einen Namen hat
 *
 * Ein Inline-Typ lässt sich nicht abfragen. Für die Laufzeit-Prüfung braucht
 * es `keyof` — und ohne die kommt die Liste der gültigen Schlüssel nicht aus
 * der Struktur, sondern steht von Hand daneben.
 *
 * Der Anlass war schärfer als eine Formalie: `DevTools.replay` reichte
 * **eine** dieser Optionen durch und verwarf die übrigen vier stillschweigend
 * — darunter `boss`, `worn` und `level`, also genau die Schalter, die man für
 * Nachweise braucht.
 */
export interface ReplayOptions {
  readonly checksumPerTick?: boolean;
  /**
   * Gegen welchen Boss abgespielt wird. Fehlt er, ist es der erste
   * (`FIRST_CRUCIBLE`) — so bleibt jeder bestehende Aufruf unverändert.
   *
   * Wählbar seit Issue #96: Der Simulator kann es über `FightConfig.boss`
   * schon länger, und ein gemeinsamer Prüffall gegen einen Boss **mit**
   * Phasen und Energieleiste braucht dieselbe Möglichkeit auf dieser Seite.
   */
  readonly boss?: BossDef;
  /**
   * Die getragenen Sonderwerte (GAME-DESIGN 6.7, Issue #177).
   *
   * Fehlt der Wert, wird nichts getragen — jeder bestehende Aufruf bleibt
   * damit unverändert, und genau das verlangt AC7.
   *
   * Wählbar, weil AC8 eine Prüfsumme je Takt über einen Lauf **mit**
   * Ausrüstung fordert. Ohne diesen Schalter ließe sich die Bitgleichheit
   * nur für den Fall belegen, in dem die neuen Werte gar nichts tun.
   */
  readonly worn?: WornValues;
  /**
   * Die Stufe des Spielers (Issue #289, AC5).
   *
   * **Fehlt sie, ist es Stufe 1** — und daran hängt, dass keine bestehende
   * Vergleichsdatei wandert. Dieselbe Bauart wie `boss` und `worn`.
   *
   * Wählbar, weil AC5 verlangt, dass beide Seiten DASSELBE liefern, nicht
   * nur dieselbe Tabelle lesen. Bis zum 2026-08-05 lag keine einzige
   * Aufzeichnung über Stufe 1 — dort ist `statAtLevel` neutral, und ein
   * Auseinanderlaufen wäre unsichtbar geblieben. Genau das ist die Klasse
   * „der Prüffall kann den Unterschied nicht zeigen", und bei der
   * Bitgleichheit ist es die teuerste Stelle, an der sie stehen kann.
   */
  readonly level?: number;
  /**
   * Welcher Platz welchen Zauber trägt (Issue #304).
   *
   * Fehlt sie, ist es die Leiste aus Schritt 1 — jeder bestehende Aufruf
   * bleibt damit unverändert. Dieselbe Bauart wie `boss`, `worn` und `level`.
   */
  readonly bindings?: readonly { readonly slot: number; readonly spellId: string }[];
}

/**
 * Die gültigen Schlüssel von [`ReplayOptions`] — **zur Laufzeit** (Issue #391).
 *
 * Dieselbe Bauart wie in `reference-run.ts`: `Required<{ [K in keyof T]: true }>`
 * zwingt den Übersetzer, jedes Feld zu verlangen. Kommt eines dazu, ist diese
 * Stelle rot, bis es hier steht — die Liste kommt damit **aus** der Struktur
 * statt daneben zu stehen.
 */
const GUELTIGE_OPTIONEN: Required<{ [K in keyof ReplayOptions]: true }> = {
  checksumPerTick: true,
  boss: true,
  worn: true,
  level: true,
  bindings: true,
};

/**
 * Meldet ein unbekanntes Feld — **mit seinem Namen**.
 *
 * # Warum eine Laufzeit-Prüfung, wo TypeScript das doch abfängt
 *
 * Weil die Dev-Werkzeuge **keine** TypeScript-Schnittstelle sind. Über
 * `window.__rota` tippt ein Mensch in eine Browser-Konsole; dort gibt es
 * keinen Übersetzer, der ihn korrigiert. Ein Tippfehler wie `bindigs` wäre
 * bis hierher stumm durchgelaufen und hätte einen Lauf mit der Standard-
 * Leiste gemessen — mit einer Zahl, die aussieht, als gehöre sie zum Aufruf.
 */
function pruefeOptionen(options: ReplayOptions): void {
  const unbekannt = Object.keys(options).filter((key) => !(key in GUELTIGE_OPTIONEN));
  if (unbekannt.length > 0) {
    throw new Error(
      `Unbekannte Option(en) fuer replay: ${unbekannt.join(', ')}. ` +
        `Gueltig: ${Object.keys(GUELTIGE_OPTIONEN).join(', ')}.`,
    );
  }
}

/**
 * Eine Aufzeichnung abspielen — **ohne Browser, ohne Uhr, ohne Bild**.
 *
 * Damit ist das Ergebnis von Bildrate und Systemlast unabhängig; zwei Aufrufe
 * liefern byteweise dasselbe.
 */
export function replayRecording(recording: Recording, options: ReplayOptions = {}): ReplayResult {
  pruefeOptionen(options);
  const combat = new Combat({
    school: CALCINATION.value,
    boss: options.boss ?? FIRST_CRUCIBLE.value,
    seed: recording.seed,
    worn: options.worn ?? NOTHING_WORN,
    level: options.level ?? 1,
  });
  const wantChecksums = options.checksumPerTick === true;
  const checksums: number[] = [];
  let sequence = 0;

  let accepted = 0;
  let rejected = 0;
  let killedAtMs = -1;
  let pressIndex = 0;

  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (killedAtMs < 0 && combat.damageDealt >= combat.boss.hp) {
      killedAtMs = timeMs;
      // Abbrechen wie im Spiel: Der Kampf endet mit dem Sieg, und der Schaden
      // bleibt damit mit der Referenz-Rechnung vergleichbar.
      const hash = combatChecksum(combat, timeMs);
      sequence = (sequence * 31 + hash) >>> 0;
      if (wantChecksums) checksums.push(hash);
      break;
    }

    // **Der Spielertod endet genauso** (GAME-DESIGN A.3, Issue #567) — mit der
    // Prüfsumme dieses Takts, damit beide Seiten denselben letzten Eintrag
    // haben.
    //
    // Die Stelle ist nicht frei wählbar: NACH dem Boss-Tod und VOR den Drücken,
    // wie `Fight::replay` in `sim/src/fight.rs`. Eine Zeile weiter oben oder
    // unten ergäbe denselben Kampf und eine andere Prüfsummen-Folge — der
    // Unterschied fiele erst in der Parität auf, und dort sähe er nach einem
    // Rechenfehler aus statt nach einer Reihenfolge.
    //
    // Bis zum 2026-08-07 kannte diese Schleife nur den Tod des BOSSES; der
    // Kommentar in `doomStacksPayload` hat genau deshalb auf sie verzichtet.
    if (combat.playerDead) {
      const hash = combatChecksum(combat, timeMs);
      sequence = (sequence * 31 + hash) >>> 0;
      if (wantChecksums) checksums.push(hash);
      break;
    }

    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      // **Die Leiste der Aufzeichnung hat Vorrang vor gar keiner** (#318).
      // `options.bindings` bleibt oben, weil ein Aufrufer sie bewusst
      // überschreiben darf; fehlt sie, kommt die aus der Datei statt der
      // festen aus `loadout.ts`.
      const spellIndex = spellForSlot(combat, press.slot, options.bindings ?? recording.bindings);
      if (spellIndex < 0) {
        rejected++;
        continue;
      }
      // **Ein begonnener Zauber zählt als angenommen** (Issue #304). Der
      // Vergleich gegen die nackte 0 stand hier bis dahin und wäre jetzt
      // falsch: Ein Zauber mit Wirkzeit meldet einen anderen Ausgang, und die
      // Aufzeichnung zählte ihn als Fehleingabe — während der Simulator ihn
      // annimmt. Genau die Sorte Unterschied, die in der Prüfsumme nicht steht,
      // weil sie die Zähler nicht führt.
      if (isCastAccepted(combat.cast(alsSpellIndex(spellIndex), timeMs).result)) accepted++;
      else rejected++;
    }

    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    if (wantChecksums) checksums.push(hash);
  }

  const casts: Record<string, number> = {};
  for (let i = 0; i < combat.spells.length; i++) {
    casts[combat.spells[i]?.id ?? String(i)] = combat.castCount(alsSpellIndex(i));
  }

  return {
    name: recording.name,
    ticks: recording.ticks,
    presses: recording.presses.length,
    accepted,
    rejected,
    damage: combat.damageDealt,
    // Die vier Größen neben dem Schaden (#576) — aus denselben Gettern, die
    // das HUD liest. Sie gehen NICHT in eine Prüfsumme ein: Die Feldfolge von
    // `combatChecksum` bleibt unberührt, damit die Parität aus #54 gilt.
    damageTaken: combat.damageTaken,
    absorbed: combat.shieldAbsorbed,
    healed: combat.healed,
    converted: combat.converted,
    diedAtMs: combat.diedAtMs,
    // **Die Heilung des Bosses** (#609, nach der Regel aus Guide 8.70). Sie
    // wird seit #218 richtig gerechnet und war bis hierher nicht abrufbar —
    // #564 hat genau das gefunden und die Leitung nicht gelegt. Eine Größe,
    // die den Fortschritt ZURÜCKDREHT, ohne dass eine Aufzeichnung sie nennen
    // kann, ist die unangenehmste Sorte: `damage` sieht dann einfach kleiner
    // aus, und niemand weiß, ob weniger getroffen oder mehr zurückgedreht
    // wurde.
    bossHealed: combat.bossHealed,
    raceWon: combat.raceWon,
    // **Die acht kumulativen Größen aus #630.** Sie standen alle in
    // `combatChecksum` und hatten keine eigene Zuweisung — für den
    // Leitungs-Wächter galten sie damit als gedeckt, obwohl kein Werkzeug sie
    // abrufen konnte. Dieselbe Lage wie `bossHealed` vor #609 und
    // `controlSpent` vor #605.
    //
    // Auch die, die heute niemand liest: Acht der zehn hatten außerhalb von
    // `combat.ts` null Treffer. Das war bei den elf Kontroll-Größen aus #605
    // genauso — sie standen auf 0,00 %, **weil** ihre Leitung fehlte. Eine
    // Größe ohne Leitung sieht immer unbenutzt aus.
    playerDamage: combat.playerDamage,
    damageLostToRace: combat.damageLostToRace,
    afflictionsCleared: combat.afflictionsCleared,
    resourceOverflow: combat.resourceOverflow,
    conversionPercent: combat.conversionPercent,
    phaseHoldPercent: combat.phaseHoldPercent,
    buffWindowSum: combat.buffWindowSum,
    synergyWindowSum: combat.synergyWindowSum,
    controlSpent: combat.controlSpent,
    controlFailed: combat.controlFailed,
    interruptsLanded: combat.interruptsLanded,
    interruptsMissed: combat.interruptsMissed,
    castsInterrupted: combat.castsInterrupted,
    castsCancelled: combat.castsCancelled,
    cycle: combat.cycle,
    stabilised: combat.stabilised,
    killedAtMs,
    casts,
    finalChecksum: combatChecksum(
      combat,
      killedAtMs >= 0 ? killedAtMs : (recording.ticks - 1) * TICK_MS,
    ),
    sequenceChecksum: sequence,
    synergyUsed: combat.synergyUsed,
    synergyAvailable: combat.synergyAvailable,
    ruleSequenceChecksum: combat.events.sequenceChecksum,
    ruleEventCount: combat.events.emittedCount,
    checksums,
    randomDraws: combat.random.totalDraws,
  };
}

/**
 * Die gute Rotation einmal durchspielen und dabei mitschreiben.
 *
 * Ergibt eine **echte** Aufzeichnung statt einer abgetippten Liste — die
 * Bedingung aus GAME-DESIGN 18.4. Die Prioritätenliste ist dieselbe wie in
 * `reference-run.ts` und wie in `tools/step1-check.py`.
 */
function recordGoodRotation(name: string, ticks: number, boss?: BossDef): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: boss ?? FIRST_CRUCIBLE.value });
  const kindle = combat.indexOf('kindle');
  const smoulder = combat.indexOf('smoulder');
  const pyre = combat.indexOf('pyre');
  const slotOf = new Map<number, number>();
  for (const binding of STEP1_BINDINGS) slotOf.set(combat.indexOf(binding.spellId), binding.slot);

  const presses: RecordedPress[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    // **Und ebenso beim Spielertod** (GAME-DESIGN A.3, Issue #567). Bis zum
    // 2026-08-07 stand hier nur der Boss-Tod: Eine Aufzeichnung lief über den
    // eigenen Tod hinaus und schrieb Drücke weiter, die nichts mehr auslösen.
    // Gemessen an `instance1-boss-2`: 41 Drücke wurden zu 4212, davon 4200
    // abgelehnt. Der Versuch ist vorbei — die Aufzeichnung endet mit ihm.
    if (combat.playerDead) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    let choice = -1;
    if (combat.dotRemainingMs(smoulder, timeMs) < GCD_MS && combat.isReady(smoulder, timeMs)) {
      choice = smoulder;
    } else if (
      combat.isReady(pyre, timeMs) &&
      combat.resource >= (combat.spellAt(pyre)?.spender?.bonusThreshold ?? 0)
    ) {
      choice = pyre;
    } else if (combat.isReady(kindle, timeMs)) {
      choice = kindle;
    }
    if (choice < 0) continue;
    combat.cast(alsSpellIndex(choice), timeMs);
    presses.push({ tick, slot: slotOf.get(choice) ?? 0 });
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Eine Runde, in der **die neuen Kalzinations-Zauber** gewirkt werden
 * (Issue #268).
 *
 * # Warum sie eine eigene Aufzeichnung braucht
 *
 * `coverage.test.ts` verlangt, dass jeder ausgelieferte Zauber in **einem**
 * Paritäts-Weg gewirkt wird — sonst gilt die Bitgleichheit zwischen Spiel und
 * Simulator für ihn als *angenommen*, nicht als geprüft.
 *
 * Eine bestehende Aufzeichnung dafür umzubauen wäre die kürzere Lösung und
 * die schlechtere: Eine Aufzeichnung ist der **Beleg eines bestimmten
 * Issues**. Wer `cooldown-precision` erweitert, ändert den Beleg für #96 — und
 * zwar so, dass er **grün bleibt**, weil die Prüfsummen mitexportiert werden.
 * Der Fall bestünde weiter und belegte etwas anderes als vorher; niemand hätte
 * einen Anlass nachzusehen.
 *
 * # Was sie drückt
 *
 * Eine feste, kurze Folge statt einer klugen Rotation: `searing` sobald sein
 * Brand ausläuft, sonst `bellows`. Was hier zählt, ist **dass beide gewirkt
 * werden**, nicht wie gut die Reihenfolge ist — die Frage nach der besten
 * Rotation beantwortet der Läufer aus #237, nicht dieser Weg.
 */
function recordNewCalcination(name: string, ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const bellows = combat.indexOf('bellows');
  const searing = combat.indexOf('searing');

  const presses: RecordedPress[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    // **Und ebenso beim Spielertod** (GAME-DESIGN A.3, Issue #567). Bis zum
    // 2026-08-07 stand hier nur der Boss-Tod: Eine Aufzeichnung lief über den
    // eigenen Tod hinaus und schrieb Drücke weiter, die nichts mehr auslösen.
    // Gemessen an `instance1-boss-2`: 41 Drücke wurden zu 4212, davon 4200
    // abgelehnt. Der Versuch ist vorbei — die Aufzeichnung endet mit ihm.
    if (combat.playerDead) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    const choice =
      combat.dotRemainingMs(searing, timeMs) < GCD_MS && combat.isReady(searing, timeMs)
        ? searing
        : bellows;
    if (!combat.isReady(choice, timeMs)) continue;
    combat.cast(alsSpellIndex(choice), timeMs);
    // Platz 0 und 1 — die Bindung steht im Payload, nicht hier.
    presses.push({ tick, slot: choice === searing ? 0 : 1 });
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Eine Runde, die vor allem die Wischblenden bewegt.
 *
 * Für die Animationsprüfung (VERIFICATION § 4): PYRE wird gezündet, sobald es
 * bereit ist — fünf Sekunden Blende, die längste im Spiel —, dazwischen KINDLE.
 * Ob dabei viel Schaden entsteht, ist gleichgültig; sichtbare Bewegung zählt.
 */
function recordCooldownSweep(name: string, ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const kindle = combat.indexOf('kindle');
  const pyre = combat.indexOf('pyre');
  const slotOf = new Map<number, number>();
  for (const binding of STEP1_BINDINGS) slotOf.set(combat.indexOf(binding.spellId), binding.slot);

  const presses: RecordedPress[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (!combat.gcd.isReady(timeMs)) continue;
    const choice = combat.isReady(pyre, timeMs)
      ? pyre
      : combat.isReady(kindle, timeMs)
        ? kindle
        : -1;
    if (choice < 0) continue;
    combat.cast(alsSpellIndex(choice), timeMs);
    presses.push({ tick, slot: slotOf.get(choice) ?? 0 });
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Eine Runde, die den **Auswahlmodus** benutzt (Issue #223).
 *
 * Sie ist der einzige Weg, den Ruecksetzer in einer Aufzeichnung zu haben —
 * und Aufzeichnungen entstehen aus einem echten Lauf, nicht aus einer
 * abgetippten Druckliste (GAME-DESIGN 18.4).
 *
 * **Ueber eine `Session`, nicht ueber `Combat`.** Der Auswahlmodus lebt in der
 * Sitzung: Sie entscheidet, ob ein Druck wirkt oder waehlt. Ein Rekorder, der
 * `combat.cast` ruft, wuerde REFLUX als gewoehnlichen Zauber wirken — und die
 * Aufzeichnung enthielte den Modus nie.
 *
 * Die Abfolge ist bewusst schlicht und wiederholt sich: PYRE wirken (er hat
 * die laengste Abklingzeit der drei), dann REFLUX, dann PYRE waehlen — er ist
 * damit sofort wieder bereit. Dazwischen KINDLE als Fueller.
 */
function recordResetRotation(name: string, ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const queue = new PressQueue();
  const session = new Session(queue, combat);
  const belegung = [
    { slot: 0, id: 'kindle' },
    { slot: 1, id: 'pyre' },
    { slot: 2, id: 'reflux' },
  ];
  for (const { slot, id } of belegung) session.bind(slot, combat.indexOf(id));
  const kindle = combat.indexOf('kindle');
  const pyre = combat.indexOf('pyre');
  const reflux = combat.indexOf('reflux');

  const presses: RecordedPress[] = [];
  // **`startFight` statt `beginCountdown`.** Der Simulator kennt keinen
  // Countdown: `Fight::replay` beginnt bei Takt 0 mit der Kampfuhr. Mit
  // Countdown liefe die Kampfuhr hier drei Sekunden spaeter los, und die
  // Zustaende waeren von Anfang an verschieden — die Paritaet scheiterte an
  // der Aufzeichnung, nicht an der Rechnung. Derselbe Weg wie in
  // `cooldownResetPayload` (E3-3c).
  session.startFight(0);
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    // Der Druck wird VOR dem Takt eingereiht — die Sitzung arbeitet ihn in
    // diesem Takt ab, genau wie im Spiel.
    let slot = -1;
    if (session.phase === PHASE_FIGHTING && combat.gcd.isReady(timeMs)) {
      if (session.isResetArmed) {
        // Steht der Modus scharf, ist der naechste Druck die WAHL. PYRE liegt
        // dann auf Abklingzeit — genau darauf soll sie fallen.
        slot = 1;
      } else if (combat.isReady(reflux, timeMs) && !combat.isOffCooldown(pyre, timeMs)) {
        slot = 2;
      } else if (combat.isReady(pyre, timeMs)) {
        slot = 1;
      } else if (combat.isReady(kindle, timeMs)) {
        slot = 0;
      }
    }
    if (slot >= 0) {
      queue.push(slot, tick, timeMs, tick);
      presses.push({ tick, slot });
    }
    session.tick(timeMs);
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Die beiden letzten Bausteine in EINEM Lauf (Issue #240, AC6).
 *
 * **Mit der Zauberleiste aus Schritt 1**, ohne eigene Belegung. Sie war einen
 * Entwurf lang noetig, weil CRUST dort keinen Platz hat und die Reaktion sonst
 * nie gehoben worden waere; mit dem Herausschneiden von Baustein 6 ist der
 * Grund entfallen — und ein Sonderweg ohne Grund ist einer zu viel. QUENCH
 * steht auf Platz 3 und entfernt die Schwaechung.
 *
 * Die Abfolge ist bewusst schlicht: QUENCH, sobald die Schwaechung liegt,
 * sonst SMOULDER und KINDLE als Fueller. Damit steht neben den passiven
 * Haelften (Verkuerzen, Schaden) auch die Handlung im Lauf, mit der der
 * Spieler dem Baustein begegnet.
 */
function recordTrialRotation(name: string, ticks: number, boss: BossDef): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss });
  const index = new Map(STEP1_BINDINGS.map(({ slot, spellId }) => [slot, combat.indexOf(spellId)]));

  const presses: RecordedPress[] = [];
  // **Kein `beginFight`** — die Wiedergabe (`replayRecording`) ruft es
  // ebenfalls nicht, und Aufnahme und Wiedergabe müssen denselben Zustand
  // erzeugen. Was `beginFight` an Uhren setzt, setzt der Konstruktor schon.
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    // **Und ebenso beim Spielertod** (GAME-DESIGN A.3, Issue #567). Bis zum
    // 2026-08-07 stand hier nur der Boss-Tod: Eine Aufzeichnung lief über den
    // eigenen Tod hinaus und schrieb Drücke weiter, die nichts mehr auslösen.
    // Gemessen an `instance1-boss-2`: 41 Drücke wurden zu 4212, davon 4200
    // abgelehnt. Der Versuch ist vorbei — die Aufzeichnung endet mit ihm.
    if (combat.playerDead) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    let slot = -1;
    for (const kandidat of [3, 1, 0]) {
      const i = index.get(kandidat) ?? -1;
      if (i < 0 || !combat.isReady(alsSpellIndex(i), timeMs)) continue;
      // Platz 3 nur, wenn es etwas zu entfernen gibt — sonst waere der GCD
      // verschenkt (#224).
      if (kandidat === 3 && !combat.afflicted) continue;
      if (kandidat === 1 && combat.dotRemainingMs(alsSpellIndex(i), timeMs) >= GCD_MS) continue;
      slot = kandidat;
      break;
    }
    if (slot >= 0 && combat.cast(alsSpellIndex(index.get(slot) ?? -1), timeMs)) {
      presses.push({ tick, slot });
    }
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Die Belegung fuer die Wirkzeit-Aufzeichnung (Issue #304).
 *
 * **Eine eigene und nicht `STEP1_BINDINGS`**: Der Pruef-Zauber steht in keiner
 * Leiste des Spiels, weil er nicht zum Spiel gehoert. Ein Platz fuer ihn in der
 * gemeinsamen Belegung waere genau die stille Aenderung, die eine bestehende
 * Aufzeichnung umdeutet.
 */
const CAST_TIME_BINDINGS: readonly { slot: number; id: string }[] = [
  { slot: 0, id: 'kindle' },
  { slot: 1, id: 'test-cast' },
];

/**
 * Eine Runde mit einem Zauber, der eine WIRKZEIT hat (Issue #304, AC5).
 *
 * **Bis zu diesem Issue hat keine einzige Vergleichsdatei eine Spieler-Wirkzeit
 * gefahren** — es gab keine: Alle fuenfzehn Zauber standen auf `castTimeMs: 0`.
 * Die Bitgleichheit war fuer diesen ganzen Zweig damit unbelegt, und ein
 * Auseinanderlaufen waere unsichtbar geblieben, weil kein Fall es zeigen konnte.
 * Dieselbe Lage wie bei `runaway` vor #256 und bei der Stufe vor #289.
 *
 * DREI DINGE IN EINEM LAUF, und der Boss `test-caster` ist darauf gerechnet:
 *
 * | | |
 * | --- | --- |
 * | 0 ms | Der Zauber beginnt und wirkt bei 2000 ms — der Schaden faellt am ENDE |
 * | 8500 ms | Ein gewoehnlicher Schlag trifft die laufende Wirkzeit und stoert sie NICHT |
 * | 9000 ms | Ein Schlag mit `interruptsPlayer` bricht sie ab |
 *
 * Der Fueller ist KINDLE. Er steht nicht zur Zierde da: Ohne ihn liefe zwischen
 * den Wirkzeiten gar nichts, und eine Abweichung im gewoehnlichen Teil des
 * Kampfes fiele in dieser Aufzeichnung nicht auf.
 */
function recordCastTimeRotation(name: string, ticks: number, boss: BossDef): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss });
  const slotOf = new Map<number, number>();
  for (const { slot, id } of CAST_TIME_BINDINGS) slotOf.set(combat.indexOf(id), slot);
  const kindle = combat.indexOf('kindle');
  const wirkzeit = combat.indexOf('test-cast');

  const presses: RecordedPress[] = [];
  let selbstAbgebrochen = false;
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    // **Und ebenso beim Spielertod** (GAME-DESIGN A.3, Issue #567). Bis zum
    // 2026-08-07 stand hier nur der Boss-Tod: Eine Aufzeichnung lief über den
    // eigenen Tod hinaus und schrieb Drücke weiter, die nichts mehr auslösen.
    // Gemessen an `instance1-boss-2`: 41 Drücke wurden zu 4212, davon 4200
    // abgelehnt. Der Versuch ist vorbei — die Aufzeichnung endet mit ihm.
    if (combat.playerDead) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    // **Während einer Wirkzeit wird gewartet, nicht gedrückt.** Seit der
    // Umkehrung vom 2026-08-05 bräche jeder Fülldruck den laufenden Zauber ab
    // — gemessen: Ein Rekorder, der KINDLE weiterdrückt, kommt auf 0 Wirkungen
    // und 2 Abbrüche in 20 Sekunden. Genau das tut ein Spieler nicht, der
    // seinen Zauber will.
    if (combat.playerCasting >= 0) {
      // **Genau EIN Abbruch von Hand**, damit die Vergleichsdatei auch den
      // zweiten Auslöser trägt. Ohne ihn prüfte die Parität nur den Abbruch
      // durch den Boss — und der eigene Druck ist der häufigere Fall.
      // **Der Zeitpunkt ist gerechnet, nicht gewählt.** Ein Abbruch von Hand
      // braucht ein Fenster, in dem eine Wirkzeit läuft UND die Wartezeit frei
      // ist: bei 1500 ms Wartezeit und 2000 ms Wirkzeit sind das die letzten
      // 500 ms jeder Wirkzeit. Die letzte dieser Gelegenheiten liegt bei
      // 20500 ms — davor würde der Abbruch einen der beiden Boss-Schläge um
      // seinen Fall bringen, weil die folgenden Wirkzeiten sich verschieben.
      //
      // Mein erster Entwurf stand auf 12000 ms und traf gar kein Fenster: Die
      // Aufzeichnung meldete 0 Abbrüche von Hand, und der zweite Auslöser wäre
      // in der Parität unbelegt geblieben.
      if (selbstAbgebrochen || timeMs < 20_000) continue;
      selbstAbgebrochen = true;
      combat.cast(kindle, timeMs);
      presses.push({ tick, slot: slotOf.get(kindle) ?? 0 });
      continue;
    }

    // **Der Zauber mit Wirkzeit hat Vorrang**, sobald er bereit ist. Wäre
    // KINDLE zuerst dran, verschöbe sein GCD die Wirkzeit bei jedem Zyklus um
    // einen anderen Betrag — und die gerechneten Zeitpunkte des Bosses
    // (8500/9000 ms) träfen nichts Bestimmtes mehr.
    const wahl = combat.isReady(wirkzeit, timeMs)
      ? wirkzeit
      : combat.isReady(kindle, timeMs)
        ? kindle
        : -1;
    if (wahl < 0) continue;
    combat.cast(alsSpellIndex(wahl), timeMs);
    presses.push({ tick, slot: slotOf.get(wahl) ?? 0 });
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Die Belegung für die GEMISCHTE Leiste (Issue #328, AC7).
 *
 * Drei Schulen auf drei Plätzen. **Das ist der Punkt**: Seit der Widerstand am
 * ZAUBER hängt und nicht am Kampf, belegt eine Aufzeichnung mit lauter
 * Kalzinations-Zaubern nur noch den Fall, in dem alle dieselbe Schule haben —
 * also genau den, der die Umstellung nicht braucht.
 */
const MIXED_BINDINGS: readonly { slot: number; id: string }[] = [
  // **Die Reihenfolge ist der Vorrang**, und sie ist gemessen, nicht geraten:
  // Mein erster Entwurf setzte KINDLE nach vorn. Er ist nach jeder Wartezeit
  // wieder bereit, kam deshalb immer an die Reihe, und die beiden anderen
  // wurden in 20 Sekunden **kein einziges Mal** gewirkt — die Vergleichsdatei
  // meldete `{"kindle":1050,"bind":0,"canker":0}`. Eine gemischte Leiste, auf
  // der nur eine Schule vorkommt, prüft genau das, was sie prüfen soll, nicht.
  { slot: 0, id: 'bind' }, // FIXATION — unberührt, lange Abklingzeit
  { slot: 1, id: 'canker' }, // PUTREFACTION — unberührt
  { slot: 2, id: 'kindle' }, // CALCINATION — vom Widerstand getroffen
  // **SMOULDER ist der wichtigste der vier**, und er hat zuerst gefehlt: Er
  // ist der einzige Zauber mit einem Effekt über Zeit. Ohne ihn fährt diese
  // Aufzeichnung drei Schulen und trotzdem keine Brand-Wirkung unter
  // Widerstand — und die Zeile, die dem Tick die Schule SEINES Zaubers gibt,
  // wäre von nichts bewacht. Gemessen: Ohne ihn ließ sich die Zuweisung aus
  // `fight.rs` entfernen, ohne dass ein einziger Fall rot wurde.
  { slot: 3, id: 'smoulder' }, // CALCINATION, mit Brand — tickt zwischen den anderen
];

/**
 * Eine Runde mit Zaubern aus DREI Schulen (Issue #328, AC7).
 *
 * Gegen einen Boss, der nur eine davon abhält. Damit steht in der
 * Vergleichsdatei ein Kampf, in dem die Zauber verschieden viel Schaden machen
 * — und beide Seiten müssen dieselbe Zuordnung treffen, nicht nur dieselbe
 * Summe.
 *
 * Die Abfolge ist schlicht und hängt an keiner Entscheidung: der erste
 * bereite Zauber in fester Reihenfolge. Eine Rotation, die abwägt, verglichen
 * zwei Kämpfe statt zweier Rechnungen.
 */
function recordMixedRotation(name: string, ticks: number, boss: BossDef): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss });
  const plaetze = MIXED_BINDINGS.map(({ slot, id }) => ({ slot, index: combat.indexOf(id) }));

  const presses: RecordedPress[] = [];
  // **Reihum, nicht nach Vorrang** — und das ist gemessen, nicht Geschmack:
  // Alle drei Zauber haben 1500 bis 2000 ms Abklingzeit, die Wartezeit hat
  // 1500. Der erste in einer Vorrangliste ist damit IMMER wieder dran, und die
  // anderen kommen nie vor. Zwei Anläufe haben das gezeigt, jeder mit einer
  // anderen Reihenfolge:
  //
  //   KINDLE zuerst   {"kindle":1050,"bind":0,"canker":0}
  //   BIND zuerst     {"bind":840,"canker":0,"kindle":0}
  //
  // Eine „gemischte" Leiste, auf der nur eine Schule vorkommt, prüft genau das
  // nicht, wofür es sie gibt. Reihum kommen alle drei dran.
  let naechster = 0;
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    // **Und ebenso beim Spielertod** (GAME-DESIGN A.3, Issue #567). Bis zum
    // 2026-08-07 stand hier nur der Boss-Tod: Eine Aufzeichnung lief über den
    // eigenen Tod hinaus und schrieb Drücke weiter, die nichts mehr auslösen.
    // Gemessen an `instance1-boss-2`: 41 Drücke wurden zu 4212, davon 4200
    // abgelehnt. Der Versuch ist vorbei — die Aufzeichnung endet mit ihm.
    if (combat.playerDead) break;
    if (!combat.gcd.isReady(timeMs)) continue;
    const wahl = plaetze[naechster % plaetze.length];
    if (wahl === undefined || wahl.index < 0 || !combat.isReady(wahl.index, timeMs)) continue;
    combat.cast(wahl.index, timeMs);
    presses.push({ tick, slot: wahl.slot });
    naechster++;
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Die eingebauten Aufzeichnungen.
 *
 * Erzeugt beim ersten Zugriff, nicht beim Laden: Wer die Werkzeuge nicht
 * benutzt, zahlt nichts dafür.
 */
let builtIn: Map<string, Recording> | null = null;

export function recordings(): Map<string, Recording> {
  if (builtIn === null) {
    builtIn = new Map<string, Recording>();
    // 60 s Kampfdauer des Testbosses, in Takten.
    builtIn.set('cooldown-precision', recordGoodRotation('cooldown-precision', 6000));
    builtIn.set('cooldown-sweep', recordCooldownSweep('cooldown-sweep', 2000));
    // Issue #268: die neuen Kalzinations-Zauber, damit coverage.test.ts sie
    // als GEWIRKT sieht und nicht nur als vorhanden.
    builtIn.set('new-calcination', recordNewCalcination('new-calcination', 3000));
    // Der gemeinsame Pruefall gegen einen Boss **mit** Phasen, Energieleiste und
    // Zeitleiste (Issue #96, AC4). 90 s Ueberdruck des Testgefaesses, in Takten.
    //
    // Dieselbe gute Rotation wie oben, nur gegen einen anderen Boss: Alles, was
    // sich zwischen beiden Aufzeichnungen unterscheidet, kommt damit aus den
    // Boss-Daten und aus nichts sonst.
    const vessel = bossById('test-vessel');
    if (vessel !== undefined) {
      builtIn.set('vessel-phases', recordGoodRotation('vessel-phases', 9000, vessel));
    }
    // Der Pruefall gegen Zaehler und exponentiellen Stapel (Issue #216).
    // 30 s in Takten — der Zaehler toetet bei 20 s, und die zehn Sekunden
    // danach gehoeren dazu: Sie belegen, dass beide Seiten nach dem Tod
    // dasselbe tun (naemlich nichts mehr).
    const doomsayer = bossById('test-doomsayer');
    if (doomsayer !== undefined) {
      builtIn.set('doom-stacks', recordGoodRotation('doom-stacks', 3000, doomsayer));
    }
    // Der Prueffall zum Auswahlmodus (Issue #223). 60 s in Takten: REFLUX hat
    // 45 s Abklingzeit, ein kuerzerer Lauf zeigte ihn nur EINMAL. Gemessen —
    // bei 20 s stand genau ein Druck auf Platz 2 in der Aufzeichnung.
    builtIn.set('cooldown-reset-run', recordResetRotation('cooldown-reset-run', 6000));
    // Der Prueffall zum Wettrennen und zur Boss-Heilung (Issue #218). 40 s in
    // Takten: Das Fenster liegt bei 10 s, der Boss-Zauber bei 30 s und trifft
    // nach 3 s — beide Bausteine kommen also in EINEM Lauf vor.
    // Der Prueffall zur davonlaufenden Reaktion mit SPIELERSCHADEN (Issue
    // #256, AC3). 2000 Takte sind 20 s — genau die Frist.
    //
    // **Es gab bis zum 2026-08-05 keine einzige Vergleichsdatei mit
    // `runaway`.** `parity_runaway.rs` prueft Zahlen, keine Pruefsummen; die
    // Pruefsummen-Dateien fahren alle Bosse ohne Reaktion. Damit war der
    // Simulator an dieser Stelle ungeprueft — aufgefallen an einer Gegenprobe,
    // die NICHT anschlug: Wer `runaway_tick` dort die falsche Herkunft
    // mitgeben liess, brach keinen Test.
    //
    // Die gute Rotation ist hier bewusst dieselbe wie sonst: Sie macht
    // Schaden, und der Ring faellt zusaetzlich von selbst. Beide Herkuenfte
    // kommen damit in EINEM Lauf vor — was der ganze Zweck ist.
    const runawayBoss = bossById('test-runaway');
    if (runawayBoss !== undefined) {
      builtIn.set('runaway-progress', recordGoodRotation('runaway-progress', 2000, runawayBoss));
    }
    // Der Prueffall zur STUFE (Issue #289, AC5). 3000 Takte sind 30 s.
    //
    // **Bis zum 2026-08-05 lag keine einzige Aufzeichnung ueber Stufe 1** —
    // und dort ist `statAtLevel` neutral. Die Bitgleichheit war damit fuer
    // jede hoehere Stufe unbelegt: Ein Auseinanderlaufen waere unsichtbar
    // geblieben, weil kein Fall es zeigen konnte.
    //
    // Stufe 30 und nicht 2: Der Faktor ist dort 4,116 und trennt deutlich.
    // Bei 1,05 saehe ein Rundungsfehler aus wie Uebereinstimmung.
    builtIn.set('level-30', recordGoodRotation('level-30', 3000));
    // Der Prueffall zu den WIDERSTAENDEN (Issue #328, AC7). 2000 Takte sind
    // 20 s.
    //
    // **Dieselbe gute Rotation wie sonst, nur gegen einen anderen Boss** —
    // dieselbe Bauart wie `vessel-phases` (#96): Alles, was sich zwischen
    // dieser Aufzeichnung und einer anderen unterscheidet, kommt damit aus den
    // BOSS-Daten und aus nichts sonst.
    //
    // Bis zu diesem Issue hat keine Vergleichsdatei einen Boss mit
    // Widerstaenden gefahren — es gab keinen. Der Simulator las das Feld gar
    // nicht (`grep -c resist sim/build.rs` -> 0), und die Abweichung war
    // unsichtbar, weil beide Seiten bei leeren Daten auf dieselbe Eins kamen.
    const resistantBoss = bossById('test-resistant');
    if (resistantBoss !== undefined) {
      builtIn.set('resistances', recordGoodRotation('resistances', 2000, resistantBoss));
      // Und dieselbe Lage mit einer GEMISCHTEN Leiste (AC7). Seit der
      // Widerstand am Zauber haengt, belegt die Aufzeichnung darueber nur noch
      // den Fall, in dem alle Zauber dieselbe Schule haben.
      builtIn.set(
        'resistances-mixed',
        recordMixedRotation('resistances-mixed', 2000, resistantBoss),
      );
    }
    // Der Prueffall zur WIRKZEIT des Spielers (Issue #304, AC5). 2000 Takte
    // sind 20 s: Der dritte Schlag des Bosses faellt bei 15 s, und die Zeit
    // danach gehoert dazu — sie belegt, dass nach einem Abbruch wieder
    // gewirkt werden kann.
    const casterBoss = bossById('test-caster');
    if (casterBoss !== undefined) {
      // 2400 Takte sind 24 s. Die Laenge ist gerechnet: Der zweite Boss-Schlag
      // faellt bei 18 s, und der einzige Abbruch VON HAND, der keinen der
      // beiden Schlaege um seinen Fall bringt, liegt bei 20,5 s.
      builtIn.set('cast-time', recordCastTimeRotation('cast-time', 2400, casterBoss));
    }
    const racer = bossById('test-racer');
    if (racer !== undefined) {
      builtIn.set('race-heal', recordGoodRotation('race-heal', 4000, racer));
    }
    // Der Prueffall zu den letzten beiden Bausteinen (Issue #240, AC6). 90 s
    // in Takten, und die Laenge ist gerechnet, nicht geschaetzt: Die
    // Schwaechung greift bei 8 s, die Zeitleiste wiederholt sich alle 12 s
    // (nach 90 s sind sieben Zyklen durch, und die Wirkzeit steht laengst auf
    // ihrer Untergrenze). BEIDE Bausteine kommen damit in EINEM Lauf vor —
    // getrennt geprueft zeigte sich nicht, dass sie einander nicht ins Gehege
    // kommen.
    //
    // Baustein 6 stand hier bis zum 2026-08-05 mit dabei und ist herausgenommen:
    // Der Boss-Ring IST die Reaktion (GAME-DESIGN 9.1), ein zweiter Wert daneben
    // waere eine Dublette gewesen. Er wird in Issue #247 gebaut, an den Ring.
    const trial = bossById('test-crucible-trial');
    if (trial !== undefined) {
      builtIn.set('trial-blocks', recordTrialRotation('trial-blocks', 9000, trial));
    }
  }
  return builtIn;
}

export function recordingNames(): string[] {
  return [...recordings().keys()];
}
