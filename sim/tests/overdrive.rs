//! Overdrive im Simulator (#1177, Abschnitt 2, „Overdrive auf G").
//!
//! Die Regeln, je eine Gruppe von Fällen:
//!
//! | Regel | Fälle |
//! | --- | --- |
//! | einmal pro Kampf, ein zweiter Druck tut nichts | `ein_zweiter_*`, `einmal_*` |
//! | Wirkung genau in der Dauer | `wirkung_genau_*`, `am_letzten_takt_*` |
//! | Tempo heißt CATALYSIS, nicht globale Wartezeit | `ein_zauber_im_fenster_*`, `die_wartezeit_*` |
//! | außerhalb der Leiste | `der_druck_ist_kein_platz_*`, `leiste_und_wartezeit_*` |
//! | die Rotation kann zünden | `apl_*` |
//!
//! **Ohne getragene Ausrüstung**, mit Absicht: Dann kommt jedes Tempo in diesen
//! Läufen aus Overdrive, und eine verkürzte Zeit kann nur von ihm stammen.

use rota_sim::apl::{parse, spell_index, AplName, OVERDRIVE_ACTION};
use rota_sim::data::{index_of, PLAYER, SPELLS};
use rota_sim::haste::hasted_ms;
use rota_sim::overdrive::{Overdrive, OVERDRIVE_PRESS};
use rota_sim::replay::{Binding, Press, Recording, ReplayResult};
use rota_sim::{Fight, FightConfig, TICK_MS};

const BASE_JSON: &str = include_str!("../../data/player/base.json");

/// Ein Zauber mit Abklingzeit über der globalen Wartezeit und ohne Wirkzeit:
/// An seiner Abklingzeit ist das Tempo direkt abzulesen.
const CRUST: &str = "crust";

fn zauber(id: &str) -> usize {
    index_of(id).unwrap_or_else(|| panic!("Zauber {id} fehlt in data/spells"))
}

/// Eine Runde mit `crust` auf Platz 0 — die Drücke als (Takt, Kennung).
fn runde(ticks: u32, presses: &[(u32, u8)]) -> (Fight, ReplayResult) {
    let recording = Recording {
        name: "overdrive-test".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![Binding {
            slot: 0,
            spell: zauber(CRUST),
        }],
        presses: presses
            .iter()
            .map(|&(tick, slot)| Press { tick, slot })
            .collect(),
    };
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, false);
    (fight, result)
}

fn dauer_in_takten() -> u32 {
    Overdrive::duration_ms() / TICK_MS
}

#[test]
fn die_zahlen_kommen_aus_den_daten() {
    let json: serde_json::Value = serde_json::from_str(BASE_JSON).expect("base.json ist kein JSON");
    assert_eq!(
        json["overdriveDurationMs"].as_u64(),
        Some(u64::from(PLAYER.overdrive_duration_ms))
    );
    assert_eq!(
        json["overdriveCatalysisPermille"].as_u64(),
        Some(u64::from(PLAYER.overdrive_catalysis_permille))
    );
    assert_eq!(Overdrive::duration_ms(), PLAYER.overdrive_duration_ms);
    assert_eq!(
        Overdrive::catalysis_bonus(),
        PLAYER.overdrive_catalysis_permille
    );
    // Sonst wären die Fälle unten an einem wirkungslosen Overdrive grün.
    assert!(Overdrive::catalysis_bonus() > 0 && Overdrive::duration_ms() > 0);
}

#[test]
fn einmal_pro_kampf() {
    let mut overdrive = Overdrive::new();
    assert!(overdrive.is_ready());
    assert!(overdrive.ignite(1000), "der erste Druck zündet");
    assert!(!overdrive.is_ready());
    assert_eq!(overdrive.ignited_at(), Some(1000));
}

#[test]
fn ein_zweiter_druck_verlaengert_und_verschiebt_nichts() {
    let mut overdrive = Overdrive::new();
    assert!(overdrive.ignite(1000));
    let ende = overdrive.ends_at();
    // Während er wirkt, danach — beides tut nichts.
    assert!(!overdrive.ignite(5000), "zweiter Druck im Fenster");
    assert!(!overdrive.ignite(40_000), "zweiter Druck nach dem Ende");
    assert_eq!(overdrive.ignited_at(), Some(1000));
    assert_eq!(overdrive.ends_at(), ende);
}

#[test]
fn wirkung_genau_in_der_dauer() {
    let mut overdrive = Overdrive::new();
    let zuendung = 1000;
    assert!(overdrive.ignite(zuendung));
    let dauer = Overdrive::duration_ms();
    let bonus = Overdrive::catalysis_bonus();

    assert_eq!(
        overdrive.catalysis_permille(zuendung - TICK_MS),
        0,
        "vor dem Zünden"
    );
    assert_eq!(
        overdrive.catalysis_permille(zuendung),
        bonus,
        "im Takt des Zündens"
    );
    assert_eq!(
        overdrive.catalysis_permille(zuendung + dauer - TICK_MS),
        bonus,
        "im letzten Takt"
    );
    assert_eq!(
        overdrive.catalysis_permille(zuendung + dauer),
        0,
        "im Takt des Endes"
    );
    assert_eq!(overdrive.remaining_ms(zuendung), dauer);
    assert_eq!(overdrive.remaining_ms(zuendung + dauer - TICK_MS), TICK_MS);
    assert_eq!(overdrive.remaining_ms(zuendung + dauer), 0);

    // Genau `Dauer / Takt` Takte wirkt er — gezählt, nicht gefolgert.
    let wirksam = (0..10_000u32)
        .map(|t| t * TICK_MS)
        .filter(|&t| overdrive.is_active(t))
        .count();
    assert_eq!(wirksam, dauer_in_takten() as usize);
}

#[test]
fn ein_zauber_im_fenster_bekommt_das_tempo() {
    let crust = zauber(CRUST);
    let grund = SPELLS[crust].cooldown_ms;
    let erwartet = hasted_ms(grund, Overdrive::catalysis_bonus());
    // Sonst prüfte der Fall nichts: Bei dieser Abklingzeit muss Tempo sichtbar sein.
    assert!(erwartet < grund, "{grund} ms bleiben unter Tempo gleich");

    let (mit, ergebnis) = runde(1, &[(0, OVERDRIVE_PRESS), (0, 0)]);
    assert_eq!(ergebnis.casts[crust], 1, "crust wurde nicht gewirkt");
    assert_eq!(mit.cooldown_remaining_ms(crust), erwartet);

    // Die Gegenrichtung im selben Aufbau: ohne Zünden die volle Zeit.
    let (ohne, _) = runde(1, &[(0, 0)]);
    assert_eq!(ohne.cooldown_remaining_ms(crust), grund);
}

#[test]
fn am_letzten_takt_noch_am_ende_nicht_mehr() {
    let crust = zauber(CRUST);
    let grund = SPELLS[crust].cooldown_ms;
    let letzter = dauer_in_takten() - 1;

    let (drin, _) = runde(letzter + 1, &[(0, OVERDRIVE_PRESS), (letzter, 0)]);
    assert_eq!(
        drin.cooldown_remaining_ms(crust),
        hasted_ms(grund, Overdrive::catalysis_bonus()),
        "im letzten Takt der Dauer wirkt er noch"
    );

    let (draussen, _) = runde(letzter + 2, &[(0, OVERDRIVE_PRESS), (letzter + 1, 0)]);
    assert_eq!(
        draussen.cooldown_remaining_ms(crust),
        grund,
        "im Takt des Endes wirkt er nicht mehr"
    );
}

#[test]
fn ein_zweiter_druck_im_kampf_ist_eine_fehleingabe_und_verlaengert_nicht() {
    let crust = zauber(CRUST);
    let grund = SPELLS[crust].cooldown_ms;
    let ende = dauer_in_takten();
    // Zweiter Druck bei 15 s; crust im Takt, an dem der ERSTE enden muss.
    let (fight, ergebnis) = runde(
        ende + 1,
        &[(0, OVERDRIVE_PRESS), (1500, OVERDRIVE_PRESS), (ende, 0)],
    );
    assert_eq!(
        ergebnis.rejected, 1,
        "der zweite Druck zählt als Fehleingabe"
    );
    assert_eq!(ergebnis.accepted, 1, "nur crust ist eine Wirkung");
    assert_eq!(fight.overdrive().ignited_at(), Some(0));
    assert_eq!(
        fight.cooldown_remaining_ms(crust),
        grund,
        "der zweite Druck hat das Fenster verlängert"
    );
}

#[test]
fn der_druck_ist_kein_platz_der_leiste() {
    // 13 Stellen mal zwei Ebenen: Plätze 0 bis 25. Die Kennung liegt dahinter.
    assert_eq!(OVERDRIVE_PRESS, 26);
    // Ein Platz, auf dem ein Zauber liegt, zündet nichts …
    let (fight, ergebnis) = runde(1, &[(0, 0)]);
    assert_eq!(fight.overdrive().ignited_at(), None);
    assert_eq!(ergebnis.rejected, 0);
    // … und ein unbelegter auch nicht.
    let (fight, ergebnis) = runde(1, &[(0, 25)]);
    assert_eq!(fight.overdrive().ignited_at(), None);
    assert_eq!(ergebnis.rejected, 1);
}

#[test]
fn leiste_und_wartezeit_bleiben_unberuehrt() {
    let (fight, ergebnis) = runde(1, &[(0, OVERDRIVE_PRESS)]);
    assert_eq!(fight.overdrive().ignited_at(), Some(0), "gezündet");
    assert!(
        ergebnis.casts.iter().all(|&n| n == 0),
        "ein Zauber wurde gewirkt"
    );
    assert_eq!(ergebnis.accepted, 0);
    assert_eq!(ergebnis.rejected, 0);
    assert_eq!(
        fight.gcd_ready_at(),
        0,
        "Overdrive hat die Wartezeit ausgelöst"
    );

    // Und er sperrt nichts: crust im selben Takt danach wird gewirkt.
    let (_, ergebnis) = runde(1, &[(0, OVERDRIVE_PRESS), (0, 0)]);
    assert_eq!(ergebnis.accepted, 1);
}

#[test]
fn die_wartezeit_bleibt_unter_overdrive_gleich() {
    // GAME-DESIGN 6.2: Der GCD sinkt ausschließlich über den Talentbaum.
    let (mit, _) = runde(1, &[(0, OVERDRIVE_PRESS), (0, 0)]);
    let (ohne, _) = runde(1, &[(0, 0)]);
    assert!(mit.gcd_ready_at() > 0, "crust löst eine Wartezeit aus");
    assert_eq!(mit.gcd_ready_at(), ohne.gcd_ready_at());
}

#[test]
fn das_zuenden_bricht_eine_laufende_wirkzeit_ab() {
    // GAME-DESIGN 7.1, entschieden am 2026-08-05: „Jede Wirkung des Spielers
    // bricht einen laufenden Cast oder eine Kanalisierung ab. Jede, ohne
    // Ausnahme — auch das Unterbrechen. […] Egal welcher Spell."
    let wirkzeit = zauber("test-cast");
    assert!(
        SPELLS[wirkzeit].cast_time_ms > 500,
        "test-cast hat keine Wirkzeit mehr"
    );
    let recording = Recording {
        name: "overdrive-abbruch".to_owned(),
        seed: 0,
        ticks: 400,
        bindings: vec![Binding {
            slot: 0,
            spell: wirkzeit,
        }],
        presses: vec![
            Press { tick: 0, slot: 0 },
            Press {
                tick: 50,
                slot: OVERDRIVE_PRESS,
            },
        ],
    };
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(400 * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);
    assert_eq!(fight.overdrive().ignited_at(), Some(500));
    assert_eq!(ergebnis.casts_cancelled, 1, "die Wirkzeit lief weiter");
    assert_eq!(
        ergebnis.casts[wirkzeit], 0,
        "der abgebrochene Zauber hat gewirkt"
    );

    // Ein Druck, der nichts zündet, bricht nichts ab — wie in `cast`.
    let recording = Recording {
        presses: vec![
            Press {
                tick: 0,
                slot: OVERDRIVE_PRESS,
            },
            Press { tick: 10, slot: 0 },
            Press {
                tick: 50,
                slot: OVERDRIVE_PRESS,
            },
        ],
        ..recording
    };
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(400 * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);
    assert_eq!(
        ergebnis.casts_cancelled, 0,
        "ein abgewiesener Druck brach ab"
    );
    assert_eq!(ergebnis.casts[wirkzeit], 1);
}

#[test]
fn die_pruefsumme_sieht_die_zuendung() {
    let (mit, _) = runde(1, &[(0, OVERDRIVE_PRESS)]);
    let (ohne, _) = runde(1, &[]);
    assert_ne!(mit.checksum_at(0), ohne.checksum_at(0));
}

#[test]
fn ohne_zuendung_bleibt_die_pruefsumme_wie_vorher() {
    // Der Fall, an dem jede bestehende Aufzeichnung hängt: Wer nicht zündet,
    // bekommt dieselbe Summe wie ein Kampf, in dem es Overdrive nicht gäbe.
    // Belegt wird das von den bestehenden Paritätsfällen; hier nur, dass ein
    // abgewiesener Druck nichts hinterlässt.
    let (abgewiesen, _) = runde(2, &[(0, OVERDRIVE_PRESS), (1, OVERDRIVE_PRESS)]);
    let (einmal, _) = runde(2, &[(0, OVERDRIVE_PRESS)]);
    assert_eq!(abgewiesen.checksum_at(10), einmal.checksum_at(10));
}

// ---------------------------------------------------------------------------
// Die Rotation kann zünden
// ---------------------------------------------------------------------------

fn kampf_mit(text: &str, dauer_ms: u32) -> Fight {
    let apl = parse(AplName("overdrive-test"), text).expect("Liste ist gültig");
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(dauer_ms),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(&apl);
    fight
}

#[test]
fn apl_ohne_zeile_zuendet_nie() {
    let fight = kampf_mit("kindle\n", 10_000);
    assert_eq!(fight.overdrive().ignited_at(), None);
}

#[test]
fn apl_ohne_bedingung_zuendet_gleich_zu_beginn() {
    let fight = kampf_mit("kindle\noverdrive\n", 10_000);
    assert_eq!(fight.overdrive().ignited_at(), Some(0));
}

#[test]
fn apl_zuendet_am_takt_ihrer_bedingung() {
    // 5000 liegt zwischen zwei Wartezeiten (4500, 6000): Ohne Weckruf fiele
    // die Zündung erst bei 6000.
    let fight = kampf_mit("kindle\noverdrive if time >= 5000\n", 10_000);
    assert_eq!(fight.overdrive().ignited_at(), Some(5000));
}

#[test]
fn apl_zuendet_auch_in_einer_laufenden_wartezeit() {
    // kindle wirkt bei 0, die Wartezeit läuft bis 1500.
    let fight = kampf_mit("overdrive if time >= 700\nkindle\n", 10_000);
    assert_eq!(fight.overdrive().ignited_at(), Some(700));
}

#[test]
fn apl_zuendet_auch_wenn_nichts_zu_wirken_ist() {
    // Die Gegenseite zum Fall darüber, und sie fehlte: Die Gegenprobe, die
    // Overdrive aus `next_wakeup` nahm, blieb grün, weil dort jede Zündung in
    // eine laufende Wartezeit fiel und der andere Weckruf sie trug. Hier ist
    // die Wartezeit von crust bei 1500 vorbei, crust selbst bis 6000 gesperrt —
    // bei 2730 kann nur der allgemeine Weckruf den Kampf anhalten.
    //
    // **2730 und nicht 2000:** Mein erster Wert war 2000, und die Gegenprobe
    // blieb grün. Die Hitze verfällt alle 2000 ms (`decayPerStepMs`), dort
    // steht also ohnehin ein Ereignis, und der Kampf hielt an, ohne gerufen
    // worden zu sein. Ein Zeitpunkt neben jedem festen Takt der Daten
    // misst den Weckruf statt den Verfall.
    let fight = kampf_mit("crust\noverdrive if time >= 2730\n", 10_000);
    assert_eq!(fight.overdrive().ignited_at(), Some(2730));
}

#[test]
fn apl_zuendung_veraendert_den_kampf() {
    // Die Suche braucht einen Unterschied, den sie finden kann.
    //
    // **Gemessen an den DoT-Ticks, nicht an der Zahl der Würfe** — und das ist
    // ein Befund, kein Geschmack. Mein erster Entwurf zählte `crust` (6000 ms)
    // und bekam mit und ohne Overdrive je 5 Würfe in 30 s: Die verkürzte
    // Abklingzeit (4620 ms) endet MITTEN in einer Wartezeit, und die Rotation
    // wirft ohnehin erst an der nächsten Grenze, also wieder bei 6000. Ein
    // Tempo-Vorteil auf Abklingzeiten zählt nur, wo er eine Wartezeit-Grenze
    // überspringt. Die Abstände eines Brands dagegen hängen an keiner Grenze.
    const LISTE: &str = "smoulder if dot.smoulder.remains < gcd\nkindle\n";
    let ohne = kampf_mit(LISTE, 30_000);
    let mit = kampf_mit(&format!("{LISTE}{OVERDRIVE_ACTION}\n"), 30_000);
    let smoulder = zauber("smoulder");
    assert!(
        mit.dot_tick_count(smoulder) > ohne.dot_tick_count(smoulder),
        "smoulder tickt mit Overdrive {} mal, ohne {} mal",
        mit.dot_tick_count(smoulder),
        ohne.dot_tick_count(smoulder)
    );
    assert!(
        mit.damage() > ohne.damage(),
        "Schaden mit Overdrive {}, ohne {}",
        mit.damage(),
        ohne.damage()
    );
}

#[test]
fn apl_zweimal_overdrive_ist_ein_fehler() {
    let fehler = parse(
        AplName("doppelt"),
        "overdrive\nkindle\noverdrive if time >= 5000\n",
    )
    .expect_err("zwei Overdrive-Zeilen müssen abgelehnt werden");
    assert_eq!(fehler.line, 3);
}

#[test]
fn apl_overdrive_ist_kein_zauber_und_bleibt_beim_zurueckschreiben() {
    assert_eq!(
        spell_index(OVERDRIVE_ACTION),
        None,
        "ein Zauber heißt overdrive"
    );
    let text = "kindle\noverdrive if boss.hp_pct <= 20\n";
    let apl = parse(AplName("hin"), text).expect("gültig");
    assert!(apl.overdrive.is_some());
    assert_eq!(apl.rules.len(), 1, "overdrive ist als Regel gelandet");
    let zurueck = parse(AplName("hin"), &apl.to_string()).expect("Ausgabe ist gültig");
    assert_eq!(zurueck, apl);
}
