//! Messung A: Ab welcher Entscheidung weicht ein Kampfverlauf ab?
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
//!   --example trace-wiederverwendung
//! ```
//!
//! # Wozu die Zahl gebraucht wird
//!
//! Die Suche bewertet tausende Rotationen, die sich meist um EINE Zeile
//! unterscheiden. Wenn zwei solche Rotationen ihre ersten dreissig
//! Entscheidungen teilen, liesse sich der gemeinsame Anfang einmal rechnen und
//! nur der Rest neu — inkrementelle Auswertung. Weichen sie dagegen sofort ab,
//! bringt das nichts.
//!
//! Diese Messung sagt NICHT, ob der Umbau sich lohnt. Sie liefert die Zahl,
//! auf der diese Entscheidung steht.
//!
//! # Was hier „Entscheidung" heisst
//!
//! Ein `LogKind::Cast` im Kampfprotokoll: der Moment, in dem die
//! Prioritaetenliste eine Zeile gewaehlt und gewirkt hat. Verglichen werden
//! die Folgen aus (Zeit, Zauber) — zwei Verlaeufe sind bis Entscheidung `n`
//! gleich, wenn ihre ersten `n` Eintraege uebereinstimmen.
//!
//! **Warum nicht `checksum_at`:** Die Pruefsumme je Zeitschritt sagt, DASS
//! sich der Zustand unterscheidet, aber nicht, an der wievielten Entscheidung.
//! Fuer die Frage nach dem gemeinsamen Praefix ist die Cast-Folge das
//! genauere Mass; die Pruefsumme steht als Gegenprobe daneben.
//!
//! # Die Einstellungen
//!
//! ```text
//!   Boss            boss-1-1
//!   Kampfdauer      60000 ms
//!   Startwert       5, ein Durchgang je Variante (deterministisch)
//!   Grundrotation   aus greedy(), max_len 8
//!   Varianten       Zeile getauscht + Zeile eingefuegt + zwei vertauscht
//! ```

use rota_sim::apl::{Apl, Rule};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{all_candidates, greedy, Candidate};
use rota_sim::{Fight, FightConfig};

const BOSS: &str = "boss-1-1";
const DAUER_MS: u32 = 60_000;
const SEED: u32 = 5;
const MAX_LEN: usize = 8;

/// Die Folge der Entscheidungen: (Zeit, Zauber) je gewirkter Zeile.
fn entscheidungen(apl: &Apl) -> Vec<(u32, u8)> {
    let boss = boss_by_id(BOSS).expect("Boss fehlt");
    let mut fight = Fight::new(FightConfig {
        seed: SEED,
        boss,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(apl);
    fight
        .log()
        .entries()
        .into_iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| (e.time, e.spell))
        .collect()
}

/// Aus Kandidaten eine Liste bauen — die Zeilennummern zaehlen von 1.
fn liste(name: &str, zeilen: &[Candidate]) -> Apl {
    Apl {
        name: name.to_owned(),
        rules: zeilen
            .iter()
            .enumerate()
            .map(|(i, k)| Rule {
                line: i + 1,
                ..k.rule()
            })
            .collect(),
    }
}

fn main() {
    let pool = all_candidates();
    let boss = boss_by_id(BOSS).expect("Boss fehlt");
    let config = BalanceConfig {
        iterations: 20,
        base_seed: SEED,
        threads: 2,
        stop_on_kill: false,
        duration_ms: Some(DAUER_MS),
        boss,
        ..BalanceConfig::default()
    };

    println!("== Einstellungen ==\n");
    println!("  Boss {BOSS} · Dauer {DAUER_MS} ms · Startwert {SEED} · max_len {MAX_LEN}");
    println!("  Kandidaten im Pool: {}\n", pool.len());

    // --- Die Grundrotation ------------------------------------------------
    let grund = greedy(&config, &pool, MAX_LEN);
    let basis: Vec<Candidate> = grund.chosen.clone();
    let basis_folge = entscheidungen(&liste("basis", &basis));
    println!("== Die Grundrotation ==\n");
    for (i, k) in basis.iter().enumerate() {
        println!("  {}. {}", i + 1, k.rule());
    }
    println!("\n  Entscheidungen im Kampf: {}\n", basis_folge.len());
    assert!(
        basis_folge.len() > 10,
        "die Grundrotation trifft nur {} Entscheidungen — dann sagt eine Verteilung ueber \
         1/5/20/35 nichts",
        basis_folge.len()
    );

    // --- Die Varianten ----------------------------------------------------
    //
    // Drei Arten, wie der Auftrag sie nennt. Erzeugt wird jede moegliche
    // Aenderung ihrer Art, nicht eine Auswahl — sonst haenge das Ergebnis
    // daran, welche ich gewaehlt haette.
    let mut varianten: Vec<(&'static str, Apl)> = Vec::new();

    // 1. Eine Zeile durch eine andere ersetzen.
    for pos in 0..basis.len() {
        for ersatz in pool.iter().take(40) {
            if ersatz.spell == basis[pos].spell {
                continue;
            }
            let mut neu = basis.clone();
            neu[pos] = ersatz.clone();
            varianten.push(("getauscht", liste("v", &neu)));
        }
    }
    // 2. Eine Zeile einfuegen.
    for pos in 0..=basis.len() {
        for zusatz in pool.iter().take(25) {
            let mut neu = basis.clone();
            neu.insert(pos, zusatz.clone());
            varianten.push(("eingefuegt", liste("v", &neu)));
        }
    }
    // 3. Zwei Zeilen vertauschen.
    for a in 0..basis.len() {
        for b in (a + 1)..basis.len() {
            let mut neu = basis.clone();
            neu.swap(a, b);
            varianten.push(("vertauscht", liste("v", &neu)));
        }
    }

    println!("== Die Varianten ==\n");
    println!("  geprueft: {}\n", varianten.len());
    assert!(
        varianten.len() >= 200,
        "nur {} Varianten — der Auftrag verlangt mindestens 200",
        varianten.len()
    );

    // --- Die Messung ------------------------------------------------------
    let mut identisch = 0_usize;
    let mut erste_abweichung: Vec<usize> = Vec::new();
    let mut je_art: Vec<(&'static str, usize, usize)> = Vec::new();

    for (art, apl) in &varianten {
        let folge = entscheidungen(apl);
        let gemeinsam = basis_folge
            .iter()
            .zip(folge.iter())
            .take_while(|(a, b)| a == b)
            .count();
        if gemeinsam == basis_folge.len() && folge.len() == basis_folge.len() {
            identisch += 1;
        }
        erste_abweichung.push(gemeinsam);
        match je_art.iter_mut().find(|(n, _, _)| n == art) {
            Some((_, anzahl, summe)) => {
                *anzahl += 1;
                *summe += gemeinsam;
            }
            None => je_art.push((art, 1, gemeinsam)),
        }
    }

    let n = erste_abweichung.len();
    println!("== Ergebnis ==\n");
    println!(
        "  identischer Verlauf ueber den ganzen Kampf: {identisch} von {n}  ({:.1} %)",
        identisch as f64 / n as f64 * 100.0
    );

    println!("\n  Verteilung der ersten Abweichung:\n");
    for schwelle in [0_usize, 1, 5, 20, 35] {
        let anzahl = erste_abweichung.iter().filter(|g| **g == schwelle).count();
        let bis = erste_abweichung.iter().filter(|g| **g <= schwelle).count();
        println!(
            "    genau bei Entscheidung {schwelle:>3}: {anzahl:>5}   bis dahin: {bis:>5}  ({:.1} %)",
            bis as f64 / n as f64 * 100.0
        );
    }

    let mut sortiert = erste_abweichung.clone();
    sortiert.sort_unstable();
    let median = sortiert[n / 2];
    let mittel = erste_abweichung.iter().sum::<usize>() as f64 / n as f64;
    println!("\n  Median der gemeinsamen Praefixlaenge: {median}");
    println!("  Mittelwert: {mittel:.1}");
    println!("  Laenge der Grundfolge: {}", basis_folge.len());

    println!("\n  Je Art der Aenderung:\n");
    for (art, anzahl, summe) in &je_art {
        println!(
            "    {art:<12} {anzahl:>5} Varianten, mittlerer gemeinsamer Praefix {:.1}",
            *summe as f64 / *anzahl as f64
        );
    }

    // --- Die Antwort, die der Auftrag ausdruecklich verlangt ---------------
    println!("\n== Die Antwort ==\n");
    let anteil_frueh = erste_abweichung.iter().filter(|g| **g <= 5).count() as f64 / n as f64;
    if anteil_frueh >= 0.5 {
        println!(
            "  DIVERGIERT SOFORT: {:.1} % der Varianten weichen spaetestens bei Entscheidung 5 ab.",
            anteil_frueh * 100.0
        );
        println!("  Ein gemeinsamer Anfang ist damit die Ausnahme, nicht die Regel.");
    } else {
        println!(
            "  DIVERGIERT SPAET: nur {:.1} % weichen bis Entscheidung 5 ab, der Median liegt \
             bei {median} von {}.",
            anteil_frueh * 100.0,
            basis_folge.len()
        );
        println!("  Ein gemeinsamer Anfang ist die Regel.");
    }
}
