//! Hält fest, dass ein strikter Vergleich (`<`) auf einem zeitabhängigen Wert
//! **später** greift als der nicht-strikte (`<=`) — und um wie viel.
//!
//! # Was dieser Fall festhält, und was nicht
//!
//! Er hält den **gemessenen Ist-Zustand** fest, nicht den gewollten. Ob `<`
//! greifen soll, wie es dasteht, ist in #1027 offen und wird vom Auftraggeber
//! entschieden. Bis dahin ist der Verzug nicht abgesichert: Er kann sich
//! verschieben, ohne dass etwas rot wird — auch in die *richtige* Richtung,
//! dann wäre das Problem behoben und niemand wüsste es.
//!
//! **Wird dieser Fall rot, ist das eine Information, keine Panne.** Er gehört
//! dann angepasst, nicht repariert: Wer ihn stillschweigend auf die neuen
//! Zahlen zieht, löscht den einzigen Beleg dafür, dass sich etwas geändert hat.
//!
//! # Warum die Eigenschaft und nicht die Zahl
//!
//! Der Verzug ist **nicht konstant**. Gemessen am 2026-08-22:
//!
//! | Boss | erster Eintrag | Umschlagpunkt | `<=` greift | `<` greift | Verzug |
//! | --- | ---: | ---: | ---: | ---: | ---: |
//! | `test-survivable-strike` | 12000 | 9000 | 9000 | 10000 | 1000 ms |
//! | `boss-1-2` | 10000 | 7000 | 7000 | 7500 | 500 ms |
//!
//! Gleiche Zeile, gleiche Schwelle, verschiedener Verzug: Er ist der Abstand
//! bis zum **nächsten regulären Ereignis**, und der hängt davon ab, wo der
//! Umschlagpunkt ins Raster fällt. Eine einzelne Zahl festzunageln hieße, eine
//! Eigenschaft des Aufbaus für eine Eigenschaft der Sache zu halten.
//!
//! Deshalb prüft dieser Fall die **Richtung** (`<` greift später) an zwei
//! Aufbauten und die konkreten Punkte je Aufbau getrennt.
//!
//! # Die Ursache ist NICHT geklärt, und das steht hier absichtlich
//!
//! Die naheliegende Erklärung lautet: Der Weckruf aus `switch_point` wird auf
//! den Zeitpunkt gestellt, an dem der Wert die Schwelle **erreicht**; bei `<=`
//! ist die Bedingung dort wahr, bei `<` noch falsch, und ein zweiter Weckruf
//! kommt nicht.
//!
//! **Diese Erklärung ist gemessen widerlegt.** Am 2026-08-22 habe ich den
//! Weckruf für `BossNextStrikeIn` abgeschaltet
//! (`Value::BossNextStrikeIn => None` in `switch_point`) — alle Fälle hier
//! blieben grün, die Zeitpunkte änderten sich nicht.
//!
//! Dass die Mutation überhaupt ankam, ist mit einer Positiv-Kontrolle belegt:
//! Liefert `next_strike_in_ms()` konstant 0, werden drei dieser vier Fälle rot.
//! Der Bauweg trägt also; der Weckruf ist nur nicht die Ursache.
//!
//! Was übrig bleibt: Der Verzug ist der Abstand zum nächsten Zeitpunkt, an dem
//! die Rotation ohnehin entscheidet. **Woher dieses Raster kommt, ist offen**
//! und gehört zu #1027 — wer es klärt, sollte diesen Abschnitt ersetzen statt
//! ihn zu streichen.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::fight::{Fight, FightConfig};

/// Erster Eintrag bei 12000 ms → Umschlagpunkt für `< 3000` liegt bei 9000.
const BOSS_12000: &str = "test-survivable-strike";
/// Erster Eintrag bei 10000 ms → Umschlagpunkt bei 7000.
const BOSS_10000: &str = "boss-1-2";

fn wuerfe(boss: &str, text: &str, spell: &str) -> u32 {
    let apl: Apl = parse(AplName("probe"), text)
        .unwrap_or_else(|e| panic!("Prioritätenliste ist fehlerhaft:\n{e}"));
    let index = SPELLS
        .iter()
        .position(|s| s.id == spell)
        .unwrap_or_else(|| panic!("Zauber {spell} gibt es nicht"));
    Fight::new(FightConfig {
        boss: boss_by_id(boss).unwrap_or_else(|| panic!("Boss {boss} fehlt")),
        stop_on_kill: false,
        ..FightConfig::default()
    })
    .run_apl(&apl)
    .casts[index]
}

/// Kleinste obere Zeitschranke, unter der die Zeile noch wirft — also der
/// Zeitpunkt des ersten Wurfs, auf 250 ms genau.
fn erster_wurf(boss: &str, bedingung: &str) -> Option<u32> {
    (500..=20_000).step_by(250).find(|grenze| {
        let apl = format!("kindle if {bedingung} and time < {grenze}\npurge\n");
        wuerfe(boss, &apl, "kindle") > 0
    })
}

/// **Trägerprüfung der Messmethode.** Ohne sie belegt `erster_wurf` nichts:
/// Eine Fassung, die immer dieselbe Zahl liefert, sähe bei einer erwarteten
/// Zahl genauso aus wie eine richtige.
#[test]
fn die_messmethode_unterscheidet_ueberhaupt() {
    assert_eq!(
        erster_wurf(BOSS_12000, "time < 0"),
        None,
        "Eine Bedingung, die nie wahr ist, hat einen Wurf ergeben — dann misst \
         `erster_wurf` nicht die Bedingung, sondern irgendetwas anderes."
    );
    assert_eq!(
        erster_wurf(BOSS_12000, "time >= 0"),
        Some(500),
        "Eine Bedingung, die immer wahr ist, muss beim ersten möglichen \
         Zeitpunkt werfen."
    );
}

#[test]
fn strikt_greift_spaeter_als_nicht_strikt_bei_eintrag_12000() {
    let nicht_strikt = erster_wurf(BOSS_12000, "boss.next_strike_in <= 3000");
    let strikt = erster_wurf(BOSS_12000, "boss.next_strike_in < 3000");
    assert_eq!(
        nicht_strikt,
        Some(9250),
        "`<= 3000` soll am Umschlagpunkt 9000 greifen"
    );
    assert_eq!(
        strikt,
        Some(10250),
        "`< 3000` greift gemessen erst bei 10000"
    );
    assert!(
        strikt > nicht_strikt,
        "Der strikte Vergleich müsste später greifen als der nicht-strikte. \
         Tut er es nicht mehr, wurde #1027 entschieden — dann gehört dieser \
         Fall angepasst, nicht repariert."
    );
}

#[test]
fn derselbe_verzug_faellt_bei_anderem_raster_anders_aus() {
    let nicht_strikt = erster_wurf(BOSS_10000, "boss.next_strike_in <= 3000");
    let strikt = erster_wurf(BOSS_10000, "boss.next_strike_in < 3000");
    assert_eq!(
        nicht_strikt,
        Some(7250),
        "Umschlagpunkt liegt hier bei 7000"
    );
    assert_eq!(
        strikt,
        Some(7750),
        "`<` greift hier bei 7500, nicht bei 8000"
    );
    assert!(
        strikt > nicht_strikt,
        "Auch bei diesem Aufbau muss der strikte Vergleich später greifen."
    );
}

/// Der Verzug ist **500 ms hier und 1000 ms dort**. Diese Ungleichheit ist der
/// eigentliche Befund: Wer abwägt, ob ein Eingriff sich lohnt, darf die 1000
/// nicht für eine Eigenschaft der Sache halten.
#[test]
fn der_verzug_ist_nicht_konstant() {
    let verzug = |boss: &str| {
        let a = erster_wurf(boss, "boss.next_strike_in <= 3000").expect("`<=` warf nie");
        let b = erster_wurf(boss, "boss.next_strike_in < 3000").expect("`<` warf nie");
        b - a
    };
    let weit = verzug(BOSS_12000);
    let eng = verzug(BOSS_10000);
    assert_eq!(weit, 1000, "Verzug bei erstem Eintrag 12000");
    assert_eq!(eng, 500, "Verzug bei erstem Eintrag 10000");
    assert_ne!(
        weit, eng,
        "Wären beide gleich, wäre der Verzug eine Eigenschaft der Sprache. Er \
         ist eine des Aufbaus: der Abstand bis zum nächsten regulären Ereignis."
    );
}
