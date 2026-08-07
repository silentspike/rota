//! Der Vorrat ueber dem gefallenen Deckel — beide Seiten gleich (Issue #549).
//!
//! # Warum es diese Datei geben MUSS
//!
//! Vor #549 waere sie nicht durchgelaufen, sondern ABGESTUERZT: Steht der
//! Vorrat ueber dem Deckel, rechnete `add_resource` `self.resource - before`
//! mit einem groesseren `before` — u16-Subtraktion.
//!
//! Keine der 33 Aufzeichnungen fuhr diese Folge, obwohl sie im Spiel vier
//! Tastendruecke braucht: SLAG, dreimal BELLOWS, Fenster ablaufen lassen,
//! weiter erzeugen. Deshalb fiel der Fehler erst beim gezielten Bau auf.
//!
//! # Was hier geprueft wird, das kein Unit-Fall zeigt
//!
//! Die Prueffaelle in `slag_wirkung.rs` und `slag.test.ts` belegen die REGEL
//! auf je einer Seite. Diese Datei belegt, dass beide sie GLEICH rechnen —
//! Takt fuer Takt, samt Ueberlauf-Zaehler, der in die Pruefsumme eingeht.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/deckel-gefallen.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("deckel-gefallen.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn zahlen(json: &Value, feld: &str) -> Vec<u32> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect()
}

fn nachspielen(json: &Value) -> (Fight, rota_sim::ReplayResult) {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| Binding {
            slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
            spell: index_of(b["spellId"].as_str().expect("spellId")).expect("Zauber unbekannt"),
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    let recording = Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: 0,
        ticks,
        bindings,
        presses,
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, true);
    (fight, ergebnis)
}

#[test]
fn ac4_der_zustand_wird_ueberhaupt_erreicht() {
    // **Die Ursache zuerst.** Bleibt der Vorrat unter dem Deckel, faehrt die
    // Aufzeichnung einen gewoehnlichen Lauf und belegt nichts.
    let json = data();
    let hoechster = json["expected"]["hoechsterVorrat"]
        .as_u64()
        .expect("hoechsterVorrat");
    assert!(
        hoechster > 20,
        "der hoechste Vorrat ist {hoechster} und damit nicht ueber dem \
         Grund-Deckel — dann faehrt diese Aufzeichnung den Zustand nicht, um \
         den es geht"
    );
}

#[test]
fn ac4_die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    // Der Ueberlauf-Zaehler steckt in der Pruefsumme (`checksum.ts`). Ein
    // Sprung um 12 statt um 8 faellt hier also auf, ohne dass ein eigener
    // Vergleich noetig waere.
    let json = data();
    let (_, ergebnis) = nachspielen(&json);
    let soll = zahlen(&json, "checksums");

    assert!(!soll.is_empty());
    if let Some(d) = first_divergence(&soll, &ergebnis.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} ({} ms)\n  \
             Spiel: {}   Simulator: {}\n  gleiche Takte davor: {}",
            d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
        );
    }
    assert_eq!(ergebnis.checksums.len(), soll.len());
}

/// Der Vorrat sinkt in der Spur NIE durch eine Erzeugung.
///
/// # Warum die Pruefsumme das nicht schon erledigt
///
/// Sie zeigt Gleichlauf, nicht die REGEL. Waeren beide Seiten gleich falsch —
/// beide mit `min` statt `max(before, min(...))` —, liefe sie durch. Hier wird
/// die Spur selbst gelesen: An jedem Takt, an dem gedrueckt wurde, darf der
/// Vorrat nicht kleiner geworden sein.
#[test]
fn ac5_kein_wurf_senkt_den_vorrat() {
    let json = data();
    let vorrat = zahlen(&json, "vorrat");
    let presses: Vec<u32> = json["presses"]
        .as_array()
        .expect("presses")
        .iter()
        .map(|p| u32::try_from(p["tick"].as_u64().expect("tick")).expect("passt"))
        .collect();
    assert!(!presses.is_empty(), "ohne Druecke belegt der Fall nichts");

    for tick in presses {
        let i = usize::try_from(tick).expect("passt");
        let (Some(&davor), Some(&danach)) = (vorrat.get(i.saturating_sub(1)), vorrat.get(i)) else {
            continue;
        };
        assert!(
            danach >= davor,
            "Takt {tick}: der Vorrat fiel von {davor} auf {danach} — ein \
             Erzeuger hat weggenommen. Das ist der Fall aus #549, und er \
             tritt auf, sobald der Deckel unter den stehenden Vorrat faellt"
        );
    }
}

/// Der Ueberlauf-Zaehler zaehlt nur, was von der ERZEUGUNG nicht ankam.
///
/// Vor #549 sprang er um 12, wo 8 erzeugt wurden: Die vier vom fallenden
/// Deckel gekappten Punkte zaehlten mit, obwohl sie bereits angekommen waren.
#[test]
fn ac5_der_ueberlauf_zaehlt_nur_die_erzeugung() {
    let json = data();
    let ueberlauf = zahlen(&json, "ueberlauf");
    let vorrat = zahlen(&json, "vorrat");

    for i in 1..ueberlauf.len() {
        let sprung = ueberlauf[i].saturating_sub(ueberlauf[i - 1]);
        if sprung == 0 {
            continue;
        }
        // BELLOWS gibt 8, KINDLE 3 — mehr als 8 kann eine einzelne Erzeugung
        // nicht verpuffen lassen. Ein groesserer Sprung heisst, dass gekappte
        // Punkte mitgezaehlt wurden.
        assert!(
            sprung <= 8,
            "Takt {i}: der Ueberlauf sprang um {sprung}, mehr als der groesste \
             Erzeuger (BELLOWS, 8) hergibt. Vorrat davor {}, danach {}",
            vorrat[i - 1],
            vorrat[i]
        );
    }
}
