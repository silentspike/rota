//! Der Balance-Läufer (Issue #56, AC1, AC2, AC4).
//!
//! AC3 — Skalierungsfaktoren für ESSENCE, CATALYSIS, PURITY, YIELD — fehlt hier
//! bewusst: Die Umrechnung eines Wert-Punkts in einen Faktor ist im Design nicht
//! festgelegt (GAME-DESIGN 6.1 bis 6.6 nennt keine Formel), und Krit wie Tempo
//! sind auch im Spiel noch nicht umgesetzt (`src/core/gcd.ts`: „Ab Epic E5 von
//! CATALYSIS beeinflusst"). Gemeldet, nicht geraten — `WORKER-GUIDE` § 7.
//!
//! Was hier steht, ist von der Lücke unberührt: die Maschinerie des Läufers.

use std::time::Instant;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig, Statistics};
use rota_sim::gear::WornValues;

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");

fn apl() -> Apl {
    parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
}

/// Getragene Werte mit Kritchance, damit jeder Durchgang würfelt.
///
/// **Ohne Würfel schrumpft ein Lauf seit #908 auf einen Kampf** und berührt
/// den parallelen Pfad gar nicht. Die vier Strangvergleiche unten liefen bis
/// #891 N6 ohne Würfel — und blieben ohne die Sortierung nach Startwert in
/// `balance::run` grün (gemessen am 29.10., Job dfe079cf).
fn wuerfelnd() -> WornValues {
    let mut worn = WornValues::none();
    worn.purity = 500;
    worn
}

#[test]
fn ac1_zwei_laeufe_mit_gleicher_konfiguration_ergeben_denselben_mittelwert() {
    let config = BalanceConfig {
        iterations: 500,
        base_seed: 12345,
        threads: 4,
        worn: wuerfelnd(),
        ..BalanceConfig::default()
    };
    let apl = apl();

    let a = run(&config, &apl);
    let b = run(&config, &apl);
    assert!(
        a.damage.stddev > 0.0,
        "der Lauf streut nicht — kein Wuerfel"
    );

    // "bis auf die letzte Stelle" aus der AC heisst hier: **exakt gleich**. Ein
    // Mittelwert, der sich im letzten Bit unterscheidet, waere schon der Beleg
    // dafuer, dass die Summe von der Strang-Reihenfolge abhaengt.
    assert_eq!(
        a.damage.mean.to_bits(),
        b.damage.mean.to_bits(),
        "die Mittelwerte unterscheiden sich: {} gegen {}",
        a.damage.mean,
        b.damage.mean
    );
    assert_eq!(a.damage.stddev.to_bits(), b.damage.stddev.to_bits());
    assert_eq!(a.damage.stderr.to_bits(), b.damage.stderr.to_bits());
    assert_eq!(
        a.samples, b.samples,
        "auch die Einzelwerte muessen gleich sein"
    );
    assert_eq!(a.kills, b.kills);
}

#[test]
fn ac2_ein_kern_und_viele_kerne_ergeben_dasselbe() {
    // **Der Test, für den die Sortier-Regel aus 18.4 da ist.** Ohne sie kaeme
    // der Mittelwert in einer Reihenfolge zusammen, die an der Systemlast
    // haengt — und dieser Vergleich schluege gelegentlich fehl, aber nicht
    // zuverlaessig. Das waere der unangenehmste Fehler von allen.
    let apl = apl();
    let basis = BalanceConfig {
        iterations: 997, // Primzahl: teilt sich auf keine Strangzahl glatt auf
        base_seed: 777,
        threads: 1,
        worn: wuerfelnd(),
        ..BalanceConfig::default()
    };

    let ein_kern = run(&basis, &apl);
    assert!(
        ein_kern.damage.stddev > 0.0,
        "der Lauf streut nicht — kein Wuerfel"
    );

    for threads in [2, 3, 4, 8, 16] {
        let viele = run(&BalanceConfig { threads, ..basis }, &apl);
        assert_eq!(
            ein_kern.damage.mean.to_bits(),
            viele.damage.mean.to_bits(),
            "{threads} Straenge ergeben einen anderen Mittelwert als einer"
        );
        assert_eq!(
            ein_kern.samples, viele.samples,
            "{threads} Straenge ergeben andere Einzelwerte"
        );
        assert_eq!(ein_kern.kills, viele.kills);
        assert_eq!(
            viele.samples.len(),
            997,
            "bei ungleicher Aufteilung darf kein Durchgang verlorengehen"
        );
    }
}

#[test]
fn jeder_durchgang_haengt_an_seinem_startwert() {
    // Die Voraussetzung dafuer, dass Sortieren ueberhaupt hilft: Der Startwert
    // bestimmt den Durchgang vollstaendig. Waere das nicht so, liesse sich ein
    // auffaelliger Durchgang auch nicht einzeln nachrechnen.
    let apl = apl();
    let a = run(
        &BalanceConfig {
            iterations: 50,
            base_seed: 1000,
            threads: 4,
            worn: wuerfelnd(),
            ..BalanceConfig::default()
        },
        &apl,
    );
    let b = run(
        &BalanceConfig {
            iterations: 50,
            base_seed: 1000,
            threads: 1,
            worn: wuerfelnd(),
            ..BalanceConfig::default()
        },
        &apl,
    );
    assert!(
        b.damage.stddev > 0.0,
        "der Lauf streut nicht — kein Wuerfel"
    );
    for (links, rechts) in a.samples.iter().zip(&b.samples) {
        assert_eq!(links.seed, rechts.seed);
        assert_eq!(links.damage, rechts.damage);
        assert_eq!(links.killed_at_ms, rechts.killed_at_ms);
    }
}

#[test]
fn die_ergebnisse_kommen_nach_startwert_sortiert_zurueck() {
    let apl = apl();
    let result = run(
        &BalanceConfig {
            iterations: 200,
            base_seed: 5,
            threads: 8,
            worn: wuerfelnd(),
            ..BalanceConfig::default()
        },
        &apl,
    );
    assert!(
        result.damage.stddev > 0.0,
        "der Lauf streut nicht — kein Wuerfel"
    );
    let seeds: Vec<u32> = result.samples.iter().map(|s| s.seed).collect();
    let mut sortiert = seeds.clone();
    sortiert.sort_unstable();
    assert_eq!(seeds, sortiert, "die Reihenfolge haengt an den Straengen");
    assert_eq!(seeds.first(), Some(&5));
    assert_eq!(seeds.last(), Some(&204));
}

#[test]
fn in_schritt_1_streut_nichts() {
    // Kein Krit heisst: kein Zufall. Alle Durchgaenge sind identisch, der
    // Fehlerbalken ist null. Das ist **kein Mangel des Laeufers**, sondern der
    // Zustand von Schritt 1 (STEP1-SPEC § 2) — und es ist der Grund, warum die
    // Statistik zusaetzlich mit synthetischen Zahlen geprueft wird
    // (`src/balance.rs`, Modul-Tests).
    let apl = apl();
    let result = run(
        &BalanceConfig {
            iterations: 100,
            threads: 4,
            ..BalanceConfig::default()
        },
        &apl,
    );
    assert!((result.damage.mean - 4240.0).abs() < 1e-9);
    assert_eq!(result.damage.stddev, 0.0);
    assert_eq!(result.damage.ci95, 0.0);
    assert_eq!(result.kills, 100);
    assert!((result.kill_time_ms.mean - 40_510.0).abs() < 1e-9);
    assert!((result.kill_rate() - 1.0).abs() < 1e-12);
}

#[test]
fn ac4_zehntausend_durchgaenge_in_vertretbarer_zeit() {
    // Die AC verlangt eine **gemessene** Dauer, keine Schaetzung. Gemessen wird
    // hier im Debug-Bau; im Release ist es ein Vielfaches schneller. Die Schranke
    // ist deshalb grosszuegig gesetzt — sie soll einen Einbruch um
    // Groessenordnungen melden, nicht die Schwankung einer belegten Maschine.
    let apl = apl();
    let config = BalanceConfig {
        iterations: 10_000,
        base_seed: 0,
        threads: 4,
        ..BalanceConfig::default()
    };

    let start = Instant::now();
    let result = run(&config, &apl);
    let dauer = start.elapsed();

    assert_eq!(result.samples.len(), 10_000);
    assert!(
        dauer.as_secs() < 60,
        "10.000 Durchgaenge brauchten {dauer:?} — erwartet werden Sekunden, nicht Minuten"
    );
    println!(
        "10.000 Durchgaenge ueber {} Straenge in {:?} ({:.0} je Sekunde)",
        config.threads,
        dauer,
        10_000.0 / dauer.as_secs_f64()
    );
}

#[test]
fn die_json_ausgabe_ist_lesbar_und_vollstaendig() {
    let apl = apl();
    let result = run(
        &BalanceConfig {
            iterations: 10,
            ..BalanceConfig::default()
        },
        &apl,
    );
    let json = result.to_json();
    let wert: serde_json::Value = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("kein gueltiges JSON: {error}\n{json}"));

    assert_eq!(wert["apl"], "step1-good");
    assert_eq!(wert["iterations"], 10);
    assert_eq!(wert["threads"], 1);
    assert_eq!(wert["damage"]["count"], 10);
    assert!((wert["damage"]["mean"].as_f64().expect("Zahl") - 4240.0).abs() < 1e-9);
    assert_eq!(wert["damage"]["stddev"], 0.0);
    assert_eq!(wert["kills"], 10);
    assert!((wert["killTimeMs"]["mean"].as_f64().expect("Zahl") - 40_510.0).abs() < 1e-9);
}

#[test]
fn der_fehlerbalken_schrumpft_mit_der_wurzel_aus_n() {
    // Die Eigenschaft, auf die es beim Monte-Carlo ankommt: Viermal so viele
    // Durchgaenge halbieren den Fehlerbalken. Geprueft an synthetischen Werten,
    // weil Schritt 1 nicht streut.
    let schmal: Vec<f64> = (0..100).map(|i| f64::from(i % 7)).collect();
    let breit: Vec<f64> = (0..400).map(|i| f64::from(i % 7)).collect();
    let a = Statistics::from_sorted(&schmal);
    let b = Statistics::from_sorted(&breit);

    let verhaeltnis = a.ci95 / b.ci95;
    assert!(
        (verhaeltnis - 2.0).abs() < 0.05,
        "vierfache Anzahl sollte den Fehlerbalken halbieren, Verhaeltnis war {verhaeltnis:.3}"
    );
}
