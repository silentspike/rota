//! Der Talent-Weg `charges` wirkt auch im Simulator (Issue #738).
//!
//! # Warum er als letzter der zwoelf drankam
//!
//! Drei Aufbauten ueber `Fight::run` blieben stumm — mit und ohne Zuschlag
//! dieselben Restzeiten, **und ohne Zuschlag wurde ebenfalls nichts
//! zurueckgesetzt**. Die zweite Haelfte war der Hinweis: Der Ruecksetzer ist
//! ein AUSWAHLMODUS. Er stellt scharf, und erst der naechste Druck sagt,
//! welchen Zauber er meint.
//!
//! Bedient wird diese Wahl nur im Abspieler (`fight.rs:1416`,
//! `if self.reset_mode.is_armed()`), nicht in `run`. Ein Takt, der einfach
//! weiterdrueckt, kommt gar nicht daran vorbei — deshalb war jeder Versuch
//! ueber `run` von vornherein stumm, ganz gleich wie viele Raenge gesetzt
//! waren.
//!
//! # Die Aufzeichnung gab es schon
//!
//! `alembic-zwei-ladungen.json` wirft ALEMBIC (zwei Ladungen) und fuehrt die
//! Ladungs-Spur bereits mit. Es brauchte also keine neue Aufzeichnung und
//! keinen neuen Export — nur denselben Lauf zweimal, mit und ohne Zuschlag.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

const JSON: &str = include_str!("../../tests/parity/recordings/alembic-zwei-ladungen.json");

fn json() -> Value {
    serde_json::from_str(JSON).expect("alembic-zwei-ladungen.json ist kein JSON")
}

/// Spielt die Aufzeichnung mit einem Ladungs-Zuschlag ab.
fn hoechste_ladung(bonus: u32) -> u32 {
    let json = json();
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks fehlen")).expect("zu viele");
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));

    let bindings: Vec<Binding> = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| Binding {
            slot: u8::try_from(b["slot"].as_u64().expect("slot fehlt")).expect("zu gross"),
            spell: index_of(b["spellId"].as_str().expect("spellId fehlt"))
                .expect("Zauber der Aufzeichnung fehlt im Katalog"),
        })
        .collect();
    let presses: Vec<Press> = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick fehlt")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot fehlt")).expect("zu gross"),
        })
        .collect();

    let recording = Recording {
        name: String::from("alembic-zwei-ladungen"),
        seed: 0,
        ticks,
        presses,
        bindings,
    };
    let mut fight = Fight::new(FightConfig {
        boss,
        talent_cooldown_charges_bonus: bonus,
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, true);

    // **Der HOECHSTE Wert der Spur, nicht der letzte.** Die Ladungen zaehlen
    // waehrend des Laufs herunter; am Ende steht regelmaessig 0, und zwei
    // Nullen zu vergleichen sagte nichts. Der Scheitelpunkt ist die Zahl, die
    // beim Scharfstellen gesetzt wurde — die Groesse, die das Talent hebt.
    ergebnis
        .reset_charges_left
        .into_iter()
        .max()
        .expect("die Ladungs-Spur ist leer")
}

#[test]
fn der_traeger_stellt_ueberhaupt_ladungen_scharf() {
    // **Zuerst der Traeger, dann die Wirkung** (AC8 aus #676). Bliebe die Spur
    // auf 0, verglichen die Faelle unten zwei Nullen — genau die Lage, in der
    // drei Anlaeufe ueber `Fight::run` steckengeblieben sind.
    let ohne = hoechste_ladung(0);
    assert!(
        ohne > 0,
        "die Ladungs-Spur bleibt bei 0 — der Ruecksetzer wurde nie scharf \
         gestellt, und ein Talent auf die Ladungen koennte sich nicht zeigen"
    );
}

#[test]
fn der_weg_charges_stellt_mehr_ladungen_scharf() {
    let ohne = hoechste_ladung(0);
    let mit = hoechste_ladung(5);

    // Ein ganzzahliger ZUSCHLAG, kein Faktor: genau fuenf mehr, nicht das
    // Sechsfache. Dass es die exakte Differenz ist und nicht nur "mehr",
    // trennt den Zuschlag von einem Faktor.
    assert_eq!(
        mit,
        ohne + 5,
        "der Weg charges wirkt nicht wie ein Zuschlag: {ohne} Ladungen ohne \
         Talent, {mit} mit fuenf Raengen"
    );
}

#[test]
fn ein_nicht_ruecksetzer_wird_durch_das_talent_keiner() {
    // **Der Fall, der den Zuschlag an die richtige Stelle zwingt.** Laege er in
    // der Pruefung `cooldown_reset_charges > 0` statt dahinter, waere jeder
    // Zauber ein Ruecksetzer, sobald ein Rang verteilt ist. Die Aufzeichnung
    // wirft KINDLE und PYRE mit; keiner von beiden traegt Ladungen.
    //
    // Gemessen wird an der Hoehe der Spur: Wuerden auch die Fueller scharf
    // stellen, laege der Scheitel bei `bonus` statt bei `ladungen + bonus`,
    // und die Gleichung im Fall darueber ginge nicht auf. Dieser Fall haelt
    // die Aussage ausdruecklich fest, statt sie aus dem anderen zu folgern.
    let mit = hoechste_ladung(5);
    let ohne = hoechste_ladung(0);
    assert!(
        mit > 5,
        "der Scheitel liegt bei {mit} und damit hoechstens beim Zuschlag \
         allein — das spricht dafuer, dass ein Zauber OHNE eigene Ladungen \
         scharf gestellt hat"
    );
    assert_eq!(mit - ohne, 5, "der Zuschlag ist nicht genau fuenf");
}
