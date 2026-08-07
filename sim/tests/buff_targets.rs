//! Eine Verstaerkung wirkt auf ihre Zielgroesse — und nur auf sie (Issue #489).
//!
//! # Warum die Gegenprobe die wichtigere Haelfte ist
//!
//! Dass ein Faktor 2 den Punktegewinn hebt, zeigt nur, dass irgendein Faktor
//! ankommt. Dass er den SCHADEN dabei nicht bewegt, zeigt, dass die fuenf
//! Vorraete wirklich getrennt sind — sonst muesste die Stapel-Regel
//! (GAME-DESIGN 328) ueber Zielgroessen hinweg gelten, und eine Verstaerkung
//! auf den Punktegewinn verdraengte eine auf den Schaden.
//!
//! Das Spiegelbild im Spiel ist `tests/unit/buff-targets.test.ts`.

use rota_sim::data::{BuffTarget, SPELLS};

/// Findet einen Zauber ueber seine Kennung.
fn zauber(id: &str) -> &'static rota_sim::data::Spell {
    SPELLS
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("Zauber {id} fehlt im erzeugten Datensatz"))
}

#[test]
fn die_vier_pruef_zauber_tragen_verschiedene_zielgroessen() {
    // **Vier Zauber, die alle dasselbe Fenster oeffnen, belegten vier Mal
    // dasselbe.** Diese Pruefung faengt genau das: Kippt einer der vier
    // versehentlich auf `Damage` zurueck — etwa durch einen Tippfehler im
    // `target`-Feld, den der Parser dann als "fehlt, also Schaden" liest —
    // messen die Faelle unten stillschweigend viermal denselben Vorrat.
    let erwartet = [
        ("test-control", BuffTarget::Control),
        ("test-heat", BuffTarget::Resource),
        ("test-tick", BuffTarget::Tick),
        ("test-stack", BuffTarget::Stack),
    ];
    for (id, ziel) in erwartet {
        let buff = zauber(id)
            .buff
            .unwrap_or_else(|| panic!("{id} traegt gar keine Verstaerkung"));
        assert_eq!(buff.target, ziel, "{id} wirkt auf die falsche Groesse");
    }
}

#[test]
fn ohne_feld_bleibt_es_beim_schaden() {
    // Der bestehende Pruef-Zauber traegt kein `target`. Waere der Vorgabewert
    // etwas anderes als `Damage`, waeren alle Zauber von vor #489 still auf
    // eine andere Groesse gewandert.
    assert_eq!(zauber("test-buff").buff.unwrap().target, BuffTarget::Damage);
}
