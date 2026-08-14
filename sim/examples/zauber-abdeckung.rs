//! Wächter über die Zauber-Abdeckung der ersten Instanz (Issue #550, AC5).
//!
//! # Was er hält, und warum es eine eigene Datei ist
//!
//! `rotation-search` misst dieselbe Zahl und **gibt sie aus** — mehr nicht. Ein
//! Nightly-Schritt, der ihn aufruft, wäre grün, egal welche Zahl herauskommt:
//! Der Läufer endet mit 0, solange er nicht abstürzt. Dieser Wächter vergleicht
//! die Zahl gegen eine Schwelle und endet mit 1, wenn sie unterschritten wird.
//!
//! # Die Zahl
//!
//! Über die **vier Bosse der ersten Instanz** zusammen: Wie viele der
//! spielbaren Zauber werden in mindestens einer besten Rotation überhaupt
//! gewirkt? Gemessen am 2026-08-14: **11 von 58**.
//!
//! Das ist die Schnittmengen-Frage aus GAME-DESIGN Leitplanke 2 („jeder Zauber
//! muss irgendwo die beste Wahl sein"), nicht die Summe der Einzelzahlen: Ein
//! Zauber, der nur gegen `boss-1-1` gewirkt wird, zählt hier mit.
//!
//! # Warum eine UNTERGRENZE und keine feste Zahl
//!
//! Steigt die Abdeckung, ist das der Zweck des Issues — dann soll der Wächter
//! nicht rot werden. Fällt sie, ist etwas verlorengegangen: ein Zauber, der
//! seine Nische hatte, oder ein Boss, der sie erzwang. Genau das soll auffallen.
//!
//! **Wer die Schwelle anhebt, muss die neue Zahl gemessen haben** — sie steht
//! hier als Konstante und nirgends sonst.
//!
//! # Kosten
//!
//! Gemessen am 2026-08-14: **21 s** auf rustbuild (`--release`, vier Bosse,
//! `--max-len 6`, je 2258 Listen). Das ist der Grund, warum AC5 einen Wächter
//! bekommt und nicht den Absatz „zu teuer" — die Alternative stand in der AC
//! ausdrücklich offen.
use rota_sim::apl::AplName;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{all_candidates, greedy};

/// Die vier Bosse der ERSTEN Instanz.
///
/// **Vier aus EINER Instanz, nicht je einer aus vier** — das ist der Schnitt,
/// den AC2 verlangt und den vor dem 2026-08-14 keine Messung hatte. `#823`
/// nahm `boss-1-1, boss-2-2, boss-3-4, boss-4-4`; damit lässt sich nicht
/// sagen, ob eine Instanz in sich Abwechslung trägt.
const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"];

/// Höchstlänge der gesuchten Listen — dieselbe wie in der Messung zu AC2.
const MAX_LEN: usize = 6;

/// Durchgänge je Liste. Dieselben wie in `rotation-search`.
const RUNS: u32 = 50;
const SEED: u32 = 0;

/// **Die Untergrenze, gemessen am 2026-08-14.**
///
/// Elf von 58 spielbaren Zaubern werden über die vier Bosse zusammen gewirkt:
/// `aether, bellows, crucible-skin, efflux, kindle, purge, pyre, quench, rot,
/// searing, smoulder`.
const MINDESTENS: usize = 11;

fn config(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

fn main() {
    println!("== Zauber-Abdeckung der ersten Instanz (Issue #550, AC5) ==");
    println!();

    let pool = all_candidates();
    let spielbar: Vec<usize> = (0..rota_sim::SPELL_COUNT)
        .filter(|&i| u32::from(SPELLS[i].unlock_level) <= rota_sim::data::MAX_LEVEL)
        .collect();
    let mut irgendwo_gewirkt: Vec<bool> = vec![false; rota_sim::SPELL_COUNT];

    for boss_id in BOSSE {
        let cfg = config(boss_id);
        let gefunden = greedy(&cfg, &pool, MAX_LEN);

        // **Was im LAUF vorkam, nicht was in der Liste steht.** Bei `boss-1-3`
        // stand `kindle` am 2026-08-14 in der besten Liste und wurde trotzdem
        // nie gewirkt: Die Bedingungen davor greifen immer. Wer die Liste
        // zählte statt den Lauf, bekäme dort einen Zauber zu viel.
        let boss = boss_by_id(boss_id).expect("Boss");
        let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
            seed: SEED,
            boss,
            ..rota_sim::FightConfig::default()
        });
        let lauf = fight.run_apl(&gefunden.best.apl);
        let mut hier = 0;
        for &i in &spielbar {
            if lauf.casts[i] > 0 {
                irgendwo_gewirkt[i] = true;
                hier += 1;
            }
        }
        println!("  {boss_id:<10} {hier:>2} von {} gewirkt", spielbar.len());
    }

    let gewirkt: Vec<&str> = spielbar
        .iter()
        .filter(|&&i| irgendwo_gewirkt[i])
        .map(|&i| SPELLS[i].id)
        .collect();

    println!();
    println!(
        "  ueber alle {} Bosse zusammen   {} von {}",
        BOSSE.len(),
        gewirkt.len(),
        spielbar.len()
    );
    println!("    {}", gewirkt.join(", "));
    println!();

    // Die Zusicherung, ohne die der Wächter nichts hält: Stünde hier eine
    // leere Liste spielbarer Zauber, wäre jede Schwelle trivial verfehlt oder
    // trivial erfüllt, je nach Vorzeichen.
    assert!(
        spielbar.len() > MINDESTENS,
        "es gibt nur {} spielbare Zauber — die Schwelle {} misst dann nichts",
        spielbar.len(),
        MINDESTENS
    );

    if gewirkt.len() < MINDESTENS {
        eprintln!(
            "FEHLGESCHLAGEN: nur {} von {} spielbaren Zaubern werden ueber die {} \
             Bosse der ersten Instanz gewirkt — erwartet mindestens {}.",
            gewirkt.len(),
            spielbar.len(),
            BOSSE.len(),
            MINDESTENS
        );
        eprintln!(
            "Ein Zauber hat seine Nische verloren, oder ein Boss erzwingt sie nicht mehr. \
             Die Schwelle steht in `sim/examples/zauber-abdeckung.rs` (MINDESTENS) und \
             darf nur mit einer neuen Messung bewegt werden."
        );
        std::process::exit(1);
    }

    println!(
        "BESTANDEN: {} von {} >= {} (Untergrenze vom 2026-08-14)",
        gewirkt.len(),
        spielbar.len(),
        MINDESTENS
    );

    // Der Handschrift-Vergleich bleibt `rotation-search` überlassen; dieser
    // Läufer hält genau eine Zahl. `AplName` wird nur importiert, damit die
    // Signatur von `greedy` erfüllt ist — ohne Verwendung meldet clippy.
    let _ = AplName("zauber-abdeckung");
}
