//! Beide Seiten mischen dieselben Werte in derselben Reihenfolge (Issue #236).
//!
//! # Was hier schiefgehen kann, und warum es keiner merkt
//!
//! Die Prüfsumme entsteht an zwei Orten: `combatChecksum` in
//! `src/core/checksum.ts` und `Fight::checksum_at` in `sim/src/fight.rs`. Beide
//! rühren dieselben Einzelwerte nacheinander ein. Die **Reihenfolge** geht in
//! das Ergebnis ein — FNV-1a ist kein Sortierverfahren.
//!
//! Hier stand eine Zahl ("vierunddreißig Werte"), und Issue #480 hat sie
//! veralten lassen: `nextCastFreeGcd` ist der fünfunddreißigste. Sie stand in
//! keiner Bedingung, also blieb der Lauf grün und der Satz wurde still falsch.
//! Die Zahl trägt an dieser Stelle nichts — der Satz sagt dasselbe ohne sie,
//! und die vier Merker-Zauber nach SUBLIME lassen sie ohnehin weiterwandern.
//!
//! Wer auf einer Seite ein Feld ergänzt und auf der anderen vergisst, bekommt
//! keine Fehlermeldung, sondern eine Zahl. Sie ist falsch, aber sie sieht aus
//! wie jede andere. Bis #236 hielt die beiden Listen nichts zusammen ausser
//! Aufmerksamkeit — und die Vergleichsdateien meldeten dann zwar eine
//! Abweichung, aber ohne zu sagen, **wo**.
//!
//! # Warum das ein Wächter bleibt und keine gemeinsame Quelle wird
//!
//! Richtig wäre eine Liste, aus der beide Seiten entstehen. Das hiesse
//! Codegenerierung über zwei Sprachen — für gut drei Dutzend Felder ein
//! Aufwand, der den Fehler teurer macht als er ist. Dieser Wächter kostet eine
//! Datei und meldet dieselbe Sorte Fehler, nur später und mit Namen. Das ist
//! die bewusste Wahl, nicht die bequeme.
//!
//! # Der Name geht NICHT in die Prüfsumme ein
//!
//! `mix_named_int` mischt genau das, was `mix_int` mischen würde. Ginge der
//! Name mit ein, änderte sich jede Prüfsumme im Projekt und jede eingecheckte
//! Vergleichsdatei wäre falsch. Belegt ist das unten und auf der TS-Seite in
//! `tests/unit/checksum-named.test.ts`.

use serde_json::Value;

use rota_sim::checksum::record_field_order;
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Der Lauf, an dem gemessen wird — derselbe wie in `parity_doom_stacks.rs`.
///
/// **Nicht ein frischer Kampf.** `checksum_at` mischt die Zerfallsstapel in
/// einer Schleife; bei einem Kampf, der noch nichts erlebt hat, ist sie leer,
/// der Name `decayExpiry` erschiene auf keiner Seite, und der Wächter wäre
/// blind für genau die Stelle, an der eine Reihenfolge am ehesten
/// auseinanderläuft. Der Zähler-Kampf hat am Ende Stapel stehen.
const DOOM_JSON: &str = include_str!("../../tests/parity/recordings/doom-stacks.json");

fn data() -> Value {
    serde_json::from_str(DOOM_JSON).expect("doom-stacks.json ist kein JSON")
}

fn recording(json: &Value) -> Recording {
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

/// Die Namensfolge dieser Seite, am selben Zustand wie das Spiel sie misst.
fn rust_folge() -> Vec<&'static str> {
    let json = data();
    let recording = recording(&json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let final_time = (recording.ticks - 1) * TICK_MS;
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let _ = fight.replay(&recording, true);
    record_field_order(|| {
        let _ = fight.checksum_at(final_time);
    })
}

/// Die Namensfolge des Spiels, wie der Export sie aufgezeichnet hat.
fn spiel_folge() -> Vec<String> {
    data()["fieldOrder"]
        .as_array()
        .expect(
            "In doom-stacks.json fehlt `fieldOrder`. Die Datei stammt aus einem Stand vor \
             Issue #236 — `npx vite-node tests/parity/export-parity.mjs` erzeugt sie neu.",
        )
        .iter()
        .map(|v| v.as_str().expect("Name ist keine Zeichenkette").to_owned())
        .collect()
}

#[test]
fn beide_seiten_messen_ueberhaupt_etwas() {
    // Der Gegenfall zuerst: Zwei leere Listen sind Position für Position
    // gleich. Ohne diese Prüfung wäre der Wächter unten grün, wenn die
    // Aufzeichnung gar nicht anspringt — die stillste Art, nichts zu prüfen.
    let rust = rust_folge();
    let spiel = spiel_folge();
    assert!(rust.len() > 50, "Rust misst nur {} Namen", rust.len());
    assert!(
        spiel.len() > 50,
        "das Spiel misst nur {} Namen",
        spiel.len()
    );

    // Und die Schleifen-Stelle, wegen der dieser Lauf gewählt ist.
    assert!(
        rust.contains(&"decayExpiry"),
        "Der Kampf hat am Ende keine Zerfallsstapel. Dann prüft dieser Waechter \
         die Schleife nicht — ein anderer Lauf muss her."
    );
}

#[test]
fn die_folgen_sind_gleich_lang() {
    // **Eine eigene Meldung, nicht die Abweichungs-Meldung unten.** Unterschiedlich
    // lange Folgen heissen: auf einer Seite fehlt ein Feld oder steht eines zu
    // viel. Wer das als "an Position 34 steht hier X und dort nichts" meldet,
    // schickt den Leser an die falsche Stelle — die Reihenfolge kann bis dahin
    // vollkommen in Ordnung sein.
    let rust = rust_folge();
    let spiel = spiel_folge();
    assert_eq!(
        rust.len(),
        spiel.len(),
        "Die Prüfsummen mischen unterschiedlich viele Werte: Rust {}, Spiel {}. \
         Auf einer Seite ist ein Feld ergänzt oder entfernt worden, auf der anderen nicht. \
         Rust-Ende: {:?} / Spiel-Ende: {:?}",
        rust.len(),
        spiel.len(),
        &rust[rust.len().saturating_sub(3)..],
        &spiel[spiel.len().saturating_sub(3)..],
    );
}

#[test]
fn beide_seiten_mischen_in_derselben_reihenfolge() {
    let rust = rust_folge();
    let spiel = spiel_folge();
    let bis = rust.len().min(spiel.len());

    for i in 0..bis {
        assert_eq!(
            rust[i], spiel[i],
            "Position {i}: Rust mischt {:?}, das Spiel {:?}. \
             Die Reihenfolge geht in die Pruefsumme ein — ab hier rechnen beide Seiten \
             verschieden. Zu vergleichen sind `Fight::checksum_at` in sim/src/fight.rs \
             und `combatChecksum` in src/core/checksum.ts.",
            rust[i], spiel[i],
        );
    }
}

#[test]
fn der_name_veraendert_die_pruefsumme_nicht() {
    // Die Zusicherung, auf der alles ruht. Wäre sie falsch, hätte #236 jede
    // eingecheckte Vergleichsdatei ungültig gemacht — und zwar unbemerkt,
    // weil die Zahlen ja weiterhin auf beiden Seiten übereinstimmten.
    use rota_sim::checksum::{mix_float, mix_int, mix_named_float, mix_named_int, FNV_OFFSET};

    for value in [0_i32, 1, -1, 40510, i32::MAX] {
        assert_eq!(
            mix_named_int(FNV_OFFSET, "kurz", value),
            mix_int(FNV_OFFSET, value)
        );
        assert_eq!(
            mix_named_int(FNV_OFFSET, "ein deutlich laengerer Name", value),
            mix_int(FNV_OFFSET, value),
        );
    }
    for value in [0.0_f64, 1.0, -1.0, 4240.0, 0.5] {
        assert_eq!(
            mix_named_float(FNV_OFFSET, "kurz", value),
            mix_float(FNV_OFFSET, value)
        );
    }
}

#[test]
fn ausserhalb_der_aufzeichnung_wird_nichts_gesammelt() {
    // GAME-DESIGN 18.4: keine Speicheranforderung im Schritt. Im Kampf läuft
    // keine Aufzeichnung, also darf auch keine Liste wachsen. Belegt wird das
    // über die Umkehrung: Ein Aufruf zwischen zwei Aufzeichnungen taucht in
    // keiner von beiden auf.
    use rota_sim::checksum::{mix_named_int, FNV_OFFSET};

    let erste = record_field_order(|| {
        let _ = mix_named_int(FNV_OFFSET, "drinnen", 1);
    });
    let _ = mix_named_int(FNV_OFFSET, "draussen", 2);
    let zweite = record_field_order(|| {
        let _ = mix_named_int(FNV_OFFSET, "wieder drinnen", 3);
    });

    assert_eq!(erste, vec!["drinnen"]);
    assert_eq!(zweite, vec!["wieder drinnen"]);
}
