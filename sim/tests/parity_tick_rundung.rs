//! Der Tick-Abstand unter Verstaerkung UND Tempo (Issue #640).
//!
//! # Die einzige Zielgroesse, deren Faktor TEILT — und die einzige ohne Aufzeichnung
//!
//! Gemessen am 2026-08-07 ueber alle Aufzeichnungen: Die sieben anderen
//! Buff-Ziele werden von `test-buff`, GRIPLOCK, FLASHPOINT, VIRULENCE, DISTIL,
//! QUICKSILVER und SLAG gefahren. `test-tick` ist alleiniger Traeger seines
//! Ziels und wurde in KEINER Aufzeichnung gedrueckt.
//!
//! Bei Punktegewinn, Hitze-Aufbau und Stapel-Wert multipliziert der Faktor; bei
//! `tick` teilt er den ABSTAND. `combat.ts` warnt dort ausdruecklich: "Wer die
//! Zeile nach dem Muster der anderen drei liest, dreht die Wirkung um."
//!
//! Vor diesem Fall hatte jede Seite ihren eigenen Waechter
//! (`tests/unit/buff-targets.test.ts` und `sim/tests/buff_wirkung.rs`), und
//! **beide pruefen die RICHTUNG** — `assert!(mit > ohne)`. Eine Richtung hat
//! zwei Werte, eine Rundung hat viele. Verglichen wurden die Zahlen nie.
//!
//! # Warum diese Aufzeichnung Ausruestung traegt
//!
//! `dot_interval` verrechnet drei Dinge: den Buff-Faktor, eine Rundung und die
//! Tempo-Rechnung. Ohne Tempo gibt letztere ihren Wert unveraendert zurueck —
//! dann waere die REIHENFOLGE der Schritte folgenlos und ein Lauf ohne
//! Ausruestung koennte sie nicht pruefen. Mit `catalysis: 170` trennt sie sich.
//!
//! # Warum der Faktor krumm sein MUSS, und warum zwei DoTs
//!
//! Ein erster Aufbau lief mit `test-tick` bei Faktor 2.0 und nur SMOULDER. Er
//! war gruen und fing die Umkehrung — aber **keine** der beiden feineren
//! Gegenproben, und zwar aus zwei verschiedenen Gruenden:
//!
//! `2000 / 2.0` geht exakt auf. `round` und `floor` liefern beide 1000, jede
//! denkbare Rechenweise kommt auf denselben Wert. Der Faktor traegt deshalb
//! jetzt 1.4 (Begruendung in `data/spells/test-buff.json`). Gemessen ueber 17
//! Kandidaten mit der echten Tempo-Rechnung bei `catalysis: 170`:
//!
//! ```text
//!            Intervall  Bestand  Rundung  Reihenfolge
//!   SMOULDER      2000     1230     1220         1221
//!   CINDER        1000      610      610          614
//! ```
//!
//! Bei SMOULDER trennt die RUNDUNG, bei CINDER nicht — `round(1000/1.4)` und
//! `floor(1000/1.4)` sind beide 714. Dafuer laesst CINDERs kurzes Intervall
//! MEHRERE verstaerkte Ticks ins Fenster fallen, und erst dadurch trennt die
//! REIHENFOLGE: Bei nur einem Tick fielen 1230 und 1221 auf denselben
//! 10-ms-Takt, und der Fall war gruen, ohne etwas unterschieden zu haben.
//!
//! Beide DoTs zusammen decken beide Gegenproben ab. Jede fuer sich nicht.
//!
//! # Die drei Mutationen, die dieser Fall faengt
//!
//! ```text
//!   teilen -> multiplizieren        die Umkehrung, vor der combat.ts warnt
//!   round  -> floor                 die Rundung
//!   Tempo vor statt nach der Teilung  die Reihenfolge
//! ```
//!
//! Alle drei faerben `die_pruefsumme_stimmt_in_jedem_takt`. Die Schluss-Summe
//! allein sieht sie nicht immer: Am Ende sind die Fenster zu und die Werte
//! gleichen sich wieder an.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::gear::WornValues;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/tick-rundung.json");

fn worn(json: &Value) -> WornValues {
    let node = &json["worn"];
    let feld = |name: &str| node[name].as_u64().expect("worn-Feld fehlt") as u32;
    WornValues {
        catalysis: feld("catalysis"),
        purity: feld("purity"),
        yield_bonus: feld("yieldBonus"),
        essence: feld("essence"),
        vessel: feld("vessel"),
        ward: feld("ward"),
        gear_effects: feld("gearEffects"),
    }
}

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("tick-rundung.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

fn u32_at(node: &Value, key: &str) -> u32 {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    u32::try_from(raw).expect("passt nicht in 32 Bit")
}

fn recording(json: &Value) -> Recording {
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt"))
                    .expect("slot > 255"),
                spell: index_of(spell_id)
                    .unwrap_or_else(|| panic!("Zauber {spell_id} kennt der Simulator nicht")),
            }
        })
        .collect();

    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick fehlt"))
                .expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name fehlt").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed fehlt")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks fehlen"))
            .expect("ticks zu gross"),
        bindings,
        presses,
    }
}

/// **Die Zusicherung vor jeder Messung.**
///
/// `test-tick` und SMOULDER muessen beide gewirkt werden — der Verstaerker
/// allein bewegt nichts, und der DoT allein liefe ungebufft. Ohne diese Zeile
/// waere der Vergleich unten gruen, wenn einer der beiden fehlte.
#[test]
fn verstaerker_und_dot_werden_beide_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    for id in ["test-tick", "smoulder"] {
        let zauber = index_of(id).unwrap_or_else(|| panic!("{id} fehlt im Simulator"));
        let slot = aufzeichnung
            .bindings
            .iter()
            .find(|b| b.spell == zauber)
            .map(|b| b.slot)
            .unwrap_or_else(|| panic!("{id} liegt nicht auf der Leiste"));
        let druecke = aufzeichnung
            .presses
            .iter()
            .filter(|p| p.slot == slot)
            .count();
        assert!(
            druecke > 0,
            "{id} liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
             der Tick-Abstand unter Verstaerkung weiterhin nur angenommen"
        );
        assert_eq!(
            u64::try_from(druecke).expect("passt"),
            json["expected"]["casts"][id]
                .as_u64()
                .unwrap_or_else(|| panic!("casts.{id} fehlt")),
            "Simulator und Spiel zaehlen verschieden viele {id}-Wirkungen"
        );
    }
}

/// **Und das Tempo ist wirklich im Spiel.**
///
/// Ohne `catalysis > 0` gibt die Tempo-Rechnung ihren Wert unveraendert
/// zurueck. Der Fall liefe dann durch, ohne die Reihenfolge der Rechenschritte
/// je zu beruehren — dieselbe Falle, die `worn-values` (#177, AC8) beschreibt.
#[test]
fn die_aufzeichnung_traegt_wirklich_tempo() {
    let json = data();
    assert!(
        worn(&json).catalysis > 0,
        "die Aufzeichnung laeuft ohne Tempo — dann ist die Reihenfolge von \
         Teilung und Tempo-Rechnung folgenlos und dieser Fall belegt sie nicht"
    );
}

/// AC2 — die ZAHL, nicht die Richtung.
#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        worn: worn(&json),
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);
    let erwartet = &json["expected"];

    // Der DoT-Beitrag steckt in dieser Summe: `tickDamage` mal Tickzahl, und
    // die Tickzahl ist die Dauer geteilt durch den ABSTAND. Dreht jemand die
    // Rechenrichtung um, wandert sie — und ebenso bei einer anderen Rundung
    // oder vertauschter Reihenfolge. Die Zahl steht in `expected.damage` der
    // Aufzeichnung und wird hier nicht wiederholt, damit sie nicht an zwei
    // Orten altert.
    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
    );
    assert_eq!(
        ergebnis.accepted,
        u32_at(erwartet, "accepted"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert_eq!(
        ergebnis.final_checksum,
        u32_at(erwartet, "finalChecksum"),
        "die Schluss-Pruefsumme weicht ab"
    );
}

/// Takt fuer Takt — hier stehen die einzelnen Tick-Zeitpunkte.
///
/// Die Schluss-Summe allein koennte uebereinstimmen, waehrend die Ticks
/// unterwegs anders fielen. Erst der Takt-Vergleich nagelt die ABSTAENDE fest.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        worn: worn(&json),
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);

    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();

    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
