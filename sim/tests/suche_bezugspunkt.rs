//! Der Bezugspunkt für verhinderten Schaden (Issue #814).
//!
//! **Was ohne ihn passierte.** Der geerbte Commit aus #472 brachte die Formel
//! mit, aber keinen Bezugspunkt: `damage_taken_baseline` stand auf `0.0`, und
//! damit rechnete `(0.0 - erlitten).max(0.0)` in jedem Fall zu null. Eine
//! Größe, die aussieht als würde sie zählen, und es nie tut — und kein
//! einziger Prüffall fasste eines der beiden Felder an.
//!
//! **Die leere Rotation ist der Bezugspunkt** (`rotation_search::greedy`): Sie
//! erleidet alles, was der Boss austeilt, weil ihm niemand in den Arm fällt.
//!
//! Gemessen wird hier an der WIRKUNG, nicht am Feld: Ändert sich die gewählte
//! Rotation, wenn `prevented_weight` steigt? Ein Test, der nur nachsieht, ob
//! `damage_taken_baseline` nach dem Lauf ungleich null ist, bliebe grün, wenn
//! die Zahl gesetzt und danach ignoriert würde.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::rotation_search::{all_candidates, greedy};

/// Kurze Läufe: Gefragt ist, OB sich die Wahl ändert, nicht wie viel Schaden
/// dabei herauskommt. Zwanzig Durchgänge und fünf Zeilen reichen dafür und
/// halten die Datei unter einer Minute.
fn rotation(boss: &str, gewicht: f64) -> Vec<&'static str> {
    let cfg = BalanceConfig {
        iterations: 20,
        threads: 2,
        stop_on_kill: false,
        boss: boss_by_id(boss).expect("Boss fehlt"),
        prevented_weight: gewicht,
        ..BalanceConfig::default()
    };
    greedy(&cfg, &all_candidates(), 5)
        .chosen
        .iter()
        .map(|k| SPELLS[k.spell].id)
        .collect()
}

/// **Träger-Zusicherung.** Ohne einen Boss, der überhaupt Schaden austeilt,
/// gäbe es nichts zu verhindern — die Baseline wäre null, jeder Vergleich
/// unten träfe zu, und die Datei bliebe grün, ohne etwas zu prüfen.
#[test]
fn ac_der_pruefboss_teilt_ueberhaupt_schaden_aus() {
    let boss = boss_by_id("boss-2-4").expect("Boss fehlt");
    let austeilend = boss.timeline.iter().filter(|e| e.damage > 0).count();
    assert!(
        austeilend > 0,
        "boss-2-4 teilt keinen Schaden aus. Dann ist die Baseline null, und \
         jeder folgende Vergleich wäre richtig gemessen und ohne Aussage."
    );
}

/// Das Gewicht ändert die Wahl — der Kern von #814.
///
/// **Vorher war jede Rotation bei jedem Gewicht identisch.** Genau das ist die
/// Aussage: Die Größe war dekorativ, weil ihr Bezugspunkt fehlte. Bricht
/// dieser Fall, ist sie es wieder.
#[test]
fn ac_das_gewicht_aendert_die_gewaehlte_rotation() {
    let ohne = rotation("boss-2-4", 0.0);
    let mit = rotation("boss-2-4", 5.0);
    assert_ne!(
        ohne, mit,
        "Mit und ohne Gewicht steht dieselbe Rotation da. Dann geht der \
         verhinderte Schaden nicht in die Bewertung ein — entweder fehlt der \
         Bezugspunkt, oder er wird gesetzt und nicht gelesen."
    );
}

/// Was neu hinzukommt, VERHINDERT Schaden — es sind nicht irgendwelche anderen
/// Zauber.
///
/// Ohne diesen Fall wäre `ac_das_gewicht_aendert_die_gewaehlte_rotation` auch
/// erfüllt, wenn das Gewicht die Suche bloß durcheinanderbrächte: Zwei
/// Rotationen mit gleichem Wert und verschiedener Reihenfolge sind ebenfalls
/// „nicht gleich".
#[test]
fn ac_die_neuen_zauber_verhindern_schaden() {
    let ohne = rotation("boss-2-4", 0.0);
    let mit = rotation("boss-2-4", 5.0);
    let neu: Vec<&str> = mit.iter().filter(|s| !ohne.contains(s)).copied().collect();
    assert!(!neu.is_empty(), "Kein einziger Zauber kam hinzu: {mit:?}");

    let verhindernd = neu.iter().any(|id| {
        let spell = SPELLS
            .iter()
            .find(|s| s.id == *id)
            .expect("Zauber aus der Rotation gibt es nicht");
        spell.shield.is_some() || spell.heal.is_some() || spell.control.is_some()
    });
    assert!(
        verhindernd,
        "Neu hinzugekommen sind {neu:?} — keiner davon trägt Schild, Heilung \
         oder Kontrolle. Dann hat das Gewicht die Suche verschoben, statt \
         Verhindern zu belohnen."
    );
}

/// **Bei `prevented_weight = 0` bleibt alles, wie es war.**
///
/// Der Bezugspunkt kostet einen zusätzlichen Lauf. Er wird deshalb nur
/// gemessen, wenn das Gewicht ihn braucht — und wer das ändert, macht jede
/// frühere Balance-Zahl still unvergleichbar.
#[test]
fn ac_ohne_gewicht_bleibt_die_wahl_reproduzierbar() {
    assert_eq!(
        rotation("boss-2-4", 0.0),
        rotation("boss-2-4", 0.0),
        "Zwei Läufe ohne Gewicht liefern verschiedene Rotationen. Dann hängt \
         die Suche an etwas, das zwischen zwei Läufen wechselt — und keine \
         Balance-Zahl wäre vergleichbar."
    );
}
