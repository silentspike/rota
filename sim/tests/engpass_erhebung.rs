//! Die Erhebung der Engpasserkennung (Issue #934, AC1, AC3, AC4): Kernklassen
//! und Adapter, die Leistungszähler, die Grundlage als Datei, die Messwerte
//! über ein Fenster und was `balance` dafür mitzählt.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, run_vollstaendig, BalanceConfig};
use rota_sim::engpass::{
    prozess_cpu_ns, Adapter, Ereignis, Erhebung, Grundlage, Kernklasse, Leistungszaehler,
    Messwerte, Prozessor, Quelle, Regel, Rohstand, Rolle, Schluessel, Strangzeit, Zaehlerlage,
    Zaehlerstand, RUHENDE_KERNKLASSEN,
};
use rota_sim::fortschritt::{Momentaufnahme, Weg, Zaehlwerk};
use rota_sim::gear::WornValues;

fn gute_liste() -> Apl {
    parse(AplName("step1-good"), include_str!("../apl/step1-good.apl")).expect("gültige Liste")
}

fn cpuinfo(hersteller: &str, familie: u32, modell: u32, name: &str) -> String {
    format!(
        "processor\t: 0\nvendor_id\t: {hersteller}\ncpu family\t: {familie}\nmodel\t\t: {modell}\n\
         model name\t: {name}\nstepping\t: 0\n\nprocessor\t: 1\nvendor_id\t: GenuineIntel\n"
    )
}

// ------------------------------------------------------------ AC3

/// Die beiden Zielrechner, wie `/proc/cpuinfo` sie am 2026-10-28 meldete.
#[test]
fn ac3_laptop_ist_zen3_und_node3_ist_zen5() {
    let laptop = Prozessor::aus_cpuinfo(&cpuinfo(
        "AuthenticAMD",
        25,
        80,
        "AMD Ryzen 9 5900HS with Radeon Graphics",
    ))
    .expect("lesbar");
    assert_eq!(laptop.kernklasse(), Kernklasse::Zen3);
    assert_eq!(laptop.name, "AMD Ryzen 9 5900HS with Radeon Graphics");
    let node3 = Prozessor::aus_cpuinfo(&cpuinfo(
        "AuthenticAMD",
        26,
        68,
        "AMD Ryzen 7 9700X 8-Core Processor",
    ))
    .expect("lesbar");
    assert_eq!(node3.kernklasse(), Kernklasse::Zen5);
}

/// Zen 4 sieht aus wie Zen 3 (Familie 25) und bekommt trotzdem dessen
/// Kodierungen nicht: Modell 0x61 lädt in `perf` die Tabelle von Zen 4.
#[test]
fn ac3_zen4_erbt_die_kodierungen_von_zen3_nicht() {
    for modell in [0x10, 0x30, 0x61, 0x74, 0xa0] {
        let p =
            Prozessor::aus_cpuinfo(&cpuinfo("AuthenticAMD", 25, modell, "Zen 4")).expect("lesbar");
        assert_eq!(p.kernklasse(), Kernklasse::Unbekannt, "Modell {modell:#x}");
        assert!(p.kernklasse().adapter().is_err());
    }
    for modell in [0x00, 0x21, 0x44, 0x50] {
        let p =
            Prozessor::aus_cpuinfo(&cpuinfo("AuthenticAMD", 25, modell, "Zen 3")).expect("lesbar");
        assert_eq!(p.kernklasse(), Kernklasse::Zen3, "Modell {modell:#x}");
    }
}

/// Die ruhenden Klassen werden erkannt und sagen, warum sie keinen Adapter
/// haben — sie benutzen nicht die Kodierungen einer anderen.
#[test]
fn ac3_ruhende_kernklassen_nennen_ihren_grund() {
    for (modell, klasse) in [
        (0x2a, Kernklasse::SandyBridge),
        (0x9a, Kernklasse::AlderLake),
    ] {
        let p =
            Prozessor::aus_cpuinfo(&cpuinfo("GenuineIntel", 6, modell, "Intel")).expect("lesbar");
        assert_eq!(p.kernklasse(), klasse);
        assert_eq!(p.kernklasse().adapter().err(), Some(RUHENDE_KERNKLASSEN));
    }
    assert!(RUHENDE_KERNKLASSEN.contains("Gracemont E"));
    assert!(
        Prozessor::aus_cpuinfo("vendor_id\t: AuthenticAMD\n").is_none(),
        "unvollständig ist nicht geraten"
    );
}

/// **Derselbe Zähler bedeutet nicht überall dasselbe.** Die Masken der drei
/// Ebenen und das Ereignis der Speicher-Wartezeit unterscheiden sich
/// zwischen den beiden Adaptern.
#[test]
fn ac3_die_adapter_deuten_dieselbe_rolle_verschieden() {
    let zen3 = Kernklasse::Zen3.adapter().expect("Adapter");
    let zen5 = Kernklasse::Zen5.adapter().expect("Adapter");
    let quelle = |a: &Adapter, rolle: Rolle| -> (Quelle, u64) {
        let (g, p) = a.platz(rolle).expect("Rolle im Adapter");
        let e: &Ereignis = &a.gruppen[g][p];
        (e.quelle, e.faktor)
    };
    for rolle in [
        Rolle::FehlgriffeL1,
        Rolle::FehlgriffeL2,
        Rolle::FehlgriffeL3,
        Rolle::Speicherwartezeit,
    ] {
        assert_ne!(quelle(zen3, rolle), quelle(zen5, rolle), "{rolle:?}");
    }
    // Zen 3 zählt die Wartezeit geviertelt, Zen 5 nicht.
    assert_eq!(quelle(zen3, Rolle::Speicherwartezeit).1, 4);
    assert_eq!(quelle(zen5, Rolle::Speicherwartezeit).1, 1);
    // Jede Gruppe beginnt mit ihren eigenen Zyklen, und jede Rolle steht da.
    for a in [zen3, zen5] {
        for gruppe in a.gruppen {
            assert_eq!(gruppe[0].rolle, Rolle::Zyklen);
            assert!(
                gruppe.len() <= 5,
                "mehr als fünf Zähler je Gruppe passen nicht"
            );
        }
        for rolle in [
            Rolle::Zyklen,
            Rolle::Befehle,
            Rolle::Fehlspruenge,
            Rolle::FehlgriffeL1,
            Rolle::FehlgriffeL2,
            Rolle::FehlgriffeL3,
            Rolle::Speicherwartezeit,
        ] {
            assert!(a.platz(rolle).is_some(), "{:?}: {rolle:?}", a.kernklasse);
        }
    }
}

/// Die Bitlage der rohen Kodierung: `event` auf `config:0-7,32-35`,
/// `umask` auf `config:8-15` (abgelesen in `/sys/bus/event_source/devices/cpu/format`).
#[test]
fn ac3_rohe_kodierung_legt_die_oberen_bits_der_ereignisnummer_ueber_32() {
    assert_eq!(
        Quelle::Roh {
            ereignis: 0x43,
            maske: 0x5e
        }
        .typ_und_config(),
        (4, 0x5e43)
    );
    assert_eq!(
        Quelle::Roh {
            ereignis: 0x1a0,
            maske: 0x1e
        }
        .typ_und_config(),
        (4, 0x1ea0 | (1 << 32))
    );
    assert_eq!(Quelle::FEHLSPRUENGE.typ_und_config(), (0, 5));
    for k in [
        Kernklasse::Zen3,
        Kernklasse::Zen5,
        Kernklasse::SandyBridge,
        Kernklasse::AlderLake,
        Kernklasse::Unbekannt,
    ] {
        assert_eq!(Kernklasse::aus_kennung(k.kennung()), Some(k));
    }
}

// ------------------------------------------------------------ AC4

/// Ein Adapter aus allgemeinen Ereignissen — läuft auf jedem x86-Rechner, der
/// Zähler erlaubt, auch auf dem Bau-Server.
static ALLGEMEIN: Adapter = Adapter {
    kernklasse: Kernklasse::Unbekannt,
    gruppen: [
        &[
            Ereignis {
                rolle: Rolle::Zyklen,
                quelle: Quelle::ZYKLEN,
                faktor: 1,
                name: "cycles",
            },
            Ereignis {
                rolle: Rolle::Befehle,
                quelle: Quelle::BEFEHLE,
                faktor: 1,
                name: "instructions",
            },
        ],
        &[Ereignis {
            rolle: Rolle::Zyklen,
            quelle: Quelle::ZYKLEN,
            faktor: 1,
            name: "cycles",
        }],
    ],
    hinweis: "Prüffall",
};

fn arbeit(n: u64) -> u64 {
    (0..n).fold(0u64, |a, i| {
        std::hint::black_box(a.wrapping_mul(31).wrapping_add(i))
    })
}

/// **Die Fähigkeit wird gemessen, nicht angenommen.** Auf einem Rechner, der
/// Zähler erlaubt, zählen sie — auch die Befehle eines Strangs, der NACH dem
/// Öffnen gestartet wurde (`inherit`). Wo sie gesperrt sind, liefert das
/// Öffnen eine Sperre mit dem Fehler des Systemaufrufs, statt abzustürzen.
#[test]
fn ac4_zaehler_zaehlen_erben_oder_melden_ihre_sperre() {
    match Leistungszaehler::oeffnen(&ALLGEMEIN) {
        Ok(z) => {
            let vorher = z.lesen().expect("lesbar");
            // 50 Millionen Schleifendurchläufe in einem neuen Strang: Jeder
            // kostet mehrere Befehle. Der eigene Strang wartet nur.
            std::thread::spawn(|| arbeit(50_000_000))
                .join()
                .expect("Strang");
            let nachher = z.lesen().expect("lesbar");
            let befehle = nachher.gruppen[0].werte[1] - vorher.gruppen[0].werte[1];
            assert!(
                befehle >= 50_000_000,
                "nur {befehle} Befehle — der gestartete Strang wurde nicht mitgezählt"
            );
            assert!(nachher.gruppen[0].eingeschaltet_ns > vorher.gruppen[0].eingeschaltet_ns);
            eprintln!("KONTROLLE: Zähler verfügbar, {befehle} Befehle aus dem Erben gezählt");
        }
        Err(sperre) => {
            assert!(sperre.errno.is_some(), "{sperre:?}");
            assert!(sperre.satz().contains("errno"), "{}", sperre.satz());
            eprintln!("KONTROLLE: Zähler gesperrt — {}", sperre.satz());
        }
    }
}

/// Die Zeitmessung des Rückfalls braucht keine Erlaubnis und zählt die
/// Arbeit aller Stränge.
#[test]
fn ac4_die_prozesszeit_waechst_mit_der_arbeit_eines_strangs() {
    let vorher = prozess_cpu_ns().expect("auf Linux immer lesbar");
    std::thread::spawn(|| arbeit(20_000_000))
        .join()
        .expect("Strang");
    let nachher = prozess_cpu_ns().expect("lesbar");
    assert!(nachher > vorher, "{vorher} -> {nachher}");
}

/// Ohne Zähler läuft die Erhebung weiter: Sie nennt den Grund im Bild und
/// prüft die übrigen Zeilen.
#[test]
fn ac4_eine_erhebung_ohne_zaehler_faellt_nicht_aus() {
    let schluessel = Schluessel {
        rechner: "node3".to_owned(),
        kernklasse: Kernklasse::Zen5,
        lastprofil: "bench".to_owned(),
    };
    let e = Erhebung::ohne_zaehler(
        schluessel,
        None,
        "perf_event_open gesperrt (errno 13 EACCES)",
    );
    assert!(e.zaehlerlage().is_err());
    let (alt, neu) = proben(10.0, |r| {
        r.straenge = vec![
            Strangzeit {
                arbeit_ns: 5_000,
                warten_ns: 5_000
            };
            2
        ];
    });
    let bild = e.bild(&alt, &neu);
    assert!(matches!(&bild.messwerte.zaehler, Zaehlerlage::Fehlt(g) if g.contains("EACCES")));
    assert_eq!(bild.diagnose.befunde[0].regel, Regel::StraengeWarten);
    let zeilen = bild.zeilen().join("\n");
    assert!(zeilen.contains("ohne Zähler"), "{zeilen}");
    assert!(zeilen.contains("Zeitmessung und Druckwerte"), "{zeilen}");
    let json: serde_json::Value = serde_json::from_str(&bild.json()).expect("JSON");
    assert_eq!(json["counters"]["available"], false);
    assert_eq!(json["counters"]["fallback"], "cpu_time_and_psi");
    assert_eq!(json["findings"][0]["rule"], "threads_waiting");
}

// ------------------------------------------------------------ Grundlage

fn eine_grundlage() -> Grundlage {
    Grundlage {
        schluessel: Schluessel {
            rechner: "laptop".to_owned(),
            kernklasse: Kernklasse::Zen3,
            lastprofil:
                "bench boss=boss-1-1 apl=step1-good#0123abcd fights=400000 threads=4 bau=release"
                    .to_owned(),
        },
        werte: rota_sim::engpass::Vergleichswerte {
            ipc: Some(2.651_234_5),
            zyklen_je_kampf: Some(59_340.25),
            fehlspruenge_je_kampf: Some(155.125),
            l2_je_kampf: Some(0.1 + 0.2),
            l3_je_kampf: None,
            speicherwartezyklen_je_kampf: Some(1e-9),
            cpu_ns_je_kampf: Some(18_234.5),
        },
        kaempfe: 400_000,
        sekunden: 12.345,
    }
}

/// Bit für Bit zurück — auch 0,1 + 0,2 und ein fehlender Wert.
#[test]
fn die_grundlage_kommt_bitgleich_zurueck() {
    let g = eine_grundlage();
    let text = g.speichern();
    assert!(text.starts_with("rota-engpass-grundlage 1\n"), "{text}");
    assert_eq!(Grundlage::lesen(&text), Ok(g));
}

/// **Eine kaputte Zeile macht die ganze Grundlage ungültig.**
#[test]
fn eine_kaputte_grundlage_wird_abgelehnt_statt_teilweise_gelesen() {
    let text = eine_grundlage().speichern();
    let ohne_kopf = text.replacen("rota-engpass-grundlage 1", "rota-engpass-grundlage 2", 1);
    assert!(Grundlage::lesen(&ohne_kopf).is_err());
    let kaputte_zahl = text.replacen("wert ipc ", "wert ipc zz", 1);
    assert!(Grundlage::lesen(&kaputte_zahl).is_err());
    let fremder_name = text.replacen("wert ipc ", "wert ipcc ", 1);
    assert!(Grundlage::lesen(&fremder_name).is_err());
    let ohne_profil: String = text
        .lines()
        .filter(|z| !z.starts_with("lastprofil"))
        .map(|z| format!("{z}\n"))
        .collect();
    assert_eq!(
        Grundlage::lesen(&ohne_profil).map_err(|e| e.to_string()),
        Err("die Angabe 'lastprofil' fehlt".to_owned())
    );
}

// ------------------------------------------------------------ AC1

/// Zwei Proben im Abstand `sekunden`; `neu` verändert den Rohstand der
/// zweiten.
fn proben(sekunden: f64, neu: impl FnOnce(&mut Rohstand)) -> (Momentaufnahme, Momentaufnahme) {
    let werk = Zaehlwerk::neu(1);
    let alt = werk.momentaufnahme();
    let mut zweite = alt.clone();
    zweite.elapsed_ns = alt.elapsed_ns + (sekunden * 1e9) as u64;
    zweite.kaempfe = alt.kaempfe + 1000;
    neu(&mut zweite.engpass);
    (alt, zweite)
}

fn gruppe(an: u64, gezaehlt: u64, werte: &[u64]) -> rota_sim::engpass::Gruppenstand {
    rota_sim::engpass::Gruppenstand {
        eingeschaltet_ns: an,
        gezaehlt_ns: gezaehlt,
        werte: werte.to_vec(),
    }
}

/// Je Kampf, je Gruppe hochgerechnet wie `perf stat`, Anteile nur aus einer
/// Gruppe.
#[test]
fn ac1_die_zaehlerwerte_rechnen_je_kampf_und_je_gruppe() {
    let adapter = Kernklasse::Zen3.adapter().expect("Adapter");
    let mut alt = proben(10.0, |_| {}).0;
    alt.engpass.leistung = Some(Zaehlerstand {
        gruppen: vec![gruppe(0, 0, &[0, 0, 0, 0]), gruppe(0, 0, &[0, 0, 0, 0])],
    });
    let mut neu = alt.clone();
    neu.elapsed_ns += 10_000_000_000;
    neu.kaempfe += 1000;
    // Gruppe 1 lief die halbe Zeit: ihre Werte zählen doppelt. Gruppe 2 die
    // ganze.
    neu.engpass.leistung = Some(Zaehlerstand {
        gruppen: vec![
            gruppe(100, 50, &[30_000_000, 80_000_000, 75_000, 450_000]),
            gruppe(100, 100, &[60_000_000, 40_000, 4_000, 6_000_000]),
        ],
    });
    let m = Messwerte::aus_fenster(&alt, &neu, Ok(adapter));
    let Zaehlerlage::Gemessen(z) = m.zaehler else {
        panic!("gemessen erwartet: {:?}", m.zaehler)
    };
    let nah = |a: Option<f64>, b: f64| (a.expect("Wert") - b).abs() < 1e-9 * b.abs().max(1.0);
    assert!(nah(z.ipc, 80.0 / 30.0), "{:?}", z.ipc);
    assert!(nah(z.zyklen_je_kampf, 60_000.0), "{:?}", z.zyklen_je_kampf);
    assert!(nah(z.fehlspruenge_je_kampf, 150.0));
    assert!(nah(z.l1_je_kampf, 900.0));
    assert!(nah(z.l2_je_kampf, 40.0));
    assert!(nah(z.l3_je_kampf, 4.0));
    assert!(nah(z.speicherwartezyklen_je_kampf, 6_000.0));
    assert!(
        nah(z.speicher_wartezeit_anteil, 0.1),
        "nur durch Zyklen derselben Gruppe"
    );
    assert_eq!(z.abdeckung, [Some(0.5), Some(1.0)]);
}

/// Eine Gruppe, die im Fenster nie gezählt wurde, liefert **keine** Werte —
/// keine Null.
#[test]
fn ac1_eine_nie_geplante_gruppe_liefert_nichts_statt_null() {
    let adapter = Kernklasse::Zen5.adapter().expect("Adapter");
    let (mut alt, mut neu) = proben(10.0, |_| {});
    alt.engpass.leistung = Some(Zaehlerstand {
        gruppen: vec![gruppe(0, 0, &[0; 4]), gruppe(0, 0, &[0; 4])],
    });
    neu.engpass.leistung = Some(Zaehlerstand {
        gruppen: vec![
            gruppe(100, 100, &[1000, 2000, 3, 4]),
            gruppe(100, 0, &[0; 4]),
        ],
    });
    let Zaehlerlage::Gemessen(z) = Messwerte::aus_fenster(&alt, &neu, Ok(adapter)).zaehler else {
        panic!("gemessen erwartet")
    };
    assert!(z.ipc.is_some());
    assert_eq!(z.l2_je_kampf, None);
    assert_eq!(z.speicherwartezyklen_je_kampf, None);
    assert_eq!(z.abdeckung[1], Some(0.0));
}

/// Was fehlt, sagt warum: kein Adapter, kein Stand, kein Netz.
#[test]
fn ac1_fehlende_groessen_tragen_ihren_grund() {
    let (alt, neu) = proben(10.0, |_| {});
    let m = Messwerte::aus_fenster(&alt, &neu, Err("unbekannte Kernklasse"));
    assert_eq!(
        m.zaehler,
        Zaehlerlage::Fehlt("unbekannte Kernklasse".to_owned())
    );
    assert_eq!(m.netz, None, "es gibt kein Netz im Lauf");
    let adapter = Kernklasse::Zen3.adapter().expect("Adapter");
    let m = Messwerte::aus_fenster(&alt, &neu, Ok(adapter));
    assert!(
        matches!(m.zaehler, Zaehlerlage::Fehlt(_)),
        "kein Stand im Fenster"
    );
    if !rota_sim::diagnostics::im_bau() {
        assert_eq!(m.schlange, None);
        assert_eq!(m.ablauf, None);
    }
}

fn konfiguration(iterations: u32, threads: usize, purity: u32) -> BalanceConfig {
    let mut worn = WornValues::none();
    worn.purity = purity;
    BalanceConfig {
        iterations,
        threads,
        boss: rota_sim::data::boss_by_id("boss-1-1").expect("Boss"),
        worn,
        ..BalanceConfig::default()
    }
}

/// **Zu wenige Pakete:** zwei Durchgänge auf vier Strängen. Zwei Stränge
/// rechnen, zwei warten den ganzen Abschnitt — gezählt vom Lauf selbst, nicht
/// gebaut.
#[test]
fn ac1_zu_wenige_pakete_lassen_straenge_den_ganzen_abschnitt_warten() {
    let werk = Zaehlwerk::neu(4);
    let anmeldung = werk.anmelden();
    let _ = run_vollstaendig(&konfiguration(2, 4, 0), &gute_liste());
    drop(anmeldung);
    let s = werk.momentaufnahme().engpass.straenge;
    assert_eq!(s.len(), 4, "ein Eintrag je Arbeitsplatz");
    assert!(s[0].arbeit_ns > 0 && s[1].arbeit_ns > 0, "{s:?}");
    assert_eq!((s[2].arbeit_ns, s[3].arbeit_ns), (0, 0), "{s:?}");
    assert!(
        s[2].warten_ns >= s[0].arbeit_ns.min(s[1].arbeit_ns),
        "{s:?}"
    );
    let arbeit: u64 = s.iter().map(|x| x.arbeit_ns).sum();
    let warten: u64 = s.iter().map(|x| x.warten_ns).sum();
    assert!(
        warten as f64 / (arbeit + warten) as f64 > 0.4,
        "zwei von vier Strängen ohne Paket: {s:?}"
    );
}

/// Ein Strang rechnet allein (ein Durchgang, kein paralleler Abschnitt):
/// nichts wird eingetragen, und die Zeile ist ungeprüft statt grün.
#[test]
fn ac1_ohne_parallelen_abschnitt_gibt_es_keine_strangzeit() {
    let werk = Zaehlwerk::neu(4);
    let anmeldung = werk.anmelden();
    let _ = run_vollstaendig(&konfiguration(1, 4, 0), &gute_liste());
    drop(anmeldung);
    let s = werk.momentaufnahme().engpass.straenge;
    assert!(s.iter().all(|x| *x == Strangzeit::default()), "{s:?}");
}

/// Die Trefferquote des Wegs „ein Kampf": versucht von `run`, nicht von
/// `run_vollstaendig`; getroffen ohne Würfel, verfehlt mit.
#[test]
fn ac1_die_trefferquote_der_reduktion_zaehlt_versuche_und_treffer() {
    let werk = Zaehlwerk::neu(1);
    let anmeldung = werk.anmelden();
    let alt = werk.momentaufnahme();
    let _ = run(&konfiguration(20, 1, 0), &gute_liste());
    let _ = run(&konfiguration(20, 1, 500), &gute_liste());
    let _ = run_vollstaendig(&konfiguration(20, 1, 0), &gute_liste());
    let neu = werk.momentaufnahme();
    drop(anmeldung);
    let m = Messwerte::aus_fenster(&alt, &neu, Err("Prüffall"));
    assert_eq!(m.wege.len(), 1);
    assert_eq!(
        (m.wege[0].versuche, m.wege[0].treffer),
        (2, 1),
        "{:?}",
        m.wege[0]
    );
    assert_eq!(m.wege[0].quote(), Some(0.5));
    assert_eq!(neu.kandidaten[Weg::EinKampf.index()], 1);
}

/// **Die Kampfzähler eines mehrsträngigen Laufs kommen beim Leitstrang an.**
/// Vor #934 zählte nur, wer selbst rechnete — und das ist bei vier Strängen
/// nicht der Leitstrang.
#[cfg(feature = "diagnostics")]
#[test]
fn ac1_die_kampfzaehler_der_arbeitsstraenge_werden_vereinigt() {
    use rota_sim::diagnostics;
    diagnostics::reset();
    diagnostics::enable();
    let werk = Zaehlwerk::neu(4);
    let anmeldung = werk.anmelden();
    let _ = run_vollstaendig(&konfiguration(40, 4, 0), &gute_liste());
    drop(anmeldung);
    let z = diagnostics::snapshot();
    diagnostics::disable();
    diagnostics::reset();
    assert_eq!(z.fights(), 40, "alle Kämpfe aller Stränge");
    assert!(z.samples() > 0 && z.expire_calls() > 0);
    let veroeffentlicht = werk.momentaufnahme().engpass.kampf.expect("veröffentlicht");
    assert_eq!(veroeffentlicht.schritte, z.samples());
    assert_eq!(veroeffentlicht.ablauf_aufrufe, z.expire_calls());
    assert_eq!(veroeffentlicht.schlange_max, z.max());
}

/// **Die Erhebung liest nur.** Derselbe Lauf mit und ohne angemeldetes
/// Zählwerk und eingeschaltete Kampfzähler liefert bitgleich dasselbe.
#[test]
fn die_erhebung_aendert_keine_kampfzahl() {
    let config = konfiguration(40, 4, 500);
    let ohne = run(&config, &gute_liste());
    rota_sim::diagnostics::enable();
    let werk = Zaehlwerk::neu(4);
    let anmeldung = werk.anmelden();
    let mit = run(&config, &gute_liste());
    drop(anmeldung);
    rota_sim::diagnostics::disable();
    rota_sim::diagnostics::reset();
    assert_eq!(ohne.to_json(), mit.to_json());
    assert!(werk
        .momentaufnahme()
        .engpass
        .straenge
        .iter()
        .any(|s| s.arbeit_ns > 0));
}
