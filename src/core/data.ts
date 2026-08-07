/**
 * Zauber und Bosse als **Datensätze, nicht als Code** (GAME-DESIGN 7.3, 18.3,
 * Leitplanke 6).
 *
 * Das hier gelesene Format teilen sich das Spiel (TypeScript) und der Simulator
 * (Rust) — es ist damit eine Schnittstelle und keine Bequemlichkeit. Die Fassung
 * mit allen Zahlen steht in `docs/STEP1-SPEC.md` § 5.
 *
 * Fehlerhafte Einträge werden **gemeldet und übersprungen**, nicht verschluckt:
 * Ein Zauber, der still fehlt, fällt erst im Kampf auf.
 */
import rawSchool from '../../data/spells/calcination.json';
import rawCommonSpells from '../../data/spells/common.json';
import rawCoagulation from '../../data/spells/coagulation.json';
import rawFixation from '../../data/spells/fixation.json';
import rawSublimation from '../../data/spells/sublimation.json';
import rawPutrefaction from '../../data/spells/putrefaction.json';
import rawTestCast from '../../data/spells/test-cast.json';
import rawTestBuff from '../../data/spells/test-buff.json';
import rawBoss from '../../data/bosses/first-crucible.json';
import rawPlayer from '../../data/player/base.json';
import rawLevels from '../../data/player/levels.json';
import rawReward from '../../data/player/reward.json';
import rawTalents from '../../data/player/talents.json';
import { TICK_MS } from './loop.js';
import { BUFF_KIND_CAPACITY } from '../combat/schools/buffs.js';

/** Die Schul-Mechanik als Vorrat: Hitze, Kontroll-Punkte, Zersetzungs-Stapel. */
export interface ResourceDef {
  readonly id: string;
  readonly name: string;
  readonly cap: number;
  /** Abstand zwischen zwei Verfall-Schritten in ms. */
  readonly decayPerStepMs: number;
  readonly decayAmount: number;
  /** Wert, ab dem die Anzeige eine Markierung zeigt (GAME-DESIGN 9). */
  readonly highlightAt: number;
}

/** Ein Effekt über Zeit. `refreshMode` ist für V1 immer `reset` (GAME-DESIGN 7.2). */
export interface DotDef {
  readonly tickDamage: number;
  readonly tickIntervalMs: number;
  readonly durationMs: number;
  readonly refreshMode: 'reset';
  readonly resourceGainPerTick: Readonly<Record<string, number>>;
}

/**
 * Eine Verstärkung, die ein Zauber beim Wirken auslöst (GAME-DESIGN 5).
 *
 * **Die Dauer steht in Wartezeiten, nicht in Sekunden.** Das ist keine
 * Bequemlichkeit, sondern die Design-Regel: Sinkt die globale Wartezeit per
 * Talent, wird das Fenster in Sekunden kürzer — „drei Zauber" bleiben drei
 * Zauber. Wer hier Millisekunden schriebe, machte aus einem Fenster von drei
 * Zaubern eines von vier, sobald jemand schneller wirkt.
 */
export interface BuffDef {
  /**
   * Die Art der Verstärkung, `0` bis `BUFF_KIND_CAPACITY - 1`.
   *
   * Sie entscheidet über das Stapeln: **verschiedene Arten multiplizieren
   * sich, gleiche ersetzen einander** (GAME-DESIGN 5). Zwei Zauber mit
   * derselben Art überschreiben sich also gegenseitig, auch wenn der zweite
   * schwächer ist.
   */
  readonly kind: number;
  /** Der Faktor, mit dem der Schaden multipliziert wird — `1.2` heißt +20 %. */
  readonly factor: number;
  /** Länge des Fensters in globalen Wartezeiten (GAME-DESIGN 5: zwei bis drei). */
  readonly gcds: number;
  /**
   * Worauf die Verstärkung wirkt (Issue #489). Fehlt sie, ist es der Schaden.
   *
   * **Die Zielgröße wählt den Speicher, nicht nur die Rechenstelle.** Jede
   * hat eine eigene `Buffs`-Instanz; damit kann eine Verstärkung auf den
   * Punktegewinn eine auf den Schaden nicht verdrängen, auch wenn beide
   * dieselbe Art tragen (GAME-DESIGN 328).
   */
  readonly target?:
    'damage' | 'control' | 'resource' | 'tick' | 'stack' | 'cdreduce' | 'convert' | 'cap';
}

/** Ein Entlade-Zauber: verbraucht den Vorrat und schlägt entsprechend zu. */
export interface SpenderDef {
  readonly resource: string;
  readonly damagePerPoint: number;
  /** Ab diesem Vorrat greift der Bonus — **ab**, nicht darüber. */
  readonly bonusThreshold: number;
  readonly bonusMultiplier: number;
  readonly consumesAll: boolean;
}

export interface SpellDef {
  readonly id: string;
  readonly name: string;
  readonly unlockLevel: number;
  readonly baseDamage: number;
  readonly cooldownMs: number;
  readonly castTimeMs: number;
  readonly triggersGcd: boolean;
  /**
   * Aus welcher Schule der Zauber stammt, oder `null` (Issue #270, AC0).
   *
   * **`null` heißt schulunabhängig, nicht unbekannt.** GAME-DESIGN 7.4 kennt
   * die „drei schulunabhängigen Sonderzauber" — QUENCH ist einer davon. Ihn
   * einer Schule zuzuschlagen behauptete eine Zugehörigkeit, die das Design
   * ausdrücklich verneint; ein leerer Text sähe aus wie ein fehlender Wert.
   *
   * WARUM DAS FELD ÜBERHAUPT NÖTIG WAR: `SchoolDef` trägt eine Schule, und es
   * gibt genau eine — alle fünfzehn Zauber hängen an ihr, weil sie beim
   * Zusammenlegen aus sechs Dateien in EINE Liste wandern. Die Herkunft ging
   * dabei verloren, und ohne sie ist „nach Schule gruppiert" nicht schwer,
   * sondern unmöglich.
   *
   * Die Liste bleibt, wie sie ist: Ihre Reihenfolge bestimmt die Indizes, und
   * die stehen in jeder aufgezeichneten Runde (GAME-DESIGN 18.3). Das Feld
   * kommt daneben, nicht anstelle.
   */
  readonly school: string | null;
  readonly resourceGain?: Readonly<Record<string, number>>;
  readonly dot?: DotDef;
  readonly spender?: SpenderDef;
  /**
   * Um wie viele Tick-Intervalle dieser Zauber **laufende Brände verlängert**
   * (EMBER, Issue #517).
   *
   * **Fehlt das Feld, verlängert er nichts** — das ist der Fall für jeden
   * bisherigen Zauber. Der Weg „länger halten" hatte bis hierher keinen
   * einzigen.
   */
  readonly dotExtendTicks?: number;
  /**
   * Ob dieser Zauber **alle laufenden Brände verbraucht** und ihren Rest als
   * Schaden gibt (PYROCLASM, Issue #517).
   *
   * Der Betrag ist keine eigene Zahl: Es ist Restdauer mal Schaden je Tick,
   * also genau das, was die Brände noch gemacht hätten. Nach der
   * Erneuerungsregel ist Restzeit verloren, wenn zu früh erneuert wird —
   * dieser Zauber kehrt dieselbe Regel um.
   */
  readonly consumesDots?: boolean;
  /**
   * Um wie viele Millisekunden dieser Zauber eine **laufende Schutzschicht**
   * verlängert (HARDEN, Issue #523). Fehlt das Feld, verlängert er keine.
   */
  readonly extendShieldMs?: number;
  /**
   * Um wie viele Millisekunden dieser Zauber **alle laufenden
   * Zersetzungs-Stapel** verlängert (SEPSIS, Issue #523).
   *
   * Alle und um denselben Betrag — das erhält die aufsteigende Ordnung, auf
   * der das Nachrücken beruht.
   */
  readonly extendDecayMs?: number;
  /**
   * Welche Verstärkung dieser Zauber auslöst (GAME-DESIGN 5, Issue #420).
   *
   * **Fehlt das Feld, löst er keine aus** — das ist der Fall für jeden Zauber
   * des Spiels. Bis #420 gab es das Feld gar nicht, und deshalb stand
   * `factors.buffs` in jedem Kampf dauerhaft auf 1, obwohl die Stapel-Regel
   * auf beiden Seiten fertig gebaut und mit einem gemeinsamen Prüffall
   * abgeglichen war: Beide Seiten taten dasselbe nicht.
   */
  readonly buff?: BuffDef;
  /**
   * Um wie viele Millisekunden dieser Zauber **alle laufenden Abklingzeiten**
   * verkürzt (SUBLIMATION, GAME-DESIGN 4.1, Issue E3-3).
   *
   * Steht in den Daten und nicht im Code: Ein Zauber, der das kann, ist ein
   * Datensatz wie jeder andere (Leitplanke 6). Fehlt das Feld, verkürzt er
   * nichts — das ist der Fall für alle Zauber der Kalzination.
   */
  readonly cooldownReductionMs?: number;
  /**
   * **Wie viele** Abklingzeiten der Auswahlmodus zurücksetzt (SUBLIMATION,
   * Issue E3-3c, erweitert in #521).
   * Nur Abklingzeiten UNTER dieser Grenze verkürzen (ASCEND, Issue #518).
   *
   * 0 oder fehlend heißt: alle, wie bisher.
   */
  readonly cooldownReductionBelowMs?: number;
  /**
   * Nur die LÄNGSTE laufende Abklingzeit verkürzen (DECANT, Issue #518).
   *
   * Ohne das Feld trifft die Verkürzung alle — das Verhalten vor #518.
   */
  readonly cooldownReductionLongestOnly?: boolean;
  /**
   * Setzt dieser Zauber den **Auswahlmodus** scharf (SUBLIMATION, Issue E3-3c)?
   *
   * Der Rücksetzer wirkt nicht auf den nächsten gewirkten Zauber, sondern
   * fragt: Welchen? Zwei Schritte, drei Aktionen — die Entscheidung des
   * Auftraggebers vom 2026-07-29.
   *
   * # Warum eine Anzahl und kein Ja/Nein mehr
   *
   * ALEMBIC (#521) macht **zwei** Zauber bereit. Das ist kein größerer
   * Betrag, sondern dieselbe Frage zweimal — der Spieler wählt zweimal, statt
   * dass das Spiel zwei aussucht.
   *
   * Bei `1` verhält sich der Schalter exakt wie vorher; das ist die
   * Gegenprobe, die #521 AC1 verlangt und die `cooldown-reset.test.ts`
   * unverändert fährt.
   */
  readonly cooldownResetCharges?: number;
  /**
   * Ob dieser Zauber einen laufenden Boss-Zauber **unterbricht**
   * (GAME-DESIGN 11.2, Baustein 9, Issue #110).
   *
   * Steht in den Daten und nicht im Code: Ein Zauber, der das kann, ist ein
   * Datensatz wie jeder andere. Er ist zugleich die **einzige Ausnahme** von
   * der globalen Wartezeit (GAME-DESIGN 6.6) — auch das steht als
   * `triggersGcd: false` in denselben Daten.
   */
  readonly interrupt?: boolean;
  /** Schutzschicht, falls der Zauber eine aufbaut (COAGULATION, Weg 1). */
  readonly shield?: ShieldDef;
  /** Heilung, falls der Zauber eine ist (Weg 2). */
  readonly heal?: HealDef;
  /** Schaden-in-Schutz-Umwandlung, falls der Zauber sie anschaltet (Weg 3). */
  readonly conversion?: ConversionDef;
  /** Zersetzungs-Stapel, die der Zauber auf dem Boss anlegt (PUTREFACTION). */
  readonly decay?: DecayDef;
  /**
   * Um wie viel Prozent der eigene Schaden je liegendem Stapel steigt.
   *
   * Trägt FESTER. Das ist **nicht** die Verwundbarkeit des Bosses — die gilt
   * für alle Quellen und steht in der Schadensformel. **Vorläufig, wird
   * gemessen.**
   */
  readonly perDecayStackPercent?: number;
  /**
   * Die Synergie-Beziehung dieses Zaubers (GAME-DESIGN 5, Issue #318).
   *
   * Der Zauber ist der **Auslöser**: Nach seiner Wirkung steht dem Partner ein
   * Fenster offen, in dem er den Bonus bekommt.
   */
  readonly synergy?: SynergyDef;
  /**
   * Wie viele Kontroll-Punkte der Zauber **aufbaut** (FIXATION, Issue #100).
   *
   * Eigenes Feld und nicht unter `resourceGain`: Der Kampf führt genau eine
   * Schul-Ressource (Hitze), und jede Schul-Größe hat eigene Felder, weil sie
   * eigene Regeln hat. Hitze verfällt, die Schutzschicht fängt auf,
   * Kontroll-Punkte werden ausgegeben.
   */
  readonly controlGain?: number;
  /** Was der Zauber mit Kontroll-Punkten **anstellt**, falls er sie ausgibt. */
  readonly control?: ControlDef;
}

/**
 * Was ein Fixierungs-Zauber mit Kontroll-Punkten anstellt (GAME-DESIGN 4.1).
 *
 * > Der Spieler gibt sie **gezielt** aus. Ein **Planungswerkzeug**, keine
 * > passive Wirkung.
 *
 * Das „gezielt" ist die eigentliche Aussage: Punkte werden nie automatisch
 * verbraucht. Reichen sie nicht, geschieht **nichts** — der Zauber ist vertan,
 * aber er nimmt sich nicht, was er nicht hat.
 */
export interface ControlDef {
  readonly cost: number;
  /** `slowCast`, `delayStrike` oder `holdPhase` — die drei aus 4.1. */
  readonly effect: string;
  /** Um wie viel: Millisekunden bei den ersten beiden, Prozentpunkte bei der dritten. */
  readonly amount: number;
}

/**
 * Eine Schutzschicht (COAGULATION, Weg 1 — GAME-DESIGN 4.1).
 *
 * > Nimmt eingehenden Schaden auf und lässt sich stapeln — **belohnt
 * > Vorausschauen, weil sie VOR dem Schlag stehen muss.**
 *
 * Das „vorher" ist die ganze Mechanik. Eine Schutzschicht, die man nach dem
 * Schlag aufbaut, wäre eine Heilung mit anderem Namen — und die gibt es in
 * dieser Schule bereits als eigenen Weg.
 */
export interface ShieldDef {
  readonly amount: number;
  readonly durationMs: number;
  readonly maxStacks: number;
}

/** Eine Heilung (Weg 2): holt verlorenes Leben zurück. */
export interface HealDef {
  readonly amount: number;
}

/**
 * Schaden-in-Schutz-Umwandlung (Weg 3).
 *
 * `sharePercent` ist **ganzzahlig** und kein Bruch: Ganzzahlige Werte sind die
 * erste Determinismus-Regel (GAME-DESIGN 18.4). Ein `0.25` müsste von beiden
 * Seiten unabhängig gerundet werden — und genau daran laufen zwei Umsetzungen
 * auseinander.
 */
export interface ConversionDef {
  readonly sharePercent: number;
  readonly durationMs: number;
}

export interface SchoolDef {
  readonly schema: number;
  readonly school: string;
  readonly resource: ResourceDef;
  readonly spells: readonly SpellDef[];
}

/**
 * Eine Phase: ab welchem Anteil der Lebenspunkte der Boss sein Verhalten
 * wechselt (GAME-DESIGN 11.1).
 *
 * **Eskalierend, nicht tauschend:** Jede Phase legt eine Mechanik obendrauf,
 * statt die vorige zu ersetzen. Die Mechaniken selbst kommen mit E4-2; hier
 * steht nur, WANN gewechselt wird.
 */
export interface PhaseDef {
  /** Anteil der verbleibenden Lebenspunkte in Prozent, ab dem sie gilt. */
  readonly atHpPercent: number;
  readonly name: string;
}

/**
 * Die Energieleiste — die zweite Uhr eines Bosses (GAME-DESIGN 11.2,
 * Baustein 1).
 *
 * > Boss-Energie steigt **unabhängig von den Lebenspunkten** und löst bei 100
 * > eine Mechanik aus.
 *
 * Das „unabhängig" ist die eigentliche Aussage: Wer schneller Schaden macht,
 * verschiebt den Auslöser **nicht**.
 */
export interface EnergyDef {
  readonly perStepMs: number;
  readonly amount: number;
  readonly threshold: number;
  readonly resetTo: number;
}

/**
 * Der Zähler, der bei 100 tötet (GAME-DESIGN 11.2, Baustein 2; Issue #216).
 *
 * > Ein Wert steigt unfreiwillig an; bei 100 stirbt man. Zurücksetzen nur über
 * > eine **begrenzt verfügbare Handlung**.
 *
 * DER UNTERSCHIED ZUR ENERGIELEISTE
 * ---------------------------------
 * Beide steigen von selbst, und beide haben eine Schwelle. Die Energieleiste
 * **springt zurück** und löst aus; dieser Zähler **tötet** und springt gar
 * nicht zurück. Deshalb sind es zwei Felder und nicht ein Schalter an einem:
 * Wer sie zusammenlegte, müsste an jeder Auswertung fragen, welche Sorte
 * gemeint ist.
 *
 * DIE BEGRENZUNG IST DIE MECHANIK
 * -------------------------------
 * Ohne sie wäre der Zähler eine Taste, die man drückt, wenn die Zahl hoch ist.
 * Erst dass die Handlung **ausgeht**, macht ihn zu einer Entscheidung: früh
 * senken und später ohne Mittel dastehen, oder warten und knapp kalkulieren.
 *
 * **Welche Handlung das ist, steht hier nicht** — dieses Issue baut den
 * Mechanismus, nicht seine Bindung an eine Taste (das entscheidet E10 mit den
 * Signatur-Regeln). Die Daten sagen nur, wie viel sie nimmt und wie oft es sie
 * gibt.
 */
export interface DoomDef {
  /** Takt, in dem der Zähler steigt. Auf dem 10-ms-Raster. */
  readonly perStepMs: number;
  /** Was je Takt dazukommt. */
  readonly amount: number;
  /** Bei diesem Wert stirbt der Spieler. */
  readonly threshold: number;
  /** Was die Rücksetz-Handlung abzieht. */
  readonly clearAmount: number;
  /** Wie oft sie im ganzen Kampf zur Verfügung steht. */
  readonly clearCharges: number;
}

/**
 * Das Wettrennen gegen einen Zauber (GAME-DESIGN 11.2, Baustein 4; #218).
 *
 * > Der Boss wird unverwundbar, während etwas anderes vor Ablauf zerstört
 * > werden muss. **Der beste Moment für Overdrive.**
 *
 * ER WIRKT AUF DEN FORTSCHRITT, NICHT AUF DEN SPIELER
 * ---------------------------------------------------
 * Alle bisherigen Bausteine teilen Schaden aus oder töten. Dieser verändert,
 * was der ausgeteilte Schaden **bewirkt** — er setzt ihn zeitweise auf null.
 *
 * **Und das ist nicht `damage × 0`.** Schaden in ein Unverwundbarkeits-Fenster
 * ist *verlorener* Schaden und gehört in die Fehleranalyse (GAME-DESIGN 15.3).
 * Wer ihn einfach verschwinden ließe, nähme dem Spieler die einzige Auskunft,
 * die ihn beim nächsten Versuch besser macht.
 *
 * DAS ZIEL IST EIN SCHWELLENWERT, KEIN OBJEKT
 * -------------------------------------------
 * 11.2 sagt „etwas anderes muss zerstört werden". Ein zweites Ziel mit eigenen
 * Lebenspunkten wäre ein **Zusatzgegner**, und GAME-DESIGN 22 verschiebt
 * Zusatzgegner und Zielwechsel ausdrücklich auf Version 2 — sie zögen eine
 * zweite Rotationsebene ein.
 *
 * Stattdessen lädt der Schaden im Fenster einen **Zähler**. Erreicht er
 * `threshold` vor Ablauf, ist das Ziel zerstört und das Fenster endet früh.
 *
 * **Es ist die Energieleiste mit umgekehrter Füllrichtung** (Baustein 1): Dort
 * steigt sie von selbst und löst bei 100 aus, hier füllt der Spieler sie. Kein
 * zweites Ziel, kein Zielwechsel, keine neue Handlung — er drückt weiter
 * dieselben Tasten auf denselben Boss.
 *
 * Und „der beste Moment für Overdrive" trägt: Viel Schaden in ein kurzes
 * Fenster zu legen ist genau die Aufgabe.
 */
export interface RaceDef {
  /** Zeitpunkt seit Kampfbeginn, zu dem das Fenster beginnt. Auf dem Raster. */
  readonly atMs: number;
  /** Wie lange es höchstens dauert. */
  readonly windowMs: number;
  /**
   * Wie viel Schaden im Fenster das Ziel zerstört.
   *
   * Vorläufig — wird gemessen.
   */
  readonly threshold: number;
}

/**
 * Das umgekehrte Ziel (GAME-DESIGN 11.2, Baustein 6; Issue #247).
 *
 * > **Statt** Schaden zu machen muss eine kippende Reaktion stabilisiert
 * > werden. Der Kampf, in dem COAGULATION glänzt.
 *
 * **AM BOSS-RING, NICHT AN EINEM ZWEITEN WERT.** Das ist die Entscheidung, die
 * #247 trägt, und sie hat einen Anlauf gekostet: In #240 stand hier ein eigener
 * Wert `reaction` — bis im Browser auffiel, dass der Ring selbst so heißt.
 * GAME-DESIGN 9.1: „Großer Kreisring = Lebensbalken und Boss-Darstellung
 * zugleich. Es ist das Reaktionsgefäß." Es gibt ein Gefäß und eine Reaktion
 * darin.
 *
 * Erst dadurch wird die Umkehrung zur Umkehrung — **beide** Bedingungen drehen
 * sich, nicht eine:
 *
 * | | Ring auf 0 | Zeit abgelaufen |
 * | --- | --- | --- |
 * | normaler Boss | Sieg | Niederlage |
 * | mit diesem Feld | Niederlage | **Sieg** |
 *
 * Mit einem zweiten Wert hätte der Spieler eine Leiste zum Senken **und** eine
 * zum Halten gehabt — also keine Umkehrung, sondern eine Zusatzaufgabe.
 *
 * Der Name: „runaway reaction" ist der Fachbegriff für genau das, was 9.1
 * beschreibt — eine Umwandlung, die außer Kontrolle gerät. Das Ereignis
 * `boss.reaction.destabilise` (`src/core/events.ts`) trägt sie seit E1.
 */
export interface RunawayDef {
  /** Takt, in dem der Ring von selbst sinkt. Auf dem Raster. */
  readonly perStepMs: number;
  /** Wie viel Fortschritt je Takt von selbst dazukommt. */
  readonly amount: number;
}

/**
 * Die eskalierende Wiederholung (GAME-DESIGN 11.2, Baustein 7; Issue #240).
 *
 * > Dieselbe Phase kehrt mehrfach wieder — **kürzere Fenster**, mehr
 * > gleichzeitige Anforderungen.
 *
 * Sie setzt auf der vorhandenen Zeitleisten-Wiederholung auf (`timelineLoopMs`,
 * #174) und macht sie **eskalierend**: Mit jedem Zyklus schrumpft die Wirkzeit
 * unterbrechbarer Zauber, das Fenster zum Unterbrechen wird also enger.
 *
 * **Die Verkürzung steht in den Daten, nicht im Code** — sonst wäre sie eine
 * Regel des Spiels statt eine Eigenschaft dieses Bosses.
 *
 * `minCastTimeMs` ist keine Feinheit: Ohne Untergrenze schrumpfte das Fenster
 * irgendwann auf einen Takt, und der Baustein wäre kein Anspruch mehr, sondern
 * eine Unmöglichkeit (11.7 verwirft genau das).
 */
export interface EscalationDef {
  /** Um wie viel die Wirkzeit je Zyklus schrumpft. */
  readonly castTimeShrinkMs: number;
  /** Kürzer wird sie nicht. */
  readonly minCastTimeMs: number;
}

/**
 * Die Schwächung ohne Heil-Lösung (GAME-DESIGN 11.2, Baustein 8; Issue #240).
 *
 * > Ein Effekt lässt sich **nicht wegheilen**, sondern nur durch eine bestimmte
 * > Handlung loswerden. **Braucht mehrere Lösungswege**, sonst ist man ohne den
 * > richtigen Zauber hilflos.
 *
 * Das „mehrere" ist keine Empfehlung: `clearedBy` mit einem einzigen Eintrag
 * wird beim Lesen beanstandet. Ein Boss, der genau einen Zauber verlangt,
 * macht die Zusammenstellung zur Pflichtaufgabe — und 11.3 sagt ausdrücklich,
 * dass kein Boss eine bestimmte Schule verlangen darf.
 *
 * **Heilung steht nicht darin, und das ist der ganze Baustein.** Wer sie
 * einträgt, hat ihn in eine gewöhnliche Schwächung zurückverwandelt.
 */
export interface AfflictionDef {
  /** Zeitpunkt seit Kampfbeginn, zu dem sie greift. Auf dem Raster. */
  readonly atMs: number;
  /** Takt, in dem sie Schaden macht. */
  readonly perStepMs: number;
  /** Schaden je Takt, **vor** WARD. */
  readonly damage: number;
  /** Kennungen der Zauber, die sie entfernen. Mindestens zwei. */
  readonly clearedBy: readonly string[];
}

/**
 * Ein Fehler heilt den Boss (GAME-DESIGN 11.2, Baustein 5; #218).
 *
 * > Ein Fehler macht Fortschritt rückgängig. Härteste Form von Konsequenz.
 * > **Nur für einzelne Bosse**, sonst frustrierend.
 *
 * Der zweite Baustein, der auf den Fortschritt wirkt statt auf den Spieler —
 * und der einzige, der ihn **zurückdreht**.
 *
 * „Nur für einzelne Bosse" ist eine Design-Auflage und keine technische: Hier
 * entsteht der Mechanismus, wer ihn trägt, entscheidet E10.
 *
 * DER AUSLÖSER IST DER DURCHGEKOMMENE BOSS-ZAUBER
 * -----------------------------------------------
 * Kein neuer Fehler-Begriff: Baustein 9 ist gebaut, `interruptsMissed` zählt
 * bereits, und der Spieler kennt die Mechanik.
 *
 * **Aber keine feste Kopplung.** Wie viel ein verpasster Unterbrecher heilt,
 * sagt der Boss-Datensatz; fehlt das Feld, heilt nichts. Eine feste Kopplung
 * gälte für **alle** Bosse — und 11.2 sagt zu diesem Baustein ausdrücklich
 * „nur für einzelne Bosse, sonst frustrierend". Als Datenfeld ist es die
 * Eigenschaft des einen Bosses, der es bekommt; welcher, entscheidet E10.
 */
export interface BossHealDef {
  /**
   * Wie viel Fortschritt ein durchgekommener Boss-Zauber zurückdreht.
   *
   * Vorläufig — wird gemessen.
   */
  readonly amount: number;
}

/**
 * Der exponentielle Stapel (GAME-DESIGN 11.2, Baustein 3; Issue #216).
 *
 * > 1 Stapel = X, 2 = 3X, 3 = 9X. Erzwingt eine Handlung an einem exakt
 * > bestimmten Punkt. **Muss gut angekündigt sein.**
 *
 * WARUM DIE KURVE UND NICHT DIE SCHWELLE
 * --------------------------------------
 * Der Zähler oben tötet bei einem festen Wert — die Schwelle ist hart, der
 * Anstieg gleichmäßig. Hier ist es umgekehrt: Es gibt keine Schwelle, an der
 * etwas umschlägt. Der Schaden wird schlicht **unbezahlbar**, und zwar an
 * einem Punkt, den man ausrechnen kann, bevor er kommt.
 *
 * Genau deshalb ist `factor` ganzzahlig und nicht als Kurve frei einstellbar:
 * 1 → 3 → 9 ist die Aussage des Bausteins, und drei Zahlen aus dem Design
 * lassen sich nachrechnen. Eine frei geformte Kurve wäre eine andere Mechanik.
 */
export interface StackDef {
  /** Takt, in dem ein Stapel dazukommt. Auf dem 10-ms-Raster. */
  readonly perStepMs: number;
  /** Schaden beim **ersten** Stapel — das X aus 11.2. */
  readonly baseDamage: number;
  /** Wachstumsfaktor je weiterem Stapel. 3 ergibt 1 → 3 → 9. */
  readonly factor: number;
  /**
   * Obergrenze der Stapel.
   *
   * Nicht Design, sondern Arithmetik: `baseDamage * factor^(n-1)` verlässt bei
   * genügend Stapeln den Wertebereich, und Spiel und Simulator täten das
   * verschieden (JavaScript rechnet mit Fließkomma weiter, Rust sättigt bei
   * u32). Zwei Seiten, die verschieden überlaufen, sind keine Parität mehr.
   */
  readonly maxStacks: number;
}

/**
 * Ein Eintrag der Boss-Zeitleiste (GAME-DESIGN 11.1: „Schadens-Ereignisse als
 * **feste Zeitleiste**").
 *
 * Fest heißt wirklich fest: Der Boss reagiert nicht auf den Spieler, und es
 * gibt keine zufällige Streuung. Nur so sind Bestzeiten vergleichbar.
 *
 * **Was hier absichtlich fehlt, ist die Wirkung.** Welcher Baustein was tut,
 * ist Gegenstand von E4-2 bis E4-6; welcher Boss welche Einträge bekommt,
 * gehört nach E10 (GAME-DESIGN 22). Hier steht das Gerüst, das beides trägt.
 */
export interface TimelineEntryDef {
  /**
   * Zeitpunkt seit Kampfbeginn, auf dem Takt-Raster des Spiels.
   *
   * Bei einem Zauber ist das der Zeitpunkt, zu dem er **beginnt** — nicht der,
   * zu dem er trifft. Der Treffer liegt `castTimeMs` später, und dazwischen
   * liegt das Fenster, in dem unterbrochen werden kann.
   */
  readonly atMs: number;
  /**
   * Welcher Mechanik-Baustein hier greift.
   *
   * `strike` trifft sofort, `interruptibleCast` läuft eine Wirkzeit lang und
   * lässt sich abbrechen. Ein unbekannter Wert wird beim Lesen gemeldet und
   * übersprungen — `sim/build.rs` bricht dafür den Bau ab.
   */
  readonly mechanic: string;
  /** Anzeigename der Fähigkeit. Englisch, wie alle Spieltexte. */
  readonly name: string;
  /** Schaden am Spieler, **vor** WARD. */
  readonly damage: number;
  /** Wirkzeit eines Boss-Zaubers. 0 bei allem, was sofort trifft. */
  readonly castTimeMs: number;
  /**
   * Ob dieser Angriff eine laufende Wirkzeit des SPIELERS abbricht
   * (GAME-DESIGN 7.1, Issue #304).
   *
   * > Abbruch eines Zauberbalkens: **nur durch bestimmte, vorher erkennbare
   * > Boss-Angriffe.** Normale Treffer stören nicht.
   *
   * Deshalb ein Feld am Eintrag und keine Regel im Kampfcode: „bestimmte"
   * heißt, dass der Boss-Datensatz es sagt. Ohne Angabe `false` — ein
   * bestehender Boss ändert sein Verhalten durch dieses Issue nicht.
   */
  readonly interruptsPlayer: boolean;
}

/**
 * Die Grundwerte des Spielers (GAME-DESIGN 6.1).
 *
 * **Berechnet, nicht gespeichert.** Der Spielstand hält sie nicht — sie ergeben
 * sich aus Ausrüstung und Talenten, und beides gibt es noch nicht (E5, E6).
 *
 * > **Die Zahlen sind noch nicht Design.** GAME-DESIGN Teil F.6 führt sie als
 * > „vor E4 auszuplanen"; entschieden ist davon nichts.
 */
export interface PlayerBaseDef {
  /** VESSEL — Lebenspunkte. */
  readonly vessel: number;
  /** WARD — Anteil, um den eingehender Schaden gemindert wird. */
  readonly wardReduction: number;
  /**
   * Der **Balance-Deckel** auf die Minderung, in Promille (Issue #466).
   *
   * Gilt für die Minderung **insgesamt** — Basis plus Ausrüstung —, wie es
   * GAME-DESIGN unter „Die Deckel" festhält: „WARD (Minderung) | 75 %".
   *
   * **Nicht zu verwechseln mit dem Deckel bei 1,0** im Kampf. Der ist eine
   * Rechenregel: Darüber würde der Spieler von jedem Treffer geheilt. Dieser
   * hier ist eine gesetzte Zahl, die auch bei 0,4 oder 0,9 stehen könnte,
   * ohne dass die Formel etwas dagegen hätte.
   */
  readonly wardCapPermille: number;
}

/**
 * Das Wachstum der Grundwerte je Stufe (GAME-DESIGN 8.1; Issue #248).
 *
 * **In Promille und ganzzahlig, und das ist kein Format-Geschmack.** 1,05^n ist
 * eine Fließkomma-Potenz; `Math.pow` in JavaScript und `f64::powf` in Rust sind
 * nicht garantiert bitgleich. GAME-DESIGN 18.4 verlangt, dass beide Seiten
 * dieselbe Zahl rechnen — eine Tabelle rechnet gar nicht, sie steht da.
 *
 * Angewandt wird sie als `grundwert * promille / 1000` mit **Ganzzahl-Division**.
 * Damit kommt in der ganzen Kette keine Fließkommazahl vor.
 */
export interface LevelGrowthDef {
  /** Der Prozentsatz, aus dem die Tabelle entstand — zur Nachvollziehbarkeit. */
  readonly growthPerLevelPercent: number;
  /** Die höchste erreichbare Stufe. */
  readonly maxLevel: number;
  /** Faktor je Stufe in Promille. Index 0 ist Stufe 1 und trägt 1000. */
  readonly factorPerMille: readonly number[];
  /**
   * Erfahrung je Sieg, je Stufe des BOSSES.
   *
   * **Am Boss, nicht am Spieler**, und das ist keine Geschmacksfrage:
   * GAME-DESIGN 12.6 sieht einen abnehmenden Ertrag über den STUFENABSTAND vor
   * („voller Ertrag, solange der Boss zur eigenen Stufe passt"). Einen Abstand
   * gibt es nur, wenn beide Seiten eine Stufe haben — hinge die Erfahrung am
   * Spieler, wäre die schon beschlossene Regel gar nicht baubar.
   */
  readonly xpPerBossLevel: number;
  /**
   * Bedarf für den Aufstieg, Eintrag n von Stufe n nach n+1.
   *
   * 99 Einträge für 99 Aufstiege. Die erzeugende Formel und die vier
   * geprüften Varianten stehen im `_comment` der Datei — eine Zahl ohne ihre
   * Herkunft ist eine Behauptung, und die nächste Balance-Änderung würde
   * geraten statt gerechnet.
   */
  readonly xpToNextLevel: readonly number[];
  /**
   * Die Zielgröße: wie viele Talentpunkte über alle Stufen zusammenkommen.
   *
   * **Getrennt von der Verteilung, und das ist ihr Zweck.** Ein Wächter, der
   * `talentPointLevels` summiert und mit der Summe derselben Liste vergleicht,
   * prüft seine eigene Erwartung gegen sich selbst — genau davor warnt der
   * Kommentar an `barSlotsAtLevel`. Diese Zahl ist die zweite, unabhängige
   * Angabe: Wer die Liste ändert, ohne sie anzufassen, wird bemerkt (#379).
   */
  readonly talentPointTarget: number;
  /**
   * Stufen, auf denen je ein Talentpunkt anfällt (Issue #379).
   *
   * Wie `barSlotLevels` eine Liste von Stufen und keine Zahl je Stufe: Der
   * Aufstieg gibt entweder einen Punkt oder keinen, nie zwei.
   */
  readonly talentPointLevels: readonly number[];
  /** Wie viele Leisten-Plätze der Spieler auf Stufe 1 hat. */
  readonly startBarSlots: number;
  /**
   * Stufen, auf denen je ein weiterer Leisten-Platz aufgeht.
   *
   * **Nur die ANZAHL, nie welcher Platz.** GAME-DESIGN 9.1 verlangt, dass
   * Positionen von Anfang an fest vergeben sind, damit das Wachsen bestehende
   * Plätze nicht verschiebt — sonst leidet das Muskelgedächtnis, und das ist
   * das Einzige, was eine Rotation überhaupt spielbar macht.
   */
  readonly barSlotLevels: readonly number[];
}

export interface BossDef {
  readonly schema: number;
  /**
   * Stufe des Bosses — wovon die Erfahrung für einen Sieg abhängt (#248).
   *
   * **Am Boss, nicht am Spieler.** GAME-DESIGN 12.6 sieht einen abnehmenden
   * Ertrag über den STUFENABSTAND vor; einen Abstand gibt es nur, wenn beide
   * Seiten eine Stufe haben.
   *
   * Fehlt das Feld, ist es 1. Welcher Boss welche Stufe trägt, ist Inhalt und
   * entscheidet E10 — 12.4 sagt nur, dass acht Instanzen à vier Bosse die
   * Stufen 1 bis 100 tragen.
   */
  readonly level: number;
  readonly id: string;
  readonly name: string;
  readonly instance: string;
  readonly hp: number;
  readonly overpressureMs: number;
  readonly resistances: Readonly<Record<string, number>>;
  /** Phasen-Schwellen, absteigend. Leer heißt: keine Phasen. */
  readonly phases: readonly PhaseDef[];
  /** Die zweite Uhr, oder `null`. */
  readonly energy: EnergyDef | null;
  /** Der Zähler, der bei der Schwelle tötet, oder `null` (Issue #216). */
  readonly doom: DoomDef | null;
  /** Der exponentielle Stapel, oder `null` (Issue #216). */
  readonly stacks: StackDef | null;
  /** Das Unverwundbarkeits-Fenster, oder `null` (Issue #218). */
  readonly race: RaceDef | null;
  /** Was ein Fehler an Fortschritt zurückdreht, oder `null` (Issue #218). */
  readonly bossHeal: BossHealDef | null;
  /** Die Eskalation je Zyklus, oder `null` (Issue #240). */
  /**
   * Das umgekehrte Ziel, falls dieser Boss es trägt (Issue #247).
   *
   * `null` heißt: gewöhnlicher Kampf — Ring auf 0 ist der Sieg. Das gilt für
   * jeden ausgelieferten Boss.
   */
  readonly runaway: RunawayDef | null;
  readonly escalation: EscalationDef | null;
  /** Die Schwächung ohne Heil-Lösung, oder `null` (Issue #240). */
  readonly affliction: AfflictionDef | null;
  /** Die feste Abfolge der Boss-Handlungen, aufsteigend nach Zeitpunkt. */
  readonly timeline: readonly TimelineEntryDef[];
  /**
   * Länge eines Zyklus, wenn sich die Zeitleiste **wiederholt** (#174).
   *
   * `null` heißt: Sie läuft einmal und endet — so verhält sich jeder Boss.
   * Nur die Trainingspuppe wiederholt sich, weil GAME-DESIGN 15.5 ihr „kein
   * Zeitlimit" zusagt; eine Zeitleiste, die nach 48 Sekunden aufhört, wäre
   * diese Zusage mit Kleingedrucktem.
   *
   * Der Wert ist der Zeitversatz je Durchlauf, nicht die Zahl der Einträge:
   * Eintrag `i` im Zyklus `z` ist fällig bei `z * loop + atMs`.
   *
   * **Muss auf beiden Seiten gleich lauten** (`Boss::timeline_loop_ms` im
   * Simulator). Eine Regel, die nur eine Seite kennt, bricht die Parität erst
   * nach dem ersten Zyklus — also nicht im ersten Prüflauf.
   */
  readonly timelineLoopMs: number | null;
}

export interface ParseResult<T> {
  readonly value: T;
  /** Leer heißt: alles in Ordnung. */
  readonly problems: readonly string[];
}

const SUPPORTED_SCHEMA = 1;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function number(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function gains(value: unknown): Record<string, number> {
  const out: Record<string, number> = {};
  if (!isRecord(value)) return out;
  for (const [key, amount] of Object.entries(value)) {
    if (typeof amount === 'number' && Number.isFinite(amount)) out[key] = amount;
  }
  return out;
}

function parseResource(raw: unknown, problems: string[]): ResourceDef {
  const source = isRecord(raw) ? raw : {};
  if (!isRecord(raw)) problems.push('Die Schul-Mechanik fehlt.');
  const cap = number(source['cap'], 0);
  const highlightAt = number(source['highlightAt'], 0);
  if (cap <= 0) problems.push('Die Obergrenze der Schul-Mechanik ist nicht positiv.');
  if (highlightAt > cap) {
    problems.push(`Die Markierung (${highlightAt}) liegt über der Obergrenze (${cap}).`);
  }
  return {
    id: typeof source['id'] === 'string' ? source['id'] : 'resource',
    name: typeof source['name'] === 'string' ? source['name'] : 'Resource',
    cap,
    decayPerStepMs: number(source['decayPerStepMs'], 0),
    decayAmount: number(source['decayAmount'], 0),
    highlightAt,
  };
}

/**
 * Die Verstärkung eines Zaubers aus den Daten (Issue #420).
 *
 * # Warum hier geprüft wird und nicht erst im Kampf
 *
 * `Buffs.apply` gibt `false` zurück, wenn die Art außerhalb des Vorrats liegt —
 * und `false` ist im Kampf nicht zu sehen: Der Zauber wirkt, der Schaden fällt,
 * nur die Verstärkung bleibt aus. Ein Tippfehler in `kind` wäre damit ein
 * Zauber, der leise nichts tut.
 *
 * Deshalb bricht das Laden hier ab, statt still weiterzurechnen. Dasselbe
 * Vorgehen wie bei `consumesAll` auf der Rust-Seite (`sim/build.rs:481`):
 * lieber beim Bauen scheitern als im Kampf das Falsche rechnen.
 */
function parseBuff(raw: unknown, where: string, problems: string[]): BuffDef | undefined {
  if (!isRecord(raw)) return undefined;
  const kind = number(raw['kind'], -1);
  if (!Number.isInteger(kind) || kind < 0 || kind >= BUFF_KIND_CAPACITY) {
    problems.push(
      `${where}: Verstärkungs-Art ${kind} liegt außerhalb von 0..${BUFF_KIND_CAPACITY - 1}.`,
    );
    return undefined;
  }
  const factor = number(raw['factor'], 0);
  if (!(factor > 0)) {
    // Ein Faktor von 0 löschte den Schaden, ein negativer kehrte ihn um.
    // Beides ist keine Verstärkung, sondern ein Datenfehler.
    problems.push(`${where}: Verstärkungs-Faktor ${factor} muss größer als 0 sein.`);
    return undefined;
  }
  const gcds = number(raw['gcds'], 0);
  if (!Number.isInteger(gcds) || gcds <= 0) {
    problems.push(`${where}: Fensterlänge ${gcds} muss eine positive Zahl von Wartezeiten sein.`);
    return undefined;
  }
  const ziel = raw['target'];
  const ZIELE = [
    'damage',
    'control',
    'resource',
    'tick',
    'stack',
    'cdreduce',
    'convert',
    'cap',
  ] as const;
  if (ziel !== undefined && !ZIELE.includes(ziel as (typeof ZIELE)[number])) {
    // **Abbrechen statt durchreichen.** Eine unbekannte Zielgröße landete
    // sonst im Schadens-Speicher, und der Zauber wirkte auf etwas anderes
    // als er sagt — leise, weil er ja trotzdem verstärkt.
    problems.push(
      `${where}: unbekannte Verstärkungs-Zielgröße "${String(ziel)}". ` +
        `Bekannt sind ${ZIELE.join(', ')}.`,
    );
    return undefined;
  }
  return {
    kind,
    factor,
    gcds,
    ...(typeof ziel === 'string' ? { target: ziel as NonNullable<BuffDef['target']> } : {}),
  };
}

function parseDot(raw: unknown, where: string, problems: string[]): DotDef | undefined {
  if (!isRecord(raw)) return undefined;
  const mode = raw['refreshMode'];
  if (mode !== 'reset') {
    // GAME-DESIGN 7.2 und STEP1-SPEC § 5: "reset" ist für V1 der einzige zulässige
    // Wert. Ein anderer wäre eine stille Design-Änderung.
    problems.push(`${where}: refreshMode "${String(mode)}" ist in V1 nicht zulässig.`);
    return undefined;
  }
  const durationMs = number(raw['durationMs'], 0);
  const tickIntervalMs = number(raw['tickIntervalMs'], 0);
  if (tickIntervalMs <= 0) problems.push(`${where}: Tick-Abstand muss positiv sein.`);
  else if (durationMs % tickIntervalMs !== 0) {
    problems.push(`${where}: Dauer ${durationMs} ms ist kein Vielfaches von ${tickIntervalMs} ms.`);
  }
  return {
    tickDamage: number(raw['tickDamage'], 0),
    tickIntervalMs,
    durationMs,
    refreshMode: 'reset',
    resourceGainPerTick: gains(raw['resourceGainPerTick']),
  };
}

function parseSpender(raw: unknown, where: string, problems: string[]): SpenderDef | undefined {
  if (!isRecord(raw)) return undefined;
  const bonusThreshold = number(raw['bonusThreshold'], 0);
  const bonusMultiplier = number(raw['bonusMultiplier'], 1);
  if (bonusMultiplier < 1) problems.push(`${where}: Bonus-Faktor unter 1 ergibt keinen Sinn.`);
  return {
    resource: typeof raw['resource'] === 'string' ? raw['resource'] : 'resource',
    damagePerPoint: number(raw['damagePerPoint'], 0),
    bonusThreshold,
    bonusMultiplier,
    consumesAll: raw['consumesAll'] !== false,
  };
}

/**
 * Die drei Wege der Koagulation (GAME-DESIGN 4.1, Issue #79).
 *
 * Alle drei sind **optional**: Ein Zauber, der keiner von ihnen ist, nennt die
 * Felder gar nicht. Fehlerhafte Angaben werden gemeldet und der Weg
 * weggelassen — ein Schild ohne Betrag schützt nichts, und ein stillschweigend
 * auf 0 gesetzter Wert sähe im Kampf wie ein Balance-Problem aus.
 */
/**
 * Was ein Zauber an Zersetzungs-Stapeln anlegt (GAME-DESIGN 4.1).
 *
 * **Vorläufig, wird gemessen** — der Balance-Läufer bestimmt die Zahlen.
 */
export interface DecayDef {
  /** Wie viele Stapel eine Wirkung anlegt. */
  readonly stacks: number;
}

/**
 * Die Synergie-Beziehung lesen (Issue #318).
 *
 * **Schulübergreifend, deshalb keine Schul-Funktion.** GAME-DESIGN 5 nennt sie
 * ausdrücklich „schulübergreifende Synergien" — sie an `parseCalcination` zu
 * hängen hieße, sie auf eine Schule festzulegen.
 */
function parseSynergy(
  raw: Record<string, unknown>,
  where: string,
  problems: string[],
): { synergy?: SynergyDef } {
  if (!isRecord(raw['synergy'])) return {};
  const partner = raw['synergy']['partner'];
  const windowGcds = number(raw['synergy']['windowGcds'], 0);
  if (typeof partner !== 'string' || partner.length === 0) {
    problems.push(`${where}: Eine Synergie ohne Partner öffnet kein Fenster.`);
    return {};
  }
  // **Null Wartezeiten sind kein Fenster.** Ein Fenster, das nie offen steht,
  // wäre eine Beziehung, die im Kampf nichts tut — und die Auswertung führte
  // sie als vorhanden.
  if (windowGcds <= 0) {
    problems.push(`${where}: windowGcds ${windowGcds} ergibt kein Fenster.`);
    return {};
  }
  return { synergy: { partner, windowGcds } };
}

/**
 * Eine Synergie-Beziehung zwischen zwei Zaubern (GAME-DESIGN 5, Issue #318).
 *
 * # Das Fenster steht in WARTEZEITEN, nicht in Millisekunden
 *
 * GAME-DESIGN 327 sagt es und begründet es: „In Wartezeiten gerechnet, nicht
 * in Sekunden — sonst verschiebt sich das Verhältnis bei hohem Tempo." Wer das
 * Fenster in Millisekunden ablegt, hat bei hohem CATALYSIS plötzlich vier
 * Wartezeiten Zeit statt zwei, und die Fertigkeit, um die es geht
 * („Cooldown-Alignment"), wird mit Tempo leichter statt gleich schwer.
 *
 * Deshalb steht hier eine **Anzahl**. Die Millisekunden entstehen erst im
 * Kampf, durch dieselbe `hasted()`-Stelle wie jede andere Zeit — und schrumpfen
 * damit von selbst mit.
 */
export interface SynergyDef {
  /** Kennung des Zaubers, der den Bonus bekommt. */
  readonly partner: string;
  /** Länge des Fensters in globalen Wartezeiten (GAME-DESIGN 327: zwei bis drei). */
  readonly windowGcds: number;
}

/** Die Felder der PUTREFACTION (GAME-DESIGN 4.1, Issue #97). */
function parsePutrefaction(
  raw: Record<string, unknown>,
  where: string,
  problems: string[],
): Partial<SpellDef> {
  const out: { decay?: DecayDef; perDecayStackPercent?: number } = {};
  if (isRecord(raw['decay'])) {
    const stacks = number(raw['decay']['stacks'], 0);
    if (stacks <= 0) problems.push(`${where}: Ein Zersetzungs-Zauber ohne Stapel legt nichts an.`);
    else out.decay = { stacks };
  }
  if (raw['perDecayStackPercent'] !== undefined) {
    const percent = number(raw['perDecayStackPercent'], 0);
    if (percent <= 0) problems.push(`${where}: perDecayStackPercent ${percent} bewirkt nichts.`);
    else out.perDecayStackPercent = percent;
  }
  return out;
}

function parseCoagulation(
  raw: Record<string, unknown>,
  where: string,
  problems: string[],
): Partial<SpellDef> {
  const out: {
    shield?: ShieldDef;
    heal?: HealDef;
    conversion?: ConversionDef;
  } = {};

  if (isRecord(raw['shield'])) {
    const amount = number(raw['shield']['amount'], 0);
    const durationMs = number(raw['shield']['durationMs'], 0);
    const maxStacks = number(raw['shield']['maxStacks'], 0);
    if (amount <= 0) problems.push(`${where}: Eine Schutzschicht ohne Betrag schützt nichts.`);
    else if (maxStacks < 1) problems.push(`${where}: maxStacks muss mindestens 1 sein.`);
    else out.shield = { amount, durationMs, maxStacks };
  }

  if (isRecord(raw['heal'])) {
    const amount = number(raw['heal']['amount'], 0);
    if (amount <= 0) problems.push(`${where}: Eine Heilung ohne Betrag heilt nichts.`);
    else out.heal = { amount };
  }

  if (isRecord(raw['conversion'])) {
    const sharePercent = number(raw['conversion']['sharePercent'], 0);
    const durationMs = number(raw['conversion']['durationMs'], 0);
    // Über 100 % wäre kein Schutz mehr, sondern eine Vervielfachung: Der
    // Spieler bekäme mehr Schutz, als er Schaden macht.
    if (sharePercent <= 0 || sharePercent > 100) {
      problems.push(`${where}: sharePercent ${sharePercent} liegt nicht in (0, 100].`);
    } else out.conversion = { sharePercent, durationMs };
  }

  return out;
}

/**
 * Kontroll-Punkte und die drei Wirkungen (FIXATION, Issue #100).
 *
 * Die drei Wirkungen stehen wörtlich im Design; ein unbekannter Wert wird
 * gemeldet und der Zauber verliert seine Wirkung — `sim/build.rs` bricht dafür
 * den Bau ab. Ein Zauber, der versehentlich einen Schlag verzögert statt eine
 * Phase, sähe im Kampf wie ein Balance-Problem aus.
 */
function parseFixation(
  raw: Record<string, unknown>,
  where: string,
  problems: string[],
): Partial<SpellDef> {
  const out: { controlGain?: number; control?: ControlDef } = {};

  const gain = number(raw['controlGain'], 0);
  if (gain > 0) out.controlGain = gain;

  if (isRecord(raw['control'])) {
    const knoten = raw['control'];
    const cost = number(knoten['cost'], 0);
    const effect = typeof knoten['effect'] === 'string' ? knoten['effect'] : '';
    // **`amountPerPoint` ist der vierte Weg** (Issue #478, WEDGE): Die drei
    // bisherigen Wirkungen kaufen einen MOMENT und messen ihn in Millisekunden
    // oder Prozent. WEDGE kauft keinen Moment, sondern rechnet Punkte in
    // Schaden um — deshalb ein eigener Name statt einer dritten Bedeutung für
    // dasselbe Feld.
    const amount =
      number(knoten['amountMs'], 0) ||
      number(knoten['amountPercent'], 0) ||
      number(knoten['amountPerPoint'], 0);

    if (cost <= 0) {
      // Eine Wirkung ohne Kosten wäre keine Ausgabe von Kontroll-Punkten,
      // sondern eine passive Wirkung — und genau das ist FIXATION nicht.
      problems.push(`${where}: Eine Kontroll-Wirkung ohne Kosten ist keine Ausgabe.`);
    } else if (
      !['slowCast', 'delayStrike', 'holdPhase', 'convertToDamage', 'allThree'].includes(effect)
    ) {
      problems.push(`${where}: unbekannte Kontroll-Wirkung "${effect}".`);
    } else if (amount <= 0) {
      problems.push(`${where}: Die Kontroll-Wirkung "${effect}" hat keinen Betrag.`);
    } else {
      out.control = { cost, effect, amount };
    }
  }

  return out;
}

function parseSpell(raw: unknown, problems: string[]): SpellDef | null {
  if (!isRecord(raw)) {
    problems.push('Ein Zauber-Eintrag ist kein Objekt.');
    return null;
  }
  const id = raw['id'];
  if (typeof id !== 'string' || id.length === 0) {
    problems.push('Ein Zauber hat keine Kennung.');
    return null;
  }
  const where = `Zauber "${id}"`;
  const cooldownMs = number(raw['cooldownMs'], 0);
  if (cooldownMs < 0) problems.push(`${where}: Abklingzeit ist negativ.`);
  if (!Number.isInteger(cooldownMs)) {
    problems.push(`${where}: Abklingzeit ${cooldownMs} ist nicht ganzzahlig.`);
  }

  const dot = parseDot(raw['dot'], where, problems);
  const spender = parseSpender(raw['spender'], where, problems);
  const buff = parseBuff(raw['buff'], where, problems);
  const resourceGain = isRecord(raw['resourceGain']) ? gains(raw['resourceGain']) : undefined;

  const unterbricht = raw['interrupt'] === true;
  if (unterbricht && raw['triggersGcd'] !== false) {
    // Die Ausnahme aus GAME-DESIGN 6.6 mit falschem Vorzeichen: Ein
    // Unterbrecher mit Wartezeit ließe sich nicht mehr zwischen zwei Zauber
    // legen, und der ganze Baustein wäre tot.
    problems.push(`${where}: Ein Unterbrecher darf keine globale Wartezeit auslösen.`);
  }
  // Die zwei Angaben, die aus "verkürze alles" eine Auswahl machen
  // (Issue #518). Beide sind optional; fehlen sie, bleibt es beim Alten.
  const belowMs = number(raw['cooldownReductionBelowMs'], 0);
  const longestOnly = raw['cooldownReductionLongestOnly'] === true;
  // EMBER und PYROCLASM (Issue #517). Beide Felder fehlen bei jedem
  // bisherigen Zauber — das ist der Normalfall und kein Mangel.
  const extend = raw['dotExtendTicks'];
  const dotExtendTicks = typeof extend === 'number' ? extend : undefined;
  if (dotExtendTicks !== undefined) {
    if (dotExtendTicks <= 0)
      problems.push(`${where}: dotExtendTicks ${dotExtendTicks} ist nicht positiv.`);
    if (!Number.isInteger(dotExtendTicks)) {
      problems.push(`${where}: dotExtendTicks ${dotExtendTicks} ist nicht ganzzahlig.`);
    }
  }
  const consumesDots = raw['consumesDots'] === true;
  // HARDEN und SEPSIS (#523).
  const shieldMs = raw['extendShieldMs'];
  const extendShieldMs = typeof shieldMs === 'number' ? shieldMs : undefined;
  const decayMs = raw['extendDecayMs'];
  const extendDecayMs = typeof decayMs === 'number' ? decayMs : undefined;
  for (const [name, wert] of [
    ['extendShieldMs', extendShieldMs],
    ['extendDecayMs', extendDecayMs],
  ] as const) {
    if (wert === undefined) continue;
    if (wert <= 0) problems.push(`${where}: ${name} ${wert} ist nicht positiv.`);
    if (!Number.isInteger(wert)) problems.push(`${where}: ${name} ${wert} ist nicht ganzzahlig.`);
  }

  const reduction = raw['cooldownReductionMs'];
  const cooldownReductionMs = typeof reduction === 'number' ? reduction : undefined;
  if (cooldownReductionMs !== undefined) {
    if (cooldownReductionMs < 0) problems.push(`${where}: Verkürzung ist negativ.`);
    if (!Number.isInteger(cooldownReductionMs)) {
      problems.push(`${where}: Verkürzung ${cooldownReductionMs} ist nicht ganzzahlig.`);
    }
  }

  // **Der Rücksetzer trägt eine ANZAHL** (Issue #521, AC1). Die alte Form war
  // `armsCooldownReset: true`; sie liest sich hier weiter als 1, damit ein
  // Datensatz aus der Zeit davor nicht still seine Wirkung verliert — ein
  // fehlender Schalter und ein Schalter mit 0 sind sonst nicht zu
  // unterscheiden.
  const ladungenRoh = raw['cooldownResetCharges'];
  const cooldownResetCharges =
    typeof ladungenRoh === 'number'
      ? ladungenRoh
      : raw['armsCooldownReset'] === true
        ? 1
        : undefined;
  if (cooldownResetCharges !== undefined) {
    if (cooldownResetCharges < 1) {
      problems.push(
        `${where}: Rücksetzer-Ladungen ${cooldownResetCharges} — unter 1 ist ` +
          `kein Rücksetzer, dann gehört das Feld weg.`,
      );
    }
    if (!Number.isInteger(cooldownResetCharges)) {
      problems.push(`${where}: Rücksetzer-Ladungen ${cooldownResetCharges} sind nicht ganzzahlig.`);
    }
  }

  return {
    ...(belowMs > 0 ? { cooldownReductionBelowMs: belowMs } : {}),
    ...(longestOnly ? { cooldownReductionLongestOnly: true } : {}),
    id,
    name: typeof raw['name'] === 'string' ? raw['name'] : id,
    unlockLevel: number(raw['unlockLevel'], 1),
    baseDamage: number(raw['baseDamage'], 0),
    cooldownMs,
    castTimeMs: number(raw['castTimeMs'], 0),
    triggersGcd: raw['triggersGcd'] !== false,
    ...(dotExtendTicks !== undefined ? { dotExtendTicks } : {}),
    ...(consumesDots ? { consumesDots } : {}),
    ...(extendShieldMs !== undefined ? { extendShieldMs } : {}),
    ...(extendDecayMs !== undefined ? { extendDecayMs } : {}),
    // Kommt beim Zusammenlegen aus der Herkunftsdatei (siehe `CALCINATION`).
    // Fehlt es, ist der Zauber schulunabhängig — nicht fehlerhaft.
    school: typeof raw['school'] === 'string' ? raw['school'] : null,
    interrupt: unterbricht,
    ...parseCoagulation(raw, where, problems),
    ...parseFixation(raw, where, problems),
    ...parsePutrefaction(raw, where, problems),
    ...parseSynergy(raw, where, problems),
    ...(resourceGain !== undefined ? { resourceGain } : {}),
    ...(dot !== undefined ? { dot } : {}),
    ...(spender !== undefined ? { spender } : {}),
    ...(buff !== undefined ? { buff } : {}),
    ...(cooldownReductionMs !== undefined ? { cooldownReductionMs } : {}),
    ...(cooldownResetCharges !== undefined ? { cooldownResetCharges } : {}),
  };
}

export function parseSchool(raw: unknown): ParseResult<SchoolDef> {
  const problems: string[] = [];
  const source = isRecord(raw) ? raw : {};
  const schema = number(source['schema'], 0);
  if (schema !== SUPPORTED_SCHEMA) {
    problems.push(`Unbekannte Fassung des Formats: ${schema} (erwartet ${SUPPORTED_SCHEMA}).`);
  }
  const spells: SpellDef[] = [];
  const seen = new Set<string>();
  const rawSpells = Array.isArray(source['spells']) ? source['spells'] : [];
  for (const entry of rawSpells) {
    const spell = parseSpell(entry, problems);
    if (spell === null) continue;
    if (seen.has(spell.id)) {
      problems.push(`Zauber "${spell.id}": Kennung kommt mehrfach vor.`);
      continue;
    }
    seen.add(spell.id);
    spells.push(spell);
  }
  if (spells.length === 0) problems.push('Die Schule enthält keinen einzigen Zauber.');

  return {
    value: {
      schema,
      school: typeof source['school'] === 'string' ? source['school'] : 'UNKNOWN',
      resource: parseResource(source['resource'], problems),
      spells,
    },
    problems,
  };
}

export function parseBoss(raw: unknown): ParseResult<BossDef> {
  const problems: string[] = [];
  const source = isRecord(raw) ? raw : {};
  const schema = number(source['schema'], 0);
  if (schema !== SUPPORTED_SCHEMA) {
    problems.push(`Unbekannte Fassung des Formats: ${schema} (erwartet ${SUPPORTED_SCHEMA}).`);
  }
  const hp = number(source['hp'], 0);
  if (hp <= 0) problems.push('Der Boss hat keine Lebenspunkte.');
  const overpressureMs = number(source['overpressureMs'], 0);
  if (overpressureMs <= 0) problems.push('Der Überdruck-Timer ist nicht gesetzt.');

  // Fehlt das Feld, wiederholt sich nichts — der Normalfall für jeden
  // ausgelieferten Boss.
  const rohLoop = source['timelineLoopMs'];
  const timelineLoopMs =
    typeof rohLoop === 'number' && Number.isFinite(rohLoop) && rohLoop > 0 ? rohLoop : null;

  const resistances: Record<string, number> = {};
  if (isRecord(source['resistances'])) {
    for (const [school, factor] of Object.entries(source['resistances'])) {
      if (typeof factor === 'number' && Number.isFinite(factor)) resistances[school] = factor;
    }
  }

  // Phasen müssen absteigend stehen. Eine unsortierte Liste wäre eine stille
  // Falle: Der Kampf würde Phasen überspringen, ohne dass irgendwo etwas
  // bricht. `sim/build.rs` bricht dafür den Bau ab; hier wird es gemeldet,
  // weil die Spiel-Seite zur Laufzeit liest.
  const phases: PhaseDef[] = [];
  if (Array.isArray(source['phases'])) {
    let vorige = 101;
    for (const entry of source['phases']) {
      if (!isRecord(entry)) continue;
      const atHpPercent = number(entry['atHpPercent'], 0);
      if (atHpPercent <= 0 || atHpPercent >= 100) {
        problems.push(`Phasen-Schwelle bei ${atHpPercent} % ergibt keine Phase.`);
        continue;
      }
      if (atHpPercent >= vorige) {
        problems.push(
          `Phasen-Schwellen müssen absteigend stehen: ${atHpPercent} % nach ${vorige} %.`,
        );
        continue;
      }
      vorige = atHpPercent;
      phases.push({
        atHpPercent,
        name: typeof entry['name'] === 'string' ? entry['name'] : 'Unnamed',
      });
    }
  }

  let energy: EnergyDef | null = null;
  if (isRecord(source['energy'])) {
    const perStepMs = number(source['energy']['perStepMs'], 0);
    const threshold = number(source['energy']['threshold'], 0);
    const resetTo = number(source['energy']['resetTo'], 0);
    if (perStepMs <= 0) problems.push('Die Energieleiste hat keinen Takt (perStepMs).');
    // Das Spiel kann nur zu Takt-Grenzen handeln, der Simulator springt frei.
    // Bei 333 ms stiege die Leiste dort bei 333/666/999, hier bei 340/670/1000
    // — der Abstand wüchse mit jedem Schritt. `sim/build.rs` bricht dafür den
    // Bau ab; hier wird es gemeldet, weil diese Seite zur Laufzeit liest.
    if (perStepMs > 0 && perStepMs % TICK_MS !== 0) {
      problems.push(
        `perStepMs ${perStepMs} liegt nicht auf dem ${TICK_MS}-ms-Raster — ` +
          'Spiel und Simulator liefen mit jedem Anstieg weiter auseinander.',
      );
    }
    if (threshold <= 0) problems.push('Die Energieleiste hat keine Schwelle (threshold).');
    if (resetTo >= threshold) {
      problems.push(
        `resetTo ${resetTo} liegt nicht unter threshold ${threshold} — die Leiste käme nie herunter.`,
      );
    }
    energy = {
      perStepMs,
      amount: number(source['energy']['amount'], 0),
      threshold,
      resetTo,
    };
  }

  // Der Zähler und der Stapel (Issue #216). Beide steigen von selbst, also
  // gilt für beide dieselbe Raster-Prüfung wie für die Energieleiste: Ein Takt
  // neben dem 10-ms-Raster liefe hier und im Simulator mit jedem Schritt
  // weiter auseinander, und zwar still.
  let doom: DoomDef | null = null;
  if (isRecord(source['doom'])) {
    const perStepMs = number(source['doom']['perStepMs'], 0);
    const threshold = number(source['doom']['threshold'], 0);
    const amount = number(source['doom']['amount'], 0);
    const clearAmount = number(source['doom']['clearAmount'], 0);
    const clearCharges = number(source['doom']['clearCharges'], 0);
    if (perStepMs <= 0) problems.push('Der Zähler hat keinen Takt (perStepMs).');
    if (perStepMs > 0 && perStepMs % TICK_MS !== 0) {
      problems.push(
        `doom.perStepMs ${perStepMs} liegt nicht auf dem ${TICK_MS}-ms-Raster — ` +
          'Spiel und Simulator liefen mit jedem Anstieg weiter auseinander.',
      );
    }
    if (threshold <= 0) problems.push('Der Zähler hat keine Schwelle (threshold).');
    if (amount <= 0) problems.push('Der Zähler steigt nicht (amount).');
    // Eine Handlung, die weniger nimmt als ein Takt bringt, ist keine Rettung,
    // sondern eine Verzögerung mit Bedienaufwand. Das wäre eine stille
    // Fehlkonfiguration: Der Zähler stiege trotz richtigen Spiels weiter.
    if (clearCharges > 0 && clearAmount <= amount) {
      problems.push(
        `doom.clearAmount ${clearAmount} nimmt nicht mehr, als ein Takt bringt (${amount}) — ` +
          'der Zähler stiege trotz der Handlung weiter.',
      );
    }
    doom = { perStepMs, amount, threshold, clearAmount, clearCharges };
  }

  let stacks: StackDef | null = null;
  if (isRecord(source['stacks'])) {
    const perStepMs = number(source['stacks']['perStepMs'], 0);
    const baseDamage = number(source['stacks']['baseDamage'], 0);
    const factor = number(source['stacks']['factor'], 0);
    const maxStacks = number(source['stacks']['maxStacks'], 0);
    if (perStepMs <= 0) problems.push('Der Stapel hat keinen Takt (perStepMs).');
    if (perStepMs > 0 && perStepMs % TICK_MS !== 0) {
      problems.push(
        `stacks.perStepMs ${perStepMs} liegt nicht auf dem ${TICK_MS}-ms-Raster — ` +
          'Spiel und Simulator liefen mit jedem Anstieg weiter auseinander.',
      );
    }
    if (baseDamage <= 0) problems.push('Der Stapel richtet keinen Schaden an (baseDamage).');
    // Ein Faktor von 1 wäre linear — und damit ein anderer Baustein.
    if (factor <= 1) {
      problems.push(`stacks.factor ${factor} wächst nicht exponentiell — 11.2 verlangt 1 → 3 → 9.`);
    }
    // Ganzzahlig, weil `baseDamage * factor^(n-1)` sonst auf beiden Seiten
    // verschieden gerundet würde (GAME-DESIGN 18.4, erste Determinismus-Regel).
    if (!Number.isInteger(factor)) {
      problems.push(`stacks.factor ${factor} ist nicht ganzzahlig — beide Seiten rundeten anders.`);
    }
    if (maxStacks <= 0) problems.push('Der Stapel hat keine Obergrenze (maxStacks).');
    stacks = { perStepMs, baseDamage, factor, maxStacks };
  }

  // Wettrennen und Boss-Heilung (Issue #218). Beide wirken auf den
  // Fortschritt statt auf den Spieler; die Raster-Pruefung gilt trotzdem,
  // denn das Fenster beginnt zu einem Zeitpunkt.
  let race: RaceDef | null = null;
  if (isRecord(source['race'])) {
    const atMs = number(source['race']['atMs'], 0);
    const windowMs = number(source['race']['windowMs'], 0);
    if (atMs < 0) problems.push('Das Wettrennen beginnt vor dem Kampf (atMs).');
    if (atMs % TICK_MS !== 0) {
      problems.push(
        `race.atMs ${atMs} liegt nicht auf dem ${TICK_MS}-ms-Raster — ` +
          'Spiel und Simulator begännen das Fenster zu verschiedenen Zeitpunkten.',
      );
    }
    if (windowMs <= 0) problems.push('Das Wettrennen hat keine Dauer (windowMs).');
    const threshold = number(source['race']['threshold'], 0);
    // Eine Schwelle von 0 waere sofort erreicht — das Fenster endete im selben
    // Takt, in dem es beginnt, und die Mechanik gaebe es nur auf dem Papier.
    if (threshold <= 0) {
      problems.push('Das Wettrennen hat keine Schwelle (threshold) — es endete sofort.');
    }
    if (windowMs % TICK_MS !== 0) {
      problems.push(
        `race.windowMs ${windowMs} liegt nicht auf dem ${TICK_MS}-ms-Raster — ` +
          'das Fenster endete auf beiden Seiten verschieden.',
      );
    }
    race = { atMs, windowMs, threshold };
  }

  let bossHeal: BossHealDef | null = null;
  if (isRecord(source['bossHeal'])) {
    const amount = number(source['bossHeal']['amount'], 0);
    // Eine Heilung von 0 wäre eine Mechanik, die nichts tut — und sie sähe im
    // Datensatz aus wie eine, die etwas tut.
    if (amount <= 0) problems.push('Die Boss-Heilung dreht nichts zurück (amount).');
    bossHeal = { amount };
  }

  // Die drei letzten Bausteine (Issue #240).
  let runaway: RunawayDef | null = null;
  if (isRecord(source['runaway'])) {
    const perStepMs = number(source['runaway']['perStepMs'], 0);
    const amount = number(source['runaway']['amount'], 0);
    if (perStepMs <= 0 || perStepMs % TICK_MS !== 0) {
      problems.push(
        `runaway.perStepMs ${perStepMs} muss positiv und auf dem ${TICK_MS}-ms-Raster ` +
          'liegen — sonst kippte die Reaktion auf beiden Seiten zu verschiedenen ' +
          'Zeitpunkten.',
      );
    }
    if (amount <= 0) problems.push('Die Reaktion läuft nicht davon (runaway.amount).');
    runaway = { perStepMs, amount };
  }

  let escalation: EscalationDef | null = null;
  if (isRecord(source['escalation'])) {
    const castTimeShrinkMs = number(source['escalation']['castTimeShrinkMs'], 0);
    const minCastTimeMs = number(source['escalation']['minCastTimeMs'], 0);
    if (castTimeShrinkMs <= 0 || castTimeShrinkMs % TICK_MS !== 0) {
      problems.push(
        `escalation.castTimeShrinkMs ${castTimeShrinkMs} muss positiv und auf dem ` +
          `${TICK_MS}-ms-Raster liegen.`,
      );
    }
    // Ohne Untergrenze schrumpfte das Fenster auf einen Takt — kein Anspruch
    // mehr, sondern eine Unmöglichkeit (11.7).
    if (minCastTimeMs <= 0 || minCastTimeMs % TICK_MS !== 0) {
      problems.push(
        `escalation.minCastTimeMs ${minCastTimeMs} muss positiv und auf dem ` +
          `${TICK_MS}-ms-Raster liegen — ohne Untergrenze wäre der Baustein keine ` +
          'Steigerung, sondern eine Unmöglichkeit.',
      );
    }
    escalation = { castTimeShrinkMs, minCastTimeMs };
  }

  let affliction: AfflictionDef | null = null;
  if (isRecord(source['affliction'])) {
    const atMs = number(source['affliction']['atMs'], 0);
    const perStepMs = number(source['affliction']['perStepMs'], 0);
    const damage = number(source['affliction']['damage'], 0);
    const roh = source['affliction']['clearedBy'];
    const clearedBy = Array.isArray(roh)
      ? roh.filter((x): x is string => typeof x === 'string')
      : [];
    if (atMs < 0 || atMs % TICK_MS !== 0) {
      problems.push(`affliction.atMs ${atMs} liegt nicht auf dem ${TICK_MS}-ms-Raster.`);
    }
    if (perStepMs <= 0 || perStepMs % TICK_MS !== 0) {
      problems.push(
        `affliction.perStepMs ${perStepMs} muss positiv und auf dem ` +
          `${TICK_MS}-ms-Raster liegen.`,
      );
    }
    if (damage <= 0) problems.push('Die Schwächung richtet nichts an (damage).');
    // **Das „mehrere" aus 11.2 ist keine Empfehlung.** Ein Boss, der genau
    // einen Zauber verlangt, macht die Zusammenstellung zur Pflichtaufgabe —
    // und 11.3 sagt, dass kein Boss eine bestimmte Schule verlangen darf.
    if (clearedBy.length < 2) {
      problems.push(
        `affliction.clearedBy nennt ${clearedBy.length} Zauber — 11.2 verlangt ` +
          'MEHRERE Lösungswege, sonst ist man ohne den richtigen Zauber hilflos.',
      );
    }
    affliction = { atMs, perStepMs, damage, clearedBy };
  }

  // Die Zeitleiste: aufsteigend und auf dem Takt-Raster. Beide Verstöße wären
  // still — ein Eintrag hinter einem späteren würde nie ausgelöst, und einer
  // neben dem Raster griffe hier später als im Simulator.
  const timeline: TimelineEntryDef[] = [];
  if (Array.isArray(source['timeline'])) {
    let voriger = -1;
    for (const entry of source['timeline']) {
      if (!isRecord(entry)) continue;
      const atMs = number(entry['atMs'], -1);
      if (atMs < 0) {
        problems.push('Ein Zeitleisten-Eintrag hat keinen Zeitpunkt (atMs).');
        continue;
      }
      if (atMs <= voriger) {
        problems.push(`Die Zeitleiste muss aufsteigend stehen: ${atMs} ms nach ${voriger} ms.`);
        continue;
      }
      if (atMs % TICK_MS !== 0) {
        problems.push(
          `Zeitleisten-Eintrag bei ${atMs} ms liegt nicht auf dem ${TICK_MS}-ms-Raster.`,
        );
        continue;
      }
      const mechanic = typeof entry['mechanic'] === 'string' ? entry['mechanic'] : 'unknown';
      // Ein unbekannter Baustein wird gemeldet und übersprungen, nicht still
      // zu einem Standardfall gemacht: Ein Boss-Zauber, der versehentlich als
      // Schlag gilt, wäre nicht unterbrechbar — im Kampf sähe das wie ein
      // Balance-Problem aus, nicht wie ein Datenfehler. `sim/build.rs` bricht
      // dafür den Bau ab; hier wird gemeldet, weil diese Seite zur Laufzeit liest.
      if (mechanic !== 'strike' && mechanic !== 'interruptibleCast') {
        problems.push(
          `Zeitleisten-Eintrag bei ${atMs} ms nennt die unbekannte Mechanik "${mechanic}".`,
        );
        continue;
      }
      const castTimeMs = number(entry['castTimeMs'], 0);
      if (mechanic === 'interruptibleCast' && castTimeMs <= 0) {
        problems.push(
          `Der Zauber bei ${atMs} ms hat keine Wirkzeit und wäre damit nicht zu unterbrechen.`,
        );
        continue;
      }
      voriger = atMs;
      timeline.push({
        atMs,
        mechanic,
        name: typeof entry['name'] === 'string' ? entry['name'] : 'Unnamed',
        damage: number(entry['damage'], 0),
        castTimeMs,
        // Ohne Angabe false: Ein Boss, der das Feld nicht kennt, bricht keine
        // Wirkzeit ab (GAME-DESIGN 7.1, „normale Treffer stören nicht").
        interruptsPlayer: entry['interruptsPlayer'] === true,
      });
    }
  }

  return {
    value: {
      schema,
      id: typeof source['id'] === 'string' ? source['id'] : 'unknown',
      name: typeof source['name'] === 'string' ? source['name'] : 'Unknown',
      instance: typeof source['instance'] === 'string' ? source['instance'] : 'unknown',
      hp,
      overpressureMs,
      resistances,
      phases,
      energy,
      doom,
      stacks,
      race,
      bossHeal,
      // Fehlt das Feld, ist es Stufe 1 — jeder heutige Datensatz kommt so
      // durch, und E10 trägt sie nach.
      level: Math.max(Math.trunc(number(source['level'], 1)), 1),
      runaway,
      escalation,
      affliction,
      timeline,
      timelineLoopMs,
    },
    problems,
  };
}

/** Die Schule aus Schritt 1, fertig gelesen. */
export const CALCINATION: ParseResult<SchoolDef> = (() => {
  // Die schulunabhängigen Sonderzauber (GAME-DESIGN 7.4) kommen **hinter** die
  // Schulzauber, immer. Die Reihenfolge ist Teil des Determinismus
  // (GAME-DESIGN 18.3): Sie bestimmt die Indizes, und die stehen in jeder
  // aufgezeichneten Runde. Sie umzustellen deutete jede alte Aufzeichnung
  // still um — und `sim/build.rs` fügt sie an derselben Stelle an.
  const roh = rawSchool as Record<string, unknown>;
  const gemeinsam = (rawCommonSpells as Record<string, unknown>)['spells'];
  const koagulation = (rawCoagulation as Record<string, unknown>)['spells'];
  /**
   * Jedem Zauber seine Herkunftsschule mitgeben (Issue #270, AC0).
   *
   * **Hier und nur hier**, weil hier die Datei noch bekannt ist. Eine Zeile
   * weiter sind die sechs Listen eine, und die Herkunft ist fort — genau der
   * Zustand, der „nach Schule gruppiert" unmöglich machte.
   *
   * Die Reihenfolge fasst das nicht an: Jeder Eintrag bekommt ein Feld dazu,
   * keiner wechselt die Stelle. Die Indizes stehen in jeder aufgezeichneten
   * Runde (GAME-DESIGN 18.3), und ein Vergleichs-Prüffall misst genau das.
   */
  const mitSchule = (liste: unknown, quelle: Record<string, unknown>): unknown[] => {
    const schule = quelle['school'];
    return (Array.isArray(liste) ? liste : []).map((eintrag) =>
      isRecord(eintrag) && typeof schule === 'string' ? { ...eintrag, school: schule } : eintrag,
    );
  };
  const zusammen = {
    ...roh,
    // **Hier entsteht die Zauberreihenfolge — aus sieben Dateien in genau
    // dieser Folge** (Issue #408):
    //
    //     calcination, common, coagulation, fixation,
    //     sublimation, putrefaction, test-cast
    //
    // Die Reihenfolge bestimmt die Zauber-Indizes, und die Indizes stehen in
    // jeder aufgezeichneten Runde (`tests/parity/recordings/`) und in jedem
    // gespeicherten Spielstand.
    //
    // Zwei Dinge verschieben sie, und BEIDE tun es still:
    //
    //   1. ein Zauber, der nicht ganz am Ende dazukommt
    //   2. eine andere Folge DIESER DATEIEN — auch ohne einen neuen Zauber
    //
    // Der zweite Fall sieht harmlos aus („nur umsortiert, nichts entfernt")
    // und ist genauso schwer: Wer `coagulation` vor `common` zieht, deutet
    // jede bestehende Aufzeichnung um.
    //
    // Bewacht von `tests/unit/spell-order.test.ts` (Issue #402); vorführbar
    // mit `tools/reihenfolge-gegenprobe.sh`.
    spells: [
      ...mitSchule(roh['spells'], roh),
      // common.json trägt kein `school` — seine Zauber sind schulunabhängig
      // (GAME-DESIGN 7.4), und `mitSchule` lässt sie deshalb unberührt.
      ...mitSchule(gemeinsam, rawCommonSpells as Record<string, unknown>),
      ...mitSchule(koagulation, rawCoagulation as Record<string, unknown>),
      ...mitSchule(
        (rawFixation as Record<string, unknown>)['spells'],
        rawFixation as Record<string, unknown>,
      ),
      // **HIER STAND DIE REGEL „nur ans Ende der GESAMTLISTE"** (Issues #201,
      // #408). Sie ist am 2026-08-06 zurückgenommen worden, weil sie nachweislich
      // nichts sparte — Einzelheiten in `tests/unit/spell-order.test.ts`.
      //
      // In einem Satz: Die Prüfsumme rührt fünf Felder JE ZAUBER ein, also
      // bewegt ein Zauber ganz hinten genauso viele Aufzeichnungen wie einer in
      // der Mitte. Gemessen 23 gegen 23. Die Regel kostete den fachlich
      // richtigen Ort und kaufte dafür nichts.
      //
      // **Ein neuer Zauber gehört in seine Schuldatei.** Was dazugehört:
      // `spell-order.json` nachziehen und `npm run parity:export` fahren, im
      // selben Commit. Das ist ohnehin bei jedem neuen Zauber nötig.
      ...mitSchule(
        (rawSublimation as Record<string, unknown>)['spells'],
        rawSublimation as Record<string, unknown>,
      ),
      // Steht hier aus historischen Gründen (Issue #97). Die Reihenfolge der
      // Blöcke untereinander bleibt, wie sie ist — sie zu ändern bewegte jede
      // Aufzeichnung ohne Gegenwert. Neue Zauber gehören in ihre Schuldatei,
      // siehe den Block oben.
      ...mitSchule(
        (rawPutrefaction as Record<string, unknown>)['spells'],
        rawPutrefaction as Record<string, unknown>,
      ),
      // **Der Prüf-Zauber** (Issue #304). Er steht am Ende, weil er hier
      // entstanden ist — nicht, weil ein neuer Zauber hinter ihn müsste. Diese
      // Auflage ist am 2026-08-06 zurückgenommen worden.
      //
      // Er gehört nicht zum
      // Spiel — seine Stufe liegt über der Grenze, und beide Stellen, die dem
      // Spieler Zauber anbieten, filtern darüber. Er steht trotzdem in dieser
      // Liste, weil der Simulator dieselbe braucht: Was nicht in `SPELLS`
      // steht, lässt sich in keiner Aufzeichnung drücken und damit nicht auf
      // Gleichlauf prüfen.
      ...mitSchule(
        (rawTestCast as Record<string, unknown>)['spells'],
        rawTestCast as Record<string, unknown>,
      ),
      // **Und danach der Prüf-Zauber zur Verstärkung** (Issue #420). Seine
      // Stellung hinter `test-cast` bleibt, weil ein Tausch Aufzeichnungen
      // bewegte, ohne etwas zu verbessern — nicht, weil sie geschützt wäre.
      //
      // Warum es überhaupt einen zweiten Prüf-Zauber braucht: `test-cast` ist
      // der Wirkzeit-Fall aus #304. Ihm eine Verstärkung anzuhängen vermischte
      // zwei Prüfzwecke, und wer später die Wirkzeit misst, hätte still einen
      // Buff dabei.
      ...mitSchule(
        (rawTestBuff as Record<string, unknown>)['spells'],
        rawTestBuff as Record<string, unknown>,
      ),
    ],
  };
  return parseSchool(zusammen);
})();

/** Der Testboss aus Schritt 1, fertig gelesen. */
export const FIRST_CRUCIBLE: ParseResult<BossDef> = parseBoss(rawBoss);

/**
 * **Alle** Bosse, nach Bezeichner — aus dem Verzeichnis gelesen, nicht einzeln
 * aufgezählt (Issue #96, AC1: „ein neuer Boss entsteht als Datendatei, ohne
 * dass eine Zeile Programmcode geändert wird").
 *
 * Der feste `import` oben reicht dafür nicht: Er nennt genau eine Datei, und
 * ein zweiter Boss bräuchte eine zweite Zeile. `import.meta.glob` löst das zur
 * Bauzeit auf — die JSON-Dateien landen also weiterhin gebündelt im Ergebnis,
 * es wird nichts zur Laufzeit nachgeladen.
 *
 * Der Simulator macht dasselbe an seiner Stelle: `sim/build.rs` liest das
 * Verzeichnis sortiert. **Beide Seiten müssen dieselbe Menge sehen**, sonst
 * kennt einer einen Boss, den der andere nicht rechnen kann.
 */
export const BOSSES: ReadonlyMap<string, ParseResult<BossDef>> = (() => {
  const dateien = import.meta.glob<Record<string, unknown>>('../../data/bosses/*.json', {
    eager: true,
    import: 'default',
  });
  // Sortiert, damit die Reihenfolge nicht vom Dateisystem abhängt — dieselbe
  // Entscheidung wie `dateien.sort()` in `sim/build.rs` (GAME-DESIGN 18.4:
  // feste Reihenfolge, keine Hash-Container).
  const pfade = Object.keys(dateien).sort();
  const out = new Map<string, ParseResult<BossDef>>();
  for (const pfad of pfade) {
    const gelesen = parseBoss(dateien[pfad]);
    out.set(gelesen.value.id, gelesen);
  }
  return out;
})();

/**
 * Die Grundwerte des Spielers, aus `data/player/base.json`.
 *
 * Dieselbe Datei, die `sim/build.rs` einliest — **eine Wahrheit für beide
 * Seiten**. Zwei getrennte Zahlensätze wären genau die Art Abweichung, die
 * kein Prüffall meldet, weil beide Seiten für sich stimmig blieben.
 *
 * # Verhältnis zur Ausrüstung (E6)
 *
 * `src/gear/slots.ts` führt mit `BASE_STAT_IDS` die **Kennungen** der
 * Grundwerte (`essence`, `vessel`, `ward`); diese Datei führt ihre **Werte**.
 * Das sind zwei verschiedene Aussagen, kein Widerspruch.
 *
 * **Diese Werte sind der STARTWERT.** Sobald ein Ausrüstungsstück VESSEL
 * trägt, wird es darauf **addiert** — es ersetzt sie nicht. Heute trägt kein
 * Stück etwas, also fällt die Frage nicht auf; wer sie zusammenführt, findet
 * die Antwort hier.
 *
 * **Eine Formel fehlt dabei noch, und sie fehlt mit Absicht:** Ein
 * Ausrüstungs-WARD ist ein Punktwert, `wardReduction` hier ein Anteil
 * (0,2 = 20 % weniger Schaden). Wie Punkte zu einem Anteil werden, ist eine
 * Design-Entscheidung und steht in GAME-DESIGN Kapitel 22 als offen. Sie hier
 * zu erfinden hieße, sie stillschweigend zu treffen.
 */
export const PLAYER: PlayerBaseDef = (() => {
  const roh = rawPlayer as Record<string, unknown>;
  const vessel = number(roh['vessel'], 0);
  const wardReduction = number(roh['wardReduction'], 0);
  const wardCapPermille = number(roh['wardCapPermille'], 0);
  // Kein `problems`-Kanal hier: Diese Datei ist keine Inhaltsdatei, die ein
  // Autor pflegt, sondern eine Voraussetzung des Spiels. Fehlt sie oder ist
  // sie unsinnig, ist nichts spielbar — dann ist ein harter Abbruch ehrlicher
  // als ein Spieler mit null Lebenspunkten.
  if (vessel <= 0) throw new Error('data/player/base.json: vessel muss positiv sein.');
  if (wardReduction < 0 || wardReduction >= 1) {
    throw new Error(
      'data/player/base.json: wardReduction muss in [0, 1) liegen — bei 1 wäre der ' +
        'Spieler unverwundbar, darüber würde ihn Schaden heilen.',
    );
  }
  // **Ein Deckel unter der Basis wäre keine Grenze, sondern eine Absenkung.**
  // Dann minderte getragene Ausrüstung weniger als gar keine, und jeder Punkt
  // WARD wäre schädlich — ein Zustand, den niemand beabsichtigen kann und der
  // sich still auswirkte.
  if (wardCapPermille <= wardReduction * 1000) {
    throw new Error(
      `data/player/base.json: wardCapPermille (${wardCapPermille}) muss über der Basis ` +
        `wardReduction (${wardReduction * 1000} ‰) liegen — sonst mindert Ausrüstung weniger ` +
        `als keine.`,
    );
  }
  return { vessel, wardReduction, wardCapPermille };
})();

export const LEVELS: LevelGrowthDef = (() => {
  const roh = rawLevels as Record<string, unknown>;
  const maxLevel = number(roh['maxLevel'], 0);
  const growthPerLevelPercent = number(roh['growthPerLevelPercent'], 0);
  const factorPerMille = Array.isArray(roh['factorPerMille'])
    ? (roh['factorPerMille'] as unknown[]).map((x) => number(x, 0))
    : [];

  // Harter Abbruch wie bei `base.json`, aus demselben Grund: Das ist keine
  // Inhaltsdatei, sondern eine Voraussetzung. Eine kaputte Tabelle ergäbe
  // einen Spieler, dessen Werte auf halber Strecke einfrieren — und das fiele
  // erst jemandem auf Stufe 60 auf.
  if (maxLevel < 1) throw new Error('data/player/levels.json: maxLevel muss positiv sein.');
  if (factorPerMille.length !== maxLevel) {
    throw new Error(
      `data/player/levels.json: ${factorPerMille.length} Faktoren für ${maxLevel} Stufen. ` +
        'Jede Stufe braucht genau einen — sonst wächst der Spieler ab einer Stelle nicht mehr.',
    );
  }
  if (factorPerMille[0] !== 1000) {
    throw new Error(
      `data/player/levels.json: Stufe 1 trägt ${String(factorPerMille[0])} statt 1000. ` +
        'Der Spieler startet mit den Werten aus base.json, nicht darüber oder darunter.',
    );
  }
  // **Monoton steigend**, sonst würde ein Aufstieg den Spieler schwächen.
  for (let i = 1; i < factorPerMille.length; i++) {
    if ((factorPerMille[i] ?? 0) <= (factorPerMille[i - 1] ?? 0)) {
      throw new Error(
        `data/player/levels.json: Stufe ${i + 1} ist nicht größer als Stufe ${i}. ` +
          'Ein Aufstieg, der schwächer macht, ist keiner.',
      );
    }
  }
  const xpPerBossLevel = number(roh['xpPerBossLevel'], 0);
  const xpToNextLevel = Array.isArray(roh['xpToNextLevel'])
    ? (roh['xpToNextLevel'] as unknown[]).map((x) => number(x, 0))
    : [];
  if (xpPerBossLevel <= 0) {
    throw new Error('data/player/levels.json: xpPerBossLevel muss positiv sein.');
  }
  // **Ein Eintrag WENIGER als Stufen**: von 1 nach 2 bis von 99 nach 100.
  // Ein Eintrag zu viel sähe aus wie ein Aufstieg über die Höchststufe hinaus.
  if (xpToNextLevel.length !== maxLevel - 1) {
    throw new Error(
      `data/player/levels.json: ${xpToNextLevel.length} Bedarfe für ${maxLevel - 1} ` +
        'Aufstiege. Der letzte Eintrag führt von der vorletzten Stufe zur höchsten.',
    );
  }
  for (let i = 0; i < xpToNextLevel.length; i++) {
    if ((xpToNextLevel[i] ?? 0) <= 0) {
      throw new Error(
        `data/player/levels.json: Bedarf für Stufe ${i + 1} ist nicht positiv — ` +
          'eine Stufe ohne Bedarf würde beim ersten Sieg übersprungen.',
      );
    }
    if (i > 0 && (xpToNextLevel[i] ?? 0) <= (xpToNextLevel[i - 1] ?? 0)) {
      throw new Error(
        `data/player/levels.json: Stufe ${i + 1} verlangt nicht mehr als Stufe ${i}. ` +
          'Die Kurve soll steigen; sonst geht der Aufstieg gegen Ende schneller.',
      );
    }
  }
  const talentPointTarget = number(roh['talentPointTarget'], 0);
  const talentPointLevels = Array.isArray(roh['talentPointLevels'])
    ? (roh['talentPointLevels'] as unknown[]).map((x) => number(x, 0))
    : [];
  if (talentPointTarget < 1) {
    throw new Error('data/player/levels.json: talentPointTarget muss positiv sein.');
  }
  // **Die Zielgröße gegen die Verteilung**, nicht die Verteilung gegen sich
  // selbst. Das ist die ganze Begründung dafür, dass beide Zahlen dastehen.
  if (talentPointLevels.length !== talentPointTarget) {
    throw new Error(
      `data/player/levels.json: ${talentPointLevels.length} Stufen mit Talentpunkt für ein ` +
        `Ziel von ${talentPointTarget}. Eine Verteilung, die das Ziel verfehlt, verschiebt ` +
        'still, wie tief ein Spieler seine Bäume ausbauen kann.',
    );
  }
  for (let i = 0; i < talentPointLevels.length; i++) {
    const stufe = talentPointLevels[i] ?? 0;
    if (stufe < 1 || stufe > maxLevel) {
      throw new Error(
        `data/player/levels.json: talentPointLevels[${i}] ist ${stufe} — erlaubt sind 1 bis ` +
          `${maxLevel}. Ein Punkt jenseits der Höchststufe käme nie.`,
      );
    }
    // **Aufsteigend und ohne Dubletten.** Zwei Punkte auf derselben Stufe
    // wären als Liste nicht darstellbar — sie sähen aus wie einer.
    if (i > 0 && stufe <= (talentPointLevels[i - 1] ?? 0)) {
      throw new Error(
        `data/player/levels.json: talentPointLevels[${i}] (${stufe}) kommt nicht nach ` +
          `talentPointLevels[${i - 1}]. Die Liste muss aufsteigen und darf keine Stufe ` +
          'doppelt nennen.',
      );
    }
  }
  const startBarSlots = number(roh['startBarSlots'], 0);
  const barSlotLevels = Array.isArray(roh['barSlotLevels'])
    ? (roh['barSlotLevels'] as unknown[]).map((x) => number(x, 0))
    : [];
  if (startBarSlots < 1) {
    throw new Error(
      'data/player/levels.json: startBarSlots muss mindestens 1 sein — mit null Plätzen ' +
        'ließe sich kein Zauber wirken.',
    );
  }
  for (let i = 0; i < barSlotLevels.length; i++) {
    const stufe = barSlotLevels[i] ?? 0;
    if (stufe < 2 || stufe > maxLevel) {
      throw new Error(
        `data/player/levels.json: barSlotLevels[${i}] ist ${stufe} — erlaubt sind 2 bis ` +
          `${maxLevel}. Ein Platz auf Stufe 1 gehörte in startBarSlots, einer darüber käme nie.`,
      );
    }
    if (i > 0 && stufe <= (barSlotLevels[i - 1] ?? 0)) {
      throw new Error(
        `data/player/levels.json: barSlotLevels[${i}] (${stufe}) kommt nicht nach ` +
          `barSlotLevels[${i - 1}]. Die Liste muss aufsteigen, sonst ist unklar, wie viele ` +
          'Plätze eine Stufe hat.',
      );
    }
  }
  return {
    growthPerLevelPercent,
    maxLevel,
    factorPerMille,
    xpPerBossLevel,
    xpToNextLevel,
    talentPointTarget,
    talentPointLevels,
    startBarSlots,
    barSlotLevels,
  };
})();

/**
 * Was ein Sieg einbringt (GAME-DESIGN 12.6; Issue #276).
 *
 * **Eine Datei für beide Währungen.** Der Abstandsfaktor gilt für Erfahrung
 * UND Punkte — 12.6 sagt „der Ertrag", nicht „die Erfahrung". Läge er bei
 * einer der beiden, gehörte er ihr, und die andere bekäme irgendwann einen
 * zweiten daneben.
 */
export interface RewardDef {
  /**
   * Der abnehmende Ertrag als Tabelle in Promille, Index = Stufenabstand.
   *
   * 100 Einträge für die Abstände 0 bis 99. Als Tabelle statt als Potenz, weil
   * ein Stufenabstand ganzzahlig ist — die erzeugende Formel steht im
   * `_comment` der Datei.
   */
  readonly falloffPerMille: readonly number[];
  /** Bis zu diesem Abstand voller Ertrag — zur Nachvollziehbarkeit der Tabelle. */
  readonly fullRewardWithinLevels: number;
  /** Halbwertszeit in Stufen — zur Nachvollziehbarkeit der Tabelle. */
  readonly falloffHalfLifeLevels: number;
  /** Punkte je Sekunde Restlaufzeit, vor dem Exponenten. */
  readonly pointsPerSecond: number;
  /**
   * Der Exponent der Punktekurve.
   *
   * **Muss größer als 1 sein**, sonst ist die Kurve nicht überproportional und
   * 12.6 wäre verfehlt. Die Prüfung steht unten und bricht hart ab: Eine
   * lineare Punkteformel sähe im Spiel völlig normal aus.
   */
  readonly pointsExponent: number;
  /** Was ein Prozentpunkt gehaltener Ring in Sekunden Restlaufzeit wiegt. */
  readonly heldPointsPerPercent: number;
  /**
   * Der Rest-Ertrag fürs blosse Wiederholen, in Promille (Issue #377).
   *
   * **Unter 1000**, sonst wäre ein Wiederholungslauf so viel wert wie eine
   * Verbesserung — und dann trüge die Differenz-Auszahlung aus 12.6 nichts.
   */
  readonly repeatPerMille: number;
}

/**
 * Ein Talent im Baum (Issue #417, GAME-DESIGN 12.5).
 *
 * **Promille je Rang, nicht ein Faktor je Rang** — wie `factorPerMille` in
 * `levels.json` und aus demselben Grund (18.4): Der Faktor entsteht am Ende aus
 * ganzen Zahlen, statt dass jede Stufe eine Fliesskommazahl mitbringt.
 */
export interface TalentDef {
  readonly id: string;
  readonly name: string;
  /** `trunk` oder einer der drei Wege — die Struktur aus 12.5. */
  readonly path: string;
  readonly maxRanks: number;
  readonly damagePerRankPerMille: number;
  /** Welches Talent vorher Ränge braucht, oder `null` beim Stamm. */
  readonly requires: string | null;
  /**
   * Welches **Verhalten** dieses Talent umschaltet, oder `null` (Issue #418).
   *
   * **Das ist die Nicht-Zahl.** Die drei Wege aus GAME-DESIGN 12.5 tragen
   * alle `damagePerRankPerMille` — sie erhöhen Zahlen. Ein Schlüsseltalent
   * ändert stattdessen, WAS geschieht: GAME-DESIGN 13.09 nennt als Beispiel
   * „Der Entlade-Zauber verbraucht nur die HÄLFTE der Hitze, wirkt dafür
   * ZWEIMAL".
   *
   * Ein Schlüsseltalent, das sich als Faktor ausdrücken ließe, wäre keines —
   * es wäre ein Faktor mit einem großen Namen. Der Nachweis dafür ist der
   * Gegenfall: Ohne das Talent muss sich der Lauf in der **Reihenfolge** oder
   * im **Verbrauch** unterscheiden, nicht nur in der Schadenszahl.
   */
  readonly keystone: string | null;
}

/**
 * Alle Talente aller Bäume, flach.
 *
 * Flach und nicht je Baum verschachtelt: Gerechnet wird über alle, und eine
 * zweite Ebene wäre eine Schleife mehr an jeder Stelle. Zu welchem Baum ein
 * Talent gehört, steht in seiner Kennung und in `path`.
 */
export const TALENTS: readonly TalentDef[] = (() => {
  const roh = rawTalents as { trees?: { talents?: unknown[] }[] };
  const alle: TalentDef[] = [];
  const gesehen = new Set<string>();

  for (const tree of roh.trees ?? []) {
    for (const eintrag of tree.talents ?? []) {
      const t = eintrag as Record<string, unknown>;
      const id = typeof t['id'] === 'string' ? t['id'] : '';
      if (id.length === 0) {
        throw new Error('data/player/talents.json: ein Talent ohne Kennung.');
      }
      // **Kennungen sind der Schlüssel im Spielstand.** Zwei gleiche wären dort
      // ein Eintrag, und die Ränge des einen zählten für den anderen.
      if (gesehen.has(id)) {
        throw new Error(`data/player/talents.json: die Kennung "${id}" kommt zweimal vor.`);
      }
      gesehen.add(id);

      const maxRanks = number(t['maxRanks'], 0);
      if (maxRanks < 1) {
        throw new Error(
          `data/player/talents.json: "${id}" hat maxRanks ${maxRanks} — mindestens 1.`,
        );
      }
      const damagePerRankPerMille = number(t['damagePerRankPerMille'], 0);
      if (damagePerRankPerMille < 0) {
        throw new Error(
          `data/player/talents.json: "${id}" hat ${damagePerRankPerMille} Promille je Rang. ` +
            'Ein Talent, das schwächer macht, ist keines.',
        );
      }
      // **Ein Verhalten hat keine Ränge.** Es ist an oder aus — „zur Hälfte
      // verbraucht der Entlade-Zauber nur die halbe Hitze" ergibt keinen Sinn.
      // Die Prüfung steht hier und nicht im Kampf, weil ein zweiter Rang dort
      // stillschweigend wie der erste wirkte.
      if (typeof t['keystone'] === 'string' && maxRanks !== 1) {
        throw new Error(
          `data/player/talents.json: "${id}" schaltet ein Verhalten um und hat ` +
            `${maxRanks} Ränge. Ein Verhalten ist an oder aus — maxRanks muss 1 sein.`,
        );
      }
      alle.push({
        id,
        name: typeof t['name'] === 'string' ? t['name'] : id,
        path: typeof t['path'] === 'string' ? t['path'] : '',
        maxRanks,
        damagePerRankPerMille,
        requires: typeof t['requires'] === 'string' ? t['requires'] : null,
        keystone: typeof t['keystone'] === 'string' ? t['keystone'] : null,
      });
    }
  }

  // **Jede Vorbedingung muss es geben.** Ein `requires` auf eine Kennung, die
  // nicht existiert, spärrte das Talent für immer — und zwar still: Es stünde
  // im Baum und liesse sich nie steigern.
  for (const talent of alle) {
    if (talent.requires !== null && !gesehen.has(talent.requires)) {
      throw new Error(
        `data/player/talents.json: "${talent.id}" verlangt "${talent.requires}", ` +
          'das es nicht gibt.',
      );
    }
  }
  if (alle.length === 0) {
    throw new Error('data/player/talents.json: kein einziges Talent.');
  }
  return alle;
})();

export const REWARD: RewardDef = (() => {
  const roh = rawReward as Record<string, unknown>;
  const falloffPerMille = Array.isArray(roh['falloffPerMille'])
    ? (roh['falloffPerMille'] as unknown[]).map((x) => number(x, 0))
    : [];
  const fullRewardWithinLevels = number(roh['fullRewardWithinLevels'], 0);
  const falloffHalfLifeLevels = number(roh['falloffHalfLifeLevels'], 0);
  const pointsPerSecond = number(roh['pointsPerSecond'], 0);
  const pointsExponent = number(roh['pointsExponent'], 0);
  const heldPointsPerPercent = number(roh['heldPointsPerPercent'], 0);
  const repeatPerMille = number(roh['repeatPerMille'], 0);

  // **Ein Eintrag je möglichem Abstand.** Bei 100 Stufen reicht der Abstand von
  // 0 bis 99; eine kürzere Tabelle liefe irgendwo ins Leere und zahlte dort
  // still den vollen Ertrag.
  if (falloffPerMille.length !== LEVELS.maxLevel) {
    throw new Error(
      `data/player/reward.json: ${falloffPerMille.length} Einträge für ${LEVELS.maxLevel} ` +
        'mögliche Stufenabstände. Jeder Abstand braucht genau einen.',
    );
  }
  if (falloffPerMille[0] !== 1000) {
    throw new Error(
      `data/player/reward.json: Abstand 0 trägt ${String(falloffPerMille[0])} statt 1000. ` +
        'Ein Boss auf der eigenen Stufe zahlt voll — das ist die Aussage von 12.6.',
    );
  }
  // **Monoton fallend**, sonst gäbe es einen Abstand, der sich wieder lohnt.
  for (let i = 1; i < falloffPerMille.length; i++) {
    if ((falloffPerMille[i] ?? 0) > (falloffPerMille[i - 1] ?? 0)) {
      throw new Error(
        `data/player/reward.json: Abstand ${i} zahlt mehr als Abstand ${i - 1}. ` +
          'Der Ertrag soll sinken, nicht schwanken.',
      );
    }
    if ((falloffPerMille[i] ?? -1) < 0) {
      throw new Error(
        `data/player/reward.json: Abstand ${i} ist negativ. Ein Sieg darf nichts abwerfen, ` +
          'aber nichts abziehen.',
      );
    }
  }
  if (pointsPerSecond <= 0) {
    throw new Error('data/player/reward.json: pointsPerSecond muss positiv sein.');
  }
  // **Die Design-Aussage als Bedingung an die Daten.** „Überproportional" ist
  // genau das: Exponent über 1. Bei 1 wäre die Kurve linear, bei weniger
  // sänke der Wert der letzten Sekunden — das Gegenteil von 12.6, und im
  // laufenden Spiel wäre es nicht zu sehen.
  if (!(pointsExponent > 1)) {
    throw new Error(
      `data/player/reward.json: pointsExponent ist ${pointsExponent}. GAME-DESIGN 12.6 ` +
        'verlangt eine überproportionale Kurve — das heißt größer als 1. Bei genau 1 wäre ' +
        'die Formel linear und Feinoptimierung lohnte sich nicht mehr als grobe.',
    );
  }
  if (heldPointsPerPercent <= 0) {
    throw new Error(
      'data/player/reward.json: heldPointsPerPercent muss positiv sein — sonst zahlte ein ' +
        'runaway-Boss nie etwas, weil dort die Restzeit immer null ist.',
    );
  }
  // **Die Design-Aussage als Bedingung an die Daten**, wie beim Exponenten:
  // 12.6 sagt „Punkte für Bestzeit-Verbesserung PLUS ein Rest-Ertrag fürs
  // blosse Wiederholen". Bei 1000 wäre Wiederholen genauso viel wert wie
  // Verbessern, bei 0 gäbe es den Rest-Ertrag gar nicht — beides widerspricht
  // dem Satz, und beides sähe im laufenden Spiel unauffällig aus.
  if (!(repeatPerMille > 0) || repeatPerMille >= 1000) {
    throw new Error(
      `data/player/reward.json: repeatPerMille ist ${repeatPerMille}. GAME-DESIGN 12.6 ` +
        'verlangt einen Rest-Ertrag fürs blosse Wiederholen UND eine Differenz-Auszahlung ' +
        'für die Verbesserung — erlaubt ist deshalb nur echt zwischen 0 und 1000.',
    );
  }
  return {
    falloffPerMille,
    fullRewardWithinLevels,
    falloffHalfLifeLevels,
    pointsPerSecond,
    pointsExponent,
    heldPointsPerPercent,
    repeatPerMille,
  };
})();

/**
 * Ein Grundwert auf einer Stufe (Issue #248).
 *
 * **Gerechnet, nicht gespeichert** — GAME-DESIGN 18.6 und AC5 des Issues. Der
 * Spielstand hält die Stufe; was daraus folgt, entsteht bei jedem Aufruf neu.
 * Wer den Wert stattdessen ablegte, machte jede künftige Balance-Änderung für
 * bestehende Spielstände wirkungslos — und niemand merkte es, weil nichts
 * kaputtgeht.
 *
 * Stufen außerhalb der Tabelle werden geklemmt statt abgelehnt: Ein Spielstand
 * aus einer Fassung mit höherer Höchststufe soll spielbar bleiben.
 */
export function statAtLevel(base: number, level: number): number {
  const stufe = Math.min(Math.max(Math.trunc(level), 1), LEVELS.maxLevel);
  const promille = LEVELS.factorPerMille[stufe - 1] ?? 1000;
  return Math.trunc((base * promille) / 1000);
}

/**
 * Ein Boss nach Bezeichner, oder `undefined`.
 *
 * Das Gegenstück zu `boss_by_id` im Simulator (`sim/src/data.rs`).
 */
export function bossById(id: string): BossDef | undefined {
  return BOSSES.get(id)?.value;
}
