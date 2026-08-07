//! Die Prioritätenliste als Sprache (Issue #55).
//!
//! Vier Nachweise: Die beiden Rotationen aus `STEP1-SPEC` § 4 ergeben als
//! geschriebene Liste dieselben Zahlen wie als Rust-Funktion, eine fehlerhafte
//! Liste nennt Zeile und Grund, und die Auswertung fordert keinen Speicher an.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::data::{KINDLE, PYRE, SMOULDER};
use rota_sim::{rotation, Fight, FightConfig, RunResult};

// Derselbe thread-lokale Zähl-Allocator wie in `tests/allocation.rs`. Er steht
// hier noch einmal, weil jede Testdatei ein eigenes Programm ist und einen
// eigenen globalen Allocator braucht.
thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

// SAFETY: Jede Anfrage wird unverändert an den System-Allocator weitergereicht.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn count<T>(f: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATIONS.with(|c| c.set(0));
    let value = f();
    (value, ALLOCATIONS.with(Cell::get))
}

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");
/// Die schlechte Rotation, wie sie im Repository liegt.
const BAD: &str = include_str!("../apl/step1-bad.apl");

fn load(name: &str, text: &str) -> Apl {
    parse(AplName(name), text).unwrap_or_else(|error| panic!("{name} ist fehlerhaft:\n{error}"))
}

fn run_apl(apl: &Apl, stop_on_kill: bool) -> RunResult {
    Fight::new(FightConfig {
        stop_on_kill,
        ..FightConfig::default()
    })
    .run_apl(apl)
}

#[test]
fn ac1_die_gute_rotation_als_apl_ergibt_4240() {
    let apl = load("step1-good", GOOD);
    let r = run_apl(&apl, true);

    assert_eq!(r.damage, 4240, "Schaden gesamt");
    assert_eq!(r.killed_at_ms, Some(40_510), "Sieg-Zeitpunkt");
    assert_eq!(r.casts[KINDLE], 20);
    assert_eq!(r.casts[SMOULDER], 4);
    assert_eq!(r.casts[PYRE], 4);
    assert_eq!(r.lost_dot_ticks[SMOULDER], 0);
    assert_eq!(r.idle_ms, 0);
}

#[test]
fn ac2_die_schlechte_rotation_als_apl_ergibt_4044() {
    let apl = load("step1-bad", BAD);
    let r = run_apl(&apl, true);

    assert_eq!(r.damage, 4044, "Schaden gesamt");
    assert_eq!(r.killed_at_ms, None, "der Boss muss ueberleben");
    assert_eq!(r.casts[KINDLE], 24);
    assert_eq!(r.casts[SMOULDER], 8);
    assert_eq!(r.casts[PYRE], 8);
    assert_eq!(r.lost_dot_ticks[SMOULDER], 14);
}

#[test]
fn die_apl_rechnet_genau_wie_die_rust_fassung() {
    // Der eigentliche Beleg dafuer, dass die Sprache dieselbe Rotation
    // ausdrueckt: nicht nur derselbe Endschaden, sondern dasselbe Ergebnis in
    // jedem Feld. Ein anderer Weg zum selben Schaden waere kein Gleichlauf.
    for (name, text, funktion) in [
        ("gut", GOOD, rotation::good as rota_sim::fight::Rotation),
        ("schlecht", BAD, rotation::bad),
    ] {
        for stop_on_kill in [true, false] {
            let apl = load(name, text);
            let aus_datei = run_apl(&apl, stop_on_kill);
            let aus_code = Fight::new(FightConfig {
                stop_on_kill,
                ..FightConfig::default()
            })
            .run(funktion);
            assert_eq!(
                aus_datei, aus_code,
                "Rotation \"{name}\" weicht ab (stop_on_kill = {stop_on_kill})"
            );
        }
    }
}

#[test]
fn ac3_ein_unbekannter_zauber_nennt_zeile_und_grund() {
    let text = "# Kopf\nsmoulder if dot.smoulder.remains < gcd\nfeuerball if heat > 3\nkindle\n";
    let error = parse(AplName("kaputt"), text).expect_err("muss scheitern");

    assert_eq!(error.line, 3, "Zeilennummer");
    assert_eq!(error.column, 1, "Spalte");
    assert!(
        error.message.contains("feuerball"),
        "die Meldung nennt den Fehler nicht: {}",
        error.message
    );
    assert!(
        error.message.contains("kindle"),
        "die Meldung sagt nicht, was moeglich waere: {}",
        error.message
    );

    // Die Darstellung zeigt Zeile, Wortlaut und einen Zeiger auf die Fundstelle.
    let gezeigt = error.to_string();
    assert!(gezeigt.contains("Zeile 3"), "{gezeigt}");
    assert!(gezeigt.contains("feuerball if heat > 3"), "{gezeigt}");
    assert!(gezeigt.contains('^'), "{gezeigt}");
}

#[test]
fn ac3_jeder_fehlertyp_nennt_die_richtige_stelle() {
    // Jede Zeile hier ist ein Fehler, den jemand tatsaechlich macht. Geprueft
    // wird die Zeilennummer und dass die Meldung den Grund benennt — nicht der
    // Wortlaut, der sich aendern darf.
    let faelle: [(&str, usize, &str); 8] = [
        ("kindle\npyre wenn heat > 3\n", 2, "if"),
        ("kindle\npyre if\n", 2, "fehlt"),
        ("kindle\npyre if heat >\n", 2, "fehlt"),
        ("kindle\npyre if hitze > 3\n", 2, "Unbekannter Wert"),
        ("kindle\npyre if dot.pyre.laeuft\n", 2, "gibt es nicht"),
        ("kindle\npyre if cooldown.feuerball.ready\n", 2, "feuerball"),
        ("kindle\npyre if heat > 3 and\n", 2, "fehlt"),
        ("kindle\npyre if 15\n", 2, "keine Bedingung"),
    ];

    for (text, zeile, teil) in faelle {
        let error = match parse(AplName("kaputt"), text) {
            Ok(_) => panic!("haette scheitern muessen: {text:?}"),
            Err(error) => error,
        };
        assert_eq!(error.line, zeile, "falsche Zeile bei {text:?}");
        assert!(
            error.message.contains(teil),
            "die Meldung zu {text:?} nennt \"{teil}\" nicht: {}",
            error.message
        );
    }
}

#[test]
fn ac3_eine_leere_liste_ist_ein_fehler() {
    // Eine Liste aus lauter Kommentaren wuerde sonst zu einer Rotation, die nie
    // etwas wirkt — und der Kampf saehe nach einem Balance-Problem aus.
    let error = parse(AplName("leer"), "# nur ein Kommentar\n\n").expect_err("muss scheitern");
    assert!(error.message.contains("keine einzige Regel"), "{error}");
}

#[test]
fn ac4_die_auswertung_fordert_keinen_speicher_an() {
    let apl = load("step1-good", GOOD);
    let mut fight = Fight::new(FightConfig::default());

    // Das Einlesen darf anfordern — es geschieht einmal. Der Durchgang nicht.
    let (result, allocations) = count(|| fight.run_apl(&apl));

    assert_eq!(
        allocations, 0,
        "die Auswertung hat {allocations} Mal Speicher angefordert"
    );
    assert_eq!(result.damage, 4240, "ohne Rechnung waere der Test wertlos");
}

#[test]
fn ac4_gilt_auch_ueber_tausend_durchgaenge() {
    let apl = load("step1-good", GOOD);
    let mut fight = Fight::new(FightConfig::default());
    fight.run_apl(&apl);

    let (_, allocations) = count(|| {
        for seed in 0..1000u32 {
            fight.reset(seed);
            fight.run_apl(&apl);
        }
    });
    assert_eq!(allocations, 0);
}

#[test]
fn die_dateien_im_repository_sind_lesbar() {
    // Sie sind der eigentliche Gegenstand dieses Issues: „Die beiden Listen aus
    // STEP1-SPEC § 4 als Textdatei ausdrueckbar."
    let good = load("step1-good", GOOD);
    let bad = load("step1-bad", BAD);

    assert_eq!(good.rules.len(), 3);
    assert_eq!(bad.rules.len(), 3);
    assert_eq!(good.rules[0].spell, SMOULDER);
    assert_eq!(good.rules[1].spell, PYRE);
    assert_eq!(good.rules[2].spell, KINDLE);
    assert!(
        good.rules[2].condition.is_always(),
        "der Fueller am Ende hat keine Bedingung"
    );
}

#[test]
fn zeitangaben_in_sekunden_und_millisekunden_sind_dasselbe() {
    // `6s` in der schlechten Rotation muss exakt 6000 ms bedeuten — ganzzahlig,
    // ohne Umweg ueber Fliesskomma.
    let a = load("a", "smoulder if dot.smoulder.remains < 6s\nkindle\n");
    let b = load("b", "smoulder if dot.smoulder.remains < 6000\nkindle\n");
    let c = load("c", "smoulder if dot.smoulder.remains < 6000ms\nkindle\n");
    assert_eq!(a.rules[0].condition, b.rules[0].condition);
    assert_eq!(b.rules[0].condition, c.rules[0].condition);

    let d = load("d", "smoulder if dot.smoulder.remains < 1.5s\nkindle\n");
    let e = load("e", "smoulder if dot.smoulder.remains < gcd\nkindle\n");
    assert_eq!(
        run_apl(&d, true).damage,
        run_apl(&e, true).damage,
        "1.5s und gcd sind derselbe Wert"
    );
}

#[test]
fn zeitabhaengige_bedingungen_wecken_den_simulator() {
    // **Der Vorabfund aus Issue #53, hier behoben.**
    //
    // Der Kampf springt von Ereignis zu Ereignis. Wird nichts gewirkt, liegen
    // die Ereignisse weit auseinander — der Verfall alle 2 s, sonst nichts. Eine
    // Bedingung, die bei 3000 ms wahr wird, faellt damit zwischen zwei
    // Ereignisse (2000 und 4000).
    //
    // `Apl::next_wakeup` rechnet den Umschlagpunkt aus, und der Kampf plant
    // einen Weckruf ein. Ohne ihn wuerde erst bei 4000 ms gewirkt — eine ganze
    // Sekunde zu spaet.
    let apl = load(
        "zeitgesteuert",
        "smoulder if time >= 3000 and dot.smoulder.active == 0\n",
    );
    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    fight.run_apl(&apl);

    let erster_cast = fight
        .log()
        .entries()
        .iter()
        .find(|e| e.kind == rota_sim::log::LogKind::Cast)
        .map(|e| e.time);

    assert_eq!(
        erster_cast,
        Some(3000),
        "die Bedingung wurde nicht zu ihrem Zeitpunkt bemerkt"
    );
}

#[test]
fn ohne_zeitabhaengige_bedingung_gibt_es_keine_zusaetzlichen_weckrufe() {
    // Die Gegenprobe: Weckrufe kosten Platz in der Ereignis-Schlange. Eine
    // Liste, die nur den Zustand abfragt, darf keine ausloesen — sonst waere die
    // Behebung des einen Falls zulasten aller anderen gegangen.
    let apl = load("zustandsbasiert", "pyre if heat >= 15\nkindle\n");
    let mut fight = Fight::new(FightConfig::default());
    let (result, allocations) = count(|| fight.run_apl(&apl));

    assert_eq!(allocations, 0, "die Schlange musste nachfordern");
    assert!(result.damage > 0);
}

#[test]
fn weckrufe_veraendern_die_rotation_nicht() {
    // Die gute Rotation enthaelt mit `dot.smoulder.remains < gcd` eine
    // zeitabhaengige Bedingung. Sie muss trotzdem exakt dasselbe Ergebnis
    // liefern wie die Rust-Fassung ohne Weckrufe — die zusaetzlichen Pruefungen
    // duerfen die Rotation nicht veraendern, nur rechtzeitig machen.
    let apl = load("step1-good", GOOD);
    let aus_datei = run_apl(&apl, true);
    let aus_code = Fight::new(FightConfig::default()).run(rotation::good);
    assert_eq!(aus_datei, aus_code);
}
