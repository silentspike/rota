//! Kandidatenzeilen auf einen laufenden Boss-Zauber (Issue #797, AC2).
//!
//! **Warum das eine eigene Stufe ist.** Dass die Sprache `boss.casting` kennt
//! (AC1), heißt nicht, dass die Suche es je benutzt: Sie baut ihre Rotationen
//! ausschließlich aus den Kandidatenzeilen, die `candidates_for` erzeugt. Ein
//! Wert ohne Zeile ist für sie nicht schwach bewertet, sondern unsichtbar.
//!
//! Genau daran hing #472: Der Balance-Läufer wählte bei keinem Gewicht einen
//! FIXATION-Zauber, und der Grund lag nicht in der Balance.

use rota_sim::apl::Value;
use rota_sim::data::{ControlEffect, SPELLS, SPELL_COUNT};
use rota_sim::rotation_search::{all_candidates, Origin};

/// Zauber, die an einem **laufenden** Boss-Zauber etwas ausrichten.
///
/// # `MakeInterruptible` stand hier und war falsch (#797)
///
/// PIN trägt diese Wirkung, und die Liste zählte ihn deshalb mit. Die
/// Zauberdaten sagen aber zweierlei über ihn, und beides widerspricht dem
/// „laufenden" Zauber:
///
/// > Den **NAECHSTEN** Boss-Zauber unterbrechbar machen.
/// > Traf er einen ohnehin unterbrechbaren, **verpufft** er.
///
/// Die erzeugte Zeile `pin if boss.casting` feuerte damit genau dann, wenn PIN
/// nichts ausrichtet: Es läuft ja schon etwas. Sie war gültig, wählbar und
/// wirkungslos — **formal erfüllt, sachlich leer**, und in jedem Bericht sah
/// sie aus wie Abdeckung.
///
/// PIN hat seit #797 seine eigene Zeile über `Origin::NextStrikeWindow` und
/// `boss.next_strike_in`; `sim/tests/next_strike_in.rs` hält sie fest. Die
/// beiden Richtungen unten sichern hier, dass er nicht **beide** trägt.
///
/// **Und `AllThree` gehört weiterhin dazu, obwohl der Name das Gegenteil
/// nahelegt:** CAPSTONE ruft `SlowCast`, `DelayStrike` und `HoldPhase`
/// (`Fight::apply_control_effect`) — es dehnt einen laufenden Zauber, macht
/// aber nichts unterbrechbar. Wer „alle drei bisherigen Wirkungen" als die drei
/// im `enum` davor liest, zählt `MakeInterruptible` mit und liegt falsch.
fn antwortet_auf_cast(index: usize) -> bool {
    let spell = &SPELLS[index];
    spell.interrupt
        || spell
            .control
            .is_some_and(|c| matches!(c.effect, ControlEffect::SlowCast | ControlEffect::AllThree))
}

/// **Träger-Zusicherung, und sie steht vorn.**
///
/// Alles Weitere zählt Zeilen für eine Gruppe von Zaubern. Ist die Gruppe leer,
/// wäre „0 Zeilen" richtig gezählt und ohne Aussage — und die Datei bliebe grün,
/// während AC2 unerfüllt ist.
#[test]
fn ac2_es_gibt_ueberhaupt_zauber_die_auf_einen_cast_antworten() {
    let wie_viele = (0..SPELL_COUNT).filter(|i| antwortet_auf_cast(*i)).count();
    assert!(
        wie_viele > 0,
        "Kein einziger Zauber trägt `interrupt` oder eine Kontroll-Wirkung, die \
         auf einen laufenden Zauber greift. Dann kann keine Zeile dieser Datei \
         etwas belegen."
    );
}

/// Jeder Zauber, der antworten kann, bekommt seine Zeile — und keiner sonst.
///
/// **Beide Richtungen zusammen.** „Mindestens eine Zeile existiert" wäre auch
/// erfüllt, wenn die Suche jedem der 57 Zauber eine `if boss.casting`-Zeile
/// gäbe; das wären 57 zusätzliche Messungen für einen Nutzen bei dreien.
#[test]
fn ac2_genau_die_antwortenden_zauber_bekommen_eine_zeile() {
    let mit_zeile: Vec<&str> = all_candidates()
        .iter()
        .filter(|c| c.origin == Origin::BossCastWindow)
        .map(|c| SPELLS[c.spell].id)
        .collect();
    let erwartet: Vec<&str> = (0..SPELL_COUNT)
        .filter(|i| antwortet_auf_cast(*i))
        .map(|i| SPELLS[i].id)
        .collect();
    assert_eq!(
        mit_zeile, erwartet,
        "Die Zeilen decken sich nicht mit den Zaubern, die auf einen Cast \
         antworten können. Zu wenige heißt: die Suche kann sie nicht wählen. \
         Zu viele heißt: sie misst Rotationen, in denen die Zeile nichts tut."
    );
}

/// Wer einen Schlag erst **angreifbar macht**, bekommt hier keine Zeile.
///
/// # Warum das eigens dasteht, obwohl der Fall darüber es mit abdeckt
///
/// `ac2_genau_die_antwortenden_zauber_bekommen_eine_zeile` vergleicht zwei
/// Listen. Wandert `MakeInterruptible` eines Tages wieder in `antwortet_auf_cast`
/// zurück, wandern **beide** Listen mit, und der Vergleich bleibt grün — der
/// Prüffall misst dann seine eigene Änderung.
///
/// Diese Zeile hängt nicht an der Hilfsfunktion, sondern an der Wirkung aus den
/// Daten. Sie kann deshalb scheitern, wo der Vergleich es nicht kann.
#[test]
fn ac2_wer_erst_angreifbar_macht_gehoert_nicht_hierher() {
    let mit_cast_zeile: Vec<&str> = all_candidates()
        .iter()
        .filter(|c| c.origin == Origin::BossCastWindow)
        .map(|c| SPELLS[c.spell].id)
        .collect();

    let macht_angreifbar: Vec<&str> = (0..SPELL_COUNT)
        .filter(|i| {
            SPELLS[*i]
                .control
                .is_some_and(|c| c.effect == ControlEffect::MakeInterruptible)
        })
        .map(|i| SPELLS[i].id)
        .collect();

    assert!(
        !macht_angreifbar.is_empty(),
        "kein Zauber traegt MakeInterruptible — dann belegt diese Zeile nichts"
    );
    for zauber in &macht_angreifbar {
        assert!(
            !mit_cast_zeile.contains(zauber),
            "{zauber} macht den NAECHSTEN Eintrag angreifbar und verpufft an \
             einem ohnehin unterbrechbaren; eine Zeile `if boss.casting` \
             feuert bei ihm genau dann, wenn er nichts ausrichtet. \
             Zeilen auf den laufenden Cast: {mit_cast_zeile:?}"
        );
    }
}

/// Die Zeile muss den NEUEN Wert lesen — sonst steht dort eine Bedingung, die
/// zufällig denselben Ursprung trägt.
#[test]
fn ac2_die_zeile_liest_boss_casting_und_nicht_irgendetwas() {
    let zeilen: Vec<_> = all_candidates()
        .into_iter()
        .filter(|c| c.origin == Origin::BossCastWindow)
        .collect();
    for zeile in &zeilen {
        let werte: Vec<Value> = zeile
            .condition
            .groups
            .iter()
            .flatten()
            .map(|term| term.left)
            .collect();
        assert!(
            werte.contains(&Value::BossCasting),
            "Die Zeile für {} trägt Origin::BossCastWindow, liest aber \
             `boss.casting` nicht: {:?}",
            SPELLS[zeile.spell].id,
            werte
        );
    }
}
