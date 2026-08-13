//! Die SUBLIMATION-Talentwege wirken auch im Simulator (Issue #419, #738).
//!
//! # Warum diese Datei entstanden ist
//!
//! Die zwoelf Talent-Felder aus #419 stehen in `FightConfig` und werden
//! gelesen — aber **kein Rust-Test hat je gemessen, ob sie etwas bewirken**.
//! Der Orchestrator hat es an der schaerfsten Stelle belegt: Streicht man in
//! `fight.rs:967` die Wirkung von `talent_shield_amount_permille` ersatzlos,
//! bleibt `cargo test --no-fail-fast` bei **0 roten Zielen**. Nicht nur die
//! Tests fehlen — die Rechnung selbst ist ungedeckt.
//!
//! Der Grund liegt bei den Aufzeichnungen: Von 71 traegt **keine** Talente,
//! die Paritaet vergleicht also ausschliesslich Laeufe ohne. Sie faehrt den
//! Fall nie.
//!
//! # Die Groesse, an der es sich zeigt — und die, an der es sich NICHT zeigt
//!
//! Der erste Anlauf mass `cast_count` des langen Zaubers: Wer mehr Abklingzeit
//! abzieht, sollte ihn oefter herausbringen. Ergebnis **3 gegen 3, auch bei
//! dreifachem Abzug** — was wie ein wirkungsloses Talent aussah.
//!
//! Es war die falsche Groesse. Gemessen an der Restzeit statt an der Wurfzahl:
//!
//! ```text
//! aether-Restzeit nach 30 s   ohne Talent 26000 · +100% 20000 · +200% 14000
//! ```
//!
//! Der Faktor trug die ganze Zeit. `cast_count` ist durch GCD und Kampfdauer
//! gedeckelt und verschluckt die Wirkung; die Restzeit ist die Groesse, an der
//! die Verkuerzung passiert. Zwei falsche Vermutungen (Traeger zu klein,
//! `player_level`) und eine dritte des Orchestrators (`below = 0` sperrt die
//! Auswahl) gingen dieser Einsicht voraus — keine davon lag am Bau.

use rota_sim::data::SPELLS;
use rota_sim::fight::{Fight, FightConfig};

/// Der Zauber, dessen Abklingzeit gekuerzt wird — AETHER, ueber seinen
/// Bezeichner gesucht.
///
/// **Hier stand `const ZIEL: usize = 43`, und die Zahl ist am 2026-08-13
/// gekippt.** DECOCTION (#323) kam an Position 20 in den Katalog und schob
/// alles danach um eins: Index 43 zeigte auf DISTIL statt auf AETHER, und der
/// Fall meldete "der Weg reduction wirkt im Simulator nicht" — er mass einen
/// Zauber, den niemand kuerzt.
///
/// Der Kopf dieser Datei nennt AETHER beim Namen ("aether-Restzeit nach 30 s").
/// Die Zahl daneben war eine zweite Quelle fuer dieselbe Aussage, und sie
/// altert mit jedem neuen Zauber. Der Verkuerzer unten wird schon ueber seine
/// EIGENSCHAFT gesucht, mit der Begruendung "der Katalog darf sich aendern" —
/// das gilt hier genauso.
fn ziel() -> usize {
    SPELLS
        .iter()
        .position(|s| s.id == "aether")
        .expect("AETHER fehlt im Katalog")
}

/// Der erste Zauber mit einem Abzug — der Katalog darf sich aendern.
fn verkuerzer() -> usize {
    SPELLS
        .iter()
        .position(|s| s.cooldown_reduction_ms > 0)
        .expect("kein Zauber traegt cooldown_reduction_ms")
}

/// Das Ziel einmal wirken, damit es auf Abklingzeit steht; danach nur kuerzen.
fn takt(fight: &Fight) -> Option<usize> {
    if fight.cast_count(ziel()) == 0 && fight.cooldown_remaining_ms(ziel()) == 0 {
        return Some(ziel());
    }
    if fight.cooldown_remaining_ms(verkuerzer()) == 0 {
        return Some(verkuerzer());
    }
    None
}

/// Die Restzeit des Ziels nach 30 s — je kleiner, desto mehr wurde gekuerzt.
fn restzeit(config: FightConfig) -> u32 {
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(30_000),
        ..config
    });
    fight.run(takt);
    fight.cooldown_remaining_ms(ziel())
}

#[test]
fn der_traeger_bewegt_ueberhaupt_etwas() {
    // **AC8 aus #676, hier als erster Fall.** Ohne ihn koennten die Faelle
    // unten zwei Nullen gegeneinander messen: Wird der Verkuerzer nie
    // gewirkt oder trifft er nie, sind alle Restzeiten gleich — und ein
    // "kein Unterschied" saehe aus wie ein wirkungsloses Talent.
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(30_000),
        ..FightConfig::default()
    });
    fight.run(takt);

    assert!(
        fight.cast_count(verkuerzer()) > 0,
        "der Verkuerzer wurde nie gewirkt — die Probe hat keinen Traeger"
    );
    assert!(
        fight.cooldown_remaining_ms(ziel()) < SPELLS[ziel()].cooldown_ms,
        "die Abklingzeit des Ziels wurde gar nicht gekuerzt — der Traeger \
         traegt zwar, trifft aber nichts"
    );
}

#[test]
fn der_weg_reduction_kuerzt_mehr() {
    let ohne = restzeit(FightConfig::default());
    let mit = restzeit(FightConfig {
        talent_cooldown_reduction_permille: 1000,
        ..FightConfig::default()
    });

    assert!(
        mit < ohne,
        "der Weg reduction wirkt im Simulator nicht: Restzeit {ohne} ohne \
         Talent, {mit} mit doppeltem Abzug"
    );
}

#[test]
fn der_weg_reduction_waechst_gleichmaessig() {
    // **Zwei Stufen statt einer.** Ein einzelner Vergleich liesse einen
    // festen Zuschlag als Faktor durchgehen; die zweite Stufe trennt beides.
    // Bei doppeltem Abzug wird doppelt so viel gekuerzt wie ohne, bei
    // dreifachem dreifach — die Abstaende muessen deshalb gleich sein.
    let ohne = restzeit(FightConfig::default());
    let doppelt = restzeit(FightConfig {
        talent_cooldown_reduction_permille: 1000,
        ..FightConfig::default()
    });
    let dreifach = restzeit(FightConfig {
        talent_cooldown_reduction_permille: 2000,
        ..FightConfig::default()
    });

    assert_eq!(
        ohne - doppelt,
        doppelt - dreifach,
        "die Kuerzung waechst nicht gleichmaessig: {ohne} / {doppelt} / \
         {dreifach} — das spricht fuer einen Zuschlag statt eines Faktors"
    );
}

/// Der Verkuerzer MIT Schwelle — nur an ihm kann `threshold` etwas bewegen.
fn schwellen_verkuerzer() -> usize {
    SPELLS
        .iter()
        .position(|s| s.cooldown_reduction_below_ms > 0)
        .expect("kein Zauber traegt cooldown_reduction_below_ms")
}

/// Ein Ziel, dessen Restzeit zwischen alter und gehobener Schwelle liegt.
///
/// **Das ist der ganze Trick des Wirkungsfalls.** Die Schwelle sagt, WELCHE
/// Abklingzeiten gekuerzt werden: nur die unter ihr. Ein Ziel weit darunter
/// wird immer gekuerzt, eines weit darueber nie — beide zeigen nichts. Es
/// braucht eines dazwischen.
fn schwellen_ziel() -> usize {
    let grenze = SPELLS[schwellen_verkuerzer()].cooldown_reduction_below_ms;
    SPELLS
        .iter()
        .position(|s| s.cooldown_ms > grenze && s.cooldown_ms < grenze + 3_000)
        .expect("kein Ziel mit Abklingzeit knapp ueber der Schwelle")
}

/// Erst das Ziel, dann den Schwellen-Verkuerzer, sonst nichts.
fn schwellen_takt(fight: &Fight) -> Option<usize> {
    let (ziel, kurz) = (schwellen_ziel(), schwellen_verkuerzer());
    if fight.cast_count(ziel) == 0 {
        return Some(ziel);
    }
    if fight.cast_count(kurz) == 0 && fight.cooldown_remaining_ms(kurz) == 0 {
        return Some(kurz);
    }
    None
}

fn restzeit_des_schwellen_ziels(permille: u32) -> u32 {
    let mut fight = Fight::new(FightConfig {
        talent_cooldown_threshold_permille: permille,
        duration_ms: Some(5_000),
        ..FightConfig::default()
    });
    fight.run(schwellen_takt);
    fight.cooldown_remaining_ms(schwellen_ziel())
}

#[test]
fn der_traeger_der_schwelle_bewegt_ueberhaupt_etwas() {
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(5_000),
        ..FightConfig::default()
    });
    fight.run(schwellen_takt);
    assert!(
        fight.cast_count(schwellen_verkuerzer()) > 0,
        "der Schwellen-Verkuerzer wurde nie gewirkt"
    );
    assert!(
        fight.cast_count(schwellen_ziel()) > 0,
        "das Ziel wurde nie gewirkt, steht also gar nicht auf Abklingzeit"
    );
}

#[test]
fn der_weg_threshold_laesst_den_abzug_auf_mehr_zauber_greifen() {
    // Ohne Talent liegt die Restzeit des Ziels UEBER der Schwelle — es wird
    // uebersprungen. Mit vollem Weg liegt sie darunter, und derselbe Wurf
    // kuerzt es. Der Abzug selbst ist in beiden Laeufen gleich; nur die
    // AUSWAHL aendert sich, und genau das ist der Weg.
    let ohne = restzeit_des_schwellen_ziels(0);
    let mit = restzeit_des_schwellen_ziels(500);

    assert!(
        mit < ohne,
        "der Weg threshold wirkt im Simulator nicht: Restzeit {ohne} ohne \
         Talent, {mit} mit gehobener Schwelle"
    );
}

// **OFFEN: der Weg `charges` hat hier keinen Fall** (#738).
//
// Gemessen, nicht vermutet: Ein Lauf, der den Ruecksetzer wirkt und danach
// zwei Zauber auf Abklingzeit drueckt, ergibt mit und ohne Zuschlag dieselben
// Restzeiten — **und ohne Zuschlag wurde ebenfalls nichts zurueckgesetzt**:
//
// ```text
// ohne Bonus: (6000, 7500)   Ruecksetzer 1x gewirkt
// mit +5:     (6000, 7500)   Ruecksetzer 1x gewirkt
// ```
//
// Die zweite Zeile allein saehe nach einem wirkungslosen Talent aus. Die
// erste sagt, dass der Aufbau den Mechanismus gar nicht ausloest: Der
// Ruecksetzer ist ein AUSWAHLMODUS — er stellt scharf, und der Spieler sagt
// mit dem naechsten Druck, welchen Zauber er meint. Ein Takt, der einfach
// weiterdrueckt, trifft die Wahl nicht.
//
// Es fehlt also derselbe Traeger wie bei den anderen: ein Lauf, in dem die
// Groesse ueberhaupt etwas bewegt. Zwei Nullen gegeneinander zu messen ist
// woertlich AC8 aus #676.
#[test]
fn die_schwelle_laesst_den_abzug_unberuehrt() {
    // Die Kreuzprobe, die auf der Spiel-Seite die schaerfste ist: Schwelle und
    // Abzug sitzen an DERSELBEN Mechanik. Wer sie zusammenzoege, kuerzte hier
    // mehr.
    let ohne = restzeit(FightConfig::default());
    let mit_schwelle = restzeit(FightConfig {
        talent_cooldown_threshold_permille: 2000,
        ..FightConfig::default()
    });

    assert_eq!(
        mit_schwelle, ohne,
        "der Weg threshold hat den Abzug mitgehoben"
    );
}
