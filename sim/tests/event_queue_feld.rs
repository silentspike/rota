//! Die Ereigniswarteschlange mit festem Feld (Issue #909, Punkt 2).
//!
//! # Was hier belegt wird
//!
//! **AC5** — die Bitaufteilung des 64-Bit-Schlüssels, nachgerechnet statt
//! angenommen. **AC4** — der Überlauf-Rückfall greift, liefert dieselbe
//! Reihenfolge, und der Zähler steht bei normaler Last auf null.
//!
//! # Warum die Reihenfolge der eigentliche Prüfgegenstand ist
//!
//! Ein Umbau der Warteschlange darf das Ergebnis nicht bewegen. Bei gleichem
//! Zeitpunkt entscheidet der Rang, bei gleichem Rang die Einfügenummer — und
//! genau dort wäre ein Fehler still: Der Kampf liefe weiter, nur eben anders.

use rota_sim::event::{
    schluessel_breiten_passen, EventKind, EventQueue, Schluessel, FELD_KAPAZITAET, RANG_BITS,
    SEQ_BITS, SEQ_MAX, ZEIT_BITS,
};

// ------------------------------------------------------------------- AC5

#[test]
fn die_bitaufteilung_geht_genau_auf() {
    assert_eq!(
        ZEIT_BITS + RANG_BITS + SEQ_BITS,
        64,
        "Die drei Felder müssen den Schlüssel genau füllen — kein Überlappen, kein Rest."
    );
    assert_eq!(ZEIT_BITS, 32, "der volle Bereich von Millis");
    assert_eq!(SEQ_MAX, (1u32 << SEQ_BITS) - 1);
}

#[test]
fn jeder_vergebene_rang_passt_in_sein_feld() {
    // Die Ränge aller dreizehn Ereignisarten. Kommt eine dazu, ohne dass diese
    // Liste wächst, fällt der Vollzähligkeits-Fall darunter.
    let arten = [
        EventKind::DotTick {
            spell: 0,
            generation: 0,
        },
        EventKind::ResourceDecay,
        EventKind::HealTick { generation: 0 },
        EventKind::EnergyTick,
        EventKind::DoomTick,
        EventKind::RunawayTick,
        EventKind::AfflictionTick,
        EventKind::StackTick,
        EventKind::TimelineStep { index: 0 },
        EventKind::BossCastLands { index: 0 },
        EventKind::ChannelTick {
            spell: 0,
            generation: 0,
        },
        EventKind::PlayerCastLands {
            spell: 0,
            generation: 0,
        },
        EventKind::Wakeup,
    ];
    assert_eq!(
        arten.len(),
        13,
        "Dreizehn Ereignisarten. Weicht die Zahl ab, ist diese Liste nicht mehr vollständig \
         — und dann prüft der Rest dieses Falls nur einen Ausschnitt."
    );

    let raenge: Vec<u8> = arten.iter().map(|k| k.rank()).collect();
    assert!(
        schluessel_breiten_passen(&raenge),
        "Ein Rang sprengt sein Feld. Ränge: {raenge:?}, Platz: 0..{}",
        (1u32 << RANG_BITS) - 1
    );

    // Die Zusicherung, dass die Liste die Sache berührt: Ohne sie wäre der
    // Fall auch über einer leeren Liste grün.
    assert!(
        raenge.iter().any(|r| *r > 0),
        "Alle Ränge sind 0 — dann prüft der Vergleich nichts."
    );
    assert_eq!(
        *raenge.iter().max().expect("nicht leer"),
        12,
        "Der höchste vergebene Rang ist 12 (Wakeup). Ändert sich das, muss RANG_BITS \
         nachgerechnet werden."
    );
}

#[test]
fn der_schluessel_ordnet_wie_der_dreifache_vergleich() {
    // time schlägt rang, rang schlägt seq — dieselbe Ordnung wie `Event::cmp`.
    let frueh_hoher_rang = Schluessel::neu(1000, 12, 999);
    let spaet_tiefer_rang = Schluessel::neu(2000, 0, 0);
    assert!(
        frueh_hoher_rang < spaet_tiefer_rang,
        "Die Zeit entscheidet zuerst, unabhängig vom Rang."
    );

    let gleich_zeit_tiefer = Schluessel::neu(1000, 3, 999);
    let gleich_zeit_hoeher = Schluessel::neu(1000, 4, 0);
    assert!(
        gleich_zeit_tiefer < gleich_zeit_hoeher,
        "Bei gleicher Zeit entscheidet der Rang, unabhängig von der Einfügenummer."
    );

    let seq_klein = Schluessel::neu(1000, 3, 7);
    let seq_gross = Schluessel::neu(1000, 3, 8);
    assert!(seq_klein < seq_gross, "und zuletzt die Einfügenummer");

    // Die Zeit muss unbeschädigt herauskommen — sonst stimmt die Maske nicht.
    for zeit in [0, 1, 60_000, u32::MAX] {
        assert_eq!(Schluessel::neu(zeit, 12, SEQ_MAX).zeit(), zeit);
    }
}

// ------------------------------------------------------------------- AC4

#[test]
fn bei_normaler_last_greift_der_rueckfall_nicht() {
    // Die Kontrollprobe, die das Issue ausdrücklich verlangt: Ohne sie belegt
    // das Ausschlagen unter Last nichts.
    let mut q = EventQueue::with_capacity(64);
    for i in 0..FELD_KAPAZITAET as u32 {
        q.push(1000 + i, EventKind::Wakeup);
    }
    assert_eq!(q.len(), FELD_KAPAZITAET);
    assert!(
        !q.im_rueckfall(),
        "Bis zur Feldgröße darf nichts in den Rückfall gehen — sonst misst der Nachweis \
         darunter die falsche Sache."
    );
}

#[test]
fn der_rueckfall_greift_und_liefert_dieselbe_reihenfolge() {
    // Mehr Ereignisse als Plätze, in absichtlich verkehrter Zeitfolge.
    let anzahl = FELD_KAPAZITAET + 20;
    let mut q = EventQueue::with_capacity(128);
    for i in (0..anzahl).rev() {
        q.push(1000 + i as u32, EventKind::Wakeup);
    }

    assert!(
        q.im_rueckfall(),
        "Mit {anzahl} Ereignissen bei {FELD_KAPAZITAET} Plätzen MUSS der Rückfall greifen — \
         ein Rückfall, der im Test nie ausgelöst hat, ist nicht geprüft."
    );
    assert_eq!(q.len(), anzahl);

    // Und jetzt der Punkt: dieselbe Reihenfolge wie ohne Rückfall.
    let mut gesehen = Vec::with_capacity(anzahl);
    while let Some(e) = q.pop() {
        gesehen.push(e.time);
    }
    let erwartet: Vec<u32> = (0..anzahl).map(|i| 1000 + i as u32).collect();
    assert_eq!(
        gesehen, erwartet,
        "Der Rückfall darf nur langsamer sein, nicht anders. Getrennt zu leeren wäre die \
         stille Variante des Fehlers: Die Reihenfolge stimmte dann nur innerhalb jeder Hälfte."
    );
}

#[test]
fn feld_und_rueckfall_werden_gemeinsam_geordnet() {
    // Der Fall, der eine getrennte Leerung auffliegen lässt: Das späteste
    // Ereignis geht ins Feld, das früheste in den Rückfall.
    let mut q = EventQueue::with_capacity(64);
    for _ in 0..FELD_KAPAZITAET {
        q.push(9000, EventKind::Wakeup); // füllt das Feld mit SPÄTEN
    }
    assert!(!q.im_rueckfall());
    q.push(1000, EventKind::Wakeup); // dieses hier landet im Rückfall
    assert!(q.im_rueckfall());

    assert_eq!(
        q.peek_time(),
        Some(1000),
        "Das früheste Ereignis liegt im RÜCKFALL. Wer erst das Feld leert, liefert 9000 — \
         und der Kampf liefe in falscher Reihenfolge weiter, ohne abzustürzen."
    );
    assert_eq!(q.pop().map(|e| e.time), Some(1000));
    assert_eq!(q.pop().map(|e| e.time), Some(9000));
}

// ---------------------------------------------------- Ordnung wie vorher

#[test]
fn gleiche_zeit_gleicher_rang_entscheidet_die_einfuegenummer() {
    let mut q = EventQueue::with_capacity(16);
    for i in 0..5u16 {
        q.push(2000, EventKind::HealTick { generation: i });
    }
    let mut folge = Vec::new();
    while let Some(e) = q.pop() {
        if let EventKind::HealTick { generation } = e.kind {
            folge.push(generation);
        }
    }
    assert_eq!(
        folge,
        vec![0, 1, 2, 3, 4],
        "Bei gleicher Zeit UND gleichem Rang entscheidet die Reihenfolge des Einfügens. Das \
         feste Feld ist unsortiert — ohne die Einfügenummer im Schlüssel wäre die Ordnung \
         hier die des Speicherplatzes."
    );
}

#[test]
fn das_zuruecksetzen_laesst_nichts_stehen() {
    let mut q = EventQueue::with_capacity(64);
    for i in 0..FELD_KAPAZITAET as u32 + 5 {
        q.push(1000 + i, EventKind::Wakeup);
    }
    q.clear();
    assert!(q.is_empty());
    assert_eq!(q.len(), 0);
    assert!(!q.im_rueckfall());
    assert_eq!(q.peek_time(), None);

    // Und die Einfügenummer beginnt wieder bei null — sonst wüchse sie über
    // alle Kämpfe hinweg und sprengte irgendwann ihr Bitfeld.
    q.push(500, EventKind::Wakeup);
    assert_eq!(q.pop().map(|e| e.seq()), Some(0));
}
