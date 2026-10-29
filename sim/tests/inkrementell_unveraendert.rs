//! AC4 — der Zweig „ändert nichts“ für den Greedy-Einfügeoperator, am heutigen
//! Bestand neu beziffert (Issue #917).
//!
//! # Was gemessen wurde, und wie
//!
//! Anhang B.2.1 von `docs/SIM-ARCHITEKTUR-PLAN.md` (2026-08-14): boss-1-1,
//! 60 s, Startwert 5, eine Suchrunde des Greedy-Einfügens — jede offene Zeile an
//! jeder Stelle. Gezählt wurden die Kandidaten, deren **Cast-Folge aus dem
//! Kampfprotokoll** (Zeit und Zauber) die der Basis ist:
//!
//! ```text
//! Basislänge 2:  141 von 273 Kandidaten (51,6 %)
//! Basislänge 5:  216 von 528 Kandidaten (40,9 %)
//! ```
//!
//! Dieser Fall misst **denselben Weg** — Protokoll-Cast-Folge im Prüfkampf —
//! am heutigen Datenbestand, und hält den Erkenner dagegen: Für jeden
//! Kandidaten muss „ändert nichts“ genau dann gemeldet werden, wenn die
//! Cast-Folge unverändert ist und das Ergebnis bitgleich. Die Zahlen von damals
//! sind eine Basis, kein Kriterium (Präzedenz #909): Sie stehen in der Meldung,
//! die Eigenschaft je Kandidat in der Bedingung.
//!
//! # Die Kontrollprobe
//!
//! Ein Erkenner, der immer „ändert nichts“ sagt, wäre schnell und falsch. Die
//! Eigenschaft je Kandidat fängt ihn — jede eingefügte Zeile, die nachweislich
//! greift, muss als Änderung erkannt werden —, und eigens geprüft wird, dass es
//! solche Zeilen in der Runde überhaupt gibt.

#[allow(dead_code)]
mod inkrementell_hilfe;

use inkrementell_hilfe::{eingefuegt, kampf, DAUER_MS};
use rota_sim::apl::{Apl, Rule};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::fight::{AuditFight, FightConfig};
use rota_sim::inkrementell::Basislauf;
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{all_candidates, boss_kandidaten, greedy_zuege, Candidate, Zuege};
use rota_sim::Millis;

/// Die Messung aus B.2.1: Boss, Startwert, Dauer.
const BOSS: &str = "boss-1-1";
const STARTWERT: u32 = 5;

fn suchkonfiguration() -> BalanceConfig {
    BalanceConfig {
        iterations: 1,
        base_seed: STARTWERT,
        threads: 1,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        boss: boss_by_id(BOSS).expect("boss-1-1 fehlt"),
        ..BalanceConfig::default()
    }
}

/// Die Cast-Folge aus dem Kampfprotokoll, nach Zeit — der Messweg aus B.2.1.
fn cast_folge(config: FightConfig, apl: &Apl) -> (Vec<(Millis, u8)>, rota_sim::fight::Zielwerte) {
    let mut pruef = AuditFight::new(FightConfig {
        log_capacity: 1 << 16,
        ..config
    });
    pruef.run_apl(apl);
    assert!(!pruef.log().overflowed(), "das Protokoll ist übergelaufen");
    let folge = pruef
        .log()
        .entries()
        .into_iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| (e.time, e.spell))
        .collect();
    (folge, pruef.zielwerte())
}

/// Eine Suchrunde an der Basis der Länge `laenge`: (Kandidaten, als
/// unverändert erkannt, Cast-Folge unverändert).
fn runde(laenge: usize) -> (usize, usize, usize) {
    let suche = suchkonfiguration();
    let pool = all_candidates();
    let erweitert: Vec<Candidate> = pool
        .iter()
        .chain(&boss_kandidaten(&suche, &pool))
        .cloned()
        .collect();
    let gefunden = greedy_zuege(&suche, &pool, laenge, &[], Zuege::NUR_EINZELN).chosen;
    assert_eq!(
        gefunden.len(),
        laenge,
        "die Greedy-Suche hielt vor Länge {laenge} an — die Basis gibt es heute nicht"
    );
    // Was offen ist, wie in `greedy_zuege`: der erweiterte Vorrat ohne die
    // gewählten Zeilen.
    let offen: Vec<Rule> = erweitert
        .iter()
        .filter(|k| !gefunden.contains(k))
        .map(Candidate::rule)
        .collect();
    let basis_apl = Apl {
        name: format!("basis-{laenge}"),
        rules: gefunden.iter().map(Candidate::rule).collect(),
    };
    let config = kampf(BOSS, STARTWERT);
    let basis = Basislauf::aufzeichnen(config, &basis_apl, &offen);
    let (basis_folge, basis_ziel) = cast_folge(config, &basis_apl);
    assert_eq!(basis.ziel(), basis_ziel);
    assert!(
        basis_folge.len() > 10,
        "die Basis wirkt kaum — misst die Runde etwas?"
    );

    let mut suchkampf = basis.suchkampf();
    let liste = basis.basis();
    let (mut kandidaten, mut erkannt, mut folge_gleich) = (0, 0, 0);
    for k in 0..offen.len() {
        for stelle in 0..=liste.len() {
            let abgewandelt = eingefuegt(&liste, stelle, basis.vorrat(k));
            let apl = basis.apl(&abgewandelt);
            let (folge, ziel) = cast_folge(config, &apl);
            let unveraendert = folge == basis_folge;
            let auswertung = basis.auswerten(&abgewandelt, &mut suchkampf);
            // Korrektheit gegen die Vollsimulation, für jeden Kandidaten.
            assert_eq!(
                auswertung.ziel(),
                ziel,
                "Basislänge {laenge}: {:?} an Stelle {stelle}",
                offen[k].condition
            );
            // Die Eigenschaft je Kandidat: erkannt genau dann, wenn unverändert.
            assert_eq!(
                auswertung.unveraendert(),
                unveraendert && ziel == basis_ziel,
                "Basislänge {laenge}: Zeile {} ({:?}) an Stelle {stelle} — erkannt {}, \
                 Cast-Folge unverändert {unveraendert}",
                offen[k].spell,
                offen[k].condition,
                auswertung.unveraendert()
            );
            kandidaten += 1;
            erkannt += usize::from(auswertung.unveraendert());
            folge_gleich += usize::from(unveraendert);
        }
    }
    let anteil = |x: usize| 100.0 * x as f64 / kandidaten as f64;
    println!(
        "Basislänge {laenge}: {erkannt} von {kandidaten} Kandidaten ({:.1} %) als „ändert \
         nichts“ erkannt; Cast-Folge unverändert {folge_gleich} ({:.1} %). \
         B.2.1 vom 2026-08-14: {}",
        anteil(erkannt),
        anteil(folge_gleich),
        if laenge == 2 {
            "141 von 273 (51,6 %)"
        } else {
            "216 von 528 (40,9 %)"
        }
    );
    (kandidaten, erkannt, folge_gleich)
}

#[test]
fn ac4_aendert_nichts_reproduziert_die_messung_bei_basislaenge_2() {
    let (kandidaten, erkannt, folge_gleich) = runde(2);
    assert_eq!(erkannt, folge_gleich);
    // Die Kontrollprobe: Es gibt Zeilen, die greifen, und es gibt welche, die
    // es nie tun — sonst prüfte die Eigenschaft nur eine Seite.
    assert!(erkannt > 0 && erkannt < kandidaten);
}

#[test]
fn ac4_aendert_nichts_reproduziert_die_messung_bei_basislaenge_5() {
    let (kandidaten, erkannt, folge_gleich) = runde(5);
    assert_eq!(erkannt, folge_gleich);
    assert!(erkannt > 0 && erkannt < kandidaten);
}
