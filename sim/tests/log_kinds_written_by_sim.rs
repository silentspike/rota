//! **Die `sim`-Spalte gegen die Wirklichkeit** — schreibt der Simulator, was
//! die Absicht von ihm behauptet? (Issue #689)
//!
//! # Warum es diesen Fall gibt
//!
//! `EVENT_KIND_INTENT` in `src/core/log.ts` trägt seit Issue #676 zwei
//! Spalten: `game` und `sim`. Geprüft wurde nur die erste. Gemessen am
//! 2026-08-09, mit der Probe, die den Unterschied zwischen „ungedeckt" und
//! „wirkungslos" zeigt:
//!
//! ```text
//!   ALLE 17 sim-Werte umgedreht — true wird false, false wird true
//!     tsc --noEmit               sauber
//!     vitest run                 189 Dateien, 1967 Fälle, 0 rot
//!     cargo test --all-targets   110 Suiten, 110 fehlerfrei
//! ```
//!
//! Die vollständige Umkehrung der Aussage, welche Arten der Simulator
//! schreibt, bewegte **keinen einzigen Fall**. Das Feld war nicht ungedeckt,
//! es war dekorativ.
//!
//! # Warum der Fall hier steht und nicht in `parity_log_kinds_written.rs`
//!
//! Jener Fall (#151) vergleicht die Anzahl je Art und iteriert dabei über
//! **die Arten, die der Simulator tatsächlich geschrieben hat**:
//!
//! ```text
//!   for (art, anzahl) in &simulator      parity_log_kinds_written.rs
//! ```
//!
//! Hört eine Seite auf, eine Art zu schreiben, fällt diese Art aus der
//! Sammlung — und damit aus der Schleife. **Ein Vergleich, der nur über das
//! läuft, was eine Seite schreibt, kann das Aufhören dieser Seite nicht
//! bemerken.** Dieser Fall geht deshalb von der ABSICHT aus, nicht vom Ist.
//!
//! # Die drei Zustände, die er unterscheidet
//!
//! | Absicht | Variante in `LogKind` | im Lauf gesehen | Urteil            |
//! | ------- | --------------------- | --------------- | ----------------- |
//! | `true`  | ja                    | ja              | in Ordnung        |
//! | `true`  | ja                    | nein            | unerreicht (AC8)  |
//! | `true`  | **fehlt**             | —               | bekannte Lücke    |
//! | `false` | ja + geschrieben      | ja              | **rot**           |
//!
//! Die dritte Zeile ist `buffExpire`: Die Absicht sagt `sim: true`, aber der
//! Rust-Nummernraum kennt die Variante überhaupt nicht — `LogKind::BuffExpire`
//! gibt es nicht und gab es nie. Das ist keine fehlende Schreibstelle,
//! sondern eine fehlende Art, und es wird als Issue #682 geführt.

use std::collections::BTreeSet;

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::log::LogKind;
use rota_sim::{Fight, FightConfig};

const JSON: &str = include_str!("../../tests/parity/recordings/log-kinds.json");

/// Arten, deren Absicht `sim: true` lautet, für die es aber **keine
/// `LogKind`-Variante gibt** — mit dem Issue, unter dem sie geführt werden.
///
/// Sie sind vom Soll-Ist-Vergleich ausgenommen und werden trotzdem genannt.
/// Eine stille Ausnahme wäre die Bauart, die in #666 und #667 der Fehler war.
/// **Zurzeit leer — und das ist das Ergebnis, nicht der Anfangszustand.**
///
/// Beim Bau stand hier `buffExpire` mit dem Vermerk „der Simulator kennt die
/// Variante nicht, sie wurde nie angelegt". Issue #682 hat sie angelegt
/// (`LogKind::BuffExpire = 7`) samt Schreibstelle in `fight.rs`, und damit ist
/// die Lücke keine mehr. Der Eintrag wurde entfernt, nicht stehen gelassen:
/// Eine Ausnahme, deren Grund weggefallen ist, nimmt eine Art vom Vergleich,
/// die er prüfen kann.
const BEKANNTE_LUECKEN: &[(&str, &str)] = &[];

/// Arten, die keiner der Läufe unten auslöst — mit dem Grund (AC8).
///
/// **Nicht dasselbe wie „schreibt nicht".** Ein Lauf zeigt nur, was er
/// hervorruft; was hier steht, ist ungemessen, nicht widerlegt.
/// **Jeder Eintrag ist gemessen, keiner vermutet.** Ein erster Entwurf führte
/// hier auch `kill` mit der Begründung „die Läufe enden vor dem Boss-Tod" —
/// eine Nachmessung zeigte, dass der Standard-Lauf den Boss sehr wohl fällt.
/// Die Ausnahme hätte eine Art vom Vergleich genommen, die der Wächter prüfen
/// kann. Wer hier etwas einträgt, misst es vorher.
/// **Zurzeit leer — und jeder Eintrag, der hier stand, war falsch.**
///
/// Beim Bau standen drei Arten hier. Alle drei ließen sich bei genauerem
/// Hinsehen auslösen; die Begründungen waren plausibel und ungemessen:
///
/// ```text
///   kill               "die Läufe enden vor dem Boss-Tod"
///                      -> der Standard-Lauf fällt den Boss sehr wohl
///   convertedToShield  "verlangt Umwandlung und Treffer im selben Fenster"
///                      -> TEMPER stellt genau das her, an drei Bossen
///   playerDeath        "in keinem Lauf sinkt der Spieler auf null"
///                      -> boss-2-4 ohne Heilung, 300 s: hp 0, 5288 genommen
/// ```
///
/// **Eine Ausnahmeliste, die zu viel herausnimmt, ist dieselbe Bauart wie
/// eine Prüfliste, die einen Teil ihrer Gattung führt** — nur eine Ebene
/// höher und schwerer zu sehen, weil sie wie Sorgfalt aussieht. Wer hier
/// etwas einträgt, misst vorher, ob es sich nicht doch auslösen lässt.
const UNERREICHT: &[(&str, &str)] = &[];

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("log-kinds.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

/// Die Absicht je Art, wie sie aus `EVENT_KIND_INTENT` transportiert wurde.
fn absicht() -> Vec<(String, bool)> {
    data()["kinds"]
        .as_array()
        .expect("kinds fehlen")
        .iter()
        .filter_map(|eintrag| {
            let name = eintrag["name"].as_str()?;
            Some((
                name.to_owned(),
                eintrag["writtenBySim"]
                    .as_bool()
                    .unwrap_or_else(|| panic!("{name}: writtenBySim fehlt")),
            ))
        })
        .collect()
}

/// Gibt es für diesen Namen überhaupt eine `LogKind`-Variante?
///
/// **Strukturell, nicht über ein Textmuster.** `LogKind::ALL` entsteht im
/// Makro aus derselben Liste wie die Varianten selbst; wer eine Art einträgt,
/// bekommt sie hier ohne Zutun. Ein `grep` über `sim/src/` hätte dieselbe
/// Frage beantwortet und wäre bei jedem Zeilenumbruch falsch geworden —
/// genau daran ist die Vorarbeit zu diesem Issue zweimal gescheitert.
fn variante_fuer(name: &str) -> Option<LogKind> {
    LogKind::ALL.iter().copied().find(|k| k.name() == name)
}

/// Alle Arten, die irgendein Lauf dieser Datei geschrieben hat.
///
/// Jeder Lauf steht für eine Lage; die Zuordnung steht daneben, damit beim
/// Streichen auffällt, was dabei verlorengeht.
fn geschriebene_arten() -> BTreeSet<&'static str> {
    let mut gesehen = BTreeSet::new();
    let mut sammeln = |fight: &Fight| {
        for eintrag in fight.log().entries() {
            gesehen.insert(eintrag.kind.name());
        }
    };

    // Eine gute Rotation gegen den Standard-Boss.
    //   -> cast, damage, dotTick, spend, phase, playerDamage, bossTimeline
    let mut fight = Fight::new(FightConfig::default());
    fn rotation(f: &Fight) -> Option<usize> {
        let kindle = index_of("kindle").expect("kindle");
        let smoulder = index_of("smoulder").expect("smoulder");
        let pyre = index_of("pyre").expect("pyre");
        if f.dot_remaining_ms(smoulder) < 1500 && f.is_ready(smoulder) {
            Some(smoulder)
        } else if f.is_ready(pyre) && f.resource() >= 15 {
            Some(pyre)
        } else if f.is_ready(kindle) {
            Some(kindle)
        } else {
            None
        }
    }
    fight.run(rotation as fn(&Fight) -> Option<usize>);
    sammeln(&fight);

    // Ein Boss MIT Energieleiste — sonst bleibt `bossEnergyTriggered`
    // unerreicht. Genau daran scheiterte der Vergleich in #676.
    //   -> bossEnergyTriggered
    if let Some(boss) = boss_by_id("test-vessel") {
        let mut fight = Fight::new(FightConfig {
            boss,
            duration_ms: Some(60_000),
            ..FightConfig::default()
        });
        fight.run(rotation as fn(&Fight) -> Option<usize>);
        sammeln(&fight);
    }

    // Ein Boss mit Zeitleiste, die den Spieler trifft und unterbrochen werden
    // kann.  -> interruptLanded, shieldAbsorb, healed, controlSpent
    for boss_id in ["test-warden", "boss-1-2", "boss-2-4"] {
        let Some(boss) = boss_by_id(boss_id) else {
            continue;
        };
        let mut fight = Fight::new(FightConfig {
            boss,
            duration_ms: Some(60_000),
            ..FightConfig::default()
        });
        // **Die Reihenfolge trägt zwei Arten, die sonst ausfallen.**
        // Gemessen: Mit der bloßen Liste `quench > crust > salve > …` kamen
        // `healed` und `controlSpent` in KEINEM Lauf vor — SALVE heilt nur
        // einen verletzten Spieler, und Kontroll-Punkte werden nur
        // geschrieben, wenn sie auch AUSGEGEBEN werden. BIND sammelt sie,
        // ohne LATCH gibt sie niemand aus.
        fn mit_werkzeug(f: &Fight) -> Option<usize> {
            // Heilen, sobald wirklich Schaden da ist — sonst schreibt SALVE
            // nichts und belegt nur den GCD.
            if f.player_hp() < 900 {
                if let Some(i) = index_of("salve") {
                    if f.is_ready(i) {
                        return Some(i);
                    }
                }
            }
            // Kontroll-Punkte ausgeben, sobald sie reichen. LATCH ist der
            // billigste Verbraucher (cost 2).
            if f.control_points() >= 2 {
                if let Some(i) = index_of("latch") {
                    if f.is_ready(i) {
                        return Some(i);
                    }
                }
            }
            for name in ["quench", "crust", "bind", "smoulder", "kindle"] {
                if let Some(i) = index_of(name) {
                    if f.is_ready(i) {
                        return Some(i);
                    }
                }
            }
            None
        }
        fight.run(mit_werkzeug as fn(&Fight) -> Option<usize>);
        sammeln(&fight);
    }

    // **Ein Lauf OHNE CRUST** — sonst bleibt `healed` unerreicht.
    //
    // Gemessen: Mit Schutzschicht fängt CRUST den Boss-Schlag vollständig ab.
    // `damage_taken` stand danach auf **0**, obwohl `playerDamage` und
    // `shieldAbsorb` im Verlauf standen. Ein unverletzter Spieler kann nicht
    // geheilt werden — `LogKind::Healed` wird nur geschrieben, wenn
    // `heal.amount.min(fehlend) > 0` ist, und `fehlend` war null.
    //
    // Ohne CRUST kommt der Schlag durch, SALVE heilt wirklich, und der
    // Eintrag entsteht.  -> healed
    for boss_id in ["boss-2-4", "boss-1-2"] {
        let Some(boss) = boss_by_id(boss_id) else {
            continue;
        };
        let mut fight = Fight::new(FightConfig {
            boss,
            duration_ms: Some(60_000),
            ..FightConfig::default()
        });
        fn ohne_schild(f: &Fight) -> Option<usize> {
            if let Some(i) = index_of("salve") {
                if f.is_ready(i) && f.player_hp() < 4500 {
                    return Some(i);
                }
            }
            for name in ["smoulder", "kindle"] {
                if let Some(i) = index_of(name) {
                    if f.is_ready(i) {
                        return Some(i);
                    }
                }
            }
            None
        }
        fight.run(ohne_schild as fn(&Fight) -> Option<usize>);
        sammeln(&fight);
    }

    // **Ein Lauf, der eine Verstärkung wirft und ablaufen lässt** (#682).
    //
    // Keine der Rotationen oben wirft einen Verstärker, also lief auch keine
    // Verstärkung je aus — `buffExpire` blieb unerreicht, obwohl #682 die
    // Variante und die Schreibstelle angelegt hat. Ein fehlender Lauf ist
    // kein Grund für eine Ausnahme, sondern für einen Lauf.
    //                                                       -> buffExpire
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(60_000),
        ..FightConfig::default()
    });
    fn mit_verstaerker(f: &Fight) -> Option<usize> {
        for name in ["flashpoint", "temper", "distil", "slag", "kindle"] {
            if let Some(i) = index_of(name) {
                if f.is_ready(i) {
                    return Some(i);
                }
            }
        }
        None
    }
    fight.run(mit_verstaerker as fn(&Fight) -> Option<usize>);
    sammeln(&fight);

    // **TEMPER wandelt Schaden in Schutz** (25 % für 10 s). Ohne einen Lauf,
    // der ihn wirft, bleibt `convertedToShield` unerreicht — eine Zeit lang
    // stand die Art deshalb als Ausnahme hier, mit der plausiblen und
    // ungemessenen Begründung, sie verlange „Umwandlung und Treffer im selben
    // Fenster". TEMPER stellt genau das her, an jedem Boss mit Zeitleiste.
    //                                                 -> convertedToShield
    for boss_id in ["boss-2-4", "test-warden"] {
        let Some(boss) = boss_by_id(boss_id) else {
            continue;
        };
        let mut fight = Fight::new(FightConfig {
            boss,
            duration_ms: Some(60_000),
            ..FightConfig::default()
        });
        fn mit_temper(f: &Fight) -> Option<usize> {
            for name in ["temper", "kindle"] {
                if let Some(i) = index_of(name) {
                    if f.is_ready(i) {
                        return Some(i);
                    }
                }
            }
            None
        }
        fight.run(mit_temper as fn(&Fight) -> Option<usize>);
        sammeln(&fight);
    }

    // **Ein Lauf, der den Spieler wirklich sterben lässt.**
    //
    // `playerDeath` stand als unerreicht hier, weil die kürzeren Läufe den
    // Spieler durchbringen: mit CRUST fängt der Schild ab, mit SALVE heilt er
    // schneller als die Zeitleiste zuschlägt. Ohne beides und über fünf
    // Minuten fällt er — gemessen an boss-2-4: hp 0, 5288 genommen, Eintrag da.
    //
    // Der Lauf steht bewusst am Ende: Ein toter Spieler beendet den Kampf,
    // und was danach käme, gäbe es ohnehin nicht mehr.
    //                                                       -> playerDeath
    if let Some(boss) = boss_by_id("boss-2-4") {
        let mut fight = Fight::new(FightConfig {
            boss,
            duration_ms: Some(300_000),
            ..FightConfig::default()
        });
        fn nur_angriff(f: &Fight) -> Option<usize> {
            index_of("kindle").filter(|i| f.is_ready(*i))
        }
        fight.run(nur_angriff as fn(&Fight) -> Option<usize>);
        sammeln(&fight);
    }
    gesehen
}

#[test]
fn ac5_die_lage_misst_ueberhaupt_etwas() {
    // **Vor jedem Vergleich.** Wäre die Absicht leer oder schriebe kein Lauf
    // etwas, liefen alle Schleifen unten über eine leere Menge und wären grün.
    let alle = absicht();
    assert!(
        alle.len() >= 17,
        "die Absicht führt nur {} Arten — log-kinds.json ist unvollständig",
        alle.len()
    );
    let geschrieben = geschriebene_arten();
    assert!(
        !geschrieben.is_empty(),
        "kein einziger Protokoll-Eintrag in allen Läufen"
    );
    assert!(
        LogKind::ALL.len() >= 15,
        "LogKind::ALL führt nur {} Varianten",
        LogKind::ALL.len()
    );
}

#[test]
fn ac2_was_der_simulator_schreiben_soll_schreibt_er_auch() {
    let geschrieben = geschriebene_arten();
    let mut fehlend: Vec<String> = Vec::new();

    for (name, soll) in absicht() {
        if !soll {
            continue;
        }
        // Drei Gründe, hier nicht zu erscheinen — und sie sind verschieden:
        if BEKANNTE_LUECKEN.iter().any(|(n, _)| *n == name) {
            continue;
        }
        if UNERREICHT.iter().any(|(n, _)| *n == name) {
            continue;
        }
        if variante_fuer(&name).is_none() {
            fehlend.push(format!("{name} (keine LogKind-Variante)"));
            continue;
        }
        if !geschrieben.contains(name.as_str()) {
            fehlend.push(format!("{name} (Variante da, kein Lauf schreibt sie)"));
        }
    }

    assert!(
        fehlend.is_empty(),
        "laut Absicht schreibt der Simulator diese Arten — er tut es nicht: {}",
        fehlend.join(", ")
    );
}

#[test]
fn ac3_was_er_nicht_schreiben_soll_schreibt_er_auch_nicht() {
    // Die eindeutige Richtung: Hier kann kein Lauf zu wenig zeigen. Was
    // geschrieben wurde, wurde geschrieben.
    let geschrieben = geschriebene_arten();
    let unerwartet: Vec<String> = absicht()
        .into_iter()
        .filter(|(name, soll)| !soll && geschrieben.contains(name.as_str()))
        .map(|(name, _)| name)
        .collect();

    assert!(
        unerwartet.is_empty(),
        "diese Arten schreibt der Simulator, obwohl die Absicht \"nein\" sagt: {}",
        unerwartet.join(", ")
    );
}

#[test]
fn ac8_die_ausnahmen_sind_benannt_und_begruendet() {
    // Eine Ausnahme, die niemand sieht, ist eine Lücke. Diese Zeilen sind der
    // Preis dafür, dass die beiden Listen oben den Wächter beruhigen dürfen.
    let namen: Vec<String> = absicht().into_iter().map(|(n, _)| n).collect();

    for (name, grund) in BEKANNTE_LUECKEN {
        assert!(
            namen.iter().any(|n| n == name),
            "BEKANNTE_LUECKEN nennt \"{name}\" — die Art gibt es im Nummernraum nicht"
        );
        assert!(
            grund.contains('#'),
            "\"{name}\" ist als bekannte Lücke markiert, nennt aber kein Issue"
        );
        assert!(
            !grund.contains("#689"),
            "\"{name}\" verweist auf #689 — das ist der Wächter selbst, nicht der Weg hinaus"
        );
    }
    for (name, grund) in UNERREICHT {
        assert!(
            namen.iter().any(|n| n == name),
            "UNERREICHT nennt \"{name}\" — die Art gibt es nicht"
        );
        assert!(
            grund.len() > 10,
            "\"{name}\" steht ohne Grund in UNERREICHT"
        );
    }
}

#[test]
fn ac2a_kennt_der_simulator_jede_art_die_er_schreiben_soll() {
    // **Stufe 1: Datenstruktur gegen Datenstruktur.** `LogKind::ALL` entsteht
    // im Makro aus derselben Liste wie die Varianten; `writtenBySim` kommt aus
    // `EVENT_KIND_INTENT`. Kein Text, kein Lauf — und deshalb deckt dieser
    // Fall ALLE 16 Arten ab, nicht nur die, die ein Lauf hervorruft.
    let fehlend: Vec<String> = absicht()
        .into_iter()
        .filter(|(_, soll)| *soll)
        .map(|(name, _)| name)
        .filter(|name| variante_fuer(name).is_none())
        .filter(|name| !BEKANNTE_LUECKEN.iter().any(|(n, _)| *n == name.as_str()))
        .collect();

    assert!(
        fehlend.is_empty(),
        "der Simulator soll diese Arten schreiben, kennt sie aber nicht \
         einmal — es fehlt die LogKind-Variante: {}",
        fehlend.join(", ")
    );
}

#[test]
fn ac7_der_befund_als_eigenschaft_nicht_als_zahl() {
    // Der eigentliche Befund des Issues.
    //
    // **Die Zahl steht hier NICHT hart.** Ein erster Entwurf nagelte „15 von
    // 16" fest — und wäre in dem Moment rot geworden, in dem Issue #682 die
    // fehlende Variante anlegt. Ein Prüffall, der das Schließen einer Lücke
    // bestraft, erzieht dazu, ihn anzupassen statt ihn zu lesen.
    //
    // Geprüft wird stattdessen die EIGENSCHAFT: Jede Art mit `sim: true` ist
    // entweder als Variante vorhanden **oder** als bekannte Lücke geführt.
    // Ein Drittes gibt es nicht. Die Zahl steht in der Fehlermeldung und ist
    // damit ablesbar, ohne den Fall zu binden.
    let soll: Vec<String> = absicht()
        .into_iter()
        .filter(|(_, s)| *s)
        .map(|(n, _)| n)
        .collect();
    let mit_variante = soll.iter().filter(|n| variante_fuer(n).is_some()).count();

    // **Nicht die Summe, sondern die Eigenschaft je Art.** Eine Summe aus
    // „hat Variante" und „ist bekannte Lücke" zählt doppelt, sobald eine
    // Lücke geschlossen wird: Die Art bekommt ihre Variante, steht aber noch
    // in der Liste — und der Fall würde ausgerechnet dann rot, wenn jemand
    // das Richtige getan hat. Nachgemessen, indem `BuffExpire` versuchsweise
    // angelegt wurde: 16 + 1 = 17, Fall rot.
    let weder_noch: Vec<&String> = soll
        .iter()
        .filter(|n| variante_fuer(n).is_none())
        .filter(|n| !BEKANNTE_LUECKEN.iter().any(|(b, _)| *b == n.as_str()))
        .collect();

    assert_eq!(
        soll.len(),
        16,
        "die Zahl der Arten mit sim:true hat sich geändert — am 2026-08-09 waren es 16"
    );
    assert!(
        weder_noch.is_empty(),
        "diese Arten mit sim:true haben weder eine LogKind-Variante noch stehen \
         sie als bekannte Lücke: {:?}\n  (zur Einordnung: {mit_variante} von {} \
         haben eine Variante)",
        weder_noch,
        soll.len()
    );
}
