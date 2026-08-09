//! Die Zersetzungs-Mechanik wird von ausgelieferten Zaubern benutzt (#97).
//!
//! # Warum dieser Waechter existiert
//!
//! Diese Woche gab es dreimal denselben Fall: eine fertige Mechanik, die kein
//! Zauber aufruft. Die Pruefsumme deckt sie ab, also gilt sie als fertig —
//! `sublimation.rs` wurde so zu #201, `energy` zu #219, `cooldown_reset.rs` zu
//! #223.
//!
//! Ein Waechter, der erst spaeter nachgereicht wird, ist genau das, was liegen
//! bleibt. Deshalb steht er im selben Zweig wie die Mechanik, die er bewacht.

use rota_sim::data;

#[test]
fn ausgelieferte_zauber_benutzen_die_zersetzung() {
    // **Welche**, nicht **ob**. Ein `> 0` waere schon erfuellt, wenn ein
    // einziger Zauber irgendetwas traegt; die Namen halten fest, WAS die
    // Referenz-Rechnung sieht.
    let legen_an: Vec<&str> = data::SPELLS
        .iter()
        .filter(|spell| spell.decay_stacks > 0)
        .map(|spell| spell.id)
        .collect();
    // **BLIGHT kam mit Issue #479 dazu, und die Auflage wurde befolgt:** Die
    // Referenz-Rechnung IST nachgemessen, nicht die Liste einfach erweitert.
    // `4240 @ 40510` steht unveraendert (step1.test.ts, 15 Faelle) — was zu
    // erwarten war, aber nicht angenommen werden durfte: Die Referenz-Rotation
    // ist reine CALCINATION und wirkt keinen der beiden.
    assert_eq!(
        legen_an,
        vec!["canker", "blight"],
        "Wer hier einen Zauber ergaenzt, aendert die Referenz-Rechnung — \
         dann gehoert sie nachgemessen, nicht diese Liste erweitert"
    );

    let leben_davon: Vec<&str> = data::SPELLS
        .iter()
        .filter(|spell| spell.per_decay_stack_percent > 0)
        .map(|spell| spell.id)
        .collect();
    assert_eq!(
        leben_davon,
        // MIASMA und NECROSIS kamen mit #479 dazu. Beide tragen
        // `per_decay_stack_percent` ohne eigenes `decay`-Feld — sie leben von
        // den Stapeln, ohne welche zu legen. NECROSIS raeumt sie zusaetzlich
        // ab; das prueft `consumes_decay_stacks` an seiner eigenen Stelle.
        //
        // **Diese Liste ist der Rust-Gegenpart zu `putrefaction.test.ts`.** Ich
        // habe die TypeScript-Seite ergaenzt und diese hier uebersehen — sie
        // stand nicht auf meiner Liste der sieben Anschlussstellen. Jetzt sind
        // es acht, und die achte faellt NUR in der CI-Stufe `sim` auf.
        vec!["fester", "miasma", "necrosis"],
        "CANKER legt an, FESTER lebt davon — die beiden gehoeren als Paar \
         zusammen (GAME-DESIGN 4.1)"
    );
}

#[test]
fn die_mechanik_wirkt_im_kampf_und_nicht_nur_in_den_daten() {
    // **Der zweite Teil, und er ist der wichtigere.** Die Liste oben zeigt,
    // dass die Daten es sagen. Sie zeigt nicht, dass der Kampf es tut — genau
    // diese Luecke hat die drei Faelle dieser Woche entstehen lassen.
    use rota_sim::data::{boss_by_id, index_of};
    use rota_sim::{Fight, FightConfig};

    let boss = boss_by_id("test-warden").expect("test-warden");
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(30_000),
        ..FightConfig::default()
    });
    assert_eq!(fight.decay_stacks(), 0, "vor dem Wirken liegt nichts");
    fn nur_canker(f: &Fight) -> Option<usize> {
        let canker = index_of("canker").expect("canker");
        if f.is_ready(canker) {
            Some(canker)
        } else {
            None
        }
    }
    fight.run(nur_canker as fn(&Fight) -> Option<usize>);
    assert!(
        fight.decay_stacks() > 0 || fight.damage() > 0,
        "CANKER kam nicht zum Zug — dann prueft dieser Fall nichts"
    );
}
