//! Wann die Liste nach einem Wurf ohne Wartezeit und während einer laufenden
//! entscheidet (Issue #1027).
//!
//! # Was hier verglichen wird
//!
//! `gcd-freier-wurf.json` stammt aus der Schleife von `runReference`
//! (`driveRotation` in `src/devtools/reference-run.ts`). Seit dem
//! Entscheidungsmodell, das der Orchestrator am 2026-10-28 an #1027 festgelegt
//! hat, entscheidet sie in jedem Takt und nach jedem Wurf beim selben
//! Zeitstempel erneut, bis die Liste nichts mehr wählt — auch während einer
//! laufenden Wartezeit, in der dann nur Zauber ohne Wartezeit in Frage kommen.
//! Das ist, was `Session.tick` einem Spieler erlaubt. Bis dahin entschied sie
//! je Takt einmal und nur bei bereiter Wartezeit, nach QUENCH also im nächsten
//! Takt (PR #1135).
//!
//! Die Aufzeichnung trägt dafür zweierlei, und hier wird beides geprüft:
//!
//! | Prüfung | Gegenstand | Frage |
//! | --- | --- | --- |
//! | Abspielen (`Fight::replay`) | die Drücke | rechnen beide Seiten dieselben Regeln? |
//! | Entscheiden (`Fight::run_apl`) | die Liste als Text | entscheidet der Simulator **wann** das Spiel entscheidet? |
//!
//! Das Abspielen allein genügte nicht: Es wirkt die Drücke zu den
//! aufgezeichneten Takten und kann deshalb gar nicht anders entscheiden. Bis
//! #1135 entschied der Simulator nach einem freien Wurf erst am nächsten
//! eingereihten Ereignis, an `first-crucible` mit QUENCH und Füller 1990 ms zu
//! spät (Füller bei 2000 statt 10), und jede Aufzeichnung blieb grün.
//!
//! # Der Leerlauf ist Teil des Vergleichs
//!
//! `idleMs` zählt die Schleife je Takt, in dem die Wartezeit bereit war und
//! nichts gewirkt wurde; der Simulator zählt dieselbe Zeit als Spanne bis zur
//! nächsten Entscheidung. Beide Seiten müssen gleich rechnen — sonst ist
//! `RunResult::idle_ms` eine Zahl, die nur so aussieht wie die des Spiels.

mod listen_aufzeichnung;

use serde_json::Value;

use listen_aufzeichnung::{druecke, laeufe, lauf};
use rota_sim::data::{index_of, SPELLS};
use rota_sim::time::GCD_MS;

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/gcd-freier-wurf.json");

/// Die Läufe, die hier je einen eigenen Prüffall haben. `jeder_lauf_hat_einen_prueffall`
/// hält die Liste gegen die Aufzeichnung — ein neuer Lauf ohne Fall fiele sonst
/// stumm aus dem Vergleich.
const LAEUFE: [&str; 8] = [
    "quench-fueller",
    "quench-nachlegen-strikt",
    "quench-auf-der-schwelle",
    "quench-ohne-fueller",
    "sublime-befreit",
    "quench-unter-dem-fueller",
    "quench-mitten-im-gcd",
    "warten-auf-die-uhr",
];

/// Die Läufe, in denen (a) nach einem Wurf ohne Wartezeit beim selben Zeitstempel
/// weiter gewirkt wird.
const SELBER_ZEITSTEMPEL: [&str; 6] = [
    "quench-fueller",
    "quench-nachlegen-strikt",
    "quench-auf-der-schwelle",
    "quench-ohne-fueller",
    "sublime-befreit",
    "quench-unter-dem-fueller",
];

/// Die Läufe, in denen (b) QUENCH während einer laufenden Wartezeit kommt.
const IM_LAUFENDEN_GCD: [&str; 2] = ["quench-unter-dem-fueller", "quench-mitten-im-gcd"];

fn data() -> Value {
    listen_aufzeichnung::data(JSON_TEXT)
}

/// Wie viele Zauber ohne Wartezeit gedrückt wurden, während die Wartezeit eines
/// früheren Wurfs noch lief — gerechnet aus den Drücken und den Zauberdaten.
///
/// Ein Wurf, den SUBLIME befreit, startet keine Wartezeit; diese Zählung kennt
/// das nicht und ist deshalb nur für Läufe ohne SUBLIME gemeint.
fn im_laufenden_gcd(reihe: &[(u32, usize)]) -> usize {
    let mut gcd_bis = 0;
    let mut zaehler = 0;
    for &(t, spell) in reihe {
        if SPELLS[spell].triggers_gcd {
            if t >= gcd_bis {
                gcd_bis = t + GCD_MS;
            }
        } else if t < gcd_bis {
            zaehler += 1;
        }
    }
    zaehler
}

/// **Die Zusicherung vor der Messung.** Ein Lauf, der die Lage nicht trägt,
/// belegt für diese Frage nichts.
///
/// Gezählt wird nicht der Zauber, sondern die Wirkung: (a) zwei Drücke beim
/// selben Zeitstempel, (b) ein Zauber ohne Wartezeit, während eine läuft. Die
/// Gegenprobe ohne freien Wurf muss bei beidem null haben, sonst prüfte diese
/// Zählung etwas anderes.
///
/// Bis zum Entscheidungsmodell vom 2026-10-28 stand hier „zwei Drücke einen
/// Takt auseinander" — das alte Modell, nach dem die Schleife je Takt einmal
/// entschied.
#[test]
fn jeder_lauf_traegt_die_lage_die_er_pruefen_soll() {
    let json = data();
    for name in LAEUFE {
        let reihe = druecke(&json, lauf(&json, name));
        let selber_zeitstempel = reihe
            .windows(2)
            .filter(|paar| paar[1].0 == paar[0].0)
            .count();
        let im_gcd = im_laufenden_gcd(&reihe);
        if name == "warten-auf-die-uhr" {
            assert_eq!(
                (selber_zeitstempel, im_gcd),
                (0, 0),
                "{name}: die Gegenprobe enthält einen freien Wurf"
            );
            continue;
        }
        if SELBER_ZEITSTEMPEL.contains(&name) {
            assert!(
                selber_zeitstempel > 0,
                "{name}: kein zweiter Druck beim selben Zeitstempel — der Lauf trägt (a) nicht"
            );
        }
        if IM_LAUFENDEN_GCD.contains(&name) {
            assert!(
                im_gcd > 0,
                "{name}: kein QUENCH während einer laufenden Wartezeit — der Lauf trägt (b) nicht"
            );
        }
        assert!(
            SELBER_ZEITSTEMPEL.contains(&name) || IM_LAUFENDEN_GCD.contains(&name),
            "{name}: der Lauf steht in keiner der beiden Lagen"
        );
    }
}

/// Die genaue Lage der 14 Gabelungen aus dem Bericht: QUENCH fällt auf ein
/// GCD-Ende, an dem die strikte Nachlegezeile genau auf ihrer Schwelle steht.
///
/// **Dort ist `< 1500` falsch, und beim selben Zeitstempel wählt die Liste den
/// Füller** — seit dem Entscheidungsmodell vom 2026-10-28. Bis dahin entschied
/// sie einen Takt später, sah `< 1500` wahr und legte bei 10510 nach.
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
        (geworfen, index_of("kindle").expect("kindle fehlt")),
        "das Spiel wirft nach QUENCH nicht beim selben Zeitstempel den Füller"
    );
    assert_eq!(
        reihe[stelle + 2],
        (geworfen + GCD_MS, smoulder),
        "das Spiel legt nicht am nächsten GCD-Ende nach"
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
fn quench_unter_dem_fueller_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("quench-unter-dem-fueller");
}

#[test]
fn quench_mitten_im_gcd_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("quench-mitten-im-gcd");
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
