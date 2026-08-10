//! Wirkt RETORT? (Issue #480)
//!
//! **Nicht am Feld gemessen, sondern an der Wartezeit.** Ein Zauber, dessen
//! Weglassen nichts bewegt, ist nicht angeschlossen — AC3 des Issues.
//!
//! Der Aufbau ist derselbe wie in `abbruch_eigenschaften.rs`: `test-caster`
//! unterbricht bei 9000 ms, `test-cast` hat 2000 ms Wirkzeit. Wer bei 8000 ms
//! drückt, wird mittendrin getroffen und hat bei 9000 noch 500 ms Wartezeit
//! offen, die er bezahlt und nicht bekommen hat.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Wartezeit-Ende nach dem Lauf. Ohne RETORT bleibt sie stehen, mit ihm
/// rückt sie nach vorn.
fn gcd_ende(mit_retort: bool) -> u32 {
    let test_cast = index_of("test-cast").expect("test-cast");
    let retort = index_of("retort").expect("RETORT steht im Zauberkasten");
    let vapour = index_of("vapour").expect("vapour");
    let boss = boss_by_id("test-caster").expect("test-caster");

    // **Die Kontrolle wirft VAPOUR statt nichts.** Sonst unterschieden sich
    // die Läufe um RETORT UND um einen Tastendruck — und ein Druck kostet
    // selbst eine Wartezeit.
    let zweiter = if mit_retort { retort } else { vapour };

    let recording = Recording {
        name: "retort-probe".to_owned(),
        seed: 0,
        ticks: 1000,
        bindings: vec![
            Binding {
                slot: 0,
                spell: test_cast,
            },
            Binding {
                slot: 1,
                spell: zweiter,
            },
        ],
        presses: vec![
            Press { tick: 800, slot: 0 },
            // Nach dem Abbruch bei 9000 ms.
            Press { tick: 901, slot: 1 },
        ],
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(1000 * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);
    println!(
        "  angenommen {} abgelehnt {} | retort-Wuerfe {}",
        ergebnis.accepted,
        ergebnis.rejected,
        ergebnis.casts[rota_sim::data::index_of("retort").unwrap()]
    );
    fight.gcd_ready_at()
}

#[test]
fn ac2_retort_gibt_die_verlorene_wartezeit_zurueck() {
    let mit = gcd_ende(true);
    let ohne = gcd_ende(false);
    assert!(
        mit < ohne,
        "RETORT verkürzt die Wartezeit nicht: mit {mit}, ohne {ohne} — \
         wird der gemerkte Betrag beim Wurf abgezogen?"
    );
    println!(
        "Wartezeit-Ende: mit RETORT {mit}, ohne {ohne}, Ersparnis {}",
        ohne - mit
    );
}

/// **Die Gegenrichtung: Beim SELBST-Abbruch erstattet RETORT nichts.**
///
/// Ohne diesen Fall belegte der obige nur, dass RETORT *irgendwann* wirkt —
/// nicht, dass er den einen Weg trifft und den anderen nicht. Und genau die
/// Unterscheidung ist der ganze Inhalt der Entscheidung: Der Boss-Abbruch ist
/// eine Störung, der eigene Druck eine Wahl.
///
/// Aufbau ohne unterbrechenden Boss: `test-cast` wird geworfen, und ein
/// zweiter Druck während seiner Wirkzeit bricht ihn selbst ab.
#[test]
fn ac3_der_selbst_abbruch_erstattet_nichts() {
    let test_cast = index_of("test-cast").expect("test-cast");
    let retort = index_of("retort").expect("retort");
    let kindle = index_of("kindle").expect("kindle");

    let recording = Recording {
        name: "retort-selbstabbruch".to_owned(),
        seed: 0,
        ticks: 600,
        bindings: vec![
            Binding {
                slot: 0,
                spell: test_cast,
            },
            Binding {
                slot: 1,
                spell: kindle,
            },
            Binding {
                slot: 2,
                spell: retort,
            },
        ],
        presses: vec![
            // Wirkzeit beginnt, Wartezeit läuft ab hier.
            Press { tick: 0, slot: 0 },
            // Mitten in der Wirkzeit: der eigene Druck bricht ab.
            Press { tick: 50, slot: 1 },
            // Und jetzt RETORT — er darf nichts vorfinden.
            Press { tick: 60, slot: 2 },
        ],
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        duration_ms: Some(600 * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);

    // **Die Zusicherung zuerst**: Ohne einen tatsächlichen Selbst-Abbruch
    // misst der Fall eine Runde, in der es nichts zu erstatten GÄBE — und
    // wäre grün, ohne die Unterscheidung zu belegen.
    assert_eq!(
        ergebnis.casts[test_cast], 0,
        "test-cast ist durchgekommen — dann hat kein Selbst-Abbruch \
         stattgefunden und der Fall belegt nichts"
    );
    assert!(
        ergebnis.casts[retort] > 0,
        "RETORT wurde nicht geworfen — dann sagt der Vergleich unten nichts"
    );

    println!(
        "  SELBST-Abbruch: gcd_ready_at={} | kindle {} | retort {} | test-cast {}",
        fight.gcd_ready_at(),
        ergebnis.casts[kindle],
        ergebnis.casts[retort],
        ergebnis.casts[test_cast]
    );
    // **1500, nicht 2000** — und der Unterschied ist selbst eine Messung:
    // Der zweite Druck hat `test-cast` zwar ABGEBROCHEN, KINDLE aber nicht
    // gewirkt (gemessen: `casts[kindle] == 0`), weil die Wartezeit des
    // ersten Wurfs noch lief. Die 1500 ms stammen also unverändert vom Wurf
    // bei 0 ms.
    //
    // Hätte RETORT erstattet, stünde hier weniger: Beim Abbruch bei 500 ms
    // waren noch 1000 ms offen, die Wartezeit endete dann bei 500.
    assert_eq!(
        ergebnis.casts[kindle], 0,
        "KINDLE ist durchgekommen — dann misst die Zahl unten eine andere \
         Wartezeit als die des abgebrochenen Wurfs"
    );
    assert_eq!(
        fight.gcd_ready_at(),
        1500,
        "die Wartezeit wurde verkürzt, obwohl NUR ein Selbst-Abbruch \
         stattfand — greift RETORT neuerdings auch dort?"
    );
}
