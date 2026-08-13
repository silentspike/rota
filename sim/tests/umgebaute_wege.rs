//! Die vier umgebauten Talent-Wege rechnen auf beiden Seiten gleich (#419).
//!
//! # Warum sie umgebaut wurden
//!
//! Gemessen verlor jeder gegen seinen Nachbarn im selben Baum, und der Grund
//! war keine Zahl:
//!
//! > Ein Weg auf die SUMME schlaegt einen Weg auf einen SUMMANDEN, und zwar in
//! > jeder Rotation, die viel von der Summe hat.
//!
//! # Was hier geprueft wird — und was drueben
//!
//! Hier: dass die Rust-Seite dieselben Groessen bewegt. Die Trennschaerfe
//! (welcher Zauber den Zuschlag bekommt) steht im Spiel-Fall
//! `tests/unit/umgebaute-wege.test.ts`; sie braucht Zauber-Eigenschaften, und
//! die stehen auf beiden Seiten in denselben Daten.
//!
//! # Die Deckel-Zahl ist gemessen, nicht gewaehlt
//!
//! ```text
//!   Zuschlag  +1      +2      +3
//!   STAPEL    effect  build   build
//!   ENTLADER  effect  effect  build
//!   KARG      effect  build   build
//! ```
//!
//! Nur bei +2 gewinnt jeder der beiden irgendwo — das ist AC3. Ab +3 aendert
//! sich nichts mehr, weil 5 + 3 die Kapazitaet erreicht (`DECAY_STACK_CAPACITY`).

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Wirft einen Zauber `wuerfe` mal und gibt die liegenden Stapel.
fn stapel_nach(zauber: &str, wuerfe: u32, cap_bonus: usize) -> usize {
    let boss = boss_by_id("first-crucible").expect("first-crucible fehlt");
    let spell = index_of(zauber).unwrap_or_else(|| panic!("{zauber} fehlt"));
    let mut fight = Fight::new(FightConfig {
        boss,
        talent_decay_cap_bonus: cap_bonus,
        ..FightConfig::default()
    });
    let presses: Vec<Press> = (0..wuerfe)
        .map(|i| Press {
            tick: i * 200,
            slot: 0,
        })
        .collect();
    fight.replay(
        &Recording {
            name: "cap".to_owned(),
            seed: 0,
            ticks: wuerfe * 200 + 10,
            bindings: vec![Binding {
                slot: 0,
                spell,
            }],
            presses,
        },
        false,
    );
    fight.decay_stacks()
}

/// **Die Zusicherung vor der Messung.** Ohne Zuschlag muss der Deckel bei
/// fuenf liegen — sonst misst der Fall unten etwas anderes als er behauptet.
#[test]
fn ohne_zuschlag_liegt_der_deckel_bei_fuenf() {
    // Zehn Wuerfe CANKER wollen zehn Stapel legen; der Deckel laesst fuenf zu.
    assert_eq!(
        stapel_nach("canker", 10, 0),
        5,
        "die Grundgrenze liegt nicht bei fuenf"
    );
}

#[test]
fn der_zuschlag_hebt_die_obergrenze() {
    assert_eq!(
        stapel_nach("canker", 10, 2),
        7,
        "zwei zusaetzliche Plaetze kommen nicht an"
    );
}

/// **Und die Kapazitaet deckelt weiterhin.** Sie traegt die Struktur —
/// Feldlaenge UND Laenge der Pruefsummen-Schleife. Eine Pruefsumme, deren
/// Laenge vom Spielverlauf abhaengt, waere keine (#479).
#[test]
fn ueber_die_kapazitaet_hinaus_geht_es_nicht() {
    assert_eq!(
        stapel_nach("canker", 20, 99),
        8,
        "der Zuschlag durchbricht die Kapazitaet"
    );
}

/// Der Schutz je Stapel: mehr Promille, weniger erlittener Schaden.
#[test]
fn der_schutz_je_stapel_mindert_mehr() {
    let erlitten = |ward_permille: u32| -> u32 {
        let boss = boss_by_id("boss-1-1").expect("boss-1-1 fehlt");
        let canker = index_of("canker").expect("canker fehlt");
        let decay = index_of("decay").expect("decay fehlt");
        let mut fight = Fight::new(FightConfig {
            boss,
            talent_decay_ward_permille: ward_permille,
            ..FightConfig::default()
        });
        // Erst Stapel legen, dann DECAY — der Schutz haengt an der Zahl der
        // liegenden Stapel.
        let mut presses = vec![Press { tick: 0, slot: 0 }, Press { tick: 200, slot: 0 }];
        presses.push(Press { tick: 400, slot: 1 });
        fight.replay(
            &Recording {
                name: "ward".to_owned(),
                seed: 0,
                // **2000 Takte, nicht mehr**: Danach stirbt der Spieler, und
                // der erlittene Schaden ist bei seinem Leben gedeckelt. Zwei
                // gedeckelte Werte zu vergleichen sieht aus wie „keine
                // Wirkung" — genau das hat den ersten Messversuch verdorben.
                ticks: 2000,
                bindings: vec![
                    Binding {
                        slot: 0,
                        spell: canker,
                    },
                    Binding {
                        slot: 1,
                        spell: decay,
                    },
                ],
                presses,
            },
            false,
        );
        fight.damage_taken()
    };

    let ohne = erlitten(0);
    assert!(ohne > 0, "der Boss trifft gar nicht — der Vergleich misst nichts");
    assert!(
        erlitten(600) < ohne,
        "der Schutz-Weg mindert nichts: {ohne} bleibt {ohne}"
    );
}
