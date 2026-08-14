//! Die Paar-Suche: Zeilen, die einzeln verlieren und zusammen tragen (#558).
//!
//! # Was hier geprueft wird
//!
//! Die EIGENSCHAFTEN des Verfahrens, nicht sein Ergebnis an echten Daten. Ob
//! ein Verstaerker sich lohnt, ist eine Balance-Frage und haengt an Zahlen,
//! die sich aendern duerfen; dass die Suche ein Paar aus den Daten bildet und
//! es beim Abschalten nicht mehr bildet, ist eine Zusicherung ueber den Code.
//!
//! # Warum das Verfahren ueberhaupt gebraucht wird
//!
//! Der Bericht der Rotations-Suche weist es selbst aus: 25 Kandidatenzeilen
//! stammen aus Verstaerkungs-Fenstern, und in KEINER gefundenen Rotation steht
//! eine davon. Der Grund ist die Bauart:
//!
//! ```text
//!   flashpoint                        allein: kostet einen GCD, bringt nichts
//!   kindle if buff.flashpoint.active  allein: der Buff ist nie da, also tot
//! ```
//!
//! Ein gieriges Verfahren bewertet jede Zeile einzeln, beide verlieren, also
//! nimmt es keine — und die zweite KANN nicht gewinnen, solange die erste
//! fehlt.

use rota_sim::apl::{Compare, Condition, Term, Value};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, index_of, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{candidates_for, greedy_opts, Candidate, Origin};

fn config() -> BalanceConfig {
    BalanceConfig {
        iterations: 1,
        base_seed: 0,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: boss_by_id("first-crucible").expect("first-crucible fehlt"),
        ..BalanceConfig::default()
    }
}

fn pool(ids: &[&str]) -> Vec<Candidate> {
    ids.iter()
        .flat_map(|id| {
            let index = index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt"));
            candidates_for(index)
        })
        .collect()
}

/// Die Zusicherung vor allem anderen: Es GIBT das Paar in den Kandidaten.
///
/// Ohne sie prueften die Faelle darunter ein Verfahren an einem Pool, in dem
/// gar nichts zu paaren ist — und „kein Unterschied" waere die richtige
/// Antwort auf eine Frage, die niemand gestellt hat.
#[test]
fn der_pool_enthaelt_ueberhaupt_ein_paar() {
    let kandidaten = pool(&["flashpoint", "kindle", "pyre"]);

    let verstaerker = kandidaten
        .iter()
        .filter(|k| SPELLS[k.spell].id == "flashpoint" && k.origin == Origin::Always)
        .count();
    let fenster = kandidaten
        .iter()
        .filter(|k| {
            k.origin == Origin::BuffWindow && k.rule().to_string().contains("buff.flashpoint.")
        })
        .count();

    assert!(
        verstaerker > 0,
        "die unbedingte FLASHPOINT-Zeile fehlt — ohne sie gibt es keinen Partner"
    );
    assert!(
        fenster > 0,
        "keine Zeile nennt buff.flashpoint — dann hat #553 die Kandidatenzeile verloren, \
         und die Paar-Suche haette nichts zu paaren"
    );
}

/// Der Schalter trennt die beiden Verfahren wirklich.
///
/// **Das ist die Zusicherung, an der die Gegenprobe des Orchestrators haengt**
/// (Nachricht 7215, Punkt 3): „Schalt die Paar-Logik ab und miss dieselbe
/// Stelle." Taete der Schalter nichts, waere jede Aussage ueber den Gewinn der
/// Paar-Suche unbelegt.
///
/// Gemessen wird an der Zahl der MESSUNGEN, nicht am Ergebnis: Ob ein Paar
/// gewinnt, haengt an Balance-Zahlen; dass es GEPRUEFT wird, ist Sache des
/// Codes.
#[test]
fn der_schalter_aendert_die_zahl_der_messungen() {
    let kandidaten = pool(&["flashpoint", "kindle", "pyre"]);
    let ohne = greedy_opts(&config(), &kandidaten, 6, &[], false);
    let mit = greedy_opts(&config(), &kandidaten, 6, &[], true);

    assert_eq!(
        ohne.paare_geprueft, 0,
        "mit abgeschalteter Paar-Logik wurden trotzdem Paare geprueft"
    );
    assert!(
        mit.paare_geprueft > 0,
        "mit eingeschalteter Paar-Logik wurde KEIN Paar geprueft — dann sagt ein \
         Vergleich der beiden Laeufe nichts ueber Paare aus"
    );
    assert!(
        mit.measured > ohne.measured,
        "die Paar-Suche hat nicht mehr gemessen als die Einzel-Suche: {} gegen {}",
        mit.measured,
        ohne.measured
    );
}

/// Die Paar-Suche macht das Ergebnis nie schlechter.
///
/// Sie kommt erst zum Zug, wenn kein einzelner Zug mehr traegt, und nimmt das
/// Paar nur bei messbarer Verbesserung. Waere das Ergebnis je schlechter, waere
/// eine der beiden Bedingungen falsch verdrahtet.
#[test]
fn die_paar_suche_verschlechtert_nichts() {
    let kandidaten = pool(&["flashpoint", "kindle", "pyre", "smoulder"]);
    let ohne = greedy_opts(&config(), &kandidaten, 6, &[], false);
    let mit = greedy_opts(&config(), &kandidaten, 6, &[], true);

    assert!(
        mit.best.mean >= ohne.best.mean,
        "mit Paar-Suche kam weniger heraus: {:.1} gegen {:.1}",
        mit.best.mean,
        ohne.best.mean
    );
}

/// Die Paarung kommt aus den DATEN — ein Verstaerker paart nur mit dem, was
/// seine Verstaerkung ueberhaupt nutzt.
///
/// FLASHPOINT verstaerkt den Ressourcen-Gewinn (`BuffTarget::Resource`). Seine
/// Fenster-Zeilen duerfen deshalb nur an Zaubern haengen, die Ressource geben.
/// **Das ist die Eigenschaft, die eine gepflegte Liste ersetzt:** Wer einen
/// Verstaerker ergaenzt, bekommt die Paare ohne Zutun — und wer einem Zauber
/// den Ressourcen-Gewinn nimmt, verliert sie ohne Zutun.
#[test]
fn ein_verstaerker_paart_nur_mit_passenden_zaubern() {
    let index = index_of("flashpoint").expect("flashpoint fehlt");
    let mut geprueft = 0;
    for (i, _) in SPELLS.iter().enumerate() {
        for k in candidates_for(i) {
            if k.origin != Origin::BuffWindow {
                continue;
            }
            if !k.rule().to_string().contains("buff.flashpoint.") {
                continue;
            }
            geprueft += 1;
            assert!(
                SPELLS[k.spell].resource_gain > 0,
                "{} traegt eine FLASHPOINT-Fensterzeile, gibt aber keine Ressource — \
                 dann kommt die Paarung nicht mehr aus den Daten",
                SPELLS[k.spell].id
            );
        }
    }
    assert!(
        geprueft > 0,
        "keine einzige FLASHPOINT-Fensterzeile gefunden — der Fall prueft nichts"
    );
    let _ = index;
}

/// Eine zusammengesetzte Bedingung wird NICHT gepaart.
///
/// Die Paarbildung liest den Verstaerker aus einer Bedingung der Form
/// `buff.X.active >= 1`. Bei allem anderen gibt sie auf, statt zu raten — eine
/// falsch gebildete Paarung waere schlechter als keine, weil sie Messungen
/// kostet und ein Ergebnis liefert, das niemand zuordnen kann.
#[test]
fn eine_zusammengesetzte_bedingung_wird_nicht_gepaart() {
    let kindle = index_of("kindle").expect("kindle fehlt");
    let flashpoint = index_of("flashpoint").expect("flashpoint fehlt");
    // Zwei Vergleiche in einer Gruppe — nicht die Form aus `candidates_for`.
    let verdreht = Candidate {
        spell: kindle,
        condition: Condition {
            groups: vec![vec![
                Term {
                    left: Value::BuffActive(flashpoint),
                    compare: Compare::GreaterEqual,
                    right: Value::Literal(1),
                },
                Term {
                    left: Value::Resource,
                    compare: Compare::Less,
                    right: Value::Literal(5),
                },
            ]],
        },
        origin: Origin::BuffWindow,
    };
    let mut kandidaten = pool(&["flashpoint", "kindle"]);
    kandidaten.retain(|k| k.origin != Origin::BuffWindow);
    let ohne_verdreht = greedy_opts(&config(), &kandidaten, 6, &[], true).paare_geprueft;
    kandidaten.push(verdreht);
    let mit_verdreht = greedy_opts(&config(), &kandidaten, 6, &[], true).paare_geprueft;

    assert_eq!(
        ohne_verdreht, mit_verdreht,
        "die zusammengesetzte Bedingung hat ein Paar erzeugt — dann raet die Paarbildung, \
         statt die Form zu pruefen"
    );
}
