//! Der Auswahlmodus im echten Kampf-Lauf (Issue #223).
//!
//! **Was dieser Prüffall von `cooldown_reset.rs` unterscheidet:** Dort werden
//! die REGELN des Modus geprüft — Abklingzeit auf null, Wartezeit auslösen,
//! verpuffen — an einem erfundenen Zauberkasten. Das ging, als kein
//! ausgelieferter Zauber ihn benutzte.
//!
//! Hier läuft eine echte Aufzeichnung mit REFLUX durch `Fight::replay`. Genau
//! dieser Pfad kannte den Modus nicht: Er konnte „wirken" und „ablehnen", nicht
//! „wählen". Eine Aufzeichnung mit REFLUX wäre still falsch durchgelaufen —
//! der Rücksetzer als gewöhnlicher Zauber gewirkt, der nächste Druck nicht
//! zurückgesetzt, und beide Seiten auseinander, ohne dass es jemand meldet.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const RESET_JSON: &str = include_str!("../../tests/parity/recordings/cooldown-reset-run.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(RESET_JSON).expect("cooldown-reset-run.json ist kein JSON");
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

fn replay(json: &Value) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
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
fn die_aufzeichnung_enthaelt_den_ruecksetzer_wirklich() {
    // **Ohne diesen Fall belegen die anderen nichts.** Eine Aufzeichnung ohne
    // REFLUX liefe auch durch den alten Druck-Pfad — und die Pruefsummen
    // stimmten, ohne dass der Auswahlmodus je angesprungen waere.
    let json = data();
    let reflux = index_of("reflux").expect("REFLUX gehoert seit #201 zum Zauberkasten");
    let reflux_slot = json["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .find(|b| index_of(b["spellId"].as_str().expect("spellId")) == Some(reflux))
        .map(|b| b["slot"].as_u64().expect("slot"))
        .expect("REFLUX liegt auf keinem Platz");

    let druecke = json["presses"]
        .as_array()
        .expect("presses")
        .iter()
        .filter(|p| p["slot"].as_u64() == Some(reflux_slot))
        .count();
    assert!(
        druecke >= 2,
        "die Aufzeichnung wirkt REFLUX nur {druecke}-mal — zu wenig, um den Modus zu pruefen"
    );
    assert_eq!(
        json["expected"]["resetsApplied"].as_u64(),
        Some(2),
        "im Spiel haben zwei Ruecksetzer gegriffen"
    );
}

#[test]
fn ac1_die_pruefsumme_stimmt_takt_fuer_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet = zahlen(&json, "checksums");
    let scharf = zahlen(&json, "armed");

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    for (tick, (links, rechts)) in result.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(
            links,
            rechts,
            "erste Abweichung im Takt {tick} ({} ms). Der Auswahlmodus stand dort \
             im Spiel {}",
            tick as u32 * TICK_MS,
            if scharf.get(tick) == Some(&1) {
                "SCHARF"
            } else {
                "nicht scharf"
            }
        );
    }

    // **Der Auswahlmodus selbst, Takt für Takt** (Issue #223, gemessen #480).
    //
    // Das Feld `armed` steht seit #223 in der Aufzeichnung und wurde bis
    // hierher **nur in der Fehlermeldung oben** verwendet — als Zusatz, wenn
    // eine Prüfsumme abwich. Verglichen hat es niemand.
    //
    // Das ist eine Lücke, und zwar eine mit Verzögerung: Der Auswahlmodus
    // steht in KEINER der beiden Prüfsummen. Er wirkt nur mittelbar, über eine
    // Abklingzeit, die er zurücksetzt — die steht darin. Liefen beide Seiten
    // im Zustand auseinander, fiele es erst auf, wenn er greift, und dann an
    // einer anderen Stelle als der, an der es entstand.
    //
    // **Warum sich `armed` (0 oder 1) mit einer Ladungszahl vergleichen
    // lässt**: REFLUX trägt genau EINE Auswahl. Solange das gilt, ist "scharf"
    // dasselbe wie "eine Ladung offen". Der Fall darunter hält das fest —
    // bekäme REFLUX je zwei, verglichen wir 2 gegen 1 und der Fall meldete es,
    // statt still falsch zu werden.
    //
    // **Die Zusicherung zuerst**: Steht die Spur überall auf 0, misst der
    // Vergleich eine Runde, in der der Rücksetzer nie scharf war — und wäre
    // grün. Gemessen sind es 300 von 6000 Takten.
    let ladungen_von_reflux = u32::from(
        rota_sim::data::SPELLS[index_of("reflux").expect("reflux")].cooldown_reset_charges,
    );
    assert_eq!(
        ladungen_von_reflux, 1,
        "REFLUX trägt {ladungen_von_reflux} Auswahlen statt einer — dann ist \
         `armed` (0 oder 1) nicht mehr dasselbe wie die Ladungszahl, und der \
         Vergleich darunter prüft zwei verschiedene Größen gegeneinander"
    );
    let scharfe_takte: u32 = scharf.iter().sum();
    assert!(
        scharfe_takte > 0,
        "der Rücksetzer stand in keinem einzigen Takt scharf — dann vergleicht \
         der Fall darunter überall dieselbe Null"
    );
    assert_eq!(
        result.reset_charges_left.len(),
        scharf.len(),
        "verschieden viele Takte in der Spur des Auswahlmodus"
    );
    for (tick, (links, rechts)) in result
        .reset_charges_left
        .iter()
        .zip(scharf.iter())
        .enumerate()
    {
        assert_eq!(
            links,
            rechts,
            "der Auswahlmodus geht im Takt {tick} ({} ms) auseinander: \
             Simulator {links}, Spiel {rechts}. Er steht in keiner Prüfsumme — \
             ohne diesen Vergleich fiele es erst auf, wenn er greift",
            tick as u32 * TICK_MS
        );
    }

    let expected = &json["expected"];
    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"].as_u64().expect("finalChecksum")
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        expected["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum")
    );
}

#[test]
fn ac1_auch_schaden_und_annahmen_stimmen_ueberein() {
    // Die Pruefsumme deckt beides ab — aber sie sagt nur "irgendwo anders".
    // Diese drei Zahlen sagen, WAS anders ist, und sie kosten nichts.
    let json = data();
    let result = replay(&json);
    let expected = &json["expected"];

    assert_eq!(
        u64::from(result.damage),
        expected["damage"].as_u64().expect("damage"),
        "verschiedener Schaden"
    );
    assert_eq!(
        u64::from(result.accepted),
        expected["accepted"].as_u64().expect("accepted"),
        "verschieden viele angenommene Druecke"
    );
    assert_eq!(
        u64::from(result.rejected),
        expected["rejected"].as_u64().expect("rejected"),
        "verschieden viele abgelehnte Druecke"
    );
}

/// Eine Aufzeichnung von Hand — fuer die Faelle, die eine Rotation nicht
/// zuverlaessig herstellt.
fn folge(druecke: &[(u32, u8)]) -> Recording {
    Recording {
        name: "handfolge".to_owned(),
        seed: 0,
        // 800 Takte = 8 s. Mein erster Entwurf nahm 400 — und der letzte
        // Druck lag bei Takt 450, also AUSSERHALB der Aufzeichnung. Die
        // Schleife erreichte ihn nie, und der Fall meldete "nur einmal
        // gewirkt", als waere die Wahl gescheitert. Sie war es nicht.
        ticks: 800,
        bindings: vec![
            Binding {
                slot: 0,
                spell: index_of("kindle").expect("kindle"),
            },
            Binding {
                slot: 1,
                spell: index_of("pyre").expect("pyre"),
            },
            Binding {
                slot: 2,
                spell: index_of("reflux").expect("reflux"),
            },
            // **TEMPER als zweites Ziel, nicht SMOULDER.** SMOULDER hat
            // 1500 ms Abklingzeit und ist zum Zeitpunkt der Wahl laengst
            // wieder bereit — die Wahl verpufft dann, und der Fall misst das
            // Gegenteil dessen, was er messen soll. Gemessen, nicht vermutet:
            // Mein erster Entwurf bekam casts[smoulder] == 1 statt 2.
            Binding {
                slot: 3,
                spell: index_of("temper").expect("temper"),
            },
        ],
        presses: druecke
            .iter()
            .map(|&(tick, slot)| Press { tick, slot })
            .collect(),
    }
}

fn lauf(recording: &Recording) -> rota_sim::ReplayResult {
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(recording, false)
}

#[test]
fn ac2_der_ruecksetzer_trifft_zwei_verschiedene_ziele() {
    // **Zwei Durchlaeufe mit verschiedenem Ziel.** Ein Prueffall, der den
    // Ruecksetzer immer vor denselben Zauber setzt, waere auch mit einem fest
    // verdrahteten Ziel gruen — er belegt dann gar nichts.
    //
    // Beide Ziele liegen auf Abklingzeit, wenn gewaehlt wird; nach der Wahl
    // muessen sie SOFORT wieder wirkbar sein, und das zeigt sich daran, dass
    // der naechste Druck durchgeht.
    for (name, ziel_slot, ziel_id) in [("PYRE", 1_u8, "pyre"), ("TEMPER", 3_u8, "temper")] {
        let ziel = index_of(ziel_id).unwrap_or_else(|| panic!("{ziel_id}"));
        // Ziel wirken -> REFLUX -> Ziel waehlen -> Ziel erneut wirken.
        // Die Zeitpunkte liegen auf den Wartezeit-Grenzen: Jede Aktion —
        // auch die WAHL — loest die volle Wartezeit aus (GAME-DESIGN 6.6).
        // Mein erster Entwurf setzte den letzten Druck auf Takt 350 und
        // bekam ihn abgelehnt; er lag noch in der Wartezeit der Wahl.
        let recording = folge(&[(0, ziel_slot), (150, 2), (300, ziel_slot), (450, ziel_slot)]);
        let result = lauf(&recording);

        assert_eq!(
            result.casts[ziel], 2,
            "{name}: ohne den Ruecksetzer waere der zweite Druck an der \
             Abklingzeit gescheitert"
        );
        assert_eq!(
            result.casts[index_of("reflux").expect("reflux")],
            1,
            "{name}: REFLUX selbst muss gewirkt worden sein"
        );
    }
}

#[test]
fn ac3_gezuendet_und_nichts_mehr_gewirkt_laesst_alles_stehen() {
    // Der Gegenfall: Der Modus steht scharf und wird nie benutzt. Nichts darf
    // springen — die Abklingzeit laeuft nur normal weiter.
    let pyre = index_of("pyre").expect("pyre");
    let ohne = lauf(&folge(&[(0, 1)]));
    let mit = lauf(&folge(&[(0, 1), (150, 2)]));

    // In beiden Laeufen wurde PYRE genau einmal gewirkt. Der Ruecksetzer
    // aendert daran nichts, solange niemand waehlt.
    assert_eq!(ohne.casts[pyre], 1);
    assert_eq!(mit.casts[pyre], 1);
    assert_eq!(
        mit.casts[index_of("reflux").expect("reflux")],
        1,
        "REFLUX selbst wurde gewirkt"
    );
}

#[test]
fn ac4_der_ruecksetzer_wird_gewirkt_und_waehlt_nicht_sich_selbst() {
    // **Die Reihenfolge-Regel, gespiegelt von der Spielseite** (Issue #201).
    // `arm()` steht NACH der Wirkung. Zoege jemand es davor, waere der Modus
    // schon beim eigenen Druck scharf — REFLUX wuerde als Auswahl verbraucht,
    // statt gewirkt zu werden.
    //
    // Am Ergebnis unterscheidbar, ohne in den Code zu sehen: Wird REFLUX
    // gewirkt, steht er in `casts`. Und der Druck DANACH muss noch waehlen
    // koennen — beides zusammen kann nur gelten, wenn die Reihenfolge stimmt.
    let reflux = index_of("reflux").expect("reflux");
    let pyre = index_of("pyre").expect("pyre");
    let result = lauf(&folge(&[(0, 1), (150, 2), (300, 1), (450, 1)]));

    assert_eq!(result.casts[reflux], 1, "REFLUX wurde gewirkt");
    assert_eq!(
        result.casts[pyre], 2,
        "und der Druck danach hat gewaehlt — sonst waere der zweite an der \
         Abklingzeit gescheitert"
    );
}
