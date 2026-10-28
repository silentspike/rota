//! Die Regeltabelle der Engpasserkennung (Issue #934, AC2 und AC5).
//!
//! Je Zeile zwei Fälle: einer, an dem sie anschlagen MUSS, und einer, an dem
//! sie schweigen MUSS. Ein Wächter, der nie angeschlagen hat, ist nicht
//! geprüft — und einer, der nie geschwiegen hat, auch nicht.
//!
//! Die Netz- und die Wegzeile laufen hier mit gebauten Messwerten: Es gibt
//! heute weder eine Verteilung über das Netz (E15.5) noch einen Weg mit
//! eigener Schwelle (E11.4b). Die übrigen Zeilen haben zusätzlich ihre Lage
//! im laufenden Programm (AC6, siehe den Bericht zu #934).

use rota_sim::engpass::{
    diagnose, Ablauftreffer, Druck, Grundlage, Kernklasse, Messwerte, Netzwerte, Regel,
    Schlangenbelegung, Schluessel, Strangzeit, Vergleichswerte, Wegtreffer, Zaehlerlage,
    Zaehlerwerte, ABLAUF_TREFFER_ANTEIL, DRITTE_EBENE_ANTEIL, DURCHSATZ_MINUS, FEHLSPRUENGE_PLUS,
    NETZ_WARTEN_ANTEIL, SCHLANGE_BELEGUNG_ANTEIL, SPEICHER_WARTEZEIT_PLUS, STRANG_WARTEN_ANTEIL,
    ZYKLEN_JE_EREIGNIS_PLUS,
};

/// Der Schlüssel aller Fälle — ein Rechner, eine Klasse, ein Profil.
fn schluessel() -> Schluessel {
    Schluessel {
        rechner: "pruefrechner".to_owned(),
        kernklasse: Kernklasse::Zen3,
        lastprofil: "bench boss=boss-1-1 fights=400000 threads=4 bau=release".to_owned(),
    }
}

/// Die Zählerwerte der Grundlage: die Zahlen aus dem Issue (Stand
/// 2026-08-16) — Befehlsdurchsatz 2,65, 155 Fehlsprünge, 59 340 Zyklen je
/// Kampf.
fn zaehler_ruhig() -> Zaehlerwerte {
    Zaehlerwerte {
        kernklasse: Kernklasse::Zen3,
        ipc: Some(2.65),
        zyklen_je_kampf: Some(59_340.0),
        fehlspruenge_je_kampf: Some(155.0),
        l1_je_kampf: Some(900.0),
        l2_je_kampf: Some(40.0),
        l3_je_kampf: Some(4.0),
        speicherwartezyklen_je_kampf: Some(2_000.0),
        speicher_wartezeit_anteil: Some(2_000.0 / 59_340.0),
        abdeckung: [Some(0.5), Some(0.5)],
    }
}

/// Ein ruhiger Lauf: alles wie in der Grundlage, alle Anteile klein, und
/// jede Zeile, die heute erhoben werden kann, hat ihre Messwerte.
fn ruhig() -> Messwerte {
    Messwerte {
        sekunden: 30.0,
        kaempfe: 400_000,
        zaehler: Zaehlerlage::Gemessen(zaehler_ruhig()),
        cpu_ns_je_kampf: Some(18_000.0),
        straenge: vec![
            Strangzeit {
                arbeit_ns: 9_700,
                warten_ns: 300,
            };
            4
        ],
        wege: vec![Wegtreffer {
            weg: "ein Kampf (Reduktion, #908)",
            versuche: 0,
            treffer: 0,
            schwelle: None,
            ohne_schwelle: "die Prüfung kostet keinen Kampf",
        }],
        netz: None,
        schlange: Some(Schlangenbelegung {
            hoechststand: 9,
            kapazitaet: 256,
            ueberlaeufe: 0,
            schritte: 1_000_000,
        }),
        ablauf: Some(Ablauftreffer {
            aufrufe: 1_000_000,
            treffer: 30_000,
        }),
        druck: Druck {
            cpu: Some(0.5),
            speicher: Some(0.0),
            io: Some(0.0),
        },
    }
}

fn grundlage() -> Grundlage {
    Grundlage::aus_messwerten(schluessel(), &ruhig())
}

fn mit_zaehlern(f: impl FnOnce(&mut Zaehlerwerte)) -> Messwerte {
    let mut m = ruhig();
    let Zaehlerlage::Gemessen(z) = &mut m.zaehler else {
        unreachable!("ruhig() hat Zähler")
    };
    f(z);
    m
}

/// Die Regeln, die angeschlagen haben, beim Namen.
fn befunde(m: &Messwerte, g: Option<&Grundlage>) -> Vec<Regel> {
    diagnose(m, &schluessel(), g)
        .befunde
        .iter()
        .map(|b| b.regel)
        .collect()
}

/// Ob eine Zeile ungeprüft blieb — und mit welchem Grund.
fn ungeprueft(m: &Messwerte, g: Option<&Grundlage>, regel: Regel) -> Option<String> {
    diagnose(m, &schluessel(), g)
        .ungeprueft
        .into_iter()
        .find(|(r, _)| *r == regel)
        .map(|(_, grund)| grund)
}

/// Werte, die eine Schwelle sicher überschreiten — mit Abstand, damit kein
/// Fall an der Rundung einer Grenze hängt.
fn ueber(g: f64, anteil: f64) -> f64 {
    g * (1.0 + anteil * 1.5)
}

fn unter(g: f64, anteil: f64) -> f64 {
    g * (1.0 - anteil * 1.5)
}

// ------------------------------------------------------------ die Kontrolle

/// **Die Kontrollprobe:** ein Lauf wie seine Grundlage. Keine Zeile darf
/// anschlagen — und es müssen genug geprüft sein, damit das Schweigen etwas
/// heißt.
#[test]
fn ein_ruhiger_lauf_bekommt_keine_diagnose() {
    let d = diagnose(&ruhig(), &schluessel(), Some(&grundlage()));
    assert!(d.befunde.is_empty(), "{:?}", d.befunde);
    for regel in [
        Regel::Verzweigungsgebunden,
        Regel::ZweiteEbene,
        Regel::StraengeWarten,
        Regel::AblaufAnnahme,
        Regel::SchlangeKnapp,
    ] {
        assert!(
            d.geprueft.contains(&regel),
            "{regel:?} nicht geprüft: {d:?}"
        );
    }
    assert!(
        d.satz().starts_with("keine Diagnose — 5 von 7"),
        "{}",
        d.satz()
    );
}

// ------------------------------------------------------------ Zeile 1

fn verzweigungsgebunden() -> Messwerte {
    let g = zaehler_ruhig();
    mit_zaehlern(|z| {
        z.ipc = Some(unter(g.ipc.unwrap(), DURCHSATZ_MINUS));
        z.fehlspruenge_je_kampf = Some(ueber(g.fehlspruenge_je_kampf.unwrap(), FEHLSPRUENGE_PLUS));
        z.zyklen_je_kampf = Some(ueber(g.zyklen_je_kampf.unwrap(), ZYKLEN_JE_EREIGNIS_PLUS));
    })
}

#[test]
fn zeile_1_verzweigungsgebunden_schlaegt_an() {
    assert_eq!(
        befunde(&verzweigungsgebunden(), Some(&grundlage())),
        [Regel::Verzweigungsgebunden]
    );
}

/// Jede der drei Bedingungen trägt einzeln — fehlt eine, schweigt die Zeile.
#[test]
fn zeile_1_braucht_alle_drei_bedingungen() {
    let g = zaehler_ruhig();
    let ohne_ipc = {
        let mut m = verzweigungsgebunden();
        let Zaehlerlage::Gemessen(z) = &mut m.zaehler else {
            unreachable!()
        };
        z.ipc = g.ipc;
        m
    };
    let ohne_spruenge = {
        let mut m = verzweigungsgebunden();
        let Zaehlerlage::Gemessen(z) = &mut m.zaehler else {
            unreachable!()
        };
        z.fehlspruenge_je_kampf = g.fehlspruenge_je_kampf;
        m
    };
    let ohne_zyklen = {
        let mut m = verzweigungsgebunden();
        let Zaehlerlage::Gemessen(z) = &mut m.zaehler else {
            unreachable!()
        };
        z.zyklen_je_kampf = g.zyklen_je_kampf;
        m
    };
    for (name, m) in [
        ("Durchsatz wie Grundlage", ohne_ipc),
        ("Fehlsprünge wie Grundlage", ohne_spruenge),
        ("Zyklen wie Grundlage", ohne_zyklen),
    ] {
        assert!(
            !befunde(&m, Some(&grundlage())).contains(&Regel::Verzweigungsgebunden),
            "{name}: Zeile 1 schlug trotzdem an"
        );
    }
}

// ------------------------------------------------------------ Zeile 2

fn zweite_ebene() -> Messwerte {
    let g = zaehler_ruhig();
    mit_zaehlern(|z| {
        z.zyklen_je_kampf = Some(ueber(g.zyklen_je_kampf.unwrap(), ZYKLEN_JE_EREIGNIS_PLUS));
        z.speicherwartezyklen_je_kampf = Some(ueber(
            g.speicherwartezyklen_je_kampf.unwrap(),
            SPEICHER_WARTEZEIT_PLUS,
        ));
        z.l2_je_kampf = Some(g.l2_je_kampf.unwrap() * 20.0);
        // Die dritte Ebene bleibt, wo sie war: Die Daten passen noch in L3.
    })
}

#[test]
fn zeile_2_zustand_zu_gross_fuer_l2_schlaegt_an() {
    assert_eq!(
        befunde(&zweite_ebene(), Some(&grundlage())),
        [Regel::ZweiteEbene]
    );
}

/// Geht die Mehrheit der zusätzlichen Fehlgriffe auch an der dritten Ebene
/// vorbei, ist der Zustand zu groß für L3 — das sagt die Tabelle nicht, also
/// schweigt die Zeile.
#[test]
fn zeile_2_schweigt_wenn_auch_die_dritte_ebene_steigt() {
    let mut m = zweite_ebene();
    let Zaehlerlage::Gemessen(z) = &mut m.zaehler else {
        unreachable!()
    };
    let zusaetzlich = z.l2_je_kampf.unwrap() - 40.0;
    z.l3_je_kampf = Some(4.0 + zusaetzlich * (DRITTE_EBENE_ANTEIL + 0.3));
    assert!(befunde(&m, Some(&grundlage())).is_empty());
    // Die Gegenrichtung desselben Falls: knapp unter der Hälfte schlägt sie an.
    let Zaehlerlage::Gemessen(z) = &mut m.zaehler else {
        unreachable!()
    };
    z.l3_je_kampf = Some(4.0 + zusaetzlich * (DRITTE_EBENE_ANTEIL - 0.1));
    assert_eq!(befunde(&m, Some(&grundlage())), [Regel::ZweiteEbene]);
}

/// **Gemessen am 2026-10-28 auf dem Laptop** (AC6, Lage „Speicher"): `bench`
/// auf vier Kernen, auf ihren Geschwistersträngen je 2 MiB Puffer. Die dritte
/// Ebene hat in der Grundlage fast nichts zu tun — 2,27 Fehlgriffe je Kampf —,
/// und 4 zusätzliche sind relativ +178 %. Von den 138 zusätzlichen Fehlgriffen
/// der zweiten Ebene gingen aber nur diese 4 auch an der dritten vorbei: Der
/// Zustand passt in L3, nicht in L2. Genau das sagt die Zeile.
#[test]
fn zeile_2_schlaegt_auch_bei_fast_leerer_dritter_ebene_an() {
    let mut g = grundlage();
    g.werte = Vergleichswerte {
        ipc: Some(2.737_962),
        zyklen_je_kampf: Some(45_064.12),
        fehlspruenge_je_kampf: Some(95.98),
        l2_je_kampf: Some(4.601_27),
        l3_je_kampf: Some(2.271_78),
        speicherwartezyklen_je_kampf: Some(2_539.013),
        cpu_ns_je_kampf: Some(23_986.98),
    };
    let lage = mit_zaehlern(|z| {
        z.ipc = Some(1.156_903);
        z.zyklen_je_kampf = Some(106_917.66);
        z.fehlspruenge_je_kampf = Some(104.25);
        z.l1_je_kampf = Some(2_303.87);
        z.l2_je_kampf = Some(142.399_3);
        z.l3_je_kampf = Some(6.320_2);
        z.speicherwartezyklen_je_kampf = Some(31_948.63);
        z.speicher_wartezeit_anteil = Some(0.298_6);
    });
    assert_eq!(befunde(&lage, Some(&g)), [Regel::ZweiteEbene]);
}

#[test]
fn zeile_2_braucht_speicherwartezeit_und_zyklen() {
    let g = zaehler_ruhig();
    let mut ohne_warten = zweite_ebene();
    let Zaehlerlage::Gemessen(z) = &mut ohne_warten.zaehler else {
        unreachable!()
    };
    z.speicherwartezyklen_je_kampf = g.speicherwartezyklen_je_kampf;
    assert!(befunde(&ohne_warten, Some(&grundlage())).is_empty());

    let mut ohne_zyklen = zweite_ebene();
    let Zaehlerlage::Gemessen(z) = &mut ohne_zyklen.zaehler else {
        unreachable!()
    };
    z.zyklen_je_kampf = g.zyklen_je_kampf;
    assert!(befunde(&ohne_zyklen, Some(&grundlage())).is_empty());
}

// ------------------------------------------------------------ Zeile 3

fn mit_warteanteil(anteil: f64) -> Messwerte {
    let mut m = ruhig();
    let warten = (anteil * 10_000.0).round() as u64;
    m.straenge = vec![
        Strangzeit {
            arbeit_ns: 10_000 - warten,
            warten_ns: warten,
        };
        8
    ];
    m
}

#[test]
fn zeile_3_straenge_warten_schlaegt_an() {
    let m = mit_warteanteil(STRANG_WARTEN_ANTEIL * 2.0);
    assert_eq!(befunde(&m, Some(&grundlage())), [Regel::StraengeWarten]);
    // Ohne Grundlage genauso: Ein Anteil der eigenen Zeit braucht keine.
    assert_eq!(befunde(&m, None), [Regel::StraengeWarten]);
}

#[test]
fn zeile_3_schweigt_unter_der_schwelle() {
    let m = mit_warteanteil(STRANG_WARTEN_ANTEIL / 2.0);
    assert!(befunde(&m, Some(&grundlage())).is_empty());
}

#[test]
fn zeile_3_ohne_parallelen_abschnitt_ist_ungeprueft() {
    let mut m = ruhig();
    m.straenge = vec![Strangzeit::default(); 4];
    let grund = ungeprueft(&m, Some(&grundlage()), Regel::StraengeWarten).expect("ungeprüft");
    assert!(grund.contains("Abschnitt"), "{grund}");
}

// ------------------------------------------------------------ Zeile 4

fn mit_netz(anteil: f64) -> Messwerte {
    let mut m = ruhig();
    m.netz = Some(Netzwerte {
        pakete: 120,
        wartezeit_ns: (anteil * 1e9) as u64,
        laufzeit_ns: 1_000_000_000,
        verworfen: 2,
    });
    m
}

#[test]
fn zeile_4_netzwartezeit_schlaegt_an() {
    assert_eq!(
        befunde(&mit_netz(NETZ_WARTEN_ANTEIL * 2.5), Some(&grundlage())),
        [Regel::NetzWarten]
    );
}

#[test]
fn zeile_4_schweigt_unter_der_schwelle_und_nennt_ohne_netz_den_grund() {
    assert!(befunde(&mit_netz(NETZ_WARTEN_ANTEIL / 2.0), Some(&grundlage())).is_empty());
    let grund = ungeprueft(&ruhig(), Some(&grundlage()), Regel::NetzWarten).expect("ungeprüft");
    assert!(grund.contains("E15.5"), "{grund}");
}

// ------------------------------------------------------------ Zeile 5

fn mit_weg(versuche: u64, treffer: u64, schwelle: f64) -> Messwerte {
    let mut m = ruhig();
    m.wege.push(Wegtreffer {
        weg: "cache",
        versuche,
        treffer,
        schwelle: Some(schwelle),
        ohne_schwelle: "",
    });
    m
}

#[test]
fn zeile_5_weg_unter_seiner_schwelle_schlaegt_an() {
    let m = mit_weg(1000, 100, 0.30);
    let d = diagnose(&m, &schluessel(), Some(&grundlage()));
    assert_eq!(d.befunde.len(), 1, "{d:?}");
    assert_eq!(d.befunde[0].regel, Regel::WegLohntNicht);
    assert!(
        d.befunde[0].beobachtung.contains("cache"),
        "{}",
        d.befunde[0].beobachtung
    );
}

/// Die Schwelle ist die **eigene** des Wegs: Dieselbe Quote lohnt bei einer
/// niedrigeren Schwelle.
#[test]
fn zeile_5_schweigt_ueber_der_eigenen_schwelle() {
    assert!(befunde(&mit_weg(1000, 100, 0.05), Some(&grundlage())).is_empty());
    // Ein Weg ohne eigene Schwelle wird nicht gegen eine fremde geprüft.
    let grund = ungeprueft(&ruhig(), Some(&grundlage()), Regel::WegLohntNicht).expect("ungeprüft");
    assert!(grund.contains("kein Weg mit eigener Schwelle"), "{grund}");
}

// ------------------------------------------------------------ Zeile 6

fn mit_ablauf(anteil: f64) -> Messwerte {
    let mut m = ruhig();
    m.ablauf = Some(Ablauftreffer {
        aufrufe: 10_000,
        treffer: (anteil * 10_000.0) as u64,
    });
    m
}

/// **Der eigentliche Zweck des Issues:** Die Annahme aus E11.2 ist
/// abgelaufen, und der Wächter sagt es.
#[test]
fn zeile_6_abgelaufene_annahme_schlaegt_an() {
    assert_eq!(
        befunde(
            &mit_ablauf(ABLAUF_TREFFER_ANTEIL * 1.75),
            Some(&grundlage())
        ),
        [Regel::AblaufAnnahme]
    );
}

#[test]
fn zeile_6_schweigt_unter_der_schwelle() {
    assert!(befunde(&mit_ablauf(ABLAUF_TREFFER_ANTEIL / 2.0), Some(&grundlage())).is_empty());
}

// ------------------------------------------------------------ Zeile 7

fn mit_schlange(hoechststand: usize, ueberlaeufe: u64) -> Messwerte {
    let mut m = ruhig();
    m.schlange = Some(Schlangenbelegung {
        hoechststand,
        kapazitaet: 256,
        ueberlaeufe,
        schritte: 1_000,
    });
    m
}

#[test]
fn zeile_7_schlange_fast_voll_schlaegt_an() {
    let fast_voll = (256.0 * (SCHLANGE_BELEGUNG_ANTEIL + 0.05)) as usize;
    assert_eq!(
        befunde(&mit_schlange(fast_voll, 0), Some(&grundlage())),
        [Regel::SchlangeKnapp]
    );
    // Ein Ausweichen in den Rückfall ist mehr als knapp — es ist geschehen.
    assert_eq!(
        befunde(&mit_schlange(9, 1), Some(&grundlage())),
        [Regel::SchlangeKnapp]
    );
}

#[test]
fn zeile_7_schweigt_bei_neun_von_256() {
    // Der Stand aus dem Issue (2026-08-16): höchstens neun gleichzeitige
    // Ereignisse.
    assert!(befunde(&mit_schlange(9, 0), Some(&grundlage())).is_empty());
    let halb = (256.0 * SCHLANGE_BELEGUNG_ANTEIL / 2.0) as usize;
    assert!(befunde(&mit_schlange(halb, 0), Some(&grundlage())).is_empty());
}

// ------------------------------------------------------------ AC2

/// **Dieselben Zahlen, zwei Grundlagen, zwei Antworten.** Ein
/// Befehlsdurchsatz von 1,8 ist gegen 2,65 ein Abfall um 32 %, gegen 1,9 nur
/// um 5 %. Eine feste Schwelle gäbe beiden Läufen dieselbe Antwort.
#[test]
fn ac2_die_schwellen_sind_hostrelativ() {
    let lauf = mit_zaehlern(|z| {
        z.ipc = Some(1.8);
        z.fehlspruenge_je_kampf = Some(300.0);
        z.zyklen_je_kampf = Some(90_000.0);
    });
    let langsamer_kern = {
        let mut g = grundlage();
        g.werte = Vergleichswerte {
            ipc: Some(1.9),
            fehlspruenge_je_kampf: Some(280.0),
            zyklen_je_kampf: Some(85_000.0),
            ..g.werte
        };
        g
    };
    assert_eq!(
        befunde(&lauf, Some(&grundlage())),
        [Regel::Verzweigungsgebunden]
    );
    assert!(befunde(&lauf, Some(&langsamer_kern)).is_empty());
}

/// **Keine feste Schwelle, auch nicht als Voreinstellung.** Ohne Grundlage
/// bleibt ein Befehlsdurchsatz von 0,1 ungeprüft — die Zeile fällt nicht auf
/// eine Zahl zurück.
#[test]
fn ac2_ohne_grundlage_gibt_es_keine_zaehlerdiagnose() {
    let m = mit_zaehlern(|z| {
        z.ipc = Some(0.1);
        z.fehlspruenge_je_kampf = Some(1e6);
        z.zyklen_je_kampf = Some(1e7);
        z.speicherwartezyklen_je_kampf = Some(1e7);
        z.l2_je_kampf = Some(1e5);
    });
    assert!(befunde(&m, None).is_empty());
    for regel in [Regel::Verzweigungsgebunden, Regel::ZweiteEbene] {
        let grund = ungeprueft(&m, None, regel).expect("ungeprüft");
        assert!(grund.contains("keine Grundlage"), "{regel:?}: {grund}");
    }
}

/// Die Grundlage muss zu Rechner, Kernklasse **und** Lastprofil passen.
#[test]
fn ac2_eine_fremde_grundlage_wird_nicht_benutzt() {
    let m = verzweigungsgebunden();
    for (was, aendern) in [
        (
            "anderer Rechner",
            (|s: &mut Schluessel| s.rechner = "node3".to_owned()) as fn(&mut Schluessel),
        ),
        ("andere Kernklasse", |s: &mut Schluessel| {
            s.kernklasse = Kernklasse::Zen5
        }),
        ("anderes Lastprofil", |s: &mut Schluessel| {
            s.lastprofil.push_str(" purity=500")
        }),
    ] {
        let mut g = grundlage();
        aendern(&mut g.schluessel);
        assert!(
            befunde(&m, Some(&g)).is_empty(),
            "{was}: trotzdem verglichen"
        );
        let grund = ungeprueft(&m, Some(&g), Regel::Verzweigungsgebunden).expect("ungeprüft");
        assert!(grund.contains(was), "{was}: {grund}");
    }
}

// ------------------------------------------------------------ AC4

fn ohne_zaehler(cpu_ns: f64, psi: Option<f64>) -> Messwerte {
    let mut m = ruhig();
    m.zaehler = Zaehlerlage::Fehlt(
        "perf_event_open für cycles gescheitert (errno 13 EACCES), perf_event_paranoid = 4"
            .to_owned(),
    );
    m.cpu_ns_je_kampf = Some(cpu_ns);
    m.druck.cpu = psi;
    m
}

/// Ohne Zähler fällt die Diagnose auf Zeitmessung und Druckwerte zurück —
/// sie fällt nicht aus.
#[test]
fn ac4_ohne_zaehler_meldet_die_zeitmessung_eine_verlangsamung() {
    let m = ohne_zaehler(ueber(18_000.0, ZYKLEN_JE_EREIGNIS_PLUS), Some(0.5));
    assert_eq!(befunde(&m, Some(&grundlage())), [Regel::ZeitOhneZaehler]);
    for regel in [Regel::Verzweigungsgebunden, Regel::ZweiteEbene] {
        let grund = ungeprueft(&m, Some(&grundlage()), regel).expect("ungeprüft");
        assert!(grund.contains("Rückfall"), "{grund}");
        assert!(grund.contains("EACCES"), "{grund}");
    }
}

#[test]
fn ac4_ohne_zaehler_schweigt_ein_ruhiger_lauf() {
    let d = diagnose(
        &ohne_zaehler(18_500.0, Some(0.5)),
        &schluessel(),
        Some(&grundlage()),
    );
    assert!(d.befunde.is_empty(), "{d:?}");
    assert!(d.geprueft.contains(&Regel::ZeitOhneZaehler));
}

/// **Unter Fremdlast misst die Uhr die Maschine, nicht den Lauf.** Über der
/// Schwelle des Projekts wird die Zeitzeile nicht geprüft — ohne Druckwert
/// auch nicht.
#[test]
fn ac4_fremdlast_macht_die_zeitmessung_unbrauchbar() {
    let langsam = ueber(18_000.0, ZYKLEN_JE_EREIGNIS_PLUS);
    let grenze = rota_sim::hostcaps::PSI_CPU_MAX;
    let m = ohne_zaehler(langsam, Some(grenze * 3.0));
    assert!(befunde(&m, Some(&grundlage())).is_empty());
    let grund = ungeprueft(&m, Some(&grundlage()), Regel::ZeitOhneZaehler).expect("ungeprüft");
    assert!(grund.contains("Fremdlast"), "{grund}");

    let ohne_psi = ohne_zaehler(langsam, None);
    assert!(befunde(&ohne_psi, Some(&grundlage())).is_empty());
}

/// Die übrigen Zeilen brauchen keine Zähler und laufen im Rückfall weiter.
#[test]
fn ac4_im_rueckfall_pruefen_die_zeilen_ohne_zaehler_weiter() {
    let mut m = ohne_zaehler(18_000.0, Some(0.5));
    m.straenge = mit_warteanteil(0.5).straenge;
    m.ablauf = mit_ablauf(0.5).ablauf;
    assert_eq!(
        befunde(&m, Some(&grundlage())),
        [Regel::StraengeWarten, Regel::AblaufAnnahme]
    );
}

// ------------------------------------------------------------ AC5

/// **Ein Satz, nicht nur Zahlen:** Diagnose, Beobachtung mit den Zahlen,
/// nächster Schritt.
#[test]
fn ac5_der_befund_ist_ein_satz_mit_naechstem_schritt() {
    let d = diagnose(&verzweigungsgebunden(), &schluessel(), Some(&grundlage()));
    let satz = d.satz();
    for teil in [
        "verzweigungsgebunden",
        "Befehlsdurchsatz",
        "2,65",
        "Fehlsprünge je Kampf",
        "155",
        "nächster Schritt: Kernel-Spezialisierung, profilgeführter Bau",
    ] {
        assert!(satz.contains(teil), "'{teil}' fehlt in: {satz}");
    }
}

/// Jede Zeile hat ihren Satz und ihren Schritt — Wort für Wort aus der
/// Tabelle des Issues.
#[test]
fn ac5_jede_zeile_traegt_diagnose_und_schritt_der_tabelle() {
    let tabelle = [
        (
            Regel::Verzweigungsgebunden,
            "verzweigungsgebunden",
            "Kernel-Spezialisierung, profilgeführter Bau",
        ),
        (
            Regel::ZweiteEbene,
            "Zustand zu groß für die zweite Ebene",
            "Suchzustand weiter verdichten",
        ),
        (
            Regel::StraengeWarten,
            "Pakete zu groß oder zu wenige",
            "Paketlänge senken",
        ),
        (Regel::NetzWarten, "Pakete zu klein", "Paketgröße erhöhen"),
        (
            Regel::WegLohntNicht,
            "dieser Weg lohnt nicht",
            "Weg abschalten",
        ),
        (
            Regel::AblaufAnnahme,
            "die Kernel-Annahme gilt nicht mehr",
            "neu messen, Endausbau erreicht",
        ),
        (
            Regel::SchlangeKnapp,
            "Kapazität",
            "vor dem Überlauf handeln",
        ),
    ];
    assert_eq!(tabelle.len(), Regel::TABELLE.len());
    for ((regel, diagnose, schritt), aus_tabelle) in tabelle.iter().zip(Regel::TABELLE) {
        assert_eq!(*regel, aus_tabelle, "Reihenfolge der Tabelle");
        assert!(
            regel.diagnose().contains(diagnose),
            "{regel:?}: {}",
            regel.diagnose()
        );
        assert!(
            regel.naechster_schritt().starts_with(schritt),
            "{regel:?}: {}",
            regel.naechster_schritt()
        );
    }
}

/// Mehrere Befunde stehen in der Reihenfolge der Tabelle, und der Satz nennt
/// die übrigen.
#[test]
fn ac5_mehrere_befunde_in_tabellenfolge() {
    let mut m = verzweigungsgebunden();
    m.schlange = mit_schlange(250, 0).schlange;
    m.straenge = mit_warteanteil(0.4).straenge;
    let d = diagnose(&m, &schluessel(), Some(&grundlage()));
    let regeln: Vec<Regel> = d.befunde.iter().map(|b| b.regel).collect();
    assert_eq!(
        regeln,
        [
            Regel::Verzweigungsgebunden,
            Regel::StraengeWarten,
            Regel::SchlangeKnapp
        ]
    );
    assert!(d.satz().ends_with("(+2 weitere)"), "{}", d.satz());
}

/// Unter vier Messsekunden prüft keine Zeile — und der Satz sagt das, statt
/// „keine Diagnose" zu behaupten.
#[test]
fn ein_zu_kurzes_fenster_prueft_nichts_und_sagt_es() {
    let mut m = verzweigungsgebunden();
    m.sekunden = 2.0;
    let d = diagnose(&m, &schluessel(), Some(&grundlage()));
    assert!(d.befunde.is_empty());
    assert!(d.geprueft.is_empty());
    assert!(d.satz().starts_with("noch keine Diagnose"), "{}", d.satz());
}
