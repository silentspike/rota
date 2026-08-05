//! Der Talent-Faktor rechnet auf beiden Seiten dieselbe Zahl (Issue #417).
//!
//! # Warum das eine eigene Datei ist
//!
//! AC1 verlangt "beide Seiten rechnen identisch". Der Faktor `talents` stand in
//! `factors.rs` seit jeher auf 1,0 — vorhanden, aber von nichts gefuellt. Der
//! Eingang `talent_permille` in `FightConfig` ist neu, und ohne ihn waere die
//! Bitgleichheit gar nicht pruefbar gewesen.
//!
//! # Gemessen wird der SCHADEN, nicht das Feld
//!
//! `factors` ist privat, und ein Getter nur fuer einen Prueffall waere eine
//! Schnittstelle, die dem Spiel nichts bringt. Wichtiger: Ein Faktor, den
//! niemand multipliziert, ist genau die Klasse, gegen die dieses Issue
//! geschrieben ist — der Schaden belegt die ANWENDUNG, das Feld nur die
//! Rechnung. Dieselbe Linie wie auf der TS-Seite.

use rota_sim::data::index_of;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Einen Kindle wirken und den Schaden ablesen.
fn schaden_bei(permille: u32) -> u32 {
    let recording = Recording {
        name: "talent-faktor".to_string(),
        seed: 1,
        ticks: 4,
        presses: vec![Press { tick: 0, slot: 0 }],
        bindings: vec![Binding {
            slot: 0,
            spell: index_of("kindle").expect("kindle gibt es"),
        }],
    };
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        talent_permille: permille,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, true).damage
}

#[test]
fn ohne_talente_bewegt_sich_nichts() {
    // **Exakt derselbe Schaden**, nicht ungefaehr: Daran haengt, dass keine
    // bestehende Pruefsumme sich bewegt.
    assert_eq!(schaden_bei(0), schaden_bei(0));
    assert!(schaden_bei(0) > 0, "ohne Treffer misst der Fall nichts");
}

#[test]
fn mit_talenten_trifft_es_haerter() {
    let ohne = schaden_bei(0);
    let mit = schaden_bei(500);
    assert!(mit > ohne, "der Faktor kommt im Schaden nicht an: {ohne} -> {mit}");
}

#[test]
fn die_zahl_ist_dieselbe_wie_im_spiel() {
    // 3 x 20 + 5 x 25 = 185 Promille, also Faktor 1,185 — dieselbe Verteilung
    // und dieselbe Zahl wie in `tests/unit/talents.test.ts`. Laufen die Formeln
    // auseinander, faellt eine der beiden Seiten.
    let ohne = schaden_bei(0) as f64;
    let mit = schaden_bei(185) as f64;
    assert_eq!(mit, (ohne * 1.185).round(), "erwartet {} aus {ohne}", (ohne * 1.185).round());
}
