//! Die davonlaufende Reaktion, auf beiden Seiten gleich (Issue #247).
//!
//! **Warum ein eigener Prüffall und nicht nur eine Aufzeichnung:** Die drei
//! stillen Fehler aus #240 lagen alle im Tick selbst — im Anfangswert, in der
//! Rechenart und in der Rangfolge. Eine Aufzeichnung hätte sie gefunden, aber
//! erst als Prüfsummen-Abweichung in irgendeinem Takt. Hier steht die Zahl,
//! die beide Seiten treffen müssen, und sie ist gerechnet:
//!
//! ```text
//! 60 je Sekunde gegen 1000 Lebenspunkte
//! der siebzehnte Schritt bringt 1020 >= 1000
//! also: Ring leer bei 17000 ms
//! ```
//!
//! Dieselbe Zahl steht im Prüffall des Spiels (`boss-runaway.test.ts`, AC3).
//! Laufen die Seiten auseinander, sagt dieser Fall es in einer Zeile statt in
//! neuntausend Prüfsummen.
//!
//! **Die Wertung steht bewusst NICHT hier.** Ob „Ring leer" Sieg oder
//! Niederlage heißt, entscheidet das Spiel; der Simulator misst Schaden und
//! Zeit. Ein Simulator, der Ausgänge bewertet, hätte eine zweite Wahrheit
//! darüber.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

fn leerer_lauf(boss_id: &str, ticks: u32) -> rota_sim::ReplayResult {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let leer = Recording {
        name: "ohne-eingriff".to_owned(),
        seed: 0,
        ticks,
        bindings: Vec::new(),
        presses: Vec::new(),
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&leer, false)
}

#[test]
fn der_pruef_boss_traegt_das_feld() {
    let boss = boss_by_id("test-runaway").expect(
        "Der Boss aus data/bosses/test-runaway.json wurde nicht gefunden. \
         Liest build.rs das Verzeichnis noch?",
    );
    let run = boss
        .runaway
        .expect("test-runaway hat keine davonlaufende Reaktion");
    assert_eq!(run.per_step_ms, 1_000);
    assert_eq!(run.amount, 60);
    assert_eq!(boss.hp, 1_000);
    assert_eq!(boss.overpressure_ms, 20_000);
}

#[test]
fn ac3_der_ring_ist_zur_selben_zeit_leer_wie_im_spiel() {
    // **Gerechnet, bevor gemessen wird.** 1000 / 60 aufgerundet sind 17
    // Schritte à 1000 ms.
    let boss = boss_by_id("test-runaway").expect("test-runaway");
    let run = boss.runaway.expect("keine Reaktion");
    let schritte = boss.hp.div_ceil(run.amount);
    let erwartet_ms = schritte * run.per_step_ms;
    assert_eq!(erwartet_ms, 17_000);

    let result = leerer_lauf("test-runaway", 2_500);

    assert_eq!(
        result.killed_at_ms,
        Some(erwartet_ms),
        "der Ring ist zu einer anderen Zeit leer als im Spiel"
    );
    assert_eq!(
        result.damage,
        schritte * run.amount,
        "verschieden viel Fortschritt gebucht"
    );
}

#[test]
fn ac3_er_laeuft_in_gleichmaessigen_schritten() {
    // Ein Tick, der den Ring auf einmal leerte, erfuellte den Fall oben
    // ebenfalls. Gemessen wird deshalb der Verlauf — und zwar an der
    // Pruefsumme vorbei, direkt am Wert.
    //
    // **500 Takte sind t = 0 bis 4990, nicht bis 5000.** Beide Seiten laufen
    // `tick < ticks`; der Schritt bei 5000 gehoert also nicht mehr dazu. Mein
    // erster Entwurf erwartete 300 und bekam 240 — und das sah einen Moment
    // lang wie eine Abweichung zwischen den Seiten aus. Es war keine: Der
    // Pruefall im Spiel lief bis EINSCHLIESSLICH 5000 und mass damit ein
    // anderes Fenster. Zwei Faelle, die dieselbe Zahl behaupten, muessen
    // dieselbe Zeit messen.
    let result = leerer_lauf("test-runaway", 500);
    assert_eq!(result.damage, 240, "vier Schritte à 60 (1000 bis 4000 ms)");
    assert!(
        result.killed_at_ms.is_none(),
        "nach fuenf Sekunden ist der Ring noch nicht leer"
    );
}

#[test]
fn ein_boss_ohne_das_feld_bewegt_sich_nicht_von_selbst() {
    // Der Gegenfall: Ohne `runaway` bleibt der Ring stehen, wenn niemand etwas
    // tut. Ohne ihn waere nicht belegt, dass der Fortschritt vom FELD kommt
    // und nicht von irgendeiner anderen passiven Wirkung.
    let boss = boss_by_id("first-crucible").expect("first-crucible");
    assert!(boss.runaway.is_none());

    let result = leerer_lauf("first-crucible", 1_000);
    assert_eq!(result.damage, 0);
    assert!(result.killed_at_ms.is_none());
}

#[test]
fn ac4_schutz_draengt_den_ring_zurueck_wie_im_spiel() {
    // **Dieselbe Zahl wie im Spiel**, und "dieselbe" heisst: bei DEMSELBEN
    // Fenster. 2000 Takte sind t = 0 bis 19990, auf beiden Seiten `tick <
    // ticks`. Der Prueffall im Spiel misst mit genau dieser Schleife.
    //
    // Das ist die Lehre aus Etappe 2, und sie ist mir hier ein zweites Mal
    // passiert: Mein erster Entwurf liess Rust 2500 Takte laufen und verglich
    // mit einem Spiel-Lauf, der bei 20000 ms endete — 720 gegen 480. Das sah
    // nach einer Abweichung zwischen den SEITEN aus und war eine zwischen den
    // FENSTERN. Haette ich "repariert", waere daraus eine echte geworden.
    //
    // Die Belegung steht in der Aufzeichnung: CRUST hat in der Zauberleiste
    // aus Schritt 1 keinen Platz, und die zu aendern waere eine
    // Design-Entscheidung — die Belegung eines Pruefstands ist keine.
    let boss = boss_by_id("test-runaway").expect("test-runaway");
    let crust = index_of("crust").expect("CRUST unbekannt");

    // Alle 600 Takte druecken: CRUST hat 6 s Abklingzeit.
    let presses: Vec<Press> = (0..4)
        .map(|i| Press {
            tick: i * 600,
            slot: 0,
        })
        .collect();
    let recording = Recording {
        name: "mit-crust".to_owned(),
        seed: 0,
        ticks: 2_000,
        bindings: vec![Binding {
            slot: 0,
            spell: crust,
        }],
        presses,
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, false);

    assert!(
        result.stabilised > 0,
        "im Lauf wurde nie zurueckgedraengt — dann prueft dieser Fall nichts"
    );
    assert!(
        result.killed_at_ms.is_none(),
        "mit CRUST darf der Ring die Frist ueberstehen"
    );
    assert_eq!(result.stabilised, 720, "dieselbe Zahl wie im Spiel");
    assert_eq!(result.damage, 420, "dieselbe Zahl wie im Spiel");
}
