//! Wer bestimmt, wie lange eine Schutzschicht hält — und was ein Zauber
//! anrichtet, der gar nichts hinzufügt.
//!
//! # Der Befund, den diese Datei festhält
//!
//! `shield` und `shield_stacks` sind **gemeinsame** Werte für alle Schilde, und
//! dazu gehört eine gemeinsame Ablaufzeit. Wird ein Schild-Zauber gewirkt,
//! setzt er diese Zeit auf `jetzt + seine eigene Dauer` — **auch dann, wenn er
//! am Stapellimit abprallt und die Schicht überhaupt nicht erhöht.**
//!
//! Damit entscheidet ein Zauber, der nichts beiträgt, über die Haltbarkeit des
//! Ganzen. In beide Richtungen:
//!
//! | Reihenfolge | zweiter Zauber | Wirkung auf die Schicht |
//! | --- | --- | --- |
//! | `plate`, `crucible-skin` | hält 6000 ms | **verkürzt** — die Schicht ist früh weg |
//! | `crucible-skin`, `plate` | hält 15000 ms | **verlängert** — sie steht lange |
//!
//! Für einen Spieler heißt das: `crucible-skin` nach `plate` zu wirken ist
//! **schlechter, als es zu lassen**. Es fügt nichts hinzu, kostet eine globale
//! Wartezeit und nimmt Schutz weg.
//!
//! # Was diese Datei NICHT tut
//!
//! Sie entscheidet nicht, ob das richtig ist. Es kann gewollt sein („die
//! jüngste Schicht bestimmt die Haltbarkeit") — nur steht das nirgends. **Die
//! Design-Frage liegt beim Auftraggeber**; bis sie beantwortet ist, hält diese
//! Datei fest, was der Simulator tut, damit es niemand ein drittes Mal
//! herausfinden muss.
//!
//! Ändert jemand die Ablauflogik, wird sie rot. Das ist beabsichtigt: Dann ist
//! die Entscheidung gefallen und gehört hier hinein.
//!
//! # Warum keine festen Zahlen im Kriterium
//!
//! Die Zusicherungen vergleichen **zwei Läufe miteinander**, statt gemessene
//! Beträge festzuschreiben. Beträge und Dauern sind Balance-Sache und dürfen
//! sich ändern; dass die Reihenfolge über die Haltbarkeit entscheidet, ist die
//! Eigenschaft.

use rota_sim::apl::{parse, AplName};
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::{Fight, FightConfig};

/// Wann gemessen wird.
///
/// Muss **zwischen** den beiden Ablaufzeiten liegen, sonst zeigen beide
/// Anordnungen dasselbe: früher steht in beiden noch etwas, später in beiden
/// nichts mehr. `crucible-skin` hält 6000 ms, `plate` 15000 — der Zauber fällt
/// um 1500 ms, also liegt der Unterschied zwischen rund 7500 und 16500.
const MESSPUNKT_MS: u32 = 12_000;

/// Wann geprüft wird, dass der zweite Zauber **abprallt**.
///
/// Das ist ein anderer Zeitpunkt als [`MESSPUNKT_MS`], und der Unterschied ist
/// keine Feinheit: Das Abprallen geschieht beim **Aufbau**, kurz nach dem
/// Kampfbeginn. Bei 12000 ms ist die Schicht in einer der beiden Anordnungen
/// bereits abgelaufen — dort stünden dann 0 Stapel, und eine Zusicherung auf
/// „genau ein Stapel" bräche, obwohl alles in Ordnung ist.
///
/// **Genau das ist beim ersten Lauf passiert.** Der Fall war gegen den falschen
/// Zeitpunkt geschrieben und hat es selbst gemeldet.
const AUFBAU_MESSPUNKT_MS: u32 = 3_000;

/// Ein Kampf ohne jeden Schlag: `test-resistant` trägt eine leere Zeitleiste.
/// Gemessen wird damit die **Haltbarkeit**, nicht der Verbrauch.
fn fahre(anordnung: &[&str]) -> (u32, u32, Vec<u32>) {
    fahre_bis(anordnung, MESSPUNKT_MS)
}

/// Wie [`fahre`], aber mit frei gewähltem Zeitpunkt.
fn fahre_bis(anordnung: &[&str], bis_ms: u32) -> (u32, u32, Vec<u32>) {
    let boss = boss_by_id("test-resistant").expect("test-resistant fehlt im Datensatz");
    let text = anordnung
        .iter()
        .map(|z| format!("{z}\n"))
        .collect::<String>();
    let apl = parse(AplName("ablaufzeit"), &text).expect("die Liste ist gueltig");

    let mut fight = Fight::new(FightConfig {
        boss,
        duration_ms: Some(bis_ms),
        ..FightConfig::default()
    });
    let ergebnis = fight.run_apl(&apl);

    let wuerfe = anordnung
        .iter()
        .map(|z| {
            let i = SPELLS
                .iter()
                .position(|s| s.id == *z)
                .unwrap_or_else(|| panic!("Zauber {z} gibt es nicht"));
            ergebnis.casts[i]
        })
        .collect();
    (fight.shield(), fight.shield_stacks(), wuerfe)
}

/// **Träger-Zusicherung, und sie steht bewusst vorn.**
///
/// Der ganze Befund beruht darauf, dass der zweite Zauber **gewirkt wird und
/// trotzdem nichts beiträgt**. Würde er gar nicht gewirkt — weil die
/// Prioritätsliste ihn nie erreicht, weil eine Abklingzeit läuft —, wäre jede
/// folgende Zahl richtig gemessen und würde etwas anderes belegen.
#[test]
fn der_zweite_zauber_wird_gewirkt_und_traegt_trotzdem_nichts_bei() {
    for anordnung in [
        ["plate", "crucible-skin"].as_slice(),
        ["crucible-skin", "plate"].as_slice(),
    ] {
        let (_, stapel, wuerfe) = fahre_bis(anordnung, AUFBAU_MESSPUNKT_MS);
        assert!(
            wuerfe.iter().all(|&n| n >= 1),
            "in {anordnung:?} wurde bis {AUFBAU_MESSPUNKT_MS} ms nicht jeder \
             Zauber gewirkt: {wuerfe:?}. Dann prallt der zweite nicht ab, \
             sondern kam nie zum Zug."
        );
        assert_eq!(
            stapel, 1,
            "in {anordnung:?} stehen bei {AUFBAU_MESSPUNKT_MS} ms {stapel} \
             Stapel statt einem. MEHR heißt: der zweite Zauber hat etwas \
             beigetragen, dann misst der Fall nicht mehr das Abprallen. \
             WENIGER heißt: die Schicht ist schon weg, dann ist der Zeitpunkt \
             falsch gewählt und alles Weitere misst ins Leere."
        );
    }
}

/// Die Reihenfolge zweier abprallender Schilde entscheidet über die Haltbarkeit.
///
/// # Warum der Vergleich und nicht zwei feste Zahlen
///
/// Beide Läufe wirken **dieselben zwei Zauber**, gleich oft, und enden bei
/// demselben Stapelstand. Sie unterscheiden sich in nichts als der Reihenfolge.
/// Ein Unterschied im Ergebnis kann deshalb nur daher kommen — das ist ein
/// Kontrollversuch mit einer einzigen Variablen.
#[test]
fn die_reihenfolge_entscheidet_ueber_die_haltbarkeit() {
    let (kurz, _, _) = fahre(&["plate", "crucible-skin"]);
    let (lang, _, _) = fahre(&["crucible-skin", "plate"]);

    assert!(
        lang > kurz,
        "beide Anordnungen ergeben zum Messpunkt {MESSPUNKT_MS} ms dieselbe \
         Schicht (kurz {kurz}, lang {lang}). Dann bestimmt der zuletzt \
         gewirkte Zauber die Ablaufzeit NICHT mehr — die Mechanik hat sich \
         geändert, und die Design-Frage aus dem Dateikopf ist damit \
         beantwortet worden, ohne dass es jemand hier eingetragen hat."
    );

    println!(
        "REIHENFOLGE ENTSCHEIDET: plate->crucible-skin {kurz}, \
         crucible-skin->plate {lang} (gemessen bei {MESSPUNKT_MS} ms)"
    );
}

/// Der teure Fall: Der zusätzliche Zauber ist **schlechter als nichts**.
///
/// `plate` allein gegen `plate` gefolgt von `crucible-skin`. Der zweite Zauber
/// erhöht die Schicht nicht (siehe Träger-Zusicherung) — er kann also nur
/// schaden oder nichts tun. Gemessen schadet er.
///
/// Diese Zusicherung ist die eigentliche Aussage der Datei: Ein Spieler, der
/// einen bereitstehenden Schutzzauber wirkt, verliert dadurch Schutz.
#[test]
fn ein_zusaetzlicher_schild_kann_schlechter_sein_als_keiner() {
    let (allein, _, _) = fahre(&["plate"]);
    let (mit_zweitem, _, _) = fahre(&["plate", "crucible-skin"]);

    assert!(
        mit_zweitem < allein,
        "plate allein ergibt {allein}, plate gefolgt von crucible-skin {mit_zweitem}. \
         Ist der zweite Wert nicht mehr kleiner, kostet der zusätzliche Zauber \
         keinen Schutz mehr — dann ist der Befund aus dem Dateikopf behoben und \
         dieser Fall gehört gelöscht statt angepasst."
    );

    println!("ZUSAETZLICHER SCHILD KOSTET: allein {allein}, mit zweitem {mit_zweitem}");
}
