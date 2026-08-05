//! Die Synergie-Mechanik wird von ausgelieferten Zaubern benutzt (#318).
//!
//! # Warum dieser Waechter existiert
//!
//! Dieselbe Klasse wie bei `parity_putrefaction.rs`: eine fertige Mechanik, die
//! kein Zauber aufruft, gilt ueber die Pruefsumme als fertig und ist es nicht.
//! `sublimation` wurde so zu #201, `energy` zu #219, `cooldown_reset` zu #223.
//!
//! # Und die Luecke, die er schliesst
//!
//! `sim/tests/factors.rs` prueft, dass alle Schadensfaktoren gleich stark
//! wirken — `synergy` musste dort herausgenommen werden, weil der Kampf ihn
//! seit #318 selbst schreibt und einen von aussen gesetzten Wert ueberschreibt.
//! Damit fiel auf der Rust-Seite jede Pruefung der Synergie-Staerke weg.
//!
//! Hier steht sie wieder, aber richtig herum: ueber die DATEN und die
//! KONSTANTE, nicht ueber einen Faktor, den niemand setzen kann.

use rota_sim::data;
use rota_sim::fight::SYNERGY_BONUS_PERCENT;

#[test]
fn ausgelieferte_zauber_benutzen_die_synergie() {
    // **Welche**, nicht **ob**. Ein `> 0` waere schon erfuellt, wenn irgendein
    // Zauber irgendetwas traegt; die Namen halten fest, was die Rechnung sieht.
    let oeffnen: Vec<&str> = data::SPELLS
        .iter()
        .filter(|s| !s.synergy_partner.is_empty())
        .map(|s| s.id)
        .collect();
    assert_eq!(
        oeffnen,
        vec!["canker"],
        "wer hier einen Zauber ergaenzt, aendert die Rechnung — dann gehoert \
         sie nachgemessen, nicht diese Liste erweitert"
    );

    // Der Partner muss es auch geben. Ein Fenster fuer einen Zauber, den es
    // nicht gibt, oeffnet nie — und faellt sonst nirgends auf.
    for spell in data::SPELLS.iter() {
        if spell.synergy_partner.is_empty() {
            continue;
        }
        assert!(
            data::SPELLS.iter().any(|s| s.id == spell.synergy_partner),
            "{} nennt einen Partner, den es nicht gibt: {}",
            spell.id,
            spell.synergy_partner
        );
        // **Das Fenster steht in WARTEZEITEN** (GAME-DESIGN 327: zwei bis
        // drei). Eine Zahl im Millisekunden-Bereich waere hier sofort
        // auffaellig — und genau der Fehler, der sonst erst bei hohem Tempo
        // sichtbar wuerde.
        assert!(
            (1..=3).contains(&spell.synergy_window_gcds),
            "{}: windowGcds {} liegt ausserhalb von GAME-DESIGN 327",
            spell.id,
            spell.synergy_window_gcds
        );
    }
}

#[test]
fn der_bonus_ist_auf_beiden_seiten_dieselbe_zahl() {
    // **Die Luecke aus factors.rs, an ihrer Wurzel.** Laufen die beiden Werte
    // auseinander, meldet es zwar die Pruefsumme ab Takt 0 — aber erst, wenn
    // eine Aufzeichnung das Paar ueberhaupt wirkt. Heute tut das keine.
    //
    // Die Zahl steht in `src/core/combat.ts` als SYNERGY_BONUS_PERCENT. Sie
    // ist GESETZT und nicht gemessen; wer sie aendert, aendert sie hier mit.
    assert_eq!(
        SYNERGY_BONUS_PERCENT, 25,
        "der Bonus wurde geaendert — dann gehoert er im Spiel mitgeaendert \
         (SYNERGY_BONUS_PERCENT in src/core/combat.ts)"
    );
}
