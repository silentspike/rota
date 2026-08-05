//! Die Gegenprobe zur gierigen Suche (Issue #237).
//!
//! # Warum sie ein eigener Prüffall ist und keine Bemerkung im Bericht
//!
//! Sie belegt die Annahme, auf der das ganze Verfahren ruht: dass Greedy
//! findet, was erschöpfend auffindbar ist. Stünde sie nur als Absatz im
//! Bericht, wäre sie beim nächsten Umbau das Erste, was verschwindet — und die
//! Suche liefe weiter, ohne dass jemand merkt, dass ihre Begründung fehlt.
//!
//! # Warum n=5 nicht gereicht hätte
//!
//! Meine erste Fassung wollte bei fünf und sechs Kandidaten vergleichen. Der
//! Einwand des Orchestrators trifft:
//!
//! > Bei fünf Zaubern gibt es kaum eine Wahl, die gierig falsch treffen KANN.
//! > Findet Greedy dort das Optimum, ist das schwacher Beleg dafür, dass es bei
//! > fünfzehn nah dran liegt — **die Struktur, an der es scheitern würde, gibt
//! > es bei n=5 gar nicht.**
//!
//! Das ist derselbe Fehler, gegen den dieses Projekt den ganzen Tag arbeitet,
//! angewandt auf eine Gegenprobe: *Der Fall könnte den Unterschied nicht
//! zeigen.*
//!
//! # Die Messung ist bewusst rauschfrei
//!
//! `iterations = 1` mit festem Startwert. Der Vergleich soll die **Verfahren**
//! trennen, nicht die Streuung zweier Messungen: Mit vielen Durchgängen könnte
//! Greedy zufällig gewinnen oder verlieren, und die Gegenprobe maße dann das
//! Rauschen statt der Suche.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{candidates_for, exhaustive, greedy, Candidate};

/// Wie lang die Listen in der Gegenprobe höchstens werden.
///
/// **Das ist eine Grenze, keine Vollständigkeit** — und sie gehört genannt:
/// Erschöpfend geprüft werden alle Reihenfolgen aller Teilmengen **bis zu
/// dieser Länge**, nicht alle überhaupt. Bei sieben Kandidaten sind das
/// 7 + 42 + 210 + 840 = 1099 Listen statt 13.699.
const MAX_LEN: usize = 4;

fn config(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    BalanceConfig {
        iterations: 1,
        base_seed: 0,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

/// Eine Teilmenge aus benannten Zaubern, mit allen ihren Kandidatenzeilen.
fn pool(ids: &[&str]) -> Vec<Candidate> {
    ids.iter()
        .flat_map(|id| {
            let index = index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt"));
            candidates_for(index)
        })
        .collect()
}

#[test]
fn die_teilmengen_haben_ueberhaupt_struktur() {
    // Der Gegenfall zur Gegenprobe: Eine Auswahl aus lauter gleichartigen
    // Zaubern hat nichts, woran eine Reihenfolge etwas entscheiden könnte.
    // Dann fände Greedy trivial dasselbe wie die erschöpfende Suche — grün,
    // ohne etwas zu belegen.
    let gemischt = pool(&["kindle", "smoulder", "pyre"]);
    let mit_brand = gemischt
        .iter()
        .filter(|k| k.origin == rota_sim::rotation_search::Origin::DotRefresh)
        .count();
    assert!(
        mit_brand > 0,
        "keine einzige Nachlege-Zeile in der Teilmenge — dann entscheidet die \
         Reihenfolge nichts und die Gegenprobe belegt nichts"
    );
    assert!(
        gemischt.len() > 3,
        "{} Zeilen fuer drei Zauber — keine abgeleitete Bedingung dabei",
        gemischt.len()
    );
}

/// Die Teilmengen der Gegenprobe, mit ihrer gemessenen Kandidatenzahl.
///
/// **Die Zahl steht nicht im Namen der Funktion**, und das hat einen Grund:
/// Meine erste Fassung hiess `gierig_findet_bei_sieben_kandidaten_…` und mass
/// fuenf. Der Name kam aus der Auflage, nicht aus einer Messung — drei Zauber
/// ergeben eben nicht sieben Zeilen, sondern so viele, wie ihre Daten
/// hergeben. Ein Name, der eine Zahl behauptet, ist eine Aussage ohne Beleg.
const TEILMENGEN: &[(&str, &[&str])] = &[
    // Brand + Entlade-Schwelle + Fueller. Die kleinste Auswahl, an der eine
    // Reihenfolge ueberhaupt etwas entscheidet.
    ("Brand und Schwelle", &["kindle", "smoulder", "pyre"]),
    // Dazu die Werkzeug-Zauber. Sie machen keinen Schaden und muessen ihren
    // GCD ueber die Wirkung zurueckverdienen — die Struktur, an der eine
    // Reihenfolge am meisten entscheidet.
    (
        "mit Werkzeug-Zaubern",
        &["kindle", "smoulder", "pyre", "vapour", "reflux"],
    ),
    // Acht Kandidaten — die groesste Auswahl, die erschoepfend noch bezahlbar
    // ist. FESTER kommt dazu, weil er der Zauber ist, fuer den die
    // Werkzeug-Zauber arbeiten.
    (
        "acht Kandidaten",
        &["kindle", "smoulder", "pyre", "vapour", "reflux", "fester"],
    ),
];

#[test]
fn die_teilmengen_erreichen_die_geforderte_groesse() {
    // Die Auflage lautete n=7 und n=8 — nicht, weil sieben eine besondere Zahl
    // waere, sondern weil bei fuenf die Struktur fehlt, an der ein Verfahren
    // scheitern kann. Diese Pruefung misst, was die Namen sonst nur behaupten.
    let groessen: Vec<usize> = TEILMENGEN.iter().map(|(_, ids)| pool(ids).len()).collect();
    assert!(
        groessen.iter().any(|&n| n >= 8),
        "groesste Teilmenge hat {} Kandidaten, gefordert waren sieben — \
         die Gegenprobe misst dann eine Struktur, die es bei fuenfzehn gibt, \
         bei fuenf aber nicht",
        groessen.iter().max().copied().unwrap_or(0)
    );
}

#[test]
#[ignore = "misst tausende Kaempfe; laeuft auf Anforderung, nicht in jedem Lauf"]
fn einfuegen_findet_dasselbe_wie_die_erschoepfende_suche() {
    for (name, ids) in TEILMENGEN {
        let config = config("first-crucible");
        let teilmenge = pool(ids);

        let gierig = greedy(&config, &teilmenge, MAX_LEN);
        let voll = exhaustive(&config, &teilmenge, MAX_LEN);
        eprintln!(
            "{name}: {} Kandidaten | einfuegen {:.1} ({} Listen) | erschoepfend {:.1} ({} Listen)",
            teilmenge.len(),
            gierig.best.mean,
            gierig.measured,
            voll.best.mean,
            voll.measured
        );

        assert!(
            gierig.measured < voll.measured,
            "Gierig hat {} Listen gemessen, erschoepfend {} — wenn gierig nicht \
         weniger misst, ist es keine Abkuerzung",
            gierig.measured,
            voll.measured
        );

        // **Nicht auf Gleichheit der Listen prüfen.** Zwei verschiedene
        // Reihenfolgen können denselben Schaden ergeben; verlangt ist, dass Greedy
        // das Ergebnis erreicht, nicht dass es denselben Weg geht.
        assert!(
            gierig.best.mean >= voll.best.mean,
            "Gierig fand {:.1}, erschoepfend {:.1} — die Abweichung ist der Befund, \
         nicht der Fehlschlag: gierig verfehlt hier das Optimum um {:.1}",
            gierig.best.mean,
            voll.best.mean,
            voll.best.mean - gierig.best.mean
        );
    }
}

/// Welche Zauber in einem Lauf mit dieser Liste **gewirkt** wurden.
///
/// **Nicht, welche in der Liste stehen.** Ein Zauber, der ganz unten steht und
/// nie an die Reihe kommt, ist in der Liste vorhanden und im Kampf abwesend —
/// die Lehre aus Issue #236, hier auf ihren eigenen Prüffall angewandt.
fn gewirkte_zauber(boss_id: &str, apl: &rota_sim::Apl) -> Vec<usize> {
    let boss = boss_by_id(boss_id).expect("Boss unbekannt");
    let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
        seed: 0,
        boss,
        ..rota_sim::FightConfig::default()
    });
    let result = fight.run_apl(apl);
    (0..rota_sim::SPELL_COUNT)
        .filter(|&i| result.casts[i] > 0)
        .collect()
}

#[test]
#[ignore = "misst hunderte Kaempfe; laeuft auf Anforderung"]
fn ein_gewirkter_zauber_entfernt_veraendert_die_gefundene_rotation() {
    let config = config("first-crucible");
    let voller_pool = pool(TEILMENGEN[2].1);

    let gefunden = greedy(&config, &voller_pool, MAX_LEN);
    let gewirkt = gewirkte_zauber("first-crucible", &gefunden.best.apl);

    // Der Gegenfall, ohne den der Rest nichts prueft: Wenn kein Zauber
    // gewirkt wurde, ist unten jede Aussage leer.
    assert!(
        !gewirkt.is_empty(),
        "die gefundene Rotation wirkt keinen einzigen Zauber"
    );

    // **Einen entfernen, der tatsaechlich gewirkt wurde** — nicht irgendeinen.
    // Wer einen entfernt, der ohnehin nie drankam, bekommt selbstverstaendlich
    // dieselbe Liste zurueck, und der Prueffall meldete das als Fehlschlag,
    // obwohl die Suche korrekt ist.
    let raus = gewirkt[0];
    let ohne: Vec<Candidate> = voller_pool
        .iter()
        .filter(|k| k.spell != raus)
        .cloned()
        .collect();
    assert!(
        ohne.len() < voller_pool.len(),
        "der Pool ist gleich gross geblieben — nichts wurde entfernt"
    );

    let danach = greedy(&config, &ohne, MAX_LEN);
    let vorher_zauber: Vec<usize> = gefunden.chosen.iter().map(|k| k.spell).collect();
    let nachher_zauber: Vec<usize> = danach.chosen.iter().map(|k| k.spell).collect();

    eprintln!(
        "AC2: ohne {} | vorher {:?} ({:.1}) | nachher {:?} ({:.1})",
        rota_sim::SPELLS[raus].id,
        vorher_zauber,
        gefunden.best.mean,
        nachher_zauber,
        danach.best.mean
    );

    assert_ne!(
        vorher_zauber, nachher_zauber,
        "Dieselbe Rotation trotz entferntem Zauber — die Suche ist dann keine \
         Suche, sondern eine verdrahtete Liste"
    );
}
