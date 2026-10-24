//! Hält fest, dass ein strikter Vergleich (`<`) auf einem zeitabhängigen Wert
//! **später** greift als der nicht-strikte (`<=`) — und um wie viel.
//!
//! # Was dieser Fall festhält, und was nicht
//!
//! Er hält den **gemessenen Ist-Zustand** fest, nicht den gewollten. Ob `<`
//! greifen soll, wie es dasteht, ist in #1027 offen und wird vom Auftraggeber
//! entschieden. Bis dahin ist der Verzug nicht abgesichert: Er kann sich
//! verschieben, ohne dass etwas rot wird — auch in die *richtige* Richtung,
//! dann wäre das Problem behoben und niemand wüsste es.
//!
//! **Wird dieser Fall rot, ist das eine Information, keine Panne.** Er gehört
//! dann angepasst, nicht repariert: Wer ihn stillschweigend auf die neuen
//! Zahlen zieht, löscht den einzigen Beleg dafür, dass sich etwas geändert hat.
//!
//! # Warum die Eigenschaft und nicht die Zahl
//!
//! Der Verzug ist **nicht konstant**. Gemessen am 2026-08-22:
//!
//! | Boss | erster Eintrag | Umschlagpunkt | `<=` greift | `<` greift | Verzug |
//! | --- | ---: | ---: | ---: | ---: | ---: |
//! | `test-survivable-strike` | 12000 | 9000 | 9000 | 10000 | 1000 ms |
//! | `boss-1-2` | 10000 | 7000 | 7000 | 7500 | 500 ms |
//!
//! Gleiche Zeile, gleiche Schwelle, verschiedener Verzug: Er ist der Abstand
//! bis zum **nächsten regulären Ereignis**, und der hängt davon ab, wo der
//! Umschlagpunkt ins Raster fällt. Eine einzelne Zahl festzunageln hieße, eine
//! Eigenschaft des Aufbaus für eine Eigenschaft der Sache zu halten.
//!
//! Deshalb prüft dieser Fall die **Richtung** (`<` greift später) an zwei
//! Aufbauten und die konkreten Punkte je Aufbau getrennt.
//!
//! # Die Ursache ist geklärt (2026-10-24)
//!
//! Messung, Befehle und Zahlen: `docs/APL-STRICT-COMPARISON-2026-10-24.md`,
//! Abschnitt „Woher der Verzug kommt". Hier stand bis dahin, die Ursache sei
//! offen; der Abschnitt ist ersetzt, nicht gestrichen.
//!
//! **Die naheliegende Erklärung stimmt.** Der Weckruf aus `switch_point` liegt
//! auf dem Zeitpunkt, an dem der Wert die Schwelle **erreicht**; dort ist `<=`
//! wahr und `<` noch falsch, und einen zweiten Weckruf stellt `switch_point`
//! am Umschlag für keine Form. Gemessen: Mit einem Weckruf auf dem ersten Takt,
//! an dem der Vergleich kippt (Fassung B, Wegwerf-Patch), wirft `< 3000` bei
//! 9010 statt 10000 und bei 7010 statt 7500.
//!
//! **Die Mutation vom 2026-08-22 konnte das nicht zeigen.** Sie schaltete den
//! Weckruf für `BossNextStrikeIn` ab, und alle Fälle hier blieben grün — aber
//! in diesen Aufbauten weckt am Umschlag ohnehin etwas anderes: `purge` wird
//! bei 7500 gewirkt, sein GCD endet bei 9000; bei `boss-1-2` tickt bei 7000 das
//! Verhängnis. Der Weckruf von `<=` ist dort überflüssig, und der von `<` liegt
//! dort, wo `<` noch falsch ist. Ohne `purge` fällt `<= 3000` unter derselben
//! Mutation von 9000 auf 10000 (`der_weckruf_traegt_ohne_fremdes_ereignis`).
//! Die Positiv-Kontrolle von damals (`next_strike_in_ms()` konstant 0, drei von
//! vier rot) belegt den Bauweg, nicht die Ursache.
//!
//! **Das Raster ist das nächste schon eingereihte Ereignis**: der Hitzeverfall
//! alle 2000 ms (10000), das Ende der Abklingzeit von `purge` (7500), ohne
//! `purge` der Verhängnis-Takt (8000).

use rota_sim::apl::{parse, Apl, AplName, Compare, Condition, Rule, Term, Value};
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::log::LogKind;
use rota_sim::time::TICK_MS;

/// Erster Eintrag bei 12000 ms → Umschlagpunkt für `< 3000` liegt bei 9000.
const BOSS_12000: &str = "test-survivable-strike";
/// Erster Eintrag bei 10000 ms → Umschlagpunkt bei 7000.
const BOSS_10000: &str = "boss-1-2";

fn wuerfe(boss: &str, text: &str, spell: &str) -> u32 {
    let apl: Apl = parse(AplName("probe"), text)
        .unwrap_or_else(|e| panic!("Prioritätenliste ist fehlerhaft:\n{e}"));
    let index = SPELLS
        .iter()
        .position(|s| s.id == spell)
        .unwrap_or_else(|| panic!("Zauber {spell} gibt es nicht"));
    Fight::new(FightConfig {
        boss: boss_by_id(boss).unwrap_or_else(|| panic!("Boss {boss} fehlt")),
        stop_on_kill: false,
        ..FightConfig::default()
    })
    .run_apl(&apl)
    .casts[index]
}

/// Kleinste obere Zeitschranke, unter der die Zeile noch wirft — also der
/// Zeitpunkt des ersten Wurfs, auf 250 ms genau.
fn erster_wurf(boss: &str, bedingung: &str) -> Option<u32> {
    (500..=20_000).step_by(250).find(|grenze| {
        let apl = format!("kindle if {bedingung} and time < {grenze}\npurge\n");
        wuerfe(boss, &apl, "kindle") > 0
    })
}

/// **Trägerprüfung der Messmethode.** Ohne sie belegt `erster_wurf` nichts:
/// Eine Fassung, die immer dieselbe Zahl liefert, sähe bei einer erwarteten
/// Zahl genauso aus wie eine richtige.
#[test]
fn die_messmethode_unterscheidet_ueberhaupt() {
    assert_eq!(
        erster_wurf(BOSS_12000, "time < 0"),
        None,
        "Eine Bedingung, die nie wahr ist, hat einen Wurf ergeben — dann misst \
         `erster_wurf` nicht die Bedingung, sondern irgendetwas anderes."
    );
    assert_eq!(
        erster_wurf(BOSS_12000, "time >= 0"),
        Some(500),
        "Eine Bedingung, die immer wahr ist, muss beim ersten möglichen \
         Zeitpunkt werfen."
    );
}

#[test]
fn strikt_greift_spaeter_als_nicht_strikt_bei_eintrag_12000() {
    let nicht_strikt = erster_wurf(BOSS_12000, "boss.next_strike_in <= 3000");
    let strikt = erster_wurf(BOSS_12000, "boss.next_strike_in < 3000");
    assert_eq!(
        nicht_strikt,
        Some(9250),
        "`<= 3000` soll am Umschlagpunkt 9000 greifen"
    );
    assert_eq!(
        strikt,
        Some(10250),
        "`< 3000` greift gemessen erst bei 10000"
    );
    assert!(
        strikt > nicht_strikt,
        "Der strikte Vergleich müsste später greifen als der nicht-strikte. \
         Tut er es nicht mehr, wurde #1027 entschieden — dann gehört dieser \
         Fall angepasst, nicht repariert."
    );
}

#[test]
fn derselbe_verzug_faellt_bei_anderem_raster_anders_aus() {
    let nicht_strikt = erster_wurf(BOSS_10000, "boss.next_strike_in <= 3000");
    let strikt = erster_wurf(BOSS_10000, "boss.next_strike_in < 3000");
    assert_eq!(
        nicht_strikt,
        Some(7250),
        "Umschlagpunkt liegt hier bei 7000"
    );
    assert_eq!(
        strikt,
        Some(7750),
        "`<` greift hier bei 7500, nicht bei 8000"
    );
    assert!(
        strikt > nicht_strikt,
        "Auch bei diesem Aufbau muss der strikte Vergleich später greifen."
    );
}

/// Der Verzug ist **500 ms hier und 1000 ms dort**. Diese Ungleichheit ist der
/// eigentliche Befund: Wer abwägt, ob ein Eingriff sich lohnt, darf die 1000
/// nicht für eine Eigenschaft der Sache halten.
#[test]
fn der_verzug_ist_nicht_konstant() {
    let verzug = |boss: &str| {
        let a = erster_wurf(boss, "boss.next_strike_in <= 3000").expect("`<=` warf nie");
        let b = erster_wurf(boss, "boss.next_strike_in < 3000").expect("`<` warf nie");
        b - a
    };
    let weit = verzug(BOSS_12000);
    let eng = verzug(BOSS_10000);
    assert_eq!(weit, 1000, "Verzug bei erstem Eintrag 12000");
    assert_eq!(eng, 500, "Verzug bei erstem Eintrag 10000");
    assert_ne!(
        weit, eng,
        "Wären beide gleich, wäre der Verzug eine Eigenschaft der Sprache. Er \
         ist eine des Aufbaus: der Abstand bis zum nächsten regulären Ereignis."
    );
}

// ---------------------------------------------------------------------------
// Alle sechs Formen gegen die Takt-Referenz (#1027, gemessen am 2026-10-24)
// ---------------------------------------------------------------------------
//
// Die Fälle oben vergleichen `<` mit `<=` — zwei Formen an einem Wert. Die
// folgenden fragen für JEDE der sechs Formen, beide Operandenreihenfolgen und
// drei Werte: Wirft die Liste dort, wo das Spiel werfen könnte?
//
// **Die Messlatte ist kein hergeleiteter Zeitpunkt, sondern ein zweiter Lauf.**
// Das Spiel prüft seinen Zustand nur an Takt-Grenzen (`TICK_MS`). Ein
// Simulator, der richtig weckt, muss also dieselben Würfe liefern wie einer,
// der an JEDEM Takt entscheidet. Den baut `mit_takt_referenz` in der Sprache
// selbst: eine angehängte Zeile, die nie greift und je Takt eine Gruppe
// `time >= t and time < 0` trägt. Sie weckt an jedem Takt und wirft nie.
//
// **Die Tabellen halten den Ist-Zustand fest (Fassung A).** Wo Ereignis und
// Referenz auseinanderliegen, verpasst die Form heute ihren Umschlagpunkt.
// Unter Fassung B (Weckruf auf den ersten Takt, an dem der Vergleich kippt)
// ändern sich genau diese Zeilen, und jede Tabelle wird an ihrer ersten
// verspäteten Zeile rot — gemessen, siehe
// `docs/APL-STRICT-COMPARISON-2026-10-24.md`. Rot ist dann eine Information:
// anpassen, nicht reparieren.

/// Die Liste, ergänzt um eine Zeile, die nie greift und an jedem Takt in
/// `[von, bis)` weckt.
///
/// `time >= t` kippt auf der steigenden Uhr bei `t` — in Fassung A wie in B,
/// die Referenz hängt also an keiner von beiden. `time < 0` weckt nie.
fn mit_takt_referenz(apl: &Apl, von: u32, bis: u32) -> Apl {
    assert_eq!(von % TICK_MS, 0, "Referenzfenster beginnt neben dem Takt");
    let groups = (von..bis)
        .step_by(TICK_MS as usize)
        .map(|t| {
            vec![
                Term {
                    left: Value::Time,
                    compare: Compare::GreaterEqual,
                    right: Value::Literal(i64::from(t)),
                },
                Term {
                    left: Value::Time,
                    compare: Compare::Less,
                    right: Value::Literal(0),
                },
            ]
        })
        .collect();
    let mut dicht = apl.clone();
    dicht.rules.push(Rule {
        spell: zauber("kindle"),
        condition: Condition { groups },
        line: apl.rules.len() + 1,
    });
    dicht
}

fn zauber(id: &str) -> usize {
    SPELLS
        .iter()
        .position(|s| s.id == id)
        .unwrap_or_else(|| panic!("Zauber {id} gibt es nicht"))
}

/// Die Würfe von `kindle` im Fenster `[von, bis)`, über die volle Kampfzeit.
fn kindle_im_fenster(boss: &str, apl: &Apl, von: u32, bis: u32) -> Vec<u32> {
    wuerfe_im_fenster(boss, apl, "kindle", von, bis)
}

/// Die Würfe von `spell` im Fenster `[von, bis)`, über die volle Kampfzeit.
fn wuerfe_im_fenster(boss: &str, apl: &Apl, spell: &str, von: u32, bis: u32) -> Vec<u32> {
    let gesucht = zauber(spell);
    let mut fight = Fight::new(FightConfig {
        boss: boss_by_id(boss).unwrap_or_else(|| panic!("Boss {boss} fehlt")),
        stop_on_kill: false,
        log_capacity: 1 << 16,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(apl);
    assert!(
        !fight.log().overflowed(),
        "Kampfprotokoll übergelaufen — die Würfe wären ein Ausschnitt"
    );
    assert!(
        fight.died_at_ms().is_none_or(|tod| tod >= bis),
        "{boss}: Spieler stirbt vor dem Fensterende {bis} — null Würfe wären dann gleich und ohne Aussage"
    );
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::Cast && usize::from(e.spell) == gesucht)
        .map(|e| e.time)
        .filter(|t| (von..bis).contains(t))
        .collect()
}

/// Eine Zeile der Tabelle: Form, vertauschte Seiten, erster Wurf im Fenster
/// ereignisgesteuert, erster Wurf gegen die Takt-Referenz.
type Fall = (&'static str, bool, Option<u32>, Option<u32>);

/// Fährt alle zwölf Zeilen einer Tabelle und prüft sie.
///
/// - Der erste Wurf beider Läufe steht in der Tabelle — gemessen, nicht
///   hergeleitet.
/// - Stimmen beide überein, muss das GANZE Fenster übereinstimmen, nicht nur
///   der erste Wurf.
/// - Liegen sie auseinander, wirft die Referenz früher. Das ist der Verzug.
fn pruefe_alle_formen(
    boss: &str,
    vorlauf: &str,
    wert: &str,
    schwelle: i64,
    umschlag: u32,
    tabelle: &[Fall],
) {
    let von = umschlag - 500;
    let bis = umschlag + 3000;
    let mut gemessen = Vec::new();
    for &(form, vertauscht, ereignis_erwartet, referenz_erwartet) in tabelle {
        let bedingung = if vertauscht {
            format!("{schwelle} {form} {wert}")
        } else {
            format!("{wert} {form} {schwelle}")
        };
        let text = format!("{vorlauf}kindle if {bedingung}\n");
        let apl: Apl = parse(AplName("probe"), &text)
            .unwrap_or_else(|e| panic!("Prioritätenliste ist fehlerhaft:\n{e}"));
        let ereignis = kindle_im_fenster(boss, &apl, von, bis);
        let referenz = kindle_im_fenster(boss, &mit_takt_referenz(&apl, von, bis), von, bis);
        gemessen.push(format!(
            "{bedingung:<34} Ereignis {:>6?}  Referenz {:>6?}",
            ereignis.first(),
            referenz.first()
        ));
        assert_eq!(
            (ereignis.first().copied(), referenz.first().copied()),
            (ereignis_erwartet, referenz_erwartet),
            "`{bedingung}` an {boss}: erster Wurf (Ereignis, Referenz) hat sich \
             verschoben. Steht die Weckruf-Fassung zur Entscheidung (#1027), ist \
             das die Messung — anpassen, nicht reparieren."
        );
        if ereignis_erwartet == referenz_erwartet {
            assert_eq!(
                ereignis, referenz,
                "`{bedingung}` an {boss}: erster Wurf gleich, das Fenster nicht"
            );
        } else {
            assert!(
                ereignis.first() > referenz.first() || ereignis.is_empty(),
                "`{bedingung}` an {boss}: Die Referenz entscheidet an jedem Takt; \
                 ein Lauf, der FRÜHER wirft als sie, misst etwas anderes"
            );
        }
    }
    println!(
        "{wert} an {boss}, Umschlag {umschlag}:\n  {}",
        gemessen.join("\n  ")
    );
}

/// **Trägerprüfung der Referenz.** Ohne sie belegt die Gleichheit unten nichts:
/// Eine Referenz, die gar nicht weckt, stimmte mit jedem Lauf überein.
///
/// Zwei Richtungen: Die Zeile wirft selbst nie (sonst verfälschte sie den
/// Lauf), und an einem Fall, den der Simulator nicht weckt, wirft sie früher.
///
/// **Der zweite Fall hängt an keiner Weckruf-Fassung**, und das ist Absicht:
/// `time > dot.smoulder.remains` hat zwei zeitabhängige Seiten, und die
/// überspringt `Apl::next_wakeup` in Fassung A wie in B. Die Seiten kreuzen
/// sich bei 6000 (SMOULDER läuft bis 12000), wahr wird der Vergleich beim Takt
/// danach. Ein Fall, den nur A verpasst, wäre unter B aus dem falschen Grund
/// rot geworden — er hätte die Fassung geprüft statt der Referenz.
#[test]
fn die_takt_referenz_weckt_und_wirft_nie() {
    let nur_referenz = mit_takt_referenz(
        &Apl {
            name: "leer".to_owned(),
            rules: Vec::new(),
        },
        0,
        20_000,
    );
    assert!(
        kindle_im_fenster(BOSS_12000, &nur_referenz, 0, 60_000).is_empty(),
        "die Referenzzeile hat selbst gewirkt"
    );
    let gekreuzt: Apl = parse(
        AplName("probe"),
        "smoulder if time == 0\nkindle if time > dot.smoulder.remains\n",
    )
    .expect("gültige Liste");
    let ereignis = kindle_im_fenster("first-crucible", &gekreuzt, 5500, 9000);
    let referenz = kindle_im_fenster(
        "first-crucible",
        &mit_takt_referenz(&gekreuzt, 5500, 9000),
        5500,
        9000,
    );
    assert_eq!(
        referenz.first().copied(),
        Some(6010),
        "die Referenz muss den Takt nach dem Kreuzungspunkt treffen"
    );
    assert!(
        ereignis.first() > referenz.first(),
        "ereignisgesteuert {ereignis:?}, Referenz {referenz:?}: Ohne Weckruf darf \
         der Simulator nicht früher werfen als die Referenz — und nicht gleich \
         früh, sonst weckt die Referenz nichts, was er nicht ohnehin sähe"
    );
}

/// `boss.next_strike_in`: Schlag bei 12000, Schwelle 3000, Umschlag 9000.
///
/// `purge` steht vorn und wird bei 0 und 7500 gewirkt — sein GCD endet genau
/// am Umschlag. Die Rotation entscheidet dort also ohnehin, und gemessen wird
/// allein, ob eine Form den Takt DANACH noch bekommt.
///
/// Verspätet sind `v < 3000`, sein Spiegel `3000 > v` und `!=` in beiden
/// Reihenfolgen: Sie sind am Umschlag falsch und kippen einen Takt später,
/// und für diesen Takt stellt `switch_point` keinen Weckruf. Der nächste
/// Anlass ist der Hitzeverfall bei 10000.
#[test]
fn alle_formen_an_boss_next_strike_in() {
    pruefe_alle_formen(
        BOSS_12000,
        "purge\n",
        "boss.next_strike_in",
        3000,
        9000,
        &[
            ("<", false, Some(10_000), Some(9010)),
            ("<", true, None, None),
            ("<=", false, Some(9000), Some(9000)),
            ("<=", true, Some(9000), Some(9000)),
            (">", false, None, None),
            (">", true, Some(10_000), Some(9010)),
            (">=", false, Some(9000), Some(9000)),
            (">=", true, Some(9000), Some(9000)),
            ("==", false, Some(9000), Some(9000)),
            ("==", true, Some(9000), Some(9000)),
            ("!=", false, Some(10_000), Some(9010)),
            ("!=", true, Some(10_000), Some(9010)),
        ],
    );
}

/// `dot.smoulder.remains` — der zweite Wert, und der der Suche: Die
/// Nachlege-Kandidaten lauten `dot.<brand>.remains < 1500` (`GCD_MS`).
///
/// SMOULDER bei 0 läuft bis 12000; die Restzeit erreicht 1500 bei 10500. Dort
/// endet der GCD von `purge` (gewirkt bei 1500 und 9000). Dasselbe Muster wie
/// oben: `<`, sein Spiegel und `!=` verspätet — hier bis zum letzten Tick bei
/// 12000, weil zwischen 10500 und 12000 nichts in der Schlange steht.
///
/// **Diese Tabelle misst nur den Takt NACH dem Umschlag, nicht den Weckruf.**
/// Am Umschlag weckt der GCD von `purge` ohnehin; die Zeilen `<=`, `==` und
/// `>=` treffen deshalb auch dann, wenn `switch_point` für `DotRemains` gar
/// keinen Weckruf stellt. Gemessen am 2026-10-24: Mit `Value::DotRemains(_) =>
/// None` fällt in der ganzen Suite allein
/// `der_weckruf_von_dot_remains_traegt_ohne_fremdes_ereignis`, der den Weckruf
/// selbst prüft; alle zwölf Zeilen dieser Tabelle bleiben grün.
#[test]
fn alle_formen_an_dot_remains() {
    pruefe_alle_formen(
        "first-crucible",
        "smoulder if time == 0\npurge\n",
        "dot.smoulder.remains",
        1500,
        10_500,
        &[
            ("<", false, Some(12_000), Some(10_510)),
            ("<", true, None, None),
            ("<=", false, Some(10_500), Some(10_500)),
            ("<=", true, Some(10_500), Some(10_500)),
            (">", false, None, None),
            (">", true, Some(12_000), Some(10_510)),
            (">=", false, Some(10_500), Some(10_500)),
            (">=", true, Some(10_500), Some(10_500)),
            ("==", false, Some(10_500), Some(10_500)),
            ("==", true, Some(10_500), Some(10_500)),
            ("!=", false, Some(12_000), Some(10_510)),
            ("!=", true, Some(12_000), Some(10_510)),
        ],
    );
}

/// `time` — der steigende Wert, und deshalb mit gespiegeltem Befund.
///
/// Auf der Uhr kippt `time > 9000` erst nach 9000, `time >= 9000` genau dort.
/// Verspätet sind deshalb `>` und sein Spiegel `9000 < time` — nicht `<` wie
/// bei den fallenden Restzeiten — sowie `!=` in beiden Reihenfolgen.
///
/// **Auch diese Tabelle misst nur den Takt NACH dem Umschlag, nicht den
/// Weckruf** — wie die an `dot.smoulder.remains`. Am Umschlag 9000 endet der
/// GCD von `purge` (gewirkt bei 0 und 7500); entschieden wird dort ohnehin.
/// Gemessen am 2026-10-24 mit `Value::Time => None` in `switch_point`: In
/// denselben Aufbauten (`e1027-operator-measure proben`) bleibt die
/// Ereignisspalte in allen zwölf Zeilen gleich. Rot wird die Tabelle trotzdem,
/// bei `9000 < time` — aber in der Referenzspalte (10000 statt 9010), weil die
/// Takt-Referenz selbst an `time >= t` hängt. Den Weckruf von `time` halten
/// `die_takt_referenz_weckt_und_wirft_nie` (`Some(8000)` statt `Some(6010)`)
/// und, außerhalb dieser Datei, `zeitabhaengige_bedingungen_wecken_den_simulator`
/// (`apl.rs`) und `ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster`
/// (`apl_boss_cast.rs`) fest; dazu fallen unter der Mutation alle drei
/// Formtabellen über ihre Referenzspalte.
#[test]
fn alle_formen_an_time() {
    pruefe_alle_formen(
        BOSS_12000,
        "purge\n",
        "time",
        9000,
        9000,
        &[
            ("<", false, None, None),
            ("<", true, Some(10_000), Some(9010)),
            ("<=", false, Some(9000), Some(9000)),
            ("<=", true, Some(9000), Some(9000)),
            (">", false, Some(10_000), Some(9010)),
            (">", true, None, None),
            (">=", false, Some(9000), Some(9000)),
            (">=", true, Some(9000), Some(9000)),
            ("==", false, Some(9000), Some(9000)),
            ("==", true, Some(9000), Some(9000)),
            ("!=", false, Some(10_000), Some(9010)),
            ("!=", true, Some(10_000), Some(9010)),
        ],
    );
}

/// **Der Weckruf von `boss.next_strike_in` trägt — ohne fremdes Ereignis.**
///
/// Das ist der Fall, den die Mutation vom 2026-08-22
/// (`Value::BossNextStrikeIn => None`) nicht treffen konnte: In den Aufbauten
/// oben endet der GCD von `purge` bei 9000, bei `boss-1-2` tickt dort das
/// Verhängnis. Beides weckt ohnehin. Ohne `purge` steht bei 9000 nichts in der
/// Schlange — gemessen am 2026-10-24: Mit der Mutation wirft diese Zeile erst
/// beim Hitzeverfall bei 10000, ohne sie bei 9000.
#[test]
fn der_weckruf_traegt_ohne_fremdes_ereignis() {
    let apl: Apl =
        parse(AplName("probe"), "kindle if boss.next_strike_in <= 3000\n").expect("gültige Liste");
    let ereignis = kindle_im_fenster(BOSS_12000, &apl, 8500, 12_000);
    let referenz = kindle_im_fenster(
        BOSS_12000,
        &mit_takt_referenz(&apl, 8500, 12_000),
        8500,
        12_000,
    );
    assert_eq!(
        ereignis.first().copied(),
        Some(9000),
        "`<= 3000` muss am Umschlag werfen — wirft es erst bei 10000, fehlt \
         `BossNextStrikeIn` in `switch_point`"
    );
    assert_eq!(ereignis, referenz, "Ereignis und Takt-Referenz weichen ab");
}

/// **Der Weckruf von `dot.*.remains` trägt — ohne fremdes Ereignis.**
///
/// Das Gegenstück zum Wächter darüber, für den zweiten Wert und den der Suche
/// (`dot.<brand>.remains < 1500`). `alle_formen_an_dot_remains` kann ihn nicht
/// prüfen: Dort endet der GCD von `purge` genau am Umschlag 10500.
///
/// Ohne `purge` wird `<= 1500` bei 10500 wahr, und dort weckt nur der Weckruf
/// aus `switch_point`. Gemessen am 2026-10-24: Mit `Value::DotRemains(_) =>
/// None` legt diese Zeile erst beim Ablauf bei 12000 nach — zwischen 10500 und
/// 12000 weckt also nichts anderes —, ohne die Mutation bei 10500, unter
/// Fassung B (#1027) ebenfalls bei 10500.
#[test]
fn der_weckruf_von_dot_remains_traegt_ohne_fremdes_ereignis() {
    let apl: Apl = parse(
        AplName("probe"),
        "smoulder if dot.smoulder.remains <= 1500\n",
    )
    .expect("gültige Liste");
    let ereignis = wuerfe_im_fenster("first-crucible", &apl, "smoulder", 9000, 12_500);
    let referenz = wuerfe_im_fenster(
        "first-crucible",
        &mit_takt_referenz(&apl, 9000, 12_500),
        "smoulder",
        9000,
        12_500,
    );
    assert_eq!(
        ereignis.first().copied(),
        Some(10_500),
        "`<= 1500` muss am Umschlag 10500 nachlegen. Legt es erst beim Ablauf \
         (12000) nach, trägt der Weckruf von `dot.*.remains` nicht — so gemessen \
         mit `Value::DotRemains(_) => None` in `switch_point`"
    );
    assert_eq!(ereignis, referenz, "Ereignis und Takt-Referenz weichen ab");
}
