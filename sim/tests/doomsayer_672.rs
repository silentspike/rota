//! Der Verhängnis-Boss: `purge` senkt, und woran der Spieler stirbt (#672).
//!
//! # Was hier geprüft wird — und was nicht
//!
//! Die **Eigenschaften**, nicht die Balance-Zahlen. Der Läufer
//! `examples/doomsayer-672.rs` misst 9151 Schaden, fünf `purge`-Wirkungen und
//! eine doom-Spur von 10 bis 100; das sind Messwerte, die sich bei jeder
//! Balance-Änderung bewegen dürfen. Eine Bedingung darauf wäre bei jeder
//! Zahlenänderung fällig, ohne je etwas anderes zu sagen.
//!
//! Was sich NICHT bewegen darf, ist die Aussage darunter:
//!
//! | Eigenschaft | warum sie trägt |
//! | --- | --- |
//! | `purge` senkt den Zähler | sonst ist der Zauber Kulisse |
//! | ohne `purge` stirbt man am Verhängnis | sonst prüft der Boss seine Mechanik nicht |
//! | mit `purge` stirbt man woanders | das ist der Befund aus Teil 3 |
//!
//! # Der Befund, den diese Fälle festhalten
//!
//! `test-doomsayer` trägt ZWEI Mechaniken: das Verhängnis (`doom`) und die
//! Stapel (`stacks`). Deren Werte, aus der Bossdatei gelesen: `baseDamage 20`,
//! `factor 3`, `maxStacks 4`, alle 2000 ms ein Stapel — also 20, 60, 180, 540
//! je Schritt mit Deckel bei vier.
//!
//! **Die frühere Formulierung „20 Stapel mit Faktor 3" war falsch** und stand
//! bis zum 2026-08-14 hier: Es sind 20 GRUNDSCHADEN und maximal VIER Stapel.
//!
//! Gemessen am 2026-08-14:
//!
//! ```text
//!   Liste                     Schaden  purge  doom  erlitten  lebt
//!   purge ganz oben              3379      4    40      5392  false
//!   ohne purge (Gegenprobe)      2645      0   100      2800  false
//! ```
//!
//! # WELCHE Mechanik tötet — die volle Matrix, mit abgeschalteter Gegenseite
//!
//! Die Zeilen oben zeigen, dass `purge` die Todesart verschiebt, aber nicht,
//! welche Mechanik für sich genommen tödlich ist. `examples/doomsayer-mechaniken-672.rs`
//! schaltet je eine ab (in einer Kopie des Boss, die Datei bleibt unberührt)
//! und fährt jede Zeile zweimal — ohne und mit `purge`:
//!
//! ```text
//!   OHNE purge
//!     A1  nur doom       Tod 20000 ms   doom 100   erlitten     0
//!     B1  nur stacks     Tod 30000 ms   doom   0   erlitten  5392
//!     C1  beide          Tod 20000 ms   doom 100   erlitten  2800
//!
//!   MIT purge
//!     A2  nur doom       ÜBERLEBT       doom  35   erlitten     0
//!     B2  nur stacks     Tod 30000 ms   doom   0   erlitten  5392
//!     C2  beide          Tod 30000 ms   doom  40   erlitten  5392
//! ```
//!
//! Drei Befunde, die ohne die zweite Spalte nicht zu haben waren:
//!
//! 1. **Beide Mechaniken töten allein** (A1, B1) — das Verhängnis zehn
//!    Sekunden früher.
//! 2. **`purge` besiegt das Verhängnis vollständig** (A2): Für sich genommen
//!    ist es mit dem Zauber nicht mehr tödlich, der Zähler bleibt bei 35 von
//!    100. Was den Spieler dann noch holt, sind die Stapel.
//! 3. **Der Widerspruch war keiner.** C1 stirbt am Verhängnis, C2 an den
//!    Stapeln — zwei richtige Aussagen über zwei verschiedene Läufe.
//!
//! In A1 und A2 steht der erlittene Schaden auf **0**: Der Verhängnis-Tod läuft
//! nicht über den Lebensbalken. Das belegt die Ursache, statt sie aus dem
//! Zeitpunkt zu erschließen.
//!
//! Die Schwelle fällt bei **20000 ms** (`threshold 100 / amount 5` = 20
//! Schritte à 1000 ms), nicht erst am Ende der Überdruckzeit bei 60000 ms — der
//! Spieler stirbt nach einem Drittel der Begegnung, nicht an ihrem Ende.

use rota_sim::apl::{parse, AplName};
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::{Fight, FightConfig};

const BOSS: &str = "test-doomsayer";

/// Ein Kampf über die volle Überdruckzeit mit dieser Liste.
fn lauf(text: &str) -> (Fight, rota_sim::RunResult) {
    let boss = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let apl = parse(AplName("fall"), text).expect("APL gültig");
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(boss.overpressure_ms),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let ergebnis = fight.run_apl(&apl);
    (fight, ergebnis)
}

/// Die Zusicherung vor allem anderen: Der Boss trägt überhaupt ein Verhängnis.
///
/// Ohne sie prüften die Fälle darunter eine Mechanik, die es nicht gibt — und
/// „doom bleibt bei 0" wäre die richtige Antwort auf eine Frage, die niemand
/// gestellt hat.
#[test]
fn der_boss_traegt_ueberhaupt_ein_verhaengnis() {
    let boss = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let doom = boss.doom.expect("test-doomsayer trägt kein doom mehr");
    assert!(doom.amount > 0, "der Zähler steigt gar nicht");
    assert!(doom.threshold > 0, "es gibt keine Schwelle");
    assert!(doom.clear_amount > 0, "es gibt nichts zu senken");
}

/// **`purge` senkt den Zähler wirklich** — gemessen am Zähler, nicht an der
/// Zahl der Wirkungen.
///
/// Der Unterschied ist der aus #558: Eine Zeile, die gewirkt wird, ist noch
/// kein Beleg, dass sie etwas bewirkt. Hier stehen zwei Läufe nebeneinander,
/// die sich nur in dieser einen Zeile unterscheiden.
#[test]
fn purge_senkt_den_zaehler() {
    let ohne = "pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";
    let mit = "purge\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";

    let (f_ohne, r_ohne) = lauf(ohne);
    let (f_mit, r_mit) = lauf(mit);
    let purge = index_of("purge").expect("purge fehlt");

    // Die Zusicherung: Der Zauber kam überhaupt dran.
    assert_eq!(r_ohne.casts[purge], 0, "ohne die Zeile wurde purge gewirkt");
    assert!(
        r_mit.casts[purge] > 0,
        "mit der Zeile wurde purge NIE gewirkt — dann vergleicht dieser Fall zwei \
         gleiche Läufe"
    );

    assert!(
        f_mit.doom() < f_ohne.doom(),
        "purge senkt den Zähler nicht: {} mit, {} ohne",
        f_mit.doom(),
        f_ohne.doom()
    );
}

/// **Ohne `purge` stirbt der Spieler am Verhängnis** — der Zähler steht an der
/// Schwelle.
///
/// Das ist die Zusicherung, dass die Mechanik überhaupt tödlich ist. Ohne sie
/// könnte `purge` beliebig gut oder schlecht sein, ohne dass es etwas
/// bedeutete.
#[test]
fn ohne_purge_toetet_das_verhaengnis() {
    let (f, _) = lauf("pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n");
    let doom = boss_by_id(BOSS).and_then(|b| b.doom).expect("doom fehlt");

    assert!(!f.player_alive(), "der Spieler überlebt ohne purge");
    assert!(
        f.doom() >= doom.threshold,
        "der Spieler ist gestorben, aber der Zähler steht bei {} von {} — dann war es \
         nicht das Verhängnis, und dieser Fall prüft die falsche Todesart",
        f.doom(),
        doom.threshold
    );
}

/// **Mit `purge` stirbt er woanders** — der Befund aus Teil 3.
///
/// Der Zähler steht beim Tod deutlich unter der Schwelle. Was ihn holt, ist
/// die zweite Mechanik des Bosses (`stacks`), nicht das Verhängnis.
///
/// **Dieser Fall wird rot, wenn der Kampf eines Tages überlebbar ist** — und
/// das ist gewollt: Dann ist die Begegnung eine andere, und wer die Zahlen
/// ändert, soll es hier lesen statt es zu übersehen.
#[test]
fn mit_purge_stirbt_er_an_der_zweiten_mechanik() {
    let (f, r) = lauf("purge\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n");
    let boss = boss_by_id(BOSS).expect("Boss");
    let doom = boss.doom.expect("doom fehlt");
    let purge = index_of("purge").expect("purge fehlt");

    // Die Zusicherung: purge kam dran, sonst ist es der Fall darüber.
    assert!(r.casts[purge] > 0, "purge wurde nicht gewirkt");
    // Und: der Boss trägt die zweite Mechanik überhaupt.
    assert!(
        boss.stacks.is_some(),
        "test-doomsayer trägt keine Stapel mehr — dann kann er nicht mehr daran töten, \
         und dieser Fall prüft eine Lage, die es nicht gibt"
    );

    assert!(
        !f.player_alive(),
        "der Spieler ÜBERLEBT jetzt mit purge — das wäre neu (gemessen 2026-08-14: er \
         stirbt an den Stapeln). Der Befund aus #672 Teil 3 gilt nicht mehr, und die \
         Begegnung gehört neu bewertet."
    );
    assert!(
        f.doom() < doom.threshold,
        "der Zähler steht mit purge bei {} von {} — dann ist es doch das Verhängnis, \
         und der Befund aus Teil 3 stimmt nicht",
        f.doom(),
        doom.threshold
    );
}

/// **`purge` besiegt das Verhängnis vollständig** — der stärkste Befund aus der
/// Matrix, und der einzige, der ein ÜBERLEBEN zeigt.
///
/// Gemessen am 2026-08-14: Läuft der Boss nur mit `doom` (die Stapel in einer
/// Kopie abgeschaltet), stirbt der Spieler ohne `purge` bei 20000 ms und
/// überlebt mit `purge` die volle Überdruckzeit — der Zähler bleibt bei 35 von
/// 100.
///
/// **Geprüft wird die Eigenschaft, nicht die 35**: Ohne den Zauber tot, mit ihm
/// lebendig. Eine Bedingung auf den Zählerstand wäre bei jeder Balance-Änderung
/// fällig, ohne je etwas anderes zu sagen.
///
/// **Die Bossdatei bleibt unberührt.** `Boss` ist `Copy`; abgeschaltet wird in
/// einer Kopie, die über `Box::leak` die Lebensdauer bekommt, die `FightConfig`
/// verlangt. In einem Prüflauf ist das unbedenklich — er läuft einmal und endet.
#[test]
fn purge_macht_das_verhaengnis_allein_ueberlebbar() {
    let basis = boss_by_id(BOSS).expect("test-doomsayer fehlt");
    let mut ohne_stapel = *basis;
    ohne_stapel.stacks = None;
    let boss: &'static rota_sim::data::Boss = Box::leak(Box::new(ohne_stapel));

    let fahren = |text: &str| {
        let apl = parse(AplName("fall"), text).expect("APL gültig");
        let mut fight = Fight::new(FightConfig {
            seed: 0,
            boss,
            duration_ms: Some(basis.overpressure_ms),
            stop_on_kill: false,
            ..FightConfig::default()
        });
        let ergebnis = fight.run_apl(&apl);
        (fight, ergebnis)
    };

    let ohne = "pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";
    let mit = "purge\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nkindle\n";
    let (f_ohne, _) = fahren(ohne);
    let (f_mit, r_mit) = fahren(mit);
    let purge = index_of("purge").expect("purge fehlt");

    // Die Zusicherungen zuerst: Die Abschaltung hat gegriffen, und der Zauber
    // kam dran. Ohne sie prüfte dieser Fall zwei gleiche Läufe an einem Boss,
    // der noch beide Mechaniken trägt.
    assert!(
        boss.stacks.is_none(),
        "die Stapel sind nicht abgeschaltet — dann misst dieser Fall die Kombination"
    );
    assert!(
        basis.stacks.is_some(),
        "der ECHTE Boss trägt keine Stapel mehr — dann schaltet dieser Fall nichts ab \
         und vergleicht zweimal dasselbe"
    );
    assert!(r_mit.casts[purge] > 0, "purge wurde nie gewirkt");

    assert!(
        !f_ohne.player_alive(),
        "ohne purge überlebt der Spieler das Verhängnis — dann ist es allein nicht tödlich, \
         und der Befund aus der Matrix gilt nicht mehr"
    );
    assert!(
        f_mit.player_alive(),
        "MIT purge stirbt der Spieler trotzdem, bei doom {} von {} — der Zauber beherrscht \
         die Mechanik nicht mehr",
        f_mit.doom(),
        basis.doom.expect("doom").threshold
    );
}
