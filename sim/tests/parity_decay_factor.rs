//! Beide Seiten rechnen denselben Verwundbarkeits-Faktor (Issue #738).
//!
//! # Die Lücke, die diese Datei schliesst
//!
//! Die Faktoren-Spur `factorAt` uebertrug `buffs` takt-fuer-takt und
//! `vulnerability` gar nicht — zwei Faktoren derselben Familie, einer
//! verglichen, der andere nicht. Auf der Spiel-Seite gibt es
//! `combat.vulnerability` seit Langem und drei Pruefdateien lesen ihn; auf
//! dieser Seite gab es bis #738 kein Gegenstueck.
//!
//! # Warum eine EIGENE Aufzeichnung
//!
//! `buff-combat` traegt die Spur auch, faehrt aber keine Zersetzung: Ihr
//! `vulnerability` ist konstant 1,0. Sie ist damit die **Gegenprobe** — ein
//! Lauf ohne Zersetzung, der gruen bleibt — und kann genau deshalb den
//! Wirkungsnachweis nicht fuehren. Ein Wert, der nie von 1 abweicht, faerbt
//! bei keiner Mutation.

use std::fs;
use std::path::PathBuf;

use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

// `wurzel()` wird gebraucht, `programm()` und `rufe()` nicht — sie gehoeren
// zu den Suiten, die das gebaute Programm starten. Ohne das `allow` faellt
// `clippy -D warnings` ueber die ungenutzten Nachbarn, und die Warnung staende
// in einer fremden Datei.
#[allow(dead_code)]
mod gemeinsam;

/// Der Pfad einer Aufzeichnung, **zur Laufzeit** bestimmt.
///
/// # Warum nicht mehr `env!("CARGO_MANIFEST_DIR")`
///
/// Das wird zur **Bauzeit** eingesetzt. Dieses Projekt baut auf rustbuild und
/// führt auf dem Laptop aus; im Binary stand danach
/// `/tmp/builds/<id>/sim/../tests/parity/recordings/decay-factor.json` — ein
/// Pfad, den es hier nicht gibt.
///
/// **Er funktionierte trotzdem, und genau das war das Problem:** Unter
/// `/tmp/builds/<id>` liegt ein Symlink auf das Arbeitsverzeichnis, und die
/// `<id>` ist ein Hash über den Pfad — für dasselbe Verzeichnis also immer
/// dieselbe. Damit löst sich der eingebrannte Pfad hier auf, und der Test war
/// grün, ohne dass sein Code die Datei hätte finden können.
///
/// Gemessen am 2026-08-19, derselbe Stand, derselbe Befehl, der Symlink als
/// einziger Unterschied: ohne ihn Exit 101 und vier rote Fälle, mit ihm vier
/// grüne.
///
/// **Was daran ungemessen ist:** Wer den Symlink anlegt. Dass `cargo remote`
/// ihn bei jedem Bau erneuert, wäre die naheliegende Erklärung und ist
/// widerlegt — die Symlinks tragen das Datum des *ersten* Baus, nicht des
/// letzten (einer vom 18.08. 17:51 überlebte rund zwanzig Bauten am 19.08.).
/// Ob das Werkzeug ihn anlegt, wenn er fehlt, oder ob ihn jemand von Hand
/// gesetzt hat, ist offen.
///
/// Sicher ist nur die eine Richtung: Nach einem Neustart ist `/tmp` leer und
/// dieser Test rot. Und wer ihn dann untersuchen will, baut zuerst — und wenn
/// der Symlink dabei wiederkehrt, findet er ihn grün.
///
/// # Warum der Name als Literal stehen bleibt
///
/// `tests/parity/gegenstueck.test.ts` sucht `/<name>.json` **im Rust-Quelltext**
/// und meldet jede Aufzeichnung als ungedeckt, die es dort nicht findet. Ein
/// `format!` mit eingesetztem Namen wäre für ihn unsichtbar. Der Schrägstrich
/// vor dem Namen ist dabei das, worauf es ankommt — sein eigener Prüffall zeigt
/// es: `recordings/dot-lifetime.json` deckt `dot-lifetime`, `lifetime.json`
/// deckt `lifetime` nicht.
fn aufzeichnung(name_mit_pfad: &str) -> PathBuf {
    gemeinsam::wurzel().join(name_mit_pfad)
}

fn daten() -> serde_json::Value {
    lies(&aufzeichnung("tests/parity/recordings/decay-factor.json"))
}

/// Dieselbe Runde MIT einer Talent-Verteilung (#738).
fn daten_mit_talent() -> serde_json::Value {
    lies(&aufzeichnung("tests/parity/recordings/decay-talent.json"))
}

fn lies(pfad: &std::path::Path) -> serde_json::Value {
    // `display()` statt `{pfad}`: Ein `Path` hat kein `Display`, und die
    // Meldung soll den gesuchten Ort nennen — sie ist der einzige Hinweis,
    // wenn die Datei fehlt.
    let text = fs::read_to_string(pfad).unwrap_or_else(|_| panic!("{} fehlt", pfad.display()));
    serde_json::from_str(&text).unwrap_or_else(|_| panic!("{} ist kein JSON", pfad.display()))
}

/// Spielt die Aufzeichnung ab und liefert den Faktor am ENDE.
///
/// **Nur der Endwert, nicht je Takt** — und das ist eine Einschraenkung, keine
/// Wahl: `Fight::replay` fuehrt Pruefsummen je Takt, aber keine Faktoren-Spur.
/// Sie dort einzuhaengen waere der Abspieler-Umbau, den dieser Schritt
/// vermeiden soll. Der letzte Messpunkt der Aufzeichnung liegt bei Takt 599,
/// also am selben Ort.
fn faktor_am_ende(json: &serde_json::Value) -> f64 {
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks fehlen")).expect("zu viele");
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));

    let mut bindings: Vec<Binding> = Vec::new();
    let mut presses: Vec<Press> = Vec::new();
    for eintrag in json["presses"].as_array().expect("presses fehlen") {
        let id = eintrag["spellId"].as_str().expect("spellId fehlt");
        let index = SPELLS
            .iter()
            .position(|s| s.id == id)
            .unwrap_or_else(|| panic!("Zauber {id} gibt es nicht"));
        let slot = match bindings.iter().position(|b| b.spell == index) {
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
        name: String::from("decay-factor"),
        seed: 0,
        ticks,
        presses,
        bindings,
    };
    // **Die Talent-Promille kommen AUS der Aufzeichnung**, nicht aus dem Test.
    // Fehlt das Feld, laeuft der Kampf ohne Talente — so bleibt `decay-factor`
    // die Gegenprobe, ohne dass sie einen eigenen Zweig braucht.
    let permille = json["talentPermille"]["decayEffect"].as_u64().unwrap_or(0);
    let mut fight = Fight::new(FightConfig {
        boss,
        talent_decay_effect_permille: u32::try_from(permille).expect("zu gross"),
        ..FightConfig::default()
    });
    fight.replay(&recording, true);
    fight.vulnerability()
}

#[test]
fn die_aufzeichnung_traegt_ueberhaupt_zersetzung() {
    // **Die Zusicherung im Datensatz.** Ohne sie verglichen die Faelle unten
    // zweimal 1,0 gegeneinander und waeren gruen, ohne etwas zu belegen —
    // genau die Lage, in der `buff-combat` steckt.
    let json = daten();
    let spur: Vec<f64> = json["factorAt"]
        .as_array()
        .expect("factorAt fehlt")
        .iter()
        .map(|p| p["vulnerability"].as_f64().expect("vulnerability fehlt"))
        .collect();

    assert!(
        spur.iter().any(|&v| v > 1.0),
        "die Verwundbarkeit bleibt in der ganzen Spur bei 1,0 — dann faehrt \
         die Aufzeichnung ihre Zersetzung gar nicht mit: {spur:?}"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_faktor() {
    let json = daten();
    let spur = json["factorAt"].as_array().expect("factorAt fehlt");
    let letzter = spur.last().expect("factorAt ist leer")["vulnerability"]
        .as_f64()
        .expect("vulnerability fehlt");

    let gemessen = faktor_am_ende(&json);

    assert!(
        (gemessen - letzter).abs() < 1e-9,
        "am Ende rechnet der Simulator {gemessen}, das Spiel {letzter}"
    );
}

#[test]
fn die_talent_aufzeichnung_liegt_hoeher_als_die_ohne() {
    // **Die Zusicherung, dass das Talent ueberhaupt etwas bewegt.** Waeren
    // beide Spuren gleich, verglichen die Faelle darunter zwei identische
    // Laeufe, und eine Streich-Mutation am Talent-Faktor bliebe gruen — genau
    // die Lage, aus der dieses Issue entstanden ist.
    let ohne = letzter_faktor(&daten());
    let mit = letzter_faktor(&daten_mit_talent());

    assert!(
        mit > ohne,
        "die Aufzeichnung MIT Talent hat denselben Faktor wie die ohne \
         ({mit} gegen {ohne}) — dann traegt sie ihre Verteilung nicht"
    );
}

// **Der Befund aus Meldung 6189 war MEIN Aufbau, kein Paritaetsbruch.**
//
// Der Vergleich meldete einmal "Simulator 1.12, Spiel 1.16" — der Wert ohne
// Talent gegen den mit. Das sah nach einem Abspieler aus, der die Verteilung
// ignoriert, und es waere ein schwerer Fehler gewesen: Ignorierte `replay` die
// Talente, verglichen beide Seiten zwei verschiedene Kaempfe, und jede
// Aufzeichnung mit Talenten waere still wertlos.
//
// Gemessen an genau einer Stelle — dem Wert, kurz bevor `Fight` gebaut wird:
//
// ```text
// permille aus JSON:        400
// config vor Fight::new:    400
// nach replay:              vulnerability=1.16, 2 Stapel
// ```
//
// Der Abspieler rechnet ihn also. Die rote Messung stammte aus einem Lauf, in
// dem die Aufzeichnung das Feld `talentPermille` noch nicht trug — ich hatte
// es erst danach ergaenzt und den Vergleich nicht wiederholt.
//
// **Ein gemeldeter Befund, der sich in Luft aufloest, ist billiger als ein
// stiller Fehler** — aber diese Runde haette ich mir gespart, wenn ich nach
// der Aenderung am Datensatz neu gemessen haette statt das alte Ergebnis zu
// deuten.
#[test]
fn beide_seiten_rechnen_denselben_faktor_mit_talent() {
    let json = daten_mit_talent();
    assert!(
        json["talentPermille"]["decayEffect"].as_u64().unwrap_or(0) > 0,
        "die Aufzeichnung fuehrt keine Talent-Promille — dann laeuft dieser \
         Vergleich ohne Talente und sagt nichts ueber sie"
    );

    let gemessen = faktor_am_ende(&json);
    let erwartet = letzter_faktor(&json);

    assert!(
        (gemessen - erwartet).abs() < 1e-9,
        "mit Talent rechnet der Simulator {gemessen}, das Spiel {erwartet}"
    );
}

fn letzter_faktor(json: &serde_json::Value) -> f64 {
    json["factorAt"]
        .as_array()
        .expect("factorAt fehlt")
        .last()
        .expect("factorAt ist leer")["vulnerability"]
        .as_f64()
        .expect("vulnerability fehlt")
}
