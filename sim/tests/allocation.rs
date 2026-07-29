//! AC4 aus Issue #53 — keine Speicheranforderung im Schritt.
//!
//! Aus GAME-DESIGN 18.4: *"Keine Speicheranforderung im Schritt,
//! zusammenhaengende Listen."* Im Spiel geht es dabei um Ruckeln durch die
//! Speicherbereinigung (18.2); im Simulator geht es um Durchsatz — bei
//! hunderttausend Durchgaengen kostet jede Anforderung je Schritt mehr als die
//! eigentliche Rechnung.
//!
//! Gemessen wird mit einem **Zaehl-Allocator**: Er reicht jede Anforderung an
//! den Standard weiter und zaehlt sie mit. Der Aufbau des Kampfes darf und muss
//! anfordern (Ereignis-Schlange, Protokoll); ab dem ersten Schritt darf nichts
//! mehr dazukommen.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rota_sim::rotation;
use rota_sim::{Fight, FightConfig};

// Gezaehlt wird **je Strang**, nicht prozessweit.
//
// `cargo test` laesst die Tests einer Datei gleichzeitig laufen. Mit einem
// gemeinsamen Zaehler misst jeder Test die Anforderungen der anderen mit — der
// erste Anlauf dieses Tests meldete auf diese Weise 13.873 Anforderungen fuer
// 1.000 Durchgaenge, von denen keine einzige aus dem Simulator stammte. Ein
// thread-lokaler Zaehler misst genau den Strang, in dem auch gerechnet wird.
//
// `const { Cell::new(0) }` ist wichtig: Diese Form braucht keine verzoegerte
// Einrichtung und fordert deshalb selbst keinen Speicher an — sonst zaehlte der
// Zaehler seine eigene Entstehung mit.
thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

fn note(bytes: usize) {
    // `try_with` statt `with`: Waehrend ein Strang abgeraeumt wird, ist der
    // thread-lokale Speicher bereits fort. Ein `with` wuerde dort in Panik
    // geraten — ausgerechnet im Allocator.
    let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
    let _ = BYTES.try_with(|c| c.set(c.get() + bytes));
}

struct Counting;

// SAFETY: Jede Anfrage wird unveraendert an den System-Allocator weitergereicht;
// gezaehlt wird nur nebenher. Damit gelten dieselben Zusagen wie fuer `System`.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Eine Vergroesserung ist die Anforderung, auf die es hier ankommt: Sie
        // ist das Anzeichen dafuer, dass eine Liste ihre vorab angeforderte
        // Kapazitaet gesprengt hat.
        note(new_size.saturating_sub(layout.size()));
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Zaehler zuruecksetzen und die Anforderungen waehrend `f` zaehlen.
fn count<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    ALLOCATIONS.with(|c| c.set(0));
    BYTES.with(|c| c.set(0));
    let value = f();
    (value, ALLOCATIONS.with(Cell::get), BYTES.with(Cell::get))
}

#[test]
fn ac4_ein_durchgang_fordert_keinen_speicher_an() {
    // Der Aufbau darf anfordern — hier wird der Speicher fuer die gesamte
    // Laufzeit besorgt.
    let mut fight = Fight::new(FightConfig::default());

    let (result, allocations, bytes) = count(|| fight.run(rotation::good));

    assert_eq!(
        allocations, 0,
        "der Durchgang hat {allocations} Mal Speicher angefordert ({bytes} Byte); \
         erlaubt sind 0 — die Kapazitaeten in Fight::new reichen dann nicht"
    );
    // Ohne diese Zeile koennte der Test auch dann bestehen, wenn gar nichts
    // gerechnet wurde.
    assert_eq!(result.damage, 4240);
    assert_eq!(result.killed_at_ms, Some(40_510));
}

#[test]
fn ac4_gilt_auch_fuer_wiederholte_durchgaenge() {
    // Der eigentliche Anwendungsfall: derselbe Kampf, hunderttausendfach. Wenn
    // `reset` den Speicher freigaebe, faellt es hier auf.
    let mut fight = Fight::new(FightConfig::default());
    fight.run(rotation::good);

    let (_, allocations, bytes) = count(|| {
        for seed in 0..1000u32 {
            fight.reset(seed);
            fight.run(rotation::good);
        }
    });

    assert_eq!(
        allocations, 0,
        "1000 Durchgaenge haben {allocations} Anforderungen ausgeloest ({bytes} Byte)"
    );
}

#[test]
fn ac4_gilt_fuer_alle_vier_rotationen() {
    // Die schlechte Rotation wirkt doppelt so oft und laeuft ueber die vollen
    // 60 s — sie fuellt die Ereignis-Schlange staerker als die gute.
    for apl in [
        rotation::good as rota_sim::fight::Rotation,
        rotation::bad,
        rotation::dot_error_only,
        rotation::pyre_error_only,
    ] {
        let mut fight = Fight::new(FightConfig::default());
        let (_, allocations, _) = count(|| fight.run(apl));
        assert_eq!(allocations, 0);
    }
}

#[test]
fn ohne_protokoll_kostet_der_aufbau_fast_nichts() {
    // Fuer grosse Laeufe wird das Protokoll abgeschaltet. Dann kostet der
    // Aufbau nur noch die Ereignis-Schlange und die Anmeldung der
    // Schul-Mechanik.
    //
    // **Bis Issue E3-2 war es genau eine Anforderung.** Seitdem haengt der
    // Hitze-Gewinn an einem Empfaenger statt fest im Kampfcode zu stehen, und
    // ein Empfaenger anzumelden kostet:
    //
    // | Anzahl | wofuer                                                  |
    // | ------ | ------------------------------------------------------- |
    // |      1 | die Ereignis-Schlange                                    |
    // |      2 | die geteilten Zellen (angesetzter Gewinn, Zaehler)       |
    // |      2 | je ein `Box<dyn RuleReceiver>` fuer die beiden Ereignisse |
    // |      2 | die beiden Kennungen als `String`                        |
    // |      4 | die Listen der Verteilung, beim ersten Eintrag           |
    //
    // **Die Zahl steht hier exakt und nicht als Obergrenze**, damit sie
    // auffaellt, wenn sie sich bewegt. Was zaehlt, ist ohnehin die andere
    // Zusage aus GAME-DESIGN 18.4: **im Schritt** wird nichts angefordert —
    // das pruefen die drei Tests darueber, und sie stehen weiter bei 0.
    let (_, allocations, _) = count(|| {
        Fight::new(FightConfig {
            log_capacity: 0,
            ..FightConfig::default()
        })
    });
    assert_eq!(
        allocations, 11,
        "erwartet werden elf Anforderungen: Ereignis-Schlange plus Anmeldung \
         der Schul-Mechanik (Aufschluesselung im Kommentar darueber)"
    );
}
