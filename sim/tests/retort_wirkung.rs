//! **RETORT rüstet vor, statt nachträglich zu retten** (Issue #480).
//!
//! Das Raster beschreibt ihn reaktiv — „ein unterbrochener Zauber ist sofort
//! wieder bereit". Gebaut ist er präventiv, und das ist eine Entscheidung des
//! Auftraggebers: Reaktiv müsste er während der Wartezeit werfbar sein, die er
//! verkürzen soll, und das bräuchte eine zweite Ausnahme von Design-Gesetz 6.6.
//!
//! Gemessen, dass es die nicht gibt: mit `triggersGcd: true` und reaktiver
//! Bauart wurde sein Druck abgelehnt — 1 angenommen, 1 abgelehnt, 0 Würfe.
//!
//! Aufbau aller Fälle: `test-caster` unterbricht bei 9000 ms, `test-cast` hat
//! 2000 ms Wirkzeit. Wer im Fenster startet, wird mittendrin getroffen.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Fährt einen Lauf und gibt (Wartezeit-Ende, Rüstung steht noch) zurück.
fn lauf(mit_retort: bool, selbst_abbruch: bool, ticks: u32) -> (u32, bool, u32) {
    let test_cast = index_of("test-cast").expect("test-cast");
    let retort = index_of("retort").expect("RETORT steht im Zauberkasten");
    let vapour = index_of("vapour").expect("vapour");
    let quench = index_of("quench").expect("quench");
    let boss = boss_by_id("test-caster").expect("test-caster");

    // **Die Kontrolle wirft VAPOUR statt nichts.** Sonst unterschieden sich
    // die Läufe um RETORT UND um einen Tastendruck — und ein Druck kostet
    // selbst eine Wartezeit.
    let ruester = if mit_retort { retort } else { vapour };

    let mut presses = vec![
        // Erst rüsten, lange vor dem Schlag.
        Press { tick: 100, slot: 1 },
        // Dann in den Abbruch hinein: Start bei 7600, Wirkzeit bis 9600.
        Press { tick: 760, slot: 0 },
    ];
    if selbst_abbruch {
        // **QUENCH, nicht KINDLE.** Der eigene Druck muss ANGENOMMEN werden,
        // um abzubrechen — und während RETORTs Wartezeit ist nur werfbar, wer
        // keine auslöst (GAME-DESIGN 6.6). Gemessen: Mit KINDLE wurde der
        // Druck abgelehnt, es gab gar keinen Selbst-Abbruch, und der Fall
        // maß denselben Boss-Abbruch wie der Fall darüber.
        presses.push(Press { tick: 800, slot: 2 });
    }

    let recording = Recording {
        name: "retort-probe".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![
            Binding {
                slot: 0,
                spell: test_cast,
            },
            Binding {
                slot: 1,
                spell: ruester,
            },
            Binding {
                slot: 2,
                spell: quench,
            },
        ],
        presses,
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        player_level: 101,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, false);
    (
        fight.gcd_ready_at(),
        fight.retort_armed(),
        fight.self_cancelled_count(),
    )
}

#[test]
fn ac2_die_ruestung_erlaesst_den_rest_der_wartezeit() {
    let (mit, _, _) = lauf(true, false, 1000);
    let (ohne, _, _) = lauf(false, false, 1000);
    assert!(
        mit < ohne,
        "die Wartezeit läuft mit RETORT genauso lange wie ohne: {mit} gegen \
         {ohne} — greift die Rüstung beim Boss-Abbruch?"
    );
    // Der Abbruch fällt bei 9000; ab da ist der Spieler frei.
    assert_eq!(
        mit, 9000,
        "der Rest wurde nicht bis auf den Abbruch erlassen"
    );
    println!("Wartezeit-Ende: mit {mit}, ohne {ohne}");
}

#[test]
fn ac3_der_selbst_abbruch_verbraucht_die_ruestung_nicht() {
    // **Die Gegenrichtung, und sie wiegt unter dieser Bauart schwerer.**
    // Wer sich rüstet und dann selbst abbricht, hätte sich die Wartezeit
    // gekauft. Die Rüstung muss stehen bleiben — weder greifen noch
    // verbraucht werden.
    let (_, steht_noch, selbst) = lauf(true, true, 1000);
    // **Die Zusicherung zuerst.** Ohne einen tatsächlichen Selbst-Abbruch
    // misst der Fall denselben Boss-Abbruch wie der darüber — und wäre rot
    // aus dem falschen Grund.
    assert!(
        selbst > 0,
        "kein einziger Selbst-Abbruch im Lauf — dann prüft dieser Fall etwas \
         anderes, als sein Name sagt"
    );
    assert!(
        steht_noch,
        "die Rüstung ist weg, obwohl NUR ein Selbst-Abbruch stattfand — \
         verbraucht ihn `cancel_player_cast` neuerdings auch?"
    );
}

#[test]
fn ac4_doppeltes_ruesten_bleibt_eine_ruestung() {
    // **Zweimal rüsten, EINMAL unterbrochen werden — danach ist nichts übrig.**
    //
    // Zählte der Merker hoch, bliebe nach dem einen Verbrauch eine zweite
    // Rüstung stehen, und man könnte sie vorrätig halten. Er ist ein Ja/Nein;
    // der Typ garantiert das heute, und dieser Fall hält es fest, falls
    // jemand ihn später zu einer Zahl macht.
    //
    // Ein erster Entwurf ließ zweimal rüsten und zweimal unterbrechen und
    // erwartete null Rüstungen. Das war falsch konstruiert: Der zweite Schlag
    // bei 18000 findet gar keinen laufenden Wurf vor, also verbraucht er
    // nichts — der Fall wäre rot geworden, ohne dass etwas kaputt ist.
    let test_cast = index_of("test-cast").expect("test-cast");
    let retort = index_of("retort").expect("retort");
    let boss = boss_by_id("test-caster").expect("test-caster");
    let recording = Recording {
        name: "retort-doppelt".to_owned(),
        seed: 0,
        ticks: 1000,
        bindings: vec![
            Binding {
                slot: 0,
                spell: test_cast,
            },
            Binding {
                slot: 1,
                spell: retort,
            },
        ],
        presses: vec![
            Press { tick: 10, slot: 1 },
            // RETORT hat 12 s Abklingzeit — der zweite Wurf braucht einen
            // eigenen Lauf. Hier genügt der Beleg, dass ein zweiter Druck
            // nichts ANSAMMELT: Er wird abgelehnt, die Rüstung bleibt eine.
            Press { tick: 20, slot: 1 },
            // In den Abbruch bei 9000 hinein.
            Press { tick: 760, slot: 0 },
        ],
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        player_level: 101,
        duration_ms: Some(1000 * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);

    // Die Zusicherung: Es gab wirklich einen Abbruch, sonst sagt der Rest
    // nichts.
    assert_eq!(
        ergebnis.casts[test_cast], 0,
        "test-cast ist durchgekommen — dann gab es keinen Boss-Abbruch"
    );
    assert!(
        !fight.retort_armed(),
        "nach dem Verbrauch steht noch eine Rüstung — sammelt der Merker?"
    );
}

#[test]
fn ac5_eine_ruestung_ohne_abbruch_ueberdauert_den_kampf() {
    // Sie muss stehen bleiben dürfen, ohne etwas zu verfälschen — und sie
    // steht dann am Kampfende noch in der Prüfsumme. Das ist richtig so:
    // Ein Zustand, der verschwiegen wird, ist von der Parität nicht erfasst.
    let retort = index_of("retort").expect("retort");
    // FIRST_CRUCIBLE unterbricht nicht — hier kommt kein Abbruch.
    let recording = Recording {
        name: "retort-unverbraucht".to_owned(),
        seed: 0,
        ticks: 500,
        bindings: vec![Binding {
            slot: 0,
            spell: retort,
        }],
        presses: vec![Press { tick: 10, slot: 0 }],
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        duration_ms: Some(500 * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);
    assert_eq!(ergebnis.casts[retort], 1, "RETORT wurde nicht geworfen");
    assert!(
        fight.retort_armed(),
        "die Rüstung ist verschwunden, obwohl es keinen Abbruch gab — \
         verfällt sie neuerdings von selbst?"
    );
}
