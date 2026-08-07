//! **Die Boss-Heilung ist nicht die Spielerheilung.**
//!
//! Am 2026-08-07 zeigte der Balance-Laeufer "Leben zurueckgeholt 0.0", und
//! daraus wurde geschlossen, der Boss heile sich nicht. Die 0.0 war die
//! SPIELER-Heilung; die Boss-Zahl gab der Laeufer gar nicht aus, obwohl
//! `Fight` sie seit #218 fuehrt und die Zustands-Pruefsumme sie mitrechnet.
//!
//! Diese Faelle fallen, sobald jemand die beiden Groessen wieder
//! zusammenlegt oder `boss_healed` aus der Aggregation nimmt.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig};

const GOOD: &str = include_str!("../apl/step1-good.apl");

fn apl() -> Apl {
    parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
}

fn gegen(boss_id: &str) -> rota_sim::balance::BalanceResult {
    let boss = rota_sim::data::boss_by_id(boss_id).expect("Boss steht in den Daten");
    let config = BalanceConfig {
        boss,
        iterations: 8,
        ..BalanceConfig::default()
    };
    run(&config, &apl())
}

/// Ein Boss mit `bossHeal` dreht Fortschritt zurueck — mit **harter Zahl**.
///
/// `test-racer` traegt `{"amount": 600}`. Eine blosse Anwesenheitspruefung
/// (`> 0`) hielte auch dann, wenn der Betrag aus den Daten verlorenginge.
#[test]
fn der_boss_dreht_genau_seinen_betrag_zurueck() {
    assert_eq!(
        gegen("test-racer").boss_healed.mean,
        600.0,
        "test-racer traegt bossHeal.amount 600 — kommt eine andere Zahl an, \
         ist die Kette Daten -> Fight -> BalanceResult unterbrochen"
    );
}

/// **Die Gegenprobe, die den Fall vom 2026-08-07 gefangen haette.**
///
/// Bei `test-racer` heilt sich der Boss, der Spieler aber nicht. Laesen beide
/// Zeilen dieselbe Quelle, stuenden hier zwei gleiche Zahlen.
#[test]
fn spielerheilung_und_bossheilung_sind_getrennte_groessen() {
    let r = gegen("test-racer");
    assert_eq!(
        r.healed.mean, 0.0,
        "der Spieler wird hier nicht geheilt — steht hier 600, zeigt die Zeile \
         die Boss-Zahl unter dem Namen der Spielerzahl"
    );
    assert_ne!(
        r.healed.mean, r.boss_healed.mean,
        "beide Groessen gleich heisst: eine von beiden liest die falsche Quelle"
    );
}

/// Ein Boss **ohne** `bossHeal` dreht nichts zurueck.
///
/// Ohne diesen Fall hielte `der_boss_dreht_genau_seinen_betrag_zurueck` auch
/// dann, wenn die Zahl fest verdrahtet waere statt aus den Daten zu kommen.
#[test]
fn ohne_bossheal_bleibt_die_zahl_null() {
    assert_eq!(
        gegen("first-crucible").boss_healed.mean,
        0.0,
        "first-crucible traegt kein bossHeal — eine Zahl darueber waere erfunden"
    );
}
