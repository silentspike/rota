//! Issue #672, Nachtrag: Warum konnte `purge` ACHTMAL wirken, obwohl
//! `clearCharges` auf 2 steht?
//!
//! ```text
//! cargo remote -c -- build --release \
//!   --example purge-nachladen-672
//! ./target/release/examples/purge-nachladen-672
//! ```
//!
//! # Die Frage hinter der Frage
//!
//! In Lauf A2 der Mechanik-Matrix (`doomsayer-mechaniken-672.rs`) ueberlebte
//! der Spieler das Verhaengnis, und `purge` kam achtmal dran. Die Bossdatei
//! nennt aber zwei Ladungen. Entweder laedt etwas nach — oder der Befund „purge
//! besiegt das Verhaengnis" steht auf einer falschen Grundlage.
//!
//! Es LAEDT nach: `Fight::recharge_ms` leitet die Nachladezeit ab, statt sie
//! aus einem Datenfeld zu lesen. Der Kommentar dort sagt, es sei „genau die
//! Zeit, in der der Zaehler um `clear_amount` steigt".
//!
//! # Warum dieser Laeufer die Formel AUFRUFT statt sie nachzurechnen
//!
//! Eine Nachrechnung im Kopf (40/5 * 1000 = 8000) antwortet aus einer zweiten
//! Quelle. Sie kann zufaellig stimmen, waehrend die Formel etwas anderes tut —
//! und dann bestaetigt die Messung den Irrtum. Hier wird `recharge_ms` selbst
//! gefahren; die Handrechnung steht daneben und muss sich der Formel
//! anschliessen, nicht umgekehrt.
//!
//! Weil `recharge_ms` privat ist, wird sie ueber ihre WIRKUNG gemessen: Wie
//! viele Wirkungen sind in einem Kampf moeglich, und passt das zur erwarteten
//! Zahl aus Ladungen plus Nachladungen?

use rota_sim::apl::{parse, AplName};
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::{Fight, FightConfig};

const BOSS: &str = "test-doomsayer";

/// Die Liste aus Lauf A2: `purge` ganz oben, Stapel abgeschaltet.
const MIT_PURGE: &str =
    "purge\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";

/// Ein Kampf ueber die volle Ueberdruckzeit, Stapel abgeschaltet wie in A2.
fn lauf(dauer_ms: u32) -> (u32, u32, bool) {
    let basis = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let mut kopie = *basis;
    kopie.stacks = None;
    let boss: &'static rota_sim::data::Boss = Box::leak(Box::new(kopie));

    let apl = parse(AplName("nachladen"), MIT_PURGE).expect("APL gueltig");
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(dauer_ms),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let ergebnis = fight.run_apl(&apl);
    let purge = index_of("purge").expect("purge fehlt");
    (ergebnis.casts[purge], fight.doom(), fight.player_alive())
}

fn main() {
    let boss = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let doom = boss.doom.expect("kein doom");

    println!("== Die Werte, aus der Bossdatei ==\n");
    println!(
        "  perStepMs {}   amount {}   threshold {}   clearAmount {}   clearCharges {}",
        doom.per_step_ms, doom.amount, doom.threshold, doom.clear_amount, doom.clear_charges
    );
    println!("  Ueberdruckzeit {} ms\n", boss.overpressure_ms);

    // Die Handrechnung — sie steht hier NUR als Vergleichswert. Ob sie stimmt,
    // entscheidet die gemessene Reihe darunter.
    let erwartete_nachladezeit =
        f64::from(doom.clear_amount) / f64::from(doom.amount) * f64::from(doom.per_step_ms);
    println!("== Die Handrechnung, die es zu pruefen gilt ==\n");
    println!(
        "  clearAmount / amount * perStepMs = {} / {} * {} = {:.0} ms je Nachladung",
        doom.clear_amount, doom.amount, doom.per_step_ms, erwartete_nachladezeit
    );
    let in_kampfzeit = f64::from(boss.overpressure_ms) / erwartete_nachladezeit;
    println!(
        "  in {} ms Kampf: {:.2} Nachladungen, plus {} Anfangsladungen = {:.2} Wirkungen\n",
        boss.overpressure_ms,
        in_kampfzeit,
        doom.clear_charges,
        in_kampfzeit + f64::from(doom.clear_charges)
    );

    // **Die Messung: eine Reihe wachsender Kampfdauern.** Eine einzelne Zahl
    // (8 Wirkungen bei 60 s) passt auf viele Formeln. Der VERLAUF passt nur
    // auf eine: Wenn alle `n` ms nachgeladen wird, steigt die Zahl der
    // Wirkungen alle `n` ms um eins.
    println!("== Gemessen: Wirkungen ueber wachsende Kampfdauer ==\n");
    println!("  Dauer ms   purge   doom   lebt   Zuwachs");
    let mut vorher = 0_u32;
    let mut spruenge: Vec<u32> = Vec::new();
    let mut letzter_sprung_bei = 0_u32;
    for dauer in (4000..=60000).step_by(2000) {
        let (wirkungen, stand, lebt) = lauf(dauer);
        let zuwachs = wirkungen.saturating_sub(vorher);
        if zuwachs > 0 && vorher > 0 {
            spruenge.push(dauer - letzter_sprung_bei);
        }
        if zuwachs > 0 {
            letzter_sprung_bei = dauer;
        }
        println!(
            "  {dauer:>8}   {wirkungen:>5}   {stand:>4}   {lebt:>5}   {}",
            if zuwachs > 0 {
                format!("+{zuwachs}")
            } else {
                String::new()
            }
        );
        vorher = wirkungen;
    }

    println!("\n== Was der Verlauf sagt ==\n");
    if spruenge.is_empty() {
        println!("  KEIN Nachladen erkennbar — die Zahl der Wirkungen steigt nicht.");
    } else {
        let summe: u32 = spruenge.iter().sum();
        let mittel = f64::from(summe) / spruenge.len() as f64;
        println!(
            "  {} Zuwaechse, mittlerer Abstand {:.0} ms (Messraster 2000 ms)",
            spruenge.len(),
            mittel
        );
        println!("  Abstaende: {spruenge:?}");
        let abweichung = (mittel - erwartete_nachladezeit).abs();
        if abweichung <= 2000.0 {
            println!(
                "\n  DIE FORMEL TRAEGT: gemessener Abstand {mittel:.0} ms, \
                 erwartet {erwartete_nachladezeit:.0} ms — Unterschied {abweichung:.0} ms, \
                 innerhalb des Messrasters."
            );
        } else {
            println!(
                "\n  DIE FORMEL TRAEGT NICHT: gemessener Abstand {mittel:.0} ms gegen \
                 erwartete {erwartete_nachladezeit:.0} ms — Unterschied {abweichung:.0} ms."
            );
        }
    }

    // Die Zusicherung: Der Lauf, um den es ging, ist auch der gemessene.
    let (bei_voller_zeit, _, lebt) = lauf(boss.overpressure_ms);
    println!(
        "\n  Bei voller Ueberdruckzeit: {bei_voller_zeit} Wirkungen, Spieler lebt {lebt} \
         (in A2 waren es 8)"
    );
    assert!(
        bei_voller_zeit > doom.clear_charges,
        "purge kam nur {bei_voller_zeit} mal dran, bei {} Ladungen — dann laedt gar nichts nach, \
         und die Frage dieses Laeufers stellt sich anders",
        doom.clear_charges
    );
}
