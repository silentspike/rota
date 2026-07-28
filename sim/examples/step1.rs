//! Rechnet Schritt 1 durch und gibt denselben Bericht aus wie die Referenz.
//!
//! ```text
//! cargo run --example step1
//! ```
//!
//! Der Vergleich mit `python3 tools/step1-check.py` ist damit ein Blick
//! nebeneinander statt eine Behauptung. Massgeblich ist der frische Lauf der
//! Referenz, nicht diese Ausgabe — sie steht daneben, damit ein Unterschied
//! sofort auffaellt.

use rota_sim::data::{KINDLE, PYRE, SMOULDER};
use rota_sim::{rotation, Fight, FightConfig, RunResult, BOSS};

/// Ein Durchgang bis zum Sieg.
fn run(apl: rota_sim::fight::Rotation) -> RunResult {
    Fight::new(FightConfig::default()).run(apl)
}

/// Ein Durchgang ueber die volle Kampfzeit, ohne Abbruch beim Sieg.
fn run_full(apl: rota_sim::fight::Rotation) -> RunResult {
    Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    })
    .run(apl)
}

/// Zahl mit Punkt als Tausendertrenner, wie in der Referenz-Ausgabe.
fn grouped(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

fn report(title: &str, r: &RunResult) {
    println!("  {title}");
    println!("    Schaden gesamt      {:>8}", grouped(r.damage));
    println!(
        "    Wirkungen           KINDLE {:>2}  SMOULDER {:>2}  PYRE {:>2}",
        r.casts[KINDLE], r.casts[SMOULDER], r.casts[PYRE]
    );
    println!(
        "    PYRE-Schaden        {:>8}",
        grouped(r.damage_by_spell[PYRE])
    );
    println!(
        "    SMOULDER-Ticks      {:>8}",
        grouped(r.damage_by_spell[SMOULDER])
    );
    println!("    verlorene DoT-Ticks {:>8}", r.lost_dot_ticks[SMOULDER]);
    println!("    verpuffte Hitze     {:>8}", r.resource_overflow);
    println!(
        "    Leerlauf            {:>8.1} s",
        f64::from(r.idle_ms) / 1000.0
    );
    match r.killed_at_ms {
        Some(t) => println!(
            "    Boss                besiegt bei {:.2} s ({:.2} s Reserve)",
            f64::from(t) / 1000.0,
            f64::from(r.remaining_ms) / 1000.0
        ),
        None => println!(
            "    Boss                UEBERLEBT ({}/{} = {:.1} %)",
            r.damage,
            BOSS.hp,
            f64::from(r.damage) / f64::from(BOSS.hp) * 100.0
        ),
    }
    println!("    Ziehungen           {:>8}", r.random_draws);
    println!();
}

fn main() {
    println!("{}", "=".repeat(66));
    println!("ROTA Schritt 1 — Simulator (Rust, ereignisgesteuert)");
    println!(
        "Testboss {} HP, Ueberdruck nach {} s, GCD {} ms",
        BOSS.hp,
        BOSS.overpressure_ms / 1000,
        rota_sim::GCD_MS
    );
    println!("{}", "=".repeat(66));
    println!();

    let good = run(rotation::good);
    report(
        "GUTE ROTATION   (SMOULDER spaet erneuern, PYRE ab 15 Hitze)",
        &good,
    );

    let bad = run(rotation::bad);
    report("SCHLECHTE ROT.  (SMOULDER zu frueh, PYRE sofort)", &bad);

    let good_full = run_full(rotation::good).damage;
    let bad_full = run_full(rotation::bad).damage;
    let only_dot = run_full(rotation::dot_error_only).damage;
    let only_pyre = run_full(rotation::pyre_error_only).damage;

    let diff = good_full - bad_full;
    let pct = f64::from(diff) / f64::from(bad_full) * 100.0;
    // Abstand zur guten Rotation in Prozent — negativ, weil jeder Fehler kostet.
    let rel = |v: u32| (f64::from(v) - f64::from(good_full)) / f64::from(good_full) * 100.0;

    println!("{}", "-".repeat(66));
    println!("VERGLEICH ueber die vollen 60 s (ohne Kill-Abbruch)");
    println!("  gut                     {:>8}", grouped(good_full));
    println!(
        "  nur DoT zu frueh        {:>8}   ({:+.1} %)",
        grouped(only_dot),
        rel(only_dot)
    );
    println!(
        "  nur PYRE zu frueh       {:>8}   ({:+.1} %)",
        grouped(only_pyre),
        rel(only_pyre)
    );
    println!(
        "  beide Fehler            {:>8}   ({:+.1} %)",
        grouped(bad_full),
        rel(bad_full)
    );
    println!(
        "  Differenz gut/schlecht  {:>8}  = {pct:.1} % mehr",
        grouped(diff)
    );
    println!("{}", "-".repeat(66));

    if pct >= 10.0 {
        println!("BESTANDEN: {pct:.1} % >= 10 % (Kriterium GAME-DESIGN 20.3)");
    } else {
        println!("FEHLGESCHLAGEN: {pct:.1} % — das Kriterium verlangt mindestens 10 %.");
    }
    if good.killed_at_ms.is_some() && bad.killed_at_ms.is_none() {
        println!("BESTANDEN: gute Rotation legt den Boss, schlechte nicht.");
    }
}
