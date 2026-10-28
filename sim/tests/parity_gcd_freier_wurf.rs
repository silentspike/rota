//! Nach einem Wurf ohne Wartezeit entscheidet die Liste im nächsten Takt
//! (Issue #1027).
//!
//! # Was hier verglichen wird
//!
//! `gcd-freier-wurf.json` stammt aus der Schleife von `runReference`
//! (`driveRotation` in `src/devtools/reference-run.ts`): eine Entscheidung je
//! Takt, nur bei bereiter globaler Wartezeit. Nach QUENCH — oder nach dem Wurf,
//! den SUBLIME befreit — bleibt die Wartezeit bereit, und die Liste entscheidet
//! im nächsten Takt neu.
//!
//! Die Aufzeichnung trägt dafür zweierlei, und hier wird beides geprüft:
//!
//! | Prüfung | Gegenstand | Frage |
//! | --- | --- | --- |
//! | Abspielen (`Fight::replay`) | die Drücke | rechnen beide Seiten dieselben Regeln? |
//! | Entscheiden (`Fight::run_apl`) | die Liste als Text | entscheidet der Simulator **wann** das Spiel entscheidet? |
//!
//! Das Abspielen allein genügte nicht: Es wirkt die Drücke zu den
//! aufgezeichneten Takten und kann deshalb gar nicht später entscheiden. Bis
//! #1027 tat der Simulator genau das — nach einem freien Wurf erst am nächsten
//! eingereihten Ereignis, an `first-crucible` mit QUENCH und Füller 1990 ms zu
//! spät (Füller bei 2000 statt 10), und jede Aufzeichnung blieb grün.
//!
//! # Der Leerlauf ist Teil des Vergleichs
//!
//! `idleMs` zählt die Schleife je Takt, in dem gewirkt werden durfte und die
//! Liste nichts wählte; der Simulator zählt dieselbe Zeit als Spanne bis zur
//! nächsten Entscheidung. Beide Seiten müssen gleich rechnen — sonst ist
//! `RunResult::idle_ms` eine Zahl, die nur so aussieht wie die des Spiels.

use serde_json::Value;

use rota_sim::apl::{parse, AplName};
use rota_sim::data::{boss_by_id, index_of, SPELLS, SPELL_COUNT};
use rota_sim::log::LogKind;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/gcd-freier-wurf.json");

/// Die Läufe, die hier je einen eigenen Prüffall haben. `jeder_lauf_hat_einen_prueffall`
/// hält die Liste gegen die Aufzeichnung — ein neuer Lauf ohne Fall fiele sonst
/// stumm aus dem Vergleich.
const LAEUFE: [&str; 6] = [
    "quench-fueller",
    "quench-nachlegen-strikt",
    "quench-auf-der-schwelle",
    "quench-ohne-fueller",
    "sublime-befreit",
    "warten-auf-die-uhr",
];

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("gcd-freier-wurf.json ist kein JSON");
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

fn lauf<'a>(json: &'a Value, name: &str) -> &'a Value {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|run| run["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("Lauf {name} fehlt in der Aufzeichnung"))
}

fn bindings(json: &Value) -> Vec<Binding> {
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

fn recording(json: &Value, run: &Value) -> Recording {
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
fn druecke(json: &Value, run: &Value) -> Vec<(u32, usize)> {
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
fn kampf(json: &Value) -> Fight {
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

fn name_von(spell: usize) -> &'static str {
    SPELLS[spell].id
}

/// **Die Zusicherung vor der Messung.** Ein Lauf ohne freien Wurf belegt für
/// diese Frage nichts.
///
/// Gezählt wird nicht der Zauber, sondern die Wirkung: zwei Drücke einen Takt
/// auseinander. Das geht nur, wenn der erste keine Wartezeit ausgelöst hat —
/// ob über QUENCH oder über SUBLIME, ist dann gleichgültig. Die Gegenprobe ohne
/// freien Wurf muss null haben, sonst prüfte diese Zählung etwas anderes.
#[test]
fn jeder_lauf_mit_freiem_wurf_entscheidet_einen_takt_danach() {
    let json = data();
    for name in LAEUFE {
        let reihe = druecke(&json, lauf(&json, name));
        let folge = reihe
            .windows(2)
            .filter(|paar| paar[1].0 == paar[0].0 + TICK_MS)
            .count();
        if name == "warten-auf-die-uhr" {
            assert_eq!(folge, 0, "{name}: die Gegenprobe enthält einen freien Wurf");
        } else {
            assert!(
                folge > 0,
                "{name}: kein Druck einen Takt nach einem anderen — der Lauf trägt \
                 die Lage nicht, die er prüfen soll"
            );
        }
    }
}

/// Die genaue Lage der 14 Gabelungen aus dem Bericht: QUENCH fällt auf ein
/// GCD-Ende, an dem die strikte Nachlegezeile genau auf ihrer Schwelle steht.
///
/// **Aus der Aufzeichnung gerechnet, nicht abgeschrieben**: der letzte
/// SMOULDER-Druck vor QUENCH plus seine Laufzeit aus den Zauberdaten, minus der
/// Zeitpunkt von QUENCH. Stimmte das nicht, prüfte `quench-auf-der-schwelle`
/// eine Nachlegezeile, die längst wahr oder noch weit weg ist.
#[test]
fn quench_faellt_genau_auf_die_schwelle_der_nachlegezeile() {
    let json = data();
    let reihe = druecke(&json, lauf(&json, "quench-auf-der-schwelle"));
    let quench = index_of("quench").expect("quench fehlt");
    let smoulder = index_of("smoulder").expect("smoulder fehlt");
    let laufzeit = SPELLS[smoulder]
        .dot
        .expect("smoulder ist ein Brand")
        .duration_ms;

    let (stelle, &(geworfen, _)) = reihe
        .iter()
        .enumerate()
        .find(|(_, (_, spell))| *spell == quench)
        .expect("QUENCH wird nie gedrückt");
    let zuletzt_gelegt = reihe[..stelle]
        .iter()
        .rev()
        .find(|(_, spell)| *spell == smoulder)
        .map(|&(t, _)| t)
        .expect("vor QUENCH liegt kein SMOULDER");

    assert_eq!(
        zuletzt_gelegt + laufzeit - geworfen,
        1500,
        "SMOULDER steht bei QUENCH ({geworfen} ms) nicht genau auf 1500"
    );
    assert_eq!(
        reihe[stelle + 1],
        (geworfen + TICK_MS, smoulder),
        "das Spiel legt nicht einen Takt nach QUENCH nach"
    );
}

/// Die Regeln: Spielen beide Seiten dieselben Drücke gleich ab?
///
/// Grün vor und nach #1027 — das Abspielen entscheidet nichts. Steht hier rot,
/// liegt es an einer Regel, nicht an der Entscheidung, und die Fälle darunter
/// sagen über den Zeitpunkt nichts mehr.
#[test]
fn das_abspielen_trifft_das_spiel_in_jedem_takt() {
    let json = data();
    for name in LAEUFE {
        let run = lauf(&json, name);
        let aufzeichnung = recording(&json, run);
        let mut fight = kampf(&json);
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
fn entscheidet_wie_im_spiel(name: &str) {
    let json = data();
    let run = lauf(&json, name);
    let text = run["apl"].as_str().expect("apl fehlt");
    let apl = parse(AplName(name), text)
        .unwrap_or_else(|fehler| panic!("{name}: Liste ist fehlerhaft:\n{fehler}\n{text}"));
    let mut fight = kampf(&json);
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
    let spiel = druecke(&json, run);

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

#[test]
fn quench_fueller_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("quench-fueller");
}

#[test]
fn quench_nachlegen_strikt_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("quench-nachlegen-strikt");
}

#[test]
fn quench_auf_der_schwelle_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("quench-auf-der-schwelle");
}

#[test]
fn quench_ohne_fueller_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("quench-ohne-fueller");
}

#[test]
fn sublime_befreit_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("sublime-befreit");
}

#[test]
fn warten_auf_die_uhr_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("warten-auf-die-uhr");
}

/// Die Liste oben gegen die Aufzeichnung, in beide Richtungen.
#[test]
fn jeder_lauf_hat_einen_prueffall() {
    let json = data();
    let in_der_datei: Vec<&str> = json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .map(|run| run["name"].as_str().expect("name fehlt"))
        .collect();
    assert_eq!(
        in_der_datei, LAEUFE,
        "Läufe der Aufzeichnung und Prüffälle dieser Datei passen nicht zusammen"
    );
}
