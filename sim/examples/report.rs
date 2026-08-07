//! Die drei Auswertungen aus GAME-DESIGN 18.5, als Kommandozeilen-Werkzeug.
//!
//! ```text
//! cargo run --release --example report
//! cargo run --release --example report -- --json
//! ```
//!
//! Beendet sich mit 1, wenn der Erfolgs-Prüflauf keine Lösung findet — dann
//! gilt der Boss als **nicht auslieferbar** (GAME-DESIGN 12.9). Genau dafür
//! hängt er im nächtlichen Lauf.

use std::process::ExitCode;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::report::{
    achievement_run, feasibility, is_shippable, meaningless, spell_table, time_tiers,
    AchievementCheck, Feasibility, SpellContribution,
};
use rota_sim::{Fight, FightConfig, RunResult};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");

fn main() -> ExitCode {
    let json = std::env::args().any(|a| a == "--json");

    let apl = match parse(AplName("step1-good"), GOOD) {
        Ok(apl) => apl,
        Err(error) => {
            eprintln!("Die eingebaute Liste ist fehlerhaft:\n{error}");
            return ExitCode::FAILURE;
        }
    };

    let result = Fight::new(FightConfig::default()).run_apl(&apl);
    let table = spell_table(&result);
    let feas = feasibility(&apl, None);
    let checks = achievement_run(&apl, None);
    let shippable = is_shippable(&checks);

    if json {
        print_json(&apl, &result, &table, &feas, &checks, shippable);
    } else {
        print_text(&apl, &table, &feas, &checks, shippable);
    }

    if shippable {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_text(
    apl: &Apl,
    table: &[SpellContribution],
    feas: &Feasibility,
    checks: &[AchievementCheck],
    shippable: bool,
) {
    println!("{}", "=".repeat(74));
    println!("ROTA — Auswertungen (GAME-DESIGN 18.5)");
    println!("Rotation \"{}\", Boss \"{}\"", apl.name, feas.boss);
    println!("{}", "=".repeat(74));

    println!();
    println!("1. ZAUBER-VERGLEICH — deckt tote Zauber auf");
    println!(
        "   {:<10} {:>7} {:>9} {:>8} {:>12}",
        "Zauber", "Wirkt", "Schaden", "Anteil", "je Wirkung"
    );
    for row in table {
        println!(
            "   {:<10} {:>7} {:>9} {:>7.1} % {:>12.1}{}",
            row.id,
            row.casts,
            row.damage,
            row.share * 100.0,
            row.damage_per_cast,
            if row.is_dead() { "   TOT" } else { "" }
        );
    }
    let tote = table.iter().filter(|r| r.is_dead()).count();
    println!(
        "   {}",
        if tote == 0 {
            "Kein toter Zauber.".to_owned()
        } else {
            format!("{tote} Zauber werden nie gewirkt.")
        }
    );

    println!();
    println!("2. BOSS-MACHBARKEIT");
    println!("   Zeitlimit            {:>8} ms", feas.limit_ms);
    if let Some(t) = feas.kill_time_ms {
        println!("   besiegt bei          {t:>8} ms");
        println!(
            "   Reserve              {:>8} ms   ({:.1} % des Zeitlimits)",
            feas.reserve_ms,
            feas.reserve_share * 100.0
        );
    } else {
        println!("   NICHT besiegt");
        println!(
            "   abgetragen           {:>8}      ({:.1} % der Lebenspunkte)",
            feas.damage,
            feas.hp_share * 100.0
        );
    }

    println!();
    println!("3. ERFOLGS-PRUEFLAUF");
    if let Some(best) = feas.kill_time_ms {
        let t = time_tiers(best);
        println!(
            "   Zeitmarken aus der Bestzeit (GAME-DESIGN 21.6): Gold {} ms, Silber {} ms, Bronze {} ms",
            t.gold_ms, t.silver_ms, t.bronze_ms
        );
    }
    for check in checks {
        println!(
            "   {:<20} {}   {}",
            check.name,
            if check.solved { "geloest " } else { "OFFEN   " },
            check.evidence
        );
    }

    let ohne_wirkung = meaningless(checks);
    if !ohne_wirkung.is_empty() {
        println!();
        println!(
            "   HINWEIS: {} Erfolg(e) sind zwar loesbar, unterscheiden aber niemanden —",
            ohne_wirkung.len()
        );
        println!("   ihre Marke liegt ueber dem Zeitlimit und wird von jedem Sieg erfuellt:");
        for check in ohne_wirkung {
            println!("     {}", check.name);
        }
        println!("   Sie sperren die Auslieferung nicht (12.9 verlangt nur eine Loesung),");
        println!("   vergeben aber Punkte fuer nichts.");
    }

    println!();
    println!("{}", "-".repeat(74));
    if shippable {
        println!("AUSLIEFERBAR: Fuer jeden geprueften Erfolg existiert eine Loesung.");
    } else {
        println!("NICHT AUSLIEFERBAR: Mindestens ein Erfolg hat keine Loesung (GAME-DESIGN 12.9).");
    }
    println!("{}", "-".repeat(74));

    println!();
    println!("Was hier NOCH NICHT geprueft wird, weil es das Design noch nicht gibt");
    println!("(GAME-DESIGN 22): Schwierigkeitsstufen (vor E4), boss-eigene Erfolge und");
    println!("die 288 Zeitmarken (vor E10). Geprueft wird ein Boss auf einer Stufe mit");
    println!("den drei Marken, die sich aus der Bestzeit ableiten lassen.");
}

fn print_json(
    apl: &Apl,
    result: &RunResult,
    table: &[SpellContribution],
    feas: &Feasibility,
    checks: &[AchievementCheck],
    shippable: bool,
) {
    let spells: Vec<String> = table
        .iter()
        .map(|r| {
            format!(
                "{{\"id\":{:?},\"casts\":{},\"damage\":{},\"share\":{:?},\"damagePerCast\":{:?},\"dead\":{}}}",
                r.id, r.casts, r.damage, r.share, r.damage_per_cast, r.is_dead()
            )
        })
        .collect();

    let achievements: Vec<String> = checks
        .iter()
        .map(|c| {
            format!(
                "{{\"name\":{:?},\"solved\":{},\"meaningful\":{},\"evidence\":{:?}}}",
                c.name, c.solved, c.meaningful, c.evidence
            )
        })
        .collect();

    println!(
        "{{\"apl\":{:?},\"boss\":{:?},\"damage\":{},\
         \"spellTable\":[{}],\
         \"feasibility\":{{\"limitMs\":{},\"defeated\":{},\"killTimeMs\":{},\"reserveMs\":{},\"reserveShare\":{:?},\"hpShare\":{:?}}},\
         \"achievements\":[{}],\"shippable\":{}}}",
        apl.name,
        feas.boss,
        result.damage,
        spells.join(","),
        feas.limit_ms,
        feas.defeated,
        feas.kill_time_ms.map_or(-1_i64, i64::from),
        feas.reserve_ms,
        feas.reserve_share,
        feas.hp_share,
        achievements.join(","),
        shippable
    );
}
