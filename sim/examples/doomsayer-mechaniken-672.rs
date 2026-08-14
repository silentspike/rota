//! Issue #672: WELCHE Mechanik von `test-doomsayer` toetet allein?
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
//!   --example doomsayer-mechaniken-672
//! ```
//!
//! # Der Widerspruch, der diesen Laeufer noetig macht
//!
//! Zwei Aussagen standen nebeneinander, und beide konnten nicht stimmen:
//!
//! ```text
//!   Befund-Dokument   "Der Spieler stirbt AM VERHAENGNIS, doom 100 bei 60000 ms"
//!   spaetere Aussage  "der Tod kommt von der ZWEITEN Mechanik (stacks)"
//! ```
//!
//! Aus den Daten allein ist das nicht zu entscheiden: `doom` erreicht die
//! Schwelle 100 rechnerisch genau am Ende der Ueberdruckzeit, und die Stapel
//! schlagen dazwischen zu. Wer nur rechnet, bekommt eine plausible Antwort und
//! keine gemessene.
//!
//! # Der Aufbau: eine Mechanik abschalten, alles andere gleich lassen
//!
//! | Lauf | `doom` | `stacks` | was er beantwortet |
//! | --- | --- | --- | --- |
//! | A | an | AUS | toetet das Verhaengnis allein? wann? |
//! | B | AUS | an | toeten die Stapel allein? wann? |
//! | C | an | an | der heutige Stand als Vergleichspunkt |
//!
//! **Die Bossdatei wird dabei NICHT angefasst.** `Boss` ist `Copy`; der Laeufer
//! nimmt eine Kopie, setzt in ihr das eine Feld auf `None` und laesst sie ueber
//! `Box::leak` so lange leben, wie `FightConfig` es verlangt. Auf der Platte
//! aendert sich kein Byte — nachweisbar mit `git status` nach dem Lauf.
//!
//! **Die Liste ist in allen drei Laeufen dieselbe, und sie enthaelt KEIN
//! `purge`.** Das ist Absicht: `purge` senkt den Verhaengnis-Zaehler und
//! verstellt damit genau die Mechanik, deren Wirkung Lauf A messen soll.
//!
//! # Was "Todesursache" hier heisst
//!
//! In A und B ist sie durch den Aufbau eindeutig — es gibt nur eine Mechanik,
//! die toeten kann. In C wird sie am Zaehlerstand beim Tod abgelesen: Steht
//! `doom` an der Schwelle, war es das Verhaengnis; steht er darunter, kann es
//! das Verhaengnis nicht gewesen sein.

use rota_sim::apl::{parse, AplName};
use rota_sim::data::boss_by_id;
use rota_sim::{Fight, FightConfig};

const BOSS: &str = "test-doomsayer";
const SEED: u32 = 0;

/// Dieselbe Liste in allen drei Laeufen — ohne `purge`, siehe Kopf.
const LISTE: &str = "pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";

/// Ein Lauf mit abgeschalteten Mechaniken.
///
/// `mit_doom`/`mit_stacks` auf `false` setzt das jeweilige Feld der KOPIE auf
/// `None`. Die geleakte Kopie ist in einem Beispielprozess unbedenklich: Er
/// laeuft einmal und endet.
fn lauf(mit_doom: bool, mit_stacks: bool) -> (u32, Option<u32>, u32, bool, u32) {
    let basis = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let mut kopie = *basis;
    if !mit_doom {
        kopie.doom = None;
    }
    if !mit_stacks {
        kopie.stacks = None;
    }
    let boss: &'static rota_sim::data::Boss = Box::leak(Box::new(kopie));

    let apl = parse(AplName("mechanik"), LISTE).expect("APL gueltig");
    let mut fight = Fight::new(FightConfig {
        seed: SEED,
        boss,
        duration_ms: Some(basis.overpressure_ms),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let ergebnis = fight.run_apl(&apl);
    (
        fight.doom(),
        fight.died_at_ms(),
        fight.damage_taken(),
        fight.player_alive(),
        ergebnis.damage,
    )
}

fn zeile(name: &str, doom: u32, tod: Option<u32>, erlitten: u32, lebt: bool, schaden: u32) {
    let todestext = match tod {
        Some(ms) => format!("{ms} ms"),
        None => "ueberlebt".to_string(),
    };
    println!(
        "  {name:<28} {todestext:>12}  doom {doom:>3}  erlitten {erlitten:>6}  \
         Schaden {schaden:>5}  lebt {lebt}"
    );
}

fn main() {
    let boss = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let doom = boss.doom.expect("test-doomsayer traegt kein doom");
    let stacks = boss.stacks.expect("test-doomsayer traegt keine stacks");

    println!("== Die Werte, aus der Bossdatei gelesen ==\n");
    println!(
        "  doom     perStepMs {}   amount {}   threshold {}   clearAmount {}",
        doom.per_step_ms, doom.amount, doom.threshold, doom.clear_amount
    );
    println!(
        "  stacks   perStepMs {}   baseDamage {}   factor {}   maxStacks {}",
        stacks.per_step_ms, stacks.base_damage, stacks.factor, stacks.max_stacks
    );
    println!("  Ueberdruckzeit {} ms\n", boss.overpressure_ms);

    let (a_doom, a_tod, a_erl, a_lebt, a_dmg) = lauf(true, false);
    let (b_doom, b_tod, b_erl, b_lebt, b_dmg) = lauf(false, true);
    let (c_doom, c_tod, c_erl, c_lebt, c_dmg) = lauf(true, true);

    println!("== Die drei Laeufe, gleiche Liste, gleicher seed, gleiche Dauer ==\n");
    zeile("A  nur doom", a_doom, a_tod, a_erl, a_lebt, a_dmg);
    zeile("B  nur stacks", b_doom, b_tod, b_erl, b_lebt, b_dmg);
    zeile(
        "C  beide (heutiger Stand)",
        c_doom,
        c_tod,
        c_erl,
        c_lebt,
        c_dmg,
    );

    // **ZUSICHERUNG: die Abschaltung hat gewirkt.** Ohne sie koennte der
    // Laeufer dreimal denselben Kampf fahren und plausible Zahlen ausgeben —
    // genau der Fehler, den identische Zahlen sonst verraten.
    assert!(
        a_erl != b_erl,
        "A und B haben denselben erlittenen Schaden ({a_erl}) — dann hat das Abschalten \
         nichts bewirkt, und die Zeilen oben vergleichen dreimal dasselbe"
    );
    assert_eq!(
        b_doom, 0,
        "in Lauf B ist doom abgeschaltet, der Zaehler steht aber bei {b_doom}"
    );

    println!("\n== Was daraus folgt ==\n");

    let text_a = match a_tod {
        Some(ms) => format!("das Verhaengnis toetet ALLEIN, bei {ms} ms"),
        None => "das Verhaengnis toetet ALLEIN NICHT — der Spieler ueberlebt".to_string(),
    };
    let text_b = match b_tod {
        Some(ms) => format!("die Stapel toeten ALLEIN, bei {ms} ms"),
        None => "die Stapel toeten ALLEIN NICHT — der Spieler ueberlebt".to_string(),
    };
    println!("  A: {text_a}");
    println!("  B: {text_b}");

    match c_tod {
        None => println!("  C: der Spieler ueberlebt mit beiden Mechaniken"),
        Some(ms) => {
            // Die Ursache in C am Zaehlerstand, nicht an einer Vermutung.
            let ursache = if c_doom >= doom.threshold {
                "das Verhaengnis (Zaehler an der Schwelle)"
            } else {
                "NICHT das Verhaengnis (Zaehler unter der Schwelle) — es bleiben die Stapel"
            };
            println!(
                "  C: Tod bei {ms} ms, doom {c_doom} von {}: {ursache}",
                doom.threshold
            );
            match (a_tod, b_tod) {
                (Some(a), Some(b)) => println!(
                    "     allein genommen: doom bei {a} ms, stacks bei {b} ms — \
                     die fruehere von beiden ist {}",
                    if a <= b { "doom" } else { "stacks" }
                ),
                (Some(a), None) => println!("     allein toetet nur doom, bei {a} ms"),
                (None, Some(b)) => println!("     allein toeten nur die Stapel, bei {b} ms"),
                (None, None) => println!(
                    "     KEINE der beiden toetet allein — der Tod entsteht erst aus \
                     ihrer Kombination"
                ),
            }
        }
    }
}
