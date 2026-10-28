//! Gemeinsames Handwerkszeug der Listen-Aufzeichnungen (Issue #1027).
//!
//! Eine Listen-Aufzeichnung stammt aus der Schleife des Spiels
//! (`driveRotation` in `src/devtools/reference-run.ts`) und trägt je Lauf eine
//! Prioritätenliste als Text, die Drücke, die die Schleife daraus gewählt hat,
//! und deren Leerlauf. Erzeugt werden sie in `tests/parity/payloads.ts`
//! (`listenPayload`); heute sind es `gcd-freier-wurf.json` und
//! `strikter-umschlag.json`.
//!
//! Geprüft wird zweierlei, und beides steht hier genau einmal:
//!
//! | Prüfung | Gegenstand | Frage |
//! | --- | --- | --- |
//! | [`das_abspielen_trifft_das_spiel`] | die Drücke | rechnen beide Seiten dieselben Regeln? |
//! | [`entscheidet_wie_im_spiel`] | die Liste als Text | entscheidet der Simulator **wann** das Spiel entscheidet? |
//!
//! **Eine Stelle für beide Dateien**, damit zwei Aufzeichnungen nicht zwei
//! Abschriften desselben Vergleichs tragen, die auseinanderlaufen.

#![allow(dead_code)]

use serde_json::Value;

use rota_sim::apl::{parse, AplName};
use rota_sim::data::{boss_by_id, index_of, SPELLS, SPELL_COUNT};
use rota_sim::log::LogKind;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Liest die Aufzeichnung und prüft ihre Fassung.
pub fn data(text: &str) -> Value {
    let value: Value = serde_json::from_str(text).expect("Aufzeichnung ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

pub fn u32_at(node: &Value, key: &str) -> u32 {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    u32::try_from(raw).expect("passt nicht in 32 Bit")
}

pub fn lauf<'a>(json: &'a Value, name: &str) -> &'a Value {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|run| run["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("Lauf {name} fehlt in der Aufzeichnung"))
}

/// Die Namen aller Läufe, in der Reihenfolge der Datei.
pub fn laeufe(json: &Value) -> Vec<&str> {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .map(|run| run["name"].as_str().expect("name fehlt"))
        .collect()
}

pub fn bindings(json: &Value) -> Vec<Binding> {
    json["bindings"]
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
        .collect()
}

pub fn recording(json: &Value, run: &Value) -> Recording {
    let presses = run["presses"]
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
        name: run["name"].as_str().expect("name fehlt").to_owned(),
        seed: u32_at(json, "seed"),
        ticks: u32_at(json, "ticks"),
        bindings: bindings(json),
        presses,
    }
}

/// Die Drücke eines Laufs als (Zeitpunkt, Zauber-Index) — in der Reihenfolge der
/// Aufzeichnung.
pub fn druecke(json: &Value, run: &Value) -> Vec<(u32, usize)> {
    let leiste = bindings(json);
    recording(json, run)
        .presses
        .iter()
        .map(|p| {
            let spell = leiste
                .iter()
                .find(|b| b.slot == p.slot)
                .map(|b| b.spell)
                .unwrap_or_else(|| panic!("Platz {} liegt auf keinem Zauber", p.slot));
            (p.tick * TICK_MS, spell)
        })
        .collect()
}

/// Ein Kampf wie die Schleife: volle Zeit, kein Abbruch beim Sieg.
pub fn kampf(json: &Value) -> Fight {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    Fight::new(FightConfig {
        seed: u32_at(json, "seed"),
        duration_ms: Some(u32_at(json, "ticks") * TICK_MS),
        boss: boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt")),
        stop_on_kill: false,
        log_capacity: 1 << 14,
        ..FightConfig::default()
    })
}

pub fn name_von(spell: usize) -> &'static str {
    SPELLS[spell].id
}

/// Die Regeln: Spielen beide Seiten dieselben Drücke gleich ab?
///
/// Das Abspielen entscheidet nichts. Steht hier rot, liegt es an einer Regel,
/// nicht an der Entscheidung, und [`entscheidet_wie_im_spiel`] sagt über den
/// Zeitpunkt nichts mehr.
pub fn das_abspielen_trifft_das_spiel(json: &Value, laeufe: &[&str]) {
    for &name in laeufe {
        let run = lauf(json, name);
        let aufzeichnung = recording(json, run);
        let mut fight = kampf(json);
        let ergebnis = fight.replay(&aufzeichnung, true);
        let erwartet = &run["expected"];

        let pruefsummen: Vec<u32> = run["checksums"]
            .as_array()
            .expect("checksums fehlen")
            .iter()
            .map(|v| u32::try_from(v.as_u64().expect("keine Zahl")).expect("zu gross"))
            .collect();
        assert_eq!(
            first_divergence(&pruefsummen, &ergebnis.checksums, TICK_MS),
            None,
            "{name}: Simulator und Spiel laufen beim Abspielen auseinander"
        );
        assert_eq!(
            ergebnis.damage,
            u32_at(erwartet, "damage"),
            "{name}: Schaden"
        );
        assert_eq!(
            ergebnis.accepted,
            u32_at(erwartet, "accepted"),
            "{name}: angenommen"
        );
        assert_eq!(ergebnis.rejected, 0, "{name}: abgelehnte Drücke");
        assert_eq!(
            ergebnis.final_checksum,
            u32_at(erwartet, "finalChecksum"),
            "{name}: Schluss-Prüfsumme"
        );
    }
}

/// Fährt die Liste des Laufs durch den Simulator und hält Wurfreihe, Schaden und
/// Leerlauf gegen die Aufzeichnung.
///
/// Die Wurfreihe zuerst, und mit der ersten Abweichung in der Meldung: Eine
/// abweichende Schadenszahl sagt „irgendwo", der erste verschiedene Wurf sagt,
/// nach welchem Druck die Entscheidung auseinanderläuft.
pub fn entscheidet_wie_im_spiel(json: &Value, name: &str) {
    let run = lauf(json, name);
    let text = run["apl"].as_str().expect("apl fehlt");
    let apl = parse(AplName(name), text)
        .unwrap_or_else(|fehler| panic!("{name}: Liste ist fehlerhaft:\n{fehler}\n{text}"));
    let mut fight = kampf(json);
    let ergebnis = fight.run_apl(&apl);
    assert!(
        !fight.log().overflowed(),
        "{name}: Kampfprotokoll übergelaufen"
    );

    let simulator: Vec<(u32, usize)> = fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| (e.time, usize::from(e.spell)))
        .collect();
    let spiel = druecke(json, run);

    if let Some(stelle) =
        (0..simulator.len().max(spiel.len())).find(|&i| simulator.get(i) != spiel.get(i))
    {
        let zeige = |reihe: &[(u32, usize)]| -> String {
            reihe
                .iter()
                .skip(stelle.saturating_sub(2))
                .take(4)
                .map(|&(t, s)| format!("{t} {}", name_von(s)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        panic!(
            "{name}: Wurf {stelle} weicht ab.\n  Spiel:     {}\n  Simulator: {}",
            zeige(&spiel),
            zeige(&simulator)
        );
    }

    let erwartet = &run["expected"];
    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "{name}: Schaden"
    );
    let mut casts = [0_u32; SPELL_COUNT];
    for &(_, spell) in &spiel {
        casts[spell] += 1;
    }
    assert_eq!(ergebnis.casts, casts, "{name}: Wirkungen je Zauber");
    assert_eq!(
        ergebnis.idle_ms,
        u32_at(erwartet, "idleMs"),
        "{name}: Leerlauf — der Simulator zählt die Wartezeit anders als die Schleife des Spiels"
    );
}
