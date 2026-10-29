//! Die Stränge eines parallelen Laufs (#912).
//!
//! **Eine Datei, ein Prüffall.** Die Zählung der Strangstarts ist eine
//! Prozessgröße; liefe ein zweiter Fall daneben, zählte sie seine Stränge
//! mit, und der Fall wüsste nicht, wessen Stränge er sieht.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, straenge_gestartet, BalanceConfig};
use rota_sim::gear::WornValues;

const GOOD: &str = include_str!("../apl/step1-good.apl");

fn gute_liste() -> Apl {
    parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
}

fn konfiguration(iterations: u32, threads: usize) -> BalanceConfig {
    // Mit Würfel, damit der Lauf nicht auf einen Kampf schrumpft (#908) und
    // wirklich über die Stränge verteilt wird.
    let mut worn = WornValues::none();
    worn.purity = 500;
    BalanceConfig {
        iterations,
        threads,
        boss: rota_sim::data::boss_by_id("boss-1-1").expect("den Boss gibt es"),
        worn,
        ..BalanceConfig::default()
    }
}

/// **Der Aufrufer rechnet mit, und die Helfer leben über die Läufe.**
///
/// Gemessen am 2026-10-28 auf node3: Ein 20k-Paket mit 16 Strängen startete
/// 16 neue Stränge (`ns_last_pid` im Container, Kontrolle 16 → 16), und je
/// Auswertung waren Aufrufer und vier Helfer auf vier gebundenen Kernen
/// lauffähig — PSI cpu 15–20 %, `engpass-pakete` setzte seine Zeitdiagnose
/// aus, `messumgebung` verwarf 20 von 20 Blöcken.
///
/// Verlangt sind deshalb zwei Dinge: Ein Lauf mit vier Strängen braucht
/// drei Helfer, weil der Aufrufer selbst einen Bereich rechnet; und weitere
/// Läufe starten keinen Strang mehr.
#[test]
fn der_aufrufer_rechnet_mit_und_die_helfer_leben_ueber_die_laeufe() {
    let config = konfiguration(400, 4);
    let liste = gute_liste();
    let vorher = straenge_gestartet();
    let erster = run(&config, &liste);
    let im_ersten = straenge_gestartet() - vorher;
    assert_eq!(
        im_ersten, 3,
        "ein Lauf mit vier Straengen startete {im_ersten} Straenge statt drei Helfer"
    );

    let danach = straenge_gestartet();
    for _ in 0..5 {
        let r = run(&config, &liste);
        assert_eq!(
            r.damage.mean.to_bits(),
            erster.damage.mean.to_bits(),
            "derselbe Lauf, ein anderes Ergebnis"
        );
    }
    let neu = straenge_gestartet() - danach;
    assert_eq!(neu, 0, "fuenf weitere Laeufe starteten {neu} Straenge");
}
