//! AC2 aus Issue #331 — der Nachweis, dass die Seiten heute auseinanderlaufen.
//!
//! **Dieser Fall wird VOR der Reparatur gebaut, und das ist der Punkt.** Ein
//! Prüffall, der danach entsteht, kann nicht mehr belegen, dass er den
//! Unterschied je gesehen hätte — er belegt nur, dass nach der Reparatur
//! Ruhe ist.
use rota_sim::data::boss_by_id;
use rota_sim::{rotation, Fight, FightConfig};

#[test]
fn was_der_simulator_am_kollisions_boss_misst() {
    let boss = boss_by_id("test-collision").expect("Boss test-collision fehlt");
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(30_000),
        ..FightConfig::default()
    });
    // **Keine Rotation.** Beide Seiten fahren dieselbe Druckfolge, naemlich
    // gar keine — dann haengt der erlittene Schaden allein am Boss, und ein
    // Unterschied kann nur aus der Reihenfolge kommen.
    //
    // Mein erster Anlauf verglich rotation::good gegen stumpf gedruecktes
    // KINDLE und schrieb die Differenz der Kollision zu. Zwei verschiedene
    // Dinge messen und den Unterschied der Sache zuschreiben, um die es geht.
    let result = fight.run(|_| None);
    eprintln!(
        "SIMULATOR: schaden={} erlitten={} unterbrochen={}",
        result.damage,
        fight.damage_taken(),
        fight.interrupts_landed()
    );
}
