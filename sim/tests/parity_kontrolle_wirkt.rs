//! Die Kontrolle senkt den erlittenen Schaden (Issue #576, AC3).
//!
//! **Je drei Laeufe gegen zwei Bosse, die sich in genau einer Sache
//! unterscheiden:** welchen Kontroll-Zauber der Spieler beim laufenden
//! Boss-Zauber wirft — keinen, STAY oder STALL.
//!
//! | Boss | Lauf | erlitten | unterbrochen | verpasst | BIND | QUENCH |
//! | --- | --- | --- | --- | --- | --- | --- |
//! | boss-1-4 | none | 560 | 0 | 1 | 27 | 1 |
//! | boss-1-4 | stay | 560 | 0 | 1 | 26 | 1 |
//! | boss-1-4 | stall | **0** | **1** | 0 | 26 | 1 |
//! | test-vessel | none | 1280 | 0 | 1 | 27 | 1 |
//! | test-vessel | stay | 1280 | 0 | 1 | 26 | 1 |
//! | test-vessel | stall | **320** | **1** | 0 | 26 | 1 |
//!
//! # Zwei Bosse, weil einer den Nachweis allein traegt
//!
//! boss-1-4 endet auf 0 — er hat genau einen Zeitleisten-Eintrag, der Zauber
//! trifft ganz oder gar nicht. Ein Nachweis, der auf einer Null endet, faellt
//! schwerer zu widerlegen, als er sollte. test-vessel liefert dieselbe Aussage
//! mit 1280 gegen 320.
//!
//! # Der QUENCH-Zeitpunkt haelt die GROESSE der Wirkung fest
//!
//! Er liegt je Boss so spaet, dass die HALBE STALL-Wirkung nicht mehr reicht:
//!
//! ```text
//!   boss-1-4     Cast 30000+2500   ohne 32500 · halb 33500 · voll 34500
//!                QUENCH bei 34000
//!   test-vessel  Cast 20000+3000   ohne 23000 · halb 24000 · voll 25000
//!                QUENCH bei 24500
//! ```
//!
//! Der erste Entwurf legte ihn auf den Einschlag ohne STALL. Der Fall war
//! gruen und hielt zu wenig fest: `amountMs` von 2000 auf 1000 zu halbieren
//! brach ihn nicht. Geprueft war nur, DASS die Verlaengerung wirkt.
//!
//! # Warum der stay-Lauf da ist, und warum ohne ihn nichts bewiesen waere
//!
//! STALL verdraengt einen BIND-Wurf. Ein Vergleich gegen „gar kein
//! Kontroll-Zauber" unterschiede sich in ZWEI Dingen — der Wirkung und einem
//! GCD. STAY kostet Punkte, loest denselben GCD aus und verdraengt denselben
//! BIND, wirkt aber auf die Zeitleiste statt auf den laufenden Cast.
//!
//! Er aendert nichts — dieselbe Zahl wie ohne Kontrolle. Damit gehoert die
//! Differenz STALL und nicht der Verdraengung, und die gleiche BIND-Zahl
//! (26 bei beiden) belegt, dass die Last wirklich dieselbe war.
//!
//! # Warum die Null hier eine Aussage ist
//!
//! boss-1-4 traegt EINEN Zeitleisten-Eintrag: `Sealing Chant`, 700 Schaden,
//! 2500 ms Wirkzeit. Es gibt kein „ein Teil kommt durch" — der Zauber trifft
//! ganz (560 nach 20 % Ward) oder gar nicht. Die Null ist die Struktur des
//! Bosses, kein gewaehlter Randwert.
//!
//! # Warum boss-1-4 der erste Fall ist
//!
//! Er ist der einzige Boss aus dem INHALT mit unterbrechbarem Zauber, den der
//! Spieler ueberlebt. An boss-1-1 steht `damageTaken` bei 5040, ob unterbrochen wird
//! oder nicht: Dort stirbt der Spieler, und die Groesse misst dann seine
//! Lebenspunkte statt der Wirkung der Kontrolle. Wer den Nachweis dort
//! fuehrte, bekaeme in jeder Spalte dieselbe Zahl und hielte die Mechanik
//! fuer wirkungslos.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/kontrolle-wirkt.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("kontrolle-wirkt.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

/// Die beiden Lagen, in der Reihenfolge der Datei.
const BOSSE: [&str; 2] = ["boss-1-4", "test-vessel"];

fn lage<'a>(json: &'a Value, boss_id: &str) -> &'a Value {
    json["cases"]
        .as_array()
        .expect("cases fehlen")
        .iter()
        .find(|c| c["bossId"].as_str() == Some(boss_id))
        .unwrap_or_else(|| panic!("keine Lage fuer Boss {boss_id}"))
}

fn lauf<'a>(json: &'a Value, boss_id: &str, weg: &str) -> &'a Value {
    lage(json, boss_id)["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|r| r["way"].as_str() == Some(weg))
        .unwrap_or_else(|| panic!("kein Lauf fuer {boss_id}/{weg}"))
}

/// Spielt einen der drei Laeufe im Simulator nach.
///
/// Die Druecke stehen in der Datei, statt hier aus der Regel nachgebaut zu
/// werden — ein Nachbau, der minimal abweicht, ergaebe eine Abweichung, die
/// wie ein Rechenfehler aussieht.
fn replay(json: &Value, boss_id: &str, weg: &str) -> rota_sim::ReplayResult {
    let eintrag = lauf(json, boss_id, weg);
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(eintrag["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = eintrag["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| {
            let id = b["spellId"].as_str().expect("spellId");
            Binding {
                slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt")),
            }
        })
        .collect();
    let presses = eintrag["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    let recording = Recording {
        name: format!("kontrolle-wirkt-{boss_id}-{weg}"),
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
    fight.replay(&recording, true)
}

fn erwartet(json: &Value, boss_id: &str, weg: &str, feld: &str) -> u64 {
    lauf(json, boss_id, weg)["expected"][feld]
        .as_u64()
        .unwrap_or_else(|| panic!("{boss_id}/{weg}/{feld} fehlt"))
}

/// **Die Zusicherung vor jeder Messung** — mit Zahlen, nicht mit Anwesenheit.
#[test]
fn die_lagen_sind_noch_dieselben() {
    let json = data();
    assert_eq!(
        json["cases"].as_array().map(Vec::len),
        Some(2),
        "es sind nicht mehr zwei Lagen — ein Boss allein traegt den Nachweis nicht"
    );

    // boss-1-4 traegt GENAU einen Eintrag. Kaeme ein zweiter dazu, waere die
    // Null unten keine Struktur mehr, sondern ein Zufall.
    let vier = lage(&json, "boss-1-4");
    assert_eq!(
        vier["boss"]["timeline"].as_array().map(Vec::len),
        Some(1),
        "boss-1-4 traegt nicht mehr genau einen Zeitleisten-Eintrag"
    );

    // **Der QUENCH-Zeitpunkt muss nach dem HALBEN Fenster liegen.** Sonst
    // haelt der Nachweis nur fest, DASS die Verlaengerung wirkt.
    for boss_id in BOSSE {
        let l = lage(&json, boss_id);
        let quench = l["quenchAtMs"].as_u64().expect("quenchAtMs fehlt");
        let eintrag = l["boss"]["timeline"]
            .as_array()
            .expect("timeline fehlt")
            .iter()
            .find(|e| e["mechanic"].as_str() == Some("interruptibleCast"))
            .unwrap_or_else(|| panic!("{boss_id} hat keinen unterbrechbaren Zauber"));
        let landet_ohne =
            eintrag["atMs"].as_u64().expect("atMs") + eintrag["castTimeMs"].as_u64().expect("cast");
        // STALL verlaengert um 2000 ms; die halbe Wirkung waere 1000.
        assert!(
            quench > landet_ohne + 1000,
            "{boss_id}: QUENCH bei {quench} liegt im halben Fenster (Einschlag {landet_ohne}) \
             — eine halbierte STALL-Wirkung wuerde den Nachweis nicht brechen"
        );
        assert!(
            quench < landet_ohne + 2000,
            "{boss_id}: QUENCH bei {quench} liegt hinter dem vollen Fenster — auch STALL kaeme zu spaet"
        );

        // **Alle drei druecken QUENCH genau einmal.** Ohne diese Zeile bliebe
        // der Einwand offen, ob im stall-Lauf schlicht MEHR gedrueckt wurde.
        for weg in ["none", "stay", "stall"] {
            assert_eq!(
                erwartet(&json, boss_id, weg, "castsQuench"),
                1,
                "{boss_id}/{weg} drueckt QUENCH nicht genau einmal"
            );
        }
    }
}

#[test]
fn ac3_stall_senkt_den_erlittenen_schaden() {
    let json = data();
    for boss_id in BOSSE {
        let ohne = erwartet(&json, boss_id, "none", "damageTaken");
        let mit = erwartet(&json, boss_id, "stall", "damageTaken");

        assert_eq!(
            erwartet(&json, boss_id, "none", "interruptsLanded"),
            0,
            "{boss_id}: ohne Kontrolle wird schon unterbrochen"
        );
        assert_eq!(erwartet(&json, boss_id, "none", "interruptsMissed"), 1);

        assert_eq!(
            erwartet(&json, boss_id, "stall", "interruptsLanded"),
            1,
            "{boss_id}: STALL oeffnet das Fenster nicht"
        );
        assert_eq!(erwartet(&json, boss_id, "stall", "interruptsMissed"), 0);
        assert!(
            mit < ohne,
            "{boss_id}: STALL senkt den Schaden nicht ({ohne} -> {mit})"
        );
    }

    // Die eine Lage endet auf 0, die andere nicht — das ist der Grund, aus dem
    // beide dastehen.
    assert_eq!(erwartet(&json, "boss-1-4", "stall", "damageTaken"), 0);
    assert!(
        erwartet(&json, "test-vessel", "stall", "damageTaken") > 0,
        "auch der zweite Fall endet auf einer Null — dann traegt keiner den anderen"
    );
}

#[test]
fn ac3_die_kontrolle_zeigt_dass_es_nicht_die_verdraengung_ist() {
    let json = data();
    for boss_id in BOSSE {
        // STAY traegt dieselbe Last und aendert nichts.
        assert_eq!(
            erwartet(&json, boss_id, "stay", "damageTaken"),
            erwartet(&json, boss_id, "none", "damageTaken"),
            "{boss_id}: STAY veraendert den erlittenen Schaden — dann ist die Kontrolle keine"
        );
        assert_eq!(erwartet(&json, boss_id, "stay", "interruptsLanded"), 0);

        // **Gleiche Last, gemessen statt behauptet.**
        assert_eq!(
            erwartet(&json, boss_id, "stay", "castsBind"),
            erwartet(&json, boss_id, "stall", "castsBind"),
            "{boss_id}: STAY und STALL verdraengen verschieden viele BIND-Wuerfe"
        );
        assert_eq!(
            erwartet(&json, boss_id, "stay", "castsControl"),
            erwartet(&json, boss_id, "stall", "castsControl"),
            "{boss_id}: verschieden viele Kontroll-Wuerfe"
        );

        // Und der Lauf ohne Kontrolle wirft einen BIND MEHR — das ist die
        // Verdraengung, die STAY und STALL gemeinsam haben.
        assert!(
            erwartet(&json, boss_id, "none", "castsBind")
                > erwartet(&json, boss_id, "stay", "castsBind"),
            "{boss_id}: ohne Kontroll-Zauber wird nicht mehr BIND geworfen"
        );
    }
}

#[test]
fn beide_seiten_rechnen_dasselbe() {
    let json = data();
    for boss_id in BOSSE {
        for weg in ["none", "stay", "stall"] {
            let ergebnis = replay(&json, boss_id, weg);
            assert_eq!(
                u64::from(ergebnis.damage_taken),
                erwartet(&json, boss_id, weg, "damageTaken"),
                "{boss_id}/{weg}: anderer erlittener Schaden"
            );
            assert_eq!(
                u64::from(ergebnis.damage),
                erwartet(&json, boss_id, weg, "damageDealt"),
                "{boss_id}/{weg}: anderer ausgeteilter Schaden"
            );
            assert_eq!(
                u64::from(ergebnis.sequence_checksum),
                erwartet(&json, boss_id, weg, "sequenceChecksum"),
                "{boss_id}/{weg}: andere Folge-Pruefsumme"
            );
        }
    }
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    for boss_id in BOSSE {
        for weg in ["none", "stay", "stall"] {
            let ergebnis = replay(&json, boss_id, weg);
            let soll: Vec<u32> = lauf(&json, boss_id, weg)["checksums"]
                .as_array()
                .expect("checksums fehlen")
                .iter()
                .map(|v| u32::try_from(v.as_u64().expect("keine Zahl")).expect("zu gross"))
                .collect();

            assert!(!soll.is_empty(), "{boss_id}/{weg} hat keine Pruefsummen");
            if let Some(d) = first_divergence(&soll, &ergebnis.checksums, TICK_MS) {
                panic!("{boss_id}/{weg}: {d:?}");
            }
            assert_eq!(
                ergebnis.checksums.len(),
                soll.len(),
                "{boss_id}/{weg}: verschieden viele Takte"
            );
        }
    }
}
