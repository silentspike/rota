//! Durchgaenge sind ergaenzbar und deterministisch vereinigbar (Issue #967).
//!
//! # Der Fall, um den es geht
//!
//! [`vier_ergaenzt_um_zwoelf_ist_bitgleich_mit_sechzehn_von_vorn`] (AC3) und
//! [`vertauschte_paketreihenfolge_liefert_denselben_fingerabdruck`] (AC4).
//!
//! # Und der Fall, der sie erst belastbar macht
//!
//! [`kontrollprobe_gleitkomma_in_ankunftsreihenfolge_aendert_den_wert`]. Ohne
//! ihn belegte AC4 nur, dass zwei Zahlen gleich sind — nicht, dass die
//! Gleichheit etwas kostet. Die Kontrollprobe zeigt an **denselben Werten**,
//! dass ein Gleitkomma-Akkumulator hier auseinanderlaeuft.

use rota_sim::akkumulator::{
    ExactHistogram, Festwert, TrialAccumulator, TrialId, TrialIdSet, Vereinigungsfehler, SKALA,
};

/// Eine Einteilung, in der jeder ganzzahlige Wert seine eigene Klasse hat.
///
/// Breite 1 heisst: die Quantile sind **exakt**, nicht genaehert (AC5).
fn histogramm() -> ExactHistogram {
    ExactHistogram::neu(0, 1, 4096)
}

/// Ein Akkumulator ueber `[start, start+anzahl)` mit gut streuenden Werten.
///
/// Die Werte sind verschieden, damit Summe und Quadratsumme nicht zufaellig
/// gleich herauskommen — eine Reihe aus lauter gleichen Zahlen wuerde jede
/// Reihenfolge bestehen und nichts belegen.
fn paket(start: TrialId, anzahl: u32) -> TrialAccumulator {
    let mut akku = TrialAccumulator::neu(histogramm());
    for i in 0..anzahl {
        let id = start + i;
        let wert = 100 + (id * 37) % 901; // 100..1000, ohne Muster im Takt
        akku.trage_ein(id, wert, id.is_multiple_of(3))
            .expect("frische Kennungen");
    }
    akku
}

// ------------------------------------------------------------------- AC1

#[test]
fn gespeichert_wird_ein_akkumulator_und_kein_mittelwert() {
    let akku = paket(0, 4);

    // Die Destrukturierung ohne `..` ist die eigentliche Zusicherung: Ein
    // spaeter hinzugefuegtes Feld `mittelwert` braeche diesen Prueffall beim
    // UEBERSETZEN, nicht erst beim Vergleichen.
    let TrialAccumulator {
        trial_ids,
        count,
        sum,
        sum_squares,
        win_count,
        quantiles,
    } = &akku;

    assert_eq!(*count, 4);
    assert_eq!(trial_ids.anzahl(), 4);
    assert!(*sum > Festwert(0));
    assert!(*sum_squares > 0);
    assert!(*win_count <= *count);
    assert_eq!(quantiles.anzahl(), 4);

    // Der Mittelwert ist eine ABLEITUNG. Er wird bei jedem Aufruf neu
    // gerechnet und kann deshalb nicht veralten, wenn Durchgaenge dazukommen.
    let vorher = akku.mittelwert().expect("vier Durchgaenge");
    let mut ergaenzt = akku.clone();
    ergaenzt
        .vereinige(&paket(4, 12))
        .expect("frische Kennungen");
    let nachher = ergaenzt.mittelwert().expect("sechzehn Durchgaenge");
    assert_ne!(
        vorher.to_bits(),
        nachher.to_bits(),
        "Waere der Mittelwert gespeichert statt abgeleitet, stuende hier noch der alte."
    );
}

// ------------------------------------------------------------------- AC2

#[test]
fn die_kennungen_bleiben_erhalten_nicht_nur_ihre_anzahl() {
    let akku = paket(1000, 8);
    assert_eq!(akku.count, 8);
    for id in 1000..1008 {
        assert!(
            akku.trial_ids.enthaelt(id),
            "Kennung {id} fehlt — ohne sie ist die gepaarte Auswertung mit gemeinsamen \
             Zufallszahlen unmoeglich."
        );
    }
    assert!(!akku.trial_ids.enthaelt(999));
    assert!(!akku.trial_ids.enthaelt(1008));
}

#[test]
fn derselbe_durchgang_wird_nicht_zweimal_gezaehlt() {
    // Der teuerste stille Fehler: Die Summen enthielten dieselben Kaempfe
    // zweimal, der Mittelwert bliebe plausibel, nur die Streuung waere falsch.
    let mut a = paket(0, 4);
    let vorher = a.fingerabdruck();

    let fehler = a
        .vereinige(&paket(2, 4))
        .expect_err("die Kennungen 2 und 3 liegen in beiden Paketen");
    assert_eq!(fehler, Vereinigungsfehler::DurchgaengeUeberschneidenSich);
    assert_eq!(
        a.fingerabdruck(),
        vorher,
        "Nach einem abgelehnten Vereinigen muss der Akkumulator UNVERAENDERT sein — eine \
         halb vollzogene Vereinigung waere schlimmer als gar keine."
    );

    // Und einzeln ebenso.
    assert_eq!(
        a.trage_ein(1, 500, false),
        Err(Vereinigungsfehler::DurchgaengeUeberschneidenSich)
    );
    assert_eq!(a.fingerabdruck(), vorher);
}

// ------------------------------------------------------------------ AC2b

#[test]
fn tausend_fortlaufende_kennungen_sind_ein_bereich_und_nicht_tausend() {
    let mut menge = TrialIdSet::leer();
    for id in 0..1000 {
        menge.vereinige(&TrialIdSet::aus_bereich(id, id + 1));
    }
    assert_eq!(menge.anzahl(), 1000);
    assert_eq!(
        menge.bereichszahl(),
        1,
        "Tausend fortlaufende Kennungen muessen zu EINEM Bereich verschmelzen. Eine Liste \
         waere bei tausenden Durchgaengen je Eintrag zu teuer — im Speicher und beim \
         Vereinigen."
    );
}

#[test]
fn aneinandergrenzende_bereiche_verschmelzen() {
    // Ohne das Verschmelzen haetten 'vier ergaenzt um zwoelf' und 'sechzehn von
    // vorn' verschiedene Darstellungen — und AC3 waere nicht erfuellbar.
    let mut a = TrialIdSet::aus_bereich(0, 4);
    a.vereinige(&TrialIdSet::aus_bereich(4, 16));
    assert_eq!(a.bereichszahl(), 1);
    assert_eq!(a, TrialIdSet::aus_bereich(0, 16));
}

#[test]
fn luecken_bleiben_luecken() {
    // Die Gegenprobe zum Verschmelzen: Ein Bereich, der NICHT angrenzt, darf
    // nicht mit eingezogen werden — sonst enthielte die Menge Kennungen, die
    // nie gerechnet wurden.
    let mut a = TrialIdSet::aus_bereich(0, 4);
    a.vereinige(&TrialIdSet::aus_bereich(5, 8));
    assert_eq!(a.bereichszahl(), 2);
    assert!(!a.enthaelt(4), "die Luecke bei 4 muss eine Luecke bleiben");
    assert_eq!(a.anzahl(), 7);
}

#[test]
fn ueberschneidung_wird_erkannt_egal_wie_sie_liegt() {
    let grund = TrialIdSet::aus_bereich(10, 20);
    for (start, ende, erwartet) in [
        (0, 10, false),  // grenzt an, unten
        (20, 30, false), // grenzt an, oben
        (0, 11, true),   // ragt hinein, unten
        (19, 30, true),  // ragt hinein, oben
        (12, 15, true),  // ganz innen
        (0, 30, true),   // ganz aussen
        (15, 15, false), // leer
    ] {
        let andere = TrialIdSet::aus_bereich(start, ende);
        assert_eq!(
            grund.ueberschneidet(&andere),
            erwartet,
            "Bereich {start}..{ende} gegen 10..20"
        );
        assert_eq!(
            andere.ueberschneidet(&grund),
            erwartet,
            "und andersherum: {start}..{ende} gegen 10..20"
        );
    }
}

// ------------------------------------------------------------------- AC3

#[test]
fn vier_ergaenzt_um_zwoelf_ist_bitgleich_mit_sechzehn_von_vorn() {
    let mut ergaenzt = paket(0, 4);
    ergaenzt
        .vereinige(&paket(4, 12))
        .expect("keine gemeinsamen Kennungen");

    let von_vorn = paket(0, 16);

    // Die Zahlen gehoeren in den Bericht, nicht nur der Vergleich. Sichtbar
    // mit `--nocapture`.
    println!(
        "AC3  4+12 = {:#018x}   16 von vorn = {:#018x}",
        ergaenzt.fingerabdruck(),
        von_vorn.fingerabdruck()
    );

    assert_eq!(
        ergaenzt.fingerabdruck(),
        von_vorn.fingerabdruck(),
        "Vier ergaenzt um zwoelf muss BITGLEICH sein mit sechzehn von vorn — sonst ist der \
         Akkumulator nicht ergaenzbar, sondern nur addierbar."
    );
    assert_eq!(ergaenzt, von_vorn, "und zwar in jedem einzelnen Feld");
    assert_eq!(ergaenzt.count, 16);
    assert_eq!(ergaenzt.trial_ids.bereichszahl(), 1);
    assert_eq!(
        ergaenzt.sum.0, von_vorn.sum.0,
        "die Summe exakt, nicht nur gerundet gleich"
    );
    assert_eq!(ergaenzt.sum_squares, von_vorn.sum_squares);
}

#[test]
fn die_vier_werden_beim_ergaenzen_nicht_neu_gerechnet() {
    // AC3 sagt 'ohne die vier neu zu rechnen'. Messbar daran, dass der
    // Ergaenzungsschritt nur die ZWOELF eintraegt: Ein Zaehler in der
    // Erzeugung wuerde es zeigen — hier genuegt, dass die vier Kennungen
    // unveraendert bleiben und der Akkumulator sie nicht erneut annimmt.
    let mut akku = paket(0, 4);
    let vier_fingerabdruck = akku.fingerabdruck();

    akku.vereinige(&paket(4, 12)).expect("frisch");

    // Die vier sind noch da ...
    for id in 0..4 {
        assert!(akku.trial_ids.enthaelt(id));
    }
    // ... und ein erneutes Eintragen wird abgelehnt, statt sie zu verdoppeln.
    assert_eq!(
        akku.trage_ein(0, 123, false),
        Err(Vereinigungsfehler::DurchgaengeUeberschneidenSich)
    );
    assert_ne!(
        akku.fingerabdruck(),
        vier_fingerabdruck,
        "der Akkumulator hat sich durch die Ergaenzung sehr wohl geaendert"
    );
}

// ------------------------------------------------------------------- AC4

#[test]
fn vertauschte_paketreihenfolge_liefert_denselben_fingerabdruck() {
    let pakete = [paket(0, 5), paket(5, 7), paket(12, 3), paket(15, 9)];

    // Reihenfolge A: 0,1,2,3
    let mut a = pakete[0].clone();
    for p in &pakete[1..] {
        a.vereinige(p).expect("disjunkt");
    }

    // Reihenfolge B: 3,1,0,2 — voellig andere Ankunft
    let mut b = pakete[3].clone();
    for i in [1, 0, 2] {
        b.vereinige(&pakete[i]).expect("disjunkt");
    }

    // Reihenfolge C: paarweise geklammert, (0+1) + (2+3)
    let mut c01 = pakete[0].clone();
    c01.vereinige(&pakete[1]).expect("disjunkt");
    let mut c23 = pakete[2].clone();
    c23.vereinige(&pakete[3]).expect("disjunkt");
    let mut c = c01;
    c.vereinige(&c23).expect("disjunkt");

    println!(
        "AC4  Reihenfolge A = {:#018x}   B = {:#018x}   C (geklammert) = {:#018x}",
        a.fingerabdruck(),
        b.fingerabdruck(),
        c.fingerabdruck()
    );

    assert_eq!(
        a.fingerabdruck(),
        b.fingerabdruck(),
        "Andere ANKUNFTSREIHENFOLGE darf den Fingerabdruck nicht aendern — sonst wuerde ein \
         ausgefallener Zusatzrechner das Ergebnis aendern."
    );
    assert_eq!(
        a.fingerabdruck(),
        c.fingerabdruck(),
        "Andere KLAMMERUNG ebenso wenig — Pakete kommen nicht nur in anderer Reihenfolge, \
         sondern auch anders geschnitten zurueck."
    );
    assert_eq!(a, b);
    assert_eq!(a, c);
    assert_eq!(a.count, 24);
}

#[test]
fn kontrollprobe_gleitkomma_in_ankunftsreihenfolge_aendert_den_wert() {
    // Die Probe, ohne die AC4 nur belegte, dass zwei Zahlen gleich sind: An
    // DENSELBEN Werten laeuft ein Gleitkomma-Akkumulator auseinander.
    //
    // Gewaehlt sind ein sehr grosser und viele sehr kleine Beitraege — die Lage
    // ist im Simulator erreichbar, weil die QUADRATSUMME schnell jenseits von
    // 2^53 liegt: ein einziger Durchgang mit 4e9 Schaden bringt 1,6e19.
    const GROSS: u32 = 4_000_000_000;
    const KLEINE: u32 = 2048; // genau ein ULP bei 1,6e19

    let gross_quadrat = f64::from(GROSS) * f64::from(GROSS);

    // (a) erst die kleinen aufsummieren, dann den grossen dazu
    let mut a = 0.0_f64;
    for _ in 0..KLEINE {
        a += 1.0;
    }
    a += gross_quadrat;

    // (b) erst den grossen, dann die kleinen einzeln
    let mut b = gross_quadrat;
    for _ in 0..KLEINE {
        b += 1.0;
    }

    println!(
        "KONTROLLPROBE  Gleitkomma a = {a:?}   b = {b:?}   gleich = {}",
        a == b
    );

    assert_ne!(
        a.to_bits(),
        b.to_bits(),
        "Die Kontrollprobe muss den Wert AENDERN. Tut sie es nicht, sind die gewaehlten \
         Zahlen zu gutmuetig, und AC4 belegt nichts. Gemessen: a={a:?}, b={b:?}"
    );

    // Und derselbe Inhalt in Festkomma: identisch, in beiden Reihenfolgen.
    let mut fest_a: i128 = 0;
    for _ in 0..KLEINE {
        fest_a += SKALA * SKALA;
    }
    fest_a += (i128::from(GROSS) * SKALA) * (i128::from(GROSS) * SKALA);

    let mut fest_b: i128 = (i128::from(GROSS) * SKALA) * (i128::from(GROSS) * SKALA);
    for _ in 0..KLEINE {
        fest_b += SKALA * SKALA;
    }

    assert_eq!(
        fest_a, fest_b,
        "Festkomma muss in beiden Reihenfolgen exakt gleich sein — das ist der ganze Grund \
         fuer den Weg."
    );
}

#[test]
fn der_fingerabdruck_haengt_an_jedem_feld() {
    // Ein Fingerabdruck, der ein Feld nicht liest, wuerde AC4 bestehen und
    // trotzdem nichts sichern. Jede dieser Aenderungen MUSS ihn bewegen.
    let grund = paket(0, 6);

    let mut anderer_wert = TrialAccumulator::neu(histogramm());
    for i in 0..6 {
        anderer_wert
            .trage_ein(i, 500, i.is_multiple_of(3))
            .expect("frisch");
    }
    assert_ne!(
        grund.fingerabdruck(),
        anderer_wert.fingerabdruck(),
        "andere Messwerte"
    );

    let mut andere_kennungen = TrialAccumulator::neu(histogramm());
    for i in 0..6 {
        let wert = 100 + (i * 37) % 901;
        andere_kennungen
            .trage_ein(i + 100, wert, i.is_multiple_of(3))
            .expect("frisch");
    }
    assert_ne!(
        grund.fingerabdruck(),
        andere_kennungen.fingerabdruck(),
        "andere KENNUNGEN bei gleichen Werten — sonst waere AC2 im Fingerabdruck unsichtbar"
    );

    // Die QUADRATSUMME isoliert. Sie ist das Feld, das am leichtesten aus dem
    // Fingerabdruck faellt, ohne dass es auffaellt: Summe, Kennungen, Siege und
    // Klassenbesetzung bleiben hier alle gleich.
    //
    // {1, 5} und {3, 3} haben dieselbe Summe (6) und — bei Klassenbreite 10 —
    // dieselbe Klasse. Ihre Quadratsummen sind 26 gegen 18.
    let mut streuend = TrialAccumulator::neu(ExactHistogram::neu(0, 10, 4));
    streuend.trage_ein(0, 1, false).expect("frisch");
    streuend.trage_ein(1, 5, false).expect("frisch");
    let mut gleichmaessig = TrialAccumulator::neu(ExactHistogram::neu(0, 10, 4));
    gleichmaessig.trage_ein(0, 3, false).expect("frisch");
    gleichmaessig.trage_ein(1, 3, false).expect("frisch");

    assert_eq!(
        streuend.sum, gleichmaessig.sum,
        "gleiche Summe — Voraussetzung"
    );
    assert_eq!(
        streuend.trial_ids, gleichmaessig.trial_ids,
        "gleiche Kennungen"
    );
    assert_eq!(streuend.win_count, gleichmaessig.win_count, "gleiche Siege");
    assert_eq!(
        streuend.quantiles, gleichmaessig.quantiles,
        "gleiche Klassenbesetzung — sonst traegt das Histogramm den Unterschied"
    );
    assert_ne!(
        streuend.sum_squares, gleichmaessig.sum_squares,
        "und NUR die Quadratsumme unterscheidet sich"
    );
    assert_ne!(
        streuend.fingerabdruck(),
        gleichmaessig.fingerabdruck(),
        "Faellt die Quadratsumme aus dem Fingerabdruck, sind zwei Ergebnisse mit voellig \
         verschiedener Streuung nicht mehr unterscheidbar — und AC4 waere fuer dieses Feld \
         ungedeckt. Gemessen mit der Mutationsprobe: genau das war der Fall."
    );

    let mut andere_siege = TrialAccumulator::neu(histogramm());
    for i in 0..6 {
        let wert = 100 + (i * 37) % 901;
        andere_siege.trage_ein(i, wert, true).expect("frisch");
    }
    assert_ne!(
        grund.fingerabdruck(),
        andere_siege.fingerabdruck(),
        "andere Siegzahl"
    );
}

// ------------------------------------------------------------------- AC5

#[test]
fn bei_klassenbreite_eins_ist_das_quantil_exakt() {
    let mut h = ExactHistogram::neu(0, 1, 100);
    for wert in 1..=100_u64 {
        h.trage_ein(wert - 1);
    }
    assert_eq!(
        h.quantil_fehler(),
        Some(0),
        "Breite 1 heisst: jede Klasse enthaelt genau einen Wert, das Quantil ist exakt."
    );
    assert_eq!(h.quantil(0.5), Some(49));
    assert_eq!(h.quantil(1.0), Some(99));
    assert_eq!(h.quantil(0.0), Some(0));
}

#[test]
fn bei_groesserer_breite_wird_der_fehler_ausgewiesen() {
    // 'Deren Fehler als beschraenkte Naeherung ausgewiesen wird' — also muss
    // die Zahl herauskommen, nicht im Kommentar stehen.
    let mut h = ExactHistogram::neu(0, 10, 10);
    for wert in 0..100_u64 {
        h.trage_ein(wert);
    }
    assert_eq!(h.quantil_fehler(), Some(9));
    let q = h.quantil(0.5).expect("gefuellt");
    assert_eq!(q, 40, "die Untergrenze der Klasse, in der der Median liegt");
}

#[test]
fn bei_ueberlauf_ist_der_fehler_unbeschraenkt_und_wird_nicht_beziffert() {
    // Lieber keine Zahl als eine, die nicht gilt.
    let mut h = ExactHistogram::neu(0, 1, 10);
    h.trage_ein(5);
    assert_eq!(h.quantil_fehler(), Some(0));
    h.trage_ein(999);
    assert_eq!(h.ueberlauf(), 1);
    assert_eq!(
        h.quantil_fehler(),
        None,
        "Mit einem Wert ausserhalb der Klassen ist die Abweichung unbeschraenkt — eine Zahl \
         waere hier eine Behauptung ohne Deckung."
    );
    assert_eq!(h.anzahl(), 2, "der Ueberlauf zaehlt trotzdem mit");
}

#[test]
fn verschiedene_einteilungen_werden_nicht_vereinigt() {
    let mut a = TrialAccumulator::neu(ExactHistogram::neu(0, 1, 100));
    a.trage_ein(0, 5, false).expect("frisch");
    let mut b = TrialAccumulator::neu(ExactHistogram::neu(0, 10, 100));
    b.trage_ein(1, 5, false).expect("frisch");

    let vorher = a.fingerabdruck();
    assert_eq!(
        a.vereinige(&b),
        Err(Vereinigungsfehler::EinteilungVerschieden),
        "Zwei verschieden eingeteilte Quantilstrukturen zu addieren waere die stille \
         Variante des Fehlers, den dieses Modul verhindert."
    );
    assert_eq!(a.fingerabdruck(), vorher, "und nichts wurde halb vollzogen");
}

// -------------------------------------------------- Abgeleitete Groessen

#[test]
fn mittelwert_und_streuung_stimmen_gegen_die_direkte_rechnung() {
    // Gegenprobe an einer zweiten, unabhaengigen Quelle: Der Akkumulator darf
    // nicht nur in sich stimmig sein.
    let werte: Vec<u32> = (0..16).map(|i| 100 + (i * 37) % 901).collect();
    let mut akku = TrialAccumulator::neu(histogramm());
    for (i, w) in werte.iter().enumerate() {
        akku.trage_ein(i as TrialId, *w, false).expect("frisch");
    }

    let n = werte.len() as f64;
    let mittel = werte.iter().map(|w| f64::from(*w)).sum::<f64>() / n;
    let varianz = werte
        .iter()
        .map(|w| (f64::from(*w) - mittel).powi(2))
        .sum::<f64>()
        / (n - 1.0);

    let gemessen_mittel = akku.mittelwert().expect("gefuellt");
    let gemessen_streuung = akku.streuung().expect("gefuellt");

    assert!(
        (gemessen_mittel - mittel).abs() < 1e-9,
        "Mittelwert: {gemessen_mittel} gegen {mittel}"
    );
    assert!(
        (gemessen_streuung - varianz.sqrt()).abs() < 1e-6,
        "Streuung: {gemessen_streuung} gegen {}",
        varianz.sqrt()
    );
}

#[test]
fn ein_leerer_akkumulator_liefert_keine_zahlen_statt_null() {
    // 0 hiesse 'gemessen und null'. Ungemessen ist etwas anderes.
    let akku = TrialAccumulator::neu(histogramm());
    assert_eq!(akku.mittelwert(), None);
    assert_eq!(akku.streuung(), None);
    assert_eq!(akku.siegquote(), None);
    assert_eq!(akku.quantiles.quantil(0.5), None);
    assert!(akku.trial_ids.ist_leer());
}

#[test]
fn ein_einzelner_durchgang_hat_keine_streuung() {
    let mut akku = TrialAccumulator::neu(histogramm());
    akku.trage_ein(0, 500, true).expect("frisch");
    assert_eq!(akku.mittelwert(), Some(500.0));
    assert_eq!(
        akku.streuung(),
        None,
        "Die Stichprobenstreuung braucht n-1 im Nenner — bei n=1 ist sie nicht definiert, \
         und 0 waere eine falsche Antwort."
    );
    assert_eq!(akku.siegquote(), Some(1.0));
}

#[test]
fn festkomma_rundet_gleitkomma_sichtbar_und_bleibt_dann_exakt() {
    // Der Rundungsfehler entsteht EINMAL je Messwert und danach nie wieder.
    let a = Festwert::von_gleitkomma(0.123_456_789);
    assert_eq!(a.0, 123_457, "auf die Skala gerundet, nicht abgeschnitten");

    let b = Festwert::von_gleitkomma(-0.5);
    assert_eq!(b.0, -500_000, "auch negativ symmetrisch");

    assert_eq!(
        Festwert::von_gleitkomma(f64::NAN).0,
        0,
        "ein NaN im Akkumulator waere ansteckend und der ganze Eintrag verloren"
    );
    assert_eq!(Festwert::von_gleitkomma(f64::INFINITY).0, 0);

    assert_eq!(
        Festwert::von_ganzzahl(4242).0,
        4242 * SKALA,
        "ganzzahlige Messwerte gehen OHNE Rundung hinein — das ist der Normalfall"
    );
}
