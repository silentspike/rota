//! **Was ein Abbruch kostet — und was nicht** (Issue #304, gemessen für #480)
//!
//! Beide Eigenschaften standen bisher nur als Kommentar im Quelltext, und der
//! eine sagt es sogar ausdrücklich als Abwesenheit:
//!
//! > Die Abklingzeit läuft nicht an, und das steht hier nicht als Zeile,
//! > sondern als Abwesenheit einer.
//!
//! Eine Eigenschaft, die sich aus dem FEHLEN einer Zeile ergibt, überlebt
//! keinen Umbau: Wer die Abklingzeit eines Tages in `cast` setzt statt in
//! `resolve_player_cast`, ändert sie, ohne es zu merken. Diese Datei misst
//! sie stattdessen.
//!
//! Entstanden sind die Fälle bei der Vorprüfung für RETORT (#480), der einen
//! unterbrochenen Zauber "sofort wieder bereit" machen soll — und dabei
//! zeigte sich, dass er das bereits ist.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// `test-caster` unterbricht bei 9000 ms; `test-cast` hat 2000 ms Wirkzeit.
/// Wer bei 8000 ms drückt, wird mittendrin getroffen.
fn lauf_mit_abbruch(ticks: u32) -> (Fight, usize) {
    let test_cast = index_of("test-cast").expect("test-cast steht im Zauberkasten");
    let boss = boss_by_id("test-caster").expect("test-caster fehlt");
    let recording = Recording {
        name: "abbruch-probe".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![Binding {
            slot: 0,
            spell: test_cast,
        }],
        presses: vec![Press { tick: 800, slot: 0 }],
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);
    assert_eq!(
        ergebnis.casts[test_cast], 0,
        "die Probe misst gar keinen Abbruch — der Zauber ist durchgekommen"
    );
    (fight, test_cast)
}

#[test]
fn ein_abgebrochener_zauber_steht_sofort_wieder_bereit() {
    // **Die Abklingzeit läuft NICHT an.** Sie wird in `resolve_player_cast`
    // gesetzt, und die läuft bei einem Abbruch nie.
    let (fight, test_cast) = lauf_mit_abbruch(1200);
    assert_eq!(
        fight.cooldown_remaining_ms(test_cast),
        0,
        "ein abgebrochener Zauber steht auf Abklingzeit — dann kostet ein \
         Abbruch neuerdings zweierlei, und die Abbruch-Regel aus GAME-DESIGN \
         7.1 ist eine andere geworden"
    );
    assert!(fight.cooldown_ready(test_cast), "und ist nicht bereit");
}

#[test]
fn der_abbruch_kostet_trotzdem_die_globale_wartezeit() {
    // **Das ist der Preis**, und der einzige: Bei einer Wirkzeit startet die
    // Wartezeit beim BEGINN (Issue #304). Wer abgebrochen wird, hat sie
    // bezahlt und nichts dafür bekommen.
    //
    // Der Wurf fällt bei 8000 ms, die Wartezeit läuft 1500 ms — bis 9500 ms.
    // Der Abbruch bei 9000 ms ändert daran nichts.
    let (fight, _) = lauf_mit_abbruch(905);
    assert_eq!(
        fight.gcd_ready_at(),
        9500,
        "die Wartezeit nach einem Abbruch ist eine andere als erwartet — \
         entweder startet sie nicht mehr beim Beginn der Wirkzeit, oder der \
         Abbruch räumt sie neuerdings mit weg"
    );
}
