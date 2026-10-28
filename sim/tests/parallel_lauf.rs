//! Was ein paralleler Lauf mit Helfern aus dem Vorrat zusichern muss (#912).
//!
//! Seit #912 leben die Helfer über die Läufe und der Aufrufer rechnet einen
//! Bereich selbst. Zwei Dinge, die vorher die frischen Stränge je Lauf von
//! selbst sicherten, stehen deshalb hier als eigene Fälle.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig};
use rota_sim::gear::WornValues;

const GOOD: &str = include_str!("../apl/step1-good.apl");

fn gute_liste() -> Apl {
    parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
}

fn konfiguration(iterations: u32, threads: usize, base_seed: u32) -> BalanceConfig {
    // Mit Würfel, damit der Lauf über die Stränge verteilt wird (#908).
    let mut worn = WornValues::none();
    worn.purity = 500;
    BalanceConfig {
        iterations,
        threads,
        base_seed,
        boss: rota_sim::data::boss_by_id("boss-1-1").expect("den Boss gibt es"),
        worn,
        ..BalanceConfig::default()
    }
}

/// **Acht Aufrufer zugleich, jeder bekommt sein eigenes Ergebnis.** Die
/// Helfer sind geteilt; landete ein Bereich in der Sammlung eines fremden
/// Laufs, wiche dessen Ergebnis ab.
#[test]
fn gleichzeitige_laeufe_rechnen_jeder_sein_eigenes_ergebnis() {
    let liste = gute_liste();
    let konfigs: Vec<BalanceConfig> = (0..8u32)
        .map(|i| konfiguration(400, 4, 100 + i * 1000))
        .collect();
    let einzeln: Vec<_> = konfigs.iter().map(|c| run(c, &liste)).collect();
    // Positive Kontrolle: Verschiedene Startwerte geben verschiedene
    // Ergebnisse — sonst bewiese der Vergleich unten nichts.
    assert_ne!(
        einzeln[0].damage.mean.to_bits(),
        einzeln[1].damage.mean.to_bits(),
        "zwei Startwerte, dasselbe Ergebnis"
    );
    let gleichzeitig: Vec<Vec<_>> = std::thread::scope(|s| {
        let griffe: Vec<_> = konfigs
            .iter()
            .map(|c| {
                let liste = &liste;
                s.spawn(move || (0..3).map(|_| run(c, liste)).collect::<Vec<_>>())
            })
            .collect();
        griffe
            .into_iter()
            .map(|g| g.join().expect("ein Aufrufer ist abgestuerzt"))
            .collect()
    });
    for (nummer, (e, laeufe)) in einzeln.iter().zip(&gleichzeitig).enumerate() {
        for r in laeufe {
            assert_eq!(
                (
                    r.damage.mean.to_bits(),
                    r.damage_taken.mean.to_bits(),
                    r.damage.min.to_bits(),
                    r.damage.max.to_bits(),
                    &r.casts,
                ),
                (
                    e.damage.mean.to_bits(),
                    e.damage_taken.mean.to_bits(),
                    e.damage.min.to_bits(),
                    e.damage.max.to_bits(),
                    &e.casts,
                ),
                "Aufrufer {nummer}: gleichzeitig ein anderes Ergebnis als allein"
            );
        }
    }
}

/// **Der Trace-Hash bleibt einsträngig.** Er hängt an der Reihenfolge der
/// Zustände und wird über Stränge nicht vereinigt; vor #912 rechnete der
/// Aufrufer im parallelen Lauf nichts, und kein Zustand ging ein. Rechnet er
/// jetzt Bereich 0 mit, darf sein Trace davon nichts sehen.
#[cfg(feature = "diagnostics")]
#[test]
fn der_mitrechnende_aufrufer_laesst_den_trace_einstraengig() {
    use rota_sim::diagnostics;
    let liste = gute_liste();

    // Positive Kontrolle: Einsträngig nimmt der Trace Zustände auf.
    diagnostics::reset();
    diagnostics::enable_trace();
    let _ = run(&konfiguration(4, 1, 7), &liste);
    let allein = diagnostics::snapshot().trace_states();
    assert!(allein > 0, "der Trace nimmt einstraengig nichts auf");

    diagnostics::reset();
    let _ = run(&konfiguration(40, 4, 7), &liste);
    let parallel = diagnostics::snapshot().trace_states();
    assert!(
        diagnostics::is_trace_enabled(),
        "der parallele Lauf hat den Trace-Schalter nicht zurueckgestellt"
    );
    diagnostics::disable_trace();
    diagnostics::reset();
    assert_eq!(
        parallel, 0,
        "der parallele Lauf trug Zustaende in den Trace des Aufrufers"
    );
}
