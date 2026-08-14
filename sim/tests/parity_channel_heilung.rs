//! Die HEIL-Kanalisierung, Takt fuer Takt (Issue #323, Nachtrag zu #843).
//!
//! WARUM ES DIESE DATEI GIBT
//! -------------------------
//! Die Kanalisierung zahlt dreierlei aus — Schaden, Heilung, Schutz. Fuer den
//! Schaden gibt es `parity_channel.rs`, fuer den Schutz seit #858
//! `parity_channel_schutz.rs`. Der HEIL-Weg stand seit #843 in beiden
//! Umsetzungen und in keiner Aufzeichnung.
//!
//! WARUM ER NICHT IN DEN SCHUTZ-LAUF PASSTE
//! ----------------------------------------
//! Die zwei Kanaele stehen in Konkurrenz: Der Schutz faengt genau den Schaden
//! ab, den die Heilung heilen muesste, und Ueberheilung zaehlt nicht. Gemessen
//! am 2026-08-14 ueber 24 s:
//!
//! | Lauf | genommen | geheilt |
//! | --- | --- | --- |
//! | `boss-2-3` OHNE Schutz-Kanal | 1040 | 1020 |
//! | `boss-2-3` MIT Schutz-Kanal | 40 | 0 |
//! | `boss-4-4` MIT Schutz-Kanal | 0 | 0 |
//!
//! Eine gemeinsame Aufzeichnung enthielte zwei Kanaele und pruefte einen.
//!
//! DER ABBRUCH KOMMT HIER VON HAND — und das ist Absicht
//! -----------------------------------------------------
//! `boss-2-3` traegt kein `interruptsPlayer`; in diesem Lauf steht
//! `castsInterrupted` auf 0 und `castsCancelled` auf 1. Damit ist er zugleich
//! der Gegenbeleg zu einer Behauptung, die bis #858 in `recording.ts` stand:
//! Ohne einen Boss mit `interruptsPlayer` gaebe es bei einer Kanalisierung
//! nichts zu zeigen. Der Spieler bricht selbst ab.
use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const HEILUNG_JSON: &str = include_str!("../../tests/parity/recordings/channel-heilung.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(HEILUNG_JSON).expect("channel-heilung.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn recording(json: &Value) -> Recording {
    // Die Belegung kommt aus der DATEI, nicht aus einer Abschrift hier.
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
fn der_lauf_kann_den_unterschied_ueberhaupt_zeigen() {
    // **Die Zusicherung, die diesem Prueistand seinen Wert gibt.** Sie liest
    // nur die DATEI. Ein Heil-Kanal, der in volle Leben hineinheilt, zahlt
    // NICHTS aus — genau daran ist der erste Entwurf zu #858 gescheitert, wo
    // `healed` bei 0 stand, obwohl der Kanal dreimal lief.
    let json = data();
    let spur = zahlen(&json, "healedTrace");
    let hp = zahlen(&json, "hpTrace");
    assert_eq!(
        spur.len(),
        hp.len(),
        "die zwei Spuren muessen gleich lang sein"
    );

    let geheilt = json["expected"]["healed"].as_u64().expect("healed fehlt");
    assert!(
        geheilt > 500,
        "in dem Lauf wurden nur {geheilt} geheilt — bei `test-caster` waeren es \
         96 gewesen, und eine Mutation im Heil-Zweig verschoebe dort kaum etwas"
    );

    // **Die Heilung faellt SCHRITTWEISE**, nicht in einem Betrag am Ende. Ohne
    // diese Zeile bliebe der Fall auch dann gruen, wenn eine Seite die ganze
    // Summe beim Abschluss auszahlte — und genau das ist der Unterschied
    // zwischen Kanalisierung und Cast (GAME-DESIGN 7.2).
    let schritte = spur.windows(2).filter(|w| w[1] > w[0]).count();
    assert!(
        schritte >= 5,
        "die Heilung fiel in nur {schritte} Schritten — dann ist der Lauf von \
         einem Cast, der am Ende alles auf einmal gibt, nicht zu unterscheiden"
    );

    // **Der Abbruch VON HAND**, und dass danach nichts mehr nachkommt.
    assert_eq!(
        json["expected"]["castsCancelled"].as_u64(),
        Some(1),
        "gemessen: ein Abbruch durch den Spieler selbst, bei 17000 ms"
    );
    assert_eq!(
        json["expected"]["castsInterrupted"].as_u64(),
        Some(0),
        "`boss-2-3` traegt kein `interruptsPlayer` — steht hier etwas anderes, \
         ist es nicht mehr der Gegenbeleg zur Behauptung aus #858"
    );
    let letzter = *spur.last().expect("leere Spur");
    assert_eq!(
        letzter,
        i64::try_from(geheilt).expect("passt"),
        "die Spur endet woanders als die Summe"
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
        let spur = zahlen(&json, "healedTrace");
        let hp = zahlen(&json, "hpTrace");
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             dort: geheilt {:?}, Leben {:?}",
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
            spur.get(d.tick as usize),
            hp.get(d.tick as usize),
        );
    }
}

#[test]
fn die_geheilte_summe_stimmt_auf_beiden_seiten() {
    // **Die Pruefsumme fuehrt diese Zahl nicht** (#576) — sie traegt den
    // ZUSTAND (`playerHp`), nicht die Summe des Geheilten. Zwei Umsetzungen
    // koennten verschieden viel heilen und verschieden viel ueberheilen und im
    // Zustand trotzdem gleich aussehen.
    let json = data();
    let result = replay(&json);
    let im_spiel = json["expected"]["healed"].as_u64().expect("healed fehlt");

    assert!(
        im_spiel > 0,
        "in dem Lauf wurde gar nicht geheilt — dann vergleicht die Zeile \
         darunter zwei Nullen"
    );
    assert_eq!(
        u64::from(result.healed),
        im_spiel,
        "Spiel und Simulator heilen verschieden viel"
    );

    // Und der eingehende Schaden daneben: Ohne ihn faende die Heilung nichts
    // zu tun, und die Zeile oben pruefte einen Lauf, der gar nicht der
    // gemessene ist.
    assert_eq!(
        u64::from(result.damage_taken),
        json["expected"]["damageTaken"]
            .as_u64()
            .expect("damageTaken fehlt"),
        "Spiel und Simulator nehmen verschieden viel Schaden"
    );
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
        u64::from(result.casts_cancelled),
        erwartet["castsCancelled"]
            .as_u64()
            .expect("castsCancelled fehlt"),
        "Spiel und Simulator zaehlen die Abbrueche DURCH DEN SPIELER verschieden"
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
fn die_zahl_der_kanalisierungen_stimmt_auf_beiden_seiten() {
    let json = data();
    let result = replay(&json);
    let index = index_of("test-channel-heal").expect("test-channel-heal fehlt im Simulator");
    let im_spiel = json["expected"]["casts"]["test-channel-heal"]
        .as_u64()
        .unwrap_or(0);

    assert!(
        im_spiel > 1,
        "die Aufzeichnung wirkt den Heil-Kanal {im_spiel}-mal — unter zwei \
         Wuerfen faellt der abgebrochene dritte weg"
    );
    assert_eq!(
        u64::from(result.casts[index]),
        im_spiel,
        "Spiel und Simulator zaehlen den Heil-Kanal verschieden oft"
    );
}
