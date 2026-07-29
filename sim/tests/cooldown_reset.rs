//! Gleichlauf des Auswahlmodus (Issue E3-3c, AC5).
//!
//! **Aussagekraeftig wie bei den Verstaerkungen, nicht trivial wie bei der
//! Sublimation (#104):** Der Modus haengt an keinem ausgelieferten Zauber. Was
//! er tut — Abklingzeit auf null, Wartezeit ausloesen, verpuffen — laesst sich
//! auf beiden Seiten mit denselben Schritten nachrechnen.
//!
//! Die Folge stammt aus dem Spielcode (`tests/parity/payloads.ts`) und liegt
//! als `cooldown-reset.json` im Repository. Hier wird sie nachgespielt: Die
//! **Wirkungen** setzt dieser Test von Hand (sie sind anderswo geprueft), die
//! **Auswahl** macht [`rota_sim::cooldown_reset`]. Verglichen wird der Zustand
//! nach jedem Schritt.

use serde_json::Value;

use rota_sim::checksum::{mix_float, mix_int, FNV_OFFSET};
use rota_sim::cooldown_reset::{Choice, CooldownReset};
use rota_sim::data::SPELL_COUNT;
use rota_sim::time::Millis;

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
const RESET: &str = include_str!("../../tests/parity/recordings/cooldown-reset.json");

/// Die Abklingzeiten der Pruef-Schule, in der Reihenfolge der Zauber.
const COOLDOWNS: [Millis; 3] = [30_000, 1_500, 3_000];
/// Welcher Platz den Ruecksetzer traegt.
const RESET_SPELL: usize = 2;

/// Der Zustand, den beide Seiten fuehren.
struct State {
    ready_at: [Millis; SPELL_COUNT],
    casts: [u32; SPELL_COUNT],
    gcd_ready_at: Millis,
    modus: CooldownReset,
}

impl State {
    fn new() -> Self {
        Self {
            ready_at: [0; SPELL_COUNT],
            casts: [0; SPELL_COUNT],
            gcd_ready_at: 0,
            modus: CooldownReset::new(),
        }
    }

    fn remaining(&self, spell: usize, now: Millis) -> Millis {
        self.ready_at[spell].saturating_sub(now)
    }

    /// Dieselbe Feldfolge wie in `payloads.ts`.
    fn checksum(&self, now: Millis, gcd_length: Millis) -> u32 {
        let mut hash = mix_int(FNV_OFFSET, now as i32);
        for spell in 0..COOLDOWNS.len() {
            hash = mix_float(hash, f64::from(self.remaining(spell, now)));
            hash = mix_int(hash, self.casts[spell] as i32);
        }
        hash = mix_float(hash, f64::from(self.gcd_ready_at.saturating_sub(now)));
        hash = mix_int(hash, self.modus.applied_count() as i32);
        hash = mix_int(hash, self.modus.wasted_count() as i32);
        let _ = gcd_length;
        hash
    }

    /// Ein Tastendruck — dieselbe Entscheidung wie `Session.apply`.
    fn press(&mut self, spell: usize, now: Millis, gcd_length: Millis) {
        if self.modus.is_armed() {
            let ergebnis = self.modus.choose(
                spell,
                now,
                &mut self.ready_at,
                &mut self.gcd_ready_at,
                gcd_length,
            );
            assert_ne!(ergebnis, Choice::NotArmed, "der Modus stand doch scharf");
            return;
        }
        // Eine gewoehnliche Wirkung: nur, wenn Abklingzeit UND Wartezeit frei
        // sind — dieselbe Bedingung wie im Spiel.
        if self.ready_at[spell] > now || self.gcd_ready_at > now {
            return;
        }
        self.casts[spell] += 1;
        self.ready_at[spell] = now + COOLDOWNS[spell];
        self.gcd_ready_at = now + gcd_length;
        if spell == RESET_SPELL {
            // **Erst nach der eigenen Wirkung** — sonst waehlte sein eigener
            // Druck bereits, und zwar sich selbst.
            self.modus.arm(spell);
        }
    }
}

#[test]
fn dieselbe_folge_ergibt_denselben_zustand() {
    let json: Value = serde_json::from_str(RESET).expect("cooldown-reset.json ist kein JSON");
    assert_eq!(
        json["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von cooldown-reset.json"
    );
    let gcd_length = Millis::try_from(json["gcdMs"].as_u64().expect("gcdMs fehlt"))
        .expect("passt nicht in Millis");
    let steps = json["steps"].as_array().expect("steps fehlen");
    assert_eq!(steps.len(), 6, "die Folge hat ihre Laenge geaendert");

    // Dieselben Druecke wie in `payloads.ts`, in derselben Reihenfolge.
    let druecke: [(usize, Millis); 6] = [
        (0, 0),
        (RESET_SPELL, 1500),
        (RESET_SPELL, 3000),
        (RESET_SPELL, 4500),
        (0, 6000),
        (0, 7500),
    ];

    let mut state = State::new();
    for (index, (spell, now)) in druecke.into_iter().enumerate() {
        let step = &steps[index];
        assert_eq!(
            step["timeMs"].as_u64(),
            Some(u64::from(now)),
            "Schritt {index}: andere Zeit als in der Datei"
        );

        state.press(spell, now, gcd_length);

        // Erst die einzelnen Groessen — sie sagen, WAS abweicht.
        let erwartet_ready = step["readyAt"].as_array().expect("readyAt fehlt");
        for (spell_index, erwartet) in erwartet_ready.iter().enumerate() {
            assert_eq!(
                u64::from(state.remaining(spell_index, now)),
                erwartet.as_u64().expect("Zahl erwartet"),
                "Schritt {index} ({}): Zauber {spell_index} hat eine andere Restzeit",
                step["was"].as_str().unwrap_or("?")
            );
        }
        assert_eq!(
            state.modus.is_armed(),
            step["armed"].as_bool().expect("armed fehlt"),
            "Schritt {index}: der Modus steht anders"
        );
        assert_eq!(
            u64::from(state.modus.applied_count()),
            step["applied"].as_u64().expect("applied fehlt"),
            "Schritt {index}: andere Zahl gesetzter Abklingzeiten"
        );
        assert_eq!(
            u64::from(state.modus.wasted_count()),
            step["wasted"].as_u64().expect("wasted fehlt"),
            "Schritt {index}: andere Zahl verpuffter Ruecksetzer"
        );

        // Und dann die Pruefsumme ueber alles zusammen — sie faengt auch, was
        // die Einzelvergleiche nicht ansehen.
        assert_eq!(
            state.checksum(now, gcd_length),
            u32::try_from(step["checksum"].as_u64().expect("checksum fehlt"))
                .expect("passt nicht in 32 Bit"),
            "Schritt {index} ({}): der Gesamtzustand laeuft auseinander",
            step["was"].as_str().unwrap_or("?")
        );
    }
}

/// Die Folge prueft sich selbst: Die Selbstwahl verpufft, der Treffer setzt.
#[test]
fn die_folge_deckt_beide_ausgaenge_ab() {
    let json: Value = serde_json::from_str(RESET).expect("cooldown-reset.json ist kein JSON");
    let steps = json["steps"].as_array().expect("steps fehlen");
    let letzte = &steps[steps.len() - 1];

    assert_eq!(
        letzte["applied"].as_u64(),
        Some(1),
        "die Folge zeigt keinen gelungenen Ruecksetzer"
    );
    assert_eq!(
        letzte["wasted"].as_u64(),
        Some(1),
        "die Folge zeigt keinen verpufften Ruecksetzer"
    );

    // Und der Beweis, dass das Zuruecksetzen wirkt: Bei Schritt 4 faellt die
    // Restzeit des ersten Zaubers von ueber 25 s auf 0.
    let vorher = steps[3]["readyAt"][0].as_u64().expect("Zahl erwartet");
    let nachher = steps[4]["readyAt"][0].as_u64().expect("Zahl erwartet");
    assert!(vorher > 25_000, "der Zauber lag gar nicht auf Abklingzeit");
    assert_eq!(nachher, 0, "die Abklingzeit ist nicht gefallen");
}
