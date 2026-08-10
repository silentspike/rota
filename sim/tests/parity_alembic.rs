//! ALEMBIC macht ZWEI Zauber bereit — auf beiden Seiten gleich (Issue #521).
//!
//! # Was dieser Fall zeigt, den kein anderer zeigt
//!
//! Nach einem Ruecksetzer ist der naechste Druck keine Wirkung, sondern eine
//! AUSWAHL. Bei ALEMBIC sind es zwei hintereinander. Beide Seiten muessen an
//! derselben Stelle vom Wirken aufs Waehlen umschalten — und nach genau zwei
//! Auswahlen zurueck.
//!
//! Eine Seite, die nach der ersten Auswahl zurueckschaltet, macht aus der
//! zweiten eine WIRKUNG. Der Gesamtschaden weicht dann ab, und die Rotation
//! laeuft auseinander; im Zaehlerstand sieht man es als eine Auswahl weniger.
//!
//! # Warum es diese Datei erst seit #528 gibt
//!
//! Vorher lag der Auswahlmodus in `Session`, und JEDE Aufzeichnung faehrt
//! `Combat` direkt. Eine Regel ausserhalb von `Combat` war fuer die Paritaet
//! deshalb nicht schwer pruefbar, sondern unsichtbar.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, SPELLS};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/alembic-zwei-ladungen.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON).expect("alembic-zwei-ladungen.json ist kein JSON");
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
fn ac2_alembic_kommt_zum_zug_und_beide_seiten_zaehlen_dieselben_wuerfe() {
    // **Die Ursache zuerst.** Ohne einen Wurf belegt der Fall nichts.
    let json = data();
    let (_, ergebnis) = nachspielen(&json);
    let alembic = index_of("alembic").expect("alembic");
    assert!(
        ergebnis.casts[alembic] > 0,
        "ALEMBIC kam nicht zum Zug — dann sagt diese Aufzeichnung ueber ihn \
         nichts, egal wie viele Takte sie hat"
    );
    assert_eq!(
        u64::from(ergebnis.casts[alembic]),
        json["expected"]["alembicCasts"]
            .as_u64()
            .expect("alembicCasts"),
        "ALEMBIC wurde verschieden oft gewirkt"
    );
}

#[test]
fn ac2_beide_seiten_ergeben_denselben_gesamtschaden() {
    let json = data();
    let (fight, _) = nachspielen(&json);
    assert_eq!(
        u64::from(fight.damage()),
        json["expected"]["damageDealt"]
            .as_u64()
            .expect("damageDealt"),
        "der Gesamtschaden weicht ab — bei zwei Auswahlen ist das der \
         Hinweis darauf, dass eine Seite die zweite als WIRKUNG gerechnet hat"
    );
}

#[test]
fn ac2_die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
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

/// Die zwei Ladungen sind in der Spur sichtbar — und zwar als ZWEI.
///
/// # Warum die Pruefsumme das nicht schon erledigt
///
/// Sie zeigt Gleichlauf, nicht WIRKUNG. Waeren beide Seiten gleich falsch —
/// etwa beide mit nur einer Ladung —, liefe sie durch. Hier wird gemessen,
/// dass die Spur ueberhaupt einen Zustand mit ZWEI offenen Auswahlen enthaelt
/// und danach einen mit genau EINER.
///
/// Der Betrag kommt aus den Daten, nicht aus dem Code (Issue #295): Eine feste
/// 2 waere scharf und falsch, sobald jemand ALEMBIC auf drei Ladungen setzt.
#[test]
fn ac2_die_zwei_ladungen_stehen_in_der_spur() {
    let json = data();
    let spur = zahlen(&json, "ladungen");

    let alembic = index_of("alembic").expect("alembic");
    let ladungen = u32::from(SPELLS[alembic].cooldown_reset_charges);
    assert!(
        ladungen >= 2,
        "ALEMBIC traegt nur {ladungen} Ladung(en) — dann kann dieser Fall \
         nichts ueber zwei Auswahlen belegen, und die Frage ist nicht seine \
         Schaerfe, sondern ob es die Mechanik noch gibt"
    );

    assert!(
        spur.contains(&ladungen),
        "kein Takt zeigt {ladungen} offene Auswahlen — ALEMBIC hat entweder \
         nie scharf gestellt oder sofort alles verbraucht"
    );
    // Und der Zwischenzustand: Nach der ERSTEN Auswahl muss genau eine
    // offenbleiben. Ohne ihn koennte eine Seite beide Ladungen in einem
    // einzigen Druck verbrauchen.
    assert!(
        spur.contains(&1),
        "kein Takt zeigt GENAU EINE offene Auswahl — dann wurden beide \
         Ladungen in einem Druck verbraucht, statt in zweien"
    );

    // **Und jetzt gegen die andere Seite** (gemessen bei #480).
    //
    // Alles darüber prüft die Spur gegen sich selbst: „kommt eine 2 vor,
    // kommt eine 1 vor". Das ist eine Aussage über das SPIEL und keine über
    // die Parität — der Simulator kam darin bis hierher nicht vor.
    //
    // Die Lücke hat eine Verzögerung: Der Auswahlmodus steht in KEINER der
    // beiden Prüfsummen. Er wirkt nur mittelbar, über eine Abklingzeit, die er
    // zurücksetzt. Liefen beide Seiten im Zustand auseinander, fiele es erst
    // auf, wenn er greift — und dann an einer anderen Stelle als der, an der
    // es entstand.
    let (_, result) = nachspielen(&json);
    assert_eq!(
        result.reset_charges_left.len(),
        spur.len(),
        "verschieden viele Takte in der Spur der offenen Auswahlen"
    );
    for (tick, (links, rechts)) in result
        .reset_charges_left
        .iter()
        .zip(spur.iter())
        .enumerate()
    {
        assert_eq!(
            links, rechts,
            "die offenen Auswahlen gehen im Takt {tick} auseinander: \
             Simulator {links}, Spiel {rechts}"
        );
    }
}

/// Die Auswahlen haben wirklich Abklingzeiten geloescht, nicht bloss
/// stattgefunden.
#[test]
fn ac2_beide_auswahlen_haben_gegriffen() {
    let json = data();
    let soll = json["expected"]["auswahlen"].as_u64().expect("auswahlen");
    assert!(
        soll >= 2,
        "die Aufzeichnung meldet nur {soll} greifende Auswahl(en) — eine \
         Auswahl auf einen BEREITEN Zauber verpufft, und dann zeigt der Lauf \
         die Mechanik nicht"
    );
}
