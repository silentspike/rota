//! Zaehlt die Instrumentierung im ECHTEN Kampf? (Issue #896)
//!
//! # Warum diese Datei neben den Prueffaellen im Modul steht
//!
//! `src/diagnostics.rs` prueft die **Auswertung**: Perzentile, Ueberlauf,
//! Trefferquote. Alle diese Faelle bauen ihre Zaehler von Hand — sie wuerden
//! genauso gruen laufen, wenn `src/fight.rs` die Zaehler nie fuettert.
//!
//! Genau dieser Fehler ist in diesem Projekt schon vorgekommen: eine
//! Instrumentierung, die vollstaendig geprueft war und im laufenden System
//! nichts gesehen hat, weil niemand sie rief. Ein Prueffall, der den
//! **Aufrufweg** nicht beruehrt, kann das nicht bemerken.
//!
//! Diese Datei laesst deshalb einen richtigen Kampf laufen und sieht nach, ob
//! danach etwas in den Zaehlern steht.
//!
//! # Warum sie in der CI nicht laeuft
//!
//! `.github/workflows/ci.yml` faehrt `cargo test` ohne Merkmale; die
//! Aufrufstellen in `fight.rs` haengen aber an `--features diagnostics`. Ohne
//! den Schalter gibt es nichts zu pruefen, und die Datei uebersetzt zu einer
//! leeren Huelle.
//!
//! ```text
//! cargo remote -c -- \
//!   test --features diagnostics --test diagnose_im_kampf
//! ```
#![cfg(feature = "diagnostics")]

use rota_sim::{diagnostics, rotation, Fight, FightConfig};

/// Ein Kampf mit der guten Rotation — dieselbe wie im Beispiel in `lib.rs`.
fn ein_kampf() {
    let mut fight = Fight::new(FightConfig::default());
    let ergebnis = fight.run(rotation::good);
    // Traeger-Zusicherung fuer den Kampf selbst: Haette er nichts getan,
    // saehe die Diagnose auch nichts — und der Test unten waere aus dem
    // falschen Grund rot.
    assert!(ergebnis.damage > 0, "der Kampf hat gar nicht stattgefunden");
}

#[test]
fn eingeschaltet_zaehlt_der_kampf_die_schlange() {
    diagnostics::reset();
    diagnostics::enable();
    ein_kampf();
    let z = diagnostics::snapshot();
    diagnostics::disable();
    diagnostics::reset();

    assert!(
        z.samples() > 0,
        "kein einziger Blick auf die Schlange — die Aufrufstelle in fight.rs fehlt"
    );
    assert!(
        z.max() > 0,
        "Schlange nie laenger als 0, obwohl der Kampf lief"
    );
    assert_eq!(
        z.overflow(),
        0,
        "die Schlange lief ueber EVENT_CAPACITY hinaus — das waere eine \
         Speicheranforderung im Schritt (GAME-DESIGN 18.4), kein Messproblem"
    );

    // Die Perzentile muessen aufsteigen, sonst rechnet die Auswertung an den
    // echten Daten anders als an den von Hand gebauten.
    let p50 = z.percentile(0.50).expect("Stichproben liegen vor");
    let p95 = z.percentile(0.95).expect("Stichproben liegen vor");
    let p99 = z.percentile(0.99).expect("Stichproben liegen vor");
    assert!(p50 <= p95 && p95 <= p99, "p50={p50} p95={p95} p99={p99}");
    assert!(
        p99 <= z.max(),
        "p99={p99} ueber dem Groesstwert {}",
        z.max()
    );
}

#[test]
fn eingeschaltet_zaehlt_der_kampf_die_ablaeufe() {
    diagnostics::reset();
    diagnostics::enable();
    ein_kampf();
    let z = diagnostics::snapshot();
    diagnostics::disable();
    diagnostics::reset();

    assert!(
        z.expire_calls() > 0,
        "expire_buffs wurde nie gemeldet — die Aufrufstelle fehlt"
    );
    assert!(
        z.expire_hits() <= z.expire_calls(),
        "mehr Treffer als Aufrufe: {} von {}",
        z.expire_hits(),
        z.expire_calls()
    );
    let quote = z.expire_hit_rate().expect("Aufrufe liegen vor");
    assert!(
        (0.0..=1.0).contains(&quote),
        "Quote ausserhalb 0..1: {quote}"
    );
}

/// Der Kampfzaehler traegt die Nenner aller Durchsatzzahlen.
///
/// Geprueft wird eine **feste Zahl**, nicht „groesser als null": Ein Zaehler,
/// der bei drei Kaempfen einmal hochzaehlt, waere mit `> 0` ebenso gruen — und
/// jede Rate, die ihn als Nenner benutzt, waere um Faktor drei daneben.
#[test]
fn drei_kaempfe_sind_drei_kaempfe() {
    diagnostics::reset();
    diagnostics::enable();
    ein_kampf();
    ein_kampf();
    ein_kampf();
    let z = diagnostics::snapshot();
    diagnostics::disable();
    diagnostics::reset();

    assert_eq!(
        z.fights(),
        3,
        "drei gerechnete Kaempfe, gezaehlt: {}",
        z.fights()
    );
}

/// Der Trace-Hash muss auf den VERLAUF reagieren, nicht nur auf das Ergebnis.
///
/// Zwei verschiedene Rotationen am selben Boss ergeben verschiedene Hashes.
/// Ein Hash, der das nicht unterscheidet, wuerde spaeter jede Umbaute
/// durchwinken, die den Endschaden zufaellig trifft — und genau davor soll er
/// schuetzen.
#[test]
fn verschiedene_rotationen_ergeben_verschiedene_hashes() {
    let hash_von = |rot: rota_sim::fight::Rotation| {
        diagnostics::reset();
        diagnostics::enable_trace();
        let mut fight = Fight::new(FightConfig::default());
        let ergebnis = fight.run(rot);
        let z = diagnostics::snapshot();
        diagnostics::disable_trace();
        diagnostics::reset();
        assert!(z.trace_states() > 0, "der Hash hat keinen Zustand gesehen");
        (z.trace(), z.trace_states(), ergebnis.damage)
    };

    let gut = hash_von(rotation::good);
    let schlecht = hash_von(rotation::bad);
    assert_ne!(
        gut.0, schlecht.0,
        "zwei Rotationen, derselbe Trace-Hash — er unterscheidet den Verlauf nicht"
    );

    // Zweimal dasselbe muss dasselbe ergeben, sonst ist er als Vergleichswert
    // unbrauchbar.
    assert_eq!(gut, hash_von(rotation::good), "derselbe Lauf, anderer Hash");
}

/// Die beiden Schalter sind getrennt — und das muss so bleiben.
///
/// Der Trace-Hash kostet ein volles `checksum_at` je Takt, die Zaehler ein
/// Inkrement. Haengen sie an einem Schalter, zahlt jede Kalibrierung den
/// teuren Preis: Gemessen am 2026-08-18 lief ein Kalibrierlauf ueber zwoelf
/// Minuten und war nicht fertig. Dieser Fall haelt die Trennung fest.
#[test]
fn zaehler_und_trace_haengen_an_verschiedenen_schaltern() {
    diagnostics::reset();
    diagnostics::enable();
    ein_kampf();
    let nur_zaehler = diagnostics::snapshot();
    diagnostics::disable();
    diagnostics::reset();

    assert!(nur_zaehler.fights() > 0, "die Zaehler liefen nicht");
    assert_eq!(
        nur_zaehler.trace_states(),
        0,
        "der Trace lief mit, obwohl nur die Zaehler eingeschaltet waren"
    );

    diagnostics::reset();
    diagnostics::enable_trace();
    ein_kampf();
    let nur_trace = diagnostics::snapshot();
    diagnostics::disable_trace();
    diagnostics::reset();

    assert!(nur_trace.trace_states() > 0, "der Trace lief nicht");
    assert_eq!(
        nur_trace.fights(),
        0,
        "die Zaehler liefen mit, obwohl nur der Trace eingeschaltet war"
    );
}

/// Ein Trace-Hash ohne Zustaende ist kein Hash.
///
/// `FNV_OFFSET` als Anfangswert heisst: „nichts gesehen" ist erkennbar und
/// nicht 0 — 0 waere von einem echten Ergebnis nicht zu unterscheiden.
#[test]
fn ohne_lauf_steht_der_hash_auf_dem_anfangswert() {
    diagnostics::reset();
    let z = diagnostics::snapshot();
    assert_eq!(z.trace_states(), 0);
    assert_eq!(z.trace(), rota_sim::checksum::FNV_OFFSET);
}

/// Die Gegenprobe: Der Schalter muss auch WIRKEN.
///
/// Ohne diesen Fall waere „eingeschaltet zaehlt es" auch dann gruen, wenn die
/// Erfassung immer liefe — und dann kostete sie den Suchpfad etwas, ohne dass
/// es jemand merkt.
#[test]
fn ausgeschaltet_zaehlt_der_kampf_nichts() {
    diagnostics::reset();
    diagnostics::disable();
    ein_kampf();
    let z = diagnostics::snapshot();
    diagnostics::reset();

    assert_eq!(z.samples(), 0, "gezaehlt, obwohl der Schalter aus war");
    assert_eq!(z.expire_calls(), 0, "gezaehlt, obwohl der Schalter aus war");
    assert_eq!(z.fights(), 0, "gezaehlt, obwohl der Schalter aus war");
}

/// Zweimal derselbe Kampf ergibt denselben Zaehlerstand.
///
/// Der Simulator ist deterministisch (GAME-DESIGN 18.4). Waere er es in der
/// Diagnose nicht, taugte keine der Zahlen zum Vergleich zweier Staende —
/// und genau dafuer sind sie da.
#[test]
fn zweimal_derselbe_kampf_zaehlt_gleich() {
    let mut staende = Vec::new();
    for _ in 0..2 {
        diagnostics::reset();
        diagnostics::enable();
        ein_kampf();
        let z = diagnostics::snapshot();
        diagnostics::disable();
        staende.push((
            z.samples(),
            z.max(),
            z.expire_calls(),
            z.expire_hits(),
            z.percentile(0.95),
        ));
    }
    diagnostics::reset();
    assert_eq!(staende[0], staende[1], "derselbe Kampf, andere Zaehler");
    assert!(staende[0].0 > 0, "beide Laeufe haben nichts gezaehlt");
}
