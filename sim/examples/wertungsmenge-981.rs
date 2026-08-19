//! Die Klassifikationstabelle (Issue #981).
//!
//! Der Bericht zu #981 verlangt sie vollstaendig. Sie hier zu erzeugen statt
//! von Hand zu schreiben, hat denselben Grund wie die abgeleitete
//! Schulenmenge: Eine abgeschriebene Tabelle altert, sobald ein Boss dazukommt
//! — und zwar lautlos.
//!
//! Zugleich ist dieses Programm der Nachweis am lebenden Objekt fuer AC5: Legt
//! man eine Bossdatei mit einem anderen `instance`-Wert an, aendert sich die
//! Gruppierung hier sichtbar.

use rota_sim::data;
use rota_sim::scoring::{class_of, probe_bosses, BossClass, ScoringSet};

fn main() {
    let wertung = ScoringSet::all_scored();
    let gruppen = wertung.groups();

    println!(
        "=== WERTUNGSFAEHIG: {} Bosse in {} Gruppen ===",
        wertung.len(),
        gruppen.len()
    );
    for gruppe in &gruppen {
        println!("\n  [{}]  {} Bosse", gruppe.instance, gruppe.len());
        for boss in gruppe.members() {
            println!(
                "      {:<16} {:<30} {:>6} Leben",
                boss.id, boss.name, boss.hp
            );
        }
    }

    let profile = probe_bosses();
    println!(
        "\n=== PRUEFPROFILE: {} Bosse, in KEINER Wertung ===",
        profile.len()
    );
    for boss in &profile {
        println!(
            "      {:<24} instance={:<10} {:>6} Leben",
            boss.id, boss.instance, boss.hp
        );
    }

    // Die Kontrollzeile: Beide Klassen zusammen sind der ganze Bestand. Ohne
    // sie koennten beide Zahlen stimmen, waehrend ein Boss durch beide faellt.
    println!(
        "\n=== {} + {} = {} von {} Bossen im Bestand ===",
        wertung.len(),
        profile.len(),
        wertung.len() + profile.len(),
        data::BOSSES.len()
    );

    // Und die Klasse je Instanz-Wert, damit sichtbar ist, WORAN die
    // Einteilung haengt — nicht an Kennungen, sondern an diesem einen Feld.
    println!("\n=== Instanz-Werte im Bestand ===");
    let mut gesehen: Vec<&str> = Vec::new();
    for boss in data::BOSSES.iter() {
        if gesehen.contains(&boss.instance) {
            continue;
        }
        gesehen.push(boss.instance);
        let anzahl = data::BOSSES
            .iter()
            .filter(|b| b.instance == boss.instance)
            .count();
        let klasse = match class_of(boss) {
            BossClass::Scored => "wertungsfaehig",
            BossClass::Probe => "Pruefprofil",
        };
        println!(
            "      {:<12} {:<16} {} Bosse",
            boss.instance, klasse, anzahl
        );
    }
}
