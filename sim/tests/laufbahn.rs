//! Die Fortschritts-Simulation (Issue #922, E11.4e, Fassung A).
//!
//! | Fall | belegt |
//! | --- | --- |
//! | `fehlender_fund_folgt_der_geometrischen_reihe` | der lange Schwanz bei 10 % Fallrate, Toleranz aus n |
//! | `aufwertung_haelt_an_der_inhaltsstufe` | die Obergrenze aus 13.6 an `upgrade_item` |
//! | `aufwertung_bleibt_unter_der_inhaltsstufe` | dieselbe Obergrenze ueber ganze Laufbahnen |
//! | `gleicher_startwert_gleiche_ausgabe_bei_jeder_strangzahl` | Ausgabe bitgleich bei 1, 3 und 4 Straengen |
//! | `kontrollprobe_zelle_gegen_direkte_simulation` | Zelle im Speicher = direkt gerechneter Kampf |
//! | `fallrate_macht_den_fortschritt_ungueltig_nicht_die_kampftabelle` | AC5, als Lauf und als Regel |
//! | `das_orakel_lehnt_stufen_ab_die_der_kampf_nicht_kennt` | Fassung A rechnet nur normal / Leiter 0 |

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance;
use rota_sim::data::boss_by_id;
use rota_sim::laufbahn::auswertung::{
    bekannte_rate, bericht, fehlt_nach, rechne, rechne_mit, Auftrag, TOLERANZ_SIGMA,
};
use rota_sim::laufbahn::orakel::{
    laufkonfiguration, Kampfeinstellung, Kampforakel, OrakelFehler, OrakelSchluessel,
    Referenzspielweise, SpeicherOrakel, Zelle,
};
use rota_sim::laufbahn::regeln::{upgrade_cap_of, upgrade_item, KATALOG, MATERIAL_PER_LEVEL};
use rota_sim::laufbahn::schleife::{
    laufbahn, Anlegen, Aufwerten, Freischaltung, Fundziel, Laufbahnregeln, Messauftrag,
};
use rota_sim::laufbahn::{invalidierung, kampffingerabdruck, Fortschrittsfingerabdruck};
use rota_sim::speicher::ErgebnisSchluessel;

const REFERENZ: &str = include_str!("../apl/laufbahn-referenz.apl");

fn referenz() -> Referenzspielweise {
    Referenzspielweise::fuer_alle(
        parse(AplName("laufbahn-referenz"), REFERENZ).expect("Referenzliste parst nicht"),
    )
}

fn vorlage(id: &str) -> usize {
    KATALOG
        .iter()
        .position(|v| v.id == id)
        .unwrap_or_else(|| panic!("keine Vorlage {id}"))
}

/// Ein Orakel, bei dem jeder Kampf gewonnen wird, mit 15 s Reserve.
///
/// **Wozu**: Der fehlende Fund nach k Siegen haengt an den Siegen, nicht
/// daran, wie sie zustande kamen — der Beutewurf faellt erst nach dem Sieg und
/// zieht aus einem eigenen Strom. Dieser Fall prueft die Beute, also trennt er
/// sie vom Kampf; dass die Schleife den echten Kampf richtig fragt, pruefen
/// die Faelle mit [`SpeicherOrakel`] weiter unten.
struct ImmerSieg {
    abgefragt: u64,
}

impl Kampforakel for ImmerSieg {
    fn zelle(&mut self, _: &OrakelSchluessel) -> Result<Zelle, OrakelFehler> {
        self.abgefragt += 1;
        Ok(Zelle {
            siege: 1,
            durchgaenge: 1,
            siegzeit_ms: 45_000.0,
            schaden: 0.0,
            tode: 0,
        })
    }
    fn gerechnet(&self) -> u64 {
        0
    }
    fn abgefragt(&self) -> u64 {
        self.abgefragt
    }
}

fn immer_sieg() -> ImmerSieg {
    ImmerSieg { abgefragt: 0 }
}

/// Der Masterplan (Beute-Tabelle, „der lange Schwanz"): bei 10 % Fallrate
/// haben nach 10/20/30 Siegen noch 35 % / 12 % / 3,5 % den Gegenstand nicht.
/// Gerechnet ist es `0,9^k`: 34,87 %, 12,16 %, **4,24 %** — die dritte Zahl im
/// Masterplan liegt daneben, und dieser Fall misst gegen die Rechnung.
fn fund_lauf(
    rate: u32,
    n: u32,
) -> (
    Laufbahnregeln,
    Messauftrag,
    Vec<rota_sim::laufbahn::schleife::Laufbahn>,
) {
    let fokus = vorlage("focus-mix");
    let regeln = Laufbahnregeln {
        fallrate_promille: rate,
        katalog: vec![fokus],
        ..Laufbahnregeln::default()
    };
    let messung = Messauftrag {
        siege: 30,
        max_kaempfe: 30,
        fund: Fundziel {
            vorlage: fokus,
            mindest_guete: 0,
        },
    };
    let lauf = rechne_mit(n, 7, 4, std::slice::from_ref(&regeln), &messung, immer_sieg)
        .expect("ImmerSieg lehnt nichts ab");
    let laufbahnen = lauf.je_regelsatz.into_iter().next().expect("ein Regelsatz");
    (regeln, messung, laufbahnen)
}

/// Wie weit der gemessene Anteil vom erwarteten weg ist, in Toleranzen.
fn abweichung(
    laufbahnen: &[rota_sim::laufbahn::schleife::Laufbahn],
    p: f64,
    k: u32,
) -> (f64, f64, f64, usize) {
    let a = fehlt_nach(laufbahnen, k);
    let erwartet = (1.0 - p).powi(k as i32);
    let tol = a.toleranz_um(erwartet);
    (a.wert(), erwartet, tol, a.n)
}

#[test]
fn fehlender_fund_folgt_der_geometrischen_reihe() {
    const N: u32 = 10_000;
    let (regeln, messung, laufbahnen) = fund_lauf(100, N);
    let p = bekannte_rate(&regeln, &messung).expect("ein Eintrag, ab common: Rate bekannt");
    assert_eq!(p, 0.1, "Rate je Sieg");
    let masterplan = [(10, 0.35), (20, 0.12), (30, 0.035)];
    for (k, laut_masterplan) in masterplan {
        let (gemessen, erwartet, tol, n) = abweichung(&laufbahnen, p, k);
        println!(
            "nach {k:>2} Siegen: gemessen {:.4}  erwartet 0,9^{k} = {:.4}  Toleranz +-{:.4} \
             ({TOLERANZ_SIGMA} sigma, n = {n})  Masterplan {laut_masterplan}",
            gemessen, erwartet, tol
        );
        assert_eq!(n, N as usize, "jede Laufbahn hat {k} Siege");
        assert!(
            (gemessen - erwartet).abs() <= tol,
            "nach {k} Siegen fehlt der Fund bei {gemessen:.4}, erwartet {erwartet:.4} +- {tol:.4}"
        );
    }

    // **Die Gegenprobe im selben Fall**: Bei 20 % Fallrate muss dieselbe
    // Pruefung gegen 0,9^k scheitern. Sonst waere sie eine, die nicht rot
    // werden kann.
    let (_, _, anders) = fund_lauf(200, N);
    let (gemessen, erwartet, tol, _) = abweichung(&anders, 0.1, 10);
    println!("Gegenprobe 20 %: gemessen {gemessen:.4} gegen erwartet {erwartet:.4} +- {tol:.4}");
    assert!(
        (gemessen - erwartet).abs() > tol,
        "die Pruefung unterscheidet 10 % nicht von 20 %"
    );
}

#[test]
fn aufwertung_haelt_an_der_inhaltsstufe() {
    // Die Regel an der Funktion: beliebig viel Material, und die Stufe bleibt
    // bei der des Inhalts stehen.
    let grenze = i64::from(upgrade_cap_of(3, 0, 0));
    let r = upgrade_item(1, 1_000_000_000, 3, 0, 0);
    assert_eq!(r.level, grenze, "Aufwertung ueber die Inhaltsstufe hinaus");
    assert!(r.capped_by_content);
    assert_eq!(
        upgrade_item(grenze, 10_000, 3, 0, 0).spent,
        0,
        "an der Grenze kostet es nichts"
    );
}

#[test]
fn aufwertung_bleibt_unter_der_inhaltsstufe() {
    // Die Regel ueber ganze Laufbahnen: Jeder Sieg bringt Material, und das
    // Material laeuft der Obergrenze davon. Getrennt vom Fall an der Funktion,
    // damit sichtbar bleibt, ob die SCHLEIFE die Grenze haelt — sie steht dort
    // nicht ein zweites Mal (`schleife.rs`, `aufwerten`).
    let regeln = Laufbahnregeln::default();
    let messung = Messauftrag {
        siege: 150,
        max_kaempfe: 150,
        fund: Fundziel {
            vorlage: vorlage("focus-mix"),
            mindest_guete: 0,
        },
    };
    let lauf = rechne_mit(
        300,
        11,
        4,
        std::slice::from_ref(&regeln),
        &messung,
        immer_sieg,
    )
    .expect("ImmerSieg lehnt nichts ab");
    let laufbahnen = &lauf.je_regelsatz[0];
    let ueber = laufbahnen
        .iter()
        .map(|l| l.ueber_deckel)
        .max()
        .expect("Laufbahnen");
    // **Die positive Kontrolle**: Material lag da und blieb liegen. Ohne sie
    // waere `ueber <= 0` auch dann gruen, wenn nie genug Material zum
    // Aufwerten zusammenkam — und die Grenze nie gebraucht wurde.
    let zurueckgehalten = laufbahnen
        .iter()
        .filter(|l| l.material_ungenutzt >= MATERIAL_PER_LEVEL)
        .count();
    println!(
        "groesster Abstand ueber der Inhaltsstufe: {ueber}; {zurueckgehalten} von {} Laufbahnen \
         behielten Material, das die Grenze nicht ausgeben liess",
        laufbahnen.len()
    );
    assert!(
        ueber <= 0,
        "eine getragene Stufe lag {ueber} ueber der Inhaltsstufe"
    );
    assert_eq!(
        ueber, 0,
        "kein Stueck hat die Inhaltsstufe je erreicht — die Grenze wurde nie gebraucht"
    );
    assert!(
        zurueckgehalten > 100,
        "nur {zurueckgehalten} Laufbahnen mit zurueckgehaltenem Material"
    );
}

fn kleiner_auftrag(straenge: usize, startwert: u32) -> Auftrag {
    Auftrag {
        laufbahnen: 10,
        startwert,
        straenge,
        regeln: Laufbahnregeln::default(),
        messung: Messauftrag {
            siege: 6,
            max_kaempfe: 40,
            fund: Fundziel {
                vorlage: vorlage("focus-mix"),
                mindest_guete: 0,
            },
        },
        kampf: Kampfeinstellung::default(),
    }
}

#[test]
fn gleicher_startwert_gleiche_ausgabe_bei_jeder_strangzahl() {
    let referenz = referenz();
    let ausgabe = |straenge: usize, startwert: u32| {
        let auftrag = kleiner_auftrag(straenge, startwert);
        let lauf = rechne(&auftrag, std::slice::from_ref(&auftrag.regeln), &referenz)
            .expect("Orakel rechnet");
        let text = bericht(
            &auftrag,
            &auftrag.regeln,
            &lauf.je_regelsatz[0],
            "laufbahn-referenz",
            kampffingerabdruck(&referenz, auftrag.kampf),
        )
        .text;
        (lauf, text)
    };
    let (eins, text_eins) = ausgabe(1, 3);
    let siege: usize = eins.je_regelsatz[0].iter().map(|l| l.siege.len()).sum();
    println!(
        "{} Laufbahnen, {siege} Siege, {} Kaempfe gerechnet bei {} Abfragen",
        eins.je_regelsatz[0].len(),
        eins.gerechnet,
        eins.abgefragt
    );
    assert!(
        siege > 0,
        "kein einziger Sieg — dann haengt nichts an der Beute, und der Vergleich unten waere leer"
    );
    for straenge in [3, 4] {
        let (lauf, text) = ausgabe(straenge, 3);
        assert_eq!(lauf, eins, "Laufbahnen bei {straenge} Straengen");
        assert_eq!(text, text_eins, "Bericht bei {straenge} Straengen");
    }
    // Gegenprobe: ein anderer Startwert gibt eine andere Ausgabe. Sonst
    // bewiese die Gleichheit oben nur, dass der Startwert nichts bewegt.
    let (anders, text_anders) = ausgabe(3, 4);
    assert_ne!(anders, eins, "Startwert 4 gibt dieselben Laufbahnen wie 3");
    assert_ne!(text_anders, text_eins);
}

/// Ein Orakel, das jede Frage mitschreibt und an das echte weitergibt.
struct Mitschrift<'a> {
    innen: SpeicherOrakel<'a>,
    fragen: Vec<(OrakelSchluessel, Zelle)>,
}

impl Kampforakel for Mitschrift<'_> {
    fn zelle(&mut self, s: &OrakelSchluessel) -> Result<Zelle, OrakelFehler> {
        let z = self.innen.zelle(s)?;
        self.fragen.push((*s, z));
        Ok(z)
    }
    fn gerechnet(&self) -> u64 {
        self.innen.gerechnet()
    }
    fn abgefragt(&self) -> u64 {
        self.innen.abgefragt()
    }
}

fn direkt(apl: &Apl, s: &OrakelSchluessel, einstellung: Kampfeinstellung) -> Zelle {
    let boss = boss_by_id(s.boss).expect("Boss");
    let config = laufkonfiguration(boss, s.ausstattung, einstellung);
    let vektor = balance::run(&config, apl)
        .ergebnisentwurf()
        .abschliessen()
        .expect("vollstaendig");
    Zelle::aus(&vektor)
}

#[test]
fn kontrollprobe_zelle_gegen_direkte_simulation() {
    let referenz = referenz();
    let einstellung = Kampfeinstellung::default();
    let mut orakel = Mitschrift {
        innen: SpeicherOrakel::neu(&referenz, einstellung),
        fragen: Vec::new(),
    };
    let auftrag = kleiner_auftrag(1, 5);
    let ergebnis = laufbahn(0, 5, &auftrag.regeln, &auftrag.messung, &mut orakel).expect("rechnet");
    assert!(
        !orakel.fragen.is_empty(),
        "keine einzige Frage an das Orakel"
    );
    // Jede Zelle, die die Schleife bekam, direkt nachgerechnet — Feld fuer
    // Feld und bis aufs Bit. Weichen sie ab, ist die Tabelle falsch gefuellt.
    let mut verschiedene = 0;
    for (index, (schluessel, zelle)) in orakel.fragen.iter().enumerate() {
        let nach = direkt(
            referenz.liste_fuer(schluessel.boss),
            schluessel,
            einstellung,
        );
        assert_eq!(nach.siege, zelle.siege, "Frage {index}: Siege");
        assert_eq!(
            nach.durchgaenge, zelle.durchgaenge,
            "Frage {index}: Durchgaenge"
        );
        assert_eq!(nach.tode, zelle.tode, "Frage {index}: Tode");
        assert_eq!(
            nach.siegzeit_ms.to_bits(),
            zelle.siegzeit_ms.to_bits(),
            "Frage {index}: Siegzeit"
        );
        assert_eq!(
            nach.schaden.to_bits(),
            zelle.schaden.to_bits(),
            "Frage {index}: Schaden"
        );
        let ablage = orakel
            .innen
            .speicher_ansehen()
            .hole(
                &orakel
                    .innen
                    .ergebnisschluessel(schluessel)
                    .expect("Schluessel"),
            )
            .expect("die Zelle liegt im Speicher");
        assert_eq!(Zelle::aus(ablage), *zelle, "Frage {index}: Ablage");
        if index == 0 || orakel.fragen[index - 1].0 != *schluessel {
            verschiedene += 1;
        }
    }
    println!(
        "{} Fragen, {verschiedene} Wechsel des Schluessels, {} Kaempfe gerechnet, {} Siege",
        orakel.fragen.len(),
        orakel.innen.gerechnet(),
        ergebnis.siege.len()
    );
    // Die Zellen muessen sich unterscheiden koennen — sonst verglich der Fall
    // eine Zelle mit sich selbst.
    assert!(
        verschiedene >= 2,
        "nur {verschiedene} verschiedene Zellen gefragt"
    );
}

#[test]
fn fallrate_macht_den_fortschritt_ungueltig_nicht_die_kampftabelle() {
    let referenz = referenz();
    let einstellung = Kampfeinstellung::default();
    let kampf = kampffingerabdruck(&referenz, einstellung);
    let grund = Laufbahnregeln::default();
    let f0 = Fortschrittsfingerabdruck::aus(&grund);

    // ---- Die Regel: jede der fuenf Aenderungen aus AC5
    let aenderungen: Vec<(&str, Laufbahnregeln)> = vec![
        (
            "Fallrate",
            Laufbahnregeln {
                fallrate_promille: 500,
                ..grund.clone()
            },
        ),
        (
            "Beutetabelle",
            Laufbahnregeln {
                katalog: vec![vorlage("focus-mix")],
                ..grund.clone()
            },
        ),
        (
            "Aufwert-Regel",
            Laufbahnregeln {
                kosten_je_stufe: 5,
                ..grund.clone()
            },
        ),
        (
            "Aufwert-Strategie",
            Laufbahnregeln {
                aufwerten: Aufwerten::Nie,
                ..grund.clone()
            },
        ),
        (
            "Freischaltbedingung",
            Laufbahnregeln {
                freischaltung: Freischaltung::AlleOffen,
                ..grund.clone()
            },
        ),
        (
            "Anlege-Strategie",
            Laufbahnregeln {
                anlegen: Anlegen::HoehereStufe,
                ..grund.clone()
            },
        ),
    ];
    for (was, regeln) in &aenderungen {
        let i = invalidierung((f0, kampf), (Fortschrittsfingerabdruck::aus(regeln), kampf));
        assert!(i.fortschritt, "{was}: Fortschritt bliebe gueltig");
        assert!(!i.kampftabelle, "{was}: Kampf-Tabelle wuerde verworfen");
    }
    // Und die Gegenrichtung: Was den Kampf aendert, verwirft beides.
    let zwei = Kampfeinstellung {
        durchgaenge: 2,
        ..einstellung
    };
    let i = invalidierung((f0, kampf), (f0, kampffingerabdruck(&referenz, zwei)));
    assert!(
        i.kampftabelle && i.fortschritt,
        "geaenderte Durchgaenge: {i:?}"
    );
    assert!(
        !invalidierung((f0, kampf), (f0, kampf)).fortschritt,
        "nichts geaendert"
    );

    // ---- Der Lauf: dieselbe Laufbahn mit anderer Fallrate auf DEMSELBEN
    // Speicher. Jede Frage, die Lauf A schon stellte, findet Lauf B dort;
    // gerechnet wird genau, was A nicht gefragt hat.
    let auftrag = kleiner_auftrag(1, 9);
    let mut a_orakel = Mitschrift {
        innen: SpeicherOrakel::neu(&referenz, einstellung),
        fragen: Vec::new(),
    };
    let a = laufbahn(0, 9, &grund, &auftrag.messung, &mut a_orakel).expect("rechnet");
    let fragen_a = verschieden(&a_orakel.fragen);
    let gerechnet_a = a_orakel.gerechnet();
    assert_eq!(
        gerechnet_a,
        fragen_a.len() as u64,
        "innerhalb eines Laufs wurde eine Zelle doppelt gerechnet"
    );
    let speicher = a_orakel.innen.speicher();
    let vorher: Vec<ErgebnisSchluessel> = speicher.schluessel().cloned().collect();
    let werte_vorher: Vec<_> = vorher.iter().map(|k| speicher.hole(k).cloned()).collect();

    let halb = Laufbahnregeln {
        fallrate_promille: 500,
        ..grund.clone()
    };
    let mut b_orakel = Mitschrift {
        innen: SpeicherOrakel::mit_speicher(&referenz, einstellung, speicher),
        fragen: Vec::new(),
    };
    let b = laufbahn(0, 9, &halb, &auftrag.messung, &mut b_orakel).expect("rechnet");
    let fragen_b = verschieden(&b_orakel.fragen);
    let gemeinsam = fragen_b.iter().filter(|s| fragen_a.contains(s)).count() as u64;
    let gerechnet_b = b_orakel.gerechnet() - gerechnet_a;
    println!(
        "Lauf A: {} Fragen, {gerechnet_a} gerechnet. Lauf B (Fallrate 500) auf demselben Speicher: \
         {} Fragen, davon {gemeinsam} schon in A, {gerechnet_b} gerechnet",
        fragen_a.len(),
        fragen_b.len()
    );
    assert!(
        gemeinsam >= 1,
        "A und B teilen keine Frage — der Fall prueft dann kein Wiederfinden"
    );
    assert_eq!(
        gerechnet_b,
        fragen_b.len() as u64 - gemeinsam,
        "Lauf B rechnete Zellen nach, die schon im Speicher lagen"
    );
    let speicher = b_orakel.innen.speicher();
    for (k, wert) in vorher.iter().zip(&werte_vorher) {
        assert_eq!(
            speicher.hole(k).cloned(),
            *wert,
            "eine Zelle aus Lauf A hat sich veraendert"
        );
    }
    assert_ne!(a, b, "die Fallrate hat die Laufbahn nicht veraendert — der Fortschritt waere gar nicht ungueltig");

    // Dieselbe Laufbahn B auf frischem Speicher: Das Wiederverwenden hat
    // nichts am Ergebnis geaendert — die Tabelle war wirklich gueltig.
    let mut frisch = SpeicherOrakel::neu(&referenz, einstellung);
    let b_frisch = laufbahn(0, 9, &halb, &auftrag.messung, &mut frisch).expect("rechnet");
    assert_eq!(
        b, b_frisch,
        "der geteilte Speicher hat das Ergebnis verschoben"
    );

    // ---- Gegenrichtung als Lauf: Mit anderen Durchgaengen findet dieselbe
    // Frage NICHTS im alten Speicher, auch wenn A sie schon gestellt hat.
    let gerechnet_vorher = speicher.gerechnete_kaempfe();
    let mut c_orakel = Mitschrift {
        innen: SpeicherOrakel::mit_speicher(&referenz, zwei, speicher),
        fragen: Vec::new(),
    };
    let _ = laufbahn(0, 9, &grund, &auftrag.messung, &mut c_orakel).expect("rechnet");
    let fragen_c = verschieden(&c_orakel.fragen);
    let schon_gefragt = fragen_c.iter().filter(|s| fragen_a.contains(s)).count();
    let gerechnet_c = c_orakel.gerechnet() - gerechnet_vorher;
    println!(
        "Lauf C (2 Durchgaenge) auf demselben Speicher: {} Fragen, davon {schon_gefragt} schon in A, \
         {gerechnet_c} gerechnet",
        fragen_c.len()
    );
    assert!(
        schon_gefragt >= 1,
        "C stellt keine Frage aus A — dann zeigt der Fall nichts"
    );
    assert_eq!(
        gerechnet_c,
        fragen_c.len() as u64,
        "mit anderer Kampfsemantik muss jede Zelle neu gerechnet werden"
    );
}

/// Die verschiedenen Schluessel einer Mitschrift, in Reihenfolge des ersten
/// Auftretens.
fn verschieden(fragen: &[(OrakelSchluessel, Zelle)]) -> Vec<OrakelSchluessel> {
    let mut raus: Vec<OrakelSchluessel> = Vec::new();
    for (s, _) in fragen {
        if !raus.contains(s) {
            raus.push(*s);
        }
    }
    raus
}

#[test]
fn das_orakel_lehnt_stufen_ab_die_der_kampf_nicht_kennt() {
    let referenz = referenz();
    let mut orakel = SpeicherOrakel::neu(&referenz, Kampfeinstellung::default());
    let schluessel = OrakelSchluessel {
        ausstattung: rota_sim::gear::WornValues::none(),
        boss: "boss-1-1",
        schwierigkeit: 2,
        leiterstufe: 0,
    };
    assert!(matches!(
        orakel.zelle(&schluessel),
        Err(OrakelFehler::NichtImKampf { .. })
    ));
    let leiter = OrakelSchluessel {
        schwierigkeit: 0,
        leiterstufe: 3,
        ..schluessel
    };
    assert!(matches!(
        orakel.zelle(&leiter),
        Err(OrakelFehler::NichtImKampf { .. })
    ));
    let normal = OrakelSchluessel {
        schwierigkeit: 0,
        ..schluessel
    };
    assert!(
        orakel.zelle(&normal).is_ok(),
        "normal / Leiter 0 muss rechenbar sein"
    );
    assert_eq!(
        orakel.gerechnet(),
        1,
        "die abgelehnten Fragen haben gerechnet"
    );
}
