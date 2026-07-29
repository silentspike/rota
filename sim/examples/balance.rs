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
use rota_sim::balance::{run, BalanceConfig, BalanceResult, Statistics};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");
/// Die schlechte Rotation, wie sie im Repository liegt.
const BAD: &str = include_str!("../apl/step1-bad.apl");

struct Options {
    iterations: u32,
    threads: usize,
    base_seed: u32,
    json: bool,
    apl_path: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            iterations: 1000,
            threads: 4,
            base_seed: 0,
            json: false,
            apl_path: None,
        }
    }
}

fn main() -> ExitCode {
    let options = match parse_arguments() {
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
            println!(
                "  {{\"elapsedMs\":{:?},\"result\":{}}}{komma}",
                dauer.as_secs_f64() * 1000.0,
                result.to_json()
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

    if ergebnisse.len() == 2 {
        vergleich(&ergebnisse[0].1, &ergebnisse[1].1);
    }

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
            "--json" => options.json = true,
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
fn vergleich(gut: &BalanceResult, schlecht: &BalanceResult) {
    println!("{}", "-".repeat(72));
    println!("VERGLEICH");

    // **Zwei Zahlen, zwei Fragen.** Die Zeit bis zum Sieg misst, ob der Boss
    // faellt; der Schaden misst, ob Koennen sich lohnt. Wer den Schaden eines
    // abgebrochenen Laufs gegen den eines vollen haelt, vergleicht ungleich
    // lange Kaempfe — die bessere Rotation hoert frueher auf und sieht dadurch
    // schlechter aus.
    if !gut.samples.is_empty() {
        println!(
            "      Schaden bis zum Sieg     {:.0} gegen {:.0}",
            gut.damage.mean, schlecht.damage.mean
        );
        println!("         Achtung: Diese Zahlen sind nur vergleichbar, wenn beide Laeufe");
        println!("         gleich lang rechnen. Fuer das 10-%-Kriterium aus GAME-DESIGN 20.3");
        println!("         zaehlt der Lauf ueber die volle Kampfzeit (stop_on_kill = false),");
        println!("         dort 5960 gegen 4044 = +47,4 %.");
    }
    println!(
        "      Siege                    {:.1} % gegen {:.1} %",
        gut.kill_rate() * 100.0,
        schlecht.kill_rate() * 100.0
    );
    println!("{}", "-".repeat(72));
}
