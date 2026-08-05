//! Grundwerte je Stufe, auf beiden Seiten gleich (Issue #248).
//!
//! **Nur die Wachstumstabelle, nicht die Erfahrungskurve.** Der Simulator misst
//! Kämpfe, nicht Fortschritt: Was auf Stufe 50 passiert, hängt an den
//! Grundwerten; wie lange jemand für Stufe 50 braucht, an der Kurve — und das
//! eine folgt nicht aus dem anderen. Eine gespiegelte Kurve wäre eine zweite
//! Stelle, die auseinanderlaufen kann, ohne dass irgendetwas davon abhinge.
//!
//! WARUM DIE TABELLE GANZZAHLIG IST
//! --------------------------------
//! `1,05^n` ist eine Fließkomma-Potenz. `Math.pow` in JavaScript und
//! `f64::powf` hier sind **nicht garantiert bitgleich** — und 18.4 verlangt,
//! dass beide Seiten dieselbe Zahl rechnen. Eine Tabelle rechnet gar nicht,
//! sie steht da; angewandt wird sie als Multiplikation und Ganzzahl-Division,
//! und beides ist exakt definiert.
//!
//! Die Zahlen unten stehen genauso im Prüffall des Spiels
//! (`tests/unit/level-growth.test.ts`). Laufen die Seiten auseinander, sagt es
//! dieser Fall in einer Zeile.
//!
//! ZU `unlockLevel`: Der Simulator liest es seit Langem (`build.rs`), benutzt
//! es aber nicht. Das ist bekannt und beabsichtigt — die Frage „welche Zauber
//! sind auf dieser Stufe verfügbar" gehört zur Boss-Machbarkeitsprüfung (19.4)
//! und damit zu E2, nicht hierher. Wer es ungenutzt findet, hat kein Versehen
//! gefunden.

use rota_sim::data::{stat_at_level, LEVEL_FACTOR_PER_MILLE, MAX_LEVEL, PLAYER};

#[test]
fn die_tabelle_hat_fuer_jede_stufe_einen_faktor() {
    assert_eq!(MAX_LEVEL, 100);
    assert_eq!(LEVEL_FACTOR_PER_MILLE.len(), MAX_LEVEL as usize);
    assert_eq!(
        LEVEL_FACTOR_PER_MILLE[0], 1000,
        "Stufe 1 traegt den Wert aus base.json, nicht mehr und nicht weniger"
    );
}

#[test]
fn die_grundwerte_treffen_dieselben_zahlen_wie_im_spiel() {
    // **Gerechnet, bevor gemessen wird.** Zwischen Stufe 1 und 100 liegen
    // NEUNUNDNEUNZIG Aufstiege: 1,05^99 = 125,2. Das Design nennt "~130" (8.1);
    // die Differenz ist genau eine Stufe, und der Halbsatz dort sagt
    // ausdruecklich, dass der Prozentsatz noch feinjustiert wird.
    assert_eq!(LEVEL_FACTOR_PER_MILLE[99], 125_239);

    // Dieselben Stufen wie im Prueffall des Spiels.
    assert_eq!(stat_at_level(PLAYER.vessel, 1), PLAYER.vessel);
    assert_eq!(stat_at_level(1_000, 1), 1_000);
    assert_eq!(stat_at_level(1_000, 10), 1_551);
    assert_eq!(stat_at_level(1_000, 50), 10_921);
    assert_eq!(stat_at_level(1_000, 100), 125_239);
}

#[test]
fn sie_waechst_gleichmaessig_und_nimmt_nie_ab() {
    // "Jede Stufe fuehlt sich gleich wertvoll an" (8.1). Ein Sprung in der
    // Mitte erfuellte die Endpunkte oben ebenfalls, waere aber ein anderes
    // Spiel — deshalb wird der Verlauf geprueft, nicht nur Anfang und Ende.
    for stufe in 2..=MAX_LEVEL {
        let vorher = f64::from(LEVEL_FACTOR_PER_MILLE[stufe as usize - 2]);
        let jetzt = f64::from(LEVEL_FACTOR_PER_MILLE[stufe as usize - 1]);
        let wachstum = jetzt / vorher;
        assert!(
            (1.04..1.06).contains(&wachstum),
            "Stufe {stufe} waechst um {wachstum}, nicht um rund 5 %"
        );
    }
}

#[test]
fn stufen_ausserhalb_der_tabelle_werden_geklemmt() {
    // Ein Lauf aus einer Fassung mit anderer Hoechststufe soll rechenbar
    // bleiben. Ein Absturz waere die schlechteste Antwort — dieselbe Regel wie
    // im Spiel.
    assert_eq!(stat_at_level(1_000, 0), stat_at_level(1_000, 1));
    assert_eq!(stat_at_level(1_000, 500), stat_at_level(1_000, MAX_LEVEL));
}

#[test]
fn die_rechnung_bleibt_ganzzahlig() {
    // Die Bedingung fuer 18.4: Was hier herauskommt, muss das Spiel bitgleich
    // treffen koennen. Ganze Zahlen kann es — deshalb Multiplikation und
    // Ganzzahl-Division, keine Potenz.
    //
    // 1000 * 1551 / 1000 = 1551 exakt; mit Fliesskomma waere 1550.9999... ein
    // moeglicher Ausgang, und die Seiten laegen um eins auseinander.
    assert_eq!(stat_at_level(1_000, 10), 1_551);
    assert_eq!(
        stat_at_level(1, 10),
        1,
        "1 * 1551 / 1000 = 1 nach Abschneiden"
    );
    assert_eq!(stat_at_level(0, 50), 0);
}
