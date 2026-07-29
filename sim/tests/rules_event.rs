//! Das Empfaenger-Ereignis-System des Simulators (Issue E3-1b).
//!
//! Die Gegenstuecke stehen in `tests/unit/rule-events.test.ts`. Beide Seiten
//! pruefen dieselben Aussagen — feste Reihenfolge, Anmeldung veraendert nichts,
//! keine Speicheranforderung —, weil eine Reihenfolge, die nur auf einer Seite
//! geprueft ist, genau dort abweicht, wo niemand hinsieht.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rota_sim::log::LogKind;
use rota_sim::rules_event::{RuleEvent, RuleEvents, RuleReceiver, NO_SUBJECT};
use rota_sim::{rotation, Fight, FightConfig};

// Derselbe thread-lokale Zaehl-Allocator wie in `tests/allocation.rs`. Er steht
// hier noch einmal, weil jede Testdatei ein eigenes Programm ist und einen
// eigenen globalen Allocator braucht.
thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

// SAFETY: Jede Anfrage wird unveraendert an den System-Allocator weitergereicht.
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

/// Ein Empfaenger, der nur mitschreibt, wer wann drankam.
#[derive(Default)]
struct Trace {
    order: Vec<u8>,
    mark: u8,
}

impl RuleReceiver for Trace {
    fn on_rule_event(&mut self, _event: RuleEvent, _subject: i16, _value: f64, _time: u32) {
        let mark = self.mark;
        self.order.push(mark);
    }
}

#[test]
fn sechs_ereignisse_in_der_reihenfolge_des_designs() {
    // Die Kennnummern gehen in die Pruefsumme ein und muessen deshalb mit
    // `src/combat/events.ts` uebereinstimmen.
    assert_eq!(RuleEvent::SpellCast as u8, 0);
    assert_eq!(RuleEvent::Damage as u8, 1);
    assert_eq!(RuleEvent::Crit as u8, 2);
    assert_eq!(RuleEvent::DotTick as u8, 3);
    assert_eq!(RuleEvent::BossPhase as u8, 4);
    assert_eq!(RuleEvent::BuffExpired as u8, 5);
    assert_eq!(RuleEvent::COUNT, 6);
    assert_eq!(
        RuleEvent::NAMES,
        [
            "spell.cast",
            "damage",
            "crit",
            "dot.tick",
            "boss.phase",
            "buff.expired"
        ]
    );
}

#[test]
fn feste_reihenfolge_ueber_tausend_durchgaenge() {
    let mut gesehen = std::collections::BTreeSet::new();
    for run in 0..1000 {
        let mut bus = RuleEvents::with_capacity(4);
        // Bewusst wechselnde Anmelde-Folge: Das Ergebnis darf nicht davon
        // abhaengen, wer zuerst am Schalter stand.
        let ids: [(&str, i32, u8); 3] = if run % 2 == 0 {
            [("a", 1, b'a'), ("b", 2, b'b'), ("c", 3, b'c')]
        } else {
            [("c", 3, b'c'), ("b", 2, b'b'), ("a", 1, b'a')]
        };
        for (id, order, mark) in ids {
            bus.register(
                id,
                RuleEvent::SpellCast,
                order,
                Box::new(Trace {
                    order: Vec::new(),
                    mark,
                }),
            );
        }
        bus.emit(RuleEvent::SpellCast, 0, 0.0, 0);
        gesehen.insert(bus.receiver_ids(RuleEvent::SpellCast).join(">"));
    }
    assert_eq!(
        gesehen.len(),
        1,
        "die Reihenfolge schwankt zwischen Laeufen"
    );
    assert_eq!(gesehen.iter().next().map(String::as_str), Some("a>b>c"));
}

#[test]
fn neue_anmeldung_verschiebt_die_bestehenden_nicht() {
    let mut bus = RuleEvents::with_capacity(4);
    bus.register("alt-1", RuleEvent::SpellCast, 10, Box::<Trace>::default());
    bus.register("alt-2", RuleEvent::SpellCast, 20, Box::<Trace>::default());
    let vorher = bus.receiver_ids(RuleEvent::SpellCast);
    assert_eq!(vorher, ["alt-1", "alt-2"]);

    // Der Neue draengt sich VOR beide — ihre relative Ordnung bleibt.
    bus.register("neu", RuleEvent::SpellCast, 5, Box::<Trace>::default());
    assert_eq!(
        bus.receiver_ids(RuleEvent::SpellCast),
        ["neu", "alt-1", "alt-2"]
    );

    // Und dazwischen.
    bus.register("mitte", RuleEvent::SpellCast, 15, Box::<Trace>::default());
    assert_eq!(
        bus.receiver_ids(RuleEvent::SpellCast),
        ["neu", "alt-1", "mitte", "alt-2"]
    );
    let danach: Vec<String> = bus
        .receiver_ids(RuleEvent::SpellCast)
        .into_iter()
        .filter(|id| id.starts_with("alt"))
        .collect();
    assert_eq!(danach, vorher);
}

/// Ein Empfaenger, der nur zaehlt.
///
/// **Er darf selbst nichts anfordern.** Der erste Versuch benutzte hier
/// `Trace`, und der schiebt in einen `Vec` — der Zaehl-Allocator meldete
/// prompt Anforderungen, die gar nicht aus der Verteilung kamen. Genau dafuer
/// ist er da.
#[derive(Default)]
struct Zaehler(u32);

impl RuleReceiver for Zaehler {
    fn on_rule_event(&mut self, _event: RuleEvent, _subject: i16, _value: f64, _time: u32) {
        self.0 += 1;
    }
}

#[test]
fn austragen_fordert_keinen_speicher_an() {
    let mut bus = RuleEvents::with_capacity(4);
    bus.register("a", RuleEvent::Damage, 0, Box::<Zaehler>::default());
    bus.register("b", RuleEvent::Damage, 1, Box::<Zaehler>::default());

    let (_, allocations) = count(|| {
        for i in 0..100_000 {
            bus.emit(RuleEvent::Damage, 0, 100.0, i);
        }
    });
    assert_eq!(
        allocations, 0,
        "100.000 Ereignisse haben {allocations} Anforderungen ausgeloest"
    );
}

#[test]
fn pruefsumme_unterscheidet_wert_und_reihenfolge() {
    let mut a = RuleEvents::with_capacity(1);
    let mut b = RuleEvents::with_capacity(1);
    a.emit(RuleEvent::Damage, 0, 100.0, 0);
    b.emit(RuleEvent::Damage, 0, 101.0, 0);
    assert_ne!(a.sequence_checksum(), b.sequence_checksum());

    let mut c = RuleEvents::with_capacity(1);
    let mut d = RuleEvents::with_capacity(1);
    c.emit(RuleEvent::Damage, 0, 100.0, 0);
    c.emit(RuleEvent::SpellCast, 0, 100.0, 0);
    d.emit(RuleEvent::SpellCast, 0, 100.0, 0);
    d.emit(RuleEvent::Damage, 0, 100.0, 0);
    assert_ne!(c.sequence_checksum(), d.sequence_checksum());
}

#[test]
fn pruefsumme_haengt_nicht_davon_ab_wer_zuhoert() {
    // Sonst waeren zwei Laeufe mit verschiedenen Empfaengern nicht
    // vergleichbar — und genau das ist der Fall zwischen Spiel und Simulator.
    let mut ohne = RuleEvents::with_capacity(1);
    let mut mit = RuleEvents::with_capacity(1);
    mit.register("a", RuleEvent::Damage, 0, Box::<Trace>::default());
    for i in 0..50u32 {
        ohne.emit(RuleEvent::Damage, (i % 3) as i16, f64::from(i * 7), i * 10);
        mit.emit(RuleEvent::Damage, (i % 3) as i16, f64::from(i * 7), i * 10);
    }
    assert_eq!(mit.sequence_checksum(), ohne.sequence_checksum());
}

#[test]
fn leerer_bus_traegt_aus_ohne_zu_scheitern() {
    let mut bus = RuleEvents::with_capacity(1);
    for i in 0..1000u32 {
        bus.emit(RuleEvent::BossPhase, NO_SUBJECT, 2.0, i);
    }
    assert_eq!(bus.emitted_count(), 1000);
}

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
///
/// Sie entsteht mit `npx vite-node tests/parity/export-parity.mjs` und wird von
/// `tests/parity/freshness.test.ts` aktuell gehalten.
const RULE_EVENTS: &str = include_str!("../../tests/parity/recordings/rule-events.json");

/// **Rechnen beide Seiten dieselbe Pruefsumme?**
///
/// Der Nachweis steht bewusst VOR dem Anschluss an den Kampf: Weichen die
/// Rechnungen schon hier ab, meldet der spaetere Vergleich bei jedem Zauber
/// eine Abweichung und bei keinem eine echte. Dieselbe Ueberlegung wie bei
/// `parity_primitives.rs` fuer die Zufallsstroeme.
#[test]
fn dieselbe_ereignisfolge_ergibt_dieselbe_pruefsumme() {
    let data: serde_json::Value =
        serde_json::from_str(RULE_EVENTS).expect("rule-events.json ist kein JSON");
    assert_eq!(
        data["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von rule-events.json"
    );

    // Auch die Namen muessen stimmen: Sie sind die Schnittstelle zu den Daten.
    let namen: Vec<String> = data["eventNames"]
        .as_array()
        .expect("eventNames fehlt")
        .iter()
        .map(|v| v.as_str().expect("Name erwartet").to_owned())
        .collect();
    assert_eq!(namen, RuleEvent::NAMES);

    let steps = data["steps"].as_array().expect("steps fehlt");
    let erwartet: Vec<u32> = data["checksumAfterEach"]
        .as_array()
        .expect("checksumAfterEach fehlt")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl erwartet")).expect("passt nicht in 32 Bit"))
        .collect();
    assert_eq!(steps.len(), erwartet.len());
    assert!(!steps.is_empty(), "eine leere Folge bestuende immer");

    let mut bus = RuleEvents::with_capacity(0);
    for (index, step) in steps.iter().enumerate() {
        let event = RuleEvent::from_index(
            usize::try_from(step["event"].as_u64().expect("event erwartet")).expect("passt nicht"),
        )
        .expect("unbekanntes Ereignis");
        let subject = i16::try_from(step["subject"].as_i64().expect("subject erwartet"))
            .expect("passt nicht in 16 Bit");
        let value = step["value"].as_f64().expect("value erwartet");
        let time = u32::try_from(step["timeMs"].as_u64().expect("timeMs erwartet"))
            .expect("passt nicht in 32 Bit");

        bus.emit(event, subject, value, time);
        // Schritt fuer Schritt: sagt WO es auseinanderlaeuft, nicht nur DASS.
        assert_eq!(
            bus.sequence_checksum(),
            erwartet[index],
            "Abweichung bei Schritt {index} ({}, Betreff {subject}, Wert {value}, {time} ms)",
            event.name()
        );
    }

    let ende = u32::try_from(
        data["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum fehlt"),
    )
    .expect("passt nicht in 32 Bit");
    assert_eq!(bus.sequence_checksum(), ende);
    assert_eq!(
        bus.emitted_count(),
        data["emitted"].as_u64().expect("emitted fehlt")
    );
}

/// Der Befund, der zu dieser Datei gehoert: In welcher Reihenfolge schreibt
/// **der Simulator** Wirkung und Schaden ins Protokoll?
///
/// Nicht gelesen, sondern ausgefuehrt — die Antwort entscheidet, in welcher
/// Reihenfolge die Regel-Ereignisse ausgetragen werden muessen.
#[test]
fn protokoll_reihenfolge_von_wirkung_und_schaden() {
    let mut fight = Fight::new(FightConfig::default());
    fight.run(rotation::good);
    let entries = fight.log().entries();

    let erste_wirkung = entries.iter().position(|e| e.kind == LogKind::Cast);
    let erster_schaden = entries.iter().position(|e| e.kind == LogKind::Damage);
    assert!(erste_wirkung.is_some() && erster_schaden.is_some());
    assert!(
        erste_wirkung < erster_schaden,
        "Simulator schreibt Wirkung vor Schaden — im Spiel ist es umgekehrt \
         (tests/unit/log.test.ts). Steht das nicht mehr so, ist dieser Test der Ort, \
         an dem es auffaellt."
    );
}
