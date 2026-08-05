//! AC2 aus Issue #331 — der Nachweis, dass die Seiten heute auseinanderlaufen.
//!
//! **Dieser Fall wird VOR der Reparatur gebaut, und das ist der Punkt.** Ein
//! Prüffall, der danach entsteht, kann nicht mehr belegen, dass er den
//! Unterschied je gesehen hätte — er belegt nur, dass nach der Reparatur
//! Ruhe ist.
use rota_sim::data::boss_by_id;
use rota_sim::{Fight, FightConfig};

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
    let erlitten = fight.damage_taken();
    eprintln!(
        "SIMULATOR: schaden={} erlitten={erlitten} unterbrochen={}",
        result.damage,
        fight.interrupts_landed()
    );

    // **Die Zahl, nicht nur die Ausgabe.** Ohne diese Zeile war der Fall ein
    // Bericht und kein Waechter: Er lief durch, gab etwas aus, und haette
    // jede beliebige Zahl durchgelassen.
    //
    // Vier Boss-Zauber zu 600, nach `PLAYER.wardReduction` (0,2) je 480.
    // Alle vier muessen landen — auch die beiden, deren Nachfolger im selben
    // Takt beginnt. Faellt einer aus, ist die Rangfolge aus `event.rs`
    // zurueckgedreht, und der Spieler bekommt geschenkt, was er haette
    // verhindern muessen.
    assert_eq!(
        erlitten,
        1920,
        "Der Spieler erleidet {erlitten} statt 1920 — es fallen {} Boss-Zauber \
         aus. Steht BossCastLands in event.rs wieder HINTER TimelineStep?",
        (1920 - erlitten) / 480
    );
}
