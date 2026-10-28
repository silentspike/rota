//! Welche CPUs ein Lauf nach `reserved_physical_cores` benutzt — aus der
//! sysfs-Topologie DIESER Maschine (Issue #914, AC4).
//!
//! Auf dem Buildserver bauen, auf der Zielmaschine laufen lassen — die
//! Topologie des Buildservers ist nicht die des Laptops:
//! ```text
//! cargo remote -c -- build --release --example e21-0-cpus
//! ./target/release/examples/e21-0-cpus                    # Anzeige und Bericht
//! ./target/release/examples/e21-0-cpus --compute-liste    # nur die Liste für taskset
//! ```
//!
//! Die Gegenprobe mit `taskset`: Läuft das Programm auf genau den
//! Compute-CPUs, meldet `--pruefe-bindung` Exit 0; auf jeder anderen Menge
//! Exit 1. Damit ist belegt, dass die Nummern echte CPU-Nummern dieser
//! Maschine sind, die der Kernel als Bindung annimmt und zurückmeldet:
//! ```text
//! taskset -c 0-11 ./target/release/examples/e21-0-cpus --pruefe-bindung   # 0
//! taskset -c 0-12 ./target/release/examples/e21-0-cpus --pruefe-bindung   # 1
//! ./target/release/examples/e21-0-cpus --pruefe-bindung                   # 1, ungebunden
//! ```
//!
//! Exit 2 heißt: Es wurde nichts geprüft — Topologie oder Bindung nicht lesbar,
//! oder eine ungültige Angabe. Das ist kein Fehlschlag der Prüfung, sondern
//! ihr Ausbleiben, und es darf nicht wie einer aussehen.

use std::process::ExitCode;

use rota_sim::hostcaps::erlaubte_kerne;
use rota_sim::kampagne::{cpu_list, CpuTopology};

/// Die Reservierung des Betriebsprofils `balanced` auf dem Laptop (Plan E15.1,
/// „Regelung: von außen und von selbst"). Im Referenzprofil „standard" rechnet
/// der Laptop seit dem 2026-10-28 nicht mehr mit; diese Zahl gilt für einen
/// Lauf, in dem er rechnet.
const RESERVIERT_VORGABE: u16 = 2;

enum Modus {
    Anzeige,
    ComputeListe,
    PruefeBindung,
}

fn main() -> ExitCode {
    let mut reserviert = RESERVIERT_VORGABE;
    let mut modus = Modus::Anzeige;
    let mut argumente = std::env::args().skip(1);
    while let Some(arg) = argumente.next() {
        match arg.as_str() {
            "--reserve" => {
                let Some(wert) = argumente.next().and_then(|w| w.parse().ok()) else {
                    eprintln!("--reserve braucht eine Zahl physischer Kerne");
                    return ExitCode::from(2);
                };
                reserviert = wert;
            }
            "--compute-liste" => modus = Modus::ComputeListe,
            "--pruefe-bindung" => modus = Modus::PruefeBindung,
            "--help" | "-h" => {
                println!(
                    "e21-0-cpus [--reserve N] [--compute-liste | --pruefe-bindung]\n\
                     Liest /sys/devices/system/cpu und reserviert N physische Kerne\n\
                     samt SMT-Geschwistern (Vorgabe {RESERVIERT_VORGABE}).\n\
                     --pruefe-bindung: Exit 0, wenn dieser Prozess genau auf den\n\
                     Compute-CPUs läuft; 1, wenn nicht; 2, wenn nichts prüfbar war."
                );
                return ExitCode::SUCCESS;
            }
            anders => {
                eprintln!("unbekannte Angabe: {anders}; siehe --help");
                return ExitCode::from(2);
            }
        }
    }

    let belegung = match CpuTopology::from_sysfs().and_then(|t| t.assign(reserviert)) {
        Ok(b) => b,
        Err(fehler) => {
            eprintln!("BLOCKED: {fehler}");
            return ExitCode::from(2);
        }
    };

    if let Modus::ComputeListe = modus {
        println!("{}", cpu_list(belegung.compute_cpus()));
        return ExitCode::SUCCESS;
    }

    // Was der Kernel diesem Prozess tatsächlich erlaubt — die Bindung, die
    // `taskset` gesetzt hat, oder alle CPUs, wenn niemand gebunden hat.
    let Some(erlaubt) = erlaubte_kerne() else {
        eprintln!("BLOCKED: Cpus_allowed_list aus /proc/self/status nicht lesbar");
        return ExitCode::from(2);
    };
    let mut erlaubt: Vec<u32> = match erlaubt.into_iter().map(u32::try_from).collect() {
        Ok(liste) => liste,
        Err(_) => {
            eprintln!("BLOCKED: CPU-Nummer jenseits von u32");
            return ExitCode::from(2);
        }
    };
    erlaubt.sort_unstable();
    let passt = erlaubt.as_slice() == belegung.compute_cpus();

    if let Modus::Anzeige = modus {
        println!("{}", belegung.describe());
        println!("Bericht:     {}", belegung.to_json());
    }
    println!(
        "Bindung:     Cpus_allowed_list {} — {}",
        cpu_list(&erlaubt),
        if passt {
            "entspricht der Compute-Menge"
        } else {
            "entspricht NICHT der Compute-Menge"
        }
    );
    // Nur die Prüfung entscheidet über den Ausgang; die Anzeige meldet die
    // Bindung, ohne ungebundenes Laufen zum Fehler zu machen.
    match modus {
        Modus::PruefeBindung if !passt => ExitCode::FAILURE,
        _ => ExitCode::SUCCESS,
    }
}
