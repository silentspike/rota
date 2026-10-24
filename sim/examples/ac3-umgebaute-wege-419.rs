//! #419 AC3: Ist JEDER der achtzehn Talentwege irgendwo die beste Wahl?
//!
//! ```text
//! cargo remote --no-copy-lock -- run --release --example ac3-umgebaute-wege-419
//! ```
//!
//! # Was AC3 verlangt, und in welcher Lesart
//!
//! „Der Balance-Läufer belegt, dass KEINER der Wege tot ist — jeder ist in
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
//! Dieser Läufer misst nach dieser Lesart und schließt AC3 nicht ab. Wann AC3
//! als erfüllt gilt, ist beim Auftraggeber offen, und mit ihm zwei Fragen, ohne
//! die die Zahl nicht feststeht: die Zeilengrenze und die Lesart von
//! „spielbar". Beide stehen unten, und der Läufer gibt für jede Antwort die
//! Zahl aus, statt eine zu wählen.
//!
//! # Die Zeilengrenze ist offen — deshalb misst der Läufer unter vier
//!
//! Die erste Fassung (e41032e0) suchte mit höchstens 8 Zeilen und nannte ihr
//! Ergebnis die heute beste bekannte Rotation. **Das war es nicht.** Die Suche
//! brach an der Zeilengrenze ab (`Stop::LengthLimit`), nicht weil keine Zeile
//! mehr half (`Stop::NoImprovement`), und mit mehr Zeilen findet sie mehr. Einen
//! Grund für die 8 nannte die erste Fassung nicht, und die Leiste gibt keinen
//! her: Acht Plätze hat der Spieler erst ab Stufe 30 (`data/player/levels.json`).
//!
//! Welche Grenze für den Bezug und für „spielbar" gilt, entscheidet der
//! Auftraggeber. Bis dahin misst der Läufer unter jeder dieser vier, und
//! **Bezug und Vergleichslisten stehen dabei unter DERSELBEN Grenze**:
//!
//! ```text
//!   Grenze        woher (Stand 2026-10-24)
//!   5 Zeilen      die Leiste auf der Stufe, ab der ein Weg voll ausgebaut werden kann
//!   8 Zeilen      die Vorgabe der ersten Fassung
//!   12 Zeilen     die größte Leiste
//!   ohne Grenze   bis die Suche selbst aufhört
//! ```
//!
//! Die 5 und die 12 stehen nicht als Zahl im Code. Der Läufer rechnet sie aus
//! `levels.json` und `talents.json`, beide beim Bau eingesetzt (siehe
//! [`leiste`]), damit sie nicht altern wie die Promille der ersten Fassung.
//!
//! **Eine Zeilengrenze ist keine Platzgrenze.** Zwei Zeilen können denselben
//! Zauber mit verschiedenen Bedingungen tragen: N Zeilen halten eine Leiste mit
//! N Plätzen sicher ein, schöpfen sie aber nicht unbedingt aus. Neben jeder
//! Liste steht deshalb, wie viele Zauber sie braucht. Eine Grenze für Zauber
//! statt für Zeilen kennt die Suche nicht.
//!
//! **Jede Suche steht mit ihrem Abbruchgrund**, wie in
//! `docs/berichte/e607-zauberkasten.md`: Ein Bezug, der an der Zeilengrenze
//! endet, ist die beste gefundene Liste MIT dieser Grenze, nicht die beste
//! überhaupt.
//!
//! # Der Bezug ist die beste Liste, die der Lauf kennt
//!
//! Nicht allein das Ergebnis der freien Suche. Liegt eine Vergleichsliste ohne
//! Talent darüber, ist SIE die beste bekannte Rotation unter dieser Grenze, und
//! der Bezug nennt sie. Sonst läge er unter der besten bekannten, und jede
//! Liste sähe spielbarer aus, als sie ist.
//!
//! # Der Aufbau, je Grenze
//!
//! ```text
//!   1  freie Suche                         -> die erste Liste
//!   2  je Weg drei Listen, siehe unten     -> Listen, in denen der Träger drankommt
//!   3  jede Liste ohne und mit jedem Weg   -> gewinnt ein Weg gegen seine Nachbarn?
//! ```
//!
//! Die drei Listen je Weg: a) die Liste, die die Suche mit dem Träger als Start
//! findet; b) dieselbe mit dem Träger an erster Stelle — die Suche stellt einen
//! Träger, der ohne Talent nichts beiträgt, ans Ende, wo er nie gewirkt wird
//! (siehe [`vorgezogen`]); c) eine Liste, die MIT dem Talent des Wegs gesucht
//! ist, also für ihn. Eine Suche von einem vorgegebenen Startpunkt kann
//! SCHLECHTER ausfallen als die freie (`rotation_search.rs`, `greedy_from`);
//! deshalb misst jede Liste ihren Abstand zum Bezug.
//!
//! # Gewertet wird jeder Weg in JEDER Liste
//!
//! Die erste Fassung wertete einen Weg nur in den Listen seiner eigenen Zeile.
//! AC3 fragt aber nach IRGENDEINER Zusammenstellung, und gemessen war mehr:
//! `calcination-release` gewann in der Liste von `calcination-build` mit 0,0 %
//! Abstand und stand trotzdem als „nein" in der Tabelle (Abnahme vom
//! 2026-10-24). Jetzt misst der Läufer jede Liste ohne Talent und mit jedem der
//! achtzehn Wege und wertet jeden Weg in jeder Liste. Die Zählung über die
//! Listen des eigenen Baums allein steht zum Vergleich daneben.
//!
//! **Gewonnen hat ein Weg, wenn er BEIDE Nachbarn seines Baums im Schaden
//! schlägt und überhaupt etwas bewegt.** Gleichstand ist kein Sieg (siehe
//! [`lage`]).
//!
//! # Zwei Lesarten von „spielbar", beide ausgegeben
//!
//! ```text
//!   ohne Talent   die Liste OHNE Talent liegt höchstens 10 % unter dem Bezug
//!   mit Talent    dieselbe Liste MIT dem Talent des Wegs liegt höchstens 10 % darunter
//! ```
//!
//! Welche gilt, ist nicht entschieden; eine Zahl für nur eine hätte die Frage
//! vorweggenommen.
//!
//! **Beide messen nur den Schaden.** Stirbt der Spieler in der Liste, die einen
//! Weg erfüllt, steht das als Vorbehalt daneben, und die Zählung nennt, wie
//! viele Wege nur mit Vorbehalt erfüllt sind. Zeit bis zum Tod ist die zweite
//! Größe jeder Balance-Aussage (GAME-DESIGN 20.5 Punkt 4).
//!
//! # Die Zahlen kommen aus `data/player/talents.json`
//!
//! Bis 2026-10-24 standen sie hier als Literale, und sieben von neun waren
//! gealtert (zum Beispiel `putrefaction-duration` mit 300 statt 600 Promille,
//! `global-hold` nur mit der Brand-Laufzeit statt mit allen drei Feldern).
//! Jetzt liest der Läufer die Datei beim Bau ein und setzt je Weg den vollen
//! Ausbau: `maxRanks` mal Wert je Rang. Der Stamm des Baums bleibt außen vor —
//! er trägt allen drei Wegen dasselbe bei. Ein Feld, das [`setze`] nicht kennt,
//! bricht den Lauf ab, statt still zu fehlen.
//!
//! # Was der Läufer NICHT bewertet
//!
//! Den Wert von Verteidigung. „Gewinnt" heißt hier: der meiste Schaden.
//! Schutz-, Heil- und Kontroll-Wege zeigen ihre Wirkung oft nur am erlittenen
//! Schaden und an der Zeit bis zum Tod (GAME-DESIGN 20.5 Punkt 4); beide stehen
//! in der Ausgabe daneben. Wie viel ein verhinderter Schlag gegen Durchsatz
//! wiegt (`prevented_weight`), ist bis zum Optimierer vertagt — der Läufer
//! setzt ihn deshalb nicht.
//!
//! Die Stufe ebenso wenig: Unter jeder Grenze stehen alle Kandidaten der Suche
//! zur Wahl — auch Zauber, die auf der Stufe, deren Leiste eine Grenze
//! nachbildet, noch nicht freigeschaltet sind (`unlockLevel`). Gemessen wird an
//! einem Boss (`boss-1-1`), wie in den Läufern davor.

use std::collections::{BTreeMap, BTreeSet};

use rota_sim::apl::Apl;
use rota_sim::balance::{BalanceConfig, BalanceResult};
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::rotation_search::{
    all_candidates, greedy, greedy_from, Candidate, SearchResult, Stop,
};
use serde_json::Value;

/// Die Talent-Daten, beim Bau eingesetzt — dieselbe Datei, die das Spiel liest.
const TALENTE: &str = include_str!("../../data/player/talents.json");
/// Die Stufen-Daten, beim Bau eingesetzt: Leistenplätze und Talentpunkte.
const STUFEN: &str = include_str!("../../data/player/levels.json");
/// Der Boss aller Messungen, wie in den Läufern davor (#419, #881).
const BOSS: &str = "boss-1-1";
/// Die Zeilengrenze der ersten Fassung — eine der vier, dort ohne Begründung.
const ERSTE_FASSUNG: usize = 8;
/// Durchgänge je Messung in der Suche.
const SUCH_DURCHGAENGE: u32 = 20;
/// Durchgänge je Messung im Vergleich der Wege.
const MESS_DURCHGAENGE: u32 = 60;
/// „Spielbar": höchstens so viel Prozent unter dem Bezug (GAME-DESIGN 20.3).
const SCHWELLE_PROZENT: f64 = 10.0;

/// Welcher Zauber einen Weg überhaupt erst wirken lässt.
///
/// **Meine Wahl, und sie ist bestreitbar.** Die neun Zeilen für PUTREFACTION,
/// SUBLIMATION und GLOBAL stammen aus der vorigen Fassung dieses Läufers. Die
/// neun neuen folgen derselben Regel — der Zauber, an dessen Zahl der Weg
/// greift:
///
/// ```text
///   CALCINATION  build kindle (Hitze)   release pyre (Entlader)   hold smoulder (Brand)
///   FIXATION     build bind (Aufbau)    cost latch (2 -> 1, der größte Anteil)
///                effect stay (Verzögerung)
///   COAGULATION  thicker plate (eine Schicht, der Betrag zählt)
///                longer crust (Frist)   stacks scab (billig, stapelt schnell)
/// ```
///
/// Seit der Wertung über alle Listen bestimmt der Träger nur noch, welche
/// Listen ENTSTEHEN — gewertet wird jeder Weg in jeder von ihnen.
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
    /// `maxRanks` — der volle Ausbau.
    raenge: u64,
    /// Feld aus `talents.json` und sein Wert bei vollem Ausbau.
    felder: Vec<(String, u32)>,
    traeger: &'static str,
}

/// Liest die achtzehn Wege: je Baum alles außer Stamm und Schlüsseltalent.
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
                    let voll = u32::try_from(raenge * je_rang).expect("zu groß");
                    felder.push((feld.clone(), voll));
                }
            }
            let traeger = TRAEGER
                .iter()
                .find(|(s, p, _)| *s == schule && *p == pfad)
                .map(|(_, _, t)| *t)
                .unwrap_or_else(|| {
                    panic!("kein Träger für {schule} {pfad} — in TRAEGER eintragen")
                });
            aus.push(Weg {
                baum: schule.to_owned(),
                pfad: pfad.to_owned(),
                talent: talent["id"].as_str().expect("id fehlt").to_owned(),
                raenge,
                felder,
                traeger,
            });
        }
    }
    assert_eq!(aus.len(), 18, "nicht achtzehn Wege in talents.json");
    for w in &aus {
        let im_baum = aus.iter().filter(|v| v.baum == w.baum).count();
        assert_eq!(im_baum, 3, "{} hat nicht drei Wege", w.baum);
    }
    aus
}

/// Ein Feld aus `talents.json` auf die passende Größe des Simulators.
///
/// **Ein unbekanntes Feld bricht ab.** Käme eines dazu und fehlte hier,
/// rechnete der Läufer den Weg ohne Wirkung und meldete ihn als tot.
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
        // Ganze Plätze, abgerundet — wie `talentDecayCapBonus` im Spiel.
        "decayCapPerRankPerMille" => {
            c.talent_decay_cap_bonus += usize::try_from(wert / 1000).expect("zu groß");
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
        .unwrap_or_else(|| panic!("kein Kandidat für {id}"))
}

/// Eine Rotation in einer Zeile, damit sie neben ihren Zahlen benannt ist.
fn einzeilig(apl: &Apl) -> String {
    apl.to_string().lines().collect::<Vec<_>>().join(" / ")
}

/// Wie viele VERSCHIEDENE Zauber eine Liste braucht — so viele Leistenplätze.
fn zauber(apl: &Apl) -> usize {
    apl.rules
        .iter()
        .map(|r| r.spell)
        .collect::<BTreeSet<_>>()
        .len()
}

/// Dieselbe Liste, der Träger an erster Stelle.
///
/// **Die zweite Fassung, und warum es sie gibt:** Die Suche lässt einen
/// Träger, der ohne Talent nichts beiträgt, an letzter Stelle stehen. Dort
/// kommt er nie dran, und die drei Wege vergleichen sich in einer Rotation,
/// die ihn nicht wirkt. Gemessen an der ersten Fassung dieses Läufers
/// (2026-10-24): Bei elf der achtzehn Wege wurde der Träger nie gewirkt, bei
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

/// Eine Messung in Kurzform: Schaden / erlittener Schaden / Tode.
fn kurz(r: &BalanceResult) -> String {
    format!(
        "{:.0} / {:.0} / {}",
        r.damage.mean,
        r.damage_taken.mean,
        tod(r)
    )
}

/// Um wie viel Prozent ein Wert unter dem Bezug liegt; negativ heißt darüber.
fn prozent_unter(bezug: f64, wert: f64) -> f64 {
    (bezug - wert) / bezug * 100.0
}

// ---------------------------------------------------------------------------
// Die Grenzen
// ---------------------------------------------------------------------------

/// Die Leiste aus `levels.json`, und ab welcher Stufe ein Weg voll ausgebaut
/// werden kann.
struct Leiste {
    /// Plätze auf Stufe 1.
    start: usize,
    /// Je Eintrag eine Stufe, auf der ein weiterer Platz aufgeht.
    plaetze_ab: Vec<u64>,
    /// Talentpunkte für einen voll ausgebauten Weg.
    punkte_voller_weg: usize,
    /// Die Stufe, auf der so viele Punkte zusammen sind.
    stufe_voller_weg: u64,
}

impl Leiste {
    /// Plätze auf einer Stufe — dieselbe Rechnung wie `barSlotsAtLevel` in
    /// `src/progression/unlocks.ts`.
    fn plaetze(&self, stufe: u64) -> usize {
        self.start + self.plaetze_ab.iter().filter(|&&ab| stufe >= ab).count()
    }

    /// Plätze auf der höchsten Stufe.
    fn hoechstens(&self) -> usize {
        self.start + self.plaetze_ab.len()
    }

    /// Ab welcher Stufe die Leiste `n` Plätze hat; `None`, wenn nie.
    fn stufe_fuer(&self, n: usize) -> Option<u64> {
        if n <= self.start {
            return Some(1);
        }
        self.plaetze_ab.get(n - self.start - 1).copied()
    }
}

/// Liest die Leiste und rechnet, ab welcher Stufe ein Weg voll ausgebaut
/// werden kann.
///
/// Ein voller Weg kostet seine Ränge plus EINEN Rang im Stamm: `requires`
/// verlangt mindestens einen (`allocationProblem` in
/// `src/progression/talents.ts`). Die Punkte fallen auf den Stufen aus
/// `talentPointLevels` an, je Stufe einer (`talentPointsAtLevel` in
/// `src/progression/unlocks.ts`); der n-te Punkt kommt also auf Stufe
/// `talentPointLevels[n - 1]`.
fn leiste(wege: &[Weg]) -> Leiste {
    let daten: Value = serde_json::from_str(STUFEN).expect("levels.json ist kein JSON");
    let stufen = |feld: &str| -> Vec<u64> {
        daten[feld]
            .as_array()
            .unwrap_or_else(|| panic!("{feld} fehlt in levels.json"))
            .iter()
            .map(|v| {
                v.as_u64()
                    .unwrap_or_else(|| panic!("{feld} enthält keine ganze Zahl"))
            })
            .collect()
    };
    let start = daten["startBarSlots"]
        .as_u64()
        .expect("startBarSlots fehlt in levels.json");
    let plaetze_ab = stufen("barSlotLevels");
    let punkte_ab = stufen("talentPointLevels");
    let raenge = wege.iter().map(|w| w.raenge).max().expect("keine Wege");
    let punkte_voller_weg = 1 + usize::try_from(raenge).expect("zu groß");
    let stufe_voller_weg = *punkte_ab
        .get(punkte_voller_weg - 1)
        .unwrap_or_else(|| panic!("{punkte_voller_weg} Talentpunkte gibt es auf keiner Stufe"));
    Leiste {
        start: usize::try_from(start).expect("zu groß"),
        plaetze_ab,
        punkte_voller_weg,
        stufe_voller_weg,
    }
}

/// Wie lang eine gesuchte Liste werden darf.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Grenze {
    Zeilen(usize),
    /// Die Suche hört von selbst auf: `NoImprovement` oder `PoolExhausted`.
    Offen,
}

impl Grenze {
    fn max_len(self) -> usize {
        match self {
            Grenze::Zeilen(n) => n,
            Grenze::Offen => usize::MAX,
        }
    }

    fn text(self) -> String {
        match self {
            Grenze::Zeilen(n) => format!("höchstens {n} Zeilen"),
            Grenze::Offen => "ohne Zeilengrenze".to_owned(),
        }
    }
}

/// Fügt eine Grenze hinzu; fällt sie mit einer schon genannten zusammen,
/// bekommt jene den zweiten Grund dazu, statt zweimal gemessen zu werden.
fn dazu(grenzen: &mut Vec<(Grenze, String)>, grenze: Grenze, woher: String) {
    if let Some(da) = grenzen.iter_mut().find(|(g, _)| *g == grenze) {
        da.1 = format!("{}; {woher}", da.1);
    } else {
        grenzen.push((grenze, woher));
    }
}

/// Die vier Grenzen, je mit ihrem Grund, aufsteigend.
fn grenzen(leiste: &Leiste) -> Vec<(Grenze, String)> {
    let ab_stufe = |n: usize| match leiste.stufe_fuer(n) {
        Some(s) => format!("{n} Plätze hat die Leiste ab Stufe {s}"),
        None => format!("eine Leiste mit {n} Plätzen gibt es nicht"),
    };
    let mut aus: Vec<(Grenze, String)> = Vec::new();
    let voller_weg = leiste.plaetze(leiste.stufe_voller_weg);
    dazu(
        &mut aus,
        Grenze::Zeilen(voller_weg),
        format!(
            "die Leiste auf Stufe {}, ab der ein Weg voll ausgebaut werden kann \
             ({} Talentpunkte: ein Rang im Stamm, alle im Weg)",
            leiste.stufe_voller_weg, leiste.punkte_voller_weg
        ),
    );
    dazu(
        &mut aus,
        Grenze::Zeilen(ERSTE_FASSUNG),
        format!(
            "die Vorgabe der ersten Fassung, dort ohne Begründung ({})",
            ab_stufe(ERSTE_FASSUNG)
        ),
    );
    let hoechstens = leiste.hoechstens();
    dazu(
        &mut aus,
        Grenze::Zeilen(hoechstens),
        format!("die größte Leiste ({})", ab_stufe(hoechstens)),
    );
    dazu(
        &mut aus,
        Grenze::Offen,
        "bis die Suche selbst aufhört".to_owned(),
    );
    aus.sort_by_key(|(g, _)| g.max_len());
    aus
}

// ---------------------------------------------------------------------------
// Die Listen
// ---------------------------------------------------------------------------

/// Woher eine Liste kommt. Dieselbe Liste kann auf mehreren Wegen entstehen.
struct Herkunft {
    /// `frei` oder etwa `CALCINATION build a)`.
    name: String,
    /// Der Baum der Zeile, aus der sie stammt; `None` bei der freien Suche.
    baum: Option<String>,
    /// Wie sie entstand, mit dem Abbruchgrund der Suche.
    wie: String,
}

/// Eine Liste, gemessen ohne Talent und mit jedem der achtzehn Wege.
struct Liste {
    apl: Apl,
    herkunft: Vec<Herkunft>,
    ohne: BalanceResult,
    /// In der Reihenfolge von [`wege`].
    mit: Vec<BalanceResult>,
}

impl Liste {
    /// Der Name, unter dem die Liste in der Auswertung steht: ihre erste Herkunft.
    fn name(&self) -> &str {
        &self.herkunft[0].name
    }

    /// Ob sie für den Baum `baum` entstanden ist — die Zählung der ersten Fassung.
    fn aus_baum(&self, baum: &str) -> bool {
        self.herkunft
            .iter()
            .any(|h| h.baum.as_deref() == Some(baum))
    }
}

/// Nimmt eine Liste auf und misst sie — oder vermerkt nur die Herkunft, wenn
/// es dieselben Zeilen schon gibt.
///
/// Dieselben Zeilen messen dasselbe: Jeder Durchgang hat seinen festen
/// Startwert (`base_seed`), und `balance::run` sortiert die Einzelergebnisse
/// nach Startwert, bevor es rechnet (GAME-DESIGN 18.4). Gemessen: Zwei Läufe
/// der ersten Fassung gaben dieselbe Ausgabe, und gleiche Listen darin
/// dieselben Zahlen.
fn aufnehmen(listen: &mut Vec<Liste>, apl: Apl, herkunft: Herkunft, wege: &[Weg]) -> usize {
    let text = apl.to_string();
    if let Some(i) = listen.iter().position(|l| l.apl.to_string() == text) {
        listen[i].herkunft.push(herkunft);
        return i;
    }
    let ohne = rota_sim::balance::run(&config(MESS_DURCHGAENGE, None), &apl);
    let mit = wege
        .iter()
        .map(|w| rota_sim::balance::run(&config(MESS_DURCHGAENGE, Some(w)), &apl))
        .collect();
    listen.push(Liste {
        apl,
        herkunft: vec![herkunft],
        ohne,
        mit,
    });
    listen.len() - 1
}

/// Wie ein Weg in einer Liste gegen die beiden anderen Wege seines Baums
/// abschneidet.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lage {
    /// Er bewegt weder den Schaden noch den erlittenen Schaden noch die Tode.
    Wirkungslos,
    /// Er schlägt BEIDE Nachbarn im Schaden.
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

/// Wie Weg `i` in einer Liste abschneidet.
///
/// **Der Sieger muss beide Nachbarn SCHLAGEN.** Die erste Fassung dieses
/// Läufers nahm bei Gleichstand den ersten der drei — und meldete
/// `fixation-build` als erfüllt, obwohl alle drei Wege und die Rotation ohne
/// Talent dieselben 11498 ergaben. Ein Sieger aus Gleichstand ist eine
/// Reihenfolge, keine Messung.
///
/// **Und er muss überhaupt etwas bewegen.** Ein Talent kann den Schaden auch
/// senken — gemessen in der ersten Fassung: `coagulation-longer` drückte eine
/// Liste von 8152 auf 8032. Ein Weg ohne Wirkung stand dort mit 8152 vor ihm,
/// ohne etwas getan zu haben; ein Vorsprung allein ist deshalb kein Sieg.
fn lage(liste: &Liste, wege: &[Weg], i: usize) -> Lage {
    let eigener = &liste.mit[i];
    let ohne = &liste.ohne;
    let wirkt = !gleich(eigener.damage.mean, ohne.damage.mean)
        || !gleich(eigener.damage_taken.mean, ohne.damage_taken.mean)
        || eigener.deaths != ohne.deaths;
    if !wirkt {
        return Lage::Wirkungslos;
    }
    let bester_nachbar = wege
        .iter()
        .enumerate()
        .filter(|(j, w)| *j != i && w.baum == wege[i].baum)
        .map(|(j, _)| liste.mit[j].damage.mean)
        .fold(f64::MIN, f64::max);
    if eigener.damage.mean > bester_nachbar {
        Lage::Gewinnt
    } else if gleich(eigener.damage.mean, bester_nachbar) {
        Lage::Gleichstand
    } else {
        Lage::Verliert
    }
}

/// Gibt eine Liste mit allen ihren Messungen aus.
fn zeige(nummer: usize, liste: &Liste, wege: &[Weg]) {
    println!(
        "  [{}] Liste {nummer}: {} Zeilen, {} Zauber",
        liste.name(),
        liste.apl.rules.len(),
        zauber(&liste.apl)
    );
    for h in &liste.herkunft {
        println!("      entstanden als {}: {}", h.name, h.wie);
    }
    println!("      {}", einzeilig(&liste.apl));
    let mut gewirkt: Vec<String> = Vec::new();
    for (_, _, t) in TRAEGER {
        let zahl = liste.ohne.casts[index_of(t).expect("Träger fehlt")];
        let eintrag = format!("{t} {zahl}×");
        if zahl > 0 && !gewirkt.contains(&eintrag) {
            gewirkt.push(eintrag);
        }
    }
    println!(
        "      ohne Talent   {}   Träger gewirkt: {}",
        kurz(&liste.ohne),
        if gewirkt.is_empty() {
            "keiner".to_owned()
        } else {
            gewirkt.join(" · ")
        }
    );
    let mut baeume: Vec<&str> = Vec::new();
    for w in wege {
        if !baeume.contains(&w.baum.as_str()) {
            baeume.push(&w.baum);
        }
    }
    for baum in baeume {
        let mut werte: Vec<String> = Vec::new();
        let mut lagen: Vec<String> = Vec::new();
        for (i, w) in wege.iter().enumerate().filter(|(_, w)| w.baum == baum) {
            werte.push(format!("{} {}", w.pfad, kurz(&liste.mit[i])));
            lagen.push(format!("{} {}", w.pfad, lage(liste, wege, i).text()));
        }
        println!(
            "      {baum:<13} {}   -> {}",
            werte.join(" | "),
            lagen.join(" · ")
        );
    }
    println!();
}

// ---------------------------------------------------------------------------
// Die Auswertung
// ---------------------------------------------------------------------------

/// Ein Sieg eines Wegs in einer Liste, mit beiden Abständen zum Bezug.
struct Sieg {
    liste: usize,
    /// Die Liste OHNE Talent gegen den Bezug.
    abstand: f64,
    /// Dieselbe Liste MIT dem Talent des Wegs gegen den Bezug.
    abstand_mit: f64,
}

/// Die zwei Lesarten von „spielbar".
#[derive(Clone, Copy)]
enum Lesart {
    OhneTalent,
    MitTalent,
}

impl Lesart {
    fn abstand(self, s: &Sieg) -> f64 {
        match self {
            Lesart::OhneTalent => s.abstand,
            Lesart::MitTalent => s.abstand_mit,
        }
    }

    /// Der Lauf, an dem „spielbar" hängt — und damit auch der Vorbehalt.
    fn lauf(self, liste: &Liste, i: usize) -> &BalanceResult {
        match self {
            Lesart::OhneTalent => &liste.ohne,
            Lesart::MitTalent => &liste.mit[i],
        }
    }

    fn text(self) -> &'static str {
        match self {
            Lesart::OhneTalent => "ohne Talent",
            Lesart::MitTalent => "mit Talent",
        }
    }
}

/// Wo Weg `i` die Lesart erfüllt — der beste Beleg zuerst: ohne Tod vor mit
/// Tod, dann der kleinste Abstand.
fn erfuellt_in<'a>(siege: &'a [Sieg], listen: &[Liste], i: usize, lesart: Lesart) -> Vec<&'a Sieg> {
    let mut aus: Vec<&Sieg> = siege
        .iter()
        .filter(|s| lesart.abstand(s) <= SCHWELLE_PROZENT)
        .collect();
    aus.sort_by(|a, b| {
        let tod_a = lesart.lauf(&listen[a.liste], i).deaths > 0;
        let tod_b = lesart.lauf(&listen[b.liste], i).deaths > 0;
        tod_a
            .cmp(&tod_b)
            .then(lesart.abstand(a).total_cmp(&lesart.abstand(b)))
    });
    aus
}

/// Die Zelle der Tabelle: der beste Beleg, wie viele es noch gibt, und der
/// Vorbehalt, wenn der Spieler darin stirbt.
fn zelle(erfuellt: &[&Sieg], listen: &[Liste], i: usize, lesart: Lesart) -> String {
    let Some(beleg) = erfuellt.first() else {
        return "nein".to_owned();
    };
    let liste = &listen[beleg.liste];
    let mut teile = vec![format!("{} {:.1} %", liste.name(), lesart.abstand(beleg))];
    if erfuellt.len() > 1 {
        teile.push(format!("(+{} weitere)", erfuellt.len() - 1));
    }
    let lauf = lesart.lauf(liste, i);
    if lauf.deaths > 0 {
        teile.push(format!("— Vorbehalt: {} tot {}", lesart.text(), tod(lauf)));
    }
    teile.join(" ")
}

/// Was eine Lesart unter einer Grenze ergibt.
struct Zaehlung {
    /// Die erfüllten Wege, mit `(Vorbehalt)`, wenn jeder Beleg einen Tod trägt.
    wege: Vec<String>,
    /// Wie viele davon nur mit Vorbehalt.
    nur_vorbehalt: usize,
    /// Dieselbe Frage nur über die Listen des eigenen Baums.
    eigener_baum: usize,
}

impl Zaehlung {
    fn text(&self) -> String {
        format!(
            "{} von 18, davon {} nur mit Vorbehalt (Tod)",
            self.wege.len(),
            self.nur_vorbehalt
        )
    }
}

/// Was eine Grenze ergibt — eine Zeile des Überblicks am Ende.
struct Ergebnis {
    grenze: Grenze,
    bezug: f64,
    bezug_liste: String,
    bezug_zeilen: usize,
    bezug_zauber: usize,
    bezug_tod: String,
    frei: f64,
    frei_stop: Stop,
    listen: usize,
    ohne: Zaehlung,
    mit: Zaehlung,
}

/// Misst alle achtzehn Wege unter einer Grenze und gibt alles aus.
fn miss(grenze: Grenze, woher: &str, pool: &[Candidate], wege: &[Weg]) -> Ergebnis {
    let max_len = grenze.max_len();
    println!("\n################ {} ################", grenze.text());
    println!("  woher: {woher}\n");
    let mut listen: Vec<Liste> = Vec::new();

    // --- Schritt 1: die freie Suche ----------------------------------------
    let frei = greedy(&config(SUCH_DURCHGAENGE, None), pool, max_len);
    println!("== Schritt 1: die freie Suche ==\n");
    println!(
        "  Boss {BOSS}, 60 s, {SUCH_DURCHGAENGE} Durchgänge ab Startwert 5, {}, ohne Talente",
        grenze.text()
    );
    println!("  Kandidaten im Pool: {}", frei.pool_size);
    println!(
        "  Schaden {:.0}, {} Zeilen, {} Zauber, Abbruch {:?} nach {} Messungen",
        frei.best.mean,
        frei.best.apl.rules.len(),
        zauber(&frei.best.apl),
        frei.stop,
        frei.measured
    );
    if frei.stop == Stop::LengthLimit {
        println!(
            "  -> an der Zeilengrenze abgebrochen: die beste gefundene Liste mit {}, \
             nicht die beste überhaupt",
            grenze.text()
        );
    }
    println!("  {}\n", einzeilig(&frei.best.apl));
    aufnehmen(
        &mut listen,
        frei.best.apl.clone(),
        Herkunft {
            name: "frei".to_owned(),
            baum: None,
            wie: format!("freie Suche, Abbruch {:?}", frei.stop),
        },
        wege,
    );

    // --- Schritt 2: je Weg drei Listen ---------------------------------------
    println!("== Schritt 2: je Weg drei Listen ==\n");
    // Eine Suche je Träger, nicht je Weg: Zwei Wege mit demselben Träger
    // bekämen dieselbe Liste a).
    let mut suchen: BTreeMap<&'static str, SearchResult> = BTreeMap::new();
    for weg in wege {
        let felder: Vec<String> = weg.felder.iter().map(|(f, v)| format!("{f}={v}")).collect();
        let zeile_von = format!("{} {}", weg.baum, weg.pfad);
        let start = [zeile(weg.traeger, pool)];
        let gesucht = suchen
            .entry(weg.traeger)
            .or_insert_with(|| greedy_from(&config(SUCH_DURCHGAENGE, None), pool, max_len, &start));
        let a = aufnehmen(
            &mut listen,
            gesucht.best.apl.clone(),
            Herkunft {
                name: format!("{zeile_von} a)"),
                baum: Some(weg.baum.clone()),
                wie: format!(
                    "gesucht ab {}, ohne Talent, Abbruch {:?}",
                    weg.traeger, gesucht.stop
                ),
            },
            wege,
        );
        let traeger = index_of(weg.traeger).expect("Träger fehlt");
        let b = match vorgezogen(&gesucht.best.apl, traeger) {
            Some(vorn) => {
                let b = aufnehmen(
                    &mut listen,
                    vorn,
                    Herkunft {
                        name: format!("{zeile_von} b)"),
                        baum: Some(weg.baum.clone()),
                        wie: format!("a) mit {} an erster Stelle", weg.traeger),
                    },
                    wege,
                );
                listen[b].name().to_owned()
            }
            None => format!("entfällt, {} steht in a) schon vorn", weg.traeger),
        };
        let fuer_ihn = greedy_from(&config(SUCH_DURCHGAENGE, Some(weg)), pool, max_len, &start);
        let c = aufnehmen(
            &mut listen,
            fuer_ihn.best.apl.clone(),
            Herkunft {
                name: format!("{zeile_von} c)"),
                baum: Some(weg.baum.clone()),
                wie: format!(
                    "gesucht ab {} MIT {}, Abbruch {:?}",
                    weg.traeger, weg.talent, fuer_ihn.stop
                ),
            },
            wege,
        );
        println!(
            "  {zeile_von} ({})   {}   Träger {}",
            weg.talent,
            felder.join(", "),
            weg.traeger
        );
        println!(
            "      a) [{}]   b) [{b}]   c) [{}]",
            listen[a].name(),
            listen[c].name()
        );
    }
    println!();

    // --- Schritt 3: jede Liste ohne und mit jedem Weg ------------------------
    println!(
        "== Schritt 3: {} Listen, jede ohne Talent und mit jedem Weg ==\n",
        listen.len()
    );
    println!(
        "  je Messung: Schaden / erlittener Schaden / Tode des Spielers, \
         {MESS_DURCHGAENGE} Durchgänge, voller Ausbau des Wegs\n"
    );
    for (k, liste) in listen.iter().enumerate() {
        zeige(k + 1, liste, wege);
    }

    // --- Der Bezug: die beste Liste ohne Talent, die der Lauf kennt ----------
    //
    // Nur ein ECHT besserer Wert verdrängt die freie Suche. `max_by` gäbe bei
    // Gleichstand die LETZTE Liste zurück, und die Ausgabe meldete dann eine
    // Vergleichsliste als „besser als die freie Suche", die nur gleich ist.
    let mut bezug_index = 0;
    for (k, l) in listen.iter().enumerate() {
        if l.ohne.damage.mean > listen[bezug_index].ohne.damage.mean {
            bezug_index = k;
        }
    }
    let bezug_liste = &listen[bezug_index];
    let bezug = bezug_liste.ohne.damage.mean;
    assert!(bezug > 0.0, "keine Liste teilt Schaden aus");
    println!("== Auswertung {} ==\n", grenze.text());
    println!(
        "  Bezug: {bezug:.0} — Liste [{}] ({} Zeilen, {} Zauber, Tode {}), \
         die beste ohne Talent unter den {} Listen dieser Grenze",
        bezug_liste.name(),
        bezug_liste.apl.rules.len(),
        zauber(&bezug_liste.apl),
        tod(&bezug_liste.ohne),
        listen.len()
    );
    // Die Suche misst mit SUCH_DURCHGAENGE, der Vergleich mit MESS_DURCHGAENGE.
    // Weichen die beiden für dieselbe Liste ab, steht das hier, statt dass der
    // Bezug still eine andere Zahl trägt als Schritt 1.
    if !gleich(frei.best.mean, listen[0].ohne.damage.mean) {
        println!(
            "    ACHTUNG: die freie Suche maß {:.3} mit {SUCH_DURCHGAENGE} Durchgängen, \
             der Vergleich {:.3} mit {MESS_DURCHGAENGE}",
            frei.best.mean, listen[0].ohne.damage.mean
        );
    }
    if bezug_index == 0 {
        println!(
            "    das ist die Liste der freien Suche, Abbruch {:?}",
            frei.stop
        );
    } else {
        println!(
            "    besser als die freie Suche ({:.0}, Abbruch {:?}) — der Bezug ist deshalb sie",
            frei.best.mean, frei.stop
        );
    }
    println!(
        "  spielbar: höchstens {SCHWELLE_PROZENT:.0} % darunter, also ab {:.0}\n",
        bezug * (1.0 - SCHWELLE_PROZENT / 100.0)
    );

    // --- Je Weg: wo er gewinnt ------------------------------------------------
    let mut tabelle: Vec<String> = Vec::new();
    let mut ohne = Zaehlung {
        wege: Vec::new(),
        nur_vorbehalt: 0,
        eigener_baum: 0,
    };
    let mut mit = Zaehlung {
        wege: Vec::new(),
        nur_vorbehalt: 0,
        eigener_baum: 0,
    };
    for (i, weg) in wege.iter().enumerate() {
        let siege: Vec<Sieg> = listen
            .iter()
            .enumerate()
            .filter(|(_, l)| lage(l, wege, i) == Lage::Gewinnt)
            .map(|(k, l)| Sieg {
                liste: k,
                abstand: prozent_unter(bezug, l.ohne.damage.mean),
                abstand_mit: prozent_unter(bezug, l.mit[i].damage.mean),
            })
            .collect();
        println!(
            "  {} {} ({}, Träger {}): gewinnt in {} von {} Listen",
            weg.baum,
            weg.pfad,
            weg.talent,
            weg.traeger,
            siege.len(),
            listen.len()
        );
        for s in &siege {
            let l = &listen[s.liste];
            println!(
                "      [{}]  Abstand ohne Talent {:.1} % · mit Talent {:.1} %   \
                 ohne {} · mit {}",
                l.name(),
                s.abstand,
                s.abstand_mit,
                kurz(&l.ohne),
                kurz(&l.mit[i])
            );
        }
        let mut zellen: Vec<String> = Vec::new();
        for (lesart, zaehlung) in [
            (Lesart::OhneTalent, &mut ohne),
            (Lesart::MitTalent, &mut mit),
        ] {
            let erfuellt = erfuellt_in(&siege, &listen, i, lesart);
            if let Some(beleg) = erfuellt.first() {
                if lesart.lauf(&listen[beleg.liste], i).deaths > 0 {
                    zaehlung.nur_vorbehalt += 1;
                    zaehlung.wege.push(format!("{} (Vorbehalt)", weg.talent));
                } else {
                    zaehlung.wege.push(weg.talent.clone());
                }
            }
            if erfuellt.iter().any(|s| listen[s.liste].aus_baum(&weg.baum)) {
                zaehlung.eigener_baum += 1;
            }
            zellen.push(zelle(&erfuellt, &listen, i, lesart));
        }
        tabelle.push(format!(
            "| {} {} | {} | {} von {} | {} | {} |",
            weg.baum,
            weg.pfad,
            weg.traeger,
            siege.len(),
            listen.len(),
            zellen[0],
            zellen[1]
        ));
    }

    // --- Zusammenfassung --------------------------------------------------------
    println!(
        "\n== Zusammenfassung {}: 18 Wege gegen den Bezug {bezug:.0} [{}] ==\n",
        grenze.text(),
        bezug_liste.name()
    );
    println!("  AC3 je Lesart: die Liste mit dem kleinsten Abstand, ohne Tod vor mit Tod\n");
    println!("| Weg | Träger | gewinnt in | AC3 ohne Talent | AC3 mit Talent |");
    println!("| --- | --- | --- | --- | --- |");
    for z in &tabelle {
        println!("{z}");
    }
    println!(
        "\n  erfüllt, spielbar OHNE Talent gemessen: {}",
        ohne.text()
    );
    println!("    {}", ohne.wege.join(", "));
    println!(
        "  erfüllt, spielbar MIT dem Talent des Wegs gemessen: {}",
        mit.text()
    );
    println!("    {}", mit.wege.join(", "));
    println!(
        "  zum Vergleich, nur die Listen des eigenen Baums gewertet: \
         ohne Talent {} von 18, mit Talent {} von 18",
        ohne.eigener_baum, mit.eigener_baum
    );

    Ergebnis {
        grenze,
        bezug,
        bezug_liste: bezug_liste.name().to_owned(),
        bezug_zeilen: bezug_liste.apl.rules.len(),
        bezug_zauber: zauber(&bezug_liste.apl),
        bezug_tod: tod(&bezug_liste.ohne),
        frei: frei.best.mean,
        frei_stop: frei.stop,
        listen: listen.len(),
        ohne,
        mit,
    }
}

fn main() {
    let pool = all_candidates();
    let wege = wege();
    let leiste = leiste(&wege);
    let grenzen = grenzen(&leiste);

    println!("== Die Grenzen, unter denen gemessen wird ==\n");
    for (g, woher) in &grenzen {
        println!("  {:<22} {woher}", g.text());
    }

    let ergebnisse: Vec<Ergebnis> = grenzen
        .iter()
        .map(|(g, woher)| miss(*g, woher, &pool, &wege))
        .collect();

    println!("\n################ Überblick über alle Grenzen ################\n");
    println!(
        "| Grenze | freie Suche | Abbruch | Bezug | Liste | Zeilen | Zauber | Tode | Listen \
         | AC3 ohne Talent | AC3 mit Talent | nur eigener Baum, ohne / mit |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    for e in &ergebnisse {
        println!(
            "| {} | {:.0} | {:?} | {:.0} | {} | {} | {} | {} | {} | {} | {} | {} / {} |",
            e.grenze.text(),
            e.frei,
            e.frei_stop,
            e.bezug,
            e.bezug_liste,
            e.bezug_zeilen,
            e.bezug_zauber,
            e.bezug_tod,
            e.listen,
            e.ohne.text(),
            e.mit.text(),
            e.ohne.eigener_baum,
            e.mit.eigener_baum
        );
    }
    println!();
    for e in &ergebnisse {
        println!("  {}", e.grenze.text());
        println!("    ohne Talent: {}", e.ohne.wege.join(", "));
        println!("    mit Talent:  {}", e.mit.wege.join(", "));
    }
}
