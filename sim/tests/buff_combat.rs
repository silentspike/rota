//! Gleichlauf eines Kampfes mit **aktiver Verstaerkung** (Issue #464).
//!
//! # Was dieser Abgleich belegt — und was ausdruecklich nicht
//!
//! **Belegt:** Spiel und Simulator rechnen mit aktiver Verstaerkung dasselbe,
//! Takt fuer Takt, bis aufs letzte Bit der Zustands-Pruefsumme.
//!
//! **Nicht belegt, und das ist Absicht:** Dass die Verstaerkung ueberhaupt
//! wirkt. Das ist ein Prueffall, kein Abgleich — er steht auf der Spiel-Seite
//! in `tests/unit/buff-in-combat.test.ts` (#420 AC4).
//!
//! Eine Paritaets-Aufzeichnung vergleicht **Spiel gegen Simulator**, nicht zwei
//! Laeufe gegeneinander. Ein erster Entwurf fuehrte hier zusaetzlich einen Lauf
//! ohne Verstaerkung und haette damit doppelt geprueft, was schon geprueft ist.
//!
//! # Warum der Abgleich vor #420 nichts gebracht haette
//!
//! `buffs.json` rechnet die Stapel-Regel vollstaendig gegen — an einer von Hand
//! gefuetterten `Buffs`-Instanz. Kein Kampf, kein Zauber. Er war deshalb gruen,
//! waehrend **beide Seiten dasselbe nicht taten**: `factors.buffs` stand in
//! jedem Kampf dauerhaft auf 1.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
const BUFF_COMBAT: &str = include_str!("../../tests/parity/recordings/buff-combat.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(BUFF_COMBAT).expect("buff-combat.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von buff-combat.json"
    );
    value
}

/// Faehrt die Folge aus der Datei und gibt die Pruefsumme je Takt.
fn nachfahren(json: &Value) -> Vec<u32> {
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks fehlen")).expect("zu viele");
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));

    // Ein Platz je Zauber, in der Reihenfolge des ersten Vorkommens — dieselbe
    // Zuordnung wie im Spiel, wo `presses` auf Plaetze zeigen.
    let mut bindings: Vec<Binding> = Vec::new();
    let mut presses: Vec<Press> = Vec::new();
    for eintrag in json["presses"].as_array().expect("presses fehlen") {
        let id = eintrag["spellId"].as_str().expect("spellId fehlt");
        let index = index_of(id).unwrap_or_else(|| panic!("Zauber {id} gibt es nicht"));
        let slot: u8 = match bindings.iter().position(|b| b.spell == index) {
            Some(s) => u8::try_from(s).expect("mehr Plaetze als eine Leiste fasst"),
            None => {
                let neu = u8::try_from(bindings.len()).expect("mehr Plaetze als eine Leiste fasst");
                bindings.push(Binding {
                    slot: neu,
                    spell: index,
                });
                neu
            }
        };
        presses.push(Press {
            tick: u32::try_from(eintrag["tick"].as_u64().expect("tick fehlt")).expect("zu gross"),
            slot,
        });
    }

    let recording = Recording {
        name: "buff-combat".to_owned(),
        seed: 0,
        ticks,
        bindings,
        presses,
    };

    let mut fight = Fight::new(FightConfig {
        boss,
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, true);
    ergebnis.checksums
}

#[test]
fn ac1_die_verstaerkung_war_im_lauf_wirklich_aktiv() {
    // **Zuerst diese Zusicherung.** Ohne sie glichen beide Seiten einen Lauf
    // ab, in dem die Verstaerkung gar nicht wirkte — und waeren sich einig
    // darueber, nichts zu tun. Genau dieser Zustand hat #420 fuenf Issues lang
    // ueberlebt.
    let json = data();
    assert_eq!(
        json["expected"]["buffWasActive"].as_bool(),
        Some(true),
        "in der Aufzeichnung war die Verstaerkung nie aktiv — dann prueft \
         dieser Abgleich einen Lauf ohne den Gegenstand, um den es geht"
    );

    // Und der Ablauf steht mit drin: Am letzten Messpunkt ist der Faktor
    // wieder neutral. Ohne das koennte die Verstaerkung ewig laufen, und die
    // Datei saehe genauso aus.
    let punkte = json["factorAt"].as_array().expect("factorAt fehlt");
    let letzter = punkte.last().expect("factorAt ist leer");
    assert_eq!(
        letzter["buffs"].as_f64(),
        Some(1.0),
        "am letzten Messpunkt laeuft die Verstaerkung noch — dann faehrt die \
         Aufzeichnung ihren Ablauf gar nicht mit"
    );
}

#[test]
fn ac2_beide_seiten_rechnen_takt_fuer_takt_dasselbe() {
    let json = data();
    let erwartet = json["checksums"].as_array().expect("checksums fehlen");
    let gemessen = nachfahren(&json);

    assert_eq!(
        gemessen.len(),
        erwartet.len(),
        "der Simulator hat {} Takte gefahren, die Datei fuehrt {}",
        gemessen.len(),
        erwartet.len()
    );
    // **Nicht als Ganzes vergleichen.** `assert_eq!` ueber sechshundert Zahlen
    // sagt nur "nicht gleich"; die Takt-Nummer ist die Auskunft, mit der sich
    // eine Abweichung finden laesst.
    for (tick, soll) in erwartet.iter().enumerate() {
        let soll = u32::try_from(soll.as_u64().expect("checksum ist keine Zahl"))
            .expect("passt nicht in 32 Bit");
        assert_eq!(
            gemessen[tick], soll,
            "Takt {tick}: Simulator {} gegen Spiel {soll}",
            gemessen[tick]
        );
    }
}
