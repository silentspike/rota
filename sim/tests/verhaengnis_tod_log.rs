//! **Der Verhängnis-Tod steht im Verlauf** — beide Wege, ein Eintrag (Issue #693)
//!
//! # Was der Befund war
//!
//! Zwei Stellen setzen `died_at`, und nur eine schrieb den Eintrag:
//!
//! ```text
//!   fight.rs  hit_player     hp == 0 nach Schaden  -> died_at UND LogKind::PlayerDeath
//!   fight.rs  doom_tick      Verhängnis-Schwelle   -> died_at, dann return
//! ```
//!
//! Wer den Verlauf eines Verhängnis-Todes las, fand den Spieler tot und
//! nirgends vermerkt, dass er gestorben war. Gemessen an drei Bossen:
//!
//! ```text
//!   boss-2-4         hp 0, damage_taken 5288   playerDeath JA   (Schadens-Tod)
//!   boss-1-2         hp 0, damage_taken  280   playerDeath NEIN
//!   test-doomsayer   hp 0, damage_taken 2800   playerDeath NEIN
//! ```
//!
//! Die erste Zeile ist die Gegenprobe, die den Befund eingrenzt: **Es liegt
//! nicht an der Schreibstelle** — die funktioniert. Es liegt am zweiten Weg
//! dorthin.
//!
//! # Warum der Eintrag denselben Wert trägt
//!
//! Er trägt `damage_taken`, wie am anderen Weg. Das ist die Entscheidung aus
//! AC1 und sie ist bewusst: Der Eintrag sagt „der Spieler ist gestorben, mit
//! so viel genommenem Schaden" — eine Aussage, die bei beiden Todesarten
//! dasselbe bedeutet.
//!
//! Den Verhängnis-Stand einzusetzen wäre naheliegend und falsch: Dieselbe
//! Zahl hieße dann je nach Todesart etwas anderes, und genau diese
//! Verwechslung haben #512 und #568 gekostet.
//!
//! Die **Todesart** steht damit nicht im Verlauf. Das ist bekannt und keine
//! Nachlässigkeit: Es gibt überhaupt keine Protokoll-Art für das Verhängnis,
//! auch nicht für Tick oder Schwelle. Soll sie hinein, braucht sie eine
//! eigene Art — ein eigenes Issue.
//!
//! # Wie dieser Fall den Weg auseinanderhält
//!
//! Ohne einen `doom`-Getter lässt sich von außen nicht ablesen, WORAN der
//! Spieler gestorben ist. Der Fall nutzt deshalb den **Zeitpunkt**: Der
//! Verhängnis-Tod tritt rechnerisch nach `threshold / amount` Schritten ein,
//! bei `test-doomsayer` also nach 100/5 = 20 Schritten à 1000 ms. Ein
//! Eintrag zu genau dieser Zeit kann kein Schadens-Tod sein.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::log::LogKind;
use rota_sim::{Fight, FightConfig};

/// Der Boss, an dem gemessen wird: Verhängnis 5 je Sekunde, Schwelle 100.
const DOOM_BOSS: &str = "test-doomsayer";

/// Wann das Verhängnis rechnerisch zuschlägt: 100 / 5 × 1000 ms.
const TOD_BEI_MS: u32 = 20_000;

/// Ein Lauf, der nur angreift — keine Senkung, keine Heilung.
fn lauf(boss_id: &str, dauer_ms: u32) -> Fight {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));
    let mut fight = Fight::new(FightConfig {
        boss,
        duration_ms: Some(dauer_ms),
        ..FightConfig::default()
    });
    fn nur_angriff(f: &Fight) -> Option<usize> {
        index_of("kindle").filter(|i| f.is_ready(*i))
    }
    fight.run(nur_angriff as fn(&Fight) -> Option<usize>);
    fight
}

/// Die Einträge der Art `PlayerDeath` als (Zeit, Wert).
fn tod_eintraege(fight: &Fight) -> Vec<(u32, u32)> {
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::PlayerDeath)
        .map(|e| (e.time, e.value))
        .collect()
}

#[test]
fn ac2_die_lage_toetet_den_spieler_wirklich_am_verhaengnis() {
    // **Die Zusicherung VOR dem Vergleich.** Stürbe der Spieler gar nicht
    // oder stürbe er am Schaden, prüfte der Fall darunter etwas anderes als
    // seinen Namen — und wäre grün, weil dann eben ein anderer Eintrag da ist.
    let fight = lauf(DOOM_BOSS, 300_000);

    assert_eq!(
        fight.player_hp(),
        0,
        "der Spieler überlebt bei {DOOM_BOSS} — die Lage misst nichts über den Tod"
    );

    let eintraege = tod_eintraege(&fight);
    assert_eq!(
        eintraege.len(),
        1,
        "erwartet genau einen Todes-Eintrag, gefunden {}",
        eintraege.len()
    );

    // Der Zeitpunkt trennt die beiden Wege: 100/5 Schritte à 1000 ms.
    let (zeit, _) = eintraege[0];
    assert_eq!(
        zeit, TOD_BEI_MS,
        "der Eintrag steht bei {zeit} ms, das Verhängnis schlägt bei \
         {TOD_BEI_MS} ms zu — dann ist es ein Schadens-Tod und der Fall \
         belegt nicht, was er soll"
    );
}

#[test]
fn ac2b_der_verhaengnis_tod_schreibt_den_eintrag() {
    // Der eigentliche Nachweis: Vor Issue #693 stand hier nichts.
    let fight = lauf(DOOM_BOSS, 300_000);
    let eintraege = tod_eintraege(&fight);

    assert!(
        !eintraege.is_empty(),
        "der Spieler stirbt am Verhängnis, und der Verlauf schweigt darüber — \
         genau der Befund aus #693"
    );
}

#[test]
fn ac4_der_wert_ist_der_genommene_schaden_und_nicht_fest() {
    // **Ein Eintrag mit immer derselben Zahl wäre von einer Konstanten nicht
    // zu unterscheiden.** Der Wert muss die Lage widerspiegeln, sonst trägt
    // er keine Aussage.
    let fight = lauf(DOOM_BOSS, 300_000);
    let eintraege = tod_eintraege(&fight);
    let (_, wert) = eintraege[0];

    assert_eq!(
        wert,
        fight.damage_taken(),
        "der Eintrag trägt {wert}, genommen wurden {} — der Wert ist nicht \
         der genommene Schaden",
        fight.damage_taken()
    );
    assert!(
        wert > 0,
        "der Wert ist 0 — dann sagt der Eintrag nichts über die Lage aus"
    );

    // Und die Gegenprobe an einem ZWEITEN Boss mit Verhängnis: derselbe Weg,
    // andere Zahl. Wäre der Wert fest verdrahtet, stünde hier dieselbe.
    let anderer = lauf("test-stacked", 300_000);
    let andere_eintraege = tod_eintraege(&anderer);
    assert!(
        !andere_eintraege.is_empty(),
        "test-stacked erzeugt keinen Todes-Eintrag — die Gegenprobe misst nichts"
    );
    let (_, anderer_wert) = andere_eintraege[0];
    assert_ne!(
        wert, anderer_wert,
        "beide Bosse ergeben denselben Wert {wert} — der Eintrag trägt dann \
         womöglich eine Konstante statt des genommenen Schadens"
    );
}

#[test]
fn ac4b_beide_wege_tragen_dieselbe_groesse() {
    // Die Entscheidung aus AC1 in einer Zeile: Der Schadens-Tod und der
    // Verhängnis-Tod schreiben denselben Wert — `damage_taken`. Wer das
    // ändert, bricht hier und liest den Grund im Kopf dieser Datei.
    let am_schaden = lauf("boss-2-4", 300_000);
    let am_verhaengnis = lauf(DOOM_BOSS, 300_000);

    for (name, fight) in [("boss-2-4", &am_schaden), (DOOM_BOSS, &am_verhaengnis)] {
        let eintraege = tod_eintraege(fight);
        assert_eq!(
            eintraege.len(),
            1,
            "{name}: erwartet genau einen Todes-Eintrag, gefunden {}",
            eintraege.len()
        );
        assert_eq!(
            eintraege[0].1,
            fight.damage_taken(),
            "{name}: der Eintrag trägt {}, genommen wurden {} — die beiden \
             Wege tragen verschiedene Größen",
            eintraege[0].1,
            fight.damage_taken()
        );
    }
}
