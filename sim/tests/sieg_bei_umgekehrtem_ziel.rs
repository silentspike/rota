//! Der Sieg bei einem Boss mit umgekehrtem Ziel (`runaway`).
//!
//! # Warum diese Datei existiert
//!
//! Der Simulator zählte bis zum 2026-08-19 gegen `boss-2-3` die **Niederlagen
//! als Siege**. `fight.rs` setzt `killed_at`, sobald der ausgeteilte Schaden
//! die Lebenspunkte erreicht — bei einem Boss mit `runaway` ist genau das der
//! Überdruck und damit die Niederlage. `BalanceResult.kills` zählte daraus.
//!
//! Gemessen, je 20 Durchgänge über `rota-sim run --json`:
//!
//! ```text
//! apl/e939-schutz.apl       übersteht die Frist   kills meldete  0
//! apl/e607-flashpoint.apl   erzeugt Überdruck     kills meldete 20
//! ```
//!
//! In **beide** Richtungen das Gegenteil.
//!
//! # Warum die Paritätsprüfung es nicht gefangen hat
//!
//! `boss-2-3` liegt in den Vergleichsdateien — 28 Bosse sind dort vertreten.
//! Aber **keine einzige Aufzeichnung führt ein Zustandsfeld**: weder `state`
//! noch `FIGHT_WON`, `FIGHT_OVERPRESSURE`, `won` oder `outcome`. Verglichen
//! werden Schaden, Ticks und Zustands-Prüfsummen — der **Ausgang** nicht.
//!
//! Die Prüfung deckte alles ab außer der Frage, wer gewonnen hat, und genau
//! dort liefen die Seiten auseinander. Diese Datei schließt die Lücke auf der
//! Rust-Seite; sie prüft die Regel gegen die Tabelle aus `combat.ts`, nicht
//! gegen eine Aufzeichnung.
//!
//! # Die Regel, gegen die geprüft wird (GAME-DESIGN 11.2, Issue #247)
//!
//! ```text
//!              | Ring auf 0 | Zeit abgelaufen
//! gewöhnlich   | Sieg       | Niederlage
//! runaway      | Niederlage | Sieg
//! ```
//!
//! **Beide** Ausgänge kehren sich um. Nur einen zu drehen wäre keine Umkehrung
//! des Ziels, sondern ein zusätzlicher Verlustweg — deshalb prüft jeder Fall
//! hier beide Richtungen und nicht nur die auffällige.

use rota_sim::balance::{siege, Sample};
use rota_sim::data::boss_by_id;
use rota_sim::time::Millis;

/// Ein Durchgang mit frei gesetztem Ausgang.
///
/// Die übrigen Felder tragen Nullen: Dieser Prüfstand fragt allein, wie ein
/// gegebener Ausgang **gedeutet** wird — nicht, ob der Kampf ihn richtig
/// erzeugt. Das ist die Trennung, an der es vorher fehlte.
///
/// **Ohne `..Default::default()` und das ist Absicht.** Die vollständige
/// Aufzählung bricht den Bau, sobald `Sample` ein Feld dazubekommt — und wer es
/// hinzufügt, muss dann entscheiden, ob es den Ausgang mitbestimmt. Ein
/// Auffangzweig würde diese Entscheidung stillschweigend mit Null beantworten.
fn durchgang(seed: u32, hp_erreicht: Option<Millis>, gestorben: Option<Millis>) -> Sample {
    Sample {
        seed,
        damage: 0,
        killed_at_ms: hp_erreicht,
        damage_taken: 0,
        absorbed: 0,
        healed: 0,
        boss_healed: 0,
        died: gestorben.is_some(),
        died_at_ms: gestorben,
    }
}

#[test]
fn bei_gewoehnlichem_boss_ist_das_erreichen_der_lebenspunkte_der_sieg() {
    let boss = boss_by_id("boss-1-1").expect("boss-1-1 fehlt");
    assert!(
        boss.runaway.is_none(),
        "Kontrollzeile: boss-1-1 muss ein GEWOEHNLICHER Boss sein, sonst prueft \
         dieser Fall das Gegenteil dessen, was sein Name sagt"
    );

    let proben = [
        durchgang(1, Some(5000), None),
        durchgang(2, Some(6000), None),
        durchgang(3, None, None),
    ];

    assert_eq!(
        siege(&proben, boss),
        2,
        "Bei einem gewoehnlichen Boss zaehlt jeder Durchgang als Sieg, in dem \
         die Lebenspunkte fielen"
    );
}

#[test]
fn bei_umgekehrtem_ziel_ist_das_erreichen_der_lebenspunkte_die_niederlage() {
    let boss = boss_by_id("boss-2-3").expect("boss-2-3 fehlt");
    assert!(
        boss.runaway.is_some(),
        "Kontrollzeile: boss-2-3 muss das UMGEKEHRTE Ziel tragen. Ohne diese \
         Zusicherung wuerde der Fall still zum Zwilling des vorigen, sobald \
         jemand die Bossdaten aendert — und bliebe gruen"
    );

    let proben = [
        durchgang(1, Some(5000), None),
        durchgang(2, Some(6000), None),
        durchgang(3, None, None),
    ];

    assert_eq!(
        siege(&proben, boss),
        1,
        "Bei umgekehrtem Ziel ist das Erreichen der Lebenspunkte der UEBERDRUCK. \
         Sieg ist das Ueberstehen der Frist — hier genau ein Durchgang"
    );
}

#[test]
fn dieselben_durchgaenge_ergeben_an_beiden_bosstypen_verschiedene_siegzahlen() {
    // **Der Fall, der die Regel als Regel prueft.** Die beiden Faelle darueber
    // koennten einzeln bestehen, wenn `siege` eine feste Zahl zurueckgaebe, die
    // zufaellig zu beiden Erwartungen passt. Hier laufen DIESELBEN Proben durch
    // beide Bosstypen, und die Zahlen muessen sich unterscheiden.
    //
    // **Die Aufteilung ist UNGLEICH, und das ist der Punkt.** Meine erste
    // Fassung nahm zwei und zwei — dann liefert auch die kaputte Auswertung
    // a = 2 und b = 2, die Summe geht auf, und der Fall bleibt gruen. Gemessen
    // in der Mutationsprobe: Er war der einzige der fuenf, der den alten
    // Zustand NICHT gefangen hat, obwohl sein Name genau das verspricht.
    //
    // Drei zu eins trennt die beiden Welten: richtig 3 gegen 1, kaputt 3
    // gegen 3.
    let gewoehnlich = boss_by_id("boss-1-1").expect("boss-1-1 fehlt");
    let umgekehrt = boss_by_id("boss-2-3").expect("boss-2-3 fehlt");

    let proben = [
        durchgang(1, Some(5000), None),
        durchgang(2, Some(6000), None),
        durchgang(3, Some(7000), None),
        durchgang(4, None, None),
    ];

    let a = siege(&proben, gewoehnlich);
    let b = siege(&proben, umgekehrt);

    assert_eq!(a, 3, "gewoehnlich: drei erreichte Lebenspunkte");
    assert_eq!(b, 1, "umgekehrt: eine ueberstandene Frist");
    assert_ne!(
        a, b,
        "Die beiden Bosstypen MUESSEN sich bei dieser Probenmenge unterscheiden \
         — sonst kann der Fall die fehlende Umkehrung nicht sehen"
    );
    assert_eq!(
        a + b,
        proben.len(),
        "Jeder Durchgang ist an GENAU EINEM der beiden Bosstypen ein Sieg. \
         Geht die Summe nicht auf, ist die Umkehrung keine Umkehrung, sondern \
         eine zweite Bedingung daneben"
    );
}

#[test]
fn der_spielertod_dreht_sich_nicht_mit() {
    // Die Umkehrung betrifft das ZIEL, nicht den eigenen Tod. Ein toter Spieler
    // hat auch dann verloren, wenn der Boss davonlaeuft — sonst gewaenne er
    // durch Sterben. `combat.ts` haelt denselben Satz an derselben Stelle fest.
    let umgekehrt = boss_by_id("boss-2-3").expect("boss-2-3 fehlt");

    let ueberlebt = [durchgang(1, None, None)];
    let gestorben = [durchgang(1, None, Some(15_000))];

    assert_eq!(
        siege(&ueberlebt, umgekehrt),
        1,
        "Frist ueberstanden, nicht gestorben: Sieg"
    );
    assert_eq!(
        siege(&gestorben, umgekehrt),
        0,
        "Frist ueberstanden, aber gestorben: KEIN Sieg. Waere das anders, \
         gewaenne ein toter Spieler dadurch, dass der Boss davonlaeuft"
    );
}

#[test]
fn ohne_durchgaenge_gibt_es_null_siege_und_keinen_absturz() {
    // Eine leere Menge ist der Fall, den ein `filter().count()` still richtig
    // macht und ein handgeschriebener Index falsch. Er steht hier, damit eine
    // spaetere Umschreibung ihn nicht verliert.
    let leer: [Sample; 0] = [];
    for id in ["boss-1-1", "boss-2-3"] {
        let boss = boss_by_id(id).expect("Boss fehlt");
        assert_eq!(siege(&leer, boss), 0, "{id}: leere Menge, null Siege");
    }
}
