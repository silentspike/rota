//! Der kritische Treffer analytisch — **neben** dem Kampf (Issue #908, AC4).
//!
//! # Warum neben und nicht im Kampf
//!
//! Der Kampf würfelt weiter. Er muss es: GAME-DESIGN 18.4 setzt die
//! Ziehungszähler **je Takt** in die Zustands-Prüfsumme, und `fight.rs` tut das
//! auch (`mix_named_int(h, "randomDraws", …)`). Ein Kampf, der statt zu ziehen
//! den Erwartungswert führt, hätte andere Zähler, eine andere Prüfsumme und
//! bräche jede Paritäts-Aufzeichnung. AC4 sagt dasselbe kürzer: „nur dort, **wo
//! er den Kontrollfluss nicht berührt**".
//!
//! Was hier ersetzt wird, ist deshalb nicht der Kampf, sondern das **Mitteln
//! über viele Durchgänge**, mit dem eine Balance-Zahl entsteht.
//!
//! # Die Rechnung, und warum sie exakt ist
//!
//! Nicht den Erwartungswert in den Krit-Faktor setzen — das ist der
//! naheliegende und falsche Weg, weil [`crate::compute_damage`] **je Treffer**
//! rundet und Runden nicht mit Mitteln vertauscht
//! (`sim/tests/analytischer_krit_grenzen.rs` misst es: 11 von 18 Kombinationen
//! laufen auseinander).
//!
//! Stattdessen wird über die **Ausgänge** marginalisiert. Für einen einzelnen
//! Treffer mit Krit-Wahrscheinlichkeit `p`:
//!
//! ```text
//! E[Treffer] = p · runden(basis · krit) + (1 − p) · runden(basis)
//! ```
//!
//! Über einen ganzen Kampf mit vielen Treffern greift die **Linearität des
//! Erwartungswerts** — und die gilt unabhängig davon, wie die einzelnen Würfe
//! zusammenhängen:
//!
//! ```text
//! E[Σ Treffer] = Σ E[Treffer] = p · (alle kriten) + (1 − p) · (keiner kritet)
//! ```
//!
//! Damit genügen **zwei** deterministische Läufe statt fünfzig bis zweihundert
//! gewürfelter. Und weil beide Läufe wurffrei sind, reduziert die Ableitung aus
//! [`crate::durchgaenge`] jeden von ihnen auf **einen** Durchgang: aus 2 × 200
//! Kämpfen werden 2.
//!
//! # Die Voraussetzung, ohne die das nicht gilt
//!
//! Die Trefferfolge muss in beiden Läufen dieselbe sein. Sobald am Krit-Ergebnis
//! eine **Verzweigung** hängt — ein Zauber, der nach einem Krit anders weitergeht
//! — laufen die beiden Läufe auseinander, und die Summe der Erwartungswerte ist
//! nicht mehr der Erwartungswert der Summe. Genau das prüft
//! [`crate::effects::passes::AnalyticCritPass`] je Knoten, und genau deshalb
//! wird hier gefragt statt angenommen.

use crate::apl::Apl;
use crate::balance::{run, BalanceConfig};
use crate::data::EFFECT_NODES;
use crate::effects::passes::AnalyticCritPass;
use crate::effects::registry::{EffectAnalysisPass, Verdict};
use crate::effects::EffectNode;

/// Hängt am Krit-Ergebnis irgendwo eine Verzweigung?
///
/// **Abgeleitet, nicht gepflegt** (AC4): Die Antwort entsteht Knoten für Knoten
/// aus der Wirkungsbeschreibung. Eine Liste daneben wäre beim nächsten Effekt
/// still veraltet.
///
/// Gefragt wird nur bei Knoten, die überhaupt würfeln — ein deterministischer
/// Knoten kann den Krit-Kontrollfluss nicht berühren, auch wenn er verzweigt.
#[must_use]
pub fn krit_beruehrt_den_kontrollfluss(knoten: &[EffectNode]) -> bool {
    knoten
        .iter()
        .filter(|n| !n.randomness.ist_deterministisch())
        .any(|n| !matches!(AnalyticCritPass.analyze(n).verdict, Verdict::Permits))
}

/// Der Schaden eines Laufs, in dem **kein** Treffer kritet.
fn ohne_krit(config: &BalanceConfig, apl: &Apl) -> f64 {
    let mut worn = config.worn;
    worn.purity = 0;
    run(&BalanceConfig { worn, ..*config }, apl).damage.mean
}

/// Der Schaden eines Laufs, in dem **jeder** Treffer kritet.
///
/// `purity = 1000` heißt `crit_chance() == 1.0`, und
/// [`crate::rng::RandomStream::chance`] antwortet dann `true` **ohne zu
/// ziehen**. Der Lauf ist damit ebenso deterministisch wie der ohne Krit — und
/// `crit_factor()` hängt nicht an PURITY, sondern an YIELD, bleibt also
/// unverändert.
fn mit_krit(config: &BalanceConfig, apl: &Apl) -> f64 {
    let mut worn = config.worn;
    worn.purity = 1000;
    run(&BalanceConfig { worn, ..*config }, apl).damage.mean
}

/// Der Erwartungswert des Schadens — exakt, ohne Monte-Carlo.
///
/// Gibt `None` zurück, wenn die Voraussetzung nicht gilt. **Nicht** einen
/// Näherungswert: Ein Ergebnis, das manchmal exakt und manchmal ungefähr ist,
/// wäre schlimmer als keines, weil man ihm die Unterscheidung nicht ansieht.
#[must_use]
pub fn erwarteter_schaden(config: &BalanceConfig, apl: &Apl) -> Option<f64> {
    // **Der Kontrollfluss hängt nicht nur an den Knoten, sondern auch am LAUF.**
    //
    // Mit `stop_on_kill` endet der Durchgang beim Sieg. Ein Lauf, in dem jeder
    // Treffer kritet, tötet den Boss früher — und schneidet damit seinen eigenen
    // Schaden ab. Gemessen, `step1-good` gegen den Vorgabe-Boss:
    //
    // ```text
    // keiner kritet   4240
    // alle kriten     4230   <- WENIGER, weil der Kampf eher endet
    // ```
    //
    // Die beiden Läufe haben dann nicht dieselbe Trefferfolge, und die
    // Linearität des Erwartungswerts trägt nicht mehr. Das ist derselbe
    // Kontrollfluss-Vorbehalt wie bei den Knoten, nur eine Ebene höher — und
    // eine knotenweise Analyse kann ihn prinzipiell nicht sehen, weil er nicht
    // in der Wirkungsbeschreibung steht, sondern in der Konfiguration.
    if config.stop_on_kill {
        return None;
    }
    if krit_beruehrt_den_kontrollfluss(EFFECT_NODES) {
        return None;
    }
    let p = config.worn.crit_chance();
    Some(p * mit_krit(config, apl) + (1.0 - p) * ohne_krit(config, apl))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apl::{parse, AplName};

    const GOOD: &str = include_str!("../apl/step1-good.apl");

    fn gute_liste() -> Apl {
        parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
    }

    fn konfig(purity: u32) -> BalanceConfig {
        let mut worn = crate::gear::WornValues::none();
        worn.purity = purity;
        BalanceConfig {
            iterations: 200,
            base_seed: 17,
            threads: 1,
            worn,
            // **Ohne das trägt die Rechnung nicht** — und das ist gemessen, nicht
            // vorsichtshalber gesetzt: Mit `stop_on_kill` ergab der Lauf mit Krit
            // 4230 gegen 4240 ohne, weil der frühere Sieg den Schaden abschneidet.
            stop_on_kill: false,
            ..BalanceConfig::default()
        }
    }

    /// Die Konfiguration, die die Voraussetzung **verletzt** — und erkannt wird.
    #[test]
    fn mit_abbruch_beim_sieg_gibt_es_keinen_erwartungswert() {
        let apl = gute_liste();
        let mit_abbruch = BalanceConfig {
            stop_on_kill: true,
            ..konfig(500)
        };
        assert!(
            erwarteter_schaden(&mit_abbruch, &apl).is_none(),
            "mit Abbruch beim Sieg hängt die Kampfdauer am Krit — dann darf hier \
             kein Wert herauskommen, auch kein ungefährer"
        );
        assert!(
            erwarteter_schaden(&konfig(500), &apl).is_some(),
            "Trägerzusicherung: ohne Abbruch liefert dieselbe Konfiguration einen \
             Wert — sonst prüfte der Fall nur, dass die Funktion immer None sagt"
        );
    }

    /// Die Voraussetzung wird **gefragt**, nicht angenommen.
    #[test]
    fn die_voraussetzung_wird_aus_der_beschreibung_abgeleitet() {
        // Heute trägt kein würfelnder Knoten eine Verzweigung — sonst wäre der
        // analytische Weg gar nicht anwendbar und alle Fälle unten sinnlos.
        assert!(
            !krit_beruehrt_den_kontrollfluss(EFFECT_NODES),
            "Trägerzusicherung: die Voraussetzung gilt im heutigen Bestand"
        );

        // Und die Gegenprobe: Ein konstruierter Knoten, der würfelt UND
        // verzweigt, muss sie kippen. Ohne ihn wüsste niemand, ob die Funktion
        // überhaupt „nein" sagen kann.
        let mut verzweigend = EFFECT_NODES
            .iter()
            .find(|n| !n.randomness.ist_deterministisch())
            .copied()
            .expect("es gibt würfelnde Knoten");
        verzweigend.op = crate::effects::EffectOp::ApplyBuff;
        assert!(
            krit_beruehrt_den_kontrollfluss(&[verzweigend]),
            "ein würfelnder Knoten, der Aktionen verändert, MUSS die \
             Voraussetzung kippen"
        );
    }

    /// **Der Kern: der analytische Wert liegt exakt auf der Gewichtung.**
    ///
    /// Verglichen wird nicht gegen eine gespeicherte Zahl, sondern gegen die
    /// beiden Randläufe, aus denen er entsteht — und gegen die Bedingung, dass
    /// er echt dazwischen liegt. Läge er außerhalb, wäre die Gewichtung falsch
    /// herum oder ein Randlauf nicht das, was sein Name sagt.
    #[test]
    fn der_erwartungswert_liegt_exakt_auf_der_gewichtung() {
        let apl = gute_liste();
        let config = konfig(500);

        let a = ohne_krit(&config, &apl);
        let b = mit_krit(&config, &apl);
        let e = erwarteter_schaden(&config, &apl).expect("die Voraussetzung gilt");

        assert!(
            b > a,
            "Trägerzusicherung: mit Krit muss mehr Schaden entstehen ({b} gegen {a}) \
             — sonst unterscheiden sich die beiden Randläufe gar nicht"
        );
        assert!(
            e > a && e < b,
            "der Erwartungswert muss zwischen den Rändern liegen: {a} < {e} < {b}"
        );
        assert_eq!(
            e.to_bits(),
            (0.5 * b + 0.5 * a).to_bits(),
            "bei halber Chance ist es genau die Mitte"
        );
    }

    /// **Der einzige unabhängige Nachweis: gegen wirklich gewürfelte Läufe.**
    ///
    /// Die Fälle darüber prüfen die Formel gegen ihre eigenen Bestandteile —
    /// `e == 0.5·b + 0.5·a` kann gar nicht scheitern, solange `e` so gebildet
    /// wird. Erst der Vergleich mit Monte-Carlo kann zeigen, dass die Gewichtung
    /// die *richtige* ist.
    ///
    /// **Bitgleich ist dieser Vergleich nicht, und das ist keine Schwäche der
    /// Rechnung.** Der analytische Wert ist der exakte Erwartungswert; ein
    /// Monte-Carlo-Lauf ist eine Stichprobe daraus und streut um ihn. Bitgleich
    /// wären sie erst bei unendlich vielen Durchgängen. Geprüft wird deshalb,
    /// dass die Abweichung im Rahmen des **Standardfehlers** bleibt — der Größe,
    /// die der Läufer selbst mitliefert.
    #[test]
    fn der_analytische_wert_trifft_das_gewuerfelte_mittel_im_fehlerrahmen() {
        let apl = gute_liste();
        // **Nicht 500.** Bei halber Chance ist die Gewichtung symmetrisch, und
        // ein vertauschtes `p`/`1−p` käme auf denselben Wert — der Fall könnte
        // den naheliegendsten Fehler nicht fangen. Mit 250 ist er unsymmetrisch.
        let config = BalanceConfig {
            iterations: 2000,
            ..konfig(250)
        };

        let gewuerfelt = run(&config, &apl).damage;
        let analytisch = erwarteter_schaden(&config, &apl).expect("die Voraussetzung gilt");

        assert!(
            gewuerfelt.stddev > 0.0,
            "Trägerzusicherung: die gewürfelten Durchgänge STREUEN wirklich \
             (stddev {}) — ohne Streuung verglichen wir zwei deterministische \
             Läufe und der Fall wäre wertlos",
            gewuerfelt.stddev
        );

        let abstand = (gewuerfelt.mean - analytisch).abs();
        assert!(
            abstand < 4.0 * gewuerfelt.stderr,
            "der analytische Wert liegt {abstand:.2} neben dem gewürfelten Mittel \
             ({} gegen {analytisch}), erlaubt sind vier Standardfehler ({:.2}) — \
             das ist keine Streuung mehr, sondern ein Fehler in der Gewichtung",
            gewuerfelt.mean,
            4.0 * gewuerfelt.stderr
        );

        println!(
            "gewürfelt {:.2} ± {:.2} (stderr), analytisch {analytisch:.2}, \
             Abstand {abstand:.2}",
            gewuerfelt.mean, gewuerfelt.stderr
        );
    }

    /// An den Rändern muss der analytische Wert die Randläufe **treffen**.
    #[test]
    fn bei_null_und_voll_trifft_er_die_raender_bitgleich() {
        let apl = gute_liste();

        let ohne = konfig(0);
        let e_ohne = erwarteter_schaden(&ohne, &apl).expect("gilt");
        assert_eq!(
            e_ohne.to_bits(),
            ohne_krit(&ohne, &apl).to_bits(),
            "bei p = 0 ist der Erwartungswert der Lauf ohne Krit"
        );

        let voll = konfig(1000);
        let e_voll = erwarteter_schaden(&voll, &apl).expect("gilt");
        assert_eq!(
            e_voll.to_bits(),
            mit_krit(&voll, &apl).to_bits(),
            "bei p = 1 ist er der Lauf mit Krit"
        );
    }
}
