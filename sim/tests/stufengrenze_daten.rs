//! Oberhalb der Stufengrenze liegt nur, was zum Pruefen da ist (Issue #499).
//!
//! # Warum an den DATEN und nicht an den Lesestellen
//!
//! Der naheliegende Schutz waere ein Waechter ueber die Stellen, die `SPELLS`
//! lesen, ohne die Grenze zu ziehen. Gemessen (Meldung 3166): sechs solche
//! Stellen, davon fuenf zu Recht ungefiltert — zwei nach Index adressierte
//! Tabellen, die ALLE Eintraege fuehren muessen, eine Suche ueber eine
//! Kennung, und die Rotationssuche prueft bereits selbst. Ein Waechter haette
//! zwei richtige Stellen dauerhaft gemeldet.
//!
//! Diese Zusicherung steht statt dessen an der QUELLE und dreht die Frage um:
//! Nicht "wer filtert nicht?", sondern "liegt oberhalb der Grenze ueberhaupt
//! etwas, das das Spiel kennen sollte?". Liegt dort nur Pruef-Zeug, muessen
//! die Lesestellen gar nicht erst einzeln fragen.
//!
//! Und sie faengt den Fall, um den es eigentlich geht: dass ein ECHTER Zauber
//! versehentlich ueber die Grenze rutscht — etwa durch einen Tippfehler in
//! `unlockLevel`. Ein Waechter ueber die Lesestellen haette dazu geschwiegen.

use rota_sim::data::{MAX_LEVEL, SPELLS};

/// Das Kennzeichen, an dem ein Pruef-Zauber erkennbar ist.
const PRUEF_PRAEFIX: &str = "test-";

/// Verstoesst dieser Eintrag gegen die Regel?
///
/// **Als Funktion und nicht als Schleife ueber `SPELLS`**, damit die
/// Gegenprobe unten mit erfundenen Eintraegen arbeiten kann. Eine Pruefung,
/// die nur die heutigen Daten durchgehen sieht, belegt lediglich, dass sie
/// heute stimmen — nicht, dass die Regel greift.
fn verstoesst(id: &str, unlock_level: u16) -> bool {
    unlock_level > MAX_LEVEL as u16 && !id.starts_with(PRUEF_PRAEFIX)
}

#[test]
fn kein_echter_zauber_liegt_ueber_der_stufengrenze() {
    let verstoesse: Vec<&str> = SPELLS
        .iter()
        .filter(|s| verstoesst(s.id, s.unlock_level))
        .map(|s| s.id)
        .collect();

    assert!(
        verstoesse.is_empty(),
        "diese Zauber liegen ueber Stufe {MAX_LEVEL}, heissen aber nicht \"{PRUEF_PRAEFIX}…\": \
         {verstoesse:?}\n\
         Entweder ist die Stufe ein Tippfehler — dann ist der Zauber im Spiel unerreichbar — \
         oder es ist ein Pruef-Zauber, dann gehoert das Kennzeichen in den Namen."
    );
}

#[test]
fn die_regel_wird_rot_wenn_ein_echter_zauber_hinueberrutscht() {
    // **Die Auflage.** Ein Test, der nur die heutigen Pruef-Zauber durchgehen
    // sieht, belegt nur, dass sie `test-` heissen — nicht, dass die Regel
    // ueberhaupt etwas ablehnt.
    assert!(
        verstoesst("kindle", MAX_LEVEL as u16 + 1),
        "ein echter Zauber ueber der Grenze wird nicht beanstandet — dann ist \
         die Zusicherung wirkungslos"
    );
}

#[test]
fn und_bleibt_still_bei_den_beiden_erlaubten_faellen() {
    assert!(
        !verstoesst("test-buff", MAX_LEVEL as u16 + 1),
        "ein Pruef-Zauber ueber der Grenze ist erlaubt — genau dafuer ist sie da"
    );
    assert!(
        !verstoesst("kindle", 1),
        "ein echter Zauber UNTER der Grenze ist der Normalfall"
    );
}

#[test]
fn oberhalb_der_grenze_liegt_ueberhaupt_etwas() {
    // **Ohne diese Zeile misst der erste Fall nichts.** Laege dort gar nichts,
    // waere seine leere Liste kein Befund, sondern eine Selbstverstaendlichkeit
    // — und niemand merkte, wenn die Pruef-Zauber verschwinden.
    let oberhalb = SPELLS
        .iter()
        .filter(|s| s.unlock_level > MAX_LEVEL as u16)
        .count();
    assert!(
        oberhalb > 0,
        "kein einziger Eintrag liegt ueber Stufe {MAX_LEVEL} — dann laeuft die \
         Zusicherung ins Leere"
    );
}
