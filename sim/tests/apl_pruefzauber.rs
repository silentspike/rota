//! Eine Rotation darf keine Pruef-Zauber nennen (Issue #499).
//!
//! # Warum das mehr ist als eine unsaubere Vorschlagsliste
//!
//! Pruef-Zauber liegen ueber der Stufengrenze und kommen im Spiel nicht vor.
//! Wer einen in einer Rotation nennt, misst mit dem Balance-Laeufer eine
//! Rotation, die es nicht gibt — und **nichts wird dabei rot**. Das ist
//! dieselbe Klasse wie #492 und #425: ein Lauf, der plausibel antwortet,
//! obwohl er die Sache gar nicht beruehrt.
//!
//! AC1 macht den Vorschlag sauber, AC3 verhindert die Wirkung. Die zweite
//! Haelfte ist die wichtigere.

use rota_sim::apl::{parse, spielbare_zauber, AplName};
use rota_sim::data::{MAX_LEVEL, SPELLS};

#[test]
fn ac3_ein_pruef_zauber_wird_abgewiesen() {
    // **Der eigentliche Ertrag.** Vorher wurde er stillschweigend gewirkt.
    let fehler = parse(AplName("probe"), "test-buff\n").expect_err(
        "test-buff wurde angenommen — dann misst der Laeufer einen Zauber, den es nicht gibt",
    );
    assert!(
        fehler.message.contains("test-buff"),
        "die Meldung nennt den Zauber nicht: {}",
        fehler.message
    );
}

#[test]
fn ac3_gegenprobe_ein_echter_zauber_geht_durch() {
    // Ohne diese Zeile belegte der Fall oben nur, dass `parse` ueberhaupt
    // etwas abweist — nicht, dass die Stufengrenze das Kriterium ist.
    parse(AplName("probe"), "kindle\n").expect("kindle ist ein Zauber des Spiels");
}

#[test]
fn ac1_die_vorschlagsliste_nennt_keine_pruef_zauber() {
    let fehler = parse(AplName("probe"), "gibtsnicht\n").expect_err("Tippfehler muss scheitern");
    for pruef in SPELLS.iter().filter(|s| s.unlock_level > MAX_LEVEL as u16) {
        assert!(
            !fehler.message.contains(pruef.id),
            "die Liste schlaegt {} vor — einen Zauber, den es im Spiel nicht gibt",
            pruef.id
        );
    }
}

#[test]
fn ac2_die_liste_bleibt_sie_wird_nur_kuerzer() {
    // **Ein Filter, der die Hilfe abschaltet, hat das Problem nicht geloest.**
    let fehler = parse(AplName("probe"), "gibtsnicht\n").expect_err("Tippfehler muss scheitern");
    assert!(
        fehler.message.contains("kindle"),
        "die Vorschlagsliste ist leer — dann hilft die Meldung niemandem mehr: {}",
        fehler.message
    );

    let vorgeschlagen = spielbare_zauber().count();
    assert!(
        vorgeschlagen > 10,
        "nur {vorgeschlagen} spielbare Zauber — die Zaehlung misst offenbar etwas anderes"
    );
    assert!(
        vorgeschlagen < SPELLS.len(),
        "spielbare_zauber() filtert nichts heraus — dann pruefen die Faelle oben ins Leere"
    );
}
