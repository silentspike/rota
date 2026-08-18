//! MESSWERKZEUG DES ORCHESTRATORS — nicht committen.
//!
//! Prueft die "Same-Spell-Subsumption": Steht eine BEDINGUNGSLOSE Zeile eines
//! Zaubers vor einer BEDINGTEN Zeile DESSELBEN Zaubers, kann die zweite nie
//! gewaehlt werden — ist der Zauber bereit, gewinnt die erste; ist er es
//! nicht, ist er es auch in der zweiten nicht.
//!
//! Frage: Wie viele solche Paare gibt es ueberhaupt, und was faellt dadurch
//! aus dem Suchraum?

use std::collections::BTreeMap;

use rota_sim::rotation_search::{all_candidates, Origin};

fn main() {
    let alle = all_candidates();
    println!("Kandidatenzeilen gesamt: {}", alle.len());

    // Je Zauber: hat er eine bedingungslose Zeile, und wie viele bedingte?
    let mut ohne: BTreeMap<usize, usize> = BTreeMap::new();
    let mut mit: BTreeMap<usize, usize> = BTreeMap::new();
    for k in &alle {
        if matches!(k.origin, Origin::Always) {
            *ohne.entry(k.spell).or_default() += 1;
        } else {
            *mit.entry(k.spell).or_default() += 1;
        }
    }

    let mut paare = 0usize;
    let mut zauber_mit_paar = 0usize;
    for (spell, n_bedingt) in &mit {
        if ohne.contains_key(spell) {
            zauber_mit_paar += 1;
            paare += n_bedingt;
        }
    }

    println!();
    println!("  Zauber mit bedingungsloser Zeile          {}", ohne.len());
    println!("  Zauber mit mindestens einer bedingten     {}", mit.len());
    println!("  Zauber mit BEIDEM (subsumierbar)          {zauber_mit_paar}");
    println!("  betroffene bedingte Zeilen                {paare}");
    println!();

    // Was faellt aus dem Raum? Eine Rotation ist ungueltig, sobald sie ein
    // solches Paar in der Reihenfolge (ohne ... mit) enthaelt.
    // Untere Schranke: der Anteil der Paare, die in beliebiger Reihenfolge
    // vorkommen koennen — die Haelfte davon ist subsumiert.
    let n = alle.len() as f64;
    println!("== Was die Regel aus dem Raum nimmt ==");
    println!("  Fuer JEDES der {paare} Paare gilt: von den beiden Anordnungen");
    println!("  (bedingungslos zuerst) und (bedingt zuerst) ist die erste tot.");
    println!();
    println!("  Eine Rotation der Laenge L, die beide Zeilen eines Paares");
    println!("  enthaelt, ist mit Wahrscheinlichkeit 1/2 betroffen.");
    println!();
    for l in [6usize, 12] {
        // Anteil der Listen, die mindestens ein Paar vollstaendig enthalten,
        // grob geschaetzt ueber die Wahrscheinlichkeit, dass beide Zeilen
        // eines gegebenen Paares in der Liste sind.
        let p_eins = (l as f64 / n) * ((l - 1) as f64 / (n - 1.0));
        let p_keins = (1.0 - p_eins).powi(paare as i32);
        println!(
            "  Laenge {l:>2}: mindestens ein Paar in der Liste  {:>5.1} %  -> davon die Haelfte tot",
            (1.0 - p_keins) * 100.0
        );
    }
    println!();
    println!("  ACHTUNG: das ist eine WAHRSCHEINLICHKEITSRECHNUNG ueber");
    println!("  zufaellige Listen, keine Messung an der Suche. Die Suche");
    println!("  zieht nicht zufaellig — der wahre Anteil kann abweichen.");
}
