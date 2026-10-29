//! Die vier Befunde der Zauber-Vergleichstabelle (Issue #946, E12.6; darin #953 AC3).
//!
//! # Wogegen gemessen wird
//!
//! First-crucible, ein Durchgang mit festem Startwert. Ohne Ausrüstung würfelt
//! der Kampf nicht, also ist jede Messung exakt — und nur dann trägt ein
//! Beweis. Die Vorräte sind klein, damit die erschöpfende Projektion in
//! Millisekunden durch ist; die Zahlen in den Kommentaren stammen aus Läufen
//! auf node3 am 2026-10-29 (`examples/report.rs` und eine Erkundung).
//!
//! # Was die Fälle zusammen belegen
//!
//! | Fall | AC | Richtung |
//! | --- | --- | --- |
//! | `ac1_…_alle_vier_arten` | AC1 | jede Art kommt vor, je Zauber genau eine |
//! | `ac1_lebensfaehig_…_nicht_gefunden` | AC1 | ein lebensfähiger Zauber unter knappem Budget heißt „nicht gefunden", nicht „dominiert" |
//! | `ac1_ein_dominierter_…` | AC1 | die Kontrolle: ein dominierter heißt „dominiert" |
//! | `ac2_…` | AC2 | nur „bewiesen dominiert" heißt zu schwach |
//! | `ac7_…` | AC7 | jede Schranke ist echt: nachgemessen, und die Liste wirkt den Zauber |
//! | `p953_ac3_…` | #953 AC3 | Laufabschluss und Vertrag sind zwei Zeilen, und beide können fallen |
//! | `ac6_…` | AC6 | eine Mechanik mit einem Weg und eine Pflichtschule fallen auf |
//! | `faecher_…` | #939 | die Fächer kommen aus dem Lauf, nicht aus einem Literal |

use rota_sim::apl::Apl;
use rota_sim::balance::{self, BalanceConfig};
use rota_sim::data::{boss_by_id, index_of, TimelineEntry};
use rota_sim::factors::NEUTRAL;
use rota_sim::objective::Coverage;
use rota_sim::rotation_search::{
    candidates_for, exhaustive, exhaustive_messungen, measure, Candidate,
};
use rota_sim::viabilitaet::{
    befunde, keine_schule_pflicht, mechanikproben, mehrere_wege, portfolio, schulen,
    schulproben, spielbare_zauber, Antwort, Befund, Befundart, Befundtabelle, Laufabschluss,
    Suchgrund, Vertragsstand,
};
use rota_sim::Boss;

fn config() -> BalanceConfig {
    BalanceConfig {
        iterations: 1,
        base_seed: 0,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: boss_by_id("first-crucible").expect("first-crucible"),
        ..BalanceConfig::default()
    }
}

fn idx(id: &str) -> usize {
    index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt"))
}

fn vorrat(ids: &[&str]) -> Vec<Candidate> {
    ids.iter().flat_map(|id| candidates_for(idx(id))).collect()
}

fn grund(ids: &[&str]) -> Vec<usize> {
    ids.iter().map(|id| idx(id)).collect()
}

/// Ein Lauf über genau diese Zauber: sie sind Suchraum und Grundmenge.
fn lauf(ids: &[&str], max_len: usize, budget: u64) -> Befundtabelle {
    befunde(&config(), &vorrat(ids), &grund(ids), max_len, budget)
}

fn befund<'a>(t: &'a Befundtabelle, id: &str) -> &'a Befund {
    &t
        .zeilen
        .iter()
        .find(|z| z.id == id)
        .unwrap_or_else(|| panic!("{id} hat keine Zeile"))
        .befund
}

fn arten(t: &Befundtabelle) -> Vec<(&'static str, Befundart)> {
    t.zeilen.iter().map(|z| (z.id, z.befund.art())).collect()
}

/// Misst eine Liste nach, wie die Projektion sie gemessen hat.
fn nachgemessen(apl: &Apl) -> (f64, Vec<u32>) {
    let cfg = config();
    let lauf = balance::run(&cfg, apl);
    (lauf.damage.mean, lauf.casts)
}

// ------------------------------------------------------------------- AC1

/// Die drei Schritt-1-Zauber, Länge 3: Alle drei stehen in der Elite und
/// tragen bei (5960,0; ohne `smoulder` 5420,0, ohne `pyre` 4660,0). Das ist
/// der Beleg, dass `smoulder` und `pyre` **lebensfähig** sind.
const SCHRITT1: [&str; 3] = ["kindle", "smoulder", "pyre"];

#[test]
fn ac1_lebensfaehig_unter_knappem_budget_heisst_nicht_gefunden() {
    // Erst der Beleg der Lebensfähigkeit, im selben Fall — sonst prüfte der
    // Fall eine Behauptung aus einem Kommentar.
    let voll = lauf(&SCHRITT1, 3, 1_000_000);
    for id in ["smoulder", "pyre"] {
        assert_eq!(
            befund(&voll, id).art(),
            Befundart::MindestensBezeugt,
            "{id} ist bei Laenge 3 bezeugt — sonst belegt dieser Fall nichts: {}",
            befund(&voll, id).beleg()
        );
    }

    // Jetzt knapp: Länge 1 findet nur `kindle` (4000,0), und ein Budget von
    // null lässt keine Projektion zu. Die Suche hat `smoulder` und `pyre`
    // nicht gefunden — und das ist ALLES, was sich sagen lässt.
    let knapp = lauf(&SCHRITT1, 1, 0);
    for id in ["smoulder", "pyre"] {
        let b = befund(&knapp, id);
        assert!(
            matches!(b, Befund::NichtGefunden(Suchgrund::UeberBudget { budget: 0, .. })),
            "{id} ist lebensfaehig und wurde nur nicht gefunden — gemeldet: {} ({})",
            b.art().name(),
            b.beleg()
        );
        assert!(
            !b.ist_zu_schwach(),
            "{id} hiesse zu schwach, obwohl er bei Laenge 3 bezeugt ist"
        );
    }
}

#[test]
fn ac1_ein_dominierter_zauber_heisst_dominiert() {
    // Die Kontrolle zur Richtung darüber: Ohne sie prüfte der Test nur, dass
    // nichts dominiert heißt. `crust` legt eine Schutzschicht gegen einen
    // Boss, der nie zuschlägt, und kostet dafür eine globale Wartezeit —
    // gemessen 3852,0 gegen 5960,0 in 40 Listen.
    let t = lauf(
        &["kindle", "smoulder", "pyre", "crust", "quench", "fester"],
        3,
        1_000_000,
    );
    let b = befund(&t, "crust");
    let Befund::BewiesenDominiert(schranke) = b else {
        panic!(
            "crust verliert in jeder Liste, die ihn wirkt — gemeldet: {} ({})",
            b.art().name(),
            b.beleg()
        );
    };
    assert!(schranke.mit_zauber < schranke.bester, "{}", b.beleg());
}

// ------------------------------------------------------------------- AC2

#[test]
fn ac2_nur_bewiesen_dominiert_heisst_zu_schwach() {
    // Budget 50: `quench` braucht 85 Listen und bleibt „nicht gefunden",
    // `crust` und `fester` (je 40) werden bewiesen dominiert. Damit stehen in
    // EINER Tabelle beide Arten, die eine alte Tabelle als „TOT" zusammenwarf.
    let t = lauf(
        &["kindle", "smoulder", "pyre", "crust", "quench", "fester"],
        3,
        50,
    );
    let gezaehlt = t.zaehlung();
    assert!(
        gezaehlt.von(Befundart::NichtGefunden) >= 1 && gezaehlt.von(Befundart::BewiesenDominiert) >= 1,
        "der Fall braucht beide Arten, sonst trennt er nichts: {:?}",
        arten(&t)
    );

    let schwach: Vec<&str> = t.zu_schwach().iter().map(|z| z.id).collect();
    let dominiert: Vec<&str> = t
        .zeilen
        .iter()
        .filter(|z| z.befund.art() == Befundart::BewiesenDominiert)
        .map(|z| z.id)
        .collect();
    assert_eq!(
        schwach, dominiert,
        "zu schwach darf heissen, was bewiesen dominiert ist — und nur das"
    );

    for zeile in &t.zeilen {
        let dom = zeile.befund.art() == Befundart::BewiesenDominiert;
        assert_eq!(zeile.befund.ist_zu_schwach(), dom, "{}", zeile.id);
        assert_eq!(
            zeile.befund.beleg().contains("zu schwach"),
            dom,
            "{}: der Beleg sagt {:?}",
            zeile.id,
            zeile.befund.beleg()
        );
    }
    for art in Befundart::ALLE {
        assert_eq!(
            art.bedeutung().contains("zu schwach"),
            art == Befundart::BewiesenDominiert,
            "{}: {}",
            art.name(),
            art.bedeutung()
        );
    }
}

#[test]
fn auflage_bewiesen_dominiert_nie_ohne_geltungsbereich() {
    // Auflage vom 2026-10-29, aus dem eigenen Befund: Ohne Budgetgrenze heißt
    // `smoulder` in der Projektion der Länge 1 dominiert und ist bei Länge 3
    // bezeugt. Ein Beweis ist so viel wert wie seine Projektion — also steht
    // sie in der Befund-Spalte, nicht nur im Beleg.
    let t = lauf(
        &["kindle", "smoulder", "pyre", "crust", "quench", "fester"],
        3,
        1_000_000,
    );
    let bereich = "in der Projektion Elite-Zeilen + eigene Zeilen, Laenge <= 3, \
                   ohne Ausruestung, first-crucible";
    let mut dominiert = 0;
    for zeile in &t.zeilen {
        if zeile.befund.art() == Befundart::BewiesenDominiert {
            dominiert += 1;
            assert_eq!(
                zeile.befund.bezeichnung(),
                format!("bewiesen dominiert — {bereich}"),
                "{}",
                zeile.id
            );
            assert_eq!(
                zeile.befund.geltungsbereich().as_deref(),
                Some(bereich),
                "{}",
                zeile.id
            );
            assert!(zeile.befund.beleg().contains(bereich), "{}", zeile.id);
        } else {
            assert_eq!(zeile.befund.geltungsbereich(), None, "{}", zeile.id);
            assert_eq!(zeile.befund.bezeichnung(), zeile.befund.art().name());
        }
    }
    assert!(dominiert >= 2, "{:?}", arten(&t));
}

// ------------------------------------------------------------------- AC7

#[test]
fn ac7_jede_schranke_ist_nachgemessen_und_wirkt_den_zauber() {
    let t = lauf(
        &["kindle", "smoulder", "pyre", "crust", "quench", "fester"],
        3,
        1_000_000,
    );
    let mut geprueft = 0;
    for zeile in &t.zeilen {
        let Befund::BewiesenDominiert(s) = &zeile.befund else {
            continue;
        };
        geprueft += 1;
        assert!(s.mit_zauber < s.bester, "{}: {}", zeile.id, zeile.befund.beleg());
        assert!(
            s.projektion.vollstaendig(),
            "{}: {} Listen, die Projektion braucht {}",
            zeile.id,
            s.projektion.listen,
            exhaustive_messungen(s.projektion.zeilen.len(), s.projektion.max_len)
        );

        // Die Schranke ist eine LISTE, keine Zahl: nachgemessen kommt
        // derselbe Wert heraus, und sie wirkt den Zauber wirklich.
        let (wert, casts) = nachgemessen(&s.liste_mit_zauber);
        assert_eq!(wert.to_bits(), s.mit_zauber.to_bits(), "{}", zeile.id);
        assert!(
            casts[zeile.spell] > 0,
            "{}: die Schranken-Liste wirkt ihn gar nicht — dann ist sie keine",
            zeile.id
        );
        let (bester, _) = nachgemessen(&s.beste_liste);
        assert_eq!(bester.to_bits(), s.bester.to_bits(), "{}", zeile.id);

        // Und `bester` ist wirklich das Optimum der Projektion: dieselbe Zahl,
        // die `exhaustive` selbst auf diesen Zeilen findet.
        let gesamt = exhaustive(&config(), &s.projektion.zeilen, s.projektion.max_len);
        assert_eq!(gesamt.best.mean.to_bits(), s.bester.to_bits(), "{}", zeile.id);
    }
    assert!(
        geprueft >= 2,
        "nur {geprueft} dominierte Zauber — der Fall soll an mehr als einem pruefen: {:?}",
        arten(&t)
    );
}

#[test]
fn ac7_ohne_schranke_kein_dominiert_sondern_ungeklaert() {
    // `quench` kostet keine globale Wartezeit und richtet an einem Boss ohne
    // Zauber nichts aus. In einer Liste mit freiem Platz steht er umsonst mit:
    // gleichauf mit der besten, und ohne seine Zeilen rechnet sie genauso.
    // Gegen ihn lässt sich keine Schranke bilden, die verliert — und ein
    // Zeuge, der nichts bewirkt, ist keiner. Im Gesamtlauf vom 29.10. steht er
    // so: „gleichauf (9403.0), aber ohne messbaren Beitrag".
    //
    // Der Suchraum kennt ihn hier nicht (nur die Schritt-1-Zeilen), die
    // Grundmenge schon: Die Projektion bekommt seine Zeilen trotzdem.
    //
    // Das ist der Stoppfall aus dem Issue: „Halt und melden, wenn für einen
    // Zauber keine Schranke ableitbar ist. Dann ist der Befund ungeklärt."
    let t = befunde(
        &config(),
        &vorrat(&SCHRITT1),
        &grund(&["kindle", "smoulder", "pyre", "quench"]),
        6,
        1_000_000,
    );
    let b = befund(&t, "quench");
    assert!(
        matches!(b, Befund::Ungeklaert(_)),
        "quench richtet an first-crucible nichts aus und kostet keine globale \
         Wartezeit — gleichauf heisst weder bezeugt noch dominiert. Gemeldet: {} ({})",
        b.art().name(),
        b.beleg()
    );
    assert!(!b.ist_zu_schwach());
}

// -------------------------------------------------------------- #953 AC3

#[test]
fn p953_ac3_lauf_abgeschlossen_und_vertrag_verletzt_sind_zwei_zeilen() {
    // Drei von vier bezeugt, einer bewiesen dominiert: Der Lauf hat alles
    // getan, was er sich vorgenommen hatte — und der Vertrag ist trotzdem
    // verletzt. Zwei Aussagen, zwei Zeilen.
    let t = lauf(&["kindle", "smoulder", "pyre", "crust"], 3, 1_000_000);
    assert_eq!(t.laufabschluss(), Laufabschluss::Abgeschlossen);
    assert_eq!(
        t.vertrag(),
        Vertragsstand::Verletzt {
            verlangt: 4,
            bescheinigt: 3
        }
    );

    let text = t.zusammenfassung();
    let lauf_zeilen: Vec<&String> = text.iter().filter(|z| z.starts_with("LAUF:")).collect();
    let vertrag_zeilen: Vec<&String> = text
        .iter()
        .filter(|z| z.starts_with("BALANCE-VERTRAG:"))
        .collect();
    assert_eq!(lauf_zeilen.len(), 1, "{text:#?}");
    assert_eq!(vertrag_zeilen.len(), 1, "{text:#?}");
    assert!(lauf_zeilen[0].contains("abgeschlossen"), "{}", lauf_zeilen[0]);
    assert!(!lauf_zeilen[0].contains("VERTRAG"), "{}", lauf_zeilen[0]);
    assert!(
        vertrag_zeilen[0].contains("NICHT ERFUELLT"),
        "{}",
        vertrag_zeilen[0]
    );

    // Und der Gegenfall: Fehlt ein Zauber, ist der Lauf NICHT abgeschlossen —
    // sonst wäre die Zeile eine, die nicht fallen kann.
    let mut lueckenhaft = t.clone();
    let weg = lueckenhaft.zeilen.pop().expect("eine Zeile").spell;
    assert!(
        matches!(
            lueckenhaft.laufabschluss(),
            Laufabschluss::NichtAbgeschlossen { ref ohne_befund, .. } if ohne_befund == &[weg]
        ),
        "{:?}",
        lueckenhaft.laufabschluss()
    );

    // Ebenso eine Projektion, die nicht jede Liste gemessen hat.
    let mut halb = t.clone();
    for zeile in &mut halb.zeilen {
        if let Befund::BewiesenDominiert(s) = &mut zeile.befund {
            s.projektion.listen -= 1;
        }
    }
    assert!(
        matches!(
            halb.laufabschluss(),
            Laufabschluss::NichtAbgeschlossen { ref unvollstaendig, .. } if !unvollstaendig.is_empty()
        ),
        "{:?}",
        halb.laufabschluss()
    );
}

// ------------------------------------------------------------------- AC6

/// Eine künstliche Mechanik mit genau einem Weg: `test-lethal-strike`, aber der
/// Schlag fällt bei 1000 ms statt 12000 ms.
///
/// # Warum genau einer
///
/// - **Unterbrechen** geht nicht: ein `strike` ist nicht unterbrechbar, nur PIN
///   öffnet ihm ein Fenster.
/// - **FIXATION** kommt nicht rechtzeitig: Jeder Zauber der Schule löst die
///   globale Wartezeit aus (1500 ms), jeder Kontrollzauber kostet mindestens
///   zwei Punkte, und Punkte gibt es erst mit dem ersten Wurf.
/// - **COAGULATION** gewinnt: RIPOSTE gibt die Hälfte des angekommenen
///   Schlags zurück, 9200 gegen 5200 Lebenspunkte — der Boss fällt im selben
///   Takt, und dann gewinnt der Spieler.
///
/// Gemessen auf node3 (Erkundung, 2026-10-29): unterbrechen kein Sieg,
/// FIXATION kein Sieg, COAGULATION `riposte`. Der Zwilling bei 12000 ms
/// (`test-lethal-strike` selbst) hat zwei Wege: FIXATION mit `pin` und
/// `quench`, COAGULATION mit `riposte`.
fn ein_weg() -> &'static Boss {
    let basis = boss_by_id("test-lethal-strike").expect("test-lethal-strike");
    assert_eq!(basis.timeline.len(), 1, "der Zwilling traegt genau einen Schlag");
    let schlag = TimelineEntry {
        at_ms: 1000,
        ..basis.timeline[0]
    };
    Box::leak(Box::new(Boss {
        timeline: Box::leak(vec![schlag].into_boxed_slice()),
        ..*basis
    }))
}

fn mit_boss(boss: &'static Boss) -> BalanceConfig {
    BalanceConfig {
        boss,
        ..config()
    }
}

fn voller_vorrat() -> Vec<Candidate> {
    spielbare_zauber().into_iter().flat_map(candidates_for).collect()
}

#[test]
fn ac6_eine_mechanik_mit_nur_einem_weg_faellt_auf() {
    let proben = mechanikproben(&mit_boss(ein_weg()), &voller_vorrat(), 6);
    let liste = proben.as_ref().expect("nur ein strike, also messbar");
    assert_eq!(liste.len(), 1);
    let wege: Vec<Antwort> = liste[0]
        .wege
        .iter()
        .filter(|(_, sieg)| sieg.is_some())
        .map(|(weg, _)| *weg)
        .collect();
    assert_eq!(wege, [Antwort::Koagulation], "{:?}", liste[0].wege);
    let fach = mehrere_wege(&proben);
    assert_eq!(fach, Coverage::Measured { covered: 0, required: 1 });
    assert!(!fach.is_met(), "eine Mechanik mit einem Weg muss auffallen");

    // Der Zwilling: derselbe Schlag bei 12000 ms. Ohne ihn wüsste niemand, ob
    // die Zählung überhaupt zwei Wege erkennen kann.
    let zwilling = mechanikproben(
        &mit_boss(boss_by_id("test-lethal-strike").expect("boss")),
        &voller_vorrat(),
        6,
    );
    let zwilling_liste = zwilling.as_ref().expect("messbar");
    assert!(
        zwilling_liste[0].loesende() >= 2,
        "{:?}",
        zwilling_liste[0].wege
    );
    assert_eq!(
        mehrere_wege(&zwilling),
        Coverage::Measured { covered: 1, required: 1 }
    );
}

#[test]
fn ac6_eine_pflichtschule_faellt_auf() {
    // Gegen die künstliche Mechanik gewinnt nur COAGULATION — ohne diese Schule
    // findet sich kein Sieg, ohne jede andere schon.
    let grundmenge = spielbare_zauber();
    let proben = schulproben(&mit_boss(ein_weg()), &voller_vorrat(), &grundmenge, 6);
    let ohne_sieg: Vec<&str> = proben
        .iter()
        .filter(|p| p.sieg_ohne.is_none())
        .map(|p| p.schule)
        .collect();
    assert_eq!(ohne_sieg, ["COAGULATION"], "{proben:?}");
    let fach = keine_schule_pflicht(&proben);
    assert!(!fach.is_met(), "{fach:?}");

    // Und an first-crucible ist keine Schule Pflicht — gemessen 5 von 5.
    let frei = schulproben(&config(), &voller_vorrat(), &grundmenge, 6);
    let n = u32::try_from(schulen(&grundmenge).len()).expect("wenige Schulen");
    assert_eq!(
        keine_schule_pflicht(&frei),
        Coverage::Measured {
            covered: n,
            required: n
        }
    );
}

// ------------------------------------------------------------------ #939

#[test]
fn faecher_kommen_aus_dem_lauf_und_bleiben_nicht_leer() {
    // Die Fächer aus #939 standen bis hierher nur in Prüffällen gefüllt, mit
    // Literalen. Hier entstehen sie aus einem Lauf — und eine Zahl, die aus
    // dem Lauf kommt, muss mit dem Lauf übereinstimmen.
    let t = lauf(&["kindle", "smoulder", "pyre", "crust"], 3, 1_000_000);
    let gezaehlt = t.zaehlung();
    assert_eq!(
        t.kampagnenabdeckung().every_spell_has_viable_niche,
        Coverage::Measured {
            covered: u32::try_from(gezaehlt.von(Befundart::MindestensBezeugt)).expect("klein"),
            required: u32::try_from(t.grundmenge.len()).expect("klein"),
        }
    );

    let grundmenge = spielbare_zauber();
    let schul = schulproben(&config(), &voller_vorrat(), &grundmenge, 6);
    let mech = mechanikproben(&config(), &voller_vorrat(), 6);
    let menge = portfolio(&schul, &mech);
    assert_ne!(menge.no_school_is_globally_mandatory, Coverage::Unmeasured);
    // First-crucible hat keine Mechanik: gemessen null von null, nicht
    // ungemessen. Das Fach ist damit gefüllt — und erfüllt, weil es nichts zu
    // verletzen gibt.
    assert_eq!(
        menge.every_mechanic_has_multiple_solution_families,
        Coverage::Measured {
            covered: 0,
            required: 0
        }
    );
    // Robustheit über Begegnungen braucht mehr als eine: bewusst ungemessen.
    assert_eq!(menge.robustness_across_encounters, Coverage::Unmeasured);
}

// ------------------------------------------------- die Zählung der Projektion

#[test]
fn exhaustive_misst_so_viele_listen_wie_vorhergesagt() {
    // Das Budget wird VOR dem Lauf gegen diese Zahl geprüft. Läge sie neben
    // dem, was `exhaustive` tatsächlich misst, prüfte das Budget eine andere
    // Projektion als die gerechnete.
    assert_eq!(exhaustive_messungen(7, 4), 7 + 42 + 210 + 840);
    assert_eq!(exhaustive_messungen(3, 6), 3 + 6 + 6, "Laenge ueber dem Vorrat");
    assert_eq!(exhaustive_messungen(0, 6), 0);
    for (ids, max_len) in [
        (&["kindle", "smoulder", "pyre"][..], 3),
        (&["kindle", "smoulder", "pyre", "crust"][..], 2),
    ] {
        let pool = vorrat(ids);
        let gemessen = exhaustive(&config(), &pool, max_len).measured;
        assert_eq!(
            u64::try_from(gemessen).expect("klein"),
            exhaustive_messungen(pool.len(), max_len),
            "{ids:?} bis {max_len}"
        );
    }
}

#[test]
fn ein_zeuge_aus_einer_zeile_wird_gegen_die_leere_liste_gemessen() {
    // Der Zeuge zählt nur, wenn die Liste ohne die Zeilen seines Zaubers
    // messbar weniger bringt. Besteht die Elite aus einer Zeile, ist „ohne"
    // die leere Liste — sie muss rechnen, und zwar null.
    let leer = measure(&config(), "leer", &[]);
    assert_eq!(leer.mean.to_bits(), 0.0_f64.to_bits());

    let knapp = lauf(&SCHRITT1, 1, 0);
    let Befund::MindestensBezeugt(zeuge) = befund(&knapp, "kindle") else {
        panic!("kindle ist die Elite der Laenge 1: {}", befund(&knapp, "kindle").beleg());
    };
    assert_eq!(zeuge.liste.rules.len(), 1);
    assert_eq!(zeuge.ohne_zauber.to_bits(), 0.0_f64.to_bits());
    assert!(zeuge.wert > 0.0);
}
