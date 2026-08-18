//! MESSWERKZEUG DES ORCHESTRATORS — nicht committen.
//!
//! MESSUNG 8, ZWEITER ANLAUF: Macht eine bedingungslose Zeile alles unter sich
//! unerreichbar?
//!
//! # Warum der erste Anlauf nichts gemessen hat
//!
//! Er zaehlte `fight.is_ready(i)` waehrend eines Kampfes mit einer festen
//! Rotation. Ergebnis: 57 von 58 Zaubern „immer bereit" — trivialerweise, denn
//! ein Zauber, der nie gewirkt wird, laesst seine Abklingzeit nie anlaufen.
//! Die Zahl beantwortete die Frage „ist X bereit, wenn niemand X spielt".
//!
//! # Was hier gemessen wird
//!
//! Fuer jeden Zauber X eine Rotation aus GENAU ZWEI Zeilen:
//!
//! ```text
//! X            <- bedingungslos
//! <marker>     <- kommt nur dran, wenn X gerade NICHT geht
//! ```
//!
//! Wird der Marker nie gewirkt, ist jede Zeile unter X beweisbar unerreichbar —
//! und alle Permutationen darunter fallen aus dem Suchraum. Wird er gewirkt,
//! gilt die Reduktion fuer X nicht.

use rota_sim::apl::{parse, AplName};
use rota_sim::data::{boss_by_id, MAX_LEVEL, SPELLS, SPELL_COUNT};
use rota_sim::{Fight, FightConfig};

const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-2-3", "boss-3-1"];
/// Der Marker steht unter X. Er darf selbst nichts blockieren.
const MARKER: &str = "kindle";

fn main() {
    let spielbar: Vec<usize> = (0..SPELL_COUNT)
        .filter(|&i| u32::from(SPELLS[i].unlock_level) <= MAX_LEVEL)
        .collect();
    let marker_index = SPELLS
        .iter()
        .position(|s| s.id == MARKER)
        .expect("Marker gibt es");

    println!("Fuer jeden Zauber X: Rotation \"X / {MARKER}\", vier Bosse, Startwert 0.");
    println!("Der Marker kommt nur dran, wenn X gerade nicht geht.");
    println!();

    let mut sperrt_nie = Vec::new();
    let mut sperrt = Vec::new();

    for &i in &spielbar {
        let id = SPELLS[i].id;
        if id == MARKER {
            continue;
        }
        let text = format!("{id}\n{MARKER}\n");
        let apl = match parse(AplName("probe"), &text) {
            Ok(a) => a,
            Err(_) => continue, // Zauber, die allein keine gueltige Zeile ergeben
        };
        let mut marker_wirkungen = 0u32;
        let mut x_wirkungen = 0u32;
        let mut entscheidungen = 0u32;
        for bid in BOSSE {
            let boss = boss_by_id(bid).expect("Boss");
            let mut f = Fight::new(FightConfig {
                seed: 0,
                boss,
                ..FightConfig::default()
            });
            let r = f.run_apl(&apl);
            marker_wirkungen += r.casts[marker_index];
            x_wirkungen += r.casts[i];
            entscheidungen += r.casts.iter().sum::<u32>();
        }
        if marker_wirkungen == 0 {
            sperrt_nie.push((id, x_wirkungen));
        } else {
            let anteil = f64::from(marker_wirkungen) / f64::from(entscheidungen.max(1));
            sperrt.push((id, x_wirkungen, marker_wirkungen, anteil));
        }
    }

    println!(
        "SPERRT NIE — der Marker kam kein einziges Mal dran ({} von {}):",
        sperrt_nie.len(),
        spielbar.len() - 1
    );
    for (id, x) in &sperrt_nie {
        println!("  {id:<18} X selbst {x}x gewirkt");
    }
    println!();
    println!("LAESST DURCH ({}):", sperrt.len());
    let mut s = sperrt.clone();
    s.sort_by(|a, b| a.3.partial_cmp(&b.3).expect("Zahl"));
    for (id, x, m, a) in s.iter().take(12) {
        println!(
            "  {id:<18} X {x:>3}x   Marker {m:>3}x   Marker-Anteil {:.0} %",
            a * 100.0
        );
    }
    if s.len() > 12 {
        println!("  … und {} weitere", s.len() - 12);
    }

    println!();
    println!("== Was das fuer den Rotationsraum heisst ==");
    let sperrend = sperrt_nie.len();
    let n = 93usize;
    let mut voll = 1.0f64;
    for k in 0..12 {
        voll *= (n - k) as f64;
    }
    let rest = n - sperrend;
    let mut red = 0.0f64;
    for laenge in 0..=12usize {
        let mut p = 1.0f64;
        for k in 0..laenge {
            if rest <= k {
                p = 0.0;
                break;
            }
            p *= (rest - k) as f64;
        }
        if p <= 0.0 {
            continue;
        }
        red += p * if laenge < 12 {
            1.0 + sperrend as f64
        } else {
            1.0
        };
    }
    println!("  sperrende Zeilen                {sperrend}");
    println!("  Permutationen 93 ueber 12       {voll:.3e}");
    println!("  davon unterscheidbar            {red:.3e}");
    println!("  Faktor                          {:.1}", voll / red);
}
