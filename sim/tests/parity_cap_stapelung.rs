//! Zwei Verstaerkungen VERSCHIEDENER Art auf derselben Zielgroesse (Issue #562).
//!
//! # Was diese Datei zeigt
//!
//! SLAG (`kind: 1`) und `test-cap` (`kind: 0`) wirken beide auf `cap`. Von
//! dreizehn Verstaerkern im Bestand trug bis #562 genau EINER `kind: 1` — SLAG,
//! allein in seinem Speicher. Es gab damit keine Lage, in der zwei
//! verschiedene Arten EINEN Speicher teilen, und GAME-DESIGN 11.3 Satz 1
//! ("verschiedene Arten stapeln multiplikativ") war ueber einen Zauber nicht
//! pruefbar — auf keiner der beiden Seiten.
//!
//! # Wo die Regel in dieser Aufzeichnung steckt
//!
//! **Nicht im Schaden.** Gemessen: BELLOWS gibt 8 je Wartezeit, der Verfall
//! nimmt 1 je 2000 ms. Bis zum gestapelten Deckel von 60 waeren es rund neun
//! Wartezeiten, das laengste Fenster haelt vier. Der Vorrat erreicht die
//! angehobene Grenze nie, und der Schaden ist mit und ohne Stapelung derselbe.
//! Das ist ein Balance-Befund (#547), kein Fehler dieser Aufzeichnung.
//!
//! **Sondern in der Pruefsumme.** `combat_checksum` mischt die Fenstersumme
//! aller Verstaerkungen ein, und die zaehlt den `cap`-Speicher mit. Sie ist die
//! Summe der Ablaufzeiten ALLER offenen Arten:
//!
//! ```text
//!   beide Arten offen (Stapelung)   6000 + 6000 = 12000
//!   eine verdraengt die andere       6000
//! ```
//!
//! Legte der Simulator den zweiten Buff nicht an, oder unter derselben Art,
//! liefen die Pruefsummen ab Takt 150 auseinander. Die Wirkungs-Pruefung mit
//! harten Zahlen steht auf der Spiel-Seite
//! (`tests/unit/buff-stapelung-ueber-zauber.test.ts`), wo `setResource` den
//! Deckel direkt misst, statt ihn ueber einen Erzeuger zu suchen.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/cap-stapelung.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("cap-stapelung.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

fn u32_at(node: &Value, key: &str) -> u32 {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    u32::try_from(raw).expect("passt nicht in 32 Bit")
}

fn recording(json: &Value) -> Recording {
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt"))
                    .expect("slot > 255"),
                spell: index_of(spell_id)
                    .unwrap_or_else(|| panic!("Zauber {spell_id} kennt der Simulator nicht")),
            }
        })
        .collect();

    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick fehlt"))
                .expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name fehlt").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed fehlt")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks fehlen"))
            .expect("ticks zu gross"),
        bindings,
        presses,
    }
}

/// Zaehlt die Druecke auf den Platz, auf dem der Zauber liegt.
fn druecke_auf(aufzeichnung: &Recording, id: &str) -> usize {
    let zauber = index_of(id).unwrap_or_else(|| panic!("{id} fehlt im Simulator"));
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .unwrap_or_else(|| panic!("{id} liegt nicht auf der Leiste dieser Aufzeichnung"));
    aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count()
}

/// **Die Zusicherung vor jeder Messung.**
///
/// Zwei Zauber in einer Aufzeichnung: Ein gemeinsamer Fall waere schon gruen,
/// wenn nur EINER von ihnen gedrueckt wird — und genau dann gaebe es keine
/// Stapelung zu pruefen, sondern nur eine einzelne Verstaerkung.
#[test]
fn beide_verstaerker_werden_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    for id in ["slag", "test-cap"] {
        let druecke = druecke_auf(&aufzeichnung, id);
        assert!(
            druecke > 0,
            "{id} liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
             die Stapel-Regel weiterhin nur angenommen"
        );
        assert_eq!(
            u64::try_from(druecke).expect("passt"),
            json["expected"]["casts"][id]
                .as_u64()
                .unwrap_or_else(|| panic!("casts.{id} fehlt")),
            "Simulator und Spiel zaehlen verschieden viele {id}-Wirkungen"
        );
    }
}

/// Die beiden Fenster muessen sich UEBERLAPPEN, sonst stapelt nichts.
///
/// **Die zweite Zahl, die fuer die Aussage niemand braucht**, und ohne die der
/// Fall daneben gruen waere, ohne etwas zu zeigen: SLAG haelt vier
/// Wartezeiten (bis 6000 ms), `test-cap` drei ab seinem Wurf bei 1500 ms
/// (1500 + 4500 = 6000). Ruecken die Druecke auseinander, laufen die Fenster
/// nacheinander statt nebeneinander — und die Aufzeichnung zeigte zwei
/// einzelne Verstaerkungen statt einer Stapelung.
#[test]
fn die_beiden_fenster_ueberlappen_sich() {
    let json = data();
    let aufzeichnung = recording(&json);

    let tick_von = |id: &str| -> u32 {
        let zauber = index_of(id).unwrap_or_else(|| panic!("{id} fehlt"));
        let slot = aufzeichnung
            .bindings
            .iter()
            .find(|b| b.spell == zauber)
            .map(|b| b.slot)
            .unwrap_or_else(|| panic!("{id} liegt nicht auf der Leiste"));
        aufzeichnung
            .presses
            .iter()
            .find(|p| p.slot == slot)
            .map(|p| p.tick)
            .unwrap_or_else(|| panic!("{id} wird nie gedrueckt"))
    };

    let slag_ms = tick_von("slag") * TICK_MS;
    let cap_ms = tick_von("test-cap") * TICK_MS;
    let abstand = cap_ms.abs_diff(slag_ms);

    // SLAGs Fenster ist das kuerzere der beiden Enden (6000 ms ab 0). Liegt der
    // zweite Wurf spaeter als das, ist die Ueberlappung leer.
    assert!(
        abstand < 6000,
        "die beiden Verstaerker liegen {abstand} ms auseinander — SLAGs Fenster \
         (6000 ms) ist dann zu, bevor der zweite wirkt"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);
    let erwartet = &json["expected"];

    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
    );
    assert_eq!(
        ergebnis.accepted,
        u32_at(erwartet, "accepted"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert_eq!(
        ergebnis.final_checksum,
        u32_at(erwartet, "finalChecksum"),
        "die Schluss-Pruefsumme weicht ab"
    );
}

/// Takt fuer Takt, nicht nur am Ende — hier steckt die Stapel-Regel.
///
/// Die Fenstersumme geht in JEDE Takt-Pruefsumme ein. Ein Simulator, der den
/// zweiten Buff verdraengt statt ihn danebenzulegen, weicht ab Takt 150 ab und
/// findet erst nach dem Fensterende wieder zusammen — die Schluss-Pruefsumme
/// allein saehe das womoeglich nicht.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);

    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();

    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
