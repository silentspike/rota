//! Die Masterplan-Ausgaben 2 bis 4 über alle wertungsfähigen Bosse
//! (#891 N1, N4), als Kommandozeilen-Werkzeug.
//!
//! ```text
//! cargo run --release --example report
//! cargo run --release --example report -- --json
//! cargo run --release --example report -- --budget 500000
//! cargo run --release --example report -- --posten-md posten.md
//! ```
//!
//! Beendet sich mit 1, wenn für eine Kombination aus Boss und Stufe ein Erfolg
//! ohne gefundene Lösung bleibt — dann gilt der Boss als **nicht
//! auslieferbar** (Masterplan, „Prüflauf vor Auslieferung"). Genau dafür hängt
//! er im nächtlichen Lauf.
//!
//! # Was je Kombination gerechnet wird
//!
//! Der Prüfplan ist [`rota_sim::auslieferung::pruefplan`]: jeder wertungsfähige
//! Boss auf jeder Stufe, die das Spiel kennt (heute Normal). Je Kombination
//! sucht die gierige Suche die Rotation, und auf ihr stehen alle drei
//! Abschnitte: der Zauber-Vergleich mit vier Befunden je Zauber (Issue #946),
//! die Machbarkeit nach der Siegbedingung des Bosses und der Erfolgs-Prüflauf.
//! Bis #891 N1 rechneten Abschnitt 2 und 3 nur first-crucible, mit der festen
//! Liste `step1-good`.
//!
//! **Das Budget** begrenzt jede Projektion auf
//! [`rota_sim::auslieferung::BUDGET`] Listen. Was darüber liegt, wird nicht
//! gerechnet und steht als „nicht gefunden" da — mit der Zahl, die es gebraucht
//! hätte. `--budget N` setzt eine andere Grenze.
//!
//! # Wanduhr und Kampfzahl je Posten (#891 N4)
//!
//! Jede Kombination wird in fünf Teilen gemessen
//! ([`rota_sim::auslieferung::Teil`]). Die Tabelle steht am Ende der
//! Textausgabe; `--posten-md DATEI` schreibt sie zusätzlich als Markdown, für
//! die Zusammenfassung des Nachtlaufs. Die Datei entsteht auch dann, wenn ein
//! Boss nicht auslieferbar ist — gerade dann wird sie gelesen.

use std::fmt::Write as _;
use std::process::ExitCode;
use std::time::Instant;

use rota_sim::auslieferung::{
    alle_auslieferbar, nicht_auslieferbar, pruefe, pruefplan, summe_je_teil, Posten, Pruefung,
    Rahmen, Stufe, Teil, BUDGET,
};
use rota_sim::objective::{Coverage, WinCondition};
use rota_sim::report::{meaningless, time_tiers};
use rota_sim::viabilitaet::{portfolio, Befundart, EINORDNUNG};
use rota_sim::SPELLS;

/// Was die Kommandozeile setzt.
struct Optionen {
    json: bool,
    budget: u64,
    posten_md: Option<String>,
}

fn wert_nach(args: &[String], schalter: &str) -> Option<String> {
    args.iter().position(|a| a == schalter).map(|i| {
        args.get(i + 1)
            .unwrap_or_else(|| panic!("{schalter} erwartet einen Wert"))
            .clone()
    })
}

fn optionen() -> Optionen {
    let args: Vec<String> = std::env::args().collect();
    Optionen {
        json: args.iter().any(|a| a == "--json"),
        budget: wert_nach(&args, "--budget").map_or(BUDGET, |wert| {
            wert.parse()
                .unwrap_or_else(|_| panic!("--budget erwartet eine Zahl, nicht {wert:?}"))
        }),
        posten_md: wert_nach(&args, "--posten-md"),
    }
}

/// Rechnet den ganzen Prüfplan, eine Kombination nach der anderen.
///
/// Der Fortschritt geht nach stderr: stdout bleibt bei `--json` ein einziges
/// Objekt.
fn lauf(rahmen: &Rahmen) -> (Vec<Pruefung>, f64) {
    let plan = pruefplan();
    let beginn = Instant::now();
    let mut ergebnisse = Vec::with_capacity(plan.len());
    for (nummer, kombination) in plan.into_iter().enumerate() {
        let p = pruefe(kombination, rahmen);
        eprintln!(
            "[{:>2}] {:<15} {:<7} {:>8.2} s  {:>10} Kaempfe  {}",
            nummer + 1,
            kombination.boss.id,
            kombination.stufe.id(),
            p.wanduhr().as_secs_f64(),
            p.kaempfe(),
            if p.auslieferbar() {
                "auslieferbar"
            } else {
                "NICHT auslieferbar"
            }
        );
        ergebnisse.push(p);
    }
    (ergebnisse, beginn.elapsed().as_secs_f64())
}

fn main() -> ExitCode {
    let opt = optionen();
    let rahmen = Rahmen::voll(opt.budget);
    let (ergebnisse, sekunden) = lauf(&rahmen);
    let frei = alle_auslieferbar(&ergebnisse);

    if let Some(pfad) = &opt.posten_md {
        std::fs::write(pfad, posten_markdown(&ergebnisse, sekunden))
            .unwrap_or_else(|e| panic!("{pfad} laesst sich nicht schreiben: {e}"));
    }
    if opt.json {
        println!("{}", json_lauf(&ergebnisse, &rahmen, sekunden, frei));
    } else {
        for zeile in text_lauf(&ergebnisse, &rahmen, sekunden, frei) {
            println!("{zeile}");
        }
    }

    if frei {
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

fn ziel_text(ziel: WinCondition) -> &'static str {
    match ziel {
        WinCondition::RingDepleted => "Ring auf null",
        WinCondition::SurviveTimer => "Frist ueberstehen (umgekehrt)",
    }
}

// ------------------------------------------------------------------- Text

/// Abschnitt 1 einer Kombination als Zeilen — dieselben, die gedruckt werden.
fn text_vergleich(p: &Pruefung) -> Vec<String> {
    let t = &p.tabelle;
    let befunde = p
        .posten
        .iter()
        .find(|q| q.teil == Teil::Befunde)
        .map_or(0.0, |q| q.wanduhr.as_secs_f64());
    let mut out = vec![
        "1. ZAUBER-VERGLEICH — vier Befunde je Zauber (Issue #946)".to_owned(),
        format!(
            "   Boss \"{}\", gesuchte Rotation (gierig, Laenge bis {}), 1 Durchgang, Startwert 0,",
            t.boss, t.max_len
        ),
        format!(
            "   Budget {} Listen je Projektion, Befunde gerechnet in {befunde:.1} s",
            t.budget
        ),
        format!("   {EINORDNUNG}"),
        format!("   Elite ({:.1}):", t.elite.best.mean),
    ];
    for rule in &t.elite.best.apl.rules {
        out.push(format!("     {rule}"));
    }
    out.push(String::new());

    let gesamt = f64::from(p.elite_lauf.damage);
    out.push(format!(
        "   {:<14} {:<13} {:>5} {:>8} {:>7}  Befund",
        "Zauber", "Schule", "Wirkt", "Schaden", "Anteil"
    ));
    let mut zeilen: Vec<_> = t.zeilen.iter().collect();
    // Nach Befund, darin nach Schaden — der stabile Sort laesst gleiche in der
    // Reihenfolge der Daten.
    zeilen.sort_by(|a, b| {
        a.befund.art().cmp(&b.befund.art()).then_with(|| {
            p.elite_lauf.damage_by_spell[b.spell].cmp(&p.elite_lauf.damage_by_spell[a.spell])
        })
    });
    for zeile in zeilen {
        let schaden = p.elite_lauf.damage_by_spell[zeile.spell];
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
    let menge = portfolio(&p.schulen, &p.mechaniken);
    out.push(format!(
        "     every_spell_has_viable_niche                   {}",
        coverage_text(kampagne.every_spell_has_viable_niche)
    ));
    out.push(format!(
        "     no_school_is_globally_mandatory                {}",
        coverage_text(menge.no_school_is_globally_mandatory)
    ));
    for probe in &p.schulen {
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
    match &p.mechaniken {
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
        "     robustness_across_encounters                   {} (braucht die Kreuzprobe \
         ueber die Begegnungen)",
        coverage_text(menge.robustness_across_encounters)
    ));

    out.push(String::new());
    for zeile in t.zusammenfassung().iter().skip(Befundart::ALLE.len() + 1) {
        out.push(format!("   {zeile}"));
    }
    out
}

/// Abschnitt 2 und 3 einer Kombination.
fn text_machbarkeit(p: &Pruefung) -> Vec<String> {
    let f = &p.machbarkeit;
    let mut out = vec![
        "2. BOSS-MACHBARKEIT (gesuchte Rotation, Elite aus Abschnitt 1)".to_owned(),
        format!("   Ziel                 {}", ziel_text(f.ziel)),
        format!("   Zeitlimit            {:>8} ms", f.limit_ms),
    ];
    match (f.gewonnen, f.ziel) {
        (true, WinCondition::RingDepleted) => {
            out.push(format!(
                "   besiegt bei          {:>8} ms",
                f.kill_time_ms.unwrap_or_default()
            ));
            out.push(format!(
                "   Reserve              {:>8} ms   ({:.1} % des Zeitlimits)",
                f.reserve_ms,
                f.reserve_share * 100.0
            ));
        }
        (true, WinCondition::SurviveTimer) => {
            out.push(format!("   Frist ueberstanden   {:>8} ms", f.limit_ms));
            out.push(format!(
                "   Reserve              {:>8} von {} Ringpunkten ({:.1} %)",
                f.ring_rest,
                f.hp,
                f.reserve_share * 100.0
            ));
        }
        (false, _) => {
            out.push(format!("   NICHT gewonnen       Ausgang {:?}", f.ausgang));
            if let Some(t) = f.kill_time_ms {
                out.push(format!("   Ring auf null bei    {t:>8} ms"));
            }
            if let Some(t) = f.died_at_ms {
                out.push(format!("   Spieler tot bei      {t:>8} ms"));
            }
            out.push(format!(
                "   abgetragen           {:>8}      ({:.1} % der Lebenspunkte)",
                f.damage,
                f.hp_share * 100.0
            ));
        }
    }

    out.push(String::new());
    out.push("3. ERFOLGS-PRUEFLAUF".to_owned());
    if let Some(best) = f.sieg_ms {
        let t = time_tiers(best);
        out.push(format!(
            "   Zeitmarken aus der Bestzeit (GAME-DESIGN 21.6): Gold {} ms, Silber {} ms, Bronze {} ms",
            t.gold_ms, t.silver_ms, t.bronze_ms
        ));
    }
    for check in &p.erfolge {
        out.push(format!(
            "   {:<20} {}   {}",
            check.name,
            if check.solved { "geloest " } else { "OFFEN   " },
            check.evidence
        ));
    }
    let ohne_wirkung = meaningless(&p.erfolge);
    if !ohne_wirkung.is_empty() {
        let namen: Vec<&str> = ohne_wirkung.iter().map(|c| c.name.as_str()).collect();
        out.push(format!(
            "   HINWEIS: loesbar, unterscheidet aber niemanden (Marke ueber dem Zeitlimit): {}",
            namen.join(", ")
        ));
    }
    out.push(format!(
        "   -> {}",
        if p.auslieferbar() {
            "auslieferbar"
        } else {
            "NICHT AUSLIEFERBAR"
        }
    ));
    out
}

/// Eine Zeile der Übersicht über alle Kombinationen.
fn text_uebersicht_zeile(p: &Pruefung) -> String {
    let f = &p.machbarkeit;
    let reserve = match (f.gewonnen, f.ziel) {
        (true, WinCondition::RingDepleted) => format!("{:>5.1} % Zeit", f.reserve_share * 100.0),
        (true, WinCondition::SurviveTimer) => format!("{:>5.1} % Ring", f.reserve_share * 100.0),
        (false, _) => "-".to_owned(),
    };
    let marken: String = p
        .erfolge
        .iter()
        .filter(|c| c.name.starts_with("Zeitmarke"))
        .map(|c| if c.meaningful { 'x' } else { '-' })
        .collect();
    let z = p.tabelle.zaehlung();
    // Abgeleitetes `Debug` einer Variante ohne Felder schreibt ohne Auffüllung
    // — die Spalte braucht deshalb erst die Zeichenkette.
    let ausgang = format!("{:?}", f.ausgang);
    format!(
        "   {:<15} {:<7} {:<6} {:<13} {:>8} {:>13}  {:<4} {:>3}/{:>3}/{:>3}/{:>3}  {}",
        p.kombination.boss.id,
        p.kombination.stufe.id(),
        if f.ziel.is_inverted() {
            "umgek."
        } else {
            "Ring"
        },
        ausgang,
        f.sieg_ms.map_or_else(|| "-".to_owned(), |t| t.to_string()),
        reserve,
        if marken.is_empty() {
            "-".to_owned()
        } else {
            marken
        },
        z.von(Befundart::MindestensBezeugt),
        z.von(Befundart::BewiesenDominiert),
        z.von(Befundart::NichtGefunden),
        z.von(Befundart::Ungeklaert),
        if p.auslieferbar() { "ja" } else { "NEIN" }
    )
}

/// Die Posten-Tabelle (#891 N4) als Zeilen.
fn text_posten(ergebnisse: &[Pruefung], sekunden: f64) -> Vec<String> {
    let mut out = vec![
        "POSTEN (#891 N4) — Wanduhr und Kampfzahl je Teil".to_owned(),
        format!(
            "   {:<15} {:<7} {:<15} {:>10} {:>12} {:>12} {:>12}",
            "Boss", "Stufe", "Teil", "Wanduhr s", "Kaempfe", "Durchgaenge", "Auswertungen"
        ),
    ];
    let zeile = |boss: &str, stufe: &str, q: &Posten| {
        format!(
            "   {:<15} {:<7} {:<15} {:>10.3} {:>12} {:>12} {:>12}",
            boss,
            stufe,
            q.teil.name(),
            q.wanduhr.as_secs_f64(),
            q.kaempfe,
            q.durchgaenge,
            q.auswertungen
        )
    };
    for p in ergebnisse {
        for q in &p.posten {
            out.push(zeile(p.kombination.boss.id, p.kombination.stufe.id(), q));
        }
    }
    out.push(String::new());
    let summen = summe_je_teil(ergebnisse);
    let gesamt_s: f64 = summen.iter().map(|q| q.wanduhr.as_secs_f64()).sum();
    for q in &summen {
        out.push(format!(
            "{}   ({:.1} % der Wanduhr)",
            zeile("Summe", "", q),
            if gesamt_s > 0.0 {
                q.wanduhr.as_secs_f64() / gesamt_s * 100.0
            } else {
                0.0
            }
        ));
    }
    out.push(format!(
        "   Gesamt: {} Kombinationen, {:.1} s Wanduhr ueber die Posten, {:.1} s fuer den ganzen \
         Lauf, {} Kaempfe",
        ergebnisse.len(),
        gesamt_s,
        sekunden,
        summen.iter().map(|q| q.kaempfe).sum::<u64>()
    ));
    out.push(stapel_zeile(ergebnisse));
    out
}

/// Die Projektionen aller Kombinationen als Stapel — für den Auslöser von
/// #971 („Stapel ab 50 000 Kandidaten") die Größe des größten, nicht nur die
/// Summe.
fn stapel_zeile(ergebnisse: &[Pruefung]) -> String {
    let mut projektionen = 0;
    let mut listen = 0;
    let mut groesste: (u64, &str) = (0, "-");
    for p in ergebnisse {
        let s = p.stapel();
        projektionen += s.projektionen;
        listen += s.listen;
        if s.groesste > groesste.0 {
            groesste = (s.groesste, p.kombination.boss.id);
        }
    }
    format!(
        "   Befunde: {projektionen} erschoepfende Projektionen mit zusammen {listen} Listen, \
         die groesste {} Listen ({})",
        groesste.0, groesste.1
    )
}

fn text_lauf(ergebnisse: &[Pruefung], rahmen: &Rahmen, sekunden: f64, frei: bool) -> Vec<String> {
    let bosse = ergebnisse
        .iter()
        .filter(|p| p.kombination.stufe == Stufe::ALLE[0])
        .count();
    let mut out = vec![
        "=".repeat(74),
        "ROTA — Masterplan-Ausgaben 2 bis 4, Pruefllauf vor Auslieferung (#891 N1)".to_owned(),
        format!(
            "{} Kombinationen: {bosse} wertungsfaehige Bosse x {} Stufe(n) ({})",
            ergebnisse.len(),
            Stufe::ALLE.len(),
            Stufe::ALLE.map(Stufe::name).join(", ")
        ),
        format!(
            "Rotation je Kombination gesucht: gierig, Laenge bis {}, {} Kandidatenzeilen, \
             ohne Ausruestung",
            rahmen.max_len,
            rahmen.pool.len()
        ),
        "=".repeat(74),
    ];
    for p in ergebnisse {
        out.push(String::new());
        out.push(format!(
            "##### {} \"{}\" ({}), Stufe {} #####",
            p.kombination.boss.id,
            p.kombination.boss.name,
            p.kombination.boss.instance,
            p.kombination.stufe.name()
        ));
        out.push(String::new());
        out.extend(text_vergleich(p));
        out.push(String::new());
        out.extend(text_machbarkeit(p));
    }

    out.push(String::new());
    out.push("=".repeat(74));
    out.push("UEBERSICHT".to_owned());
    out.push(format!(
        "   {:<15} {:<7} {:<6} {:<13} {:>8} {:>13}  {:<4} {:>15}  {}",
        "Boss", "Stufe", "Ziel", "Ausgang", "Sieg ms", "Reserve", "GSB", "bez/dom/nf/ung", "frei"
    ));
    for p in ergebnisse {
        out.push(text_uebersicht_zeile(p));
    }
    out.push(
        "   GSB: je Zeitmarke x = unterscheidet (Marke unter dem Zeitlimit), - = nicht".to_owned(),
    );

    out.push(String::new());
    out.extend(text_posten(ergebnisse, sekunden));

    out.push(String::new());
    out.push("-".repeat(74));
    let gesperrt: Vec<String> = nicht_auslieferbar(ergebnisse)
        .iter()
        .map(|p| format!("{} ({})", p.kombination.boss.id, p.kombination.stufe.id()))
        .collect();
    if frei {
        out.push(format!(
            "AUSLIEFERBAR: Fuer jeden Erfolg aller {} Kombinationen existiert eine Loesung.",
            ergebnisse.len()
        ));
    } else {
        out.push(format!(
            "NICHT AUSLIEFERBAR: {} von {} Kombinationen ohne Loesung fuer mindestens einen \
             Erfolg: {}",
            gesperrt.len(),
            ergebnisse.len(),
            if gesperrt.is_empty() {
                "keine Kombination geprueft".to_owned()
            } else {
                gesperrt.join(", ")
            }
        ));
    }
    out.push("-".repeat(74));

    out.push(String::new());
    out.push(
        "Was hier NOCH NICHT geprueft wird, weil es das Spiel noch nicht hat (#1177):".to_owned(),
    );
    out.push(
        "Schwierigkeitsstufen ueber Normal, boss-eigene Erfolge und von Hand gesetzte".to_owned(),
    );
    out.push(
        "Zeitmarken. Die abgeleiteten Marken sind per Bauart erreichbar, sobald der Boss"
            .to_owned(),
    );
    out.push("faellt; sie pruefen den Sieg und ob die Marke unter dem Zeitlimit liegt.".to_owned());
    out.push(
        "Ein Befund im Zauber-Vergleich gilt fuer seinen Boss: eine Aussage in seinem".to_owned(),
    );
    out.push("Geltungsbereich, kein Urteil ueber den Zauber im Spiel.".to_owned());
    out
}

// ------------------------------------------------------------------- Posten als Markdown

/// Die Posten-Tabelle für die Zusammenfassung des Nachtlaufs (#891 N4).
fn posten_markdown(ergebnisse: &[Pruefung], sekunden: f64) -> String {
    let mut md = String::new();
    let summen = summe_je_teil(ergebnisse);
    let gesamt_s: f64 = summen.iter().map(|q| q.wanduhr.as_secs_f64()).sum();
    let kaempfe: u64 = summen.iter().map(|q| q.kaempfe).sum();
    let gesperrt = nicht_auslieferbar(ergebnisse);
    let _ = writeln!(
        md,
        "### Masterplan-Lauf (report): Wanduhr und Kampfzahl je Posten (#891 N4)\n"
    );
    let _ = writeln!(
        md,
        "{} Kombinationen, {:.1} s fuer den ganzen Lauf, {} Kaempfe, {} nicht auslieferbar{}.\n",
        ergebnisse.len(),
        sekunden,
        kaempfe,
        gesperrt.len(),
        if gesperrt.is_empty() {
            String::new()
        } else {
            format!(
                " ({})",
                gesperrt
                    .iter()
                    .map(|p| p.kombination.boss.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    );
    let _ = writeln!(md, "{}\n", stapel_zeile(ergebnisse).trim());
    let _ = writeln!(
        md,
        "| Teil | Wanduhr s | Anteil | Kaempfe | Durchgaenge | Auswertungen |"
    );
    let _ = writeln!(md, "| --- | ---: | ---: | ---: | ---: | ---: |");
    for q in &summen {
        let _ = writeln!(
            md,
            "| {} | {:.3} | {:.1} % | {} | {} | {} |",
            q.teil.name(),
            q.wanduhr.as_secs_f64(),
            if gesamt_s > 0.0 {
                q.wanduhr.as_secs_f64() / gesamt_s * 100.0
            } else {
                0.0
            },
            q.kaempfe,
            q.durchgaenge,
            q.auswertungen
        );
    }
    let _ = writeln!(md, "\n<details><summary>Je Boss und Teil</summary>\n");
    let _ = writeln!(
        md,
        "| Boss | Stufe | Teil | Wanduhr s | Kaempfe | Durchgaenge | Auswertungen | frei |"
    );
    let _ = writeln!(md, "| --- | --- | --- | ---: | ---: | ---: | ---: | --- |");
    for p in ergebnisse {
        for q in &p.posten {
            let _ = writeln!(
                md,
                "| {} | {} | {} | {:.3} | {} | {} | {} | {} |",
                p.kombination.boss.id,
                p.kombination.stufe.id(),
                q.teil.name(),
                q.wanduhr.as_secs_f64(),
                q.kaempfe,
                q.durchgaenge,
                q.auswertungen,
                if p.auslieferbar() { "ja" } else { "NEIN" }
            );
        }
    }
    let _ = writeln!(md, "\n</details>");
    md
}

// ------------------------------------------------------------------- JSON

/// Abschnitt 1 einer Kombination als JSON-Objekt.
fn json_vergleich(p: &Pruefung) -> String {
    let t = &p.tabelle;
    let zeilen: Vec<String> = t
        .zeilen
        .iter()
        .map(|z| {
            format!(
                "{{\"id\":{:?},\"school\":{:?},\"casts\":{},\"damage\":{},\"befund\":{:?},\"geltungsbereich\":{},\"beleg\":{:?},\"zuSchwach\":{}}}",
                z.id,
                SPELLS[z.spell].school.unwrap_or(""),
                z.casts,
                p.elite_lauf.damage_by_spell[z.spell],
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
    let menge = portfolio(&p.schulen, &p.mechaniken);
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

fn json_posten(q: &Posten) -> String {
    format!(
        "{{\"teil\":{:?},\"wanduhrS\":{:?},\"kaempfe\":{},\"durchgaenge\":{},\"auswertungen\":{}}}",
        q.teil.name(),
        q.wanduhr.as_secs_f64(),
        q.kaempfe,
        q.durchgaenge,
        q.auswertungen
    )
}

fn json_kombination(p: &Pruefung) -> String {
    let f = &p.machbarkeit;
    let achievements: Vec<String> = p
        .erfolge
        .iter()
        .map(|c| {
            format!(
                "{{\"name\":{:?},\"solved\":{},\"meaningful\":{},\"evidence\":{:?}}}",
                c.name, c.solved, c.meaningful, c.evidence
            )
        })
        .collect();
    let posten: Vec<String> = p.posten.iter().map(json_posten).collect();
    let stapel = p.stapel();
    let zahl = |t: Option<u32>| t.map_or(-1_i64, i64::from);
    format!(
        "{{\"boss\":{:?},\"stufe\":{:?},\"zauberVergleich\":{},\
         \"feasibility\":{{\"ziel\":\"{:?}\",\"ausgang\":\"{:?}\",\"won\":{},\"limitMs\":{},\"killTimeMs\":{},\
         \"winTimeMs\":{},\"reserveMs\":{},\"reserveShare\":{:?},\"ringRest\":{},\"hpShare\":{:?}}},\
         \"achievements\":[{}],\"shippable\":{},\"posten\":[{}],\
         \"stapel\":{{\"projektionen\":{},\"listen\":{},\"groesste\":{}}}}}",
        p.kombination.boss.id,
        p.kombination.stufe.id(),
        json_vergleich(p),
        f.ziel,
        f.ausgang,
        f.gewonnen,
        f.limit_ms,
        zahl(f.kill_time_ms),
        zahl(f.sieg_ms),
        f.reserve_ms,
        f.reserve_share,
        f.ring_rest,
        f.hp_share,
        achievements.join(","),
        p.auslieferbar(),
        posten.join(","),
        stapel.projektionen,
        stapel.listen,
        stapel.groesste
    )
}

fn json_lauf(ergebnisse: &[Pruefung], rahmen: &Rahmen, sekunden: f64, frei: bool) -> String {
    let kombinationen: Vec<String> = ergebnisse.iter().map(json_kombination).collect();
    let summen: Vec<String> = summe_je_teil(ergebnisse).iter().map(json_posten).collect();
    format!(
        "{{\"maxLen\":{},\"budget\":{},\"kandidatenzeilen\":{},\"kombinationen\":[{}],\
         \"postenSumme\":[{}],\"wanduhrS\":{:?},\"shippable\":{}}}",
        rahmen.max_len,
        rahmen.budget,
        rahmen.pool.len(),
        kombinationen.join(","),
        summen.join(","),
        sekunden,
        frei
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rota_sim::auslieferung::Kombination;
    use rota_sim::data::{boss_by_id, index_of};
    use rota_sim::rotation_search::{candidates_for, Candidate};

    /// Ein kleiner Lauf mit zwei bewiesen dominierten Zaubern (`crust`,
    /// `fester`; gemessen auf node3 am 2026-10-29) — so klein, dass er im
    /// Debug-Bau in Millisekunden durch ist.
    fn klein() -> Pruefung {
        let ids = ["kindle", "smoulder", "pyre", "crust", "quench", "fester"];
        let grundmenge: Vec<usize> = ids.iter().map(|id| index_of(id).expect("Zauber")).collect();
        let pool: Vec<Candidate> = grundmenge.iter().flat_map(|&i| candidates_for(i)).collect();
        pruefe(
            Kombination {
                boss: boss_by_id("first-crucible").expect("Boss"),
                stufe: Stufe::Normal,
            },
            &Rahmen {
                pool,
                grundmenge,
                max_len: 3,
                budget: 1_000_000,
            },
        )
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

    #[test]
    fn die_posten_stehen_in_text_und_markdown() {
        // **#891 N4: Die Zahl muss im Bericht STEHEN, nicht nur gemessen
        // sein.** Geprüft wird die Ausgabe: Jeder Teil hat seine Zeile, und
        // die Kampfzahl darin ist die gemessene.
        let p = klein();
        let ergebnisse = [p];
        let text = text_posten(&ergebnisse, 1.0);
        let md = posten_markdown(&ergebnisse, 1.0);
        for q in &ergebnisse[0].posten {
            let im_text = text.iter().any(|z| {
                z.contains("first-crucible")
                    && z.contains(q.teil.name())
                    && z.split_whitespace().any(|w| w == q.kaempfe.to_string())
            });
            assert!(
                im_text,
                "{} fehlt im Text:\n{}",
                q.teil.name(),
                text.join("\n")
            );
            let im_md = md.contains(&format!(
                "| first-crucible | normal | {} | {:.3} | {} |",
                q.teil.name(),
                q.wanduhr.as_secs_f64(),
                q.kaempfe
            ));
            assert!(im_md, "{} fehlt im Markdown:\n{md}", q.teil.name());
        }
        let gesamt = ergebnisse[0].kaempfe();
        assert!(gesamt > 0, "der kleine Lauf hat Kaempfe gerechnet");
        assert!(
            md.contains(&format!("{gesamt} Kaempfe")),
            "die Gesamtzahl fehlt im Markdown:\n{md}"
        );
    }
}
