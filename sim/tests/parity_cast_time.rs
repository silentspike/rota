//! Die WIRKZEIT des Spielers, Takt fuer Takt (Issue #304, AC5).
//!
//! WARUM ES DIESE DATEI GIBT
//! -------------------------
//! Bis zu diesem Issue hat **keine einzige Vergleichsdatei eine Spieler-
//! Wirkzeit gefahren** — es gab keine. Alle fuenfzehn Zauber standen auf
//! `castTimeMs: 0`, und weder `Combat.cast` noch `Fight::cast` fragte das Feld
//! an irgendeiner Stelle. Der ganze Zweig war damit unbelegt: Ein Simulator,
//! der die Wirkzeit anders rechnet als das Spiel, waere gruen geblieben, weil
//! kein Fall den Unterschied zeigen konnte.
//!
//! Dieselbe Lage wie bei `runaway` vor #256 und bei der Stufe vor #289. Es ist
//! die Frage aus der vierten Gestalt, eine Ebene hoeher gestellt: Koennte diese
//! Pruefung den Unterschied ueberhaupt zeigen?
//!
//! FUENF DINGE IN EINEM LAUF, alle an der Spur `playerCasting` ablesbar
//! -------------------------------------------------------------------
//! | Zeit | Was |
//! | --- | --- |
//! | 0 ms | Der Zauber beginnt; sein Schaden faellt bei 2000 ms, nicht sofort |
//! | 8500 ms | Ein gewoehnlicher Schlag trifft die laufende Wirkzeit und stoert sie NICHT |
//! | 9000 ms | Ein Schlag mit `interruptsPlayer` bricht sie ab |
//! | 9500 ms | Neu gewirkt — die Abklingzeit ist beim Abbruch nie angelaufen |
//! | 18000 ms | Derselbe Abbruch ein zweites Mal, damit er kein Einzelfall ist |
//! | 20500 ms | **Der Spieler bricht selbst ab**, mit einem eigenen Druck |
//!
//! Die vierte Zeile ist die zweite Haelfte von GAME-DESIGN 7.1, und sie ist
//! die, die gern vergessen wird: "Bei Abbruch geht der Zauber verloren, die
//! Abklingzeit laeuft aber nicht an." Waere sie angelaufen, koennte bei 9500 ms
//! nicht gewirkt werden.
//!
//! Die letzte Zeile kam am 2026-08-05 dazu, als der Auftraggeber die Regel
//! umgedreht hat: Die Wirkzeit sperrt nichts, und **jede** Wirkung bricht sie
//! ab — auch das Unterbrechen. Der Abbruch hat damit zwei Ausloeser, und beide
//! muessen in der Vergleichsdatei vorkommen; sonst ist der haeufigere von
//! beiden ungeprueft.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const CAST_TIME_JSON: &str = include_str!("../../tests/parity/recordings/cast-time.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(CAST_TIME_JSON).expect("cast-time.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn recording(json: &Value) -> Recording {
    // **Die Belegung kommt aus der DATEI**, nicht aus einer Abschrift hier.
    // Der Pruef-Zauber steht in keiner Leiste des Spiels; wer seinen Platz hier
    // hinschriebe, haette die Zuordnung an zwei Orten — und einer laeuft weg.
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

fn replay(json: &Value) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
}

fn zahlen(json: &Value, feld: &str) -> Vec<i64> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| v.as_i64().expect("Zahl"))
        .collect()
}

#[test]
fn der_fall_kann_den_unterschied_ueberhaupt_zeigen() {
    // **Die Zusicherung, die diesem Pruefstand seinen Wert gibt.** Ohne sie
    // waere alles Weitere gruen, auch wenn in dem Lauf nie eine Wirkzeit lief —
    // und genau dieser Zustand ist der Grund fuer diese Datei.
    let json = data();
    let casting = zahlen(&json, "playerCasting");
    let ends_at = zahlen(&json, "playerCastEndsAt");

    let laufend = casting.iter().filter(|&&v| v >= 0).count();
    assert!(
        laufend > 100,
        "in dem Lauf lief fast nie eine Wirkzeit ({laufend} Takte) — der Fall pruefte nichts"
    );
    assert_eq!(
        casting.len(),
        ends_at.len(),
        "die zwei Spuren muessen gleich lang sein"
    );

    // Und der Abbruch: Irgendwo faellt `playerCasting` von einem Zauber auf -1,
    // OHNE dass der Zeitpunkt erreicht war. Ohne diese Stelle pruefte der Fall
    // nur das Wirken und nicht das Abbrechen.
    let abbrueche = casting
        .windows(2)
        .zip(ends_at.windows(2))
        .enumerate()
        .filter(|(i, (c, e))| {
            let zeit = (*i as i64 + 1) * i64::from(TICK_MS);
            c[0] >= 0 && c[1] < 0 && zeit < e[0]
        })
        .count();
    assert!(
        abbrueche > 0,
        "keine Wirkzeit wurde abgebrochen — AC4 waere unbelegt"
    );

    let erwartet = &json["expected"];
    assert_eq!(
        erwartet["castsInterrupted"].as_u64(),
        Some(2),
        "gemessen: zwei Abbrueche durch den Boss, bei 9000 und 18000 ms"
    );
    assert_eq!(
        erwartet["castsCancelled"].as_u64(),
        Some(1),
        "gemessen: ein Abbruch durch den Spieler selbst, bei 20500 ms"
    );
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet: Vec<u32> = zahlen(&json, "checksums")
        .into_iter()
        .map(|v| u32::try_from(v).expect("32 Bit"))
        .collect();

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: Spiel {}, Simulator {}",
        erwartet.len(),
        result.checksums.len()
    );

    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        let casting = zahlen(&json, "playerCasting");
        let ends_at = zahlen(&json, "playerCastEndsAt");
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             Wirkzeit dort: Zauber {:?}, faellig {:?}",
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
            casting.get(d.tick as usize),
            ends_at.get(d.tick as usize),
        );
    }
}

#[test]
fn der_schluss_stand_stimmt() {
    let json = data();
    let result = replay(&json);
    let erwartet = &json["expected"];

    assert_eq!(
        u64::from(result.damage),
        erwartet["damage"].as_u64().expect("damage fehlt"),
        "verschiedener Schaden"
    );
    assert_eq!(
        u64::from(result.accepted),
        erwartet["accepted"].as_u64().expect("accepted fehlt"),
        "verschieden viele angenommene Druecke"
    );
    assert_eq!(
        u64::from(result.rejected),
        erwartet["rejected"].as_u64().expect("rejected fehlt"),
        "verschieden viele abgelehnte Druecke"
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        erwartet["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum fehlt"),
        "verschiedene Folge-Pruefsumme"
    );
    assert_eq!(
        u64::from(result.final_checksum),
        erwartet["finalChecksum"]
            .as_u64()
            .expect("finalChecksum fehlt"),
        "verschiedene Schluss-Pruefsumme"
    );
}

#[test]
fn beide_abbruch_arten_stimmen_auf_beiden_seiten() {
    // **Die Pruefsumme fuehrt diese Zaehler nicht.** Sie traegt den ZUSTAND
    // (`playerCasting`, `playerCastEndsAt`) — und ein Zustand, der geraeumt
    // wird, sieht gleich aus, egal WER ihn geraeumt hat. Zwei Umsetzungen, die
    // den Abbruch verschieden zuordnen, laufen in der Pruefsumme nie
    // auseinander.
    //
    // Deshalb stehen die zwei Zahlen hier zusaetzlich, aus der Datei gegen den
    // Simulator. Dieselbe Klasse Luecke wie bei den angenommenen Druecken, die
    // in #223 gemessen wurde: "Die Pruefsumme merkte davon nichts — sie fuehrt
    // die Zaehler nicht."
    let json = data();
    let result = replay(&json);
    let erwartet = &json["expected"];

    assert_eq!(
        u64::from(result.casts_interrupted),
        erwartet["castsInterrupted"]
            .as_u64()
            .expect("castsInterrupted fehlt"),
        "Spiel und Simulator zaehlen die Abbrueche DURCH DEN BOSS verschieden"
    );
    assert_eq!(
        u64::from(result.casts_cancelled),
        erwartet["castsCancelled"]
            .as_u64()
            .expect("castsCancelled fehlt"),
        "Spiel und Simulator zaehlen die Abbrueche DURCH DEN SPIELER verschieden"
    );
}

#[test]
fn die_zahl_der_wirkungen_stimmt_auf_beiden_seiten() {
    // **Die Pruefsumme fuehrt die Zaehler mit, aber nicht die Herleitung.**
    // Zwei Umsetzungen koennen bei gleicher Summe verschieden oft gewirkt
    // haben, solange sich die Unterschiede aufheben — deshalb steht die Zahl
    // hier zusaetzlich, aus der Datei gegen den Simulator.
    let json = data();
    let result = replay(&json);
    let index = index_of("test-cast").expect("test-cast fehlt im Simulator");
    assert_eq!(
        u64::from(result.casts[index]),
        json["expected"]["testCastCasts"]
            .as_u64()
            .expect("testCastCasts fehlt"),
        "Spiel und Simulator haben den Zauber verschieden oft AUFGELOEST"
    );
}

#[test]
fn der_boss_traegt_die_zwei_sorten_schlag_auch_im_simulator() {
    // Sonst pruefte alles oben einen Boss, bei dem entweder jeder Schlag
    // abbricht oder keiner — und der Unterschied, um den es in 7.1 geht, waere
    // gar nicht im Lauf. Dieselbe Falle wie ein Gegenfall am falschen Datensatz.
    let boss = boss_by_id("test-caster").expect("test-caster fehlt");
    let stoerend = boss.timeline.iter().filter(|e| e.interrupts_player).count();
    let harmlos = boss
        .timeline
        .iter()
        .filter(|e| !e.interrupts_player)
        .count();
    assert_eq!(stoerend, 2, "zwei Schlaege brechen ab");
    assert_eq!(harmlos, 1, "und einer stoert ausdruecklich nicht");
}
