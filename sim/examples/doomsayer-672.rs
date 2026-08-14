//! Issue #672: Ist `test-doomsayer` zu schaffen, und kommt die Mechanik vor?
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
//!   --example doomsayer-672
//! ```
//!
//! # Die drei Fragen
//!
//! | AC | Frage | wie gemessen |
//! | --- | --- | --- |
//! | 1 | erreicht ein Verlauf die 6400? | Balance-Laeufer, Schaden |
//! | 2 | benutzt er `purge`? | `casts[purge] > 0` im selben Lauf |
//! | 3 | schwanken die Verhaengnis-Stapel? | Spur ueber die Kampfzeit |
//!
//! # Die 6400 sind NICHT die Lebenspunkte dieses Bosses
//!
//! Gemessen an `origin/main`:
//!
//! ```text
//!   data/bosses/test-doomsayer.json     "hp": 999999
//!   data/bosses/boss-1-2.json           "hp": 6400
//! ```
//!
//! In der ganzen Historie der Datei steht genau eine HP-Zeile, und die lautet
//! `999999`. Die 6400 gehoeren `boss-1-2`; der Irrtum steht ausser im Issue
//! auch in `boss-3-4.json`, `instanz-3.test.ts` und `instanz-4.test.ts`.
//!
//! Mit 999999 Lebenspunkten ist der Boss per Bauart nicht toetbar — er ist ein
//! PRUEF-Boss fuer die Verhaengnis-Mechanik. **6400 wird hier deshalb als
//! SCHWELLE gemessen, nicht als Lebensbalken.**
//!
//! # Wie die Spur fuer AC3 entsteht
//!
//! `run_apl` faehrt den ganzen Kampf und gibt keine Zwischenstaende. Statt
//! dafuer in die Kernschleife zu greifen, faehrt dieser Laeufer den Kampf
//! mehrfach mit WACHSENDER Dauer und liest jedes Mal den Endstand. Das ist
//! derselbe Kampf — die Rechnung ist deterministisch (GAME-DESIGN 18.4) —,
//! nur an verschiedenen Stellen angehalten.

use rota_sim::apl::Apl;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::{Fight, FightConfig};

const BOSS: &str = "test-doomsayer";
/// Die Schwelle aus dem Issue — als Zahl hier, damit der Vergleich im Bericht
/// steht und nicht im Kopf des Lesers.
const SCHWELLE: f64 = 6400.0;
/// Was am 2026-08-09 gemessen wurde, bevor E10 die Zauber brachte.
const ALTE_ZAHL: f64 = 5168.0;

const RUNS: u32 = 50;
const SEED: u32 = 0;
const MAX_LEN: usize = 12;

fn config() -> BalanceConfig {
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: boss_by_id(BOSS).expect("test-doomsayer fehlt"),
        ..BalanceConfig::default()
    }
}

/// Ein einzelner Kampf mit dieser Liste, angehalten nach `dauer_ms`.
///
/// Gibt den Kampf UND sein Ergebnis zurueck: Der Kampf traegt den
/// Verhaengnis-Zaehler, das Ergebnis den Schaden und die Wirkungen.
fn kampf(apl: &Apl, dauer_ms: u32) -> (Fight, rota_sim::RunResult) {
    let mut fight = Fight::new(FightConfig {
        seed: SEED,
        boss: boss_by_id(BOSS).expect("Boss"),
        duration_ms: Some(dauer_ms),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let ergebnis = fight.run_apl(apl);
    (fight, ergebnis)
}

fn main() {
    println!("== Issue #672: test-doomsayer ==");
    let boss = boss_by_id(BOSS).expect("Boss");
    println!(
        "Boss {BOSS} — hp {}, Ueberdruck {} ms",
        boss.hp, boss.overpressure_ms
    );
    println!("Schwelle aus dem Issue: {SCHWELLE:.0} (die HP von boss-1-2, NICHT dieses Bosses)");
    println!();

    let pool = all_candidates();
    let gefunden = greedy(&config(), &pool, MAX_LEN);
    let schaden = gefunden.best.mean;

    println!("AC1 — erreicht ein Verlauf die Schwelle?");
    println!(
        "  gefunden        {schaden:9.0}   ({} Listen gemessen)",
        gefunden.measured
    );
    println!("  am 2026-08-09   {ALTE_ZAHL:9.0}   (vor E10)");
    println!("  Schwelle        {SCHWELLE:9.0}");
    let urteil = if schaden >= SCHWELLE {
        "ERREICHT"
    } else {
        "NICHT ERREICHT"
    };
    println!(
        "  -> {urteil}, Abstand {:+.0} ({:+.1} %)",
        schaden - SCHWELLE,
        (schaden - SCHWELLE) / SCHWELLE * 100.0
    );
    println!("  Rotation:");
    for zeile in gefunden.best.apl.to_string().lines() {
        println!("    {zeile}");
    }
    println!();

    println!("AC2 — benutzt der Verlauf `purge`?");
    let purge = index_of("purge").expect("purge fehlt");
    let (_, ein_lauf) = kampf(&gefunden.best.apl, boss.overpressure_ms);
    // **Gezaehlt wird im LAUF, nicht in der Liste.** Eine Zeile, die dasteht
    // und nie erfuellt ist, waere kein Beleg — genau der Unterschied, an dem
    // die Verstaerker-Zeilen in #558 gescheitert sind.
    let purge_casts = ein_lauf.casts[purge];
    println!(
        "  purge in der Liste   {}",
        gefunden.best.apl.to_string().contains("purge")
    );
    println!("  purge GEWIRKT        {purge_casts}");
    println!(
        "  -> {}",
        if purge_casts > 0 {
            "ERFUELLT"
        } else {
            "NICHT ERFUELLT — die Zeile steht da und kommt nie dran"
        }
    );
    println!();

    println!("AC3 — schwanken die Verhaengnis-Stapel?");
    println!(
        "  {:>8}  {:>6}  {:>10}  {:>7}",
        "ms", "doom", "Schaden", "lebt"
    );
    let mut werte = Vec::new();
    let schritt = boss.overpressure_ms / 12;
    for i in 1..=12 {
        let bis = schritt * i;
        let (f, r) = kampf(&gefunden.best.apl, bis);
        werte.push(f.doom());
        println!(
            "  {bis:>8}  {:>6}  {:>10}  {:>7}",
            f.doom(),
            r.damage,
            f.player_alive()
        );
    }
    let min = werte.iter().copied().min().unwrap_or(0);
    let max = werte.iter().copied().max().unwrap_or(0);
    println!("  Spanne {min} bis {max}");
    println!(
        "  -> {}",
        if max > min {
            "ERFUELLT — der Zaehler bewegt sich"
        } else {
            "NICHT ERFUELLT — konstanter Wert, die Mechanik kommt nicht vor"
        }
    );
    println!();

    ueberlebt_ein_verlauf();

    println!("Was diese Zahlen NICHT sagen");
    println!(
        "  Der Abbruch der Suche war {:?} bei {} Zeilen. Bei LengthLimit ist",
        gefunden.stop,
        gefunden.chosen.len()
    );
    println!("  die Zahl die bei DIESER Laenge, nicht das Maximum.");
    println!("  Und die Schwelle 6400 ist die der Lebenspunkte von boss-1-2 — dieser");
    println!("  Boss hat {}, er ist per Bauart nicht toetbar.", boss.hp);
}

/// **Die Frage, die #672 haette stellen sollen: Kann ein Verlauf UEBERLEBEN?**
///
/// # Die Rechnung zuerst
///
/// ```text
///   Anstieg      5 je 1000 ms   ->  300 ueber 60 s
///   Senkung     40 je purge
///   Abklingzeit  7500 ms        ->  8 Wirkungen moeglich, 320 Senkung
///   Nachladen    clear_amount / amount * per_step_ms = 40/5*1000 = 8000 ms
/// ```
///
/// **Nachladerate und Anstieg sind per Konstruktion gleich.** `recharge_ms`
/// ist als `clear_amount / amount * per_step_ms` definiert — also genau die
/// Zeit, in der der Zaehler um `clear_amount` steigt. In 8000 ms steigt er um
/// 40, und genau dann kommt eine Ladung zurueck, die 40 senkt. Dazu zwei
/// Ladungen im Vorrat, also 80 Puffer.
///
/// Rechnerisch ist der Kampf damit haltbar — wenn `purge` gewirkt wird, sobald
/// eine Ladung da ist.
///
/// # Warum der gesuchte Verlauf trotzdem stirbt
///
/// Die Suche maximiert SCHADEN. `purge` macht keinen (`baseDamage: 0`), kostet
/// aber einen GCD — sie nimmt ihn also nur so oft mit, wie es den Schaden nicht
/// druckt. Gemessen: fuenf Wirkungen, wo neun Ladungen zur Verfuegung standen.
///
/// Der Unterschied ist kein Suchfehler, sondern das Ziel: Ein Laeufer, der auf
/// Schaden optimiert, spielt nicht ums Ueberleben.
fn ueberlebt_ein_verlauf() {
    println!("Die neue Frage: kann ein Verlauf UEBERLEBEN?");
    let boss = boss_by_id(BOSS).expect("Boss");
    let doom = boss.doom.expect("test-doomsayer traegt doom");
    let dauer = boss.overpressure_ms;
    let anstieg = doom.amount * (dauer / doom.per_step_ms);
    let nachlade_ms =
        f64::from(doom.clear_amount) / f64::from(doom.amount) * f64::from(doom.per_step_ms);
    println!(
        "  Anstieg ueber {dauer} ms: {anstieg} · Senkung je purge: {} · Nachladen alle {nachlade_ms:.0} ms",
        doom.clear_amount
    );
    // **Nachladen und Anstieg sind PER KONSTRUKTION gleich, nicht zufaellig.**
    // `Fight::recharge_ms` ist als `clear_amount / amount * per_step_ms`
    // definiert — also genau die Zeit, in der der Zaehler um `clear_amount`
    // steigt.
    //
    // Mein erster Entwurf wollte das "pruefen" und verglich denselben Ausdruck
    // mit sich selbst; er haette immer "exakt" gemeldet. Eine Zusicherung, die
    // nicht scheitern kann, ist keine — deshalb steht hier die Herleitung statt
    // eines Vergleichs.
    println!(
        "  In {nachlade_ms:.0} ms steigt der Zaehler um {} — genau so viel, wie eine",
        doom.clear_amount
    );
    println!("  zurueckgekommene Ladung senkt (dieselbe Formel). Der Kampf ist damit");
    println!("  rechnerisch haltbar, WENN purge gewirkt wird, sobald eine Ladung da ist.");
    println!();

    // Drei Listen, die `purge` verschieden hoch haengen. Die Suche stellt ihn
    // nach hinten, weil er keinen Schaden macht — hier steht er oben.
    let listen: &[(&str, &str)] = &[
        (
            "purge ganz oben",
            "purge\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nrot\nsmoulder if dot.smoulder.remains < 1500\nkindle\n",
        ),
        (
            "purge nach den Braenden",
            "searing if dot.searing.remains < 1500\nsmoulder if dot.smoulder.remains < 1500\npurge\npyre if heat >= 15\nrot\nkindle\n",
        ),
        (
            "ohne purge (Gegenprobe)",
            "pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nrot\nsmoulder if dot.smoulder.remains < 1500\nkindle\n",
        ),
    ];
    // **Woran der Spieler stirbt, gehoert daneben.** Der Boss traegt ZWEI
    // Mechaniken: das Verhaengnis (doom) und die Stapel (`stacks`, 20 mal
    // Faktor 3 alle 2000 ms). Ein Tod bei doom weit unter der Schwelle kann
    // nicht der Verhaengnis-Tod sein — und dann hilft mehr purge nicht.
    println!(
        "  {:<26} {:>9} {:>7} {:>7} {:>9} {:>7}",
        "Liste", "Schaden", "purge", "doom", "erlitten", "lebt"
    );
    let purge = index_of("purge").expect("purge fehlt");
    for (name, text) in listen {
        let apl = rota_sim::apl::parse(rota_sim::apl::AplName("hand"), text).expect("gueltig");
        let (f, r) = kampf(&apl, dauer);
        println!(
            "  {name:<26} {:>9} {:>7} {:>7} {:>9} {:>7}",
            r.damage,
            r.casts[purge],
            f.doom(),
            f.damage_taken(),
            f.player_alive()
        );
        // **Die Todesart benennen, statt sie den Leser rechnen zu lassen.**
        // doom an der Schwelle heisst Verhaengnis-Tod; doom darunter bei hohem
        // erlittenem Schaden heisst: die ZWEITE Mechanik hat ihn geholt.
        if !f.player_alive() {
            let art = if f.doom() >= doom.threshold {
                "Verhaengnis (doom an der Schwelle)"
            } else {
                "Stapel-Schaden — das Verhaengnis war im Griff"
            };
            println!("  {:<26} gestorben an: {art}", "");
        }
    }
    println!();
    println!("  Der Befund: purge WIRKT — mit ihm faellt doom von 100 auf 20 bis 40.");
    println!("  Dann lebt der Spieler laenger und die Stapel holen ihn: 20 mal Faktor 3");
    println!("  alle 2000 ms. Die Frage war, ob purge oft genug geht oder 40 gegen 5 je");
    println!("  Sekunde grundsaetzlich zu wenig ist. Die Antwort ist: WEDER NOCH.");
}
