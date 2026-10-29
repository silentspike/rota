//! Die drei Auswertungen aus GAME-DESIGN 18.5, als Kommandozeilen-Werkzeug.
//!
//! ```text
//! cargo run --release --example report
//! cargo run --release --example report -- --json
//! cargo run --release --example report -- --budget 500000
//! ```
//!
//! Beendet sich mit 1, wenn der Erfolgs-Prüflauf keine Lösung findet — dann
//! gilt der Boss als **nicht auslieferbar** (GAME-DESIGN 12.9). Genau dafür
//! hängt er im nächtlichen Lauf.
//!
//! # Der Zauber-Vergleich trennt vier Befunde (Issue #946)
//!
//! Abschnitt 1 nennt je spielbarem Zauber genau einen Befund aus
//! [`rota_sim::viabilitaet`]: mindestens bezeugt, bewiesen dominiert, nicht
//! gefunden, ungeklärt. Bis hierher stand dort „TOT", sobald die feste Liste
//! `step1-good` einen Zauber nicht wirkte — und das hieß dasselbe, ob die
//! Suche ihn nur nicht gefunden hatte oder ob er bewiesen verliert.
//!
//! Gerechnet wird gegen die **gesuchte** Rotation (gierig, Länge bis
//! [`MAX_LEN`]), nicht gegen die feste: Ein Zeuge aus einer Handschrift sagt,
//! was sie wirkt, nicht was die beste Liste wirkt. Die Abschnitte 2 und 3
//! bleiben bei `step1-good`.
//!
//! **Das Budget** begrenzt jede Projektion auf [`BUDGET`] Listen. Was darüber
//! liegt, wird nicht gerechnet und steht als „nicht gefunden" da — mit der
//! Zahl, die es gebraucht hätte. `--budget N` setzt eine andere Grenze.

use std::process::ExitCode;
use std::time::Instant;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::BalanceConfig;
use rota_sim::factors::NEUTRAL;
use rota_sim::objective::Coverage;
use rota_sim::report::{
    achievement_run, feasibility, is_shippable, meaningless, time_tiers, AchievementCheck,
    Feasibility,
};
use rota_sim::rotation_search::{all_candidates, Candidate};
use rota_sim::viabilitaet::{
    befunde, mechanikproben, portfolio, schulproben, spielbare_zauber, Befundart, Befundtabelle,
    Mechanikprobe, Schulprobe, EINORDNUNG,
};
use rota_sim::{Fight, FightConfig, RunResult, BOSS, SPELLS};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");

/// Höchstlänge der gesuchten Liste und der Projektionen — dieselbe wie im
/// Abdeckungs-Wächter aus #550.
const MAX_LEN: usize = 6;

/// Wie viele Listen eine Projektion höchstens messen darf.
///
/// Gemessen am 2026-10-29 auf node3 (CT 105): First-crucible bei Länge 6
/// braucht 53 Projektionen mit zusammen 1 266 939 Listen, die größte hat
/// 79 209; der ganze Abschnitt 1 lief in 15,0 s. Die Grenze liegt so, dass
/// keine Projektion an ihr scheitert, die sich in Sekunden rechnen lässt — sie
/// fängt erst die Projektionen, die mit der Länge der Elite in die Millionen
/// wachsen (neun Zeilen bei Länge 8 sind schon 623 529 Listen,
/// `exhaustive_messungen(9, 8)`).
const BUDGET: u64 = 1_000_000;

/// Die Einstellungen des Zauber-Vergleichs.
///
/// **Ein Durchgang mit festem Startwert**, wie in der erschöpfenden Suche:
/// Ohne Ausrüstung würfelt der Kampf nicht, also ist jede Messung exakt — und
/// nur dann trägt ein Beweis. `stop_on_kill` steht auf `false`, sonst wären
/// zwei verschieden schnelle Listen nicht vergleichbar (`rotation_search::measure`).
fn befund_config() -> BalanceConfig {
    BalanceConfig {
        iterations: 1,
        base_seed: 0,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: BOSS,
        ..BalanceConfig::default()
    }
}

/// Alles, was Abschnitt 1 braucht.
struct Vergleich {
    tabelle: Befundtabelle,
    /// Der Lauf der Elite, für Schaden und Anteil je Zauber.
    elite_lauf: RunResult,
    schulen: Vec<Schulprobe>,
    mechaniken: Result<Vec<Mechanikprobe>, Vec<&'static str>>,
    sekunden: f64,
}

fn vergleich(budget: u64) -> Vergleich {
    vergleich_mit(
        &befund_config(),
        &all_candidates(),
        &spielbare_zauber(),
        MAX_LEN,
        budget,
    )
}

/// Abschnitt 1 für einen beliebigen Vorrat — der Prüffall unten rechnet ihn
/// auf einem kleinen.
fn vergleich_mit(
    config: &BalanceConfig,
    pool: &[Candidate],
    grundmenge: &[usize],
    max_len: usize,
    budget: u64,
) -> Vergleich {
    let beginn = Instant::now();
    let tabelle = befunde(config, pool, grundmenge, max_len, budget);
    let schulen = schulproben(config, pool, grundmenge, max_len);
    let mechaniken = mechanikproben(config, pool, max_len);

    // Schaden je Zauber gibt der Balance-Lauf nicht her, nur der einzelne
    // Kampf. Derselbe Kampf wie im Lauf — und das wird geprüft, nicht
    // angenommen: Wirft er anders, ist die Schadensspalte die eines anderen
    // Kampfs.
    let elite_lauf = Fight::new(FightConfig {
        stop_on_kill: config.stop_on_kill,
        boss: config.boss,
        log_capacity: 0,
        ..FightConfig::default()
    })
    .run_apl(&tabelle.elite.best.apl);
    for zeile in &tabelle.zeilen {
        assert_eq!(
            elite_lauf.casts[zeile.spell], zeile.casts,
            "{} wird im Einzelkampf {}-mal gewirkt, im Lauf {}-mal — die Schadensspalte \
             gehoerte dann zu einem anderen Kampf",
            zeile.id, elite_lauf.casts[zeile.spell], zeile.casts
        );
    }

    Vergleich {
        tabelle,
        elite_lauf,
        schulen,
        mechaniken,
        sekunden: beginn.elapsed().as_secs_f64(),
    }
}

fn budget_aus_argumenten() -> u64 {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--budget")
        .and_then(|i| args.get(i + 1))
        .map_or(BUDGET, |wert| {
            wert.parse()
                .unwrap_or_else(|_| panic!("--budget erwartet eine Zahl, nicht {wert:?}"))
        })
}

fn main() -> ExitCode {
    let json = std::env::args().any(|a| a == "--json");
    let budget = budget_aus_argumenten();

    let apl = match parse(AplName("step1-good"), GOOD) {
        Ok(apl) => apl,
        Err(error) => {
            eprintln!("Die eingebaute Liste ist fehlerhaft:\n{error}");
            return ExitCode::FAILURE;
        }
    };

    let result = Fight::new(FightConfig::default()).run_apl(&apl);
    let feas = feasibility(&apl, None);
    let checks = achievement_run(&apl, None);
    let shippable = is_shippable(&checks);
    let v = vergleich(budget);

    if json {
        print_json(&apl, &result, &v, &feas, &checks, shippable);
    } else {
        print_text(&apl, &v, &feas, &checks, shippable);
    }

    if shippable {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn coverage_text(c: Coverage) -> String {
    match c {
        Coverage::Measured { covered, required } => format!("{covered} von {required}"),
        Coverage::Unmeasured => "ungemessen".to_owned(),
    }
}

/// Abschnitt 1 als Zeilen — dieselben, die gedruckt werden.
fn text_vergleich(v: &Vergleich) -> Vec<String> {
    let t = &v.tabelle;
    let mut out = vec![
        "1. ZAUBER-VERGLEICH — vier Befunde je Zauber (Issue #946)".to_owned(),
        format!(
            "   Boss \"{}\", gesuchte Rotation (gierig, Laenge bis {}), 1 Durchgang, Startwert 0,",
            t.boss, t.max_len
        ),
        format!(
            "   Budget {} Listen je Projektion, gerechnet in {:.1} s",
            t.budget, v.sekunden
        ),
        format!("   {EINORDNUNG}"),
        format!("   Elite ({:.1}):", t.elite.best.mean),
    ];
    for rule in &t.elite.best.apl.rules {
        out.push(format!("     {rule}"));
    }
    out.push(String::new());

    let gesamt = f64::from(v.elite_lauf.damage);
    out.push(format!(
        "   {:<14} {:<13} {:>5} {:>8} {:>7}  Befund",
        "Zauber", "Schule", "Wirkt", "Schaden", "Anteil"
    ));
    let mut zeilen: Vec<_> = t.zeilen.iter().collect();
    // Nach Befund, darin nach Schaden — der stabile Sort laesst gleiche in der
    // Reihenfolge der Daten.
    zeilen.sort_by(|a, b| {
        a.befund.art().cmp(&b.befund.art()).then_with(|| {
            v.elite_lauf.damage_by_spell[b.spell].cmp(&v.elite_lauf.damage_by_spell[a.spell])
        })
    });
    for zeile in zeilen {
        let schaden = v.elite_lauf.damage_by_spell[zeile.spell];
        out.push(format!(
            "   {:<14} {:<13} {:>5} {:>8} {:>5.1} %  {}",
            zeile.id,
            SPELLS[zeile.spell].school.unwrap_or("-"),
            zeile.casts,
            schaden,
            if gesamt > 0.0 {
                f64::from(schaden) / gesamt * 100.0
            } else {
                0.0
            },
            zeile.befund.bezeichnung()
        ));
        out.push(format!("{:>18}Beleg: {}", "", zeile.befund.beleg()));
    }

    out.push(String::new());
    for zeile in t.zusammenfassung().iter().take(Befundart::ALLE.len()) {
        out.push(format!("   {zeile}"));
    }
    let schwach: Vec<&str> = t.zu_schwach().iter().map(|z| z.id).collect();
    out.push(String::new());
    out.push(
        "   Zu schwach — NUR bewiesen dominiert, und nur in der eigenen Projektion".to_owned(),
    );
    out.push(format!(
        "   (Elite-Zeilen + eigene Zeilen, Laenge <= {}, Boss \"{}\"):",
        t.max_len, t.boss
    ));
    out.push(format!(
        "     {}",
        if schwach.is_empty() {
            "keiner".to_owned()
        } else {
            schwach.join(", ")
        }
    ));
    out.push("   \"nicht gefunden\" ist ein Such-Befund und heisst NICHT zu schwach.".to_owned());

    out.push(String::new());
    out.push("   Faecher des Zielvertrags (#939), aus diesem Lauf:".to_owned());
    let kampagne = t.kampagnenabdeckung();
    let menge = portfolio(&v.schulen, &v.mechaniken);
    out.push(format!(
        "     every_spell_has_viable_niche                   {}",
        coverage_text(kampagne.every_spell_has_viable_niche)
    ));
    out.push(format!(
        "     no_school_is_globally_mandatory                {}",
        coverage_text(menge.no_school_is_globally_mandatory)
    ));
    for probe in &v.schulen {
        out.push(format!(
            "       ohne {:<13} {}",
            probe.schule,
            if probe.sieg_ohne.is_some() {
                "Sieg gefunden"
            } else {
                "kein Sieg gefunden"
            }
        ));
    }
    out.push(format!(
        "     every_mechanic_has_multiple_solution_families  {}",
        coverage_text(menge.every_mechanic_has_multiple_solution_families)
    ));
    match &v.mechaniken {
        Ok(liste) if liste.is_empty() => {
            out.push(
                "       der Boss hat keine Mechanik — nichts zu loesen, nichts verletzt".to_owned(),
            );
        }
        Ok(liste) => {
            for probe in liste {
                let wege: Vec<&str> = probe
                    .wege
                    .iter()
                    .filter(|(_, sieg)| sieg.is_some())
                    .map(|(weg, _)| weg.name())
                    .collect();
                out.push(format!(
                    "       {:<18} {} Weg(e): {}",
                    probe.mechanik.name(),
                    probe.loesende(),
                    wege.join(", ")
                ));
            }
        }
        Err(bausteine) => out.push(format!(
            "       nicht gezaehlt — der Boss traegt {}, die sich nicht allein messen lassen",
            bausteine.join(", ")
        )),
    }
    out.push(format!(
        "     robustness_across_encounters                   {} (braucht mehr als einen Boss)",
        coverage_text(menge.robustness_across_encounters)
    ));

    out.push(String::new());
    for zeile in t.zusammenfassung().iter().skip(Befundart::ALLE.len() + 1) {
        out.push(format!("   {zeile}"));
    }
    out
}

fn print_text(
    apl: &Apl,
    v: &Vergleich,
    feas: &Feasibility,
    checks: &[AchievementCheck],
    shippable: bool,
) {
    println!("{}", "=".repeat(74));
    println!("ROTA — Auswertungen (GAME-DESIGN 18.5)");
    println!(
        "Abschnitt 1 gegen die gesuchte Rotation, 2 und 3 gegen \"{}\", Boss \"{}\"",
        apl.name, feas.boss
    );
    println!("{}", "=".repeat(74));

    println!();
    for zeile in text_vergleich(v) {
        println!("{zeile}");
    }

    println!();
    println!("2. BOSS-MACHBARKEIT");
    println!("   Zeitlimit            {:>8} ms", feas.limit_ms);
    if let Some(t) = feas.kill_time_ms {
        println!("   besiegt bei          {t:>8} ms");
        println!(
            "   Reserve              {:>8} ms   ({:.1} % des Zeitlimits)",
            feas.reserve_ms,
            feas.reserve_share * 100.0
        );
    } else {
        println!("   NICHT besiegt");
        println!(
            "   abgetragen           {:>8}      ({:.1} % der Lebenspunkte)",
            feas.damage,
            feas.hp_share * 100.0
        );
    }

    println!();
    println!("3. ERFOLGS-PRUEFLAUF");
    if let Some(best) = feas.kill_time_ms {
        let t = time_tiers(best);
        println!(
            "   Zeitmarken aus der Bestzeit (GAME-DESIGN 21.6): Gold {} ms, Silber {} ms, Bronze {} ms",
            t.gold_ms, t.silver_ms, t.bronze_ms
        );
    }
    for check in checks {
        println!(
            "   {:<20} {}   {}",
            check.name,
            if check.solved { "geloest " } else { "OFFEN   " },
            check.evidence
        );
    }

    let ohne_wirkung = meaningless(checks);
    if !ohne_wirkung.is_empty() {
        println!();
        println!(
            "   HINWEIS: {} Erfolg(e) sind zwar loesbar, unterscheiden aber niemanden —",
            ohne_wirkung.len()
        );
        println!("   ihre Marke liegt ueber dem Zeitlimit und wird von jedem Sieg erfuellt:");
        for check in ohne_wirkung {
            println!("     {}", check.name);
        }
        println!("   Sie sperren die Auslieferung nicht (12.9 verlangt nur eine Loesung),");
        println!("   vergeben aber Punkte fuer nichts.");
    }

    println!();
    println!("{}", "-".repeat(74));
    if shippable {
        println!("AUSLIEFERBAR: Fuer jeden geprueften Erfolg existiert eine Loesung.");
    } else {
        println!("NICHT AUSLIEFERBAR: Mindestens ein Erfolg hat keine Loesung (GAME-DESIGN 12.9).");
    }
    println!("{}", "-".repeat(74));

    println!();
    println!("Was hier NOCH NICHT geprueft wird, weil es das Design noch nicht gibt");
    println!("(GAME-DESIGN 22): Schwierigkeitsstufen (vor E4), boss-eigene Erfolge und");
    println!("die 288 Zeitmarken (vor E10). Geprueft wird ein Boss auf einer Stufe mit");
    println!("den drei Marken, die sich aus der Bestzeit ableiten lassen.");
    println!("Der Zauber-Vergleich gilt fuer diesen einen Boss: Ein Befund ist eine");
    println!("Aussage in seinem Geltungsbereich, kein Urteil ueber den Zauber im Spiel.");
}

/// Abschnitt 1 als JSON-Objekt — dasselbe, das `--json` ausgibt.
fn json_vergleich(v: &Vergleich) -> String {
    let t = &v.tabelle;
    let zeilen: Vec<String> = t
        .zeilen
        .iter()
        .map(|z| {
            format!(
                "{{\"id\":{:?},\"school\":{:?},\"casts\":{},\"damage\":{},\"befund\":{:?},\"geltungsbereich\":{},\"beleg\":{:?},\"zuSchwach\":{}}}",
                z.id,
                SPELLS[z.spell].school.unwrap_or(""),
                z.casts,
                v.elite_lauf.damage_by_spell[z.spell],
                z.befund.bezeichnung(),
                z.befund
                    .geltungsbereich()
                    .map_or_else(|| "null".to_owned(), |g| format!("{g:?}")),
                z.befund.beleg(),
                z.befund.ist_zu_schwach()
            )
        })
        .collect();
    let zaehlung = t.zaehlung();
    let je_art: Vec<String> = Befundart::ALLE
        .iter()
        .map(|&art| format!("{:?}:{}", art.name(), zaehlung.von(art)))
        .collect();
    let zusammenfassung = t.zusammenfassung();
    let lauf = zusammenfassung
        .get(Befundart::ALLE.len() + 1)
        .cloned()
        .unwrap_or_default();
    let vertrag = zusammenfassung
        .get(Befundart::ALLE.len() + 2)
        .cloned()
        .unwrap_or_default();
    let menge = portfolio(&v.schulen, &v.mechaniken);
    let kampagne = t.kampagnenabdeckung();
    format!(
        "{{\"boss\":{:?},\"maxLen\":{},\"budget\":{},\"einordnung\":{:?},\"elite\":{:?},\"eliteWert\":{:?},\
         \"grundmenge\":{},\"befunde\":{{{}}},\"zeilen\":[{}],\"lauf\":{:?},\"vertrag\":{:?},\
         \"faecher\":{{\"everySpellHasViableNiche\":{:?},\"noSchoolIsGloballyMandatory\":{:?},\
         \"everyMechanicHasMultipleSolutionFamilies\":{:?},\"robustnessAcrossEncounters\":{:?}}}}}",
        t.boss,
        t.max_len,
        t.budget,
        EINORDNUNG,
        t.elite.best.apl.to_string(),
        t.elite.best.mean,
        zaehlung.grundmenge,
        je_art.join(","),
        zeilen.join(","),
        lauf,
        vertrag,
        coverage_text(kampagne.every_spell_has_viable_niche),
        coverage_text(menge.no_school_is_globally_mandatory),
        coverage_text(menge.every_mechanic_has_multiple_solution_families),
        coverage_text(menge.robustness_across_encounters),
    )
}

fn print_json(
    apl: &Apl,
    result: &RunResult,
    v: &Vergleich,
    feas: &Feasibility,
    checks: &[AchievementCheck],
    shippable: bool,
) {
    let achievements: Vec<String> = checks
        .iter()
        .map(|c| {
            format!(
                "{{\"name\":{:?},\"solved\":{},\"meaningful\":{},\"evidence\":{:?}}}",
                c.name, c.solved, c.meaningful, c.evidence
            )
        })
        .collect();

    println!(
        "{{\"apl\":{:?},\"boss\":{:?},\"damage\":{},\"zauberVergleich\":{},\
         \"feasibility\":{{\"limitMs\":{},\"defeated\":{},\"killTimeMs\":{},\"reserveMs\":{},\"reserveShare\":{:?},\"hpShare\":{:?}}},\
         \"achievements\":[{}],\"shippable\":{}}}",
        apl.name,
        feas.boss,
        result.damage,
        json_vergleich(v),
        feas.limit_ms,
        feas.defeated,
        feas.kill_time_ms.map_or(-1_i64, i64::from),
        feas.reserve_ms,
        feas.reserve_share,
        feas.hp_share,
        achievements.join(","),
        shippable
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use rota_sim::data::index_of;
    use rota_sim::rotation_search::candidates_for;

    /// Ein kleiner Lauf mit zwei bewiesen dominierten Zaubern (`crust`,
    /// `fester`; gemessen auf node3 am 2026-10-29) — so klein, dass er im
    /// Debug-Bau in Millisekunden durch ist.
    fn klein() -> Vergleich {
        let ids = ["kindle", "smoulder", "pyre", "crust", "quench", "fester"];
        let grundmenge: Vec<usize> = ids.iter().map(|id| index_of(id).expect("Zauber")).collect();
        let pool: Vec<Candidate> = grundmenge.iter().flat_map(|&i| candidates_for(i)).collect();
        vergleich_mit(&befund_config(), &pool, &grundmenge, 3, 1_000_000)
    }

    #[test]
    fn jede_dominiert_zeile_nennt_ihren_geltungsbereich() {
        // Auflage des Orchestrators vom 2026-10-29: Wer nur die Befund-Spalte
        // liest, darf nicht „tot an first-crucible" verstehen. Geprüft wird
        // die AUSGABE, nicht die Bibliothek — Text und JSON, genau wie sie
        // gedruckt werden.
        let v = klein();
        let bereich = "in der Projektion Elite-Zeilen + eigene Zeilen, Laenge <= 3, \
                       ohne Ausruestung, first-crucible";

        let text = text_vergleich(&v);
        // Die Zeile jedes dominierten Zaubers, gesucht über die Tabelle — nicht
        // über das Wort, das geprüft wird.
        let ids: Vec<&str> = v
            .tabelle
            .zeilen
            .iter()
            .filter(|z| z.befund.art() == Befundart::BewiesenDominiert)
            .map(|z| z.id)
            .collect();
        assert!(
            ids.len() >= 2,
            "der kleine Lauf traegt zwei dominierte Zauber: {ids:?}"
        );
        for id in &ids {
            let anfang = format!("   {id:<14} ");
            let zeile = text
                .iter()
                .find(|z| z.starts_with(&anfang))
                .unwrap_or_else(|| panic!("{id} hat keine Zeile im Text"));
            assert!(
                zeile.contains(&format!("bewiesen dominiert — {bereich}")),
                "die Zeile nennt bewiesen dominiert ohne Geltungsbereich:\n{zeile}"
            );
        }
        assert!(
            text.iter().any(|z| z.contains(EINORDNUNG)),
            "der Kopf nennt die Einordnung nicht"
        );

        let json = json_vergleich(&v);
        assert!(json.contains(&format!("\"einordnung\":{EINORDNUNG:?}")));
        let mit_dominiert = json.matches("\"befund\":\"bewiesen dominiert").count();
        let mit_bereich = json
            .matches(&format!(
                "\"befund\":\"bewiesen dominiert — {bereich}\",\"geltungsbereich\":\"{bereich}\""
            ))
            .count();
        assert!(mit_dominiert >= 2, "{json}");
        assert_eq!(
            mit_bereich, mit_dominiert,
            "im JSON steht bewiesen dominiert ohne Geltungsbereich: {json}"
        );
    }
}
