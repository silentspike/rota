//! #1027 — Vorher-Nachher-Lauf der Fassung B auf dem 10-ms-Takt.
//!
//! **Eine Messung, keine Semantikänderung.** Diese Datei wird mehrmals gebaut:
//! gegen den Bestand (Fassung A) und gegen einen Wegwerf-Patch an
//! `sim/src/apl/mod.rs`, der `switch_point` den Vergleichsoperator durchreicht
//! und den Weckruf auf den ersten 10-ms-Takt legt, an dem der Vergleich
//! tatsächlich kippt (Fassung B). Der Patch wird nicht eingecheckt; er steht
//! mitsamt Befehlen und Zahlen in `docs/APL-STRICT-COMPARISON-2026-10-24.md`.
//!
//! # Aufruf — immer aus der Workspace-Wurzel
//!
//! ```text
//! cargo remote -c -- build --release --example e1027-operator-measure
//! M=./target/release/examples/e1027-operator-measure
//! $M > a.jsonl                                  # messen
//! $M proben > proben-a.jsonl                    # nur die Wurfproben
//! $M kreuz a.jsonl b.jsonl > kreuz-a.jsonl      # feste Listen unter dieser Fassung
//! $M vergleich a.jsonl b.jsonl kreuz-a.jsonl kreuz-b.jsonl
//! $M pfad a.jsonl > pfad-a.jsonl                # Weg der Suche dieser Fassung
//! $M pfadkreuz pfad-a.jsonl pfad-b.jsonl > pfadkreuz-a.jsonl
//! $M pfadvergleich pfad-a.jsonl pfad-b.jsonl pfadkreuz-a.jsonl pfadkreuz-b.jsonl
//! $M nachbarn a.jsonl kreuz-a.jsonl kreuz-b.jsonl
//! ```
//!
//! `pfad` braucht die Messung DIESES Baus (A mit dem A-Bau, B mit dem B-Bau),
//! `kreuz` und `pfadkreuz` laufen mit jedem Bau über dieselben Eingaben.
//!
//! Der Messteil liest `sim/apl/*.apl` und zählt `data/bosses/*.json`. Aus
//! `sim/` heraus bricht er ab, statt mit leerem Bestand zu messen.
//!
//! # Was die Ausgabe enthält
//!
//! | Art | Gegenstand |
//! | --- | --- |
//! | `meta` | Bosszahl aus `BOSSES` und aus `data/bosses/`, Takt, GCD |
//! | `operator` | Wahrheitswert und Weckruf je Form, einen Takt vor, auf und nach dem Umschlag |
//! | `probe` | Würfe je Form gegen die Takt-Referenz |
//! | `raster` | die Aufbauten aus `apl_strikte_vergleiche.rs` — woher ihr Verzug kommt |
//! | `gcd` | die Nachlege-Zeile `< GCD_MS` der Suche, mit und ohne Füller, mit und ohne GCD-freien Zug |
//! | `gcdfrei` | ein GCD-freier Zug vor dem Füller, ganz ohne zeitabhängige Bedingung |
//! | `bestand` | jede Datei aus `sim/apl/` gegen jeden Boss |
//! | `search` | die gierige Suche je Boss in zwei Einstellungen |
//! | `search_le` | dieselbe Suche, die Nachlege-Kandidaten mit `<=` statt `<` |
//!
//! Die Unterbefehle schreiben eigene Arten: `kreuz` (feste Listen), `pfad` (der
//! Weg jeder Suche, Schritt für Schritt) und `pfadkreuz` (die erste Gabelung
//! zweier Wege, samt Takt-Referenz); `vergleich`, `pfadvergleich` und
//! `nachbarn` geben je eine Zusammenfassung aus.
//!
//! **Keine Laufzeiten in der Ausgabe.** Zwei Läufe derselben Fassung müssen
//! bytegleich sein; das ist die Determinismus-Probe, und eine Uhrzeit im
//! Datensatz würde sie unmöglich machen.
//!
//! # Die Takt-Referenz
//!
//! Das Spiel prüft seinen Zustand nur an Takt-Grenzen (`time::TICK_MS`). Ein
//! Simulator, der richtig weckt, muss deshalb dieselben Würfe liefern wie einer,
//! der an JEDEM Takt entscheidet. Diesen zweiten Simulator gibt es nicht als
//! Schnittstelle — `Fight::run_with` ist privat —, wohl aber in der Sprache
//! selbst: Eine angehängte Zeile, die nie greift und für jeden Takt eines
//! Fensters die Gruppe `time >= t and time < 0` trägt, weckt an jedem Takt.
//!
//! Sie hängt an keiner der beiden Fassungen: `>=` auf der steigenden Uhr kippt
//! in A und B bei `t`, und `time < 0` weckt in keiner. `vergleich` prüft das
//! nach, statt es zu glauben — die Referenzwürfe müssen unter beiden Fassungen
//! dieselben sein.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use rota_sim::apl::{ist_spielbar, parse, Apl, AplName, Compare, Condition, Rule, Term, Value};
use rota_sim::balance::{self, BalanceConfig};
use rota_sim::data::{boss_by_id, index_of, Boss, BOSSES, BOSS_COUNT, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::fight::{Fight, FightConfig, RunResult};
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{
    all_candidates, boss_kandidaten, greedy, measure, Candidate, Origin, SearchResult,
};
use rota_sim::time::{GCD_MS, TICK_MS};
use serde_json::{json, Value as Json};

/// Die sechs Formen aus `Compare`, in der Reihenfolge des `enum`.
const FORMEN: [&str; 6] = ["<", "<=", ">", ">=", "==", "!="];

/// Kapazität des Kampfprotokolls. Die Voreinstellung (4096) reicht für einen
/// 300-s-Boss nicht; ein übergelaufenes Protokoll gäbe einen Fingerabdruck
/// über einen Ausschnitt. `ereignis` bricht deshalb bei Überlauf ab.
const PROTOKOLL: usize = 1 << 16;

/// Wie weit vor dem Umschlag die Takt-Referenz beginnt, und wie lange sie läuft.
const FENSTER_VOR: u32 = 500;
const FENSTER_NACH: u32 = 3000;

fn liste(name: &str, text: &str) -> Apl {
    parse(AplName(name), text)
        .unwrap_or_else(|fehler| panic!("Liste {name} ist fehlerhaft:\n{fehler}\n{text}"))
}

fn zauber(id: &str) -> usize {
    index_of(id).unwrap_or_else(|| panic!("Zauber {id} fehlt im Datensatz"))
}

fn boss(id: &str) -> &'static Boss {
    boss_by_id(id).unwrap_or_else(|| panic!("Boss {id} fehlt im Datensatz"))
}

/// Ein Kampf über die volle Zeit (oder `dauer`), ohne Abbruch beim Sieg.
fn kampf(boss: &'static Boss, dauer: Option<u32>) -> Fight {
    Fight::new(FightConfig {
        boss,
        duration_ms: dauer,
        stop_on_kill: false,
        log_capacity: PROTOKOLL,
        ..FightConfig::default()
    })
}

/// Der Wert hinter einem Ausdruck der Sprache, über den Parser gelesen — damit
/// die Trägerprüfung denselben Weg nimmt wie jede Bedingung.
fn wert(ausdruck: &str) -> Value {
    let probe = liste("wert", &format!("kindle if {ausdruck} >= 0\n"));
    probe.rules[0].condition.groups[0][0].left
}

/// Die Zeitpunkte, zu denen `spell` gewirkt wurde.
fn wuerfe(fight: &Fight, spell: usize) -> Vec<u32> {
    assert!(
        !fight.log().overflowed(),
        "Kampfprotokoll übergelaufen — der Fingerabdruck wäre ein Ausschnitt"
    );
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::Cast && usize::from(e.spell) == spell)
        .map(|e| e.time)
        .collect()
}

/// Wie viele Protokolleinträge NICHT auf einer Takt-Grenze liegen.
///
/// Der 1-ms-Versuch vom 2026-09-06 wirkte bei 10.501 und 27.501 ms — Zeitpunkte,
/// an denen das Spiel nicht handeln kann. Diese Zahl muss unter jeder Fassung,
/// die hier gemessen wird, null sein.
fn abseits_vom_takt(fight: &Fight) -> usize {
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.time % TICK_MS != 0)
        .count()
}

const FNV_START: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn protokoll_hash(fight: &Fight) -> String {
    let mut hash = FNV_START;
    for e in fight.log().entries() {
        hash = fnv(&e.time.to_le_bytes(), hash);
        hash = fnv(&[e.kind.number(), e.spell], hash);
        hash = fnv(&e.value.to_le_bytes(), hash);
    }
    format!("{hash:016x}")
}

/// Alles, was ein Kampf an Ergebnis hinterlässt — der Fingerabdruck.
///
/// **Ohne `idle_ms`, und das ist gemessen, nicht vorsichtig.** `fight.rs`
/// zählt beim Warten die Spanne bis zum nächsten BEREITS eingereihten Ereignis;
/// der Weckruf der Entscheidung wird erst danach eingereiht. Jeder Weckruf vor
/// diesem Ereignis zählt die Spanne doppelt. Gemessen am 2026-10-24 an den
/// `gcd`-Datensätzen (`first-crucible`, 30 s): Die Takt-Referenz zu
/// `smoulder if dot.smoulder.remains < 1500` meldet 1 846 410 ms Leerlauf
/// (`idle_ms_reference` von `smoulder/strikt`), und schon Fassung A meldet bei
/// `searing if dot.searing.remains < 1500` 30 000 ms bei fünf gewirkten GCDs
/// (`idle_ms` von `searing/strikt`). Eine Änderung NUR an dieser Zahl ist
/// deshalb keine verschobene Rotation; sie steht getrennt im Datensatz.
fn ergebnis(fight: &Fight, result: &RunResult) -> Json {
    assert!(!fight.log().overflowed(), "Kampfprotokoll übergelaufen");
    let je_zauber = |werte: &[u32]| -> Vec<Json> {
        werte
            .iter()
            .enumerate()
            .filter(|(_, n)| **n != 0)
            .map(|(i, n)| json!([SPELLS[i].id, n]))
            .collect()
    };
    json!({
        "damage": result.damage,
        "killed_at_ms": result.killed_at_ms,
        "died_at_ms": fight.died_at_ms(),
        "damage_taken": fight.damage_taken(),
        "interrupts": fight.interrupts_landed(),
        "casts": je_zauber(&result.casts),
        "lost_dot_ticks": je_zauber(&result.lost_dot_ticks),
        "log_len": fight.log().entries().len(),
        "log_hash": protokoll_hash(fight),
        "checksum": fight.checksum_at(fight.time()),
    })
}

/// Die Gruppen der Takt-Referenz: je Takt in `[von, bis)` einmal
/// `time >= t and time < 0` (siehe Kopfkommentar).
fn takt_gruppen(von: u32, bis: u32) -> Vec<Vec<Term>> {
    assert_eq!(von % TICK_MS, 0, "Referenzfenster beginnt neben dem Takt");
    (von..bis)
        .step_by(TICK_MS as usize)
        .map(|t| {
            vec![
                Term {
                    left: Value::Time,
                    compare: Compare::GreaterEqual,
                    right: Value::Literal(i64::from(t)),
                },
                Term {
                    left: Value::Time,
                    compare: Compare::Less,
                    right: Value::Literal(0),
                },
            ]
        })
        .collect()
}

/// Dieselbe Liste, ergänzt um eine Zeile, die nie greift und an jedem Takt
/// in `[von, bis)` weckt (siehe Kopfkommentar).
fn mit_takt_referenz(apl: &Apl, von: u32, bis: u32) -> Apl {
    let groups = takt_gruppen(von, bis);
    let mut dicht = apl.clone();
    dicht.rules.push(Rule {
        spell: zauber("kindle"),
        condition: Condition { groups },
        line: apl.rules.len() + 1,
    });
    dicht.name = format!("{} + Takt-Referenz", apl.name);
    dicht
}

/// Die Referenzzeile als Kandidat, damit `measure` eine Liste samt Referenz
/// durch dieselbe Formel bewertet wie in der Suche — statt sie nachzurechnen.
fn referenz_kandidat(von: u32, bis: u32) -> Candidate {
    Candidate {
        spell: zauber("kindle"),
        condition: Condition {
            groups: takt_gruppen(von, bis),
        },
        origin: Origin::Always,
    }
}

fn ausgeben(record: &Json) {
    println!(
        "{}",
        serde_json::to_string(record).expect("Datensatz lässt sich schreiben")
    );
}

// ---------------------------------------------------------------------------
// Zustandsproben: Wahrheitswert und Weckruf, einen Takt vor, auf, nach
// ---------------------------------------------------------------------------

/// Ein Wert, an dem die Zustandsproben gemessen werden.
///
/// **Nur Werte, die nicht an der Kampfdauer hängen.** Die Probe hält den Kampf
/// an, indem sie die Dauer auf `t + 1` setzt — `boss.next_strike_in` und
/// `fight.remains` rechnen mit der Restdauer und läsen dort 1. Für sie gibt es
/// nur die Würfe (`probe`), die über die volle Zeit laufen.
struct Zustand {
    wert: &'static str,
    boss: &'static str,
    /// Zeilen vor dem Weckruf. Sie dürfen den GCD zum Probezeitpunkt nicht
    /// sperren — sonst verschiebt `schedule_decision_wakeup` den Weckruf auf
    /// das GCD-Ende, und der Kampf hält nicht bei `t`.
    vorlauf: &'static str,
    schwelle: i64,
    umschlag: u32,
    steigend: bool,
}

const ZUSTAENDE: &[Zustand] = &[
    Zustand {
        wert: "dot.smoulder.remains",
        boss: "first-crucible",
        vorlauf: "smoulder if time == 0\n",
        schwelle: 1500,
        umschlag: 10_500,
        steigend: false,
    },
    Zustand {
        wert: "boss.cast_remains",
        boss: "boss-1-2",
        vorlauf: "purge\n",
        schwelle: 500,
        umschlag: 27_500,
        steigend: false,
    },
    Zustand {
        wert: "cooldown.purge.remains",
        boss: "test-survivable-strike",
        vorlauf: "purge\n",
        schwelle: 1500,
        umschlag: 6000,
        steigend: false,
    },
    Zustand {
        wert: "time",
        boss: "test-survivable-strike",
        vorlauf: "",
        schwelle: 9000,
        umschlag: 9000,
        steigend: true,
    },
];

fn zustandsproben() {
    for z in ZUSTAENDE {
        let traeger = wert(z.wert);
        for t in [z.umschlag - TICK_MS, z.umschlag, z.umschlag + TICK_MS] {
            let aufbau = liste(
                "zustand",
                &format!("{}kindle if time >= {t} and time < 0\n", z.vorlauf),
            );
            let mut fight = kampf(boss(z.boss), Some(t + 1));
            let _ = fight.run_apl(&aufbau);
            assert_eq!(
                fight.time(),
                t,
                "{}: Probezeitpunkt {t} nicht erreicht",
                z.wert
            );
            let gelesen = traeger.read(&fight);
            let erwartet = if z.steigend {
                i64::from(t)
            } else {
                z.schwelle + i64::from(z.umschlag) - i64::from(t)
            };
            assert_eq!(gelesen, erwartet, "{} bei {t}: Träger verändert", z.wert);
            for form in FORMEN {
                for vertauscht in [false, true] {
                    let bedingung = if vertauscht {
                        format!("{} {form} {}", z.schwelle, z.wert)
                    } else {
                        format!("{} {form} {}", z.wert, z.schwelle)
                    };
                    let probe = liste("operator", &format!("kindle if {bedingung}\n"));
                    let holds = probe.rules[0].condition.holds(&fight);
                    let weckruf = probe.next_wakeup(&fight);
                    ausgeben(&json!({
                        "kind": "operator",
                        "id": format!("{}/{t}/{form}/{vertauscht}", z.wert),
                        "group": z.wert,
                        "condition": bedingung,
                        "time_ms": t,
                        "value": gelesen,
                        "holds": holds,
                        "next_wakeup_ms": weckruf,
                        "gcd_ready_at": fight.gcd_ready_at(),
                        "fp": format!("{holds}/{weckruf:?}"),
                    }));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Würfe je Form gegen die Takt-Referenz
// ---------------------------------------------------------------------------

/// Ein Aufbau für die Würfe einer Form.
///
/// **`purge` steht VOR der Probezeile.** Es wird gewirkt, sobald es bereit ist
/// (Abklingzeit 7500 ms), und sein GCD endet so genau auf dem Umschlagpunkt
/// (`test-survivable-strike`: 7500 + 1500 = 9000; `first-crucible`: 9000 +
/// 1500 = 10500). Das ist der Fall der Nebenfrage: Die Rotation entscheidet
/// ohnehin genau dort, wo der strikte Vergleich noch falsch ist.
struct Probe {
    wert: &'static str,
    /// Angehängter Zusatz, etwa `and boss.casting` — ohne ihn trifft
    /// `boss.cast_remains < 500` auch, wenn nichts gewirkt wird.
    zusatz: &'static str,
    boss: &'static str,
    vorlauf: &'static str,
    schwelle: i64,
    umschlag: u32,
}

const PROBEN: &[Probe] = &[
    Probe {
        wert: "boss.next_strike_in",
        zusatz: "",
        boss: "test-survivable-strike",
        vorlauf: "purge\n",
        schwelle: 3000,
        umschlag: 9000,
    },
    Probe {
        wert: "boss.next_strike_in",
        zusatz: "",
        boss: "boss-1-2",
        vorlauf: "purge\n",
        schwelle: 3000,
        umschlag: 7000,
    },
    Probe {
        wert: "dot.smoulder.remains",
        zusatz: "",
        boss: "first-crucible",
        vorlauf: "smoulder if time == 0\npurge\n",
        schwelle: 1500,
        umschlag: 10_500,
    },
    Probe {
        wert: "boss.cast_remains",
        zusatz: " and boss.casting",
        boss: "boss-1-2",
        vorlauf: "purge\n",
        schwelle: 500,
        umschlag: 27_500,
    },
    Probe {
        wert: "cooldown.purge.remains",
        zusatz: "",
        boss: "test-survivable-strike",
        vorlauf: "purge\n",
        schwelle: 1500,
        umschlag: 6000,
    },
    Probe {
        wert: "fight.remains",
        zusatz: "",
        boss: "test-survivable-strike",
        vorlauf: "purge\n",
        schwelle: 3000,
        umschlag: 57_000,
    },
    Probe {
        wert: "time",
        zusatz: "",
        boss: "test-survivable-strike",
        vorlauf: "purge\n",
        schwelle: 9000,
        umschlag: 9000,
    },
];

/// Würfe von `kindle` im Fenster, einmal ereignisgesteuert und einmal gegen
/// die Takt-Referenz.
fn wuerfe_mit_referenz(boss_id: &str, text: &str, von: u32, bis: u32) -> (Json, Json) {
    let kindle = zauber("kindle");
    let ziel = boss(boss_id);
    let im_fenster = |alle: Vec<u32>| -> Vec<u32> {
        alle.into_iter()
            .filter(|t| (von..bis).contains(t))
            .collect()
    };
    let probe = liste("probe", text);

    let mut ereignis = kampf(ziel, None);
    let _ = ereignis.run_apl(&probe);
    let mut referenz = kampf(ziel, None);
    let _ = referenz.run_apl(&mit_takt_referenz(&probe, von, bis));

    // **Trägerprüfung: Das Fenster wird erlebt.** Stirbt der Spieler davor,
    // stehen in beiden Läufen null Würfe — gleich, und ohne Aussage.
    for (lauf, fight) in [("ereignis", &ereignis), ("referenz", &referenz)] {
        assert!(
            fight.died_at_ms().is_none_or(|tod| tod >= bis),
            "{boss_id}: Spieler stirbt im Lauf {lauf} vor dem Fensterende {bis}"
        );
    }
    let e = im_fenster(wuerfe(&ereignis, kindle));
    let r = im_fenster(wuerfe(&referenz, kindle));
    (
        json!({"casts": e, "first": e.first(), "off_grid": abseits_vom_takt(&ereignis)}),
        json!({"casts": r, "first": r.first(), "off_grid": abseits_vom_takt(&referenz)}),
    )
}

fn wurfproben() {
    for p in PROBEN {
        let dauer = boss(p.boss).overpressure_ms;
        let von = p.umschlag - FENSTER_VOR;
        let bis = (p.umschlag + FENSTER_NACH).min(dauer);
        for form in FORMEN {
            for vertauscht in [false, true] {
                let bedingung = if vertauscht {
                    format!("{} {form} {}{}", p.schwelle, p.wert, p.zusatz)
                } else {
                    format!("{} {form} {}{}", p.wert, p.schwelle, p.zusatz)
                };
                let text = format!("{}kindle if {bedingung}\n", p.vorlauf);
                let (ereignis, referenz) = wuerfe_mit_referenz(p.boss, &text, von, bis);
                let gleich = ereignis["casts"] == referenz["casts"];
                ausgeben(&json!({
                    "kind": "probe",
                    "id": format!("{}@{}/{form}/{vertauscht}", p.wert, p.boss),
                    "group": format!("{}@{}", p.wert, p.boss),
                    "apl": text,
                    "window": [von, bis],
                    "event": ereignis,
                    "reference": referenz,
                    "matches_reference": gleich,
                    "fp": ereignis["casts"].to_string(),
                    "fp_reference": referenz["casts"].to_string(),
                }));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Woher der Verzug in `apl_strikte_vergleiche.rs` kommt
// ---------------------------------------------------------------------------

/// Die Aufbauten der bestehenden Prüffälle, ohne deren `time < grenze`, und
/// dieselben ohne `purge`. Unterscheiden sich die beiden Hälften, liegt der
/// Verzug an dem, was `purge` in die Schlange legt.
const RASTER: &[(&str, u32, &str)] = &[
    (
        "test-survivable-strike",
        9000,
        "kindle if boss.next_strike_in <= 3000\npurge\n",
    ),
    (
        "test-survivable-strike",
        9000,
        "kindle if boss.next_strike_in < 3000\npurge\n",
    ),
    (
        "test-survivable-strike",
        9000,
        "kindle if boss.next_strike_in <= 3000\n",
    ),
    (
        "test-survivable-strike",
        9000,
        "kindle if boss.next_strike_in < 3000\n",
    ),
    (
        "boss-1-2",
        7000,
        "kindle if boss.next_strike_in <= 3000\npurge\n",
    ),
    (
        "boss-1-2",
        7000,
        "kindle if boss.next_strike_in < 3000\npurge\n",
    ),
    ("boss-1-2", 7000, "kindle if boss.next_strike_in <= 3000\n"),
    ("boss-1-2", 7000, "kindle if boss.next_strike_in < 3000\n"),
];

fn rasterproben() {
    let purge = zauber("purge");
    for (boss_id, umschlag, text) in RASTER {
        let von = umschlag - FENSTER_VOR;
        let bis = umschlag + FENSTER_NACH;
        let (ereignis, referenz) = wuerfe_mit_referenz(boss_id, text, von, bis);
        let mut fight = kampf(boss(boss_id), None);
        let _ = fight.run_apl(&liste("raster", text));
        let purge_vorher: Vec<u32> = wuerfe(&fight, purge)
            .into_iter()
            .filter(|t| *t < bis)
            .collect();
        ausgeben(&json!({
            "kind": "raster",
            "id": format!("{boss_id}/{}", text.trim_end().replace('\n', " | ")),
            "group": boss_id,
            "apl": text,
            "window": [von, bis],
            "event": ereignis,
            "reference": referenz,
            "purge_casts": purge_vorher,
            "fp": ereignis["casts"].to_string(),
            "fp_reference": referenz["casts"].to_string(),
        }));
    }
}

// ---------------------------------------------------------------------------
// Die Nebenfrage: die Nachlege-Zeile `< GCD_MS` der Suche
// ---------------------------------------------------------------------------

/// Kampfdauer der GCD-Probe. Lang genug für mehrere Nachlege-Runden aller vier
/// Brände, kurz genug, dass die Takt-Referenz über die ganze Zeit läuft.
const GCD_PROBE_MS: u32 = 30_000;

/// Die vier Nachlegezeilen der Suche an `first-crucible`, über 30 s gegen die
/// Takt-Referenz: ohne Füller, mit Füller und mit einem GCD-freien Zug vor dem
/// Füller (QUENCH löst keine Wartezeit aus, SUBLIME befreit den Wurf danach).
/// Die ersten vier Varianten je Brand sind die der ersten Messung und schreiben
/// dieselben Felder wie dort.
fn gcd_proben() {
    let brande: Vec<usize> = (0..SPELLS.len())
        .filter(|&i| SPELLS[i].dot.is_some() && ist_spielbar(&SPELLS[i]))
        .collect();
    // **Dieselbe Menge wie die Nachlege-Kandidaten der Suche** — sonst misst
    // die Nebenfrage etwas anderes als das, wonach sie fragt.
    let kandidaten: Vec<usize> = all_candidates()
        .iter()
        .filter(|k| k.origin == Origin::DotRefresh)
        .map(|k| k.spell)
        .collect();
    assert_eq!(
        brande, kandidaten,
        "Brand-Zauber und Nachlege-Kandidaten weichen ab"
    );

    let ziel = boss("first-crucible");
    let kindle = zauber("kindle");
    for &brand in &brande {
        let id = SPELLS[brand].id;
        // Die letzten vier stellen einen GCD-freien Zug vor den Füller: QUENCH
        // löst keine Wartezeit aus, SUBLIME befreit den Wurf danach. Für sie
        // stehen zusätzlich die Würfe des freien Zugs und des Füllers im
        // Datensatz — die ersten vier bleiben, wie sie waren.
        let varianten = [
            (
                "strikt+fueller",
                format!("{id} if dot.{id}.remains < {GCD_MS}\nkindle\n"),
                None,
            ),
            (
                "gleich+fueller",
                format!("{id} if dot.{id}.remains <= {GCD_MS}\nkindle\n"),
                None,
            ),
            (
                "strikt",
                format!("{id} if dot.{id}.remains < {GCD_MS}\n"),
                None,
            ),
            (
                "gleich",
                format!("{id} if dot.{id}.remains <= {GCD_MS}\n"),
                None,
            ),
            (
                "strikt+quench+fueller",
                format!("{id} if dot.{id}.remains < {GCD_MS}\nquench\nkindle\n"),
                Some(gcd_freier_zug("quench")),
            ),
            (
                "gleich+quench+fueller",
                format!("{id} if dot.{id}.remains <= {GCD_MS}\nquench\nkindle\n"),
                Some(gcd_freier_zug("quench")),
            ),
            (
                "strikt+sublime+fueller",
                format!("{id} if dot.{id}.remains < {GCD_MS}\nsublime\nkindle\n"),
                Some(gcd_freier_zug("sublime")),
            ),
            (
                "gleich+sublime+fueller",
                format!("{id} if dot.{id}.remains <= {GCD_MS}\nsublime\nkindle\n"),
                Some(gcd_freier_zug("sublime")),
            ),
        ];
        for (variante, text, frei) in varianten {
            let probe = liste("gcd", &text);
            // `idle_ms` steht neben dem Ergebnis, nicht darin (siehe `ergebnis`):
            // In der Takt-Referenz zählt es jede Spanne vielfach.
            let messen = |apl: &Apl| -> (Json, u32) {
                let mut fight = kampf(ziel, Some(GCD_PROBE_MS));
                let result = fight.run_apl(apl);
                let mut werte = json!({
                    "dot_casts": wuerfe(&fight, brand),
                    "lost_ticks": result.lost_dot_ticks[brand],
                    "ticks": result.dot_tick_counts[brand],
                    "damage": result.damage,
                    "off_grid": abseits_vom_takt(&fight),
                });
                if let Some(frei) = frei {
                    werte["free_casts"] = json!(wuerfe(&fight, frei));
                    werte["filler_casts"] = json!(wuerfe(&fight, kindle));
                }
                (werte, result.idle_ms)
            };
            let (ereignis, leerlauf) = messen(&probe);
            let (referenz, leerlauf_referenz) = messen(&mit_takt_referenz(&probe, 0, GCD_PROBE_MS));
            ausgeben(&json!({
                "kind": "gcd",
                "id": format!("{id}/{variante}"),
                "group": id,
                "apl": text,
                "event": ereignis,
                "reference": referenz,
                "idle_ms": leerlauf,
                "idle_ms_reference": leerlauf_referenz,
                "matches_reference": ereignis == referenz,
                "fp": ereignis.to_string(),
                "fp_reference": referenz.to_string(),
            }));
        }
    }
}

/// Ein Zauber, nach dessen Wurf keine Wartezeit läuft: `triggersGcd: false`
/// (QUENCH) oder `freesNextGcd` (SUBLIME befreit den Wurf danach). Bricht ab,
/// wenn die Daten das nicht mehr hergeben — sonst mäße die Probe einen
/// gewöhnlichen Zauber und hieße trotzdem „GCD-frei".
fn gcd_freier_zug(id: &str) -> usize {
    let index = zauber(id);
    assert!(
        ist_gcd_frei(index),
        "{id} löst laut Daten eine Wartezeit aus und befreit keinen Wurf"
    );
    index
}

fn ist_gcd_frei(index: usize) -> bool {
    !SPELLS[index].triggers_gcd || SPELLS[index].frees_next_gcd
}

/// **Ein GCD-freier Zug vor dem Füller, ganz ohne zeitabhängige Bedingung.**
///
/// Hier gibt es keinen Weckruf aus `switch_point`, der sich zwischen A und B
/// unterscheiden könnte — die Liste liest keinen Wert. Was die Takt-Referenz
/// hier anders macht als der ereignisgesteuerte Lauf, liegt also nicht an der
/// Fassung, sondern daran, wann nach dem freien Zug überhaupt wieder
/// entschieden wird.
fn gcdfrei_proben() {
    let ziel = boss("first-crucible");
    let kindle = zauber("kindle");
    for id in ["quench", "sublime"] {
        let frei = gcd_freier_zug(id);
        let text = format!("{id}\nkindle\n");
        let probe = liste("gcdfrei", &text);
        let messen = |apl: &Apl| -> (Json, u32) {
            let mut fight = kampf(ziel, Some(GCD_PROBE_MS));
            let result = fight.run_apl(apl);
            let werte = json!({
                "free_casts": wuerfe(&fight, frei),
                "filler_casts": wuerfe(&fight, kindle),
                "damage": result.damage,
                "off_grid": abseits_vom_takt(&fight),
            });
            (werte, result.idle_ms)
        };
        let (ereignis, leerlauf) = messen(&probe);
        let (referenz, leerlauf_referenz) = messen(&mit_takt_referenz(&probe, 0, GCD_PROBE_MS));
        ausgeben(&json!({
            "kind": "gcdfrei",
            "id": format!("{id}+fueller"),
            "group": id,
            "apl": text,
            "event": ereignis,
            "reference": referenz,
            "idle_ms": leerlauf,
            "idle_ms_reference": leerlauf_referenz,
            "matches_reference": ereignis == referenz,
            "fp": ereignis.to_string(),
            "fp_reference": referenz.to_string(),
        }));
    }
}

// ---------------------------------------------------------------------------
// Der Bestand: jede Datei aus sim/apl gegen jeden Boss
// ---------------------------------------------------------------------------

fn apl_dateien() -> Vec<(String, String)> {
    let ordner = Path::new("sim/apl");
    let mut dateien: Vec<_> = fs::read_dir(ordner)
        .unwrap_or_else(|e| {
            panic!(
                "{} nicht lesbar ({e}) — Aufruf aus der Wurzel?",
                ordner.display()
            )
        })
        .filter_map(|eintrag| eintrag.ok().map(|e| e.path()))
        .filter(|pfad| pfad.extension().is_some_and(|ext| ext == "apl"))
        .collect();
    dateien.sort();
    assert!(!dateien.is_empty(), "sim/apl enthält keine .apl-Datei");
    dateien
        .into_iter()
        .map(|pfad| {
            let name = pfad
                .file_name()
                .expect("Dateiname")
                .to_string_lossy()
                .into_owned();
            let text = fs::read_to_string(&pfad).unwrap_or_else(|e| panic!("{name}: {e}"));
            (name, text)
        })
        .collect()
}

/// Die Form bei vertauschten Seiten: `T < v` ist `v > T`.
const fn gespiegelt(form: Compare) -> Compare {
    match form {
        Compare::Less => Compare::Greater,
        Compare::LessEqual => Compare::GreaterEqual,
        Compare::Greater => Compare::Less,
        Compare::GreaterEqual => Compare::LessEqual,
        Compare::Equal => Compare::Equal,
        Compare::NotEqual => Compare::NotEqual,
    }
}

/// Ob ein Vergleich erst NACH dem Erreichen seiner Schwelle von falsch auf
/// wahr kippt: fallend `v < T` und `v != T`, auf der Uhr `time > T` und
/// `time != T`, jeweils auch gespiegelt. Das sind die Formen, deren Würfe in
/// den `probe`-Datensätzen unter Fassung A neben der Takt-Referenz liegen.
///
/// Nur Werte, für die `switch_point` einen Umschlag aus der Schwelle rechnet:
/// fallend die fünf Restzeiten, steigend die Uhr. Stehen auf beiden Seiten
/// zeitabhängige Werte, weckt keine Fassung. Exhaustiv aus demselben Grund
/// wie `switch_point`: Ein neuer Wert muss hier eingeordnet werden.
fn strikt_zeitabhaengig(term: &Term) -> bool {
    let (wert, form) = if term.left.is_time_dependent() && !term.right.is_time_dependent() {
        (term.left, term.compare)
    } else if term.right.is_time_dependent() && !term.left.is_time_dependent() {
        (term.right, gespiegelt(term.compare))
    } else {
        return false;
    };
    match wert {
        Value::DotRemains(_)
        | Value::CooldownRemains(_)
        | Value::FightRemains
        | Value::BossNextStrikeIn
        | Value::BossCastRemains => matches!(form, Compare::Less | Compare::NotEqual),
        Value::Time => matches!(form, Compare::Greater | Compare::NotEqual),
        // Zeitabhängig, aber ohne Umschlag aus der Schwelle — beide Fassungen
        // behandeln sie gleich (`switch_point`).
        Value::CooldownReady(_)
        | Value::DotActive(_)
        | Value::BuffActive(_)
        | Value::BuffRemains(_)
        // Nicht zeitabhängig, oben schon ausgeschlossen.
        | Value::Literal(_)
        | Value::Resource
        | Value::DotTicks(_)
        | Value::BossHp
        | Value::BossHpPercent
        | Value::BossCasting
        | Value::Gcd
        | Value::Decay
        | Value::Shield
        | Value::Control
        | Value::Doom
        | Value::DoomCharges
        | Value::Afflicted
        | Value::RaceProgress => false,
    }
}

/// Wie viele Regeln einen GCD-freien Zug wirken und wie viele Vergleiche eine
/// strikte Form auf einem zeitabhängigen Wert haben — Eigenschaften der Datei,
/// unter jeder Fassung gleich, deshalb nicht im Fingerabdruck.
fn aufbau(apl: &Apl) -> (usize, usize) {
    let frei = apl.rules.iter().filter(|r| ist_gcd_frei(r.spell)).count();
    let strikt = apl
        .rules
        .iter()
        .flat_map(|r| r.condition.groups.iter().flatten())
        .filter(|t| strikt_zeitabhaengig(t))
        .count();
    (frei, strikt)
}

fn bestand(dateien: &[(String, String)]) {
    for (name, text) in dateien {
        let datei_hash = format!("{:016x}", fnv(text.as_bytes(), FNV_START));
        let apl = match parse(AplName(name), text) {
            Ok(apl) => apl,
            Err(fehler) => {
                ausgeben(&json!({
                    "kind": "bestand_fehler",
                    "id": name,
                    "group": "parse",
                    "file_hash": datei_hash,
                    "error": fehler.to_string(),
                    "fp": fehler.to_string(),
                }));
                continue;
            }
        };
        let (gcd_frei, strikt) = aufbau(&apl);
        for ziel in BOSSES.iter() {
            let mut fight = kampf(ziel, None);
            let result = fight.run_apl(&apl);
            let fp = ergebnis(&fight, &result);
            ausgeben(&json!({
                "kind": "bestand",
                "id": format!("{name}@{}", ziel.id),
                "group": ziel.id,
                "file": name,
                "file_hash": datei_hash,
                "gcd_free_lines": gcd_frei,
                "strict_time_terms": strikt,
                "result": fp,
                "idle_ms": result.idle_ms,
                "off_grid": abseits_vom_takt(&fight),
                "fp": fp.to_string(),
            }));
        }
    }
}

// ---------------------------------------------------------------------------
// Die Suche je Boss
// ---------------------------------------------------------------------------

/// Eine Sucheinstellung.
#[derive(Clone, Copy)]
struct Suche {
    id: &'static str,
    iterations: u32,
    prevented_weight: f64,
    max_len: usize,
}

/// **Zwei Einstellungen, beide aus dem Bestand übernommen**, keine eigene:
///
/// - `bericht` wie `config()` in `examples/rotation-search.rs` (50 Durchgänge,
///   Startwert 0, `stop_on_kill: false`, Länge 6) — daher stammen die
///   Balance-Zahlen der Rotationssuche
/// - `kontrolle` wie `konfiguration()` in `tests/ac5_kontrolle_lohnt_sich.rs`
///   (Gewicht 1 auf Verhindertes, Länge 10) — die Einstellung, unter der der
///   1-ms-Versuch die FIXATION-Zeilen verlor
const SUCHEN: [Suche; 2] = [
    Suche {
        id: "bericht",
        iterations: 50,
        prevented_weight: 0.0,
        max_len: 6,
    },
    Suche {
        id: "kontrolle",
        iterations: 1000,
        prevented_weight: 1.0,
        max_len: 10,
    },
];

fn konfiguration(suche: Suche, ziel: &'static Boss) -> BalanceConfig {
    BalanceConfig {
        boss: ziel,
        iterations: suche.iterations,
        base_seed: 0,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        prevented_weight: suche.prevented_weight,
        factors: NEUTRAL,
        ..BalanceConfig::default()
    }
}

/// Der Vorrat, wie `greedy_zuege` ihn bildet: die Kandidaten aus `basis` und
/// dahinter die boss-abhängigen Zeilen aus `boss_kandidaten`.
fn vorrat(config: &BalanceConfig, basis: &[Candidate]) -> Vec<Candidate> {
    let mut alle = basis.to_vec();
    alle.extend(boss_kandidaten(config, basis));
    for (i, a) in alle.iter().enumerate() {
        assert!(
            !alle[i + 1..].contains(a),
            "Kandidat {i} steht doppelt im Vorrat — Indizes wären mehrdeutig"
        );
    }
    alle
}

/// Der Bezugspunkt für „verhindert", wie `greedy_zuege` ihn setzt.
///
/// **Das ist eine Nachbildung**, und deshalb wird sie geprüft: `suchen` misst
/// die gefundene Liste damit nach und verlangt dasselbe Bitmuster wie
/// `SearchResult::best`. Weicht die Nachbildung ab, bricht der Lauf ab, statt
/// die Kreuzbewertung auf einem anderen Bezugspunkt zu rechnen.
fn mit_bezugspunkt(mut config: BalanceConfig) -> BalanceConfig {
    if config.prevented_weight > 0.0 {
        let leer = Apl {
            name: "bezugspunkt".to_owned(),
            rules: Vec::new(),
        };
        config.damage_taken_baseline = balance::run(&config, &leer).damage_taken.mean;
    }
    config
}

/// Ein Kampf mit derselben Umgebung wie die Suche.
fn nachlauf_kampf(config: &BalanceConfig) -> Fight {
    Fight::new(FightConfig {
        boss: config.boss,
        seed: config.base_seed,
        duration_ms: config.duration_ms,
        stop_on_kill: config.stop_on_kill,
        factors: config.factors,
        worn: config.worn,
        log_capacity: PROTOKOLL,
        ..FightConfig::default()
    })
}

/// Der Nachlauf einer Liste, mit derselben Umgebung wie die Suche.
/// Gibt Ergebnis, Einträge abseits des Takts und `idle_ms` zurück.
fn nachlauf(config: &BalanceConfig, apl: &Apl) -> (Json, usize, u32) {
    let mut fight = nachlauf_kampf(config);
    let result = fight.run_apl(apl);
    (
        ergebnis(&fight, &result),
        abseits_vom_takt(&fight),
        result.idle_ms,
    )
}

fn indizes(vorrat: &[Candidate], gewaehlt: &[Candidate]) -> Vec<usize> {
    gewaehlt
        .iter()
        .map(|k| {
            vorrat
                .iter()
                .position(|v| v == k)
                .expect("gewählte Zeile steht nicht im Vorrat")
        })
        .collect()
}

/// Eine Nachlegezeile mit `<=` statt `<`; jede andere Zeile unverändert.
///
/// Bricht ab, wenn eine Nachlegezeile nicht die Form `dot.X.remains < GCD_MS`
/// hat — sonst wäre die Gegenprobe `search_le` still eine halbe.
fn nicht_strikt(kandidat: &Candidate) -> Candidate {
    let mut neu = kandidat.clone();
    if kandidat.origin == Origin::DotRefresh {
        let groups = &mut neu.condition.groups;
        assert!(
            groups.len() == 1
                && groups[0].len() == 1
                && matches!(groups[0][0].left, Value::DotRemains(_))
                && groups[0][0].compare == Compare::Less
                && groups[0][0].right == Value::Literal(i64::from(GCD_MS)),
            "Nachlegezeile hat nicht die Form `dot.X.remains < GCD_MS`"
        );
        groups[0][0].compare = Compare::LessEqual;
    }
    neu
}

/// Die gierige Suche je Boss und Einstellung, ausgehend von `basis`.
///
/// `search` sucht mit den Kandidaten der Suche selbst. `search_le` ist die
/// Gegenprobe zur Frage, woher die Verschiebungen unter Fassung B kommen:
/// dieselbe Suche, nur legen die Nachlegezeilen mit `<=` nach. Die übrigen
/// Kandidatenarten vergleichen mit `>=` auf Werten, die nicht von der Uhr
/// abhängen oder in keiner Fassung einen Umschlag haben, bzw. mit `<=`
/// (`boss.next_strike_in`) — ändert sich unter B auch hier etwas, kommt es
/// von dort.
fn suchen(kind: &str, basis: &[Candidate]) {
    for suche in SUCHEN {
        for ziel in BOSSES.iter() {
            let config = konfiguration(suche, ziel);
            let alle = vorrat(&config, basis);
            let gefunden: SearchResult = greedy(&config, basis, suche.max_len);
            let nachgemessen = measure(&mit_bezugspunkt(config), "gefunden", &gefunden.chosen);
            assert_eq!(
                nachgemessen.mean.to_bits(),
                gefunden.best.mean.to_bits(),
                "{}/{}: Bezugspunkt nicht nachgebildet",
                suche.id,
                ziel.id
            );
            let (nach, abseits, leerlauf) = nachlauf(&config, &gefunden.best.apl);
            let herkunft: Vec<&str> = gefunden.chosen.iter().map(|k| k.origin.label()).collect();
            let nachlegen = gefunden
                .chosen
                .iter()
                .filter(|k| k.origin == Origin::DotRefresh)
                .count();
            let apl_text = gefunden.best.apl.to_string();
            ausgeben(&json!({
                "kind": kind,
                "id": format!("{}/{}", suche.id, ziel.id),
                "group": ziel.id,
                "config": {
                    "search": suche.id, "boss": ziel.id, "iterations": suche.iterations,
                    "seed": 0, "threads": 1, "stop_on_kill": false,
                    "prevented_weight": suche.prevented_weight, "max_len": suche.max_len,
                    "pool_len": gefunden.pool_size, "extended_pool_len": alle.len(),
                },
                "apl": apl_text,
                "chosen": indizes(&alle, &gefunden.chosen),
                "origins": herkunft,
                "dot_refresh_lines": nachlegen,
                "score": gefunden.best.mean,
                "score_bits": gefunden.best.mean.to_bits(),
                "stderr_bits": gefunden.best.stderr.to_bits(),
                "measured": gefunden.measured,
                "stop": format!("{:?}", gefunden.stop),
                "pairs": gefunden.paare_geprueft,
                "triples": gefunden.tripel_geprueft,
                "added": gefunden.ergaenzt,
                "result": nach,
                "idle_ms": leerlauf,
                "off_grid": abseits,
                "fp": json!([apl_text, gefunden.best.mean.to_bits(), nach]).to_string(),
            }));
        }
    }
}

fn messen() {
    let bossdateien = fs::read_dir("data/bosses")
        .unwrap_or_else(|e| panic!("data/bosses nicht lesbar ({e}) — Aufruf aus der Wurzel?"))
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .count();
    assert_eq!(
        bossdateien, BOSS_COUNT,
        "data/bosses zählt anders als der einkompilierte Datensatz — alter Bau?"
    );
    let dateien = apl_dateien();
    ausgeben(&json!({
        "kind": "meta",
        "id": "meta",
        "schema": 2,
        "boss_count": BOSS_COUNT,
        "boss_files": bossdateien,
        "apl_files": dateien.len(),
        "tick_ms": TICK_MS,
        "gcd_ms": GCD_MS,
        "searches": SUCHEN.iter().map(|s| json!({
            "id": s.id, "iterations": s.iterations,
            "prevented_weight": s.prevented_weight, "max_len": s.max_len,
        })).collect::<Vec<_>>(),
        "fp": "meta",
    }));
    zustandsproben();
    wurfproben();
    rasterproben();
    gcd_proben();
    gcdfrei_proben();
    bestand(&dateien);
    let kandidaten = all_candidates();
    suchen("search", &kandidaten);
    let nicht_strikte: Vec<Candidate> = kandidaten.iter().map(nicht_strikt).collect();
    assert_ne!(
        nicht_strikte, kandidaten,
        "Die Gegenprobe `search_le` hat keine Nachlegezeile verändert"
    );
    suchen("search_le", &nicht_strikte);
}

// ---------------------------------------------------------------------------
// kreuz: dieselben Listen unter der Fassung DIESES Baus
// ---------------------------------------------------------------------------

fn jsonl(pfad: &str) -> Vec<Json> {
    fs::read_to_string(pfad)
        .unwrap_or_else(|e| panic!("{pfad}: {e}"))
        .lines()
        .map(|zeile| serde_json::from_str(zeile).unwrap_or_else(|e| panic!("{pfad}: {e}")))
        .collect()
}

fn gewaehlte(record: &Json, alle: &[Candidate]) -> Vec<Candidate> {
    record["chosen"]
        .as_array()
        .expect("chosen fehlt")
        .iter()
        .map(|i| alle[usize::try_from(i.as_u64().expect("Index")).expect("Index")].clone())
        .collect()
}

/// Die Würfe eines Kampfes in ihrer Reihenfolge, als `[Zeitpunkt, Zauber]`.
fn wurfreihe(fight: &Fight) -> Vec<Json> {
    assert!(!fight.log().overflowed(), "Kampfprotokoll übergelaufen");
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| json!([e.time, SPELLS[usize::from(e.spell)].id]))
        .collect()
}

/// Genau die Listen, die der nächste Suchschritt ansähe, in fester Reihenfolge:
/// erst jede Zeile einmal entfernt, dann jeder übrige Kandidat an jeder Stelle
/// eingefügt. `kreuz` bewertet sie, `nachbarn` ordnet sie nach ihrem Aufbau —
/// beide über diese eine Funktion, damit die Reihenfolge, an der die Zuordnung
/// hängt, nicht zweimal gebaut wird.
fn nachbarschaft(liste: &[Candidate], alle: &[Candidate]) -> Vec<Vec<Candidate>> {
    let mut umgebung: Vec<Vec<Candidate>> = Vec::new();
    for i in 0..liste.len() {
        let mut v = liste.to_vec();
        v.remove(i);
        umgebung.push(v);
    }
    for kandidat in alle {
        if liste.contains(kandidat) {
            continue;
        }
        for stelle in 0..=liste.len() {
            let mut v = liste.to_vec();
            v.insert(stelle, kandidat.clone());
            umgebung.push(v);
        }
    }
    umgebung
}

/// Bewertet die gefundenen Listen beider Fassungen und die Nachbarschaft der
/// Liste aus `a` — alles feste Listen, unabhängig davon, was DIESER Bau suchen
/// würde. So wird „dieselbe Rotation zählt anders" von „die Suche wählt eine
/// andere" getrennt.
///
/// Die Nachbarschaft ist genau das, was der nächste Schritt der Suche ansähe:
/// jede Zeile einmal entfernt, jeder übrige Kandidat an jeder Stelle eingefügt.
///
/// Dazu zwei Gegenproben an der A-Liste selbst: dieselbe Liste mit `<=` in den
/// Nachlegezeilen (`a_le_*`) und ohne ihre GCD-freien Züge (`a_ohne_gcdfrei_*`),
/// und die Wurfreihe der A-Liste, an der `vergleich` die erste Abweichung
/// zwischen zwei Fassungen zeigt.
fn kreuz(pfad_a: &str, pfad_b: &str) {
    let suchen_in = |pfad: &str| -> BTreeMap<String, Json> {
        jsonl(pfad)
            .into_iter()
            .filter(|r| r["kind"] == "search")
            .map(|r| (r["id"].as_str().expect("id").to_owned(), r))
            .collect()
    };
    let a = suchen_in(pfad_a);
    let b = suchen_in(pfad_b);
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "die beiden Messungen enthalten verschiedene Suchen"
    );
    for (id, ra) in &a {
        let rb = &b[id];
        let suche = SUCHEN
            .iter()
            .copied()
            .find(|s| ra["config"]["search"] == s.id)
            .expect("unbekannte Sucheinstellung");
        let ziel = boss(ra["config"]["boss"].as_str().expect("boss"));
        let config = konfiguration(suche, ziel);
        let alle = vorrat(&config, &all_candidates());
        let bewertet = mit_bezugspunkt(config);
        let liste_a = gewaehlte(ra, &alle);
        let liste_b = gewaehlte(rb, &alle);
        let wert_von =
            |gewaehlt: &[Candidate]| measure(&bewertet, "kreuz", gewaehlt).mean.to_bits();

        let nachbar_werte: Vec<u64> = nachbarschaft(&liste_a, &alle)
            .iter()
            .map(|v| wert_von(v.as_slice()))
            .collect();
        let apl_aus = |gewaehlt: &[Candidate]| measure(&bewertet, "kreuz", gewaehlt).apl;
        let (nach_a, _, _) = nachlauf(&config, &apl_aus(&liste_a));
        let (nach_b, _, _) = nachlauf(&config, &apl_aus(&liste_b));
        let a_le: Vec<Candidate> = liste_a.iter().map(nicht_strikt).collect();
        let a_ohne_frei: Vec<Candidate> = liste_a
            .iter()
            .filter(|k| !ist_gcd_frei(k.spell))
            .cloned()
            .collect();
        let mut fight = nachlauf_kampf(&config);
        let _ = fight.run_apl(&apl_aus(&liste_a));
        ausgeben(&json!({
            "kind": "kreuz",
            "id": id,
            "group": ziel.id,
            "a_score_bits": wert_von(&liste_a),
            "b_score_bits": wert_von(&liste_b),
            "a_result": nach_a,
            "b_result": nach_b,
            "neighbours": nachbar_werte.len(),
            "neighbour_score_bits": nachbar_werte,
            "a_le_changed_lines": liste_a.iter().zip(&a_le).filter(|(x, y)| x != y).count(),
            "a_le_score_bits": wert_von(&a_le),
            "a_gcd_free_lines": liste_a.len() - a_ohne_frei.len(),
            "a_ohne_gcdfrei_score_bits": wert_von(&a_ohne_frei),
            "a_casts": wurfreihe(&fight),
        }));
    }
}

// ---------------------------------------------------------------------------
// vergleich: zwei Messungen (und optional zwei Kreuzbewertungen)
// ---------------------------------------------------------------------------

fn schluessel(kind: &str, id: &str) -> (String, String) {
    (kind.to_owned(), id.to_owned())
}

fn nach_schluessel(pfad: &str) -> BTreeMap<(String, String), Json> {
    let mut alle = BTreeMap::new();
    for r in jsonl(pfad) {
        let key = (
            r["kind"].as_str().expect("kind").to_owned(),
            r["id"].as_str().expect("id").to_owned(),
        );
        assert!(
            alle.insert(key.clone(), r).is_none(),
            "{pfad}: {key:?} doppelt"
        );
    }
    alle
}

fn abseits_summe(alle: &BTreeMap<(String, String), Json>) -> u64 {
    let mut summe = 0;
    for r in alle.values() {
        for feld in [
            &r["off_grid"],
            &r["event"]["off_grid"],
            &r["reference"]["off_grid"],
        ] {
            summe += feld.as_u64().unwrap_or(0);
        }
    }
    summe
}

/// Der erste Wurf, an dem zwei Wurfreihen auseinandergehen, mit den vier
/// gemeinsamen davor und je drei danach. `None`, wenn beide gleich sind.
fn erste_abweichung(reihe_a: &[Json], reihe_b: &[Json]) -> Option<Json> {
    (0..reihe_a.len().max(reihe_b.len()))
        .find(|&i| reihe_a.get(i) != reihe_b.get(i))
        .map(|i| {
            let ausschnitt = |reihe: &[Json], von: usize, bis: usize| -> Vec<Json> {
                reihe[von.min(reihe.len())..bis.min(reihe.len())].to_vec()
            };
            json!({
                "index": i,
                "common_before": ausschnitt(reihe_a, i.saturating_sub(4), i),
                "before_next": ausschnitt(reihe_a, i, i + 3),
                "after_next": ausschnitt(reihe_b, i, i + 3),
            })
        })
}

#[allow(clippy::too_many_lines)]
fn vergleich(pfad_a: &str, pfad_b: &str, kreuz_pfade: Option<(&str, &str)>) {
    let a = nach_schluessel(pfad_a);
    let b = nach_schluessel(pfad_b);
    let nur_a: Vec<_> = a.keys().filter(|k| !b.contains_key(*k)).take(10).collect();
    let nur_b: Vec<_> = b.keys().filter(|k| !a.contains_key(*k)).take(10).collect();
    assert!(
        nur_a.is_empty() && nur_b.is_empty(),
        "Fallmengen verschieden — nur vorher {nur_a:?}, nur nachher {nur_b:?}"
    );
    assert_eq!(
        a[&schluessel("meta", "meta")],
        b[&schluessel("meta", "meta")]
    );

    let mut je_art: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    // Fälle, in denen sich NUR `idle_ms` ändert — siehe `ergebnis`.
    let mut nur_leerlauf: BTreeMap<String, usize> = BTreeMap::new();
    let mut operator_wechsel = Vec::new();
    let mut wahrheit_geaendert = Vec::new();
    let mut proben = Vec::new();
    let mut referenz_geaendert = Vec::new();
    let mut gcd = Vec::new();
    let mut raster = Vec::new();
    let mut bestand_je_boss: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    // (GCD-freier Zug in der Datei, strikte zeitabhängige Form in der Datei)
    // -> (Dateien, Läufe, geänderte Läufe, Dateien mit geändertem Lauf)
    type Feld = (BTreeSet<String>, usize, usize, BTreeSet<String>);
    let mut bestand_nach_aufbau: BTreeMap<(bool, bool), Feld> = BTreeMap::new();
    let mut bestand_geaendert = Vec::new();
    let mut suchen_tabelle = Vec::new();
    let mut suchen_le_tabelle = Vec::new();

    for (key, alt) in &a {
        let neu = &b[key];
        let geaendert = alt["fp"] != neu["fp"];
        let eintrag = je_art.entry(key.0.clone()).or_default();
        eintrag.0 += 1;
        eintrag.1 += usize::from(geaendert);
        if !geaendert && alt.get("idle_ms") != neu.get("idle_ms") {
            *nur_leerlauf.entry(key.0.clone()).or_default() += 1;
        }
        if alt.get("fp_reference") != neu.get("fp_reference") {
            referenz_geaendert.push(key.1.clone());
        }
        match key.0.as_str() {
            "operator" => {
                if alt["holds"] != neu["holds"] {
                    wahrheit_geaendert.push(key.1.clone());
                }
                if geaendert {
                    operator_wechsel.push(json!({
                        "id": key.1, "condition": alt["condition"], "value": alt["value"],
                        "holds": alt["holds"], "gcd_ready_at": alt["gcd_ready_at"],
                        "before": alt["next_wakeup_ms"], "after": neu["next_wakeup_ms"],
                    }));
                }
            }
            "probe" => proben.push(json!({
                "id": key.1, "group": alt["group"], "changed": geaendert,
                "before_first": alt["event"]["first"], "after_first": neu["event"]["first"],
                "reference_first": alt["reference"]["first"],
                "before_matches": alt["matches_reference"], "after_matches": neu["matches_reference"],
                "before_casts": alt["event"]["casts"], "after_casts": neu["event"]["casts"],
                "reference_casts": alt["reference"]["casts"],
            })),
            "raster" => raster.push(json!({
                "id": key.1, "before_first": alt["event"]["first"],
                "after_first": neu["event"]["first"], "reference_first": alt["reference"]["first"],
                "purge_casts": alt["purge_casts"],
            })),
            "gcd" | "gcdfrei" => gcd.push(json!({
                "kind": key.0, "id": key.1, "before": alt["event"], "after": neu["event"],
                "reference": alt["reference"],
                "before_matches": alt["matches_reference"], "after_matches": neu["matches_reference"],
                "idle_ms": [alt["idle_ms"], neu["idle_ms"]],
                "idle_ms_reference": [alt["idle_ms_reference"], neu["idle_ms_reference"]],
            })),
            "bestand" => {
                let zeile = bestand_je_boss
                    .entry(alt["group"].as_str().expect("group").to_owned())
                    .or_default();
                zeile.0 += 1;
                zeile.1 += usize::from(geaendert);
                let aufbau = (
                    alt["gcd_free_lines"].as_u64().expect("gcd_free_lines") > 0,
                    alt["strict_time_terms"].as_u64().expect("strict_time_terms") > 0,
                );
                let feld = bestand_nach_aufbau.entry(aufbau).or_default();
                feld.0.insert(alt["file"].as_str().expect("file").to_owned());
                feld.1 += 1;
                feld.2 += usize::from(geaendert);
                if geaendert {
                    feld.3.insert(alt["file"].as_str().expect("file").to_owned());
                }
                if geaendert {
                    bestand_geaendert.push(json!({
                        "id": key.1,
                        "damage": [alt["result"]["damage"], neu["result"]["damage"]],
                        "killed_at_ms": [alt["result"]["killed_at_ms"], neu["result"]["killed_at_ms"]],
                        "died_at_ms": [alt["result"]["died_at_ms"], neu["result"]["died_at_ms"]],
                        "damage_taken": [alt["result"]["damage_taken"], neu["result"]["damage_taken"]],
                    }));
                }
            }
            "search" => suchen_tabelle.push(json!({
                "id": key.1, "search": alt["config"]["search"], "boss": alt["group"],
                "apl_changed": alt["apl"] != neu["apl"],
                "score_changed": alt["score_bits"] != neu["score_bits"],
                "result_changed": geaendert,
                "score_before": alt["score"], "score_after": neu["score"],
                "dot_refresh_lines": [alt["dot_refresh_lines"], neu["dot_refresh_lines"]],
                "measured": [alt["measured"], neu["measured"]],
                "apl_before": alt["apl"], "apl_after": neu["apl"],
                "died_at_ms": [alt["result"]["died_at_ms"], neu["result"]["died_at_ms"]],
                "interrupts": [alt["result"]["interrupts"], neu["result"]["interrupts"]],
            })),
            "search_le" => suchen_le_tabelle.push(json!({
                "id": key.1, "changed": geaendert,
                "apl_changed": alt["apl"] != neu["apl"],
                "score": [alt["score"], neu["score"]],
                "apl_before": alt["apl"], "apl_after": neu["apl"],
            })),
            _ => {}
        }
    }

    let mut kreuz_tabelle = Vec::new();
    if let Some((pfad_ka, pfad_kb)) = kreuz_pfade {
        let ka = nach_schluessel(pfad_ka);
        let kb = nach_schluessel(pfad_kb);
        assert_eq!(
            ka.keys().collect::<Vec<_>>(),
            kb.keys().collect::<Vec<_>>(),
            "Kreuzbewertungen über verschiedene Fälle"
        );
        for (key, alt) in &ka {
            let neu = &kb[key];
            let suche = &a[&schluessel("search", &key.1)];
            let suche_b = &b[&schluessel("search", &key.1)];
            // Die Kreuzbewertung unter der EIGENEN Fassung muss die Suche
            // bitgleich treffen — sonst bewertet sie etwas anderes.
            assert_eq!(
                alt["a_score_bits"], suche["score_bits"],
                "{key:?}: A unter A"
            );
            assert_eq!(
                neu["b_score_bits"], suche_b["score_bits"],
                "{key:?}: B unter B"
            );
            let werte_a = alt["neighbour_score_bits"].as_array().expect("Nachbarn");
            let werte_b = neu["neighbour_score_bits"].as_array().expect("Nachbarn");
            assert_eq!(
                werte_a.len(),
                werte_b.len(),
                "{key:?}: Nachbarschaft verschieden groß"
            );
            let verschoben = werte_a.iter().zip(werte_b).filter(|(x, y)| x != y).count();
            let wert = |r: &Json, feld: &str| f64::from_bits(r[feld].as_u64().expect("bits"));
            // Die ersten Nachbarn sind die Entfernungen, in der Reihenfolge der
            // Zeilen (siehe `kreuz`). Welche Zeile muss weg, damit beide
            // Fassungen dieselbe Zahl liefern?
            let zeilen: Vec<&str> = suche["apl"]
                .as_str()
                .expect("apl")
                .lines()
                .filter(|z| !z.trim().is_empty())
                .collect();
            assert_eq!(
                zeilen.len(),
                suche["chosen"].as_array().expect("chosen").len(),
                "{key:?}: Zeilen und gewählte Kandidaten zählen verschieden"
            );
            let entfernt_gleich: Vec<&str> = zeilen
                .iter()
                .enumerate()
                .filter(|(i, _)| werte_a[*i] == werte_b[*i])
                .map(|(_, z)| *z)
                .collect();
            let reihe_a = alt["a_casts"].as_array().expect("a_casts");
            let reihe_b = neu["a_casts"].as_array().expect("a_casts");
            let abweichung = erste_abweichung(reihe_a, reihe_b);
            kreuz_tabelle.push(json!({
                "id": key.1,
                "a_list_score_changed": alt["a_score_bits"] != neu["a_score_bits"],
                "a_list_result_changed": alt["a_result"] != neu["a_result"],
                "b_list_score_changed": alt["b_score_bits"] != neu["b_score_bits"],
                "a_list_score": [wert(alt, "a_score_bits"), wert(neu, "a_score_bits")],
                "neighbours": werte_a.len(),
                "neighbours_changed": verschoben,
                "removals_equal": entfernt_gleich,
                "le_changed_lines": alt["a_le_changed_lines"],
                "le_score": [wert(alt, "a_le_score_bits"), wert(neu, "a_le_score_bits")],
                "le_score_changed": alt["a_le_score_bits"] != neu["a_le_score_bits"],
                "gcd_free_lines": alt["a_gcd_free_lines"],
                "without_gcd_free_score": [wert(alt, "a_ohne_gcdfrei_score_bits"),
                                           wert(neu, "a_ohne_gcdfrei_score_bits")],
                "without_gcd_free_score_changed":
                    alt["a_ohne_gcdfrei_score_bits"] != neu["a_ohne_gcdfrei_score_bits"],
                "casts_changed": reihe_a != reihe_b,
                "first_divergence": abweichung,
            }));
        }
    }

    let zusammenfassung = json!({
        "before": pfad_a,
        "after": pfad_b,
        "meta": a[&schluessel("meta", "meta")],
        "records_per_kind": je_art.iter().map(|(k, (n, g))| json!({"kind": k, "total": n, "changed": g})).collect::<Vec<_>>(),
        "off_grid": {"before": abseits_summe(&a), "after": abseits_summe(&b)},
        "idle_only_changes": nur_leerlauf,
        "truth_changed": wahrheit_geaendert,
        "reference_changed": referenz_geaendert,
        "operator_wakeup_changes": operator_wechsel,
        "probes": proben,
        "raster": raster,
        "gcd": gcd,
        "bestand_per_boss": bestand_je_boss.iter().map(|(k, (n, g))| json!({"boss": k, "total": n, "changed": g})).collect::<Vec<_>>(),
        "bestand_by_structure": bestand_nach_aufbau.iter().map(|((frei, strikt), (dateien, n, g, betroffen))| json!({
            "gcd_free_line": frei, "strict_time_term": strikt,
            "files": dateien.len(), "runs": n, "changed": g,
            "files_changed": betroffen.iter().collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "bestand_changed": bestand_geaendert,
        "searches": suchen_tabelle,
        "searches_le": suchen_le_tabelle,
        "kreuz": kreuz_tabelle,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&zusammenfassung).expect("Zusammenfassung")
    );
}

// ---------------------------------------------------------------------------
// pfad: der Weg der Suche, Schritt für Schritt
// ---------------------------------------------------------------------------

/// Ein Füller: eine unbedingte Zeile eines Zaubers, der spätestens nach einer
/// globalen Wartezeit wieder bereit ist (KINDLE, BIND, WISP …).
///
/// Steht einer in der Liste, wird an jedem GCD-Ende gewirkt: Die Rotation
/// wartet nie, und ein Weckruf zwischen zwei GCD-Enden fällt in einen GCD. Das
/// ist die Lage „mit Füller" der `gcd`-Proben, hier aus den Zauberdaten
/// bestimmt statt am Namen KINDLE.
fn ist_fueller(kandidat: &Candidate) -> bool {
    kandidat.condition.is_always() && SPELLS[kandidat.spell].cooldown_ms <= GCD_MS
}

/// Der Aufbau einer Kandidatenliste — Eigenschaften der Liste, unter jeder
/// Fassung gleich: GCD-freie Züge, Füller, strikte Zeitbedingungen.
fn aufbau_liste(liste: &[Candidate]) -> Json {
    let namen = |passt: fn(&Candidate) -> bool| -> Vec<&'static str> {
        liste
            .iter()
            .filter(|k| passt(k))
            .map(|k| SPELLS[k.spell].id)
            .collect()
    };
    json!({
        "gcd_free": namen(|k| ist_gcd_frei(k.spell)),
        "fillers": namen(ist_fueller),
        "strict_time_terms": liste
            .iter()
            .flat_map(|k| k.condition.groups.iter().flatten())
            .filter(|t| strikt_zeitabhaengig(t))
            .count(),
    })
}

/// Der Weg der Suche: `greedy` mit wachsender Längengrenze, 1 bis `max_len`.
///
/// **Die Bibliothek selbst, kein Nachbau.** `greedy(n)` hört auf, sobald die
/// Liste n Zeilen hat. Trägt bis dahin in jeder Runde ein Einzelzug, ist das
/// genau der Stand nach n Runden der vollen Suche: Paar- und Gruppenzüge kommen
/// nur zum Zug, wenn kein Einzelzug trägt, und die Grenze greift erst nach dem
/// Zug. Ob das gilt, steht in jedem Datensatz — `extends` heißt, die Liste bei
/// n ist die bei n - 1 mit genau einer eingefügten Zeile. Wo ein Schritt das
/// nicht ist, beschreibt der Datensatz die Suche mit dieser Grenze, nicht mehr
/// den Weg der vollen.
///
/// Bei n = `max_len` muss der Weg die Suche aus der übergebenen Messung
/// bitgleich treffen, Liste und Suchwert; sonst bricht der Lauf ab. Die Messung
/// muss also mit DIESEM Bau entstanden sein.
fn pfad(pfad_messung: &str) {
    let kandidaten = all_candidates();
    for r in jsonl(pfad_messung).iter().filter(|r| r["kind"] == "search") {
        let id = r["id"].as_str().expect("id");
        let suche = SUCHEN
            .iter()
            .copied()
            .find(|s| r["config"]["search"] == s.id)
            .expect("unbekannte Sucheinstellung");
        let ziel = boss(r["config"]["boss"].as_str().expect("boss"));
        let config = konfiguration(suche, ziel);
        let alle = vorrat(&config, &kandidaten);
        let mut vorher: Vec<Candidate> = Vec::new();
        for n in 1..=suche.max_len {
            let gefunden = greedy(&config, &kandidaten, n);
            let erweitert = gefunden.chosen.len() == vorher.len() + 1
                && (0..gefunden.chosen.len()).any(|i| {
                    let mut ohne = gefunden.chosen.clone();
                    ohne.remove(i);
                    ohne == vorher
                });
            let gewaehlt = indizes(&alle, &gefunden.chosen);
            if n == suche.max_len {
                assert!(
                    json!(gewaehlt) == r["chosen"]
                        && json!(gefunden.best.mean.to_bits()) == r["score_bits"],
                    "{id}: der Weg endet nicht bei der Suche der Messung — anderer Bau?"
                );
            }
            ausgeben(&json!({
                "kind": "pfad",
                "id": format!("{id}/{n:02}"),
                "group": id,
                "search": suche.id,
                "boss": ziel.id,
                "n": n,
                "len": gefunden.chosen.len(),
                "chosen": gewaehlt,
                "apl": gefunden.best.apl.to_string(),
                "score": gefunden.best.mean,
                "score_bits": gefunden.best.mean.to_bits(),
                "stop": format!("{:?}", gefunden.stop),
                "extends": erweitert,
                "pairs": gefunden.paare_geprueft,
                "triples": gefunden.tripel_geprueft,
                "measured": gefunden.measured,
            }));
            vorher = gefunden.chosen;
        }
    }
}

/// Die Datensätze aus `pfad`, je Suche in der Reihenfolge der Grenze.
fn pfade(datei: &str) -> BTreeMap<String, Vec<Json>> {
    let mut alle: BTreeMap<String, Vec<Json>> = BTreeMap::new();
    for r in jsonl(datei).into_iter().filter(|r| r["kind"] == "pfad") {
        let schritte = alle
            .entry(r["group"].as_str().expect("group").to_owned())
            .or_default();
        assert_eq!(
            r["n"],
            json!(schritte.len() + 1),
            "{datei}: Schritte nicht lückenlos in Folge"
        );
        schritte.push(r);
    }
    alle
}

/// Der erste Schritt, an dem zwei Wege verschiedene Listen haben.
fn gabelung(weg_a: &[Json], weg_b: &[Json]) -> Option<usize> {
    assert_eq!(weg_a.len(), weg_b.len(), "Wege verschieden lang");
    (0..weg_a.len()).find(|&i| weg_a[i]["chosen"] != weg_b[i]["chosen"])
}

/// Die erste Gabelung zweier Suchwege, bewertet unter der Fassung DIESES Baus.
///
/// An der Gabelung stehen zwei Listen aus derselben Nachbarschaft: die, die A
/// wählt, und die, die B wählt. Unter A ist die erste mindestens so viel wert
/// wie die zweite, unter B umgekehrt — zwischen den Fassungen ändert sich also
/// der Wert mindestens einer der beiden. Welche, sagt `pfadvergleich`. Hier
/// stehen je Liste ihr Wert, ihre Wurfreihe und dieselbe Liste gegen die
/// Takt-Referenz über den ganzen Kampf, bewertet durch `measure` wie in der
/// Suche.
fn pfadkreuz(pfad_a: &str, pfad_b: &str) {
    let a = pfade(pfad_a);
    let b = pfade(pfad_b);
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "die beiden Wege enthalten verschiedene Suchen"
    );
    let kandidaten = all_candidates();
    for (gruppe, weg_a) in &a {
        let weg_b = &b[gruppe];
        let Some(i) = gabelung(weg_a, weg_b) else {
            ausgeben(&json!({"kind": "pfadkreuz", "id": gruppe, "diverges": false}));
            continue;
        };
        let suche = SUCHEN
            .iter()
            .copied()
            .find(|s| weg_a[i]["search"] == s.id)
            .expect("unbekannte Sucheinstellung");
        let ziel = boss(weg_a[i]["boss"].as_str().expect("boss"));
        let config = konfiguration(suche, ziel);
        assert!(
            config.duration_ms.is_none(),
            "die Referenz deckt nur die volle Kampfzeit ab"
        );
        let alle = vorrat(&config, &kandidaten);
        let bewertet = mit_bezugspunkt(config);
        let referenz = referenz_kandidat(0, ziel.overpressure_ms);
        let lauf = |apl: &Apl| -> (Vec<Json>, Option<u32>, usize) {
            let mut fight = nachlauf_kampf(&config);
            let _ = fight.run_apl(apl);
            (
                wurfreihe(&fight),
                fight.died_at_ms(),
                abseits_vom_takt(&fight),
            )
        };
        let bewerte = |schritt: &Json| -> Json {
            let liste = gewaehlte(schritt, &alle);
            let gemessen = measure(&bewertet, "pfadkreuz", &liste);
            let mut mit_referenz = liste.clone();
            mit_referenz.push(referenz.clone());
            let gegen_referenz = measure(&bewertet, "pfadkreuz", &mit_referenz);
            let (wuerfe, tod, abseits) = lauf(&gemessen.apl);
            let (wuerfe_referenz, tod_referenz, abseits_referenz) = lauf(&gegen_referenz.apl);
            json!({
                "apl": gemessen.apl.to_string(),
                "structure": aufbau_liste(&liste),
                "score": gemessen.mean,
                "score_bits": gemessen.mean.to_bits(),
                "reference_score": gegen_referenz.mean,
                "reference_score_bits": gegen_referenz.mean.to_bits(),
                "matches_reference": wuerfe == wuerfe_referenz,
                "casts": wuerfe,
                "reference_casts": wuerfe_referenz,
                "died_at_ms": [tod, tod_referenz],
                "off_grid": [abseits, abseits_referenz],
            })
        };
        let schritt = |s: &Json| {
            json!({"len": s["len"], "stop": s["stop"], "extends": s["extends"],
                   "pairs": s["pairs"], "triples": s["triples"]})
        };
        let gemeinsam = if i == 0 {
            json!([])
        } else {
            weg_a[i - 1]["chosen"].clone()
        };
        ausgeben(&json!({
            "kind": "pfadkreuz",
            "id": gruppe,
            "diverges": true,
            "step": weg_a[i]["n"],
            "common": gemeinsam,
            "a_step": schritt(&weg_a[i]),
            "b_step": schritt(&weg_b[i]),
            "a_list": bewerte(&weg_a[i]),
            "b_list": bewerte(&weg_b[i]),
        }));
    }
}

/// Ob eine Abweichung zweier Wurfreihen direkt auf einen GCD-freien Wurf folgt:
/// Der letzte gemeinsame Wurf ist QUENCH selbst (`triggersGcd: false`) oder
/// der Wurf, den ein SUBLIME unmittelbar davor befreit hat (`freesNextGcd`).
///
/// **Abgeleitet, nicht vom Kampf gemeldet** — aus der Wurfreihe und den
/// Zauberdaten. Der Kampf führt keinen Merker im Protokoll, der sagt, ob ein
/// Wurf befreit war.
fn nach_gcd_freiem_wurf(abweichung: &Json) -> Option<bool> {
    let gemeinsam = abweichung["common_before"].as_array()?;
    let zauber_von = |eintrag: &Json| zauber(eintrag[1].as_str().expect("Zauber"));
    let letzter = zauber_von(gemeinsam.last()?);
    let befreit =
        gemeinsam.len() >= 2 && SPELLS[zauber_von(&gemeinsam[gemeinsam.len() - 2])].frees_next_gcd;
    Some(!SPELLS[letzter].triggers_gcd || befreit)
}

/// `erste_abweichung`, dazu die Lage davor: ob sie direkt auf einen GCD-freien
/// Wurf folgt (`nach_gcd_freiem_wurf`) und wie lange jede der beiden Reihen
/// nach dem letzten gemeinsamen Wurf bis zu ihrem nächsten wartet.
fn abweichung_mit_lage(reihe_a: &[Json], reihe_b: &[Json]) -> Option<Json> {
    let mut abweichung = erste_abweichung(reihe_a, reihe_b)?;
    let frei = nach_gcd_freiem_wurf(&abweichung);
    let zeit = |eintrag: Option<&Json>| eintrag.and_then(|e| e[0].as_u64());
    let letzter = zeit(
        abweichung["common_before"]
            .as_array()
            .and_then(|g| g.last()),
    );
    let warten = |feld: &str| -> Option<u64> {
        zeit(abweichung[feld].as_array().and_then(|r| r.first()))?.checked_sub(letzter?)
    };
    let gewartet = json!([warten("before_next"), warten("after_next")]);
    let objekt = abweichung
        .as_object_mut()
        .expect("Abweichung ist ein Objekt");
    objekt.insert("after_gcd_free_cast".to_owned(), json!(frei));
    objekt.insert("wait_ms".to_owned(), gewartet);
    Some(abweichung)
}

/// Je Gabelung: welche der beiden Listen zwischen den Fassungen ihren Wert
/// ändert, wie sie gebaut ist und ob A oder B dort die Takt-Referenz trifft.
///
/// Prüft dabei dreierlei, statt es zu glauben: Unter der eigenen Fassung trifft
/// die Bewertung den Weg bitgleich, die Referenz fällt unter A und B gleich aus,
/// und jede Suche mit verschobenem Ergebnis hat eine Gabelung.
///
/// `reference_prefers` vergleicht nur die beiden Listen der Gabelung gegen die
/// Takt-Referenz — welche die Referenz höher bewertet, nicht welche Liste eine
/// Suche gegen die Referenz wählen würde; dafür müsste die ganze Nachbarschaft
/// gegen sie laufen.
#[allow(clippy::too_many_lines)]
fn pfadvergleich(pfad_a: &str, pfad_b: &str, kreuz_a: &str, kreuz_b: &str) {
    let wege_a = pfade(pfad_a);
    let wege_b = pfade(pfad_b);
    let ka = nach_schluessel(kreuz_a);
    let kb = nach_schluessel(kreuz_b);
    assert_eq!(
        wege_a.keys().collect::<Vec<_>>(),
        wege_b.keys().collect::<Vec<_>>(),
        "die beiden Wege enthalten verschiedene Suchen"
    );
    let bits = |r: &Json| f64::from_bits(r.as_u64().expect("bits"));
    let leer = |r: &Json, feld: &str| r[feld].as_array().expect("Liste").is_empty();
    let mut faelle = Vec::new();
    // Lage der Liste, deren Wert sich an der Gabelung ändert, in den Suchen mit
    // verschobenem Ergebnis.
    let mut lagen: BTreeMap<String, usize> = BTreeMap::new();
    // Welche der beiden Listen der Gabelung die Takt-Referenz höher bewertet.
    let mut referenz_wahl: BTreeMap<&str, usize> = BTreeMap::new();
    let mut gegabelt = 0;
    let mut verschoben = 0;
    for (gruppe, weg_a) in &wege_a {
        let weg_b = &wege_b[gruppe];
        let ende_verschoben =
            weg_a.last().expect("Weg")["chosen"] != weg_b.last().expect("Weg")["chosen"];
        let ra = &ka[&schluessel("pfadkreuz", gruppe)];
        let rb = &kb[&schluessel("pfadkreuz", gruppe)];
        let Some(i) = gabelung(weg_a, weg_b) else {
            assert!(
                !ende_verschoben,
                "{gruppe}: Ergebnis verschoben ohne Gabelung"
            );
            assert!(
                ra["diverges"].as_bool() == Some(false) && rb["diverges"].as_bool() == Some(false),
                "{gruppe}: pfadkreuz sieht eine Gabelung, die die Wege nicht haben"
            );
            continue;
        };
        gegabelt += 1;
        verschoben += usize::from(ende_verschoben);
        assert!(ra["step"] == weg_a[i]["n"] && rb["step"] == weg_a[i]["n"]);
        assert_eq!(
            ra["a_list"]["score_bits"], weg_a[i]["score_bits"],
            "{gruppe}: Bewertung der A-Liste unter A trifft den Weg nicht"
        );
        assert_eq!(
            rb["b_list"]["score_bits"], weg_b[i]["score_bits"],
            "{gruppe}: Bewertung der B-Liste unter B trifft den Weg nicht"
        );
        let liste = |seite: &str| -> Json {
            let (x, y) = (&ra[seite], &rb[seite]);
            for feld in ["apl", "reference_casts", "reference_score_bits"] {
                assert_eq!(
                    x[feld], y[feld],
                    "{gruppe}/{seite}: {feld} unter A und B verschieden"
                );
            }
            let reihe = |r: &Json, feld: &str| r[feld].as_array().expect("Würfe").clone();
            json!({
                "apl": x["apl"],
                "structure": x["structure"],
                "score": [bits(&x["score_bits"]), bits(&y["score_bits"])],
                "reference_score": bits(&x["reference_score_bits"]),
                "changed": x["score_bits"] != y["score_bits"],
                "casts_changed": x["casts"] != y["casts"],
                "a_matches_reference": x["matches_reference"],
                "b_matches_reference": y["matches_reference"],
                "first_divergence_a_b": abweichung_mit_lage(&reihe(x, "casts"), &reihe(y, "casts")),
                "first_divergence_a_reference":
                    abweichung_mit_lage(&reihe(x, "casts"), &reihe(x, "reference_casts")),
                "first_divergence_b_reference":
                    abweichung_mit_lage(&reihe(y, "casts"), &reihe(y, "reference_casts")),
                "died_at_ms": [x["died_at_ms"], y["died_at_ms"]],
            })
        };
        let liste_a = liste("a_list");
        let liste_b = liste("b_list");
        let referenz_a = liste_a["reference_score"].as_f64().expect("Referenzwert");
        let referenz_b = liste_b["reference_score"].as_f64().expect("Referenzwert");
        let bevorzugt = if referenz_a > referenz_b {
            "a"
        } else if referenz_b > referenz_a {
            "b"
        } else {
            "tie"
        };
        if ende_verschoben {
            *referenz_wahl.entry(bevorzugt).or_default() += 1;
            for l in [&liste_a, &liste_b] {
                if l["changed"].as_bool() != Some(true) {
                    continue;
                }
                let nach_frei = |feld: &str| l[feld]["after_gcd_free_cast"].clone();
                let lage = format!(
                    "gcd_free={} filler={} a_b_after_free={} a=reference={} \
                     a_reference_after_free={} b=reference={} b_reference_after_free={}",
                    !leer(&l["structure"], "gcd_free"),
                    !leer(&l["structure"], "fillers"),
                    nach_frei("first_divergence_a_b"),
                    l["a_matches_reference"],
                    nach_frei("first_divergence_a_reference"),
                    l["b_matches_reference"],
                    nach_frei("first_divergence_b_reference"),
                );
                *lagen.entry(lage).or_default() += 1;
            }
        }
        let weg = |w: &[Json]| -> Vec<Json> {
            w.iter()
                .map(|s| {
                    json!({"n": s["n"], "len": s["len"], "stop": s["stop"], "extends": s["extends"],
                           "triples": s["triples"], "score": s["score"], "apl": s["apl"]})
                })
                .collect()
        };
        faelle.push(json!({
            "id": gruppe,
            "final_shifted": ende_verschoben,
            "step": weg_a[i]["n"],
            "reference_prefers": bevorzugt,
            "a_step": ra["a_step"],
            "b_step": ra["b_step"],
            "a_list": liste_a,
            "b_list": liste_b,
            "a_path": weg(weg_a),
            "b_path": weg(weg_b),
        }));
    }
    let zusammenfassung = json!({
        "paths": [pfad_a, pfad_b],
        "searches": wege_a.len(),
        "diverging": gegabelt,
        "final_shifted": verschoben,
        "changed_lists_in_shifted_searches": lagen,
        "reference_prefers_in_shifted_searches": referenz_wahl,
        "cases": faelle,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&zusammenfassung).expect("Zusammenfassung")
    );
}

/// Die Nachbarschaft jeder A-Liste nach Aufbau, mit den Werten aus `kreuz`
/// unter A und unter B: Ändert B den Wert nur dort, wo ein GCD-freier Zug
/// steht oder kein Füller?
///
/// Die Nachbarn entstehen in `nachbarschaft`, derselben Funktion, mit der
/// `kreuz` sie bewertet hat; ihre Zahl muss je Suche mit `kreuz`
/// übereinstimmen. Dazu dieselbe Einteilung für die A-Listen selbst.
fn nachbarn(pfad_messung: &str, kreuz_a: &str, kreuz_b: &str) {
    let messung = nach_schluessel(pfad_messung);
    let ka = nach_schluessel(kreuz_a);
    let kb = nach_schluessel(kreuz_b);
    let kandidaten = all_candidates();
    let klasse = |liste: &[Candidate]| -> (bool, bool) {
        (
            liste.iter().any(|k| ist_gcd_frei(k.spell)),
            liste.iter().any(ist_fueller),
        )
    };
    type Klassen = BTreeMap<(bool, bool), (usize, usize)>;
    let zaehle = |klassen: &mut Klassen, k: (bool, bool), geaendert: bool| {
        let eintrag = klassen.entry(k).or_default();
        eintrag.0 += 1;
        eintrag.1 += usize::from(geaendert);
    };
    let als_json = |klassen: &Klassen| -> Vec<Json> {
        klassen
            .iter()
            .map(|((frei, fueller), (n, geaendert))| {
                json!({"gcd_free": frei, "filler": fueller, "total": n, "changed": geaendert})
            })
            .collect()
    };
    let mut alle_nachbarn = Klassen::new();
    let mut a_listen = Klassen::new();
    let mut je_suche = Vec::new();
    for ((art, id), r) in &messung {
        if art != "search" {
            continue;
        }
        let suche = SUCHEN
            .iter()
            .copied()
            .find(|s| r["config"]["search"] == s.id)
            .expect("unbekannte Sucheinstellung");
        let config = konfiguration(suche, boss(r["config"]["boss"].as_str().expect("boss")));
        let alle = vorrat(&config, &kandidaten);
        let liste_a = gewaehlte(r, &alle);
        let xa = &ka[&schluessel("kreuz", id)];
        let xb = &kb[&schluessel("kreuz", id)];
        zaehle(
            &mut a_listen,
            klasse(&liste_a),
            xa["a_score_bits"] != xb["a_score_bits"],
        );
        let werte_a = xa["neighbour_score_bits"].as_array().expect("Nachbarn");
        let werte_b = xb["neighbour_score_bits"].as_array().expect("Nachbarn");
        let umgebung = nachbarschaft(&liste_a, &alle);
        assert!(
            umgebung.len() == werte_a.len() && werte_a.len() == werte_b.len(),
            "{id}: Nachbarschaft zählt anders als in kreuz"
        );
        let mut eigene = Klassen::new();
        for (liste, (x, y)) in umgebung.iter().zip(werte_a.iter().zip(werte_b)) {
            zaehle(&mut alle_nachbarn, klasse(liste), x != y);
            zaehle(&mut eigene, klasse(liste), x != y);
        }
        je_suche.push(json!({"id": id, "neighbours": als_json(&eigene)}));
    }
    let zusammenfassung = json!({
        "inputs": [pfad_messung, kreuz_a, kreuz_b],
        "neighbours": als_json(&alle_nachbarn),
        "a_lists": als_json(&a_listen),
        "per_search": je_suche,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&zusammenfassung).expect("Zusammenfassung")
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => messen(),
        // Nur die Wurfproben, für eine Fassung, unter der die Zustandsproben
        // gar nicht anhalten: Sie halten den Kampf mit `time >= t` an, und
        // ohne Weckruf für `time` (Mutation MT) erreicht er `t` nie.
        ["proben"] => wurfproben(),
        ["kreuz", a, b] => kreuz(a, b),
        ["vergleich", a, b] => vergleich(a, b, None),
        ["vergleich", a, b, ka, kb] => vergleich(a, b, Some((ka, kb))),
        ["pfad", messung] => pfad(messung),
        ["pfadkreuz", a, b] => pfadkreuz(a, b),
        ["pfadvergleich", a, b, ka, kb] => pfadvergleich(a, b, ka, kb),
        ["nachbarn", messung, ka, kb] => nachbarn(messung, ka, kb),
        _ => panic!(
            "Aufruf: e1027-operator-measure [proben | kreuz A B | vergleich A B [KREUZ-A KREUZ-B] \
             | pfad A | pfadkreuz PFAD-A PFAD-B \
             | pfadvergleich PFAD-A PFAD-B PFADKREUZ-A PFADKREUZ-B \
             | nachbarn A KREUZ-A KREUZ-B]"
        ),
    }
}
