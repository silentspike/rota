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

mod listen_aufzeichnung;

use serde_json::Value;

use listen_aufzeichnung::{druecke, laeufe, lauf};
use rota_sim::data::{index_of, SPELLS};
use rota_sim::TICK_MS;

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
    listen_aufzeichnung::data(JSON_TEXT)
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
    listen_aufzeichnung::das_abspielen_trifft_das_spiel(&data(), &LAEUFE);
}

fn entscheidet_wie_im_spiel(name: &str) {
    listen_aufzeichnung::entscheidet_wie_im_spiel(&data(), name);
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
    let in_der_datei = laeufe(&json);
    assert_eq!(
        in_der_datei, LAEUFE,
        "Läufe der Aufzeichnung und Prüffälle dieser Datei passen nicht zusammen"
    );
}
