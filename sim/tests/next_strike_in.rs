//! `boss.next_strike_in` — der Wert, mit dem sich ein Schlag vorbereiten lässt
//! (Issue #797).
//!
//! # Warum es ihn gibt
//!
//! #797 hat `boss.casting` und `boss.cast_remains` gebaut: den **laufenden**
//! Boss-Zauber. Das ist richtig für QUENCH, das einen laufenden Cast beendet.
//! Für PIN ist es falsch — der Zauber gilt laut `data/spells/fixation.json`
//! dem **nächsten** Eintrag, und traf er einen ohnehin unterbrechbaren,
//! verpufft er. Die Zeile, die der Kandidaten-Erzeuger für PIN bildete
//! (`pin if boss.casting`), feuerte also genau dann, wenn PIN nichts
//! ausrichtet.
//!
//! Daran ist AC2 gescheitert: **formal erfüllt, sachlich leer.** Ein Zauber
//! hatte eine Kandidatenzeile, sie stand in jedem Bericht als Abdeckung, und
//! sie konnte nie etwas bewirken. Das ist der teurere von beiden Fehlern —
//! eine fehlende Zeile fällt beim Zählen auf.
//!
//! # Wie hier mitten im Kampf gemessen wird
//!
//! Der Simulator hat keinen Schrittbetrieb: `run_apl` läuft bis zum Ende, und
//! wer die Dauer auf den Messzeitpunkt kürzt, misst dort immer die Restzeit 0.
//! Beobachtet wird deshalb mit einer **Sonde** — einer Prioritätenliste aus
//! einer einzigen Zeile:
//!
//! ```text
//! wisp if boss.next_strike_in <= 3000
//! ```
//!
//! `wisp` kostet nichts, hat die Abklingzeit eines GCD und keine Nebenwirkung
//! außer 20 Schaden. Der **Zeitpunkt seines ersten Wurfs** steht im Protokoll
//! und sagt, wann der Wert die Schwelle unterschritten hat. Damit ist der
//! Wertverlauf von außen ablesbar, ohne dass der Prüffall die Formel nachbaut.

use rota_sim::apl::{parse, AplName, Value};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, ControlEffect, SPELLS, SPELL_COUNT};
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{
    all_candidates, candidates_for, greedy, paar_zuege, Origin, NEXT_STRIKE_FENSTER_MS,
};
use rota_sim::{Fight, FightConfig};

/// Der Boss mit einem Schlag bei 12000 ms, den man verhindern **muss**.
const TOEDLICH: &str = "test-lethal-strike";

/// Sein Zwilling: derselbe Boss, Schlag auf 4600 gesenkt.
///
/// Die beiden unterscheiden sich in **genau einer Zahl**. Damit ist er die
/// Kontrolle für jede Aussage über die Tödlichkeit — was an beiden gleich
/// ausgeht, liegt nicht am Schlag.
const UEBERLEBBAR: &str = "test-survivable-strike";

/// Ein Boss mit **leerer** Zeitleiste.
///
/// Er ist der Fall „kein Schlag kommt mehr" in Reinform — und zugleich der
/// Beleg für den Weckruf: Ohne Zeitleiste gibt es im ganzen Kampf kein
/// Boss-Ereignis, das die Prioritätenliste von selbst wecken würde.
const OHNE_SCHLAG: &str = "test-resistant";

/// Der Sondenzauber.
const SONDE: &str = "wisp";

/// Fährt eine Prioritätenliste und gibt den Zeitpunkt jedes Wurfs von
/// `zauber_id` zurück.
fn wuerfe_von(boss_id: &str, apl_text: &str, zauber_id: &str) -> Vec<u32> {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("{boss_id} fehlt im Datensatz"));
    let apl = parse(AplName("sonde"), apl_text)
        .unwrap_or_else(|e| panic!("Sondenliste ist ungueltig: {e:?}"));

    let gesucht = SPELLS
        .iter()
        .position(|s| s.id == zauber_id)
        .unwrap_or_else(|| panic!("{zauber_id} fehlt im Zauberdatensatz"));

    let mut fight = Fight::new(FightConfig {
        boss,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(&apl);

    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::Cast && usize::from(e.spell) == gesucht)
        .map(|e| e.time)
        .collect()
}

/// Der erste Wurf der Sonde, oder `None`, wenn sie nie geworfen wurde.
fn erster_wurf(boss_id: &str, apl_text: &str) -> Option<u32> {
    wuerfe_von(boss_id, apl_text, SONDE).first().copied()
}

// ---------------------------------------------------------------------------
// Was der Wert zählt
// ---------------------------------------------------------------------------

/// Der Wert zählt bis zum nächsten Zeitleisten-Eintrag herunter.
///
/// Gemessen wird der Zeitpunkt, an dem eine Schwelle unterschritten wird —
/// **nicht** eine Zahl, die der Prüffall selbst ausrechnet. Deshalb fährt er
/// drei Schwellen und prüft den **Abstand** zwischen ihnen: Steigt die
/// Schwelle um 1500, muss der Wurf um 1500 nach vorn wandern.
///
/// **Eine einzelne Schwelle wäre der schwächere Fall.** Sie ließe sich auch
/// erfüllen, wenn der Wert eine feste Zahl zurückgäbe, die zufällig passt. Der
/// Abstand kann das nicht: Er verlangt, dass der Wert tatsächlich mit der Uhr
/// sinkt.
#[test]
fn der_wert_zaehlt_bis_zum_schlag_herunter() {
    let messungen: Vec<(u32, Option<u32>)> = [1500_u32, 3000, 4500]
        .into_iter()
        .map(|schwelle| {
            (
                schwelle,
                erster_wurf(
                    TOEDLICH,
                    &format!("{SONDE} if boss.next_strike_in <= {schwelle}\n"),
                ),
            )
        })
        .collect();

    for (schwelle, wurf) in &messungen {
        assert!(
            wurf.is_some(),
            "bei Schwelle {schwelle} wurde die Sonde nie geworfen — dann \
             unterschreitet der Wert die Schwelle nie, und er zaehlt nicht \
             herunter. Reihe: {messungen:?}"
        );
    }

    let zeiten: Vec<u32> = messungen.iter().filter_map(|(_, w)| *w).collect();
    for paar in zeiten.windows(2) {
        // **In i64 gerechnet, nicht in u32.** Wandert der Wurf in die falsche
        // Richtung, liefe eine u32-Subtraktion unter und der Fall bräche mit
        // „attempt to subtract with overflow" — einer Meldung, die nichts über
        // die Sache sagt. Mit Vorzeichen steht die gemessene Richtung da.
        let abstand = i64::from(paar[0]) - i64::from(paar[1]);
        assert_eq!(
            abstand, 1500,
            "eine um 1500 groessere Schwelle muss den Wurf um 1500 nach VORN \
             holen; gemessen {paar:?} (Abstand {abstand}) in der Reihe \
             {messungen:?}"
        );
    }

    // Der Schlag steht bei 12000. Eine Schwelle von 3000 muss ihn also 3000 ms
    // vorher sichtbar machen — die Zahl steht hier, damit ein Verschieben des
    // Bezugspunkts auffaellt und nicht bloss die Abstaende weiterstimmen.
    assert_eq!(
        zeiten[1], 9000,
        "bei Schwelle 3000 gehoert der erste Wurf auf 9000 (Schlag bei 12000); \
         gemessen {zeiten:?}"
    );
}

/// Die Verzögerung aus STAY wandert mit — der Wert folgt dem **tatsächlich**
/// eingeplanten Zeitpunkt, nicht dem in den Boss-Daten.
///
/// # Warum das der Fall ist, der leicht fehlt
///
/// `timeline_at` liefert den Zeitpunkt aus der Zeitleiste. Was der Kampf
/// einplant, ist `at_ms + next_strike_delay` (`schedule_next_timeline_step`).
/// Ein Getter, der nur `timeline_at` abfragt, ist in jedem gewöhnlichen Kampf
/// richtig und genau dann falsch, wenn STAY oder LATCH gewirkt wurde — also in
/// der Lage, für die der Wert gebaut wurde.
/// # Warum STAY hier an ein Zeitfenster gebunden wird
///
/// Die erste Fassung schrieb `stay if control >= 3` und erwartete eine
/// Verschiebung um 3000. Gemessen wurden **6500**, und der Grund war nicht der
/// Wert, sondern die Liste: STAY wurde **mehrfach** gewirkt, sobald die
/// Abklingzeit fiel und wieder genug Punkte da waren. Die Verzögerungen
/// summieren sich (`next_strike_delay` wird erst beim Auslösen zurückgesetzt),
/// und die Erwartung „3000" beschrieb einen Lauf, den es nicht gab.
///
/// `time >= 3000 and time < 4000` lässt genau einen Takt zu. Damit steht die
/// Zahl 3000 nicht mehr als Annahme im Test, sondern folgt aus der Wirkung
/// **eines** Wurfs.
#[test]
fn die_verzoegerung_aus_stay_wandert_mit() {
    // Beide Läufe tragen dieselbe Liste bis auf die STAY-Zeile: derselbe
    // Aufbau von Kontroll-Punkten, dieselbe Sonde, derselbe Platz. Ohne das
    // unterschieden sich die Läufe um den Zauber UND um die GCD-Last.
    let ohne = erster_wurf(
        TOEDLICH,
        &format!(
            "bind if control < 3\n\
             {SONDE} if boss.next_strike_in <= 3000\n"
        ),
    )
    .expect("ohne Verzoegerung wird die Sonde geworfen");

    let mit = erster_wurf(
        TOEDLICH,
        &format!(
            "stay if time >= 3000 and time < 4000\n\
             bind if control < 3\n\
             {SONDE} if boss.next_strike_in <= 3000\n"
        ),
    )
    .expect("mit Verzoegerung wird die Sonde geworfen");

    // Die Zusicherung, ohne die die Zahl nichts bedeutet: Wurde STAY gar nicht
    // oder mehrfach geworfen, misst der Vergleich etwas anderes als die
    // Verzögerung eines Wurfs.
    let stay_wuerfe = wuerfe_von(
        TOEDLICH,
        &format!(
            "stay if time >= 3000 and time < 4000\n\
             bind if control < 3\n\
             {SONDE} if boss.next_strike_in <= 3000\n"
        ),
        "stay",
    );
    assert_eq!(
        stay_wuerfe.len(),
        1,
        "dieser Fall braucht GENAU einen STAY-Wurf; gemessen wurden \
         {stay_wuerfe:?}. Bei mehreren summieren sich die Verzoegerungen, und \
         die erwarteten 3000 beschreiben dann keinen Lauf"
    );

    assert_eq!(
        i64::from(mit) - i64::from(ohne),
        3000,
        "STAY verzoegert den Schlag um 3000 ms, also muss der Wert 3000 ms \
         spaeter unter die Schwelle fallen; gemessen ohne {ohne}, mit {mit}, \
         STAY geworfen bei {stay_wuerfe:?}"
    );
}

// ---------------------------------------------------------------------------
// Wenn kein Schlag mehr kommt
// ---------------------------------------------------------------------------

/// Ohne weiteren Eintrag ist der Wert die verbleibende Kampfzeit — und das ist
/// die **Definition**, kein Näherungswert.
///
/// # Wie das nachprüfbar wird, ohne trivial zu sein
///
/// Am Kampfende sind beide Werte 0, und `0 == 0` wäre eine Zusicherung, die
/// gar nicht scheitern kann. Verglichen wird deshalb das **Verhalten**: Zwei
/// Sondenlisten, eine mit `boss.next_strike_in`, eine mit `fight.remains`,
/// müssen an einem Boss ohne Zeitleiste denselben ersten Wurf ergeben.
///
/// Und die **Gegenprobe** gehört dazu: An einem Boss MIT Schlag müssen sie
/// auseinandergehen. Ohne sie bliebe der Fall auch dann grün, wenn jemand
/// `next_strike_in` schlicht auf `fight_remaining_ms` verkürzte — der Wert
/// wäre dann ein zweiter Name für dieselbe Sache.
#[test]
fn ohne_schlag_ist_der_wert_die_restzeit() {
    let neu = erster_wurf(
        OHNE_SCHLAG,
        &format!("{SONDE} if boss.next_strike_in <= 3000\n"),
    );
    let alt = erster_wurf(OHNE_SCHLAG, &format!("{SONDE} if fight.remains <= 3000\n"));

    assert!(
        neu.is_some(),
        "ohne Zeitleiste muss der Wert trotzdem unter die Schwelle fallen — \
         sonst meldet er dort etwas anderes als die Restzeit"
    );
    assert_eq!(
        neu, alt,
        "ohne weiteren Eintrag IST der Wert die Restzeit; beide Sonden \
         gehoeren auf denselben Zeitpunkt (next_strike_in {neu:?}, \
         fight.remains {alt:?})"
    );

    let neu_mit_schlag = erster_wurf(
        TOEDLICH,
        &format!("{SONDE} if boss.next_strike_in <= 3000\n"),
    );
    let alt_mit_schlag = erster_wurf(TOEDLICH, &format!("{SONDE} if fight.remains <= 3000\n"));
    assert_ne!(
        neu_mit_schlag, alt_mit_schlag,
        "an einem Boss MIT Schlag muessen die beiden auseinandergehen — sonst \
         ist next_strike_in nur ein zweiter Name fuer fight.remains \
         (beide {neu_mit_schlag:?})"
    );
}

/// Der Weckruf steht — und zwar gemessen an seiner Wirkung, nicht an der
/// Einordnung in `is_time_dependent`.
///
/// # Warum die Einordnung allein nichts belegt
///
/// `assert!(Value::BossNextStrikeIn.is_time_dependent())` liest die Liste
/// vor, in die der Wert eingetragen wurde. Sie ist grün, weil dort etwas
/// steht — nicht, weil es etwas bewirkt.
///
/// Der Boss ohne Zeitleiste trennt beides: In seinem Kampf gibt es **kein**
/// Boss-Ereignis. Fällt der Wert nicht unter die Weckruf-Regel, hält der
/// ereignisgesteuerte Simulator nirgends an, und die Sonde wird nie geworfen.
/// Dass sie fliegt, kann nur am Weckruf liegen.
#[test]
fn der_weckruf_oeffnet_das_fenster() {
    let boss = boss_by_id(OHNE_SCHLAG).expect("test-resistant fehlt im Datensatz");
    assert!(
        boss.timeline.is_empty(),
        "dieser Fall braucht einen Boss OHNE Zeitleiste — {OHNE_SCHLAG} traegt \
         inzwischen {} Eintraege, und dann weckt die Zeitleiste die Liste, \
         nicht der Wert",
        boss.timeline.len()
    );

    assert!(
        erster_wurf(
            OHNE_SCHLAG,
            &format!("{SONDE} if boss.next_strike_in <= 3000\n")
        )
        .is_some(),
        "ohne Boss-Ereignis kann die Sonde nur ueber einen Weckruf fliegen — \
         sie flog nicht, also fehlt {:?} in Value::is_time_dependent",
        Value::BossNextStrikeIn
    );

    // Die Einordnung selbst gehoert trotzdem festgenagelt: Der Wirkungsfall
    // oben bricht auch, wenn jemand ganz woanders etwas kaputt macht, und dann
    // waere nicht ablesbar, WAS gemeint war.
    assert!(
        Value::BossNextStrikeIn.is_time_dependent(),
        "der Wert sinkt mit jeder Millisekunde und braucht deshalb einen Weckruf"
    );
}

// ---------------------------------------------------------------------------
// Der Kandidaten-Erzeuger — die Stelle, die kein Übersetzer erzwingt
// ---------------------------------------------------------------------------

/// PIN bekommt eine Zeile mit dem neuen Wert — und **keine** mehr auf
/// `boss.casting`.
///
/// # Warum beide Hälften nötig sind
///
/// Die erste allein hieße „der Wert ist irgendwo angeschlossen". Die zweite
/// ist die eigentliche Korrektur: Die alte Zeile war gültig, wählbar und
/// wirkungslos, und solange sie danebensteht, kann die Suche sie weiter
/// wählen.
#[test]
fn der_erzeuger_bildet_die_zeile_fuer_pin() {
    let pin = SPELLS
        .iter()
        .position(|s| s.id == "pin")
        .expect("pin fehlt im Zauberdatensatz");

    let zeilen = candidates_for(pin);
    let herkuenfte: Vec<Origin> = zeilen.iter().map(|k| k.origin).collect();

    assert!(
        herkuenfte.contains(&Origin::NextStrikeWindow),
        "PIN braucht eine Zeile aus dem Fenster vor dem naechsten Schlag; \
         gebildet wurden {herkuenfte:?}"
    );
    assert!(
        !herkuenfte.contains(&Origin::BossCastWindow),
        "PIN darf keine Zeile auf den LAUFENDEN Cast mehr bekommen — laut \
         Zauberdaten verpufft er an einem ohnehin unterbrechbaren Eintrag; \
         gebildet wurden {herkuenfte:?}"
    );

    let zeile = zeilen
        .iter()
        .find(|k| k.origin == Origin::NextStrikeWindow)
        .expect("gerade zugesichert");
    assert_eq!(
        format!("{}", zeile.condition),
        format!("boss.next_strike_in <= {NEXT_STRIKE_FENSTER_MS}"),
        "die gebildete Zeile muss den Wert und die gemessene Fensterlaenge \
         tragen — eine Zeile mit einer anderen Schwelle waere eine zweite, \
         ungemessene Zahl"
    );
}

/// Genau die Zauber mit `MakeInterruptible` bekommen die neue Zeile — und
/// CAPSTONE ausdrücklich **nicht**.
///
/// # Der Fund, der diesen Fall nötig macht
///
/// Der Doc-Kommentar an `ControlEffect::AllThree` sagt „alle drei bisherigen
/// Wirkungen auf einmal". Wer daraus die drei im `enum` davor stehenden liest,
/// zählt `MakeInterruptible` mit. `Fight::apply_control_effect` ruft aber
/// `SlowCast`, `DelayStrike` und `HoldPhase` — CAPSTONE macht **nichts**
/// unterbrechbar.
///
/// Der Fall nagelt die gemessene Liste fest, nicht die gelesene. Und er nennt
/// die betroffenen Zauber beim Namen statt ihrer Anzahl: Kommt ein zweiter
/// Zauber mit `MakeInterruptible` dazu, soll der Fall die Änderung zeigen und
/// nicht bloß eine Zahl bemängeln.
#[test]
fn nur_wer_unterbrechbar_macht_bekommt_die_zeile() {
    let mit_zeile: Vec<&'static str> = (0..SPELL_COUNT)
        .filter(|&i| {
            candidates_for(i)
                .iter()
                .any(|k| k.origin == Origin::NextStrikeWindow)
        })
        .map(|i| SPELLS[i].id)
        .collect();

    let macht_unterbrechbar: Vec<&'static str> = (0..SPELL_COUNT)
        .filter(|&i| {
            SPELLS[i]
                .control
                .is_some_and(|c| c.effect == ControlEffect::MakeInterruptible)
        })
        .map(|i| SPELLS[i].id)
        .collect();

    assert_eq!(
        mit_zeile, macht_unterbrechbar,
        "die Zeile gehoert genau den Zaubern mit MakeInterruptible"
    );

    // CAPSTONE beim Namen, weil er der Zauber ist, an dem die Verwechslung
    // haengt: Er traegt AllThree und steht im `enum` hinter MakeInterruptible.
    assert!(
        !mit_zeile.contains(&"capstone"),
        "CAPSTONE macht nichts unterbrechbar (apply_control_effect ruft \
         SlowCast, DelayStrike, HoldPhase) und darf die Zeile nicht bekommen"
    );
    let capstone = SPELLS
        .iter()
        .position(|s| s.id == "capstone")
        .expect("capstone fehlt im Zauberdatensatz");
    assert!(
        candidates_for(capstone)
            .iter()
            .any(|k| k.origin == Origin::BossCastWindow),
        "CAPSTONE traegt SlowCast und behaelt deshalb seine Zeile auf den \
         laufenden Cast — der Umbau darf sie ihm nicht mit wegnehmen"
    );
}

/// Die neue Zeile taucht in `all_candidates` auf.
///
/// `candidates_for` und `all_candidates` sind zwei Wege zu derselben Liste,
/// und die Suche benutzt den zweiten. Ein Fall, der nur den ersten prüft,
/// bliebe grün, wenn der zweite die Zeile unterwegs verlöre.
#[test]
fn die_zeile_steht_auch_in_der_gesamtliste() {
    let alle = all_candidates();
    let neue: Vec<&'static str> = alle
        .iter()
        .filter(|k| k.origin == Origin::NextStrikeWindow)
        .map(|k| SPELLS[k.spell].id)
        .collect();

    assert_eq!(
        neue,
        vec!["pin"],
        "die Gesamtliste muss genau die Zeile fuer PIN enthalten"
    );
}

/// `Origin::ALL` ist vollständig — geprüft an einer Stelle, an der der
/// Übersetzer nicht hinsieht.
///
/// # Der Fund, der diesen Fall nötig macht
///
/// `sim/examples/rotation-search.rs` zählt die Kandidatenzeilen je Herkunft.
/// Bis #797 führte er dafür eine wörtlich hingeschriebene Aufzählung, und der
/// Kommentar daneben sagte den Fehler voraus: „Wer einen Herkunfts-Typ in
/// `Origin` ergänzt, bekommt hier keine Zeile und merkt es nicht, weil die
/// Zahl einfach fehlt."
///
/// Beim Bau von [`Origin::NextStrikeWindow`] ist genau das eingetreten — und
/// zwar **nur halb** sichtbar: Das `match` in der Zählfunktion brach den Bau
/// (`cargo clippy --all-targets`), die Aufzählung in der Ausgabe hätte still
/// weitergezählt. Ohne `--all-targets` wäre auch die Hälfte nicht aufgefallen:
/// `cargo test` allein übersetzt die Beispiele nicht.
///
/// # Warum die Prüfung so und nicht über eine Zahl geht
///
/// `assert_eq!(Origin::ALL.len(), 6)` wäre eine zweite Stelle, die beim
/// nächsten Typ gepflegt werden müsste — dieselbe Klasse Fehler noch einmal.
/// Geprüft wird stattdessen die **Eigenschaft**: Jeder Platz in `ALL` trägt
/// die Art, die ihn für sich beansprucht. Doppelte Plätze und eine falsche
/// Reihenfolge fallen damit auf.
#[test]
fn jede_herkunft_steht_in_der_liste() {
    for (platz, origin) in Origin::ALL.iter().enumerate() {
        assert_eq!(
            origin.ordnung(),
            platz,
            "{origin:?} steht auf Platz {platz}, beansprucht aber {} — dann \
             zaehlen zwei Arten auf denselben Zaehler oder die Liste ist \
             vertauscht",
            origin.ordnung()
        );
    }

    // Die Gegenprobe zur Vollständigkeit: Jede Art, die tatsächlich eine Zeile
    // erzeugt, muss in der Liste stehen. Das fängt den Fall, den die Schleife
    // oben nicht sieht — eine Art, die es gibt und die in `ALL` fehlt.
    for kandidat in all_candidates() {
        assert!(
            Origin::ALL.contains(&kandidat.origin),
            "{:?} erzeugt Kandidatenzeilen, steht aber nicht in Origin::ALL — \
             dann fehlt sie in jeder Aufschluesselung des Berichts",
            kandidat.origin
        );
    }
}

// ---------------------------------------------------------------------------
// Die Wirkung
// ---------------------------------------------------------------------------

/// Mit der neuen Zeile ist der Schlag abwendbar — ohne sie nicht.
///
/// # Warum hier zwei Läufe verglichen werden
///
/// „Der Spieler überlebt" allein wäre auch dann grün, wenn er den Schlag
/// schlicht aushielte. Der Gegenlauf trägt dieselbe Rotation **ohne** die
/// PIN-Zeile und muss sterben — erst die Differenz zeigt, dass die Zeile es
/// war.
///
/// **Zur GCD-Last:** Die beiden Läufe unterscheiden sich um genau einen Takt,
/// und zwar um dessen Inhalt, nicht um seine Existenz. Im Lauf ohne PIN füllt
/// der Sondenzauber aus der letzten Zeile denselben Platz — die Liste endet in
/// beiden Fällen auf einem bedingungslosen Füller, der jeden freien Takt
/// belegt. Eine zusätzliche Platzhalterzeile wäre wirkungslos gewesen: Eine
/// Prioritätenliste wählt die erste zutreffende Zeile, und der Füller trifft
/// immer.
#[test]
fn die_zeile_wendet_den_schlag_ab() {
    fn ueberlebt(apl_text: &str) -> bool {
        let boss = boss_by_id(TOEDLICH).expect("test-lethal-strike fehlt im Datensatz");
        let apl = parse(AplName("abwehr"), apl_text)
            .unwrap_or_else(|e| panic!("Liste ist ungueltig: {e:?}"));
        let mut fight = Fight::new(FightConfig {
            boss,
            ..FightConfig::default()
        });
        let _ = fight.run_apl(&apl);
        !fight.player_dead()
    }

    let mit = format!(
        "quench if boss.casting\n\
         pin if boss.next_strike_in <= {NEXT_STRIKE_FENSTER_MS}\n\
         bind if control < 3\n\
         {SONDE}\n"
    );
    let ohne = format!(
        "quench if boss.casting\n\
         bind if control < 3\n\
         {SONDE}\n"
    );

    assert!(
        ueberlebt(&mit),
        "mit PIN wird der Schlag unterbrechbar und QUENCH beendet ihn — der \
         Spieler muss ueberleben"
    );
    assert!(
        !ueberlebt(&ohne),
        "ohne PIN ist der Schlag unabwendbar (18400 gegen eine gemessene \
         Obergrenze von 9200) — ueberlebt der Spieler auch so, misst der Fall \
         nicht mehr, was er zu messen vorgibt"
    );
}

/// Die Fensterlänge ist **gemessen**, nicht gewählt.
///
/// # Was hier gefahren wird
///
/// Für eine Reihe von Schwellen wird gefragt, ob eine Rotation mit
/// `pin if boss.next_strike_in <= X` den Schlag abwendet. Die Reihe steht
/// vollständig in der Fehlermeldung, damit bei einer Änderung ablesbar ist,
/// wie sich das Fenster verschoben hat — und nicht nur, dass die Zusicherung
/// brach.
///
/// Zwei Grenzen sind zu erwarten: Unterhalb eines GCD kann das Fenster
/// zwischen zwei Auswertungen verstreichen; PIN wird dann nie geworfen. Nach
/// oben hält `pin_window` bis zum nächsten Eintrag, ganz gleich welcher es ist
/// — an einem Boss mit **einem** Eintrag gibt es also keine obere Grenze, an
/// einem mit mehreren schon.
#[test]
fn das_fenster_ist_gemessen() {
    fn wendet_ab(schwelle: u32) -> bool {
        let boss = boss_by_id(TOEDLICH).expect("test-lethal-strike fehlt im Datensatz");
        let text = format!(
            "quench if boss.casting\n\
             pin if boss.next_strike_in <= {schwelle}\n\
             bind if control < 3\n\
             {SONDE}\n"
        );
        let apl = parse(AplName("fenster"), &text)
            .unwrap_or_else(|e| panic!("Liste ist ungueltig: {e:?}"));
        let mut fight = Fight::new(FightConfig {
            boss,
            ..FightConfig::default()
        });
        let _ = fight.run_apl(&apl);
        !fight.player_dead()
    }

    let reihe: Vec<(u32, bool)> = [500_u32, 1000, 1500, 3000, 4500, 6000, 9000]
        .into_iter()
        .map(|s| (s, wendet_ab(s)))
        .collect();

    assert!(
        reihe.iter().any(|(_, ab)| !ab),
        "keine Schwelle scheitert — dann misst die Reihe das Fenster nicht, \
         sondern zeigt nur, dass PIN ueberhaupt wirkt. Reihe: {reihe:?}"
    );
    assert!(
        reihe.iter().any(|(_, ab)| *ab),
        "keine Schwelle wendet den Schlag ab. Reihe: {reihe:?}"
    );

    let gewaehlt = reihe
        .iter()
        .find(|(s, _)| *s == NEXT_STRIKE_FENSTER_MS)
        .map(|(_, ab)| *ab);
    assert_eq!(
        gewaehlt,
        Some(true),
        "NEXT_STRIKE_FENSTER_MS = {NEXT_STRIKE_FENSTER_MS} muss in der Reihe \
         stehen und den Schlag abwenden. Reihe: {reihe:?}"
    );
}

/// Die Suche findet die Zeile jetzt auch von selbst.
///
/// # Warum das über die vorigen Fälle hinausgeht
///
/// Dort steht die PIN-Zeile in einer **von Hand** geschriebenen Liste. Hier
/// baut `greedy` die Rotation aus den Kandidaten — genau der Weg, der in AC2
/// leerlief. Ein Wert, für den keine wählbare Zeile existiert, taucht in
/// keiner gefundenen Rotation auf, ganz gleich wie nützlich er wäre.
/// Das Paar „erst angreifbar machen, dann unterbrechen" wird überhaupt
/// gebildet.
///
/// # Warum das eine eigene Frage ist
///
/// „Die Suche wählt die Zeile nicht" hat zwei ganz verschiedene Ursachen: Das
/// Paar entsteht nicht, oder es entsteht und gewinnt nicht. Beide sehen im
/// Ergebnis gleich aus, und die Abhilfen haben nichts miteinander zu tun.
/// Dieser Fall beantwortet die erste, damit die zweite nicht geraten wird.
#[test]
fn das_paar_wird_gebildet() {
    let pool = all_candidates();
    let offen: Vec<usize> = (0..pool.len()).collect();
    let paare = paar_zuege(&pool, &offen);

    let unseres: Vec<(&'static str, &'static str)> = paare
        .iter()
        .filter(|(v, n)| {
            pool[offen[*v]].origin == Origin::NextStrikeWindow
                && pool[offen[*n]].origin == Origin::BossCastWindow
        })
        .map(|(v, n)| {
            (
                SPELLS[pool[offen[*v]].spell].id,
                SPELLS[pool[offen[*n]].spell].id,
            )
        })
        .collect();

    assert!(
        !unseres.is_empty(),
        "kein Paar aus einer NextStrikeWindow-Zeile und einem Unterbrecher; \
         gebildet wurden {} Paare insgesamt",
        paare.len()
    );
    assert!(
        unseres.iter().any(|(v, n)| *v == "pin" && *n == "quench"),
        "das Paar (pin, quench) fehlt; gebildet wurden {unseres:?}"
    );
}

/// # Was hier gemessen wird und was der Fall NICHT behauptet
///
/// Mit dem **vollen** Kandidatenpool wählt die Suche die PIN-Zeile bei keinem
/// Gewicht — gemessen über 0, 1, 5, 20, 100. Das ist ein Befund über das
/// Suchverfahren, nicht über den Wert, und er hat einen benennbaren Grund:
///
/// **Die kleinste Einheit, die etwas wert ist, sind hier DREI Zeilen.** PIN
/// kostet drei Kontroll-Punkte, die im Kampf erst erzeugt werden müssen. Ohne
/// einen Erzeuger (`bind`, `tether`, `clasp`) ist PIN nie wirkbar, und die
/// Zeile bleibt wirkungslos, egal wo sie steht.
///
/// ```text
/// bind          erzeugt Kontroll-Punkte      allein: ein mittelmäßiger Füller
/// pin           macht den Schlag angreifbar  allein: nicht wirkbar
/// quench        unterbricht ihn              allein: findet nie etwas Laufendes
/// ```
///
/// Das Paar-Verfahren aus #558 sieht Paare, keine Tripel — `das_paar_wird_gebildet`
/// belegt, dass (pin, quench) gebildet WIRD; es gewinnt nur nichts, solange der
/// Erzeuger fehlt.
///
/// Deshalb misst dieser Fall mit einem **kleinen Pool**: Dort kommt der
/// Erzeuger als gewöhnlicher Einzelzug hinein, und die Frage „kann die Suche
/// die Zeile überhaupt wählen" wird von der Frage „findet Greedy den Weg
/// dorthin" getrennt. Beide sind offen; nur die zweite ist hier zugesichert.
#[test]
fn die_suche_kann_die_zeile_waehlen() {
    fn suche_mit(pool: Vec<rota_sim::rotation_search::Candidate>, gewicht: f64) -> Vec<String> {
        let boss = boss_by_id(TOEDLICH).expect("test-lethal-strike fehlt im Datensatz");
        let config = BalanceConfig {
            boss,
            // Ausdruecklich gesetzt: Die Voreinstellung ist `true` und beendet
            // den Kampf beim Tod des Bosses — der Schlag bei 12000 kaeme dann
            // je nach Rotation gar nicht vor.
            stop_on_kill: false,
            prevented_weight: gewicht,
            ..BalanceConfig::default()
        };
        greedy(&config, &pool, 10)
            .chosen
            .iter()
            .map(|k| format!("{} if {}", SPELLS[k.spell].id, k.condition))
            .collect()
    }

    fn zeilen_von(ids: &[&str]) -> Vec<rota_sim::rotation_search::Candidate> {
        ids.iter()
            .map(|id| {
                SPELLS
                    .iter()
                    .position(|s| s.id == *id)
                    .unwrap_or_else(|| panic!("{id} fehlt im Zauberdatensatz"))
            })
            .flat_map(candidates_for)
            .collect()
    }

    // Der kleine Pool: Erzeuger, PIN, Unterbrecher, Füller. Mehr braucht die
    // Aufgabe nicht, und weniger geht nicht.
    let klein = zeilen_von(&["bind", "pin", "quench", SONDE]);
    let gefunden = suche_mit(klein, 100.0);

    assert!(
        gefunden.iter().any(|z| z.starts_with("pin if")),
        "im kleinen Pool muss die Suche die PIN-Zeile waehlen koennen — sonst \
         ist der Wert gebaut und fuer die Suche unerreichbar, also formal \
         erfuellt und sachlich leer. Gefunden: {gefunden:?}"
    );

    // **Die Kontrollzeile, und sie stand zuerst falsch da.**
    //
    // Erster Entwurf: „bei Gewicht 0 darf PIN nicht gewählt werden, er trägt
    // `baseDamage: 0` und kostet einen GCD". Gemessen wurde er auch dort —
    // und das ist richtig so, nicht kaputt: Ein toter Spieler wirkt nicht
    // weiter. Der Schlag bei 12000 beendet einen 60-Sekunden-Kampf nach zwölf,
    // also zahlt sich PIN schon in **Schaden** aus, ganz ohne Gewicht auf das
    // Verhinderte.
    //
    // Die Annahme war der Fehler, nicht die Messung. Die Kontrolle gehört
    // deshalb an den **Zwilling**: `test-survivable-strike` unterscheidet sich
    // in genau einer Zahl (4600 statt 18400), der Spieler überlebt dort
    // ohnehin, und der Kampf wird nicht abgeschnitten. Dort ist PIN reiner
    // Verlust — wird er auch da gewählt, wählt die Suche ihn unabhängig
    // davon, ob er etwas bewirkt, und der Fall oben belegt nichts.
    fn suche_gegen_zwilling(gewicht: f64) -> Vec<String> {
        let boss = boss_by_id(UEBERLEBBAR).expect("test-survivable-strike fehlt im Datensatz");
        let config = BalanceConfig {
            boss,
            stop_on_kill: false,
            prevented_weight: gewicht,
            ..BalanceConfig::default()
        };
        let pool: Vec<rota_sim::rotation_search::Candidate> = ["bind", "pin", "quench", SONDE]
            .iter()
            .map(|id| {
                SPELLS
                    .iter()
                    .position(|s| s.id == *id)
                    .unwrap_or_else(|| panic!("{id} fehlt im Zauberdatensatz"))
            })
            .flat_map(candidates_for)
            .collect();
        greedy(&config, &pool, 10)
            .chosen
            .iter()
            .map(|k| format!("{} if {}", SPELLS[k.spell].id, k.condition))
            .collect()
    }

    let harmlos = suche_gegen_zwilling(0.0);
    assert!(
        !harmlos.iter().any(|z| z.starts_with("pin if")),
        "am ueberlebbaren Zwilling und ohne Gewicht bringt PIN nichts — wird \
         er dort trotzdem gewaehlt, waehlt die Suche ihn unabhaengig von \
         seiner Wirkung. Gefunden: {harmlos:?}"
    );
}
