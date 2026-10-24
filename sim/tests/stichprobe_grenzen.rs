//! Die Grenzen der gestaffelten Stichprobe (Issue #915, AC3).
//!
//! # Warum diese Datei neben den Prüffällen im Modul steht
//!
//! Die acht Prüffälle in `src/stichprobe.rs` fahren zwei Fehlerzahlen: **null**
//! Fehler und **jeden zweiten** Versuch. Dazwischen liegt der Fall, für den die
//! Stichprobe gebaut ist — ein Fehler, eine Handvoll Fehler, eine Rate knapp
//! über der Schwelle —, und er war ungeprüft.
//!
//! Genau dort rechnete `hoechstens_k_fehler` falsch. Der Startterm
//! `p.powi(n as i32)` unterläuft bei kleiner Rate zu `0.0`, und danach bleibt
//! die ganze Summe null. Die gespiegelte Obergrenze in `gestaffelt` ruft die
//! Funktion genau so auf (die Erfolgsrate der Spiegelung ist die Fehlerrate,
//! also klein). Ergebnis: Nach einem einzigen Fehler meldete die Folge die
//! Schwelle als sicher **unter**schritten, obwohl die Rate weit darüber lag.
//! Und `n as i32` verengt: Ab `n = 2^31` kippt das Vorzeichen des Exponenten.
//!
//! # Woher die Sollwerte kommen
//!
//! Die geschlossenen Formen stehen im jeweiligen Prüffall selbst.
//!
//! Drei Werte stehen als Zahl da: die beiden Befunde der gestaffelten Läufe und
//! die Untergrenze bei 1200 Fehlern in 2^20 Versuchen. Sie sind **unabhängig vom
//! Code unter Test** gerechnet, mit 60 Dezimalstellen (mpmath 1.3.0), und zwar
//! über drei Wege, die auf mindestens 54 Stellen übereinstimmen:
//!
//! ```text
//! 1  direkte Summe der Binomialterme über die kürzere Flanke (binomial())
//! 2  dieselbe Summe über loggamma statt binomial()
//! 3  regularisierte unvollständige Betafunktion I_p(n-k, k+1)
//! ```
//!
//! Die Schranke selbst ist dabei genau so definiert wie in `untergrenze`: das
//! kleinste `p`, für das „höchstens k Fehler" mindestens die Wahrscheinlichkeit
//! `a` hat, gesucht über 200 Halbierungen.

use rota_sim::stichprobe::{
    budget_am_haltepunkt, gestaffelt, hoechstens_k_fehler, untergrenze, Befund,
};

/// Die Schwelle aus dem Issue: Mechanikerfolg mindestens 99,9 Prozent.
const SCHWELLE: f64 = 0.999;

/// Die Irrtumswahrscheinlichkeit aus dem Issue: einseitig 99 Prozent.
const ALPHA: f64 = 0.01;

/// Das Irrtumsbudget, das `gestaffelt` an dem Haltepunkt mit `n` Versuchen
/// vergibt. Die Haltepunkte sind 1, 2, 4, …, also gehört zu `n = 2^m` der
/// Haltepunkt `k = m + 1`.
fn budget_bei(n: u64) -> f64 {
    assert!(
        n.is_power_of_two(),
        "nur Haltepunkte sind Zweierpotenzen: {n}"
    );
    budget_am_haltepunkt(n.trailing_zeros() + 1, ALPHA)
}

/// **Der Fall aus dem Auftrag: EIN Fehler, bei Versuch 600.**
///
/// 16 383 von 16 384 Versuchen gelingen, die Rate liegt also weit über 99,9
/// Prozent. Eine Folge, die das als **verfehlt** meldet, hat nicht vorsichtig
/// gerechnet, sondern falsch: Ein „verfehlt" ist eine Aussage mit derselben
/// Irrtumsgrenze wie ein „erfüllt".
///
/// Sollwert aus der Referenz (siehe Dateikopf), an den beiden letzten
/// Haltepunkten:
///
/// ```text
/// n =  8192   1 Fehler   Untergrenze 0,998467957   noch unter 0,999
/// n = 16384   1 Fehler   Untergrenze 0,999224919   darüber -> Erfuellt
/// ```
///
/// Der Abstand zur Schwelle beträgt an beiden Stellen mehr als 2·10⁻⁴; die
/// Zahl hängt also nicht an der letzten Stelle einer Gleitkommarechnung.
#[test]
fn ein_einzelner_fehler_bei_versuch_600_ist_kein_verfehlt() {
    let befund = gestaffelt(SCHWELLE, ALPHA, 1 << 14, |i| i != 600);

    assert_ne!(
        befund,
        Befund::Verfehlt { versuche: 1024 },
        "ein einziger Fehler in 1024 Versuchen widerlegt 99,9 Prozent nicht"
    );
    assert!(
        !matches!(befund, Befund::Verfehlt { .. }),
        "die Rate liegt bei 16383 von 16384 Versuchen ÜBER der Schwelle, \
         gemeldet wurde: {befund:?}"
    );
    assert_eq!(
        befund,
        Befund::Erfuellt { versuche: 16384 },
        "Sollwert aus der Referenz: erfüllt am Haltepunkt 16384"
    );
    println!("ein Fehler bei Versuch 600: {befund:?}");
}

/// **Eine Rate ÜBER der Schwelle gilt nie als verfehlt.**
///
/// Jeder 2000. Versuch scheitert, die Rate ist 99,95 Prozent — die halbe
/// Fehlerrate dessen, was die Schwelle erlaubt.
///
/// ```text
/// n = 32768   16 Fehler   Untergrenze 0,998827540   noch unter 0,999
/// n = 65536   32 Fehler   Untergrenze 0,999068514   darüber -> Erfuellt
/// ```
#[test]
fn eine_rate_ueber_der_schwelle_gilt_nie_als_verfehlt() {
    let befund = gestaffelt(SCHWELLE, ALPHA, 1 << 16, |i| i % 2000 != 1999);

    assert!(
        !matches!(befund, Befund::Verfehlt { .. }),
        "99,95 Prozent liegen über 99,9 Prozent, gemeldet wurde: {befund:?}"
    );
    assert_eq!(
        befund,
        Befund::Erfuellt { versuche: 65536 },
        "Sollwert aus der Referenz: erfüllt am Haltepunkt 65536"
    );
    println!("Rate 99,95 Prozent: {befund:?}");
}

/// **Die Obergrenze nach genau einem Fehler, gegen ihre geschlossene Form.**
///
/// Bei `n - 1` Fehlern in `n` Versuchen ist „höchstens `n - 1` Fehler" das
/// Gegenteil von „alle scheitern": `1 - (1-p)^n`. Die Schranke `untergrenze`
/// sucht das `p`, an dem das gleich `a` wird, also
///
/// ```text
/// 1 - (1-p)^n = a      ->      1 - p = (1-a)^(1/n)
/// ```
///
/// Genau diesen Aufruf macht `gestaffelt` für die Obergrenze, wenn EIN Fehler
/// aufgetreten ist (gespiegelt: `n - fehler` Erfolge als Fehler gezählt).
///
/// Geprüft wird an jedem Haltepunkt von 8 bis 2^20, mit dem Budget, das
/// `gestaffelt` dort vergibt. **Alle Abweichungen werden gesammelt**, nicht
/// nur die erste — die Liste zeigt, ab welchem `n` die Rechnung kippt.
#[test]
fn die_obergrenze_nach_einem_fehler_trifft_die_geschlossene_form() {
    let mut abweichungen = Vec::new();
    let mut geprueft = 0;
    for m in 3..=20u32 {
        let n = 1u64 << m;
        let a = budget_bei(n);
        let oben = 1.0 - untergrenze(n, n - 1, a);
        let formel = (1.0 - a).powf(1.0 / n as f64);
        geprueft += 1;
        if (oben - formel).abs() > 1e-12 {
            abweichungen.push(format!("n = {n}: {oben:.12} statt {formel:.12}"));
        }
    }
    assert_eq!(geprueft, 18, "Kontrollzeile: 18 Haltepunkte von 8 bis 2^20");
    assert!(
        abweichungen.is_empty(),
        "die Obergrenze nach einem Fehler weicht an {} von {geprueft} \
         Haltepunkten ab:\n  {}",
        abweichungen.len(),
        abweichungen.join("\n  ")
    );
    println!("Obergrenze nach einem Fehler: {geprueft} Haltepunkte geprüft");
}

/// **Die Untergrenze liegt nie über dem Punktschätzer.**
///
/// 1200 Fehler in 2^20 Versuchen: Der Anteil der Erfolge ist 0,998855591. Eine
/// **untere** Schranke darüber wäre anti-konservativ — sie behauptete mehr
/// Erfolg, als beobachtet wurde, und machte aus einer verfehlten Schwelle ein
/// falsches „erfüllt".
///
/// `a` ist das Budget, das `gestaffelt` am Haltepunkt `n = 2^20` vergibt
/// (`k = 21`, also `0,01 / 462`). Sollwert aus der Referenz:
///
/// ```text
/// untergrenze(2^20, 1200, 0,01/462) = 0,998714553257086
/// ```
#[test]
fn die_untergrenze_liegt_nie_ueber_dem_punktschaetzer() {
    let n = 1u64 << 20;
    let fehler = 1200u64;
    let a = budget_bei(n);
    let unten = untergrenze(n, fehler, a);
    let punkt = 1.0 - fehler as f64 / n as f64;

    assert!(
        unten < punkt,
        "die UNTERgrenze {unten:.9} liegt über dem beobachteten Anteil {punkt:.9}"
    );
    let referenz = 0.998_714_553_257_086_3;
    assert!(
        (unten - referenz).abs() < 1e-9,
        "Untergrenze {unten:.12}, Referenz {referenz:.12}"
    );
    println!("untergrenze(2^20, 1200) = {unten:.12}, Punktschätzer {punkt:.12}");
}

/// **Ohne Fehler, jenseits von `i32`: die Verengung `n as i32`.**
///
/// Bei null Fehlern ist die Verteilungsfunktion `p^n` und die Schranke
/// geschlossen bekannt, `a^(1/n)`. Ab `n = 2^31` passt `n` nicht mehr in ein
/// `i32`: `2^31` wird zu `i32::MIN`, `2^32` und `2^40` werden zu null.
///
/// `n = 2^31 - 1` ist die **Gegenprobe**, und zwar nur für die Schranke: Es
/// ist das größte `n`, das die Verengung heil übersteht. Fällt die Schranke
/// dort, liegt der Fehler woanders als in der Verengung. Für die
/// Verteilungsfunktion selbst taugt dieses `n` NICHT als Gegenprobe: `powi`
/// quadriert wiederholt, und jede Quadrierung eines Werts nahe 1 verdoppelt den
/// relativen Fehler. Gemessen mit Rust 1.98.1 auf dem Bau-Dienst, an
/// `p = 1 - 10⁻¹⁰` gegen `powf`: 1,7·10⁻⁹ bei `n = 2^31 - 1`, 8,4·10⁻¹³ bei
/// `n = 2^20` — beides ohne Verengung, und das Erste liegt über der Toleranz
/// unten. Die Schranke spürt es nicht: Vor der Behebung traf
/// `untergrenze(2^31 - 1, 0, a)` den geschlossenen Wert auf 10⁻¹⁴ genau, die
/// drei größeren `n` lieferten 0.
///
/// So viele Versuche sind kein Gedankenspiel: Die Stichprobe hat keine feste
/// Obergrenze (AC3), und bei 350 000 Kämpfen je Sekunde sind 2^31 Versuche
/// rund 100 Minuten.
#[test]
fn die_schranke_ohne_fehler_haelt_jenseits_von_i32() {
    let mut abweichungen = Vec::new();

    // Die Verteilungsfunktion selbst, an einem festen p.
    let p = 1.0 - 1e-10;
    for n in [1u64 << 31, 1 << 32, 1 << 40] {
        let wert = hoechstens_k_fehler(n, 0, p);
        let formel = p.powf(n as f64);
        if (wert - formel).abs() > 1e-9 * formel {
            abweichungen.push(format!(
                "hoechstens_k_fehler(n = {n}, k = 0): {wert:e} statt {formel:e}"
            ));
        }
    }

    // Die Schranke darüber, mit der Gegenprobe knapp unter 2^31.
    for n in [(1u64 << 31) - 1, 1 << 31, 1 << 32, 1 << 40] {
        let unten = untergrenze(n, 0, ALPHA);
        let schranke = ALPHA.powf(1.0 / n as f64);
        if (unten - schranke).abs() > 1e-14 {
            abweichungen.push(format!(
                "untergrenze(n = {n}, k = 0): {unten:.17} statt {schranke:.17}"
            ));
        }
    }

    assert!(
        abweichungen.is_empty(),
        "ohne Fehler, große n:\n  {}",
        abweichungen.join("\n  ")
    );
    println!("ohne Fehler: n bis 2^40 trifft die geschlossene Form");
}

/// **Kontrolle, schon vor der Behebung grün: der Preis der Staffelung.**
///
/// Ohne einen einzigen Fehler ist die Zahl genau bestimmt. Ungestaffelt
/// genügten 4603 fehlerfreie Versuche; gestaffelt ist die Schwelle erst am
/// Haltepunkt 16 384 sicher — der 15., an dem nur noch `0,01 / 240` zur
/// Verfügung steht.
///
/// Der Fall im Modul prüft nur `versuche >= 4603` und „die Staffelung kostet
/// etwas". Beide Zusicherungen hielten auch, wenn jeder Haltepunkt das volle
/// `alpha` bekäme (`let a = alpha;`): Dann käme `Erfuellt` schon bei 8192 —
/// das verbotene „je Blick neu vergeben", und die Aussage wäre beim häufigen
/// Hinsehen nicht mehr gültig. Diese Zeile nagelt den Haltepunkt fest.
#[test]
fn der_preis_der_staffelung_ist_genau_ein_haltepunkt() {
    let befund = gestaffelt(SCHWELLE, ALPHA, 1 << 17, |_| true);
    assert_eq!(
        befund,
        Befund::Erfuellt { versuche: 16384 },
        "ohne Fehler: erfüllt am Haltepunkt 16384, nicht früher"
    );
    println!("ohne Fehler: {befund:?}; ungestaffelt genügten 4603");
}
