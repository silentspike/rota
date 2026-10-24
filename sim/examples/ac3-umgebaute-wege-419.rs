//! #419 AC3: Ist JEDER der achtzehn Talentwege irgendwo die beste Wahl?
//!
//! ```text
//! cargo remote --no-copy-lock -- run --release --example ac3-umgebaute-wege-419
//! ```
//!
//! # Was AC3 verlangt, und in welcher Lesart
//!
//! „Der Balance-Laeufer belegt, dass KEINER der Wege tot ist — jeder ist in
//! mindestens einer Zusammenstellung die beste Wahl." Dazu zwei Festlegungen
//! aus #419:
//!
//! ```text
//!   2026-08-13  die Zusammenstellung muss SPIELBAR sein: innerhalb 10 % der
//!               besten bekannten Rotation gegen denselben Boss (GAME-DESIGN 20.3)
//!   2026-08-19  gemessen wird gegen die HEUTE beste bekannte Rotation, und wer
//!               misst, nennt sie
//! ```
//!
//! Dieser Laeufer misst nach dieser Lesart. Er schliesst AC3 nicht ab — wann
//! AC3 als erfuellt gilt, ist beim Auftraggeber offen.
//!
//! # Der Aufbau in drei Schritten
//!
//! ```text
//!   1  freie Suche                       -> die heute beste bekannte Rotation
//!   2  Suche MIT dem Traeger als Start   -> liegt sie innerhalb der 10 %?
//!   3  in dieser Rotation                -> gewinnt der Weg gegen seine Nachbarn?
//! ```
//!
//! Erst wenn Schritt 2 durchfaellt, ist „der Weg ist tot" eine Aussage ueber
//! die Sache. Eine Suche von einem vorgegebenen Startpunkt kann SCHLECHTER
//! ausfallen als die freie (`rotation_search.rs`, `greedy_from`); der Vergleich
//! mit Schritt 1 ist deshalb Pflicht und nicht Beiwerk.
//!
//! Schritt 3 laeuft in DREI Fassungen: a) die Liste, wie die Suche ohne
//! Talent sie ordnet; b) dieselbe Liste mit dem Traeger an erster Stelle —
//! die Suche stellt einen Traeger, der ohne Talent nichts beitraegt, ans Ende,
//! wo er nie gewirkt wird (siehe [`vorgezogen`]); c) eine Liste, die MIT dem
//! Talent des Wegs gesucht ist, also fuer ihn.
//!
//! **Gewonnen hat ein Weg, wenn er BEIDE Nachbarn im Schaden schlaegt und
//! ueberhaupt etwas bewegt.** Gleichstand ist kein Sieg (siehe [`bewerte`]).
//!
//! **Der Abstand misst immer die Liste OHNE Talent** gegen die beste bekannte
//! Rotation, auch in Fassung c). Spielbar heisst damit: ohne den Weg hoechstens
//! 10 % unter der besten. Eine Liste, die erst durch ihr Talent aufholt, zaehlt
//! hier nicht als spielbar — das ist eine Lesart, keine Entscheidung.
//!
//! # Die Zahlen kommen aus `data/player/talents.json`
//!
//! Bis 2026-10-24 standen sie hier als Literale, und sieben von neun waren
//! gealtert (zum Beispiel `putrefaction-duration` mit 300 statt 600 Promille,
//! `global-hold` nur mit der Brand-Laufzeit statt mit allen drei Feldern).
//! Jetzt liest der Laeufer die Datei beim Bau ein und setzt je Weg den vollen
//! Ausbau: `maxRanks` mal Wert je Rang. Der Stamm des Baums bleibt aussen vor —
//! er traegt allen drei Wegen dasselbe bei. Ein Feld, das [`setze`] nicht
//! kennt, bricht den Lauf ab, statt still zu fehlen.
//!
//! # Was der Laeufer NICHT bewertet
//!
//! Den Wert von Verteidigung. „Gewinnt" heisst hier: der meiste Schaden.
//! Schutz-, Heil- und Kontroll-Wege zeigen ihre Wirkung oft nur am
//! erlittenen Schaden und an der Zeit bis zum Tod (GAME-DESIGN 20.5 Punkt 4);
//! beide stehen in der Ausgabe daneben. Wie viel ein verhinderter Schlag
//! gegen Durchsatz wiegt (`prevented_weight`), ist bis zum Optimierer
//! vertagt — der Laeufer setzt ihn deshalb nicht.

use std::collections::BTreeMap;

use rota_sim::apl::Apl;
use rota_sim::balance::{BalanceConfig, BalanceResult};
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::rotation_search::{all_candidates, greedy, greedy_from, Candidate, SearchResult};
use serde_json::Value;

/// Die Talent-Daten, beim Bau eingesetzt — dieselbe Datei, die das Spiel liest.
const TALENTE: &str = include_str!("../../data/player/talents.json");
/// Der Boss aller Messungen, wie in den Laeufern davor (#419, #881).
const BOSS: &str = "boss-1-1";
/// Wie lang eine gesuchte Rotation hoechstens wird.
const MAX_LEN: usize = 8;
/// Durchgaenge je Messung in der Suche (Schritt 1 und 2).
const SUCH_DURCHGAENGE: u32 = 20;
/// Durchgaenge je Messung im Vergleich der Wege (Schritt 3).
const MESS_DURCHGAENGE: u32 = 60;

/// Welcher Zauber einen Weg ueberhaupt erst wirken laesst.
///
/// **Meine Wahl, und sie ist bestreitbar.** Die neun Zeilen fuer
/// PUTREFACTION, SUBLIMATION und GLOBAL stammen aus der vorigen Fassung
/// dieses Laeufers. Die neun neuen folgen derselben Regel — der Zauber, an
/// dessen Zahl der Weg greift:
///
/// ```text
///   CALCINATION  build kindle (Hitze)   release pyre (Entlader)   hold smoulder (Brand)
///   FIXATION     build bind (Aufbau)    cost latch (2 -> 1, der groesste Anteil)
///                effect stay (Verzoegerung)
///   COAGULATION  thicker plate (eine Schicht, der Betrag zaehlt)
///                longer crust (Frist)   stacks scab (billig, stapelt schnell)
/// ```
const TRAEGER: [(&str, &str, &str); 18] = [
    ("CALCINATION", "build", "kindle"),
    ("CALCINATION", "release", "pyre"),
    ("CALCINATION", "hold", "smoulder"),
    ("FIXATION", "build", "bind"),
    ("FIXATION", "cost", "latch"),
    ("FIXATION", "effect", "stay"),
    ("SUBLIMATION", "reduction", "efflux"),
    ("SUBLIMATION", "charges", "reflux"),
    ("SUBLIMATION", "threshold", "vapour"),
    ("COAGULATION", "thicker", "plate"),
    ("COAGULATION", "longer", "crust"),
    ("COAGULATION", "stacks", "scab"),
    ("PUTREFACTION", "build", "putrid-bloom"),
    ("PUTREFACTION", "effect", "fester"),
    ("PUTREFACTION", "duration", "decay"),
    ("GLOBAL", "build", "kindle"),
    ("GLOBAL", "release", "pyre"),
    ("GLOBAL", "hold", "smoulder"),
];

/// Ein Weg, wie er in den Daten steht.
struct Weg {
    baum: String,
    pfad: String,
    talent: String,
    /// Feld aus `talents.json` und sein Wert bei vollem Ausbau.
    felder: Vec<(String, u32)>,
    traeger: &'static str,
}

/// Liest die achtzehn Wege: je Baum alles ausser Stamm und Schluesseltalent.
fn wege() -> Vec<Weg> {
    let daten: Value = serde_json::from_str(TALENTE).expect("talents.json ist kein JSON");
    let mut aus = Vec::new();
    for baum in daten["trees"].as_array().expect("trees fehlt") {
        let schule = baum["school"].as_str().expect("school fehlt");
        for talent in baum["talents"].as_array().expect("talents fehlt") {
            let pfad = talent["path"].as_str().expect("path fehlt");
            if pfad == "trunk" || talent.get("keystone").is_some() {
                continue;
            }
            let raenge = talent["maxRanks"].as_u64().expect("maxRanks fehlt");
            let mut felder = Vec::new();
            for (feld, wert) in talent.as_object().expect("Talent ist kein Objekt") {
                if !feld.contains("PerRank") {
                    continue;
                }
                let je_rang = wert
                    .as_u64()
                    .unwrap_or_else(|| panic!("{feld} ist keine Zahl"));
                if je_rang > 0 {
                    let voll = u32::try_from(raenge * je_rang).expect("zu gross");
                    felder.push((feld.clone(), voll));
                }
            }
            let traeger = TRAEGER
                .iter()
                .find(|(s, p, _)| *s == schule && *p == pfad)
                .map(|(_, _, t)| *t)
                .unwrap_or_else(|| {
                    panic!("kein Traeger fuer {schule} {pfad} — in TRAEGER eintragen")
                });
            aus.push(Weg {
                baum: schule.to_owned(),
                pfad: pfad.to_owned(),
                talent: talent["id"].as_str().expect("id fehlt").to_owned(),
                felder,
                traeger,
            });
        }
    }
    assert_eq!(aus.len(), 18, "nicht achtzehn Wege in talents.json");
    aus
}

/// Ein Feld aus `talents.json` auf die passende Groesse des Simulators.
///
/// **Ein unbekanntes Feld bricht ab.** Kaeme eines dazu und fehlte hier,
/// rechnete der Laeufer den Weg ohne Wirkung und meldete ihn als tot.
fn setze(c: &mut BalanceConfig, feld: &str, wert: u32) {
    match feld {
        "damagePerRankPerMille" => c.talent_permille += wert,
        "resourceGainPerRankPerMille" => c.talent_resource_permille += wert,
        "dotDurationPerRankPerMille" => c.talent_dot_duration_permille += wert,
        "controlGainPerRankPerMille" => c.talent_control_gain_permille += wert,
        "controlCostPerRankPerMille" => c.talent_control_cost_permille += wert,
        "controlEffectPerRankPerMille" => c.talent_control_effect_permille += wert,
        "decayEffectPerRankPerMille" => c.talent_decay_effect_permille += wert,
        "decayStackPerRankPerMille" => c.talent_decay_stack_permille += wert,
        "decayDurationPerRankPerMille" => c.talent_decay_duration_permille += wert,
        "decayWardPerRankPerMille" => c.talent_decay_ward_permille += wert,
        // Ganze Plaetze, abgerundet — wie `talentDecayCapBonus` im Spiel.
        "decayCapPerRankPerMille" => {
            c.talent_decay_cap_bonus += usize::try_from(wert / 1000).expect("zu gross");
        }
        "shieldAmountPerRankPerMille" => c.talent_shield_amount_permille += wert,
        "shieldDurationPerRankPerMille" => c.talent_shield_duration_permille += wert,
        "shieldStacksPerRank" => c.talent_shield_stacks_bonus += wert,
        "cooldownReductionPerRankPerMille" => c.talent_cooldown_reduction_permille += wert,
        "cooldownResetChargesPerRank" => c.talent_cooldown_charges_bonus += wert,
        "cooldownThresholdPerRankPerMille" => c.talent_cooldown_threshold_permille += wert,
        "spenderDamagePerRankPerMille" => c.talent_spender_damage_permille += wert,
        "buildDamagePerRankPerMille" => c.talent_build_damage_permille += wert,
        "convertPerRankPerMille" => c.talent_convert_permille += wert,
        anderes => panic!("Das Feld {anderes} kennt `setze` nicht — dort eintragen"),
    }
}

fn config(iterations: u32, weg: Option<&Weg>) -> BalanceConfig {
    let mut c = BalanceConfig {
        iterations,
        base_seed: 5,
        threads: 2,
        stop_on_kill: false,
        duration_ms: Some(60_000),
        boss: boss_by_id(BOSS).expect("Boss fehlt"),
        ..BalanceConfig::default()
    };
    if let Some(w) = weg {
        for (feld, wert) in &w.felder {
            setze(&mut c, feld, *wert);
        }
    }
    c
}

/// Der erste Kandidat zu einem Zauber — die Zeile ohne Bedingung.
fn zeile(id: &str, pool: &[Candidate]) -> Candidate {
    let index = index_of(id).unwrap_or_else(|| panic!("Zauber {id} fehlt"));
    pool.iter()
        .find(|k| k.spell == index)
        .cloned()
        .unwrap_or_else(|| panic!("kein Kandidat fuer {id}"))
}

/// Eine Rotation, eingerueckt ausgegeben — damit sie benannt ist.
fn zeige(apl: &Apl) {
    for z in apl.to_string().lines() {
        println!("          {z}");
    }
}

/// Dieselbe Liste, der Traeger an erster Stelle.
///
/// **Die zweite Fassung, und warum es sie gibt:** Die Suche laesst einen
/// Traeger, der ohne Talent nichts beitraegt, an letzter Stelle stehen. Dort
/// kommt er nie dran, und die drei Wege vergleichen sich in einer Rotation,
/// die ihn nicht wirkt. Gemessen an der ersten Fassung dieses Laeufers
/// (2026-10-24): Bei elf der achtzehn Wege wurde der Traeger nie gewirkt, bei
/// zehn davon in derselben Liste mit demselben Schaden (10729). Vorgezogen
/// wird er gewirkt, sobald er bereit ist; ob die Rotation dann noch spielbar
/// ist, sagt der Abstand.
fn vorgezogen(apl: &Apl, traeger: usize) -> Option<Apl> {
    let pos = apl.rules.iter().position(|r| r.spell == traeger)?;
    if pos == 0 {
        return None;
    }
    let mut rules = apl.rules.clone();
    let regel = rules.remove(pos);
    rules.insert(0, regel);
    for (i, r) in rules.iter_mut().enumerate() {
        r.line = i + 1;
    }
    Some(Apl {
        rules,
        name: format!("{}-vorgezogen", apl.name),
    })
}

/// Bitgleich, nicht „fast gleich": Der Lauf ist deterministisch, und eine
/// Abweichung in der letzten Stelle IST eine Wirkung.
fn gleich(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

/// Die Zeit bis zum Tod des Spielers — Spielzeit, keine Laufzeit.
fn tod(r: &BalanceResult) -> String {
    if r.deaths == 0 {
        format!("0/{}", r.samples.len())
    } else {
        format!(
            "{}/{} bei {:.0} ms Spielzeit",
            r.deaths,
            r.samples.len(),
            r.death_time_ms.mean
        )
    }
}

/// Wie ein Weg in einer Rotation abschneidet.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lage {
    /// Er bewegt weder den Schaden noch den erlittenen Schaden noch die Tode.
    Wirkungslos,
    /// Er schlaegt BEIDE Nachbarn im Schaden.
    Gewinnt,
    /// Gleichauf mit dem besten Nachbarn.
    Gleichstand,
    /// Ein Nachbar liegt vorn.
    Verliert,
}

impl Lage {
    fn text(self) -> &'static str {
        match self {
            Lage::Wirkungslos => "wirkungslos",
            Lage::Gewinnt => "gewinnt",
            Lage::Gleichstand => "gleichauf",
            Lage::Verliert => "verliert",
        }
    }
}

/// Ein Weg in einer Fassung der Rotation.
struct Fassung {
    abstand: f64,
    /// Derselbe Abstand, aber mit dem Talent des Wegs — fuer eine andere Lesart.
    abstand_mit: f64,
    wirkungen: u32,
    lage: Lage,
    sieger: String,
}

impl Fassung {
    /// AC3 nach der Lesart oben: spielbar UND vorn.
    fn erfuellt(&self) -> bool {
        self.abstand <= 10.0 && self.lage == Lage::Gewinnt
    }

    /// Dieselbe Frage in der anderen Lesart: spielbar MIT dem Talent des Wegs.
    ///
    /// Steht daneben, nicht an ihrer Stelle — welche Lesart gilt, ist nicht
    /// entschieden, und eine Zahl fuer nur eine haette sie vorweggenommen.
    fn erfuellt_mit(&self) -> bool {
        self.abstand_mit <= 10.0 && self.lage == Lage::Gewinnt
    }

    fn zelle(&self) -> String {
        format!(
            "{:.1} % (mit {:.1} %) · {}× · {}",
            self.abstand,
            self.abstand_mit,
            self.wirkungen,
            self.lage.text()
        )
    }
}

/// Misst einen Weg und seine Nachbarn in einer Rotation und gibt alles aus.
///
/// **Der Sieger muss beide Nachbarn SCHLAGEN.** Die erste Fassung dieses
/// Laeufers nahm bei Gleichstand den ersten der drei — und meldete
/// `fixation-build` als erfuellt, obwohl alle drei Wege und die Rotation ohne
/// Talent dieselben 11498 ergaben. Ein Sieger aus Gleichstand ist eine
/// Reihenfolge, keine Messung.
fn bewerte(weg: &Weg, nachbarn: &[&Weg], apl: &Apl, beste: f64) -> Fassung {
    let ohne = rota_sim::balance::run(&config(MESS_DURCHGAENGE, None), apl);
    let abstand = (beste - ohne.damage.mean) / beste * 100.0;
    // Die Zahl der Wirkungen des Traegers: null heisst, die Rotation wirkt
    // ihn nie. Der Weg kann trotzdem ueber andere Zauber greifen — deshalb
    // zaehlt fuer das Urteil allein, ob er etwas BEWEGT.
    let wirkungen = ohne.casts[index_of(weg.traeger).expect("Traeger fehlt")];
    println!(
        "        ohne Talent  {:>7.0} / {:>6.0} / {}   ({abstand:.1} % unter der besten, \
         {} {wirkungen}-mal)",
        ohne.damage.mean,
        ohne.damage_taken.mean,
        tod(&ohne),
        weg.traeger
    );
    let mut werte: Vec<(&str, BalanceResult)> = Vec::new();
    for n in nachbarn {
        let r = rota_sim::balance::run(&config(MESS_DURCHGAENGE, Some(n)), apl);
        println!(
            "        {:<12} {:>7.0} / {:>6.0} / {}",
            n.pfad,
            r.damage.mean,
            r.damage_taken.mean,
            tod(&r)
        );
        werte.push((n.pfad.as_str(), r));
    }

    let eigener = &werte
        .iter()
        .find(|(n, _)| *n == weg.pfad)
        .expect("der eigene Weg fehlt")
        .1;
    let wirkt = !gleich(eigener.damage.mean, ohne.damage.mean)
        || !gleich(eigener.damage_taken.mean, ohne.damage_taken.mean)
        || eigener.deaths != ohne.deaths;
    let bester_nachbar = werte
        .iter()
        .filter(|(n, _)| *n != weg.pfad)
        .map(|(_, r)| r.damage.mean)
        .fold(f64::MIN, f64::max);
    let lage = if !wirkt {
        Lage::Wirkungslos
    } else if eigener.damage.mean > bester_nachbar {
        Lage::Gewinnt
    } else if gleich(eigener.damage.mean, bester_nachbar) {
        Lage::Gleichstand
    } else {
        Lage::Verliert
    };
    let spitze = werte
        .iter()
        .map(|(_, r)| r.damage.mean)
        .fold(f64::MIN, f64::max);
    let vorn: Vec<&str> = werte
        .iter()
        .filter(|(_, r)| gleich(r.damage.mean, spitze))
        .map(|(n, _)| *n)
        .collect();
    let sieger = vorn.join(" = ");
    let abstand_mit = (beste - eigener.damage.mean) / beste * 100.0;
    let fassung = Fassung {
        abstand,
        abstand_mit,
        wirkungen,
        lage,
        sieger,
    };
    println!(
        "        -> vorn nach Schaden: {}   {}: {}{}",
        fassung.sieger,
        weg.pfad,
        lage.text(),
        if fassung.erfuellt() {
            "   AC3 nach dieser Lesart ERFUELLT"
        } else {
            ""
        }
    );
    fassung
}

fn main() {
    let pool = all_candidates();
    let wege = wege();

    // --- Schritt 1: die heute beste bekannte Rotation ----------------------
    let frei = greedy(&config(SUCH_DURCHGAENGE, None), &pool, MAX_LEN);
    let beste = frei.best.mean;
    println!("== Schritt 1: die freie Suche ==\n");
    println!(
        "  Boss {BOSS}, 60 s, {SUCH_DURCHGAENGE} Durchgaenge ab Startwert 5, \
         hoechstens {MAX_LEN} Zeilen, ohne Talente"
    );
    println!("  Kandidaten im Pool: {}", frei.pool_size);
    println!("  bester Schaden: {beste:.0}");
    println!(
        "  Schwelle fuer Spielbarkeit: {:.0} (10 % darunter)",
        beste * 0.9
    );
    println!("  die Rotation:");
    zeige(&frei.best.apl);
    println!();
    println!(
        "  Je Weg: Schaden / erlittener Schaden / Tode des Spielers, \
         {MESS_DURCHGAENGE} Durchgaenge, voller Ausbau des Wegs\n"
    );

    // Eine Suche je Traeger, nicht je Weg: Zwei Wege mit demselben Traeger
    // bekaemen dieselbe Liste.
    let mut suchen: BTreeMap<&'static str, SearchResult> = BTreeMap::new();
    let mut zeilen: Vec<(String, &'static str, [Option<Fassung>; 3])> = Vec::new();

    for baum in [
        "CALCINATION",
        "FIXATION",
        "SUBLIMATION",
        "COAGULATION",
        "PUTREFACTION",
        "GLOBAL",
    ] {
        println!("== {baum} ==\n");
        let nachbarn: Vec<&Weg> = wege.iter().filter(|w| w.baum == baum).collect();
        assert_eq!(nachbarn.len(), 3, "{baum} hat nicht drei Wege");

        for weg in &nachbarn {
            let felder: Vec<String> = weg.felder.iter().map(|(f, v)| format!("{f}={v}")).collect();
            println!(
                "  {} ({})   {}   Traeger {}",
                weg.pfad,
                weg.talent,
                felder.join(", "),
                weg.traeger
            );

            // --- Schritt 2: die beste Rotation MIT dem Traeger -------------
            let gesucht = suchen.entry(weg.traeger).or_insert_with(|| {
                let start = [zeile(weg.traeger, &pool)];
                greedy_from(&config(SUCH_DURCHGAENGE, None), &pool, MAX_LEN, &start)
            });

            // --- Schritt 3: dieselbe Rotation, ohne und mit jedem Weg -------
            println!("      a) die gesuchte Liste:");
            zeige(&gesucht.best.apl);
            let a = bewerte(weg, &nachbarn, &gesucht.best.apl, beste);

            let traeger = index_of(weg.traeger).expect("Traeger fehlt");
            let b = if let Some(vorn) = vorgezogen(&gesucht.best.apl, traeger) {
                println!("      b) dieselbe Liste, {} vorgezogen:", weg.traeger);
                zeige(&vorn);
                Some(bewerte(weg, &nachbarn, &vorn, beste))
            } else {
                println!("      b) entfaellt: {} steht schon vorn", weg.traeger);
                None
            };

            // --- c) die Suche MIT dem Talent dieses Wegs --------------------
            //
            // a) und b) suchen ohne Talent und setzen es erst danach ein. Mit
            // dem Talent kann eine andere Liste die beste sein; c) gibt jedem
            // Weg die Liste, die fuer IHN gesucht ist. Der Abstand misst
            // weiterhin die Liste ohne Talent gegen die beste bekannte.
            let fuer_ihn = greedy_from(
                &config(SUCH_DURCHGAENGE, Some(weg)),
                &pool,
                MAX_LEN,
                &[zeile(weg.traeger, &pool)],
            );
            println!("      c) gesucht MIT {}:", weg.talent);
            zeige(&fuer_ihn.best.apl);
            let c = bewerte(weg, &nachbarn, &fuer_ihn.best.apl, beste);
            println!();
            zeilen.push((
                format!("{baum} {}", weg.pfad),
                weg.traeger,
                [Some(a), b, Some(c)],
            ));
        }
    }

    // --- Zusammenfassung -----------------------------------------------------
    println!("== Zusammenfassung: 18 Wege gegen die beste bekannte Rotation ({beste:.0}) ==\n");
    println!("  je Fassung: Abstand zur besten · Wirkungen des Traegers · Lage des Wegs\n");
    println!(
        "| Weg | Traeger | a) gesucht ohne Talent | vorn | b) Traeger vorgezogen | vorn \
         | c) gesucht mit dem Weg | vorn | AC3 |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    let mut erfuellt = 0;
    let mut erfuellt_mit = 0;
    for (name, traeger, fassungen) in &zeilen {
        let ok = fassungen.iter().flatten().any(Fassung::erfuellt);
        if ok {
            erfuellt += 1;
        }
        if fassungen.iter().flatten().any(Fassung::erfuellt_mit) {
            erfuellt_mit += 1;
        }
        let zellen: Vec<String> = fassungen
            .iter()
            .map(|f| {
                f.as_ref().map_or_else(
                    || "— | —".to_owned(),
                    |f| format!("{} | {}", f.zelle(), f.sieger),
                )
            })
            .collect();
        println!(
            "| {name} | {traeger} | {} | {} |",
            zellen.join(" | "),
            if ok { "erfuellt" } else { "nein" }
        );
    }
    println!(
        "\n  erfuellt, spielbar OHNE Talent gemessen (Spalte AC3): {erfuellt} von {}",
        zeilen.len()
    );
    println!(
        "  erfuellt, spielbar MIT dem Talent des Wegs gemessen:   {erfuellt_mit} von {}",
        zeilen.len()
    );
}
