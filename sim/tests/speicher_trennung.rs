//! Ergebnis und Bewertung sind getrennt (Issue #913).
//!
//! # Was hier NICHT geprueft werden kann
//!
//! AC2 — dass ein zielfunktions-abhaengiges Feld den **Bau** bricht — ist ein
//! Uebersetzungsfehler. Gaebe es das Feld, liesse sich diese Datei nicht
//! uebersetzen. Der Nachweis steht im Pull Request als echter roter Lauf mit
//! der tatsaechlichen Fehlermeldung.
//!
//! Alles Uebrige ist zur Laufzeit sichtbar: die Unabhaengigkeit des Schemas
//! (AC1), der Hash im Schluessel (AC3), der Kampfzaehler beim Zielwechsel
//! samt **beiden** Gegenproben (AC4), die Trennung im Archiv (AC5) und die
//! Sperre gegen Torsi in **beide** Richtungen (AC6).
//!
//! # Und einer, den es ohne Speisung nicht gaebe
//!
//! [`der_echte_laeufer_fuellt_den_vektor`] faehrt einen wirklichen
//! Balance-Lauf. Ohne ihn waere jeder Fall hier ueber von Hand gesetzten
//! Zahlen gruen — und das Modul trotzdem an keinen Lauf angeschlossen.

use rota_sim::apl::AplName;
use rota_sim::balance::{BalanceConfig, Statistics};
use rota_sim::data::boss_by_id;
use rota_sim::factors::NEUTRAL;
use rota_sim::speicher::{
    Bewertung, BewertungsSchluessel, Bewertungsspeicher, ErgebnisSchluessel, Ergebnisentwurf,
    Ergebnisspeicher, Nischenbelegung, Optimalitaet, SemantikHash, Siegzahl, Sucharchiv,
    Wirkungszahlen, ZielfunktionsHash, Zielwechsel, FELDER_IM_SCHEMA,
};

const SEMANTIK_A: SemantikHash = SemantikHash(0xA1);
const SEMANTIK_B: SemantikHash = SemantikHash(0xB2);
const ZIEL_A: ZielfunktionsHash = ZielfunktionsHash(0x1111);
const ZIEL_B: ZielfunktionsHash = ZielfunktionsHash(0x2222);

/// Eine Kennzahl mit dem gewuenschten Mittelwert.
///
/// Ueber `from_sorted` und nicht ueber von Hand gesetzte Felder: Die Formel
/// wird aufgerufen, nicht nachgebaut. Zwei Werte, damit die Streuung definiert
/// ist.
fn stat(mittel: f64) -> Statistics {
    Statistics::from_sorted(&[mittel - 1.0, mittel + 1.0])
}

/// Ein vollstaendiger Entwurf.
fn voller_entwurf(schaden: f64) -> Ergebnisentwurf {
    Ergebnisentwurf {
        schaden: Some(stat(schaden)),
        erlittener_schaden: Some(stat(100.0)),
        eingehender_schaden: Some(stat(140.0)),
        absorption: Some(stat(40.0)),
        heilung: Some(stat(20.0)),
        boss_heilung: Some(stat(5.0)),
        siegzeit_ms: Some(stat(40_000.0)),
        todeszeit_ms: Some(stat(0.0)),
        durchgaenge: Some(1000),
        siege: Some(Siegzahl(900)),
        tode: Some(5),
        wirkungen: Some(Wirkungszahlen(vec![12, 0, 7])),
    }
}

fn schluessel(aufbau: &str, semantik: SemantikHash) -> ErgebnisSchluessel {
    ErgebnisSchluessel {
        aufbau: aufbau.to_string(),
        boss: "boss-1-1".to_string(),
        semantik,
    }
}

// ------------------------------------------------------------------- AC1

#[test]
fn kein_feld_des_ergebnisses_haengt_an_der_zielfunktion() {
    // Die Aufzaehlung laeuft durch `nur_zielfunktionsfrei`, das den Marker
    // verlangt. Was hier zurueckkommt, hat den Uebersetzer passiert — das ist
    // die eigentliche Zusicherung; dieser Fall haelt fest, dass sie greift und
    // dass die Zahl der Felder stimmt.
    let vektor = voller_entwurf(5000.0).abschliessen().expect("vollstaendig");
    let felder = vektor.felder_aufgezaehlt();

    assert_eq!(
        felder.len(),
        FELDER_IM_SCHEMA,
        "Das Schema hat {FELDER_IM_SCHEMA} Felder. Weicht die Zahl ab, hat die \
         Destrukturierung eines verloren — und dann haette der Bau brechen muessen."
    );

    // Kein Feldname deutet auf eine Bewertung hin. Das ist eine schwaechere
    // Pruefung als der Marker, aber sie faengt einen anderen Fall: ein Feld
    // vom Typ Statistics, das inhaltlich eine Bewertung ist.
    let verdaechtig = ["score", "wert", "rang", "rank", "guete", "note", "punkte"];
    for (name, _) in &felder {
        for v in verdaechtig {
            assert!(
                !name.contains(v),
                "Das Feld '{name}' klingt nach einer Bewertung ('{v}'). Auch ein Gesamtwert \
                 oder ein Rang ist bereits eine Bewertung und gehoert nicht in den \
                 Ergebnisspeicher."
            );
        }
    }

    // Kein Feld doppelt: Sonst koennte die Aufzaehlung zwoelf Eintraege haben
    // und trotzdem ein Feld auslassen.
    let mut namen: Vec<&str> = felder.iter().map(|(n, _)| *n).collect();
    namen.sort_unstable();
    let vorher = namen.len();
    namen.dedup();
    assert_eq!(
        namen.len(),
        vorher,
        "Ein Feldname taucht doppelt auf — dann verdeckt er ein anderes Feld."
    );
}

// -------------------------------------------------- Die Speisung (AC1, real)

/// Der einzige Fall hier, der einen **echten** Kampf rechnet.
///
/// Er belegt, dass das Schema nicht neben dem Simulator steht, sondern von ihm
/// gefuellt wird — und dass jedes der zwoelf Felder aus einer gemessenen
/// Groesse kommt, nicht aus einer Voreinstellung.
#[test]
fn der_echte_laeufer_fuellt_den_vektor() {
    let boss = boss_by_id("boss-1-1").expect("boss-1-1 ist im Bestand");
    let config = BalanceConfig {
        iterations: 3,
        base_seed: 0,
        threads: 1,
        // Feste Dauer statt des Ueberdruck-Timers: Ein Lauf, der bei 0 ms
        // endet, liefert einen vollstaendigen Vektor voller Nullen — und der
        // saehe gruen aus.
        duration_ms: Some(60_000),
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    };
    // KINDLE zuerst, EMBER als Fueller — und diese Reihenfolge ist gemessen,
    // nicht gewaehlt: EMBER hat laut Datenbestand **keinen** Grundschaden.
    // Stand er vorn, verdraengte er KINDLE vollstaendig, und der Lauf lieferte
    // 90 Wirkungen bei 0 Schaden — ein vollstaendiger Vektor voller Nullen.
    let apl = rota_sim::apl::parse(AplName("speicher-913"), "kindle\nember\n")
        .expect("zwei bekannte Zauber");
    let lauf = rota_sim::balance::run(&config, &apl);

    let vektor = lauf
        .ergebnisentwurf()
        .abschliessen()
        .expect("ein echter Lauf liefert IMMER einen vollstaendigen Vektor");

    // Die Zusicherung, dass der Lauf die Sache ueberhaupt beruehrt hat: Ohne
    // sie waere dieser Fall auch ueber einem Lauf gruen, der nichts tut.
    assert!(
        vektor.schaden.mean > 0.0,
        "Ein Lauf ohne Schaden misst nichts — dann sagt der Rest dieses Falls nichts aus. \
         Gemessen: schaden.mean={}, count={}, durchgaenge={}, wirkungen={}",
        vektor.schaden.mean,
        vektor.schaden.count,
        vektor.durchgaenge,
        vektor.wirkungen.gesamt()
    );
    assert!(
        vektor.wirkungen.gesamt() > 0,
        "Es wurde kein einziger Zauber gewirkt. Die Ereignisse sind das Feld aus AC1, das \
         am leichtesten leer bleibt, ohne dass es auffaellt."
    );

    // Bitgleich uebernommen, nicht neu gerechnet.
    assert_eq!(vektor.schaden.mean.to_bits(), lauf.damage.mean.to_bits());
    assert_eq!(
        vektor.erlittener_schaden.mean.to_bits(),
        lauf.damage_taken.mean.to_bits()
    );
    assert_eq!(
        vektor.eingehender_schaden.mean.to_bits(),
        lauf.damage_incoming.mean.to_bits()
    );
    assert_eq!(
        vektor.absorption.mean.to_bits(),
        lauf.absorbed.mean.to_bits()
    );
    assert_eq!(vektor.heilung.mean.to_bits(), lauf.healed.mean.to_bits());
    assert_eq!(
        vektor.boss_heilung.mean.to_bits(),
        lauf.boss_healed.mean.to_bits()
    );
    assert_eq!(
        vektor.siegzeit_ms.mean.to_bits(),
        lauf.kill_time_ms.mean.to_bits()
    );
    assert_eq!(
        vektor.todeszeit_ms.mean.to_bits(),
        lauf.death_time_ms.mean.to_bits()
    );
    assert_eq!(vektor.durchgaenge, lauf.iterations);
    assert_eq!(vektor.siege.0 as usize, lauf.kills);
    assert_eq!(vektor.tode as usize, lauf.deaths);
    assert_eq!(vektor.wirkungen.0, lauf.casts);

    // Und er laesst sich ablegen.
    let mut speicher = Ergebnisspeicher::neu();
    speicher
        .speichere(
            schluessel("ember-kindle", SEMANTIK_A),
            lauf.ergebnisentwurf(),
        )
        .expect("ein echter Lauf wird angenommen");
    assert_eq!(speicher.anzahl(), 1);
    assert_eq!(speicher.abgelehnte(), 0);
}

// ------------------------------------------------------------------- AC3

#[test]
fn zwei_zielfunktionen_ueberschreiben_einander_nicht() {
    // AC3: Der Zielfunktions-Hash steht im SCHLUESSEL. Stuende er im Wert,
    // lieferte die zweite Bewertung die erste zurueck — still und plausibel.
    let mut bewertungen = Bewertungsspeicher::neu();
    let erg = schluessel("aufbau-1", SEMANTIK_A);

    bewertungen.setze(
        BewertungsSchluessel {
            ergebnis: erg.clone(),
            zielfunktion: ZIEL_A,
        },
        Bewertung(1.0),
    );
    bewertungen.setze(
        BewertungsSchluessel {
            ergebnis: erg.clone(),
            zielfunktion: ZIEL_B,
        },
        Bewertung(2.0),
    );

    assert_eq!(
        bewertungen.anzahl(),
        2,
        "Zwei Zielfunktionen ueber DEMSELBEN Ergebnis muessen zwei Eintraege ergeben — \
         sonst ueberschreibt die zweite die erste."
    );
    assert_eq!(
        bewertungen.hole(&BewertungsSchluessel {
            ergebnis: erg.clone(),
            zielfunktion: ZIEL_A,
        }),
        Some(Bewertung(1.0)),
        "Die Bewertung der ersten Zielfunktion muss unveraendert abrufbar bleiben."
    );
    assert_eq!(
        bewertungen.hole(&BewertungsSchluessel {
            ergebnis: erg,
            zielfunktion: ZIEL_B,
        }),
        Some(Bewertung(2.0))
    );
    assert_eq!(bewertungen.anzahl_fuer(ZIEL_A), 1);
    assert_eq!(bewertungen.anzahl_fuer(ZIEL_B), 1);
}

#[test]
fn eine_unbekannte_zielfunktion_liefert_nichts_statt_der_falschen() {
    let mut bewertungen = Bewertungsspeicher::neu();
    let erg = schluessel("aufbau-1", SEMANTIK_A);
    bewertungen.setze(
        BewertungsSchluessel {
            ergebnis: erg.clone(),
            zielfunktion: ZIEL_A,
        },
        Bewertung(1.0),
    );
    assert_eq!(
        bewertungen.hole(&BewertungsSchluessel {
            ergebnis: erg,
            zielfunktion: ZIEL_B,
        }),
        None,
        "Fuer eine Zielfunktion ohne Bewertung darf NICHTS zurueckkommen — nicht die \
         Bewertung einer anderen. Lieber keine Antwort als eine falsche."
    );
}

// ------------------------------------------------------------------- AC4

/// Faehrt einen Satz Aufbauten und zaehlt die gerechneten Kaempfe.
fn fahre(speicher: &mut Ergebnisspeicher, semantik: SemantikHash, aufbauten: &[&str]) {
    for (i, a) in aufbauten.iter().enumerate() {
        speicher
            .hole_oder_rechne(schluessel(a, semantik), || {
                voller_entwurf(1000.0 + i as f64)
            })
            .expect("vollstaendiger Entwurf");
    }
}

#[test]
fn ein_zielwechsel_rechnet_null_kaempfe_neu() {
    let aufbauten = ["a", "b", "c", "d"];
    let mut ergebnisse = Ergebnisspeicher::neu();
    let mut bewertungen = Bewertungsspeicher::neu();

    // Erster Lauf: kalt.
    fahre(&mut ergebnisse, SEMANTIK_A, &aufbauten);
    let nach_erstem = ergebnisse.gerechnete_kaempfe();
    assert_eq!(
        nach_erstem, 4,
        "Ein kalter Speicher MUSS rechnen — sonst misst der Zaehler nichts."
    );

    // Bewertung nach Ziel A.
    for a in aufbauten {
        let erg = schluessel(a, SEMANTIK_A);
        let mittel = ergebnisse.hole(&erg).expect("liegt vor").schaden.mean;
        bewertungen.setze(
            BewertungsSchluessel {
                ergebnis: erg,
                zielfunktion: ZIEL_A,
            },
            Bewertung(mittel),
        );
    }

    // ---- DER ZIELWECHSEL ----
    for a in aufbauten {
        let erg = schluessel(a, SEMANTIK_A);
        let vektor = ergebnisse
            .hole(&erg)
            .expect("das rohe Ergebnis bleibt gueltig");
        // Eine voellig andere Zielfunktion ueber DENSELBEN Zahlen.
        let neuer_wert = -vektor.erlittener_schaden.mean;
        bewertungen.setze(
            BewertungsSchluessel {
                ergebnis: erg,
                zielfunktion: ZIEL_B,
            },
            Bewertung(neuer_wert),
        );
    }

    assert_eq!(
        ergebnisse.gerechnete_kaempfe(),
        nach_erstem,
        "EIN ZIELWECHSEL DARF KEINEN EINZIGEN KAMPF KOSTEN. Der Zaehler stand bei \
         {nach_erstem} und steht jetzt bei {}.",
        ergebnisse.gerechnete_kaempfe()
    );
    assert_eq!(bewertungen.anzahl_fuer(ZIEL_A), 4);
    assert_eq!(bewertungen.anzahl_fuer(ZIEL_B), 4);
}

#[test]
fn gegenprobe_eins_ohne_zielwechsel_rechnet_ebenfalls_null() {
    // Ohne diese Gegenprobe misst der Fall oben nur, dass ein Zaehler auf
    // einem Wert steht — nicht, dass der Speicher warm ist.
    let aufbauten = ["a", "b", "c", "d"];
    let mut ergebnisse = Ergebnisspeicher::neu();

    fahre(&mut ergebnisse, SEMANTIK_A, &aufbauten);
    let nach_erstem = ergebnisse.gerechnete_kaempfe();

    fahre(&mut ergebnisse, SEMANTIK_A, &aufbauten);

    assert_eq!(
        ergebnisse.gerechnete_kaempfe(),
        nach_erstem,
        "Derselbe Lauf ueber einem warmen Speicher darf ebenfalls nichts rechnen."
    );
    assert_eq!(ergebnisse.anzahl(), 4, "und nichts doppelt ablegen");
}

#[test]
fn gegenprobe_zwei_geaenderte_kampfsemantik_rechnet_alles_neu() {
    // Die wichtigere der beiden: Sie belegt, dass der Zaehler ueberhaupt
    // ausschlaegt. Ohne sie waere 'null Kaempfe' auch dann gruen, wenn der
    // Speicher jeden Schluessel faelschlich als Treffer liest.
    let aufbauten = ["a", "b", "c", "d"];
    let mut ergebnisse = Ergebnisspeicher::neu();

    fahre(&mut ergebnisse, SEMANTIK_A, &aufbauten);
    let nach_erstem = ergebnisse.gerechnete_kaempfe();
    assert_eq!(nach_erstem, 4);

    // Dieselben Aufbauten, andere KAMPFSEMANTIK.
    fahre(&mut ergebnisse, SEMANTIK_B, &aufbauten);

    assert_eq!(
        ergebnisse.gerechnete_kaempfe(),
        nach_erstem + 4,
        "Eine geaenderte Kampfsemantik MUSS alle Kaempfe neu rechnen — die alten Ergebnisse \
         gelten dann nicht mehr. Der Semantik-Fingerabdruck steht deshalb im SCHLUESSEL."
    );
    assert_eq!(
        ergebnisse.anzahl(),
        8,
        "Beide Semantiken liegen nebeneinander, keine ueberschreibt die andere."
    );
}

// ------------------------------------------------------------------- AC5

#[test]
fn das_archiv_haelt_nischen_und_optimalitaet_getrennt() {
    let mut archiv = Sucharchiv::neu();
    archiv.nischen.belege("hoher-schaden");
    archiv.nischen.belege("lange-ueberlebenszeit");
    archiv
        .optimalitaet
        .melde("aufbau-1", Bewertung(5.0), ZIEL_A);

    assert_eq!(archiv.nischen.anzahl(), 2);
    assert_eq!(
        archiv.optimalitaet.bester(ZIEL_A),
        Some(("aufbau-1", Bewertung(5.0)))
    );

    // ---- DER ZIELWECHSEL ----
    archiv.zielwechsel();

    assert_eq!(
        archiv.nischen.anzahl(),
        2,
        "Die Nischenbelegung UEBERLEBT: Eine Nische ist durch das ERGEBNIS definiert, nicht \
         dadurch, ob es gut war. Sie zu verwerfen hiesse, gerechnete Kaempfe wegzuwerfen."
    );
    assert!(
        archiv.nischen.ist_belegt("hoher-schaden"),
        "und zwar namentlich, nicht nur der Zahl nach"
    );
    assert_eq!(
        archiv.optimalitaet.bester(ZIEL_A),
        None,
        "Die Optimalitaet WIRD VERWORFEN: 'das beste Ergebnis' ist ohne Zielfunktion nicht \
         definiert, und ein alter Bestwert unter neuem Ziel waere keine Behauptung, sondern \
         eine falsche."
    );
}

#[test]
fn die_entscheidung_steht_im_typ_nicht_im_kommentar() {
    // 'Unentschieden ist falsch' — also muss die Entscheidung ablesbar sein,
    // ohne den Quelltext zu lesen.
    assert_eq!(Nischenbelegung::VERHALTEN, Zielwechsel::Ueberlebt);
    assert_eq!(Optimalitaet::VERHALTEN, Zielwechsel::Verworfen);
    assert_ne!(
        Nischenbelegung::VERHALTEN,
        Optimalitaet::VERHALTEN,
        "Die beiden Teile des Archivs muessen sich unterscheiden — sonst waere die Trennung \
         ohne Folge."
    );
}

#[test]
fn ein_bestwert_der_falschen_zielfunktion_wird_nicht_ausgegeben() {
    // Auch ohne ausdruecklichen Zielwechsel: Wer nach ZIEL_B fragt, bekommt
    // keinen Bestwert, der zu ZIEL_A gehoert.
    let mut opt = Optimalitaet::default();
    opt.melde("aufbau-1", Bewertung(9.0), ZIEL_A);
    assert!(opt.bester(ZIEL_A).is_some());
    assert_eq!(
        opt.bester(ZIEL_B),
        None,
        "Ein Bestwert gilt nur fuer die Zielfunktion, unter der er entstand."
    );
}

#[test]
fn ein_schlechterer_kandidat_verdraengt_den_besten_nicht() {
    // Sonst waere `melde` ein Setzer und `bester` waere 'der zuletzt
    // gemeldete' — der Name waere dann falsch, und AC5 haette nichts, worauf
    // es sich beziehen koennte.
    let mut opt = Optimalitaet::default();
    opt.melde("gut", Bewertung(9.0), ZIEL_A);
    opt.melde("schlecht", Bewertung(1.0), ZIEL_A);
    assert_eq!(opt.bester(ZIEL_A), Some(("gut", Bewertung(9.0))));
}

// ------------------------------------------------------------------- AC6

#[test]
fn ein_unvollstaendiger_vektor_wird_abgelehnt() {
    let mut ergebnisse = Ergebnisspeicher::neu();
    let mut torso = voller_entwurf(1000.0);
    torso.heilung = None;
    torso.tode = None;

    let ergebnis = ergebnisse.speichere(schluessel("torso", SEMANTIK_A), torso);
    let fehler = ergebnis.expect_err(
        "Ein unvollstaendiger Vektor MUSS abgelehnt werden. Ein Torso im Speicher waere \
         spaeter nicht mehr als solcher erkennbar — ein Lauf mit anderer Zielfunktion faende \
         ihn und hielte ihn fuer ein Ergebnis.",
    );
    assert_eq!(
        fehler.fehlend,
        vec!["heilung", "tode"],
        "Die Meldung muss nennen, WAS fehlt — sonst ist der Torso nicht zu reparieren."
    );
    assert_eq!(
        ergebnisse.anzahl(),
        0,
        "und nichts darf abgelegt worden sein"
    );
    assert_eq!(ergebnisse.abgelehnte(), 1);
}

#[test]
fn ein_vollstaendiger_vektor_wird_angenommen() {
    // Die zweite Richtung, die das Issue ausdruecklich verlangt: 'Eine Sperre,
    // die alles ablehnt, ist keine Sperre, sondern ein kaputter Speicher.'
    let mut ergebnisse = Ergebnisspeicher::neu();
    ergebnisse
        .speichere(schluessel("ganz", SEMANTIK_A), voller_entwurf(1000.0))
        .expect("ein vollstaendiger Vektor MUSS angenommen werden");
    assert_eq!(ergebnisse.anzahl(), 1);
    assert_eq!(ergebnisse.abgelehnte(), 0);
    assert!(ergebnisse.hole(&schluessel("ganz", SEMANTIK_A)).is_some());
}

#[test]
fn jedes_einzelne_fehlende_feld_wird_erkannt() {
    // Sonst faengt die Sperre nur den Fall, in dem viele Felder fehlen — und
    // ein Torso mit genau einer Luecke kaeme durch.
    /// Name des Feldes und der Griff, der es entfernt.
    type Luecke = (&'static str, fn(&mut Ergebnisentwurf));

    let setzer: [Luecke; FELDER_IM_SCHEMA] = [
        ("schaden", |e| e.schaden = None),
        ("erlittener_schaden", |e| e.erlittener_schaden = None),
        ("eingehender_schaden", |e| e.eingehender_schaden = None),
        ("absorption", |e| e.absorption = None),
        ("heilung", |e| e.heilung = None),
        ("boss_heilung", |e| e.boss_heilung = None),
        ("siegzeit_ms", |e| e.siegzeit_ms = None),
        ("todeszeit_ms", |e| e.todeszeit_ms = None),
        ("durchgaenge", |e| e.durchgaenge = None),
        ("siege", |e| e.siege = None),
        ("tode", |e| e.tode = None),
        ("wirkungen", |e| e.wirkungen = None),
    ];
    for (name, entferne) in setzer {
        let mut entwurf = voller_entwurf(1000.0);
        entferne(&mut entwurf);
        let fehler = entwurf
            .abschliessen()
            .expect_err("ein fehlendes Feld muss auffallen");
        assert_eq!(
            fehler.fehlend,
            vec![name],
            "Bei fehlendem '{name}' muss genau dieses Feld gemeldet werden."
        );
    }
}

// -------------------------------------------------- Die Trennung als Ganzes

#[test]
fn der_ergebnisspeicher_kennt_die_zielfunktion_nicht_einmal_im_schluessel() {
    // Der Schluessel traegt Aufbau, Boss und SEMANTIK — keine Zielfunktion.
    // Waere sie darin, entstuende je Zielfunktion ein eigener Ergebnissatz,
    // und ein Zielwechsel koestete wieder jeden Kampf.
    let a = schluessel("x", SEMANTIK_A);
    let b = schluessel("x", SEMANTIK_A);
    assert_eq!(
        a, b,
        "Zwei Schluessel mit gleichem Aufbau, Boss und gleicher Semantik muessen gleich sein \
         — unabhaengig davon, welche Zielfunktion gerade gilt."
    );
    let anders = schluessel("x", SEMANTIK_B);
    assert_ne!(a, anders, "aber verschiedene Semantik trennt sie");
}

#[test]
fn ein_zielwechsel_laesst_die_rohen_zahlen_bitgleich() {
    // Die Zusicherung, um die es im ganzen Issue geht: Das rohe Ergebnis
    // bleibt VOLLSTAENDIG gueltig.
    let mut ergebnisse = Ergebnisspeicher::neu();
    let s = schluessel("a", SEMANTIK_A);
    ergebnisse
        .speichere(s.clone(), voller_entwurf(1234.5))
        .expect("vollstaendig");
    let vorher = ergebnisse.hole(&s).expect("liegt vor").clone();

    let mut bewertungen = Bewertungsspeicher::neu();
    for zf in [ZIEL_A, ZIEL_B] {
        bewertungen.setze(
            BewertungsSchluessel {
                ergebnis: s.clone(),
                zielfunktion: zf,
            },
            Bewertung(1.0),
        );
    }

    let nachher = ergebnisse.hole(&s).expect("liegt weiter vor");
    assert_eq!(
        vorher.schaden.mean.to_bits(),
        nachher.schaden.mean.to_bits(),
        "Das rohe Ergebnis muss BITGLEICH bleiben — nicht nur gleich gerundet."
    );
    assert_eq!(&vorher, nachher);
    assert_eq!(ergebnisse.gerechnete_kaempfe(), 0, "und ohne einen Kampf");
}
