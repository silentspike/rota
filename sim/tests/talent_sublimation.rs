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

/// Der Zauber, dessen Abklingzeit gekuerzt wird.
const ZIEL: usize = 43;

/// Der erste Zauber mit einem Abzug — der Katalog darf sich aendern.
fn verkuerzer() -> usize {
    SPELLS
        .iter()
        .position(|s| s.cooldown_reduction_ms > 0)
        .expect("kein Zauber traegt cooldown_reduction_ms")
}

/// Das Ziel einmal wirken, damit es auf Abklingzeit steht; danach nur kuerzen.
fn takt(fight: &Fight) -> Option<usize> {
    if fight.cast_count(ZIEL) == 0 && fight.cooldown_remaining_ms(ZIEL) == 0 {
        return Some(ZIEL);
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
    fight.cooldown_remaining_ms(ZIEL)
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
        fight.cooldown_remaining_ms(ZIEL) < SPELLS[ZIEL].cooldown_ms,
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

// **OFFEN: der Weg `threshold` hat hier keinen Wirkungsfall.** Gemessen mit
// derselben Mutation, die den `reduction`-Faktor faengt:
//
// ```text
// Faktor reduction ersatzlos gestrichen  -> rot
// Schwellen-Faktor gestrichen            -> GRUEN   <- ungedeckt
// ```
//
// Der Grund ist inhaltlich, nicht handwerklich: `verkuerzer()` waehlt den
// ersten Zauber mit einem Abzug, und der traegt `below = 0` — also "keine
// Beschraenkung". Auf ihn kann eine Schwelle gar nicht wirken. Ein
// Wirkungsfall braucht einen Verkuerzer MIT Schwelle (`ascend`) und ein Ziel,
// dessen Restzeit ueber der alten und unter der gehobenen Schwelle liegt.
//
// Das ist dieselbe Lage wie bei `coagulation-longer`: eine Kreuzprobe, die
// belegt, dass ein Weg nichts Fremdes anfasst, waehrend niemand prueft, ob er
// das Eigene bewegt. Steht in #738.
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
