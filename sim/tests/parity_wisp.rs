//! WISPs Verkuerzung rechnet auf beiden Seiten gleich (Issue #480).
//!
//! # Warum WISP einen eigenen Lauf hat
//!
//! Er stand in `sublimation.json` und hat den dortigen Fall zerstoert: WISP
//! ist Fueller UND Verkuerzer, hat dieselbe Wartezeit wie KINDLE (1500 ms) und
//! ist ihm sonst ueberlegen — er verdraengt ihn also vollstaendig. Ohne
//! KINDLE-Wuerfe verliert `parity_sublimation_run` seinen TRAEGER: Er misst
//! VAPOURs Verkuerzung an einer laufenden Abklingzeit, und ohne KINDLE laeuft
//! keine.
//!
//! Gemessen war das an `ac1_der_lauf_verkuerzt_ueberhaupt_etwas`, der bei
//! `casts[kindle] > 0` fiel. Vier Anlaeufe, die Auswahl-Reihenfolge und die
//! Bedingungen zu drehen, endeten jedes Mal damit, dass EINER der beiden bei
//! null stand. Zwei Fueller passen nicht auf eine Wartezeit.
//!
//! # Warum VAPOUR der Traeger ist — und warum nicht PYRE oder REFLUX
//!
//! Der Traeger muss OHNE KINDLE wirkbar sein: PYRE verlangt 15 Hitze, und
//! Hitze kommt allein aus KINDLE. In einem Lauf ohne KINDLE stuende PYREs
//! Restzeit konstant bei null — zwei Seiten, die dasselbe Nichts rechnen.
//! Genau dieser Fehler ist dieser Aufzeichnungs-Familie schon einmal
//! unterlaufen (siehe den Kommentar an `pyreReadyAt` in `payloads.ts`).
//!
//! REFLUX war der erste Entwurf und ist an einem ECHTEN BEFUND gescheitert:
//! Mit ihm als Traeger lief die Pruefsumme ab Takt 150 auseinander, Schaden
//! 880 gegen 640. Sein `arms_cooldown_reset` sitzt auf VERSCHIEDENEN
//! SCHICHTEN — im Spiel in `session.ts`, hier in `fight.rs` (Zeile 1224). Eine
//! Aufzeichnung faehrt `Combat` direkt und sieht die Spiel-Seite deshalb nie,
//! der Simulator schaerft aber. Gemeldet, verwandt mit #521; hier umgangen,
//! nicht behoben.
//!
//! VAPOUR traegt sauber, GERADE WEIL er selbst verkuerzt: Der wirkende Zauber
//! ist von seiner eigenen Verkuerzung ausgenommen, jeder Sprung in SEINER
//! Restzeit kann also nur von WISP kommen.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, SPELLS};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/wisp-verkuerzung.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("wisp-verkuerzung.json ist kein JSON");
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
fn ac1_wisp_kommt_ueberhaupt_zum_zug_und_hat_etwas_zu_verkuerzen() {
    // **Die Ursache zuerst.** Dieselbe Falle wie bei #104: Ohne einen Wurf des
    // Verkuerzers oder ohne laufende Abklingzeit vergleicht der Fall zwei
    // Nullen und ist trotzdem gruen.
    let json = data();
    let (_, ergebnis) = nachspielen(&json);

    let wisp = index_of("wisp").expect("wisp");
    assert!(
        ergebnis.casts[wisp] > 0,
        "WISP kam nicht zum Zug — dann verkuerzt nichts, und dieser Fall belegt \
         nichts. Genau dafuer gibt es diese Aufzeichnung: In der Sublimations-\
         Aufzeichnung stand er zwar in der Leiste, verdraengte dort aber KINDLE"
    );
    let vapour = index_of("vapour").expect("vapour");
    assert!(
        ergebnis.casts[vapour] > 0,
        "VAPOUR kam nicht zum Zug — dann lief keine Abklingzeit, die WISP \
         haette verkuerzen koennen. Er ist der Traeger dieses Laufs"
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
    let wisp = index_of("wisp").expect("wisp");
    assert_eq!(
        u64::from(ergebnis.casts[wisp]),
        erwartet["wispCasts"].as_u64().expect("wispCasts"),
        "WISP wurde verschieden oft gewirkt"
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

/// WISPs Verkuerzung ist in der Spur sichtbar — in der richtigen GROESSE.
///
/// Der Betrag wird aus den DATEN hergeleitet, nicht hingeschrieben (Issue
/// #295): Eine feste 510 waere scharf und falsch — sie pruefte eine Zahl, die
/// niemand mehr mit WISP verbindet, und laege beim naechsten Balance-Lauf
/// still daneben.
#[test]
fn ac2_die_verkuerzung_ist_in_der_spur_wirklich_sichtbar() {
    let json = data();
    let rest = zahlen(&json, "vapourReadyAt");

    let wisp = index_of("wisp").expect("wisp");
    let verkuerzung = SPELLS[wisp].cooldown_reduction_ms;
    assert!(
        verkuerzung > 0,
        "WISP traegt keine Verkuerzung mehr — dann kann dieser Fall nichts \
         belegen, und die Frage ist nicht seine Schaerfe, sondern ob es die \
         Mechanik noch gibt"
    );
    let erwarteter_sprung = TICK_MS + verkuerzung;

    let mut treffer = 0;
    let mut groesster = 0;
    for paar in rest.windows(2) {
        if paar[0] <= paar[1] {
            continue;
        }
        let fall = paar[0] - paar[1];
        groesster = groesster.max(fall);
        // Ohne Verkuerzung faellt die Restzeit je Takt um genau TICK_MS.
        if fall == erwarteter_sprung {
            treffer += 1;
        }
    }

    assert!(
        treffer > 0,
        "kein einziger Fall der Restzeit betraegt {erwarteter_sprung} ms \
         ({} ms Uhr + {verkuerzung} ms WISP). Der groesste gemessene Fall war \
         {groesster} ms — bei {TICK_MS} ms hat WISP gar nicht gewirkt, bei \
         einem anderen Wert rechnet die Verkuerzung anders als die Daten sagen",
        TICK_MS
    );
}

/// Jeder wirkende Wurf ist auch angekommen — nicht nur einer.
///
/// # Warum die Einzelspruenge nicht genuegen
///
/// Der Fall darueber belegt, dass die Verkuerzung EINMAL in der richtigen
/// Groesse gewirkt hat. Verschluckte der Simulator die Haelfte der Wuerfe,
/// bliebe er trotzdem gruen — er saehe ja immer noch Spruenge der richtigen
/// Groesse. Hier wird die ANZAHL gegengerechnet.
///
/// # Warum die Zahl aus der Aufzeichnung kommt und nicht aus `casts`
///
/// Nicht jeder WISP-Wurf verkuerzt etwas: Ist VAPOUR gerade bereit, geht die
/// Wirkung ins Leere. Das Spiel zaehlt beim Aufzeichnen mit, wie viele Wuerfe
/// auf eine LAUFENDE Abklingzeit trafen (`wirkendeWuerfe`) — diese Zahl ist
/// die Zusicherung, gegen die der Simulator antritt.
#[test]
fn ac3_jeder_wirkende_wurf_ist_auch_angekommen() {
    let json = data();
    let rest = zahlen(&json, "vapourReadyAt");

    let wisp = index_of("wisp").expect("wisp");
    let verkuerzung = SPELLS[wisp].cooldown_reduction_ms;
    let erwarteter_sprung = TICK_MS + verkuerzung;

    let treffer = rest
        .windows(2)
        .filter(|p| p[0] > p[1] && p[0] - p[1] == erwarteter_sprung)
        .count();

    let soll = json["expected"]["wirkendeWuerfe"]
        .as_u64()
        .expect("wirkendeWuerfe fehlt");
    assert!(
        soll > 0,
        "die Aufzeichnung meldet KEINEN wirkenden Wurf — dann steht dieser \
         Fall auf nichts, egal wie scharf er formuliert ist"
    );
    assert_eq!(
        u64::try_from(treffer).expect("passt"),
        soll,
        "das Spiel hat {soll} Wuerfe auf eine laufende Abklingzeit gezaehlt, \
         in der Spur stehen aber {treffer} Spruenge von {erwarteter_sprung} ms"
    );
}
