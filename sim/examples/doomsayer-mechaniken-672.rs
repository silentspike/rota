//! Issue #672: WELCHE Mechanik von `test-doomsayer` toetet, und was aendert
//! `purge` daran?
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- build --release \
//!   --example doomsayer-mechaniken-672
//! ./target/release/examples/doomsayer-mechaniken-672
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
//! Aus den Daten allein ist das nicht zu entscheiden, und Rechnen liefert eine
//! plausible Antwort statt einer gemessenen.
//!
//! # Der Aufbau: zwei Achsen, sechs Laeufe
//!
//! ```text
//!                    ohne purge        mit purge
//!     nur doom           A1                A2
//!     nur stacks         B1                B2
//!     beide              C1                C2
//! ```
//!
//! Die erste Achse schaltet je eine Mechanik ab, die zweite nimmt `purge` in
//! die Liste. Drei Fragen, die nur zusammen zu beantworten sind:
//!
//! | Frage | Vergleich |
//! | --- | --- |
//! | Welche Mechanik toetet ALLEIN? | A1 gegen B1 |
//! | Was aendert `purge` daran? | Spalte gegen Spalte |
//! | Loest das den Widerspruch auf? | C1 gegen C2 |
//!
//! **Warum `purge` nicht weggelassen werden darf**, obwohl es genau die
//! Mechanik verstellt, die A messen soll: Gefragt ist nicht, wie `doom` sich in
//! Reinform verhaelt, sondern WORAN DER SPIELER STIRBT — und der Spieler hat
//! `purge`. Der erste Entwurf dieses Laeufers liess es weg und konnte die
//! naheliegendste Erklaerung des Widerspruchs weder bestaetigen noch widerlegen.
//!
//! **Die Bossdatei wird nicht angefasst.** `Boss` ist `Copy`; der Laeufer nimmt
//! eine Kopie, setzt in ihr das eine Feld auf `None` und laesst sie ueber
//! `Box::leak` so lange leben, wie `FightConfig` es verlangt. Auf der Platte
//! aendert sich kein Byte — nachweisbar mit `git status data/` nach dem Lauf.
//!
//! # Was "Todesursache" hier heisst
//!
//! In den Zeilen A und B ist sie durch den Aufbau eindeutig: Es gibt nur eine
//! Mechanik, die toeten kann. In C wird sie am Zaehlerstand beim Tod
//! abgelesen — steht `doom` an der Schwelle, war es das Verhaengnis.
//!
//! Und es gibt ein zweites, unabhaengiges Merkmal: Der Verhaengnis-Tod laeuft
//! NICHT ueber den Lebensbalken. In einem Lauf, in dem nur `doom` wirkt, steht
//! der erlittene Schaden auf 0.

use rota_sim::apl::{parse, AplName};
use rota_sim::data::boss_by_id;
use rota_sim::{Fight, FightConfig};

const BOSS: &str = "test-doomsayer";
const SEED: u32 = 0;

/// Die Liste ohne `purge`.
const OHNE_PURGE: &str = "pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";
/// Dieselbe Liste mit `purge` an oberster Stelle — sonst Zeile fuer Zeile gleich.
const MIT_PURGE: &str =
    "purge\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";

/// Was ein Lauf ueber sich verraet.
struct Lauf {
    doom: u32,
    tod: Option<u32>,
    erlitten: u32,
    schaden: u32,
    purge_gewirkt: u32,
}

/// Ein Kampf mit abgeschalteten Mechaniken und wahlweise `purge`.
fn lauf(mit_doom: bool, mit_stacks: bool, mit_purge: bool) -> Lauf {
    let basis = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let mut kopie = *basis;
    if !mit_doom {
        kopie.doom = None;
    }
    if !mit_stacks {
        kopie.stacks = None;
    }
    let boss: &'static rota_sim::data::Boss = Box::leak(Box::new(kopie));

    let text = if mit_purge { MIT_PURGE } else { OHNE_PURGE };
    let apl = parse(AplName("mechanik"), text).expect("APL gueltig");
    let mut fight = Fight::new(FightConfig {
        seed: SEED,
        boss,
        duration_ms: Some(basis.overpressure_ms),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let ergebnis = fight.run_apl(&apl);
    let purge = rota_sim::data::index_of("purge").expect("purge fehlt");
    Lauf {
        doom: fight.doom(),
        tod: fight.died_at_ms(),
        erlitten: fight.damage_taken(),
        schaden: ergebnis.damage,
        purge_gewirkt: ergebnis.casts[purge],
    }
}

/// Die Todesursache, aus dem Aufbau UND dem Zaehlerstand.
fn ursache(l: &Lauf, mit_doom: bool, mit_stacks: bool, schwelle: u32) -> &'static str {
    if l.tod.is_none() {
        return "ueberlebt";
    }
    match (mit_doom, mit_stacks) {
        (true, false) => "Verhaengnis (einzige Mechanik)",
        (false, true) => "Stapel (einzige Mechanik)",
        (true, true) => {
            if l.doom >= schwelle {
                "Verhaengnis (Zaehler an der Schwelle)"
            } else {
                "Stapel (Zaehler unter der Schwelle)"
            }
        }
        (false, false) => "weder noch — beide abgeschaltet",
    }
}

fn zeile(name: &str, l: &Lauf, ursache: &str) {
    let tod = match l.tod {
        Some(ms) => format!("{ms} ms"),
        None => "ueberlebt".to_string(),
    };
    println!(
        "  {name:<6} {tod:>10}  doom {:>3}  erlitten {:>6}  Schaden {:>5}  purge {:>2}  {ursache}",
        l.doom, l.erlitten, l.schaden, l.purge_gewirkt
    );
}

fn main() {
    let boss = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let doom = boss.doom.expect("test-doomsayer traegt kein doom");
    let stacks = boss.stacks.expect("test-doomsayer traegt keine stacks");

    println!("== Die Werte, aus der Bossdatei gelesen ==\n");
    println!(
        "  doom     perStepMs {}   amount {}   threshold {}   clearAmount {}   clearCharges {}",
        doom.per_step_ms, doom.amount, doom.threshold, doom.clear_amount, doom.clear_charges
    );
    println!(
        "  stacks   perStepMs {}   baseDamage {}   factor {}   maxStacks {}",
        stacks.per_step_ms, stacks.base_damage, stacks.factor, stacks.max_stacks
    );
    println!("  Ueberdruckzeit {} ms\n", boss.overpressure_ms);

    let a1 = lauf(true, false, false);
    let b1 = lauf(false, true, false);
    let c1 = lauf(true, true, false);
    let a2 = lauf(true, false, true);
    let b2 = lauf(false, true, true);
    let c2 = lauf(true, true, true);

    println!("== OHNE purge ==\n");
    zeile("A1", &a1, ursache(&a1, true, false, doom.threshold));
    zeile("B1", &b1, ursache(&b1, false, true, doom.threshold));
    zeile("C1", &c1, ursache(&c1, true, true, doom.threshold));

    println!("\n== MIT purge ==\n");
    zeile("A2", &a2, ursache(&a2, true, false, doom.threshold));
    zeile("B2", &b2, ursache(&b2, false, true, doom.threshold));
    zeile("C2", &c2, ursache(&c2, true, true, doom.threshold));

    // **ZUSICHERUNGEN.** Ohne sie koennte der Laeufer sechsmal denselben Kampf
    // fahren und sechs plausible Zeilen ausgeben — der Fehler, den identische
    // Zahlen sonst verraten, nur diesmal vorher abgefangen.
    assert!(
        a1.erlitten != b1.erlitten,
        "A1 und B1 haben denselben erlittenen Schaden ({}) — dann hat das Abschalten nichts \
         bewirkt, und die Tabelle vergleicht dasselbe mit sich selbst",
        a1.erlitten
    );
    assert_eq!(
        b1.doom, 0,
        "in B1 ist doom abgeschaltet, der Zaehler steht aber bei {}",
        b1.doom
    );
    assert_eq!(
        a1.purge_gewirkt, 0,
        "in der Spalte OHNE purge wurde purge {} mal gewirkt",
        a1.purge_gewirkt
    );
    assert!(
        c2.purge_gewirkt > 0,
        "in C2 wurde purge NIE gewirkt — dann ist die zweite Spalte eine Kopie der ersten, \
         und der Vergleich beantwortet nichts"
    );

    println!("\n== Was daraus folgt ==\n");

    let text = |l: &Lauf| match l.tod {
        Some(ms) => format!("Tod bei {ms} ms"),
        None => "ueberlebt".to_string(),
    };
    println!(
        "  allein toedlich?   doom {}   stacks {}",
        text(&a1),
        text(&b1)
    );
    println!(
        "  was purge aendert  doom {}   stacks {}",
        text(&a2),
        text(&b2)
    );
    println!(
        "  beide zusammen     ohne purge {}   mit purge {}",
        text(&c1),
        text(&c2)
    );

    let c1_doom = c1.tod.is_some() && c1.doom >= doom.threshold;
    let c2_doom = c2.tod.is_some() && c2.doom >= doom.threshold;
    if c1_doom && !c2_doom {
        println!(
            "\n  DER WIDERSPRUCH IST KEINER: C1 stirbt am Verhaengnis, C2 an den Stapeln.\n  \
             Zwei richtige Aussagen ueber zwei verschiedene Laeufe."
        );
    } else if c1_doom && c2_doom {
        println!("\n  In BEIDEN Faellen toetet das Verhaengnis — purge verschiebt es nur.");
    } else if !c1_doom && !c2_doom {
        println!("\n  In BEIDEN Faellen toeten die Stapel.");
    } else {
        println!("\n  Umgekehrt als erwartet: C1 an den Stapeln, C2 am Verhaengnis.");
    }
}
