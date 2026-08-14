//! Die SCHUTZ-Kanalisierung, Takt fuer Takt (Issue #323, GAME-DESIGN 20.5).
//!
//! WARUM ES DIESE DATEI GIBT
//! -------------------------
//! `parity_channel.rs` daneben deckt den SCHADENS-Kanal ab. Die Kanalisierung
//! zahlt aber dreierlei aus — Schaden, Heilung, Schutz — und der Schutz-Weg
//! stand in beiden Umsetzungen, aber in keiner Aufzeichnung.
//!
//! **Gemessen am 2026-08-14, und das ist der Grund fuer diese Datei:** Eine
//! Mutation im Schutz-Zweig des Simulators (`self.shield += anteil * 2`) liess
//! 165 von 165 Suiten gruen. Kein Prueffall der Rust-Seite fasste die Stelle
//! an; „beide Seiten bitgleich" hiess dort „auf keiner Seite gemessen".
//!
//! WAS DER LAUF ZEIGT
//! ------------------
//! Punkt 3 des Designs lautet: „Ein Schutz-Kanal legt EINE Schicht, deren
//! Betrag mit jedem Schritt steigt; bricht er ab, bleibt sie auf dem erreichten
//! Wert stehen." Alle drei Halbsaetze brauchen eine Stelle im Lauf:
//!
//! | Zeit | was dort steht |
//! | --- | --- |
//! | 1000..5000 ms | 200, 400, 600, 800, 1000 — der Betrag STEIGT je Schritt |
//! | 15500..17500 ms | 1136, 1336 — er waechst auf einer BESTEHENDEN Schicht weiter |
//! | 18000 ms | ein Schlag bricht ab, und der Wert bleibt bis zum Ende stehen |
//! | durchgehend | `shieldStacks` ist nie groesser als 1 |
//!
//! Die mittlere Zeile ist der Unterschied zur verworfenen Variante: Ein Kanal,
//! der je Schritt eine NEUE Schicht legte, waere bei `maxStacks: 1` nach dem
//! ersten Schritt am Anschlag gewesen und haette nichts mehr nachgelegt.
use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const SCHUTZ_JSON: &str = include_str!("../../tests/parity/recordings/channel-schutz.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(SCHUTZ_JSON).expect("channel-schutz.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn recording(json: &Value) -> Recording {
    // Die Belegung kommt aus der DATEI, nicht aus einer Abschrift hier — wie
    // in `parity_channel.rs`. Der Pruefzauber steht in keiner Leiste des
    // Spiels; eine zweite Liste hier liefe der ersten davon.
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
    // **Die Zusicherung, die diesem Pruefstand seinen Wert gibt.** Sie liest
    // nur die DATEI, ohne den Simulator zu fragen: Steht in dem Lauf gar keine
    // wachsende Schicht, dann ist alles Weitere gruen und prueft nichts.
    //
    // Sie ist nicht theoretisch. Der erste Entwurf dieser Aufzeichnung drueckte
    // den Kanal, sobald die Abklingzeit frei war — und traf damit zweimal genau
    // den Takt eines Boss-Schlags. Nach 5000 ms stieg die Schicht kein
    // einziges Mal mehr, und der Fall haette trotzdem gruen gemeldet.
    let json = data();
    let spur = zahlen(&json, "shieldTrace");
    let stapel = zahlen(&json, "shieldStacksTrace");

    assert_eq!(
        spur.len(),
        stapel.len(),
        "die zwei Spuren muessen gleich lang sein"
    );

    let anstiege = spur.windows(2).filter(|w| w[1] > w[0]).count();
    assert!(
        anstiege >= 7,
        "in dem Lauf stieg die Schicht nur {anstiege}-mal — fuer „waechst mit \
         jedem Schritt“ braucht es die fuenf des ersten Kanals UND mindestens \
         zwei auf einer bestehenden Schicht"
    );

    // **Und ein Anstieg auf einer BESTEHENDEN Schicht.** Ohne ihn zeigte der
    // Lauf nur einen Kanal von null an, und die verworfene Variante (je Schritt
    // eine neue Schicht) waere bei `maxStacks: 1` nicht zu unterscheiden.
    let auf_bestehender = spur.windows(2).filter(|w| w[1] > w[0] && w[0] > 0).count();
    assert!(
        auf_bestehender >= 2,
        "kein Schritt fiel auf eine schon stehende Schicht ({auf_bestehender}) \
         — genau daran scheitert die verworfene Variante, und der Lauf kaeme \
         gar nicht dort vorbei"
    );

    // **EINE Schicht, nie mehr.** Der Kern von Punkt 3.
    let hoechster = stapel.iter().copied().max().unwrap_or(0);
    assert_eq!(
        hoechster, 1,
        "der Schutz-Kanal legte {hoechster} Schichten statt einer"
    );

    // **Der Abbruch, und dass der Wert danach STEHEN bleibt.** Ohne ihn waere
    // der dritte Halbsatz des Designs unbelegt.
    assert_eq!(
        json["expected"]["castsInterrupted"].as_u64(),
        Some(1),
        "gemessen: ein Abbruch durch den Boss, bei 18000 ms"
    );
    let letzter = *spur.last().expect("leere Spur");
    assert!(
        letzter > 1000,
        "am Ende des Laufs stand nur noch {letzter} — dann sagt der Lauf \
         nichts darueber, ob die Schicht nach dem Abbruch stehen blieb"
    );
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    // Hier faellt die Mutation: `shield` und `shieldStacks` gehen beide in die
    // Pruefsumme ein (`checksum.ts`), also schlaegt jede Abweichung im Betrag
    // oder in der Stapelzahl in dem Takt durch, in dem sie entsteht.
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
        let spur = zahlen(&json, "shieldTrace");
        let stapel = zahlen(&json, "shieldStacksTrace");
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}\n  \
             Schicht dort: Betrag {:?}, Stapel {:?}",
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
            spur.get(d.tick as usize),
            stapel.get(d.tick as usize),
        );
    }
}

#[test]
fn der_abgefangene_schaden_stimmt_auf_beiden_seiten() {
    // **Die Pruefsumme fuehrt diese Zahl nicht.** Sie traegt den ZUSTAND der
    // Schicht, nicht die Summe dessen, was sie ueber den Lauf abgefangen hat —
    // dieselbe Klasse Luecke wie bei den Abbruch-Zaehlern in
    // `parity_channel.rs`. Zwei Umsetzungen koennten den Abtrag verschieden
    // verteilen und im Zustand trotzdem gleich aussehen.
    let json = data();
    let result = replay(&json);
    let im_spiel = json["expected"]["absorbed"]
        .as_u64()
        .expect("absorbed fehlt");

    assert!(
        im_spiel > 0,
        "in dem Lauf fing die Schicht gar nichts ab — dann vergleicht die \
         Zeile darunter zwei Nullen"
    );
    assert_eq!(
        u64::from(result.shield_absorbed),
        im_spiel,
        "Spiel und Simulator fangen verschieden viel Schaden ab"
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
        u64::from(result.casts_interrupted),
        erwartet["castsInterrupted"]
            .as_u64()
            .expect("castsInterrupted fehlt"),
        "Spiel und Simulator zaehlen die Abbrueche DURCH DEN BOSS verschieden"
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
    // Wie in `parity_channel.rs`: Die Pruefsumme fuehrt die Zahl je Zauber
    // nicht, und zwei Umsetzungen koennen bei gleichem Zustand verschieden oft
    // begonnen haben.
    let json = data();
    let result = replay(&json);
    let index = index_of("test-channel-shield").expect("test-channel-shield fehlt im Simulator");
    let im_spiel = json["expected"]["casts"]["test-channel-shield"]
        .as_u64()
        .unwrap_or(0);

    assert!(
        im_spiel > 1,
        "die Aufzeichnung wirkt den Schutz-Kanal {im_spiel}-mal — unter zwei \
         Wuerfen faellt der Fall weg, in dem er auf eine bestehende Schicht \
         nachlegt"
    );
    assert_eq!(
        u64::from(result.casts[index]),
        im_spiel,
        "Spiel und Simulator zaehlen den Schutz-Kanal verschieden oft"
    );
}
