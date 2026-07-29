//! Boss-Datenmodell, Phasen, Energieleiste und Zeitleiste (Issue #96).
//!
//! Die drei bestehenden Vergleichsdateien laufen alle gegen `first-crucible` —
//! einen Boss **ohne** Phasen, ohne Energieleiste, ohne Zeitleiste. Sie belegen
//! damit genau nichts über die neuen Felder: Phase, Energie und Zeiger stehen
//! dort von Anfang bis Ende auf null, und eine Umsetzung, die von all dem nichts
//! weiß, bestünde jeden dieser Prüffälle.
//!
//! Hier läuft deshalb dieselbe gute Rotation gegen `test-vessel`, einen Boss mit
//! allen dreien. Was sich zwischen beiden Aufzeichnungen unterscheidet, kommt aus
//! den Boss-Daten und aus nichts sonst.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, BOSS, BOSSES};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Der aufgezeichnete Kampf gegen den Boss mit Phasen, wie das Spiel ihn
/// geschrieben hat.
const VESSEL_JSON: &str = include_str!("../../tests/parity/recordings/vessel-phases.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(VESSEL_JSON).expect("vessel-phases.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn recording(json: &Value) -> Recording {
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(spell_id).unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt")),
            }
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick")).expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("ticks zu gross"),
        bindings,
        presses,
    }
}

fn replay(json: &Value, with_checksums: bool) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        // Der Boss soll fallen duerfen — sonst blieben die spaeten Phasen
        // ungeprueft.
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, with_checksums)
}

fn zahlen(json: &Value, feld: &str) -> Vec<u32> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect()
}

#[test]
fn ac1_ein_neuer_boss_entsteht_allein_aus_einer_datendatei() {
    // **Der eigentliche Nachweis ist ein negativer:** Der Bezeichner dieses
    // Bosses darf in keiner handgeschriebenen Quelldatei vorkommen. Stuende er
    // dort, waere er eben doch im Programmcode verankert.
    //
    // Gefunden wird er trotzdem — weil `build.rs` das Verzeichnis liest.
    let boss = boss_by_id("test-vessel").expect(
        "Der Boss aus data/bosses/test-vessel.json wurde nicht gefunden. \
         Liest build.rs das Verzeichnis noch?",
    );
    assert_eq!(boss.hp, 8000);
    assert_eq!(boss.overpressure_ms, 90_000);
    assert_eq!(boss.phases.len(), 2);
    assert!(boss.energy.is_some());
    assert_eq!(boss.timeline.len(), 3);

    // Und die Gegenprobe zur Gegenprobe: Ein Boss, den es nicht gibt, wird auch
    // nicht gefunden. Sonst koennte `boss_by_id` alles bejahen.
    assert!(boss_by_id("gibt-es-nicht").is_none());
}

#[test]
fn ac1_beide_seiten_sehen_dieselbe_boss_menge() {
    // Das Spiel liest das Verzeichnis ueber `import.meta.glob`, der Simulator
    // ueber `build.rs` — zwei voellig verschiedene Wege. AC1 ist erst erfuellt,
    // wenn eine neue Datendatei auf **beiden** ankommt.
    //
    // Ohne diesen Vergleich faellt ein Auseinanderlaufen erst auf, wenn jemand
    // gegen den fehlenden Boss rechnen will — und dann sieht es aus wie ein
    // Fehler in der Rotation.
    let json = data();
    let vom_spiel: Vec<&str> = json["allBossIds"]
        .as_array()
        .expect("allBossIds fehlt")
        .iter()
        .map(|v| v.as_str().expect("Zeichenkette"))
        .collect();
    let vom_simulator: Vec<&str> = BOSSES.iter().map(|b| b.id).collect();

    assert_eq!(
        vom_spiel, vom_simulator,
        "Spiel und Simulator kennen verschiedene Bosse. \
         Liegt eine neue Datei unter data/bosses/, die eine Seite nicht liest?"
    );
    assert!(vom_simulator.len() >= 2, "zu wenige Bosse zum Vergleichen");
}

#[test]
fn ac2_der_phasenwechsel_greift_auf_einen_takt_genau() {
    let json = data();
    let result = replay(&json, true);
    let erwartet = zahlen(&json, "phases");

    assert_eq!(
        result.phases, erwartet,
        "Spiel und Simulator wechseln die Phase zu verschiedenen Zeitpunkten"
    );

    // Nicht nur gleich, sondern auch richtig: Die Phase steigt, springt nicht,
    // und faellt nie zurueck (eskalierend, GAME-DESIGN 11.1).
    let mut wechsel = 0;
    for paar in result.phases.windows(2) {
        let (davor, jetzt) = (paar[0], paar[1]);
        assert!(jetzt >= davor, "die Phase ist zurueckgefallen");
        if jetzt != davor {
            assert_eq!(jetzt, davor + 1, "eine Phase wurde uebersprungen");
            wechsel += 1;
        }
    }
    assert_eq!(wechsel, 2, "beide Phasenwechsel muessen vorkommen");
}

#[test]
fn ac3_die_energieleiste_haengt_an_der_uhr_und_nicht_am_schaden() {
    let json = data();
    let result = replay(&json, true);

    assert_eq!(
        result.energy,
        zahlen(&json, "energy"),
        "Spiel und Simulator fuehren die Energieleiste verschieden"
    );

    // **Der Kern des Bausteins.** Derselbe Boss, dieselbe Uhr, aber viel
    // weniger Schaden: Die Energieleiste muss trotzdem identisch verlaufen.
    // Waere sie an den Schaden gekoppelt, liefe sie hier anders.
    let boss = boss_by_id("test-vessel").expect("test-vessel");
    let recording = recording(&json);
    let mut kaum = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    // Dieselbe Aufzeichnung, aber nur jeder zehnte Druck.
    let ausgeduennt = Recording {
        presses: recording
            .presses
            .iter()
            .step_by(10)
            .copied()
            .collect::<Vec<_>>(),
        ..recording.clone()
    };
    let schwach = kaum.replay(&ausgeduennt, true);

    assert!(
        schwach.damage < result.damage,
        "die ausgeduennte Runde muss weniger Schaden machen: {} gegen {}",
        schwach.damage,
        result.damage
    );
    // Die Energie-Spur des schwachen Laufs ist laenger (kein frueher Sieg) —
    // verglichen wird deshalb der gemeinsame Anfang.
    let gemeinsam = result.energy.len().min(schwach.energy.len());
    assert!(gemeinsam > 1000, "zu kurzer gemeinsamer Abschnitt");
    assert_eq!(
        result.energy[..gemeinsam],
        schwach.energy[..gemeinsam],
        "die Energieleiste hat sich mit dem Schaden veraendert — sie ist an ihn \
         gekoppelt, obwohl sie es nicht sein darf"
    );

    // Und die Zeitleiste genauso: Sie haengt allein an der Uhr.
    let gemeinsam = result
        .timeline_index
        .len()
        .min(schwach.timeline_index.len());
    assert_eq!(
        result.timeline_index[..gemeinsam],
        schwach.timeline_index[..gemeinsam],
        "die Zeitleiste hat sich mit dem Schaden veraendert"
    );
}

#[test]
fn ac3_die_zeitleiste_rueckt_zu_den_zeitpunkten_der_daten_vor() {
    let json = data();
    let result = replay(&json, true);
    let boss = boss_by_id("test-vessel").expect("test-vessel");

    assert_eq!(
        result.timeline_index,
        zahlen(&json, "timelineIndex"),
        "Spiel und Simulator arbeiten die Zeitleiste verschieden ab"
    );

    let mut getroffen = 0;
    for (tick, paar) in result.timeline_index.windows(2).enumerate() {
        if paar[1] == paar[0] {
            continue;
        }
        assert_eq!(paar[1], paar[0] + 1, "ein Eintrag wurde uebersprungen");
        // Der Takt, in dem vorgerueckt wird, ist GENAU der Zeitpunkt aus den
        // Daten. `tick + 1`, weil `windows(2)` beim Vorgaenger zaehlt.
        let zeitpunkt = (tick as u32 + 1) * TICK_MS;
        assert_eq!(
            zeitpunkt, boss.timeline[paar[0] as usize].at_ms,
            "der Eintrag griff zur falschen Zeit"
        );
        getroffen += 1;
    }
    assert_eq!(getroffen, boss.timeline.len(), "nicht jeder Eintrag griff");
}

#[test]
fn ac4_die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let result = replay(&json, true);
    let erwartet = zahlen(&json, "checksums");

    assert!(!erwartet.is_empty());
    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             uebereinstimmende Takte davor: {}\n  \
             Phase dort: {:?}, Energie: {:?}, Zeitleiste: {:?}",
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
            d.matched_ticks,
            result.phases.get(d.tick as usize),
            result.energy.get(d.tick as usize),
            result.timeline_index.get(d.tick as usize),
        );
    }
    assert_eq!(result.checksums.len(), erwartet.len());

    let expected = &json["expected"];
    assert_eq!(
        u64::from(result.damage),
        expected["damage"].as_u64().expect("damage"),
        "der Simulator rechnet einen anderen Schaden als das Spiel"
    );
    assert_eq!(
        result.killed_at_ms.map(u64::from),
        expected["killedAtMs"]
            .as_i64()
            .and_then(|v| u64::try_from(v).ok()),
        "der Boss faellt zu einem anderen Zeitpunkt"
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        expected["sequenceChecksum"].as_u64().expect("fehlt")
    );
    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"].as_u64().expect("fehlt")
    );
}

#[test]
fn ac4_der_prueffall_beruehrt_ueberhaupt_das_neue() {
    // **Ohne diesen Test behauptet der Prueffall eine Pruefung, die nicht
    // stattfindet** — dieselbe Falle wie in `parity_misplays.rs`. Ein Kampf,
    // in dem der Boss nie unter 70 % faellt, die Energie nie ausloest und kein
    // Zeitleisten-Eintrag greift, saehe fuer jede Umsetzung gleich aus.
    let json = data();
    let result = replay(&json, true);

    assert_eq!(*result.phases.last().expect("Spur leer"), 2);
    assert_eq!(*result.timeline_index.last().expect("Spur leer"), 3);
    assert!(
        result.energy.iter().any(|&e| e > 0),
        "die Energieleiste ist nie gestiegen"
    );
    // Mindestens ein Rueckfall heisst: Sie hat ausgeloest.
    let ausloesungen = result
        .energy
        .windows(2)
        .filter(|paar| paar[1] < paar[0])
        .count();
    assert_eq!(
        u64::try_from(ausloesungen).expect("passt"),
        json["expected"]["energyTriggers"]
            .as_u64()
            .expect("energyTriggers fehlt")
    );
    assert!(ausloesungen > 1, "die Leiste muss mehrfach ausloesen");

    // Und die Pruefsummen muessen sich unterscheiden — waeren sie alle gleich,
    // wuerde ein Vergleich je Takt nichts pruefen.
    let verschiedene: std::collections::BTreeSet<u32> = result.checksums.iter().copied().collect();
    assert!(verschiedene.len() > result.checksums.len() / 2);
}

#[test]
fn ac5_der_erste_boss_verhaelt_sich_unveraendert() {
    // Der erste Boss hat weder Phasen noch Energieleiste noch Zeitleiste. Wenn
    // das Datenmodell ihn nicht mehr korrekt abbildet, ist die bestehende
    // Paritaet hin — an einer Stelle, an der niemand sie vermutet.
    let boss = boss_by_id("first-crucible").expect("first-crucible fehlt");
    assert_eq!(boss.hp, 4200);
    assert_eq!(boss.overpressure_ms, 60_000);
    assert!(boss.phases.is_empty(), "der erste Boss hat keine Phasen");
    assert!(boss.energy.is_none(), "der erste Boss hat keine zweite Uhr");
    assert!(
        boss.timeline.is_empty(),
        "der erste Boss hat keine Zeitleiste"
    );

    // Und er ist weiterhin der voreingestellte.
    assert_eq!(BOSS.id, "first-crucible");

    // Der eigentliche Nachweis: Ein Kampf gegen ihn laesst alle drei neuen
    // Zustaende ueber die volle Dauer auf null. Eine Umsetzung, die etwa die
    // Energieleiste auch ohne Daten laufen liesse, faellt hier durch — und
    // zwar bevor irgendeine Pruefsumme davon beruehrt waere.
    let mut fight = Fight::new(FightConfig::default());
    fight.run(|f| {
        let kindle = index_of("kindle").expect("kindle");
        if f.is_ready(kindle) {
            Some(kindle)
        } else {
            None
        }
    });
    assert_eq!(fight.phase(), 0);
    assert_eq!(fight.energy(), 0);
    assert_eq!(fight.energy_triggers(), 0);
    assert_eq!(fight.timeline_index(), 0);
}
