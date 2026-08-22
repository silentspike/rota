//! Die Sprache kann die Bausteine 4, 5 und 8 abfragen (Issue #1074).
//!
//! # Was hier fehlte, und warum es keine Fehlfunktion war
//!
//! `doom`, `affliction` und `runaway` sind drei der neun Bausteine aus
//! GAME-DESIGN 11.2. Ihre Zustände liefen im Kampf mit — `doom: u32`,
//! `afflicted: bool`, `race_progress: u32` — und standen in der Prüfsumme.
//! Nur konnte keine Bedingung sie lesen.
//!
//! Das äußerte sich als **Stille**, nicht als Fehler: Der Balance-Läufer maß an
//! einem `doom`-Boss, wie viel Schaden eine Rotation nebenbei macht, nicht ob
//! sie die Uhr bedient. Ein Boss, dessen Signatur-Regel auf `doom` beruht, war
//! für die Suche derselbe Boss ohne `doom`.
//!
//! # Warum die Namen nicht die der Bausteine sind
//!
//! Die Zustandsfelder heißen anders als die Bausteine: `afflicted` statt
//! `affliction`, `race_progress` statt `runaway`. Wer nach dem Baustein-Namen
//! sucht, findet den Zustand nicht — beim Erkunden dieses Issues zweimal
//! passiert. Die APL-Namen folgen den **Feldern**, weil eine Bedingung den
//! Zustand liest und nicht die Mechanik.

use rota_sim::apl::{parse, AplName, Value};
use rota_sim::data::boss_by_id;
use rota_sim::fight::{Fight, FightConfig};

/// Jeder der vier neuen Ausdrücke lässt sich in einer echten Regel benutzen.
///
/// Nicht nur „der Parser kennt das Wort": Die Zeile steht in einer
/// vollständigen Liste mit Zauber und Vergleich, so wie eine Rotation sie
/// schreiben würde.
#[test]
fn ac1_jeder_baustein_ist_in_einer_regel_abfragbar() {
    for (ausdruck, erwartet) in [
        ("doom", Value::Doom),
        ("doom_charges", Value::DoomCharges),
        ("afflicted", Value::Afflicted),
        ("race_progress", Value::RaceProgress),
    ] {
        let text = format!("kindle if {ausdruck} >= 1\npurge\n");
        let apl = parse(AplName("probe"), &text)
            .unwrap_or_else(|e| panic!("`{ausdruck}` ließ sich nicht lesen:\n{e}"));
        let gefunden = apl
            .rules
            .iter()
            .flat_map(|r| r.condition.groups.iter())
            .flatten()
            .any(|t| t.left == erwartet || t.right == erwartet);
        assert!(
            gefunden,
            "`{ausdruck}` wurde gelesen, ergab aber nicht {erwartet:?} — dann \
             zeigt der Parser auf den falschen Wert, und die Bedingung fragt \
             etwas anderes ab als sie sagt."
        );
    }
}

/// **Die Gegenrichtung**: Ein Wort, das es nicht gibt, muss abgelehnt werden.
///
/// Ohne diesen Fall wäre ein Parser grün, der jeden Bezeichner durchwinkt —
/// und dann hätte `ac1` auch für einen Tippfehler bestanden.
#[test]
fn ac1_ein_erfundener_wert_wird_abgelehnt() {
    let fehler = parse(AplName("probe"), "kindle if doooom >= 1\npurge\n");
    assert!(
        fehler.is_err(),
        "Ein Wert, den es nicht gibt, wurde gelesen. Dann belegt das Lesen von \
         `doom` nicht, dass der Wert angeschlossen ist."
    );
}

/// Die Werte lassen sich **im Kampf** lesen und liefern den Zustand.
///
/// `boss-1-2` trägt `doom` mit zwei Senkungen; zu Kampfbeginn steht die Uhr auf
/// null und die Senkungen sind unverbraucht.
#[test]
fn ac1_die_werte_lesen_den_zustand_und_nicht_null() {
    let fight = Fight::new(FightConfig {
        boss: boss_by_id("boss-1-2").expect("boss-1-2 fehlt"),
        ..FightConfig::default()
    });
    assert_eq!(Value::Doom.read(&fight), 0, "Die Uhr beginnt bei null");
    assert!(
        Value::DoomCharges.read(&fight) > 0,
        "boss-1-2 hat Senkungen — steht hier null, liest der Wert nicht den \
         Zustand, sondern einen Vorgabewert. Genau das sähe wie ein \
         funktionierender Anschluss aus."
    );
    assert_eq!(
        Value::Afflicted.read(&fight),
        0,
        "Zu Beginn liegt keine Schwächung an"
    );
    assert_eq!(
        Value::RaceProgress.read(&fight),
        0,
        "Der Ring beginnt bei null"
    );
}

/// **AC2 — der Wert wirkt im Kampf, nicht nur in der Sprache.**
///
/// Zwei Rotationen an `boss-1-2`, beide werfen `purge`, beide haben denselben
/// Füller: Die eine wartet, bis die Uhr hoch steht, die andere drückt sofort.
///
/// Sind die Ergebnisse **gleich**, ist der Wert angeschlossen und wirkungslos —
/// das wäre die Klasse, die dieses Projekt am teuersten zu stehen kommt, weil
/// sie bei jeder Prüfung wie Erfolg aussieht.
///
/// # Warum derselbe Füller in beiden Listen steht
///
/// Ohne ihn unterschieden sich die Läufe auch in der GCD-Last, und der
/// Unterschied im Ergebnis wäre nicht dem gelesenen Wert zuzuschreiben,
/// sondern der Zahl der Würfe.
#[test]
fn ac2_die_bedingung_aendert_den_kampfverlauf() {
    let lauf = |text: &str| {
        let apl = parse(AplName("probe"), text).expect("Liste ist fehlerhaft");
        Fight::new(FightConfig {
            boss: boss_by_id("boss-1-2").expect("boss-1-2 fehlt"),
            stop_on_kill: false,
            ..FightConfig::default()
        })
        .run_apl(&apl)
    };

    // Die Schwelle 60 ist GEMESSEN, nicht geschätzt: `boss-1-2` trägt
    // `threshold: 100` — die Uhr läuft dort voll. Ein erster Versuch mit 200
    // war unerreichbar, die Liste warf nie, und ohne die Zusicherung unten
    // hätte der Fall „verschieden" gemeldet und nichts belegt.
    let gezielt = lauf("purge if doom >= 60\nkindle\n");
    let sofort = lauf("purge\nkindle\n");

    // ZUSICHERUNG ZUERST: Beide Läufe müssen `purge` überhaupt geworfen haben.
    // Wirft einer keinen, vergleicht der Fall „mit Zauber" gegen „ohne" — der
    // Unterschied wäre dann trivial und sagte nichts über den gelesenen Wert.
    let purge = rota_sim::data::SPELLS
        .iter()
        .position(|s| s.id == "purge")
        .expect("purge fehlt");
    assert!(
        gezielt.casts[purge] > 0 && sofort.casts[purge] > 0,
        "Eine der beiden Listen hat purge nie geworfen (gezielt {}, sofort {}) — \
         dann misst dieser Fall nicht die Bedingung, sondern die Anwesenheit \
         des Zaubers.",
        gezielt.casts[purge],
        sofort.casts[purge]
    );

    assert_ne!(
        (gezielt.casts[purge], gezielt.damage),
        (sofort.casts[purge], sofort.damage),
        "Beide Rotationen ergaben denselben Verlauf. Dann liest `doom` zwar \
         einen Wert, aber die Bedingung ändert nichts — der Wert wäre \
         angeschlossen und wirkungslos."
    );
}
