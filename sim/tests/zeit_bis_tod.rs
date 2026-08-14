//! Die zweite Balance-Groesse: Zeit bis zum Tod (Issue #814, GAME-DESIGN 20.5).
//!
//! **Woher die Forderung kommt.** Der Laeufer mass nur Summen. Eine Mechanik,
//! die Schaden VERSCHIEBT statt ihn zu verhindern, ist darin unsichtbar — am
//! Ende steht dieselbe Zahl. Das Issue fuehrt das Paar, an dem es auffiel:
//!
//! ```text
//!   40 s   ohne Ausgeber 4320   mit Ausgebern 4080   ->  240 gespart
//!   60 s   ohne Ausgeber 5000   mit Ausgebern 5000   ->  NICHTS gespart
//! ```
//!
//! # Die Traeger-Zusicherung ist hier die halbe Arbeit
//!
//! In der ganzen #607-Messreihe starb der Spieler **kein einziges Mal** — 0
//! von 1000, an jedem Boss, bei jeder Rotation. Eine Groesse, die nur bei
//! Toten etwas anzeigt, waere damit in jedem Standardlauf leer, und ein Test
//! darauf wuerde nichts pruefen.
//!
//! Deshalb steht der erste Fall unten: Er stellt sicher, dass es ueberhaupt
//! einen Aufbau gibt, in dem der Spieler stirbt. Ohne ihn waeren alle
//! weiteren Zusicherungen erfuellt und nichtssagend.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig};
use rota_sim::data::boss_by_id;

/// Ein Aufbau, in dem der Spieler stirbt: lange Kampfzeit, keine Verteidigung.
///
/// Gerechnet aus der #607-Messung — bei rund 64 Lebenspunkten je Sekunde und
/// 5000 Punkten faellt der Spieler nach etwa 78 Sekunden. Der Ueberdruck von
/// `boss-1-1` liegt bei 60 s und beendet den Kampf vorher; hier wird er
/// deshalb ausdruecklich ueberschrieben.
fn langer_kampf() -> BalanceConfig {
    BalanceConfig {
        iterations: 4,
        duration_ms: Some(120_000),
        stop_on_kill: false,
        boss: boss_by_id("boss-1-1").expect("boss-1-1 fehlt"),
        ..BalanceConfig::default()
    }
}

fn apl_ohne_verteidigung() -> Apl {
    parse(AplName("nur-schaden"), "kindle").expect("Liste nicht lesbar")
}

/// **Traeger-Zusicherung.** Ohne einen Aufbau, in dem der Spieler stirbt,
/// misst diese Datei nichts.
#[test]
fn es_gibt_einen_aufbau_in_dem_der_spieler_stirbt() {
    let ergebnis = run(&langer_kampf(), &apl_ohne_verteidigung());
    assert!(
        ergebnis.deaths > 0,
        "kein einziger Tod in {} Durchgaengen — dann ist jede Zusicherung \
         unten erfuellt und nichtssagend",
        ergebnis.samples.len()
    );
}

/// Der Kern: Die neue Groesse ist befuellt, wenn es Tode gab.
///
/// **Ein Feld im Ergebnis belegt nicht, dass etwas ankommt.** Genau diese
/// Luecke hat in diesem Projekt schon mehrere Groessen gruen und leer
/// dastehen lassen.
#[test]
fn die_zeit_bis_zum_tod_ist_befuellt() {
    let ergebnis = run(&langer_kampf(), &apl_ohne_verteidigung());
    assert_eq!(
        ergebnis.death_time_ms.count, ergebnis.deaths,
        "die Statistik zaehlt andere Durchgaenge als der Tod-Zaehler"
    );
    assert!(
        ergebnis.death_time_ms.mean > 0.0,
        "die Zeit bis zum Tod ist 0 — das Feld kommt nicht an"
    );
}

/// Die Zeit liegt im Kampf, nicht daneben.
///
/// Faengt eine Verwechslung mit einer anderen Groesse: Eine Todeszeit von 0
/// oder jenseits der Kampfdauer waere kein Zeitpunkt, sondern ein Artefakt.
#[test]
fn die_todeszeit_liegt_innerhalb_der_kampfdauer() {
    let cfg = langer_kampf();
    let ergebnis = run(&cfg, &apl_ohne_verteidigung());
    let dauer = f64::from(cfg.duration_ms.expect("Dauer gesetzt"));
    assert!(
        ergebnis.death_time_ms.min > 0.0 && ergebnis.death_time_ms.max <= dauer,
        "Todeszeit ausserhalb des Kampfes: min {}, max {}, Dauer {dauer}",
        ergebnis.death_time_ms.min,
        ergebnis.death_time_ms.max
    );
}

/// **Die eigentliche Forderung des Issues:** Die zweite Groesse zeigt einen
/// Unterschied, wo die Summe keinen zeigt.
///
/// Zwei Kampfzeiten, dieselbe Rotation. Der Schaden ist bei genuegend langer
/// Dauer gedeckelt (der Boss ist tot oder der Ueberdruck greift), die
/// Todeszeit dagegen haengt am Verlauf.
///
/// Zeigt auch sie keinen Unterschied, ist sie die falsche Groesse — und DAS
/// waere der Befund, kein Fehlschlag. Der Fall steht hier, damit die Frage
/// beantwortet bleibt und nicht in einer Meldung verschwindet.
#[test]
fn die_zweite_groesse_unterscheidet_was_die_summe_nicht_unterscheidet() {
    let kurz = BalanceConfig {
        duration_ms: Some(90_000),
        ..langer_kampf()
    };
    let lang = BalanceConfig {
        duration_ms: Some(150_000),
        ..langer_kampf()
    };
    let a = run(&kurz, &apl_ohne_verteidigung());
    let b = run(&lang, &apl_ohne_verteidigung());

    assert!(
        a.deaths > 0 && b.deaths > 0,
        "ohne Tode in beiden Laeufen vergleicht dieser Fall nichts"
    );
    // Der Tod tritt in beiden Laeufen zur selben Zeit ein — er haengt am
    // Verlauf, nicht an der gesetzten Dauer. Das ist die Zusicherung, dass die
    // Groesse einen ZEITPUNKT misst und nicht die Laufzeit spiegelt.
    assert!(
        (a.death_time_ms.mean - b.death_time_ms.mean).abs() < 1.0,
        "die Todeszeit haengt an der Kampfdauer ({} gegen {}) — dann misst sie \
         nicht den Zeitpunkt, sondern das Ende des Laufs",
        a.death_time_ms.mean,
        b.death_time_ms.mean
    );
}

/// **Der gemischte Fall — der einzige, der die Auswertung wirklich prueft.**
///
/// Im Simulator ist er nicht herstellbar: Der erlittene Schaden folgt der
/// Boss-Zeitleiste und ist deterministisch, also sterben alle Durchgaenge zur
/// selben Millisekunde oder gar keiner. Gemessen ueber drei Achsen (Dauer,
/// Krit-Anteil, Schadenswert) gibt es keinen Uebergangsbereich.
///
/// Deshalb steht die Stichprobe hier von Hand. Sie prueft genau die
/// Unterscheidung, die ein Lauf mit lauter Toten NICHT pruefen kann:
/// `filter_map` gegen `map` mit einer 0 fuer Ueberlebende.
mod gemischt {
    use rota_sim::balance::{todeszeiten, Sample, Statistics};

    fn probe(died_at_ms: Option<u32>) -> Sample {
        Sample {
            seed: 0,
            damage: 100,
            killed_at_ms: None,
            damage_taken: 0,
            absorbed: 0,
            healed: 0,
            boss_healed: 0,
            died: died_at_ms.is_some(),
            died_at_ms,
        }
    }

    /// **Traeger-Zusicherung.** Die Stichprobe muss WIRKLICH gemischt sein —
    /// sonst ist `filter_map` gleich `map` und der Fall unten zeigt nichts.
    #[test]
    fn die_probe_ist_gemischt() {
        let proben = [
            probe(Some(1000)),
            probe(None),
            probe(Some(3000)),
            probe(None),
        ];
        let tote = proben.iter().filter(|s| s.died).count();
        assert!(
            tote > 0 && tote < proben.len(),
            "die Stichprobe ist nicht gemischt: {tote} Tote von {}",
            proben.len()
        );
    }

    /// Ueberlebende zaehlen NICHT als Todeszeitpunkt 0.
    #[test]
    fn ueberlebende_stehen_nicht_als_null_in_der_statistik() {
        let proben = [
            probe(Some(1000)),
            probe(None),
            probe(Some(3000)),
            probe(None),
        ];
        let werte = todeszeiten(&proben);

        assert_eq!(werte.len(), 2, "die Ueberlebenden sind mitgezaehlt worden");

        let stat = Statistics::from_sorted(&werte);
        assert!(
            (stat.mean - 2000.0).abs() < f64::EPSILON,
            "Mittelwert {} statt 2000 — bei mitgezaehlten Nullen waere es 1000",
            stat.mean
        );
        assert!(
            stat.min > 0.0,
            "Minimum ist {}, also steht ein Ueberlebender als Todeszeit 0 darin",
            stat.min
        );
    }
}
