//! Die COAGULATION-Talentwege wirken auch im Simulator (Issue #738).
//!
//! # Die Messgroesse entscheidet, nicht die Wirkstelle
//!
//! Bei SUBLIMATION hat eine gedeckelte Groesse (`cast_count`, begrenzt durch
//! GCD und Kampfdauer) drei Laeufe lang **3 gegen 3** gemeldet, waehrend der
//! Faktor die ganze Zeit trug. Vier Vermutungen ueber die Wirkstelle gingen
//! ins Leere, weil die Frage die Groesse war.
//!
//! Deshalb steht hier bei jedem Weg dabei, ob seine Groesse einen Deckel hat:
//!
//! | Weg | Groesse | Deckel |
//! | --- | --- | --- |
//! | thicker | `shield()` | keiner |
//! | stacks | `shield_stacks()` | `max_stacks` — **und der IST das Ziel** |
//!
//! Bei `stacks` ist der Deckel kein Hindernis, sondern die gemessene Sache:
//! Der Weg hebt ihn. Die Probe muss ihn deshalb wirklich BERUEHREN — wer zu
//! selten wirft, misst eine Grenze, die nie erreicht wurde. Auf der
//! Spiel-Seite hat genau das sechs Lagen statt acht ergeben.

use rota_sim::data::SPELLS;
use rota_sim::fight::{Fight, FightConfig};

/// Der erste Zauber, der eine Schutzschicht aufbaut.
fn schild_zauber() -> usize {
    SPELLS
        .iter()
        .position(|s| s.shield.is_some())
        .expect("kein Zauber baut eine Schutzschicht auf")
}

/// Immer die Schicht, sobald sie bereit ist — so wird die Grenze beruehrt.
fn takt(fight: &Fight) -> Option<usize> {
    let s = schild_zauber();
    if fight.cooldown_remaining_ms(s) == 0 {
        return Some(s);
    }
    None
}

fn lauf(config: FightConfig) -> Fight {
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(60_000),
        ..config
    });
    fight.run(takt);
    fight
}

#[test]
fn der_traeger_bewegt_ueberhaupt_etwas() {
    // **Zuerst pruefen, dass der Traeger traegt.** Ohne diesen Fall messen die
    // folgenden zwei Nullen gegeneinander, und ein "kein Unterschied" saehe
    // aus wie ein wirkungsloses Talent (AC8 aus #676).
    let fight = lauf(FightConfig::default());
    assert!(
        fight.cast_count(schild_zauber()) > 0,
        "der Schild-Zauber wurde nie gewirkt — die Probe hat keinen Traeger"
    );
    assert!(
        fight.shield() > 0,
        "es steht keine Schutzschicht — die Groesse ist null, egal was das \
         Talent tut"
    );
    assert!(
        fight.shield_stacks() > 1,
        "es liegt nur eine Lage — die Stapel-Grenze wurde nie beruehrt, und \
         ein Talent auf sie koennte sich gar nicht zeigen"
    );
}

#[test]
fn der_weg_thicker_baut_mehr_schutz_auf() {
    let ohne = lauf(FightConfig::default()).shield();
    let mit = lauf(FightConfig {
        talent_shield_amount_permille: 500,
        ..FightConfig::default()
    })
    .shield();

    assert!(
        mit > ohne,
        "der Weg thicker wirkt im Simulator nicht: {ohne} Schutz ohne Talent, \
         {mit} mit anderthalbfacher Dicke"
    );
}

#[test]
fn der_weg_stacks_erlaubt_mehr_lagen() {
    let ohne = lauf(FightConfig::default()).shield_stacks();
    let mit = lauf(FightConfig {
        talent_shield_stacks_bonus: 5,
        ..FightConfig::default()
    })
    .shield_stacks();

    assert_eq!(
        mit,
        ohne + 5,
        "der Weg stacks wirkt nicht wie ein ganzzahliger Zuschlag: {ohne} \
         Lagen ohne Talent, {mit} mit fuenf Raengen"
    );
}

/// Steht nach `bis_ms` noch Schutz? Der Takt wirft **nur einmal**.
///
/// **Gemessen am Verfall, weil es keinen Getter fuer die Ablaufzeit gibt.**
/// Der reguläre Takt oben taugt dafuer nicht: Er wirft im Abstand der
/// Abklingzeit (6000 ms) neu, waehrend die Schicht 12000 ms haelt — es laeuft
/// also nie etwas ab. Dieselbe Falle wie bei `lagen()` auf der Spiel-Seite,
/// nur andersherum.
fn schutz_nach(bis_ms: u32, permille: u32) -> u32 {
    fn einmal(fight: &Fight) -> Option<usize> {
        let s = schild_zauber();
        if fight.cast_count(s) == 0 {
            return Some(s);
        }
        None
    }
    let mut fight = Fight::new(FightConfig {
        talent_shield_duration_permille: permille,
        duration_ms: Some(bis_ms),
        ..FightConfig::default()
    });
    fight.run(einmal);
    fight.shield()
}

#[test]
fn der_weg_longer_laesst_die_schicht_laenger_halten() {
    // Ein Zeitpunkt NACH der normalen Dauer und VOR der gehobenen: Ohne Talent
    // ist die Schicht dann weg, mit Talent steht sie noch. Ein Zeitpunkt vor
    // beiden oder nach beiden zeigte nichts — dieselbe Ueberlegung wie beim
    // Schwellen-Ziel in `talent_sublimation.rs`.
    let dauer = SPELLS[schild_zauber()]
        .shield
        .expect("der gewaehlte Zauber hat keine Schicht")
        .duration_ms;
    let dazwischen = dauer + dauer / 4; // 125 % — unter dem Faktor 1,5 unten

    assert_eq!(
        schutz_nach(dazwischen, 0),
        0,
        "die Schicht steht ohne Talent noch — der Messzeitpunkt liegt vor \
         ihrem Ablauf, dort kann sich kein Unterschied zeigen"
    );
    assert!(
        schutz_nach(dazwischen, 500) > 0,
        "der Weg longer wirkt im Simulator nicht: die Schicht ist bei \
         anderthalbfacher Dauer zum selben Zeitpunkt verfallen"
    );
}

#[test]
fn die_wege_bewegen_verschiedene_groessen() {
    // Die Kreuzprobe: `thicker` darf die Lagenzahl nicht heben, `stacks` nicht
    // die Dicke je Lage. Wer beide auf dieselbe Groesse legte, kaeme hier
    // durch — ohne diesen Fall.
    let ohne = lauf(FightConfig::default());
    let dicker = lauf(FightConfig {
        talent_shield_amount_permille: 500,
        ..FightConfig::default()
    });
    let mehr = lauf(FightConfig {
        talent_shield_stacks_bonus: 5,
        ..FightConfig::default()
    });

    assert_eq!(
        dicker.shield_stacks(),
        ohne.shield_stacks(),
        "thicker hat die Zahl der Lagen mitgehoben"
    );
    // **Nicht `shield()`, sondern die Dicke JE LAGE.** Wer mehr Lagen legt,
    // hat mehr Schutz — das ist der Weg, nicht sein Fehler. Dieselbe
    // Unterscheidung wie auf der Spiel-Seite.
    assert_eq!(
        mehr.shield() / mehr.shield_stacks(),
        ohne.shield() / ohne.shield_stacks(),
        "stacks hat die Dicke je Lage mitgehoben"
    );
}
