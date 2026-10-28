//! Der Balance-Läufer: viele Durchgänge, Mittelwert, Fehlerbalken.
//!
//! Aus GAME-DESIGN 18.5, das dritte von SimulationCraft übernommene Konzept:
//!
//! > **Monte-Carlo** — Viele Durchgänge, Mittelwert und Fehlerbalken — nötig,
//! > weil Krit echter Zufall ist.
//!
//! Und aus 6.5:
//!
//! > Das „Optimum" ist ein **Erwartungswert**, kein fester Wert. Die
//! > Optimums-Suche im Simulator muss über viele Durchläufe mitteln.
//!
//! # Warum die Reihenfolge der Addition hier ein eigenes Thema ist
//!
//! GAME-DESIGN 18.4 verlangt: *"Ergebnisse nach Startwert sortieren vor dem
//! Zusammenzählen. Fließkomma-Addition ist nicht assoziativ; sonst schwankt der
//! Mittelwert je nach Laune des Betriebssystems."*
//!
//! Das ist keine Theorie. Läuft die Rechnung über sechzehn Kerne, kommen die
//! Ergebnisse in der Reihenfolge zurück, in der die Kerne fertig werden — und
//! die hängt an der Systemlast. `(a + b) + c` und `a + (b + c)` unterscheiden
//! sich im letzten Bit; über zehntausend Durchgänge wird daraus eine Ziffer, die
//! sich zwischen zwei Läufen ändert. Ein Balance-Ergebnis, das sich beim zweiten
//! Hinsehen ändert, ist keine Entscheidungsgrundlage.
//!
//! Deshalb: **Erst alle Ergebnisse einsammeln, nach Startwert sortieren, dann
//! summieren.** Die Reihenfolge hängt damit am Startwert und nicht am
//! Betriebssystem.
//!
//! # Warum parallel nur zwischen Durchgängen
//!
//! Ebenfalls 18.4: *"Parallel zwischen Durchgängen, nie innerhalb — ein Schritt
//! hängt vom vorherigen ab."* Ein Kampf ist eine Kette: Der Zustand nach Takt n
//! folgt aus dem nach Takt n−1. Zwei Kerne, die sich denselben Kampf teilen,
//! müssten ihn ständig abgleichen und wären langsamer als einer. Zwei Kerne, die
//! **verschiedene** Durchgänge rechnen, teilen dagegen nichts.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

use crate::apl::Apl;
use crate::data::{Boss, BOSS};
use crate::factors::{DamageFactors, NEUTRAL};
use crate::fight::{Fight, FightConfig};
use crate::gear::WornValues;
use crate::objective::{Outcome, WinCondition};
use crate::stichprobe::{gestaffelt, Befund};
use crate::time::Millis;

/// Einstellungen eines Balance-Laufs.
#[derive(Debug, Clone, Copy)]
pub struct BalanceConfig {
    /// Wie viele Durchgänge gerechnet werden.
    pub iterations: u32,
    /// Startwert des ersten Durchgangs. Jeder weitere zählt um eins hoch.
    ///
    /// Damit ist der Lauf vollständig durch diesen einen Wert bestimmt, und ein
    /// auffälliger Durchgang lässt sich einzeln nachrechnen.
    pub base_seed: u32,
    /// Über wie viele Stränge gerechnet wird. 1 heißt: gar nicht parallel.
    pub threads: usize,
    /// Kampfdauer. `None` heißt: der Überdruck-Timer des Bosses.
    pub duration_ms: Option<Millis>,
    /// Ob ein Durchgang mit dem Sieg endet.
    ///
    /// **Zwei Stellungen, zwei Fragen** — und sie zu verwechseln ist der Fehler,
    /// vor dem `STEP1-SPEC` § 4 ausdruecklich warnt:
    ///
    /// * `true` misst, **ob der Boss faellt**. Der Durchgang endet mit dem Sieg,
    ///   der Schaden ist damit abgeschnitten.
    /// * `false` misst, **ob Koennen sich lohnt**. Beide Rotationen rechnen
    ///   gleich lang, nur so sind ihre Schadenszahlen vergleichbar.
    ///
    /// Nur die zweite Zahl gehoert in das 10-%-Kriterium aus GAME-DESIGN 20.3.
    pub stop_on_kill: bool,
    /// Wie schwer verhinderter Schaden gegen angerichteten wiegt (Issue #472).
    ///
    /// **0.0 heisst: wie bisher, nur Schaden zaehlt.** Der Wert ist bewusst
    /// Teil der Konfiguration und nicht fest verdrahtet — welche Gewichtung
    /// richtig ist, haengt vom Boss ab und ist eine Balance-Entscheidung, keine
    /// technische. Gemessen an boss-1-2: `pin` verhindert 280 Schaden und
    /// kostet 3368 — dort lohnt es sich erst ab einem Gewicht um 12.
    pub prevented_weight: f64,
    /// Wieviel Schaden der Spieler OHNE Kontroll-Zauber einsteckt.
    ///
    /// Der Bezugspunkt fuer `prevented_weight`. Er steht in der Konfiguration,
    /// weil ein Vergleichslauf je Messung die Laufzeit einer Suche
    /// vervielfachte — sie braucht heute schon 1778 Messungen.
    pub damage_taken_baseline: f64,
    /// Die Faktoren der Schadensformel.
    pub factors: DamageFactors,
    /// Gegen welchen Boss gerechnet wird.
    pub boss: &'static Boss,
    /// Die getragenen Sonderwerte (GAME-DESIGN 6.7, Issue #177).
    ///
    /// **Der Weg, auf dem Ausruestung ueberhaupt messbar wird.** Bis zu diesem
    /// Issue las kein einziger Wirkort einen Ausruestungs-Wert; ein Vergleich
    /// „Satz gegen Affix" konnte deshalb nur eine Modellrechnung sein (#166).
    /// Mit diesem Feld bekommt der Laeufer etwas zu messen.
    pub worn: WornValues,
    /// Talent-Promille auf den **Ressourcen-Gewinn** (Issue #419).
    ///
    /// Zusammen mit `talent_dot_duration_permille` macht es AC3 messbar:
    /// Anders als der Schadensfaktor wirken diese beiden NICHT auf alle
    /// Rotationen gleich, also kann jeder Weg irgendwo die beste Wahl sein.
    pub talent_resource_permille: u32,
    /// Talent-Promille auf die **Laufzeit** der Brand-Effekte (Issue #419).
    pub talent_dot_duration_permille: u32,
    /// Talent-Promille auf den **Schaden** (Issue #417).
    ///
    /// **Bis #419 stand er hier fest auf 0**, mit der Begruendung, er wirke auf
    /// alle Rotationen gleich und verschoebe nur die Achse. Das stimmt, ist
    /// aber kein Grund, ihn UNERREICHBAR zu machen: Wer den globalen Baum
    /// gegen einen Schul-Weg abwaegt, braucht genau diesen Vergleich. Die
    /// Voreinstellung bleibt 0, also aendert sich ohne Zutun nichts.
    pub talent_permille: u32,
    /// Talent-Promille auf den **Aufbau** der Kontroll-Punkte (FIXATION).
    pub talent_control_gain_permille: u32,
    /// Talent-Promille auf die **Wertigkeit** eines Faeulnis-Stapels.
    pub talent_decay_effect_permille: u32,
    /// Talent-Promille auf den **Preis** der Kontroll-Wirkungen (FIXATION).
    pub talent_control_cost_permille: u32,
    /// Talent-Promille auf die **Staerke** der Kontroll-Wirkungen (FIXATION).
    pub talent_control_effect_permille: u32,
    /// Talent-Promille auf die **Zahl** gelegter Faeulnis-Stapel.
    pub talent_decay_stack_permille: u32,
    /// Talent-Promille auf die **Haltbarkeit** der Faeulnis-Stapel.
    pub talent_decay_duration_permille: u32,
    /// Talent-Promille auf die **Dicke** der Schutzschicht (COAGULATION).
    pub talent_shield_amount_permille: u32,
    /// Talent-Promille auf die **Haltbarkeit** der Schutzschicht.
    pub talent_shield_duration_permille: u32,
    /// Zusaetzliche **Schicht-Stapel** (COAGULATION) — eine Anzahl, kein Faktor.
    pub talent_shield_stacks_bonus: u32,
    /// Talent-Promille auf die **Verkuerzung** der Abklingzeiten (SUBLIMATION).
    pub talent_cooldown_reduction_permille: u32,
    /// Zusaetzliche **Ruecksetzer-Ladungen** — eine Anzahl, kein Faktor.
    pub talent_cooldown_charges_bonus: u32,
    /// Talent-Promille auf die **Schwelle** des Ruecksetzers (SUBLIMATION).
    pub talent_cooldown_threshold_permille: u32,
    /// Talent-Promille auf den Schaden der **Entlader** (GLOBAL, Issue #419).
    pub talent_spender_damage_permille: u32,
    /// Talent-Promille auf den Schaden der **Aufbau-Zauber** (GLOBAL, #419).
    pub talent_build_damage_permille: u32,
    /// Zusaetzliche Zersetzungs-Stapel (PUTREFACTION, #419) — eine Anzahl.
    pub talent_decay_cap_bonus: usize,
    /// Talent-Promille auf den Schutz je Zersetzungs-Stapel (#419).
    pub talent_decay_ward_permille: u32,
    /// Talent-Promille auf EFFLUX' Umwandlung (SUBLIMATION, #419).
    pub talent_convert_permille: u32,
}

impl Default for BalanceConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            base_seed: 0,
            threads: 1,
            duration_ms: None,
            stop_on_kill: true,
            // **0.0 haelt jede bestehende Messung gleich.** Wer die zweite
            // Groesse will, setzt sie ausdruecklich — sonst waeren alle
            // frueheren Balance-Zahlen still unvergleichbar geworden.
            prevented_weight: 0.0,
            damage_taken_baseline: 0.0,
            factors: NEUTRAL,
            // Ohne Talente — dieselbe Voreinstellung wie in `FightConfig`.
            //
            // **Alle fuenfzehn stehen auf 0, und das ist der Punkt** (#419):
            // Ein Bestandslauf misst weiter genau das, was er vorher mass. Wer
            // einen Weg wiegen will, setzt IHN — und nur ihn.
            talent_resource_permille: 0,
            talent_dot_duration_permille: 0,
            talent_permille: 0,
            talent_control_gain_permille: 0,
            talent_decay_effect_permille: 0,
            talent_control_cost_permille: 0,
            talent_control_effect_permille: 0,
            talent_decay_stack_permille: 0,
            talent_decay_duration_permille: 0,
            talent_shield_amount_permille: 0,
            talent_shield_duration_permille: 0,
            talent_shield_stacks_bonus: 0,
            talent_cooldown_reduction_permille: 0,
            talent_cooldown_charges_bonus: 0,
            talent_cooldown_threshold_permille: 0,
            talent_spender_damage_permille: 0,
            talent_build_damage_permille: 0,
            talent_decay_cap_bonus: 0,
            talent_decay_ward_permille: 0,
            talent_convert_permille: 0,
            boss: BOSS,
            worn: WornValues::none(),
        }
    }
}

/// Das Ergebnis eines einzelnen Durchgangs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Startwert dieses Durchgangs — die Sortierordnung.
    pub seed: u32,
    /// Gesamtschaden.
    pub damage: u32,
    /// Kampfzeit des Sieges, oder `None`.
    pub killed_at_ms: Option<Millis>,
    /// **Eingesteckter** Schaden (Issue #525).
    ///
    /// Bis hierher mass der Laeufer nur die eine Haelfte. Jede Aussage
    /// "zu schwach" war damit eine Aussage ueber die gemessene Haelfte —
    /// und jeder Verteidigungszauber unmessbar.
    pub damage_taken: u32,
    /// Davon von der Schutzschicht abgefangen (COAGULATION, Weg 1).
    ///
    /// **Kein Teil von `damage_taken`, sondern der Betrag DANEBEN.** Die
    /// Rechnung in `fight.rs` legt die Schicht VOR den Schlag: was sie
    /// auffaengt, kommt gar nicht erst an. Die Summe dessen, was auf den
    /// Spieler zukam, ist deshalb `damage_taken + absorbed`.
    pub absorbed: u32,
    /// Zurueckgeholtes Leben (COAGULATION, Weg 2).
    ///
    /// Steht neben den beiden anderen und nicht in ihrer Summe: Heilung
    /// mindert keinen Schlag, sie macht ihn nachtraeglich wett.
    pub healed: u32,
    /// Vom BOSS zurueckgedrehter Fortschritt (Baustein 5, #218).
    ///
    /// Steht getrennt von `healed`: Das eine ist die Heilung des Spielers,
    /// das andere eine Strafe fuer sein Versaeumnis. Ohne diese Zeile war
    /// die Groesse im Laeufer unerreichbar, obwohl `Fight` sie fuehrt.
    pub boss_healed: u32,
    /// Ob der Spieler gestorben ist.
    pub died: bool,
    /// **Wann** der Spieler gestorben ist, oder `None` (Issue #814).
    ///
    /// Das Gegenstueck zu [`Sample::killed_at_ms`]: die eine Zahl sagt, wann
    /// der Boss fiel, diese, wann der Spieler fiel.
    ///
    /// # Warum die Summe allein nicht reicht
    ///
    /// Eine Mechanik, die Schaden nur VERSCHIEBT statt ihn zu verhindern, ist
    /// fuer den Laeufer unsichtbar — am Ende steht dieselbe Summe. Das Issue
    /// fuehrt das Zahlenpaar, an dem es auffiel:
    ///
    /// ```text
    ///   40 s   ohne Ausgeber 4320   mit Ausgebern 4080   ->  240 gespart
    ///   60 s   ohne Ausgeber 5000   mit Ausgebern 5000   ->  NICHTS gespart
    /// ```
    ///
    /// Bei 60 Sekunden sagt die Summe nichts, obwohl die Mechanik wirkt.
    /// GAME-DESIGN 20.5 Punkt 4 gibt dem Laeufer deshalb die Zeit bis zum Tod
    /// als zweite Groesse — neben dem Durchsatz, nicht statt seiner.
    pub died_at_ms: Option<Millis>,
}

/// Mittelwert und Streuung einer Messreihe.
///
/// Die Felder heißen bewusst nach dem, was sie sind: `stddev` streut die
/// **Einzelwerte**, `stderr` streut den **Mittelwert**. Wer die beiden
/// verwechselt, hält einen Lauf für zehnmal genauer, als er ist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Statistics {
    /// Anzahl der Werte.
    pub count: usize,
    /// Arithmetisches Mittel.
    pub mean: f64,
    /// Standardabweichung der Einzelwerte (Stichprobe, mit n−1).
    pub stddev: f64,
    /// Standardfehler des Mittelwerts: `stddev / sqrt(n)`.
    pub stderr: f64,
    /// Halbe Breite des 95-%-Intervalls: `1,96 × stderr`.
    ///
    /// Das ist der Fehlerbalken, den GAME-DESIGN 18.5 verlangt. Der Faktor 1,96
    /// gilt für die Normalverteilung; bei wenigen Durchgängen ist er zu
    /// optimistisch, deshalb steht die Anzahl immer daneben.
    pub ci95: f64,
    /// Kleinster Wert.
    pub min: f64,
    /// Größter Wert.
    pub max: f64,
}

/// Halbe Breite des 95-%-Intervalls bei Normalverteilung.
const Z_95: f64 = 1.96;

impl Statistics {
    /// Rechnet Mittelwert und Streuung aus **bereits sortierten** Werten.
    ///
    /// Die Sortierung ist Voraussetzung und nicht Aufgabe dieser Funktion: Sie
    /// gehört zur Herkunft der Werte (Startwert-Reihenfolge), nicht zur Größe
    /// des Werts. Nach Größe zu sortieren wäre für die Summe stabiler, würde
    /// aber verschleiern, dass zwei Läufe verschiedene Durchgänge enthalten.
    #[must_use]
    pub fn from_sorted(values: &[f64]) -> Self {
        let count = values.len();
        if count == 0 {
            return Self {
                count: 0,
                mean: 0.0,
                stddev: 0.0,
                stderr: 0.0,
                ci95: 0.0,
                min: 0.0,
                max: 0.0,
            };
        }

        // Summe in der übergebenen Reihenfolge — deshalb muss sie feststehen.
        let mut sum = 0.0_f64;
        for value in values {
            sum += value;
        }
        let mean = sum / count as f64;

        let mut variance_sum = 0.0_f64;
        for value in values {
            let diff = value - mean;
            variance_sum += diff * diff;
        }
        // Stichproben-Varianz mit n−1. Bei einem einzigen Wert gibt es keine
        // Streuung, die man schätzen könnte — dann 0 statt Division durch null.
        let stddev = if count > 1 {
            (variance_sum / (count - 1) as f64).sqrt()
        } else {
            0.0
        };
        let stderr = stddev / (count as f64).sqrt();

        let mut min = values[0];
        let mut max = values[0];
        for &value in values {
            if value < min {
                min = value;
            }
            if value > max {
                max = value;
            }
        }

        Self {
            count,
            mean,
            stddev,
            stderr,
            ci95: Z_95 * stderr,
            min,
            max,
        }
    }
}

/// Das Ergebnis eines Balance-Laufs.
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceResult {
    /// Name der gerechneten Prioritätenliste.
    pub apl_name: String,
    /// Die Einstellungen, mit denen gerechnet wurde.
    pub iterations: u32,
    /// Startwert des ersten Durchgangs.
    pub base_seed: u32,
    /// Über wie viele Stränge gerechnet wurde.
    pub threads: usize,
    /// Schaden über alle Durchgänge.
    pub damage: Statistics,
    /// Wie oft der Boss gefallen ist.
    pub kills: usize,
    /// **Durchgekommener** Schaden über alle Durchgänge (Issue #525).
    ///
    /// Der Name der Ausgabe hiess bis #525 "Schaden eingesteckt" und meinte
    /// diese Zahl — das war irrefuehrend: Was die Schutzschicht abgefangen
    /// hat, steckt der Spieler NICHT ein. Die Summe steht daneben.
    pub damage_taken: Statistics,
    /// Was der Boss insgesamt geschickt hat: durchgekommen PLUS abgefangen.
    ///
    /// **Je Durchgang summiert, nicht aus den zwei Statistiken gerechnet.**
    /// Mittelwerte addieren sich, Streuungen nicht — `stddev(a+b)` ist nur
    /// dann `stddev(a)+stddev(b)`, wenn beide vollstaendig korreliert sind.
    /// Hier wird deshalb erst je Kampf addiert und dann ausgewertet.
    pub damage_incoming: Statistics,
    /// Von der Schutzschicht abgefangen (Weg 1).
    pub absorbed: Statistics,
    /// Zurueckgeholtes Leben (Weg 2).
    pub healed: Statistics,
    /// Vom BOSS zurueckgedrehter Fortschritt — **nicht** die Spielerheilung.
    pub boss_healed: Statistics,
    /// Wie oft der SPIELER gestorben ist.
    ///
    /// Bei einer Rotation, die ohne Verteidigung stirbt, ist das die
    /// aussagekräftigste Zahl des ganzen Laufs.
    pub deaths: usize,
    /// Wirkungen je Zauber, summiert über alle Durchgänge.
    ///
    /// Sie lagen in `RunResult.casts` schon vor und wurden weggeworfen. Ohne
    /// sie braucht jede Zauber-Messung eine isolierende Rotation.
    pub casts: Vec<u32>,
    /// Zeit bis zum Sieg, nur über die erfolgreichen Durchgänge.
    ///
    /// Getrennt vom Schaden, weil beide verschiedene Fragen beantworten: Der
    /// Schaden misst, ob Können sich lohnt; die Zeit misst, ob der Boss fällt.
    /// Wer sie vermengt, vergleicht einen abgebrochenen Lauf mit einem vollen.
    pub kill_time_ms: Statistics,
    /// Zeit bis zum **Tod des Spielers** — die zweite Groesse aus 20.5 Punkt 4.
    ///
    /// **Neben dem Durchsatz, nicht statt seiner.** Zaehlt nur die Durchgaenge,
    /// in denen der Spieler starb, wie [`BalanceResult::kill_time_ms`] nur die
    /// gewonnenen zaehlt. Bei `deaths == 0` steht hier eine leere Statistik —
    /// und das ist heute die haeufigste Lage: In der ganzen #607-Messreihe
    /// starb der Spieler kein einziges Mal.
    pub death_time_ms: Statistics,
    /// Alle Einzelergebnisse, nach Startwert sortiert.
    pub samples: Vec<Sample>,
}

impl BalanceResult {
    /// Anteil der Durchgänge, in denen der Boss gefallen ist.
    #[must_use]
    pub fn kill_rate(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.kills as f64 / self.samples.len() as f64
    }

    /// Das Ergebnis als JSON.
    ///
    /// GAME-DESIGN 18.5: *"Ausgabe (nur für Claude, Textausgabe/JSON genügt,
    /// keine Diagramme nötig)"*. Von Hand geschrieben statt über eine
    /// Serialisierungs-Kiste, damit die Bibliothek abhängigkeitsfrei bleibt —
    /// bei einer Handvoll Zahlen ist das kein Kunststück.
    ///
    /// Fließkommazahlen stehen in der kürzesten Form, die sich verlustfrei
    /// zurücklesen lässt (`{:?}`). Auf Stellen zu runden wäre hier falsch: Die
    /// letzte Stelle des Mittelwerts ist genau die, an der sich ein
    /// nichtdeterministischer Lauf verraten würde.
    #[must_use]
    pub fn to_json(&self) -> String {
        let stats = |s: &Statistics| {
            format!(
                "{{\"count\":{},\"mean\":{:?},\"stddev\":{:?},\"stderr\":{:?},\"ci95\":{:?},\"min\":{:?},\"max\":{:?}}}",
                s.count, s.mean, s.stddev, s.stderr, s.ci95, s.min, s.max
            )
        };
        format!(
            "{{\"apl\":{:?},\"iterations\":{},\"baseSeed\":{},\"threads\":{},\
             \"damage\":{},\"kills\":{},\"killRate\":{:?},\"killTimeMs\":{},\
             \"damageIncoming\":{},\"damageTaken\":{},\
             \"absorbed\":{},\"healed\":{},\"bossHealed\":{},\
             \"deaths\":{},\"casts\":{{{}}}}}",
            self.apl_name,
            self.iterations,
            self.base_seed,
            self.threads,
            stats(&self.damage),
            self.kills,
            self.kill_rate(),
            stats(&self.kill_time_ms),
            stats(&self.damage_incoming),
            stats(&self.damage_taken),
            stats(&self.absorbed),
            stats(&self.healed),
            stats(&self.boss_healed),
            self.deaths,
            // Nur die Zauber, die WIRKLICH gedrueckt wurden — eine Liste mit
            // sechzig Nullen verdeckt die drei Zahlen, auf die es ankommt.
            self.casts
                .iter()
                .enumerate()
                .filter(|(_, n)| **n > 0)
                .map(|(i, n)| format!("{:?}:{}", crate::data::SPELLS[i].id, n))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Der Lauf als **roher Ergebnisentwurf**, ohne jede Bewertung (Issue #913).
    ///
    /// # Warum diese Funktion und nicht ein zweites Schema
    ///
    /// Das Ergebnisschema aus [`crate::speicher`] ist die Abbildung genau
    /// dieses Ergebnissatzes. Ohne diese Funktion waere es ein Nachbau daneben:
    /// gebaut, gruen und von keinem Lauf je gefuellt.
    ///
    /// # Die Destrukturierung ohne `..` ist Absicht
    ///
    /// Sie bricht den Bau, wenn dieser Ergebnissatz ein Feld dazubekommt. Der
    /// dann faellige Satz lautet nicht "das Feld eintragen", sondern: **gehoert
    /// es ins rohe Ergebnis oder ist es eine Laufmetadatei?** Ohne den Bruch
    /// waere die Voreinstellung "wird stillschweigend nicht gespeichert" — und
    /// das faellt niemandem auf, weil der Speicher weiter plausibel aussieht.
    ///
    /// Vier Felder gehen bewusst **nicht** in den Vektor:
    ///
    /// | Feld | Warum nicht |
    /// | --- | --- |
    /// | `apl_name` | benennt den **Aufbau** — gehoert in den Schluessel, nicht ins Ergebnis |
    /// | `base_seed` | Teil des Laufvertrags, nicht des Gemessenen |
    /// | `threads` | reine Rechenaufteilung; sie darf das Ergebnis nicht beruehren |
    /// | `samples` | die Rohdaten **hinter** den Kennzahlen, nicht das Ergebnis selbst |
    ///
    /// `kills` und `deaths` sind `usize` und werden auf `u32` verengt. Bei
    /// [`u32::MAX`] Durchgaengen liefe der Simulator laenger als das Universum
    /// alt ist; die Saettigung ist trotzdem ausdruecklich hingeschrieben, weil
    /// ein stiller Ueberlauf schlimmer waere als eine falsche Zahl, die man
    /// sieht.
    #[must_use]
    pub fn ergebnisentwurf(&self) -> crate::speicher::Ergebnisentwurf {
        use crate::speicher::{Ergebnisentwurf, Siegzahl, Wirkungszahlen};

        let BalanceResult {
            apl_name: _,
            iterations,
            base_seed: _,
            threads: _,
            damage,
            kills,
            damage_taken,
            damage_incoming,
            absorbed,
            healed,
            boss_healed,
            deaths,
            casts,
            kill_time_ms,
            death_time_ms,
            samples: _,
        } = self;

        Ergebnisentwurf {
            schaden: Some(*damage),
            erlittener_schaden: Some(*damage_taken),
            eingehender_schaden: Some(*damage_incoming),
            absorption: Some(*absorbed),
            heilung: Some(*healed),
            boss_heilung: Some(*boss_healed),
            siegzeit_ms: Some(*kill_time_ms),
            todeszeit_ms: Some(*death_time_ms),
            durchgaenge: Some(*iterations),
            siege: Some(Siegzahl(u32::try_from(*kills).unwrap_or(u32::MAX))),
            tode: Some(u32::try_from(*deaths).unwrap_or(u32::MAX)),
            wirkungen: Some(Wirkungszahlen(casts.clone())),
        }
    }
}

/// Die Todeszeitpunkte aus einer Stichprobe — **nur die Durchgaenge mit Toten**.
///
/// # Warum das eine eigene Funktion ist
///
/// Sie stand als drei Zeilen mitten in `run()`, und dort war sie nicht
/// pruefbar: Ein gemischter Fall — manche Durchgaenge toedlich, andere nicht —
/// laesst sich im Simulator nicht herstellen. Der einzige Zufall im Kampf
/// (`fight.rs`, `roll_crit`) betrifft den SPIELER-Schaden; der erlittene
/// Schaden folgt der Boss-Zeitleiste und ist deterministisch. Alle Durchgaenge
/// sterben zur selben Millisekunde oder gar nicht.
///
/// Gemessen ueber drei Achsen (Dauer, Krit-Anteil, Schadenswert): kein
/// Uebergangsbereich. `kindle` allein stirbt immer, die beste gefundene Liste
/// gewinnt immer — auch ganz ohne Ausruestung.
///
/// **Als Funktion ist der Fall trotzdem pruefbar**, mit einer von Hand
/// gemischten Liste. Das ist der Unterschied zwischen „nicht messbar" und
/// „nicht pruefbar".
///
/// # Warum `filter_map` und nicht `map` mit 0
///
/// Ein ueberlebender Durchgang hat keinen Todeszeitpunkt — er hat nicht den
/// Zeitpunkt 0. Wer ihn als Null mitzaehlt, halbiert bei halber Sterberate den
/// Mittelwert und setzt das Minimum auf 0. Die Zahl saehe plausibel aus und
/// waere falsch, und zwar genau bei den grenzwertigen Rotationen, um die es
/// geht.
#[must_use]
pub fn todeszeiten(samples: &[Sample]) -> Vec<f64> {
    samples
        .iter()
        .filter_map(|s| s.died_at_ms.map(f64::from))
        .collect()
}

/// Wie viele Durchgaenge **gewonnen** wurden — mit der Umkehrung des Ziels.
///
/// # Warum das nicht `killed_at_ms.is_some()` ist
///
/// `killed_at_ms` sagt: der ausgeteilte Schaden hat die Lebenspunkte des Bosses
/// erreicht. Bei einem Boss mit `runaway` ist genau das die **Niederlage** —
/// der Ring ist auf null gelaufen, der Ueberdruck entsteht. Der Sieg ist dort
/// das Ueberstehen der Frist.
///
/// Das Spiel fuehrt diese Unterscheidung seit Issue #247 (`combat.ts`,
/// `checkDefeat`):
///
/// ```text
///              | Ring auf 0 | Zeit abgelaufen
/// gewoehnlich  | Sieg       | Niederlage
/// runaway      | Niederlage | Sieg
/// ```
///
/// Der Simulator fuehrte sie nicht. Gemessen am 2026-08-19, je 20 Durchgaenge
/// gegen `boss-2-3`:
///
/// ```text
/// apl/e939-schutz.apl       uebersteht die Frist   kills meldete  0
/// apl/e607-flashpoint.apl   erzeugt Ueberdruck     kills meldete 20
/// ```
///
/// In **beide** Richtungen das Gegenteil. Betroffen waren `kills`,
/// `kill_rate()`, die Siegzahl im Ergebnisspeicher und die Ausgabe von
/// `rota-sim run`.
///
/// # Warum hier und nicht in `fight.rs`
///
/// `killed_at` loest auch `stop_on_kill` aus und geht in die Trace-Hashes ein.
/// Dort die Bedeutung zu drehen hiesse, jede Paritaets-Aufzeichnung zu brechen
/// — und die Aufzeichnungen sind richtig: Beide Seiten erkennen dieselbe
/// Schwelle im selben Takt. Falsch war allein, sie als Sieg zu **deuten**.
///
/// # Der Spielertod dreht sich nicht mit
///
/// Die Umkehrung betrifft das ZIEL, nicht den eigenen Tod. Ein toter Spieler
/// hat auch dann verloren, wenn der Boss davonlaeuft — sonst gewaenne er durch
/// Sterben. `combat.ts` haelt denselben Satz an derselben Stelle fest.
#[must_use]
pub fn siege(samples: &[Sample], boss: &Boss) -> usize {
    let umgekehrt = boss.runaway.is_some();
    samples
        .iter()
        .filter(|s| {
            if umgekehrt {
                // **`s.died`, nicht `s.died_at_ms.is_none()`** — gemeldet von
                // Worker-3, und der Einwand traegt: `Sample` fuehrt den Tod in
                // ZWEI Feldern, und `objective::Outcome::of` liest `died`. Zwei
                // Lesarten im selben Auswertungspfad sichern einander nicht;
                // liefen sie je auseinander, ergaeben Vertrag und Simulator
                // verschiedene Siegzahlen, und der Prueffall auf ihre
                // Uebereinstimmung fiele zu Recht — ohne zu sagen, warum.
                //
                // Heute KOENNEN sie nicht auseinanderlaufen, beide kommen aus
                // `Fight::died_at`:
                //
                // ```text
                // player_dead()  ->  self.died_at.is_some()   fight.rs:5493
                // died_at_ms()   ->  self.died_at             fight.rs:3389
                // ```
                //
                // Das ist der Grund, warum es bisher nichts kostete — und genau
                // deshalb faellt es niemandem auf, wenn eine der beiden
                // Funktionen sich aendert. `pruefe_todesfelder` haelt die
                // Gleichheit deshalb als Zusicherung fest.
                s.killed_at_ms.is_none() && !s.died
            } else {
                s.killed_at_ms.is_some()
            }
        })
        .count()
}

/// Ob die beiden Todes-Felder eines Durchgangs dasselbe sagen.
///
/// # Warum es diese Funktion gibt
///
/// [`Sample`] fuehrt den Tod des Spielers doppelt: `died` als Wahrheitswert und
/// `died_at_ms` als Zeitpunkt. Beide entstehen aus `Fight::died_at`, sind heute
/// also zwangslaeufig gleich — aber **nichts haelt das fest**. Wer eine der
/// beiden Quellfunktionen aendert, laesst sie auseinanderlaufen, und die
/// Auswertung liest je nach Stelle die eine oder die andere:
///
/// ```text
/// balance::siege             liest  died
/// objective::Outcome::of     liest  died
/// frueher: balance::siege    las    died_at_ms.is_none()
/// ```
///
/// **Zwei Felder fuer dieselbe Sache sichern einander nicht.** Ein Prueffall
/// ueber echte Durchgaenge tut es — deshalb steht diese Funktion oeffentlich und
/// nicht als `debug_assert` im Erzeugungspfad: Ein `debug_assert` liefe in der
/// CI mit, aber nicht im Release-Messlauf, und dort entstehen die Zahlen.
///
/// Gefunden hat den Mangel Worker-3, an meinem eigenen Fix aus PR #1000.
#[must_use]
pub fn todesfelder_stimmen(samples: &[Sample]) -> bool {
    erster_todesfeld_widerspruch(samples).is_none()
}

/// Der erste Durchgang, dessen Todes-Felder einander widersprechen.
///
/// # Warum das nicht `bool` genuegt
///
/// Gemeldet von Worker-3: [`todesfelder_stimmen`] sagt, DASS etwas nicht stimmt,
/// aber nicht WO. Bei einem einzigen abweichenden Durchgang unter tausend
/// stuende in der Fehlermeldung „die Felder stimmen nicht" — und die Suche
/// danach faengt bei null an.
///
/// **Ein Waechter wird genau in dem Moment gelesen, in dem er anschlaegt.** Was
/// er dann nicht sagt, kostet Suchzeit, und zwar unter Zeitdruck.
///
/// Zurueck kommt der **Startwert** des Durchgangs, nicht sein Index: Der
/// Startwert bestimmt den Durchgang vollstaendig und laesst sich einzeln
/// nachfahren, ein Index in einer Stichprobe nicht.
#[must_use]
pub fn erster_todesfeld_widerspruch(samples: &[Sample]) -> Option<u32> {
    samples
        .iter()
        .find(|s| s.died != s.died_at_ms.is_some())
        .map(|s| s.seed)
}

/// Welchen Weg ein Lauf mit dieser Konfiguration nimmt (#933).
///
/// **Dieselbe Entscheidung wie in [`run`], nicht ihr Nachbau:** Beide rufen
/// [`weg_von`]. Die Anzeige braucht den Weg, bevor der erste Kampf läuft — für
/// die Kosten einer Auswertung —, und ein zweites Mal hingeschrieben liefe die
/// Bedingung beim nächsten Umbau auseinander.
#[must_use]
pub fn auswertungsweg(config: &BalanceConfig) -> crate::fortschritt::Weg {
    weg_von(config, true)
}

/// Der Weg einer Auswertung: ein Kampf, wenn die Reduktion erlaubt ist und
/// nirgends gewürfelt wird — sonst die feste Stichprobe.
fn weg_von(config: &BalanceConfig, reduktion: bool) -> crate::fortschritt::Weg {
    if reduktion
        && crate::durchgaenge::ein_durchgang_genuegt(crate::data::EFFECT_NODES, &config.worn)
    {
        crate::fortschritt::Weg::EinKampf
    } else {
        crate::fortschritt::Weg::Stichprobe
    }
}

/// Rechnet jeden Durchgang als eigenen Kampf, ohne die Reduktion aus #908.
///
/// **Für Durchsatzmessungen.** Ohne Würfel rechnet [`run`] einen Kampf und
/// vervielfacht ihn; eine Messung, die „Kämpfe je Sekunde" meldet, teilte dann
/// die angeforderte Zahl durch die Zeit eines einzigen. Genau so hat
/// `rota-sim bench` bis #933 gemessen — und `orch-verteilung` vor ihm, siehe
/// [`crate::kalibrierung::referenz_fights_per_s`].
///
/// Das Ergebnis ist bitgleich mit dem von [`run`]; `tests/fortschritt.rs`
/// hält das fest.
#[must_use]
pub fn run_vollstaendig(config: &BalanceConfig, apl: &Apl) -> BalanceResult {
    run_with_control(config, apl, false, None)
}

/// Wie viele Stränge [`run`] und seine Geschwister je gestartet haben.
static STRAENGE_GESTARTET: AtomicUsize = AtomicUsize::new(0);

/// Wie viele Stränge die parallelen Läufe dieses Prozesses bisher gestartet
/// haben (#912).
///
/// Eine Diagnosegröße: Sie wächst mit jedem Strang, der für einen
/// parallelen Abschnitt neu entsteht.
#[must_use]
pub fn straenge_gestartet() -> usize {
    STRAENGE_GESTARTET.load(Ordering::Relaxed)
}

/// Rechnet einen Balance-Lauf.
///
/// # Panics
///
/// Wenn ein Strang beim Rechnen abstürzt — dann ist das Ergebnis unvollständig,
/// und ein stillschweigend gekürzter Mittelwert wäre schlimmer als ein Abbruch.
#[must_use]
pub fn run(config: &BalanceConfig, apl: &Apl) -> BalanceResult {
    run_intern(config, apl, true)
}

/// Runs the same seed set with a runtime limit checked before each complete fight.
/// Scheduling settings never enter the combat configuration or result identity.
pub fn run_controlled(
    config: &BalanceConfig,
    apl: &Apl,
    control: &crate::drosselung::RuntimeControl,
) -> BalanceResult {
    run_with_control(config, apl, true, Some(control))
}

/// Wie [`run`], aber die Reduktion aus #908 lässt sich abschalten.
///
/// **Der Bezugslauf für AC3.** „Bitgleich wie vorher" ist keine Aussage über
/// eine gespeicherte Zahl, sondern über zwei Läufe derselben Konfiguration —
/// und den zweiten kann nur erzeugen, wer die Reduktion ausschalten kann. Ohne
/// diesen Schalter bliebe die Gleichheit behauptet.
pub(crate) fn run_intern(config: &BalanceConfig, apl: &Apl, reduktion: bool) -> BalanceResult {
    run_with_control(config, apl, reduktion, None)
}

fn run_with_control(
    config: &BalanceConfig,
    apl: &Apl,
    reduktion: bool,
    control: Option<&crate::drosselung::RuntimeControl>,
) -> BalanceResult {
    let threads = config.threads.max(1);
    let iterations = config.iterations;
    let weg = weg_von(config, reduktion);
    if reduktion {
        // Der Nenner der Trefferquote dieses Wegs (#934): geprüft wurde, ob
        // ein Kampf genügt — getroffen hat es, wenn `weg` EinKampf ist.
        crate::fortschritt::reduktionsversuch();
    }

    let mut casts_total = [0u32; crate::data::SPELL_COUNT];
    let mut samples: Vec<Sample> = if weg == crate::fortschritt::Weg::EinKampf && iterations > 1 {
        // **Ein Durchgang statt fuenfzig bis zweihundert** (Issue #908, AC1).
        //
        // Die Bedingung ist abgeleitet, nicht `purity == 0`: Sobald irgendetwas
        // wuerfelt, faellt sie von selbst auf `false` zurueck. Siehe
        // [`crate::durchgaenge`].
        let (einer, casts_eines, wuerfe) = run_range(config, apl, 0, 1, control);

        // **Die Voraussetzung wird nachgeprueft, nicht geglaubt** (Auflage 2).
        // Beschreibt ein Effekt seine Quelle falsch, reduziert die Ableitung
        // einen Lauf, der streut — und das Ergebnis saehe plausibel aus. Diese
        // Zeile ist die einzige Stelle, an der das auffaellt.
        assert_eq!(
            wuerfe, 0,
            "reduziert auf einen Durchgang, aber es wurde {wuerfe}-mal gezogen — \
             die abgeleitete Quellenmaske passt nicht zum Lauf"
        );

        // Der eine Kampf wird **vervielfacht statt hochgerechnet**: Die
        // Statistik bekommt dieselbe Zahl von Werten in derselben Reihenfolge
        // wie vorher und rechnet Zeile fuer Zeile denselben Weg. Eine
        // geschlossene Formel waere schneller und nicht mehr bitgleich.
        let muster = einer[0];
        for (ziel, n) in casts_total.iter_mut().zip(casts_eines.iter()) {
            *ziel = n * iterations;
        }
        (0..iterations)
            .map(|index| Sample {
                seed: config.base_seed.wrapping_add(index),
                ..muster
            })
            .collect()
    } else if threads == 1 || iterations < 2 {
        let (s, c, _) = run_range(config, apl, 0, iterations, control);
        casts_total = c;
        s
    } else {
        // Gleichmäßig aufteilen; der Rest verteilt sich auf die vorderen
        // Stränge. Welcher Strang welchen Durchgang rechnet, ist für das
        // Ergebnis gleichgültig — jeder Durchgang hängt allein an seinem
        // Startwert.
        let per_thread = iterations / threads as u32;
        let mut rest = iterations % threads as u32;
        let mut ranges = Vec::with_capacity(threads);
        let mut start = 0;
        for _ in 0..threads {
            let mut len = per_thread;
            if rest > 0 {
                len += 1;
                rest -= 1;
            }
            if len > 0 {
                ranges.push((start, len));
                start += len;
            }
        }

        // Die Arbeitsstränge erben das Zählwerk des Leitstrangs (#933), jeder
        // auf dem Platz seines Bereichs — sonst sähe ein mehrsträngiger Lauf
        // nur die Kämpfe des Leitstrangs, und das sind keine.
        let leitstrang = crate::fortschritt::strang();
        // Ebenso den Schalter der Kampfzähler (#934): Sie sind strang-lokal,
        // und ohne ihn zählte ein mehrsträngiger Lauf nichts. Ihre Zähler
        // kommen nach dem Ende in den Leitstrang zurück. Der Trace-Hash bleibt
        // einsträngig (siehe `diagnostics::vereinigen`).
        let kampfzaehler = crate::diagnostics::is_enabled();
        thread::scope(|scope| {
            let handles: Vec<_> = ranges
                .iter()
                .enumerate()
                .map(|(nummer, &(offset, len))| {
                    let arbeiter = leitstrang.as_ref().map(|s| s.arbeiter(nummer));
                    STRAENGE_GESTARTET.fetch_add(1, Ordering::Relaxed);
                    scope.spawn(move || {
                        let _anmeldung = arbeiter.map(crate::fortschritt::Strang::anmelden);
                        if kampfzaehler {
                            crate::diagnostics::enable();
                        }
                        let beginn = Instant::now();
                        let cpu_vorher = crate::engpass::strang_cpu_ns();
                        let teil = run_range(config, apl, offset, len, control);
                        let cpu = crate::engpass::strang_cpu_ns()
                            .zip(cpu_vorher)
                            .map(|(nachher, vorher)| nachher.saturating_sub(vorher));
                        let ende = Instant::now();
                        let zaehler = kampfzaehler.then(crate::diagnostics::snapshot);
                        (teil, (beginn, ende, cpu), zaehler)
                    })
                })
                .collect();
            let mut all = Vec::with_capacity(iterations as usize);
            let mut zeiten = Vec::with_capacity(handles.len());
            for handle in handles {
                let ((s, c, _), zeit, zaehler) = handle.join().expect("ein Strang ist abgestuerzt");
                for (i, n) in c.iter().enumerate() {
                    casts_total[i] += *n;
                }
                if let Some(z) = zaehler {
                    crate::diagnostics::vereinigen(&z);
                }
                zeiten.push(zeit);
                all.extend(s);
            }
            if let Some(s) = &leitstrang {
                abschnitt_eintragen(s, &zeiten, threads);
            }
            all
        })
    };

    // **Der entscheidende Schritt** (GAME-DESIGN 18.4): Erst sortieren, dann
    // zusammenzaehlen. Ohne ihn haengt die letzte Stelle des Mittelwerts daran,
    // welcher Strang zuerst fertig wurde.
    samples.sort_unstable_by_key(|sample| sample.seed);

    let damage_values: Vec<f64> = samples.iter().map(|s| f64::from(s.damage)).collect();
    let mut taken_values: Vec<f64> = samples.iter().map(|s| f64::from(s.damage_taken)).collect();
    let mut incoming_values: Vec<f64> = samples
        .iter()
        .map(|s| f64::from(s.damage_taken.saturating_add(s.absorbed)))
        .collect();
    let mut absorbed_values: Vec<f64> = samples.iter().map(|s| f64::from(s.absorbed)).collect();
    let mut healed_values: Vec<f64> = samples.iter().map(|s| f64::from(s.healed)).collect();
    let mut boss_healed_values: Vec<f64> =
        samples.iter().map(|s| f64::from(s.boss_healed)).collect();
    taken_values.sort_by(f64::total_cmp);
    // **Statistics::from_sorted verlangt sortierte Werte** — der Name sagt
    // es, und ohne diese zwei Zeilen waere der Median still falsch.
    incoming_values.sort_by(f64::total_cmp);
    absorbed_values.sort_by(f64::total_cmp);
    healed_values.sort_by(f64::total_cmp);
    boss_healed_values.sort_by(f64::total_cmp);
    let kill_values: Vec<f64> = samples
        .iter()
        .filter_map(|s| s.killed_at_ms.map(f64::from))
        .collect();
    // **Ohne `sort_by`, wie `kill_values` daneben.** `from_sorted` erwartet
    // die STARTWERT-Reihenfolge, nicht aufsteigende Groesse — sein
    // Doc-Kommentar verwirft das Sortieren nach Groesse ausdruecklich, weil
    // es verschleiern wuerde, dass zwei Laeufe verschiedene Durchgaenge
    // enthalten. `samples` kommt bereits in dieser Reihenfolge.
    let death_values = todeszeiten(&samples);

    let ergebnis = BalanceResult {
        apl_name: apl.name.clone(),
        iterations,
        base_seed: config.base_seed,
        threads,
        damage: Statistics::from_sorted(&damage_values),
        // **Nicht `kill_values.len()`** — das zaehlte bei einem Boss mit
        // umgekehrtem Ziel die Niederlagen. Siehe `siege`.
        kills: siege(&samples, config.boss),
        // `kill_time_ms` bleibt der Zeitpunkt, zu dem die Lebenspunkte fielen.
        // Bei einem umgekehrten Boss ist das die Zeit bis zum UEBERDRUCK, nicht
        // bis zum Sieg — die Groesse ist dieselbe geblieben, nur ihr Name passt
        // dort nicht. Sie umzubenennen waere eine Schnittstellenaenderung an
        // einer Stelle, die dieses Issue nicht anfasst.
        kill_time_ms: Statistics::from_sorted(&kill_values),
        death_time_ms: Statistics::from_sorted(&death_values),
        damage_taken: Statistics::from_sorted(&taken_values),
        damage_incoming: Statistics::from_sorted(&incoming_values),
        absorbed: Statistics::from_sorted(&absorbed_values),
        healed: Statistics::from_sorted(&healed_values),
        boss_healed: Statistics::from_sorted(&boss_healed_values),
        deaths: samples.iter().filter(|s| s.died).count(),
        casts: casts_total.to_vec(),
        samples,
    };

    // Eine Auswertung ist fertig (#933), wenn ihr Ergebnis vorliegt — also
    // NACH Sortieren und Statistik, nicht nach dem letzten Kampf. Gemessen am
    // 2026-10-28: Bei `bench --fights 3000000` stand die erste Meldung mit allen
    // Kämpfen 0,61 s vor dem Ende des Laufs. Gezählt im Strang, der die
    // Auswertung angestoßen hat, einmal je Lauf und nicht je Kampf.
    crate::fortschritt::kandidat(weg, u64::from(iterations));
    ergebnis
}

/// Trägt einen parallelen Abschnitt ins Zählwerk ein: je Strang des Laufs,
/// wie lange er gerechnet und wie lange er gewartet hat (#934).
///
/// `zeiten` hält je Paket, in der Reihenfolge der Bereiche, Beginn und Ende
/// an der Wanduhr und die CPU-Zeit des Strangs. Was daraus Arbeit und was
/// Warten ist, rechnet [`crate::engpass::abschnitt`]; fehlt die CPU-Zeit,
/// gilt die Dauer an der Wanduhr.
///
/// Zwei Zeitstempel und zwei Uhrabfragen je Paket, ein Eintrag je Strang und
/// Abschnitt — der heiße Pfad sieht davon nichts.
fn abschnitt_eintragen(
    leitstrang: &crate::fortschritt::Strang,
    zeiten: &[(Instant, Instant, Option<u64>)],
    threads: usize,
) {
    if zeiten.is_empty() {
        return;
    }
    let ns = |d: std::time::Duration| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX);
    let arbeit: Vec<u64> = zeiten
        .iter()
        .map(|&(beginn, ende, cpu)| cpu.unwrap_or_else(|| ns(ende - beginn)))
        .collect();
    for (nummer, zeit) in crate::engpass::abschnitt(&arbeit, threads)
        .into_iter()
        .enumerate()
    {
        leitstrang.abschnitt(nummer, zeit.arbeit_ns, zeit.warten_ns);
    }
}

/// Rechnet `len` Durchgänge ab `offset`, alle in einem Strang.
///
/// Legt **einen** Kampf an und setzt ihn zurück, statt je Durchgang einen neuen
/// zu bauen: So bleibt es bei einer Speicheranforderung für den ganzen Bereich.
fn run_range(
    config: &BalanceConfig,
    apl: &Apl,
    offset: u32,
    len: u32,
    control: Option<&crate::drosselung::RuntimeControl>,
) -> (Vec<Sample>, [u32; crate::data::SPELL_COUNT], u32) {
    let mut fight = Fight::new(kampf_konfiguration(
        config,
        config.base_seed.wrapping_add(offset),
    ));

    let mut out = Vec::with_capacity(len as usize);
    // **Die Cast-Zahlen lagen schon vor** (Issue #525): `RunResult.casts`
    // fuehrt sie je Zauber, der Laeufer hat sie bisher weggeworfen. Ohne sie
    // brauchte jede Zauber-Messung eine isolierende Rotation (#517, #523).
    let mut casts = [0u32; crate::data::SPELL_COUNT];
    // **Wie oft ueberhaupt gewuerfelt wurde** (Issue #908). Der Laeufer hat die
    // Zahl bisher weggeworfen; sie ist die Nachpruefung fuer den reduzierten
    // Pfad in [`run`] — nicht der Beleg, dass die Ableitung richtig gerechnet
    // hat, sondern der, dass ihre Voraussetzung wirklich galt.
    let mut wuerfe = 0u32;
    // Einmal je Bereich geholt, nicht je Kampf (#933): Im Kampf selbst bleibt
    // es bei einer Erhöhung auf der eigenen Cache-Zeile.
    let strang = crate::fortschritt::strang();
    for index in 0..len {
        let _permit = control.map(crate::drosselung::RuntimeControl::acquire);
        let seed = config.base_seed.wrapping_add(offset + index);
        fight.reset(seed);
        let result = fight.run_apl(apl);
        if let Some(s) = &strang {
            s.kampf();
        }
        wuerfe += result.random_draws;
        for (i, n) in result.casts.iter().enumerate() {
            casts[i] += *n;
        }
        out.push(probe_aus(&fight, seed, &result));
    }
    (out, casts, wuerfe)
}

/// Die Kampfkonfiguration eines Balance-Laufs für einen Startwert.
///
/// **Eine Stelle für beide Läufer.** [`run_range`] und [`run_gestaffelt`] bauen
/// ihren Kampf hier; stünde das Literal zweimal da, liefe ein neues Feld nur in
/// einen der beiden durch, und die Stichprobe mäße einen anderen Kampf als der
/// Balance-Lauf daneben.
fn kampf_konfiguration(config: &BalanceConfig, seed: u32) -> FightConfig {
    FightConfig {
        seed,
        duration_ms: config.duration_ms,
        stop_on_kill: config.stop_on_kill,
        factors: config.factors,
        boss: config.boss,
        worn: config.worn,
        // **Alle fuenfzehn kommen jetzt durch** (Issue #419, AC3).
        //
        // Bis hierher standen dreizehn davon an dieser Stelle fest auf 0, mit
        // einer richtigen Begruendung — der Bestandslauf misst ohne Talente —
        // und einer falschen Folge: Wer einen Weg wiegen wollte, KONNTE es
        // nicht. Die Voreinstellung in `BalanceConfig` ist weiterhin 0; kein
        // Bestandslauf verschiebt sich. Nur ist die Tuer jetzt da.
        //
        // **Dass ein Weg durchkommt, heisst nicht, dass er sich zeigt.** Ein
        // Schild-Weg gegen einen Boss, der nicht zuschlaegt, bewegt weiterhin
        // nichts — das ist der Befund aus #525 und der Grund, warum die Zahl
        // "durchgereicht" und die Zahl "messbar" getrennt gemeldet werden.
        talent_permille: config.talent_permille,
        talent_control_gain_permille: config.talent_control_gain_permille,
        talent_decay_effect_permille: config.talent_decay_effect_permille,
        talent_control_cost_permille: config.talent_control_cost_permille,
        talent_control_effect_permille: config.talent_control_effect_permille,
        talent_decay_stack_permille: config.talent_decay_stack_permille,
        talent_decay_duration_permille: config.talent_decay_duration_permille,
        talent_shield_amount_permille: config.talent_shield_amount_permille,
        talent_shield_duration_permille: config.talent_shield_duration_permille,
        talent_shield_stacks_bonus: config.talent_shield_stacks_bonus,
        talent_cooldown_reduction_permille: config.talent_cooldown_reduction_permille,
        talent_cooldown_charges_bonus: config.talent_cooldown_charges_bonus,
        talent_cooldown_threshold_permille: config.talent_cooldown_threshold_permille,
        talent_spender_damage_permille: config.talent_spender_damage_permille,
        talent_build_damage_permille: config.talent_build_damage_permille,
        talent_decay_cap_bonus: config.talent_decay_cap_bonus,
        talent_decay_ward_permille: config.talent_decay_ward_permille,
        talent_convert_permille: config.talent_convert_permille,
        // Der Balance-Laeufer misst OHNE Schluesseltalent — wie beim
        // Talent-Faktor daneben. Wer es messen will, setzt es hier.
        auto_reset: false,
        partial_control: false,
        shield_refund: false,
        decay_echo: false,
        // **Die beiden ANDEREN Talent-Groessen kommen durch** (Issue #419) —
        // und das ist der Unterschied, der AC3 ueberhaupt erst messbar macht.
        //
        // Ein Faktor auf den Ressourcen-Gewinn oder auf die Brand-Laufzeit
        // wirkt NICHT auf alle Rotationen gleich: Eine Rotation, die den
        // Entlader oft wirft, gewinnt an mehr Hitze; eine mit vielen Braenden
        // gewinnt an laengerer Laufzeit. Genau deshalb kann hier ein Weg die
        // beste Wahl sein und dort ein anderer — die Frage von GAME-DESIGN 4.6.
        //
        // Voreinstellung ist 0, also Faktor 1,0: Wer sie nicht setzt, rechnet
        // wie vorher, und keine bestehende Messung verschiebt sich.
        talent_resource_permille: config.talent_resource_permille,
        talent_dot_duration_permille: config.talent_dot_duration_permille,
        // Der Balance-Laeufer misst auf Stufe 1: Er vergleicht Rotationen, und
        // die Stufe wirkt auf beide gleich (Issue #248).
        player_level: 1,
        // Das Protokoll kostet Speicher und wird hier von niemandem gelesen.
        log_capacity: 0,
        // **Kein Schluesseltalent im Balance-Laeufer** (Issue #418), aus
        // demselben Grund wie bei den Talenten: Er vergleicht Rotationen,
        // nicht Ausbauten. Ein Verhalten, das eine Rotation umbaut, waere
        // hier ein zweiter beweglicher Teil.
        split_spender: false,
        // Der Balance-Lauf misst Werte-Aufbauten; der Sechser-Effekt gehoert in
        // den Prueflauf aus #118 und nicht hierher.
        ashen_regimen: false,
        whole_work: false,
        slow_rot: false,
        sealed_vessel: false,
        long_grip: false,
        perpetual_draft: false,
    }
}

/// Ein Durchgang als [`Sample`], gelesen aus dem Kampf direkt nach seinem Lauf.
fn probe_aus(fight: &Fight, seed: u32, result: &crate::fight::RunResult) -> Sample {
    Sample {
        seed,
        damage: result.damage,
        killed_at_ms: result.killed_at_ms,
        damage_taken: fight.damage_taken(),
        absorbed: fight.shield_absorbed(),
        healed: fight.healed(),
        boss_healed: fight.boss_healed(),
        died: fight.player_dead(),
        died_at_ms: fight.died_at_ms(),
    }
}

// ------------------------------------------------ Durchgangssteuerung (#915)

/// Welcher Weg die Zahl der Durchgänge bestimmt (Issue #915, AC3).
///
/// # Vorerst genau zwei Wege
///
/// | Weg | wann | Kämpfe |
/// | --- | --- | --- |
/// | [`Durchgangswahl::EinDurchgang`] | kein Strom zieht ([`crate::durchgaenge::ein_durchgang_genuegt`]) | genau einer |
/// | [`Durchgangswahl::Gestaffelt`] | sonst | bis die Schwelle sicher über- oder unterschritten ist, höchstens das Budget |
///
/// **Die Verteilungs-DP und der analytische Krit sind NICHT Teil der Wahl.**
/// Beide wären schneller, und beide setzen voraus, dass der Krit den
/// Kontrollfluss des Kampfes nicht berührt — bei einem Boss mit
/// Lebenspunkt-Phasen kann er es (#915, AC2). Woran die Durchgangssteuerung
/// das erkennt, ist noch nicht entschieden. Bis dahin läuft alles, was
/// würfelt, über die Stichprobe: langsamer, aber ohne eine Voraussetzung, die
/// niemand geprüft hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durchgangswahl {
    /// Kein Strom zieht: Ein Kampf ist die ganze Verteilung, sein Ausgang ist
    /// exakt.
    EinDurchgang,
    /// Es wird gewürfelt: die gestaffelte Stichprobe aus [`crate::stichprobe`].
    Gestaffelt,
}

impl Durchgangswahl {
    /// Die Wahl für diese Konfiguration — abgeleitet aus der Beschreibung und
    /// der getragenen Ausrüstung, nicht aus einer Bossliste.
    #[must_use]
    pub fn fuer(config: &BalanceConfig) -> Self {
        if crate::durchgaenge::ein_durchgang_genuegt(crate::data::EFFECT_NODES, &config.worn) {
            Self::EinDurchgang
        } else {
            Self::Gestaffelt
        }
    }
}

/// Mehr verschiedene Startwerte hat ein Lauf nicht: `BalanceConfig::base_seed`
/// ist ein `u32`, Versuch `i` rechnet mit `base_seed + i`.
///
/// Ein größeres Budget würde Kämpfe **wiederholen** und sie als unabhängige
/// Versuche zählen. Das ist keine feste Obergrenze der Stichprobe, sondern die
/// Grenze dessen, was dieser Läufer überhaupt unterscheiden kann.
pub const STARTWERTE: u64 = 1 << 32;

/// Die Vorgabe einer gestaffelten Stichprobe (Issue #915, AC3).
///
/// # Keine Voreinstellung, mit Absicht
///
/// Eine voreingestellte Schwelle wäre eine Balance-Entscheidung, die niemand
/// getroffen hat, und ein voreingestelltes Budget genau die feste Obergrenze,
/// die das Issue verbietet — „auch nicht als Voreinstellung". Deshalb gibt es
/// kein `Default`, und alle drei Felder muss der Aufrufer nennen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Staffelung {
    /// Die geforderte **Siegwahrscheinlichkeit**, echt zwischen 0 und 1.
    ///
    /// Gezählt wird der Sieg wie im Zielvertrag ([`crate::objective::Outcome::of`]),
    /// also mit umgekehrtem Ziel bei einem Boss mit `runaway`. Der Zielvertrag
    /// misst „Überleben" als Siegwahrscheinlichkeit; einen Mechanikerfolg gibt
    /// der Kampf heute nicht heraus.
    pub schwelle: f64,
    /// Die Irrtumswahrscheinlichkeit über **alle** Haltepunkte zusammen, echt
    /// zwischen 0 und 1. Die Aufteilung macht [`crate::stichprobe::gestaffelt`].
    pub alpha: f64,
    /// Höchstens so viele Kämpfe — eine Grenze der Kosten, keine
    /// Stichprobengröße. Wird sie erreicht, ohne dass eine Seite sicher ist,
    /// lautet der Befund `Ungeklaert`, nicht eine Zahl.
    pub budget: u64,
}

impl Staffelung {
    /// Prüft die Vorgabe, bevor ein Kampf startet.
    ///
    /// # Errors
    ///
    /// Der erste unzulässige Wert, als [`Staffelungsfehler`].
    pub fn pruefe(&self) -> Result<(), Staffelungsfehler> {
        // `!(0 < x < 1)` statt `x <= 0 || x >= 1`: Ein NaN fällt so ebenfalls
        // heraus — beide Vergleiche wären für NaN falsch und ließen es durch.
        if !(self.schwelle > 0.0 && self.schwelle < 1.0) {
            return Err(Staffelungsfehler::Schwelle(self.schwelle));
        }
        if !(self.alpha > 0.0 && self.alpha < 1.0) {
            return Err(Staffelungsfehler::Alpha(self.alpha));
        }
        if self.budget == 0 {
            return Err(Staffelungsfehler::BudgetNull);
        }
        if self.budget > STARTWERTE {
            return Err(Staffelungsfehler::BudgetUeberStartwerten(self.budget));
        }
        Ok(())
    }
}

/// Warum eine [`Staffelung`] nicht gefahren wird.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Staffelungsfehler {
    /// Die Schwelle liegt nicht echt zwischen 0 und 1.
    ///
    /// Auch 1 selbst nicht: Eine Siegwahrscheinlichkeit von genau 1 lässt sich
    /// mit einer Stichprobe nie belegen; ohne einen Fehler liefe die Folge bis
    /// zum Budget.
    Schwelle(f64),
    /// `alpha` liegt nicht echt zwischen 0 und 1.
    Alpha(f64),
    /// Ein Budget von null Kämpfen misst nichts.
    BudgetNull,
    /// Das Budget übersteigt [`STARTWERTE`].
    BudgetUeberStartwerten(u64),
}

impl std::fmt::Display for Staffelungsfehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schwelle(wert) => write!(
                f,
                "Die Schwelle muss echt zwischen 0 und 1 liegen, bekommen: {wert}. \
                 Eine Siegwahrscheinlichkeit von genau 1 belegt keine Stichprobe."
            ),
            Self::Alpha(wert) => write!(
                f,
                "alpha muss echt zwischen 0 und 1 liegen, bekommen: {wert}."
            ),
            Self::BudgetNull => write!(f, "Ein Budget von null Kaempfen misst nichts."),
            Self::BudgetUeberStartwerten(wert) => write!(
                f,
                "Das Budget {wert} uebersteigt die {STARTWERTE} verschiedenen \
                 Startwerte; mehr Versuche wiederholten Kaempfe."
            ),
        }
    }
}

/// Das Ergebnis eines Laufs mit Durchgangssteuerung.
///
/// **Die Zahl der Versuche steht immer da**, auch beim Befund `Erfuellt`: AC3
/// verlangt, dass ein Bericht nennt, wie viele Versuche die Sicherheit
/// tatsächlich gekostet hat — nicht nur, dass sie erreicht wurde.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestaffelterLauf {
    /// Welcher Weg gewählt wurde.
    pub wahl: Durchgangswahl,
    /// Die Vorgabe, mit der gerechnet wurde.
    pub staffelung: Staffelung,
    /// Der Befund zur Schwelle, mit seiner Versuchszahl.
    pub befund: Befund,
    /// Wie viele Kämpfe gerechnet wurden — **gezählt**, nicht aus dem Befund
    /// übernommen. Die beiden müssen übereinstimmen; ein Prüffall hält das fest.
    pub kaempfe: u64,
    /// Wie viele davon gewonnen wurden.
    pub siege: u64,
    /// Wie oft insgesamt gewürfelt wurde.
    ///
    /// Beim Weg [`Durchgangswahl::EinDurchgang`] ist das die Nachprüfung der
    /// Wahl (Auflage 2): Es muss null sein, sonst bricht der Lauf ab.
    pub wuerfe: u64,
    /// Der Schaden aller Kämpfe zusammen.
    ///
    /// Für den Mittelwert je Kampf — und als Abgleich: Er streut mit dem Krit,
    /// hängt also am Startwert. Ein Prüffall vergleicht ihn mit dem festen Lauf
    /// auf denselben Startwerten; Siegzahl und Befund allein fingen einen
    /// verschobenen Startwert nicht, weil fast alle Kämpfe gleich ausgehen.
    pub schaden: u64,
}

/// Fährt einen Balance-Lauf mit Durchgangssteuerung (Issue #915, AC3).
///
/// Die Wahl kommt aus [`Durchgangswahl::fuer`]. Beim Weg
/// [`Durchgangswahl::EinDurchgang`] wird **ein** Kampf gerechnet, und sein
/// Ausgang ist exakt: gewonnen heißt Siegwahrscheinlichkeit 1, also
/// `Erfuellt` nach einem Versuch, verloren heißt 0, also `Verfehlt`. Beim Weg
/// [`Durchgangswahl::Gestaffelt`] läuft [`gestaffelt`], ein Kampf je Versuch,
/// Versuch `i` mit dem Startwert `base_seed + i` — dieselben Startwerte wie
/// [`run`], also dieselben Kämpfe wie ein fester Lauf derselben Länge.
///
/// # Speicher
///
/// **Konstant, unabhängig vom Budget.** Es gibt einen Kampf, der je Versuch
/// zurückgesetzt wird, und vier Zähler; kein Einzelergebnis wird aufgehoben.
/// Das ist die Speichergrenze, die das Issue für Läufe ohne feste
/// Versuchszahl verlangt: Sie folgt aus dem Aufbau, nicht aus einer Zahl, die
/// jemand richtig wählen müsste.
///
/// # Ein Strang
///
/// `config.threads` wird nicht gelesen. Die Haltepunkte prüfen in fester
/// Reihenfolge, und jeder Versuch hängt allein an seinem Startwert — ein
/// paralleler Lauf wäre möglich, ist aber nicht gebaut.
///
/// # Errors
///
/// [`Staffelungsfehler`], wenn die Vorgabe unzulässig ist. Geprüft wird, bevor
/// ein Kampf startet.
///
/// # Panics
///
/// Wenn der Weg [`Durchgangswahl::EinDurchgang`] gewählt war und der Kampf
/// trotzdem gewürfelt hat — dann passt die abgeleitete Quellenmaske nicht zum
/// Lauf, wie in [`run`].
pub fn run_gestaffelt(
    config: &BalanceConfig,
    apl: &Apl,
    staffelung: Staffelung,
) -> Result<GestaffelterLauf, Staffelungsfehler> {
    staffelung.pruefe()?;
    let wahl = Durchgangswahl::fuer(config);
    // Dieselbe Frage wie im festen Lauf — genügt ein Kampf? —, also derselbe
    // Nenner der Trefferquote (#934).
    crate::fortschritt::reduktionsversuch();
    let bedingung = WinCondition::of(config.boss);
    let mut fight = Fight::new(kampf_konfiguration(config, config.base_seed));
    let mut kaempfe = 0u64;
    let mut siege = 0u64;
    let mut wuerfe = 0u64;
    let mut schaden = 0u64;
    let strang = crate::fortschritt::strang();
    let mut versuch = |i: u64| -> bool {
        // `pruefe` hat `budget <= STARTWERTE` sichergestellt, also passt jeder
        // Index in ein `u32`.
        let index = u32::try_from(i).expect("das Budget ist auf STARTWERTE begrenzt");
        let seed = config.base_seed.wrapping_add(index);
        fight.reset(seed);
        let result = fight.run_apl(apl);
        if let Some(s) = &strang {
            s.kampf();
        }
        let sieg = Outcome::of(&probe_aus(&fight, seed, &result), bedingung).is_win();
        kaempfe += 1;
        siege += u64::from(sieg);
        wuerfe += u64::from(result.random_draws);
        schaden += u64::from(result.damage);
        sieg
    };

    let befund = match wahl {
        Durchgangswahl::EinDurchgang => {
            let sieg = versuch(0);
            // Exakt, nicht geschätzt: Die Schwelle liegt echt zwischen 0 und 1,
            // also ist eine Siegwahrscheinlichkeit von 1 sicher darüber und
            // eine von 0 sicher darunter — nach einem Versuch.
            if sieg {
                Befund::Erfuellt { versuche: 1 }
            } else {
                Befund::Verfehlt { versuche: 1 }
            }
        }
        Durchgangswahl::Gestaffelt => gestaffelt(
            staffelung.schwelle,
            staffelung.alpha,
            staffelung.budget,
            &mut versuch,
        ),
    };

    if wahl == Durchgangswahl::EinDurchgang {
        // **Die Wahl wird nachgeprüft, nicht geglaubt** (Auflage 2), mit
        // derselben Begründung wie im reduzierten Pfad von `run`.
        assert_eq!(
            wuerfe, 0,
            "ein Durchgang war gewählt, aber es wurde {wuerfe}-mal gezogen — \
             die abgeleitete Quellenmaske passt nicht zum Lauf"
        );
    }

    crate::fortschritt::kandidat(
        match wahl {
            Durchgangswahl::EinDurchgang => crate::fortschritt::Weg::EinKampf,
            Durchgangswahl::Gestaffelt => crate::fortschritt::Weg::Gestaffelt,
        },
        kaempfe,
    );

    Ok(GestaffelterLauf {
        wahl,
        staffelung,
        befund,
        kaempfe,
        siege,
        wuerfe,
        schaden,
    })
}

#[cfg(test)]
mod tests {
    use super::{factor_value, run_intern, with_factor, BalanceConfig, FACTOR_NAMES};
    use crate::apl::{parse, AplName};
    use crate::factors::{DamageFactors, NEUTRAL};

    /// Die gute Rotation, wie sie im Repository liegt.
    const GOOD: &str = include_str!("../apl/step1-good.apl");

    fn gute_liste() -> crate::apl::Apl {
        parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
    }

    /// **AC3: bitgleich, nicht „ungefähr gleich"** (Issue #908).
    ///
    /// Verglichen werden zwei Läufe **derselben** Konfiguration — einmal mit
    /// Reduktion, einmal ohne. Eine gespeicherte Zahl täte es nicht: Sie würde
    /// mit jeder Balance-Änderung altern, und dann wüsste niemand mehr, ob die
    /// Abweichung von der Reduktion kommt oder von den Daten.
    #[test]
    fn ac3_reduzierter_lauf_ist_bitgleich_mit_dem_vollen() {
        let apl = gute_liste();
        let config = BalanceConfig {
            iterations: 20,
            base_seed: 4242,
            threads: 1,
            ..BalanceConfig::default()
        };

        // **Trägerzusicherung:** Ohne sie prüfte der Fall zwei identische
        // Läufe — beide unreduziert — und wäre grün, ohne die Reduktion je
        // berührt zu haben.
        assert!(
            crate::durchgaenge::ein_durchgang_genuegt(crate::data::EFFECT_NODES, &config.worn),
            "die Reduktion greift bei dieser Konfiguration gar nicht"
        );

        let reduziert = run_intern(&config, &apl, true);
        let voll = run_intern(&config, &apl, false);

        assert_eq!(
            reduziert.samples, voll.samples,
            "die Durchgänge selbst weichen ab"
        );
        assert_eq!(
            reduziert.damage.count, voll.damage.count,
            "der reduzierte Lauf muss dieselbe ANZAHL Werte ausweisen, nicht einen"
        );
        assert_eq!(
            reduziert.damage.mean.to_bits(),
            voll.damage.mean.to_bits(),
            "Mittelwert: {} gegen {}",
            reduziert.damage.mean,
            voll.damage.mean
        );
        assert_eq!(
            reduziert.damage.stddev.to_bits(),
            voll.damage.stddev.to_bits()
        );
        assert_eq!(
            reduziert.damage_taken.mean.to_bits(),
            voll.damage_taken.mean.to_bits()
        );
        assert_eq!(reduziert.kills, voll.kills);
        assert_eq!(reduziert.casts, voll.casts, "die Cast-Zahlen weichen ab");
    }

    /// **AC3, zweite Hälfte: der Faktor — gezählt, nicht gestoppt.**
    ///
    /// Das Issue verlangt „ein Fünfzigstel bis ein Zweihundertstel der
    /// Kampfzahl". Eine Zeitmessung wäre der naheliegende Beleg und der
    /// schlechtere: Auf dieser Maschine streuten zwei identische Läufe
    /// desselben Verfahrens um 2,01 % und 10,51 % (#896 AC1). Der Kampfzähler
    /// aus derselben Messumgebung ist exakt und vom Takt des Rechners
    /// unabhängig.
    ///
    /// `threads: 1` ist nicht Bequemlichkeit: Die Zähler sind **strang-lokal**,
    /// ein mehrsträngiger Lauf ließe den Hauptstrang nichts sehen.
    #[cfg(feature = "diagnostics")]
    #[test]
    fn ac3_der_reduzierte_lauf_rechnet_genau_einen_kampf() {
        use crate::diagnostics;

        let apl = gute_liste();
        let config = BalanceConfig {
            iterations: 50,
            base_seed: 7,
            threads: 1,
            ..BalanceConfig::default()
        };

        diagnostics::enable();

        diagnostics::reset();
        let ergebnis_reduziert = run_intern(&config, &apl, true);
        let kaempfe_reduziert = diagnostics::snapshot().fights();

        diagnostics::reset();
        let ergebnis_voll = run_intern(&config, &apl, false);
        let kaempfe_voll = diagnostics::snapshot().fights();

        diagnostics::disable();

        assert_eq!(
            kaempfe_voll, 50,
            "Trägerzusicherung: der unreduzierte Lauf rechnet wirklich 50 Kämpfe \
             — sonst verglichen wir zwei reduzierte Läufe"
        );
        assert_eq!(
            kaempfe_reduziert, 1,
            "der reduzierte Lauf darf genau einen Kampf rechnen"
        );

        // Und das Ergebnis ist dabei dasselbe geblieben — der Faktor ist nur
        // dann ein Gewinn, wenn er nichts kostet.
        assert_eq!(ergebnis_reduziert.samples, ergebnis_voll.samples);
        assert_eq!(
            ergebnis_reduziert.damage.mean.to_bits(),
            ergebnis_voll.damage.mean.to_bits()
        );
    }

    /// **AC3 spricht vom SUCHLAUF, nicht vom Balance-Lauf.**
    ///
    /// Dass der Suchlauf über [`run`] geht, steht in einer einzigen Zeile
    /// (`rotation_search::measure`). Aus ihr zu schließen, die Reduktion greife
    /// dort ebenfalls, wäre eine Ableitung — und genau diese Sorte Ableitung
    /// („der Baustein ist da, also wirkt er") hat sich in diesem Modul heute
    /// zweimal als falsch erwiesen. Also gemessen.
    #[cfg(feature = "diagnostics")]
    #[test]
    fn ac3_auch_der_suchpfad_rechnet_nur_einen_kampf() {
        use crate::diagnostics;
        use crate::rotation_search::{candidates_for, measure};

        let pool = candidates_for(0);
        assert!(
            !pool.is_empty(),
            "Trägerzusicherung: es gibt überhaupt Kandidaten zu messen"
        );
        let gewaehlt = &pool[..1];

        let mut streuend = crate::gear::WornValues::none();
        streuend.purity = 500;

        let basis = BalanceConfig {
            iterations: 50,
            base_seed: 11,
            threads: 1,
            ..BalanceConfig::default()
        };

        diagnostics::enable();

        diagnostics::reset();
        let _ = measure(&basis, "probe-ohne-krit", gewaehlt);
        let kaempfe_reduziert = diagnostics::snapshot().fights();

        diagnostics::reset();
        let _ = measure(
            &BalanceConfig {
                worn: streuend,
                ..basis
            },
            "probe-mit-krit",
            gewaehlt,
        );
        let kaempfe_streuend = diagnostics::snapshot().fights();

        diagnostics::disable();

        assert_eq!(
            kaempfe_streuend, 50,
            "Kontrollzeile: wo gewürfelt wird, rechnet auch der Suchpfad alle 50"
        );
        assert_eq!(
            kaempfe_reduziert, 1,
            "der Suchpfad muss die Reduktion mitbekommen, nicht nur der Balance-Lauf"
        );
    }

    /// **AC6: der Zähler steht auf null — und das belegt nur etwas, weil er
    /// vorher nachweislich hochgegangen ist.**
    ///
    /// Das Issue verlangt den Nachweis „über einen Zähler der Neubestimmungen"
    /// und schreibt bei AC5 die Regel dazu: „Ein Zähler, der immer null war,
    /// belegt nichts." Deshalb zwei Messungen statt einer — erst ein direkter
    /// Aufruf, der den Zähler bewegt, dann der Lauf, in dem er stillstehen soll.
    #[cfg(feature = "diagnostics")]
    #[test]
    fn ac6_ein_kampflauf_bestimmt_keine_auspraegung_neu() {
        use crate::diagnostics;

        let apl = gute_liste();
        let config = BalanceConfig {
            iterations: 5,
            base_seed: 3,
            threads: 1,
            ..BalanceConfig::default()
        };

        diagnostics::enable();

        diagnostics::reset();
        let _ = crate::gear::budget_neu_berechnen(50, 1);
        let nach_direktem_aufruf = diagnostics::snapshot().gear_budgets();

        // Und die andere Seite derselben Frage: Nachschlagen zählt NICHT.
        diagnostics::reset();
        let _ = crate::gear::budget_of(50, 1);
        let nach_nachschlagen = diagnostics::snapshot().gear_budgets();

        diagnostics::reset();
        let ergebnis = run_intern(&config, &apl, true);
        let im_kampflauf = diagnostics::snapshot().gear_budgets();
        let kaempfe = diagnostics::snapshot().fights();

        diagnostics::disable();

        assert_eq!(
            nach_direktem_aufruf, 1,
            "Trägerzusicherung: der Zähler bewegt sich überhaupt — ohne das wäre \
             die Null unten nur die Abwesenheit einer Messung"
        );
        assert_eq!(
            nach_nachschlagen, 0,
            "budget_of schlägt nach und rechnet nicht — das ist AC6 in einer Zeile"
        );
        assert!(
            kaempfe > 0,
            "Trägerzusicherung: der Lauf hat wirklich gekämpft ({kaempfe})"
        );
        assert!(
            ergebnis.damage.mean > 0.0,
            "Trägerzusicherung: und dabei Schaden gemacht"
        );
        assert_eq!(
            im_kampflauf, 0,
            "ein Balance-Lauf darf keine Ausrüstungs-Ausprägung neu bestimmen"
        );
    }

    /// Die Gegenprobe: Wo gewürfelt wird, darf **nicht** reduziert werden.
    ///
    /// Ohne diesen Fall belegte der obige nichts — eine Reduktion, die immer
    /// zuschlägt, bestünde ihn ebenfalls.
    #[test]
    fn ac1_bei_halber_krit_chance_wird_nicht_reduziert() {
        let apl = gute_liste();
        let mut worn = crate::gear::WornValues::none();
        worn.purity = 500;
        let config = BalanceConfig {
            iterations: 8,
            base_seed: 99,
            threads: 1,
            worn,
            ..BalanceConfig::default()
        };

        assert!(
            !crate::durchgaenge::ein_durchgang_genuegt(crate::data::EFFECT_NODES, &config.worn),
            "bei halber Chance darf die Ableitung nicht reduzieren"
        );

        let a = run_intern(&config, &apl, true);
        let b = run_intern(&config, &apl, false);
        assert_eq!(
            a.samples, b.samples,
            "der Schalter darf am unreduzierten Pfad nichts ändern"
        );
        assert!(
            a.samples.iter().any(|s| s.damage != a.samples[0].damage),
            "Trägerzusicherung: hier streuen die Durchgänge wirklich — sonst \
             wäre der Fall von der Reduktion gar nicht zu unterscheiden"
        );
    }

    /// **Die Wurfzahl der Stichprobe ist die des festen Läufers** (Issue #915).
    ///
    /// `GestaffelterLauf::wuerfe` steht in der Ausgabe von `rota-sim` als
    /// Beleg, dass gewürfelt wurde. Geprüft war nur, dass sie über null liegt;
    /// eine doppelt gezählte Zahl blieb grün. Hier steht sie gegen die Zählung
    /// von [`super::run_range`] auf denselben Startwerten: die Zahl, mit der
    /// der feste Lauf seine Reduktion nachprüft, aus einer eigenen Schleife.
    ///
    /// Drei Startwerte, einer davon am Überlauf des `u32`: Die Würfe hängen am
    /// Kampf, also am Startwert.
    #[test]
    fn die_wurfzahl_der_stichprobe_ist_die_des_festen_laeufers() {
        use super::{run_gestaffelt, run_range, Durchgangswahl, Staffelung};

        let apl = gute_liste();
        let mut worn = crate::gear::WornValues::none();
        worn.purity = 500;
        let vorgabe = Staffelung {
            schwelle: 0.9,
            alpha: 0.05,
            budget: 2048,
        };
        for base_seed in [0, 1000, u32::MAX - 50] {
            let config = BalanceConfig {
                base_seed,
                threads: 1,
                boss: crate::data::boss_by_id("boss-1-1").expect("den Boss gibt es"),
                worn,
                ..BalanceConfig::default()
            };
            let lauf = run_gestaffelt(&config, &apl, vorgabe).expect("gültige Vorgabe");
            assert_eq!(
                lauf.wahl,
                Durchgangswahl::Gestaffelt,
                "Startwert {base_seed}"
            );
            let laenge = u32::try_from(lauf.kaempfe).expect("passt");
            let (_, _, wuerfe_fest) = run_range(&config, &apl, 0, laenge, None);
            assert!(
                wuerfe_fest > 0,
                "Trägerzusicherung: ab Startwert {base_seed} wird gewürfelt"
            );
            assert_eq!(
                lauf.wuerfe,
                u64::from(wuerfe_fest),
                "Startwert {base_seed}: die Stichprobe meldet andere Würfe als \
                 der feste Läufer über dieselben {laenge} Kämpfe"
            );
            println!(
                "Startwert {base_seed}: {} Kämpfe, {} Würfe",
                lauf.kaempfe, lauf.wuerfe
            );
        }
    }

    /// **Waechter: Jeder Faktor der Struktur steht in der Liste** (Issue #159).
    ///
    /// # Warum das nicht ueber die Anzahl geht
    ///
    /// `FACTOR_NAMES.len() == 7` waere die naheliegende Pruefung und beim
    /// naechsten Zuwachs wieder blind: Sie verlangte nur, dass jemand die Zahl
    /// mitpflegt — dieselbe Handarbeit mit einem zusaetzlichen Schritt.
    ///
    /// # Wie er stattdessen arbeitet
    ///
    /// Der Vergleichswert unten wird als **vollstaendiges Struktur-Literal**
    /// gebaut. Der Uebersetzer erzwingt dabei eine Zeile je Feld: Wer der
    /// Schadensformel einen Faktor hinzufuegt und diese Stelle vergisst,
    /// bekommt `E0063: missing field` — keinen gruenen Lauf.
    ///
    /// Und dieselbe Struktur wird ueber [`FACTOR_NAMES`] aufgebaut. Fehlt dort
    /// ein Eintrag, bleibt sein Feld auf 1,0 stehen, waehrend das Literal 2,0
    /// traegt — die beiden laufen auseinander, und die Meldung nennt das Feld.
    ///
    /// Damit folgt die Pruefung aus der **Struktur**, nicht aus einer Zahl.
    #[test]
    fn jeder_faktor_der_formel_steht_in_der_liste() {
        // Exhaustiv — der Uebersetzer laesst kein Feld aus (E0063).
        let alle_gesetzt = DamageFactors {
            essence: 2.0,
            talents: 2.0,
            gear_effects: 2.0,
            buffs: 2.0,
            synergy: 2.0,
            resistance: 2.0,
            vulnerability: 2.0,
            crit: 2.0,
        };

        let ueber_die_liste = FACTOR_NAMES
            .iter()
            .fold(NEUTRAL, |f, (name, _)| with_factor(f, name, 2.0));

        assert_eq!(
            ueber_die_liste, alle_gesetzt,
            "Ein Faktor der Schadensformel fehlt in FACTOR_NAMES. Die Liste ist eine von \
             Hand gefuehrte Abschrift und kann nicht bemerken, dass ihr ein Eintrag fehlt — \
             genau so hat die Empfindlichkeits-Messung bis Issue #159 den Ausruestungs-Faktor \
             ausgelassen. Ergaenze ihn in FACTOR_NAMES, with_factor und factor_value."
        );
    }

    /// **Waechter, andere Richtung: Jeder Name der Liste wird angenommen.**
    ///
    /// `with_factor` und `factor_value` haben je einen `unreachable!`-Zweig.
    /// Ein Name, den sie nicht kennen, laesst den Lauf abstuerzen — mitten in
    /// einer Messung, die Minuten gedauert hat. Hier faellt er in
    /// Millisekunden auf.
    #[test]
    fn jeder_name_der_liste_wird_angenommen() {
        for (name, _) in FACTOR_NAMES {
            let gesetzt = with_factor(NEUTRAL, name, 3.0);
            assert_eq!(
                factor_value(&gesetzt, name),
                3.0,
                "\"{name}\" steht in FACTOR_NAMES, aber with_factor oder factor_value \
                 kennen ihn nicht"
            );
            // Und er muss auch wirklich etwas veraendern — ein Name, der auf
            // ein totes Feld zeigt, waere still wirkungslos.
            assert_ne!(gesetzt, NEUTRAL, "\"{name}\" veraendert die Faktoren nicht");
        }
    }

    /// Kein Name steht zweimal, keiner zeigt auf dasselbe Feld.
    ///
    /// Zwei Eintraege auf demselben Feld saehen im Bericht wie zwei Faktoren
    /// aus und waeren einer.
    #[test]
    fn kein_name_und_kein_feld_zweimal() {
        let mut namen: Vec<&str> = FACTOR_NAMES.iter().map(|(n, _)| *n).collect();
        let vorher = namen.len();
        namen.sort_unstable();
        namen.dedup();
        assert_eq!(
            namen.len(),
            vorher,
            "ein Name steht zweimal in FACTOR_NAMES"
        );

        for (a, _) in FACTOR_NAMES {
            for (b, _) in FACTOR_NAMES {
                if a == b {
                    continue;
                }
                let nur_a = with_factor(NEUTRAL, a, 5.0);
                assert_eq!(
                    factor_value(&nur_a, b),
                    1.0,
                    "\"{a}\" und \"{b}\" zeigen auf dasselbe Feld"
                );
            }
        }
    }

    use super::*;

    #[test]
    fn statistik_einer_bekannten_reihe() {
        // Von Hand nachgerechnet: Mittelwert 3, Stichproben-Standardabweichung
        // sqrt(2.5) = 1.5811..., Standardfehler davon durch sqrt(5).
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];
        let s = Statistics::from_sorted(&values);
        assert_eq!(s.count, 5);
        assert!((s.mean - 3.0).abs() < 1e-12);
        assert!((s.stddev - 2.5_f64.sqrt()).abs() < 1e-12);
        assert!((s.stderr - 2.5_f64.sqrt() / 5.0_f64.sqrt()).abs() < 1e-12);
        assert!((s.ci95 - Z_95 * s.stderr).abs() < 1e-12);
        assert!((s.min - 1.0).abs() < 1e-12);
        assert!((s.max - 5.0).abs() < 1e-12);
    }

    #[test]
    fn eine_reihe_ohne_streuung_hat_keinen_fehlerbalken() {
        // Genau der Fall von Schritt 1: kein Krit, also jeder Durchgang gleich.
        let values = [4240.0; 100];
        let s = Statistics::from_sorted(&values);
        assert!((s.mean - 4240.0).abs() < 1e-12);
        assert_eq!(s.stddev, 0.0);
        assert_eq!(s.stderr, 0.0);
        assert_eq!(s.ci95, 0.0);
    }

    #[test]
    fn ein_einzelner_wert_hat_keine_streuung() {
        // n−1 waere hier null. Ohne Sonderfall gaebe es eine Division durch null.
        let s = Statistics::from_sorted(&[7.0]);
        assert_eq!(s.count, 1);
        assert!((s.mean - 7.0).abs() < 1e-12);
        assert_eq!(s.stddev, 0.0);
        assert_eq!(s.stderr, 0.0);
    }

    #[test]
    fn eine_leere_reihe_stuerzt_nicht_ab() {
        let s = Statistics::from_sorted(&[]);
        assert_eq!(s.count, 0);
        assert_eq!(s.mean, 0.0);
    }

    #[test]
    fn die_reihenfolge_der_summe_ist_sichtbar() {
        // Der Grund fuer die Sortier-Regel aus 18.4, in drei Zahlen: Dieselben
        // Werte in anderer Reihenfolge ergeben einen anderen Mittelwert. Der
        // Unterschied liegt im letzten Bit — und genau der wandert bei einem
        // parallelen Lauf mit der Systemlast.
        let vorwaerts = [1e16, 1.0, -1e16];
        let rueckwaerts = [-1e16, 1e16, 1.0];
        let a = Statistics::from_sorted(&vorwaerts);
        let b = Statistics::from_sorted(&rueckwaerts);
        assert_ne!(
            a.mean, b.mean,
            "wenn diese beiden gleich sind, prueft der Test seine Voraussetzung nicht mehr"
        );
    }
}

/// Wie stark ein Faktor der Schadensformel auf den Schaden durchschlägt.
///
/// Das ist die Antwort auf die Frage aus GAME-DESIGN 18.5 — *„Wie viel Schaden
/// bringt ein Punkt CATALYSIS?"*. Gemessen wird die Empfindlichkeit gegenüber
/// dem **Faktor**, nicht gegenüber dem **Punkt**.
///
/// # Für GEGENSTÄNDE ist diese Umrechnung nicht offen — sie stellt sich nicht
///
/// Hier stand bis Issue #455, die Umrechnung Punkt → Faktor sei „im Design noch
/// nicht festgelegt". Für Gegenstände ist das falsch, und zwar nicht seit
/// gestern: GAME-DESIGN 483 sagt wörtlich *„Ein Fund trägt +3 % Tempo, nicht
/// +120 Tempo. Was er bringt, hängt nicht von der Spielerstufe ab."*
///
/// Auf Gegenständen gibt es überhaupt keine Punkte. Die Werte stehen in
/// **Promille**, und `worn.rs` beziehungsweise `src/gear/worn.ts` machen daraus
/// `1 + promille/1000` — das *ist* die Umrechnung, und sie ist gebaut und
/// geprüft.
///
/// # Für TALENTE gilt der Vorbehalt weiter
///
/// Ein Talentpunkt gibt eine Menge, die noch nicht feststeht — dort steht die
/// Umrechnung tatsächlich aus. Der ursprüngliche Verweis auf GAME-DESIGN 22
/// zeigte auf fehlende **Inhalte** (welche Talente, welche Gegenstände), nicht
/// auf eine offene Umrechnung; das eine als das andere zu lesen, hat in #455
/// beinahe einen Blocker erzeugt, den es nicht gibt.
#[derive(Debug, Clone, PartialEq)]
pub struct ScalingFactor {
    /// Feld der Schadensformel, das verändert wurde.
    pub factor: &'static str,
    /// Welcher Spielwert dahintersteht, soweit benannt.
    pub stat: &'static str,
    /// Um wie viel der Faktor angehoben wurde.
    pub delta: f64,
    /// Mittlerer Schaden ohne Anhebung.
    pub baseline_mean: f64,
    /// Mittlerer Schaden mit Anhebung.
    pub raised_mean: f64,
    /// Zuwachs, **paarweise** je Durchgang gemessen.
    ///
    /// Paarweise und nicht als Differenz zweier Mittelwerte: Beide Läufe
    /// benutzen dieselben Startwerte, also hebt sich der Zufall in der Differenz
    /// weg. Bei echtem Krit (ab Schritt 3) ist das der Unterschied zwischen
    /// einem brauchbaren und einem unbrauchbaren Skalierungswert.
    pub gain: Statistics,
    /// Zuwachs je Prozentpunkt auf dem Faktor.
    pub gain_per_percent: f64,
}

/// Die Faktoren der Schadensformel und die Werte, die dahinterstehen.
///
/// Für `talents`, `buffs`, `synergy` und `resistance` gibt es keinen einzelnen
/// Spielwert — sie sammeln, was Talentbaum, Verstärkungen, Synergie-Fenster und
/// Boss-Widerstand beitragen (GAME-DESIGN 6.3).
const FACTOR_NAMES: [(&str, &str); 8] = [
    ("essence", "ESSENCE"),
    ("talents", "(Talentbaum)"),
    // **Fehlte bis Issue #159.** Er kam mit #123 in die Formel; die Liste hier
    // ist eine von Hand gefuehrte Abschrift und konnte nicht bemerken, dass ihr
    // ein Eintrag fehlt. Damit liess die Empfindlichkeits-Messung ausgerechnet
    // den Faktor aus, ueber den Satz-Boni und legendaere Effekte einfliessen —
    // also das, was #118 vergleichen soll.
    ("gearEffects", "Satz-Boni und legendaere Effekte"),
    ("buffs", "(Verstaerkungen)"),
    ("synergy", "(Synergie)"),
    ("resistance", "(Boss-Widerstand)"),
    ("vulnerability", "(Boss-Verwundbarkeit)"),
    ("crit", "YIELD, nur wenn jeder Treffer kritisch waere"),
];

/// Setzt einen Faktor auf einen neuen Wert.
fn with_factor(mut factors: DamageFactors, name: &str, value: f64) -> DamageFactors {
    match name {
        "essence" => factors.essence = value,
        "talents" => factors.talents = value,
        "gearEffects" => factors.gear_effects = value,
        "buffs" => factors.buffs = value,
        "synergy" => factors.synergy = value,
        "resistance" => factors.resistance = value,
        "vulnerability" => factors.vulnerability = value,
        "crit" => factors.crit = value,
        _ => unreachable!("unbekannter Faktor: {name}"),
    }
    factors
}

/// Liest einen Faktor aus.
fn factor_value(factors: &DamageFactors, name: &str) -> f64 {
    match name {
        "essence" => factors.essence,
        "talents" => factors.talents,
        "gearEffects" => factors.gear_effects,
        "buffs" => factors.buffs,
        "synergy" => factors.synergy,
        "resistance" => factors.resistance,
        "vulnerability" => factors.vulnerability,
        "crit" => factors.crit,
        _ => unreachable!("unbekannter Faktor: {name}"),
    }
}

/// Misst die Empfindlichkeit gegenüber jedem Faktor der Schadensformel.
///
/// `delta` ist die Anhebung, üblich sind 0,01 für ein Prozent. Für jeden Faktor
/// laufen zwei Durchgangs-Reihen mit **denselben Startwerten**; verglichen wird
/// paarweise.
///
/// Der Vergleich läuft über die **volle Kampfzeit** (`stop_on_kill = false`),
/// unabhängig davon, was in `config` steht: Ein Lauf, der beim Sieg abbricht,
/// misst nicht mehr Schaden, sondern nur früheren Sieg — und der Zuwachs wäre
/// systematisch zu klein.
#[must_use]
pub fn scaling(config: &BalanceConfig, apl: &Apl, delta: f64) -> Vec<ScalingFactor> {
    let full = BalanceConfig {
        stop_on_kill: false,
        ..*config
    };
    let baseline = run(&full, apl);

    FACTOR_NAMES
        .iter()
        .map(|&(factor, stat)| {
            let raised_factors = with_factor(
                full.factors,
                factor,
                factor_value(&full.factors, factor) + delta,
            );
            let raised = run(
                &BalanceConfig {
                    factors: raised_factors,
                    ..full
                },
                apl,
            );

            // Paarweise: Die Ergebnisse sind nach Startwert sortiert, also
            // gehoeren gleiche Positionen zum selben Durchgang.
            let gains: Vec<f64> = baseline
                .samples
                .iter()
                .zip(&raised.samples)
                .map(|(a, b)| f64::from(b.damage) - f64::from(a.damage))
                .collect();
            let gain = Statistics::from_sorted(&gains);

            ScalingFactor {
                factor,
                stat,
                delta,
                baseline_mean: baseline.damage.mean,
                raised_mean: raised.damage.mean,
                gain_per_percent: if delta == 0.0 {
                    0.0
                } else {
                    gain.mean / (delta * 100.0)
                },
                gain,
            }
        })
        .collect()
}
