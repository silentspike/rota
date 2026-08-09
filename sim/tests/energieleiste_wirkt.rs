//! **WIRKUNGSPRUEFUNG** der Energieleiste (Issue #568, AC3).
//!
//! # Diese Datei ersetzt einen Waechter, und zwar mit Absicht
//!
//! Bis #568 stand hier `energieleiste_ist_zustand.rs` (Commit 63b97fe). Der
//! hielt fest, dass die Leiste **nichts ausloest**: Sie stieg, erreichte 100,
//! sprang zurueck — und keine Zeile im Kampf fragte danach. `test-vessel`
//! lieferte mit und ohne `energy` bitgleich 8080/1600/0.
//!
//! Sein eigener Fehlertext sagte, was zu tun ist, wenn er rot wird:
//!
//! > Er ist kein Fehlschlag. Er haelt fest, dass die Leiste bis #568 NICHTS
//! > ausloest. Hat jemand sie angeschlossen, gehoert dieser Waechter geloescht
//! > und durch eine Wirkungspruefung ersetzt.
//!
//! Genau das ist geschehen. Der Waechter wurde rot, weil die Kulisse zur
//! Mechanik geworden ist — nicht, weil etwas kaputtging.
//!
//! # Was der Ausloeser tut, und warum es keine neue Mechanik-Art ist
//!
//! Vom Auftraggeber entschieden: ein **Boss-Schlag ausserhalb der
//! Zeitleiste**. Verworfen wurden ein ausgeloester Boss-Zauber und der
//! sofortige Tod als eigene Bauart — letzterer ist ein `strike` mit hohem
//! Schaden. Deshalb derselbe Weg wie in der Zeitleiste, nur kommt der
//! Zeitpunkt aus der Leiste statt aus `at_ms`. Das ist der „innere Rhythmus,
//! der nichts mit dem Lebensbalken zu tun hat" (GAME-DESIGN 11.2, Baustein 1).
//!
//! # Die Gegenprobe ist der Kern
//!
//! Der Vorgaenger nagelte 8080/1600 fest, um zu zeigen, dass sich **nichts**
//! bewegt. Hier wird dieselbe Stelle festgenagelt, um zu zeigen, dass sich
//! **etwas** bewegt — und zwar nur auf einer Seite: Der ausgeteilte Schaden
//! bleibt bei 8080, der eingesteckte steigt auf 2400. Waere auch die erste
//! Zahl gewandert, kaeme der Unterschied von woanders her.

use rota_sim::data::boss_by_id;
use rota_sim::{rotation, Fight, FightConfig};

/// Faehrt denselben Lauf wie der Vorgaenger und liefert (Schaden, eingesteckt, Ausloeser).
fn lauf(boss_id: &str) -> (u32, u32, u32) {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        ..FightConfig::default()
    });
    let result = fight.run(rotation::good);
    (result.damage, fight.damage_taken(), fight.energy_triggers())
}

#[test]
fn die_daten_tragen_die_wirkung_ueberhaupt() {
    // **Zusicherung in den Datensatz.** Ohne `on_trigger` in den Daten
    // pruefte alles darunter eine Leiste ohne Wirkung — und waere gruen,
    // weil dann eben nichts passiert. `build.rs` bricht dafuer zwar schon
    // beim Bau ab; diese Zeile sagt es an der Stelle, an der es gemessen
    // wird.
    let boss = boss_by_id("test-vessel").expect("test-vessel unbekannt");
    let energy = boss
        .energy
        .expect("test-vessel hat keine Energieleiste mehr");
    assert!(
        energy.on_trigger.damage > 0,
        "der Ausloeser richtet keinen Schaden an — dann stiege die Leiste wieder ins Leere"
    );
}

#[test]
fn ihre_ausloeser_bewegen_den_eingesteckten_schaden() {
    let (schaden, eingesteckt, ausloeser) = lauf("test-vessel");

    // Ohne Ausloeser sagte der Fall nichts darueber, ob ihr Erreichen etwas
    // bewirkt — dieselbe Zusicherung wie im Vorgaenger.
    assert!(
        ausloeser >= 5,
        "zu wenige Ausloeser ({ausloeser}) fuer eine Aussage"
    );

    // **Der Kern, und die Zahlen stammen aus der Messung, nicht aus einer
    // Rechnung.** Der Vorgaenger stand hier auf (8080, 1600) und hielt damit
    // die Wirkungslosigkeit fest.
    assert_eq!(
        (schaden, eingesteckt),
        (8080, 2400),
        "der Kampfverlauf hat sich gegenueber der Messung von #568 verschoben"
    );

    // **Der ausgeteilte Schaden darf sich NICHT bewegt haben.** Die Leiste
    // trifft den Spieler, nicht den Boss. Waere auch diese Zahl gewandert,
    // kaeme der Unterschied von woanders her — und der Fall waere gruen aus
    // dem falschen Grund.
    assert_eq!(
        schaden, 8080,
        "der ausgeteilte Schaden hat sich bewegt — der Ausloeser trifft den falschen"
    );
}
