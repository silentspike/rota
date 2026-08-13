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

/// Legt acht Stapel so schnell wie moeglich und misst SOFORT danach.
///
/// **Zwei Quellen im Wechsel** — CANKER hat 2000 ms Abklingzeit, BLIGHT 1500.
/// Abwechselnd alle 150 Takte kommt jeder von beiden alle 300 Takte dran, also
/// weit ueber seiner Abklingzeit: acht Wuerfe in 10,5 Sekunden.
///
/// **Und gemessen wird bei Takt 1060, nicht spaeter.** Ein Stapel haelt 12 s;
/// mein erster Entwurf mass 300 Takte nach dem letzten Wurf und sah 4 statt 5,
/// weil die ersten drei bereits verfallen waren. Der Deckel war richtig, der
/// Messzeitpunkt falsch — und die Zahl sah aus wie ein Fehler im Code.
fn stapel_nach_acht_wuerfen(cap_bonus: usize) -> usize {
    let boss = boss_by_id("first-crucible").expect("first-crucible fehlt");
    let canker = index_of("canker").expect("canker fehlt");
    let blight = index_of("blight").expect("blight fehlt");
    let mut fight = Fight::new(FightConfig {
        boss,
        talent_decay_cap_bonus: cap_bonus,
        ..FightConfig::default()
    });
    let presses: Vec<Press> = (0..8)
        .map(|i| Press {
            tick: i * 150,
            slot: u8::try_from(i % 2).expect("passt"),
        })
        .collect();
    fight.replay(
        &Recording {
            name: "cap".to_owned(),
            seed: 0,
            // Letzter Wurf bei 1050, Ende bei 1060 — vor dem ersten Verfall.
            ticks: 1060,
            bindings: vec![
                Binding {
                    slot: 0,
                    spell: canker,
                },
                Binding {
                    slot: 1,
                    spell: blight,
                },
            ],
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
    // Acht Wuerfe wollen acht Stapel legen; der Deckel laesst fuenf zu.
    assert_eq!(
        stapel_nach_acht_wuerfen(0),
        5,
        "die Grundgrenze liegt nicht bei fuenf"
    );
}

#[test]
fn der_zuschlag_hebt_die_obergrenze() {
    assert_eq!(
        stapel_nach_acht_wuerfen(2),
        7,
        "zwei zusaetzliche Plaetze kommen nicht an"
    );
}

/// **Und die Kapazitaet deckelt weiterhin.** Sie traegt die Struktur —
/// Feldlaenge UND Laenge der Pruefsummen-Schleife. Eine Pruefsumme, deren
/// Laenge vom Spielverlauf abhaengt, waere keine (#479).
#[test]
fn ueber_die_kapazitaet_hinaus_geht_es_nicht() {
    // Acht Wuerfe koennen hoechstens acht legen — und genau da deckelt die
    // Kapazitaet, obwohl der Zuschlag weit darueber liegt.
    assert_eq!(
        stapel_nach_acht_wuerfen(99),
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
    assert!(
        ohne > 0,
        "der Boss trifft gar nicht — der Vergleich misst nichts"
    );
    assert!(
        erlitten(600) < ohne,
        "der Schutz-Weg mindert nichts: {ohne} bleibt {ohne}"
    );
}
