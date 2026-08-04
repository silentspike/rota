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

use std::thread;

use crate::apl::Apl;
use crate::data::{Boss, BOSS};
use crate::factors::{DamageFactors, NEUTRAL};
use crate::fight::{Fight, FightConfig};
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
    /// Die Faktoren der Schadensformel.
    pub factors: DamageFactors,
    /// Gegen welchen Boss gerechnet wird.
    pub boss: &'static Boss,
}

impl Default for BalanceConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            base_seed: 0,
            threads: 1,
            duration_ms: None,
            stop_on_kill: true,
            factors: NEUTRAL,
            boss: BOSS,
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
    /// Zeit bis zum Sieg, nur über die erfolgreichen Durchgänge.
    ///
    /// Getrennt vom Schaden, weil beide verschiedene Fragen beantworten: Der
    /// Schaden misst, ob Können sich lohnt; die Zeit misst, ob der Boss fällt.
    /// Wer sie vermengt, vergleicht einen abgebrochenen Lauf mit einem vollen.
    pub kill_time_ms: Statistics,
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
             \"damage\":{},\"kills\":{},\"killRate\":{:?},\"killTimeMs\":{}}}",
            self.apl_name,
            self.iterations,
            self.base_seed,
            self.threads,
            stats(&self.damage),
            self.kills,
            self.kill_rate(),
            stats(&self.kill_time_ms)
        )
    }
}

/// Rechnet einen Balance-Lauf.
///
/// # Panics
///
/// Wenn ein Strang beim Rechnen abstürzt — dann ist das Ergebnis unvollständig,
/// und ein stillschweigend gekürzter Mittelwert wäre schlimmer als ein Abbruch.
#[must_use]
pub fn run(config: &BalanceConfig, apl: &Apl) -> BalanceResult {
    let threads = config.threads.max(1);
    let iterations = config.iterations;

    let mut samples: Vec<Sample> = if threads == 1 || iterations < 2 {
        run_range(config, apl, 0, iterations)
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

        thread::scope(|scope| {
            let handles: Vec<_> = ranges
                .iter()
                .map(|&(offset, len)| scope.spawn(move || run_range(config, apl, offset, len)))
                .collect();
            let mut all = Vec::with_capacity(iterations as usize);
            for handle in handles {
                all.extend(handle.join().expect("ein Strang ist abgestuerzt"));
            }
            all
        })
    };

    // **Der entscheidende Schritt** (GAME-DESIGN 18.4): Erst sortieren, dann
    // zusammenzaehlen. Ohne ihn haengt die letzte Stelle des Mittelwerts daran,
    // welcher Strang zuerst fertig wurde.
    samples.sort_unstable_by_key(|sample| sample.seed);

    let damage_values: Vec<f64> = samples.iter().map(|s| f64::from(s.damage)).collect();
    let kill_values: Vec<f64> = samples
        .iter()
        .filter_map(|s| s.killed_at_ms.map(f64::from))
        .collect();

    BalanceResult {
        apl_name: apl.name.clone(),
        iterations,
        base_seed: config.base_seed,
        threads,
        damage: Statistics::from_sorted(&damage_values),
        kills: kill_values.len(),
        kill_time_ms: Statistics::from_sorted(&kill_values),
        samples,
    }
}

/// Rechnet `len` Durchgänge ab `offset`, alle in einem Strang.
///
/// Legt **einen** Kampf an und setzt ihn zurück, statt je Durchgang einen neuen
/// zu bauen: So bleibt es bei einer Speicheranforderung für den ganzen Bereich.
fn run_range(config: &BalanceConfig, apl: &Apl, offset: u32, len: u32) -> Vec<Sample> {
    let mut fight = Fight::new(FightConfig {
        seed: config.base_seed.wrapping_add(offset),
        duration_ms: config.duration_ms,
        stop_on_kill: config.stop_on_kill,
        factors: config.factors,
        boss: config.boss,
        // Das Protokoll kostet Speicher und wird hier von niemandem gelesen.
        log_capacity: 0,
    });

    let mut out = Vec::with_capacity(len as usize);
    for index in 0..len {
        let seed = config.base_seed.wrapping_add(offset + index);
        fight.reset(seed);
        let result = fight.run_apl(apl);
        out.push(Sample {
            seed,
            damage: result.damage,
            killed_at_ms: result.killed_at_ms,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{factor_value, with_factor, FACTOR_NAMES};
    use crate::factors::{DamageFactors, NEUTRAL};

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
/// bringt ein Punkt CATALYSIS?"* — soweit sie heute beantwortbar ist. Gemessen
/// wird die Empfindlichkeit gegenüber dem **Faktor**, nicht gegenüber dem
/// **Punkt**: Die Umrechnung Punkt → Faktor ist im Design noch nicht festgelegt
/// (GAME-DESIGN 22: Talent-Inhalte vor E5, Gegenstands-Katalog vor E6). Sobald
/// sie feststeht, ist der Wert je Punkt eine Multiplikation dieser Zahl.
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
const FACTOR_NAMES: [(&str, &str); 7] = [
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
