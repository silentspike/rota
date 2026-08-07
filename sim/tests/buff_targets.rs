//! Die Pruef-Zauber tragen verschiedene Zielgroessen (Issue #489, #562).
//!
//! **Diese Datei prueft die DATEN, nicht die Wirkung.** Sie belegt, dass
//! `test-heat` das Feld `target: Resource` traegt — nicht, dass der Faktor im
//! Kampf etwas bewegt. Die Wirkungspruefung steht auf beiden Seiten:
//! `tests/unit/buff-targets.test.ts` im Spiel und `sim/tests/buff_wirkung.rs`
//! in Rust (Issue #502, seit dem 2026-08-07 geschlossen — der Kopf nannte es
//! bis dahin als offen, und wer das las, hielt die Rust-Wirkung fuer
//! ungeprueft).
//!
//! Kippte einer per Tippfehler auf `damage` zurueck, maessen die Faelle dort
//! stillschweigend mehrfach denselben Vorrat. Das ist der Zweck dieser Datei —
//! und ihre Grenze.
//!
//! # Warum der alte Kopf hier stand
//!
//! Er beschrieb eine Wirkungspruefung ("dass er den SCHADEN dabei nicht
//! bewegt, zeigt, dass die fuenf Vorraete wirklich getrennt sind") ueber
//! einer Datei, die nichts davon tut. Der Satz war wahr und zeigte auf die
//! falsche Datei — wer ihn las, hielt die Rust-Seite fuer geprueft. Genau so
//! ist sie abgenommen worden.
use rota_sim::data::{BuffTarget, SPELLS};

/// Findet einen Zauber ueber seine Kennung.
fn zauber(id: &str) -> &'static rota_sim::data::Spell {
    SPELLS
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("Zauber {id} fehlt im erzeugten Datensatz"))
}

#[test]
fn die_pruef_zauber_tragen_verschiedene_zielgroessen() {
    // **Zauber, die alle dasselbe Fenster oeffnen, belegten mehrfach
    // dasselbe.** Diese Pruefung faengt genau das: Kippt einer
    // versehentlich auf `Damage` zurueck — etwa durch einen Tippfehler im
    // `target`-Feld, den der Parser dann als "fehlt, also Schaden" liest —
    // messen die Faelle unten stillschweigend mehrfach denselben Vorrat.
    //
    // **Die Liste war unvollstaendig, und das fiel beim Anbau von `test-cap`
    // auf** (#562): Sie fuehrte vier von damals sechs Ziel-Pruefern —
    // `test-cdreduce` und `test-convert` kamen mit E10 dazu, ohne hier
    // einzutragen. Deshalb stehen jetzt alle sieben da, nicht nur der neue.
    // Eine Liste, die einen Teil ihrer Gattung fuehrt, sieht aus wie ein
    // Waechter und ist eine Stichprobe.
    let erwartet = [
        ("test-control", BuffTarget::Control),
        ("test-heat", BuffTarget::Resource),
        ("test-tick", BuffTarget::Tick),
        ("test-stack", BuffTarget::Stack),
        ("test-cdreduce", BuffTarget::CdReduce),
        ("test-convert", BuffTarget::Convert),
        ("test-cap", BuffTarget::Cap),
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
