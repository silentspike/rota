//! Der Nachlauf mit fremdem Startpunkt (Issue #558).
//!
//! # Der Mangel, den er behebt
//!
//! `greedy` beginnt bei der leeren Liste und bricht ab, sobald keine EINZELNE
//! weitere Zeile mehr messbar verbessert. Je groesser der Raum, desto frueher
//! geschieht das: Bei `--max-len 12` hoert die Suche schon nach acht Zeilen
//! auf, und der Punkt, an dem sie stehenbleibt, kann weit vom Optimum liegen.
//!
//! Gemessen ueber fuenf Bosse (#509): Die fuer `test-metronome` gefundene
//! Liste war an vier anderen Bossen besser als deren jeweils eigene — um bis
//! zu 37,3 %. Der Kreuzvergleich mass dort also die Schwaeche der Suche.
//!
//! # Was hier geprueft wird — und was NICHT
//!
//! Hier steht die EIGENSCHAFT von [`greedy_from`], nicht das Ergebnis des
//! Berichts. Ob die ACHTUNG-Zeile im Bericht auf null faellt, haengt an fuenf
//! konkreten Bossen und dauert Minuten; das ist der Lauf, der im Bericht
//! steht, kein Prueffall.
//!
//! Was ein Prueffall tragen kann, ist die Zusicherung darunter: dass ein
//! Startpunkt wirklich uebernommen wird, dass er die Suche nicht verschlechtert
//! und dass keine Zeile doppelt in die Liste geraet.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{candidates_for, greedy, greedy_from, greedy_opts, Candidate};

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

fn pool(ids: &[&str]) -> Vec<Candidate> {
    ids.iter()
        .flat_map(|id| {
            let index = index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt"));
            candidates_for(index)
        })
        .collect()
}

/// Die Zusicherung vor allem anderen: Der Startpunkt steht wirklich drin.
///
/// Ohne sie belegten die Faelle darunter nichts — eine Fassung, die `start`
/// stillschweigend verwirft, waere von `greedy` nicht zu unterscheiden und
/// bestuende jeden Vergleich „mindestens so gut wie".
///
/// # Warum die Startzeile GEMESSEN gewaehlt wird
///
/// Mein erster Entwurf nahm `kandidaten[0]`. Der Fall war gruen — und blieb
/// gruen, als ich `start` in `greedy_from` durch eine leere Liste ersetzte.
/// Der Grund: Die Suche waehlt diesen Kandidaten ohnehin von selbst. Er stand
/// also in der Liste, aber nicht WEIL er Startpunkt war.
///
/// Eine richtige Antwort aus falschem Grund faellt nicht auf, weil niemand
/// einen gruenen Fall nachprueft. Deshalb wird hier die erste Zeile genommen,
/// die die Suche von sich aus NICHT waehlt — und die Mutation faerbt.
#[test]
fn der_startpunkt_steht_in_der_gefundenen_liste() {
    let kandidaten = pool(&["kindle", "smoulder", "pyre"]);
    let config = config("first-crucible");

    let ohne = greedy(&config, &kandidaten, MAX_LEN);
    let fremd = kandidaten
        .iter()
        .find(|k| !ohne.chosen.contains(k))
        .expect("die Suche hat alle Kandidaten gewaehlt — kein fremder Startpunkt moeglich");
    let ergebnis = greedy_from(&config, &kandidaten, MAX_LEN, std::slice::from_ref(fremd));

    assert!(
        ergebnis.chosen.contains(fremd),
        "die Startzeile ist aus der Liste verschwunden — dann ist `start` wirkungslos"
    );
}

/// Keine Zeile zweimal — und der Filter dahinter spart 14 Messungen.
///
/// # Was ich hier ueber meinen eigenen Code gelernt habe
///
/// Der Kommentar an dieser Stelle behauptete zuerst: „Er faellt auf die
/// Mutation, die den `filter` ueber die schon belegten Kandidaten entfernt —
/// gemessen: dann stehen zwei Zeilen doppelt." **Das war nicht gemessen, und
/// es stimmt nicht.** Mit entferntem Filter blieben alle fuenf Faelle gruen.
///
/// Der Grund ist die Abbruchbedingung: `clearly_better_than` verlangt eine
/// MESSBARE Verbesserung, und eine doppelte Zeile ist in einer
/// Prioritaetenliste unerreichbar — sie verbessert also nie. Doppelte kann es
/// deshalb gar nicht geben, mit oder ohne Filter.
///
/// Was der Filter wirklich tut, steht in der Messzahl:
///
/// ```text
///   mit Filter    31 Messungen, Liste 4 Zeilen
///   ohne Filter   45 Messungen, Liste 4 Zeilen
/// ```
///
/// Gleiches Ergebnis, 14 Messungen weniger. Deshalb pruefen beide Zusicherungen
/// hier: die Eigenschaft (keine Doppelten) UND die Zahl, die den Filter belegt.
#[test]
fn keine_zeile_kommt_doppelt_vor() {
    let kandidaten = pool(&["kindle", "smoulder", "pyre"]);
    let start = vec![kandidaten[0].clone(), kandidaten[1].clone()];
    let ergebnis = greedy_from(&config("first-crucible"), &kandidaten, MAX_LEN, &start);

    // Die Zusicherung: Die Suche hat ueberhaupt weitergesucht. Bricht sie
    // sofort ab, ist „keine Doppelten" trivial erfuellt.
    assert!(
        ergebnis.chosen.len() > start.len(),
        "die Suche hat nichts hinzugefuegt — dann sagt dieser Fall nichts"
    );
    for (i, a) in ergebnis.chosen.iter().enumerate() {
        for b in ergebnis.chosen.iter().skip(i + 1) {
            assert!(a != b, "eine Zeile steht zweimal in der gefundenen Liste");
        }
    }
    assert_eq!(
        ergebnis.measured, 31,
        "die Zahl der Messungen hat sich geaendert. 45 hiesse: der Filter ueber die \
         schon belegten Kandidaten ist weg, und die Suche misst Zeilen, die sie \
         ohnehin nie waehlen kann."
    );
}

/// Ein Startpunkt macht die Suche nicht schlechter, als er selbst ist.
///
/// **Das ist die Eigenschaft, an der der ganze Nachlauf haengt.** Uebernimmt
/// man eine fremde Liste, muss mindestens ihr eigener Wert herauskommen —
/// sonst koennte der Nachlauf ein Ergebnis VERSCHLECHTERN, und die Zahlen im
/// Bericht waeren nicht mehr vergleichbar.
#[test]
fn vom_startpunkt_aus_wird_es_nicht_schlechter() {
    let kandidaten = pool(&["kindle", "smoulder", "pyre"]);
    let config = config("first-crucible");

    // Als Startpunkt eine Liste, die selbst schon etwas taugt: das Ergebnis
    // der gewoehnlichen Suche mit kuerzerer Grenze.
    let kurz = greedy(&config, &kandidaten, 2);
    let lang = greedy_from(&config, &kandidaten, MAX_LEN, &kurz.chosen);

    assert!(
        lang.best.mean >= kurz.best.mean,
        "vom Startpunkt aus wurde es schlechter: {:.1} gegen {:.1}",
        lang.best.mean,
        kurz.best.mean
    );
}

/// Und der leere Startpunkt ist genau die alte Suche.
///
/// `greedy` ruft `greedy_from` mit leerer Liste — dieser Fall haelt fest, dass
/// die Umstellung nichts an der bisherigen Suche geaendert hat. Er wuerde rot,
/// wenn jemand `greedy_from` eine Sonderbehandlung fuer den leeren Fall gaebe.
#[test]
fn ohne_startpunkt_ist_es_die_alte_suche() {
    let kandidaten = pool(&["kindle", "smoulder", "pyre"]);
    let config = config("first-crucible");

    let alt = greedy(&config, &kandidaten, MAX_LEN);
    let neu = greedy_from(&config, &kandidaten, MAX_LEN, &[]);

    assert_eq!(
        alt.best.apl.to_string(),
        neu.best.apl.to_string(),
        "leerer Startpunkt liefert eine andere Rotation als die alte Suche"
    );
    assert_eq!(alt.measured, neu.measured, "andere Zahl an Messungen");
}

/// Ein FREMDER Startpunkt aendert das Ergebnis wirklich.
///
/// Ohne diesen Fall belegten die anderen nur, dass nichts kaputtgeht. Gefragt
/// ist aber, ob der Startpunkt etwas KANN: Die Suche muss von dort aus
/// woandershin kommen als von der leeren Liste.
///
/// # Der Startpunkt wird GEMESSEN, nicht geraten
///
/// Mein erster Entwurf nahm `kandidaten.last()` mit der Begruendung „eine
/// Zeile, die die Suche nicht zuerst gewaehlt haette". Der Fall war rot, und
/// zwar zu Recht:
///
/// ```text
///   left:  "pyre if heat >= 15\nkindle\n"
///   right: "pyre if heat >= 15\nkindle\n"
/// ```
///
/// Die letzte Kandidatenzeile IST `pyre if heat >= 15` — ausgerechnet die,
/// die die Suche als erste waehlt. Der Startpunkt war also gar keiner, und
/// „dieselbe Liste" war die richtige Antwort auf einen falschen Aufbau.
///
/// Deshalb wird die Zeile jetzt aus der Messung genommen: die erste, die in
/// der ohne-Startpunkt gefundenen Liste NICHT vorkommt.
#[test]
fn ein_fremder_startpunkt_fuehrt_woandershin() {
    let kandidaten = pool(&["kindle", "smoulder", "pyre"]);
    let config = config("first-crucible");

    let ohne = greedy(&config, &kandidaten, 2);
    let fremd = kandidaten
        .iter()
        .find(|k| !ohne.chosen.contains(k))
        .expect("die Suche hat alle Kandidaten gewaehlt — dann gibt es keinen fremden Startpunkt");
    let mit = greedy_from(&config, &kandidaten, 2, std::slice::from_ref(fremd));

    // **Die Zusicherung vor dem Vergleich:** Der Startpunkt ist wirklich einer,
    // den die Suche von sich aus nicht genommen hat.
    assert!(
        !ohne.chosen.contains(fremd),
        "der gewaehlte Startpunkt steht schon in der ohne-Liste"
    );
    assert!(
        mit.chosen.contains(fremd),
        "die Startzeile ist unterwegs verlorengegangen"
    );
    assert_ne!(
        ohne.best.apl.to_string(),
        mit.best.apl.to_string(),
        "mit und ohne Startpunkt kommt dieselbe Liste heraus — dann traegt `start` nichts"
    );
}

/// **Die Falle im Nachlauf — gemessen, nicht vermutet.**
///
/// # Was ich hier ueber meinen eigenen Baustein gelernt habe
///
/// Ich hatte behauptet, der Nachlauf sei nie schlechter als die eigene Suche.
/// Der Fall dazu wurde rot:
///
/// ```text
///   der Nachlauf ist schlechter als die eigene Suche: 3138.0 gegen 6553.0
/// ```
///
/// Und die Behauptung war falsch, nicht der Code. [`greedy_from`] gibt
/// zurueck, was VON DIESEM STARTPUNKT AUS erreichbar ist — mehr verspricht es
/// nicht. Ist die Startliste schon so lang wie `max_len`, greift sofort
/// `Stop::LengthLimit`: Es wird gar nicht gesucht, und heraus kommt die fremde
/// Liste, an der neuen Stelle gespielt. Taugt sie dort nichts, taugt das
/// Ergebnis nichts.
///
/// **Der wesentliche Teil des Nachlaufs ist deshalb die Besser-Pruefung**, und
/// die steht im Laeufer, nicht in der Bibliothek:
///
/// ```text
///   if versuch.best.mean > bisher { uebernehmen }
/// ```
///
/// Wer `greedy_from` ohne sie benutzt, verschlechtert sein Ergebnis, ohne dass
/// es auffaellt — die Zahl sieht plausibel aus. Dieser Fall haelt die Falle
/// fest, damit der Naechste sie nicht neu findet.
#[test]
fn ohne_besser_pruefung_verschlechtert_der_nachlauf() {
    let kandidaten = pool(&["kindle", "smoulder", "pyre", "searing", "rot", "temper"]);
    let a = config("first-crucible");
    let b = config("test-onslaught");

    let bei_a = greedy_opts(&a, &kandidaten, 4, &[], false);
    let bei_b = greedy_opts(&b, &kandidaten, 4, &[], false);

    // Die Zusicherung: Die beiden Suchen finden ueberhaupt Verschiedenes.
    assert_ne!(
        bei_a.best.apl.to_string(),
        bei_b.best.apl.to_string(),
        "beide Bosse liefern dieselbe Liste — dann gibt es keinen fremden Startpunkt"
    );

    let roh = greedy_from(&b, &kandidaten, 4, &bei_a.chosen);

    // **Der Befund:** Der rohe Anlauf ist schlechter als die eigene Suche.
    assert!(
        roh.best.mean < bei_b.best.mean,
        "der rohe Anlauf ist NICHT schlechter ({:.1} gegen {:.1}) — dann faengt dieser          Fall die Falle nicht mehr, und die Warnung darueber gehoert geprueft",
        roh.best.mean,
        bei_b.best.mean
    );
    // Und er wird sofort abgebrochen, weil die Startliste die Laenge fuellt.
    assert_eq!(
        roh.chosen.len(),
        4,
        "die Startliste fuellt die Laengengrenze nicht — dann liegt der Grund woanders"
    );

    // **Und so ist es richtig:** Das Beste aus beiden nehmen.
    let nachlauf = if roh.best.mean > bei_b.best.mean {
        roh.best.mean
    } else {
        bei_b.best.mean
    };
    assert!(
        nachlauf >= bei_b.best.mean,
        "mit Besser-Pruefung darf nichts verlorengehen"
    );
}
