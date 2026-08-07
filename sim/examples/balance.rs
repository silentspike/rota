//! Der Balance-Läufer als Kommandozeilen-Werkzeug.
//!
//! ```text
//! cargo run --release --example balance
//! cargo run --release --example balance -- --iterations 10000 --threads 8
//! cargo run --release --example balance -- --json
//! cargo run --release --example balance -- --apl meine.apl
//! ```
//!
//! **Im Release laufen lassen.** Im Debug-Bau ist der Läufer rund achtzehnmal
//! langsamer; für einen Blick genügt das, für zehntausend Durchgänge nicht.
//!
//! Die Ausgabe ist Text oder JSON, keine Diagramme — sie liest Claude, nicht der
//! Auftraggeber (GAME-DESIGN 18.5).

use std::process::ExitCode;
use std::time::Instant;

use rota_sim::apl::{parse, Apl};
use rota_sim::balance::{run, scaling, BalanceConfig, BalanceResult, ScalingFactor, Statistics};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");
/// Die schlechte Rotation, wie sie im Repository liegt.
const BAD: &str = include_str!("../apl/step1-bad.apl");

struct Options {
    iterations: u32,
    threads: usize,
    base_seed: u32,
    json: bool,
    /// Ob nur die Nutzung ausgegeben werden soll (Issue #525).
    help: bool,
    /// Kennung des Bosses, gegen den gerechnet wird (Issue #525, AC3).
    boss_id: Option<String>,
    apl_path: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            iterations: 1000,
            threads: 4,
            base_seed: 0,
            json: false,
            help: false,
            boss_id: None,
            apl_path: None,
        }
    }
}

fn main() -> ExitCode {
    let options = match parse_arguments() {
        Ok(options) if options.help => {
            // Auf stdout, nicht stderr: Wer danach fragt, hat keinen Fehler
            // gemacht. Und der PFAD ist relativ zum Workspace-Root, nicht zu
            // `sim/` — daran bin ich bei #517 zweimal danebengegriffen.
            println!(
                "Aufruf: cargo run --release --example balance -- \
                 [--iterations N] [--threads N] [--seed N] [--json] [--apl DATEI]\n\n\
                 --apl  Pfad zur Rotationsdatei, RELATIV ZUM WORKSPACE-ROOT\n       \
                 (also sim/apl/DATEI.apl, nicht apl/DATEI.apl)\n\
                 --boss <id>  gegen welchen Boss gerechnet wird; ohne ihn gegen\n              \
                 den Standard, der KEINEN Schaden austeilt\n\
                 --json Ausgabe als JSON statt als Text\n\n\
                 Ausgegeben werden: Schaden, Siege, eingesteckter Schaden, Tode\n\
                 und die Wirkungen je Zauber."
            );
            return ExitCode::SUCCESS;
        }
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            eprintln!(
                "Aufruf: cargo run --release --example balance -- \
                 [--iterations N] [--threads N] [--seed N] [--json] [--apl DATEI]"
            );
            return ExitCode::FAILURE;
        }
    };

    // Der Boss, gegen den gerechnet wird (Issue #525, AC3). Ohne `--boss` der
    // Standard — und der teilt KEINEN Schaden aus, womit jeder
    // Verteidigungszauber unmessbar bleibt.
    let boss = match &options.boss_id {
        Some(id) => match rota_sim::data::boss_by_id(id) {
            Some(b) => b,
            None => {
                eprintln!("Unbekannter Boss: {id}");
                eprintln!(
                    "Moeglich sind: {}",
                    rota_sim::data::BOSSES
                        .iter()
                        .map(|b| b.id)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return ExitCode::FAILURE;
            }
        },
        None => BalanceConfig::default().boss,
    };

    let listen: Vec<(String, Apl)> = match &options.apl_path {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => match parse(path, &text) {
                Ok(apl) => vec![(path.clone(), apl)],
                Err(error) => {
                    eprintln!("{path} ist fehlerhaft:\n{error}");
                    return ExitCode::FAILURE;
                }
            },
            Err(error) => {
                eprintln!("{path} ist nicht lesbar: {error}");
                return ExitCode::FAILURE;
            }
        },
        None => vec![
            ("step1-good".to_owned(), load("step1-good", GOOD)),
            ("step1-bad".to_owned(), load("step1-bad", BAD)),
        ],
    };

    let mut ergebnisse = Vec::with_capacity(listen.len());
    for (name, apl) in &listen {
        let config = BalanceConfig {
            iterations: options.iterations,
            base_seed: options.base_seed,
            threads: options.threads,
            boss,
            ..BalanceConfig::default()
        };
        let start = Instant::now();
        let result = run(&config, apl);
        ergebnisse.push((name.clone(), result, start.elapsed()));
    }

    if options.json {
        println!("[");
        for (index, (_, result, dauer)) in ergebnisse.iter().enumerate() {
            let komma = if index + 1 < ergebnisse.len() {
                ","
            } else {
                ""
            };
            // Die Dauer steht neben dem Ergebnis, nicht darin: Sie ist die
            // einzige Zahl, die sich zwischen zwei gleichen Läufen ändern darf.
            let werte = scaling(
                &BalanceConfig {
                    iterations: options.iterations,
                    base_seed: options.base_seed,
                    threads: options.threads,
                    boss,
                    ..BalanceConfig::default()
                },
                &listen[index].1,
                0.01,
            );
            println!(
                "  {{\"elapsedMs\":{:?},\"result\":{},\"scaling\":{}}}{komma}",
                dauer.as_secs_f64() * 1000.0,
                result.to_json(),
                scaling_json(&werte)
            );
        }
        println!("]");
        return ExitCode::SUCCESS;
    }

    println!("{}", "=".repeat(72));
    println!("ROTA — Balance-Lauf");
    println!(
        "{} Durchgaenge je Liste, {} Straenge, Startwert ab {}",
        options.iterations, options.threads, options.base_seed
    );
    println!("{}", "=".repeat(72));
    println!();

    for (name, result, dauer) in &ergebnisse {
        report(name, result, dauer.as_secs_f64());
    }

    if listen.len() == 2 {
        vergleich(&listen[0].1, &listen[1].1, &options);
    }
    skalierung(&listen[0].1, &options);

    ExitCode::SUCCESS
}

fn load(name: &str, text: &str) -> Apl {
    parse(name, text).unwrap_or_else(|error| panic!("{name} ist fehlerhaft:\n{error}"))
}

fn parse_arguments() -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);

    while let Some(argument) = args.next() {
        let mut wert = |name: &str| -> Result<String, String> {
            args.next()
                .ok_or_else(|| format!("Nach {name} fehlt der Wert."))
        };
        match argument.as_str() {
            "--iterations" => {
                options.iterations = wert("--iterations")?
                    .parse()
                    .map_err(|_| "--iterations braucht eine ganze Zahl.".to_owned())?;
            }
            "--threads" => {
                options.threads = wert("--threads")?
                    .parse()
                    .map_err(|_| "--threads braucht eine ganze Zahl.".to_owned())?;
            }
            "--seed" => {
                options.base_seed = wert("--seed")?
                    .parse()
                    .map_err(|_| "--seed braucht eine ganze Zahl.".to_owned())?;
            }
            "--apl" => options.apl_path = Some(wert("--apl")?),
            // **Das Feld gab es schon** (BalanceConfig::boss), nur keinen Weg,
            // es zu setzen. Ohne diesen Schalter rechnet der Laeufer immer
            // gegen einen Boss, der NICHTS austeilt — und dann ist jeder
            // Verteidigungszauber unmessbar (Issue #525, AC3).
            "--boss" => options.boss_id = Some(wert("--boss")?),
            "--json" => options.json = true,
            // `--help` war bis Issue #525 eine "Unbekannte Angabe": Die
            // Nutzungszeile kam nur als FEHLER, und wer sie suchte, musste
            // erst danebengreifen. Das ist die Art Huerde, an der ein Werkzeug
            // ungenutzt bleibt.
            "--help" | "-h" => options.help = true,
            other => return Err(format!("Unbekannte Angabe: {other}")),
        }
    }

    if options.iterations == 0 {
        return Err("--iterations muss groesser als null sein.".to_owned());
    }
    Ok(options)
}

fn report(name: &str, result: &BalanceResult, sekunden: f64) {
    println!("  {name}");
    zeile("Durchgaenge", &format!("{}", result.samples.len()));
    zeile_stat("Schaden", &result.damage);
    zeile(
        "Siege",
        &format!(
            "{} von {} ({:.1} %)",
            result.kills,
            result.samples.len(),
            result.kill_rate() * 100.0
        ),
    );
    // Die zweite Haelfte des Spiels (Issue #525). Bis hierher mass der Laeufer
    // nur den ausgehenden Schaden — jeder Verteidigungszauber war damit
    // unmessbar, und COAGULATION ist die Schule, deren Zweck Ueberleben ist.
    zeile_stat("Schaden eingesteckt", &result.damage_taken);
    zeile(
        "Tode",
        &format!("{} von {}", result.deaths, result.samples.len()),
    );
    let gedrueckt: Vec<String> = result
        .casts
        .iter()
        .enumerate()
        .filter(|(_, n)| **n > 0)
        .map(|(i, n)| format!("{} {}", rota_sim::data::SPELLS[i].id, n))
        .collect();
    if !gedrueckt.is_empty() {
        zeile("Wirkungen", &gedrueckt.join(", "));
    }
    if result.kills > 0 {
        zeile_stat("Zeit bis Sieg (ms)", &result.kill_time_ms);
    }
    zeile(
        "Dauer",
        &format!(
            "{:.1} ms  ({:.0} Durchgaenge je Sekunde)",
            sekunden * 1000.0,
            result.samples.len() as f64 / sekunden
        ),
    );
    println!();
}

fn zeile(name: &str, wert: &str) {
    println!("      {name:<20} {wert}");
}

fn zeile_stat(name: &str, s: &Statistics) {
    // Der Fehlerbalken steht immer daneben, nie der Mittelwert allein: Ein
    // Mittelwert ohne Streuung sieht genauer aus, als er ist.
    println!(
        "      {name:<20} {:.1}  +/- {:.1}   (Streuung {:.1}, von {:.0} bis {:.0})",
        s.mean, s.ci95, s.stddev, s.min, s.max
    );
}

/// Stellt zwei Läufe gegenüber — der Prüfstein aus GAME-DESIGN 20.3.
///
/// **Zwei Zahlen, zwei Fragen, und sie sind nicht austauschbar.** Diese Funktion
/// rechnet beide selbst, statt eine davon zu zeigen und vor der anderen zu
/// warnen: Wer nur den Schaden bis zum Sieg sieht, hält 4240 gegen 4044 und
/// kommt auf +4,8 % — dabei hat die gute Rotation 19,5 s früher aufgehört zu
/// rechnen. Über die volle Kampfzeit sind es 5960 gegen 4044, also +47,4 %.
fn vergleich(gut_apl: &Apl, schlecht_apl: &Apl, options: &Options) {
    let voll = BalanceConfig {
        iterations: options.iterations,
        base_seed: options.base_seed,
        threads: options.threads,
        stop_on_kill: false,
        ..BalanceConfig::default()
    };
    let gut = run(&voll, gut_apl);
    let schlecht = run(&voll, schlecht_apl);

    println!("{}", "-".repeat(72));
    println!("VERGLEICH — zwei Zahlen, zwei Fragen");
    println!();
    println!("  1. FAELLT DER BOSS?   Lauf endet mit dem Sieg (stop_on_kill = true)");
    println!("     Diese Zahlen sind NICHT untereinander vergleichbar: Die bessere");
    println!("     Rotation hoert frueher auf und sammelt danach keinen Schaden mehr.");
    println!();
    println!("  2. LOHNT SICH KOENNEN?   Lauf ueber die volle Kampfzeit");
    println!("     Nur diese Zahl gehoert in das 10-%-Kriterium aus GAME-DESIGN 20.3.");
    println!("        {:<12} {:>8.0}", "gut", gut.damage.mean);
    println!("        {:<12} {:>8.0}", "schlecht", schlecht.damage.mean);

    if schlecht.damage.mean > 0.0 {
        let abstand = (gut.damage.mean - schlecht.damage.mean) / schlecht.damage.mean * 100.0;
        println!("        {:<12} {:>8.1} %", "Abstand", abstand);
        let urteil = if abstand >= 10.0 {
            "BESTANDEN: Koennen lohnt sich messbar."
        } else {
            "FEHLGESCHLAGEN: unter 10 % — GAME-DESIGN 20.3 verlangt Nachbesserung an den Mechaniken."
        };
        println!("        {urteil}");
    }
    println!("{}", "-".repeat(72));
}

/// Zeigt, wie stark jeder Faktor der Schadensformel durchschlägt.
fn skalierung(apl: &Apl, options: &Options) {
    let config = BalanceConfig {
        iterations: options.iterations,
        base_seed: options.base_seed,
        threads: options.threads,
        ..BalanceConfig::default()
    };
    let werte = scaling(&config, apl, 0.01);

    println!();
    println!("{}", "-".repeat(72));
    println!("SKALIERUNG — Zuwachs je Prozentpunkt auf dem Faktor");
    println!("  Gemessen ueber die volle Kampfzeit, paarweise mit gleichen Startwerten.");
    println!();
    println!(
        "  {:<12} {:<44} {:>10}",
        "Faktor", "Spielwert dahinter", "je 1 %"
    );
    for wert in &werte {
        println!(
            "  {:<12} {:<44} {:>10.1}",
            wert.factor, wert.stat, wert.gain_per_percent
        );
    }
    println!();
    println!("  Was hier NICHT steht: der Zuwachs je WERT-PUNKT. Die Umrechnung");
    println!("  Punkt -> Faktor ist im Design noch nicht festgelegt (GAME-DESIGN 22:");
    println!("  Talent-Inhalte vor E5, Gegenstands-Katalog vor E6). Sobald sie da ist,");
    println!("  ist der Wert je Punkt eine Multiplikation dieser Zahlen.");
    println!();
    println!("  PURITY und CATALYSIS fehlen ganz: Krit wird nirgends gewuerfelt und");
    println!("  Tempo ist auch im Spiel nicht umgesetzt (src/core/gcd.ts: \"Ab Epic E5");
    println!("  von CATALYSIS beeinflusst\").");
    println!("{}", "-".repeat(72));
}

/// Eine Zeile der JSON-Ausgabe für einen Skalierungswert.
fn scaling_json(werte: &[ScalingFactor]) -> String {
    let eintraege: Vec<String> = werte
        .iter()
        .map(|w| {
            format!(
                "{{\"factor\":{:?},\"stat\":{:?},\"delta\":{:?},\"gainMean\":{:?},\"gainCi95\":{:?},\"gainPerPercent\":{:?}}}",
                w.factor, w.stat, w.delta, w.gain.mean, w.gain.ci95, w.gain_per_percent
            )
        })
        .collect();
    format!("[{}]", eintraege.join(","))
}
