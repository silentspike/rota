//! Findet die Rotationssuche an einem `doom`-Boss eine andere Liste, wenn
//! `PURGE` im Vorrat liegt? (Issue #1085)
//!
//! # Was diese Messung beantworten kann — und was nicht
//!
//! Sie zeigt, **was** die Suche mit `PURGE` tut: ob sie ihn wählt, wie oft er
//! im Kampf geworfen wird, ob sich der Schaden ändert.
//!
//! Sie beantwortet **nicht**, ob die Sprache der Engpass war. Der Grund steht
//! im Code und ist vor dieser Messung erhoben worden: `rotation_search.rs`
//! **baut** seine Kandidatenzeilen selbst und kennt genau fünf Formen —
//! `DotRemains < gcd`, `Resource >= schwelle`, `BuffActive >= 1`,
//! `BossCasting >= 1`, `BossNextStrikeIn <= fenster`. Keine davon kann `doom`
//! tragen; eine Suche nach `Doom|Afflicted|RaceProgress` in
//! `rotation_search.rs` und `objective.rs` findet null Treffer, gegen 16 auf
//! `Value::` als Kontrolle.
//!
//! **Die Suche kann eine `doom`-Bedingung also gar nicht bilden.** Findet sie
//! keine, ist das kein Beleg dafür, dass die Sprache nicht der Engpass war —
//! es ist der Beleg für einen zweiten Engpass (#1088).
//!
//! # Der Aufbau
//!
//! Zwei Läufe je Boss, gleiche Konfiguration, gleicher Startwert. Der einzige
//! Unterschied ist der Vorrat: einmal **mit** allen Kandidatenzeilen für
//! `PURGE`, einmal **ohne**.
//!
//! Die Gegenprobe an einem Boss **ohne** `doom` gehört dazu: Ändert sich dort
//! zwischen den zwei Läufen etwas, misst der Aufbau nicht den Baustein,
//! sondern den Vorrat.
//!
//! # Das Ergebnis vom 2026-08-22 — der Bezugspunkt für #1088
//!
//! | Boss | Vorrat | Schaden | `purge` in der Liste |
//! | --- | --- | ---: | --- |
//! | `boss-1-2` (doom) | mit `purge` | **6836** | ja, **ohne Bedingung** |
//! | `boss-1-2` (doom) | ohne `purge` | **2555** | — |
//! | `boss-1-1` (kein doom) | mit `purge` | 5810 | nein |
//! | `boss-1-1` (kein doom) | ohne `purge` | 5810 | — |
//!
//! Faktor 2,7 am doom-Boss: Ohne `purge` läuft die Uhr voll, der Spieler
//! stirbt, der Schaden bricht ab. Der Zauber ist dort keine Verbesserung,
//! sondern die Bedingung dafür, überhaupt zu spielen.
//!
//! Am Boss **ohne** `doom` sind beide Listen wortgleich und der Schaden
//! identisch — der Aufbau misst also den Baustein und nicht den Vorrat.
//!
//! **Wer #1088 baut, stellt seinen Lauf gegen diese Zahlen.** Ein Bezugspunkt,
//! den man nicht nachfahren kann, ist eine Behauptung — deshalb liegt dieses
//! Programm im Repository und nicht nur sein Ergebnis im Issue.
//!
//! Weicht ein späterer Lauf ab, ist die erste Frage nicht „hat sich der Sucher
//! geändert", sondern „messe ich denselben Aufbau": gleiche Bosse, gleiches
//! `MAX_LEN`, gleiche Iterationszahl, gleicher Vorrat.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::rotation_search::{all_candidates, greedy, Candidate};

/// Trägt `doom` — hier soll `PURGE` etwas ausmachen.
const MIT_DOOM: &str = "boss-1-2";
/// Trägt kein `doom` — hier darf sich nichts ändern (AC4).
const OHNE_DOOM: &str = "boss-1-1";

const MAX_LEN: usize = 6;

fn purge_index() -> usize {
    SPELLS
        .iter()
        .position(|s| s.id == "purge")
        .expect("purge fehlt im Zauberbuch")
}

fn messen(boss_id: &str, pool: &[Candidate], etikett: &str) {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));
    let config = BalanceConfig {
        iterations: 20,
        boss,
        ..BalanceConfig::default()
    };
    let ergebnis = greedy(&config, pool, MAX_LEN);

    println!("\n=== {boss_id}  {etikett} ===");
    println!("  Vorrat: {} Kandidatenzeilen", ergebnis.pool_size);
    println!(
        "  Schaden: {:.1} +/- {:.1}",
        ergebnis.best.mean, ergebnis.best.stderr
    );
    println!("  gefundene Liste:");
    for regel in &ergebnis.best.apl.rules {
        println!("    {}", SPELLS[regel.spell].id);
    }
    println!("  im Wortlaut:");
    for zeile in format!("{}", ergebnis.best.apl).lines() {
        println!("    {zeile}");
    }

    // AC3 — DIE TRÄGERZEILE. Ohne sie hieße "keine doom-Bedingung gefunden"
    // auch dann Entwarnung, wenn PURGE aus einem ganz anderen Grund nie zur
    // Wahl stand.
    let purge = purge_index();
    let im_vorrat = pool.iter().filter(|c| c.spell == purge).count();
    let gewaehlt = ergebnis
        .best
        .apl
        .rules
        .iter()
        .filter(|r| r.spell == purge)
        .count();
    println!("  TRAEGER  purge im Vorrat: {im_vorrat}   in der Liste: {gewaehlt}");
}

fn main() {
    let voll = all_candidates();
    let purge = purge_index();
    let ohne_purge: Vec<Candidate> = voll.iter().filter(|c| c.spell != purge).cloned().collect();

    // ZUSICHERUNG: Die beiden Vorräte müssen sich unterscheiden. Sind sie
    // gleich, gab es für PURGE gar keine Kandidatenzeile — dann vergleicht
    // die Messung zweimal dasselbe und jedes "kein Unterschied" wäre trivial.
    let entfernt = voll.len() - ohne_purge.len();
    println!(
        "ZUSICHERUNG  Kandidatenzeilen gesamt {}, davon fuer purge {entfernt}",
        voll.len()
    );
    assert!(
        entfernt > 0,
        "Es gibt keine einzige Kandidatenzeile fuer purge. Dann sind beide \
         Vorraete gleich und die ganze Messung vergleicht zweimal dasselbe."
    );

    for boss in [MIT_DOOM, OHNE_DOOM] {
        messen(boss, &voll, "MIT purge im Vorrat");
        messen(boss, &ohne_purge, "OHNE purge im Vorrat");
    }
}
