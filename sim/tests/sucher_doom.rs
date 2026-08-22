//! Die Rotationssuche bildet Bedingungen auf der Verhängnis-Uhr (Issue #1088).
//!
//! # Was vorher fehlte
//!
//! Seit #1074 kann eine **von Hand geschriebene** Prioritätenliste `doom`
//! abfragen. Die Suche konnte es nicht: Sie baut ihre Bedingungen aus festen
//! Formen, und keine davon trug einen der vier neuen Werte. Ein Boss, dessen
//! Signatur auf `doom` beruht, wurde gespielt, als hätte er kein `doom` —
//! gemessen an `boss-1-2`: 6836 Schaden mit `PURGE` gegen 2555 ohne, und
//! `PURGE` stand **unbedingt** in der Liste, weil es die einzige Form war
//! (`sim/examples/e1-1085-doom-suche.rs`).
//!
//! # Warum die Zeile in der Suche entsteht und nicht in `candidates_for`
//!
//! Alle bisherigen Formen leiten ihre Schwelle aus dem **Zauber** ab. Diese
//! braucht den **Boss** — `threshold`, `amount` und `clear_amount` stehen dort.
//! `candidates_for(index)` kennt ihn nicht, und ihn durchzureichen fasst 32
//! aufrufende Dateien an.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::rotation_search::{all_candidates, boss_kandidaten, greedy, Candidate, Origin};

const MIT_DOOM: &str = "boss-1-2";
const OHNE_DOOM: &str = "boss-1-1";
const MAX_LEN: usize = 6;

fn suche(boss_id: &str, pool: &[Candidate]) -> rota_sim::rotation_search::SearchResult {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));
    let config = BalanceConfig {
        iterations: 20,
        boss,
        ..BalanceConfig::default()
    };
    greedy(&config, pool, MAX_LEN)
}

fn purge_index() -> usize {
    SPELLS
        .iter()
        .position(|s| s.id == "purge")
        .expect("purge fehlt")
}

/// **AC1 (Fassung b)** — die Suche kann die Bedingung **bilden**.
///
/// Nicht: ob sie sie wählt. Das entscheidet die Bewertung, und sie darf
/// negativ ausgehen — gemessen ist sie es: An `boss-1-2` ist die bedingte
/// Zeile an keiner Schwelle besser als die unbedingte.
///
/// Beides in einem Kriterium zu prüfen hieße, zwei Fragen in einen Satz zu
/// packen. Wer die Schwelle so lange dreht, bis die Zeile gewählt wird, leitet
/// die Zahl aus dem Kriterium ab statt aus der Sache.
#[test]
fn ac1_die_suche_kann_eine_doom_bedingung_bilden() {
    let boss = boss_by_id(MIT_DOOM).expect("Boss fehlt");
    let config = BalanceConfig {
        iterations: 20,
        boss,
        ..BalanceConfig::default()
    };
    let gebildet = boss_kandidaten(&config, &all_candidates());
    assert!(
        !gebildet.is_empty(),
        "An {MIT_DOOM} (trägt doom) wurde keine Zeile gebildet."
    );
    for k in &gebildet {
        assert_eq!(
            k.origin,
            Origin::DoomClear,
            "falsche Herkunft: {:?}",
            k.origin
        );
        let zeile = format!("{}", k.rule());
        assert!(
            zeile.contains("doom"),
            "Die Zeile trägt Origin::DoomClear, nennt `doom` aber nicht: {zeile}"
        );
        assert!(
            SPELLS[k.spell].clears_doom,
            "Die Zeile gehört zu {}, der doom gar nicht senkt.",
            SPELLS[k.spell].id
        );
    }
}

/// **AC3, Auflage 3** — die Meldeliste ist am doom-Boss NICHT leer und am Boss
/// ohne doom LEER.
///
/// Die zweite Hälfte ist die eigentliche: Sie belegt, dass die Ergänzung
/// **boss-abhängig** ist. Ein Sucher, der die Zeile überall anhängt, wäre in
/// der ersten Hälfte ebenfalls grün.
#[test]
fn ac3_die_ergaenzung_haengt_am_boss() {
    let mit = suche(MIT_DOOM, &all_candidates());
    let ohne = suche(OHNE_DOOM, &all_candidates());
    assert!(
        !mit.ergaenzt.is_empty(),
        "An {MIT_DOOM} (trägt doom) wurde nichts ergänzt — dann kann die \
         Meldezeile nie etwas enthalten, und sie wäre eine Behauptung."
    );
    assert!(
        ohne.ergaenzt.is_empty(),
        "An {OHNE_DOOM} (kein doom) wurde ergänzt: {:?}. Dann hängt die \
         Ergänzung nicht am Boss, sondern passiert immer.",
        ohne.ergaenzt
    );
}

/// **Auflage 1** — was nicht im Pool steht, wird nicht ergänzt.
///
/// Ohne diese Zusicherung wäre jede Gegenprobe still kaputt, die einen Zauber
/// aus dem Vorrat filtert, um seine Wirkung zu messen: Die Suche legte ihn
/// wieder hinein, und das Ergebnis sähe aus wie vorher.
#[test]
fn auflage1_gefilterte_zauber_kommen_nicht_zurueck() {
    let purge = purge_index();
    let ohne_purge: Vec<Candidate> = all_candidates()
        .into_iter()
        .filter(|k| k.spell != purge)
        .collect();

    // ZUSICHERUNG: Der Filter muss überhaupt etwas entfernt haben.
    assert!(
        ohne_purge.len() < all_candidates().len(),
        "Der Filter hat nichts entfernt — dann prüft dieser Fall nichts."
    );

    // TRÄGERZEILE IM SELBEN FALL: Mit vollem Pool MUSS etwas ergänzt werden.
    // Ohne sie wäre dieser Fall auch dann grün, wenn `boss_kandidaten` gar
    // nichts mehr bildet — dann ist `ergaenzt` trivial leer und die Aussage
    // entwertet. Die positive Kontrolle steht sonst nur in `ac3`, also in
    // einem ANDEREN Fall: Fällt der, bleibt dieser grün und behauptet weiter
    // etwas.
    assert!(
        !suche(MIT_DOOM, &all_candidates()).ergaenzt.is_empty(),
        "Mit vollem Pool wurde nichts ergänzt — dann prüft dieser Fall nicht \
         den Filter, sondern nur, dass überhaupt nichts passiert."
    );

    let ergebnis = suche(MIT_DOOM, &ohne_purge);
    assert!(
        ergebnis.ergaenzt.is_empty(),
        "purge wurde aus dem Vorrat gefiltert und von der Suche wieder \
         ergänzt: {:?}",
        ergebnis.ergaenzt
    );
    assert!(
        !ergebnis.chosen.iter().any(|k| k.spell == purge),
        "purge steht in der gefundenen Liste, obwohl er nicht im Vorrat war."
    );
}

/// **AC2** — die Schwelle kommt aus den Bossdaten, nicht aus einer Konstante.
///
/// Geprüft wird die **gebildete** Zeile, nicht die gewählte — aus demselben
/// Grund wie bei AC1.
///
/// # Warum die Zahlen festgenagelt sind und nicht nur ihre Ungleichheit
///
/// Ein `assert_ne!` fängt eine **konstante** Schwelle, aber keine **falsche**.
/// Ein Vorzeichenfehler — `threshold + vorlauf` statt `- vorlauf` — ergäbe 109
/// und 103: ebenfalls verschieden, ebenfalls grün, und vollkommen unbrauchbar,
/// weil oberhalb von `threshold` gesenkt würde, also nie.
///
/// Die Zahlen sind gerechnet **und an einem Lauf abgelesen**, bevor sie hier
/// standen (2026-08-22):
///
/// | Boss | `threshold` | `amount` | Schwelle |
/// | --- | ---: | ---: | ---: |
/// | `boss-1-2` | 100 | 6 | 91 |
/// | `boss-3-4` | 100 | 2 | 97 |
///
/// `threshold` ist bei **jedem** doom-Boss 100 — eine Ableitung allein daraus
/// wäre überall gleich und käme formal trotzdem aus den Daten. Deshalb trennt
/// erst `amount` die beiden Fälle.
#[test]
fn ac2_die_schwelle_stammt_aus_den_bossdaten() {
    let zeile = |boss_id: &str| -> String {
        let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));
        let config = BalanceConfig {
            iterations: 5,
            boss,
            ..BalanceConfig::default()
        };
        boss_kandidaten(&config, &all_candidates())
            .first()
            .map_or_else(|| "keine".to_string(), |k| format!("{}", k.rule()))
    };
    assert_eq!(
        zeile("boss-1-2"),
        "purge if doom >= 91",
        "boss-1-2: threshold 100 minus amount 6 mal einem GCD sind 91."
    );
    assert_eq!(
        zeile("boss-3-4"),
        "purge if doom >= 97",
        "boss-3-4: threshold 100 minus amount 2 mal einem GCD sind 97. \
         Weicht nur diese ab, hängt die Schwelle nicht an `amount`."
    );
}
