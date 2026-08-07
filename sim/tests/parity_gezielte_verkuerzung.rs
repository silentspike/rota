//! ASCEND und DECANT verkuerzen GEZIELT — auf beiden Seiten gleich (Issue #518).
//!
//! # Warum es diese Datei gibt, und warum sie beinahe gefehlt haette
//!
//! `gezielte-verkuerzung.json` wurde exportiert und vom Frische-Fall des
//! Spiels geprueft — aber KEIN Simulator-Fall spielte sie nach. Damit war sie
//! eine Aufzeichnung ohne Aussage: Sie belegte, dass das Spiel mit sich selbst
//! uebereinstimmt, und ueber die Paritaet gar nichts.
//!
//! Aufgefallen ist es an einem falschen Verweis: Ein Kommentar in
//! `tests/sublimation.rs` nannte diese Datei als Fundort der Umfangs-Pruefung,
//! und es gab sie nicht. `grep -rln gezielte-verkuerzung sim/` lieferte null
//! Treffer, waehrend `wisp-verkuerzung` seinen Fall hatte.
//!
//! # Was hier geprueft wird, das anderswo nicht steht
//!
//! Die breiten Verkuerzer (VAPOUR, WISP) nehmen ALLEN laufenden Abklingzeiten
//! denselben Betrag. Diese zwei nicht:
//!
//! ```text
//! ASCEND   10 000 ms, aber nur auf Abklingzeiten UNTER 10 000 ms
//! DECANT    2 000 ms, aber nur auf die LAENGSTE
//! ```
//!
//! Der Umfang ist die eigentliche Mechanik. Ein Fall, der nur "es wurde
//! verkuerzt" belegt, wuerde einen Simulator durchlassen, der beide als breite
//! Verkuerzer rechnet — und genau das ist der teure Fehler, weil er im
//! Gesamtschaden fast verschwindet.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/gezielte-verkuerzung.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("gezielte-verkuerzung.json ist kein JSON");
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
fn ac1_beide_gezielten_verkuerzer_kommen_zum_zug() {
    // **Die Ursache zuerst.** Ohne Wuerfe belegt der Fall nichts — dieselbe
    // Falle wie bei #104, wo zwei Nullen verglichen wurden und der Fall gruen
    // war.
    let json = data();
    let (_, ergebnis) = nachspielen(&json);

    for id in ["ascend", "decant"] {
        let index = index_of(id).unwrap_or_else(|| panic!("{id} fehlt"));
        assert!(
            ergebnis.casts[index] > 0,
            "{id} kam nicht zum Zug — dann sagt diese Aufzeichnung ueber ihn \
             nichts, egal wie viele Takte sie hat"
        );
    }
}

#[test]
fn ac1_beide_seiten_ergeben_denselben_gesamtschaden_und_dieselben_wuerfe() {
    let json = data();
    let (fight, ergebnis) = nachspielen(&json);
    let erwartet = &json["expected"];

    assert_eq!(
        u64::from(fight.damage()),
        erwartet["damageDealt"].as_u64().expect("damageDealt"),
        "der Gesamtschaden weicht ab"
    );

    // **Die Wurfzahlen einzeln**, nicht ihre Summe. Zwei Zauber, die sich
    // gegenseitig ausgleichen, ergaeben dieselbe Summe und eine andere
    // Rotation — und der Umfang der Verkuerzung haengt daran, WELCHER von
    // beiden wann geworfen wurde.
    let casts = &erwartet["casts"];
    for id in ["ascend", "decant"] {
        let index = index_of(id).unwrap_or_else(|| panic!("{id} fehlt"));
        assert_eq!(
            u64::from(ergebnis.casts[index]),
            casts[id]
                .as_u64()
                .unwrap_or_else(|| panic!("casts.{id} fehlt")),
            "{id} wurde verschieden oft gewirkt"
        );
    }
}

#[test]
fn ac2_die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    // **Nicht nur am Ende.** Eine Verkuerzung verschiebt Zeitpunkte: Beide
    // Seiten koennten am Schluss gleich viele Zauber durchgelassen haben und
    // sie trotzdem zu verschiedenen Zeiten gewirkt haben. Bei GEZIELTEN
    // Verkuerzern wiegt das schwerer, weil der Umfang selbst davon abhaengt,
    // welche Abklingzeit im Moment des Wurfs wie weit gelaufen ist.
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

/// Die Verkuerzung ist an PYREs Restzeit sichtbar — und in einer Groesse, die
/// zu einem GEZIELTEN Verkuerzer passt.
///
/// # Warum die blosse Sichtbarkeit hier zu wenig waere
///
/// PYRE hat 5 000 ms und liegt damit UNTER ASCENDs Schwelle von 10 000 ms —
/// ASCEND trifft ihn also. Ein Simulator, der ASCEND als breiten Verkuerzer
/// rechnete, traefe ihn ebenfalls: Der Unterschied zeigt sich nicht an PYRE,
/// sondern an dem, was ASCEND NICHT anfasst. Deshalb steht hier nur die
/// GROESSE des Sprungs; den UMFANG pruefen die Faelle an `shorten_cooldowns`
/// in `sim/src/sublimation.rs` — `ascend_trifft_nur_abklingzeiten_unter_der_schwelle`,
/// `decant_trifft_nur_die_laengste_abklingzeit` und vier weitere. Die gab es
/// bis zum Nachziehen von #518 auf main nicht: `below` und `longest_only`
/// kamen dort acht Mal vor, jedes Mal mit dem Standardwert.
///
/// Der Betrag wird aus den Daten hergeleitet, nicht hingeschrieben (Issue
/// #295).
#[test]
fn ac2_die_verkuerzung_ist_in_der_spur_wirklich_sichtbar() {
    let json = data();
    let rest = zahlen(&json, "pyreReadyAt");

    // Ein Sprung der Restzeit, der groesser ist als der Lauf der Uhr, kann nur
    // eine Verkuerzung gewesen sein. Welcher der beiden Zauber ihn ausloeste,
    // sagt die Spur nicht — deshalb zaehlt hier die Existenz eines Sprungs
    // UEBER dem Uhrenschritt, und die Zuordnung macht der Fall darueber ueber
    // die Wurfzahlen.
    let mut ueber_der_uhr = 0;
    let mut groesster = 0;
    for paar in rest.windows(2) {
        if paar[0] <= paar[1] {
            continue;
        }
        let fall = paar[0] - paar[1];
        groesster = groesster.max(fall);
        if fall > TICK_MS {
            ueber_der_uhr += 1;
        }
    }

    assert!(
        ueber_der_uhr > 0,
        "PYREs Restzeit faellt nie um mehr als die Uhr ({TICK_MS} ms) — dann \
         hat keiner der beiden Verkuerzer sie je getroffen, und die \
         Aufzeichnung belegt ueber die Mechanik nichts. Groesster gemessener \
         Fall: {groesster} ms"
    );
}
