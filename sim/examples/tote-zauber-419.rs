//! Kommt ein Zauber in seiner BESTEN Bedingung dran? (#419 AC3, Vorarbeit)
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
//!   --example tote-zauber-419
//! ```
//!
//! # Der Fehler, gegen den dieser Laeufer gebaut ist — mein eigener
//!
//! Die erste Fassung gab der Suche zu jedem Zauber die ERSTE Kandidatenzeile
//! vor, und das ist die OHNE Bedingung. Fuer einen Brand ist die nutzlos: Ohne
//! „if dot.remains < …" wird er verdraengt, die Suche schiebt ihn ans Ende,
//! und er kommt nie dran.
//!
//! Ergebnis: 40 von 70 Zaubern „tot" — darunter `smoulder`, der in der freien
//! besten Rotation nachweislich vorkommt. **Ein Zauber, dessen Antwort
//! feststand, hat den Fehler aufgedeckt.**
//!
//! `candidates_for` erzeugt je Zauber MEHRERE Zeilen. Dieser Laeufer probiert
//! sie ALLE durch und nimmt die beste — dann ist „kommt nie dran" eine Aussage
//! ueber den Zauber und nicht ueber meine Auswahl.
//!
//! # Was „tot" hier heisst, und was nicht
//!
//! Nur: „traegt in einer schadensoptimalen Rotation nichts bei". Ein
//! Schutzzauber, der nie drankommt, kann trotzdem seinen Zweck haben — der
//! Balance-Laeufer misst Schaden, sonst nichts (#814).

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::rotation_search::{all_candidates, greedy_from};
use rota_sim::{Fight, FightConfig};

fn main() {
    let pool = all_candidates();
    let boss = boss_by_id("boss-1-1").expect("boss-1-1 fehlt");
    // **`prevented_weight` MUSS gesetzt werden, sonst misst die Suche blind.**
    //
    // Der Vorgabewert ist 0.0. Mit ihm rechnet `measure` nur `damage.mean` —
    // ein Zauber, der Schaden VERHINDERT statt austeilt, ist dann unsichtbar.
    // Genau das ist der Befund aus #472 und #814, und mein erster Lauf ist
    // hineingelaufen: Er meldete 38 tote Zauber, darunter jeden Schutzzauber.
    //
    // Gemessen wird deshalb ZWEIMAL: einmal blind (Gewicht 0), einmal mit
    // Gewicht. Die Differenz ist die Aussage — welche Zauber sichtbar werden,
    // sobald Verhinderung zaehlt.
    let bauen = |gewicht: f64| BalanceConfig {
        iterations: 10,
        base_seed: 5,
        threads: 2,
        stop_on_kill: false,
        duration_ms: Some(60_000),
        boss,
        prevented_weight: gewicht,
        ..BalanceConfig::default()
    };

    for gewicht in [0.0_f64, 1.0_f64] {
        let c = bauen(gewicht);
        println!("\n######## prevented_weight = {gewicht} ########\n");
        let mut gewirkt = Vec::new();
        let mut tot = Vec::new();

        for (index, spell) in SPELLS.iter().enumerate() {
            let zeilen: Vec<_> = pool.iter().filter(|k| k.spell == index).cloned().collect();
            if zeilen.is_empty() {
                continue;
            }
            // **Jede Bedingung durchprobieren, die beste behalten.**
            let mut bestes: Option<(u32, f64, String)> = None;
            for zeile in &zeilen {
                let start = [zeile.clone()];
                let r = greedy_from(&c, &pool, 8, &start);
                let mut f = Fight::new(FightConfig {
                    seed: 5,
                    boss,
                    duration_ms: Some(60_000),
                    stop_on_kill: false,
                    ..FightConfig::default()
                });
                let casts = f.run_apl(&r.best.apl).casts[index];
                let besser = bestes.as_ref().is_none_or(|(c_alt, m_alt, _)| {
                    casts > *c_alt || (casts == *c_alt && r.best.mean > *m_alt)
                });
                if besser {
                    bestes = Some((casts, r.best.mean, zeile.rule().to_string()));
                }
            }
            let (casts, mean, regel) = bestes.expect("mindestens eine Zeile");
            if casts > 0 {
                gewirkt.push((spell.id, casts, mean, regel, zeilen.len()));
            } else {
                tot.push((spell.id, mean, zeilen.len()));
            }
        }

        println!("== GEWIRKT: der Zauber kommt in seiner besten Bedingung dran ==\n");
        for (id, casts, mean, regel, n) in &gewirkt {
            println!("  {id:<16} {casts:>3} Wirkungen   Rotation {mean:>7.0}   ({n} Zeilen geprueft)   beste: {regel}");
        }
        println!("\n== TOT: kommt in KEINER seiner Bedingungen dran ==\n");
        for (id, mean, n) in &tot {
            println!("  {id:<16}   0 Wirkungen   Rotation {mean:>7.0}   ({n} Zeilen geprueft)");
        }
        println!(
            "\n== Zusammen bei Gewicht {gewicht}: {} gewirkt, {} tot ==",
            gewirkt.len(),
            tot.len()
        );
    }
}
