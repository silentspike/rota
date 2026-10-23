//! Die Gewichts-Kurve: Was waehlt die Suche bei welchem `prevented_weight`?
//!
//! ```text
//! cargo remote -c -- build --release \
//!   --example gewicht-reihe-419
//! ./target/release/examples/gewicht-reihe-419
//! ```
//!
//! # Die Frage, die dieser Laeufer beantwortet
//!
//! ```text
//!   1  Ab welchem Wert kommt Verteidigung ueberhaupt herein?
//!   2  Ab welchem Wert VERDRAENGT sie den Schaden?
//! ```
//!
//! Beide fragen nach dem, was die Suche VON SELBST waehlt. Deshalb faehrt
//! dieser Laeufer je Gewicht EINE freie Suche und liest ihr Ergebnis — nicht
//! je Zauber eine erzwungene.
//!
//! **Der Unterschied ist keine Sparmassnahme, sondern die Frage.** „Kann
//! dieser Zauber drankommen, wenn man ihn aufzwingt?" ist die Frage von AC3
//! (`ac3-umgebaute-wege-419.rs`). „Was waehlt die Suche?" ist diese hier.
//!
//! # `prevented_weight` steht nirgends fest
//!
//! ```text
//!   rotation_search.rs:367   mean = damage.mean + prevented_weight * verhindert
//!   balance.rs:161           prevented_weight: 0.0     <- Vorgabewert
//! ```
//!
//! Mit dem Vorgabewert ist die Suche blind fuer Verteidigung (#472, #814). Der
//! Wert steht nicht im GAME-DESIGN und nicht in den Daten. **Dieser Laeufer
//! legt ihn auch nicht fest** — die Zahl gehoert dem Auftraggeber; eine
//! Empfehlung im Code waere eine Design-Entscheidung an der unsichtbarsten
//! Stelle.
//!
//! # Zwei Zusicherungen, aus frueheren Fehlschluessen
//!
//! **Gemessen wird, ob ein Zauber GEWIRKT wird** — nicht, ob er in der Liste
//! steht. Sechs Zauber standen einmal auf Platz 8 von 8 mit null Wirkungen und
//! ergaben alle exakt denselben Schaden.
//!
//! **Und der Durchsatz wird je Gewicht mitgefuehrt.** Ohne ihn saehe man, dass
//! Verteidigung hereinkommt, aber nicht, was sie kostet.
//!
//! # Die Einteilung in Arten ist ABGELEITET, nicht gelesen
//!
//! Die Zauberdaten fuehren kein Art-Feld. Die Zuordnung entsteht aus den
//! vorhandenen Merkmalen und steht hier, damit sie bestritten werden kann:
//!
//! ```text
//!   Schutz      shield, decay_ward, reflect
//!   Heilung     heal
//!   Kontrolle   control, control_gain, interrupt
//!   Schaden     base_damage > 0 oder ein dot, und nichts von oben
//!   sonstige    keins davon (Umwandlung, Abklingzeit-Werkzeuge, Buffs)
//! ```

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, Spell, SPELLS};
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::{Fight, FightConfig};

/// Die Stufen der Reihe.
///
/// **Zwischen 0 und 0,25 liegen vier zusaetzliche**, und das ist gemessen
/// begruendet: Der erste Durchgang zeigte dort den ganzen Sprung — der
/// Durchsatz faellt von 12051 auf 8300 und der erlittene Schaden von 4800 auf
/// 480. Ohne feinere Stufen saehe man ein Kippen, aber nicht wo.
const GEWICHTE: [f64; 12] = [
    0.0, 0.05, 0.1, 0.15, 0.2, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0,
];

/// Die Bosse, an denen gemessen wird.
///
/// **Mehr als einer, und der Grund ist eine Einschraenkung meiner eigenen
/// ersten Messung:** Eine Kurve an einem einzigen Boss sagt nichts darueber,
/// ob sie verallgemeinerbar ist. `boss-2-3` traegt `runaway` und damit ein
/// anderes Ziel — liegt die Kurve dort anders, ist eine Entscheidung ueber
/// `prevented_weight` keine allgemeine, sondern eine ueber boss-1-1.
const BOSSE: [&str; 3] = ["boss-1-1", "boss-1-2", "boss-2-3"];
const ARTEN: [&str; 5] = ["Schaden", "Schutz", "Kontrolle", "Heilung", "sonstige"];

/// Die abgeleitete Art eines Zaubers — siehe Kopf.
fn art(s: &Spell) -> &'static str {
    if s.shield.is_some() || s.decay_ward.is_some() || s.reflect.is_some() {
        "Schutz"
    } else if s.heal.is_some() {
        "Heilung"
    } else if s.control.is_some() || s.control_gain > 0 || s.interrupt {
        "Kontrolle"
    } else if s.base_damage > 0 || s.dot.is_some() {
        "Schaden"
    } else {
        "sonstige"
    }
}

fn main() {
    let pool = all_candidates();
    for boss_id in BOSSE {
        messen(&pool, boss_id);
    }
}

fn messen(pool: &[rota_sim::rotation_search::Candidate], boss_id: &str) {
    println!("\n############  {boss_id}  ############\n");
    let boss = boss_by_id(boss_id).expect("Boss fehlt");

    println!("== Wie viele Zauber es je Art gibt (der Bezugspunkt) ==\n  ");
    for a in ARTEN {
        print!("{a}: {}   ", SPELLS.iter().filter(|s| art(s) == a).count());
    }
    println!("\n");

    let mut zeilen: Vec<(f64, u32, u32, Vec<&'static str>)> = Vec::new();

    for gewicht in GEWICHTE {
        let c = BalanceConfig {
            iterations: 20,
            base_seed: 5,
            threads: 2,
            stop_on_kill: false,
            duration_ms: Some(60_000),
            boss,
            prevented_weight: gewicht,
            ..BalanceConfig::default()
        };
        let frei = greedy(&c, pool, 8);

        // Einen Kampf mit der gefundenen Liste fahren: Schaden, erlittener
        // Schaden, und WELCHE Zauber wirklich gewirkt werden.
        let mut f = Fight::new(FightConfig {
            seed: 5,
            boss,
            duration_ms: Some(60_000),
            stop_on_kill: false,
            ..FightConfig::default()
        });
        let e = f.run_apl(&frei.best.apl);
        let gewirkt: Vec<&'static str> = SPELLS
            .iter()
            .enumerate()
            .filter(|(i, _)| e.casts[*i] > 0)
            .map(|(_, s)| s.id)
            .collect();
        zeilen.push((gewicht, e.damage, f.damage_taken(), gewirkt));
    }

    println!("== Die Kurve ==\n");
    print!("  {:<9}{:>10}{:>10}", "Gewicht", "Schaden", "erlitten");
    for a in ARTEN {
        print!("{a:>10}");
    }
    println!("{:>8}", "Zeilen");
    for (gewicht, schaden, erlitten, gewirkt) in &zeilen {
        print!("  {gewicht:<9.2}{schaden:>10}{erlitten:>10}");
        for a in ARTEN {
            let n = gewirkt
                .iter()
                .filter(|id| SPELLS.iter().any(|s| &s.id == *id && art(s) == a))
                .count();
            print!("{n:>10}");
        }
        println!("{:>8}", gewirkt.len());
    }

    println!("\n== Welche Zauber die Suche waehlt ==\n");
    for (gewicht, _, _, gewirkt) in &zeilen {
        println!("  w={gewicht:<6} {}", gewirkt.join(" "));
    }

    println!("\n== Was sich zwischen den Stufen aendert ==\n");
    for paar in zeilen.windows(2) {
        let (g0, _, _, a0) = &paar[0];
        let (g1, _, _, a1) = &paar[1];
        let neu: Vec<&str> = a1.iter().filter(|x| !a0.contains(x)).copied().collect();
        let weg: Vec<&str> = a0.iter().filter(|x| !a1.contains(x)).copied().collect();
        println!(
            "  {g0:<5} -> {g1:<5} herein: {:<34} heraus: {}",
            if neu.is_empty() {
                "—".to_owned()
            } else {
                neu.join(" ")
            },
            if weg.is_empty() {
                "—".to_owned()
            } else {
                weg.join(" ")
            }
        );
    }
}
