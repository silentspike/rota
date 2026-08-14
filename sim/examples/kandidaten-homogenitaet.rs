//! Messung B: Wie gleichfoermig sind die Rotationen einer Suchrunde?
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
//!   --example kandidaten-homogenitaet
//! ```
//!
//! # Wozu die Zahl gebraucht wird
//!
//! Eine GPU rechnet viele gleichartige Aufgaben gleichzeitig. Ob sich das fuer
//! die Rotationssuche lohnt, haengt daran, ob die bewerteten Rotationen
//! einander aehneln: Viele grosse gleichfoermige Gruppen sprechen dafuer,
//! lauter Einzelgaenger dagegen.
//!
//! Diese Messung entscheidet nichts. Sie liefert die Verteilung.
//!
//! # Was hier eine „Suchrunde" ist
//!
//! Genau das, was `greedy` in einem Durchgang tut: Ausgehend von der bisher
//! besten Liste wird JEDER Kandidat an JEDER Position eingefuegt und die
//! entstehende Rotation bewertet. Bei 93 Kandidaten und einer Liste der Laenge
//! 5 sind das 93 x 6 = 558 Rotationen — eine Runde, nicht die ganze Suche.
//!
//! **Nachgebaut, nicht abgegriffen:** `SearchResult` fuehrt nur die gewaehlte
//! Liste, nicht die verworfenen. Der Nachbau steht hier deshalb neben dem
//! Original in `rotation_search::greedy_opts` — wer die Suche aendert, muss
//! diese Datei mitziehen. Das ist der Preis dafuer, die Zwischenstaende zu
//! sehen.
//!
//! # Was „Gestalt" heisst
//!
//! Der Auftrag nennt drei Merkmale, und genau die stehen hier:
//!
//! ```text
//!   gleicher Boss                    (in einer Runde immer derselbe)
//!   gleiche Menge benutzter Zauber   als sortierte Liste der Indizes
//!   gleiche Zahl bedingter Zeilen    Zeilen mit einer if-Bedingung
//! ```
//!
//! Zwei Rotationen mit derselben Gestalt unterscheiden sich also nur noch in
//! der REIHENFOLGE ihrer Zeilen oder in den konkreten Bedingungen.
//!
//! # Die Einstellungen
//!
//! ```text
//!   Boss           boss-1-1
//!   Kampfdauer     60000 ms
//!   Startliste     die ersten 5 Zeilen aus greedy()
//! ```

use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::rotation_search::{all_candidates, greedy, Candidate};

const BOSS: &str = "boss-1-1";
const DAUER_MS: u32 = 60_000;
const STARTLAENGE: usize = 5;

/// Die Gestalt einer Rotation: benutzte Zauber (sortiert) und Zahl der
/// bedingten Zeilen.
fn gestalt(zeilen: &[Candidate]) -> (Vec<usize>, usize) {
    let mut zauber: Vec<usize> = zeilen.iter().map(|k| k.spell).collect();
    zauber.sort_unstable();
    zauber.dedup();
    let bedingt = zeilen
        .iter()
        .filter(|k| !k.rule().condition.groups.is_empty())
        .count();
    (zauber, bedingt)
}

fn main() {
    let pool = all_candidates();
    let boss = boss_by_id(BOSS).expect("Boss fehlt");
    let config = BalanceConfig {
        iterations: 20,
        base_seed: 5,
        threads: 2,
        stop_on_kill: false,
        duration_ms: Some(DAUER_MS),
        boss,
        ..BalanceConfig::default()
    };

    println!("== Einstellungen ==\n");
    println!("  Boss {BOSS} · Dauer {DAUER_MS} ms · Startlaenge {STARTLAENGE}");
    println!("  Kandidaten im Pool: {}\n", pool.len());

    // Die Startliste: die ersten Zeilen einer echten Suche.
    let voll = greedy(&config, &pool, STARTLAENGE);
    let start: Vec<Candidate> = voll.chosen.clone();
    println!("== Die Startliste der Runde ==\n");
    for (i, k) in start.iter().enumerate() {
        println!("  {}. {}", i + 1, k.rule());
    }

    // **Die Runde:** jeder Kandidat an jeder Position.
    let mut gestalten: Vec<((Vec<usize>, usize), usize)> = Vec::new();
    let mut gesamt = 0_usize;
    for pos in 0..=start.len() {
        for kandidat in &pool {
            let mut neu = start.clone();
            neu.insert(pos, kandidat.clone());
            gesamt += 1;
            let g = gestalt(&neu);
            match gestalten.iter_mut().find(|(k, _)| *k == g) {
                Some((_, anzahl)) => *anzahl += 1,
                None => gestalten.push((g, 1)),
            }
        }
    }

    // ZUSICHERUNG: Die Runde hat die Groesse, die `greedy` auch haette.
    let erwartet = pool.len() * (start.len() + 1);
    assert_eq!(
        gesamt, erwartet,
        "die nachgebaute Runde hat {gesamt} Rotationen, greedy haette {erwartet} — dann misst \
         diese Datei etwas anderes als die Suche"
    );

    let mut groessen: Vec<usize> = gestalten.iter().map(|(_, n)| *n).collect();
    groessen.sort_unstable_by(|a, b| b.cmp(a));

    println!("\n== Ergebnis ==\n");
    println!("  bewertete Rotationen in EINER Runde: {gesamt}");
    println!("  verschiedene Gestalten:              {}", gestalten.len());
    println!(
        "  mittlere Gruppengroesse:             {:.2}",
        gesamt as f64 / gestalten.len() as f64
    );

    println!("\n  Die zehn groessten Gruppen:\n");
    for (i, n) in groessen.iter().take(10).enumerate() {
        println!(
            "    {:>2}. {n:>5} Rotationen  ({:.1} % der Runde)",
            i + 1,
            *n as f64 / gesamt as f64 * 100.0
        );
    }

    let median = groessen[groessen.len() / 2];
    let einzel = groessen.iter().filter(|n| **n == 1).count();
    println!("\n  Median der Gruppengroesse: {median}");
    println!(
        "  Einzelgaenger (Gruppe der Groesse 1): {einzel} von {} Gruppen  ({:.1} %)",
        gestalten.len(),
        einzel as f64 / gestalten.len() as f64 * 100.0
    );
    println!(
        "  Anteil der Rotationen in Einzelgaenger-Gruppen: {:.1} %",
        einzel as f64 / gesamt as f64 * 100.0
    );

    // Wie viel deckt die groesste Gruppe? Das ist die Zahl, an der eine
    // GPU-Entscheidung haengt: Ein Stapel lohnt sich, wenn er gross ist.
    let top10: usize = groessen.iter().take(10).sum();
    println!(
        "\n  Die zehn groessten Gruppen decken zusammen {top10} von {gesamt} Rotationen \
         ({:.1} %)",
        top10 as f64 / gesamt as f64 * 100.0
    );
}
