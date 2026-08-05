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
import rawBoss from '../../data/bosses/first-crucible.json';
import rawPlayer from '../../data/player/base.json';
import { TICK_MS } from './loop.js';

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
  readonly resourceGain?: Readonly<Record<string, number>>;
  readonly dot?: DotDef;
  readonly spender?: SpenderDef;
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
   * Setzt dieser Zauber den **Auswahlmodus** scharf (SUBLIMATION, Issue E3-3c)?
   *
   * Der Rücksetzer wirkt nicht auf den nächsten gewirkten Zauber, sondern
   * fragt: Welchen? Zwei Schritte, drei Aktionen — die Entscheidung des
   * Auftraggebers vom 2026-07-29.
   */
  readonly armsCooldownReset?: boolean;
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
 * Das umgekehrte Ziel (GAME-DESIGN 11.2, Baustein 6; Issue #240).
 *
 * > Statt Schaden zu machen muss eine kippende Reaktion **stabilisiert**
 * > werden. Der Kampf, in dem COAGULATION glänzt.
 *
 * DER ERSTE ZWECK FÜR EINE GANZE SCHULE
 * -------------------------------------
 * Die drei Wege der Koagulation sind seit #79 gebaut, und **kein Boss
 * verlangte sie**: Wer sie mitnimmt, verliert Schaden und gewinnt nichts. Der
 * Befund aus #224 sagt, warum das nicht bloß Geschmackssache ist — jeder
 * Zauber ohne direkten Schaden kostet einen GCD und muss ihn zurückverdienen,
 * und für Koagulation gab es nichts zurückzuverdienen.
 *
 * WOMIT STABILISIERT WIRD, STEHT NICHT IN DIESEN DATEN
 * ---------------------------------------------------
 * Sondern in den **Zauberdaten**: Schutz und Heilung heben den Wert um ihren
 * eigenen Betrag. Eine zweite Zahl „stabilisiert um X" neben `shield.amount`
 * wäre dieselbe Größe an zwei Orten, und die zweite läuft beim Nachziehen weg.
 *
 * TEMPER BLEIBT DRAUSSEN — UND DAS IST DIE GRENZE DES BAUSTEINS, KEINE LÜCKE
 * ---------------------------------------------------------------------------
 * 11.2 sagt: „**Statt** Schaden zu machen muss eine kippende Reaktion
 * stabilisiert werden."
 *
 * TEMPER wandelt ausgeteilten Schaden laufend in Schutz. Würde dieser Schutz
 * stabilisieren, dann stabilisierte man **durch Schaden machen** — und die
 * Bedingung, die den ganzen Kampf ausmacht, wäre aufgehoben. Der Boss-Typ
 * verwandelte sich in einen normalen Kampf, bei dem man nebenbei stabilisiert.
 *
 * Es ist auch kein Mangel an TEMPER: GAME-DESIGN 4.1 nennt die drei Wege
 * ausdrücklich „drei Wege **nebeneinander**" — drei Spielweisen, nicht drei
 * Antworten auf jede Frage. Ein Boss, der Schutzschicht und Heilung belohnt,
 * muss nicht auch die Umwandlung belohnen.
 *
 * > **Offene Frage, damit sie niemand für ein Versehen hält:** Welcher
 * > Boss-Typ belohnt die Umwandlung? Vermutlich einer mit hohem Dauerschaden,
 * > bei dem laufender Schutz besser trägt als punktuelle Heilung. Der
 * > Orchestrator schneidet das als eigenes Issue, wenn #240 durch ist.
 *
 * Wer hier „die Lücke repariert", nimmt dem Baustein seinen Kern.
 */
export interface StabiliseDef {
  /** Womit die Reaktion beginnt. */
  readonly startValue: number;
  /** Takt, in dem sie sinkt. Auf dem 10-ms-Raster. */
  readonly dropPerStepMs: number;
  /** Wie viel sie je Takt verliert. */
  readonly dropAmount: number;
  /** Darunter kippt sie — der Lauf endet. */
  readonly failBelow: number;
  /** Obergrenze; darüber hinaus wird nicht stabilisiert. */
  readonly maxValue: number;
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
}

export interface BossDef {
  readonly schema: number;
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
  /** Die kippende Reaktion, oder `null` (Issue #240). */
  readonly stabilise: StabiliseDef | null;
  /** Die Eskalation je Zyklus, oder `null` (Issue #240). */
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
    const amount = number(knoten['amountMs'], 0) || number(knoten['amountPercent'], 0);

    if (cost <= 0) {
      // Eine Wirkung ohne Kosten wäre keine Ausgabe von Kontroll-Punkten,
      // sondern eine passive Wirkung — und genau das ist FIXATION nicht.
      problems.push(`${where}: Eine Kontroll-Wirkung ohne Kosten ist keine Ausgabe.`);
    } else if (!['slowCast', 'delayStrike', 'holdPhase'].includes(effect)) {
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
  const resourceGain = isRecord(raw['resourceGain']) ? gains(raw['resourceGain']) : undefined;

  const unterbricht = raw['interrupt'] === true;
  if (unterbricht && raw['triggersGcd'] !== false) {
    // Die Ausnahme aus GAME-DESIGN 6.6 mit falschem Vorzeichen: Ein
    // Unterbrecher mit Wartezeit ließe sich nicht mehr zwischen zwei Zauber
    // legen, und der ganze Baustein wäre tot.
    problems.push(`${where}: Ein Unterbrecher darf keine globale Wartezeit auslösen.`);
  }
  const reduction = raw['cooldownReductionMs'];
  const cooldownReductionMs = typeof reduction === 'number' ? reduction : undefined;
  if (cooldownReductionMs !== undefined) {
    if (cooldownReductionMs < 0) problems.push(`${where}: Verkürzung ist negativ.`);
    if (!Number.isInteger(cooldownReductionMs)) {
      problems.push(`${where}: Verkürzung ${cooldownReductionMs} ist nicht ganzzahlig.`);
    }
  }

  return {
    id,
    name: typeof raw['name'] === 'string' ? raw['name'] : id,
    unlockLevel: number(raw['unlockLevel'], 1),
    baseDamage: number(raw['baseDamage'], 0),
    cooldownMs,
    castTimeMs: number(raw['castTimeMs'], 0),
    triggersGcd: raw['triggersGcd'] !== false,
    interrupt: unterbricht,
    ...parseCoagulation(raw, where, problems),
    ...parseFixation(raw, where, problems),
    ...parsePutrefaction(raw, where, problems),
    ...(resourceGain !== undefined ? { resourceGain } : {}),
    ...(dot !== undefined ? { dot } : {}),
    ...(spender !== undefined ? { spender } : {}),
    ...(cooldownReductionMs !== undefined ? { cooldownReductionMs } : {}),
    ...(raw['armsCooldownReset'] === true ? { armsCooldownReset: true } : {}),
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
  let stabilise: StabiliseDef | null = null;
  if (isRecord(source['stabilise'])) {
    const startValue = number(source['stabilise']['startValue'], 0);
    const dropPerStepMs = number(source['stabilise']['dropPerStepMs'], 0);
    const dropAmount = number(source['stabilise']['dropAmount'], 0);
    const failBelow = number(source['stabilise']['failBelow'], 0);
    const maxValue = number(source['stabilise']['maxValue'], 0);
    if (dropPerStepMs <= 0 || dropPerStepMs % TICK_MS !== 0) {
      problems.push(
        `stabilise.dropPerStepMs ${dropPerStepMs} muss positiv und auf dem ` +
          `${TICK_MS}-ms-Raster liegen — sonst kippte die Reaktion auf beiden Seiten ` +
          'zu verschiedenen Zeitpunkten.',
      );
    }
    if (dropAmount <= 0) problems.push('Die Reaktion sinkt nicht (dropAmount).');
    // Startet sie schon unter der Schwelle, ist der Lauf im ersten Takt vorbei
    // — und die Mechanik gäbe es nur auf dem Papier.
    if (startValue <= failBelow) {
      problems.push(
        `stabilise.startValue ${startValue} liegt nicht über failBelow ${failBelow} — ` +
          'der Lauf endete sofort.',
      );
    }
    if (maxValue < startValue) {
      problems.push(`stabilise.maxValue ${maxValue} liegt unter dem Startwert.`);
    }
    stabilise = { startValue, dropPerStepMs, dropAmount, failBelow, maxValue };
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
      stabilise,
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
  const zusammen = {
    ...roh,
    spells: [
      ...(Array.isArray(roh['spells']) ? roh['spells'] : []),
      ...(Array.isArray(gemeinsam) ? gemeinsam : []),
      ...(Array.isArray(koagulation) ? koagulation : []),
      ...(Array.isArray((rawFixation as Record<string, unknown>)['spells'])
        ? ((rawFixation as Record<string, unknown>)['spells'] as unknown[])
        : []),
      // **Ganz hinten angefügt** (Issue #201). Die Reihenfolge bestimmt die
      // Indizes, und die stehen in jeder aufgezeichneten Runde — vorne
      // einzufügen deutete jede alte Aufzeichnung still um.
      ...(Array.isArray((rawSublimation as Record<string, unknown>)['spells'])
        ? ((rawSublimation as Record<string, unknown>)['spells'] as unknown[])
        : []),
      // Ebenfalls hinten (Issue #97), aus demselben Grund.
      ...(Array.isArray((rawPutrefaction as Record<string, unknown>)['spells'])
        ? ((rawPutrefaction as Record<string, unknown>)['spells'] as unknown[])
        : []),
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
  return { vessel, wardReduction };
})();

/**
 * Ein Boss nach Bezeichner, oder `undefined`.
 *
 * Das Gegenstück zu `boss_by_id` im Simulator (`sim/src/data.rs`).
 */
export function bossById(id: string): BossDef | undefined {
  return BOSSES.get(id)?.value;
}
