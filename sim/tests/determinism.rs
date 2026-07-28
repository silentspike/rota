//! AC1 und AC2 aus Issue #53 — Determinismus.
//!
//! * **AC1:** Zwei Laeufe mit demselben Startwert liefern byteweise dasselbe.
//! * **AC2:** Ein zusaetzlicher Krit-Wurf verschiebt die DoT-Tick-Folge nicht.
//!
//! Beide pruefen Regeln aus GAME-DESIGN 18.4, und beide sind schaerfer gefasst
//! als noetig: AC1 vergleicht nicht die Endsumme, sondern **jeden einzelnen
//! Protokoll-Eintrag als Bytefolge**. Zwei Laeufe, die zufaellig auf denselben
//! Schaden kommen, ihn aber anders erzeugt haben, faellt dieser Test auf.

use rota_sim::log::Entry;
use rota_sim::rng::Stream;
use rota_sim::rotation;
use rota_sim::{Fight, FightConfig, RunResult};

/// Packt einen Durchgang in eine Bytefolge: erst das Ergebnis, dann jeder
/// Protokoll-Eintrag. Alles in fester Reihenfolge und mit fester Bytebreite —
/// ein Vergleich, an dem sich nichts wegdiskutieren laesst.
fn fingerprint(result: &RunResult, entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + entries.len() * 10);

    out.extend_from_slice(&result.damage.to_le_bytes());
    out.extend_from_slice(&result.killed_at_ms.unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&result.remaining_ms.to_le_bytes());
    out.extend_from_slice(&result.resource_overflow.to_le_bytes());
    out.extend_from_slice(&result.idle_ms.to_le_bytes());
    out.extend_from_slice(&result.random_draws.to_le_bytes());
    for value in result
        .casts
        .iter()
        .chain(&result.damage_by_spell)
        .chain(&result.lost_dot_ticks)
    {
        out.extend_from_slice(&value.to_le_bytes());
    }

    for entry in entries {
        out.extend_from_slice(&entry.time.to_le_bytes());
        out.push(entry.kind as u8);
        out.push(entry.spell);
        out.extend_from_slice(&entry.value.to_le_bytes());
    }
    out
}

fn run_once(seed: u64) -> Vec<u8> {
    let mut fight = Fight::new(FightConfig {
        seed,
        ..FightConfig::default()
    });
    let result = fight.run(rotation::good);
    fingerprint(&result, &fight.log().entries())
}

#[test]
fn ac1_zwei_laeufe_mit_gleichem_startwert_sind_byteweise_gleich() {
    let erster = run_once(0xDEAD_BEEF);
    let zweiter = run_once(0xDEAD_BEEF);

    assert_eq!(
        erster.len(),
        zweiter.len(),
        "unterschiedlich viele Protokoll-Eintraege"
    );
    assert_eq!(erster, zweiter, "die Laeufe unterscheiden sich");
    assert!(
        erster.len() > 100,
        "der Vergleich muss etwas zu vergleichen haben, hat aber nur {} Byte",
        erster.len()
    );
}

#[test]
fn ac1_gilt_auch_ueber_zuruecksetzen_hinweg() {
    // `reset` behaelt den Speicher und legt nur den Zustand neu. Waere dabei ein
    // Feld vergessen, saehe man es genau hier — und sonst wahrscheinlich nie.
    let mut fight = Fight::new(FightConfig {
        seed: 7,
        ..FightConfig::default()
    });
    let erst = fight.run(rotation::good);
    let erst_log = fight.log().entries();

    fight.reset(7);
    let dann = fight.run(rotation::good);
    let dann_log = fight.log().entries();

    assert_eq!(
        fingerprint(&erst, &erst_log),
        fingerprint(&dann, &dann_log),
        "nach dem Zuruecksetzen rechnet derselbe Kampf anders"
    );
}

#[test]
fn ac1_alle_vier_rotationen_sind_wiederholbar() {
    for (name, apl) in [
        ("good", rotation::good as rota_sim::fight::Rotation),
        ("bad", rotation::bad),
        ("dot_error_only", rotation::dot_error_only),
        ("pyre_error_only", rotation::pyre_error_only),
    ] {
        let mut a = Fight::new(FightConfig::default());
        let mut b = Fight::new(FightConfig::default());
        let ra = a.run(apl);
        let rb = b.run(apl);
        assert_eq!(
            fingerprint(&ra, &a.log().entries()),
            fingerprint(&rb, &b.log().entries()),
            "Rotation {name} liefert zwei verschiedene Laeufe"
        );
    }
}

/// Zeitpunkt und Schaden jedes DoT-Ticks eines Durchgangs.
fn dot_tick_folge(seed: u64) -> Vec<(u32, u32)> {
    let mut fight = Fight::new(FightConfig {
        seed,
        ..FightConfig::default()
    });
    fight.run(rotation::good);
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == rota_sim::log::LogKind::DotTick)
        .map(|e| (e.time, e.value))
        .collect()
}

#[test]
fn ac2_zusaetzliche_krit_wuerfe_verschieben_die_dot_tick_folge_nicht() {
    let ohne = dot_tick_folge(1234);
    assert!(
        !ohne.is_empty(),
        "ohne DoT-Ticks prueft dieser Test gar nichts"
    );

    // Derselbe Kampf, aber vorher wird 500-mal aus dem Krit-Strom gezogen — so,
    // wie es ab Schritt 3 durch jeden zusaetzlichen Treffer geschieht.
    let mut fight = Fight::new(FightConfig {
        seed: 1234,
        ..FightConfig::default()
    });
    for _ in 0..500 {
        fight.rng().next_u64(Stream::Crit);
    }
    fight.run(rotation::good);
    let mit: Vec<(u32, u32)> = fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == rota_sim::log::LogKind::DotTick)
        .map(|e| (e.time, e.value))
        .collect();

    assert_eq!(
        ohne, mit,
        "500 Krit-Wuerfe haben die DoT-Ticks verschoben — die Stroeme sind nicht getrennt"
    );
    assert_eq!(fight.draws(Stream::Crit), 500);
    assert_eq!(
        fight.draws(Stream::DotTick),
        0,
        "in Schritt 1 zieht der DoT-Strom noch nicht"
    );
}

#[test]
fn ac2_gilt_auch_fuer_die_naechsten_ziehungen_des_dot_stroms() {
    // Der Test oben zeigt, dass die **Ticks** unbeeinflusst bleiben. Weil in
    // Schritt 1 aber niemand aus dem DoT-Strom zieht, waere er allein noch kein
    // Beleg dafuer, dass der Strom selbst unberuehrt ist. Also zusaetzlich:
    // Die naechsten Ziehungen muessen dieselben sein.
    let mut ohne = Fight::new(FightConfig {
        seed: 99,
        ..FightConfig::default()
    });
    ohne.run(rotation::good);
    let erwartet: Vec<u64> = (0..16)
        .map(|_| ohne.rng().next_u64(Stream::DotTick))
        .collect();

    let mut mit = Fight::new(FightConfig {
        seed: 99,
        ..FightConfig::default()
    });
    for _ in 0..500 {
        mit.rng().next_u64(Stream::Crit);
    }
    mit.run(rotation::good);
    let tatsaechlich: Vec<u64> = (0..16)
        .map(|_| mit.rng().next_u64(Stream::DotTick))
        .collect();

    assert_eq!(
        erwartet, tatsaechlich,
        "Ziehungen aus dem Krit-Strom haben den DoT-Strom bewegt"
    );
}

#[test]
fn verschiedene_startwerte_aendern_in_schritt_1_nichts() {
    // Kein Krit heisst: kein Zufall. Der Startwert darf das Ergebnis deshalb
    // nicht beruehren — und wenn doch, hat sich irgendwo eine Ziehung
    // eingeschlichen, die es nicht geben duerfte.
    let a = run_once(1);
    let b = run_once(0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(
        a, b,
        "der Startwert wirkt sich aus, obwohl in Schritt 1 niemand zieht"
    );
}
