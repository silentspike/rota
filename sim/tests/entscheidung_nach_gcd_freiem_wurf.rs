//! Entscheidung und Leerlauf nach einem Wurf ohne Wartezeit (Issue #1027).
//!
//! Die Sollwerte sind hier **von Hand hergeleitet**, aus der Regel des Spiels
//! (`driveRotation` in `src/devtools/reference-run.ts`) und dem
//! Entscheidungsmodell, das der Orchestrator am 2026-10-28 an #1027 festgelegt
//! hat:
//!
//! | | |
//! | --- | --- |
//! | (a) | nach einem Wurf entscheidet die Liste beim **selben** Zeitstempel erneut, solange sie etwas wählt |
//! | (b) | ein Zauber ohne Wartezeit kommt auch **während** einer laufenden zum Zug |
//! | (c) | ein Nullzeit-Budget begrenzt die Würfe je Zeitstempel |
//!
//! Je Takt, in dem die Wartezeit bereit war und nichts gewirkt wurde, 10 ms
//! Leerlauf. Dieselben Lagen fährt `parity_gcd_freier_wurf.rs` gegen die
//! Aufzeichnung des Spiels; dieser Fall hängt an keiner Datei und zeigt, was die
//! Zahl bedeutet.
//!
//! **Das Spiel selbst konnte beides schon immer** (`Session.tick` arbeitet alle
//! Drücke eines Takts ab, QUENCH wird von keiner Wartezeit gesperrt); nur die
//! Schleife und der Simulator konnten es nicht. Bis zu dieser Festlegung
//! entschieden beide nach QUENCH einen Takt später und nie während eines GCD.
//!
//! # Was bis #1027 geschah
//!
//! Nach QUENCH (oder dem Wurf, den SUBLIME befreit) steht keine
//! GCD-Wiedervorlage in der Schlange. Der Simulator entschied deshalb erst am
//! nächsten eingereihten Ereignis neu — an `first-crucible` mit QUENCH und
//! Füller bei 2000 statt bei 10. Den Leerlauf zählte er beim Warten als Spanne
//! bis zum nächsten **bereits eingereihten** Ereignis und reihte den Weckruf
//! der Entscheidung erst danach ein; lag der davor, zählte die Spanne doppelt.

use rota_sim::apl::{parse, AplName};
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::log::LogKind;
use rota_sim::{Fight, FightConfig, RunResult};

/// 30 s an `first-crucible`, ohne Abbruch beim Sieg: kein Boss-Ereignis, nur
/// die Uhr und die eigenen Würfe.
fn lauf(text: &str) -> (Fight, RunResult) {
    let apl = parse(AplName("gcd-frei"), text)
        .unwrap_or_else(|fehler| panic!("Liste fehlerhaft:\n{fehler}\n{text}"));
    let mut fight = Fight::new(FightConfig {
        boss: boss_by_id("first-crucible").expect("first-crucible fehlt"),
        duration_ms: Some(30_000),
        stop_on_kill: false,
        log_capacity: 1 << 14,
        ..FightConfig::default()
    });
    let ergebnis = fight.run_apl(&apl);
    assert!(!fight.log().overflowed(), "Kampfprotokoll übergelaufen");
    (fight, ergebnis)
}

fn wuerfe(fight: &Fight, id: &str) -> Vec<u32> {
    let spell = index_of(id).unwrap_or_else(|| panic!("{id} fehlt"));
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::Cast && usize::from(e.spell) == spell)
        .map(|e| e.time)
        .collect()
}

/// QUENCH bei 0 löst keine Wartezeit aus; (a) der Füller folgt beim selben
/// Zeitstempel.
///
/// Bis zur Festlegung vom 2026-10-28 stand hier 10 — „im nächsten Takt", weil
/// die Schleife je Takt einmal entschied. Ein Spieler kann im selben Takt
/// nachdrücken, und so entscheidet jetzt auch die Liste.
#[test]
fn nach_quench_wirft_die_liste_den_fueller_beim_selben_zeitstempel() {
    let (fight, _) = lauf("quench\nkindle\n");
    assert_eq!(
        wuerfe(&fight, "quench").first(),
        Some(&0),
        "QUENCH fehlt bei 0"
    );
    assert_eq!(
        wuerfe(&fight, "kindle").first(),
        Some(&0),
        "der Füller kommt nicht beim selben Zeitstempel wie QUENCH"
    );
}

/// SUBLIME bei 0 (mit Wartezeit bis 1500) befreit KINDLE bei 1500. Danach ist die
/// Wartezeit bereit, KINDLE abgeklungen erst bei 3000 — SMOULDER, nie gelegt,
/// ist (a) noch bei 1500 dran. Bis zur Festlegung vom 2026-10-28: 1510.
#[test]
fn nach_dem_befreiten_wurf_entscheidet_die_liste_beim_selben_zeitstempel() {
    let (fight, _) = lauf("sublime\nkindle\nsmoulder if dot.smoulder.remains <= 1500\n");
    assert_eq!(
        wuerfe(&fight, "kindle").first(),
        Some(&1500),
        "KINDLE nicht befreit bei 1500"
    );
    assert_eq!(
        wuerfe(&fight, "smoulder").first(),
        Some(&1500),
        "nach dem befreiten Wurf entscheidet die Liste nicht beim selben Zeitstempel"
    );
}

/// Nichts als Warten bis 5000, danach KINDLE an jedem GCD-Ende (Abklingzeit =
/// GCD): 500 leere Takte, also 5000 ms — keiner davon doppelt.
#[test]
fn leerlauf_zaehlt_jeden_wartenden_takt_genau_einmal() {
    let (fight, ergebnis) = lauf("kindle if time >= 5000\n");
    assert_eq!(
        wuerfe(&fight, "kindle").first(),
        Some(&5000),
        "KINDLE nicht bei 5000"
    );
    assert_eq!(
        ergebnis.casts[index_of("kindle").expect("kindle")],
        17,
        "KINDLE-Würfe"
    );
    assert_eq!(ergebnis.idle_ms, 5000, "Leerlauf vor 5000");
}

/// Nur QUENCH: gewirkt bei 0, 5000, …, 25000 (Abklingzeit 5000), dazwischen
/// bereit und nichts zu tun. 3000 Takte, 6 davon mit Wurf — 2994 leere Takte.
///
/// **Die Wartezeit nach dem freien Wurf zählt mit.** Bis #1027 fehlte sie: Nach
/// QUENCH entschied niemand, also zählte auch niemand.
#[test]
fn leerlauf_nach_einem_wurf_ohne_wartezeit_zaehlt_mit() {
    let (fight, ergebnis) = lauf("quench\n");
    assert_eq!(
        wuerfe(&fight, "quench"),
        vec![0, 5000, 10_000, 15_000, 20_000, 25_000],
        "QUENCH-Würfe"
    );
    assert_eq!(
        ergebnis.idle_ms, 29_940,
        "Leerlauf: 2994 leere Takte zu 10 ms"
    );
}

/// (b) QUENCH steht UNTER dem Füller. KINDLE ist an jedem GCD-Ende bereit, also
/// kam QUENCH bis zur Festlegung vom 2026-10-28 **nie** zum Zug. Jetzt: nach
/// KINDLE bei 0 im selben Takt, danach am Ende jeder Abklingzeit — bei 5000
/// mitten im GCD des KINDLE von 4500.
#[test]
fn quench_unter_dem_fueller_kommt_waehrend_der_wartezeit() {
    let (fight, ergebnis) = lauf("kindle\nquench\n");
    assert_eq!(
        wuerfe(&fight, "quench"),
        vec![0, 5000, 10_000, 15_000, 20_000, 25_000],
        "QUENCH-Würfe"
    );
    assert_eq!(
        ergebnis.casts[index_of("kindle").expect("kindle")],
        20,
        "KINDLE-Würfe: QUENCH darf keinen GCD verdrängen"
    );
    assert_eq!(
        ergebnis.idle_ms, 0,
        "kein Takt mit bereiter Wartezeit bleibt leer"
    );
}

/// (b) Eine Bedingung, die mitten im GCD wahr wird, greift an ihrem Takt —
/// dafür braucht der Simulator einen Weckruf **innerhalb** der Wartezeit, nicht
/// erst an ihrem Ende (1500).
#[test]
fn eine_bedingung_mitten_im_gcd_greift_an_ihrem_takt() {
    let (fight, ergebnis) = lauf("kindle\nquench if time >= 700\n");
    assert_eq!(
        wuerfe(&fight, "quench").first(),
        Some(&700),
        "QUENCH nicht bei 700, im GCD des KINDLE von 0"
    );
    assert_eq!(ergebnis.idle_ms, 0, "Leerlauf");
}

/// Leerlauf: Der Takt mit QUENCH ist nicht leer, die danach schon — QUENCH bei
/// 0 und 5000, KINDLE ab 5000: leer sind die Takte 10 bis 4990.
#[test]
fn nach_quench_beginnt_der_leerlauf_im_naechsten_takt() {
    let (fight, ergebnis) = lauf("quench\nkindle if time >= 5000\n");
    assert_eq!(wuerfe(&fight, "quench")[..2], [0, 5000], "QUENCH-Würfe");
    assert_eq!(
        wuerfe(&fight, "kindle").first(),
        Some(&5000),
        "KINDLE nicht beim selben Zeitstempel wie QUENCH bei 5000"
    );
    assert_eq!(ergebnis.idle_ms, 4990, "Leerlauf: 499 leere Takte zu 10 ms");
}
