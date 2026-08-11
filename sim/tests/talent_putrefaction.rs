//! Die PUTREFACTION-Talentwege wirken auch im Simulator (Issue #738).
//!
//! # Die Deckel-Tabelle steht vorn, wie in den anderen beiden Dateien
//!
//! | Weg | Groesse | Deckel |
//! | --- | --- | --- |
//! | build | `decay_stacks()` | `decay_cap` — **muss beruehrt werden** |
//! | duration | `decay_expiry_at()` | keiner |
//! | effect | Verwundbarkeit | **kein Getter** — siehe unten |
//!
//! Der Grund fuer die Tabelle steht in `talent_sublimation.rs`: Eine gedeckelte
//! Groesse zeigt eine Steigerung nur unterhalb ihres Deckels und ist darueber
//! stumm — und stumm sieht aus wie wirkungslos.

use rota_sim::data::SPELLS;
use rota_sim::fight::{Fight, FightConfig};

/// Der erste Zauber, der Faeulnis-Stapel legt.
fn stapel_zauber() -> usize {
    SPELLS
        .iter()
        .position(|s| s.decay_stacks > 0)
        .expect("kein Zauber legt Faeulnis-Stapel")
}

/// Immer den Stapel-Zauber, sobald er bereit ist.
fn takt(fight: &Fight) -> Option<usize> {
    let s = stapel_zauber();
    if fight.cooldown_remaining_ms(s) == 0 {
        return Some(s);
    }
    None
}

fn lauf(config: FightConfig, dauer_ms: u32) -> Fight {
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(dauer_ms),
        ..config
    });
    fight.run(takt);
    fight
}

#[test]
fn der_traeger_bewegt_ueberhaupt_etwas() {
    // **Zuerst der Traeger, dann die Wirkung** — sonst messen die Faelle
    // unten zwei Nullen gegeneinander (AC8 aus #676). Beim ersten Anlauf an
    // SUBLIMATION hat das Fehlen dieser Zusicherung vier Vermutungen und drei
    // Laeufe gekostet.
    let fight = lauf(FightConfig::default(), 30_000);
    assert!(
        fight.cast_count(stapel_zauber()) > 0,
        "der Stapel-Zauber wurde nie gewirkt — die Probe hat keinen Traeger"
    );
    assert!(
        fight.decay_stacks() > 0,
        "es liegt kein Stapel — die Groesse ist null, egal was das Talent tut"
    );
}

#[test]
fn der_weg_build_legt_mehr_stapel() {
    // **Ein KURZER Lauf, damit die Obergrenze nicht erreicht wird.** Bei 30 s
    // lagen beide Laeufe bei 5 Stapeln — dem `decay_cap`. Die Zahl bewegte
    // sich nicht, und das sah aus wie ein wirkungsloses Talent, obwohl der
    // Faktor trug. Genau der Fall, vor dem die Deckel-Tabelle oben warnt, und
    // er ist mir hier trotzdem passiert.
    //
    // 2000 ms reichen fuer wenige Wuerfe; der Weg vervielfacht die Stapel JE
    // Wirkung, also zeigt er sich schon dort.
    let ohne = lauf(FightConfig::default(), 2_000).decay_stacks();
    let mit = lauf(
        FightConfig {
            talent_decay_stack_permille: 1000,
            ..FightConfig::default()
        },
        2_000,
    )
    .decay_stacks();

    assert!(
        mit > ohne,
        "der Weg build wirkt im Simulator nicht: {ohne} Stapel ohne Talent, \
         {mit} mit doppeltem Aufbau. Liegt die Zahl in beiden Laeufen auf der \
         Obergrenze, ist nicht das Talent stumm, sondern die Messung"
    );
}

#[test]
fn der_weg_duration_laesst_den_stapel_laenger_liegen() {
    // **Der erste Stapel, nicht die Zahl der Stapel.** `decay_expiry_at(0)`
    // liest den Ablaufzeitpunkt eines einzelnen Stapels und hat keinen Deckel —
    // anders als `decay_stacks()`, das an `decay_cap` haengt.
    let ohne = lauf(FightConfig::default(), 5_000).decay_expiry_at(0);
    let mit = lauf(
        FightConfig {
            talent_decay_duration_permille: 500,
            ..FightConfig::default()
        },
        5_000,
    )
    .decay_expiry_at(0);

    assert!(
        mit > ohne,
        "der Weg duration wirkt im Simulator nicht: Ablauf bei {ohne} ohne \
         Talent, bei {mit} mit anderthalbfacher Haltbarkeit"
    );
}

#[test]
fn die_dauer_laesst_die_stapelzahl_unberuehrt() {
    // Kreuzprobe: `duration` verlaengert, `build` vermehrt. Wer beide auf
    // dieselbe Groesse legte, kaeme ohne diesen Fall durch.
    //
    // **Auch hier 2000 ms, nicht 30 000.** Eine Kreuzprobe, die zwei Laeufe auf
    // der Obergrenze vergleicht, ist immer gruen — sie belegt dann nicht, dass
    // `duration` die Stapelzahl in Ruhe laesst, sondern nur, dass beide den
    // Deckel erreichen. Der Fall stand eine Zeile unter dem, an dem ich denselben
    // Fehler gerade behoben hatte.
    let ohne = lauf(FightConfig::default(), 2_000).decay_stacks();
    let mit_dauer = lauf(
        FightConfig {
            talent_decay_duration_permille: 500,
            ..FightConfig::default()
        },
        2_000,
    )
    .decay_stacks();

    assert_eq!(
        mit_dauer, ohne,
        "der Weg duration hat die Zahl der Stapel mitgehoben"
    );
}

// **OFFEN: der Weg `effect` hat hier keinen Fall** (#738).
//
// Er hebt die Verwundbarkeit (`fight.rs:2666` und `:2802`), und dafuer gibt es
// keinen Getter. Messbar waere er am Schaden — aber der haengt an so vielen
// anderen Groessen, dass ein Unterschied dort nichts ueber DIESEN Weg sagt.
//
// Einen Getter allein fuer den Test zu bauen waere ein zweiter Messpunkt neben
// der Sache; das ist gemeldet statt gebaut.
