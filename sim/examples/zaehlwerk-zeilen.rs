//! Kostet eine geteilte Cache-Zeile messbar etwas? (#933, AC9)
//!
//! ```text
//! # BAUEN — entfernt:
//! cargo remote -c -- build --release --example zaehlwerk-zeilen
//!
//! # FAHREN — hier, gebunden, im Messfenster:
//! taskset -c 0-7 ./target/release/examples/zaehlwerk-zeilen
//! ```
//!
//! # Wozu es dieses Beispiel gibt
//!
//! AC9 verlangt getrennte Cache-Zeilen je Strang, „sonst kostet die Anzeige
//! genau die Bandbreite, die sie messen soll". Ob sie getrennt LIEGEN, prüft
//! `tests/fortschritt.rs` an den Adressen. Ob das hier überhaupt etwas
//! AUSMACHT, prüft nur eine Messung — und ohne sie wäre die A/B-Reihe der
//! Messumgebung eine Null ohne positive Kontrolle: „nicht unterscheidbar"
//! hieße dann nicht „die Anordnung trägt", sondern womöglich „die Messung
//! sieht so etwas nie".
//!
//! Drei Anordnungen, dieselbe Arbeit — jeder Strang erhöht seinen Zähler
//! [`ERHOEHUNGEN`]-mal, sonst nichts:
//!
//! ```text
//!   getrennt     Zählwerk, jeder Strang auf seinem Platz        (so wird gezählt)
//!   eine Zeile   Zählwerk mit einem Platz für alle Stränge      (Platzmangel)
//!   gepackt      acht AtomicU64 nebeneinander, ohne Abstand     (die verbotene Form)
//! ```
//!
//! **Was es NICHT zeigt:** die Kosten im echten Lauf. Dort liegt zwischen zwei
//! Erhöhungen ein ganzer Kampf, hier nichts. Das ist der schlechteste Fall,
//! und genau deshalb die Kontrolle: Sieht man hier keinen Unterschied, kann
//! ihn der echte Lauf erst recht nicht zeigen.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Instant;

use rota_sim::fortschritt::{strang, Strang, Zaehlwerk};
use rota_sim::hostcaps::erlaubte_kerne;

/// Erhöhungen je Strang und Durchgang.
const ERHOEHUNGEN: u64 = 20_000_000;

/// Durchgänge je Anordnung, im Wechsel gefahren.
const DURCHGAENGE: usize = 5;

/// Acht Zähler ohne Abstand — eine Cache-Zeile für alle.
#[derive(Default)]
struct Gepackt {
    zaehler: [AtomicU64; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Anordnung {
    Getrennt,
    EineZeile,
    Gepackt,
}

impl Anordnung {
    const fn name(self) -> &'static str {
        match self {
            Self::Getrennt => "getrennt  ",
            Self::EineZeile => "eine Zeile",
            Self::Gepackt => "gepackt   ",
        }
    }
}

/// Ein Durchgang: Nanosekunden je Erhöhung, gemittelt über die Stränge.
fn durchgang(anordnung: Anordnung, straenge: usize) -> (f64, u64) {
    let start = Arc::new(Barrier::new(straenge + 1));
    let gepackt = Arc::new(Gepackt::default());
    // Das Zählwerk wird für „eine Zeile" mit null Arbeitsplätzen angelegt —
    // `neu` gibt dann genau einen, und `arbeiter` legt alle Stränge darauf.
    let werk = Zaehlwerk::neu(if anordnung == Anordnung::EineZeile {
        0
    } else {
        straenge
    });
    let anmeldung = werk.anmelden();
    let leit: Strang = strang().expect("eben angemeldet");
    let griffe: Vec<_> = (0..straenge)
        .map(|nummer| {
            let start = Arc::clone(&start);
            let gepackt = Arc::clone(&gepackt);
            let arbeiter = leit.arbeiter(nummer);
            std::thread::spawn(move || {
                start.wait();
                match anordnung {
                    Anordnung::Getrennt | Anordnung::EineZeile => {
                        for _ in 0..ERHOEHUNGEN {
                            arbeiter.kampf();
                        }
                    }
                    Anordnung::Gepackt => {
                        let z = &gepackt.zaehler[nummer % 8];
                        for _ in 0..ERHOEHUNGEN {
                            z.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            })
        })
        .collect();
    let beginn = Instant::now();
    start.wait();
    for g in griffe {
        g.join().expect("ein Zählstrang ist abgestürzt");
    }
    let ns = beginn.elapsed().as_nanos() as f64 / ERHOEHUNGEN as f64;
    drop(anmeldung);
    // Die Zusicherung: gezählt wurde, was gezählt werden sollte.
    let gezaehlt = match anordnung {
        Anordnung::Gepackt => gepackt
            .zaehler
            .iter()
            .map(|z| z.load(Ordering::Relaxed))
            .sum(),
        _ => werk.momentaufnahme().kaempfe,
    };
    (ns, gezaehlt)
}

fn median(mut werte: Vec<f64>) -> f64 {
    werte.sort_by(f64::total_cmp);
    let n = werte.len();
    if n % 2 == 1 {
        werte[n / 2]
    } else {
        (werte[n / 2 - 1] + werte[n / 2]) / 2.0
    }
}

fn main() {
    let straenge = erlaubte_kerne().map_or(1, |k| k.len()).clamp(2, 8);
    println!("ZÄHLWERK-ZEILEN (#933, AC9)");
    println!("  Stränge {straenge}, je {ERHOEHUNGEN} Erhöhungen, {DURCHGAENGE} Durchgänge je Anordnung im Wechsel\n");
    let alle = [
        Anordnung::Getrennt,
        Anordnung::EineZeile,
        Anordnung::Gepackt,
    ];
    let mut werte: Vec<Vec<f64>> = vec![Vec::new(); alle.len()];
    let soll = ERHOEHUNGEN * straenge as u64;
    for d in 0..DURCHGAENGE {
        for (i, a) in alle.iter().enumerate() {
            let (ns, gezaehlt) = durchgang(*a, straenge);
            if gezaehlt != soll {
                println!("  BLOCKED — {} zählte {gezaehlt} statt {soll}", a.name());
                std::process::exit(2);
            }
            println!(
                "  Durchgang {} {}  {ns:>7.3} ns je Erhöhung",
                d + 1,
                a.name()
            );
            werte[i].push(ns);
        }
    }
    println!();
    let getrennt = median(werte[0].clone());
    for (i, a) in alle.iter().enumerate() {
        let min = werte[i].iter().copied().fold(f64::MAX, f64::min);
        let max = werte[i].iter().copied().fold(f64::MIN, f64::max);
        let m = median(werte[i].clone());
        println!(
            "  {}  Median {m:>7.3} ns   Spanne {min:.3}–{max:.3}   Faktor gegen getrennt {:.2}",
            a.name(),
            m / getrennt
        );
    }
}
