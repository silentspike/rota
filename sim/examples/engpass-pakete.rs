//! Die Paketlage für die Engpasserkennung (Issue #934, AC6): viele kleine
//! Auswertungen, weniger Pakete als Stränge.
//!
//! # Warum ein eigenes Programm
//!
//! „Stränge warten über 15 %" braucht viele parallele Abschnitte über
//! mindestens vier Messsekunden. `rota-sim` hat heute keinen Befehl, der das
//! liefert: `bench` und `run --search none` sind **eine** Auswertung, und die
//! gierige Suche endet nach Zehntelsekunden, sobald keine Zeile mehr trägt
//! (gemessen am 2026-10-28 auf node3: 0,13 bis 0,17 s). Dieses Programm
//! wiederholt deshalb dieselbe Auswertung, bis die Zeit um ist — über genau
//! den Weg, den `rota-sim` geht: `balance::run`, angemeldetes Zählwerk,
//! [`Erhebung`], Anzeigestrang, JSON-Strom.
//!
//! ```text
//! cargo remote -c -- build --release --example engpass-pakete
//! target/release/examples/engpass-pakete --iterations 4 --threads 8 --sekunden 15 --jsonl lage.jsonl
//! target/release/examples/engpass-pakete --iterations 400 --threads 8 --sekunden 15 --jsonl kontrolle.jsonl
//! ```
//!
//! Mit vier Durchgängen auf acht Strängen bekommen vier Stränge kein Paket
//! und warten jeden Abschnitt ganz — „zu wenige Pakete". Mit vierhundert
//! rechnet jeder Strang fünfzig Kämpfe; gewartet wird nur, bis der
//! langsamste fertig ist. Beide Läufe unterscheiden sich in genau einer
//! Angabe.
//!
//! Gewürfelt wird mit PURITY 500, damit keine Auswertung auf einen Kampf
//! schrumpft (#908) und jede über die Stränge verteilt wird.

use std::sync::Arc;
use std::time::{Duration, Instant};

use rota_sim::apl::{parse, AplName};
use rota_sim::balance::{run, BalanceConfig};
use rota_sim::engpass::{Erhebung, Grundlage};
use rota_sim::fortschritt::{
    Anzeigestrang, Arbeitsmenge, Ausgabe, Kampfzahl, Kostenbild, Laufbeschreibung, Weg, Zaehlwerk,
};
use rota_sim::gear::WornValues;

fn main() {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    let wert = |name: &str| -> Option<String> {
        argumente
            .iter()
            .position(|a| a == name)
            .and_then(|i| argumente.get(i + 1).cloned())
    };
    let zahl = |name: &str| -> u64 {
        wert(name).and_then(|w| w.parse().ok()).unwrap_or_else(|| {
            eprintln!(
                "Aufruf: engpass-pakete --iterations N --threads T --sekunden S --jsonl PFAD \
                 [--baseline G]"
            );
            std::process::exit(2)
        })
    };
    let iterations = u32::try_from(zahl("--iterations")).expect("passt in u32");
    let threads = usize::try_from(zahl("--threads")).expect("passt in usize");
    let sekunden = zahl("--sekunden");
    let jsonl = wert("--jsonl").expect("--jsonl PFAD");

    let mut worn = WornValues::none();
    worn.purity = 500;
    let config = BalanceConfig {
        iterations,
        threads,
        boss: rota_sim::data::boss_by_id("boss-1-1").expect("Boss"),
        worn,
        ..BalanceConfig::default()
    };
    let apl = parse(AplName("step1-good"), include_str!("../apl/step1-good.apl")).expect("Liste");

    let werk = Zaehlwerk::neu(threads);
    let _anmeldung = werk.anmelden();
    werk.setze_menge(Arbeitsmenge::Unbekannt);
    let grundlage = wert("--baseline").map(|pfad| {
        Grundlage::lesen(&std::fs::read_to_string(&pfad).expect("Grundlage lesbar"))
            .expect("Grundlage gültig")
    });
    let mut beschreibung = Laufbeschreibung::neu(
        "pakete",
        Some("boss-1-1"),
        threads,
        Kostenbild {
            weg: Weg::Stichprobe,
            kaempfe_je_kandidat: Kampfzahl::Fest(u64::from(iterations)),
        },
    );
    // Dieselbe Reihenfolge wie in `rota-sim`: Erhebung vor Anzeigestrang und
    // vor dem ersten Arbeitsstrang.
    rota_sim::diagnostics::enable();
    beschreibung.engpass = Some(Arc::new(Erhebung::starten(
        &beschreibung.host_id,
        &format!("pakete boss=boss-1-1 iterations={iterations} purity=500 threads={threads}"),
        grundlage,
    )));
    let anzeige = Anzeigestrang::starten(
        Arc::clone(&werk),
        beschreibung,
        Ausgabe {
            zeile: None,
            terminal: false,
            jsonl: Some(Box::new(std::io::BufWriter::new(
                std::fs::File::create(&jsonl).expect("Strom anlegbar"),
            ))),
        },
    );

    let ende = Instant::now() + Duration::from_secs(sekunden);
    let mut auswertungen = 0u64;
    while Instant::now() < ende {
        let _ = run(&config, &apl);
        auswertungen += 1;
    }
    let letzte = anzeige.beenden().expect("Anzeige");
    println!(
        "{auswertungen} Auswertungen, {} Kämpfe",
        werk.momentaufnahme().kaempfe
    );
    if let Some(bild) = &letzte.engpass {
        println!("{}", bild.diagnose.satz());
    }
}
