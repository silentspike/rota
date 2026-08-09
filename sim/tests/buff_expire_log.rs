//! Der Simulator schreibt das Auslaufen einer Verstaerkung in den Verlauf
//! (Issue #682, AC3 fuer die Rust-Seite).
//!
//! # Warum dieser Fall eigens noetig ist
//!
//! Bis #682 kannte diese Seite die Art 7 **gar nicht** — die Aufzaehlung in
//! `log.rs` sprang von 6 auf 8. Im Spiel war sie vergeben, dokumentiert und
//! gezaehlt; hier fehlte sie vollstaendig.
//!
//! Und nichts haette es gemeldet. Gemessen beim Bau von #682:
//!
//! ```text
//! combat.ts  Schreibstelle raus  ->  3 Faelle rot, beide nennen die Art
//! fight.rs   Schreibstelle raus  ->  KEIN einziger Fall faerbt
//! ```
//!
//! Auch `log-parity.json` faengt es nicht, obwohl dort `buffExpire: 1` steht:
//! Der Vergleich in `parity_log_kinds_written.rs` laeuft ueber die
//! SIMULATOR-Zaehlung. Hoert der Simulator auf zu schreiben, verschwindet die
//! Art aus dieser Zaehlung — und damit aus der Schleife. Der Vergleich merkt
//! nicht, dass etwas fehlt; er vergleicht nur noch weniger.
//!
//! > Ein Vergleich, der nur ueber das laeuft, was eine Seite tatsaechlich
//! > schreibt, kann das AUFHOEREN dieser Seite nicht bemerken.
//!
//! # Warum FLASHPOINT und kein Pruef-Zauber
//!
//! `test-cap` und seine sieben Geschwister bedienen zwar genau die acht
//! Sammlungen — aber eine Rotation darf sie nicht nennen (Issue #499), sie
//! werden abgewiesen. Auf dieser Seite fuehrt der Weg deshalb ueber einen
//! ECHTEN Zauber: `flashpoint` traegt `buff.target = resource`.
//!
//! # Umfang
//!
//! Dieser Fall deckt **nur Art 7** ab. Dass 16 Arten mit `sim: true` in der
//! Absicht stehen und keine davon eine Schreibstelle-Pruefung hat, ist Issue
//! #689 und wird dort allgemein geloest — nicht hier nebenbei.

use rota_sim::apl::{parse, AplName};
use rota_sim::log::LogKind;
use rota_sim::{Fight, FightConfig};

/// Eine Rotation, die FLASHPOINT wirft — er setzt die Verstaerkung, deren
/// Ablauf hier gemessen wird.
const MIT_VERSTAERKUNG: &str = "flashpoint\nkindle\n";

#[test]
fn der_ablauf_schreibt_eintraege_der_art_sieben() {
    let apl = parse(AplName("buff-expire"), MIT_VERSTAERKUNG)
        .unwrap_or_else(|error| panic!("die Rotation ist fehlerhaft:\n{error}"));

    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(&apl);

    let werte: Vec<u32> = fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::BuffExpire)
        .map(|e| e.value)
        .collect();

    // **Die Zusicherung in den Datensatz.** Ohne einen einzigen Ablauf
    // pruefte der Fall nichts und waere trotzdem gruen: Eine Rotation, die
    // FLASHPOINT nie wirft, kann auch nichts ablaufen lassen.
    assert!(
        !werte.is_empty(),
        "kein Eintrag der Art 7 — entweder wirft die Rotation FLASHPOINT nicht, \
         oder die Schreibstelle in fight.rs fehlt"
    );

    // **Der Wert ist die ART, nicht die Anzahl.** FLASHPOINT setzt `kind: 0`.
    // Truege der Eintrag eine Anzahl, stuende hier bei jedem Ablauf eine 1 —
    // und beide waeren nicht zu unterscheiden.
    assert!(
        werte.iter().all(|&v| v == 0),
        "erwartet wird die Art 0 (FLASHPOINT), gefunden: {werte:?}"
    );
}
