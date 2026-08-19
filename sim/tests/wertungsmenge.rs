//! Wertungsmenge und Instanzgruppen (Issue #981).
//!
//! Die Bauzeit-Bruecke (AC4, AC7) laesst sich hier nicht pruefen — ein Test
//! kann den eigenen Bau nicht scheitern lassen. Sie ist im Bericht mit den
//! tatsaechlichen Fehlermeldungen belegt; hier steht, was zur Laufzeit gilt,
//! **und** die Gegenrichtung zu AC7: dass der Bestand die Pruefung besteht.

use rota_sim::data::{self, Boss};
use rota_sim::scoring::{
    class_of, probe_bosses, BossClass, InstanceGroup, ScoringError, ScoringSet,
};

/// Die Zahlen des Bestands, gemessen gegen `origin/main` (AC6).
///
/// **`PRUEFPROFILE` ist am 2026-08-19 von 18 auf 20 gewachsen**, und die
/// Herkunft gehört dazu: 18 war der Stand aus #981; dazu kamen
/// `test-lethal-strike` und `test-survivable-strike` aus #797 — die beiden
/// Zwillinge für AC5, beide in der Instanz `testing`.
///
/// **`WERTUNGSFAEHIG` bleibt bei 17.** Das ist kein Zufall, sondern die Probe:
/// Läge eines der neuen Profile versehentlich in einer gewerteten Instanz,
/// müsste diese Zahl mitwachsen. Eine unveränderte Konstante als Beleg ist
/// stärker als eine angepasste.
const WERTUNGSFAEHIG: usize = 17;
const GRUPPEN: usize = 5;
const PRUEFPROFILE: usize = 20;

fn boss(id: &str) -> &'static Boss {
    data::boss_by_id(id).unwrap_or_else(|| panic!("Boss {id} fehlt im Datensatz"))
}

// ------------------------------------------------------------------- AC1

/// AC1 — jeder Boss traegt eine Klasse, abgeleitet aus seinen Daten.
#[test]
fn ac1_jeder_boss_traegt_eine_klasse() {
    for b in data::BOSSES.iter() {
        let klasse = BossClass::of(b.instance);
        assert!(
            klasse.is_some(),
            "Boss {} traegt instance {:?} ohne Klasse",
            b.id,
            b.instance
        );
    }
    // Die Klassifikation haengt am instance-Wert, nicht an einer Kennung: Zwei
    // Bosse derselben Instanz bekommen dieselbe Klasse, ohne dass sie
    // irgendwo einzeln aufgezaehlt waeren.
    assert_eq!(BossClass::of("instance-2"), Some(BossClass::Scored));
    assert_eq!(BossClass::of("testing"), Some(BossClass::Probe));
    assert_eq!(
        BossClass::of("instance-5"),
        None,
        "ein unbekannter Wert hat keine Klasse — und bricht im Bau"
    );
}

/// Es gibt keine Liste von Boss-Kennungen im Quelltext.
///
/// **Der Prueffall, der die Stop-Condition traegt.** Eine solche Liste altert
/// bei jedem neuen Boss, und zwar lautlos. Hier wird gemessen, dass die
/// Klassifikation allein am `instance`-Wert haengt: Ein frei erfundener Boss
/// mit einer Kennung, die nirgends steht, bekommt trotzdem die richtige Klasse.
#[test]
fn die_klassifikation_kennt_keine_boss_kennungen() {
    let erfunden = Boss {
        id: "gibt-es-nicht-und-wird-es-nie-geben",
        instance: "instance-3",
        ..*boss("boss-1-1")
    };
    assert_eq!(class_of(&erfunden), BossClass::Scored);

    let erfunden_pruef = Boss {
        id: "auch-diesen-nicht",
        instance: "testing",
        ..*boss("boss-1-1")
    };
    assert_eq!(class_of(&erfunden_pruef), BossClass::Probe);
}

// ------------------------------------------------------------------- AC2

/// AC2 — ein Pruefboss in einer Wertungsmenge bricht ab, mit Boss und Stelle.
#[test]
fn ac2_ein_pruefboss_bricht_die_wertungsmenge() {
    let menge = [boss("boss-1-1"), boss("test-runaway"), boss("boss-1-2")];
    let fehler = ScoringSet::of(&menge).expect_err("ein Pruefboss darf nicht durchgehen");

    assert_eq!(
        fehler,
        ScoringError::ProbeBossInScoringSet {
            boss: "test-runaway",
            // **`test`, nicht `testing`.** Beide Werte kommen im Bestand vor —
            // 5 Bosse tragen `test`, 13 tragen `testing`. Ich hatte hier
            // `testing` erwartet; der Fall hat es gefangen. Die zwei Namen
            // unterscheiden sich um zwei Buchstaben und um nichts sonst, was
            // ein Mensch sieht.
            instance: "test",
            position: 1,
        },
        "der Fehler muss den Boss UND die Stelle nennen"
    );

    // Die Meldung selbst gehoert geprueft: Sie ist das, was ein Mensch sieht.
    let text = fehler.to_string();
    assert!(text.contains("test-runaway"), "Meldung ohne Boss: {text}");
    assert!(text.contains("Stelle 1"), "Meldung ohne Stelle: {text}");
}

/// AC2, Kontrollprobe — dieselbe Menge ohne den Pruefboss laeuft durch.
///
/// Ohne sie pruefte der Fall darueber nur, dass der Lader **alles** ablehnt.
#[test]
fn ac2_kontrollprobe_ohne_pruefboss_laeuft_durch() {
    let menge = [boss("boss-1-1"), boss("boss-1-2")];
    let geprueft = ScoringSet::of(&menge).expect("zwei Spielbosse sind eine gueltige Menge");
    assert_eq!(geprueft.len(), 2);
    assert!(!geprueft.is_empty());
}

/// Jeder einzelne Pruefboss des Bestands wird zurueckgewiesen.
///
/// **Nicht nur einer.** Ein Fall mit `test-runaway` allein belegt, dass die
/// Sperre greift; erst dieser belegt, dass sie fuer alle achtzehn greift — und
/// die Zahl steht daneben, damit ein geschrumpfter Bestand auffaellt.
#[test]
fn jedes_pruefprofil_wird_zurueckgewiesen() {
    let profile = probe_bosses();
    assert_eq!(profile.len(), PRUEFPROFILE);
    for p in &profile {
        assert!(
            ScoringSet::of(&[p]).is_err(),
            "{} ging als wertungsfaehig durch",
            p.id
        );
    }
}

// ------------------------------------------------------------------- AC3

/// AC3 — die wertungsfaehigen Bosse sind nach Instanz gruppiert.
#[test]
fn ac3_wertungsfaehige_bosse_sind_nach_instanz_gruppiert() {
    let menge = ScoringSet::all_scored();
    assert_eq!(menge.len(), WERTUNGSFAEHIG);

    let gruppen = menge.groups();
    assert_eq!(gruppen.len(), GRUPPEN);

    // Jeder Boss der Menge liegt in genau einer Gruppe, und die Gruppen
    // decken die Menge vollstaendig ab.
    let summe: usize = gruppen.iter().map(InstanceGroup::len).sum();
    assert_eq!(
        summe,
        menge.len(),
        "die Gruppen decken die Wertungsmenge weder mehrfach noch lueckenhaft ab"
    );

    // Und keine Gruppe ist leer — eine leere Gruppe waere eine Instanz ohne
    // Bosse, also ein Datenfehler.
    for g in &gruppen {
        assert!(!g.is_empty(), "Gruppe {} ist leer", g.instance);
    }
}

/// Es gibt keine Gruppe „alle Bosse" (Stop-Condition).
///
/// Geprueft an der Eigenschaft, nicht am Namen: **Keine** Gruppe enthaelt alle
/// wertungsfaehigen Bosse. Ein Test auf den Namen „alle" liesse sich mit einer
/// Umbenennung aushebeln.
#[test]
fn keine_gruppe_umfasst_die_ganze_wertungsmenge() {
    let menge = ScoringSet::all_scored();
    for g in menge.groups() {
        assert!(
            g.len() < menge.len(),
            "Gruppe {} umfasst die ganze Wertungsmenge — das waere eine globale Elite",
            g.instance
        );
    }
}

// ------------------------------------------------------------------- AC5

/// AC5 — `instance` hat nachweislich Leser: Eine Aenderung kippt die
/// Gruppierung.
///
/// # Warum das der eigentliche Nachweis ist
///
/// Das Feld hatte vor diesem Issue bereits Leser — drei in `rota-sim.rs`, aus
/// #897. Zwei davon geben es nur aus. „Hat Leser" ist damit zu schwach als
/// Kriterium: Ein Feld, das nur gedruckt wird, traegt keine Entscheidung.
/// Gemessen wird deshalb, dass die **Gruppierung** sich aendert.
#[test]
fn ac5_eine_aenderung_an_instance_kippt_die_gruppierung() {
    let vorher = InstanceGroup {
        instance: "instance-1",
    };
    let nachher = InstanceGroup {
        instance: "instance-2",
    };
    let mitglieder_vorher: Vec<&str> = vorher.members().iter().map(|b| b.id).collect();
    let mitglieder_nachher: Vec<&str> = nachher.members().iter().map(|b| b.id).collect();

    assert_ne!(
        mitglieder_vorher, mitglieder_nachher,
        "zwei Instanzen mit derselben Mitgliederliste — dann gruppiert niemand"
    );
    assert_eq!(mitglieder_vorher.len(), 4);
    assert_eq!(mitglieder_nachher.len(), 4);

    // Und am einzelnen Boss: Derselbe Boss, ein anderer instance-Wert, eine
    // andere Gruppe. Das ist die Gegenprobe am lebenden Objekt, nur ohne die
    // Bossdatei anzufassen.
    let original = boss("boss-1-1");
    let umgehaengt = Boss {
        instance: "instance-4",
        ..*original
    };
    let menge_original = ScoringSet::require(&[original]);
    let umgehaengt_static: &'static Boss = Box::leak(Box::new(umgehaengt));
    let menge_umgehaengt = ScoringSet::require(&[umgehaengt_static]);

    assert_eq!(menge_original.groups()[0].instance, "instance-1");
    assert_eq!(menge_umgehaengt.groups()[0].instance, "instance-4");
    assert_ne!(
        menge_original.groups()[0].instance,
        menge_umgehaengt.groups()[0].instance,
        "die Gruppierung folgt dem Feld — sonst liest es niemand"
    );
}

/// Und die Klasse kippt mit: Ein Spielboss, auf ein Pruefprofil umgehaengt,
/// wird zurueckgewiesen.
#[test]
fn ac5_umhaengen_auf_ein_pruefprofil_macht_unzulaessig() {
    let umgehaengt: &'static Boss = Box::leak(Box::new(Boss {
        instance: "testing",
        ..*boss("boss-1-1")
    }));
    assert!(
        ScoringSet::of(&[umgehaengt]).is_err(),
        "derselbe Boss, ein anderer instance-Wert — und er gehoert in keine Wertung mehr"
    );
}

// ------------------------------------------------------------------- AC6

/// AC6 — die Zahlen des Bestands werden beim Lauf geprueft.
///
/// Weicht der Bestand ab, faellt dieser Fall. Das ist gewollt: Eine geaenderte
/// Bosszahl ist entweder Absicht oder ein Fehler, und beides muss jemand sehen.
///
/// **Laeuft dieser Fall ueber einen Arbeitsbaum mit nicht eingecheckten
/// Bossdateien, meldet er eine Abweichung — das ist richtig und kein
/// Fehlalarm.** Er liest die Daten so, wie der Bau sie gesehen hat.
#[test]
fn ac6_die_zahlen_des_bestands() {
    let wertungsfaehig = ScoringSet::all_scored();
    let pruefprofile = probe_bosses();

    assert_eq!(
        wertungsfaehig.len(),
        WERTUNGSFAEHIG,
        "wertungsfaehige Bosse: erwartet {WERTUNGSFAEHIG}, gemessen {}",
        wertungsfaehig.len()
    );
    assert_eq!(
        wertungsfaehig.groups().len(),
        GRUPPEN,
        "Instanzgruppen: erwartet {GRUPPEN}, gemessen {}",
        wertungsfaehig.groups().len()
    );
    assert_eq!(
        pruefprofile.len(),
        PRUEFPROFILE,
        "Pruefprofile: erwartet {PRUEFPROFILE}, gemessen {}",
        pruefprofile.len()
    );

    // Die Kontrollzeile: Beide Klassen zusammen sind der ganze Bestand. Ohne
    // sie koennten beide Zahlen stimmen, waehrend ein Boss in keine der
    // beiden Mengen faellt.
    assert_eq!(
        wertungsfaehig.len() + pruefprofile.len(),
        data::BOSSES.len(),
        "ein Boss faellt durch beide Klassen"
    );
}

// ------------------------------------------------------------------- AC7

/// AC7, die Gegenrichtung — der Bestand besteht die Schluesselpruefung.
///
/// Der Bau bricht bei einem Schluessel, der keine Schule benennt; das ist im
/// Bericht mit der echten Fehlermeldung belegt. Hier steht die andere Haelfte:
/// **dass gueltige Schluessel durchgehen.** Eine Pruefung, die alles ablehnt,
/// ist keine.
///
/// Die Schulenmenge entsteht auch hier aus den Daten — aus dem Feld `school`
/// der Zauber —, nicht aus einer Liste im Prueffall.
#[test]
fn ac7_jeder_widerstands_schluessel_benennt_eine_schule() {
    let mut schulen: Vec<&str> = Vec::new();
    for spell in data::SPELLS.iter() {
        if let Some(schule) = spell.school {
            if !schulen.contains(&schule) {
                schulen.push(schule);
            }
        }
    }
    assert!(
        schulen.len() >= 5,
        "nur {} Schulen gefunden — die Ableitung greift nicht, und dann bestuende \
         dieser Fall aus dem falschen Grund",
        schulen.len()
    );

    let mut geprueft = 0;
    for b in data::BOSSES.iter() {
        for (schluessel, _) in b.resistances {
            assert!(
                schulen.contains(schluessel),
                "Boss {} fuehrt Widerstand {:?}, das keine Schule benennt — bekannt: {:?}",
                b.id,
                schluessel,
                schulen
            );
            geprueft += 1;
        }
    }
    // Die Zahl in den Datensatz: Faende dieser Fall null Schluessel, waere er
    // gruen und wertlos.
    assert_eq!(
        geprueft, 4,
        "erwartet 4 Widerstands-Schluessel im Bestand, gemessen {geprueft}"
    );
}
