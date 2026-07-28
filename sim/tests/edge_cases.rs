//! Zwei Pfade, die in den vier Rotationen aus `STEP1-SPEC` § 4 nie vorkommen.
//!
//! Beide sind im Simulator umgesetzt, aber keine der vier Rotationen laeuft
//! durch sie hindurch — und ungepruefter Code, der trotzdem ausgeliefert wird,
//! ist genau die Art Luecke, die spaeter als Rechenfehler erscheint:
//!
//! 1. **Leerlauf.** In allen vier Rotationen ist `idle_ms` gleich 0. Die
//!    Umrechnung von "je leerem Takt 10 ms" (Referenz) in "eine Spanne am
//!    Stueck" (ereignisgesteuert) war damit nie belegt.
//! 2. **Zwei Anwendungen desselben Brand-Effekts auf demselben Zeitpunkt.**
//!    Wird SMOULDER auf einem Vielfachen seines Tick-Abstands erneuert, faellt
//!    der naechste Tick der alten Anwendung mit dem ersten Tick der neuen
//!    zusammen. Ohne den Generations-Zaehler aus `event.rs` wuerde dort
//!    **doppelt** getickt — bei laufender Uhr schwer zu bemerken, weil der
//!    Schaden nur ein wenig zu hoch waere.
//!
//! Die Sollwerte stammen aus `tools/step1-check.py`, mit denselben beiden
//! Rotationen gerechnet.

use rota_sim::data::{PYRE, SMOULDER, SPELLS};
use rota_sim::log::LogKind;
use rota_sim::{Fight, FightConfig};

/// Wirkt ausschliesslich PYRE ab der Bonus-Schwelle — und wartet sonst.
///
/// Ohne den Fueller kommt nie Hitze zusammen, also wird nie gewirkt. Genau
/// darum geht es: ein Kampf, der aus nichts als Warten besteht.
fn nur_pyre(fight: &Fight) -> Option<usize> {
    let schwelle = SPELLS[PYRE]
        .spender
        .map_or(u16::MAX, |spender| spender.bonus_threshold);
    if fight.is_ready(PYRE) && fight.resource() >= schwelle {
        return Some(PYRE);
    }
    None
}

/// Wirkt SMOULDER genau bei 0 ms und bei 4000 ms.
///
/// 4000 ms ist ein Vielfaches des Tick-Abstands von 2 s — dadurch treffen der
/// naechste Tick der ersten Anwendung und der erste Tick der zweiten beide auf
/// 6000 ms.
fn smoulder_bei_0_und_4000(fight: &Fight) -> Option<usize> {
    if matches!(fight.time(), 0 | 4000) && fight.is_ready(SMOULDER) {
        return Some(SMOULDER);
    }
    None
}

/// **Grenze des ereignisgesteuerten Modells — festgehalten, nicht behoben.**
///
/// Die Rotation wird nur an Ereigniszeitpunkten befragt. Fuer eine Rotation, die
/// den **Zustand** abfragt, ist das vollstaendig: Der Zustand aendert sich
/// ausschliesslich an Ereignissen. Alle vier Rotationen aus `STEP1-SPEC` § 4
/// sind so gebaut, und sie treffen die Sollwerte auf den Punkt.
///
/// Fuer eine Rotation, die auf die **Uhr** schaut, ist es unvollstaendig: Bei
/// 3000 ms liegt in diesem Kampf kein Ereignis, also wird dort nicht gefragt und
/// nicht gewirkt. Die Referenz mit ihrem 10-ms-Takt wirkt an derselben Stelle
/// sehr wohl.
///
/// Das ist beim Bau von E2-1 aufgefallen und gehoert zu **E2-3 (APL-Sprache)**:
/// Erlaubt die Prioritaetenliste zeitabhaengige Bedingungen — etwa "in den
/// letzten zehn Sekunden des Kampfes" —, dann muss die Auswertung fuer jede
/// solche Bedingung einen Weckruf einplanen. Bis dahin haelt dieser Test fest,
/// **dass** die Grenze besteht, damit sie in E2-3 nicht neu entdeckt werden muss.
///
/// Betroffen ist dabei nur der Fall, dass die globale Wartezeit bereits
/// abgelaufen ist und trotzdem nichts gewirkt wird. Wer wirkt, loest damit einen
/// Weckruf aus.
#[test]
fn zeitgesteuerte_rotationen_werden_nur_an_ereigniszeitpunkten_befragt() {
    fn smoulder_bei_0_und_3000(fight: &Fight) -> Option<usize> {
        if matches!(fight.time(), 0 | 3000) && fight.is_ready(SMOULDER) {
            return Some(SMOULDER);
        }
        None
    }

    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let r = fight.run(smoulder_bei_0_und_3000);

    // 3000 ms ist in diesem Kampf kein Ereigniszeitpunkt: Die Ticks liegen auf
    // 2000er-Schritten, der Verfall ebenso, die Weckrufe aus Abklingzeit und
    // Wartezeit auf 1500. Die zweite Wirkung entfaellt deshalb.
    assert_eq!(
        r.casts[SMOULDER], 1,
        "unerwartet: bei 3000 ms wurde doch gewirkt"
    );

    // Zum Vergleich derselbe Zeitpunkt, aber auf einem Ereignis (1500 ms):
    // dort wirkt der Simulator wie die Referenz.
    fn smoulder_bei_0_und_1500(fight: &Fight) -> Option<usize> {
        if matches!(fight.time(), 0 | 1500) && fight.is_ready(SMOULDER) {
            return Some(SMOULDER);
        }
        None
    }
    let mut zweiter = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    assert_eq!(zweiter.run(smoulder_bei_0_und_1500).casts[SMOULDER], 2);
}

#[test]
fn leerlauf_wird_ueber_die_volle_kampfzeit_richtig_gezaehlt() {
    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let r = fight.run(nur_pyre);

    // Sollwerte aus der Referenz mit derselben Rotation:
    //   Schaden 0, PYRE-Wirkungen 0, Leerlauf 60000 ms
    assert_eq!(r.damage, 0, "ohne Fueller kommt keine Hitze zusammen");
    assert_eq!(r.casts[PYRE], 0, "die Schwelle wird nie erreicht");
    assert_eq!(
        r.idle_ms, 60_000,
        "der ganze Kampf ist Leerlauf — gezaehlt werden muessen alle 60 s"
    );
}

#[test]
fn teilweiser_leerlauf_zaehlt_nur_die_wartezeit() {
    // Gegenprobe: Waere die Zaehlung schlicht "Kampfdauer, wenn irgendwann
    // gewartet wurde", faende der Test oben das nicht heraus. Die gute Rotation
    // wartet nie, also muss hier 0 stehen.
    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let r = fight.run(rota_sim::rotation::good);
    assert_eq!(r.idle_ms, 0);
}

#[test]
fn zwei_anwendungen_auf_demselben_zeitpunkt_ticken_nur_einmal() {
    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let r = fight.run(smoulder_bei_0_und_4000);

    let ticks: Vec<u32> = fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::DotTick)
        .map(|e| e.time)
        .collect();

    // Sollwerte aus der Referenz mit derselben Rotation:
    //   2 Wirkungen, 8 Ticks, 320 Schaden, 4 verlorene Ticks,
    //   Zeitpunkte 2000 bis 16000 im Abstand von 2000
    assert_eq!(r.casts[SMOULDER], 2);
    assert_eq!(
        ticks,
        vec![2000, 4000, 6000, 8000, 10_000, 12_000, 14_000, 16_000]
    );
    assert_eq!(r.damage_by_spell[SMOULDER], 320);
    assert_eq!(r.lost_dot_ticks[SMOULDER], 4);

    // Der Kern des Falls: **ein** Tick bei 6000, nicht zwei.
    assert_eq!(
        ticks.iter().filter(|&&t| t == 6000).count(),
        1,
        "die alte Anwendung hat neben der neuen weitergetickt — \
         der Generations-Zaehler greift nicht"
    );
}

#[test]
fn erneuern_zwischen_zwei_ticks_verwirft_den_alten_tick() {
    // Dieselbe Gefahr, nur ohne Zusammenfallen: Wird bei 1500 ms erneuert — dem
    // Ablauf der globalen Wartezeit —, liegt der Tick der alten Anwendung (2000)
    // noch in der Schlange, waehrend die neue erst bei 3500 tickt. Ohne
    // Generations-Zaehler gaebe es bei 2000 einen Tick, den es nicht geben darf.
    fn smoulder_bei_0_und_1500(fight: &Fight) -> Option<usize> {
        if matches!(fight.time(), 0 | 1500) && fight.is_ready(SMOULDER) {
            return Some(SMOULDER);
        }
        None
    }

    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let r = fight.run(smoulder_bei_0_und_1500);

    let ticks: Vec<u32> = fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::DotTick)
        .map(|e| e.time)
        .collect();

    assert!(
        !ticks.contains(&2000),
        "bei 2000 ms hat die verworfene Anwendung getickt: {ticks:?}"
    );
    // Sollwerte aus der Referenz mit derselben Rotation: 6 Ticks, 240 Schaden,
    // 5 verlorene Ticks.
    assert_eq!(
        ticks,
        vec![3500, 5500, 7500, 9500, 11_500, 13_500],
        "nach dem Erneuern bei 1500 laeuft das Raster um 1,5 s versetzt weiter"
    );
    assert_eq!(r.damage_by_spell[SMOULDER], 240);
    assert_eq!(r.lost_dot_ticks[SMOULDER], 5);
}
