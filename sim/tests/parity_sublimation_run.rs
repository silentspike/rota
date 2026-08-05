//! Die Sublimation rechnet auf beiden Seiten gleich (Issue #104).
//!
//! # Warum es diese Datei erst jetzt gibt
//!
//! AC5 von #78 lautete „die Mechanik rechnet auf beiden Seiten gleich" und war
//! damals nicht baubar: Kein ausgelieferter Zauber trug eine Verkuerzung, ein
//! Prueffall haette zwei Nullen verglichen. Worker-1 hat das gemeldet, statt
//! den AC trivial gruen zu setzen. Seit #201 traegt VAPOUR sie.
//!
//! # Was die Pruefsumme je Takt hier leistet
//!
//! Eine Verkuerzung verschiebt ZEITPUNKTE, nicht Betraege. Ein Vergleich nur
//! am Ende saehe denselben Gesamtschaden auch dann, wenn beide Seiten die
//! Zauber zu verschiedenen Zeiten wirken — solange am Schluss gleich viele
//! durchgelaufen sind. Erst die Summe je Takt haelt fest, dass sie es
//! gleichzeitig tun.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/sublimation.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("sublimation.json ist kein JSON");
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
fn ac1_der_lauf_verkuerzt_ueberhaupt_etwas() {
    // **Die Ursache zuerst, und sie ist hier die ganze Geschichte.** Genau
    // dieser Fall war bis #201 nicht baubar: Ohne einen Zauber, der verkuerzt,
    // vergleicht der Prueffall zwei Nullen und ist trotzdem gruen.
    let json = data();
    let (_, ergebnis) = nachspielen(&json);
    let vapour = index_of("vapour").expect("vapour");
    assert!(
        ergebnis.casts[vapour] > 0,
        "VAPOUR kam nicht zum Zug — dann verkuerzt nichts, und dieser Fall \
         belegt nichts (genau der Zustand, der zu Issue #104 gefuehrt hat)"
    );
    // Und es muss auch etwas ZU verkuerzen gegeben haben.
    let kindle = index_of("kindle").expect("kindle");
    assert!(
        ergebnis.casts[kindle] > 0,
        "KINDLE kam nicht zum Zug — dann lief keine Abklingzeit, die VAPOUR \
         haette verkuerzen koennen"
    );
}

#[test]
fn ac1_beide_seiten_ergeben_denselben_gesamtschaden() {
    let json = data();
    let (fight, ergebnis) = nachspielen(&json);
    let erwartet = &json["expected"];

    assert_eq!(
        u64::from(fight.damage()),
        erwartet["damageDealt"].as_u64().expect("damageDealt"),
        "der Gesamtschaden weicht ab"
    );
    let vapour = index_of("vapour").expect("vapour");
    assert_eq!(
        u64::from(ergebnis.casts[vapour]),
        erwartet["vapourCasts"].as_u64().expect("vapourCasts"),
        "VAPOUR wurde verschieden oft gewirkt"
    );
}

#[test]
fn ac2_die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    // **Nicht nur am Ende.** Eine Verkuerzung verschiebt Zeitpunkte: Beide
    // Seiten koennten am Schluss gleich viele Zauber durchgelassen haben und
    // sie trotzdem zu verschiedenen Zeiten gewirkt haben.
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

#[test]
fn ac2_die_verkuerzung_ist_in_der_spur_wirklich_sichtbar() {
    // Die Gegenprobe zur Pruefsumme: Sie zeigt Gleichlauf, nicht WIRKUNG.
    // Hier wird gemessen, dass die Restzeit von KINDLE zwischen zwei Takten
    // um mehr faellt, als die Uhr weiterlaeuft — das kann nur die Verkuerzung
    // gewesen sein.
    let json = data();
    let rest = zahlen(&json, "kindleReadyAt");
    let mut spruenge = 0;
    for paar in rest.windows(2) {
        // Ohne Verkuerzung faellt die Restzeit je Takt um genau TICK_MS.
        if paar[0] > paar[1] && paar[0] - paar[1] > TICK_MS {
            spruenge += 1;
        }
    }
    assert!(
        spruenge > 0,
        "die Restzeit von KINDLE ist nie schneller gefallen als die Uhr — \
         dann hat VAPOUR nichts bewirkt, und die Pruefsumme belegt nur, dass \
         beide Seiten dasselbe Nichts rechnen"
    );
}
