//! Der Ablehnungspfad: eine Runde, in der **falsch gespielt wird** (Issue #95).
//!
//! Die Prüffälle aus #54 belegen, dass Spiel und Simulator dieselbe Zahl
//! rechnen, wenn der Spieler alles richtig macht. Sie belegen **nicht**, dass
//! beide auch dann übereinstimmen, wenn er etwas falsch macht — und das ist die
//! größere Hälfte. Ein Kampf besteht aus Fehlgriffen, verpassten Fenstern und zu
//! früh gedrückten Tasten.
//!
//! Vier Fehlertypen sind in der Aufzeichnung, jeder mindestens einmal:
//!
//! | Fehler | Erwartung an beide Seiten |
//! | --- | --- |
//! | Taste während der globalen Wartezeit | abgelehnt, **nichts gemerkt** |
//! | Zauber auf Abklingzeit | abgelehnt |
//! | Brand-Effekt zu früh erneuert | angenommen, Restlaufzeit verloren |
//! | Leerlauf ohne Eingabe | der GCD läuft leer |
//!
//! Der erste ist der heikelste: Das Spiel merkt sich einen abgelehnten Druck
//! ausdrücklich **nicht** (GAME-DESIGN 21.1 — die Eingabe-Vormerkung wurde
//! verworfen, weil sie dem Spieler Präzisionsleistung abnimmt). Ein Simulator,
//! der ihn stattdessen nachholte, käme auf mehr Wirkungen und mehr Schaden.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Die aufgezeichnete Runde mit Fehlgriffen, wie das Spiel sie geschrieben hat.
const MISPLAYS_JSON: &str = include_str!("../../tests/parity/recordings/misplays.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(MISPLAYS_JSON).expect("misplays.json ist kein JSON");
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

fn replay(json: &Value, with_checksums: bool) -> (Recording, rota_sim::ReplayResult) {
    let recording = recording(json);
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, with_checksums);
    (recording, result)
}

#[test]
fn ac1_die_aufzeichnung_enthaelt_jeden_der_vier_fehlertypen() {
    let json = data();
    let expected = &json["expected"];
    let reasons = &expected["rejectedByReason"];

    // 1. Taste waehrend der globalen Wartezeit
    let auf_gcd = reasons["onGcd"].as_u64().expect("onGcd fehlt");
    assert!(auf_gcd > 0, "kein Druck waehrend der Wartezeit");

    // 2. Zauber auf Abklingzeit
    let auf_cooldown = reasons["onCooldown"].as_u64().expect("onCooldown fehlt");
    assert!(
        auf_cooldown > 0,
        "kein Druck auf einen Zauber in Abklingzeit"
    );

    // 3. Brand-Effekt zu frueh erneuert
    let verlorene_ticks = expected["lostDotTicks"]
        .as_u64()
        .expect("lostDotTicks fehlt");
    assert!(verlorene_ticks > 0, "kein zu frueh erneuerter Brand-Effekt");

    // 4. Leerlauf
    let leerlauf = expected["idleTicks"].as_u64().expect("idleTicks fehlt");
    assert!(leerlauf > 0, "kein Fenster ohne Eingabe");

    // Und eine angenommene Wirkung muss es auch geben — sonst prueft die Runde
    // nur Ablehnungen und keinen Kampf.
    assert!(reasons["ok"].as_u64().expect("ok fehlt") > 0);
}

#[test]
fn ac2_spiel_und_simulator_ergeben_denselben_endschaden() {
    let json = data();
    let (recording, result) = replay(&json, false);
    let expected = &json["expected"];

    assert_eq!(
        u64::from(result.damage),
        expected["damage"].as_u64().expect("damage fehlt"),
        "der Simulator rechnet einen anderen Schaden als das Spiel"
    );
    assert_eq!(
        u64::from(result.accepted),
        expected["accepted"].as_u64().expect("accepted fehlt"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert_eq!(
        u64::from(result.rejected),
        expected["rejected"].as_u64().expect("rejected fehlt"),
        "andere Zahl abgelehnter Tastendruecke"
    );
    assert!(
        result.rejected > result.accepted,
        "diese Runde soll ueberwiegend aus Fehlgriffen bestehen: \
         {} angenommen, {} abgelehnt",
        result.accepted,
        result.rejected
    );
    assert!(!recording.presses.is_empty());
}

#[test]
fn ac2_ein_abgelehnter_druck_wird_nicht_nachgeholt() {
    // **Der Kern des Ablehnungspfads.** GAME-DESIGN 21.1 hat die
    // Eingabe-Vormerkung ausdruecklich verworfen: Ein Druck zur falschen Zeit
    // ist verloren, nicht aufgeschoben. Ein Simulator, der ihn nachholte, kaeme
    // auf mehr Wirkungen — und die Zahl waere trotzdem plausibel.
    let json = data();
    let (_, result) = replay(&json, false);
    let expected = &json["expected"];

    let casts: serde_json::Map<String, Value> =
        serde_json::from_value(expected["casts"].clone()).expect("casts fehlen");
    for (spell_id, count) in casts {
        let index = index_of(&spell_id).unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt"));
        assert_eq!(
            u64::from(result.casts[index]),
            count.as_u64().expect("Zahl"),
            "{spell_id} wurde anders oft gewirkt — ein abgelehnter Druck wurde \
             vermutlich nachgeholt statt verworfen"
        );
    }

    // Die Gegenprobe zur Zahl selbst: Deutlich mehr Druecke als Wirkungen.
    let wirkungen: u32 = result.casts.iter().sum();
    assert_eq!(
        wirkungen, result.accepted,
        "jede Wirkung braucht einen Druck"
    );
}

#[test]
fn ac3_die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let (_, result) = replay(&json, true);
    let expected: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect();

    assert!(!expected.is_empty());

    if let Some(d) = first_divergence(&expected, &result.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             uebereinstimmende Takte davor: {}",
            d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
        );
    }
    assert_eq!(result.checksums.len(), expected.len());

    // Und die beiden zusammenfassenden Summen.
    assert_eq!(
        u64::from(result.sequence_checksum),
        json["expected"]["sequenceChecksum"]
            .as_u64()
            .expect("fehlt")
    );
    assert_eq!(
        u64::from(result.final_checksum),
        json["expected"]["finalChecksum"].as_u64().expect("fehlt")
    );
}

#[test]
fn ac4_eine_verfaelschte_erwartung_laesst_den_vergleich_fehlschlagen() {
    // **Ohne diesen Test behauptet der Prueffall eine Pruefung, die nicht
    // stattfindet.** Worker-1 hat in #87 einen gefunden, der vor und nach der
    // Aenderung gruen blieb.
    //
    // Bei einer Vergleichsdatei ist die Gefahr besonders gross: Wenn beide
    // Seiten dieselbe Datei erzeugen und dagegen pruefen, kann der Vergleich
    // trivial wahr sein. Hier wird deshalb jede der drei Erwartungen einzeln
    // verfaelscht und geprueft, dass der Vergleich es merkt.
    let json = data();
    let (_, result) = replay(&json, true);

    // 1. Verfaelschter Schaden
    let echter_schaden = json["expected"]["damage"].as_u64().expect("damage");
    assert_ne!(
        u64::from(result.damage),
        echter_schaden + 1,
        "ein um eins verschobener Schaden darf nicht mehr uebereinstimmen"
    );

    // 2. Verfaelschte Pruefsummen-Folge — an drei Stellen einzeln.
    for stelle in [0, result.checksums.len() / 2, result.checksums.len() - 1] {
        let mut verfaelscht = result.checksums.clone();
        verfaelscht[stelle] ^= 1;
        let d = first_divergence(&result.checksums, &verfaelscht, TICK_MS)
            .unwrap_or_else(|| panic!("Verfaelschung bei Takt {stelle} wurde nicht gefunden"));
        assert_eq!(
            d.tick as usize, stelle,
            "der gemeldete Takt stimmt nicht mit der Verfaelschung ueberein"
        );
    }

    // 3. Verkuerzte Folge — eine Seite bricht frueher ab.
    let gekuerzt = &result.checksums[..result.checksums.len() - 1];
    assert!(
        first_divergence(&result.checksums, gekuerzt, TICK_MS).is_some(),
        "eine kuerzere Folge muss auffallen"
    );

    // 4. Und die Gegenprobe zur Gegenprobe: unveraendert faellt nichts auf.
    assert!(
        first_divergence(&result.checksums, &result.checksums, TICK_MS).is_none(),
        "der Vergleich meldet eine Abweichung, wo keine ist"
    );
}

#[test]
fn die_runde_unterscheidet_sich_wirklich_von_der_fehlerfreien() {
    // Sonst pruefte diese Datei dasselbe wie cooldown-precision, nur unter
    // anderem Namen.
    let json = data();
    let (_, result) = replay(&json, false);

    assert_ne!(
        result.damage, 4240,
        "diese Runde darf nicht zufaellig dasselbe ergeben wie die fehlerfreie"
    );
    assert!(
        result.damage < 4240,
        "wer Fehler macht, muss weniger Schaden anrichten: {} gegen 4240",
        result.damage
    );
    assert_eq!(
        result.killed_at_ms, None,
        "mit so vielen Fehlgriffen faellt der Boss nicht"
    );
}
