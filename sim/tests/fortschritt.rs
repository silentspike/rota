//! Fortschritt, zwei Durchsatzzahlen und eine ehrliche Restzeit (#933, E15.2).
//!
//! Je Abnahmekriterium mindestens ein Fall, der beim Namen rot wird, wenn die
//! zugesicherte Eigenschaft fehlt. Die Fälle zur Zählung fahren echte
//! Balance-Läufe; die zur Restzeit und zur Anzeige bauen ihre Momentaufnahmen
//! selbst, damit Zeit und Zahlen festliegen und nicht an der Maschine hängen.

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{
    auswertungsweg, run, run_gestaffelt, run_vollstaendig, BalanceConfig, Staffelung,
};
use rota_sim::fortschritt::{
    ablauf, format_ganzzahl, restzeit, restzeit_aus, strang, utc_iso8601, Anzeigestrang,
    Arbeitsmenge, Ausgabe, Bestes, Ereignis, Fenster, Kampfzahl, Kostenbild, Laufbeschreibung,
    Laufstand, Momentaufnahme, Rechner, Restanteil, Restkosten, Restzeit, Strangzaehler, Stufe,
    Weg, Zaehlwerk, ANZEIGETAKT, CACHE_ZEILE, FENSTER, WEGE_OHNE_LAUF,
};
use rota_sim::gear::WornValues;
use rota_sim::kalibrierung::{Abbruchgrund, Arbeitsart, RateModel};
use rota_sim::rotation_search::{all_candidates, greedy, restmessungen_hoechstens};

const GOOD: &str = include_str!("../apl/step1-good.apl");

fn gute_liste() -> Apl {
    parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
}

fn konfiguration(iterations: u32, threads: usize, purity: u32) -> BalanceConfig {
    let mut worn = WornValues::none();
    worn.purity = purity;
    BalanceConfig {
        iterations,
        threads,
        boss: rota_sim::data::boss_by_id("boss-1-1").expect("den Boss gibt es"),
        worn,
        ..BalanceConfig::default()
    }
}

fn rate(je_strang: f64, streuung: f64) -> RateModel {
    RateModel {
        rate_per_s: je_strang,
        spread: streuung,
        samples: 30,
        duration_ms: 30_000,
        reason: Abbruchgrund::Stabil,
        measured_at: 0,
        psi_cpu: None,
        takt_drift: None,
    }
}

fn rechner(id: &str, straenge: usize, je_strang: f64, streuung: f64) -> Rechner {
    Rechner {
        id: id.to_owned(),
        straenge,
        raten: vec![(Arbeitsart::ScalarFullFight, rate(je_strang, streuung))],
        trend: 0.0,
    }
}

fn kaempfe_stufe(aufgaben: u64, wiederholungen: u64) -> Stufe {
    Stufe {
        art: Arbeitsart::ScalarFullFight,
        aufgaben,
        einheiten_je_aufgabe: 1.0,
        wiederholungen,
    }
}

/// Eine Probe nach `sekunden`, mit gleichmäßig wachsenden Zählern.
fn probe(sekunden: f64, kaempfe: u64, kandidaten: [u64; 3], stand: Laufstand) -> Momentaufnahme {
    Momentaufnahme {
        elapsed_ns: (sekunden * 1e9).round() as u64,
        kaempfe,
        durchgaenge: kaempfe,
        kandidaten,
        kaempfe_je_strang: vec![kaempfe],
        stand,
    }
}

fn nah(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

// ---------------------------------------------------------------- AC9

/// **AC9: Die Zähler je Strang liegen auf getrennten Cache-Zeilen.**
///
/// Bewusst ein Laufzeit-Fall und keine Übersetzungsbedingung: Die Gegenprobe
/// (`align(128)` durch `align(8)` ersetzen) muss hier beim Namen rot werden,
/// nicht als Baufehler ohne Prüffall.
#[test]
fn ac9_strangzaehler_liegen_auf_getrennten_cache_zeilen() {
    assert!(
        std::mem::align_of::<Strangzaehler>() >= CACHE_ZEILE,
        "Ausrichtung {} statt mindestens {CACHE_ZEILE}",
        std::mem::align_of::<Strangzaehler>()
    );
    let werk = Zaehlwerk::neu(8);
    assert_eq!(werk.plaetze(), 9, "Leitstrang und acht Arbeitsstränge");
    for a in 0..werk.plaetze() {
        assert_eq!(
            werk.adresse(a) % CACHE_ZEILE,
            0,
            "Platz {a} beginnt nicht auf einer {CACHE_ZEILE}-Byte-Grenze"
        );
        for b in a + 1..werk.plaetze() {
            let abstand = werk.adresse(b).abs_diff(werk.adresse(a));
            assert!(
                abstand >= CACHE_ZEILE,
                "Platz {a} und {b} liegen nur {abstand} Bytes auseinander"
            );
        }
    }
}

// ---------------------------------------------------------------- Zählung

#[test]
fn ein_reduzierter_lauf_zaehlt_einen_kampf_und_zwanzig_durchgaenge() {
    let config = konfiguration(20, 1, 0);
    // Trägerzusicherung: Ohne Reduktion prüfte der Fall die Stichprobe.
    assert_eq!(auswertungsweg(&config), Weg::EinKampf);
    let werk = Zaehlwerk::neu(1);
    let _anmeldung = werk.anmelden();
    let _ = run(&config, &gute_liste());
    let m = werk.momentaufnahme();
    assert_eq!(m.kaempfe, 1, "gerechnet wird EIN Kampf, nicht zwanzig");
    assert_eq!(m.durchgaenge, 20, "geliefert werden zwanzig Durchgänge");
    assert_eq!(
        m.kandidaten,
        [1, 0, 0],
        "eine Auswertung auf dem Weg 'ein Kampf'"
    );
}

#[test]
fn eine_stichprobe_zaehlt_jeden_kampf_im_strang_der_ihn_rechnet() {
    let config = konfiguration(20, 4, 500);
    assert_eq!(auswertungsweg(&config), Weg::Stichprobe);
    let werk = Zaehlwerk::neu(4);
    let _anmeldung = werk.anmelden();
    let _ = run(&config, &gute_liste());
    let m = werk.momentaufnahme();
    assert_eq!(m.kaempfe, 20);
    assert_eq!(m.kandidaten, [0, 1, 0]);
    assert_eq!(
        m.kaempfe_je_strang,
        vec![0, 5, 5, 5, 5],
        "der Leitstrang rechnet nicht, jeder Arbeitsstrang fünf"
    );
}

#[test]
fn der_vollstaendige_lauf_rechnet_jeden_durchgang() {
    // Der Weg, den `rota-sim bench` braucht: Ohne Würfel greift sonst die
    // Reduktion, und die Durchsatzmessung misst einen Kampf.
    let config = konfiguration(20, 1, 0);
    let werk = Zaehlwerk::neu(1);
    let _anmeldung = werk.anmelden();
    let voll = run_vollstaendig(&config, &gute_liste());
    let m = werk.momentaufnahme();
    assert_eq!(m.kaempfe, 20, "jeder Durchgang ein gerechneter Kampf");
    assert_eq!(m.kandidaten, [0, 1, 0]);
    // Und dasselbe Ergebnis wie der reduzierte Lauf — Bitgleichheit.
    let reduziert = run(&config, &gute_liste());
    assert_eq!(voll.samples, reduziert.samples);
    assert_eq!(voll.damage.mean.to_bits(), reduziert.damage.mean.to_bits());
}

#[test]
fn der_gestaffelte_lauf_zaehlt_seine_kaempfe() {
    let config = konfiguration(1, 1, 500);
    let werk = Zaehlwerk::neu(1);
    let _anmeldung = werk.anmelden();
    let lauf = run_gestaffelt(
        &config,
        &gute_liste(),
        Staffelung {
            schwelle: 0.9,
            alpha: 0.05,
            budget: 512,
        },
    )
    .expect("die Vorgabe ist gültig");
    let m = werk.momentaufnahme();
    assert!(lauf.kaempfe > 1, "Trägerzusicherung: es wurde gestaffelt");
    assert_eq!(m.kaempfe, lauf.kaempfe);
    assert_eq!(m.durchgaenge, lauf.kaempfe);
    assert_eq!(m.kandidaten, [0, 0, 1]);
}

#[test]
fn ohne_anmeldung_zaehlt_nichts_und_das_ergebnis_bleibt_bitgleich() {
    let config = konfiguration(20, 2, 500);
    assert!(
        strang().is_none(),
        "ein frischer Prüfstrang ist nicht angemeldet"
    );
    let ohne = run(&config, &gute_liste());
    let werk = Zaehlwerk::neu(2);
    let mit = {
        let _anmeldung = werk.anmelden();
        assert!(strang().is_some());
        run(&config, &gute_liste())
    };
    assert!(strang().is_none(), "die Anmeldung endet mit ihrem Wert");
    assert_eq!(ohne.samples, mit.samples);
    assert_eq!(ohne.damage.mean.to_bits(), mit.damage.mean.to_bits());
    assert_eq!(ohne.casts, mit.casts);
    assert_eq!(werk.momentaufnahme().kaempfe, 20);
}

#[test]
fn die_gierige_suche_meldet_runden_bestes_und_eine_haltende_obergrenze() {
    let werk = Zaehlwerk::neu(1);
    let suchend = Arc::clone(&werk);
    let suche = std::thread::spawn(move || {
        let _anmeldung = suchend.anmelden();
        greedy(&konfiguration(4, 1, 0), &all_candidates(), 3)
    });
    let mut gesehen: Vec<(Option<u64>, Arbeitsmenge)> = Vec::new();
    while !suche.is_finished() {
        let m = werk.momentaufnahme();
        gesehen.push((m.stand.runde, m.stand.menge));
        std::thread::sleep(Duration::from_micros(500));
    }
    let ergebnis = suche.join().expect("die Suche lief durch");
    let ende = werk.momentaufnahme();
    let gesamt = ende.kandidaten_gesamt();
    assert_eq!(
        gesamt,
        ergebnis.measured as u64 + 2,
        "gezählt wird jede Messung, auch Start- und Fundliste"
    );
    assert!(ende.stand.bestes.is_some(), "kein bestes Ergebnis gemeldet");
    let mut runden: Vec<u64> = gesehen.iter().filter_map(|(r, _)| *r).collect();
    runden.dedup();
    assert!(
        runden.len() >= 2,
        "Zusicherung: nur {} Runde(n) beobachtet — die Prüfung sähe dann fast nichts",
        runden.len()
    );
    let mut geprueft = 0;
    for (_, menge) in &gesehen {
        if let Arbeitsmenge::Hoechstens {
            mindestens,
            hoechstens,
        } = *menge
        {
            assert!(
                mindestens <= gesamt && gesamt <= hoechstens,
                "die Schranken {mindestens}..={hoechstens} schließen die {gesamt} Messungen nicht ein"
            );
            geprueft += 1;
        }
    }
    assert!(geprueft > 0, "keine einzige Schranke beobachtet");
}

#[test]
fn die_obergrenze_der_suche_rechnet_runde_fuer_runde() {
    // Eine Runde, drei offene Zeilen, leere Liste: drei Einzelzüge.
    assert_eq!(restmessungen_hoechstens(0, 3, 1, 0, 0), 3);
    // Zwei Runden: 3·1 + (2)·min(0+3,1)+1 = 3 + 2·2.
    assert_eq!(restmessungen_hoechstens(0, 3, 2, 0, 0), 3 + 2 * 2);
    // Längengrenze schon erreicht: nichts mehr.
    assert_eq!(restmessungen_hoechstens(4, 3, 4, 5, 5), 0);
    // Paare zählen nur, solange zwei Zeilen passen: Runde eins mit einer
    // Stelle (2 + 7), Runde zwei mit einer offenen Zeile an zwei Stellen.
    assert_eq!(restmessungen_hoechstens(0, 2, 1, 7, 0), 2);
    let zweite_runde = 2;
    assert_eq!(
        restmessungen_hoechstens(0, 2, 2, 7, 0),
        2 + 7 + zweite_runde
    );
}

// ---------------------------------------------------------------- AC4

fn gleichmaessig(fenster: &mut Fenster, bis_s: f64, ab_s: f64, rate: f64, mut kaempfe: f64) -> f64 {
    let mut t = ab_s;
    while t <= bis_s + 1e-9 {
        fenster.aufnehmen(probe(
            t,
            kaempfe.round() as u64,
            [0, 0, 0],
            Laufstand::default(),
        ));
        t += 0.25;
        kaempfe += rate * 0.25;
    }
    kaempfe - rate * 0.25
}

#[test]
fn ac4_gleitend_mittelt_ueber_die_letzten_dreissig_sekunden() {
    assert_eq!(FENSTER, Duration::from_secs(30));
    let mut fenster = Fenster::neu(FENSTER);
    let stand = gleichmaessig(&mut fenster, 60.0, 0.0, 1000.0, 0.0);
    // Ab 60 s gedrosselt auf ein Zehntel.
    let _ = gleichmaessig(&mut fenster, 75.0, 60.25, 100.0, stand + 25.0);
    let r = fenster.raten().expect("Raten");
    assert!(
        nah(r.kaempfe_gleitend, (15.0 * 100.0 + 15.0 * 1000.0) / 30.0),
        "15 s nach der Drosselung: {} statt 550",
        r.kaempfe_gleitend
    );
    assert!(nah(r.fenster_s, 30.0), "Fenster {} s", r.fenster_s);

    let mut fenster = Fenster::neu(FENSTER);
    let stand = gleichmaessig(&mut fenster, 60.0, 0.0, 1000.0, 0.0);
    let ende = gleichmaessig(&mut fenster, 95.0, 60.25, 100.0, stand + 25.0);
    let r = fenster.raten().expect("Raten");
    assert!(
        nah(r.kaempfe_gleitend, 100.0),
        "35 s nach der Drosselung zählt nur noch die neue Rate: {}",
        r.kaempfe_gleitend
    );
    let gesamtmittel = ende / 95.0;
    assert!(
        gesamtmittel > 600.0,
        "Kontrolle: der Gesamtmittelwert ({gesamtmittel}) liegt weit daneben"
    );
}

#[test]
fn ac4_am_anfang_mittelt_das_fenster_ueber_den_bisherigen_lauf() {
    let mut fenster = Fenster::neu(FENSTER);
    let _ = gleichmaessig(&mut fenster, 10.0, 0.0, 400.0, 0.0);
    let r = fenster.raten().expect("Raten");
    assert!(nah(r.fenster_s, 10.0), "Fenster {} s statt 10", r.fenster_s);
    assert!(nah(r.kaempfe_gleitend, 400.0));
}

#[test]
fn aktuell_mittelt_ueber_eine_sekunde() {
    let mut fenster = Fenster::neu(FENSTER);
    let stand = gleichmaessig(&mut fenster, 20.0, 0.0, 1000.0, 0.0);
    let _ = gleichmaessig(&mut fenster, 22.0, 20.25, 3000.0, stand + 750.0);
    let r = fenster.raten().expect("Raten");
    assert!(
        nah(r.kaempfe_aktuell, 3000.0),
        "aktuell {} statt 3000",
        r.kaempfe_aktuell
    );
    assert!(r.kaempfe_gleitend < 2000.0);
}

#[test]
fn strangrate_braucht_vier_sekundenabschnitte_und_traegt_die_streuung() {
    let mut fenster = Fenster::neu(FENSTER);
    let mut kaempfe = 0u64;
    let mut t = 0.0;
    // Abwechselnd 900 und 1100 Kämpfe je Sekunde auf einem Strang.
    for sekunde in 0..8u32 {
        let je_viertel = if sekunde % 2 == 0 { 225 } else { 275 };
        for _ in 0..4 {
            fenster.aufnehmen(probe(t, kaempfe, [0, 0, 0], Laufstand::default()));
            t += 0.25;
            kaempfe += je_viertel;
        }
        if sekunde == 2 {
            assert!(
                fenster.strangrate().is_none(),
                "nach drei Sekunden gibt es noch keine vier Abschnitte"
            );
        }
    }
    fenster.aufnehmen(probe(t, kaempfe, [0, 0, 0], Laufstand::default()));
    let r = fenster.strangrate().expect("acht Abschnitte");
    assert!(nah(r.rate_per_s, 1000.0), "Rate {}", r.rate_per_s);
    assert!(nah(r.spread, 0.1), "Streuung {}", r.spread);
    assert_eq!(r.samples, 8);
}

#[test]
fn aktive_straenge_sind_die_der_letzten_sekunde() {
    let mut fenster = Fenster::neu(FENSTER);
    for i in 0..=12u32 {
        let t = f64::from(i) * 0.25;
        let mut p = probe(t, 0, [0, 0, 0], Laufstand::default());
        // Platz 1 rechnet die ganze Zeit, Platz 2 hört nach einer Sekunde auf.
        p.kaempfe_je_strang = vec![0, u64::from(i) * 10, u64::from(i.min(4)) * 10];
        p.kaempfe = p.kaempfe_je_strang.iter().sum();
        fenster.aufnehmen(p);
    }
    assert_eq!(fenster.raten().expect("Raten").aktive_straenge, 1);
}

// ---------------------------------------------------------------- AC6

#[test]
fn ac6_zehn_rechner_machen_eine_kette_nicht_schneller() {
    let kette = [kaempfe_stufe(1, 1000)];
    let einer = ablauf(&kette, &[rechner("a", 1, 1000.0, 0.0)]).expect("Ablauf");
    let zehn: Vec<Rechner> = (0..10)
        .map(|i| rechner(&format!("r{i}"), 1, 1000.0, 0.0))
        .collect();
    let verteilt = ablauf(&kette, &zehn).expect("Ablauf");
    assert!(nah(einer.dauer_s, 1.0), "Dauer {} s", einer.dauer_s);
    assert!(
        nah(verteilt.dauer_s, 1.0),
        "zehn Rechner verkürzen eine Kette aus 1000 Sammelpunkten: {}",
        verteilt.dauer_s
    );
    assert!(nah(verteilt.naiv_s, 0.1), "die naive Division sagt 0,1 s");
    assert!(nah(verteilt.kritischer_pfad_s, verteilt.dauer_s));
}

#[test]
fn ac6_eine_parallele_stufe_endet_mit_ihrem_langsamsten_strang() {
    // Fünf Kämpfe über vier Stränge: Einer rechnet zwei.
    let a = ablauf(&[kaempfe_stufe(5, 10)], &[rechner("a", 4, 100.0, 0.0)]).expect("Ablauf");
    assert!(nah(a.dauer_s, 10.0 * 2.0 / 100.0), "Dauer {} s", a.dauer_s);
    assert!(nah(a.naiv_s, 10.0 * 5.0 / 400.0), "naiv {} s", a.naiv_s);
    assert!(nah(a.kritischer_pfad_s, 10.0 / 100.0));
}

#[test]
fn ac6_verschieden_schnelle_rechner_teilen_nach_leistung() {
    // Vier Kämpfe, ein Strang mit 3/s und einer mit 1/s: drei gegen einen,
    // beide nach genau einer Sekunde fertig.
    let a = ablauf(
        &[kaempfe_stufe(4, 1)],
        &[
            rechner("schnell", 1, 3.0, 0.0),
            rechner("langsam", 1, 1.0, 0.0),
        ],
    )
    .expect("Ablauf");
    assert!(nah(a.dauer_s, 1.0), "Dauer {} s", a.dauer_s);
    assert_eq!(a.rechner, 2);
}

#[test]
fn ac6_ein_rechner_ohne_die_arbeitsart_bekommt_keine_aufgabe() {
    let avx = Stufe {
        art: Arbeitsart::AvxFullFight,
        ..kaempfe_stufe(8, 1)
    };
    let gonet = rechner("gonet", 4, 100.0, 0.0);
    assert_eq!(
        ablauf(&[avx], std::slice::from_ref(&gonet)),
        Err(rota_sim::fortschritt::Ablauffehler::KeinRechner(
            Arbeitsart::AvxFullFight
        ))
    );
    let laptop = Rechner {
        id: "laptop".to_owned(),
        straenge: 2,
        raten: vec![(Arbeitsart::AvxFullFight, rate(100.0, 0.0))],
        trend: 0.0,
    };
    let a = ablauf(&[avx], &[gonet, laptop]).expect("Ablauf");
    assert!(
        nah(a.dauer_s, 4.0 / 100.0),
        "nur die zwei Stränge des Laptops rechnen: {}",
        a.dauer_s
    );
    assert_eq!(a.rechner, 1);
}

// ---------------------------------------------------------------- AC5

fn exakt(weg: Weg, kaempfe: u64, anzahl: u64) -> Restanteil {
    Restanteil {
        weg,
        kaempfe_je_kandidat: Kampfzahl::Fest(kaempfe),
        mindestens: anzahl,
        hoechstens: anzahl,
    }
}

fn mitte(r: &Restzeit) -> f64 {
    match r {
        Restzeit::Spanne {
            mitte_s: Some(m), ..
        } => *m,
        anderes => panic!("keine Restzeit mit Mitte: {anderes:?}"),
    }
}

fn raten_mit(kandidaten_je_s: f64) -> rota_sim::fortschritt::Raten {
    rota_sim::fortschritt::Raten {
        kaempfe_aktuell: 1000.0,
        kaempfe_gleitend: 1000.0,
        kandidaten_aktuell: kandidaten_je_s,
        kandidaten_gleitend: kandidaten_je_s,
        wege_gleitend: [kandidaten_je_s, 0.0, 0.0],
        fenster_s: 30.0,
        aktive_straenge: 1,
    }
}

#[test]
fn ac5_gemischte_kosten_ergeben_eine_andere_restzeit_als_die_division() {
    // 900 billige Auswertungen und 100 teure, das Fenster sah nur billige.
    let anteile = [
        exakt(Weg::EinKampf, 1, 900),
        exakt(Weg::Stichprobe, 1000, 100),
    ];
    let r = restzeit(
        &anteile,
        0,
        &[rechner("a", 1, 1000.0, 0.0)],
        Some(&raten_mit(900.0)),
        0.0,
    );
    let gewichtet = mitte(&r);
    assert!(
        nah(gewichtet, 0.9 + 100.0),
        "gewichtet 100,9 s erwartet, bekommen {gewichtet}"
    );
    let Restzeit::Spanne { herleitung, .. } = r else {
        unreachable!()
    };
    let naiv = herleitung.naiv_kandidaten_s.expect("Vergleichswert");
    assert!(nah(naiv, 1000.0 / 900.0), "naiv {naiv}");
    assert!(
        gewichtet / naiv > 50.0,
        "die Kontrollprobe: gewichtet {gewichtet} gegen naiv {naiv}"
    );
}

#[test]
fn ac5_die_gewichtung_folgt_der_kampfzahl() {
    let r = |kaempfe| {
        mitte(&restzeit(
            &[exakt(Weg::Stichprobe, kaempfe, 10)],
            0,
            &[rechner("a", 1, 100.0, 0.0)],
            None,
            0.0,
        ))
    };
    assert!(nah(r(50), 5.0));
    assert!(nah(r(100), 10.0), "doppelte Kampfzahl, doppelte Restzeit");
}

#[test]
fn begonnene_kaempfe_der_laufenden_auswertung_zaehlen_ab() {
    let r = restzeit(
        &[exakt(Weg::Stichprobe, 100, 2)],
        40,
        &[rechner("a", 1, 100.0, 0.0)],
        None,
        0.0,
    );
    assert!(nah(mitte(&r), 1.6), "{r:?}");
}

// ---------------------------------------------------------------- AC7

fn herleitung_von(r: &Restzeit) -> rota_sim::fortschritt::Herleitung {
    match r {
        Restzeit::Spanne { herleitung, .. } => *herleitung,
        anderes => panic!("keine Spanne: {anderes:?}"),
    }
}

#[test]
fn ac7_ohne_streuung_und_bei_exakter_menge_ist_die_spanne_null() {
    let r = restzeit(
        &[exakt(Weg::EinKampf, 1, 1000)],
        0,
        &[rechner("a", 4, 1000.0, 0.0)],
        None,
        0.0,
    );
    let Restzeit::Spanne {
        unten_s, oben_s, ..
    } = r
    else {
        panic!("{r:?}")
    };
    assert!(nah(unten_s, 1.0) && nah(oben_s, 1.0), "{unten_s}..{oben_s}");
    let h = herleitung_von(&r);
    assert_eq!(h.restkosten, Restkosten::Exakt { kaempfe: 1000 });
    assert!(
        nah(h.naiv_division_s, 0.25),
        "vier Stränge teilen die naive Division"
    );
}

#[test]
fn ac7_die_ratenstreuung_geht_auf_einem_rechner_voll_ein() {
    let r = restzeit(
        &[exakt(Weg::Stichprobe, 1000, 10)],
        0,
        &[rechner("a", 10, 100.0, 0.1)],
        None,
        0.0,
    );
    let h = herleitung_von(&r);
    let m = mitte(&r);
    assert!(nah(m, 10.0), "Mitte {m} s");
    // Ein Rechner: seine Stränge drosseln gemeinsam, nichts mittelt sich weg.
    assert!(nah(h.sigma_rate_s, 0.1 * 10.0), "σ {}", h.sigma_rate_s);
    let Restzeit::Spanne {
        unten_s, oben_s, ..
    } = r
    else {
        unreachable!()
    };
    assert!(nah(unten_s, 9.0) && nah(oben_s, 11.0));
}

#[test]
fn ac7_mehr_rechner_mitteln_nur_den_parallelen_teil() {
    // Eine Stufe aus 1000 Kämpfen über zehn Rechner mit je einem Strang:
    // kritischer Pfad 0,01 s, Dauer 1 s.
    let zehn: Vec<Rechner> = (0..10)
        .map(|i| rechner(&format!("r{i}"), 1, 100.0, 0.1))
        .collect();
    let r = restzeit(&[exakt(Weg::Stichprobe, 1000, 1)], 0, &zehn, None, 0.0);
    let h = herleitung_von(&r);
    assert!(nah(h.kritischer_pfad_s, 0.01));
    let erwartet = 0.1 * (0.01 + (1.0 - 0.01) / 10f64.sqrt());
    assert!(
        nah(h.sigma_rate_s, erwartet),
        "σ {} statt {erwartet}",
        h.sigma_rate_s
    );
    // Und eine Kette über dieselben zehn: Nichts mittelt sich weg.
    let kette = restzeit(&[exakt(Weg::EinKampf, 1, 100)], 0, &zehn, None, 0.0);
    let hk = herleitung_von(&kette);
    assert!(
        nah(hk.sigma_rate_s, 0.1 * mitte(&kette)),
        "eine Kette behält die volle Streuung: {}",
        hk.sigma_rate_s
    );
}

#[test]
fn ac7_wiederholungen_verschieben_die_mitte_und_weiten_die_spanne() {
    let ohne = restzeit(
        &[exakt(Weg::EinKampf, 1, 100)],
        0,
        &[rechner("a", 1, 100.0, 0.0)],
        None,
        0.0,
    );
    let mit = restzeit(
        &[exakt(Weg::EinKampf, 1, 100)],
        0,
        &[rechner("a", 1, 100.0, 0.0)],
        None,
        0.1,
    );
    assert!(nah(mitte(&mit), mitte(&ohne) / 0.9));
    let h = herleitung_von(&mit);
    assert!(h.sigma_wiederholung_s > 0.0);
    assert!(nah(
        h.sigma_wiederholung_s,
        mitte(&mit) * (0.1f64 / 100.0).sqrt()
    ));
    assert!(nah(herleitung_von(&ohne).sigma_wiederholung_s, 0.0));
}

#[test]
fn ac7_ein_trend_der_rate_weitet_die_spanne() {
    // Der Fall aus der Messung vom 2026-10-28: Die Rate wandert, weil spätere
    // Kämpfe teurer werden. Ohne den Term lag die obere Grenze unter der
    // tatsächlichen Restzeit.
    let mut wandernd = rechner("a", 10, 100.0, 0.1);
    wandernd.trend = 0.2;
    let r = restzeit(
        &[exakt(Weg::Stichprobe, 1000, 10)],
        0,
        &[wandernd],
        None,
        0.0,
    );
    let h = herleitung_von(&r);
    assert!(nah(h.kosten_trend, 0.2));
    assert!(
        nah(h.sigma_kosten_s, 0.2 * 10.0),
        "σ_kosten {}",
        h.sigma_kosten_s
    );
    let Restzeit::Spanne {
        unten_s, oben_s, ..
    } = r
    else {
        unreachable!()
    };
    // σ_rate 1,0 und σ_kosten 2,0 addieren sich: beide gehören zur Rate.
    assert!(
        nah(unten_s, 7.0) && nah(oben_s, 13.0),
        "{unten_s}..{oben_s}"
    );
}

#[test]
fn der_trend_ist_die_verschiebung_zwischen_den_fensterhaelften() {
    let mut fenster = Fenster::neu(FENSTER);
    let mut kaempfe = 0u64;
    let mut t = 0.0;
    // Vier Sekunden mit 1000, vier mit 800 Kämpfen je Sekunde.
    for sekunde in 0..8u32 {
        let je_viertel = if sekunde < 4 { 250 } else { 200 };
        for _ in 0..4 {
            fenster.aufnehmen(probe(t, kaempfe, [0, 0, 0], Laufstand::default()));
            t += 0.25;
            kaempfe += je_viertel;
        }
    }
    fenster.aufnehmen(probe(t, kaempfe, [0, 0, 0], Laufstand::default()));
    let trend = fenster.strangtrend().expect("acht Abschnitte");
    assert!(nah(trend, 0.2), "Trend {trend}");
    // Und ohne Wandern kein Trend — sonst weitete der Term jede Spanne.
    assert!(nah(
        fenster_mit(Laufstand::default())
            .strangtrend()
            .expect("Trend"),
        0.0
    ));
}

#[test]
fn ac7_eine_obere_schranke_gibt_eine_spanne_ohne_mitte() {
    let r = restzeit(
        &[Restanteil {
            weg: Weg::EinKampf,
            kaempfe_je_kandidat: Kampfzahl::Fest(1),
            mindestens: 10,
            hoechstens: 100,
        }],
        0,
        &[rechner("a", 1, 10.0, 0.0)],
        None,
        0.0,
    );
    let Restzeit::Spanne {
        mitte_s,
        unten_s,
        oben_s,
        herleitung,
    } = r
    else {
        panic!("{r:?}")
    };
    assert_eq!(mitte_s, None, "eine Obergrenze hat keine Mitte");
    assert!(
        nah(unten_s, 1.0) && nah(oben_s, 10.0),
        "{unten_s}..{oben_s}"
    );
    assert_eq!(
        herleitung.restkosten,
        Restkosten::Spanne {
            mindestens: 10,
            hoechstens: 100
        }
    );
}

#[test]
fn ohne_rate_gibt_es_einen_grund_statt_einer_zahl() {
    let r = restzeit(&[exakt(Weg::EinKampf, 1, 10)], 0, &[], None, 0.0);
    assert!(matches!(r, Restzeit::Unbestimmt(_)), "{r:?}");
}

// ---------------------------------------------------------------- AC3 und Anzeige

fn beschreibung(kosten: Kostenbild) -> Laufbeschreibung {
    let mut b = Laufbeschreibung::neu("greedy", Some("boss-1-1"), 1, kosten);
    b.run_id = "pruef-1".to_owned();
    b.host_id = "pruefrechner".to_owned();
    b.straenge_erlaubt = Some(16);
    b
}

const EIN_KAMPF: Kostenbild = Kostenbild {
    weg: Weg::EinKampf,
    kaempfe_je_kandidat: Kampfzahl::Fest(1),
};

/// Ein Fenster über acht Sekunden mit 100 Auswertungen je Sekunde.
fn fenster_mit(stand: Laufstand) -> Fenster {
    let mut fenster = Fenster::neu(FENSTER);
    for i in 0..=32u64 {
        let n = i * 25;
        let mut p = probe(i as f64 * 0.25, n, [n, 0, 0], stand);
        p.durchgaenge = n * 50;
        fenster.aufnehmen(p);
    }
    fenster
}

fn anzeige_zu(stand: Laufstand) -> rota_sim::fortschritt::Anzeige {
    beschreibung(EIN_KAMPF)
        .anzeige(
            &fenster_mit(stand),
            Ereignis::Fortschritt,
            4812,
            Duration::from_secs(1_787_000_000),
        )
        .expect("eine Anzeige")
}

fn json(zeile: &str) -> serde_json::Value {
    serde_json::from_str(zeile).unwrap_or_else(|e| panic!("kein JSON ({e}): {zeile}"))
}

fn plateau_stand() -> Laufstand {
    Laufstand {
        menge: Arbeitsmenge::Offen,
        runde: Some(7),
        runde_ab_kandidaten: 700,
        runde_geplant: 120,
        bestes: Some(Bestes {
            wert: 14_208.0,
            seit_ns: 3_000_000_000,
            bei_kandidaten: 300,
        }),
    }
}

#[test]
fn ac3_eine_offene_suche_zeigt_keine_restzeit() {
    let stand = plateau_stand();
    let fenster = fenster_mit(stand);
    let r = restzeit_aus(
        fenster.letzte().expect("Probe"),
        &EIN_KAMPF,
        &[rechner("a", 1, 100.0, 0.0)],
        fenster.raten().as_ref(),
        0.0,
    );
    assert_eq!(r, Restzeit::Plateau, "eine offene Suche bekommt eine Zahl");

    let a = anzeige_zu(stand);
    let zeile = a
        .zeilen()
        .into_iter()
        .find(|z| z.trim_start().starts_with("Restzeit"))
        .expect("eine Zeile 'Restzeit'");
    assert!(zeile.contains("offene Suche"), "{zeile}");
    assert!(
        !zeile.chars().any(|c| c.is_ascii_digit()),
        "die Restzeit-Zeile einer offenen Suche nennt eine Zahl: {zeile}"
    );

    let j = json(&a.jsonl("pruef-1", "pruefrechner"));
    let eta = &j["metrics"]["eta"];
    assert_eq!(eta["kind"], "none", "{eta}");
    for feld in ["mid_s", "low_s", "high_s", "derivation"] {
        assert!(eta.get(feld).is_none(), "eta trägt '{feld}': {eta}");
    }
    let plateau = &j["metrics"]["plateau"];
    assert_eq!(plateau["best"], 14_208.0);
    assert_eq!(plateau["round"], 7);
    assert!(plateau["best_unchanged_s"].as_f64().expect("Zahl") > 4.0);
    assert_eq!(plateau["evaluations_since_best"], 800 - 300);

    // Auch in der ersten Sekunde, ohne jede Rate: das Plateau, nicht „noch
    // keine vier Messsekunden". Die Entscheidung fällt vor jeder Rechnung.
    let mut kurz = Fenster::neu(FENSTER);
    for i in 0..=4u64 {
        kurz.aufnehmen(probe(i as f64 * 0.25, i * 25, [i * 25, 0, 0], stand));
    }
    assert!(kurz.strangrate().is_none(), "Zusicherung: noch keine Rate");
    let frueh = beschreibung(EIN_KAMPF)
        .anzeige(&kurz, Ereignis::Fortschritt, 1, Duration::ZERO)
        .expect("Anzeige");
    assert_eq!(frueh.restzeit, Restzeit::Plateau);
}

#[test]
fn ac3_eine_bekannte_menge_zeigt_eine_restzeit() {
    let stand = Laufstand {
        menge: Arbeitsmenge::Bekannt { kandidaten: 2000 },
        ..Laufstand::default()
    };
    let a = anzeige_zu(stand);
    assert!(
        matches!(
            a.restzeit,
            Restzeit::Spanne {
                mitte_s: Some(_),
                ..
            }
        ),
        "{:?}",
        a.restzeit
    );
    let zeile = a
        .zeilen()
        .into_iter()
        .find(|z| z.trim_start().starts_with("Restzeit"))
        .expect("eine Zeile 'Restzeit'");
    assert!(zeile.contains('±'), "{zeile}");
    let j = json(&a.jsonl("pruef-1", "pruefrechner"));
    assert_eq!(j["metrics"]["eta"]["kind"], "estimate");
    assert!(j["metrics"]["eta"]["mid_s"].as_f64().is_some());
}

#[test]
fn ac3_die_gierige_suche_zeigt_eine_obergrenze() {
    let stand = Laufstand {
        menge: Arbeitsmenge::Hoechstens {
            mindestens: 900,
            hoechstens: 3000,
        },
        runde: Some(2),
        runde_ab_kandidaten: 700,
        runde_geplant: 200,
        ..Laufstand::default()
    };
    let a = anzeige_zu(stand);
    let j = json(&a.jsonl("pruef-1", "pruefrechner"));
    let eta = &j["metrics"]["eta"];
    assert_eq!(eta["kind"], "estimate");
    assert!(
        eta["mid_s"].is_null(),
        "eine Obergrenze hat keine Mitte: {eta}"
    );
    assert!(eta["low_s"].as_f64().expect("Zahl") < eta["high_s"].as_f64().expect("Zahl"));
}

#[test]
fn ac1_die_anzeige_enthaelt_alle_angaben() {
    let stand = Laufstand {
        menge: Arbeitsmenge::Hoechstens {
            mindestens: 900,
            hoechstens: 3000,
        },
        runde: Some(2),
        runde_ab_kandidaten: 700,
        runde_geplant: 200,
        bestes: Some(Bestes {
            wert: 14_208.0,
            seit_ns: 3_000_000_000,
            bei_kandidaten: 300,
        }),
    };
    let text = anzeige_zu(stand).zeilen().join("\n");
    for angabe in [
        "greedy",
        "boss-1-1",
        "Runde 2",
        "█",
        "Kandidaten",
        "800",
        "seit 00:00:08",
        "Durchsatz",
        "⌀",
        "davon",
        "ein Kampf",
        "Kernel",
        "roh",
        "Restzeit",
        "Systeme",
        "pruefrechner",
        "Stränge",
        "Drosselung",
        "Engpass",
        "keine Diagnose",
        "Bestes",
        "14 208,0",
        "unverändert",
    ] {
        assert!(text.contains(angabe), "'{angabe}' fehlt in:\n{text}");
    }
}

#[test]
fn ac2_beide_durchsatzzahlen_stehen_nebeneinander_und_laufen_auseinander() {
    // Eine Stichprobe mit 50 Kämpfen je Auswertung: Kernel und Suche liegen
    // um Faktor 50 auseinander.
    let mut fenster = Fenster::neu(FENSTER);
    for i in 0..=32u64 {
        let n = i * 25;
        fenster.aufnehmen(probe(
            i as f64 * 0.25,
            n * 50,
            [0, n, 0],
            Laufstand::default(),
        ));
    }
    let kosten = Kostenbild {
        weg: Weg::Stichprobe,
        kaempfe_je_kandidat: Kampfzahl::Fest(50),
    };
    let a = beschreibung(kosten)
        .anzeige(&fenster, Ereignis::Fortschritt, 1, Duration::ZERO)
        .expect("Anzeige");
    let j = json(&a.jsonl("pruef-1", "pruefrechner"));
    let durchsatz = &j["metrics"]["throughput"];
    let kandidaten = durchsatz["resolved_candidates_per_s"]["window"]
        .as_f64()
        .expect("Zahl");
    let kaempfe = durchsatz["full_fights_per_s"]["window"]
        .as_f64()
        .expect("Zahl");
    assert!(nah(kandidaten, 100.0), "Kandidaten {kandidaten} /s");
    assert!(nah(kaempfe, 5000.0), "Kämpfe {kaempfe} /s");
    let text = a.zeilen().join("\n");
    assert!(text.contains("100 /s"), "{text}");
    assert!(text.contains("5 000 /s"), "{text}");
}

#[test]
fn ac8_zeile_und_jsonl_kommen_aus_derselben_momentaufnahme() {
    let a = anzeige_zu(plateau_stand());
    let j = json(&a.jsonl("pruef-1", "pruefrechner"));
    assert_eq!(j["elapsed_ns"], a.probe.elapsed_ns);
    assert_eq!(j["host_sequence"], a.sequenz);
    assert_eq!(j["metrics"]["fights"], a.probe.kaempfe);
    assert_eq!(j["metrics"]["samples"], a.probe.durchgaenge);
    assert_eq!(
        j["metrics"]["candidates"]["done"],
        a.probe.kandidaten_gesamt()
    );
    let text = a.zeilen().join("\n");
    assert!(
        text.contains(&format!("#{}", a.sequenz)),
        "die Zeile nennt ihre Meldungsnummer nicht:\n{text}"
    );
    assert!(text.contains(&format_ganzzahl(a.probe.kandidaten_gesamt())));
}

#[test]
fn ac8_der_strom_traegt_das_schema() {
    let a = anzeige_zu(plateau_stand());
    let j = json(&a.jsonl("pruef-1", "pruefrechner"));
    assert_eq!(j["schema_version"], 1);
    assert_eq!(j["run_id"], "pruef-1");
    assert_eq!(j["event"], "progress");
    assert_eq!(j["host_id"], "pruefrechner");
    assert_eq!(j["phase"], "greedy");
    assert!(j["metrics"].is_object());
    let zeit = j["timestamp_utc"].as_str().expect("Zeichenkette");
    // date -u -d @1787000000  ->  2026-08-17T20:53:20
    assert_eq!(zeit, "2026-08-17T20:53:20.000000Z");
    assert!(j["elapsed_ns"].as_u64().is_some());
    assert!(j["host_sequence"].as_u64().is_some());
}

#[test]
fn utc_zeitstempel_stimmen_mit_bekannten_daten() {
    assert_eq!(utc_iso8601(Duration::ZERO), "1970-01-01T00:00:00.000000Z");
    // date -u -d '2026-08-16T20:31:12Z' +%s  ->  1786912272
    assert_eq!(
        utc_iso8601(Duration::new(1_786_912_272, 123_456_789)),
        "2026-08-16T20:31:12.123456Z"
    );
    // Schalttag: date -u -d '2024-02-29T23:59:59Z' +%s  ->  1709251199
    assert_eq!(
        utc_iso8601(Duration::from_secs(1_709_251_199)),
        "2024-02-29T23:59:59.000000Z"
    );
    // date -u -d '2000-03-01T00:00:00Z' +%s  ->  951868800
    assert_eq!(
        utc_iso8601(Duration::from_secs(951_868_800)),
        "2000-03-01T00:00:00.000000Z"
    );
}

#[test]
fn engpass_sagt_keine_diagnose() {
    let a = anzeige_zu(plateau_stand());
    let zeile = a
        .zeilen()
        .into_iter()
        .find(|z| z.trim_start().starts_with("Engpass"))
        .expect("eine Zeile 'Engpass'");
    assert!(zeile.contains("keine Diagnose"), "{zeile}");
    assert!(zeile.contains("#934"), "{zeile}");
    let j = json(&a.jsonl("pruef-1", "pruefrechner"));
    assert!(j["metrics"]["bottleneck"]["diagnosis"].is_null());
}

#[test]
fn wege_ohne_lauf_stehen_mit_namen_und_nicht_als_null() {
    let j = json(&anzeige_zu(plateau_stand()).jsonl("pruef-1", "pruefrechner"));
    let wege = &j["metrics"]["paths"];
    let ohne: Vec<&str> = wege["not_in_run"]
        .as_array()
        .expect("Liste")
        .iter()
        .map(|w| w.as_str().expect("Name"))
        .collect();
    assert_eq!(ohne, WEGE_OHNE_LAUF);
    for name in WEGE_OHNE_LAUF {
        assert!(wege.get(name).is_none(), "'{name}' steht mit einer Zahl da");
    }
    for weg in Weg::ALLE {
        assert!(wege[weg.schluessel()]["done"].as_u64().is_some(), "{wege}");
    }
}

#[test]
fn ganzzahlen_mit_tausendertrenner() {
    assert_eq!(format_ganzzahl(0), "0");
    assert_eq!(format_ganzzahl(999), "999");
    assert_eq!(format_ganzzahl(1000), "1 000");
    assert_eq!(format_ganzzahl(1_084_000), "1 084 000");
}

// ---------------------------------------------------------------- Anzeigestrang

#[derive(Clone, Default)]
struct Geteilt(Arc<Mutex<Vec<u8>>>);

impl Write for Geteilt {
    fn write(&mut self, daten: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("Sperre").extend_from_slice(daten);
        Ok(daten.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn der_anzeigestrang_meldet_im_viertelsekundentakt() {
    assert_eq!(ANZEIGETAKT, Duration::from_millis(250));
    let werk = Zaehlwerk::neu(1);
    werk.setze_menge(Arbeitsmenge::Bekannt { kandidaten: 1 });
    let strom = Geteilt::default();
    let zeile = Geteilt::default();
    let anzeige = Anzeigestrang::starten(
        Arc::clone(&werk),
        beschreibung(EIN_KAMPF),
        Ausgabe {
            zeile: Some(Box::new(zeile.clone())),
            terminal: false,
            jsonl: Some(Box::new(strom.clone())),
        },
    );
    std::thread::sleep(Duration::from_millis(1300));
    let ende = anzeige.beenden().expect("beendet");
    assert_eq!(ende.ereignis, Ereignis::Ende);

    let text = String::from_utf8(strom.0.lock().expect("Sperre").clone()).expect("UTF-8");
    let meldungen: Vec<serde_json::Value> = text.lines().map(json).collect();
    assert!(
        meldungen.len() >= 5,
        "{} Meldungen:\n{text}",
        meldungen.len()
    );
    assert_eq!(meldungen[0]["event"], "start");
    assert_eq!(meldungen.last().expect("eine")["event"], "end");
    for (i, m) in meldungen.iter().enumerate() {
        assert_eq!(
            m["host_sequence"],
            i as u64 + 1,
            "Lücke oder Sprung in der laufenden Nummer"
        );
    }
    let zeiten: Vec<u64> = meldungen
        .iter()
        .filter(|m| m["event"] == "progress")
        .map(|m| m["elapsed_ns"].as_u64().expect("Zahl"))
        .collect();
    assert!(zeiten.len() >= 3, "{} Fortschrittsmeldungen", zeiten.len());
    for paar in zeiten.windows(2) {
        assert!(
            paar[1] - paar[0] >= 240_000_000,
            "zwei Meldungen nur {} ns auseinander",
            paar[1] - paar[0]
        );
    }
    let menschen = String::from_utf8(zeile.0.lock().expect("Sperre").clone()).expect("UTF-8");
    assert!(
        menschen.contains("Engpass"),
        "die Zeile wurde nicht geschrieben:\n{menschen}"
    );
}
