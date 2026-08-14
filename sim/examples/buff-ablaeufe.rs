//! Wie oft laeuft ueberhaupt eine Verstaerkung ab? (Vorarbeit zu expire_buffs)
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
//!   --example buff-ablaeufe
//! ```
//!
//! # Was diese Messung beantwortet — und was nicht
//!
//! Die Frage war: Wie oft findet `expire_buffs` tatsaechlich etwas? Davon
//! haengt ab, ob ein gefuehrter `next_expiry_at` (`if now < next_expiry
//! { return; }`) sich lohnt.
//!
//! **Vollstaendig beantworten kann sie dieser Laeufer nicht**, und der Grund
//! liegt im Bestand, nicht in der Messung:
//!
//! ```text
//!   fight.rs   log_expired()   schreibt LogKind::BuffExpire mit der ART
//!                              (kind 0..7), NICHT mit der Sammlung
//!   buffs.rs   expirations     privat, kein Getter
//!   fight.rs   die acht Sammlungen sind private Felder
//! ```
//!
//! Da jede der acht Sammlungen ihre Arten von 0 bis 7 zaehlt, ist aus dem
//! Protokoll nicht zu erkennen, aus WELCHER ein Eintrag stammt. Eine
//! Aufschluesselung je Sammlung braucht deshalb eine Lesefunktion im Bestand;
//! sie ist hier bewusst NICHT gebaut.
//!
//! Was hier steht, ist die Obergrenze: Wie viele Ablaeufe es ueberhaupt gibt.
//! Ist diese Zahl klein, ist sie fuer JEDE einzelne Sammlung erst recht klein
//! — und genau das ist die Frage hinter der Frage.
//!
//! # Der Nenner fehlt, und auch das ist der Bestand
//!
//! Die Schleife in `run_with` ist EREIGNISGESTEUERT
//! (`while let Some(now) = self.queue.peek_time()`), nicht taktgesteuert. Wie
//! viele Durchlaeufe ein Kampf hat, protokolliert niemand. Statt eine
//! Trefferquote zu erfinden, steht hier die Zahl der Ablaeufe je Kampf neben
//! der Kampfdauer — wer den Nenner hat, kann die Quote selbst bilden.
//!
//! # Die Einstellungen
//!
//! ```text
//!   Bosse       boss-1-1, boss-1-2, boss-2-3
//!   Kaempfe     je Boss 70, zusammen 210 (der Auftrag verlangt 200)
//!   Startwerte  0 bis 69, ein Durchgang je Startwert
//!   Dauer       60000 ms
//!   Rotation    je Boss die aus greedy() gefundene
//! ```

use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::{Fight, FightConfig};

const BOSSE: [&str; 3] = ["boss-1-1", "boss-1-2", "boss-2-3"];
const KAEMPFE_JE_BOSS: u32 = 70;
const DAUER_MS: u32 = 60_000;

fn main() {
    let pool = all_candidates();

    println!("== Einstellungen ==\n");
    println!(
        "  Bosse {:?} · je {KAEMPFE_JE_BOSS} Kaempfe · Dauer {DAUER_MS} ms",
        BOSSE
    );
    println!(
        "  zusammen {} Kaempfe\n",
        KAEMPFE_JE_BOSS as usize * BOSSE.len()
    );

    println!("== Ablaeufe je Kampf ==\n");
    println!(
        "  {:<12}{:>10}{:>12}{:>12}{:>14}{:>14}",
        "Boss", "Kaempfe", "Ablaeufe", "je Kampf", "frueheste ms", "spaeteste ms"
    );

    let mut gesamt_ablaeufe = 0_usize;
    let mut gesamt_kaempfe = 0_usize;
    let mut kaempfe_ohne = 0_usize;

    for boss_id in BOSSE {
        let boss = boss_by_id(boss_id).expect("Boss fehlt");
        let config = BalanceConfig {
            iterations: 20,
            base_seed: 5,
            threads: 2,
            stop_on_kill: false,
            duration_ms: Some(DAUER_MS),
            boss,
            ..BalanceConfig::default()
        };
        let apl = greedy(&config, &pool, 8).best.apl;

        let mut ablaeufe = 0_usize;
        let mut frueheste: Option<u32> = None;
        let mut spaeteste: Option<u32> = None;
        let mut ohne = 0_usize;

        for seed in 0..KAEMPFE_JE_BOSS {
            let mut fight = Fight::new(FightConfig {
                seed,
                boss,
                duration_ms: Some(DAUER_MS),
                stop_on_kill: false,
                ..FightConfig::default()
            });
            let _ = fight.run_apl(&apl);
            let treffer: Vec<u32> = fight
                .log()
                .entries()
                .into_iter()
                .filter(|e| e.kind == LogKind::BuffExpire)
                .map(|e| e.time)
                .collect();
            if treffer.is_empty() {
                ohne += 1;
            }
            for t in &treffer {
                frueheste = Some(frueheste.map_or(*t, |f: u32| f.min(*t)));
                spaeteste = Some(spaeteste.map_or(*t, |s: u32| s.max(*t)));
            }
            ablaeufe += treffer.len();
        }

        println!(
            "  {boss_id:<12}{KAEMPFE_JE_BOSS:>10}{ablaeufe:>12}{:>12.2}{:>14}{:>14}",
            ablaeufe as f64 / f64::from(KAEMPFE_JE_BOSS),
            frueheste.map_or("—".to_owned(), |v| v.to_string()),
            spaeteste.map_or("—".to_owned(), |v| v.to_string())
        );
        gesamt_ablaeufe += ablaeufe;
        gesamt_kaempfe += KAEMPFE_JE_BOSS as usize;
        kaempfe_ohne += ohne;
    }

    println!("\n== Zusammen ==\n");
    println!("  Kaempfe:                {gesamt_kaempfe}");
    println!("  Ablaeufe insgesamt:     {gesamt_ablaeufe}");
    println!(
        "  je Kampf:               {:.2}",
        gesamt_ablaeufe as f64 / gesamt_kaempfe as f64
    );
    println!(
        "  Kaempfe GANZ OHNE Ablauf: {kaempfe_ohne} von {gesamt_kaempfe}  ({:.1} %)",
        kaempfe_ohne as f64 / gesamt_kaempfe as f64 * 100.0
    );

    // **Die Zusicherung.** Ohne sie koennte der Laeufer melden „null Ablaeufe"
    // und damit das Ergebnis liefern, das die Vermutung bestaetigt — ohne dass
    // je ein Buff gesetzt worden waere. Ein Kampf ohne jede Verstaerkung
    // beantwortet die Frage nicht, er stellt sie nicht.
    assert!(
        gesamt_ablaeufe > 0,
        "in {gesamt_kaempfe} Kaempfen ist NIE eine Verstaerkung abgelaufen. Dann setzt die \
         gefundene Rotation gar keine — und die Zahl sagt nichts ueber expire_buffs, sondern \
         nur ueber die Rotation."
    );
}
