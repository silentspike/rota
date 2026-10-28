//! Künstliche Lagen für die Engpasserkennung (Issue #934, AC6).
//!
//! **Eine Last NEBEN dem Simulator, nicht in ihm.** Sie läuft als eigener
//! Prozess auf den Geschwistersträngen der Kerne, auf denen `rota-sim`
//! rechnet, und teilt sich mit ihm, was ein Kern zwei Strängen gibt: den
//! L1- und L2-Speicher, die Sprungvorhersage, die Rechenwerke. Der Lauf
//! selbst bleibt unverändert — dieselbe Konfiguration, dieselben Kämpfe,
//! dieselbe Grundlage —, nur die Bedingung um ihn herum wird zur Lage.
//!
//! Die Leistungszähler sehen sie nicht: Sie zählen nur die Stränge von
//! `rota-sim` (`perf_event_open` mit `pid = 0` und `inherit`), nicht einen
//! fremden Prozess. Was sich ändert, ist allein, was die Stränge des Laufs
//! erleben.
//!
//! ```text
//! cargo remote -c -- build --release --example engpass-last
//! taskset -c 1,3,5,7 target/release/examples/engpass-last speicher 2048 60 &
//! taskset -c 0,2,4,6 target/release/rota-sim bench --threads 4 … --baseline g.txt
//! ```
//!
//! | Lage | was sie tut | welche Zeile sie treffen soll |
//! | --- | --- | --- |
//! | `speicher <KiB> <s>` | schreibt je Strang einen Puffer der Größe immer wieder ganz | Zustand zu groß für die zweite Ebene |
//! | `spruenge <s>` | springt nach Zufallsbits durch vier nie eingebettete Funktionen | verzweigungsgebunden |
//!
//! Die Puffergröße wählt, welche Ebene leidet: größer als L2 (512 KiB auf dem
//! Laptop), deutlich kleiner als L3 (16 MiB) — dann landet, was aus L2
//! verdrängt wird, im L3, und die dritte Ebene bleibt ruhig.
//!
//! Gestartet werden so viele Stränge, wie `taskset` CPUs erlaubt. Jede
//! Sekunde steht auf der Standardausgabe, wie viel jeder bis dahin getan hat
//! — auch wenn die Last von außen beendet wird. Eine Last, die nicht
//! gerechnet hat, ist keine Lage.

use std::hint::black_box;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

fn main() {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    let art = argumente.first().map_or("", String::as_str);
    let zahl = |i: usize| -> u64 {
        argumente
            .get(i)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                eprintln!("Aufruf: engpass-last speicher <KiB> <Sekunden> | spruenge <Sekunden>");
                std::process::exit(2)
            })
    };
    let (kib, sekunden) = match art {
        "speicher" => (zahl(1), zahl(2)),
        "spruenge" => (0, zahl(1)),
        _ => {
            eprintln!("Aufruf: engpass-last speicher <KiB> <Sekunden> | spruenge <Sekunden>");
            std::process::exit(2)
        }
    };
    let straenge = std::thread::available_parallelism().map_or(1, |n| n.get());
    let beginn = Instant::now();
    let ende = beginn + Duration::from_secs(sekunden);
    let geleistet: &'static [AtomicU64] =
        Box::leak((0..straenge).map(|_| AtomicU64::new(0)).collect());
    let griffe: Vec<_> = (0..straenge)
        .map(|nummer| {
            let art = art.to_owned();
            let zaehler = &geleistet[nummer];
            std::thread::spawn(move || match art.as_str() {
                "speicher" => speicher(kib, ende, zaehler),
                _ => spruenge(nummer as u64 + 1, ende, zaehler),
            })
        })
        .collect();
    // Jede Sekunde der Stand: Wird die Last von außen beendet, steht trotzdem
    // da, was sie bis dahin getan hat.
    while Instant::now() < ende {
        std::thread::sleep(Duration::from_secs(1).min(ende - Instant::now().min(ende)));
        let stand: Vec<String> = geleistet
            .iter()
            .map(|z| z.load(Ordering::Relaxed).to_string())
            .collect();
        println!(
            "{:>5.1} s  {art} je Strang: {}",
            beginn.elapsed().as_secs_f64(),
            stand.join(" ")
        );
        let _ = std::io::stdout().flush();
    }
    for griff in griffe {
        griff.join().expect("Strang");
    }
}

/// Schreibt einen Puffer von `kib` KiB immer wieder ganz, eine Cache-Zeile
/// je Zugriff, und zählt die Durchgänge.
fn speicher(kib: u64, ende: Instant, durchgaenge: &AtomicU64) {
    let mut puffer = vec![0u64; (kib * 1024 / 8) as usize];
    while Instant::now() < ende {
        for zeile in puffer.iter_mut().step_by(8) {
            *zeile = zeile.wrapping_add(1);
        }
        black_box(&puffer);
        durchgaenge.fetch_add(1, Ordering::Relaxed);
    }
}

#[inline(never)]
fn a(x: u64) -> u64 {
    x.wrapping_mul(3)
}
#[inline(never)]
fn b(x: u64) -> u64 {
    x ^ 0x5555
}
#[inline(never)]
fn c(x: u64) -> u64 {
    x.rotate_left(7)
}
#[inline(never)]
fn d(x: u64) -> u64 {
    x.wrapping_add(0x9e37)
}

/// Springt nach Zufallsbits (xorshift64) in eine von vier Funktionen — eine
/// Folge, die keine Vorhersage lernen kann — und zählt die Sprünge.
fn spruenge(saat: u64, ende: Instant, spruenge: &AtomicU64) {
    let mut x = 0x2545_f491_4f6c_dd1d_u64 ^ saat;
    let mut summe = 0u64;
    while Instant::now() < ende {
        for _ in 0..4096 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            summe = match black_box(x >> 61) & 3 {
                0 => a(summe),
                1 => b(summe),
                2 => c(summe),
                _ => d(summe),
            };
            if black_box(x >> 40) & 1 == 1 {
                summe = summe.wrapping_add(x);
            }
        }
        spruenge.fetch_add(4096, Ordering::Relaxed);
    }
    black_box(summe);
}
