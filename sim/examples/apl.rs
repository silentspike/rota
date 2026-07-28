//! Führt Prioritätenlisten aus und zeigt, was eine fehlerhafte meldet.
//!
//! ```text
//! cargo run --example apl                 # die beiden Listen aus STEP1-SPEC § 4
//! cargo run --example apl -- meine.apl    # eine eigene Datei
//! ```
//!
//! Ohne Argument läuft der vollständige Nachweis für Issue #55: Beide Listen
//! werden gerechnet und gegen ihre Sollwerte gehalten, danach werden fehlerhafte
//! Listen eingelesen, damit die Meldungen sichtbar sind.
//!
//! Der letzte Teil ist der wichtigere. Eine Liste, die sich vertippt hat und
//! trotzdem läuft, erzeugt ein Balance-Ergebnis, das niemand anzweifelt — die
//! Zahlen sehen plausibel aus, nur die Rotation war eine andere als gemeint.

use std::process::ExitCode;

use rota_sim::apl::parse;
use rota_sim::data::{KINDLE, PYRE, SMOULDER};
use rota_sim::{Fight, FightConfig, RunResult, BOSS};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");
/// Die schlechte Rotation, wie sie im Repository liegt.
const BAD: &str = include_str!("../apl/step1-bad.apl");

fn main() -> ExitCode {
    let argument = std::env::args().nth(1);

    if let Some(path) = argument {
        return match std::fs::read_to_string(&path) {
            Ok(text) => run_file(&path, &text),
            Err(error) => {
                eprintln!("{path} ist nicht lesbar: {error}");
                ExitCode::FAILURE
            }
        };
    }

    println!("{}", "=".repeat(70));
    println!("ROTA — Prioritätenlisten aus STEP1-SPEC § 4");
    println!("{}", "=".repeat(70));
    println!();

    let mut ok = true;
    ok &= show("apl/step1-good.apl", GOOD, 4240, Some(40_510));
    ok &= show("apl/step1-bad.apl", BAD, 4044, None);

    println!("{}", "-".repeat(70));
    println!("FEHLERHAFTE LISTEN — was der Einleser meldet");
    println!("{}", "-".repeat(70));
    println!();

    for (was, text) in [
        ("unbekannter Zauber", "smoulder\nfeuerball if heat > 3\n"),
        ("unbekannter Wert", "pyre if hitze >= 15\n"),
        ("fehlendes \"if\"", "pyre wenn heat >= 15\n"),
        ("Vergleich ohne Wert", "pyre if heat >=\n"),
        ("Zahl statt Bedingung", "pyre if 15\n"),
        ("unbekanntes Feld", "smoulder if dot.smoulder.laeuft\n"),
        (
            "unbekannte Einheit",
            "smoulder if dot.smoulder.remains < 5min\n",
        ),
        ("Liste ohne Regeln", "# nur ein Kommentar\n"),
    ] {
        match parse("beispiel", text) {
            Ok(_) => {
                println!("  {was}: KEIN FEHLER GEMELDET — das ist selbst der Fehler");
                ok = false;
            }
            Err(error) => {
                println!("  {was}:");
                for line in error.to_string().lines() {
                    println!("    {line}");
                }
                println!();
            }
        }
    }

    println!("{}", "-".repeat(70));
    if ok {
        println!("BESTANDEN: Beide Listen treffen ihre Sollwerte, jede fehlerhafte wird gemeldet.");
        ExitCode::SUCCESS
    } else {
        println!("FEHLGESCHLAGEN: siehe oben.");
        ExitCode::FAILURE
    }
}

/// Rechnet eine Liste durch und hält sie gegen ihren Sollwert.
fn show(name: &str, text: &str, expected_damage: u32, expected_kill: Option<u32>) -> bool {
    let apl = match parse(name, text) {
        Ok(apl) => apl,
        Err(error) => {
            println!("{name} ist fehlerhaft:\n{error}\n");
            return false;
        }
    };

    let result = Fight::new(FightConfig::default()).run_apl(&apl);
    println!("  {name}");
    for rule in &apl.rules {
        let bedingung = if rule.condition.is_always() {
            "immer".to_owned()
        } else {
            format!("{} Bedingung(en)", rule.condition.groups.len())
        };
        println!(
            "      Zeile {:>2}   {:<9} {bedingung}",
            rule.line,
            rota_sim::SPELLS[rule.spell].id
        );
    }
    report(&result);

    let damage_ok = result.damage == expected_damage;
    let kill_ok = result.killed_at_ms == expected_kill;
    if !damage_ok {
        println!(
            "      ABWEICHUNG: erwartet {expected_damage} Schaden, gerechnet {}",
            result.damage
        );
    }
    if !kill_ok {
        println!(
            "      ABWEICHUNG: erwartet Sieg bei {expected_kill:?}, gerechnet {:?}",
            result.killed_at_ms
        );
    }
    println!();
    damage_ok && kill_ok
}

/// Führt eine Datei aus, ohne Sollwert — für eigene Listen.
fn run_file(path: &str, text: &str) -> ExitCode {
    match parse(path, text) {
        Ok(apl) => {
            let result = Fight::new(FightConfig::default()).run_apl(&apl);
            println!("  {path}");
            report(&result);
            ExitCode::SUCCESS
        }
        Err(error) => {
            // Auf stderr, damit ein aufrufendes Skript den Fehler vom Ergebnis
            // trennen kann.
            eprintln!("{path} ist fehlerhaft:");
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn report(r: &RunResult) {
    println!("      Schaden gesamt      {:>8}", r.damage);
    println!(
        "      Wirkungen           KINDLE {:>2}  SMOULDER {:>2}  PYRE {:>2}",
        r.casts[KINDLE], r.casts[SMOULDER], r.casts[PYRE]
    );
    println!(
        "      verlorene DoT-Ticks {:>8}",
        r.lost_dot_ticks[SMOULDER]
    );
    println!("      Leerlauf            {:>8} ms", r.idle_ms);
    match r.killed_at_ms {
        Some(t) => println!("      Boss                besiegt bei {t} ms"),
        None => println!(
            "      Boss                ueberlebt ({}/{})",
            r.damage, BOSS.hp
        ),
    }
}
