//! Bitmengen und Listen ohne feste Breite (Issue #917, AC3).
//!
//! **Eine Bitmenge, die überläuft, meldet keinen Fehler, sie verliert Regeln.**
//! Jeder Fall hier prüft eine Breite an ihrer Kante: das letzte Element, das
//! hineinpasst, und das erste, das nicht hineinpasst — und dieses muss
//! abbrechen, nicht verschwinden.

use rota_sim::bitmenge::{
    woerter_fuer, ActiveSpells, Bits, RuleBits, SpellBits, LEISTE_INLINE, ZAUBER_WOERTER,
};
use rota_sim::SPELL_COUNT;

#[test]
fn die_wortzahl_waechst_mit_der_elementzahl() {
    // ≤ 64 ein Wort, 65–128 zwei, darüber der Blockpfad.
    assert_eq!(woerter_fuer(1), 1);
    assert_eq!(woerter_fuer(64), 1);
    assert_eq!(woerter_fuer(65), 2);
    assert_eq!(woerter_fuer(128), 2);
    assert_eq!(woerter_fuer(129), 3);
    assert_eq!(woerter_fuer(1000), 16);
}

#[test]
fn die_zaubermenge_ist_so_breit_wie_die_daten() {
    assert_eq!(ZAUBER_WOERTER, woerter_fuer(SPELL_COUNT));
    let mut alle = SpellBits::leer();
    for zauber in 0..SPELL_COUNT {
        alle.setzen(zauber);
    }
    assert_eq!(alle.anzahl() as usize, SPELL_COUNT);
    assert!((0..SPELL_COUNT).all(|z| alle.enthaelt(z)));
    assert_eq!(alle.elemente().count(), SPELL_COUNT);
}

#[test]
fn jede_breite_fasst_ihre_kante() {
    let mut eins = Bits::<1>::leer();
    eins.setzen(63);
    assert!(eins.enthaelt(63) && !eins.enthaelt(62));
    let mut zwei = Bits::<2>::leer();
    zwei.setzen(64);
    zwei.setzen(127);
    assert_eq!(zwei.elemente().collect::<Vec<_>>(), vec![64, 127]);
    // Der Blockpfad: drei Wörter, jedes Element.
    let mut drei = Bits::<3>::leer();
    for i in 0..192 {
        drei.setzen(i);
    }
    assert_eq!(drei.anzahl(), 192);
}

#[test]
#[should_panic(expected = "verlöre ihn still")]
fn ein_wort_bricht_beim_65_element_ab() {
    let mut eins = Bits::<1>::leer();
    eins.setzen(64);
}

#[test]
#[should_panic(expected = "verlöre ihn still")]
fn zwei_woerter_brechen_beim_129_element_ab() {
    let mut zwei = Bits::<2>::leer();
    zwei.setzen(128);
}

#[test]
fn die_regelmenge_waechst_ueber_128_in_den_blockpfad() {
    // 93 Kandidatenzeilen und je Schule eine neue: 98 — eine feste Breite von
    // 96 hätte zwei verloren.
    let mut knapp = RuleBits::mit_breite(98);
    knapp.setzen(97);
    assert!(knapp.enthaelt(97) && !knapp.im_blockpfad());

    let mut gross = RuleBits::mit_breite(300);
    assert!(gross.im_blockpfad());
    for i in (0..300).step_by(7) {
        gross.setzen(i);
    }
    assert_eq!(gross.anzahl() as usize, (0..300).step_by(7).count());
    assert!((0..300).all(|i| gross.enthaelt(i) == (i % 7 == 0)));
}

#[test]
#[should_panic(expected = "verlöre sie still")]
fn die_regelmenge_bricht_jenseits_ihrer_breite_ab() {
    let mut regeln = RuleBits::mit_breite(200);
    regeln.setzen(200);
}

#[test]
fn zwanzig_tasten_laufen_in_den_ueberlauf_ohne_verlust() {
    // Der Lastfall des Auftraggebers: 20 Tasten bei einer Leiste von 16.
    let mut aktiv: ActiveSpells = ActiveSpells::new();
    let zauber: Vec<usize> = (0..20).map(|i| (i * 3) % SPELL_COUNT).collect();
    for &z in &zauber {
        aktiv.push(z);
    }
    assert_eq!(aktiv.len(), 20);
    assert!(
        aktiv.ueberlaeuft(),
        "20 > {LEISTE_INLINE}: der Überlauf muss greifen"
    );
    assert_eq!(aktiv.iter().collect::<Vec<_>>(), zauber);
    assert_eq!(aktiv.global(19), Some(zauber[19]));
    assert_eq!(aktiv.global(20), None);

    let mut leiste: ActiveSpells = ActiveSpells::new();
    for z in 0..LEISTE_INLINE {
        leiste.push(z);
    }
    assert!(
        !leiste.ueberlaeuft(),
        "eine volle Leiste fordert keinen Speicher an"
    );
}
