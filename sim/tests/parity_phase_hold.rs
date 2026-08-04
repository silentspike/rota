//! Die Phasen-Senkung ist befristet und gilt nur fuer den naechsten Wechsel
//! (GAME-DESIGN 4.2b, Issue #100 — AC3, AC6, AC7).
//!
//! # Warum es diese Datei gibt
//!
//! Die vorhandenen Faelle in `parity_fixation.rs` wirken ANCHOR bei Takt 500 —
//! **vor** dem Wechsel. Damit kann keiner von ihnen die Schutzregel in
//! `check_phase` beruehren, obwohl ein Kommentar dort genau das behauptete.
//! Gemessen am 2026-08-04: `>` durch `!=` ersetzt, volle Suite, 0 Fehler.
//!
//! Dazu kam, dass die Senkung ueberhaupt nie ablief. Ein Druck senkte die
//! Schwelle bis zum Kampfende und verschob bei einem Boss mit zwei Schwellen
//! **beide** Wechsel.
//!
//! # Der Vergleichslauf wirkt STAY, nicht nichts
//!
//! ANCHOR kostet eine globale Wartezeit. Ein Lauf ohne jeden Druck macht
//! deshalb mehr Schaden, und **beide** Wechsel kaemen frueher — die
//! Verschiebung des ersten waere von der Wartezeit nicht zu unterscheiden.
//!
//! STAY kostet dieselbe Wartezeit, richtet ebenfalls keinen Schaden an und
//! verschiebt einen Zeitleisten-Eintrag. Der Prueff-Boss hat eine **leere**
//! Zeitleiste; STAY ist hier also folgenlos. Der einzige Unterschied zwischen
//! den beiden Laeufen ist damit die Phasen-Wirkung selbst.
//!
//! # Warum zwei Schwellen noetig sind
//!
//! Mit nur einer Schwelle waere „nur der naechste Wechsel" von „alle
//! folgenden" nicht zu unterscheiden — dieselbe Falle wie bei STAY, wo erst
//! der **dritte** Schlag zeigte, dass nur der zweite rueckt.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::PHASE_HOLD_DURATION_MS;
use rota_sim::log::LogKind;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, ReplayResult, TICK_MS};

const BOSS: &str = "test-gatekeeper";
/// Erste Schwelle des Prueff-Bosses in Prozent (`data/bosses/test-gatekeeper.json`).
const ERSTE_SCHWELLE: u32 = 70;

/// Spielt eine Runde. `wahl` bestimmt je Takt, welcher Platz gedrueckt wird —
/// `None` heisst: dieser Takt bleibt leer.
fn spielen<F>(zauber: &[&str], ticks: u32, wahl: F) -> (Fight, ReplayResult)
where
    F: Fn(u32) -> Option<u8>,
{
    let boss = boss_by_id(BOSS).unwrap_or_else(|| panic!("Boss {BOSS} unbekannt"));
    let bindings = zauber
        .iter()
        .enumerate()
        .map(|(slot, id)| Binding {
            slot: u8::try_from(slot).expect("wenige Plaetze"),
            spell: index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt")),
        })
        .collect();
    let presses = (0..ticks)
        .filter_map(|tick| wahl(tick).map(|slot| Press { tick, slot }))
        .collect();

    let recording = Recording {
        name: "phase-hold".to_owned(),
        seed: 0,
        ticks,
        bindings,
        presses,
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, true);
    (fight, ergebnis)
}

/// Wie breit das Druck-Fenster ist: eine volle globale Wartezeit.
///
/// **Ein Fenster, kein Zeitpunkt.** Bei einem festen Takt trifft der Druck die
/// laufende Wartezeit von BIND, wird abgelehnt und der Fall laeuft ins Leere —
/// gemessen am 2026-08-04, alle drei Faelle scheiterten daran. Dieselbe Falle
/// hatte STALL in `parity_fixation.rs` schon einmal.
const FENSTER: u32 = 160;

/// Wann der Zauber tatsaechlich gewirkt wurde.
///
/// Innerhalb des Fensters entscheidet die Wartezeit, welcher Takt es wird —
/// **geraten werden darf das nicht**, sonst rechnet ein Fall die Frist ab
/// einem Zeitpunkt, an dem gar nichts geschah.
fn gewirkt_bei(fight: &Fight, zauber: &str) -> Option<u32> {
    let index = u8::try_from(index_of(zauber).expect("Zauber unbekannt")).expect("wenige Zauber");
    fight
        .log()
        .entries()
        .into_iter()
        .find(|e| e.kind == LogKind::Cast && e.spell == index)
        .map(|e| e.time)
}

/// Die Takte, in denen die Phase gestiegen ist.
fn wechsel_bei(spur: &[u32]) -> Vec<u32> {
    spur.windows(2)
        .enumerate()
        .filter(|(_, paar)| paar[1] != paar[0])
        .map(|(tick, _)| tick as u32 + 1)
        .collect()
}

/// Der Takt des ersten Wechsels ohne jede Ausgabe von Kontroll-Punkten.
///
/// **Gemessen statt verdrahtet.** Aendert sich der Schaden von BIND oder die
/// Lebensmenge des Prueff-Bosses, wandern die Faelle mit, statt still an der
/// interessanten Stelle vorbeizulaufen.
fn erster_wechsel_ohne_ausgabe(ticks: u32) -> u32 {
    let (_, ergebnis) = spielen(&["bind"], ticks, |_| Some(0));
    let wechsel = wechsel_bei(&ergebnis.phases);
    assert!(
        wechsel.len() >= 2,
        "der Prueff-Boss muss in {ticks} Takten ZWEI Schwellen unterschreiten, \
         gemessen wurden {} — sonst kann kein Fall zwischen „der naechste“ und \
         „alle folgenden“ unterscheiden",
        wechsel.len()
    );
    wechsel[0]
}

#[test]
fn ac3_die_senkung_nach_dem_wechsel_nimmt_ihn_nicht_zurueck() {
    // **Der Kern des Issues.** Es gibt genau einen Fall, in dem die Phase
    // zurueckfallen koennte: Der Boss hat bereits gewechselt, DANN wird die
    // Senkung gewirkt, und der gerechnete Anteil steigt wieder ueber die
    // Schwelle. Ohne die Zusicherung `neue > self.phase` fiele er zurueck.
    //
    // **Gegenprobe** (von Hand, sie gehoert in die Meldung):
    //   sed -i 's/if neue > self.phase {/if neue != self.phase {/' sim/src/fight.rs
    // Dieser Fall muss dann brechen. Vor dem 2026-08-04 tat das kein einziger.
    let ende = 4_000;
    let wechsel = erster_wechsel_ohne_ausgabe(ende);

    // ANCHOR erst NACH dem Wechsel — darin liegt der ganze Unterschied zu den
    // Faellen in `parity_fixation.rs`, die ihn davor wirken.
    let fenster = wechsel + 20;
    let (fight, ergebnis) = spielen(&["bind", "anchor"], fenster + FENSTER + 60, |tick| {
        Some(u8::from((fenster..fenster + FENSTER).contains(&tick)))
    });

    // **Die Ursache zuerst**, in drei Teilen. Faellt einer aus, prueft der Fall
    // seine eigene Leere und waere trotzdem gruen.
    let anchor = index_of("anchor").expect("anchor");
    assert!(
        ergebnis.casts[anchor] > 0,
        "ANCHOR kam nicht zum Zug — dann liegt gar keine Senkung an"
    );
    assert!(
        fight.phase() > 0,
        "der Boss hat nicht gewechselt — dann wird die Senkung VOR dem Wechsel \
         gewirkt, und das ist der bereits abgedeckte Fall"
    );
    let gewirkt = gewirkt_bei(&fight, "anchor").expect("ANCHOR steht im Protokoll");
    let wechsel_ms = wechsel_bei(&ergebnis.phases)
        .first()
        .copied()
        .expect("der Wechsel steht in der Spur")
        * TICK_MS;
    assert!(
        gewirkt > wechsel_ms,
        "ANCHOR wirkte bei {gewirkt} ms und der Wechsel geschah bei \
         {wechsel_ms} ms — die Senkung kam also VOR dem Wechsel, und genau \
         dieser Fall ist bereits abgedeckt"
    );
    let anteil = fight.boss_hp_percent() + fight.phase_hold_percent();
    assert!(
        anteil > ERSTE_SCHWELLE,
        "der gerechnete Anteil liegt bei {anteil} und damit nicht ueber der \
         Schwelle {ERSTE_SCHWELLE} — dann hat die Schutzregel nichts zu tun \
         und der Fall belegt sie nicht"
    );

    // Der Nachweis: Die Phase ist nie gefallen, in keinem einzelnen Takt.
    for paar in ergebnis.phases.windows(2) {
        assert!(
            paar[1] >= paar[0],
            "die Phase ist von {} auf {} zurueckgefallen — eine gesenkte \
             Schwelle verschiebt den Wechsel, sie nimmt ihn nicht zurueck",
            paar[0],
            paar[1]
        );
    }
    assert_eq!(fight.phase(), 1);
}

#[test]
fn ac6_die_senkung_laeuft_ab_und_der_wechsel_kommt_ohne_weiteren_schaden() {
    // **Zwei Zeitpunkte im SELBEN Lauf**, wie die Auflage es verlangt: Zwei
    // getrennte Laeufe (einer mit, einer ohne) zeigen nur, DASS die Senkung
    // wirkt — nicht, dass sie endet.
    //
    // Der Aufbau haelt den Boss zwischen beiden Schwellenwerten fest:
    //
    //   ANCHOR gewirkt          Schwelle 70 -> effektiv 60
    //   noch etwas Schaden      Boss faellt unter 70, aber nicht unter 60
    //   Druecke eingestellt     der Lebensstand steht still
    //   Frist laeuft ab         Wechsel — ohne dass ein Punkt Schaden dazukam
    //
    // Am Ende steht der Boss ueber 60 und ist trotzdem gewechselt. Das kann
    // nichts anderes ausgeloest haben als das Ende der Senkung.
    //
    // Der zweite Zeitpunkt kommt aus einem **Praefix desselben Laufs**:
    // dieselbe Aufzeichnung, frueher abgeschnitten. Der Simulator ist
    // deterministisch (belegt durch die Pruefsummen je Takt), der kuerzere Lauf
    // ist also Takt fuer Takt derselbe — kein zweiter Lauf mit anderer
    // Bedingung, sondern ein Blick auf einen frueheren Takt.
    let wechsel = erster_wechsel_ohne_ausgabe(4_000);

    // Kurz VOR dem Wechsel wirken, danach noch etwas weiter Schaden.
    let fenster = wechsel.saturating_sub(FENSTER + 40);
    let letzter_schaden = fenster + FENSTER + 350;
    let wahl = move |tick: u32| {
        if (fenster..fenster + FENSTER).contains(&tick) {
            Some(1) // ANCHOR
        } else if tick < letzter_schaden {
            Some(0) // BIND
        } else {
            None // ab hier steht der Lebensstand still
        }
    };

    // **Der Wirkzeitpunkt kommt aus dem Protokoll**, nicht aus dem Fenster: Die
    // Frist laeuft ab dem Takt, an dem ANCHOR wirklich gewirkt hat.
    let (sondierung, _) = spielen(&["bind", "anchor"], letzter_schaden + 10, wahl);
    let gewirkt = gewirkt_bei(&sondierung, "anchor").expect("ANCHOR steht im Protokoll");
    let ablauf_takt = (gewirkt + PHASE_HOLD_DURATION_MS) / TICK_MS;

    // Zeitpunkt A: kurz vor Ablauf der Frist.
    let vor_ablauf = ablauf_takt - 50;
    assert!(
        vor_ablauf > letzter_schaden,
        "der Praefix endet bei Takt {vor_ablauf} und damit VOR dem letzten \
         Schaden bei {letzter_schaden} — dann steht der Lebensstand noch nicht \
         still und der Vergleich der beiden Zeitpunkte belegt nichts"
    );
    let (a, ergebnis_a) = spielen(&["bind", "anchor"], vor_ablauf, wahl);

    // Zeitpunkt B: eine Sekunde nach Ablauf der Frist, gleiche Aufzeichnung.
    let (b, ergebnis_b) = spielen(&["bind", "anchor"], ablauf_takt + 100, wahl);

    // **Die Ursache zuerst:** ANCHOR muss gewirkt worden sein, und der Boss
    // muss zum Zeitpunkt A tatsaechlich unter der NOMINALEN Schwelle stehen.
    // Stuende er darueber, waere sein Verbleiben in Phase 0 kein Verdienst der
    // Senkung, sondern schlicht sein Lebensstand.
    let anchor = index_of("anchor").expect("anchor");
    assert!(
        ergebnis_a.casts[anchor] > 0,
        "ANCHOR kam nicht zum Zug — dann liegt keine Senkung an"
    );
    assert!(
        a.boss_hp_percent() < ERSTE_SCHWELLE,
        "der Boss steht bei {} Prozent und damit nicht unter der Schwelle \
         {ERSTE_SCHWELLE} — dieser Fall zeigt dann nichts",
        a.boss_hp_percent()
    );
    // Und ueber der GESENKTEN Schwelle: Waere er auch darunter, muesste er
    // ohnehin wechseln, und sein Verbleiben in Phase 0 waere ein Fehler statt
    // eines Nachweises.
    let gesenkt = ERSTE_SCHWELLE - a.phase_hold_percent();
    assert!(
        a.boss_hp_percent() > gesenkt,
        "der Boss steht bei {} Prozent und damit unter der gesenkten Schwelle \
         {gesenkt} — dann haelt ihn nicht die Senkung in Phase 0",
        a.boss_hp_percent()
    );

    // **Zeitpunkt A — die Senkung gilt.** Der Boss ist unter der nominalen
    // Schwelle und wechselt trotzdem nicht.
    assert!(
        a.phase_hold_percent() > 0,
        "die Senkung liegt vor Ablauf der Frist nicht mehr an"
    );
    assert_eq!(
        a.phase(),
        0,
        "der Boss ist trotz gesenkter Schwelle gewechselt — bei {} Prozent \
         und einer Senkung um {} Punkte",
        a.boss_hp_percent(),
        a.phase_hold_percent()
    );

    // **Zeitpunkt B — die Senkung ist abgelaufen**, und zwar ohne dass ein
    // Punkt Schaden dazugekommen waere. Genau das belegt der gleiche
    // Lebensstand in A und B.
    assert_eq!(
        a.boss_hp_percent(),
        b.boss_hp_percent(),
        "zwischen beiden Zeitpunkten ist Schaden entstanden — dann koennte \
         der Wechsel auch davon kommen und nicht vom Ablauf der Frist"
    );
    assert_eq!(
        b.phase_hold_percent(),
        0,
        "die Senkung liegt nach Ablauf der Frist immer noch an"
    );
    assert_eq!(
        b.phase(),
        1,
        "der Boss hat nach Ablauf der Frist nicht gewechselt"
    );

    // Und der Wechsel liegt auf dem Ablauf der Frist, nicht irgendwo.
    let wechsel_b = wechsel_bei(&ergebnis_b.phases);
    assert_eq!(wechsel_b.len(), 1, "genau ein Wechsel erwartet");
    assert!(
        wechsel_b[0] >= ablauf_takt,
        "der Wechsel kam bei Takt {} und damit VOR dem Ablauf der Frist bei \
         Takt {ablauf_takt}",
        wechsel_b[0]
    );
}

#[test]
fn ac7_ein_druck_verschiebt_den_naechsten_wechsel_und_den_uebernaechsten_nicht() {
    // **Die Aussage aus GAME-DESIGN 4.2b, die nur mit ZWEI Schwellen pruefbar
    // ist.** Vor dem 2026-08-04 verschob ein einziger Druck beide Wechsel, weil
    // die Senkung nie endete — und kein Fall bemerkte es.
    let ende = 4_000;
    let wechsel = erster_wechsel_ohne_ausgabe(ende);
    let fenster = wechsel.saturating_sub(FENSTER + 40);

    // Beide Laeufe geben Kontroll-Punkte aus und zahlen dieselbe Wartezeit.
    // Unterschied ist allein die Wirkung. Siehe den Kopf dieser Datei.
    let bauen = |zauber: &'static str| {
        let (fight, ergebnis) = spielen(&["bind", zauber], ende, move |tick| {
            Some(u8::from((fenster..fenster + FENSTER).contains(&tick)))
        });
        let gewirkt = ergebnis.casts[index_of(zauber).expect("Zauber")];
        (
            wechsel_bei(&ergebnis.phases),
            gewirkt,
            gewirkt_bei(&fight, zauber),
        )
    };

    let (mit_stay, stay_gewirkt, stay_bei) = bauen("stay");
    let (mit_anchor, anchor_gewirkt, anchor_bei) = bauen("anchor");

    // **Die Ursache zuerst:** Beide Zauber muessen zum Zug gekommen sein. Waere
    // ANCHOR abgelehnt worden (zu wenig Punkte, laufende Wartezeit), liefen
    // beide Laeufe gleich — und der Fall bliebe gruen, ohne etwas zu zeigen.
    assert!(stay_gewirkt > 0, "STAY kam im Vergleichslauf nicht zum Zug");
    assert!(anchor_gewirkt > 0, "ANCHOR kam nicht zum Zug");
    assert_eq!(mit_stay.len(), 2, "der Vergleichslauf braucht zwei Wechsel");
    assert_eq!(mit_anchor.len(), 2, "beide Laeufe brauchen zwei Wechsel");

    // **Und beide zum selben Zeitpunkt.** Waeren die Wartezeiten verschieden
    // gefallen, unterschieden sich die Laeufe nicht nur in der Wirkung,
    // sondern auch im Schadensverlauf — dann verglichen die beiden Aussagen
    // unten zwei verschiedene Kaempfe.
    assert_eq!(
        anchor_bei, stay_bei,
        "die beiden Zauber wirkten zu verschiedenen Zeitpunkten \
         ({anchor_bei:?} gegen {stay_bei:?}) — der Vergleich haelt dann nicht \
         alles ausser der Wirkung fest"
    );

    // Der naechste Wechsel rueckt. Geprueft wird die UNGLEICHUNG, nicht ein
    // Zahlenwert: Ein erwarteter Betrag, der aus derselben Rechnung stammt,
    // waere Zirkelschluss.
    assert!(
        mit_anchor[0] > mit_stay[0],
        "der naechste Wechsel ist nicht gerueckt: Takt {} mit ANCHOR gegen {} \
         mit STAY — dann wirkt die Senkung nicht",
        mit_anchor[0],
        mit_stay[0]
    );

    // **Und der uebernaechste bleibt, wo er war.** Das ist der eigentliche
    // Nachweis; alles davor zeigt nur, dass ueberhaupt etwas geschieht.
    assert_eq!(
        mit_anchor[1], mit_stay[1],
        "der uebernaechste Wechsel ist mitgerueckt (Takt {} gegen {}) — ein \
         Druck darf nur den NAECHSTEN verschieben (GAME-DESIGN 4.2b)",
        mit_anchor[1], mit_stay[1]
    );
}

#[test]
fn ac7b_der_wechsel_verbraucht_die_senkung_obwohl_die_frist_noch_laeuft() {
    // **Diesen Fall gibt es wegen einer Gegenprobe, die NICHT griff.**
    //
    // Am 2026-08-04 wurde `phase_hold_percent = 0` aus `check_phase` entfernt —
    // der Verbrauch also abgeschaltet. `ac7` blieb gruen. Der Grund: Bei
    // diesem Prueff-Boss liegen die beiden Schwellen weiter auseinander, als
    // die Frist lang ist. Bis der uebernaechste Wechsel faellig wird, ist die
    // Senkung ohnehin abgelaufen — der Fristablauf erledigt, was eigentlich
    // der Verbrauch tun soll.
    //
    // `ac7` zeigt damit das ERGEBNIS („der uebernaechste rueckt nicht"), aber
    // nicht die URSACHE. Beide Regeln aus GAME-DESIGN 4.2b sind eigenstaendig,
    // und eine davon war nicht abgedeckt.
    //
    // Hier wird sie es: Der Lauf endet unmittelbar nach dem Wechsel, **waehrend
    // die Frist noch laeuft**. Ist die Senkung dann schon fort, kann das nur
    // der Wechsel getan haben.
    let wechsel = erster_wechsel_ohne_ausgabe(4_000);
    // Das Fenster liegt so spaet wie moeglich vor dem Wechsel: Je spaeter
    // gewirkt wird, desto weiter reicht die Frist ueber den verschobenen
    // Wechsel hinaus — und nur in diesem Streifen ist der Verbrauch vom
    // Fristablauf zu unterscheiden.
    let fenster = wechsel.saturating_sub(FENSTER);
    let wahl = move |tick: u32| Some(u8::from((fenster..fenster + FENSTER).contains(&tick)));

    // **Wo der verschobene Wechsel liegt, wird gemessen, nicht geschaetzt.**
    // Ein geratenes Ende landete entweder davor (dann hat der Boss nicht
    // gewechselt) oder hinter dem Fristablauf (dann belegt der Fall nichts).
    let (_, sondierung) = spielen(&["bind", "anchor"], 4_000, wahl);
    let verschoben = *wechsel_bei(&sondierung.phases)
        .first()
        .expect("der Boss muss auch mit Senkung wechseln");
    let ende = verschoben + 20;

    let (fight, ergebnis) = spielen(&["bind", "anchor"], ende, wahl);

    // **Die Ursache zuerst**, und hier gehoert die Frist ausdruecklich dazu.
    let anchor = index_of("anchor").expect("anchor");
    assert!(ergebnis.casts[anchor] > 0, "ANCHOR kam nicht zum Zug");
    assert!(
        fight.phase() > 0,
        "der Boss hat nicht gewechselt — dann ist nichts zu verbrauchen"
    );
    let gewirkt = gewirkt_bei(&fight, "anchor").expect("ANCHOR steht im Protokoll");
    let ende_ms = ende * TICK_MS;
    assert!(
        ende_ms < gewirkt + PHASE_HOLD_DURATION_MS,
        "der Lauf endet bei {ende_ms} ms, die Frist erst bei {} ms — dann \
         koennte auch der Fristablauf die Senkung genommen haben, und der Fall \
         belegt den Verbrauch nicht",
        gewirkt + PHASE_HOLD_DURATION_MS
    );

    // Der Nachweis.
    assert_eq!(
        fight.phase_hold_percent(),
        0,
        "die Senkung liegt nach dem Wechsel noch an — dann verschiebt ein \
         einziger Druck auch den uebernaechsten Wechsel, sobald zwei Schwellen \
         nahe genug beieinander liegen (GAME-DESIGN 4.2b: „nur der naechste“)"
    );
}

#[test]
fn ac8_die_referenz_rechnung_ist_unberuehrt() {
    // Weder der neue Prueff-Boss noch die Befristung duerfen die Referenz
    // bewegen. Sie laeuft gegen einen anderen Boss und gibt keine
    // Kontroll-Punkte aus.
    let mut fight = Fight::new(FightConfig::default());
    fn gute_rotation(f: &Fight) -> Option<usize> {
        let kindle = index_of("kindle").expect("kindle");
        let smoulder = index_of("smoulder").expect("smoulder");
        let pyre = index_of("pyre").expect("pyre");
        if f.dot_remaining_ms(smoulder) < 1500 && f.is_ready(smoulder) {
            Some(smoulder)
        } else if f.is_ready(pyre) && f.resource() >= 15 {
            Some(pyre)
        } else if f.is_ready(kindle) {
            Some(kindle)
        } else {
            None
        }
    }
    let ergebnis = fight.run(gute_rotation as fn(&Fight) -> Option<usize>);
    assert_eq!(
        ergebnis.damage, 4240,
        "die Referenz-Rechnung hat sich bewegt"
    );
    assert_eq!(ergebnis.killed_at_ms, Some(40_510));
}
