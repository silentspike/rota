//! Die Durchgangssteuerung des Balance-Läufers (Issue #915, AC3).
//!
//! `balance::run_gestaffelt` wählt zwischen genau zwei Wegen: **ein Kampf**,
//! wenn nirgends gewürfelt wird, sonst die **gestaffelte Stichprobe**.
//!
//! # Woher die Sollwerte kommen
//!
//! Nicht aus einer Bestandszahl. Jeder Fall leitet seine Erwartung aus dem
//! festen Läufer [`run`] ab, auf denselben Startwerten: Versuch `i` der
//! Stichprobe ist der Kampf mit `base_seed + i`, genau wie Durchgang `i` eines
//! festen Laufs. Eine falsche Siegbedingung oder ein anderer Kampfaufbau fiele
//! deshalb als Abweichung zwischen den beiden auf — und eine Balance-Änderung
//! an den Daten verschiebt beide gleich.
//!
//! # Welcher Fall welchen falschen Startwert fängt
//!
//! **Ab Startwert 0 sind `base_seed + i` und `i` dieselbe Zahl.** Ein Läufer,
//! der `base_seed` vergisst, rechnet dort genau die richtigen Kämpfe; die
//! Fälle ab 0 können ihn nicht fangen. Dafür stehen zwei Fälle mit einem
//! anderen Startwert da, ab 1000 und am Überlauf des `u32`. Ein um eins
//! verschobener Startwert fällt dagegen auch ab 0 auf, über den Schaden in
//! `mit_zufall_wird_gestaffelt_ueber_dieselben_kaempfe_wie_im_festen_lauf`.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{
    run, run_gestaffelt, siege, BalanceConfig, BalanceResult, Durchgangswahl, Staffelung,
    Staffelungsfehler, STARTWERTE,
};
use rota_sim::data::Boss;
use rota_sim::gear::WornValues;
use rota_sim::stichprobe::{gestaffelt, Befund};

fn gute_liste() -> Apl {
    parse(AplName("step1-good"), include_str!("../apl/step1-good.apl"))
        .expect("die eingebaute Liste ist gültig")
}

/// Ein Lauf gegen `boss_id` mit dieser PURITY, in einem Strang ab Startwert 0.
fn gegen(boss_id: &str, purity: u32, iterations: u32) -> BalanceConfig {
    let mut worn = WornValues::none();
    worn.purity = purity;
    BalanceConfig {
        iterations,
        base_seed: 0,
        threads: 1,
        boss: rota_sim::data::boss_by_id(boss_id).expect("den Boss gibt es"),
        worn,
        ..BalanceConfig::default()
    }
}

/// Eine Vorgabe, die bei halber Kritchance gegen `boss-1-1` schnell entscheidet.
const VORGABE: Staffelung = Staffelung {
    schwelle: 0.9,
    alpha: 0.05,
    budget: 2048,
};

/// Die Versuchszahl eines Befunds.
fn versuche(befund: Befund) -> u64 {
    match befund {
        Befund::Erfuellt { versuche }
        | Befund::Verfehlt { versuche }
        | Befund::Ungeklaert { versuche, .. } => versuche,
    }
}

/// Ob Durchgang `i` des festen Laufs gewonnen wurde — mit `balance::siege`,
/// also mit derselben Umkehrung bei einem Boss mit `runaway`.
fn gewonnen(fest: &BalanceResult, config: &BalanceConfig, i: u64) -> bool {
    let i = usize::try_from(i).expect("passt");
    siege(&fest.samples[i..=i], config.boss) == 1
}

/// Ob der Durchgang mit diesem **Startwert** im festen Lauf gewonnen wurde.
///
/// Gesucht wird nach dem Startwert, nicht nach dem Index wie in [`gewonnen`]:
/// `run` sortiert seine Proben nach Startwert, und am Überlauf des `u32` steht
/// der erste Versuch dann nicht mehr vorn.
fn gewonnen_bei(fest: &BalanceResult, boss: &Boss, seed: u32) -> bool {
    let probe = fest
        .samples
        .iter()
        .find(|probe| probe.seed == seed)
        .unwrap_or_else(|| panic!("der feste Lauf hat keinen Durchgang mit Startwert {seed}"));
    siege(std::slice::from_ref(probe), boss) == 1
}

/// Der Schaden aller Durchgänge eines festen Laufs zusammen.
fn schadensumme(fest: &BalanceResult) -> u64 {
    fest.samples
        .iter()
        .map(|probe| u64::from(probe.damage))
        .sum()
}

/// **Ohne Zufall genügt ein Kampf, und sein Ausgang ist exakt.**
///
/// „Ohne Zufall" heißt zweierlei: nie kritisch (PURITY 0) und **immer**
/// kritisch (PURITY 1000) — an beiden Enden zieht der Strom nicht. Drei Läufe,
/// damit beide Zweige des Befunds laufen. Welcher Lauf gewinnt, sagt ein
/// fester Lauf über genau den einen Kampf, den auch die Steuerung rechnet,
/// nicht eine Annahme; die Trägerzusicherung hält fest, dass wirklich einer
/// gewinnt und einer verliert.
///
/// > **Ein Durchgang, nicht zwanzig.** Eine frühere Fassung ließ den festen
/// > Lauf hier 20 Durchgänge rechnen und sicherte zu, dass alle gleich
/// > ausgehen. Die Zeile konnte nicht scheitern: Ohne Zufall kürzt `run`
/// > selbst auf einen Kampf und vervielfacht ihn (#908). Dass ohne Zufall ein
/// > Kampf die ganze Verteilung ist, prüft nicht dieser Fall, sondern die
/// > Nullwurf-Zusicherung in `run` und `run_gestaffelt` (Auflage 2) und
/// > `balance::tests::ac3_reduzierter_lauf_ist_bitgleich_mit_dem_vollen`.
///
/// > Die erste Fassung nahm an, die gute Rotation schlage `boss-1-1` auch ohne
/// > Krit. Sie tut es nicht — die Trägerzusicherung meldete zweimal
/// > `Verfehlt`. Gewonnen wird dort erst mit Krit.
#[test]
fn ohne_zufall_genuegt_ein_kampf_und_der_befund_ist_exakt() {
    let apl = gute_liste();
    let mut gesehen = Vec::new();
    for (boss_id, purity) in [("boss-1-1", 0), ("boss-1-1", 1000), ("boss-1-2", 0)] {
        // Ein Durchgang: Bei `iterations = 1` kürzt `run` nichts, es rechnet
        // den einen Kampf mit Startwert 0 selbst.
        let config = gegen(boss_id, purity, 1);
        let fest = run(&config, &apl);
        let gewonnen_fest = siege(&fest.samples, config.boss) == 1;

        let lauf = run_gestaffelt(&config, &apl, VORGABE).expect("gültige Vorgabe");
        let fall = format!("{boss_id} bei PURITY {purity}");
        assert_eq!(lauf.wahl, Durchgangswahl::EinDurchgang, "{fall}");
        assert_eq!(lauf.kaempfe, 1, "{fall}: genau ein Kampf");
        assert_eq!(lauf.wuerfe, 0, "{fall}: ohne Zufall wird nicht gezogen");
        let erwartet = if gewonnen_fest {
            Befund::Erfuellt { versuche: 1 }
        } else {
            Befund::Verfehlt { versuche: 1 }
        };
        assert_eq!(lauf.befund, erwartet, "{fall}");
        gesehen.push(lauf.befund);
        println!("{fall}: {:?}, {} Kampf", lauf.befund, lauf.kaempfe);
    }
    assert!(
        gesehen.contains(&Befund::Erfuellt { versuche: 1 })
            && gesehen.contains(&Befund::Verfehlt { versuche: 1 }),
        "Trägerzusicherung: ein gewonnener und ein verlorener Lauf, sonst lief \
         nur ein Zweig — gesehen: {gesehen:?}"
    );
}

/// **Mit Zufall wird gestaffelt — und es sind dieselben Kämpfe wie im festen
/// Lauf.**
///
/// Bei halber Kritchance verliert die gute Rotation gegen `boss-1-1` ab und
/// zu. Der Fall prüft dreierlei:
///
/// 1. Gewählt ist die Stichprobe, und es wurde wirklich gewürfelt.
/// 2. Siege **und Schaden** der Stichprobe sind die der ersten `kaempfe`
///    Durchgänge eines festen Laufs.
/// 3. Dieselbe Folge, über die Ausgänge des festen Laufs gerechnet, ergibt
///    denselben Befund — `gestaffelt` sieht also genau diese Kämpfe.
///
/// > **Der Schaden steht da, weil die Siege allein nicht trugen.** Gegenprobe
/// > mit einem um eins verschobenen Startwert (`base_seed + i + 1`): Bei 127
/// > von 128 Siegen blieben Siegzahl und Befund gleich, der Fall war grün. Der
/// > Schaden streut mit dem Krit und fängt die Verschiebung.
#[test]
fn mit_zufall_wird_gestaffelt_ueber_dieselben_kaempfe_wie_im_festen_lauf() {
    let apl = gute_liste();
    let budget = u32::try_from(VORGABE.budget).expect("passt");
    let config = gegen("boss-1-1", 500, budget);
    let lauf = run_gestaffelt(&config, &apl, VORGABE).expect("gültige Vorgabe");

    assert_eq!(lauf.wahl, Durchgangswahl::Gestaffelt);
    assert!(lauf.wuerfe > 0, "Trägerzusicherung: es wurde gewürfelt");
    assert_eq!(
        lauf.kaempfe,
        versuche(lauf.befund),
        "gezählte Kämpfe und Versuchszahl des Befunds"
    );

    let fest = run(&config, &apl);
    let siege_fest = (0..lauf.kaempfe)
        .filter(|&i| gewonnen(&fest, &config, i))
        .count() as u64;
    assert_eq!(
        lauf.siege, siege_fest,
        "die Stichprobe zählt andere Siege als der feste Lauf auf denselben \
         Startwerten"
    );
    let schaden_fest: u64 = fest.samples[..usize::try_from(lauf.kaempfe).expect("passt")]
        .iter()
        .map(|s| u64::from(s.damage))
        .sum();
    assert_eq!(
        lauf.schaden, schaden_fest,
        "die Stichprobe rechnet andere Kämpfe als der feste Lauf auf denselben \
         Startwerten"
    );
    assert!(
        lauf.siege < lauf.kaempfe,
        "Trägerzusicherung: mindestens ein Kampf ging verloren — sonst prüft \
         der Fall nur den fehlerfreien Weg ({} von {})",
        lauf.siege,
        lauf.kaempfe
    );

    let nachgerechnet = gestaffelt(VORGABE.schwelle, VORGABE.alpha, VORGABE.budget, |i| {
        gewonnen(&fest, &config, i)
    });
    assert_eq!(lauf.befund, nachgerechnet);
    println!(
        "boss-1-1, PURITY 500: {:?}, {} von {} gewonnen, {} Würfe",
        lauf.befund, lauf.siege, lauf.kaempfe, lauf.wuerfe
    );
}

/// Die Stichprobe ab `base_seed` gegen einen festen Lauf gleicher Länge ab
/// demselben Startwert; zurück kommt der feste Lauf.
///
/// Verglichen werden **Siegzahl, Schadensumme und Befund**, nicht
/// `fest.samples[..n]`: `run` sortiert seine Proben nach Startwert, und am
/// Überlauf ist der Index dann nicht der Versuch. Der Befund wird deshalb über
/// [`gewonnen_bei`] nachgerechnet, mit dem Startwert `base_seed + i`.
///
/// Die Trägerzusicherung hält fest, dass ab Startwert 0 andere Kämpfe
/// herauskämen. Sonst fiele ein vergessener `base_seed` auch hier nicht auf.
fn vergleiche_ab_startwert(base_seed: u32) -> BalanceResult {
    let apl = gute_liste();
    let config = BalanceConfig {
        base_seed,
        ..gegen("boss-1-1", 500, 1)
    };
    let lauf = run_gestaffelt(&config, &apl, VORGABE).expect("gültige Vorgabe");
    assert_eq!(
        lauf.wahl,
        Durchgangswahl::Gestaffelt,
        "Startwert {base_seed}"
    );
    assert_eq!(
        lauf.kaempfe,
        versuche(lauf.befund),
        "Startwert {base_seed}: gezählte Kämpfe und Versuchszahl des Befunds"
    );

    let laenge = u32::try_from(lauf.kaempfe).expect("passt");
    let fest = run(
        &BalanceConfig {
            iterations: laenge,
            ..config
        },
        &apl,
    );
    let ab_null = run(
        &BalanceConfig {
            iterations: laenge,
            base_seed: 0,
            ..config
        },
        &apl,
    );
    assert_ne!(
        schadensumme(&fest),
        schadensumme(&ab_null),
        "Trägerzusicherung: ab Startwert {base_seed} und ab 0 kommen dieselben \
         {laenge} Kämpfe heraus, dann fiele ein vergessener base_seed nicht auf"
    );

    assert_eq!(
        lauf.siege, fest.kills as u64,
        "Startwert {base_seed}: die Stichprobe zählt andere Siege als der feste \
         Lauf auf denselben Startwerten"
    );
    assert_eq!(
        lauf.schaden,
        schadensumme(&fest),
        "Startwert {base_seed}: die Stichprobe rechnet andere Kämpfe als der \
         feste Lauf auf denselben Startwerten"
    );
    let nachgerechnet = gestaffelt(VORGABE.schwelle, VORGABE.alpha, VORGABE.budget, |i| {
        let index = u32::try_from(i).expect("passt");
        gewonnen_bei(&fest, config.boss, base_seed.wrapping_add(index))
    });
    assert_eq!(lauf.befund, nachgerechnet, "Startwert {base_seed}");
    println!(
        "ab Startwert {base_seed}: {:?}, {} von {} gewonnen, Schaden {} \
         (ab 0 wären es {})",
        lauf.befund,
        lauf.siege,
        lauf.kaempfe,
        lauf.schaden,
        schadensumme(&ab_null)
    );
    fest
}

/// **Ab Startwert 1000 rechnet die Stichprobe die Kämpfe ab 1000.**
///
/// Ab 0 fiele ein vergessener `base_seed` nicht auf, siehe Dateikopf.
#[test]
fn ab_startwert_1000_rechnet_die_stichprobe_die_kaempfe_des_festen_laufs() {
    vergleiche_ab_startwert(1000);
}

/// **Am Überlauf des Startwerts dieselben Kämpfe wie im festen Lauf.**
///
/// Ab `u32::MAX - 50` laufen die Startwerte nach 51 Kämpfen über null weiter
/// (`wrapping_add`, in beiden Läufern). Die Trägerzusicherung hält fest, dass
/// der Lauf wirklich so weit reicht.
#[test]
fn am_ueberlauf_des_startwerts_rechnet_die_stichprobe_die_kaempfe_des_festen_laufs() {
    let base_seed = u32::MAX - 50;
    let fest = vergleiche_ab_startwert(base_seed);
    assert!(
        fest.samples.iter().any(|probe| probe.seed < base_seed),
        "Trägerzusicherung: der Lauf reicht über u32::MAX hinaus, sonst prüft \
         der Fall den Überlauf nicht ({} Kämpfe)",
        fest.samples.len()
    );
}

/// **Bei umgekehrtem Ziel zählt der Sieg wie im Zielvertrag.**
///
/// Gegen einen Boss mit `runaway` ist der leere Ring die Niederlage. Wer
/// `killed_at_ms` roh als Sieg zählte, käme hier auf das Gegenteil — die
/// Trägerzusicherung hält fest, dass der Ring in diesem Lauf wirklich fällt.
#[test]
fn bei_umgekehrtem_ziel_zaehlt_der_sieg_wie_im_zielvertrag() {
    let apl = gute_liste();
    let config = gegen("boss-2-3", 500, 64);
    assert!(
        config.boss.runaway.is_some(),
        "Kontrollzeile: boss-2-3 kehrt das Ziel um"
    );
    let lauf = run_gestaffelt(&config, &apl, VORGABE).expect("gültige Vorgabe");
    assert_eq!(lauf.wahl, Durchgangswahl::Gestaffelt);

    // Der feste Lauf rechnet genau so viele Durchgänge wie die Stichprobe —
    // zählte sie falsch und liefe länger, meldete sich das unten als
    // Abweichung und nicht als Indexfehler.
    let fest = run(
        &BalanceConfig {
            iterations: u32::try_from(lauf.kaempfe).expect("passt"),
            ..config
        },
        &apl,
    );
    let ring_leer = fest
        .samples
        .iter()
        .filter(|s| s.killed_at_ms.is_some())
        .count() as u64;
    let siege_fest = (0..lauf.kaempfe)
        .filter(|&i| gewonnen(&fest, &config, i))
        .count() as u64;
    assert!(
        ring_leer > 0,
        "Trägerzusicherung: der Ring fällt — sonst ist roh und umgekehrt \
         nicht zu unterscheiden"
    );
    assert_eq!(
        lauf.siege, siege_fest,
        "dieselbe Deutung wie balance::siege"
    );
    assert_ne!(
        lauf.siege, ring_leer,
        "gezählt wurde der leere Ring statt des Siegs"
    );
    println!(
        "boss-2-3: {:?}, Ring leer in {ring_leer} von {}, Siege {}",
        lauf.befund, lauf.kaempfe, lauf.siege
    );
}

/// **Am Budgetende kommt UNGEKLÄRT heraus, keine Zahl.**
///
/// Eine Schwelle nahe an der wahren Rate lässt sich mit 64 Kämpfen nicht
/// entscheiden; die Folge rechnet genau das Budget und meldet die Fehlerzahl,
/// die der feste Lauf auf denselben Startwerten auch sieht.
#[test]
fn am_budgetende_steht_ungeklaert_mit_der_fehlerzahl() {
    let apl = gute_liste();
    let vorgabe = Staffelung {
        schwelle: 0.99,
        alpha: 0.01,
        budget: 64,
    };
    let config = gegen("boss-1-1", 500, 64);
    let lauf = run_gestaffelt(&config, &apl, vorgabe).expect("gültige Vorgabe");
    let fest = run(&config, &apl);
    let fehler_fest = 64 - siege(&fest.samples, config.boss) as u64;
    assert_eq!(
        lauf.befund,
        Befund::Ungeklaert {
            versuche: 64,
            fehler: fehler_fest
        }
    );
    assert_eq!(lauf.kaempfe, 64, "genau das Budget, kein Kampf mehr");
    println!("Budget 64 bei Schwelle 0,99: {:?}", lauf.befund);
}

/// **Die Vorgabe wird geprüft, und zwar vollständig.**
#[test]
fn die_vorgabe_wird_vor_dem_lauf_geprueft() {
    let gut = Staffelung {
        schwelle: 0.9,
        alpha: 0.05,
        budget: 10,
    };
    assert_eq!(gut.pruefe(), Ok(()));
    assert_eq!(
        Staffelung {
            budget: STARTWERTE,
            ..gut
        }
        .pruefe(),
        Ok(()),
        "genau so viele Versuche, wie es Startwerte gibt, sind zulässig"
    );

    for schwelle in [0.0, 1.0, -0.5, 1.5, f64::NAN] {
        let fehler = Staffelung { schwelle, ..gut }
            .pruefe()
            .expect_err("unzulässig");
        assert!(
            matches!(fehler, Staffelungsfehler::Schwelle(_)),
            "Schwelle {schwelle}: {fehler:?}"
        );
    }
    for alpha in [0.0, 1.0, f64::NAN] {
        let fehler = Staffelung { alpha, ..gut }
            .pruefe()
            .expect_err("unzulässig");
        assert!(
            matches!(fehler, Staffelungsfehler::Alpha(_)),
            "alpha {alpha}: {fehler:?}"
        );
    }
    assert_eq!(
        Staffelung { budget: 0, ..gut }.pruefe(),
        Err(Staffelungsfehler::BudgetNull)
    );
    assert_eq!(
        Staffelung {
            budget: STARTWERTE + 1,
            ..gut
        }
        .pruefe(),
        Err(Staffelungsfehler::BudgetUeberStartwerten(STARTWERTE + 1))
    );

    // Und der Lauf selbst lehnt ab, statt zu rechnen.
    let apl = gute_liste();
    assert_eq!(
        run_gestaffelt(
            &gegen("boss-1-1", 500, 1),
            &apl,
            Staffelung { budget: 0, ..gut }
        ),
        Err(Staffelungsfehler::BudgetNull)
    );
}

/// **Gezählt, nicht behauptet: so viele Kämpfe, wie der Lauf meldet.**
///
/// Die Kampfzähler aus `diagnostics` sind strang-lokal; der gestaffelte Lauf
/// rechnet in einem Strang, also sieht der Zähler jeden seiner Kämpfe.
#[cfg(feature = "diagnostics")]
#[test]
fn die_kampfzahl_ist_gemessen() {
    use rota_sim::diagnostics;

    let apl = gute_liste();
    diagnostics::enable();

    diagnostics::reset();
    let gestaffelt_lauf =
        run_gestaffelt(&gegen("boss-1-1", 500, 1), &apl, VORGABE).expect("gültig");
    let kaempfe_gestaffelt = diagnostics::snapshot().fights();

    diagnostics::reset();
    let ein_lauf = run_gestaffelt(&gegen("boss-1-1", 0, 1), &apl, VORGABE).expect("gültig");
    let kaempfe_einer = diagnostics::snapshot().fights();

    diagnostics::reset();
    let abgelehnt = run_gestaffelt(
        &gegen("boss-1-1", 500, 1),
        &apl,
        Staffelung {
            alpha: 0.0,
            ..VORGABE
        },
    );
    let kaempfe_abgelehnt = diagnostics::snapshot().fights();

    diagnostics::disable();

    assert!(
        kaempfe_gestaffelt > 1,
        "Trägerzusicherung: der Zähler bewegt sich ({kaempfe_gestaffelt})"
    );
    assert_eq!(kaempfe_gestaffelt, gestaffelt_lauf.kaempfe);
    assert_eq!(kaempfe_einer, 1);
    assert_eq!(ein_lauf.kaempfe, 1);
    assert!(abgelehnt.is_err());
    assert_eq!(
        kaempfe_abgelehnt, 0,
        "eine unzulässige Vorgabe startet keinen Kampf"
    );
    println!("gemessen: {kaempfe_gestaffelt} Kämpfe gestaffelt, {kaempfe_einer} ohne Zufall");
}
