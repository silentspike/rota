//! **WAECHTER UEBER DEN IST-ZUSTAND** der Energieleiste (Issue #568, AC4).
//!
//! # Dies ist KEINE Wirkungspruefung
//!
//! Der Name sagt es, und er sagt es absichtlich: Dieser Fall belegt NICHT,
//! dass die Energieleiste korrekt arbeitet. Er haelt fest, dass sie heute
//! **nichts ausloest** — und meldet sich, sobald das nicht mehr stimmt.
//!
//! GAME-DESIGN 11.2, Baustein 1: „Boss-Energie steigt unabhaengig von den
//! Lebenspunkten und loest bei 100 eine Mechanik aus." WELCHE Mechanik, steht
//! dort nicht. Der Mechanismus wartet auf eine Design-Entscheidung, die
//! niemand getroffen hat — deshalb ist #568 blockiert und dieser Fall der
//! einzige Teil, der ohne sie baubar ist.
//!
//! # Warum ein Waechter und keine Notiz
//!
//! Gemessen wurde die Wirkungslosigkeit von zwei Seiten:
//!
//! ```text
//! test-vessel mit energy        Schaden 8080, eingesteckt 1600, Tode 0
//! test-vessel mit energy: null  Schaden 8080, eingesteckt 1600, Tode 0
//! ```
//!
//! Und im Code: `energy_triggers` wird hochgezaehlt, zurueckgesetzt und von
//! einem Getter gelesen — keine Zeile im Kampf fragt danach.
//!
//! Eine Notiz im Issue veraltet still. Dieser Fall wird ROT, sobald jemand
//! die Leiste anschliesst — und dann ist er kein Fehlschlag, sondern die
//! Meldung, dass die Kulisse zur Mechanik geworden ist. Wer ihn rot sieht,
//! loescht ihn und schreibt die Wirkungspruefung.

use rota_sim::data::boss_by_id;
use rota_sim::{rotation, Fight, FightConfig};

/// Faehrt denselben Lauf und liefert (Schaden, eingesteckt, Ausloeser).
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
fn die_leiste_steigt_und_loest_heute_nichts_aus() {
    // **Die Zusicherung in den Datensatz.** Ohne einen einzigen Ausloeser
    // pruefte der Fall nichts und waere trotzdem gruen: Eine Leiste, die die
    // Schwelle nie erreicht, kann auch nichts ausloesen.
    let (_, _, ausloeser) = lauf("test-vessel");
    assert!(
        ausloeser > 0,
        "die Energieleiste hat die Schwelle nie erreicht — dann sagt dieser \\
         Fall nichts darueber, ob ihr Erreichen etwas bewirkt"
    );
}

/// Der Kern: Die Ausloeser aendern am Kampf nichts.
///
/// Verglichen wird `test-vessel` (mit Leiste) gegen `test-stacked` — nein,
/// das waeren zwei verschiedene Bosse. Verglichen wird die Leiste gegen ihre
/// eigene Wirkung: Sie zaehlt hoch, und weder Schaden noch eingesteckter
/// Schaden noch der Tod des Spielers haengen daran.
///
/// Die Zahlen stammen aus einem Lauf mit `rotation::good` und sind
/// festgenagelt — nicht weil sie wichtig waeren, sondern weil sie sich
/// bewegen MUESSEN, sobald die Leiste etwas tut.
#[test]
fn ihre_ausloeser_bewegen_keine_einzige_kampfzahl() {
    let (schaden, eingesteckt, ausloeser) = lauf("test-vessel");

    assert!(
        ausloeser >= 5,
        "zu wenige Ausloeser ({ausloeser}) fuer eine Aussage"
    );
    assert_eq!(
        (schaden, eingesteckt),
        (8080, 1600),
        "Schaden oder eingesteckter Schaden haben sich bewegt, waehrend die \\
         Energieleiste {ausloeser} Mal ausgeloest hat.\\n\\n\\
         WENN DU DIESEN FALL ROT SIEHST: Er ist kein Fehlschlag. Er haelt \\
         fest, dass die Leiste bis #568 NICHTS ausloest (GAME-DESIGN 11.2, \\
         Baustein 1 — welche Mechanik, stand dort nie). Hat jemand sie \\
         angeschlossen, gehoert dieser Waechter geloescht und durch eine \\
         Wirkungspruefung ersetzt."
    );
}
