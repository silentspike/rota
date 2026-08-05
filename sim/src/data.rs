//! Zauber- und Boss-Daten — einkompiliert aus `data/`.
//!
//! Die Werte in diesem Modul stammen **nicht** aus dieser Quelldatei. Sie werden
//! von `build.rs` aus `data/spells/calcination.json` und
//! `data/bosses/first-crucible.json` erzeugt, und zwar bei jedem Bau neu
//! (`cargo:rerun-if-changed`). Wer eine Zahl aendern will, aendert die JSON-Datei;
//! hier stehen nur die Formen, in die sie gegossen wird.
//!
//! Damit gilt fuer den Simulator dasselbe wie fuers Spiel: `data/` ist die
//! einzige Wahrheit (GAME-DESIGN 18.3, `STEP1-SPEC` § 5 — das Format ist
//! ausdruecklich eine Schnittstelle zwischen beiden Umsetzungen).

use crate::time::Millis;

/// Eine Schul-Mechanik: die Ressource, die sich auflaedt und wieder verfaellt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resource {
    /// Bezeichner aus der Datei, z. B. `heat`.
    pub id: &'static str,
    /// Anzeigename im Spiel (englisch, siehe Design-Gesetz 5).
    pub name: &'static str,
    /// Obergrenze. Was darueber hinaus erzeugt wird, verpufft.
    pub cap: u16,
    /// Abstand zwischen zwei Verfall-Schritten.
    pub decay_per_step_ms: Millis,
    /// Wie viel je Verfall-Schritt abgeht.
    pub decay_amount: u16,
    /// Ab hier zeigt das HUD eine Markierung — fuer den Simulator nur mitgefuehrt.
    pub highlight_at: u16,
}

/// Ein Brand-Effekt: Schaden ueber Zeit statt beim Wirken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dot {
    /// Schaden je Tick.
    pub tick_damage: u32,
    /// Abstand zwischen zwei Ticks.
    pub tick_interval_ms: Millis,
    /// Gesamtlaufzeit ab dem Wirken.
    pub duration_ms: Millis,
    /// Ressourcen-Gewinn je Tick.
    pub resource_gain_per_tick: u16,
}

/// Ein Entlade-Zauber: verbraucht die Ressource und schlaegt dafuer zu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spender {
    /// Schaden je Ressourcen-Punkt.
    pub damage_per_point: u32,
    /// Ab diesem Stand greift der Bonus — **ab**, nicht darueber.
    pub bonus_threshold: u16,
    /// Womit der Schaden bei erreichtem Bonus vervielfacht wird.
    pub bonus_multiplier: u32,
}

/// Eine Schutzschicht (COAGULATION, Weg 1 — GAME-DESIGN 4.1).
///
/// > Nimmt eingehenden Schaden auf und laesst sich stapeln — **belohnt
/// > Vorausschauen, weil sie VOR dem Schlag stehen muss.**
///
/// Das „vorher" ist die ganze Mechanik. Eine Schutzschicht, die man nach dem
/// Schlag aufbaut, waere eine Heilung mit anderem Namen — und die gibt es in
/// dieser Schule bereits als eigenen Weg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shield {
    /// Wie viel Schaden eine Anwendung aufnimmt.
    pub amount: u32,
    /// Wie lange sie steht, wenn sie nicht aufgebraucht wird.
    pub duration_ms: Millis,
    /// Wie viele Anwendungen sich stapeln lassen.
    pub max_stacks: u32,
}

/// Eine Heilung (COAGULATION, Weg 2).
///
/// > Holt verlorenes Leben zurueck und **verzeiht Fehler nachtraeglich.**
///
/// Der Gegenentwurf zur Schutzschicht: Sie wirkt, nachdem der Schaden
/// gefallen ist, kostet dafuer aber eine Wirkung, die kein Schaden war.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heal {
    /// Wie viele Lebenspunkte zurueckkommen.
    pub amount: u32,
}

/// Schaden-in-Schutz-Umwandlung (COAGULATION, Weg 3).
///
/// > Verwandelt einen Teil des **ausgeteilten** Schadens laufend in Schutz.
///
/// Der dritte Weg haengt damit am eigenen Schaden statt am eingehenden: Wer
/// hart zuschlaegt, schuetzt sich nebenbei. Er ist der einzige der drei, der
/// nicht in dem Moment wirkt, in dem er gewirkt wird — er schaltet einen
/// Zustand fuer eine Weile an.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conversion {
    /// Anteil des ausgeteilten Schadens in **Prozent**, ganzzahlig.
    ///
    /// Ganzzahlig und nicht als Bruch: Ganzzahlige Werte sind die erste
    /// Determinismus-Regel (GAME-DESIGN 18.4). Ein `0.25` in der Datei waere
    /// eine Fliesskommazahl, die beide Seiten unabhaengig runden muessten —
    /// und genau daran laufen zwei Umsetzungen auseinander.
    pub share_percent: u32,
    /// Wie lange die Umwandlung nach dem Wirken laeuft.
    pub duration_ms: Millis,
}

/// Was ein Fixierungs-Zauber mit Kontroll-Punkten anstellt (FIXATION,
/// GAME-DESIGN 4.1, Issue #100).
///
/// > Kontroll-Punkte als Vorrat zum Ausgeben. Der Spieler gibt sie **gezielt**
/// > aus, um Boss-Handlungen zu beeinflussen. Ein **Planungswerkzeug**, keine
/// > passive Wirkung.
///
/// Das „gezielt" ist die eigentliche Aussage: Punkte werden nie automatisch
/// verbraucht. Reichen sie nicht, geschieht **nichts** — der Zauber ist
/// vertan, aber er nimmt sich nicht, was er nicht hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Control {
    /// Wie viele Punkte die Wirkung kostet.
    pub cost: u32,
    /// Welche der drei Boss-Handlungen sie beeinflusst.
    pub effect: ControlEffect,
    /// Um wie viel — in Millisekunden oder Prozentpunkten, je nach Wirkung.
    pub amount: u32,
}

/// Die drei Wirkungen aus GAME-DESIGN 4.1.
///
/// **Diese drei stehen fest**, anders als ihre Zahlen: Sie stammen woertlich
/// aus dem Design und sind keine Umsetzungs-Entscheidung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlEffect {
    /// Einen Boss-Zauber verlangsamen: Die Wirkzeit dehnt sich, und damit wird
    /// das Fenster zum Unterbrechen groesser (#110, Baustein 9).
    SlowCast,
    /// Einen Schlag verzoegern: Der Zeitleisten-Eintrag rueckt nach hinten.
    DelayStrike,
    /// Eine Phase hinauszoegern: Die Schwelle sinkt voruebergehend.
    ///
    /// **Sie nimmt einen Wechsel nicht zurueck.** Die Phase kann nur steigen
    /// (GAME-DESIGN 11.1, eskalierend; `Fight::check_phase`) — eine gesenkte
    /// Schwelle verschiebt den naechsten Wechsel, mehr nicht. Laeuft die
    /// Fixierung waehrend einer hoeheren Phase aus, faellt der Boss nicht
    /// zurueck.
    HoldPhase,
}

/// Ein Zauber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spell {
    /// Bezeichner aus der Datei, z. B. `kindle`.
    pub id: &'static str,
    /// Anzeigename im Spiel.
    pub name: &'static str,
    /// Ab welcher Spielerstufe der Zauber verfuegbar ist.
    pub unlock_level: u16,
    /// Sofortschaden beim Wirken.
    pub base_damage: u32,
    /// Eigene Abklingzeit.
    pub cooldown_ms: Millis,
    /// Wirkzeit. In Schritt 1 durchweg 0 (Sofort-Zauber).
    pub cast_time_ms: Millis,
    /// Ob der Zauber die globale Wartezeit ausloest.
    pub triggers_gcd: bool,
    /// Ob dieser Zauber einen laufenden Boss-Zauber **unterbricht**
    /// (GAME-DESIGN 11.2, Baustein 9).
    ///
    /// Steht in den Daten und nicht im Code: Ein Zauber, der das kann, ist ein
    /// Datensatz wie jeder andere (Leitplanke 6). Er ist zugleich die **einzige
    /// Ausnahme** von der globalen Wartezeit im ganzen Spiel (GAME-DESIGN 6.6)
    /// — aber auch das steht als `triggers_gcd: false` in denselben Daten und
    /// nicht als Sonderfall in der Kampfschleife.
    pub interrupt: bool,
    /// Ressourcen-Gewinn beim Wirken.
    pub resource_gain: u16,
    /// Brand-Effekt, falls der Zauber einen hat.
    pub dot: Option<Dot>,
    /// Entlade-Verhalten, falls der Zauber die Ressource verbraucht.
    pub spender: Option<Spender>,
    /// Um wie viel dieser Zauber **alle laufenden Abklingzeiten** verkuerzt
    /// (SUBLIMATION, GAME-DESIGN 4.1, Issue E3-3).
    ///
    /// **0 heisst: verkuerzt nichts.** Das Feld ist in der Datei optional; die
    /// drei Zauber der Kalzination fuehren es nicht. Ein Pflichtfeld waere hier
    /// falsch — ein Zauber ohne Sublimations-Wirkung soll es gar nicht erst
    /// nennen muessen.
    pub cooldown_reduction_ms: Millis,
    /// Ob dieser Zauber den **Auswahlmodus** scharf stellt (SUBLIMATION,
    /// Issue E3-3c; getragen seit Issue #201 von REFLUX).
    ///
    /// **Das Feld steht hier, weil `data/` die einzige Wahrheit ist** — es
    /// beschreibt den Zauber, so wie die Datei ihn beschreibt.
    ///
    /// GELESEN WIRD ES IM SIMULATOR NOCH NICHT. `cooldown_reset.rs` baut den
    /// Modus vollstaendig, aber kein Aufruf im Kampf ruft ihn: Der Druck-Pfad
    /// in `Fight::replay` kennt nur "wirken oder ablehnen", nicht "waehlen".
    /// Das ist die offene Haelfte von Issue #201 (AC4) und ausdruecklich
    /// gemeldet — nicht uebersehen.
    pub arms_cooldown_reset: bool,
    /// Schutzschicht, falls der Zauber eine aufbaut (COAGULATION, Weg 1).
    pub shield: Option<Shield>,
    /// Heilung, falls der Zauber eine ist (Weg 2).
    pub heal: Option<Heal>,
    /// Schaden-in-Schutz-Umwandlung, falls der Zauber sie anschaltet (Weg 3).
    pub conversion: Option<Conversion>,
    /// Wie viele Zersetzungs-Stapel der Zauber anlegt (PUTREFACTION, #97).
    pub decay_stacks: u32,
    /// Um wie viel Prozent der eigene Schaden je liegendem Stapel steigt.
    ///
    /// Traegt FESTER. **Nicht** die Verwundbarkeit des Bosses — die gilt fuer
    /// alle Quellen. **Vorlaeufig, wird gemessen.**
    pub per_decay_stack_percent: u32,
    /// Wie viele Kontroll-Punkte der Zauber **aufbaut** (FIXATION, #100).
    ///
    /// Steht als eigenes Feld und nicht unter `resource_gain`: Der Kampf
    /// fuehrt genau eine Schul-Ressource (Hitze), und jede Schul-Groesse hat
    /// eigene Felder, weil sie eigene Regeln hat. Hitze verfaellt, die
    /// Schutzschicht faengt auf, Kontroll-Punkte werden ausgegeben — das sind
    /// verschiedene Dinge, keine Instanzen desselben Dings.
    pub control_gain: u32,
    /// Was der Zauber mit Kontroll-Punkten **anstellt**, falls er sie ausgibt.
    pub control: Option<Control>,
}

/// Eine Phase: ab welchem Anteil der Lebenspunkte der Boss sein Verhalten
/// wechselt (GAME-DESIGN 11.1).
///
/// **Eskalierend, nicht tauschend:** Jede Phase legt eine Mechanik obendrauf,
/// statt die vorige zu ersetzen. Die Mechaniken selbst kommen mit E4-2; hier
/// steht nur, WANN gewechselt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase {
    /// Anteil der verbleibenden Lebenspunkte in Prozent, ab dem sie gilt.
    pub at_hp_percent: u32,
    /// Anzeigename der Phase.
    pub name: &'static str,
}

/// Ein Eintrag der Boss-Zeitleiste (GAME-DESIGN 11.1: „Schadens-Ereignisse als
/// **feste Zeitleiste**“).
///
/// **Fest heisst wirklich fest.** Der Boss reagiert nicht auf den Spieler, und
/// es gibt keine zufaellige Streuung — nur so sind Bestzeiten vergleichbar und
/// der Kampf ueberhaupt simulierbar (GAME-DESIGN 11.1, Absatz danach).
///
/// **Seit Issue #110 hat ein Eintrag Wirkung.** In #96 war dies noch ein
/// leeres Geruest: Der Zeitpunkt wurde taktgenau angesteuert, aber es geschah
/// nichts. Jetzt teilt der Boss damit Schaden aus — sofort ([`TimelineKind::Strike`])
/// oder nach einer Wirkzeit, die sich unterbrechen laesst
/// ([`TimelineKind::InterruptibleCast`]).
///
/// Was **weiterhin** fehlt, sind die uebrigen sieben Mechanik-Bausteine
/// (E4-3 und spaeter) und die Zeitleisten der einzelnen Bosse, die nach E10
/// gehoeren (GAME-DESIGN 22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEntry {
    /// Zeitpunkt seit Kampfbeginn. Liegt auf dem Takt-Raster des Spiels.
    ///
    /// Bei einem Zauber ist das der Zeitpunkt, zu dem er **beginnt** — nicht
    /// der, zu dem er trifft. Der Treffer liegt `cast_time_ms` spaeter, und
    /// dazwischen liegt das Fenster, in dem unterbrochen werden kann.
    pub at_ms: Millis,
    /// Kennung des Mechanik-Bausteins, der hier greift.
    pub mechanic: &'static str,
    /// Anzeigename der Faehigkeit. Englisch, wie alle Spieltexte.
    pub name: &'static str,
    /// Was hier geschieht.
    pub kind: TimelineKind,
    /// Schaden am Spieler, **vor** WARD.
    pub damage: u32,
    /// Wirkzeit eines Boss-Zaubers. 0 bei allem, was sofort trifft.
    pub cast_time_ms: Millis,
}

/// Was ein Zeitleisten-Eintrag tut.
///
/// Nur zwei Arten, und das ist Absicht: Die uebrigen der neun Mechanik-
/// Bausteine (GAME-DESIGN 11.2) kommen in E4-3 und spaeter. Wer hier eine
/// dritte Art braucht, fuegt sie hinzu — das Geruest aus #96 traegt sie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineKind {
    /// Trifft sofort. Nicht zu verhindern, nur auszuhalten.
    Strike,
    /// Ein Zauber mit Wirkzeit. **Unterbrechbar**, solange er laeuft
    /// (GAME-DESIGN 11.2, Baustein 9).
    ///
    /// Kommt er durch, trifft er mit vollem Schaden; wird er unterbrochen,
    /// richtet er gar nichts an. Ein Mittelding gibt es nicht — das ist die
    /// Entscheidung, die den Baustein ausmacht.
    InterruptibleCast,
}

/// Die Energieleiste — die zweite Uhr eines Bosses.
///
/// Aus GAME-DESIGN 11.2, dem ersten der neun Mechanik-Bausteine:
///
/// > Boss-Energie steigt **unabhaengig von den Lebenspunkten** und loest bei 100
/// > eine Mechanik aus. Gibt dem Kampf einen inneren Rhythmus, der nichts mit
/// > dem Lebensbalken zu tun hat.
///
/// Das „unabhaengig" ist die eigentliche Aussage: Wer schneller Schaden macht,
/// verschiebt den Ausloeser **nicht**. Damit entsteht ein zweiter Takt, gegen
/// den der Spieler planen muss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Energy {
    /// Abstand zwischen zwei Anstiegen.
    pub per_step_ms: Millis,
    /// Wie viel je Anstieg dazukommt.
    pub amount: u32,
    /// Ab diesem Stand loest sie aus.
    pub threshold: u32,
    /// Worauf sie nach dem Ausloesen faellt.
    pub reset_to: u32,
}

/// Der Zaehler, der bei seiner Schwelle toetet (GAME-DESIGN 11.2, Baustein 2).
///
/// > Ein Wert steigt unfreiwillig an; bei 100 stirbt man. Zuruecksetzen nur
/// > ueber eine **begrenzt verfuegbare Handlung**.
///
/// Der Unterschied zur Energieleiste: Die springt zurueck und loest aus, dieser
/// Zaehler toetet und springt gar nicht zurueck. Zwei Felder statt eines
/// Schalters — wer sie zusammenlegte, muesste an jeder Auswertung fragen,
/// welche Sorte gemeint ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doom {
    /// Abstand zwischen zwei Anstiegen.
    pub per_step_ms: Millis,
    /// Wie viel je Anstieg dazukommt.
    pub amount: u32,
    /// Bei diesem Stand stirbt der Spieler.
    pub threshold: u32,
    /// Was die Ruecksetz-Handlung abzieht.
    pub clear_amount: u32,
    /// Wie oft sie im ganzen Kampf zur Verfuegung steht.
    pub clear_charges: u32,
}

/// Der exponentielle Stapel (GAME-DESIGN 11.2, Baustein 3).
///
/// > 1 Stapel = X, 2 = 3X, 3 = 9X. Erzwingt eine Handlung an einem exakt
/// > bestimmten Punkt.
///
/// Beim Zaehler oben ist die SCHWELLE hart, hier die KURVE: Es gibt keinen
/// Punkt, an dem etwas umschlaegt — der Schaden wird schlicht unbezahlbar, und
/// zwar an einer Stelle, die sich ausrechnen laesst, bevor sie kommt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stacks {
    /// Abstand zwischen zwei Stapeln.
    pub per_step_ms: Millis,
    /// Schaden beim ERSTEN Stapel — das X aus 11.2.
    pub base_damage: u32,
    /// Wachstumsfaktor je weiterem Stapel. 3 ergibt 1 -> 3 -> 9.
    pub factor: u32,
    /// Obergrenze. Nicht Design, sondern Arithmetik: Ohne sie verliesse
    /// `base_damage * factor^(n-1)` den Wertebereich, und beide Seiten taeten
    /// das verschieden.
    pub max_stacks: u32,
}

/// Schaden eines exponentiellen Stapels.
///
/// `base_damage * factor^(stacks-1)` — bei EINEM Stapel steht X, nicht 3X.
/// Der Exponent ist um eins versetzt; mit `factor^stacks` begaenne die Reihe
/// bei 3X und der erste Stapel waere schon so teuer wie der zweite sein soll.
///
/// Ganzzahlig in einer Schleife, wie `stackDamage` in `src/core/damage.ts`.
#[must_use]
pub fn stack_damage(def: Stacks, stacks: u32) -> u32 {
    if stacks == 0 {
        return 0;
    }
    let mut value = def.base_damage;
    for _ in 1..stacks {
        value = value.saturating_mul(def.factor);
    }
    value
}

/// Das Wettrennen gegen einen Zauber (GAME-DESIGN 11.2, Baustein 4; #218).
///
/// > Der Boss wird unverwundbar, waehrend etwas anderes vor Ablauf zerstoert
/// > werden muss. **Der beste Moment fuer Overdrive.**
///
/// DAS ZIEL IST EIN SCHWELLENWERT, KEIN OBJEKT. Ein zweites Ziel mit eigenen
/// Lebenspunkten waere ein Zusatzgegner, und GAME-DESIGN 22 verschiebt die auf
/// Version 2. Stattdessen laedt der Schaden im Fenster einen Zaehler; erreicht
/// er `threshold`, ist das Ziel zerstoert und das Fenster endet frueh.
///
/// Es ist die Energieleiste mit umgekehrter Fuellrichtung: Dort steigt sie von
/// selbst, hier fuellt der Spieler sie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Race {
    /// Zeitpunkt seit Kampfbeginn, zu dem das Fenster beginnt.
    pub at_ms: Millis,
    /// Wie lange es hoechstens dauert.
    pub window_ms: Millis,
    /// Wie viel Schaden im Fenster das Ziel zerstoert. Vorlaeufig.
    pub threshold: u32,
}

/// Ein Fehler heilt den Boss (GAME-DESIGN 11.2, Baustein 5; #218).
///
/// > Ein Fehler macht Fortschritt rueckgaengig. Haerteste Form von Konsequenz.
/// > **Nur fuer einzelne Bosse**, sonst frustrierend.
///
/// Der Ausloeser ist der durchgekommene Boss-Zauber — kein neuer
/// Fehler-Begriff. Dass es ein DATENFELD ist und keine feste Kopplung, ist die
/// Umsetzung von "nur fuer einzelne": Eine Kopplung im Code gaelte fuer alle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossHeal {
    /// Wie viel Fortschritt ein durchgekommener Boss-Zauber zurueckdreht.
    pub amount: u32,
}

/// Ein Boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Boss {
    /// Bezeichner aus der Datei.
    pub id: &'static str,
    /// Anzeigename im Spiel.
    pub name: &'static str,
    /// Instanz, zu der er gehoert.
    pub instance: &'static str,
    /// Lebenspunkte.
    pub hp: u32,
    /// Zeitlimit: Ist es abgelaufen, hat der Spieler verloren.
    pub overpressure_ms: Millis,
    /// Phasen-Schwellen, absteigend. Leer heisst: keine Phasen.
    pub phases: &'static [Phase],
    /// Die zweite Uhr, falls dieser Boss eine hat.
    pub energy: Option<Energy>,
    /// Der toedliche Zaehler, falls dieser Boss einen hat (Issue #216).
    pub doom: Option<Doom>,
    /// Der exponentielle Stapel, falls dieser Boss einen hat (Issue #216).
    pub stacks: Option<Stacks>,
    /// Das Unverwundbarkeits-Fenster, falls dieser Boss eines hat (#218).
    pub race: Option<Race>,
    /// Was ein Fehler an Fortschritt zurueckdreht, falls ueberhaupt (#218).
    pub boss_heal: Option<BossHeal>,
    /// Die feste Abfolge der Boss-Handlungen, aufsteigend nach Zeitpunkt.
    pub timeline: &'static [TimelineEntry],
    /// Laenge eines Zyklus, wenn sich die Zeitleiste **wiederholt** (#174).
    ///
    /// `None` heisst: Sie laeuft einmal und endet — so verhaelt sich jeder
    /// Boss. Nur die Trainingspuppe wiederholt sich, weil GAME-DESIGN 15.5 ihr
    /// „kein Zeitlimit" zusagt; eine Zeitleiste, die nach 48 Sekunden aufhoert,
    /// waere diese Zusage mit Kleingedrucktem.
    ///
    /// Der Wert ist der Zeitversatz je Durchlauf, nicht die Zahl der Eintraege:
    /// Eintrag `i` im Zyklus `z` ist faellig bei `z * loop + at_ms`.
    pub timeline_loop_ms: Option<Millis>,
}

impl Boss {
    /// Welche Phase bei diesem Anteil der Lebenspunkte gilt.
    ///
    /// 0 ist der Start. Die Zaehlung steigt, waehrend die Lebenspunkte fallen —
    /// der Boss geht also von Phase 0 nach oben.
    #[must_use]
    pub fn phase_at(&self, hp_percent: u32) -> usize {
        let mut phase = 0;
        for (index, schwelle) in self.phases.iter().enumerate() {
            if hp_percent <= schwelle.at_hp_percent {
                phase = index + 1;
            }
        }
        phase
    }
}

/// Die Grundwerte des Spielers (GAME-DESIGN 6.1).
///
/// **Berechnet, nicht gespeichert.** Der Spielstand haelt sie nicht — sie
/// ergeben sich aus Ausruestung und Talenten, und beides gibt es noch nicht
/// (E5, E6). Bis dahin stehen sie in `data/player/base.json`.
///
/// > **Die Zahlen sind noch nicht Design.** GAME-DESIGN Teil F.6 fuehrt sie
/// > als „vor E4 auszuplanen"; entschieden ist davon nichts. Sie tragen die
/// > Prueffaelle aus Issue #110 und sonst nichts.
///
/// # Verhaeltnis zur Ausruestung (E6)
///
/// `src/gear/slots.ts` fuehrt mit `BASE_STAT_IDS` die **Kennungen** der
/// Grundwerte; diese Datei fuehrt ihre **Werte**. Zwei verschiedene Aussagen,
/// kein Widerspruch.
///
/// **Diese Werte sind der STARTWERT.** Ausruestung wird darauf **addiert**,
/// sie ersetzt ihn nicht. Heute traegt kein Stueck etwas, also faellt die
/// Frage nicht auf; wer sie zusammenfuehrt, findet die Antwort hier und in
/// `src/core/data.ts`.
///
/// Eine Formel von WARD-Punkten zu Minderungsanteil fehlt dabei noch, und
/// zwar mit Absicht — sie ist Design und steht in Kapitel 22 als offen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerBase {
    /// VESSEL — Lebenspunkte.
    pub vessel: u32,
    /// WARD — Anteil, um den eingehender Schaden gemindert wird.
    ///
    /// Ein Anteil und keine Ruestungszahl: Eine Ruestungsformel waere eine
    /// Design-Entscheidung, und die ist nicht gefallen. Ein Anteil traegt den
    /// Prueffall, ohne etwas vorwegzunehmen.
    pub ward_reduction: f64,
}

impl PlayerBase {
    /// Wie viel Schaden nach WARD ankommt.
    ///
    /// **Einmal gerundet, am Ende** — dieselbe Regel wie bei ausgehendem
    /// Schaden (GAME-DESIGN 6.3). Zweimal zu runden ergaebe je nach
    /// Reihenfolge andere Zahlen, und genau daran laufen zwei Umsetzungen
    /// auseinander.
    #[must_use]
    pub fn incoming(&self, raw: u32) -> u32 {
        let gemindert = f64::from(raw) * (1.0 - self.ward_reduction);
        // Nie unter null, und nie mehr als der Rohwert: Ein WARD ueber 1.0
        // wuerde den Spieler sonst heilen.
        gemindert.round().max(0.0) as u32
    }
}

include!(concat!(env!("OUT_DIR"), "/data_generated.rs"));

/// Sucht einen Boss ueber seinen Bezeichner.
///
/// Lineare Suche ueber eine Liste mit fester Reihenfolge — bei 68 Bossen ist
/// der Unterschied zu einer Hash-Abbildung nicht messbar, und die Suche laeuft
/// ohnehin nur beim Aufbau, nie im Schritt.
#[must_use]
pub fn boss_by_id(id: &str) -> Option<&'static Boss> {
    BOSSES.iter().find(|boss| boss.id == id)
}

/// Sucht einen Zauber ueber seinen Bezeichner.
///
/// Lineare Suche ueber eine Liste mit fester Reihenfolge — **absichtlich keine
/// Hash-Abbildung** (GAME-DESIGN 18.3). Bei drei bis sechzig Zaubern ist der
/// Unterschied nicht messbar, und die Suche laeuft ohnehin nur beim Aufbau,
/// nie im Schritt.
#[must_use]
pub fn index_of(id: &str) -> Option<usize> {
    let mut i = 0;
    while i < SPELL_COUNT {
        if SPELLS[i].id == id {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn die_drei_zauber_aus_schritt_1_sind_da() {
        // Drei Schadenszauber, dazu seit Issue #110 der Unterbrecher. Er zaehlt
        // nicht zu den dreien aus `STEP1-SPEC`: Er richtet keinen Schaden an
        // und kommt in keiner der vier Pruefrotationen vor.
        // **Abgeleitet, nicht abgeschrieben** (Issue #201). Hier stand die Zahl
        // 11 und darueber ein Kommentar, der sie aufzaehlte — bei jedem neuen
        // Zauber von Hand hochzusetzen. Was hier zaehlt, ist nicht die Zahl,
        // sondern dass beide Seiten dieselbe Menge sehen; das prueft
        // `parity_step1` gegen die Aufzeichnung des Spiels.
        //
        // Scharf bleibt der Fall trotzdem: Faellt ein Zauber weg, bricht eine
        // der Zusicherungen darunter.
        assert_eq!(SPELL_COUNT, SPELLS.len());
        // "Es darf keiner verschwinden" — an den Kennungen geprueft, nicht an
        // einer Zahl. Ein `assert!(SPELL_COUNT >= 11)` waere zur Uebersetzungszeit
        // konstant; clippy lehnt das mit -D warnings ab, und zu Recht: Es
        // prueft nichts, was sich zur Laufzeit noch aendern koennte.
        for id in [
            "kindle", "smoulder", "pyre", "quench", "crust", "salve", "temper", "bind", "stall",
            "stay", "anchor",
        ] {
            assert!(index_of(id).is_some(), "Zauber {id} ist verschwunden");
        }
        assert_eq!(index_of("kindle"), Some(KINDLE));
        assert_eq!(index_of("smoulder"), Some(SMOULDER));
        assert_eq!(index_of("pyre"), Some(PYRE));
        assert_eq!(index_of("gibtesnicht"), None);

        // Nicht ueber `base_damage` gezaehlt: SMOULDER ist ein Brand-Effekt und
        // PYRE ein Entlader, beide haben 0 Sofortschaden. Der Unterbrecher
        // unterscheidet sich nicht durch den Schaden, sondern durch die Rolle.
        // Die drei aus Schritt 1: keine Unterbrecher, keine Koagulation.
        let schritt1 = SPELLS
            .iter()
            .filter(|s| {
                !s.interrupt
                    && s.shield.is_none()
                    && s.heal.is_none()
                    && s.conversion.is_none()
                    && s.control.is_none()
                    && s.control_gain == 0
                    // Und seit Issue #201 die Sublimation: VAPOUR verkuerzt,
                    // REFLUX setzt zurueck. Ohne diese zwei Zeilen zaehlte der
                    // Filter sie zu den "drei Kampfzaubern aus Schritt 1" —
                    // sie tragen keins der Merkmale darueber.
                    && s.cooldown_reduction_ms == 0
                    && !s.arms_cooldown_reset
                    // Und seit Issue #97 die Zersetzung: CANKER legt Stapel an,
                    // FESTER lebt von ihnen. Auch sie tragen keins der Merkmale
                    // darueber und zaehlten sonst zu den drei aus Schritt 1.
                    && s.decay_stacks == 0
                    && s.per_decay_stack_percent == 0
            })
            .count();
        assert_eq!(schritt1, 3, "Schritt 1 hat genau drei Kampfzauber");
    }

    #[test]
    fn der_unterbrecher_ist_die_einzige_ausnahme_von_der_wartezeit() {
        // GAME-DESIGN 6.6: **Genau eine** Ausnahme. Ein zweiter Zauber ohne
        // globale Wartezeit waere eine Design-Aenderung, die sich in eine
        // Datendatei einschleichen koennte, ohne dass jemand sie beschliesst.
        let ohne_gcd: Vec<&str> = SPELLS
            .iter()
            .filter(|s| !s.triggers_gcd)
            .map(|s| s.id)
            .collect();
        assert_eq!(ohne_gcd, vec!["quench"]);

        let unterbrecher: Vec<&str> = SPELLS
            .iter()
            .filter(|s| s.interrupt)
            .map(|s| s.id)
            .collect();
        assert_eq!(unterbrecher, vec!["quench"]);
    }

    #[test]
    fn die_reihenfolge_ist_die_der_datei() {
        // Sie ist Teil des Determinismus: Spiel und Simulator muessen dieselbe
        // Reihenfolge sehen, sonst weichen Ereignisse bei Gleichstand ab.
        assert_eq!(SPELLS[0].id, "kindle");
        assert_eq!(SPELLS[1].id, "smoulder");
        assert_eq!(SPELLS[2].id, "pyre");
    }

    #[test]
    fn die_schul_mechanik_ist_die_hitze() {
        assert_eq!(SCHOOL, "CALCINATION");
        assert_eq!(RESOURCE.id, "heat");
    }
}
