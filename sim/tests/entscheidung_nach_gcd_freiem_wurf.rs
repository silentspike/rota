//! Entscheidung und Leerlauf nach einem Wurf ohne Wartezeit (Issue #1027).
//!
//! Die Sollwerte sind hier **von Hand hergeleitet**, aus der Regel des Spiels
//! (`driveRotation` in `src/devtools/reference-run.ts`): je Takt eine
//! Entscheidung, nur bei bereiter globaler Wartezeit, und je Takt, in dem dabei
//! nichts gewählt wird, 10 ms Leerlauf. Dieselben Lagen fährt
//! `parity_gcd_freier_wurf.rs` gegen die Aufzeichnung des Spiels; dieser Fall
//! hängt an keiner Datei und zeigt, was die Zahl bedeutet.
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

/// QUENCH bei 0 löst keine Wartezeit aus; im nächsten Takt ist der Füller dran.
///
/// **Nicht im selben Takt:** Die Schleife des Spiels entscheidet je Takt einmal.
/// Ein Füller bei 0 wäre eine Regel, die dort niemand fährt.
#[test]
fn nach_quench_wirft_die_liste_den_fueller_einen_takt_spaeter() {
    let (fight, _) = lauf("quench\nkindle\n");
    assert_eq!(
        wuerfe(&fight, "quench").first(),
        Some(&0),
        "QUENCH fehlt bei 0"
    );
    assert_eq!(
        wuerfe(&fight, "kindle").first(),
        Some(&10),
        "der Füller kommt nicht einen Takt nach QUENCH"
    );
}

/// SUBLIME bei 0 (mit Wartezeit bis 1500) befreit KINDLE bei 1500. Danach ist die
/// Wartezeit bereit, KINDLE abgeklungen erst bei 3000 — SMOULDER, nie gelegt,
/// ist bei 1510 dran.
#[test]
fn nach_dem_befreiten_wurf_entscheidet_die_liste_einen_takt_spaeter() {
    let (fight, _) = lauf("sublime\nkindle\nsmoulder if dot.smoulder.remains <= 1500\n");
    assert_eq!(
        wuerfe(&fight, "kindle").first(),
        Some(&1500),
        "KINDLE nicht befreit bei 1500"
    );
    assert_eq!(
        wuerfe(&fight, "smoulder").first(),
        Some(&1510),
        "nach dem befreiten Wurf entscheidet die Liste nicht im nächsten Takt"
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
