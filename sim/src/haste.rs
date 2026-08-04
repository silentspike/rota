//! Tempo (CATALYSIS) wirkt auf **Zeiten**, nicht auf den Schaden
//! (GAME-DESIGN 6.7, Issue #177).
//!
//! Das Gegenstueck zu `src/core/haste.ts` — Zeile fuer Zeile dieselbe Rechnung.
//!
//! > Diablo 4 formuliert es genau: *„Attack speed does not enter the damage
//! > formula, but rather it multiplies how often you get to use it."*
//!
//! ```text
//! neue_zeit = auf_takt_aufrunden(basis_zeit × 1000 / (1000 + tempo_promille))
//! ```
//!
//! # Warum Promille und nicht `f64`
//!
//! Die Determinismus-Regel aus 18.4 verbietet Fliesskomma-**Zeit**. Ein
//! Tempo-Faktor von 1,17 auf eine Abklingzeit von 8 s ergibt 6,837606... s —
//! eine Zahl, die zwei Sprachen nicht garantiert gleich runden. Ganzzahlig in
//! Promille gerechnet und einmal auf den Logik-Takt gerastert, ist das Ergebnis
//! auf beiden Seiten dieselbe ganze Zahl.
//!
//! Der Zwischenschritt laeuft in `u64`, damit `basis × 1000` auch bei langen
//! Zeiten nicht ueberlaeuft: Die laengste Abklingzeit aus 6.4 ist 180 s, also
//! 180 000 ms — mal 1000 sind 1,8e8, das passt noch in `u32`. Bei einer
//! Kampfdauer von 49 Tagen (dem Bereich von [`Millis`]) waere es 4,2e12 und
//! passte nicht mehr. `u64` kostet hier nichts und macht die Grenze
//! gegenstandslos.
//!
//! # Was hier NICHT steht: der Deckel
//!
//! 6.7 nennt fuer CATALYSIS einen Deckel von 50 %, abgeleitet aus
//! `1500 / (1 + t) = 1000` — „die Untergrenze aus 6.6".
//!
//! **6.6 enthaelt keine Untergrenze.** Nachgezaehlt am 2026-08-04: Die 1,0 s
//! stehen an genau einer Stelle im Design, naemlich in 6.4, und dort als
//! „per **Talent** senkbar bis **ca.** 1,0 s". Aus einem Ungefaehr fuer einen
//! anderen Mechanismus laesst sich kein Deckel ableiten, und ein
//! `const HASTE_CAP = 500` waere genau die Handabschrift, vor der die Auflage
//! zu Issue #177 warnt.
//!
//! Dieselbe Stelle traegt den zweiten offenen Punkt: 6.7 fuehrt die **globale
//! Wartezeit** als Wirkort, 6.2 schliesst sie ausdruecklich aus („Der GCD sinkt
//! ausschliesslich ueber den Talentbaum, nie ueber Ausruestung", Festlegung vom
//! 2026-07-29). Beides ist gemeldet; bis zur Entscheidung wirkt Tempo hier auf
//! **Abklingzeiten und DoT-Tick-Abstaende** — die Wirkorte, in denen 6.2 und
//! 6.7 uebereinstimmen.

use crate::time::{snap_up, Millis, TICK_MS};

/// Der Nenner der Promille-Rechnung. Kein Balance-Wert, sondern die Einheit.
const PER_MILLE: u64 = 1000;

/// Eine Zeit unter dem Einfluss von Tempo.
///
/// `permille` ist der **Zuwachs**: 0 heisst kein Tempo, 300 heisst „+30 %".
///
/// # Die drei Sonderfaelle, und warum jeder einzeln zaehlt
///
/// | Fall            | Ergebnis   | Grund                                          |
/// | --------------- | ---------- | ---------------------------------------------- |
/// | `base_ms == 0`  | `0`        | keine Zeit bleibt keine Zeit — nicht ein Takt   |
/// | `permille == 0` | `base_ms`  | **ohne Tempo aendert sich nichts** (AC7)       |
/// | Ergebnis `< 10` | `TICK_MS`  | ein Takt ist die kleinste Zeit, die es gibt     |
///
/// Der mittlere Fall ist nicht bloss eine Abkuerzung. Ohne ihn liefe jede
/// bestehende Zeit durch [`snap_up`], und **falls** eine davon nicht auf dem
/// Takt-Raster laege, verschoebe sie sich — ein Umbau, der ohne getragene
/// Ausruestung etwas veraendert. Genau das schliesst AC7 aus. Mit dem
/// Kurzschluss haengt die Zusicherung nicht daran, dass alle heutigen Werte
/// zufaellig gerastert sind.
///
/// ```
/// use rota_sim::haste::hasted_ms;
/// assert_eq!(hasted_ms(8000, 0), 8000);      // ohne Tempo unveraendert
/// assert_eq!(hasted_ms(8000, 200), 6670);    // 8000/1,2 = 6666,6 -> aufgerastert
/// assert_eq!(hasted_ms(0, 500), 0);          // keine Abklingzeit bleibt keine
/// ```
#[must_use]
pub fn hasted_ms(base_ms: Millis, permille: u32) -> Millis {
    if base_ms == 0 {
        return 0;
    }
    if permille == 0 {
        return base_ms;
    }
    let scaled = u64::from(base_ms) * PER_MILLE / (PER_MILLE + u64::from(permille));
    // Der Zwischenwert ist hoechstens `base_ms` und passt damit immer zurueck.
    let snapped = snap_up(scaled as Millis);
    if snapped < TICK_MS {
        TICK_MS
    } else {
        snapped
    }
}

/// Wie viele Ticks ein Effekt ueber Zeit bei diesem Abstand abgibt.
///
/// > **DoTs: die Dauer bleibt, die Ticks werden mehr** (6.7).
/// >
/// > `tick_zahl = dauer / tick_abstand` — ganzzahlig, Rest verfaellt.
///
/// **Kein anteiliger letzter Tick.** WoW zahlt ihn aus, wenn der Rest
/// mindestens 0,1 s betraegt; bei uns waere das eine Kommazahl im **Schaden**,
/// also genau dort, wo Spiel und Simulator bitgleich sein muessen.
///
/// Die Funktion rechnet die Zahl nicht ab, sondern **sagt sie vorher** — der
/// Kampf plant Tick fuer Tick ein und braucht sie nicht. Sie steht hier, weil
/// die Zusicherung aus AC4 (Dauer fest, Summe der Tick-Zeiten nie ueber der
/// Dauer) sonst nirgends nachpruefbar waere.
#[must_use]
pub fn tick_count(duration_ms: Millis, tick_interval_ms: Millis) -> u32 {
    if tick_interval_ms == 0 {
        return 0;
    }
    duration_ms / tick_interval_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ohne_tempo_bleibt_jede_zeit_auf_ihrem_wert() {
        // AC7 an der Wurzel: Was hier durchlaeuft, darf sich nicht bewegen.
        // Bewusst auch Werte, die NICHT auf dem Takt-Raster liegen — der
        // Kurzschluss muss auch die tragen, sonst haengt AC7 daran, dass alle
        // heutigen Daten zufaellig gerastert sind.
        for base in [0, 1, 7, 10, 1500, 3000, 8000, 40_501, 180_000] {
            assert_eq!(hasted_ms(base, 0), base, "{base} ms ohne Tempo");
        }
    }

    #[test]
    fn tempo_verkuerzt_und_rastert_nach_oben() {
        // 8000 / 1,2 = 6666,67 -> 6670. Aufgerundet, nie abgerundet: Ein
        // Simulator, der abrundet, waere um bis zu einen Takt je Zauber
        // schneller als das Spiel.
        assert_eq!(hasted_ms(8000, 200), 6670);
        assert_eq!(hasted_ms(3000, 500), 2000); // 3000/1,5 = 2000, glatt
        assert_eq!(hasted_ms(1500, 500), 1000); // der Wert aus der Deckel-Rechnung
    }

    #[test]
    fn keine_zeit_bleibt_keine_zeit() {
        // Ein Zauber ohne Abklingzeit bekommt durch Tempo keine.
        for permille in [1, 200, 1000, 100_000] {
            assert_eq!(hasted_ms(0, permille), 0, "+{permille} Promille auf 0 ms");
        }
    }

    #[test]
    fn unter_einem_takt_geht_es_nicht() {
        // Der Gegenfall zur Zeile darueber: Eine Zeit, die es GIBT, verschwindet
        // auch bei absurdem Tempo nicht.
        assert_eq!(hasted_ms(10, 1_000_000), TICK_MS);
        assert_eq!(hasted_ms(3000, 10_000_000), TICK_MS);
        // Und sie ist wirklich nicht null — ohne diese Zeile bliebe die Pruefung
        // gruen, wenn `TICK_MS` je auf 0 fiele.
        assert!(hasted_ms(3000, 10_000_000) > 0);
    }

    #[test]
    fn mehr_tempo_ist_nie_langsamer() {
        // Die Ordnung als Ungleichung ueber den ganzen Bereich, statt gegen
        // einzelne Zahlen: monoton fallend, nie steigend.
        for base in [500, 1500, 3000, 8000, 180_000] {
            let mut vorher = hasted_ms(base, 0);
            for permille in (10..2000).step_by(10) {
                let jetzt = hasted_ms(base, permille);
                assert!(
                    jetzt <= vorher,
                    "{base} ms: +{permille} Promille ergab {jetzt}, vorher {vorher}"
                );
                vorher = jetzt;
            }
        }
    }

    #[test]
    fn jedes_ergebnis_mit_tempo_liegt_auf_dem_takt_raster() {
        // Ohne diese Pruefung koennte eine Zeit zwischen zwei Takten enden —
        // das Spiel bemerkte sie erst beim naechsten und liefe auseinander.
        //
        // **`permille` beginnt bei 1, und das ist keine Nachlaessigkeit.**
        // Bei 0 widersprechen sich die beiden Zusicherungen dieser Datei, und
        // die erste Fassung dieser Pruefung ist genau daran gefallen:
        //
        //     hasted_ms(12345, 0)  ->  12345, nicht 12350
        //
        // "Ohne Tempo aendert sich nichts" (AC7) und "jedes Ergebnis liegt auf
        // dem Raster" koennen fuer eine ungerasterte Eingabe nicht beide
        // gelten. AC7 gewinnt: Der Umbau muss bruchfrei einfuehrbar sein, und
        // dazu gehoert, dass er auch einen krummen Wert unberuehrt laesst statt
        // ihn stillschweigend zu verschieben. Gerastert wird, was Tempo
        // anfasst — sonst nichts.
        for base in [500, 1500, 3000, 8000, 12_345, 180_000] {
            for permille in (1..3000).step_by(7) {
                let t = hasted_ms(base, permille);
                assert_eq!(t % TICK_MS, 0, "{base} ms bei +{permille} ergab {t}");
            }
        }
    }

    #[test]
    fn kein_ueberlauf_bei_langen_zeiten() {
        // `base × 1000` sprengt `u32` ab 4 294 968 ms. Der Zwischenschritt
        // laeuft deshalb in `u64`. Ohne ihn ergaebe das hier einen falschen,
        // aber plausibel aussehenden Wert.
        let sehr_lang = 4_300_000; // gut 71 Minuten
        assert_eq!(hasted_ms(sehr_lang, 1000), 2_150_000);
        assert!(hasted_ms(Millis::MAX, 1000) > 0);
    }

    #[test]
    fn die_ticks_teilen_die_dauer_ohne_rest_auszuzahlen() {
        // 12 s Dauer, 2 s Abstand -> 6 Ticks.
        assert_eq!(tick_count(12_000, 2000), 6);
        // Mit Tempo wird der Abstand kuerzer, die Zahl steigt, die Dauer bleibt.
        let schneller = hasted_ms(2000, 200); // 1670
        assert_eq!(schneller, 1670);
        assert_eq!(tick_count(12_000, schneller), 7);
        // **Der Rest verfaellt**: 7 × 1670 = 11 690 < 12 000. Kein achter Tick,
        // kein anteiliger.
        assert!(7 * schneller <= 12_000);
        assert!(8 * schneller > 12_000);
    }

    #[test]
    fn die_summe_der_tick_zeiten_ueberschreitet_die_dauer_nie() {
        // AC4 als Ungleichung ueber den ganzen Bereich — der Fall, der die
        // Zusicherung traegt. Ein anteiliger letzter Tick fiele hier auf.
        for dauer in [6000, 12_000, 18_000, 30_000] {
            for basis_abstand in [1000, 1500, 2000, 3000] {
                for permille in (0..2000).step_by(13) {
                    let abstand = hasted_ms(basis_abstand, permille);
                    let ticks = tick_count(dauer, abstand);
                    assert!(
                        ticks * abstand <= dauer,
                        "{dauer} ms, Abstand {abstand} ms: {ticks} Ticks passen nicht"
                    );
                    // Und der Gegenfall: Einer mehr passte nicht mehr — sonst
                    // waere die Pruefung mit `ticks = 0` immer erfuellt.
                    assert!((ticks + 1) * abstand > dauer);
                }
            }
        }
    }

    #[test]
    fn ein_abstand_von_null_ergibt_keine_ticks_statt_einer_division_durch_null() {
        assert_eq!(tick_count(12_000, 0), 0);
    }
}
