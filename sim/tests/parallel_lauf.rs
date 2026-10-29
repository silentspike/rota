//! Was ein paralleler Lauf mit Helfern aus dem Vorrat zusichern muss (#912).
//!
//! Seit #912 leben die Helfer über die Läufe und der Aufrufer rechnet einen
//! Bereich selbst. Zwei Dinge, die vorher die frischen Stränge je Lauf von
//! selbst sicherten, stehen deshalb hier als eigene Fälle.

use rota_sim::apl::{parse, Apl, AplName};
use std::time::{Duration, Instant};

use rota_sim::balance::{run, run_vollstaendig, strom, BalanceConfig, BalanceResult};
use rota_sim::fingerabdruck::{abdruck, abweichungen};
use rota_sim::fortschritt::Zaehlwerk;
use rota_sim::gear::WornValues;

const GOOD: &str = include_str!("../apl/step1-good.apl");

fn gute_liste() -> Apl {
    parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
}

fn konfiguration(iterations: u32, threads: usize, base_seed: u32) -> BalanceConfig {
    // Mit Würfel, damit der Lauf über die Stränge verteilt wird (#908).
    let mut worn = WornValues::none();
    worn.purity = 500;
    BalanceConfig {
        iterations,
        threads,
        base_seed,
        boss: rota_sim::data::boss_by_id("boss-1-1").expect("den Boss gibt es"),
        worn,
        ..BalanceConfig::default()
    }
}

/// Ein Lauf mit eigenem Zählwerk: das Ergebnis, die Kämpfe je Platz.
///
/// Das Zählwerk zählt im heißen Pfad (#933), also das, was wirklich gerechnet
/// wurde — nicht das, was bestellt war. Platz 0 ist der Leitstrang, ab Platz 1
/// stehen die Teilnehmer des parallelen Laufs.
fn gezaehlt(config: &BalanceConfig, liste: &Apl) -> (BalanceResult, Vec<u64>) {
    let werk = Zaehlwerk::neu(config.threads);
    let anmeldung = werk.anmelden();
    let ergebnis = run_vollstaendig(config, liste);
    drop(anmeldung);
    (ergebnis, werk.momentaufnahme().kaempfe_je_strang)
}

/// **Bitgleich über jede Strangzahl — mit Würfel, über das ganze Ergebnis**
/// (#919 AC6).
///
/// Die Strangvergleiche in `tests/balance.rs` und in `rota-sim verify`
/// fuhren bis #891 N6 ohne Würfel; seit #908 schrumpft ein solcher Lauf auf
/// einen Kampf und berührt den parallelen Pfad gar nicht (`verify` meldete
/// „Kaempfe gerechnet: 3"). Hier würfelt jeder Kampf, und 997 Durchgänge
/// teilen sich auf keine Strangzahl glatt auf.
///
/// **Verglichen wird der Fingerabdruck über jedes Feld**, nicht eine Auswahl.
/// Bis zum 29.10. standen hier Mittelwerte, min/max, Siege und Wirkungen. Ohne
/// die Sortierung nach Startwert in `balance::run` blieb der Fall grün (Job
/// d27fee0e, nachgefahren in Job dfe079cf), obwohl der Kommentar dort sie den
/// entscheidenden Schritt nennt: Keines dieser Felder wich ab. Rot wird er
/// seitdem an den Streuungen und an der Probenfolge — welche Felder es waren,
/// steht im Kopf von `fingerabdruck.rs`.
#[test]
fn jede_strangzahl_rechnet_bitgleich_wie_einer_auch_mit_wuerfel() {
    let liste = gute_liste();
    let (einer, plaetze) = gezaehlt(&konfiguration(997, 1, 5), &liste);
    // Positive Kontrollen: Der Lauf hat gewürfelt — ohne Würfel streut er
    // nicht —, und er hat jeden Durchgang selbst gerechnet statt einen zu
    // vervielfachen (#908). Sonst verglichen die Läufe unten nichts, was eine
    // Reihenfolge haben kann.
    assert!(
        einer.damage.stddev > 0.0,
        "der Lauf streut nicht — es wurde nicht gewuerfelt"
    );
    assert_eq!(
        plaetze.iter().sum::<u64>(),
        997,
        "ein Strang rechnete nicht jeden Durchgang: {plaetze:?}"
    );
    let bezug = abdruck(&einer);
    for threads in [2usize, 3, 4, 8, 16] {
        let (viele, plaetze) = gezaehlt(&konfiguration(997, threads, 5), &liste);
        assert_eq!(
            plaetze.iter().sum::<u64>(),
            997,
            "{threads} Straenge rechneten nicht jeden Durchgang: {plaetze:?}"
        );
        // Und der Lauf war wirklich parallel: Mindestens zwei Teilnehmer
        // haben gerechnet. Rechnete einer allein, käme die Folge in seiner
        // Reihenfolge an — und die ist ohnehin die nach Startwert.
        let beteiligt = plaetze.iter().filter(|k| **k > 0).count();
        assert!(
            beteiligt >= 2,
            "{threads} Straenge, aber nur {beteiligt} rechnete: {plaetze:?}"
        );
        assert_eq!(
            abdruck(&viele),
            bezug,
            "{threads} Straenge rechnen anders als einer, abweichend: {:?}",
            abweichungen(&viele, &einer)
        );
    }
}

/// **Acht Aufrufer zugleich, jeder bekommt sein eigenes Ergebnis.** Die
/// Helfer sind geteilt; landete ein Bereich in der Sammlung eines fremden
/// Laufs, wiche dessen Ergebnis ab.
#[test]
fn gleichzeitige_laeufe_rechnen_jeder_sein_eigenes_ergebnis() {
    let liste = gute_liste();
    let konfigs: Vec<BalanceConfig> = (0..8u32)
        .map(|i| konfiguration(400, 4, 100 + i * 1000))
        .collect();
    let einzeln: Vec<_> = konfigs.iter().map(|c| run(c, &liste)).collect();
    // Positive Kontrolle: Verschiedene Startwerte geben verschiedene
    // Ergebnisse — sonst bewiese der Vergleich unten nichts.
    assert_ne!(
        einzeln[0].damage.mean.to_bits(),
        einzeln[1].damage.mean.to_bits(),
        "zwei Startwerte, dasselbe Ergebnis"
    );
    let gleichzeitig: Vec<Vec<_>> = std::thread::scope(|s| {
        let griffe: Vec<_> = konfigs
            .iter()
            .map(|c| {
                let liste = &liste;
                s.spawn(move || (0..3).map(|_| run(c, liste)).collect::<Vec<_>>())
            })
            .collect();
        griffe
            .into_iter()
            .map(|g| g.join().expect("ein Aufrufer ist abgestuerzt"))
            .collect()
    });
    for (nummer, (e, laeufe)) in einzeln.iter().zip(&gleichzeitig).enumerate() {
        for r in laeufe {
            assert_eq!(
                (
                    r.damage.mean.to_bits(),
                    r.damage_taken.mean.to_bits(),
                    r.damage.min.to_bits(),
                    r.damage.max.to_bits(),
                    &r.casts,
                ),
                (
                    e.damage.mean.to_bits(),
                    e.damage_taken.mean.to_bits(),
                    e.damage.min.to_bits(),
                    e.damage.max.to_bits(),
                    &e.casts,
                ),
                "Aufrufer {nummer}: gleichzeitig ein anderes Ergebnis als allein"
            );
        }
    }
}

/// **Der Trace-Hash bleibt einsträngig.** Er hängt an der Reihenfolge der
/// Zustände und wird über Stränge nicht vereinigt; vor #912 rechnete der
/// Aufrufer im parallelen Lauf nichts, und kein Zustand ging ein. Rechnet er
/// jetzt Bereich 0 mit, darf sein Trace davon nichts sehen.
#[cfg(feature = "diagnostics")]
#[test]
fn der_mitrechnende_aufrufer_laesst_den_trace_einstraengig() {
    use rota_sim::diagnostics;
    let liste = gute_liste();

    // Positive Kontrolle: Einsträngig nimmt der Trace Zustände auf.
    diagnostics::reset();
    diagnostics::enable_trace();
    let _ = run(&konfiguration(4, 1, 7), &liste);
    let allein = diagnostics::snapshot().trace_states();
    assert!(allein > 0, "der Trace nimmt einstraengig nichts auf");

    diagnostics::reset();
    let _ = run(&konfiguration(40, 4, 7), &liste);
    let parallel = diagnostics::snapshot().trace_states();
    assert!(
        diagnostics::is_trace_enabled(),
        "der parallele Lauf hat den Trace-Schalter nicht zurueckgestellt"
    );
    diagnostics::disable_trace();
    diagnostics::reset();
    assert_eq!(
        parallel, 0,
        "der parallele Lauf trug Zustaende in den Trace des Aufrufers"
    );
}

/// **Ein Helfer trägt keine Zählung in den nächsten Lauf.** Zweimal derselbe
/// Lauf mit eingeschalteten Kampfzählern ergibt zweimal dieselbe Zahl —
/// zählte ein Helfer weiter, stünde beim zweiten Lauf mehr.
#[cfg(feature = "diagnostics")]
#[test]
fn ein_helfer_traegt_keine_zaehlung_in_den_naechsten_lauf() {
    use rota_sim::diagnostics;
    let liste = gute_liste();
    let mut gezaehlt = Vec::new();
    for _ in 0..2 {
        diagnostics::reset();
        diagnostics::enable();
        let _ = run(&konfiguration(40, 4, 7), &liste);
        gezaehlt.push(diagnostics::snapshot().fights());
        diagnostics::disable();
        diagnostics::reset();
    }
    assert_eq!(gezaehlt, [40, 40], "Kaempfe je Lauf: {gezaehlt:?}");
}

/// **Ein durchgehender Lauf zählt bis zum Fensterende und hält dann an**
/// (#912, Weg b). Kalibriert wird so, wie die echte Arbeit läuft: lange,
/// ohne Sperre je Paket. Gemessen am 28.10. auf node3: Unter 8 Fremdsträngen
/// kostete die Sperre je 20k-Paket 10–13 ms, und kein Paket-Verfahren kam
/// unter 10 % an die Arbeit am Stück heran.
#[test]
fn ein_strom_zaehlt_bis_zum_fensterende_und_haelt_dann_an() {
    let liste = gute_liste();
    let config = konfiguration(20_000, 4, 5);
    let m = strom(&config, &liste, Duration::from_millis(200));
    assert!(
        m.fenster >= Duration::from_millis(200) && m.fenster < Duration::from_millis(300),
        "Fenster {:?} statt rund 200 ms",
        m.fenster
    );
    assert!(m.kaempfe >= 100, "nur {} Kaempfe in 200 ms", m.kaempfe);
    assert!(
        m.nachlauf <= 4,
        "nach dem Fensterende liefen noch {} Kaempfe — mehr als einer je Helfer",
        m.nachlauf
    );

    // Positive Kontrolle gegen dieselbe Arbeit am Stück: Die Rate des Stroms
    // liegt in derselben Größenordnung. Eng ist das nicht — eng prüft AC1 auf
    // node3; hier geht es darum, dass der Strom überhaupt Kämpfe rechnet und
    // nicht etwas anderes zählt.
    let t = Instant::now();
    let _ = run_vollstaendig(&konfiguration(4000, 4, 5), &liste);
    let stueck = 4000.0 / t.elapsed().as_secs_f64();
    #[allow(clippy::cast_precision_loss)]
    let rate = m.kaempfe as f64 / m.fenster.as_secs_f64();
    assert!(
        rate > stueck / 3.0 && rate < stueck * 3.0,
        "Strom {rate:.0}/s gegen am Stueck {stueck:.0}/s"
    );
}
