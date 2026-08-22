//! Wie stark wächst der Suchraum, wenn die Suche eine Zeile selbst ergänzt?
//! (Issue #1088, AC4)
//!
//! # Warum isoliert an einem Boss gemessen wird
//!
//! Der naheliegende Weg wäre der übliche Prüfstein über alle Bosse. Er
//! **verschmutzt aber genau die Zahl, um die es geht**: Er addiert die
//! geprüften Listen mehrerer Bosse, und `boss_kandidaten` bildet nur an
//! denen etwas, die `doom` tragen. Ein Zuwachs von einem Prozent über die
//! Summe kann dann zwei Prozent an einem Boss und null an drei anderen sein
//! — die Zahl wäre richtig und die Aussage falsch.
//!
//! `boss-1-2` ist der Boss, an dem die Ergänzung stattfindet. Dort ist der
//! gemessene Zuwachs der Zuwachs.
//!
//! # Das Ergebnis vom 2026-08-22
//!
//! | Länge | geprüfte Listen vorher | nachher | Zuwachs |
//! | ---: | ---: | ---: | ---: |
//! | `MAX_LEN 6` | 2273 | 2294 | +21 (+0,92 %) |
//! | `MAX_LEN 8` | 3908 | 3944 | +36 (+0,92 %) |
//!
//! Der Vorrat wächst von 93 auf 94 Kandidatenzeilen, also um 1,08 %. **Der
//! Zuwachs an geprüften Listen bleibt darunter** und ist bei beiden Längen
//! derselbe Anteil. Das ist die eigentliche Aussage: Die Ergänzung kostet
//! ungefähr ihren Anteil am Vorrat und wirkt **linear**, nicht multiplikativ.
//! Eine Zeile, die den Suchraum mit der Listenlänge aufbläht, sähe hier
//! anders aus — bei `MAX_LEN 8` müsste der Anteil dann steigen.
//!
//! Zwei Längen stehen hier, weil **eine** den Unterschied zwischen linear und
//! multiplikativ gar nicht zeigen kann: Ein einzelner Anteil ist mit beidem
//! vereinbar.
//!
//! # Was diese Messung NICHT belegt
//!
//! **Die Laufzeit.** Sie wird mitgedruckt, weil sie beim Nachfahren
//! interessiert, ist aber bei einem Lauf je Stand und fremder Last auf der
//! Maschine nicht belastbar — sichtbar daran, dass der Nachher-Lauf bei
//! `MAX_LEN 6` *schneller* war (0,799 s gegen 0,819 s). Das kann die Änderung
//! nicht bewirkt haben. Wer eine Laufzeitaussage braucht, misst mehrfach auf
//! ruhiger Maschine.
//!
//! `measured` dagegen ist **deterministisch** — dieselbe Eingabe ergibt
//! dieselbe Zahl, unabhängig von Last und Messfenster. Deshalb hängt AC4 an
//! dieser Zahl und nicht an der Uhr.
//!
//! Ebenfalls nicht belegt: andere Bosse, andere Iterationszahlen, und ob der
//! Zuwachs bei sehr viel längeren Listen linear bleibt.
//!
//! # Den Vorher-Stand nachfahren
//!
//! Auf einem Stand ohne #1088 gibt es das Feld `ergaenzt` nicht. Dort die
//! `ergaenzt`-Ausgabe aus der `println!`-Zeile entfernen; alles andere bleibt
//! gleich.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::rotation_search::{all_candidates, greedy};
use std::time::Instant;

/// Der Boss, an dem `boss_kandidaten` etwas bildet. Ein Boss ohne `doom`
/// würde hier zwangsläufig null Zuwachs zeigen — und damit nichts belegen.
const BOSS: &str = "boss-1-2";

fn main() {
    for max_len in [6usize, 8] {
        let boss = boss_by_id(BOSS).unwrap_or_else(|| panic!("Boss {BOSS} fehlt"));
        let config = BalanceConfig {
            iterations: 20,
            boss,
            ..BalanceConfig::default()
        };
        let pool = all_candidates();
        let t0 = Instant::now();
        let ergebnis = greedy(&config, &pool, max_len);
        let dauer = t0.elapsed();

        // ZUSICHERUNG: Ohne eine ergänzte Zeile misst dieser Lauf den Zuwachs
        // von nichts — dann wäre jedes "kaum gewachsen" trivial wahr und
        // sagte über die Ergänzung genau gar nichts aus.
        assert!(
            !ergebnis.ergaenzt.is_empty(),
            "An {BOSS} wurde nichts ergaenzt. Dann vergleicht diese Messung \
             zweimal denselben Vorrat und der Zuwachs ist trivial null."
        );

        println!(
            "  MAX_LEN {max_len}  Dauer {:>7.3} s  geprueft {:>6}  Vorrat {:>4} (+{})  \
             ergaenzt {:?}  Schaden {:.1}",
            dauer.as_secs_f64(),
            ergebnis.measured,
            ergebnis.pool_size,
            ergebnis.ergaenzt.len(),
            ergebnis.ergaenzt,
            ergebnis.best.mean
        );
    }
}
