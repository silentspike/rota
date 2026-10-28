//! Was ein paralleler Lauf mit Helfern aus dem Vorrat zusichern muss (#912).
//!
//! Seit #912 leben die Helfer über die Läufe und der Aufrufer rechnet einen
//! Bereich selbst. Zwei Dinge, die vorher die frischen Stränge je Lauf von
//! selbst sicherten, stehen deshalb hier als eigene Fälle.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, run_vollstaendig, BalanceConfig};
use rota_sim::fortschritt::Zaehlwerk;
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

/// **Bitgleich über jede Strangzahl — mit Würfel.** Die Strangvergleiche in
/// `tests/balance.rs` und in `rota-sim verify` fahren ohne Würfel; seit #908
/// schrumpft ein solcher Lauf auf einen Kampf und berührt den parallelen
/// Pfad gar nicht (`verify` meldet „Kaempfe gerechnet: 3"). Hier würfelt
/// jeder Kampf, und 997 Durchgänge teilen sich auf keine Strangzahl glatt auf.
#[test]
fn jede_strangzahl_rechnet_bitgleich_wie_einer_auch_mit_wuerfel() {
    let liste = gute_liste();
    let einer = run_vollstaendig(&konfiguration(997, 1, 5), &liste);
    for threads in [2usize, 3, 4, 8, 16] {
        let viele = run_vollstaendig(&konfiguration(997, threads, 5), &liste);
        assert_eq!(
            (
                viele.damage.mean.to_bits(),
                viele.damage_taken.mean.to_bits(),
                viele.healed.mean.to_bits(),
                viele.absorbed.mean.to_bits(),
                viele.damage.min.to_bits(),
                viele.damage.max.to_bits(),
                viele.kills,
                &viele.casts,
            ),
            (
                einer.damage.mean.to_bits(),
                einer.damage_taken.mean.to_bits(),
                einer.healed.mean.to_bits(),
                einer.absorbed.mean.to_bits(),
                einer.damage.min.to_bits(),
                einer.damage.max.to_bits(),
                einer.kills,
                &einer.casts,
            ),
            "{threads} Straenge rechnen anders als einer"
        );
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

/// **Die Strangzeit eines Bereichs landet auf seinem Platz** (#934). Sechs
/// Kämpfe auf vier Strängen sind die Bereiche 2, 2, 1, 1: Die Plätze 0 und 1
/// rechnen doppelt so viel wie 2 und 3. Die Zeit ist CPU-Zeit des Strangs
/// und hängt nicht daran, ob ein Strang verdrängt wurde.
#[test]
fn die_strangzeit_landet_auf_dem_platz_ihres_bereichs() {
    let werk = Zaehlwerk::neu(4);
    let anmeldung = werk.anmelden();
    let _ = run_vollstaendig(&konfiguration(6, 4, 7), &gute_liste());
    drop(anmeldung);
    let m = werk.momentaufnahme();
    assert_eq!(
        m.kaempfe_je_strang,
        vec![0, 2, 2, 1, 1],
        "die Kaempfe je Platz sind nicht die Bereiche 2, 2, 1, 1"
    );
    let s = &m.engpass.straenge;
    assert_eq!(s.len(), 4, "ein Eintrag je Bereich");
    for gross in [0, 1] {
        for klein in [2, 3] {
            assert!(
                s[gross].arbeit_ns > s[klein].arbeit_ns,
                "Platz {gross} (zwei Kaempfe) rechnete nicht laenger als Platz {klein} (einer): {s:?}"
            );
        }
    }
}

/// **Ein Helfer trägt keine Zählung in den nächsten Lauf.** Zweimal derselbe
/// Lauf mit eingeschalteten Kampfzählern ergibt zweimal dieselbe Zahl —
/// zählte ein Helfer weiter, stünde beim zweiten Lauf mehr.
#[cfg(feature = "diagnostics")]
#[test]
fn ein_helfer_traegt_keine_zaehlung_in_den_naechsten_lauf() {
    use rota_sim::diagnostics;
    let liste = gute_liste();
    let mut gezaehlt = Vec::new();
    for _ in 0..2 {
        diagnostics::reset();
        diagnostics::enable();
        let _ = run(&konfiguration(40, 4, 7), &liste);
        gezaehlt.push(diagnostics::snapshot().fights());
        diagnostics::disable();
        diagnostics::reset();
    }
    assert_eq!(gezaehlt, [40, 40], "Kaempfe je Lauf: {gezaehlt:?}");
}
